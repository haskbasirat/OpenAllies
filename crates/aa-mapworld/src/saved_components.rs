//! Native saved-component wire payloads. These values are applied after actual
//! component construction; they do not stand in for constructor defaults.
use crate::{
    Error, Result,
    actor_payload::{CountedList, PropertiesPayload, UpdatablePayload},
    inventory::Reader,
};
use serde::{Deserialize, Serialize};
fn updatable(r: &mut Reader<'_>) -> Result<UpdatablePayload> {
    let (v, n) = UpdatablePayload::decode_prefix(&r.bytes[r.at..])?;
    r.take(n)?;
    Ok(v)
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyPayload {
    pub version: u32,
    pub updatable: Option<UpdatablePayload>,
    pub dying_started_bits: Option<u32>,
    pub health_multiplier_bits: u32,
    pub sink_duration_bits: Option<u32>,
    pub last_heal_time_bits: Option<u32>,
}
impl BodyPayload {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let version = r.version(4)?;
        Ok(Self {
            version,
            updatable: if version >= 2 {
                Some(updatable(r)?)
            } else {
                None
            },
            dying_started_bits: if version >= 1 { Some(r.u32()?) } else { None },
            health_multiplier_bits: r.u32()?,
            sink_duration_bits: if version >= 3 { Some(r.u32()?) } else { None },
            last_heal_time_bits: if version >= 4 { Some(r.u32()?) } else { None },
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoverPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    pub vector_10: [u32; 3],
    pub word_20: u32,
    pub word_24: u32,
    pub speed_multiplier_bits: Option<u32>,
    pub legacy_vector: Option<[u32; 3]>,
    pub word_44: u32,
    pub word_48: u32,
    pub byte_4c: Option<u8>,
    pub byte_54: Option<u8>,
    pub vector_38: Option<[u32; 3]>,
    pub word_50: Option<u32>,
    pub byte_56: Option<u8>,
    pub word_58: Option<u32>,
    pub byte_5c: Option<u8>,
    pub vector_60: Option<[u32; 3]>,
    pub byte_6c: Option<u8>,
    pub vector_70: Option<[u32; 3]>,
    pub vector_7c: Option<[u32; 3]>,
}
impl MoverPayload {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let updatable = updatable(r)?;
        let version = r.version(8)?;
        Ok(Self {
            updatable,
            version,
            vector_10: r.vector()?,
            word_20: r.u32()?,
            word_24: r.u32()?,
            speed_multiplier_bits: if version >= 8 { Some(r.u32()?) } else { None },
            legacy_vector: if version < 4 { Some(r.vector()?) } else { None },
            word_44: r.u32()?,
            word_48: r.u32()?,
            byte_4c: if (1..7).contains(&version) {
                Some(r.take(1)?[0])
            } else {
                None
            },
            byte_54: if version >= 2 {
                Some(r.take(1)?[0])
            } else {
                None
            },
            vector_38: if version >= 3 {
                Some(r.vector()?)
            } else {
                None
            },
            word_50: if version >= 3 { Some(r.u32()?) } else { None },
            byte_56: if version >= 5 {
                Some(r.take(1)?[0])
            } else {
                None
            },
            word_58: if version >= 5 { Some(r.u32()?) } else { None },
            byte_5c: if version >= 6 {
                Some(r.take(1)?[0])
            } else {
                None
            },
            vector_60: if version >= 6 {
                Some(r.vector()?)
            } else {
                None
            },
            byte_6c: if version >= 6 {
                Some(r.take(1)?[0])
            } else {
                None
            },
            vector_70: if version >= 6 {
                Some(r.vector()?)
            } else {
                None
            },
            vector_7c: if version >= 6 {
                Some(r.vector()?)
            } else {
                None
            },
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    pub word_28: u32,
    pub legacy_word: Option<u32>,
    pub animation_kind: i32,
    pub animation_index: i32,
    pub animation_version: u32,
    pub elapsed_bits: u32,
    pub resumed_at_bits: u32,
    pub rate_bits: u32,
    pub active_byte: u8,
    pub variant: Option<i32>,
}
impl CharacterPayload {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let updatable = updatable(r)?;
        let version = r.version(2)?;
        Ok(Self {
            updatable,
            version,
            word_28: r.u32()?,
            legacy_word: if version < 1 { Some(r.u32()?) } else { None },
            animation_kind: r.u32()? as i32,
            animation_index: r.u32()? as i32,
            animation_version: r.version(0)?,
            elapsed_bits: r.u32()?,
            resumed_at_bits: r.u32()?,
            rate_bits: r.u32()?,
            active_byte: r.take(1)?[0],
            variant: if version >= 2 {
                Some(r.u32()? as i32)
            } else {
                None
            },
        })
    }
}
fn word_list(r: &mut Reader<'_>) -> Result<CountedList<u32>> {
    let serialized_count = r.u32()?;
    let count = (serialized_count as i32).max(0) as usize;
    let bytes = count
        .checked_mul(4)
        .ok_or_else(|| Error::Format("component vector count overflow".into()))?;
    let values = r
        .take(bytes)?
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    Ok(CountedList {
        serialized_count,
        values,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructionPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    pub words_10: CountedList<u32>,
    pub words_1c: Option<CountedList<u32>>,
    pub reference_28_utf16: Option<Vec<u16>>,
    pub legacy_upgrade_words: Option<[u32; 2]>,
}
impl ConstructionPayload {
    pub fn apply(
        &self,
        state: &mut aa_actors::construction_state::ConstructionState,
        definition: &aa_actors::construction_state::Definition,
        reset_cost: bool,
    ) -> Result<()> {
        state.header.scheduled_head = None;
        // NaN fld/fstp behavior requires hardware-backed validation: Unicorn
        // does not reliably reproduce signaling-NaN quieting. Preserve proven
        // ordinary/infinite words and both zero signs; reject unknown cases.
        let copied = |bits: u32| -> Result<u32> {
            if bits & 0x7f800000 == 0x7f800000 && bits & 0x007fffff != 0 {
                Err(Error::Format(
                    "Construction saved NaN copy requires physical x87 validation".into(),
                ))
            } else {
                Ok(bits)
            }
        };
        for (dst, src) in state.words_10.iter_mut().zip(&self.words_10.values) {
            *dst = copied(*src)?;
        }
        if let Some(source) = &self.words_1c {
            for (dst, src) in state.words_1c.iter_mut().zip(&source.values) {
                *dst = copied(*src)?;
            }
        }
        if reset_cost {
            state
                .reset_base_cost(definition)
                .map_err(|e| Error::Format(e.to_string()))?;
        }
        if let Some(reference) = &self.reference_28_utf16 {
            if reference.is_empty() {
                state.definition_28 = None;
            } else {
                return Err(Error::Format(
                    "Construction saved upgrade definition requires canonical identity lookup"
                        .into(),
                ));
            }
        }
        if self
            .legacy_upgrade_words
            .is_some_and(|words| words != [0, 0])
        {
            return Err(Error::Format(
                "native Construction v0 rejects nonzero legacy upgrade fields".into(),
            ));
        }
        Ok(())
    }
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let updatable = updatable(r)?;
        let version = r.version(1)?;
        Ok(Self {
            updatable,
            version,
            words_10: word_list(r)?,
            words_1c: if version >= 1 {
                Some(word_list(r)?)
            } else {
                None
            },
            reference_28_utf16: if version >= 1 {
                Some(r.string()?)
            } else {
                None
            },
            legacy_upgrade_words: if version == 0 {
                Some([r.u32()?, r.u32()?])
            } else {
                None
            },
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformerStateEntry {
    pub words_00_04: [u32; 2],
    pub word_08: Option<u32>,
    pub present_byte: Option<u8>,
    pub words_10_14: Option<[u32; 2]>,
    pub word_18: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerformerPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    pub word_10: u32,
    pub multiplier_20: u32,
    pub word_28: u32,
    pub words_2c_30: [u32; 2],
    pub ability_index: i32,
    pub words_3c_40: [u32; 2],
    pub states: CountedList<PerformerStateEntry>,
    pub word_58: u32,
    pub word_44: Option<u32>,
    pub abilities: CountedList<crate::kingdoms::AbilityState>,
    pub byte_5c: Option<u8>,
}
fn counted<T>(
    r: &mut Reader<'_>,
    minimum: usize,
    mut read: impl FnMut(&mut Reader<'_>) -> Result<T>,
) -> Result<CountedList<T>> {
    let serialized_count = r.u32()?;
    let count = (serialized_count as i32).max(0) as usize;
    if count > (r.bytes.len() - r.at) / minimum {
        return Err(Error::Format("component list exceeds input extent".into()));
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
impl PerformerPayload {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let updatable = updatable(r)?;
        let version = r.version(9)?;
        if version < 5 {
            return Err(Error::Format(format!(
                "Performer version{version} requires legacy target descriptor codec"
            )));
        }
        let word_10 = r.u32()?;
        let multiplier_20 = r.u32()?;
        let word_28 = r.u32()?;
        let words_2c_30 = [r.u32()?, r.u32()?];
        let ability_index = r.u32()? as i32;
        let words_3c_40 = [r.u32()?, r.u32()?];
        let states = counted(r, 8, |r| {
            Ok(PerformerStateEntry {
                words_00_04: [r.u32()?, r.u32()?],
                word_08: if version >= 7 { Some(r.u32()?) } else { None },
                present_byte: if version >= 8 {
                    Some(r.take(1)?[0])
                } else {
                    None
                },
                words_10_14: if version >= 8 {
                    Some([r.u32()?, r.u32()?])
                } else {
                    None
                },
                word_18: if version >= 9 { Some(r.u32()?) } else { None },
            })
        })?;
        let word_58 = r.u32()?;
        let word_44 = if version >= 6 { Some(r.u32()?) } else { None };
        let abilities = counted(r, 17, crate::kingdoms::ability)?;
        let byte_5c = if version >= 7 {
            Some(r.take(1)?[0])
        } else {
            None
        };
        Ok(Self {
            updatable,
            version,
            word_10,
            multiplier_20,
            word_28,
            words_2c_30,
            ability_index,
            words_3c_40,
            states,
            word_58,
            word_44,
            abilities,
            byte_5c,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyPayload {
    pub version: u32,
    pub updatable: Option<UpdatablePayload>,
    pub amount_multiplier_bits: Option<u32>,
    pub word_1c: Option<u32>,
}
impl SupplyPayload {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let version = r.version(2)?;
        Ok(Self {
            version,
            updatable: if version > 0 {
                Some(updatable(r)?)
            } else {
                None
            },
            amount_multiplier_bits: if version >= 2 { Some(r.u32()?) } else { None },
            word_1c: if version >= 2 { Some(r.u32()?) } else { None },
        })
    }
}
/// Original 5849E0: Updatable record, version 0, then the in-progress
/// Property references (4DAAA0 UTF-16 IDS each).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    pub references: CountedList<Vec<u16>>,
}
/// Original 567020: Updatable record, version (max 2), Ability records
/// (version > 0) and saved entry pairs (version 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefenderPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    pub abilities: Option<CountedList<crate::kingdoms::AbilityState>>,
    pub pairs: Option<CountedList<[u32; 2]>>,
}
impl DefenderPayload {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let updatable = updatable(r)?;
        let version = r.version(2)?;
        let abilities = if version > 0 {
            Some(counted(r, 17, crate::kingdoms::ability)?)
        } else {
            None
        };
        let pairs = if version >= 2 {
            Some(counted(r, 8, |r| Ok([r.u32()?, r.u32()?]))?)
        } else {
            None
        };
        Ok(Self {
            updatable,
            version,
            abilities,
            pairs,
        })
    }
}
/// Original 583C80: Updatable record first, then version (max 3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecruitPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    /// +1C byte, present for version > 0.
    pub byte_1c: Option<u8>,
    /// +14/+18, present when the +1C byte is nonzero.
    pub words_14: Option<[u32; 2]>,
    /// Versions 1/2: legacy queue records consumed and discarded.
    pub legacy: Option<LegacyRecruitQueue>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyRecruitQueue {
    pub serialized_count: u32,
    /// (name, object flag); a set flag constructs a 4C2690 object, which is
    /// unported and rejected.
    pub items: Vec<(Vec<u16>, u8)>,
    pub word_after: u32,
    /// Version 2 only (583EEF reads one byte).
    pub byte_v2: Option<u8>,
}
impl RecruitPayload {
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let updatable = updatable(r)?;
        let version = r.version(3)?;
        let mut byte_1c = None;
        let mut words_14 = None;
        if version != 0 {
            let flag = r.take(1)?[0];
            byte_1c = Some(flag);
            if flag != 0 {
                words_14 = Some([r.u32()?, r.u32()?]);
            }
        }
        let legacy = if version == 1 || version == 2 {
            let serialized_count = r.u32()?;
            let mut items = Vec::new();
            for _ in 0..(serialized_count as i32).max(0) {
                let name = r.string()?;
                let flag = r.take(1)?[0];
                if flag != 0 {
                    return Err(Error::Format(
                        "legacy Recruit queue object (4C2690/4C3050) remains unported".into(),
                    ));
                }
                items.try_reserve(1).map_err(|_| Error::Allocation)?;
                items.push((name, flag));
            }
            let word_after = r.u32()?;
            let byte_v2 = if version > 1 { Some(r.take(1)?[0]) } else { None };
            Some(LegacyRecruitQueue {
                serialized_count,
                items,
                word_after,
                byte_v2,
            })
        } else {
            None
        };
        Ok(Self {
            updatable,
            version,
            byte_1c,
            words_14,
            legacy,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenizenPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    pub byte_20: Option<u8>,
    pub byte_21: Option<u8>,
    pub occupants: Option<CountedList<[u32; 2]>>,
    pub word_34: Option<u32>,
    pub handles: Option<CountedList<u32>>,
    pub command_count: Option<u32>,
}
impl DenizenPayload {
    /// `health(target)` is the slot target's ActorData+284 bits (45D730,
    /// null modifier).
    pub fn apply(
        &self,
        state: &mut aa_actors::denizen_state::DenizenState,
        health: impl FnMut(&str) -> std::result::Result<u32, aa_actors::organization_state::LoadError>,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        state.header.scheduled_head = None;
        if let Some(value) = self.byte_20 {
            state.byte_20 = value;
        }
        if let Some(value) = self.byte_21 {
            state.byte_21 = value;
        }
        // 56C39E..56C45B: the first min(count, slots) saved pairs go to the
        // constructed slots head first; the rest were read and discarded.
        if let Some(occupants) = &self.occupants {
            state
                .apply_occupants(&occupants.values, health, pc)
                .map_err(|e| Error::Format(e.to_string()))?;
        }
        if let Some(value) = self.word_34 {
            state.words.insert(0x34, value);
        }
        if let Some(handles) = &self.handles {
            for handle in &handles.values {
                state
                    .handles_2c
                    .try_reserve(1)
                    .map_err(|_| Error::Allocation)?;
                state.handles_2c.insert(0, *handle);
            }
        }
        if self.command_count.is_some_and(|count| (count as i32) > 0) {
            return Err(Error::Format(
                "Denizen command objects require concrete command factories".into(),
            ));
        }
        Ok(())
    }
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let updatable = updatable(r)?;
        let version = r.version(4)?;
        let byte_20 = if version >= 1 {
            Some(r.take(1)?[0])
        } else {
            None
        };
        let byte_21 = if version >= 2 {
            Some(r.take(1)?[0])
        } else {
            None
        };
        let occupants = if version >= 3 {
            Some(counted(r, 8, |r| Ok([r.u32()?, r.u32()?]))?)
        } else {
            None
        };
        let word_34 = if version >= 4 { Some(r.u32()?) } else { None };
        let handles = if version >= 3 {
            Some(word_list(r)?)
        } else {
            None
        };
        let command_count = if version >= 3 { Some(r.u32()?) } else { None };
        if command_count.is_some_and(|n| n as i32 > 0) {
            return Err(Error::Format(
                "nonempty Denizen command objects require native command codec".into(),
            ));
        }
        Ok(Self {
            updatable,
            version,
            byte_20,
            byte_21,
            occupants,
            word_34,
            handles,
            command_count,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructurePayload {
    pub version: u32,
    pub updatable: Option<UpdatablePayload>,
    pub discarded_object_handle: u32,
    pub legacy_reference_count: Option<u32>,
    pub queue_count: Option<u32>,
    pub word_20: Option<u32>,
    pub word_2c: Option<u32>,
    pub word_24: Option<u32>,
    pub word_28: Option<u32>,
    pub word_34: Option<u32>,
}
impl StructurePayload {
    pub fn apply(&self, state: &mut aa_actors::structure_state::StructureState) -> Result<()> {
        if self.updatable.is_some() {
            state.header.scheduled_head = None;
        }
        // Native58A7C6 explicitly discards this serialized reference.
        state.object_handle_10 = 0;
        if self.legacy_reference_count.is_some_and(|n| n as i32 > 0) {
            return Err(Error::Format(
                "Structure legacy references require native definition lookup".into(),
            ));
        }
        if self.queue_count.is_some_and(|n| n as i32 > 0) {
            return Err(Error::Format(
                "Structure queue requires native item construction".into(),
            ));
        }
        if let Some(value) = self.word_20 {
            state.word_20 = value;
        }
        if let Some(value) = self.word_2c {
            state.word_2c = value;
        }
        if let Some(value) = self.word_24 {
            state.word_24 = value;
        } else if self.version >= 2 && !state.queue_14.is_empty() {
            return Err(Error::Format(
                "Structure legacy nonempty queue requires immediate scheduling".into(),
            ));
        }
        if let Some(value) = self.word_28 {
            state.word_28 = value;
        }
        if let Some(value) = self.word_34 {
            state.word_34 = value;
        }
        Ok(())
    }
    fn read(r: &mut Reader<'_>) -> Result<Self> {
        let version = r.version(13)?;
        let updatable = if version >= 6 {
            Some(updatable(r)?)
        } else {
            None
        };
        let discarded_object_handle = r.u32()?;
        let legacy_reference_count = if version == 0 { Some(r.u32()?) } else { None };
        if legacy_reference_count.is_some_and(|n| n as i32 > 0) {
            return Err(Error::Format(
                "nonempty Structure v0 references require native definition codec".into(),
            ));
        }
        let queue_count = if version >= 2 { Some(r.u32()?) } else { None };
        if queue_count.is_some_and(|n| n as i32 > 0) {
            return Err(Error::Format(
                "nonempty Structure queue requires native queue item codec".into(),
            ));
        }
        Ok(Self {
            version,
            updatable,
            discarded_object_handle,
            legacy_reference_count,
            queue_count,
            word_20: if version >= 2 { Some(r.u32()?) } else { None },
            word_2c: if version >= 2 { Some(r.u32()?) } else { None },
            word_24: if version >= 11 { Some(r.u32()?) } else { None },
            word_28: if version >= 12 { Some(r.u32()?) } else { None },
            word_34: if version >= 7 { Some(r.u32()?) } else { None },
        })
    }
}
/// Saved Mind (slot 4). Its wire format depends on the concrete Mind the
/// factory constructs (536340 Element, 528C20 Regiment, 51C3C0 Aircraft), so
/// the restore adapter decodes it (aa-mind saved_load); this keeps exactly the
/// consumed bytes and the base-Load Updatable record (50BCD0).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MindPayload {
    pub updatable: Option<UpdatablePayload>,
    pub bytes: Vec<u8>,
}
/// Saved Transport (slot 24, 58C810): no length prefix and no Updatable
/// record; the bytes are exactly what the native Load consumed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportPayload {
    pub bytes: Vec<u8>,
}
/// Saved Organization (slot 7, 57D130): its embedded member records are
/// loaded during the Load, so it is never decoded ahead; these are the bytes
/// the Load consumed (Updatable record included and registered by the Load).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationPayload {
    pub bytes: Vec<u8>,
}
/// HQ574040 owns its Updatable registration while reading these exact bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqPayload { pub bytes: Vec<u8> }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedComponent {
    Hq(HqPayload),
    Mind(MindPayload),
    Transport(TransportPayload),
    Organization(OrganizationPayload),
    Body(BodyPayload),
    Character(CharacterPayload),
    Construction(ConstructionPayload),
    Performer(PerformerPayload),
    Mover(MoverPayload),
    Denizen(DenizenPayload),
    Supply(SupplyPayload),
    Recruit(RecruitPayload),
    Defender(DefenderPayload),
    Research(ResearchPayload),
    /// 5751F0: Updatable, version 0, word vector, +1C, +20.
    Lair {
        updatable: UpdatablePayload,
        version: u32,
        words: CountedList<u32>,
        word_1c: u32,
        word_20: u32,
    },
    /// 564CE0: Updatable, version (max 1), version 1 word list (4D1AF0).
    City {
        updatable: UpdatablePayload,
        version: u32,
        words: Option<CountedList<u32>>,
    },
    Structure(StructurePayload),
    Properties(PropertiesPayload),
    UpdatableOnly {
        updatable: UpdatablePayload,
        version: u32,
    },
    Experience {
        version: u32,
        word_08: u32,
    },
    Element {
        version: u32,
        updatable: Option<UpdatablePayload>,
        legacy_byte: Option<u8>,
        history_count: Option<u32>,
    },
    MobileDeployment {
        updatable: UpdatablePayload,
        version: u32,
        word_10: u32,
        word_14: Option<u32>,
        /// Version 2: a temporary ResourceVector (4CD440) loaded by 4CD3F0
        /// (version 0, signed count, values) and destroyed (4CCF10).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        discarded_resources: Option<Vec<u32>>,
    },
}
impl SavedComponent {
    pub fn updatable(&self) -> Option<&UpdatablePayload> {
        match self {
            Self::Body(v) => v.updatable.as_ref(),
            Self::Character(v) => Some(&v.updatable),
            Self::Construction(v) => Some(&v.updatable),
            Self::Performer(v) => Some(&v.updatable),
            Self::Mover(v) => Some(&v.updatable),
            Self::Denizen(v) => Some(&v.updatable),
            Self::Supply(v) => v.updatable.as_ref(),
            Self::Recruit(v) => Some(&v.updatable),
            Self::Defender(v) => Some(&v.updatable),
            Self::Research(v) => Some(&v.updatable),
            Self::Lair { updatable, .. } | Self::City { updatable, .. } => Some(updatable),
            Self::Structure(v) => v.updatable.as_ref(),
            Self::Properties(v) => Some(&v.updatable),
            Self::UpdatableOnly { updatable, .. } | Self::MobileDeployment { updatable, .. } => {
                Some(updatable)
            }
            Self::Element { updatable, .. } => updatable.as_ref(),
            Self::Mind(v) => v.updatable.as_ref(),
            Self::Transport(_) | Self::Organization(_) | Self::Hq(_) => None,
            Self::Experience { .. } => None,
        }
    }
    pub fn decode_prefix(slot: u8, bytes: &[u8]) -> Result<(Self, usize)> {
        let mut r = Reader::new(bytes);
        let value = match slot {
            30 => {
                let consumed = crate::saved_hq::decode_prefix(bytes)?;
                r.take(consumed)?;
                Self::Hq(HqPayload { bytes: bytes[..consumed].to_vec() })
            }
            0 => Self::Body(BodyPayload::read(&mut r)?),
            2 => Self::Character(CharacterPayload::read(&mut r)?),
            15 => Self::Construction(ConstructionPayload::read(&mut r)?),
            5 => Self::Performer(PerformerPayload::read(&mut r)?),
            10 => Self::Denizen(DenizenPayload::read(&mut r)?),
            11 => Self::Supply(SupplyPayload::read(&mut r)?),
            16 => Self::Recruit(RecruitPayload::read(&mut r)?),
            20 => Self::Defender(DefenderPayload::read(&mut r)?),
            25 => {
                let updatable = updatable(&mut r)?;
                let version = r.version(0)?;
                let words = counted(&mut r, 4, |r| r.u32())?;
                Self::Lair {
                    updatable,
                    version,
                    words,
                    word_1c: r.u32()?,
                    word_20: r.u32()?,
                }
            }
            29 => {
                let updatable = updatable(&mut r)?;
                let version = r.version(1)?;
                let words = if version != 0 {
                    Some(counted(&mut r, 4, |r| r.u32())?)
                } else {
                    None
                };
                Self::City {
                    updatable,
                    version,
                    words,
                }
            }
            21 => {
                let updatable = updatable(&mut r)?;
                let version = r.version(0)?;
                let references = counted(&mut r, 2, |r| r.string())?;
                Self::Research(ResearchPayload {
                    updatable,
                    version,
                    references,
                })
            }
            13 => Self::Structure(StructurePayload::read(&mut r)?),
            3 => Self::Mover(MoverPayload::read(&mut r)?),
            6 => {
                let (v, n) = PropertiesPayload::decode_prefix(bytes)?;
                r.take(n)?;
                Self::Properties(v)
            }
            9 | 18 | 22 | 28 => Self::UpdatableOnly {
                updatable: updatable(&mut r)?,
                version: r.version(0)?,
            },
            17 => Self::Experience {
                version: r.version(0)?,
                word_08: r.u32()?,
            },
            8 => {
                let version = r.version(3)?;
                let updatable = if version > 0 {
                    Some(updatable(&mut r)?)
                } else {
                    None
                };
                let legacy_byte = if version < 2 {
                    Some(r.take(1)?[0])
                } else {
                    None
                };
                let history_count = if version >= 3 {
                    let count = r.u32()?;
                    if count as i32 > 0 {
                        return Err(Error::Format(
                            "Element damage-history payload remains unported".into(),
                        ));
                    }
                    Some(count)
                } else {
                    None
                };
                Self::Element {
                    version,
                    updatable,
                    legacy_byte,
                    history_count,
                }
            }
            19 => {
                let updatable = updatable(&mut r)?;
                let version = r.version(3)?;
                let word_10 = r.u32()?;
                let word_14 = if version > 0 { Some(r.u32()?) } else { None };
                let discarded_resources = if version == 2 {
                    r.version(0)?;
                    let count = r.u32()? as i32;
                    if count < 0 {
                        return Err(Error::Format(format!(
                            "negative ResourceVector count {count} (4CD0F0 stores it as the size)"
                        )));
                    }
                    let mut values = Vec::new();
                    for _ in 0..count {
                        values.push(r.u32()?);
                    }
                    Some(values)
                } else {
                    None
                };
                Self::MobileDeployment {
                    updatable,
                    version,
                    word_10,
                    word_14,
                    discarded_resources,
                }
            }
            4 => {
                return Err(Error::Format(
                    "saved Mind requires the concrete Mind kind decoder (restore adapter)".into(),
                ));
            }
            _ => {
                return Err(Error::Format(format!(
                    "unported saved component slot {slot}"
                )));
            }
        };
        Ok((value, r.at))
    }
}

impl BodyPayload {
    /// Apply after the native Body constructor has resolved maximum/defense.
    /// Serialized updatable identity binding is a separate world-table step.
    pub fn apply(
        &self,
        state: &mut aa_actors::ground_states::BodyState,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        if self.updatable.is_some() {
            state.header.scheduled_head = None;
        }
        if let Some(bits) = self.dying_started_bits {
            // Original FUCOM/test-AH44 changes either signed zero to -1;
            // other bit patterns, including unordered values, remain untouched.
            state.body.dying_started.0 = if bits & 0x7fff_ffff == 0 {
                0xbf80_0000
            } else {
                bits
            };
        }
        state.body.health.0 = aa_numeric::Binary::from_f32_bits(self.health_multiplier_bits)
            .and_then(|v| {
                v.mul(
                    &aa_numeric::Binary::from_f32_bits(state.body.maximum.0)?,
                    pc,
                )
            })
            .and_then(|v| v.store_f32())
            .map_err(|e| Error::Format(e.to_string()))?;
        if let Some(bits) = self.sink_duration_bits {
            state.body.sink_duration.0 = bits;
        }
        if let Some(bits) = self.last_heal_time_bits {
            state.body.last_heal_time.0 = bits;
        }
        Ok(())
    }
}
impl MoverPayload {
    /// Version8 requires the actual null-context speed getter result. The
    /// current-component predicate is native4DB0F0 pointer identity; a freshly
    /// constructed actor has no current component during this load phase.
    pub fn apply(
        &self,
        state: &mut aa_actors::ground_states::MoverState,
        resolved_speed_bits: Option<u32>,
        is_current_component: bool,
        owner_flags: &mut u32,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        state.header.scheduled_head = None;
        for (i, value) in self.vector_10.into_iter().enumerate() {
            state.words.insert(0x10 + i as u32 * 4, value);
        }
        state.words.insert(0x20, self.word_20);
        state.words.insert(0x24, self.word_24);
        if let Some(multiplier) = self.speed_multiplier_bits {
            let speed = resolved_speed_bits.ok_or_else(|| {
                Error::Format("Mover v8 requires resolved null-context speed".into())
            })?;
            state.speed.0 = aa_numeric::Binary::from_f32_bits(multiplier)
                .and_then(|v| v.mul(&aa_numeric::Binary::from_f32_bits(speed)?, pc))
                .and_then(|v| v.store_f32())
                .map_err(|e| Error::Format(e.to_string()))?;
        }
        for (offset, vector) in [
            (0x38, self.vector_38),
            (0x60, self.vector_60),
            (0x70, self.vector_70),
            (0x7c, self.vector_7c),
        ] {
            if let Some(vector) = vector {
                for (i, value) in vector.into_iter().enumerate() {
                    state.words.insert(offset + i as u32 * 4, value);
                }
            }
        }
        for (offset, value) in [
            (0x44, Some(self.word_44)),
            (0x48, Some(self.word_48)),
            (0x50, self.word_50),
            (0x58, self.word_58),
        ] {
            if let Some(value) = value {
                state.words.insert(offset, value);
            }
        }
        for (offset, value) in [
            (0x4c, self.byte_4c),
            (0x54, self.byte_54),
            (0x56, self.byte_56),
            (0x5c, self.byte_5c),
            (0x6c, self.byte_6c),
        ] {
            if let Some(value) = value {
                state.bytes.insert(offset, value);
            }
        }
        if state.bytes.get(&0x56).copied().unwrap_or(0) != 0 {
            *owner_flags |= 0x1000;
        }
        state.words.insert(
            0x1c,
            if is_current_component {
                self.word_20
            } else {
                0
            },
        );
        Ok(())
    }
}
