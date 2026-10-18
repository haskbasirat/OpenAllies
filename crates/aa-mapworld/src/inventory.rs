use crate::{Chunk, Error, MapArchive, Result, SourceIdentity};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteSpan {
    pub offset: usize,
    pub length: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThingState {
    pub version: u32,
    pub name_utf16: Vec<u16>,
    pub word_1c: u32,
    pub position_bits: [u32; 3],
    pub facing_bits: [u32; 3],
    pub serialized_flags: u32,
    pub legacy_version_one_word: Option<u32>,
}
impl ThingState {
    /// Serialized input prefix, before the native runtime load side effects.
    pub fn decode_prefix(bytes: &[u8]) -> Result<(Self, usize)> {
        let mut r = Reader::new(bytes);
        let version = r.version(2)?;
        let out = Self {
            version,
            name_utf16: r.string()?,
            word_1c: r.u32()?,
            position_bits: r.vector()?,
            facing_bits: r.vector()?,
            serialized_flags: r.u32()?,
            legacy_version_one_word: if version == 1 { Some(r.u32()?) } else { None },
        };
        Ok((out, r.at))
    }
    /// Original 4EF21D..4EF248. Definition flags are already evaluated by the
    /// caller; this does not interpret inspector text or default missing flags.
    pub fn runtime_flags(&self, existing_flags: u32, definition_flags: u32) -> u32 {
        (existing_flags & 0xfc00_00ff) | definition_flags | (self.serialized_flags & 0xff)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectState {
    pub version: u32,
    pub scale_multiplier_bits: u32,
    pub variant_index: Option<i32>,
    pub associated_object_id: Option<u32>,
    pub legacy_word: Option<u16>,
    pub nested_count: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorPrefix {
    pub version: u32,
    /// Native Load resolves this through the restored GTID table only when the
    /// definition-derived Thing flag bit23 is set; never assume it is a side index.
    pub owner_handle: u32,
    /// Serialized actor state, before per-state transient-bit clearing.
    pub state_low: u32,
    pub state_high: Option<u32>,
}
impl ActorPrefix {
    pub fn decode_prefix(bytes: &[u8]) -> Result<(Self, usize)> {
        let mut r = Reader::new(bytes);
        let version = r.version(14)?;
        let out = Self {
            version,
            owner_handle: r.u32()?,
            state_low: r.u32()?,
            state_high: if version >= 9 { Some(r.u32()?) } else { None },
        };
        Ok((out, r.at))
    }
    /// State immediately after the serialized read, BEFORE the later native
    /// transient-state clearing and component load. No complete actor implied.
    pub fn state_after_read(&self, prior: [u32; 2]) -> [u32; 2] {
        match self.state_high {
            Some(high) => [self.state_low, high],
            None => [prior[0] | self.state_low, prior[1]],
        }
    }
    pub fn owner_reference(&self, thing_flags: u32) -> Option<u32> {
        (thing_flags & (1 << 23) != 0).then_some(self.owner_handle)
    }
    pub fn upgraded_world_version(&self, prior: u32) -> u32 {
        if prior == 14 && self.version > 7 {
            15
        } else {
            prior
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub record_ordinal: usize,
    pub source_span: ByteSpan,
    pub thing_id_utf16: Vec<u16>,
    pub class_code: u32,
    /// Native serialized handle; not considered validated until IDMGR is read.
    pub serialized_id: u32,
    pub thing: ThingState,
    pub object: Option<ObjectState>,
    pub actor: Option<ActorPrefix>,
    pub opaque_tail_span: ByteSpan,
    pub opaque_tail: Vec<u8>,
}
#[derive(Clone, Debug, Serialize)]
pub struct UnsupportedRecord {
    pub record_ordinal: usize,
    pub source_span: ByteSpan,
    pub reason: String,
    pub raw: Vec<u8>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Inventory {
    pub map_source: SourceIdentity,
    pub data_source: SourceIdentity,
    pub index_source: SourceIdentity,
    pub data_header_word: u32,
    pub placements: Vec<Placement>,
    pub unsupported: Vec<UnsupportedRecord>,
}

pub(crate) struct Reader<'a> {
    pub bytes: &'a [u8],
    pub at: usize,
}
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| Error::Format(format!("record truncated at byte {}", self.at)))?;
        let b = &self.bytes[self.at..end];
        self.at = end;
        Ok(b)
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn string(&mut self) -> Result<Vec<u16>> {
        let count = self.u16()? as usize;
        Ok(self
            .take(count * 2)?
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect())
    }
    pub fn vector(&mut self) -> Result<[u32; 3]> {
        Ok([self.u32()?, self.u32()?, self.u32()?])
    }
    pub fn version(&mut self, max: u32) -> Result<u32> {
        let v = self.u32()?;
        if v > max {
            Err(Error::Format(format!(
                "unsupported version {v} (native max {max})"
            )))
        } else {
            Ok(v)
        }
    }
}
/// A record embedded in a component Load (4FC200 -> 4FAE80): the same wire
/// format with no index length, so `bytes` runs to the end of the enclosing
/// stream; spans are relative to the embedded record's first byte.
pub fn decode_embedded_record(bytes: &[u8], ordinal: usize) -> Result<Placement> {
    record(bytes, ordinal, 0)
}
fn record(bytes: &[u8], ordinal: usize, offset: usize) -> Result<Placement> {
    let mut r = Reader::new(bytes);
    let thing_id_utf16 = r.string()?;
    let class_code = r.u32()?;
    let serialized_id = r.u32()?;
    let (thing, consumed) = ThingState::decode_prefix(&bytes[r.at..])?;
    r.take(consumed)?;
    let object = if matches!(class_code, 2 | 3 | 6 | 7 | 9) {
        let version = r.version(5)?;
        Some(ObjectState {
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
        })
    } else if class_code == 1 {
        None
    } else {
        return Err(Error::Format(format!(
            "unsupported native class {class_code}"
        )));
    };
    let actor = if class_code == 3
        && object
            .as_ref()
            .is_some_and(|o| o.nested_count.unwrap_or(0) == 0)
    {
        let (prefix, consumed) = ActorPrefix::decode_prefix(&bytes[r.at..])?;
        r.take(consumed)?;
        Some(prefix)
    } else {
        None
    };
    Ok(Placement {
        record_ordinal: ordinal,
        source_span: ByteSpan {
            offset,
            length: bytes.len(),
        },
        thing_id_utf16,
        class_code,
        serialized_id,
        thing,
        object,
        actor,
        opaque_tail_span: ByteSpan {
            offset: offset + r.at,
            length: bytes.len() - r.at,
        },
        opaque_tail: bytes[r.at..].to_vec(),
    })
}
impl Inventory {
    pub fn decode(map: &MapArchive, max_chunk_bytes: u64) -> Result<Self> {
        Self::from_chunks(
            map.decode_chunk("DYNAMIC_THINGS", max_chunk_bytes)?,
            map.decode_chunk("DYNAMIC_THINGS_INDEX", max_chunk_bytes)?,
        )
    }
    pub fn from_chunks(data: Chunk, index: Chunk) -> Result<Self> {
        data.verify()?;
        index.verify()?;
        if data.map_source != index.map_source {
            return Err(Error::Format(
                "placement chunks belong to different map sources".into(),
            ));
        }
        if data.directory.name != "DYNAMIC_THINGS" || index.directory.name != "DYNAMIC_THINGS_INDEX"
        {
            return Err(Error::Format("incorrect placement chunk identities".into()));
        }
        let mut r = Reader::new(&index.bytes);
        r.version(0)?;
        let count = r.u32()? as usize;
        if count.checked_mul(4).and_then(|n| n.checked_add(8)) != Some(index.bytes.len()) {
            return Err(Error::Format(
                "placement index count/length mismatch".into(),
            ));
        }
        let mut d = Reader::new(&data.bytes);
        d.version(0)?;
        let data_header_word = d.u32()?;
        let mut inventory = Self {
            map_source: data.map_source,
            data_source: data.payload_source,
            index_source: index.payload_source,
            data_header_word,
            placements: Vec::new(),
            unsupported: Vec::new(),
        };
        inventory
            .placements
            .try_reserve(count)
            .map_err(|_| Error::Allocation)?;
        for ordinal in 0..count {
            let length = r.u32()? as usize;
            let offset = d.at;
            let bytes = d.take(length)?;
            match record(bytes, ordinal, offset) {
                Ok(p) => inventory.placements.push(p),
                Err(error) => inventory.unsupported.push(UnsupportedRecord {
                    record_ordinal: ordinal,
                    source_span: ByteSpan { offset, length },
                    reason: error.to_string(),
                    raw: bytes.to_vec(),
                }),
            }
        }
        if d.at != data.bytes.len() {
            return Err(Error::Format(
                "placement index does not cover data chunk".into(),
            ));
        }
        Ok(inventory)
    }
}
