//! GMap tile zone state (`.\Source\Sim\Map`), the per-kingdom fog words and
//! the tile routines driven by zone types.
//!
//! Tile (0x30 bytes) fields owned here: `+4` zone set ([`crate::set`]),
//! `+0x14` fog word, `+0x18` supply word. The actor list `+8` belongs to the
//! spatial index and is read through [`ZoneWorld::tile_actors`].
//!
//! Fog word (tile `+0x14`, and the same layout at actor `+0x54`): bit `k`
//! (`k` = kingdom `+0x18C`, shift taken mod 32) = EXPLORED by kingdom k (set
//! by detection, cleared only by the "fog restored in area" trigger pass
//! `0x54F930`); bit `k + 16` (`0xA11740` = 16) = VISIBLE to kingdom k (set by
//! detection, cleared when no detecting zone covers the tile any more,
//! `0x54F5E0`). Non-major kingdoms (`+0x190 != 0`) are never marked and always
//! read as explored/visible. Supply word (`+0x18`): bit `k` = a supply zone
//! (type 0) of kingdom k covers the tile.
use crate::query::{Kingdom, VisitWorld, any_owned_by, detects};
use crate::set::{ActorKey, SetRef, ZonePool};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Weak};

/// Bit offset of the visible half (`0xA11740`).
pub const VISIBLE_SHIFT: u32 = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileZones {
    /// `+4`: interned zone set (None = null).
    pub set: Option<SetRef>,
    /// `+0x14`: explored (low 16) / visible (high 16) bits.
    pub fog: u32,
    /// `+0x18`: supply-zone presence bits.
    pub supply: u16,
}

/// GMap fields read by the zone code (GWorld+0x30).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapGeometry {
    /// `+4`: world-to-tile shift.
    pub shift: u32,
    /// `+8`: tile size (binary32 bits); also the clamp minimum of `0x552640`.
    pub tile_size_bits: u32,
    /// `+0xC`/`+0x10`: world extent (binary32 bits), clamp maximum base.
    pub extent_bits: [u32; 2],
    /// `+0x1C`/`+0x20`: tiles per row / rows.
    pub width: i32,
    pub height: i32,
}

/// The zone-relevant state of every tile plus the global intern tables.
#[derive(Debug)]
pub struct ZoneMap {
    pub geometry: MapGeometry,
    pub pool: ZonePool,
    pub tiles: Vec<TileZones>,
    /// Flood-solver shared state (busy byte, stamp counter, tile +0x24).
    pub solver: crate::flood::SolverState,
    identity: Arc<()>,
}

/// Opaque weak witness to one concrete map owner. Moves preserve it; deep
/// clones have a new identity. Weak ownership prevents allocation-address ABA
/// without keeping the map or its tiles alive.
#[derive(Clone, Debug)]
pub struct MapOwnerWitness {
    identity: Weak<()>,
}
impl MapOwnerWitness {
    pub fn matches(&self, map: &ZoneMap) -> bool {
        self.identity
            .upgrade()
            .is_some_and(|owner| Arc::ptr_eq(&owner, &map.identity))
    }
}
impl Clone for ZoneMap {
    fn clone(&self) -> Self {
        Self {
            geometry: self.geometry,
            pool: self.pool.clone(),
            tiles: self.tiles.clone(),
            solver: self.solver.clone(),
            identity: Arc::new(()),
        }
    }
}

impl ZoneMap {
    pub fn new(geometry: MapGeometry) -> Result<Self> {
        if geometry.width < 0 || geometry.height < 0 {
            return Err(Error::World("negative map size".into()));
        }
        let n = geometry.width as usize * geometry.height as usize;
        Ok(Self {
            geometry,
            pool: ZonePool::new(),
            tiles: vec![TileZones::default(); n],
            solver: crate::flood::SolverState::new(n, None),
            identity: Arc::new(()),
        })
    }

    pub fn owner_witness(&self) -> MapOwnerWitness {
        MapOwnerWitness {
            identity: Arc::downgrade(&self.identity),
        }
    }

