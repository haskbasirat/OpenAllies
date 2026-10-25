//! KKC_Zone (`.\Source\Sim\Zones.cpp`, vtable `0xA3229C`, 0x18 bytes):
//! `{vtable, actor +4, radius +8 (binary32, -1.0 = inactive), tile x +0xC,
//! tile y +0x10, flags u16 +0x14 (= 1 << zone type), state byte +0x16}`.
//!
//! Actors keep up to nine zones in `actor+0x148` (vector `{data, size +0x14C,
//! capacity +0x150}`, created at size 9 by `KKC_GActor::CreateZone`
//! `0x4D69A0`; slot = zone type). Zone types and their creators (native):
//! 0 ZOS supply (SupplyComponent `0x58B0E0`), 1 RZOD reveal-to-listed
//! kingdoms and 6 ZOD detection (detection component at actor+0xB8,
//! `0x56EB40`/`0x56EC10`/`0x56EFC0`), 2 discovery (`0x4D7230`,
//! component vtable `0xA3EEA0` `0x585940`, radius `0x4BB000`), 3 (`0x58CF50`),
//! 4 ZOC (component `0xA3BBD4` `0x566DB0`, Organization `0x579DD0`), 5 ZOG
//! (`0x568040`), 7 (`0x4DCDA0`), 8 (`0x563E10`).
use crate::fpu;
use crate::query::KINGDOM_BITS;
use crate::set::ActorKey;
use crate::solver;
use crate::tile::{ComponentEffect, ZoneMap, ZoneWorld, hide, occupy, reveal, unoccupy};
use crate::{Error, Result};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};

/// Inactive radius (`0xBF800000`, -1.0).
pub const INACTIVE: u32 = fpu::MINUS_ONE_BITS;
/// Zone slots per actor (`0x4D69A0` sizes the vector to 9; queries scan 9 bits).
pub const ZONE_TYPES: u32 = 9;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Zone {
    /// Native identity of the zone object (pointer), reported to services.
    pub key: u32,
    pub actor: ActorKey,
    pub radius_bits: u32,
    pub x: i32,
    pub y: i32,
    pub flags: u16,
    pub state: u8,
}

impl Zone {
    /// `0x516640(actor, flags)`: inactive; state bit0 unless the zone is a
    /// type 1/6 zone (`flags & 0x42`), bit1 (fast circle builder) for types
    /// 1, 2, 6 (`flags & 0x46`).
    pub fn new(key: u32, actor: ActorKey, flags: u16) -> Self {
        let mut state = 1u8;
        if flags as u8 & 0x42 != 0 {
            state = 0;
        }
        if flags as u8 & 0x46 != 0 {
            state |= 2;
        }
        Self { key, actor, radius_bits: INACTIVE, x: 0, y: 0, flags, state }
    }

    /// Virtual `+4` (`0x514920`): state bit 0.
    pub fn state_bit0(&self) -> bool {
        self.state & 1 != 0
    }

    /// Virtual `+8` (`0x514930`): state bit 1 (fast circle builder eligible).
    pub fn fast_eligible(&self) -> bool {
        self.state & 2 != 0
    }

    /// `0x515940(bit, on)`: set/clear a state bit (byte shift).
    pub fn set_state_bit(&mut self, bit: u8, on: bool) {
        let m = 1u8.wrapping_shl(u32::from(bit));
        if on {
            self.state |= m;
        } else {
            self.state &= !m;
        }
    }

    /// Virtual `+0xC` (`0x514CB0`): a ZOC zone (`flags & 0x10`) does not pass
    /// a tile whose blocker word is non-zero.
    pub fn passes(&self, blocker: u32) -> bool {
        !(blocker != 0 && self.flags as u8 & 0x10 != 0)
    }

    pub fn active(&self) -> bool {
        self.radius_bits != INACTIVE
    }
}

/// `0x514CD0`: zone flags that survive an actor's death flag (types 1 and 6).
pub fn kept_on_dead_actor(flags: u8) -> bool {
    flags & 0x42 != 0
}

