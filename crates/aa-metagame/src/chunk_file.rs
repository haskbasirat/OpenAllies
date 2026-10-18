//! The chunk-file container of saved games ("TGCK"), the framing around the
//! `METAGAME` payload ([`crate::save`]) and every other map chunk. Ported from
//! the original AA.exe base chunk file class (vtable AB4E38, ctor 8584A0);
//! proven by `research/phase8-metagame/native-chunks.json`
//! (`tests/native_chunks.rs`).
//!
//! | native | role |
//! |---|---|
//! | 4F2DB0 / 4FC600 | map save / load: chunk order (METAGAME after KINGDOMS, before USED_REQUIRED_OBJECTS / STATIC_THINGS / DYNAMIC_THINGS) |
//! | 859690 | open: file object (877B80), `+0x34` base 0, `+0x38` file size, 858540, 859380 |
//! | 859380 | read the header and directory (new file: header written by 859090) |
//! | 859150 | open a chunk for writing (name, compress flag) |
//! | 858E40 / 858DB0 | write / chunk size update |
//! | 859030 | open a chunk for reading (name, required) |
//! | 858C90 | read |
//! | 858F50, 858ED0 | chunk seek, file seek |
//! | 859330 / 859090 | close: header and directory |
//! | 8582B0 / 8580E0 | directory record / descriptor |
//! | 858BA0 | error: `"File: %s Chunk: %s Offset: %ld - "` + reason, thrown |
//!
//! Layout (little endian): a 12-byte header `{u32 'TGCK' (0x4B434754),
//! u32 version (2), u32 chunk count}`, the chunk data (first chunk at 0x0C,
//! each next chunk at the previous offset + stored size), then one directory
//! record per chunk: name (32 UTF-16 units, `wcsncpy`, zero padded), `u64`
//! offset, `u64` stored size, `u64` flags (bit 0: zlib-compressed), `u64`
//! uncompressed size (0x60 bytes; version 1 records stop before the
//! uncompressed size, version 0 records also before the flags). The reader
//! locates the directory from the end of the file.
//!
//! Not reproduced: flag bits 1..63 of a written record are uninitialised
//! stack memory in the original (859090 builds the record in a stack buffer
//! and 8582B0 only clears bit 0); this writer writes 0 there and the reader
//! ignores them like the original. The writer supports `MapSaveCompressed`
//! with one retained level-6 zlib stream, including flush then continuation.
//! Its bytes are format compatible with the original zlib1.1.4 inflater
//! (development10/native_inflate_interop.py); deflater byte identity is not
//! claimed. Compressed payload reads in this module remain unsupported.
//! The engine file object (open, short reads) is not
//! part of this module: reading past the end of the file is refused.
use crate::{Error, Result};
use flate2::{Compression,write::ZlibEncoder};
use std::io::Write;

/// `'TGCK'` header magic (859380).
pub const MAGIC: u32 = 0x4B43_4754;
/// Version written by 859090.
pub const VERSION: u32 = 2;
/// Offset of the first chunk (859150 with no previous chunk).
pub const FIRST_CHUNK_OFFSET: u64 = 0x0C;
/// Name field length in UTF-16 units (`wcsncpy(.., 0x20)`).
pub const NAME_UNITS: usize = 32;

/// One directory record / chunk descriptor (8580E0 layout, `+0x00..+0x5F`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkRecord {
    /// `+0x00`: name field (zero padded).
    pub name: [u16; NAME_UNITS],
    /// `+0x40`: offset of the chunk data.
    pub offset: u64,
    /// `+0x48`: stored size.
    pub size: u64,
    /// `+0x50`: flags (bit 0: compressed).
    pub flags: u64,
    /// `+0x58`: uncompressed (logical) size.
    pub uncompressed_size: u64,
}

impl ChunkRecord {
    pub fn compressed(&self) -> bool {
        self.flags & 1 != 0
    }
    /// The name as the C string the original compares and prints: the name
    /// field up to its first zero unit; an unterminated 32-unit field runs on
    /// into the offset field (`+0x40`), whose high dword ends it.
    pub fn c_name(&self) -> String {
        let tail = [self.offset as u16, (self.offset >> 16) as u16, 0];
        let units: Vec<u16> = self.name.iter().chain(tail.iter()).copied().take_while(|&u| u != 0).collect();
        String::from_utf16_lossy(&units)
    }
}

