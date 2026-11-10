//! The map file writer: the chunk file ("TGCK" version 2) the original's map
//! save writes, and the exact inverse of `aa_mapworld::MapArchive` (the
//! reader of `.RMP` maps).
//!
//! | native | role |
//! |---|---|
//! | 859690 / 859380 / 859090 | create: header `TGCK`, version 2, count 0 |
//! | 859150 | open a chunk: empty name refused, `wcsncpy(name, 0x20)`, flag bit 0 and a level-6 zlib 1.1.4 compressor (8B72E0) when compressed; an open compressed chunk is finished first |
//! | 858E40 / 858DB0 | write: through the compressor, or straight to the file; stored and logical sizes |
//! | 859330 / 859090 | close: count in the header, one 0x60-byte record per chunk after the last chunk's data |
//!
//! Layout (little endian): `{ 'TGCK', u32 2, u32 count }`, the chunk data in
//! order (the first at 0x0C, each next one right after the previous one),
//! then the records: name (32 UTF-16 units, zero padded), `u64` offset,
//! `u64` stored size, `u64` flags, `u64` logical size.
//!
//! The flags word: bit 0 is the compression flag. The original never sets
//! bits 1..63: 859090 builds every record in one 0x60-byte stack buffer
//! (8582B0 clears bit 0 and sets it only for a compressed chunk, and never
//! copies the descriptor's +0x50), so bits 1..63 are stale stack bytes, one
//! value for every record of a file. Every shipped map has such bits (the
//! reader names the word `source_timestamp`). The writer takes them as data
//! ([`MapFileWriter::open_chunk`] `stale_flag_bits`) so a map re-encodes to
//! its own bytes; a new map has no such history, and 0 is the value to use.
use crate::deflate::Deflater;

/// `'TGCK'`.
pub const MAGIC: [u8; 4] = *b"TGCK";
/// The version 859090 writes.
pub const VERSION: u32 = 2;
/// Where the first chunk starts.
pub const FIRST_CHUNK_OFFSET: u64 = 0x0C;
/// Size of a directory record.
pub const RECORD_BYTES: usize = 0x60;
/// Name field in UTF-16 units (`wcsncpy(.., 0x20)`).
pub const NAME_UNITS: usize = 32;
/// The zlib level of the chunk compressor (8B72E0).
pub const ZLIB_LEVEL: u8 = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// 859150 throws "chunk name cannot be empty".
    EmptyName,
    /// A name with a zero unit, or longer than the 32-unit field
    /// (`wcsncpy` would cut it without a terminator; not reproduced).
    UnsupportedName(String),
    /// 858E40 with no open chunk.
    NoChunk,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::EmptyName => f.write_str("chunk name cannot be empty"),
            WriteError::UnsupportedName(n) => write!(f, "chunk name {n:?} does not fit the 32-unit field"),
            WriteError::NoChunk => f.write_str("writing with no chunk"),
        }
    }
}
impl std::error::Error for WriteError {}

#[derive(Clone, Debug)]
struct Record {
    name: [u16; NAME_UNITS],
    offset: u64,
    stored: u64,
    flags: u64,
    logical: u64,
}

/// A map file being written (859690 in write mode ... 859330).
pub struct MapFileWriter {
    data: Vec<u8>,
    records: Vec<Record>,
    open: Option<usize>,
    compressor: Option<Deflater>,
}

impl Default for MapFileWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl MapFileWriter {
    /// The header with a chunk count of 0.
    pub fn new() -> Self {
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&MAGIC);
        data.extend_from_slice(&VERSION.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        MapFileWriter { data, records: Vec::new(), open: None, compressor: None }
    }

    /// 859150. `stale_flag_bits` are flag bits 1..63 of the record (bit 0
    /// is ignored there and set from `compressed`).
    pub fn open_chunk(&mut self, name: &str, compressed: bool, stale_flag_bits: u64) -> Result<(), WriteError> {
        self.finish_chunk();
        self.open = None;
        if name.is_empty() {
            return Err(WriteError::EmptyName);
        }
        let units: Vec<u16> = name.encode_utf16().collect();
        if units.len() > NAME_UNITS || units.contains(&0) {
            return Err(WriteError::UnsupportedName(name.to_owned()));
        }
        let mut field = [0u16; NAME_UNITS];
        field[..units.len()].copy_from_slice(&units);
        let offset = self.records.last().map_or(FIRST_CHUNK_OFFSET, |r| r.offset + r.stored);
        self.records.push(Record {
            name: field,
            offset,
            stored: 0,
            flags: (stale_flag_bits & !1) | u64::from(compressed),
            logical: 0,
        });
        self.open = Some(self.records.len() - 1);
        if compressed {
            self.compressor = Some(Deflater::new(ZLIB_LEVEL).expect("level 6 is ported"));
        }
        Ok(())
    }