/// Services of the zone object beyond tiles.
pub trait ZoneObjectWorld: crate::flood::FloodWorld {
    /// GWorld+0x244 (release solver) / +0x248 (move solver) non-null.
    fn solver_present(&mut self, release: bool) -> Result<bool>;
    /// SVar `ZoneBuilderFastDetectionZones` (shipped Data/Game/SVars.tgi: true).
    fn fast_detection_zones(&mut self) -> Result<bool>;
    /// `0x50C4C0` on [GWorld+0x130]: zone presentation (owner, centre, radius,
    /// flags). The original gates it internally on GWorld+0xD8 != 6 and the
    /// presentation object's +4 (UI owner).
    fn presentation(&mut self, owner: u32, x_bits: u32, y_bits: u32, radius_bits: u32, flags: u16) -> Result<()>;
    /// `0x404F60` "actor %d implicitly clearing zones %x" (Zones.cpp 0x44A).
    fn log_implicit_clear(&mut self, actor_id: u32, flags: u16) -> Result<()>;
    /// Actor +0x14 (id for the log).
    fn actor_id(&mut self, actor: ActorKey) -> Result<u32>;
}

fn presentation_point(map: &ZoneMap, zone: &Zone, p: Precision) -> Result<(u32, u32)> {
    let g = &map.geometry;
    if p == Precision::Double53 {
        use fpu::pc53;
        let fast = (|| {
            let half = pc53::half_tile(g.tile_size_bits)?;
            let y = pc53::cell_center(zone.y, g.shift, half)?;
            let x = pc53::cell_center(zone.x, g.shift, half)?;
            Some((pc53::store(x)?, pc53::store(y)?))
        })();
        if let Some(v) = fast {
            return Ok(v);
        }
    }
    let half = fpu::half_tile(g.tile_size_bits, p)?;
    let y = fpu::cell_center(zone.y, g.shift, &half, p)?;
    let x = fpu::cell_center(zone.x, g.shift, &half, p)?;
    Ok((fpu::f32_store(&x)?, fpu::f32_store(&y)?))
}

/// `0x514CE0` (KKC_Zone release): when active and GWorld+0x244 exists, remove
/// the zone from its tiles (`0x540590`) and present the change (radius read
/// after the solver); then mark inactive.
pub fn release<W: ZoneObjectWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &mut Zone, p: Precision) -> Result<()> {
    if !zone.active() {
        return Ok(());
    }
    if w.solver_present(true)? {
        solver::remove_zone(map, w, zone, p)?;
        let (x, y) = presentation_point(map, zone, p)?;
        let owner = w.owner(zone.actor)?;
        w.presentation(owner, x, y, zone.radius_bits, zone.flags)?;
    }
    zone.radius_bits = INACTIVE;
    Ok(())
}

/// `0x515970` (KKC_Zone::Move): a dead actor (`+8` bit 0) clears every zone
/// but types 1/6 (logged); a move of more than one tile releases first; then
/// the GWorld+0x248 solver adds the zone at `(x, y, radius)` (`moved` = an
/// active zone moved by at most one tile: incremental update) and the change
/// is presented.
pub fn move_to_tile<W: ZoneObjectWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    zone: &mut Zone,
    x: i32,
    y: i32,
    radius_bits: u32,
    p: Precision,
) -> Result<()> {
    if w.actor_flags(zone.actor)? & 1 != 0 && !kept_on_dead_actor(zone.flags as u8) {
        let id = w.actor_id(zone.actor)?;
        w.log_implicit_clear(id, zone.flags)?;
        if zone.active() {
            release(map, w, zone, p)?;
        }
        return Ok(());
    }
    let mut moved = false;
    if zone.active() {
        if x.wrapping_sub(zone.x).wrapping_abs() <= 1 && y.wrapping_sub(zone.y).wrapping_abs() <= 1 {
            moved = true;
        } else {
            release(map, w, zone, p)?;
        }
    }
    if !w.solver_present(false)? {
        return Ok(());
    }
    solver::add_zone(map, w, zone, x, y, radius_bits, moved, p)?;
    let (px, py) = presentation_point(map, zone, p)?;
    let owner = w.owner(zone.actor)?;
    w.presentation(owner, px, py, zone.radius_bits, zone.flags)
}

/// `0x552640`: clamp a world position into `[tile, extent - tile - 1e-4]`
/// per axis (the maximum computed at PC and stored as binary32; NaN kept).
pub fn clamp_position(map: &ZoneMap, pos: [u32; 2], p: Precision) -> Result<[u32; 2]> {
    if p == Precision::Double53
        && let Some(v) = clamp_position_pc53(map, pos)
    {
        return Ok(v);
    }
    clamp_position_exact(map, pos, p)
}

