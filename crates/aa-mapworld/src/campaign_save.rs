//! A saved World War II campaign of the original, read whole: the file the
//! MetaGameSaveDialog writes (file dialog mode 3, `0x68DCE0(3, ..)`, AVar
//! PathDynamicCampaignSavedGames, extension RSG) through the map save 4F2DB0, and
//! that the map load 4FC600 reads back.
//!
//! The file is an ordinary saved map (WorldRuleMode 5, `savegame_metagame_map` of
//! 4981A0, a saved-game mode: [`crate::world::saved_game_mode`]) with one chunk
//! more, `METAGAME`, between KINGDOMS and USED_REQUIRED_OBJECTS: the KKC_MGState
//! payload of 938EE0 / 93A3B0 (its reader is `aa_metagame::save`). The armies are
//! not in that payload. Each army is an actor of DYNAMIC_THINGS (4FC340,
//! GActor::Load 4D8CF0) whose MetaGame component (`MetaGameComponent`, slot 27 of
//! `aa_rules::native::COMPONENTS`) holds the army's own record: Save 93EEA0, Load 940740
//! (`aa_metagame::save::ArmyComponentImage`). The payload names armies by the
//! record's serialized handle.
//!
//! This module takes the file apart in the original's byte order and keeps the two
//! payloads as bytes for the metagame reader, which sits above this crate:
//! [`CampaignSave::metagame`] and each [`SavedArmy::component`]. With a Mind decoder
//! every army record is walked to its end (the index entry's length), so a
//! component the walk cannot read is an error, never skipped.
use crate::actor_payload::{ActorTail, PropertiesPayload, UpdatablePayload};
use crate::inventory::{ByteSpan, Reader};
use crate::saved_components::SavedComponent;
use crate::saved_world::MindDecoder;
use crate::{Error, MapArchive, Result, WorldInventory};

/// The chunk 4F2DB0 opens for the MGState (859150 "METAGAME").
pub const METAGAME_CHUNK: &str = "METAGAME";
/// The MetaGame component's slot in the actor's component table.
pub const METAGAME_SLOT: u8 = 27;
/// WorldRuleMode `savegame_metagame_map` (4981A0): the mode of a saved campaign
/// (a battle fought from it is saved in mode 4, `savegame_metagame_rts`).
pub const MODE_SAVED_CAMPAIGN: u32 = 5;

/// One army: an actor record of DYNAMIC_THINGS with a saved MetaGame component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedArmy {
    /// The record's position in DYNAMIC_THINGS_INDEX.
    pub record_ordinal: usize,
    /// The Thing's IDS (the army type).
    pub thing_id: String,
    /// The record's serialized handle: what the METAGAME payload names.
    pub serialized_id: u32,
    /// GActor +owner handle (the kingdom actor's).
    pub owner_handle: u32,
    /// The component's bytes as 93EEA0 wrote them.
    pub component: Vec<u8>,
    /// Where they lie in the DYNAMIC_THINGS chunk.
    pub component_span: ByteSpan,
    /// The record was walked to the end of its index entry (the stages after the
    /// MetaGame component, the Mind last, and the actor's trailer). False when no
    /// Mind decoder was given: the component's bytes are complete either way, as
    /// its stage comes before the slots 28..30 and the Mind.
    pub walked_to_end: bool,
}

/// What the file holds of a campaign.
#[derive(Clone, Debug)]
pub struct CampaignSave {
    /// The rules' mode of MAP_HEADER ([`MODE_SAVED_CAMPAIGN`] for a saved campaign).
    pub mode: u32,
    /// The METAGAME chunk.
    pub metagame: Vec<u8>,
    /// The armies in DYNAMIC_THINGS order.
    pub armies: Vec<SavedArmy>,
    /// The map world the rest of the file restores.
    pub world: WorldInventory,
}

/// The length of a MetaGame component record, walked as 940740 reads it: version
/// (8589B0, at most 1; 0 reads the same fields), battles fought, the territory's
/// index, the path and the waypoints (a signed count, then indices), the
/// divisions (a signed count, then a UTF-16 IDS each, 41D290), then the
/// component's Updatable (50BCD0).
pub fn component_length(bytes: &[u8]) -> Result<usize> {
    let mut r = Reader::new(bytes);
    let version = r.u32()? as i32;
    if !(0..=1).contains(&version) {
        return Err(Error::Format(format!("MetaGame component version {version} (native max 1)")));
    }
    r.u32()?; // +0x10 battles fought
    r.u32()?; // +0x20 territory index (9389F0)
    for _ in 0..2 {
        let count = (r.u32()? as i32).max(0);
        for _ in 0..count {
            r.u32()?;
        }
    }
    let count = (r.u32()? as i32).max(0);
    for _ in 0..count {
        r.string()?;
    }
    let (_, n) = UpdatablePayload::decode_prefix(&bytes[r.at..])?;
    Ok(r.at + n)
}

