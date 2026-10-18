//! Actor detection state (`.\Source\Sim\GObject.cpp`): the per-actor fog
//! word at actor `+0x54` (same layout as the tile word: bit k = kingdom k has
//! seen the actor, bit k+16 = the actor is visible to kingdom k), its updates
//! from the tiles the actor stands on, the "who sees whom" queries and the
//! detection memory (per-kingdom ghost records of actors seen earlier).
//!
//! Tile -> actor callbacks (virtuals called by [`crate::tile`]):
//! - `+0x64` `0x4EC010` [`reveal`]: the tile became visible/explored for k;
//! - `+0x68` `0x4EC1B0` [`hide`]: the tile stopped being visible to k;
//! - `+0x6C` `0x4EA0C0` [`unexplore`]: fog restored over the tile for k.
//!
//! Queries: `0x4EA0D0` [`shared_explored`], `0x4EA1D0` [`shared_visible`],
//! `+0x70` `0x4D3B80` [`visible_to`], `+0x74` `0x4D3C50` [`explored_by`].
//! Memory: `0x4EBC70` [`refresh_memory`]. Bulk setters `0x4EBFB0`,
//! `0x4EBFD0`, `0x4EC270`.
use crate::fpu;
use crate::query::{KINGDOM_BITS, Kingdom};
use crate::set::ActorKey;
use crate::tile::{
    MapGeometry, ZoneMap, VISIBLE_SHIFT, ZoneWorld, explored, set_explored, set_visible,
    shared_explored as word_shared_explored, shared_visible as word_shared_visible, visible,
};
use crate::Result;
use aa_numeric::{Binary, Precision};

/// Actor reads, virtuals and seams of the detection routines.
pub trait ActorDetectionWorld: ZoneWorld {
    /// Actor `+0x54` detection word.
    fn detection_word(&mut self, actor: ActorKey) -> Result<u32>;
    fn set_detection_word(&mut self, actor: ActorKey, word: u32) -> Result<()>;
    /// Actor virtual `+0x100` (the carrier/container whose `+0x64` receives
    /// the reveal too; 0 = none).
    fn actor_container(&mut self, actor: ActorKey) -> Result<u32>;
    /// Actor virtual `+0x0` (binary32 size: radius used by `0x4EC1B0`).
    fn actor_size_bits(&mut self, actor: ActorKey) -> Result<u32>;
    /// The tiles of the actor's shape (`0x4EA0F0` descriptor iterated by the
    /// spatial grid `0x554DD0`/`0x554330`, row-major).
    fn shape_tiles(&mut self, actor: ActorKey) -> Result<Vec<usize>>;
    /// Tile `+0x14` fog word.
    fn tile_fog(&mut self, tile: usize) -> Result<u32>;
    /// The tiles the actor occupies (`0x4CF210` descriptor iterated by
    /// `0x554DD0`/`0x554330`, row-major), for `0x4EA150`.
    fn occupancy_tiles(&mut self, actor: ActorKey) -> Result<Vec<usize>>;
    /// Actor virtual `+0x8C` (keeps detection memory records).
    fn keeps_memory(&mut self, actor: ActorKey) -> Result<bool>;
    /// `0x4EB490` (memory eligibility).
    fn memory_eligible(&mut self, actor: ActorKey) -> Result<bool>;
    /// Actor `+0x5C[index]` (memory record pointer, 0 = none; no bound check
    /// in the original).
    fn memory_slot(&mut self, actor: ActorKey, index: u32) -> Result<u32>;
    /// Actor `+0x58` count of memory slots, read with every slot in order.
    fn memory_slots(&mut self, actor: ActorKey) -> Result<Vec<u32>>;
    /// Actor virtual `+0x148(kingdom)`: create a memory record.
    fn create_memory(&mut self, actor: ActorKey, kingdom: u32) -> Result<()>;
    /// Actor virtual `+0x14C(kingdom)`: delete the memory record.
    fn delete_memory(&mut self, actor: ActorKey, kingdom: u32) -> Result<()>;
    /// `0x4EBB80`: clear all memory (SAI notification per record).
    fn clear_memory(&mut self, actor: ActorKey) -> Result<()>;
    /// `[actor+4]+0x16D` (ActorData byte: announce first sighting).
    fn announces_first_sighting(&mut self, actor: ActorKey) -> Result<bool>;
    /// `0x4EC0EF..0x4EC187`: first sighting of an enemy by `kingdom`
    /// (GTID `0x432E40` on actor+0x50, message `0x42D490` type 9 with the
    /// actor name, SAI `0x79EA90`/`0x7AD9A0`).
    fn first_sighting(&mut self, actor: ActorKey, kingdom: u32) -> Result<()>;
    /// Actor `+0xC8` carrier (0 = none).
    fn carrier(&mut self, actor: ActorKey) -> Result<u32>;
    /// `0x5733A0(kingdom)` on the carrier.
    fn carrier_visible(&mut self, carrier: u32, kingdom: u32) -> Result<bool>;
    /// Actor `+0x7C` organization members (`0x4364C0` over org+0x28/+0x2C,
    /// nulls skipped), `None` without an organization.
    fn organization_members(&mut self, actor: ActorKey) -> Result<Option<Vec<ActorKey>>>;
    /// Actor `+0x80` component `+0x10` (0 when either is null).
    fn component_80_target(&mut self, actor: ActorKey) -> Result<u32>;
    /// Virtual `+0x70(kingdom, deep)` / `+0x74(kingdom, deep)` on another
    /// actor (GActor: [`visible_to`] / [`explored_by`]).
    fn virtual_seen(&mut self, actor: ActorKey, kingdom: u32, deep: bool, explored: bool) -> Result<bool>;
}