/// [`clamp_position`] in the Binary model.
pub fn clamp_position_exact(map: &ZoneMap, pos: [u32; 2], p: Precision) -> Result<[u32; 2]> {
    let g = &map.geometry;
    let tile = fpu::load(g.tile_size_bits)?;
    let margin = fpu::load(fpu::CLAMP_MARGIN_BITS)?;
    let mut out = pos;
    for axis in 0..2 {
        let hi = fpu::sub(&fpu::sub(&fpu::load(g.extent_bits[axis])?, &tile, p)?, &margin, p)?;
        let hi_bits = fpu::f32_store(&hi)?;
        let v = fpu::load(pos[axis])?;
        if fpu::less(&v, &tile) {
            out[axis] = g.tile_size_bits;
        } else if fpu::less(&fpu::load(hi_bits)?, &v) {
            out[axis] = hi_bits;
        }
    }
    Ok(out)
}

/// [`clamp_position`] at PC53 in binary64 (`None`: take the Binary model).
pub fn clamp_position_pc53(map: &ZoneMap, pos: [u32; 2]) -> Option<[u32; 2]> {
    use fpu::pc53;
    let g = &map.geometry;
    let tile = pc53::load(g.tile_size_bits)?;
    let margin = pc53::load(fpu::CLAMP_MARGIN_BITS)?;
    let mut out = pos;
    for axis in 0..2 {
        let hi_bits = pc53::store(pc53::sub(pc53::sub(pc53::load(g.extent_bits[axis])?, tile)?, margin)?)?;
        let v = pc53::load(pos[axis])?;
        if v < tile {
            out[axis] = g.tile_size_bits;
        } else if pc53::load(hi_bits)? < v {
            out[axis] = hi_bits;
        }
    }
    Some(out)
}

/// `0x515F40`: move a zone to a world position: clamp (`0x552640`), `_ftol`
/// each axis, arithmetic shift by the map shift, then [`move_to_tile`].
pub fn move_to_position<W: ZoneObjectWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    zone: &mut Zone,
    pos_bits: [u32; 2],
    radius_bits: u32,
    p: Precision,
) -> Result<()> {
    let (tx, ty) = position_tile(map, pos_bits, p)?;
    move_to_tile(map, w, zone, tx, ty, radius_bits, p)
}

/// The tile `0x515F40` moves a zone to: clamp, `_ftol`, shift.
pub fn position_tile(map: &ZoneMap, pos_bits: [u32; 2], p: Precision) -> Result<(i32, i32)> {
    let pos = clamp_position(map, pos_bits, p)?;
    let s = map.geometry.shift;
    let fast = (|| Some((fpu::pc53::ftol(fpu::pc53::load(pos[0])?)?, fpu::pc53::ftol(fpu::pc53::load(pos[1])?)?)))();
    // _ftol of a binary32 load does not depend on the precision control
    if let Some((x, y)) = fast {
        return Ok((x.wrapping_shr(s), y.wrapping_shr(s)));
    }
    let ty = fpu::ftol(&fpu::load(pos[1])?)?.wrapping_shr(s);
    let tx = fpu::ftol(&fpu::load(pos[0])?)?.wrapping_shr(s);
    Ok((tx, ty))
}

fn reveal_mask<W: ZoneWorld + ?Sized>(w: &mut W, actor: ActorKey) -> Result<u32> {
    w.reveal_mask(actor)?.ok_or(Error::NativeNull("actor+0xB8 detection component"))
}

/// Zone virtual `+0x14` (`0x515AD0`), called by `0x54FA10` after the zone was
/// added to tile `t` with `flags`: component effects on the tile's actors
/// (`0x54F7F0`, types 3/4/5/8), owner reveal (type 6) and supply (type 0),
/// reveal to the listed kingdoms (type 1).
pub fn on_tile_added<W: ZoneWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &Zone, t: usize, flags: u16) -> Result<()> {
    let owner = w.owner(zone.actor)?;
    let lo = flags as u8;
    if lo & 0x20 != 0 || lo & 0x18 != 0 || (flags >> 8) as u8 & 1 != 0 {
        tile_actor_effects(map, w, zone, t)?;
    }
    if owner != 0 && w.kingdom(owner)?.nonmajor == 0 {
        if lo & 0x40 != 0 {
            reveal(map, w, t, owner, true, true)?;
        }
        if lo & 1 != 0 {
            occupy(map, w, t, owner)?;
        }
    }
    if lo & 2 != 0 {
        let mut mask = reveal_mask(w, zone.actor)?;
        let mut i = 0;
        while i < KINGDOM_BITS && mask != 0 {
            if mask & (1 << i) != 0 {
                let k = w.kingdom_slot(i)?;
                reveal(map, w, t, k, false, true)?;
                mask &= !(1 << i);
            }
            i += 1;
        }
    }
    Ok(())
}

