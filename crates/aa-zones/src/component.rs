//! Detection component (actor `+0xB8`, vtable `0xA3C568`, updatable at +8
//! vtable `0xA3C540`, component type 0x16): per-actor detection ranges and the
//! two detection zones it drives.
//!
//! Layout: `{vtable, actor +4, updatable +8, reveal mask +0x10, ZOD radius
//! +0x14 (binary32)}`. The ZOD (zone type 6) radius is ActorData `+0x1CC`
//! (`detection_range`, through `0x45D790` and the modifier system
//! `0x501C30`); the RZOD (type 1) radius is ActorData `+0x1D0`
//! (`remote_detection_range`). An actor under construction (actor `+0x100`
//! bit 21, 0x200000; aa-production `construction.rs`) uses `+0x1D0` for its
//! type-6 zone.
//!
//! Also here: `KKC_GActor::CreateZone` `0x4D69A0` over the actor zone vector
//! (`actor+0x148`).
use crate::fpu;
use crate::set::ActorKey;
use crate::solver;
use crate::tile::ZoneMap;
use crate::zone::{self, Zone, ZONE_TYPES, ZoneObjectWorld, kept_on_dead_actor};
use crate::{Error, Result};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};

/// Actor `+0x148` zone vector (`{data, size +0x14C, capacity +0x150}`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorZones {
    pub slots: Vec<Option<Zone>>,
}

/// Detection component fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionComponent {
    pub actor: ActorKey,
    /// `+0x10`: kingdoms that see this actor's RZOD.
    pub reveal_mask: u32,
    /// `+0x14`: ZOD radius (binary32 bits).
    pub zod_radius_bits: u32,
}

/// Services of the detection component. The component itself lives in the
/// world (actor +0xB8): `ZoneActors::reveal_mask` must read the same
/// `reveal_mask` field (the original reads component +0x10 there).
pub trait DetectionComponentWorld: ZoneObjectWorld {
    /// The actor's detection component.
    fn detection_component(&mut self, actor: ActorKey) -> Result<&mut DetectionComponent>;
    /// The actor's zone vector.
    fn zones(&mut self, actor: ActorKey) -> Result<&mut ActorZones>;
    /// Native identity for a new zone object (`0x6F7810(0x18)`).
    fn new_zone_key(&mut self, actor: ActorKey, index: u32) -> Result<u32>;
    /// `0x404F60` "actor %d shouldn't have zone %d" (GActor.cpp 0x1555).
    fn log_unexpected_zone(&mut self, actor: ActorKey, index: u32) -> Result<()>;
    /// Actor `+0x20`/`+0x24` position (binary32 bits).
    fn actor_position(&mut self, actor: ActorKey) -> Result<[u32; 2]>;
    /// Actor virtual `+0x100` (non-zero when the actor is inside/attached).
    fn actor_virtual_100(&mut self, actor: ActorKey) -> Result<u32>;
    /// Actor virtual `+0x158`.
    fn actor_virtual_158(&mut self, actor: ActorKey) -> Result<bool>;
    /// Actor `+0x80` component: `(pointer, +0x10, byte +0x1C)`.
    fn actor_80(&mut self, actor: ActorKey) -> Result<(u32, u32, u8)>;
    /// ActorData (`[actor+4]`) word at `offset` (+0x1CC, +0x1D0, +0x430).
    fn data_word(&mut self, actor: ActorKey, offset: u32) -> Result<u32>;
    /// `0x45D790(context)` with a non-zero modifier context (`0x501C30`).
    fn modified_detection_range(&mut self, actor: ActorKey, context: u32) -> Result<u32>;
    /// `0x56EDC0(actor)`: kingdoms engaging the actor (lists +0x118, +0x130,
    /// component +0xB0 +0x24, denizen threats `0x569300`, and the +0x70
    /// object when SVar [B39AB0]+0x1C8 is set).
    fn reveal_sources(&mut self, actor: ActorKey) -> Result<u32>;
    /// Actor `+0x7C` organization member array (org +0x28, count +0x2C,
    /// nulls kept), `None` without an organization.
    fn organization_slots(&mut self, actor: ActorKey) -> Result<Option<Vec<ActorKey>>>;
    /// Kingdom virtual `+0x140(owner)`.
    fn kingdom_enemy(&mut self, kingdom: u32, owner: u32) -> Result<bool>;
    /// `0x4CF110`: kingdom `+0x78` (+0x1C) byte, `None` when `+0x78` is null.
    fn kingdom_reveal_byte(&mut self, kingdom: u32) -> Result<Option<u8>>;
    /// `0x50ACC0(delay, 0)` on the updatable (+8): schedule the tick.
    fn schedule_tick(&mut self, actor: ActorKey, delay_bits: u32) -> Result<()>;
}

