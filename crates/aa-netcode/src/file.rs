//! Lobby file transfer (map/save): TGC_FileSender and TGC_FileReceiver of
//! the original AA.exe, ported instruction by instruction and replayed against
//! research/phase8-net/native-file-transfer.json (tests/file_transfer.rs).
//!
//! Native objects (embedded in TGC_AdminConn, ctor 0x756BC0):
//! * TGC_FileSender at AdminConn+0x39C (ctor 0x7611B0, vtable 0xA91BEC):
//!   BlockSender base (+0..+0x2C, span = AVar NetworkFileTransferBlocksPerFrame,
//!   one copy in flight), block-buffer window +0x2C, TGC_File +0x3C, file size
//!   +0x48. Slots: available 0x761920, release 0x761850, estimate 0x761310,
//!   serialize 0x7613C0; reset 0x761400, start 0x761540 (called by
//!   TGC_AdminConn 0x74FE30 after incrementing the generation +0x418), block
//!   read 0x761420, read-ahead 0x7617E0, progress 0x7612D0.
//! * TGC_FileReceiver at AdminConn+0x3E8 (ctor 0x761870, vtable 0xA91C00): a
//!   TGC_BlockReceiver (span +4), block-buffer window +8, TGC_File +0x18, file
//!   size +0x24. Slots: receive 0x761DB0, skip 0x761150; reset 0x761500,
//!   payload codec 0x7610A0, drain 0x761CE0, header 0x7619A0, data write
//!   0x761220. The generation word is checked by TGC_AdminConn 0x74C8B0.
//!
//! Wire: every block travels as a 32-bit byte count followed by one 8-bit
//! value per byte (writer 0x75FFA0, reader 0x7610A0 with 0x411FA0(256)).
//! Block 0 is a finalized bitstream (0x74D0A0: 3-bit pad header) holding a
//! 16-bit name length, the UTF-16 name and the 32-bit file size. Block k >= 1
//! holds file bytes [(k-1)*BS, k*BS); the terminal index is
//! L = (size + BS - 1) / BS (32-bit wrapping add, truncating division).
//!
//! Terminal block length, two native images (both oracle-proven):
//! * `TerminalTail::Original` (2004 AA.exe 0x76144D..0x76146D sender,
//!   0x76124A..0x761273 receiver): `size % BS`. A non-zero exact multiple of BS
//!   sends an empty terminal block and the receiver writes nothing for it: the
//!   last BS bytes are lost while progress 0x7612D0 reports the full size.
//! * `TerminalTail::V14` (accepted v14 AA.exe, docs/BASELINE.txt "exact-block-
//!   tail repair"): sender cave 0x9EFF80 uses BS when the remainder is 0;
//!   receiver cave 0x9EFF90 uses min(payload length, BS) (unsigned compare)
//!   when the remainder is 0.
//!
//! A size of 0 has L = 0: no data block exists, so neither side ever closes
//! its file (both images).
//!
//! Native capacities replaced by growable structures (no fixed caps here):
//! * 0x7611FA / 0x7618B4: 0x74B790(0x100), block-buffer rings with initial
//!   capacity 256 slots (grown by 0x74CDB0) -> [`BlockWindow`] (sparse map).
//! * 0x761648: header writer buffer 0x767110(0x100) bytes, grown by 0x40D420
//!   -> 0x767000 -> `WireWriter`.
//! * 0x7610E5: payload buffer of the decoded 32-bit length -> `Vec`, checked
//!   against the bits left in the packet before allocating.
//!
//! Kept as rules (they change behaviour or the wire): NetworkFileTransfer-
//! BlocksPerFrame 20 (span; both ends must agree), BlockSize 64 (both ends must
//! agree), MaxPendingBlocks 256 (sender read-ahead only), the 16-bit header
//! name length (0x7616CE `movzx di`: names of 65536+ units are cut on the wire).
//!
//! Native crashes or undefined reads become explicit errors here: zero or
//! overflowing block size (`idiv`), block windows that would span more than
//! 2^31 slots (native ring aliasing), serializing an empty slot (null buffer
//! 0x766F80), negative block lengths, receiver writes longer than the
//! received payload (heap over-read in 0x761220), retiring a window backwards
//! (stale ring slots), and payload lengths larger than the packet.
use crate::channel::{BlockReceiver, BlockSender, BlockSink, BlockSource};
use crate::{Error, Result};
use aa_orders::commands::WireWriter;
use aa_orders::net::transport::finalize;
use aa_orders::{Cursor, Position};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Mode word of the sender's open (DAT_00AB4784, .rdata): access 1 (read),
/// share 0x10 (FILE_SHARE_READ), OPEN_EXISTING, 0x20000 = raise
/// TGC_FileException on failure (impl open 0x876C60, 0x876E04).
pub const MODE_SENDER_READ: u32 = 0x20011;
/// Mode word of the receiver's open (DAT_00AB4788, .rdata): access 2 (write),
/// share 0x08 (none), 0x100 = CREATE_ALWAYS (0x200 would select OPEN_ALWAYS),
/// preceded by a call of [0xB43B74] vtable +0x34 with the path (inferred:
/// create the parent directories), 0x20000 = raise TGC_FileException when the
/// CreateFile pointer [0xB3640C] fails (0x876DF9..0x876E2D).
pub const MODE_RECEIVER_WRITE: u32 = 0x2010A;
/// The name 0x761970 (receiver vtable +8) returns, printed by log line 0xAC.
pub const RECEIVER_NAME: &str = "file receiver";

