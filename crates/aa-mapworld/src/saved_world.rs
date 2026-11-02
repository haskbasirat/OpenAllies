//! A saved game's things, read whole: the record of each DYNAMIC_THINGS entry with
//! every saved component it holds (0x4FAE80, GActor::Load 0x4D8CF0), an
//! organization's member records included (57D130 -> 4FC200 -> 4FAE80).
//!
//! This is the original's byte order and nothing else: it builds no live thing and
//! needs no world, so it takes a thing's bytes apart before anything is known of
//! the definitions they name. `restore::load_record` is the native Load with all
//! its side effects; this walks the same wire to read the state a world is rebuilt
//! from. The Mind (slot 4) needs its concrete decoder from aa-mind, which is passed
//! in: aa-mind sits above this crate.
use crate::actor_payload::{ActorTail, PropertiesPayload, UpdatablePayload};
use crate::inventory::{ActorPrefix, Placement, Reader, decode_embedded_record};
use crate::kingdoms::{PropertyState, read_property};
use crate::saved_components::{SavedComponent, StructurePayload};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// A property object of a saved Properties component (581AE0, factory 5030A0):
/// class 0 (vtable A29678, Load 503B10) or 1, a modifier property (vtable A2EBE0,
/// Load 503BC0, which holds a word more from version 3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedProperty {
    pub class: u32,
    pub state: PropertyState,
    pub word_18: Option<u32>,
}
impl SavedProperty {
    /// The IDS of the property's data (+0xC).
    pub fn id(&self) -> String {
        String::from_utf16_lossy(&self.state.reference_utf16)
    }
}

/// A damage record (4C0BA0), an Element's history and an Organization's last hit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageRecord {
    pub word_00: u32,
    pub word_04: u32,
    pub damage_type: String,
    pub word_0c: u32,
    pub word_10: u32,
    pub word_14: Option<u32>,
    pub word_18: Option<u32>,
}

/// One item of a saved Structure queue (58A470). `kind`: 0 a recruit, 1 research,
/// 2 a recruit at a location.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedQueueItem {
    /// Structure version > 12: the item's id.
    pub id: Option<u32>,
    pub kind: u32,
    /// Kinds 0 and 2: the ActorData's IDS (4C2E60).
    pub actor: Option<String>,
    /// The recruit's layout, when the item has its own (4C3050, 4C3180).
    pub layout: Option<(String, Vec<String>)>,
    /// Kind 1: the SharedProperty's IDS (4DAAA0).
    pub research: Option<String>,
    /// Kind 2: where.
    pub location: Option<[u32; 2]>,
    /// Version > 3: +1C and +2C, what was paid and the upkeep it asks.
    pub paid: Vec<u32>,
    pub requested: Vec<u32>,
    /// Version > 7: +3C, the upkeep it holds.
    pub current: Vec<u32>,
}

/// A saved AircraftComponent (slot 23, 55FEE0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedAircraft {
    pub version: u32,
    pub byte_10: u8,
    /// +14 / definition speed, +18 / definition +414.
    pub speed_fraction_bits: u32,
    pub turn_fraction_bits: u32,
    /// +1C: position on the path.
    pub position_bits: [u32; 3],
    /// Segments of the path (+28, Load 55F070) and its five words.
    pub segments: u32,
    pub path_words: Vec<u32>,
    pub byte_90: u8,
    /// The three group references, read through 51B580.
    pub groups: [u32; 3],
    pub updatable: UpdatablePayload,
}

/// What a kingdom remembers of an object it no longer sees: GObject::Load 0x4EC7A0
/// holds one per kingdom (+0x5C), a 0x38 object 4EB9F0 with its Load 4EC670.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedSighting {
    /// The kingdom's slot in the object's list.
    pub kingdom: u32,
    pub word_00: u32,
    pub text_04: Vec<u16>,
    pub position_bits: [u32; 2],
    /// The ObjectData's IDS (4EC5A0).
    pub definition: Vec<u16>,
    pub word_1c: u32,
    pub word_20: u32,
    pub byte_30: Option<u8>,
    pub word_2c: Option<u32>,
    pub word_34: Option<u32>,
}