/// `0x4D69A0` (KKC_GActor::CreateZone): size the vector to nine empty slots
/// on first use, then create the zone of `index` when its slot is empty
/// (logging when a dead actor gets a zone other than types 1/6).
pub fn create_zone<W: DetectionComponentWorld + ?Sized>(w: &mut W, actor: ActorKey, index: u32) -> Result<()> {
    if w.zones(actor)?.slots.is_empty() {
        w.zones(actor)?.slots = vec![None; ZONE_TYPES as usize];
    }
    let len = w.zones(actor)?.slots.len();
    if index as usize >= len {
        return Err(Error::World(format!("zone index {index} past the actor vector (native overrun)")));
    }
    if w.zones(actor)?.slots[index as usize].is_none() {
        let flags = 1u16.wrapping_shl(index);
        if w.actor_flags(actor)? & 1 != 0 && !kept_on_dead_actor(flags as u8) {
            w.log_unexpected_zone(actor, index)?;
        }
        let key = w.new_zone_key(actor, index)?;
        w.zones(actor)?.slots[index as usize] = Some(Zone::new(key, actor, flags));
    }
    Ok(())
}

fn slot<W: DetectionComponentWorld + ?Sized>(w: &mut W, actor: ActorKey, index: u32) -> Result<Option<Zone>> {
    Ok(w.zones(actor)?.slots.get(index as usize).copied().flatten())
}

fn with_zone<W: DetectionComponentWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    actor: ActorKey,
    index: u32,
    f: impl FnOnce(&mut ZoneMap, &mut W, &mut Zone) -> Result<()>,
) -> Result<()> {
    let mut z = slot(w, actor, index)?.ok_or(Error::NativeNull("actor zone slot"))?;
    let r = f(map, w, &mut z);
    w.zones(actor)?.slots[index as usize] = Some(z);
    r
}

/// `0x56EB40(index, enable, radius)`: enable -> create the zone if needed and
/// move it to the actor when inactive or the radius differs; disable ->
/// release an existing zone.
#[allow(clippy::too_many_arguments)]
pub fn enable_zone<W: DetectionComponentWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    actor: ActorKey,
    index: u32,
    enable: bool,
    radius_bits: u32,
    p: Precision,
) -> Result<()> {
    if !enable {
        if slot(w, actor, index)?.is_some() {
            with_zone(map, w, actor, index, |map, w, z| zone::release(map, w, z, p))?;
        }
        return Ok(());
    }
    if slot(w, actor, index)?.is_none() {
        create_zone(w, actor, index)?;
    }
    let z = slot(w, actor, index)?.ok_or(Error::NativeNull("zone after CreateZone"))?;
    if z.active() && fpu::equal_bits(z.radius_bits, radius_bits)? {
        return Ok(());
    }
    let pos = w.actor_position(actor)?;
    with_zone(map, w, actor, index, |map, w, z| zone::move_to_position(map, w, z, pos, radius_bits, p))
}

fn positive(bits: u32) -> Result<bool> {
    fpu::less_bits(0, bits)
}