/// Terminal-block length rule of the native image being reproduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminalTail {
    /// 2004 AA.exe: `size % BS` (exact multiples lose their last block).
    Original,
    /// Accepted v14 AA.exe caves 0x9EFF80 (sender) / 0x9EFF90 (receiver).
    V14,
}

/// AVars read through the table [0xB39990] (registered at 0x403760..0x40380C)
/// plus the image's terminal-block rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTransferConfig {
    /// AVar 0x48 NetworkFileTransferBlocksPerFrame (+0x120), default 20
    /// (0x403762). Copied into the BlockSender/BlockReceiver span at
    /// construction (0x7611B0, 0x761870).
    pub blocks_per_frame: i32,
    /// AVar 0x49 NetworkFileTransferBlockSize (+0x124), default 64 (0x4037A5).
    pub block_size: i32,
    /// AVar 0x4A NetworkFileTransferMaxPendingBlocks (+0x128), default 256
    /// (0x4037E8). Sender read-ahead limit only.
    pub max_pending_blocks: i32,
    pub tail: TerminalTail,
}
impl FileTransferConfig {
    /// Native AVar defaults with the 2004 terminal rule.
    pub const ORIGINAL: Self = Self {
        blocks_per_frame: 20,
        block_size: 64,
        max_pending_blocks: 256,
        tail: TerminalTail::Original,
    };
    /// Native AVar defaults with the accepted v14 terminal rule.
    pub const V14: Self = Self { tail: TerminalTail::V14, ..Self::ORIGINAL };
}
impl Default for FileTransferConfig {
    /// docs/BASELINE.txt: the rewrite targets the accepted v14 executable.
    fn default() -> Self {
        Self::V14
    }
}

/// The embedded TGC_File object (ctor 0x857FE0, vtable 0xAB4DE8): one handle
/// that `open` binds to a path. Method order and arguments mirror the native
/// virtual calls so call sequences can be compared with the oracle.
pub trait FileStore {
    /// +0x20 (0x405750 -> 0x858000 -> impl 0x876C60). `Err` is the
    /// TGC_FileException (0x869200) raised when CreateFileW fails.
    fn open(&mut self, path: &[u16], mode: u32) -> Result<()>;
    /// +0x14 (0x856DF0).
    fn is_open(&mut self) -> bool;
    /// +0x00 (0x858020 -> impl +0x2C). Returns the byte count; the native
    /// callers ignore it (the buffer was zero-filled by 0x767110).
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    /// +0x04 (0x856BE0 -> impl +0x30).
    fn write(&mut self, data: &[u8]) -> Result<()>;
    /// +0x30 (0x856DA0 -> impl 0x8767F0: low 32 bits of GetFileSize).
    fn size(&mut self) -> Result<i32>;
    /// +0x28 (0x858080): closes when open, no-op otherwise.
    fn close(&mut self) -> Result<()>;
}

/// One recorded [`FileStore`] call (tests and diagnostics).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileCall {
    Open { path: Vec<u16>, mode: u32 },
    IsOpen { result: bool },
    Read { len: usize, returned: usize },
    Write { bytes: Vec<u8> },
    Close,
    Size { result: i32 },
}

