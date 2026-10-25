//! Zone-set virtual queries (leaf `0xA323E8` / union `0xA32388` slots) and the
//! GMap tile wrappers that the AI and other systems call.
//!
//! A union slot iterates the set in native order (head first) and returns the
//! first leaf result that is non-zero; the leaf bodies below are per entry.
//! Masks are tested on their low 16 bits (`test ax, ax` / `test cx, cx`); the
//! callers pass `1 << zone_type` (zone types 0..=8: 0 ZOS supply, 1 RZOD
//! reveal-to-listed-kingdoms, 2 discovery, 4 ZOC control, 5 ZOG guard,
//! 6 ZOD detection; 3, 7, 8 are component zones, see `zone`).
//!
//! | slot | leaf | union | port |
//! |---|---|---|---|
//! | +0x08 | `0x516190` actor == x | `0x969B10` false | (internal to add/remove) |
//! | +0x10 | `0x4F0500` 1 | `0x5160B0` +0x1C | [`ZonePool::len`] |
//! | +0x14 | `0x514BF0` | `0x515420` | [`any_owned_by`] |
//! | +0x18, +0x34 | `0x514A70` | `0x515040`, `0x515290` | [`contains`] |
//! | +0x1C | `0x5149F0` | `0x5150F0` | [`first_zone`] |
//! | +0x20 | `0x514AA0` | `0x515150` | [`first_enemy_zone`] |
//! | +0x24 | `0x514B10` | `0x5151A0` | [`first_owned_zone`] |
//! | +0x28 | `0x514B80` | `0x5151F0` | [`enemy_discovery_with_b0`] |
//! | +0x2C | `0x516190` | `0x515240` | [`has_actor`] |
//! | +0x30 | `0x4E5240` identity | `0x5152F0` | [`contains_leaf`] |
//! | +0x38 | `0x514BB0` | `0x515340` | [`any_allied`] |
//! | +0x3C | `0x515EC0` | `0x5153B0` | [`collect_allied`] |
//! | +0x40 | `0x5155C0` | `0x515490` | [`detects`] |
//! | +0x44 | `0x4E5240` identity | `0x5150A0` | [`contains_part`] |
//! | +0x58 | `0x5158A0` | `0x515000` | [`visit`] |
//! | +0x5C | `0x514C30` | `0x515580` | [`notify_type`] |
use crate::set::{ActorKey, Entry, SetRef, ZonePool};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// Native kingdom fields read by the zone and fog code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kingdom {
    /// Native identity (pointer) passed to actor virtuals and compared with
    /// actor virtual `+0x108`.
    pub key: u32,
    /// `+0x18C`: bit index in fog/supply words (shift count, taken mod 32).
    pub index: u32,
    /// `+0x190`: non-zero for kingdoms outside the fog system (always "true"
    /// for visibility, never marked).
    pub nonmajor: u32,
    /// `+0x228`: shared-vision mask in fog-word layout (low 16 explored,
    /// high 16 visible).
    pub shared_mask: u32,
    /// `+0x22C`: supply mask over the tile supply word.
    pub supply_mask: u16,
}

/// Actor reads and virtual calls made by the zone-set queries. Every method is
/// a native read or call at the point the original makes it.
pub trait ZoneActors {
    /// Actor virtual `+0x108`: owning kingdom key (0 for null).
    fn owner(&mut self, actor: ActorKey) -> Result<u32>;
    /// Actor virtual `+0x138(kingdom)`.
    fn allied(&mut self, actor: ActorKey, kingdom: u32) -> Result<bool>;
    /// Actor virtual `+0x140(kingdom)`.
    fn enemy(&mut self, actor: ActorKey, kingdom: u32) -> Result<bool>;
    /// `actor+0x148[index]` when `index < actor+0x14C`, else 0 (the zone
    /// object pointer; 0 = none).
    fn zone_slot(&mut self, actor: ActorKey, index: u32) -> Result<u32>;
    /// `actor+4` (compared with the caller's exclusion word in `0x5149F0`).
    fn word4(&mut self, actor: ActorKey) -> Result<u32>;
    /// `actor+0xB0` (component pointer, tested for non-zero).
    fn component_b0(&mut self, actor: ActorKey) -> Result<u32>;
    /// `[actor+0xB8]+0x10`: reveal mask of the detection component; `None`
    /// when `actor+0xB8` is null (the original then dereferences null).
    fn reveal_mask(&mut self, actor: ActorKey) -> Result<Option<u32>>;
}

