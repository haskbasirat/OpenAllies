//! Bounded wire decoding for GActor::Load (4D8CF0) and the Properties component
//! (581AE0). This preserves serialized fields, not post-load runtime side effects.
use crate::inventory::{ActorPrefix, ByteSpan, Reader};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdatablePayload {
    pub version: u32,
    /// Signed native table ID. Negative values do not register an updatable.
    pub serialized_id: Option<i32>,
}
impl UpdatablePayload {
    pub fn decode_prefix(bytes: &[u8]) -> Result<(Self, usize)> {
        let mut r = Reader::new(bytes);
        let version = r.version(1)?;
        let value = Self {
            version,
            serialized_id: if version >= 1 {
                Some(r.u32()? as i32)
            } else {
                None
            },
        };
        Ok((value, r.at))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertiesPayload {
    pub updatable: UpdatablePayload,
    pub version: u32,
    /// Only the native -1 (no property object) marker is currently supported.
    /// Non-sentinel property classes need their own virtual Load wire codecs.
    pub property_classes: Vec<u32>,
}
impl PropertiesPayload {
    pub fn decode_prefix(bytes: &[u8]) -> Result<(Self, usize)> {
        let (updatable, consumed) = UpdatablePayload::decode_prefix(bytes)?;
        let mut r = Reader::new(bytes);
        r.take(consumed)?;
        let version = r.version(1)?;
        let count = r.u32()? as usize;
        // Every iteration reads a class word, before any variable class payload.
        if count > (r.bytes.len() - r.at) / 4 {
            return Err(Error::Format(
                "Properties count exceeds remaining class words".into(),
            ));
        }
        let mut property_classes = Vec::new();
        property_classes
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        for _ in 0..count {
            let class = r.u32()?;
            if class != u32::MAX {
                return Err(Error::Format(format!(
                    "unsupported Properties object class {class:#x} at byte {}",
                    r.at - 4
                )));
            }
            property_classes.push(class);
        }
        Ok((
            Self {
                updatable,
                version,
                property_classes,
            },
            r.at,
        ))
    }
}

/// Native list loops test the serialized count as a signed integer. Preserve
/// negative counts (which consume no entries), rather than allocating from them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountedList<T> {
    pub serialized_count: u32,
    pub values: Vec<T>,
}
fn list<const N: usize>(r: &mut Reader<'_>) -> Result<CountedList<[u32; N]>> {
    let serialized_count = r.u32()?;
    let count = (serialized_count as i32).max(0) as usize;
    if count > (r.bytes.len() - r.at) / (N * 4) {
        return Err(Error::Format(
            "Actor list count exceeds remaining entries".into(),
        ));
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    for _ in 0..count {
        let mut entry = [0; N];
        for word in &mut entry {
            *word = r.u32()?;
        }
        values.push(entry);
    }
    Ok(CountedList {
        serialized_count,
        values,
    })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorLists {
    pub list_118: CountedList<[u32; 1]>,
    pub list_120: CountedList<[u32; 1]>,
    pub list_128: CountedList<[u32; 1]>,
    pub list_130: CountedList<[u32; 3]>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentString {
    /// Native treats every nonzero byte as present; keep the original byte.
    pub presence: u8,
    pub utf16: Option<Vec<u16>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentWord {
    pub presence: u8,
    pub value: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorTail {
    pub saved_components_mask: u32,
    pub default_components_mask: u32,
    pub properties: Option<PropertiesPayload>,
    /// Byte span relative to the start of this tail, if Properties was saved.
    pub properties_span: Option<ByteSpan>,
    pub current_component: i32,
    pub word_158: Option<u32>,
    /// Versions 3..11 read this word then force the runtime field to -1.
    pub legacy_word_15c: Option<u32>,
    /// Resolved by the original owner's 4E2450 lookup after reading this string.
    pub word_160_source: Option<PresentString>,
    pub word_164: Option<PresentWord>,
    /// Mode-zero map stream stores the localization key only.
    pub localized_name_key_utf16: Option<Vec<u16>>,
    pub word_168: Option<u32>,
    pub word_16c: Option<u32>,
    pub word_170: Option<u32>,
    pub lists: Option<ActorLists>,
    pub word_174: Option<u32>,
}
impl ActorTail {
    /// Existing bounded Properties-only decoder. Full restoration supplies
    /// additional independently verified component codecs via the callback API.
    pub fn decode_prefix(
        bytes: &[u8],
        actor_version: u32,
        localization_mode: u32,
    ) -> Result<(Self, usize)> {
        Self::decode_prefix_with_components(
            bytes,
            actor_version,
            localization_mode,
            |slot, _, bytes| {
                if slot != 6 {
                    return Err(Error::Format(format!(
                        "unsupported saved actor components: slot {slot}"
                    )));
                }
                let (properties, consumed) = PropertiesPayload::decode_prefix(bytes)?;
                Ok((consumed, Some(properties)))
            },
        )
    }
    /// Original4D919C dispatch: Construction first, ordinary slots in numeric order, Mind last. Callback offset is relative to this
    /// Actor tail. Previously completed callbacks remain observable if a later
    /// codec fails; caller can retain its staged component states. This method
    /// never skips an unknown unbounded component to search for a later field.
    pub fn decode_prefix_with_components(
        bytes: &[u8],
        actor_version: u32,
        localization_mode: u32,
        mut component: impl FnMut(u8, usize, &[u8]) -> Result<(usize, Option<PropertiesPayload>)>,
    ) -> Result<(Self, usize)> {
        Self::decode_prefix_with_stages(
            bytes,
            actor_version,
            localization_mode,
            |slot, saved, _, offset, bytes| {
                if saved {
                    component(slot, offset, bytes)
                } else {
                    Ok((0, None))
                }
            },
        )
    }
    /// Visits all31 native load stages, including absent serialized components.
    /// This allows the actual4D8290 default-constructor branch at its original
    /// position. The callback gets the saved bit and original default mask;
    /// absent stages must consume zero bytes. This API never decides whether a
    /// missing component should exist from authored or saved masks alone.
    pub fn decode_prefix_with_stages(
        bytes: &[u8],
        actor_version: u32,
        localization_mode: u32,
        mut component: impl FnMut(
            u8,
            bool,
            u32,
            usize,
            &[u8],
        ) -> Result<(usize, Option<PropertiesPayload>)>,
    ) -> Result<(Self, usize)> {
        if !(1..=14).contains(&actor_version) {
            return Err(Error::Format(format!(
                "unsupported actor tail version {actor_version}; supported 1..14"
            )));
        }
        if localization_mode != 0 {
            return Err(Error::Format(format!(
                "unsupported actor localization mode {localization_mode}"
            )));
        }
        let mut r = Reader::new(bytes);
        let default_components_mask = r.u32()?;
        let saved_components_mask = r.u32()?;
        if saved_components_mask & 0x8000_0000 != 0 {
            return Err(Error::Format(
                "unsupported saved component bit31 outside native31-slot table".into(),
            ));
        }
        let mut properties = None;
        let mut properties_span = None;
        for slot in std::iter::once(15)
            .chain((0..31).filter(|slot| *slot != 15 && *slot != 4))
            .chain(std::iter::once(4))
        {
            let saved = saved_components_mask & (1 << slot) != 0;
            let start = r.at;
            let (consumed, payload) =
                component(slot, saved, default_components_mask, start, &bytes[start..])?;
            if !saved && consumed != 0 {
                return Err(Error::Format(format!(
                    "absent component stage {slot} consumed source bytes"
                )));
            }
            r.take(consumed)?;
            if slot == 6 && saved {
                properties = payload;
                properties_span = Some(ByteSpan {
                    offset: start,
                    length: consumed,
                });
            }
        }
        let current_component = r.u32()? as i32;
        let word_158 = if actor_version >= 2 {
            Some(r.u32()?)
        } else {
            None
        };
        let legacy_word_15c = if (3..12).contains(&actor_version) {
            Some(r.u32()?)
        } else {
            None
        };
        let word_160_source = if actor_version >= 4 {
            let presence = r.take(1)?[0];
            Some(PresentString {
                presence,
                utf16: if presence != 0 {
                    Some(r.string()?)
                } else {
                    None
                },
            })
        } else {
            None
        };
        let word_164 = if actor_version >= 5 {
            let presence = r.take(1)?[0];
            Some(PresentWord {
                presence,
                value: if presence != 0 { Some(r.u32()?) } else { None },
            })
        } else {
            None
        };
        let localized_name_key_utf16 = if actor_version >= 6 {
            Some(r.string()?)
        } else {
            None
        };
        let word_168 = if actor_version >= 8 {
            Some(r.u32()?)
        } else {
            None
        };
        let word_16c = if actor_version >= 8 {
            Some(r.u32()?)
        } else {
            None
        };
        let word_170 = if actor_version >= 10 {
            Some(r.u32()?)
        } else {
            None
        };
        let lists = if actor_version >= 11 {
            Some(ActorLists {
                list_118: list(&mut r)?,
                list_120: list(&mut r)?,
                list_128: list(&mut r)?,
                list_130: list(&mut r)?,
            })
        } else {
            None
        };
        let word_174 = if actor_version >= 13 {
            Some(r.u32()?)
        } else {
            None
        };
        Ok((
            Self {
                saved_components_mask,
                default_components_mask,
                properties,
                properties_span,
                current_component,
                word_158,
                legacy_word_15c,
                word_160_source,
                word_164,
                localized_name_key_utf16,
                word_168,
                word_16c,
                word_170,
                lists,
                word_174,
            },
            r.at,
        ))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorPayload {
    pub prefix: ActorPrefix,
    pub tail: ActorTail,
}
impl ActorPayload {
    /// Starts at the actor version word, after the complete GObject payload.
    pub fn decode_prefix(bytes: &[u8], localization_mode: u32) -> Result<(Self, usize)> {
        let (prefix, n) = ActorPrefix::decode_prefix(bytes)?;
        let (tail, m) = ActorTail::decode_prefix(&bytes[n..], prefix.version, localization_mode)?;
        Ok((Self { prefix, tail }, n + m))
    }
}