/// In-memory [`FileStore`]: a private "disk" for one machine plus one handle.
/// Records every call. Opening with mode bit 0x100 creates/truncates
/// (CREATE_ALWAYS); other modes need an existing file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryFile {
    pub disk: BTreeMap<Vec<u16>, Vec<u8>>,
    /// Size reported instead of the content length (models a file that
    /// changed after the size was taken).
    pub reported_size: BTreeMap<Vec<u16>, i32>,
    pub calls: Vec<FileCall>,
    handle: Option<(Vec<u16>, usize)>,
}
impl MemoryFile {
    pub fn with_files(files: impl IntoIterator<Item = (Vec<u16>, Vec<u8>)>) -> Self {
        Self { disk: files.into_iter().collect(), ..Self::default() }
    }
    /// Path the handle is bound to while open.
    pub fn open_path(&self) -> Option<&[u16]> {
        self.handle.as_ref().map(|(p, _)| p.as_slice())
    }
}
impl FileStore for MemoryFile {
    fn open(&mut self, path: &[u16], mode: u32) -> Result<()> {
        self.calls.push(FileCall::Open { path: path.to_vec(), mode });
        if mode & 0x100 != 0 {
            self.disk.insert(path.to_vec(), Vec::new());
        } else if !self.disk.contains_key(path) {
            self.handle = None;
            return Err(Error::Io(std::io::Error::from(std::io::ErrorKind::NotFound)));
        }
        self.handle = Some((path.to_vec(), 0));
        Ok(())
    }
    fn is_open(&mut self) -> bool {
        let result = self.handle.is_some();
        self.calls.push(FileCall::IsOpen { result });
        result
    }
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let (path, pos) = self.handle.as_mut().ok_or(Error::Unsupported(
            "read on an unopened TGC_File (native dereferences a null impl, 0x858020)",
        ))?;
        let data = &self.disk[path.as_slice()];
        let n = buf.len().min(data.len().saturating_sub(*pos));
        buf[..n].copy_from_slice(&data[*pos..*pos + n]);
        *pos += n;
        self.calls.push(FileCall::Read { len: buf.len(), returned: n });
        Ok(n)
    }
    fn write(&mut self, data: &[u8]) -> Result<()> {
        let (path, _) = self.handle.as_ref().ok_or(Error::Unsupported(
            "write on an unopened TGC_File (native dereferences a null impl, 0x856BE0)",
        ))?;
        self.disk.get_mut(path.as_slice()).expect("bound file exists").extend_from_slice(data);
        self.calls.push(FileCall::Write { bytes: data.to_vec() });
        Ok(())
    }
    fn size(&mut self) -> Result<i32> {
        let (path, _) = self.handle.as_ref().ok_or(Error::Unsupported(
            "size of an unopened TGC_File (native dereferences a null impl, 0x856DA0)",
        ))?;
        let result = match self.reported_size.get(path) {
            Some(&v) => v,
            None => self.disk[path.as_slice()].len() as u32 as i32,
        };
        self.calls.push(FileCall::Size { result });
        Ok(result)
    }
    fn close(&mut self) -> Result<()> {
        self.calls.push(FileCall::Close);
        self.handle = None;
        Ok(())
    }
}