/// `0xA11740`: the number of kingdom bits in fog/supply/reveal words (16).
/// This is a data-layout constant of the 32-bit fog word (16 explored + 16
/// visible bits) and of saved games, not an engine buffer cap.
pub const KINGDOM_BITS: u32 = 16;

fn masked(flags: u16, mask: u16) -> bool {
    flags & mask != 0
}

/// Leaf `0x5149F0`/`0x514AA0`/`0x514B10` tail: first set bit below 9 of the
/// mask selects `actor+0x148[bit]` (0 past `+0x14C` or without a bit < 9).
fn zone_of<W: ZoneActors + ?Sized>(w: &mut W, actor: ActorKey, mask: u16) -> Result<u32> {
    for i in 0..9u32 {
        if mask & (1 << i) != 0 {
            return w.zone_slot(actor, i);
        }
    }
    Ok(0)
}

fn first<R, F>(pool: &ZonePool, set: SetRef, mut f: F) -> Result<Option<R>>
where
    F: FnMut(Entry) -> Result<Option<R>>,
{
    for e in pool.iter(Some(set))? {
        if let Some(r) = f(e)? {
            return Ok(Some(r));
        }
    }
    Ok(None)
}

fn any<F>(pool: &ZonePool, set: SetRef, mut f: F) -> Result<bool>
where
    F: FnMut(Entry) -> Result<bool>,
{
    Ok(first(pool, set, |e| Ok(f(e)?.then_some(())))?.is_some())
}

/// Slot `+0x14`: an entry with `flags & mask`, `actor != exclude` and
/// `owner(actor) == kingdom` (owner read only after the first two tests).
pub fn any_owned_by<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    kingdom: u32,
    mask: u16,
    exclude: ActorKey,
) -> Result<bool> {
    any(pool, set, |e| {
        Ok(masked(e.flags, mask) && e.actor != exclude && w.owner(e.actor)? == kingdom)
    })
}

/// Slots `+0x18`/`+0x34`: an entry of `actor` with `flags & mask`.
pub fn contains(pool: &ZonePool, set: SetRef, actor: ActorKey, mask: u16) -> Result<bool> {
    any(pool, set, |e| Ok(e.actor == actor && masked(e.flags, mask)))
}

/// Slot `+0x1C`: first zone object of an entry with `flags & mask`, whose
/// actor is not `exclude_actor` (0 disables) and whose `actor+4` is not
/// `exclude_word4` (0 disables). Returns the zone pointer (0 = none).
pub fn first_zone<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    mask: u16,
    exclude_actor: ActorKey,
    exclude_word4: u32,
) -> Result<u32> {
    Ok(first(pool, set, |e| {
        if mask == 0 || !masked(e.flags, mask) || (exclude_actor != 0 && e.actor == exclude_actor) {
            return Ok(None);
        }
        if exclude_word4 != 0 && w.word4(e.actor)? == exclude_word4 {
            return Ok(None);
        }
        let z = zone_of(w, e.actor, mask)?;
        Ok((z != 0).then_some(z))
    })?
    .unwrap_or(0))
}

/// Slot `+0x20`: first zone of an entry with `flags & mask` whose actor is an
/// enemy of `kingdom` (actor virtual `+0x140`).
pub fn first_enemy_zone<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    kingdom: u32,
    mask: u16,
) -> Result<u32> {
    Ok(first(pool, set, |e| {
        if mask == 0 || !masked(e.flags, mask) || !w.enemy(e.actor, kingdom)? {
            return Ok(None);
        }
        let z = zone_of(w, e.actor, mask)?;
        Ok((z != 0).then_some(z))
    })?
    .unwrap_or(0))
}

/// Slot `+0x24`: first zone of an entry with `flags & mask` owned by `kingdom`.
pub fn first_owned_zone<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    kingdom: u32,
    mask: u16,
) -> Result<u32> {
    Ok(first(pool, set, |e| {
        if mask == 0 || !masked(e.flags, mask) || w.owner(e.actor)? != kingdom {
            return Ok(None);
        }
        let z = zone_of(w, e.actor, mask)?;
        Ok((z != 0).then_some(z))
    })?
    .unwrap_or(0))
}