/// Shared tail of `0x56EA30`/`0x56EC10`: an attached actor (virtual +0x100)
/// with a +0x94 component that fails virtual +0x158, or with a +0x90
/// component, is excluded; so is an actor whose +0x80 component has a target
/// with byte +0x1C set.
fn zone_allowed<W: DetectionComponentWorld + ?Sized>(w: &mut W, actor: ActorKey) -> Result<bool> {
    if w.actor_virtual_100(actor)? != 0 {
        if w.component(actor, 0x94)? != 0 && !w.actor_virtual_158(actor)? {
            return Ok(false);
        }
        if w.component(actor, 0x90)? != 0 {
            return Ok(false);
        }
    }
    let (c, target, byte) = w.actor_80(actor)?;
    Ok(c == 0 || target == 0 || byte == 0)
}

/// `0x56EA30`: the ZOD is enabled (radius > 0 and the actor allowed).
pub fn zod_enabled<W: DetectionComponentWorld + ?Sized>(w: &mut W, actor: ActorKey) -> Result<bool> {
    let r = w.detection_component(actor)?.zod_radius_bits;
    if !positive(r)? {
        return Ok(false);
    }
    zone_allowed(w, actor)
}

/// `0x56EC10`: the RZOD is enabled (reveal mask set, remote range > 0, actor
/// allowed).
pub fn rzod_enabled<W: DetectionComponentWorld + ?Sized>(w: &mut W, actor: ActorKey) -> Result<bool> {
    if w.detection_component(actor)?.reveal_mask == 0 {
        return Ok(false);
    }
    let r = w.data_word(actor, 0x1d0)?;
    if !positive(r)? {
        return Ok(false);
    }
    zone_allowed(w, actor)
}

/// `0x56EFC0`: recompute the reveal mask (own and flagged organization
/// members' engagers, plus every listed kingdom that is an enemy of the owner
/// and has the reveal byte's sign bit), minus the owner; on a change, update
/// the RZOD tiles (`0x5406D0` on GWorld+0x244 with the new mask and the
/// changed bits).
pub fn update_reveal_mask<W: DetectionComponentWorld + ?Sized>(
    map: &mut ZoneMap,
    w: &mut W,
    actor: ActorKey,
    p: Precision,
) -> Result<()> {
    if w.actor_flags(actor)? & 1 != 0 {
        return Ok(());
    }
    let mut mask = w.reveal_sources(actor)?;
    if let Some(members) = w.organization_slots(actor)? {
        for m in members {
            if m == 0 {
                continue;
            }
            let (comp, _, byte) = w.actor_80(m)?;
            if comp == 0 {
                return Err(Error::NativeNull("organization member +0x80"));
            }
            if byte != 0 {
                mask |= w.reveal_sources(m)?;
            }
        }
    }
    for i in 0..w.kingdom_count()? {
        let k = w.kingdom_at(i)?;
        if k == 0 {
            continue;
        }
        let owner = w.owner(actor)?;
        if w.kingdom_enemy(k, owner)?
            && let Some(b) = w.kingdom_reveal_byte(k)?
            && b & 0x80 != 0
        {
            mask |= 1u32.wrapping_shl(w.kingdom(k)?.index);
        }
    }
    let owner = w.owner(actor)?;
    if owner != 0 {
        let ko = w.kingdom(owner)?;
        if ko.nonmajor == 0 {
            mask &= !1u32.wrapping_shl(ko.index);
        }
    }
    let c = w.detection_component(actor)?;
    let old = c.reveal_mask;
    c.reveal_mask = mask;
    let changed = old ^ mask;
    let zones = w.zones(actor)?.slots.len();
    if zones > 1 && slot(w, actor, 1)?.is_some() && changed != 0 {
        with_zone(map, w, actor, 1, |map, w, z| solver::kingdom_mask_changed(map, w, z, mask, changed, p))?;
    }
    Ok(())
}