/// [`FileStore`] on the host file system. The native path is the profile
/// directory plus the name; CreateFileW sees it only up to the first NUL code
/// unit, which this adapter reproduces. Access (mode & 7) 1 read, 2 write,
/// 4 both; bit 0x100 creates parent directories (inferred from the
/// [0xB43B74]+0x34 call) and CREATE_ALWAYS (bit 0x200: OPEN_ALWAYS);
/// otherwise OPEN_EXISTING (impl open 0x876C60). Share modes are not
/// reproduced. Not oracle-tested (the oracle models TGC_File).
#[derive(Debug, Default)]
pub struct StdFile {
    handle: Option<std::fs::File>,
}
impl StdFile {
    fn native_path(path: &[u16]) -> Result<std::path::PathBuf> {
        let end = path.iter().position(|&u| u == 0).unwrap_or(path.len());
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            Ok(std::ffi::OsString::from_wide(&path[..end]).into())
        }
        #[cfg(not(windows))]
        {
            String::from_utf16(&path[..end])
                .map(Into::into)
                .map_err(|_| Error::Protocol("file name is not valid UTF-16".into()))
        }
    }
}
impl FileStore for StdFile {
    fn open(&mut self, path: &[u16], mode: u32) -> Result<()> {
        use std::fs::OpenOptions;
        self.handle = None;
        let path = Self::native_path(path)?;
        let mut options = OpenOptions::new();
        match mode & 7 {
            1 => options.read(true),
            2 => options.write(true),
            4 => options.read(true).write(true),
            _ => return Err(Error::Unsupported("TGC_File access mode other than 1/2/4")),
        };
        if mode & 0x100 != 0 {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if mode & 0x200 != 0 {
                options.create(true);
            } else {
                options.create(true).truncate(true);
            }
        }
        self.handle = Some(options.open(path)?);
        Ok(())
    }
    fn is_open(&mut self) -> bool {
        self.handle.is_some()
    }
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        use std::io::Read;
        let file = self.handle.as_mut().ok_or(Error::Unsupported("read on an unopened TGC_File"))?;
        let mut n = 0;
        while n < buf.len() {
            match file.read(&mut buf[n..])? {
                0 => break,
                k => n += k,
            }
        }
        Ok(n)
    }
    fn write(&mut self, data: &[u8]) -> Result<()> {
        use std::io::Write;
        let file = self.handle.as_mut().ok_or(Error::Unsupported("write on an unopened TGC_File"))?;
        file.write_all(data)?;
        Ok(())
    }
    fn size(&mut self) -> Result<i32> {
        let file = self.handle.as_ref().ok_or(Error::Unsupported("size of an unopened TGC_File"))?;
        Ok(file.metadata()?.len() as u32 as i32)
    }
    fn close(&mut self) -> Result<()> {
        self.handle = None;
        Ok(())
    }
}

/// Native log lines of the receiver (0x404F60, .\Source\Networking\FileTransfer.cpp).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileLog {
    /// Line 0xBF "NET: Aborting file transfer because .. in name %s".
    DotDotRejected { name: Vec<u16> },
    /// Line 0xAC "NET: %s Starting download of file %s size %d " (%s =
    /// [`RECEIVER_NAME`]).
    StartingDownload { name: Vec<u16>, size: i32 },
    /// Catch@0x761BCD: localized UI message
    /// "networking_file_transfer_could_not_write" (0x4B9F40, 0x40BC50(0xB399B0,
    /// 0x11)) then line 0xB9 "NET: Aborting file transfer because file was
    /// already unable to be opened %s". Not oracle-proven (C++ unwinding).
    OpenFailed { name: Vec<u16> },
}

/// Native 4-byte buffer window {begin, end, capacity, ring} (ctor 0x74B790,
/// extend 0x74EC30, retire 0x750880, clear 0x7507D0). Slots hold byte buffers
/// (0x767110) or null; a null slot is an absent key here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockWindow {
    begin: i32,
    end: i32,
    slots: BTreeMap<i32, Vec<u8>>,
}
impl Default for BlockWindow {
    fn default() -> Self {
        Self::cleared()
    }
}
impl BlockWindow {
    /// State after 0x74B790 / 0x7507D0: [INT_MIN, INT_MIN).
    pub fn cleared() -> Self {
        Self { begin: i32::MIN, end: i32::MIN, slots: BTreeMap::new() }
    }
    pub fn begin(&self) -> i32 {
        self.begin
    }
    pub fn end(&self) -> i32 {
        self.end
    }
    pub fn get(&self, index: i32) -> Option<&Vec<u8>> {
        self.slots.get(&index)
    }
    /// Non-null slots in index order.
    pub fn slots(&self) -> impl Iterator<Item = (i32, &Vec<u8>)> {
        self.slots.iter().map(|(&i, b)| (i, b))
    }
    /// 0x7507D0.
    pub fn clear(&mut self) {
        *self = Self::cleared();
    }
    /// 0x74EC30: new slots are null; `end` is stored as given.
    pub fn extend_to(&mut self, end: i32) -> Result<()> {
        if i64::from(end) - i64::from(self.begin) > i64::from(i32::MAX) {
            return Err(Error::Protocol(format!(
                "block window [{}, {end}) spans more than 2^31 slots (native ring aliasing in 0x74EC30)",
                self.begin
            )));
        }
        if end < self.end {
            // Native leaves the buffers in the ring until the next extension
            // nulls them; no read can observe them in between.
            self.slots.split_off(&end);
        }
        self.end = end;
        Ok(())
    }
    /// 0x750880: release [begin, min(new, end)), begin = new, end = max(end, new).
    pub fn retire_to(&mut self, begin: i32) -> Result<()> {
        if begin < self.begin {
            return Err(Error::Protocol(format!(
                "block window retired backwards from {} to {begin} (native re-exposes stale ring slots)",
                self.begin
            )));
        }
        let kept = self.slots.split_off(&begin);
        self.slots = kept;
        self.begin = begin;
        if self.end < begin {
            self.end = begin;
        }
        Ok(())
    }
    /// 0x766FC0 into slot `index` (inside the window).
    fn put(&mut self, index: i32, buffer: Vec<u8>) {
        debug_assert!(self.begin <= index && index < self.end);
        self.slots.insert(index, buffer);
    }
}