/// Zone virtual `+0x10` (`0x515BA0`), called by `0x54FA80` after the zone was
/// removed from tile `t`: hide for the owner (type 6), clear supply (type 0),
/// hide for the listed kingdoms (type 1). Uses the zone's own flags.
pub fn on_tile_removed<W: ZoneWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &Zone, t: usize) -> Result<()> {
    let owner = w.owner(zone.actor)?;
    let lo = zone.flags as u8;
    if owner != 0 && w.kingdom(owner)?.nonmajor == 0 {
        if lo & 0x40 != 0 {
            hide(map, w, t, owner)?;
        }
        if lo & 1 != 0 {
            unoccupy(map, w, t, owner)?;
        }
    }
    if lo & 2 != 0 {
        let mut mask = reveal_mask(w, zone.actor)?;
        let mut i = 0;
        while i < KINGDOM_BITS && mask != 0 {
            if mask & (1 << i) != 0 {
                let k = w.kingdom_slot(i)?;
                hide(map, w, t, k)?;
                mask &= !(1 << i);
            }
            i += 1;
        }
    }
    Ok(())
}

/// `0x54F7F0(zone)` on tile `t`: when enabled, for each actor of the tile
/// (virtual `+0xB4`, and multi-tile `0x4EE700` or standing on this tile
/// `0x4EA200`): `0x514DA0`, `0x514E00`, `0x514E20`.
pub fn tile_actor_effects<W: ZoneWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, zone: &Zone, t: usize) -> Result<()> {
    let _ = map;
    if !w.zone_effects_enabled()? {
        return Ok(());
    }
    for a in w.tile_actors(t)? {
        if !w.actor_b4(a)? {
            continue;
        }
        if !w.actor_multi_tile(a)? && w.actor_tile(a)? != t as i64 {
            continue;
        }
        enemy_entered(w, zone, a)?;
        type3_effect(w, zone, a)?;
        type8_effect(w, zone, a)?;
    }
    Ok(())
}

/// `0x514DA0(actor)`: an actor (`+8` bit 14 set, bit 0 clear) that is an enemy
/// of the zone owner (`actor.+0x140(zone actor.+0x108())`) triggers zone
/// virtual `+0x18` (`0x514E40`) on it or on its `+0x80` redirect target.
pub fn enemy_entered<W: ZoneWorld + ?Sized>(w: &mut W, zone: &Zone, actor: ActorKey) -> Result<()> {
    enemy_entered_fields(&mut EnemyZone { world: w, zone }, actor)
}

/// Borrowed fields of514DA0/514E40. Component handles refer to the same
/// allocation read at the native getter, without a second lookup.
pub trait EnemyEnteredFields {
    type Component;
    fn actor_flags(&mut self, actor: ActorKey) -> Result<u32>;
    fn zone_owner(&mut self) -> Result<u32>;
    fn enemy(&mut self, actor: ActorKey, owner: u32) -> Result<bool>;
    fn actor_redirect(&mut self, actor: ActorKey) -> Result<Option<ActorKey>>;
    fn zone_flags(&mut self) -> Result<u16>;
    fn component(&mut self, offset: u32) -> Result<Option<Self::Component>>;
    fn effect(&mut self, effect: ComponentEffect, component: Self::Component, target: ActorKey) -> Result<()>;
}

/// The same native guard and child order, independent of storage. A missing
/// zone is an error only when zone_owner is reached, after the actor flags.
pub fn enemy_entered_fields(w: &mut impl EnemyEnteredFields, actor: ActorKey) -> Result<()> {
    let f = w.actor_flags(actor)?;
    if f >> 14 & 1 == 0 || f & 1 != 0 {
        return Ok(());
    }
    let owner = w.zone_owner()?;
    if !w.enemy(actor, owner)? {
        return Ok(());
    }
    let target = w.actor_redirect(actor)?.unwrap_or(actor);
    let lo = w.zone_flags()? as u8;
    if lo & 0x20 != 0 {
        if let Some(c) = w.component(0x88)? { w.effect(ComponentEffect::Guard, c, target)?; }
    } else if lo & 0x10 != 0 {
        if let Some(c) = w.component(0x7c)? { w.effect(ComponentEffect::ControlOrganization, c, target)?; }
        if let Some(c) = w.component(0xb0)? { w.effect(ComponentEffect::ControlB0, c, target)?; }
    }
    Ok(())
}