/// `__wcsicmp` in the C locale (ASCII case folding).
fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// 858BA0: `"File: %s Chunk: %s Offset: %ld - "` followed by the reason.
fn chunk_error(path: &str, chunk: Option<&ChunkRecord>, logical: u32, reason: &str) -> Error {
    let name = chunk.map_or_else(|| "NONE".to_owned(), ChunkRecord::c_name);
    let offset = if chunk.is_some() { logical as i32 } else { 0 };
    Error::NativeData(format!("File: {path} Chunk: {name} Offset: {offset} - {reason}"))
}

fn name_field(name: &str) -> Result<[u16; NAME_UNITS]> {
    let units: Vec<u16> = name.encode_utf16().collect();
    if units.contains(&0) {
        return Err(Error::Unsupported("chunk name with a zero unit".into()));
    }
    if units.len() >= NAME_UNITS {
        return Err(Error::Unsupported(format!(
            "chunk name of {} units: 859150 stores it unterminated",
            units.len()
        )));
    }
    let mut field = [0u16; NAME_UNITS];
    field[..units.len()].copy_from_slice(&units);
    Ok(field)
}

/// A chunk file being written (859690 in write mode ... 859330).
#[derive(Debug)]
pub struct ChunkFileWriter {
    path: String,
    data: Vec<u8>,
    chunks: Vec<ChunkRecord>,
    current: Option<usize>,
    compressor:Option<ZlibEncoder<Vec<u8>>>,
}
///Borrowed current storage after flush. An ongoing compressed chunk remains
///in its SAME encoder buffer, so exposing two slices avoids copying a growing
///film on every flush. The directory is absent until actual close.
#[derive(Clone,Copy,Debug)]
pub struct FlushedBytes<'a> {pub prefix:&'a [u8],pub compressed_tail:&'a [u8]}

impl ChunkFileWriter {
    /// 859690 on a new (write-mode) file: 859380 writes the header with
    /// count 0 (859090).
    pub fn create(path: &str) -> Self {
        let mut data = Vec::with_capacity(64);
        data.extend_from_slice(&MAGIC.to_le_bytes());
        data.extend_from_slice(&VERSION.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        ChunkFileWriter { path: path.to_owned(), data, chunks: Vec::new(), current: None,compressor:None }
    }

    ///859150: start a chunk after the last one. Compressed payloads use a
    ///representation-compatible zlib level6 backend; byte identity with the
    ///original zlib1.1.4 compressor is not claimed. Native framing is retained.
    pub fn open_chunk(&mut self, name: &str, compressed: bool) -> Result<()> {
        self.finish_compressed()?;
        self.current = None;
        if name.is_empty() {
            return Err(chunk_error(&self.path, None, 0, "chunk name cannot be empty"));
        }
        let field = name_field(name)?;
        let offset = self.chunks.last().map_or(FIRST_CHUNK_OFFSET, |c| c.offset + c.size);
        self.chunks.push(ChunkRecord { name: field, offset, size: 0, flags: u64::from(compressed), uncompressed_size: 0 });
        self.current = Some(self.chunks.len() - 1);
        self.compressor=compressed.then(||ZlibEncoder::new(Vec::new(),Compression::new(6)));
        Ok(())
    }

    /// 858E40 + 858DB0: append to the current (last) chunk.
    pub fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let Some(i) = self.current else {
            return Err(chunk_error(&self.path, None, 0, "writing with no chunk"));
        };
        let c = &mut self.chunks[i];
        if let Some(encoder)=self.compressor.as_mut() {
            encoder.write_all(bytes).map_err(|e|Error::NativeData(format!("zlib output failed: {e}")))?;
            c.size=encoder.get_ref().len() as u64;
            c.uncompressed_size=c.uncompressed_size.checked_add(bytes.len() as u64)
                .ok_or_else(||Error::Unsupported("chunk logical size exceeds u64".into()))?;
            return Ok(());
        }
        let at = (c.offset + c.size) as usize;
        if self.data.len() < at {
            self.data.resize(at, 0);
        }
        self.data.truncate(at);
        self.data.extend_from_slice(bytes);
        c.size += bytes.len() as u64;
        c.uncompressed_size = c.size;
        Ok(())
    }