fn block_size(config: &FileTransferConfig) -> Result<i32> {
    match config.block_size {
        0 => Err(Error::Protocol("NetworkFileTransferBlockSize is 0 (native idiv fault)".into())),
        bs => Ok(bs),
    }
}

/// L = (size + BS - 1) / BS as `lea` + `idiv` (0x76144D, 0x7617FB, 0x7612E4,
/// 0x76124A).
pub fn terminal_index(size: i32, block_size: i32) -> Result<i32> {
    size.wrapping_add(block_size)
        .wrapping_sub(1)
        .checked_div(block_size)
        .ok_or_else(|| Error::Protocol(format!("terminal index of size {size} with block size {block_size} faults (idiv)")))
}

fn remainder(size: i32, block_size: i32) -> Result<i32> {
    size.checked_rem(block_size)
        .ok_or_else(|| Error::Protocol(format!("size {size} % block size {block_size} faults (idiv)")))
}

/// Sender block length 0x761420: (length, terminal).
pub fn sender_block_length(config: &FileTransferConfig, size: i32, index: i32) -> Result<(i32, bool)> {
    let bs = block_size(config)?;
    let terminal = index == terminal_index(size, bs)?;
    if !terminal {
        return Ok((bs, false));
    }
    let r = remainder(size, bs)?;
    Ok(match config.tail {
        TerminalTail::Original => (r, true),
        // Cave 0x9EFF80: test edx, edx; jne; mov edx, ecx.
        TerminalTail::V14 => (if r == 0 { bs } else { r }, true),
    })
}

/// Receiver write length 0x761220 for a block whose payload has
/// `payload_len` bytes: (length, terminal).
pub fn receiver_block_length(
    config: &FileTransferConfig,
    size: i32,
    index: i32,
    payload_len: usize,
) -> Result<(i32, bool)> {
    let bs = block_size(config)?;
    let terminal = index == terminal_index(size, bs)?;
    if !terminal {
        return Ok((bs, false));
    }
    let r = remainder(size, bs)?;
    Ok(match config.tail {
        TerminalTail::Original => (r, true),
        // Cave 0x9EFF90: remainder 0 -> buffer length, capped at BS (jbe,
        // unsigned).
        TerminalTail::V14 if r == 0 => {
            let len = payload_len as u32;
            (if len <= bs as u32 { len as i32 } else { bs }, true)
        }
        TerminalTail::V14 => (r, true),
    })
}

/// Header block bitstream (0x761540: 0x40D420 writes then finalizer 0x74D0A0).
pub fn encode_header(name: &[u16], size: i32) -> Result<Vec<u8>> {
    let units = name.len() & 0xFFFF; // movzx di (0x7616CE)
    let mut w = WireWriter::new(Position::PACKET_START)?;
    w.write_uint(units as u32, 16)?;
    let raw: Vec<u8> = name[..units].iter().flat_map(|u| u.to_le_bytes()).collect();
    w.write_raw_bits(&raw, units * 16)?;
    w.write_uint(size as u32, 32)?;
    Ok(finalize(&w)?)
}