fn kingdom_of<W: ZoneWorld + ?Sized>(w: &mut W, key: u32) -> Result<Option<Kingdom>> {
    if key == 0 { Ok(None) } else { w.kingdom(key).map(Some) }
}

/// Actor virtual `+0x64` (`0x4EC010`): `explored` is the tile's explored bit,
/// `show` the visible flag of the reveal. Marks the actor visible (when
/// `show`) and seen (when the tile is explored), forwards a seen update to
/// the container, refreshes the memory and announces a first sighting.
pub fn reveal<W: ActorDetectionWorld + ?Sized>(
    w: &mut W,
    actor: ActorKey,
    kingdom: u32,
    tile_explored: bool,
    show: bool,
) -> Result<()> {
    let k = kingdom_of(w, kingdom)?;
    let mut word = w.detection_word(actor)?;
    let update = if show && !visible(word, k.as_ref())? {
        true
    } else if !tile_explored {
        false
    } else {
        !explored(word, k.as_ref())?
    };
    if !update {
        return Ok(());
    }
    let _ = visible(word, k.as_ref())?;
    let was_seen = explored(word, k.as_ref())?;
    if show {
        set_visible(&mut word, k.as_ref(), true)?;
        w.set_detection_word(actor, word)?;
    }
    if tile_explored {
        set_explored(&mut word, k.as_ref(), true)?;
        w.set_detection_word(actor, word)?;
        if w.actor_b4(actor)? {
            let c = w.actor_container(actor)?;
            if c != 0 {
                w.actor_reveal(c, kingdom, tile_explored, show)?;
            }
        }
    }
    refresh_memory(w, actor)?;
    if w.announces_first_sighting(actor)? && tile_explored && !was_seen {
        w.first_sighting(actor, kingdom)?;
    }
    Ok(())
}

/// Actor virtual `+0x68` (`0x4EC1B0`): a multi-tile actor (`ftol(tile) <
/// 2 * size`) stays visible while any of its tiles is visible to a
/// vision-sharing kingdom (or fog is off); otherwise clear visible and
/// refresh the memory.
pub fn hide<W: ActorDetectionWorld + ?Sized>(
    w: &mut W,
    geometry: &MapGeometry,
    actor: ActorKey,
    kingdom: u32,
    p: Precision,
) -> Result<()> {
    let t = fpu::ftol(&fpu::load(geometry.tile_size_bits)?)?;
    let size = fpu::load(w.actor_size_bits(actor)?)?;
    let twice = fpu::add(&size, &size, p)?;
    if fpu::less(&Binary::from_i32(t), &twice) {
        let k = kingdom_of(w, kingdom)?;
        for tile in w.shape_tiles(actor)? {
            if !w.fog_enabled()? || word_shared_visible(w.tile_fog(tile)?, k.as_ref()) {
                return Ok(());
            }
        }
    }
    let k = kingdom_of(w, kingdom)?;
    let mut word = w.detection_word(actor)?;
    set_visible(&mut word, k.as_ref(), false)?;
    w.set_detection_word(actor, word)?;
    refresh_memory(w, actor)
}