/// Slot `+0x28` (leaf `0x514B80`): an entry whose actor has `+0xB0` and whose
/// leaf answers `+0x20(kingdom, 4)` (enemy discovery zone, type 2).
pub fn enemy_discovery_with_b0<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    kingdom: u32,
) -> Result<bool> {
    any(pool, set, |e| {
        if w.component_b0(e.actor)? == 0 {
            return Ok(false);
        }
        Ok(first_enemy_zone(pool, SetRef::Leaf(e), w, kingdom, 4)? != 0)
    })
}

/// Slot `+0x2C` (and leaf `+0x08`): an entry of `actor`.
pub fn has_actor(pool: &ZonePool, set: SetRef, actor: ActorKey) -> Result<bool> {
    any(pool, set, |e| Ok(e.actor == actor))
}

/// Slot `+0x30`: `x` is one of the set's leaves (object identity).
pub fn contains_leaf(pool: &ZonePool, set: SetRef, x: SetRef) -> Result<bool> {
    any(pool, set, |e| Ok(SetRef::Leaf(e) == x))
}

/// Slot `+0x44` (union `0x5150A0`): `x` is the set itself, any union suffix
/// on the tail chain, or any of its leaves.
pub fn contains_part(pool: &ZonePool, set: SetRef, x: SetRef) -> Result<bool> {
    let mut cur = set;
    loop {
        if cur == x {
            return Ok(true);
        }
        match cur {
            SetRef::Leaf(_) => return Ok(false),
            SetRef::Node(_) => {
                let entries = pool.entries(Some(cur))?;
                if SetRef::Leaf(entries[0]) == x {
                    return Ok(true);
                }
                cur = pool.tail(cur)?;
            }
        }
    }
}

/// Slot `+0x38` (leaf `0x514BB0`): an entry with `flags & mask`,
/// `actor != exclude` and `allied(actor, kingdom)`.
pub fn any_allied<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    kingdom: u32,
    mask: u16,
    exclude: ActorKey,
) -> Result<bool> {
    any(pool, set, |e| {
        Ok(masked(e.flags, mask) && e.actor != exclude && w.allied(e.actor, kingdom)?)
    })
}

/// Caller-owned actor list `{head, tail}` of `{actor, next}` nodes, kept in
/// head-to-tail order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorList {
    pub items: Vec<ActorKey>,
}

/// Slot `+0x3C`. Leaf `0x515EC0`: when the leaf answers `+0x38`, prepend its
/// actor unless already listed and return true; else false. Union `0x5153B0`
/// calls every leaf and returns whether the list is still EMPTY (`[list] ==
/// 0`), the opposite sense of the leaf result; both are ported as found.
pub fn collect_allied<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    list: &mut ActorList,
    kingdom: u32,
    mask: u16,
    exclude: ActorKey,
) -> Result<bool> {
    let mut leaf_result = false;
    for e in pool.iter(Some(set))? {
        leaf_result = false;
        if masked(e.flags, mask) && e.actor != exclude && w.allied(e.actor, kingdom)? {
            if !list.items.contains(&e.actor) {
                list.items.insert(0, e.actor);
            }
            leaf_result = true;
        }
    }
    Ok(match set {
        SetRef::Leaf(_) => leaf_result,
        SetRef::Node(_) => list.items.is_empty(),
    })
}

/// Slot `+0x40` (leaf `0x5155C0`): the set gives `kingdom` sight of the tile:
/// a type-6 (ZOD, `0x40`) entry owned by it, or a type-1 (RZOD, `0x2`) entry
/// whose actor's reveal mask has the kingdom's bit. Non-major kingdoms
/// (`+0x190 != 0`) never match.
pub fn detects<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    kingdom: &Kingdom,
) -> Result<bool> {
    any(pool, set, |e| {
        if kingdom.nonmajor != 0 {
            return Ok(false);
        }
        if e.flags & 0x40 != 0 && w.owner(e.actor)? == kingdom.key {
            return Ok(true);
        }
        if e.flags & 2 != 0 {
            let mask = w.reveal_mask(e.actor)?.ok_or(Error::NativeNull("actor+0xB8 detection component"))?;
            return Ok(mask & 1u32.wrapping_shl(kingdom.index) != 0);
        }
        Ok(false)
    })
}

