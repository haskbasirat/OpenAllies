//! Map and film transfer in a LAN session: the file channel of every
//! TGC_AdminConn (FileSender +0x39C, FileReceiver +0x3E8; crate::file has
//! both, oracle-proven) and the three native steps that drive it:
//!
//! * a client that lacks the session's file asks for it: SendUpdateMapStatus
//!   0x770B70 (here [`map_status`]) requests TGC_SetMapTransferStatus (3)
//!   with status 2;
//! * the host starts the transfer on that client's connection:
//!   SetMapTransferStatus 0x7554B0 with status 2 calls 0x74FE30 with the
//!   session's file name [session+0x5C] (generation +0x418 += 1, then
//!   FileSender::Start 0x761540 on profile directory + name);
//! * every async heartbeat 0x75B820 the host runs UpdateFileTransferStatus
//!   0x74F310 on each connection of its list +0x2AC (AdminConn vtable +4
//!   0x8D6180 returns the connection, the node's own entry 0x4C9FA0 returns
//!   NULL): while +0x38 is 2 it sends UpdateMapStatus (305) {player +8,
//!   status +0x38, progress 0x7612D0 / size 0x7A6610}, and +0x38 becomes 1
//!   once the acknowledged progress reaches the size.
//!
//! The blocks travel in the type-5 ServerToClient section (0x74E280 writer,
//! 0x74C8B0 reader) that [`crate::packets`] already writes and reads; the
//! receiver writes profile directory + the header's name (0x7619A0, ".."
//! refused).
//!
//! The profile directory ([[0xB36540]+0x90]+4) comes from
//! [`super::LanData::profile_directory`]. A front end that gives none keeps
//! the connection's channel empty, as before this module: the host then says
//! the map will not transfer (its message for status 3).
use crate::file::{map_checksum, FileReceiver, FileSender, FileTransferConfig, StdFile, TerminalTail};
use crate::packets::{FileChannel, FileReceiving, FileSending, NetworkAvars, NoFileTransfer};
use crate::Result;
use aa_orders::commands::WireWriter;
use aa_orders::Cursor;

/// What the front end's resource lookup knows of a session file (0x59F570
/// and, when found, 0x858030 over the resource's bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapLookup {
    /// The front end does not look: the client reports it has the file
    /// (status 1), the shortcut this port used before the lookup existed.
    Unchecked,
    /// 0x59F570 found no resource of that name.
    Missing,
    /// The resource exists; its checksum (0x858030).
    Found { crc: u32 },
}
impl MapLookup {
    /// The lookup of a file the front end can read in full.
    pub fn of_bytes(bytes: Option<&[u8]>) -> Self {
        match bytes {
            Some(b) => Self::Found { crc: map_checksum(b) },
            None => Self::Missing,
        }
    }
}

/// What SendUpdateMapStatus 0x770B70 does for (file, crc, source check).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapStatus {
    /// TGC_SetMapTransferStatusOrderRequest (3) {status, [session+0x5C],
    /// [session+0x154]} and the native return value.
    Request { status: u32, returns: bool },
    /// Status 3 and the message map_will_not_transfer_client_film_source|s
    /// (0x770CAA / 0x770F8D) with [[session+0x68]+0x24]; returns false.
    FilmSource,
    /// The file differs and its name has no ".." (0x770E7A wcsstr): the
    /// event of 0x7844F0 / 0x7519C0 (kind 0xE) is posted to [0xB40BA8]
    /// (0x7851C0); no request; returns false. What reads that event is not
    /// traced.
    Differs,
    /// Nothing sent; the native return value.
    Nothing { returns: bool },
}