/// Actor virtual `+0x6C` (`0x4EA0C0`): clear the seen bit (no memory refresh).
pub fn unexplore<W: ActorDetectionWorld + ?Sized>(w: &mut W, actor: ActorKey, kingdom: u32) -> Result<()> {
    let k = kingdom_of(w, kingdom)?;
    let mut word = w.detection_word(actor)?;
    set_explored(&mut word, k.as_ref(), false)?;
    w.set_detection_word(actor, word)
}

/// `0x4EA0D0`: seen by a vision-sharing kingdom (null or non-major: true).
pub fn shared_explored<W: ActorDetectionWorld + ?Sized>(w: &mut W, actor: ActorKey, kingdom: u32) -> Result<bool> {
    let k = kingdom_of(w, kingdom)?;
    Ok(word_shared_explored(w.detection_word(actor)?, k.as_ref()))
}

/// `0x4EA1D0`: fog off, or visible to a vision-sharing kingdom.
pub fn shared_visible<W: ActorDetectionWorld + ?Sized>(w: &mut W, actor: ActorKey, kingdom: u32) -> Result<bool> {
    if !w.fog_enabled()? {
        return Ok(true);
    }
    let k = kingdom_of(w, kingdom)?;
    Ok(word_shared_visible(w.detection_word(actor)?, k.as_ref()))
}