    /// Tile index of `(x, y)` when inside the map.
    pub fn index(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && y >= 0 && x < self.geometry.width && y < self.geometry.height)
            .then(|| y as usize * self.geometry.width as usize + x as usize)
    }

    pub fn tile(&self, t: usize) -> Result<&TileZones> {
        self.tiles.get(t).ok_or(Error::TileOutside(t as i64))
    }

    fn tile_mut(&mut self, t: usize) -> Result<&mut TileZones> {
        self.tiles.get_mut(t).ok_or(Error::TileOutside(t as i64))
    }
}

/// SAI region-counter hooks called on [B3652C] when [B36528]+0x20 is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaiHook {
    /// `0x7A99A0`: supply bit set (region +0xC += 1).
    OccupiedInc,
    /// `0x7A99D0`: supply bit cleared (region +0xC -= 1).
    OccupiedDec,
    /// `0x7A9A00`: explored bit set (region +8 += 1).
    ExploredInc,
    /// `0x7A9A30`: visible bit set (region +0x10 += 1).
    VisibleInc,
    /// `0x7A9A60`: visible bit cleared (`0x7B8E70` white-fog timer).
    VisibleDec,
    /// `0x7A9A90`: explored bit cleared (region +8 -= 1).
    ExploredDec,
}

/// Component effects reached from zones covering actors (`0x514E40`,
/// `0x514E00`, `0x514E20`); each is owned by its component's port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentEffect {
    /// `0x568D80` on zone actor +0x88 (type 5 ZOG / guard).
    Guard,
    /// `0x57AB70` on zone actor +0x7C (type 4 ZOC, organization).
    ControlOrganization,
    /// `0x567840` on zone actor +0xB0 (type 4 ZOC, component +0xB0).
    ControlB0,
    /// `0x58CEA0` on zone actor +0x90 (type 3).
    Type3,
    /// `0x564300` on zone actor +0xD4 (type 8).
    Type8,
}

/// World services of the zone/fog routines, called at the native points.
pub trait ZoneWorld: VisitWorld {
    /// `[[B39AB0]+0x108] != 0` (fog of war enabled).
    fn fog_enabled(&mut self) -> Result<bool>;
    /// `[B36528]+0x20` (SAI region tracking enabled).
    fn sai_enabled(&mut self) -> Result<bool>;
    fn sai(&mut self, hook: SaiHook, tile: usize, kingdom: u32) -> Result<()>;
    /// GWorld+0x154 vector (with null holes), in order.
    fn kingdom_list(&mut self) -> Result<Vec<u32>>;
    /// GWorld+0x158 (the length of [`Self::kingdom_list`]); a world may
    /// answer it without building the list.
    fn kingdom_count(&mut self) -> Result<usize> {
        Ok(self.kingdom_list()?.len())
    }
    /// GWorld+0x154[i] for `i <` [`Self::kingdom_count`].
    fn kingdom_at(&mut self, i: usize) -> Result<u32> {
        self.kingdom_list()?.get(i).copied().ok_or(Error::World(format!("kingdom list index {i}")))
    }
    /// Tile `+8` actor list, head first.
    fn tile_actors(&mut self, tile: usize) -> Result<Vec<ActorKey>>;
    /// Actor virtual `+0x64(kingdom, explored, visible)` (`0x4EC010` for GActor).
    fn actor_reveal(
        &mut self,
        actor: ActorKey,
        kingdom: u32,
        explored: bool,
        visible: bool,
    ) -> Result<()>;
    /// Actor virtual `+0x68(kingdom)` (`0x4EC1B0`).
    fn actor_hide(&mut self, actor: ActorKey, kingdom: u32) -> Result<()>;
    /// Actor virtual `+0x6C(kingdom)` (`0x4EA0C0`).
    fn actor_unexplore(&mut self, actor: ActorKey, kingdom: u32) -> Result<()>;
    /// `0x54F7F0` gate: `[B36550]+0xE4 != 3 || GWorld+0x48 != 0`.
    fn zone_effects_enabled(&mut self) -> Result<bool>;
    /// Actor virtual `+0xB4`.
    fn actor_b4(&mut self, actor: ActorKey) -> Result<bool>;
    /// `0x4EE700` (multi-tile actor test).
    fn actor_multi_tile(&mut self, actor: ActorKey) -> Result<bool>;
    /// `0x4EA200`: tile index under the actor (actor +0x44/+0x48), as the
    /// native pointer arithmetic computes it (may be outside the map).
    fn actor_tile(&mut self, actor: ActorKey) -> Result<i64>;
    /// Actor `+8` flags word.
    fn actor_flags(&mut self, actor: ActorKey) -> Result<u32>;
    /// Actor `+0x80` component: `Some(+0x10)` when it and `+0x10` are non-null
    /// and byte `+0x1C` is set (the zone effect then targets `+0x10`).
    fn actor_redirect(&mut self, actor: ActorKey) -> Result<Option<ActorKey>>;
    /// Actor component pointer at `offset` (+0x7C, +0x88, +0x90, +0xB0, +0xD4).
    fn component(&mut self, actor: ActorKey, offset: u32) -> Result<u32>;
    fn component_effect(
        &mut self,
        effect: ComponentEffect,
        component: u32,
        target: ActorKey,
    ) -> Result<()>;
}