/// 0x770B70 KKC_Session::SendUpdateMapStatus(file, crc, source check).
pub fn map_status(lookup: MapLookup, file: &[u16], crc: u32, source_check: bool) -> MapStatus {
    match lookup {
        MapLookup::Unchecked => {
            if source_check {
                MapStatus::Nothing { returns: true }
            } else {
                MapStatus::Request { status: 1, returns: true }
            }
        }
        // 0x770BD6: not found.
        MapLookup::Missing => {
            if source_check {
                MapStatus::FilmSource
            } else {
                MapStatus::Request { status: 2, returns: false }
            }
        }
        // 0x770E5C: the computed checksum against the session's.
        MapLookup::Found { crc: mine } if mine != crc => {
            if source_check {
                MapStatus::FilmSource
            } else if file[..file.iter().position(|&c| c == 0).unwrap_or(file.len())].windows(2).any(|w| w == [0x2E, 0x2E]) {
                // wcsstr (0x4054B0) stops at the first NUL.
                MapStatus::Nothing { returns: false }
            } else {
                MapStatus::Differs
            }
        }
        MapLookup::Found { .. } => {
            if source_check {
                MapStatus::Nothing { returns: true }
            } else {
                MapStatus::Request { status: 1, returns: true }
            }
        }
    }
}

/// The file channel of one connection (AdminConn +0x39C and +0x3E8).
pub(crate) enum ConnFile {
    /// No profile directory: the constructed, never started channel.
    Empty(NoFileTransfer),
    Ported { send: FileChannel<StdFile>, receive: FileReceiver<StdFile> },
}
impl ConnFile {
    /// AdminConn ctor 0x756BC0 (FileSender 0x7611B0, FileReceiver
    /// 0x761870) with the AVars of 0x403760..0x40380C and the v14 terminal
    /// rule (docs/BASELINE.txt).
    pub(crate) fn new(avars: &NetworkAvars, profile_dir: Option<&[u16]>) -> Result<Self> {
        let Some(dir) = profile_dir else { return Ok(Self::Empty(NoFileTransfer::new(avars))) };
        let config = FileTransferConfig {
            blocks_per_frame: avars.file_blocks_per_frame,
            block_size: avars.file_block_size,
            max_pending_blocks: avars.file_max_pending_blocks,
            tail: TerminalTail::V14,
        };
        let mut sender = FileSender::new(config, dir.to_vec(), StdFile::default());
        let mut channel = FileSender::<StdFile>::new_channel(&config);
        sender.reset(&mut channel);
        let receive = FileReceiver::new(config, dir.to_vec(), StdFile::default())?;
        Ok(Self::Ported { send: FileChannel { channel, sender }, receive })
    }
    /// 0x74FE30: generation +1, then FileSender::Start 0x761540 with `name`.
    /// `Ok(false)`: this connection has no file channel.
    pub(crate) fn start(&mut self, generation: &mut i32, name: &[u16]) -> Result<bool> {
        match self {
            Self::Empty(_) => Ok(false),
            Self::Ported { send, .. } => {
                crate::file::start_file_transfer(generation, &mut send.channel, &mut send.sender, name)?;
                Ok(true)
            }
        }
    }
    /// 0x7612D0 progress and 0x7A6610 size (+0x48) of the sender.
    pub(crate) fn progress(&self) -> Result<Option<(i32, i32)>> {
        match self {
            Self::Empty(_) => Ok(None),
            Self::Ported { send, .. } => Ok(Some((send.sender.progress()?, send.sender.size))),
        }
    }
    /// The receiver's log lines (0x7619A0) since the last call.
    pub(crate) fn take_log(&mut self) -> Vec<crate::file::FileLog> {
        match self {
            Self::Empty(_) => Vec::new(),
            Self::Ported { receive, .. } => std::mem::take(&mut receive.log),
        }
    }
}
impl FileSending for ConnFile {
    fn serialize(&mut self, w: &mut WireWriter, sequence: i32, limit: i32) -> Result<()> {
        match self {
            Self::Empty(f) => f.serialize(w, sequence, limit),
            Self::Ported { send, .. } => send.serialize(w, sequence, limit),
        }
    }
    fn packet_result(&mut self, sequence: i32, acked: bool) -> Result<()> {
        match self {
            Self::Empty(f) => f.packet_result(sequence, acked),
            Self::Ported { send, .. } => send.packet_result(sequence, acked),
        }
    }
    fn reset(&mut self) -> Result<()> {
        match self {
            Self::Empty(f) => FileSending::reset(f),
            Self::Ported { send, .. } => FileSending::reset(send),
        }
    }
    fn unsent_blocks_pending(&self) -> bool {
        match self {
            Self::Empty(f) => f.unsent_blocks_pending(),
            Self::Ported { send, .. } => send.unsent_blocks_pending(),
        }
    }
}
impl FileReceiving for ConnFile {
    fn receive(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        match self {
            Self::Empty(f) => f.receive(r),
            Self::Ported { receive, .. } => receive.receive(r),
        }
    }
    fn skip(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        match self {
            Self::Empty(f) => f.skip(r),
            Self::Ported { receive, .. } => receive.skip(r),
        }
    }
    fn reset(&mut self) -> Result<()> {
        match self {
            Self::Empty(f) => FileReceiving::reset(f),
            Self::Ported { receive, .. } => FileReceiving::reset(receive),
        }
    }
}