/// 4FC600's chunks of a saved campaign. `minds` decodes each army's Mind (slot 4,
/// the last stage), as for any saved actor; without it each army record is walked
/// up to the end of its MetaGame component only ([`SavedArmy::walked_to_end`]).
pub fn read(bytes: Vec<u8>, max_chunk_bytes: u64, mut minds: Option<&mut dyn MindDecoder>) -> Result<CampaignSave> {
    let map = MapArchive::from_bytes(bytes)?;
    let world = WorldInventory::decode(&map, max_chunk_bytes)?;
    let mode = world
        .header
        .as_ref()
        .map(|h| h.rules.mode)
        .ok_or_else(|| Error::Format(world.header_error.clone().unwrap_or_else(|| "no MAP_HEADER".into())))?;
    // 4FC600 reads the METAGAME chunk between KINGDOMS and USED_REQUIRED_OBJECTS
    let metagame = map.decode_chunk(METAGAME_CHUNK, max_chunk_bytes)?;
    metagame.verify()?;
    let stream = u32::from(world.is_saved_game());
    let mut armies = Vec::new();
    for p in &world.dynamic_things.placements {
        let Some(actor) = p.actor.as_ref() else { continue };
        let tail = &p.opaque_tail;
        let mut component = None;
        let mut stopped = false;
        let walked = ActorTail::decode_prefix_with_stages(tail, actor.version, stream, |slot, saved, mask, offset, bytes| {
            if !saved {
                return Ok((0, None));
            }
            if component.is_some() && minds.is_none() {
                stopped = true;
                return Err(Error::Format("walk stopped after the MetaGame component".into()));
            }
            let n = match slot {
                METAGAME_SLOT => {
                    let n = component_length(bytes)?;
                    component = Some((offset, bytes[..n].to_vec()));
                    n
                }
                4 => match minds.as_deref_mut() {
                    Some(m) => m.decode_mind(mask, bytes)?.0,
                    None => return Err(Error::Format("a saved Mind before the MetaGame component".into())),
                },
                6 => PropertiesPayload::decode_prefix(bytes)?.1,
                7 | 10 | 13 | 23 | 24 => {
                    return Err(Error::Format(format!("saved component {slot} beside a MetaGame component is not read here")));
                }
                _ => SavedComponent::decode_prefix(slot, bytes)?.1,
            };
            Ok((n, None))
        });
        let Some((offset, bytes)) = component else {
            // not an army: a failed walk before slot 27 is only an error when the
            // record holds a MetaGame component, which is not known without it
            if let Err(e) = walked
                && actor_has_metagame(tail)
            {
                return Err(Error::Format(format!("record {} ({}): {e}", p.record_ordinal, String::from_utf16_lossy(&p.thing_id_utf16))));
            }
            continue;
        };
        let walked_to_end = !stopped;
        if walked_to_end {
            let (_, consumed) = walked.map_err(|e| Error::Format(format!("army record {} ({}): {e}", p.record_ordinal, String::from_utf16_lossy(&p.thing_id_utf16))))?;
            if consumed != tail.len() {
                return Err(Error::Format(format!(
                    "army record {} ends {} bytes before its index entry",
                    p.record_ordinal,
                    tail.len() - consumed
                )));
            }
        }
        armies.push(SavedArmy {
            record_ordinal: p.record_ordinal,
            thing_id: String::from_utf16_lossy(&p.thing_id_utf16),
            serialized_id: p.serialized_id,
            owner_handle: actor.owner_handle,
            component_span: ByteSpan { offset: p.opaque_tail_span.offset + offset, length: bytes.len() },
            component: bytes,
            walked_to_end,
        });
    }
    Ok(CampaignSave { mode, metagame: metagame.bytes, armies, world })
}

/// The saved component mask's slot-27 bit (the tail starts with the default and
/// the saved masks, 4D8CF0).
fn actor_has_metagame(tail: &[u8]) -> bool {
    tail.get(4..8).is_some_and(|m| u32::from_le_bytes([m[0], m[1], m[2], m[3]]) & (1 << METAGAME_SLOT) != 0)
}