    /// 858E40: append to the open chunk.
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        let i = self.open.ok_or(WriteError::NoChunk)?;
        let r = &mut self.records[i];
        r.logical += bytes.len() as u64;
        match self.compressor.as_mut() {
            Some(z) => z.write(bytes),
            None => {
                self.data.extend_from_slice(bytes);
                r.stored += bytes.len() as u64;
            }
        }
        Ok(())
    }

    /// The compressed stream of an open compressed chunk goes to the file.
    fn finish_chunk(&mut self) {
        if let Some(z) = self.compressor.take() {
            let bytes = z.finish();
            let i = self.open.expect("a compressor belongs to the open chunk");
            self.records[i].stored = bytes.len() as u64;
            self.data.extend_from_slice(&bytes);
        }
    }

    /// 859330 / 859090: the count and the directory. Returns the file.
    pub fn close(mut self) -> Vec<u8> {
        self.finish_chunk();
        let count = self.records.len() as u32;
        self.data[8..12].copy_from_slice(&count.to_le_bytes());
        self.data.reserve(self.records.len() * RECORD_BYTES);
        for r in &self.records {
            for u in r.name {
                self.data.extend_from_slice(&u.to_le_bytes());
            }
            self.data.extend_from_slice(&r.offset.to_le_bytes());
            self.data.extend_from_slice(&r.stored.to_le_bytes());
            self.data.extend_from_slice(&r.flags.to_le_bytes());
            self.data.extend_from_slice(&r.logical.to_le_bytes());
        }
        self.data
    }
}

/// One chunk of a map to write, in file order.
#[derive(Clone, Copy, Debug)]
pub struct ChunkSource<'a> {
    pub name: &'a str,
    /// The record's flags word: bit 0 compresses the chunk, bits 1..63 are
    /// the stale bits described in the module notes.
    pub flags: u64,
    /// The chunk's logical (decoded) bytes.
    pub payload: &'a [u8],
}

/// A whole map file from its chunks, each written in one piece.
pub fn write_map(chunks: &[ChunkSource<'_>]) -> Result<Vec<u8>, WriteError> {
    let mut w = MapFileWriter::new();
    for c in chunks {
        w.open_chunk(c.name, c.flags & 1 != 0, c.flags)?;
        w.write(c.payload)?;
    }
    Ok(w.close())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_is_the_bare_header() {
        assert_eq!(MapFileWriter::new().close(), b"TGCK\x02\0\0\0\0\0\0\0");
    }

    #[test]
    fn raw_and_compressed_chunks_follow_each_other() {
        let bytes = write_map(&[
            ChunkSource { name: "HEADER", flags: 0, payload: b"abc" },
            ChunkSource { name: "Z", flags: 0x1234_0001, payload: b"hello" },
        ])
        .unwrap();
        assert_eq!(&bytes[..12], b"TGCK\x02\0\0\0\x02\0\0\0");
        assert_eq!(&bytes[12..15], b"abc");
        let z = crate::deflate::compress(b"hello", 6).unwrap();
        assert_eq!(&bytes[15..15 + z.len()], &z[..]);
        let dir = &bytes[15 + z.len()..];
        assert_eq!(dir.len(), 2 * RECORD_BYTES);
        let q = |r: &[u8], o: usize| u64::from_le_bytes(r[o..o + 8].try_into().unwrap());
        let (a, b) = (&dir[..RECORD_BYTES], &dir[RECORD_BYTES..]);
        assert_eq!((q(a, 0x40), q(a, 0x48), q(a, 0x50), q(a, 0x58)), (12, 3, 0, 3));
        assert_eq!((q(b, 0x40), q(b, 0x48), q(b, 0x50), q(b, 0x58)), (15, z.len() as u64, 0x1234_0001, 5));
        assert_eq!(&a[..12], "HEADER".encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>().as_slice());
        assert!(a[12..64].iter().all(|&x| x == 0));
    }

    #[test]
    fn names_are_checked_like_the_original() {
        let mut w = MapFileWriter::new();
        assert_eq!(w.open_chunk("", false, 0), Err(WriteError::EmptyName));
        assert!(w.open_chunk(&"x".repeat(33), false, 0).is_err());
        assert!(w.open_chunk(&"x".repeat(32), false, 0).is_ok());
        assert_eq!(MapFileWriter::new().write(b"a"), Err(WriteError::NoChunk));
    }
}