/// 0x74F310 fraction (0x74F3C7..0x74F403): fild progress, fstp dword; fild
/// size, fstp dword; then fld dword, fdiv dword, fstp dword. Both operands
/// are rounded to binary32 (nearest) before the divide, and a quotient of
/// two binary32 values rounded to 24, 53 or 64 bits and then stored as
/// binary32 is the correctly rounded binary32 quotient under every
/// precision control, so the thread's setting does not matter.
pub(crate) fn fraction(progress: i32, size: i32) -> f32 {
    progress as f32 / size as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_is_times_seven_plus_byte() {
        // 0x85806A: imul edi, edi, 7; add edi, byte.
        assert_eq!(map_checksum(&[]), 0);
        assert_eq!(map_checksum(&[1]), 1);
        assert_eq!(map_checksum(&[1, 2]), 9);
        assert_eq!(map_checksum(&[0xff, 0xff, 0xff]), 0xff * 49 + 0xff * 7 + 0xff);
        // wrapping
        let many = vec![0xffu8; 64];
        let mut c = 0u32;
        for _ in 0..64 {
            c = c.wrapping_mul(7).wrapping_add(0xff);
        }
        assert_eq!(map_checksum(&many), c);
    }

    /// What 0x858030 itself returned under emulation
    /// (research/phase8-net/native_map_checksum.py on the accepted AA.exe).
    #[test]
    fn checksum_matches_the_original_function() {
        let named: Vec<u8> = b"Maps\\(2v2)Transfer.RMP".repeat(37);
        let all: Vec<u8> = (0..=255).collect();
        let cases: [(&[u8], u32); 7] = [
            (b"", 0),
            (b"\x01", 1),
            (b"\x01\x02", 9),
            (b"\xff\xff\xff", 0x38c7),
            (&all, 0xce41_f080),
            (&[0xff; 64], 0xff1f_1700),
            (&named, 0xe541_0cd8),
        ];
        for (bytes, native) in cases {
            assert_eq!(map_checksum(bytes), native, "{} bytes", bytes.len());
        }
    }

    #[test]
    fn map_status_follows_0x770b70() {
        let file: Vec<u16> = "Maps/x.RMP".encode_utf16().collect();
        let dots: Vec<u16> = "Maps/../x.RMP".encode_utf16().collect();
        assert_eq!(map_status(MapLookup::Missing, &file, 5, false), MapStatus::Request { status: 2, returns: false });
        assert_eq!(map_status(MapLookup::Missing, &file, 5, true), MapStatus::FilmSource);
        assert_eq!(map_status(MapLookup::Found { crc: 5 }, &file, 5, false), MapStatus::Request { status: 1, returns: true });
        assert_eq!(map_status(MapLookup::Found { crc: 5 }, &file, 5, true), MapStatus::Nothing { returns: true });
        assert_eq!(map_status(MapLookup::Found { crc: 4 }, &file, 5, false), MapStatus::Differs);
        assert_eq!(map_status(MapLookup::Found { crc: 4 }, &dots, 5, false), MapStatus::Nothing { returns: false });
        assert_eq!(map_status(MapLookup::Found { crc: 4 }, &file, 5, true), MapStatus::FilmSource);
    }
}