/// Detection visitor of slot `+0x58`: actor virtual `+0x64(kingdom, explored,
/// visible)` (the same call `0x54F4F0` makes for actors on a tile).
pub trait DetectionVisitor {
    fn reveal(&mut self, kingdom: u32, explored: bool, visible: bool) -> Result<()>;
}

/// World reads of slot `+0x58` beyond [`ZoneActors`].
pub trait VisitWorld: ZoneActors {
    /// Kingdom fields for an owner key (non-null).
    fn kingdom(&mut self, key: u32) -> Result<Kingdom>;
    /// `GWorld+0x154[i]` when `i < GWorld+0x158`, else 0.
    fn kingdom_slot(&mut self, index: u32) -> Result<u32>;
}

/// Slot `+0x58` (leaf `0x5158A0`, union `0x515000` calls every leaf): replay
/// the set's detection onto a visitor. Type 6: owner non-null and major ->
/// `reveal(owner, true, true)`. Type 1: for each set bit `i < 16` of the
/// reveal mask (lowest first) -> `reveal(GWorld kingdom i or 0, false, true)`.
pub fn visit<W: VisitWorld + ?Sized, V: DetectionVisitor + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    v: &mut V,
) -> Result<()> {
    for e in pool.iter(Some(set))? {
        if e.flags & 0x40 != 0 {
            let owner = w.owner(e.actor)?;
            if owner != 0 && w.kingdom(owner)?.nonmajor == 0 {
                v.reveal(owner, true, true)?;
            }
        }
        if e.flags & 2 != 0 {
            let mut mask = w.reveal_mask(e.actor)?.ok_or(Error::NativeNull("actor+0xB8 detection component"))?;
            let mut i = 0;
            while i < KINGDOM_BITS && mask != 0 {
                if mask & (1 << i) != 0 {
                    let k = w.kingdom_slot(i)?;
                    v.reveal(k, false, true)?;
                    mask &= !(1 << i);
                }
                i += 1;
            }
        }
    }
    Ok(())
}

/// Slot `+0x5C` (leaf `0x514C30`, union `0x515580` calls every leaf): for an
/// entry with bit `zone_type`, tail-call `0x4F3D90(GWorld, actor+0x148[type]
/// or 0)`; `notify` receives that zone pointer.
pub fn notify_type<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: SetRef,
    w: &mut W,
    zone_type: u32,
    notify: &mut dyn FnMut(u32) -> Result<()>,
) -> Result<()> {
    for e in pool.iter(Some(set))? {
        if u32::from(e.flags) & 1u32.wrapping_shl(zone_type) & 0xffff != 0 {
            let z = if (zone_type as i32) < 0 { 0 } else { w.zone_slot(e.actor, zone_type)? };
            notify(z)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// GMap tile wrappers (`this` = tile; tile+4 is the zone set).

/// `0x54EF40(zone)`: with no zone, whether the tile has a set; else whether
/// the set contains the zone's actor with any of its flags (`0x514940` ->
/// slot `+0x18(zone.actor, zone.flags)`).
pub fn tile_has_zone(pool: &ZonePool, set: Option<SetRef>, zone: Option<(ActorKey, u16)>) -> Result<bool> {
    match (set, zone) {
        (s, None) => Ok(s.is_some()),
        (None, Some(_)) => Ok(false),
        (Some(s), Some((actor, flags))) => contains(pool, s, actor, flags),
    }
}

/// `1 << zone_type` truncated to 16 bits (the wrappers store `si` as a word).
pub fn type_mask(zone_type: u32) -> u16 {
    1u32.wrapping_shl(zone_type) as u16
}

/// `0x54EF80(type, exclude_actor, exclude_word4)`: slot `+0x1C`; 0 on an
/// empty tile.
pub fn tile_first_zone<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: Option<SetRef>,
    w: &mut W,
    zone_type: u32,
    exclude_actor: ActorKey,
    exclude_word4: u32,
) -> Result<u32> {
    match set {
        None => Ok(0),
        Some(s) => first_zone(pool, s, w, type_mask(zone_type), exclude_actor, exclude_word4),
    }
}

/// `0x54EFC0(kingdom, type)`: enemy zone of a type (AI: type 4 ZOC, type 5
/// ZOG); 0 on an empty tile.
pub fn tile_enemy_zone<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: Option<SetRef>,
    w: &mut W,
    kingdom: u32,
    zone_type: u32,
) -> Result<u32> {
    match set {
        None => Ok(0),
        Some(s) => first_enemy_zone(pool, s, w, kingdom, type_mask(zone_type)),
    }
}

/// `0x54EFF0(kingdom, type)`: own zone of a type; 0 on an empty tile.
pub fn tile_owned_zone<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: Option<SetRef>,
    w: &mut W,
    kingdom: u32,
    zone_type: u32,
) -> Result<u32> {
    match set {
        None => Ok(0),
        Some(s) => first_owned_zone(pool, s, w, kingdom, type_mask(zone_type)),
    }
}