fn bit(index: u32) -> u32 {
    1u32.wrapping_shl(index)
}

/// `0x54ECB0`: explored by `k` (non-major: true). The original dereferences
/// a null kingdom.
pub fn explored(word: u32, k: Option<&Kingdom>) -> Result<bool> {
    let k = k.ok_or(Error::NativeNull("kingdom (0x54ECB0)"))?;
    Ok(k.nonmajor != 0 || word & bit(k.index) != 0)
}

/// `0x54EC70`: visible to `k` (non-major: true).
pub fn visible(word: u32, k: Option<&Kingdom>) -> Result<bool> {
    let k = k.ok_or(Error::NativeNull("kingdom (0x54EC70)"))?;
    Ok(k.nonmajor != 0 || word & bit(k.index.wrapping_add(VISIBLE_SHIFT)) != 0)
}

fn set_bit(word: &mut u32, k: Option<&Kingdom>, offset: u32, on: bool) -> Result<()> {
    let k = k.ok_or(Error::NativeNull("kingdom (fog word update)"))?;
    if k.nonmajor == 0 {
        let b = bit(k.index.wrapping_add(offset));
        if on {
            *word |= b;
        } else {
            *word &= !b;
        }
    }
    Ok(())
}

/// `0x54EA80` / `0x54EAB0`: set / clear explored.
pub fn set_explored(word: &mut u32, k: Option<&Kingdom>, on: bool) -> Result<()> {
    set_bit(word, k, 0, on)
}

/// `0x54EAE0` / `0x54EB10`: set / clear visible.
pub fn set_visible(word: &mut u32, k: Option<&Kingdom>, on: bool) -> Result<()> {
    set_bit(word, k, VISIBLE_SHIFT, on)
}

/// `0x54EB40`: explored by any kingdom sharing vision with `k` (null or
/// non-major: true).
pub fn shared_explored(word: u32, k: Option<&Kingdom>) -> bool {
    match k {
        Some(k) if k.nonmajor == 0 => k.shared_mask & word & 0xffff != 0,
        _ => true,
    }
}

/// `0x54EB70`: visible to any kingdom sharing vision with `k`.
pub fn shared_visible(word: u32, k: Option<&Kingdom>) -> bool {
    match k {
        Some(k) if k.nonmajor == 0 => k.shared_mask & word & 0xffff_0000 != 0,
        _ => true,
    }
}

/// `0x54EA50`: replace the explored half.
pub fn reset_explored(word: &mut u32, value: u16) {
    *word = (*word & 0xffff_0000) | u32::from(value);
}

/// `0x54EA60`: replace the visible half (`value << 0xA11740`).
pub fn reset_visible(word: &mut u32, value: u16) {
    *word = (*word & 0xffff) | (u32::from(value) << VISIBLE_SHIFT);
}

