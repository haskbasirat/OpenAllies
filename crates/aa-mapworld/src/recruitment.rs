//! Native kingdom recruit-list wire records (4E6C20/4E79F0/4E7A30).
//! IDs remain UTF16 references. Registry resolution, duplicate suppression and
//! publication happen after decoding; no item is silently discarded here.
use crate::{Error, Result, inventory::Reader};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Base {
    pub version: u32,
    pub property_id: Vec<u16>,
    pub enabled: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Single {
    pub version: u32,
    pub word_0c: u32,
    pub word_10: u32,
    pub word_14: u32,
    pub word_18: u32,
    pub word_1c: u32,
    /// Version0 consumes this byte but does not copy it into the runtime object.
    pub legacy_v0_byte: Option<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemState {
    Single(Single),
    Group { version: u32, word_0c: u32 },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub base: Base,
    pub state: ItemState,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recruitment {
    pub version: u32,
    /// Original loop is signed: negative count words consume zero items.
    pub serialized_count: u32,
    pub items: Vec<Item>,
    pub word_24: Option<u32>,
}
impl Recruitment {
    pub fn decode_prefix(bytes: &[u8]) -> Result<(Self, usize)> {
        let mut r = Reader::new(bytes);
        let value = Self::read(&mut r)?;
        Ok((value, r.at))
    }
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self> {
        let version = r.version(1)?;
        let serialized_count = r.u32()?;
        let count = (serialized_count as i32).max(0) as usize;
        if count
            .checked_mul(19)
            .is_none_or(|bytes| bytes > r.bytes.len() - r.at)
        {
            return Err(Error::Format(
                "recruit count exceeds remaining serialized extent".into(),
            ));
        }
        let mut items = Vec::new();
        items.try_reserve(count).map_err(|_| Error::Allocation)?;
        for _ in 0..count {
            let kind = r.u32()?;
            if kind > 1 {
                return Err(Error::Format(format!("unknown recruit item kind {kind}")));
            }
            let base = Base {
                version: r.version(0)?,
                property_id: r.string()?,
                enabled: r.take(1)?[0],
            };
            let state = if kind == 1 {
                ItemState::Group {
                    version: r.version(0)?,
                    word_0c: r.u32()?,
                }
            } else {
                let version = r.version(2)?;
                let word_0c = r.u32()?;
                let word_10 = if version > 0 { r.u32()? } else { 0 };
                let word_14 = r.u32()?;
                let legacy_v0_byte = if version == 0 {
                    Some(r.take(1)?[0])
                } else {
                    None
                };
                let word_1c = r.u32()?;
                let word_18 = if version >= 2 { r.u32()? } else { 0 };
                ItemState::Single(Single {
                    version,
                    word_0c,
                    word_10,
                    word_14,
                    word_18,
                    word_1c,
                    legacy_v0_byte,
                })
            };
            items.push(Item { base, state });
        }
        let word_24 = if version > 0 { Some(r.u32()?) } else { None };
        Ok(Self {
            version,
            serialized_count,
            items,
            word_24,
        })
    }
}
