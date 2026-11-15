//! The map file writer: the chunk file ("TGCK" version 2) the original's map
//! save writes, and the exact inverse of `aa_mapworld::MapArchive` (the
//! reader of `.RMP` maps).
//!
//! There is one chunk file writer in the workspace,
//! `aa_metagame::chunk_file::ChunkFileWriter` (859690 create, 859150 open a
//! chunk, 858E40 write, 859330 / 859090 close), which films, saved games and
//! maps share; its compressed chunks go through the one zlib 1.1.4 deflater,
//! `aa_formats::zlib_deflate` (8B72E0: level 6). This module adds only what
//! a map needs on top of it:
//!
//! * The flags word of a record: bit 0 is the compression flag. The original
//!   never sets bits 1..63: 859090 builds every record in one 0x60-byte stack
//!   buffer (8582B0 clears bit 0 and sets it only for a compressed chunk, and
//!   never copies the descriptor's +0x50), so bits 1..63 are stale stack
//!   bytes, one value for every record of a file. Every shipped map has such
//!   a value (the reader names the word `source_timestamp`). The shared
//!   writer writes 0 there; [`set_stale_flag_bits`] writes a file's value
//!   into every record after the close, so a shipped map re-encodes to its
//!   own bytes. A new map has no such history, and 0 is the value to use.
//! * Chunk names follow the shared writer: an empty name is the original's
//!   "chunk name cannot be empty"; a name with a zero unit, or of 32 units or
//!   more, is refused, because 859150's `wcsncpy(name, 0x20)` would store it
//!   unterminated (cut to 32 units) and the original then reads such a name
//!   on into the offset field. That case is not reproduced; no shipped map
//!   has such a name.
pub use aa_metagame::chunk_file::{FIRST_CHUNK_OFFSET, MAGIC, NAME_UNITS, VERSION};
use aa_metagame::chunk_file::ChunkFileWriter;

/// Size of a directory record.
pub const RECORD_BYTES: usize = 0x60;
/// Where bits 1..63 of the flags word sit in a record.
const FLAGS_AT: usize = 0x50;

/// One chunk of a map to write, in file order.
#[derive(Clone, Copy, Debug)]
pub struct ChunkSource<'a> {
    pub name: &'a str,
    /// Flag bit 0: the chunk goes through the level-6 compressor.
    pub compressed: bool,
    /// The chunk's logical (decoded) bytes.
    pub payload: &'a [u8],
}

/// The directory of a whole chunk file: the count at 0x08 and the records
/// at the end of the file (859380 reads it the same way).
fn directory(file: &[u8]) -> Option<(usize, usize)> {
    let count = u32::from_le_bytes(file.get(8..12)?.try_into().ok()?) as usize;
    let start = file.len().checked_sub(count.checked_mul(RECORD_BYTES)?)?;
    (start >= FIRST_CHUNK_OFFSET as usize).then_some((start, count))
}

/// Bits 1..63 of the flags word when every record of `file` has the same
/// value there (as the original writes it); `None` for a file whose
/// records disagree or that is not a version 2 chunk file.
pub fn stale_flag_bits(file: &[u8]) -> Option<u64> {
    if file.get(..4)? != MAGIC.to_le_bytes() || u32::from_le_bytes(file.get(4..8)?.try_into().ok()?) != VERSION {
        return None;
    }
    let (start, count) = directory(file)?;
    let mut value = None;
    for i in 0..count {
        let at = start + i * RECORD_BYTES + FLAGS_AT;
        let flags = u64::from_le_bytes(file[at..at + 8].try_into().ok()?) & !1;
        if *value.get_or_insert(flags) != flags {
            return None;
        }
    }
    Some(value.unwrap_or(0))
}

/// Writes `bits` (bit 0 ignored) into bits 1..63 of every record of a
/// closed chunk file, keeping each record's bit 0. Returns false when the
/// directory is not where the header says.
pub fn set_stale_flag_bits(file: &mut [u8], bits: u64) -> bool {
    let Some((start, count)) = directory(file) else { return false };
    for i in 0..count {
        let at = start + i * RECORD_BYTES + FLAGS_AT;
        let flags = u64::from_le_bytes(file[at..at + 8].try_into().expect("8 bytes"));
        file[at..at + 8].copy_from_slice(&((flags & 1) | (bits & !1)).to_le_bytes());
    }
    true
}