struct EnemyZone<'a, W: ?Sized> { world: &'a mut W, zone: &'a Zone }
impl<W: ZoneWorld + ?Sized> EnemyEnteredFields for EnemyZone<'_, W> {
    type Component = u32;
    fn actor_flags(&mut self, a: ActorKey) -> Result<u32> { self.world.actor_flags(a) }
    fn zone_owner(&mut self) -> Result<u32> { self.world.owner(self.zone.actor) }
    fn enemy(&mut self, a: ActorKey, k: u32) -> Result<bool> { self.world.enemy(a, k) }
    fn actor_redirect(&mut self, a: ActorKey) -> Result<Option<ActorKey>> { self.world.actor_redirect(a) }
    fn zone_flags(&mut self) -> Result<u16> { Ok(self.zone.flags) }
    fn component(&mut self, offset: u32) -> Result<Option<u32>> {
        let c = self.world.component(self.zone.actor, offset)?;
        Ok((c != 0).then_some(c))
    }
    fn effect(&mut self, effect: ComponentEffect, component: u32, target: ActorKey) -> Result<()> {
        self.world.component_effect(effect, component, target)
    }
}

/// Zone virtual `+0x18` (`0x514E40`): type 5 -> guard component `0x568D80`;
/// else type 4 -> organization `0x57AB70` then component +0xB0 `0x567840`.
pub fn actor_effect<W: ZoneWorld + ?Sized>(w: &mut W, zone: &Zone, target: ActorKey) -> Result<()> {
    let lo = zone.flags as u8;
    if lo & 0x20 != 0 {
        let c = w.component(zone.actor, 0x88)?;
        if c != 0 {
            w.component_effect(ComponentEffect::Guard, c, target)?;
        }
    } else if lo & 0x10 != 0 {
        let c = w.component(zone.actor, 0x7c)?;
        if c != 0 {
            w.component_effect(ComponentEffect::ControlOrganization, c, target)?;
        }
        let c = w.component(zone.actor, 0xb0)?;
        if c != 0 {
            w.component_effect(ComponentEffect::ControlB0, c, target)?;
        }
    }
    Ok(())
}

/// `0x514E00(actor)`: type 3 -> zone actor +0x90 component `0x58CEA0`.
pub fn type3_effect<W: ZoneWorld + ?Sized>(w: &mut W, zone: &Zone, actor: ActorKey) -> Result<()> {
    if zone.flags as u8 & 8 != 0 {
        let c = w.component(zone.actor, 0x90)?;
        if c != 0 {
            w.component_effect(ComponentEffect::Type3, c, actor)?;
        }
    }
    Ok(())
}

/// `0x514E20(actor)`: type 8 -> zone actor +0xD4 component `0x564300`.
pub fn type8_effect<W: ZoneWorld + ?Sized>(w: &mut W, zone: &Zone, actor: ActorKey) -> Result<()> {
    if (zone.flags >> 8) as u8 & 1 != 0 {
        let c = w.component(zone.actor, 0xd4)?;
        if c != 0 {
            w.component_effect(ComponentEffect::Type8, c, actor)?;
        }
    }
    Ok(())
}

/// Services of the actor-entered effects (zone-set slot `+0x54`).
pub trait EnterWorld: ZoneWorld {
    /// `actor+0x148[index]` when `index < actor+0x14C` and non-null.
    fn zone_object(&mut self, actor: ActorKey, index: u32) -> Result<Option<Zone>>;
    /// Actor `+0xB8` (detection component) non-null.
    fn has_detection_component(&mut self, actor: ActorKey) -> Result<bool>;
    /// ActorData (`[actor+4]`) virtual `+0x24` (settlement marker / static).
    fn data_is_marker(&mut self, actor: ActorKey) -> Result<bool>;
    /// Actor virtual `+0x74(kingdom, deep)` (GActor `0x4D3C50`).
    fn virtual_explored(&mut self, actor: ActorKey, kingdom: u32, deep: bool) -> Result<bool>;
    /// GWorld+0x48 non-zero.
    fn world_flag_48(&mut self) -> Result<bool>;
    /// SVar `MarkerRevealPadding` (binary32; shipped Data/Game/SVars.tgi: 4).
    fn marker_reveal_padding_bits(&mut self) -> Result<u32>;
    /// Actor virtual `+0` (binary32 size).
    fn actor_size(&mut self, actor: ActorKey) -> Result<u32>;
    /// Tiles of the circle `0x46E080(radius)` at the actor (`0x432730`
    /// query, `0x554DD0`/`0x554330` iteration, row-major).
    fn discovery_tiles(&mut self, actor: ActorKey, radius_bits: u32) -> Result<Vec<usize>>;
    /// Actor `+8` flags write.
    fn set_actor_flags(&mut self, actor: ActorKey, flags: u32) -> Result<()>;
}