/// GObject::Load's list after its count: presence bytes from object version 5, and
/// the old form's own flag in 4EC670 before that.
fn sightings(bytes: &[u8], count: u32, object_version: u32) -> Result<(Vec<SavedSighting>, usize)> {
    let mut r = Reader::new(bytes);
    let mut out = Vec::new();
    for kingdom in 0..(count as i32).max(0) {
        if object_version >= 5 && r.take(1)?[0] == 0 {
            continue;
        }
        let version = r.version(4)?;
        if version < 1 && r.take(1)?[0] == 0 {
            continue;
        }
        let word_00 = r.u32()?;
        let text_04 = r.string()?;
        let position_bits = [r.u32()?, r.u32()?];
        let definition = r.string()?;
        r.u32()?;
        r.u32()?;
        let word_1c = r.u32()?;
        let word_20 = r.u32()?;
        let byte_30 = if version > 1 { Some(r.take(1)?[0]) } else { None };
        let word_2c = if version > 2 { Some(r.u32()?) } else { None };
        let word_34 = if version > 3 { Some(r.u32()?) } else { None };
        out.push(SavedSighting {
            kingdom: kingdom as u32,
            word_00,
            text_04,
            position_bits,
            definition,
            word_1c,
            word_20,
            byte_30,
            word_2c,
            word_34,
        });
    }
    Ok((out, r.at))
}

/// One entry of a GGroup (class 8, 4E0A90): the member's id, its offset and flags.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedGroupMember {
    pub id: u32,
    pub offset_bits: [u32; 2],
    pub flags: [u8; 2],
}

/// A saved Organization's words (57D130).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedOrganization {
    pub version: u32,
    /// The Updatable's table id (50BCD0).
    pub updatable_id: Option<i32>,
    /// The LayoutData IDS and its parameters (4C3180).
    pub layout: String,
    pub parameters: Vec<String>,
    /// Layout slots the save holds.
    pub slots: u32,
    /// The member of each occupied slot, in saved slot order.
    pub members: Vec<(i32, SavedThing)>,
    /// Version >= 7: the recruit item single of each slot, by ActorData IDS.
    pub recruit_singles: Vec<String>,
    /// Leader's slot; negative for none.
    pub leader: i32,
    /// 59E300: morale as a fraction of the organization's maximum.
    pub morale_bits: u32,
    /// Formation indices (+70, +74); negative for none.
    pub formation: i32,
    pub secondary_formation: Option<i32>,
    /// +78: each slot's formation offset.
    pub offsets: Vec<[u32; 2]>,
    /// +50 words of each slot.
    pub words_50: Vec<u32>,
    pub word_84: Option<u32>,
    pub bytes_8c: Option<[u8; 2]>,
    /// 4C0BA0 damage record +90, +94, its damage type's IDS, +9C, +A0.
    pub damage: Option<(u32, u32, String, u32, u32)>,
    pub word_a4: Option<u32>,
    pub word_a8: Option<u32>,
    pub word_88: Option<u32>,
    /// +B4: when it was created.
    pub created_at_bits: Option<u32>,
    /// +BC, +C0.
    pub position_bits: Option<[u32; 2]>,
    pub word_d0: Option<u32>,
    pub word_d4: Option<u32>,
}

/// One record of a saved game with what it holds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedThing {
    pub record: Placement,
    pub tail: Option<ActorTail>,
    /// The components in their saved order, except the Organization and the Mind.
    pub components: Vec<(u8, SavedComponent)>,
    pub organization: Option<Box<SavedOrganization>>,
    /// The saved Mind's bytes, for the concrete decoder.
    pub mind: Option<Vec<u8>>,
    /// Slot 6: the property objects the Properties component holds.
    pub properties: Vec<SavedProperty>,
    /// Slot 8: the Element's damage history.
    pub element_history: Vec<DamageRecord>,
    /// Slot 13: the Structure's queue, head first.
    pub queue: Vec<SavedQueueItem>,
    /// Slot 10: the Denizen's command group references (51B580).
    pub denizen_commands: Vec<u32>,
    /// Slot 23.
    pub aircraft: Option<SavedAircraft>,
    /// Class 8: the group's members.
    pub group_members: Vec<SavedGroupMember>,
    /// GObject's per-kingdom memories of this object.
    pub sightings: Vec<SavedSighting>,
    /// The Updatable table id of each component that holds one (50BCD0): what the
    /// UPDATABLES chunk's entries name.
    pub updatables: Vec<(u8, i32)>,
    /// What stopped the walk: the components before it are read, the record's
    /// own length (a top-level record's index entry) passed over the rest.
    pub partial: Option<String>,
    /// Bytes of the record this walk read (the whole record for a top-level one).
    pub consumed: usize,
}

