//! Original serialized team/kingdom inputs. Runtime ownership is represented
//! by saved handles; names and header player slots are never substituted for IDs.
use crate::{
    Chunk, Error, Result, SourceIdentity, ThingState,
    inventory::{ByteSpan, Reader},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamRecord {
    pub source_span: ByteSpan,
    pub definition_id_utf16: Vec<u16>,
    pub serialized_id: u32,
    pub thing: ThingState,
    pub version: u32,
    pub slot_index: u32,
    pub display_key_utf16: Vec<u16>,
    pub words_4c: Vec<u32>,
    pub words_58: Vec<u32>,
    pub string_64_utf16: Option<Vec<u16>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TeamSlot {
    pub source_presence_byte: u8,
    pub record: Option<TeamRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Teams {
    pub source: SourceIdentity,
    pub slots: Vec<TeamSlot>,
}

fn words(r: &mut Reader<'_>) -> Result<Vec<u32>> {
    let count = r.u32()? as usize;
    let bytes = count
        .checked_mul(4)
        .ok_or_else(|| Error::Format("team vector overflow".into()))?;
    Ok(r.take(bytes)?
        .chunks_exact(4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
        .collect())
}
impl Teams {
    /// Original key-only RMP localization mode, as executed by 49AD70 with
    /// stream+48=true/editor=false. The translated display string is a later
    /// host lookup; this decoder preserves only the serialized key.
    pub fn decode(chunk: &Chunk) -> Result<Self> {
        chunk.verify()?;
        if chunk.directory.name != "TEAMS" {
            return Err(Error::Format("incorrect TEAMS chunk identity".into()));
        }
        let mut r = Reader::new(&chunk.bytes);
        let count = r.u32()? as usize;
        if count > chunk.bytes.len() - r.at {
            return Err(Error::Format("team slot count exceeds chunk".into()));
        }
        let mut slots = Vec::new();
        slots.try_reserve(count).map_err(|_| Error::Allocation)?;
        for index in 0..count {
            let source_presence_byte = r.take(1)?[0];
            let record = if source_presence_byte == 0 {
                None
            } else {
                let start = r.at;
                let definition_id_utf16 = r.string()?;
                let class = r.u32()?;
                if class != 5 {
                    return Err(Error::Format(format!(
                        "TEAMS slot{index} has native class {class}, expected5"
                    )));
                }
                let serialized_id = r.u32()?;
                let (thing, consumed) = ThingState::decode_prefix(&chunk.bytes[r.at..])?;
                r.take(consumed)?;
                let version = r.version(1)?;
                let slot_index = r.u32()?;
                if slot_index != index as u32 {
                    return Err(Error::Format(format!(
                        "serialized team index {slot_index} differs from slot {index}"
                    )));
                }
                let display_key_utf16 = r.string()?;
                let words_4c = words(&mut r)?;
                let words_58 = words(&mut r)?;
                let string_64_utf16 = if version >= 1 {
                    Some(r.string()?)
                } else {
                    None
                };
                Some(TeamRecord {
                    source_span: ByteSpan {
                        offset: start,
                        length: r.at - start,
                    },
                    definition_id_utf16,
                    serialized_id,
                    thing,
                    version,
                    slot_index,
                    display_key_utf16,
                    words_4c,
                    words_58,
                    string_64_utf16,
                })
            };
            slots.push(TeamSlot {
                source_presence_byte,
                record,
            });
        }
        if r.at != chunk.bytes.len() {
            return Err(Error::Format("TEAMS contains trailing bytes".into()));
        }
        Ok(Self {
            source: chunk.payload_source.clone(),
            slots,
        })
    }
}

use crate::{
    actor_payload::{ActorPayload, CountedList, UpdatablePayload},
    inventory::ObjectState,
    recruitment::Recruitment,
};

fn counted<T>(
    r: &mut Reader<'_>,
    minimum: usize,
    mut read: impl FnMut(&mut Reader<'_>) -> Result<T>,
) -> Result<CountedList<T>> {
    let serialized_count = r.u32()?;
    let count = (serialized_count as i32).max(0) as usize;
    if count > (r.bytes.len() - r.at) / minimum.max(1) {
        return Err(Error::Format(format!(
            "kingdom list count {count} stride {minimum} at {} exceeds remaining {} bytes",
            r.at - 4,
            r.bytes.len() - r.at
        )));
    }
    let mut values = Vec::new();
    values.try_reserve(count).map_err(|_| Error::Allocation)?;
    for _ in 0..count {
        values.push(read(r)?);
    }
    Ok(CountedList {
        serialized_count,
        values,
    })
}
fn word_list(r: &mut Reader<'_>) -> Result<CountedList<u32>> {
    counted(r, 4, |r| r.u32())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceWord {
    pub reference_utf16: Vec<u16>,
    pub word: u32,
}
fn reference_words(r: &mut Reader<'_>) -> Result<CountedList<ReferenceWord>> {
    counted(r, 6, |r| {
        Ok(ReferenceWord {
            reference_utf16: r.string()?,
            word: r.u32()?,
        })
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityState {
    pub version: u32,
    pub word_0c: u32,
    pub word_10: u32,
    pub word_14: u32,
    pub byte_18: u8,
    pub words: Option<CountedList<u32>>,
}
pub(crate) fn ability(r: &mut Reader<'_>) -> Result<AbilityState> {
    let version = r.version(1)?;
    Ok(AbilityState {
        version,
        word_0c: r.u32()?,
        word_10: r.u32()?,
        word_14: r.u32()?,
        byte_18: r.take(1)?[0],
        words: if version > 0 {
            Some(word_list(r)?)
        } else {
            None
        },
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderState {
    pub version: u32,
    pub word_0c: u32,
    pub abilities: CountedList<AbilityState>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commander {
    pub presence: Option<u8>,
    pub reference_utf16: Option<Vec<u16>>,
    pub state: Option<CommanderState>,
}
fn commander(r: &mut Reader<'_>, version: u32) -> Result<Commander> {
    let presence = if version >= 13 {
        Some(r.take(1)?[0])
    } else {
        None
    };
    if presence == Some(0) {
        return Ok(Commander {
            presence,
            reference_utf16: None,
            state: None,
        });
    }
    let reference_utf16 = Some(r.string()?);
    let state = if version >= 13 {
        Some(CommanderState {
            version: r.version(0)?,
            word_0c: r.u32()?,
            abilities: counted(r, 17, ability)?,
        })
    } else {
        None
    };
    Ok(Commander {
        presence,
        reference_utf16,
        state,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyState {
    pub version: u32,
    pub word_04: u32,
    pub word_08: Option<u32>,
    pub reference_utf16: Vec<u16>,
    pub byte_10: Option<u8>,
    pub word_14: Option<u32>,
}
fn property(r: &mut Reader<'_>) -> Result<PropertyState> {
    let version = r.version(3)?;
    let word_04 = r.u32()?;
    // 503B10 versions0 and1 do not both carry the same payload.
    let word_08 = if version > 0 { Some(r.u32()?) } else { None };
    let reference_utf16 = r.string()?;
    let byte_10 = if version > 1 {
        Some(r.take(1)?[0])
    } else {
        None
    };
    let word_14 = if version > 1 { Some(r.u32()?) } else { None };
    Ok(PropertyState {
        version,
        word_04,
        word_08,
        reference_utf16,
        byte_10,
        word_14,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TacticalState {
    pub version: u32,
    pub definition_groups: Vec<CountedList<ReferenceWord>>,
    pub handle_groups: Vec<CountedList<[u32; 2]>>,
}
fn tactical(r: &mut Reader<'_>) -> Result<TacticalState> {
    let version = r.version(2)?;
    let count = 4 + usize::from(version > 1);
    let mut definition_groups = Vec::new();
    let mut handle_groups = Vec::new();
    for _ in 0..count {
        definition_groups.push(reference_words(r)?);
    }
    if version > 0 {
        for _ in 0..count {
            handle_groups.push(counted(r, 8, |r| Ok([r.u32()?, r.u32()?]))?);
        }
    }
    Ok(TacticalState {
        version,
        definition_groups,
        handle_groups,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicState {
    pub version: u32,
    pub words: Vec<u32>,
}
fn strategic(r: &mut Reader<'_>) -> Result<StrategicState> {
    let version = r.version(1)?;
    let mut words = Vec::new();
    for _ in 0..if version == 0 { 60 } else { 66 } {
        words.push(r.u32()?);
    }
    Ok(StrategicState { version, words })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicEntry {
    pub version: u32,
    pub state: StrategicState,
    pub words_10c: Vec<u32>,
}
fn strategic_entry(r: &mut Reader<'_>) -> Result<StrategicEntry> {
    let version = r.version(0)?;
    let state = strategic(r)?;
    let mut words_10c = Vec::new();
    for _ in 0..21 {
        words_10c.push(r.u32()?);
    }
    Ok(StrategicEntry {
        version,
        state,
        words_10c,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiState {
    pub tactical: Option<TacticalState>,
    pub strategic: StrategicState,
    pub entries: CountedList<StrategicEntry>,
    pub word_334: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomPayload {
    pub version: u32,
    pub word_188: u32,
    pub word_18c: u32,
    pub word_190: u32,
    pub display_key_utf16: Vec<u16>,
    pub color_reference_utf16: Vec<u16>,
    pub words_1d0: CountedList<u32>,
    pub resources_1c4: CountedList<ReferenceWord>,
    pub team_handle: Option<u32>,
    pub words_1dc: Option<CountedList<u32>>,
    pub byte_23a: Option<u8>,
    pub recruitment: Option<Recruitment>,
    pub commander: Option<Commander>,
    pub nation_reference_utf16: Option<Vec<u16>>,
    pub named_words: Option<CountedList<ReferenceWord>>,
    pub word_19c: Option<u32>,
    pub references_24c: Option<CountedList<Vec<u16>>>,
    pub properties: Option<CountedList<PropertyState>>,
    pub bytes_23b_23c: Option<[u8; 2]>,
    pub string_240_utf16: Option<Vec<u16>>,
    pub string_348_utf16: Option<Vec<u16>>,
    pub ai: Option<AiState>,
    pub named_states_count: Option<u32>,
    pub references_35c: Option<CountedList<ReferenceWord>>,
    pub words_36c: Option<CountedList<u32>>,
    pub byte_238: Option<u8>,
    pub byte_239: Option<u8>,
    pub byte_314: Option<u8>,
    pub resources_258: Option<CountedList<ReferenceWord>>,
}
impl KingdomPayload {
    pub fn decode_prefix(bytes: &[u8]) -> Result<(Self, usize)> {
        let mut r = Reader::new(bytes);
        let value = Self::read(&mut r)?;
        Ok((value, r.at))
    }
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let version = r.version(20)?;
        let word_188 = r.u32()?;
        let word_18c = r.u32()?;
        let word_190 = r.u32()?;
        let display_key_utf16 = r.string()?;
        let color_reference_utf16 = r.string()?;
        let words_1d0 = word_list(r)?;
        let resources_1c4 = reference_words(r)?;
        let team_handle = if version > 0 { Some(r.u32()?) } else { None };
        let words_1dc = if version > 0 {
            Some(word_list(r)?)
        } else {
            None
        };
        let byte_23a = if version > 2 {
            Some(r.take(1)?[0])
        } else {
            None
        };
        let recruitment = if version > 1 {
            Some(Recruitment::read(r)?)
        } else {
            None
        };
        let commander = if version > 3 {
            Some(commander(r, version)?)
        } else {
            None
        };
        let nation_reference_utf16 = if version > 3 { Some(r.string()?) } else { None };
        let named_words = if version > 4 {
            Some(reference_words(r)?)
        } else {
            None
        };
        let word_19c = if version > 5 { Some(r.u32()?) } else { None };
        let references_24c = if version > 13 {
            Some(counted(r, 2, |r| r.string())?)
        } else {
            None
        };
        let properties = if version > 6 {
            Some(counted(r, 10, property)?)
        } else {
            None
        };
        let bytes_23b_23c = if version > 7 {
            Some(r.take(2)?.try_into().unwrap())
        } else {
            None
        };
        let string_240_utf16 = if version > 7 { Some(r.string()?) } else { None };
        let string_348_utf16 = if version > 8 { Some(r.string()?) } else { None };
        let ai = if version > 9 && word_190 == 0 {
            let tactical = if version > 10 {
                Some(tactical(r)?)
            } else {
                None
            };
            let before = if version > 10 {
                Some(strategic(r)?)
            } else {
                None
            };
            let entries = counted(r, 332, strategic_entry)?;
            let strategic = if let Some(state) = before {
                state
            } else {
                strategic(r)?
            };
            let word_334 = r.u32()?;
            Some(AiState {
                tactical,
                strategic,
                entries,
                word_334,
            })
        } else {
            None
        };
        let named_states_count = if version > 11 {
            let count = r.u32()?;
            if count as i32 > 0 {
                return Err(Error::Format(format!(
                    "unported named kingdom state records at byte {}",
                    r.at - 4
                )));
            }
            Some(count)
        } else {
            None
        };
        let references_35c = if version > 14 {
            Some(reference_words(r)?)
        } else {
            None
        };
        let words_36c = if version > 15 {
            Some(word_list(r)?)
        } else {
            None
        };
        let byte_238 = if version > 16 {
            Some(r.take(1)?[0])
        } else {
            None
        };
        let byte_239 = if version > 17 {
            Some(r.take(1)?[0])
        } else {
            None
        };
        let byte_314 = if version > 18 {
            Some(r.take(1)?[0])
        } else {
            None
        };
        let resources_258 = if version > 19 {
            Some(reference_words(r)?)
        } else {
            None
        };
        Ok(Self {
            version,
            word_188,
            word_18c,
            word_190,
            display_key_utf16,
            color_reference_utf16,
            words_1d0,
            resources_1c4,
            team_handle,
            words_1dc,
            byte_23a,
            recruitment,
            commander,
            nation_reference_utf16,
            named_words,
            word_19c,
            references_24c,
            properties,
            bytes_23b_23c,
            string_240_utf16,
            string_348_utf16,
            ai,
            named_states_count,
            references_35c,
            words_36c,
            byte_238,
            byte_239,
            byte_314,
            resources_258,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomRecord {
    pub source_span: ByteSpan,
    pub definition_id_utf16: Vec<u16>,
    pub serialized_id: u32,
    pub thing: ThingState,
    pub object: Option<ObjectState>,
    pub actor: Option<ActorPayload>,
    pub updatable: Option<UpdatablePayload>,
    pub payload: KingdomPayload,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomSlot {
    pub source_presence_byte: u8,
    pub record: Option<KingdomRecord>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kingdoms {
    pub source: SourceIdentity,
    pub world_version: u32,
    pub major_count: u32,
    pub slots: Vec<KingdomSlot>,
}
impl Kingdoms {
    pub fn decode(chunk: &Chunk, world_version: u32) -> Result<Self> {
        chunk.verify()?;
        if chunk.directory.name != "KINGDOMS" {
            return Err(Error::Format("incorrect KINGDOMS chunk identity".into()));
        }
        let mut r = Reader::new(&chunk.bytes);
        let major_count = r.u32()?;
        let count = r.u32()? as usize;
        if count > r.bytes.len() - r.at {
            return Err(Error::Format(
                "kingdom slot count exceeds remaining bytes".into(),
            ));
        }
        let mut slots = Vec::new();
        slots.try_reserve(count).map_err(|_| Error::Allocation)?;
        for index in 0..count {
            let source_presence_byte = r.take(1)?[0];
            let record = if source_presence_byte == 0 {
                None
            } else {
                let start = r.at;
                let definition_id_utf16 = r.string()?;
                let class = r.u32()?;
                if class != 4 {
                    return Err(Error::Format(format!(
                        "KINGDOMS slot {index} has class {class}, expected4"
                    )));
                }
                let serialized_id = r.u32()?;
                let (thing, n) = ThingState::decode_prefix(&r.bytes[r.at..])?;
                r.take(n)?;
                let (object, actor) = if world_version >= 19 {
                    let version = r.version(5)?;
                    let object = ObjectState {
                        version,
                        scale_multiplier_bits: r.u32()?,
                        variant_index: if version >= 1 {
                            Some(r.u32()? as i32)
                        } else {
                            None
                        },
                        associated_object_id: if version >= 2 { Some(r.u32()?) } else { None },
                        legacy_word: if version >= 3 { Some(r.u16()?) } else { None },
                        nested_count: if version >= 4 { Some(r.u32()?) } else { None },
                    };
                    if object.nested_count.unwrap_or(0) != 0 {
                        return Err(Error::Format(
                            "nested kingdom Object payload is unsupported".into(),
                        ));
                    }
                    let (actor, n) = ActorPayload::decode_prefix(&r.bytes[r.at..], 0)?;
                    r.take(n)?;
                    (Some(object), Some(actor))
                } else {
                    (None, None)
                };
                let updatable = if world_version > 21 {
                    let (value, n) = UpdatablePayload::decode_prefix(&r.bytes[r.at..])?;
                    r.take(n)?;
                    Some(value)
                } else {
                    None
                };
                let payload = KingdomPayload::read(&mut r)?;
                if payload.word_188 != index as u32 {
                    return Err(Error::Format(format!(
                        "kingdom slot field {} differs from source slot {index}",
                        payload.word_188
                    )));
                }
                Some(KingdomRecord {
                    source_span: ByteSpan {
                        offset: start,
                        length: r.at - start,
                    },
                    definition_id_utf16,
                    serialized_id,
                    thing,
                    object,
                    actor,
                    updatable,
                    payload,
                })
            };
            slots.push(KingdomSlot {
                source_presence_byte,
                record,
            });
        }
        if r.at != r.bytes.len() {
            return Err(Error::Format(format!(
                "KINGDOMS trailing bytes: consumed {} of {}",
                r.at,
                r.bytes.len()
            )));
        }
        Ok(Self {
            source: chunk.payload_source.clone(),
            world_version,
            major_count,
            slots,
        })
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    #[test]
    fn versioned_leaf_payloads_match_original_readers() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase6-kingdoms/native-leaves.json"
        ))
        .unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 23);
        for case in cases {
            let hex = case["payload"].as_str().unwrap();
            let mut bytes: Vec<u8> = hex
                .as_bytes()
                .chunks_exact(2)
                .map(|c| u8::from_str_radix(std::str::from_utf8(c).unwrap(), 16).unwrap())
                .collect();
            bytes.extend_from_slice(b"FOLLOWING");
            let mut r = Reader::new(&bytes);
            let got = match case["kind"].as_str().unwrap() {
                "property" => serde_json::to_value(property(&mut r).unwrap()).unwrap(),
                "ability" => serde_json::to_value(ability(&mut r).unwrap()).unwrap(),
                "tactical" => serde_json::to_value(tactical(&mut r).unwrap()).unwrap(),
                "strategic" => serde_json::to_value(strategic(&mut r).unwrap()).unwrap(),
                other => panic!("unexpected fixture {other}"),
            };
            assert_eq!(got, case["expected"]);
            assert_eq!(r.at, case["native"]["consumed"].as_u64().unwrap() as usize);
        }
    }
}