/// A whole map file from its chunks, each written in one piece through the
/// shared chunk file writer, then the file's stale flag bits. `path` is the
/// file name the writer's error messages show (858BA0).
pub fn write_map(path: &str, chunks: &[ChunkSource<'_>], stale_bits: u64) -> aa_metagame::Result<Vec<u8>> {
    let mut w = ChunkFileWriter::create(path);
    for c in chunks {
        w.open_chunk(c.name, c.compressed)?;
        w.write(c.payload)?;
    }
    let mut file = w.close()?;
    set_stale_flag_bits(&mut file, stale_bits);
    Ok(file)
}

/// Reads a map with our map reader and writes it again with the map file
/// writer, keeping its chunk order, encodings and stale flag bits. For a
/// shipped map the result is the file itself (tests/shipped_maps.rs).
pub fn rewrite_map(path: &str, file: &[u8]) -> Result<Vec<u8>, String> {
    let stale = stale_flag_bits(file).ok_or_else(|| format!("{path}: not a version 2 map file with one flags value"))?;
    let archive = aa_mapworld::MapArchive::from_bytes(file.to_vec()).map_err(|e| format!("{path}: {e}"))?;
    let decoded = archive
        .directory()
        .iter()
        .map(|row| archive.decode_chunk(&row.name, u64::MAX))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("{path}: {e}"))?;
    let sources: Vec<ChunkSource<'_>> = decoded
        .iter()
        .map(|c| ChunkSource {
            name: &c.directory.name,
            compressed: c.encoding == aa_mapworld::archive::Encoding::Zlib,
            payload: &c.bytes,
        })
        .collect();
    write_map(path, &sources, stale).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_is_the_bare_header() {
        assert_eq!(write_map("x.rmp", &[], 0).unwrap(), b"TGCK\x02\0\0\0\0\0\0\0");
    }

    #[test]
    fn raw_and_compressed_chunks_follow_each_other() {
        let bytes = write_map(
            "x.rmp",
            &[
                ChunkSource { name: "HEADER", compressed: false, payload: b"abc" },
                ChunkSource { name: "Z", compressed: true, payload: b"hello" },
            ],
            0x1234_0000,
        )
        .unwrap();
        assert_eq!(&bytes[..12], b"TGCK\x02\0\0\0\x02\0\0\0");
        assert_eq!(&bytes[12..15], b"abc");
        let z = crate::deflate::compress(b"hello", 6).unwrap();
        assert_eq!(&bytes[15..15 + z.len()], &z[..]);
        let dir = &bytes[15 + z.len()..];
        assert_eq!(dir.len(), 2 * RECORD_BYTES);
        let q = |r: &[u8], o: usize| u64::from_le_bytes(r[o..o + 8].try_into().unwrap());
        let (a, b) = (&dir[..RECORD_BYTES], &dir[RECORD_BYTES..]);
        assert_eq!((q(a, 0x40), q(a, 0x48), q(a, 0x50), q(a, 0x58)), (12, 3, 0x1234_0000, 3));
        assert_eq!((q(b, 0x40), q(b, 0x48), q(b, 0x50), q(b, 0x58)), (15, z.len() as u64, 0x1234_0001, 5));
        assert_eq!(&a[..12], "HEADER".encode_utf16().flat_map(u16::to_le_bytes).collect::<Vec<_>>().as_slice());
        assert!(a[12..64].iter().all(|&x| x == 0));
        assert_eq!(stale_flag_bits(&bytes), Some(0x1234_0000));
    }

    #[test]
    fn names_follow_the_shared_writer() {
        let write = |name: &str| write_map("x.rmp", &[ChunkSource { name, compressed: false, payload: b"" }], 0);
        assert_eq!(
            write("").unwrap_err().to_string(),
            "native data error: File: x.rmp Chunk: NONE Offset: 0 - chunk name cannot be empty"
        );
        assert!(write(&"x".repeat(32)).is_err());
        assert!(write(&"x".repeat(31)).is_ok());
    }
}