/// Header block parser (0x7619A0 before the path check): name, size. The
/// reader starts at bit 3 (0x74AF40) and is bounded by the buffer length (the
/// native reader would read past it).
pub fn decode_header(block: &[u8]) -> Result<(Vec<u16>, i32)> {
    let mut r = Cursor::with_bounds(block, Position::PACKET_START, block.len() * 8)
        .map_err(|_| Error::Protocol("header block shorter than its 3-bit pad field".into()))?;
    let units = r.read_uint(16)? as usize;
    let raw = r.read_raw_bits(units * 16)?;
    let name = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let size = r.read_uint(32)? as i32;
    Ok((name, size))
}

/// Block payload writer 0x75FFA0.
pub fn encode_payload(w: &mut WireWriter, bytes: &[u8]) -> Result<()> {
    w.write_uint(bytes.len() as u32, 32)?;
    for &b in bytes {
        w.write_uint(u32::from(b), 8)?;
    }
    Ok(())
}

/// Block payload reader 0x7610A0 (each byte via 0x411FA0(256)).
pub fn decode_payload(r: &mut Cursor<'_>) -> Result<Vec<u8>> {
    let n = r.read_uint(32)? as usize;
    let needed = n.saturating_mul(8);
    if needed > r.remaining_bits() {
        return Err(aa_orders::Error::Truncated { requested: needed, remaining: r.remaining_bits() }.into());
    }
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(r.read_possibility(256)? as u8);
    }
    Ok(out)
}

fn concat(a: &[u16], b: &[u16]) -> Vec<u16> {
    // 0x4108C0 -> 0x40F4B0: length-based, embedded NULs kept.
    [a, b].concat()
}

/// TGC_FileSender without its BlockSender base, which the caller owns (it is
/// driven through [`BlockSource`]).
#[derive(Debug, Clone)]
pub struct FileSender<F: FileStore> {
    pub config: FileTransferConfig,
    /// [[0xB36540]+0x90]+4: the profile directory the name is appended to.
    pub profile_dir: Vec<u16>,
    /// +0x3C.
    pub file: F,
    /// +0x2C.
    pub queue: BlockWindow,
    /// +0x48 (-1 = nothing to send).
    pub size: i32,
}
impl<F: FileStore> FileSender<F> {
    /// 0x7611B0 followed by AdminConn reset 0x752110 -> 0x761400 (the native
    /// ctor leaves +0x48 unset; 0x756BC0 always resets right after).
    pub fn new(config: FileTransferConfig, profile_dir: Vec<u16>, file: F) -> Self {
        Self { config, profile_dir, file, queue: BlockWindow::cleared(), size: -1 }
    }
    /// The BlockSender base built by 0x7611B0: 0x760390(span =
    /// BlocksPerFrame, copies = 1).
    pub fn new_channel(config: &FileTransferConfig) -> BlockSender {
        BlockSender::new(config.blocks_per_frame, 1)
    }
    /// 0x761400.
    pub fn reset(&mut self, channel: &mut BlockSender) {
        channel.reset();
        self.queue.clear();
        self.size = -1;
    }
    /// 0x761540. An open failure (TGC_FileException) propagates as in the
    /// original, after the channel and queue were reset and before the size
    /// and header change.
    pub fn start(&mut self, channel: &mut BlockSender, name: &[u16]) -> Result<()> {
        channel.reset();
        self.queue.clear();
        self.queue.retire_to(0)?;
        let path = concat(&self.profile_dir, name);
        self.file.open(&path, MODE_SENDER_READ)?;
        self.size = self.file.size()?;
        let header = encode_header(name, self.size)?;
        let index = self.queue.end();
        self.queue.extend_to(next(index)?)?;
        self.queue.put(index, header);
        Ok(())
    }
    /// 0x761420: read block `end` into a zero-filled buffer, close after the
    /// terminal block.
    fn read_block(&mut self) -> Result<()> {
        let index = self.queue.end();
        let (len, terminal) = sender_block_length(&self.config, self.size, index)?;
        let len = usize::try_from(len)
            .map_err(|_| Error::Protocol(format!("negative block length {len} (size {})", self.size)))?;
        let mut buffer = Vec::new();
        buffer.try_reserve_exact(len).map_err(|_| aa_orders::Error::Allocation)?;
        buffer.resize(len, 0);
        let _ignored = self.file.read(&mut buffer)?;
        self.queue.extend_to(next(index)?)?;
        self.queue.put(index, buffer);
        if terminal {
            self.file.close()?;
        }
        Ok(())
    }
    /// 0x7617E0.
    fn read_ahead(&mut self) -> Result<()> {
        loop {
            let last = terminal_index(self.size, block_size(&self.config)?)?;
            if self.queue.end() > last {
                return Ok(());
            }
            if self.queue.end().wrapping_sub(self.queue.begin()) >= self.config.max_pending_blocks {
                return Ok(());
            }
            self.read_block()?;
        }
    }
    /// 0x7612D0: acknowledged byte progress (released blocks).
    pub fn progress(&self) -> Result<i32> {
        let bs = block_size(&self.config)?;
        let last = terminal_index(self.size, bs)?;
        if self.queue.begin() == last.wrapping_add(1) {
            return Ok(self.size);
        }
        Ok(self.queue.begin().wrapping_sub(2).wrapping_mul(bs))
    }
}

