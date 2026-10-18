use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io::Read};

pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub sha256: String,
    pub bytes: u64,
}
impl SourceIdentity {
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            sha256: sha256(bytes),
            bytes: bytes.len() as u64,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkDirectory {
    pub name: String,
    pub directory_ordinal: usize,
    pub packed_offset: u64,
    pub packed_bytes: u64,
    pub source_timestamp: u64,
    pub decoded_bytes: u64,
}
pub struct MapArchive {
    source: SourceIdentity,
    bytes: Vec<u8>,
    directory: Vec<ChunkDirectory>,
}
#[derive(Clone, Debug)]
pub struct Chunk {
    pub map_source: SourceIdentity,
    pub directory: ChunkDirectory,
    pub payload_source: SourceIdentity,
    pub encoding: Encoding,
    pub bytes: Vec<u8>,
}
impl Chunk {
    pub fn verify(&self) -> Result<()> {
        if self.payload_source != SourceIdentity::of(&self.bytes)
            || self.directory.decoded_bytes != self.bytes.len() as u64
        {
            return Err(Error::Format(
                "chunk payload identity/length mismatch".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Encoding {
    Raw,
    Zlib,
}

fn bad(message: &str) -> Error {
    Error::Format(message.into())
}
fn le64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().unwrap())
}
impl MapArchive {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < 12
            || &bytes[..4] != b"TGCK"
            || u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 2
        {
            return Err(bad("expected TGCK version 2"));
        }
        let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
        let directory_bytes = count
            .checked_mul(96)
            .ok_or_else(|| bad("directory overflow"))?;
        let start = bytes
            .len()
            .checked_sub(directory_bytes)
            .filter(|s| *s >= 12)
            .ok_or_else(|| bad("directory exceeds source"))?;
        let mut directory = Vec::new();
        directory
            .try_reserve(count)
            .map_err(|_| Error::Allocation)?;
        let mut names = BTreeSet::new();
        let mut extents = Vec::new();
        extents.try_reserve(count).map_err(|_| Error::Allocation)?;
        for (ordinal, entry) in bytes[start..].chunks_exact(96).enumerate() {
            let units: Vec<u16> = entry[..64]
                .chunks_exact(2)
                .map(|u| u16::from_le_bytes([u[0], u[1]]))
                .collect();
            let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
            if units[end..].iter().any(|&u| u != 0) {
                return Err(bad("nonzero data after chunk-name terminator"));
            }
            let name =
                String::from_utf16(&units[..end]).map_err(|_| bad("invalid UTF16 chunk name"))?;
            if name.is_empty() || !names.insert(name.clone()) {
                return Err(bad("empty/duplicate chunk name"));
            }
            let row = ChunkDirectory {
                name,
                directory_ordinal: ordinal,
                packed_offset: le64(&entry[64..72]),
                packed_bytes: le64(&entry[72..80]),
                source_timestamp: le64(&entry[80..88]),
                decoded_bytes: le64(&entry[88..96]),
            };
            let end = row
                .packed_offset
                .checked_add(row.packed_bytes)
                .ok_or_else(|| bad("chunk extent overflow"))?;
            if row.packed_offset < 12 || end > start as u64 {
                return Err(bad("chunk extent outside payload"));
            }
            extents.push((row.packed_offset, end));
            directory.push(row);
        }
        extents.sort_unstable();
        if extents.windows(2).any(|w| w[0].1 > w[1].0) {
            return Err(bad("overlapping chunk extents"));
        }
        Ok(Self {
            source: SourceIdentity::of(&bytes),
            bytes,
            directory,
        })
    }
    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }
    pub fn directory(&self) -> &[ChunkDirectory] {
        &self.directory
    }
    /// Caller budget is an input-decoding allocation guard, not a game RAM cap.
    pub fn decode_chunk(&self, name: &str, max_decoded_bytes: u64) -> Result<Chunk> {
        let row = self
            .directory
            .iter()
            .find(|r| r.name == name)
            .ok_or_else(|| Error::Format(format!("missing chunk {name}")))?;
        if row.decoded_bytes > max_decoded_bytes {
            return Err(Error::Budget {
                needed: row.decoded_bytes,
                budget: max_decoded_bytes,
            });
        }
        let len = usize::try_from(row.decoded_bytes)
            .map_err(|_| bad("decoded chunk exceeds address space"))?;
        let packed = &self.bytes
            [row.packed_offset as usize..(row.packed_offset + row.packed_bytes) as usize];
        let mut decoder = flate2::read::ZlibDecoder::new(packed);
        let mut payload = Vec::new();
        payload
            .try_reserve_exact(len)
            .map_err(|_| Error::Allocation)?;
        let decoded = decoder
            .by_ref()
            .take(
                row.decoded_bytes
                    .checked_add(1)
                    .ok_or_else(|| bad("decoded length overflow"))?,
            )
            .read_to_end(&mut payload);
        let encoding = match decoded {
            Ok(_) if payload.len() == len && decoder.total_in() == row.packed_bytes => {
                Encoding::Zlib
            }
            Err(_) if packed.len() == len => {
                payload.clear();
                payload.extend_from_slice(packed);
                Encoding::Raw
            }
            _ => return Err(bad("chunk decoded length/stream extent mismatch")),
        };
        Ok(Chunk {
            map_source: self.source.clone(),
            directory: row.clone(),
            payload_source: SourceIdentity::of(&payload),
            encoding,
            bytes: payload,
        })
    }
}