    ///Flush the actual memory codec output without closing its chunk or
    ///discarding its state. Later writes continue the SAME zlib stream.
    pub fn flush(&mut self)->Result<()> {
        if let Some(encoder)=self.compressor.as_mut() {
            encoder.flush().map_err(|e|Error::NativeData(format!("zlib flush failed: {e}")))?;
            let i=self.current.ok_or_else(||Error::Unsupported("compressor without current chunk".into()))?;
            self.chunks[i].size=encoder.get_ref().len() as u64;
        }
        Ok(())
    }
    pub fn current_logical_size(&self)->Result<u64> {
        let i=self.current.ok_or_else(||chunk_error(&self.path,None,0,"no current chunk"))?;
        Ok(self.chunks[i].uncompressed_size)
    }
    ///Current emitted prefix. Calling actual `flush` first includes buffered
    ///codec input; a plain write exposes only bytes the codec already emitted.
    pub fn flushed_bytes(&self)->Result<FlushedBytes<'_>> {
        if let Some(encoder)=self.compressor.as_ref() {
            let i=self.current.ok_or_else(||Error::Unsupported("compressor without current chunk".into()))?;
            let offset=usize::try_from(self.chunks[i].offset).map_err(|_|Error::Unsupported("chunk offset exceeds address space".into()))?;
            let prefix=self.data.get(..offset).ok_or_else(||Error::Unsupported("compressed chunk offset exceeds retained file prefix".into()))?;
            Ok(FlushedBytes{prefix,compressed_tail:encoder.get_ref()})
        }else{Ok(FlushedBytes{prefix:&self.data,compressed_tail:&[]})}
    }
    fn finish_compressed(&mut self)->Result<()> {
        if let Some(encoder)=self.compressor.take() {
            let bytes=encoder.finish().map_err(|e|Error::NativeData(format!("zlib finalization failed: {e}")))?;
            let i=self.current.ok_or_else(||Error::Unsupported("compressor without current chunk".into()))?;
            let c=&mut self.chunks[i];
            let offset=usize::try_from(c.offset).map_err(|_|Error::Unsupported("chunk offset exceeds address space".into()))?;
            self.data.truncate(offset);
            self.data.extend_from_slice(&bytes);
            c.size=bytes.len() as u64;
        }
        Ok(())
    }

    /// 859330 / 859090: header with the chunk count, directory after the last
    /// chunk. Returns the file contents.
    pub fn close(mut self) -> Result<Vec<u8>> {
        self.finish_compressed()?;
        let count = u32::try_from(self.chunks.len()).map_err(|_| Error::Unsupported("chunk count".into()))?;
        self.data[8..12].copy_from_slice(&count.to_le_bytes());
        let end = self.chunks.last().map_or(FIRST_CHUNK_OFFSET, |c| c.offset + c.size) as usize;
        self.data.truncate(end);
        for c in &self.chunks {
            for u in c.name {
                self.data.extend_from_slice(&u.to_le_bytes());
            }
            self.data.extend_from_slice(&c.offset.to_le_bytes());
            self.data.extend_from_slice(&c.size.to_le_bytes());
            self.data.extend_from_slice(&(c.flags & 1).to_le_bytes());
            self.data.extend_from_slice(&c.uncompressed_size.to_le_bytes());
        }
        Ok(self.data)
    }
}

/// A chunk file opened for reading (859690 in read mode + 859380).
#[derive(Clone, Debug)]
pub struct ChunkFileReader<'a> {
    path: String,
    bytes: &'a [u8],
    pub version: u32,
    pub chunks: Vec<ChunkRecord>,
    current: Option<usize>,
    /// `+0x64` / `+0x68` of the current chunk.
    actual: u32,
    logical: u32,
}