fn next(index: i32) -> Result<i32> {
    index
        .checked_add(1)
        .ok_or_else(|| Error::Protocol("block index 2^31-1 wraps the window end (native sets end to INT_MIN)".into()))
}

impl<F: FileStore> BlockSource for FileSender<F> {
    /// 0x761920.
    fn available(&mut self) -> Result<i32> {
        if self.size != -1 {
            self.read_ahead()?;
        }
        Ok(self.queue.end())
    }
    /// 0x761850.
    fn release(&mut self, index: i32) -> Result<()> {
        self.queue.retire_to(index)?;
        self.read_ahead()
    }
    /// 0x761310: 32 + bits of the buffer read as a finalized bitstream
    /// (8 * len - 3 - (byte0 & 7)); 32 for a null slot. For an empty buffer the
    /// native reads the heap byte after it; 0 is used here (what the oracle's
    /// zero-filled heap shows).
    fn estimate(&mut self, index: i32) -> Result<i32> {
        if index >= self.queue.end() {
            self.queue.extend_to(next(index)?)?;
        }
        Ok(match self.queue.get(index) {
            None => 32,
            Some(b) => {
                let pad = i32::from(b.first().copied().unwrap_or(0) & 7);
                (b.len() as u32).wrapping_mul(8).wrapping_sub(3).wrapping_sub(pad as u32).wrapping_add(32) as i32
            }
        })
    }
    /// 0x7613C0 -> 0x75FFA0.
    fn serialize(&mut self, w: &mut WireWriter, index: i32) -> Result<()> {
        if index >= self.queue.end() {
            self.queue.extend_to(next(index)?)?;
        }
        let buffer = self.queue.get(index).ok_or_else(|| {
            Error::Protocol(format!("file block {index} has no buffer (native dereferences null in 0x766F80)"))
        })?;
        encode_payload(w, buffer)
    }
}