/// `0x54EF10`: an actor entered tile `t`: when zone effects are enabled
/// (`[B36550]+0xE4 != 3 || GWorld+0x48`) and the tile has zones, apply every
/// leaf's `+0x54` (union `0x514FC0` visits all leaves).
pub fn actor_entered_tile<W: EnterWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, t: usize, actor: ActorKey, p: Precision) -> Result<()> {
    if !w.zone_effects_enabled()? {
        return Ok(());
    }
    let Some(set) = map.tile(t)?.set else { return Ok(()) };
    for e in map.pool.entries(Some(set))? {
        leaf_actor_entered(map, w, e.actor, e.flags, actor, p)?;
    }
    Ok(())
}

fn zone_or_entered<W: EnterWorld + ?Sized>(w: &mut W, owner_actor: ActorKey, index: u32, actor: ActorKey) -> Result<()> {
    match w.zone_object(owner_actor, index)? {
        Some(z) => enemy_entered(w, &z, actor),
        None => {
            // 0x514DA0 with a null zone faults once it reads zone+4.
            let f = w.actor_flags(actor)?;
            if f >> 14 & 1 == 0 || f & 1 != 0 {
                Ok(())
            } else {
                Err(Error::NativeNull("zone slot (0x514DA0)"))
            }
        }
    }
}

/// Leaf `+0x54` (`0x515620`) for leaf `(zone_actor, flags)` and the entering
/// actor: guard (type 5) and control (type 4) effects through `0x514DA0`,
/// type 3 / type 8 component effects, and the settlement-marker discovery of
/// type 2: an entering actor with a detection component whose owner has not
/// seen the marker explores the tiles within size + MarkerRevealPadding of it
/// (`0x54F4F0(owner, 1, 0)`) and marks the marker seen (`+0x64(owner, 1, 0)`
/// with actor flag bit 30 cleared during the call).
pub fn leaf_actor_entered<W: EnterWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    zone_actor: ActorKey,
    flags: u16,
    actor: ActorKey,
    p: Precision,
) -> Result<()> {
    let lo = flags as u8;
    if lo & 0x20 != 0 {
        zone_or_entered(w, zone_actor, 5, actor)?;
    }
    if lo & 0x10 != 0 {
        zone_or_entered(w, zone_actor, 4, actor)?;
    }
    if lo & 8 != 0 {
        let z = w.zone_object(zone_actor, 3)?.ok_or(Error::NativeNull("zone slot 3"))?;
        if z.flags as u8 & 8 != 0 {
            let c = w.component(z.actor, 0x90)?;
            if c != 0 {
                w.component_effect(ComponentEffect::Type3, c, actor)?;
            }
        }
    }
    if flags & 0x100 != 0 {
        let z = w.zone_object(zone_actor, 8)?.ok_or(Error::NativeNull("zone slot 8"))?;
        if z.flags & 0x100 != 0 {
            let c = w.component(z.actor, 0xd4)?;
            if c != 0 {
                w.component_effect(ComponentEffect::Type8, c, actor)?;
            }
        }
    }
    if lo & 4 == 0 || !w.has_detection_component(actor)? {
        return Ok(());
    }
    let owner = w.owner(actor)?;
    if !w.data_is_marker(zone_actor)? || owner == 0 {
        return Ok(());
    }
    if w.virtual_explored(zone_actor, owner, true)? || !w.world_flag_48()? {
        return Ok(());
    }
    let pad = fpu::load(w.marker_reveal_padding_bits()?)?;
    let size = fpu::load(w.actor_size(zone_actor)?)?;
    let radius = fpu::f32_store(&fpu::add(&size, &pad, p)?)?;
    for t in w.discovery_tiles(zone_actor, radius)? {
        reveal(map, w, t, owner, true, false)?;
    }
    let f = w.actor_flags(zone_actor)?;
    w.set_actor_flags(zone_actor, f & 0xbfff_ffff)?;
    w.actor_reveal(zone_actor, owner, true, false)?;
    if f >> 30 & 1 != 0 {
        let now = w.actor_flags(zone_actor)?;
        w.set_actor_flags(zone_actor, now | 0x4000_0000)?;
    }
    Ok(())
}