impl<'a> ChunkFileReader<'a> {
    fn file(&self, at: u64, n: u64) -> Result<&'a [u8]> {
        let end = at.checked_add(n).filter(|&e| e <= self.bytes.len() as u64).ok_or_else(|| {
            Error::Unsupported(format!(
                "file read of {n} bytes at {at} past the end ({}) (engine file behaviour not established)",
                self.bytes.len()
            ))
        })?;
        Ok(&self.bytes[at as usize..end as usize])
    }

    fn err(&self, reason: &str) -> Error {
        chunk_error(&self.path, self.current.map(|i| &self.chunks[i]), self.logical, reason)
    }

    /// 859380 on a read-mode file.
    pub fn open(path: &str, bytes: &'a [u8]) -> Result<Self> {
        let mut r = ChunkFileReader {
            path: path.to_owned(),
            bytes,
            version: 0,
            chunks: Vec::new(),
            current: None,
            actual: 0,
            logical: 0,
        };
        let header = r.file(0, 12)?;
        let word = |i: usize| u32::from_le_bytes(header[4 * i..4 * i + 4].try_into().expect("4 bytes"));
        let (magic, version, count) = (word(0), word(1), word(2));
        if magic != MAGIC {
            return Err(r.err("not a chunk file"));
        }
        if version > 2 {
            return Err(r.err("unknown chunk file version"));
        }
        r.version = version;
        let record_size: u32 = match version {
            0 => 0x50,
            1 => 0x58,
            _ => 0x60,
        };
        // 858ED0 origin 2 (32-bit): end - count * size, at most the end
        let size = bytes.len() as u32;
        let dir = size.wrapping_sub(count.wrapping_mul(record_size));
        if dir > size {
            return Err(r.err("seeking beyond end of chunk file"));
        }
        let mut at = u64::from(dir);
        for _ in 0..count {
            let rec = r.file(at, u64::from(record_size))?;
            at += u64::from(record_size);
            let q = |o: usize| u64::from_le_bytes(rec[o..o + 8].try_into().expect("8 bytes"));
            let mut name = [0u16; NAME_UNITS];
            for (i, u) in name.iter_mut().enumerate() {
                *u = u16::from_le_bytes([rec[2 * i], rec[2 * i + 1]]);
            }
            let offset = q(0x40);
            let stored = q(0x48);
            // 8580E0 leaves flag bit 0 clear; version 0 records have no flags
            let flags = if version >= 1 { q(0x50) } else { 0 };
            let uncompressed_size = if version >= 2 { q(0x58) } else { stored };
            let c = ChunkRecord { name, offset, size: stored, flags, uncompressed_size };
            // 32-bit compare of the offset's low dword with the file size
            if (offset as u32) > size || c.c_name().is_empty() {
                return Err(r.err("bad chunk data"));
            }
            r.chunks.push(c);
        }
        for i in 0..r.chunks.len() {
            let name = r.chunks[i].c_name();
            if r.chunks[..i].iter().any(|c| same(&c.c_name(), &name)) {
                return Err(r.err("Chunk already exists"));
            }
        }
        Ok(r)
    }

    /// 859030: select a chunk by name (case-insensitive). `Ok(false)` when it
    /// does not exist and is not required (the current chunk is kept).
    pub fn open_chunk(&mut self, name: &str, required: bool) -> Result<bool> {
        let wanted = name.split('\0').next().unwrap_or("");
        match self.chunks.iter().position(|c| same(&c.c_name(), wanted)) {
            Some(i) => {
                if self.chunks[i].compressed() {
                    return Err(Error::Unsupported("compressed chunks (zlib inflate 8B7340) are not ported".into()));
                }
                // 858F50(0)
                self.current = Some(i);
                self.actual = 0;
                self.logical = 0;
                Ok(true)
            }
            None if required => Err(self.err(&format!("could not find chunk '{wanted}'"))),
            None => Ok(false),
        }
    }

    /// 858C90 on an uncompressed chunk.
    pub fn read(&mut self, n: usize) -> Result<&'a [u8]> {
        let Some(i) = self.current else {
            return Err(self.err("reading with no chunk"));
        };
        let n32 = u32::try_from(n).map_err(|_| Error::Unsupported("read size".into()))?;
        let c = &self.chunks[i];
        if u64::from(self.actual.wrapping_add(n32)) > (c.size & 0xffff_ffff) {
            return Err(self.err("reading past end of chunk (actual)"));
        }
        if u64::from(self.logical.wrapping_add(n32)) > (c.uncompressed_size & 0xffff_ffff) {
            return Err(self.err("reading past end of chunk (logical)"));
        }
        let at = c.offset + u64::from(self.actual);
        self.logical = self.logical.wrapping_add(n32);
        self.actual = self.actual.wrapping_add(n32);
        self.file(at, n as u64)
    }

    /// The whole payload of a chunk (859030 required + one read of its size).
    pub fn chunk(&mut self, name: &str) -> Result<&'a [u8]> {
        self.open_chunk(name, true)?;
        let size = self.chunks[self.current.expect("opened")].size as usize;
        self.read(size)
    }
}