/// TGC_FileReceiver (its BlockReceiver base is `channel`).
#[derive(Debug, Clone)]
pub struct FileReceiver<F: FileStore> {
    pub config: FileTransferConfig,
    pub profile_dir: Vec<u16>,
    /// +4: span = BlocksPerFrame at construction (0x75FE00).
    pub channel: BlockReceiver,
    /// +8.
    pub queue: BlockWindow,
    /// +0x18.
    pub file: F,
    /// +0x24 (-1 before a header).
    pub size: i32,
    /// Log lines in emission order (callers may drain it).
    pub log: Vec<FileLog>,
}
impl<F: FileStore> FileReceiver<F> {
    /// 0x761870.
    pub fn new(config: FileTransferConfig, profile_dir: Vec<u16>, file: F) -> Result<Self> {
        let mut receiver = Self {
            config,
            profile_dir,
            channel: BlockReceiver { span: config.blocks_per_frame },
            queue: BlockWindow::cleared(),
            file,
            size: -1,
            log: Vec::new(),
        };
        receiver.reset()?;
        Ok(receiver)
    }
    /// 0x761500 (also the tail of the ctor 0x7618CB..0x7618FE).
    pub fn reset(&mut self) -> Result<()> {
        if self.file.is_open() {
            self.file.close()?;
        }
        self.queue.clear();
        self.queue.retire_to(0)?;
        self.size = -1;
        Ok(())
    }
    /// 0x761CE0: process contiguous blocks from `begin`.
    fn drain(&mut self) -> Result<()> {
        while self.queue.begin() < self.queue.end() {
            let index = self.queue.begin();
            let Some(buffer) = self.queue.get(index).cloned() else {
                return Ok(());
            };
            if index == 0 {
                self.unblock_header(&buffer)?;
            } else {
                self.write_block(index, &buffer)?;
            }
            self.queue.retire_to(self.queue.begin() + 1)?;
        }
        Ok(())
    }
    /// 0x7619A0 TGC_FileReceiver::UnblockHeader.
    fn unblock_header(&mut self, block: &[u8]) -> Result<()> {
        let (name, size) = decode_header(block)?;
        self.size = size;
        // _wcsstr(name, L"..") sees the string up to its first NUL.
        let visible = &name[..name.iter().position(|&u| u == 0).unwrap_or(name.len())];
        if visible.windows(2).any(|w| w == [0x2E, 0x2E]) {
            self.log.push(FileLog::DotDotRejected { name });
            return Ok(());
        }
        self.log.push(FileLog::StartingDownload { name: name.clone(), size });
        let path = concat(&self.profile_dir, &name);
        if self.file.open(&path, MODE_RECEIVER_WRITE).is_err() {
            self.log.push(FileLog::OpenFailed { name });
        }
        Ok(())
    }
    /// 0x761220.
    fn write_block(&mut self, index: i32, block: &[u8]) -> Result<()> {
        let (len, terminal) = receiver_block_length(&self.config, self.size, index, block.len())?;
        if self.file.is_open() {
            let n = usize::try_from(len)
                .ok()
                .filter(|&n| n <= block.len())
                .ok_or_else(|| {
                    Error::Protocol(format!(
                        "block {index} write length {len} exceeds its {}-byte payload (native heap over-read in 0x761220)",
                        block.len()
                    ))
                })?;
            self.file.write(&block[..n])?;
            if terminal {
                self.file.close()?;
            }
        }
        Ok(())
    }
}
impl<F: FileStore> BlockSink for FileReceiver<F> {
    /// 0x761DB0.
    fn receive(&mut self, r: &mut Cursor<'_>, index: i32) -> Result<()> {
        let buffer = decode_payload(r)?;
        if index >= self.queue.begin() {
            if index >= self.queue.end() {
                self.queue.extend_to(next(index)?)?;
            }
            self.queue.put(index, buffer);
            self.drain()?;
        }
        Ok(())
    }
    /// 0x761150.
    fn skip(&mut self, r: &mut Cursor<'_>, _index: i32) -> Result<()> {
        decode_payload(r).map(drop)
    }
}

/// Result of the generation check 0x74C8B0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Generation {
    /// Older than ours: section skipped (0x75FEA0), then log line 0x168C
    /// "NET: Ignoring old file index %d (my index = %d)".
    Old { packet: i32, mine: i32 },
    /// Equal: section received (0x760F20).
    Current,
    /// Newer: log line 0x1694 "NET: Resetting to file transfer index %d (my
    /// index = %d)", receiver reset 0x761500, section received, generation
    /// stored (after the receive).
    Newer { packet: i32, mine: i32 },
}

/// 0x74C8B0 TGC_AdminConn::SerializeFileTransfer (read side): 32-bit
/// generation, then the file channel section through `channel`
/// (AdminConn+0x3E8 is both the BlockReceiver and the FileReceiver).
pub fn receive_file_section<F: FileStore>(
    generation: &mut i32,
    channel: &BlockReceiver,
    receiver: &mut FileReceiver<F>,
    r: &mut Cursor<'_>,
) -> Result<Generation> {
    let packet = r.read_uint(32)? as i32;
    let mine = *generation;
    if packet < mine {
        channel.skip(receiver, r)?;
        return Ok(Generation::Old { packet, mine });
    }
    if packet == mine {
        channel.receive(receiver, r)?;
        return Ok(Generation::Current);
    }
    receiver.reset()?;
    channel.receive(receiver, r)?;
    *generation = packet;
    Ok(Generation::Newer { packet, mine })
}

/// 0x74FE30: increment AdminConn+0x418, then 0x761540 with `name`.
pub fn start_file_transfer<F: FileStore>(
    generation: &mut i32,
    channel: &mut BlockSender,
    sender: &mut FileSender<F>,
    name: &[u16],
) -> Result<()> {
    *generation = generation.wrapping_add(1);
    sender.start(channel, name)
}