/// Updatable tick (`0x56F140`): nothing for a dead actor; an actor under
/// construction (`+0x100` bit 21, 0x200000) without `+0x138` keeps a type-6
/// zone of radius
/// ActorData+0x1D0 at its position; other actors update the ZOD
/// (`0x56EB40(6, 0x56EA30, +0x14)`), the reveal mask (`0x56EFC0`) and the
/// RZOD (`0x56EB40(1, 0x56EC10, ActorData+0x1D0)`).
pub fn tick<W: DetectionComponentWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, actor: ActorKey, p: Precision) -> Result<()> {
    if w.actor_flags(actor)? & 1 != 0 {
        return Ok(());
    }
    if w.component(actor, 0x100)? & 0x20_0000 != 0 {
        if w.component(actor, 0x138)? != 0 {
            return Ok(());
        }
        return structure_zod(map, w, actor, p);
    }
    let radius = w.detection_component(actor)?.zod_radius_bits;
    let on = zod_enabled(w, actor)?;
    enable_zone(map, w, actor, 6, on, radius, p)?;
    update_reveal_mask(map, w, actor, p)?;
    let r = w.data_word(actor, 0x1d0)?;
    let on = rzod_enabled(w, actor)?;
    enable_zone(map, w, actor, 1, on, r, p)
}

fn structure_zod<W: DetectionComponentWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, actor: ActorKey, p: Precision) -> Result<()> {
    let r = w.data_word(actor, 0x1d0)?;
    if slot(w, actor, 6)?.is_none() {
        create_zone(w, actor, 6)?;
    }
    let z = slot(w, actor, 6)?.ok_or(Error::NativeNull("zone 6 after CreateZone"))?;
    if z.active() && fpu::equal_bits(z.radius_bits, r)? {
        return Ok(());
    }
    let pos = w.actor_position(actor)?;
    with_zone(map, w, actor, 6, |map, w, z| zone::move_to_position(map, w, z, pos, r, p))
}

/// Component virtual `+0x14` (`0x56EC90`): structures as in [`tick`];
/// otherwise an actor with `+0x138` set schedules its tick (unless ActorData
/// +0x430) when it has a +0x80 component, and the rest enable/disable the ZOD
/// (dead actors or radius <= 0 disable).
pub fn activate<W: DetectionComponentWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, actor: ActorKey, p: Precision) -> Result<()> {
    let flagged = w.component(actor, 0x100)? & 0x20_0000 != 0;
    let w138 = w.component(actor, 0x138)?;
    if flagged {
        if w138 != 0 {
            return Ok(());
        }
        return structure_zod(map, w, actor, p);
    }
    if w138 != 0 && w.data_word(actor, 0x430)? == 0 {
        if w.actor_80(actor)?.0 == 0 {
            return Ok(());
        }
        return w.schedule_tick(actor, fpu::CLAMP_MARGIN_BITS);
    }
    let radius = w.detection_component(actor)?.zod_radius_bits;
    let enable = w.actor_flags(actor)? & 1 == 0 && positive(radius)?;
    enable_zone(map, w, actor, 6, enable, radius, p)
}

/// `0x56EA10(context)`: ZOD radius from ActorData+0x1CC (`0x45D790`: raw
/// without a context, else through the modifier system).
pub fn set_zod_radius<W: DetectionComponentWorld + ?Sized>(w: &mut W, actor: ActorKey, context: u32) -> Result<()> {
    let r = if context == 0 { w.data_word(actor, 0x1cc)? } else { w.modified_detection_range(actor, context)? };
    w.detection_component(actor)?.zod_radius_bits = r;
    Ok(())
}

/// Zone part of the GActor tile change `0x4D6350` (after `0x4EC270`,
/// `0x4D4130`, the organization `0x579BA0` and the SAI region hook): every
/// active zone of the actor moves to the actor's position with its current
/// radius (`0x515F40`, slot order).
pub fn move_actor_zones<W: DetectionComponentWorld + ?Sized>(map: &mut ZoneMap, w: &mut W, actor: ActorKey, p: Precision) -> Result<()> {
    let n = w.zones(actor)?.slots.len();
    for i in 0..n {
        let Some(z) = w.zones(actor)?.slots[i] else { continue };
        if !z.active() {
            continue;
        }
        let pos = w.actor_position(actor)?;
        let r = z.radius_bits;
        with_zone(map, w, actor, i as u32, |map, w, z| zone::move_to_position(map, w, z, pos, r, p))?;
    }
    Ok(())
}