/// `0x54F3C0`: fog disabled, or the tile is visible to `k` itself.
pub fn visible_or_no_fog<W: ZoneWorld + ?Sized>(
    w: &mut W,
    word: u32,
    k: Option<&Kingdom>,
) -> Result<bool> {
    if !w.fog_enabled()? {
        return Ok(true);
    }
    visible(word, k)
}

/// `0x54F3E0`: fog disabled, or visible to a vision-sharing kingdom.
pub fn shared_visible_or_no_fog<W: ZoneWorld + ?Sized>(
    w: &mut W,
    word: u32,
    k: Option<&Kingdom>,
) -> Result<bool> {
    if !w.fog_enabled()? {
        return Ok(true);
    }
    Ok(shared_visible(word, k))
}

fn kingdom_of<W: VisitWorld + ?Sized>(w: &mut W, key: u32) -> Result<Option<Kingdom>> {
    if key == 0 {
        Ok(None)
    } else {
        w.kingdom(key).map(Some)
    }
}

/// `0x54F4F0(kingdom, explore, visible)`: mark the tile explored and/or
/// visible for a kingdom, with SAI counters and actor callbacks
/// (`+0x64(k, explored-now, visible)`) when something new was revealed.
pub fn reveal<W: ZoneWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    t: usize,
    kingdom: u32,
    explore: bool,
    show: bool,
) -> Result<()> {
    let k = kingdom_of(w, kingdom)?;
    let mut notify = false;
    if explore && !explored(map.tile(t)?.fog, k.as_ref())? {
        set_explored(&mut map.tile_mut(t)?.fog, k.as_ref(), true)?;
        notify = show;
        if w.sai_enabled()? {
            w.sai(SaiHook::ExploredInc, t, kingdom)?;
        }
    }
    if show && !visible(map.tile(t)?.fog, k.as_ref())? {
        let no_fog = !w.fog_enabled()?;
        set_visible(&mut map.tile_mut(t)?.fog, k.as_ref(), true)?;
        if w.sai_enabled()? && !no_fog {
            w.sai(SaiHook::VisibleInc, t, kingdom)?;
        }
        notify = true;
    }
    if !notify {
        return Ok(());
    }
    let e = explored(map.tile(t)?.fog, k.as_ref())?;
    for a in w.tile_actors(t)? {
        w.actor_reveal(a, kingdom, e, show)?;
    }
    Ok(())
}

/// `0x54F5E0` (and thunk `0x54F690`): when no zone of the tile detects for
/// `kingdom` (slot `+0x40`) and the tile is visible to it, clear visible,
/// count it down for the SAI and tell the actors (`+0x68`).
pub fn hide<W: ZoneWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    t: usize,
    kingdom: u32,
) -> Result<()> {
    let k = kingdom_of(w, kingdom)?;
    if let Some(s) = map.tile(t)?.set {
        let kk = k.ok_or(Error::NativeNull("kingdom (slot +0x40)"))?;
        if detects(&map.pool, s, w, &kk)? {
            return Ok(());
        }
    }
    if !visible(map.tile(t)?.fog, k.as_ref())? {
        return Ok(());
    }
    set_visible(&mut map.tile_mut(t)?.fog, k.as_ref(), false)?;
    let c = if !w.fog_enabled()? {
        true
    } else {
        visible(map.tile(t)?.fog, k.as_ref())?
    };
    if w.sai_enabled()? && !c {
        w.sai(SaiHook::VisibleDec, t, kingdom)?;
    }
    for a in w.tile_actors(t)? {
        w.actor_hide(a, kingdom)?;
    }
    Ok(())
}