/// `0x54F020(kingdom)`: slot `+0x28`; false on an empty tile.
pub fn tile_enemy_discovery<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: Option<SetRef>,
    w: &mut W,
    kingdom: u32,
) -> Result<bool> {
    match set {
        None => Ok(false),
        Some(s) => enemy_discovery_with_b0(pool, s, w, kingdom),
    }
}

/// `0x559530`: supply word bit of a major kingdom (null or non-major: false).
pub fn supply_bit(word: u16, kingdom: Option<&Kingdom>) -> bool {
    match kingdom {
        Some(k) if k.nonmajor == 0 => word & (1u32.wrapping_shl(k.index) as u16) != 0,
        _ => false,
    }
}

/// `0x5595D0`: supply word against the kingdom's `+0x22C` mask (null or
/// non-major: false).
pub fn supply_shared(word: u16, kingdom: Option<&Kingdom>) -> bool {
    match kingdom {
        Some(k) if k.nonmajor == 0 => word & k.supply_mask != 0,
        _ => false,
    }
}

/// `0x54F870(kingdom, exclude)`: `exclude == 0` -> own supply bit (`0x559530`
/// on tile+0x18); else slot `+0x14(kingdom, 1, exclude)` (false on an empty
/// tile): another of the kingdom's supply zones covers the tile.
pub fn tile_own_supply<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: Option<SetRef>,
    supply_word: u16,
    w: &mut W,
    kingdom: Option<&Kingdom>,
    exclude: ActorKey,
) -> Result<bool> {
    if exclude == 0 {
        return Ok(supply_bit(supply_word, kingdom));
    }
    match set {
        None => Ok(false),
        Some(s) => any_owned_by(pool, s, w, kingdom.map_or(0, |k| k.key), 1, exclude),
    }
}

/// `0x54F8B0(kingdom, exclude)` (AI friendly ZOS): `exclude == 0` -> supply
/// word against the kingdom's shared supply mask (`0x5595D0`); else slot
/// `+0x38(kingdom, 1, exclude)` (allied supply zone of another actor).
pub fn tile_friendly_supply<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: Option<SetRef>,
    supply_word: u16,
    w: &mut W,
    kingdom: Option<&Kingdom>,
    exclude: ActorKey,
) -> Result<bool> {
    if exclude == 0 {
        return Ok(supply_shared(supply_word, kingdom));
    }
    match set {
        None => Ok(false),
        Some(s) => any_allied(pool, s, w, kingdom.map_or(0, |k| k.key), 1, exclude),
    }
}

/// `0x54F8F0(list, kingdom, type, exclude)`: slot `+0x3C`; false on an empty
/// tile.
#[allow(clippy::too_many_arguments)]
pub fn tile_collect_allied<W: ZoneActors + ?Sized>(
    pool: &ZonePool,
    set: Option<SetRef>,
    w: &mut W,
    list: &mut ActorList,
    kingdom: u32,
    zone_type: u32,
    exclude: ActorKey,
) -> Result<bool> {
    match set {
        None => Ok(false),
        Some(s) => collect_allied(pool, s, w, list, kingdom, type_mask(zone_type), exclude),
    }
}