/// Actor virtual `+0x70` (`0x4D3B80`, trigger `actor_visible_to_kingdom`):
/// a carried actor answers through its carrier (`0x5733A0`); else its own
/// word; with `deep`, any organization member (`+0x70(k, false)`) or, without
/// an organization, the `+0x80` component's target (`+0x70(k, true)`).
pub fn visible_to<W: ActorDetectionWorld + ?Sized>(w: &mut W, actor: ActorKey, kingdom: u32, deep: bool) -> Result<bool> {
    let c = w.carrier(actor)?;
    if c != 0 {
        return w.carrier_visible(c, kingdom);
    }
    if shared_visible(w, actor, kingdom)? {
        return Ok(true);
    }
    if !deep {
        return Ok(false);
    }
    match w.organization_members(actor)? {
        Some(members) => {
            for m in members {
                if w.virtual_seen(m, kingdom, false, false)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        None => {
            let t = w.component_80_target(actor)?;
            if t == 0 { Ok(false) } else { w.virtual_seen(t, kingdom, true, false) }
        }
    }
}

/// Actor virtual `+0x74` (`0x4D3C50`, trigger `actor_explored_to_kingdom`):
/// a carried actor is never reported; else its own word; with `deep`, any
/// organization member (`+0x74(k, false)`) or the `+0x80` target
/// (`+0x74(k, false)`).
pub fn explored_by<W: ActorDetectionWorld + ?Sized>(w: &mut W, actor: ActorKey, kingdom: u32, deep: bool) -> Result<bool> {
    if w.carrier(actor)? != 0 {
        return Ok(false);
    }
    if shared_explored(w, actor, kingdom)? {
        return Ok(true);
    }
    if !deep {
        return Ok(false);
    }
    match w.organization_members(actor)? {
        Some(members) => {
            for m in members {
                if w.virtual_seen(m, kingdom, false, true)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        None => {
            let t = w.component_80_target(actor)?;
            if t == 0 { Ok(false) } else { w.virtual_seen(t, kingdom, false, true) }
        }
    }
}

/// `0x4EBC70`: for actors that keep memory (virtual `+0x8C`) or are eligible
/// (`0x4EB490`): per listed kingdom, delete the record when the actor is no
/// longer "remembered" (seen by a vision-sharing kingdom but not visible to
/// one), create one when remembered and the actor is alive (`+8` bits 0 and
/// 7 clear); after a deletion with every slot empty, clear the memory.
pub fn refresh_memory<W: ActorDetectionWorld + ?Sized>(w: &mut W, actor: ActorKey) -> Result<()> {
    if !w.keeps_memory(actor)? && !w.memory_eligible(actor)? {
        return Ok(());
    }
    let mut deleted = false;
    for key in w.kingdom_list()? {
        if key == 0 {
            continue; // 0x4F0680 / 0x4F06B0 skip null entries
        }
        let k = kingdom_of(w, key)?;
        let has = match k {
            Some(kk) if kk.nonmajor == 0 && w.keeps_memory(actor)? => w.memory_slot(actor, kk.index)? != 0,
            _ => false,
        };
        let word = w.detection_word(actor)?;
        let remembered = !word_shared_visible(word, k.as_ref()) && word_shared_explored(word, k.as_ref());
        let flags = w.actor_flags(actor)?;
        let alive = flags & 1 == 0 && (flags >> 7) & 1 == 0;
        if has {
            if !remembered {
                w.delete_memory(actor, key)?;
                deleted = true;
            }
        } else if remembered && alive {
            w.create_memory(actor, key)?;
        }
    }
    if deleted && w.keeps_memory(actor)? && w.memory_slots(actor)?.iter().all(|&s| s == 0) {
        w.clear_memory(actor)?;
    }
    Ok(())
}

/// `0x4EA150`: replay the detection of every zone set under the actor onto it
/// (zone-set slot `+0x58` with the actor as visitor: type 6 -> `+0x64(owner,
/// true, true)` for a major owner, type 1 -> `+0x64(kingdom i or 0, false,
/// true)` per reveal-mask bit below 16).
pub fn refresh_from_tiles<W: ActorDetectionWorld + ?Sized>(map: &ZoneMap, w: &mut W, actor: ActorKey) -> Result<()> {
    for t in w.occupancy_tiles(actor)? {
        let Some(set) = map.tile(t)?.set else { continue };
        for e in map.pool.entries(Some(set))? {
            if e.flags & 0x40 != 0 {
                let owner = w.owner(e.actor)?;
                if owner != 0 && w.kingdom(owner)?.nonmajor == 0 {
                    w.actor_reveal(actor, owner, true, true)?;
                }
            }
            if e.flags & 2 != 0 {
                let mut mask = w
                    .reveal_mask(e.actor)?
                    .ok_or(crate::Error::NativeNull("actor+0xB8 detection component"))?;
                let mut i = 0;
                while i < KINGDOM_BITS && mask != 0 {
                    if mask & (1 << i) != 0 {
                        let k = w.kingdom_slot(i)?;
                        w.actor_reveal(actor, k, false, true)?;
                        mask &= !(1 << i);
                    }
                    i += 1;
                }
            }
        }
    }
    Ok(())
}

/// `0x4EC270(x, y)` (GObject tile change, from `0x4D6350`): clear every
/// visible bit, refresh the memory, then replay the zones under the actor.
pub fn tile_changed<W: ActorDetectionWorld + ?Sized>(map: &ZoneMap, w: &mut W, actor: ActorKey) -> Result<()> {
    let word = w.detection_word(actor)?;
    w.set_detection_word(actor, word & 0xffff)?;
    refresh_memory(w, actor)?;
    refresh_from_tiles(map, w, actor)
}

/// `0x4EBFB0`: seen by every kingdom, then refresh.
pub fn set_all_explored<W: ActorDetectionWorld + ?Sized>(w: &mut W, actor: ActorKey) -> Result<()> {
    let word = w.detection_word(actor)?;
    w.set_detection_word(actor, (word & 0xffff_0000) | 0xffff)?;
    refresh_memory(w, actor)
}

/// `0x4EBFD0`: visible to every kingdom, then refresh.
pub fn set_all_visible<W: ActorDetectionWorld + ?Sized>(w: &mut W, actor: ActorKey) -> Result<()> {
    let word = w.detection_word(actor)?;
    w.set_detection_word(actor, (word & 0xffff) | (0xffff << VISIBLE_SHIFT))?;
    refresh_memory(w, actor)
}