/// `0x54ED20`: set the kingdom's supply bit (SAI counter first) when clear.
pub fn occupy<W: ZoneWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    t: usize,
    kingdom: u32,
) -> Result<()> {
    let k = kingdom_of(w, kingdom)?;
    if crate::query::supply_bit(map.tile(t)?.supply, k.as_ref()) {
        return Ok(());
    }
    if w.sai_enabled()? {
        w.sai(SaiHook::OccupiedInc, t, kingdom)?;
    }
    let k = k.ok_or(Error::NativeNull("kingdom (0x559570)"))?;
    if k.nonmajor == 0 {
        map.tile_mut(t)?.supply |= bit(k.index) as u16;
    }
    Ok(())
}

/// `0x54F6A0`: unless another supply zone of the kingdom covers the tile
/// (slot `+0x14(k, 1, 0)`), clear its supply bit and count it down (the SAI
/// hook runs even when the bit was already clear).
pub fn unoccupy<W: ZoneWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    t: usize,
    kingdom: u32,
) -> Result<()> {
    if let Some(s) = map.tile(t)?.set
        && any_owned_by(&map.pool, s, w, kingdom, 1, 0)?
    {
        return Ok(());
    }
    let k = kingdom_of(w, kingdom)?.ok_or(Error::NativeNull("kingdom (0x5595A0)"))?;
    if k.nonmajor == 0 {
        map.tile_mut(t)?.supply &= !(bit(k.index) as u16);
    }
    if w.sai_enabled()? {
        w.sai(SaiHook::OccupiedDec, t, kingdom)?;
    }
    Ok(())
}

/// `0x54F930`: trigger "fog restored in area" per tile. With fog enabled, for
/// every listed kingdom that explored the tile and no zone detects for it,
/// clear explored, count it down and tell the actors (`+0x6C`).
pub fn restore_fog<W: ZoneWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, t: usize) -> Result<()> {
    if !w.fog_enabled()? {
        return Ok(());
    }
    for key in w.kingdom_list()? {
        if key == 0 {
            continue;
        }
        let k = w.kingdom(key)?;
        if !explored(map.tile(t)?.fog, Some(&k))? {
            continue;
        }
        if let Some(s) = map.tile(t)?.set
            && detects(&map.pool, s, w, &k)?
        {
            continue;
        }
        set_explored(&mut map.tile_mut(t)?.fog, Some(&k), false)?;
        if w.sai_enabled()? {
            w.sai(SaiHook::ExploredDec, t, key)?;
        }
        for a in w.tile_actors(t)? {
            w.actor_unexplore(a, key)?;
        }
    }
    Ok(())
}

/// Kingdom alliance query used by the shared-vision masks.
pub trait AllianceWorld: ZoneWorld {
    /// Kingdom virtual `+0x138(other)` (allied).
    fn kingdom_allied(&mut self, kingdom: u32, other: u32) -> Result<bool>;
}

/// `0x4E0F90` (Kingdom): recompute the shared-vision mask `+0x228`
/// (`0x54EBA0`: bits k and k+16 of every listed major kingdom k allied with
/// this one, itself included when it answers allied) and then the supply mask
/// `+0x22C` (`0x559600`: bit k), each with its own pass of `+0x138` calls. A
/// non-major kingdom gets 0 masks without any call.
pub fn kingdom_masks<W: AllianceWorld + ?Sized>(
    w: &mut W,
    kingdom: &Kingdom,
) -> Result<(u32, u16)> {
    let mut shared = 0u32;
    let mut supply = 0u16;
    if kingdom.nonmajor != 0 {
        return Ok((shared, supply));
    }
    let list = w.kingdom_list()?;
    for &k in list.iter().filter(|&&k| k != 0) {
        if w.kingdom_allied(kingdom.key, k)? {
            let kk = w.kingdom(k)?;
            if kk.nonmajor == 0 {
                shared |= bit(kk.index);
                shared |= bit(kk.index.wrapping_add(VISIBLE_SHIFT));
            }
        }
    }
    for &k in list.iter().filter(|&&k| k != 0) {
        if w.kingdom_allied(kingdom.key, k)? {
            let kk = w.kingdom(k)?;
            if kk.nonmajor == 0 {
                supply |= bit(kk.index) as u16;
            }
        }
    }
    Ok((shared, supply))
}