/// What walking a record needs from above this crate.
pub trait MindDecoder {
    /// The Mind (slot 4) of an actor with component mask `mask`, as 4CF480 selects
    /// it: the bytes its Load consumes and its Updatable's table id.
    fn decode_mind(&mut self, mask: u32, bytes: &[u8]) -> Result<(usize, Option<i32>)>;
}

fn string_of(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

/// Layout object 4C3180: version 0, the LayoutData's IDS, the saved parameter
/// count and one ActorData IDS per parameter.
fn layout(r: &mut Reader<'_>) -> Result<(String, Vec<String>)> {
    r.version(0)?;
    let id = string_of(&r.string()?);
    let count = (r.u32()? as i32).max(0);
    let mut parameters = Vec::new();
    for _ in 0..count {
        parameters.push(string_of(&r.string()?));
    }
    Ok((id, parameters))
}

/// 57D130's bytes. Member records are walked as they come; what the native Load
/// does with them (0x57CBA0, the recruit pairing, 57CBA0's checks) is not this
/// reader's.
fn organization(
    bytes: &[u8],
    world_version: u32,
    stream: u32,
    minds: &mut dyn MindDecoder,
) -> Result<(SavedOrganization, usize)> {
    let mut r = Reader::new(bytes);
    let updatable = r.version(1)?;
    let updatable_id = if updatable >= 1 { Some(r.u32()? as i32) } else { None };
    let version = r.version(10)?;
    let (layout_id, parameters) = layout(&mut r)?;
    let slots = r.u32()? as i32;
    if slots < 0 {
        return Err(Error::Format(format!("negative organization slots {slots}")));
    }
    let mut members = Vec::new();
    for i in 0..slots {
        let slot = r.u32()? as i32;
        if slot < 0 {
            continue;
        }
        let rest = &bytes[r.at..];
        let placement = decode_embedded_record(rest, 0).map_err(|e| Error::Format(format!("member {i} of {slots} (layout {layout_id}, version {version}) at {}: {e}", r.at)))?;
        let mut thing = walk_record(placement, world_version, stream, true, minds)?;
        let head = thing.record.opaque_tail_span.offset;
        r.take(head + thing.consumed)?;
        thing.consumed += head;
        members.push((slot, thing));
    }
    if (2..=6).contains(&version) {
        for _ in 0..slots {
            r.u32()?;
        }
    }
    let mut recruit_singles = Vec::new();
    if version >= 7 {
        for _ in 0..slots {
            recruit_singles.push(string_of(&r.string()?));
        }
    }
    let leader = r.u32()? as i32;
    let morale_bits = r.u32()?;
    let formation = r.u32()? as i32;
    let secondary_formation = if version >= 3 { Some(r.u32()? as i32) } else { None };
    if version == 0 {
        r.u32()?;
    }
    let mut offsets = Vec::new();
    for _ in 0..slots {
        offsets.push([r.u32()?, r.u32()?]);
    }
    let mut words_50 = Vec::new();
    if version >= 1 {
        for _ in 0..slots {
            words_50.push(r.u32()?);
        }
    }
    let (mut word_84, mut bytes_8c, mut damage, mut word_a4, mut word_a8) = (None, None, None, None, None);
    if version >= 4 {
        word_84 = Some(r.u32()?);
        let pair = r.take(2)?;
        bytes_8c = Some([pair[0], pair[1]]);
        let (w90, w94) = (r.u32()?, r.u32()?);
        let ids = string_of(&r.string()?);
        let (w9c, wa0) = (r.u32()?, r.u32()?);
        damage = Some((w90, w94, ids, w9c, wa0));
        if world_version >= 0xf {
            if world_version >= 0x12 {
                word_a4 = Some(r.u32()?);
            } else {
                r.take(1)?;
            }
        }
        if world_version >= 0x14 {
            word_a8 = Some(r.u32()?);
        }
    }
    let word_88 = if version >= 5 { Some(r.u32()?) } else { None };
    let created_at_bits = if version >= 6 { Some(r.u32()?) } else { None };
    let position_bits = if version >= 8 { Some([r.u32()?, r.u32()?]) } else { None };
    let word_d0 = if version >= 9 { Some(r.u32()?) } else { None };
    let word_d4 = if version >= 10 { Some(r.u32()?) } else { None };
    Ok((
        SavedOrganization {
            version,
            updatable_id,
            layout: layout_id,
            parameters,
            slots: slots as u32,
            members,
            recruit_singles,
            leader,
            morale_bits,
            formation,
            secondary_formation,
            offsets,
            words_50,
            word_84,
            bytes_8c,
            damage,
            word_a4,
            word_a8,
            word_88,
            created_at_bits,
            position_bits,
            word_d0,
            word_d4,
        },
        r.at,
    ))
}

/// Transport 58C810's framing (`aa_actors::transport_state`): no Updatable record.
fn transport(bytes: &[u8]) -> Result<usize> {
    let mut r = Reader::new(bytes);
    let version = r.version(6)?;
    r.string()?;
    if r.take(1)?[0] != 0 {
        // 4C3050's reference, then the fresh object's Load 4C3180.
        r.string()?;
        layout(&mut r)?;
    }
    r.u32()?;
    r.u32()?;
    r.take(2)?;
    r.u32()?;
    r.u32()?;
    if version == 0 {
        let count = (r.u32()? as i32).max(0);
        for _ in 0..count {
            r.u32()?;
        }
    }
    if version >= 4 {
        r.u32()?;
    }
    r.u32()?;
    for _ in 0..2 {
        r.version(0)?;
        let count = r.u32()? as i32;
        if count < 0 {
            return Err(Error::Format(format!("negative ResourceVector count {count}")));
        }
        for _ in 0..count {
            r.u32()?;
        }
    }
    if version >= 3 && r.take(1)?[0] != 0 {
        r.u32()?;
        r.u32()?;
    }
    if version >= 5 {
        r.string()?;
    }
    if version >= 6 {
        r.u32()?;
    }
    Ok(r.at)
}

/// 4CD3F0: a ResourceVector.
fn resources(r: &mut Reader<'_>) -> Result<Vec<u32>> {
    r.version(0)?;
    let count = r.u32()? as i32;
    if count < 0 {
        return Err(Error::Format(format!("negative ResourceVector count {count}")));
    }
    let mut out = Vec::new();
    for _ in 0..count {
        out.push(r.u32()?);
    }
    Ok(out)
}

/// 4C0BA0 at world version `world_version`.
fn damage_record(r: &mut Reader<'_>, world_version: u32) -> Result<DamageRecord> {
    let (word_00, word_04) = (r.u32()?, r.u32()?);
    let damage_type = string_of(&r.string()?);
    let (word_0c, word_10) = (r.u32()?, r.u32()?);
    let mut word_14 = None;
    if world_version >= 0xf {
        if world_version >= 0x12 {
            word_14 = Some(r.u32()?);
        } else {
            r.take(1)?;
        }
    }
    let word_18 = if world_version >= 0x14 { Some(r.u32()?) } else { None };
    Ok(DamageRecord { word_00, word_04, damage_type, word_0c, word_10, word_14, word_18 })
}

/// 581AE0 with its property objects.
fn properties(bytes: &[u8]) -> Result<(PropertiesPayload, Vec<SavedProperty>, usize)> {
    let (updatable, n) = UpdatablePayload::decode_prefix(bytes)?;
    let mut r = Reader::new(bytes);
    r.take(n)?;
    let version = r.version(1)?;
    let count = r.u32()? as i32;
    let mut classes = Vec::new();
    let mut objects = Vec::new();
    for _ in 0..count.max(0) {
        let class = r.u32()?;
        classes.push(class);
        if class == u32::MAX {
            continue;
        }
        if class > 1 {
            return Err(Error::Format(format!("Unknown property class {class}")));
        }
        let state = read_property(&mut r)?;
        let word_18 = if class == 1 && state.version > 2 {
            r.version(0)?;
            Some(r.u32()?)
        } else {
            None
        };
        objects.push(SavedProperty { class, state, word_18 });
    }
    Ok((PropertiesPayload { updatable, version, property_classes: classes }, objects, r.at))
}

/// 570FA0.
fn element(bytes: &[u8], world_version: u32) -> Result<(SavedComponent, Vec<DamageRecord>, usize)> {
    let mut r = Reader::new(bytes);
    let version = r.version(3)?;
    let updatable = if version > 0 {
        let (value, n) = UpdatablePayload::decode_prefix(&bytes[r.at..])?;
        r.take(n)?;
        Some(value)
    } else {
        None
    };
    let legacy_byte = if version < 2 { Some(r.take(1)?[0]) } else { None };
    let mut history = Vec::new();
    let history_count = if version >= 3 {
        let count = r.u32()?;
        for _ in 0..(count as i32).max(0) {
            history.push(damage_record(&mut r, world_version)?);
        }
        Some(count)
    } else {
        None
    };
    Ok((SavedComponent::Element { version, updatable, legacy_byte, history_count }, history, r.at))
}

/// 58A770 with its queue items (58A470).
fn structure(bytes: &[u8]) -> Result<(StructurePayload, Vec<SavedQueueItem>, usize)> {
    let mut r = Reader::new(bytes);
    let version = r.version(13)?;
    let updatable = if version >= 6 {
        let (value, n) = UpdatablePayload::decode_prefix(&bytes[r.at..])?;
        r.take(n)?;
        Some(value)
    } else {
        None
    };
    let discarded_object_handle = r.u32()?;
    let legacy_reference_count = if version == 0 { Some(r.u32()?) } else { None };
    for _ in 0..legacy_reference_count.map_or(0, |n| (n as i32).max(0)) {
        r.string()?;
    }
    let queue_count = if version >= 2 { Some(r.u32()?) } else { None };
    let mut queue = Vec::new();
    for _ in 0..queue_count.map_or(0, |n| (n as i32).max(0)) {
        let id = if version > 12 { Some(r.u32()?) } else { None };
        let kind = r.u32()?;
        let mut item = SavedQueueItem {
            id,
            kind,
            actor: None,
            layout: None,
            research: None,
            location: None,
            paid: Vec::new(),
            requested: Vec::new(),
            current: Vec::new(),
        };
        match kind {
            0 | 2 => {
                item.actor = Some(string_of(&r.string()?));
                if r.take(1)?[0] != 0 {
                    // 4C3050's reference, then the fresh object's Load 4C3180.
                    r.string()?;
                    item.layout = Some(layout(&mut r)?);
                }
                if kind == 2 {
                    item.location = Some([r.u32()?, r.u32()?]);
                }
            }
            1 => item.research = Some(string_of(&r.string()?)),
            other => return Err(Error::Format(format!("unknown Structure queue item kind {other}"))),
        }
        if version > 3 {
            item.paid = resources(&mut r)?;
            item.requested = resources(&mut r)?;
        }
        if version > 7 {
            item.current = resources(&mut r)?;
        }
        if version == 9 {
            r.u32()?;
        }
        queue.push(item);
    }
    let (word_20, word_2c) = if version >= 2 { (Some(r.u32()?), Some(r.u32()?)) } else { (None, None) };
    let word_24 = if version >= 11 { Some(r.u32()?) } else { None };
    let word_28 = if version >= 12 { Some(r.u32()?) } else { None };
    let word_34 = if version >= 7 { Some(r.u32()?) } else { None };
    Ok((
        StructurePayload {
            version,
            updatable,
            discarded_object_handle,
            legacy_reference_count,
            queue_count,
            word_20,
            word_2c,
            word_24,
            word_28,
            word_34,
        },
        queue,
        r.at,
    ))
}

/// 56C320's framing up to its command references (`DenizenPayload`) and those.
fn denizen(bytes: &[u8]) -> Result<(Option<i32>, Vec<u32>, usize)> {
    let mut r = Reader::new(bytes);
    let updatable = r.version(1)?;
    let id = if updatable >= 1 { Some(r.u32()? as i32) } else { None };
    let version = r.version(4)?;
    if version >= 1 {
        r.take(1)?;
    }
    if version >= 2 {
        r.take(1)?;
    }
    if version >= 3 {
        let count = (r.u32()? as i32).max(0);
        for _ in 0..count {
            r.u32()?;
            r.u32()?;
        }
    }
    if version >= 4 {
        r.u32()?;
    }
    let mut commands = Vec::new();
    if version >= 3 {
        let handles = (r.u32()? as i32).max(0);
        for _ in 0..handles {
            r.u32()?;
        }
        let count = (r.u32()? as i32).max(0);
        for _ in 0..count {
            commands.push(r.u32()?);
        }
    }
    Ok((id, commands, r.at))
}

/// 55FEE0.
fn aircraft(bytes: &[u8]) -> Result<(SavedAircraft, usize)> {
    let mut r = Reader::new(bytes);
    let version = r.version(3)?;
    let mut out = SavedAircraft {
        version,
        byte_10: 0,
        speed_fraction_bits: 0,
        turn_fraction_bits: 0,
        position_bits: [0; 3],
        segments: 0,
        path_words: Vec::new(),
        byte_90: 0,
        groups: [0; 3],
        updatable: UpdatablePayload { version: 0, serialized_id: None },
    };
    if version > 0 {
        out.byte_10 = r.take(1)?[0];
        out.speed_fraction_bits = r.u32()?;
        out.turn_fraction_bits = r.u32()?;
        out.position_bits = [r.u32()?, r.u32()?, r.u32()?];
        // 55F070: the path.
        if r.version(1)? >= 1 {
            let segments = (r.u32()? as i32).max(0);
            out.segments = segments as u32;
            for _ in 0..segments {
                // 55D860.
                let segment = r.version(2)?;
                if segment >= 1 {
                    r.take(12 * 4 + 4)?;
                    if segment >= 2 {
                        r.u32()?;
                    }
                    r.take(12 + 4 + 4)?;
                }
            }
            for _ in 0..5 {
                out.path_words.push(r.u32()?);
            }
        }
        out.byte_90 = r.take(1)?[0];
        if version < 2 {
            r.u32()?;
        }
        out.groups = [r.u32()?, r.u32()?, r.u32()?];
        if version < 3 {
            r.take(2)?;
        }
    }
    let (updatable, n) = UpdatablePayload::decode_prefix(&bytes[r.at..])?;
    r.take(n)?;
    out.updatable = updatable;
    Ok((out, r.at))
}

/// The start of 4E0A90 after GActor::Load: the Updatable from world version 22, the
/// version and the members. What follows (+194, the group's commands) is passed
/// over: a group is a top-level record with its length in the index.
fn group(bytes: &[u8], world_version: u32) -> Result<Vec<SavedGroupMember>> {
    let mut r = Reader::new(bytes);
    if world_version > 0x15 {
        let (_, n) = UpdatablePayload::decode_prefix(bytes)?;
        r.take(n)?;
    }
    r.version(2)?;
    let count = (r.u32()? as i32).max(0);
    let mut out = Vec::new();
    for _ in 0..count {
        let id = r.u32()?;
        let offset_bits = [r.u32()?, r.u32()?];
        let pair = r.take(2)?;
        out.push(SavedGroupMember { id, offset_bits, flags: [pair[0], pair[1]] });
    }
    Ok(out)
}

/// A whole record: `placement.opaque_tail` runs to the end of the enclosing stream
/// for an embedded record, and `consumed` says where this one ended.
pub fn walk(
    placement: Placement,
    world_version: u32,
    stream: u32,
    minds: &mut dyn MindDecoder,
) -> Result<SavedThing> {
    walk_record(placement, world_version, stream, false, minds)
}

/// `embedded`: inside an Organization, with no index entry to fall back on, so a
/// failure ends the whole enclosing record.
fn walk_record(
    mut placement: Placement,
    world_version: u32,
    stream: u32,
    embedded: bool,
    minds: &mut dyn MindDecoder,
) -> Result<SavedThing> {
    // The record reader stops at an object with a list (a map has none): the list,
    // then the actor's prefix.
    let mut seen = Vec::new();
    if matches!(placement.class_code, 3 | 8)
        && placement.actor.is_none()
        && let Some(object) = placement.object.clone()
        && let Some(count) = object.nested_count.filter(|n| *n != 0)
    {
        let (list, n) = sightings(&placement.opaque_tail, count, object.version)?;
        let (prefix, m) = ActorPrefix::decode_prefix(&placement.opaque_tail[n..])?;
        seen = list;
        placement.actor = Some(prefix);
        placement.opaque_tail.drain(..n + m);
        placement.opaque_tail_span.offset += n + m;
        placement.opaque_tail_span.length -= n + m;
    }
    let Some(actor) = placement.actor.as_ref() else {
        let consumed = placement.opaque_tail.len();
        return Ok(SavedThing {
            record: placement,
            tail: None,
            components: Vec::new(),
            organization: None,
            mind: None,
            properties: Vec::new(),
            element_history: Vec::new(),
            queue: Vec::new(),
            denizen_commands: Vec::new(),
            aircraft: None,
            group_members: Vec::new(),
            sightings: seen,
            updatables: Vec::new(),
            partial: None,
            consumed,
        });
    };
    let mut components = Vec::new();
    let mut org = None;
    let mut mind = None;
    let mut saved_properties = Vec::new();
    let mut element_history = Vec::new();
    let mut queue = Vec::new();
    let mut denizen_commands = Vec::new();
    let mut saved_aircraft = None;
    let mut updatables: Vec<(u8, i32)> = Vec::new();
    let walked = ActorTail::decode_prefix_with_stages(
        &placement.opaque_tail,
        actor.version,
        stream,
        |slot, saved, mask, _, bytes| {
            if !saved {
                return Ok((0, None));
            }
            let consumed = match slot {
                4 => {
                    let (n, id) = minds.decode_mind(mask, bytes).map_err(|e| Error::Format(format!("saved Mind: {e}")))?;
                    mind = Some(bytes[..n].to_vec());
                    updatables.extend(id.map(|id| (slot, id)));
                    n
                }
                7 => {
                    let (value, n) = organization(bytes, world_version, stream, minds).map_err(|e| Error::Format(format!("saved Organization: {e}")))?;
                    updatables.extend(value.updatable_id.map(|id| (slot, id)));
                    org = Some(Box::new(value));
                    n
                }
                24 => transport(bytes).map_err(|e| Error::Format(format!("saved Transport: {e}")))?,
                6 => {
                    let (value, objects, n) = properties(bytes).map_err(|e| Error::Format(format!("saved Properties: {e}")))?;
                    updatables.extend(value.updatable.serialized_id.map(|id| (slot, id)));
                    components.push((slot, SavedComponent::Properties(value)));
                    saved_properties = objects;
                    n
                }
                8 => {
                    let (value, history, n) = element(bytes, world_version).map_err(|e| Error::Format(format!("saved Element: {e}")))?;
                    updatables.extend(value.updatable().and_then(|u| u.serialized_id).map(|id| (slot, id)));
                    components.push((slot, value));
                    element_history = history;
                    n
                }
                13 => {
                    let (value, items, n) = structure(bytes).map_err(|e| Error::Format(format!("saved Structure: {e}")))?;
                    updatables.extend(value.updatable.as_ref().and_then(|u| u.serialized_id).map(|id| (slot, id)));
                    components.push((slot, SavedComponent::Structure(value)));
                    queue = items;
                    n
                }
                10 => {
                    let (id, commands, n) = denizen(bytes).map_err(|e| Error::Format(format!("saved Denizen: {e}")))?;
                    updatables.extend(id.map(|id| (slot, id)));
                    denizen_commands = commands;
                    n
                }
                23 => {
                    let (value, n) = aircraft(bytes).map_err(|e| Error::Format(format!("saved Aircraft: {e}")))?;
                    updatables.extend(value.updatable.serialized_id.map(|id| (slot, id)));
                    saved_aircraft = Some(value);
                    n
                }
                _ => {
                    let (value, n) = SavedComponent::decode_prefix(slot, bytes).map_err(|e| Error::Format(format!("saved component {slot}: {e}")))?;
                    let id = value.updatable().and_then(|u| u.serialized_id);
                    let id = id.or_else(|| {
                        // 574040 holds its Updatable first.
                        (slot == 30 && bytes.len() >= 8 && bytes[..4] != [0; 4]).then(|| i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]))
                    });
                    updatables.extend(id.map(|id| (slot, id)));
                    components.push((slot, value));
                    n
                }
            };
            Ok((consumed, None))
        },
    );
    let (tail, consumed, partial) = match walked {
        Ok((tail, consumed)) => (Some(tail), consumed, None),
        Err(e) if !embedded => (None, placement.opaque_tail.len(), Some(e.to_string())),
        Err(e) => return Err(e),
    };
    let group_members = if partial.is_none() && placement.class_code == 8 {
        group(&placement.opaque_tail[consumed..], world_version)?
    } else {
        Vec::new()
    };
    let mut record = placement;
    let consumed = if record.class_code == 8 {
        record.opaque_tail.len()
    } else {
        consumed
    };
    record.opaque_tail.truncate(consumed);
    record.opaque_tail_span.length = consumed;
    Ok(SavedThing {
        record,
        tail,
        components,
        organization: org,
        mind,
        properties: saved_properties,
        element_history,
        queue,
        denizen_commands,
        aircraft: saved_aircraft,
        group_members,
        sightings: seen,
        updatables,
        partial,
        consumed,
    })
}
