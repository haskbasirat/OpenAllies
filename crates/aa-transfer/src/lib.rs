//! A small streaming transfer protocol for the independent rebuild.
//!
//! This is not the original game's multiplayer protocol. It has no authentication,
//! encryption, resume, acknowledgement, compression, rate limiter, or socket setup.
//! Callers supply a reliable ordered stream, authorization, cancellation/timeouts,
//! and their own disk/transfer policy. A valid SHA-256 trailer detects corruption;
//! it does not authenticate a sender who can replace both payload and digest.
//!
//! Framing is `AAXFER01`, little-endian `u64` length, exactly that many payload
//! bytes, and a 32-byte SHA-256 digest of the payload. One receive consumes one
//! frame, leaving subsequent bytes available. Length controls framing, never an
//! allocation. The 64 KiB copy buffer bounds memory, not payload size or bandwidth.
//!
//! A sender's successful return means the frame was written and flushed, not that
//! the remote application verified it. Receivers may have written unverified
//! bytes before an error. Never use the output until receive returns success.

#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

pub const MAGIC: [u8; 8] = *b"AAXFER01";
pub const HEADER_BYTES: usize = 16;
pub const DIGEST_BYTES: usize = 32;
/// Working storage only; this is not a transfer-size or throughput limit.
pub const COPY_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Receipt {
    pub length: u64,
    pub sha256: [u8; DIGEST_BYTES],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    HeaderRead,
    HeaderWrite,
    PayloadRead,
    PayloadWrite,
    DigestRead,
    DigestWrite,
    Flush,
    SourceOpen,
    SourceMetadata,
    DestinationCreate,
    DestinationSync,
}

#[derive(Debug)]
pub enum TransferError {
    Io {
        stage: Stage,
        source: io::Error,
    },
    InvalidMagic {
        found: [u8; 8],
    },
    HashMismatch {
        transmitted: [u8; DIGEST_BYTES],
        computed: [u8; DIGEST_BYTES],
    },
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { stage, source } => write!(f, "transfer {stage:?}: {source}"),
            Self::InvalidMagic { .. } => write!(f, "unsupported transfer magic or version"),
            Self::HashMismatch { .. } => write!(f, "payload SHA-256 verification failed"),
        }
    }
}

impl std::error::Error for TransferError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn io_at(stage: Stage, source: io::Error) -> TransferError {
    TransferError::Io { stage, source }
}

/// Send exactly `length` bytes in one pass, hashing the same bytes written.
///
/// Early source EOF is an error; any source bytes beyond `length` remain unread.
/// A failure may leave a partial frame on the transport, which must not be reused
/// unless the caller has an independent resynchronization mechanism. This API
/// adds no sleeps or throughput limit; blocking I/O supplies backpressure.
pub fn send<R: Read + ?Sized, W: Write + ?Sized>(
    source: &mut R,
    transport: &mut W,
    length: u64,
) -> Result<Receipt, TransferError> {
    let mut header = [0u8; HEADER_BYTES];
    header[..8].copy_from_slice(&MAGIC);
    header[8..].copy_from_slice(&length.to_le_bytes());
    transport
        .write_all(&header)
        .map_err(|e| io_at(Stage::HeaderWrite, e))?;
    let sha256 = copy_hashed(source, transport, length)?;
    transport
        .write_all(&sha256)
        .map_err(|e| io_at(Stage::DigestWrite, e))?;
    transport.flush().map_err(|e| io_at(Stage::Flush, e))?;
    Ok(Receipt { length, sha256 })
}

/// Receive and verify one frame. The sink is flushed only after digest validation.
///
/// The sink can contain partial or corrupt bytes on failure. This function does
/// not allocate according to the advertised length or choose any filesystem path.
pub fn receive<R: Read + ?Sized, W: Write + ?Sized>(
    transport: &mut R,
    destination: &mut W,
) -> Result<Receipt, TransferError> {
    let length = read_header(transport)?;
    receive_body(transport, destination, length)
}

/// Open a caller-selected source read-only and stream its current metadata length.
///
/// This does not lock/snapshot a changing source: a shrink fails; concurrent edits
/// or growth require caller coordination if a stable artifact is needed.
pub fn send_file<W: Write + ?Sized>(
    source_path: impl AsRef<Path>,
    transport: &mut W,
) -> Result<Receipt, TransferError> {
    let mut source = File::open(source_path).map_err(|e| io_at(Stage::SourceOpen, e))?;
    let length = source
        .metadata()
        .map_err(|e| io_at(Stage::SourceMetadata, e))?
        .len();
    send(&mut source, transport, length)
}

/// Receive into a caller-selected, atomically create-new file and sync on success.
///
/// The wire contains no path. Existing destinations (including symlinks) are not
/// overwritten, and parent directories are not created. Use a trusted parent
/// directory: this API is not a defense against hostile concurrent ancestor
/// renames. Invalid headers create nothing. After a later failure, the new partial
/// file is deliberately retained; no automatic deletion can remove a path another
/// actor replaced. The caller must not publish/use that file on `Err` and owns its
/// explicit cleanup. There is no atomic-publish or resumable-transfer claim.
pub fn receive_file_new<R: Read + ?Sized>(
    transport: &mut R,
    destination_path: impl AsRef<Path>,
) -> Result<Receipt, TransferError> {
    let length = read_header(transport)?;
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination_path)
        .map_err(|e| io_at(Stage::DestinationCreate, e))?;
    let receipt = receive_body(transport, &mut destination, length)?;
    destination
        .sync_all()
        .map_err(|e| io_at(Stage::DestinationSync, e))?;
    Ok(receipt)
}

fn read_header<R: Read + ?Sized>(transport: &mut R) -> Result<u64, TransferError> {
    let mut header = [0u8; HEADER_BYTES];
    transport
        .read_exact(&mut header)
        .map_err(|e| io_at(Stage::HeaderRead, e))?;
    let mut found = [0; 8];
    found.copy_from_slice(&header[..8]);
    if found != MAGIC {
        return Err(TransferError::InvalidMagic { found });
    }
    Ok(u64::from_le_bytes(header[8..].try_into().unwrap()))
}

fn receive_body<R: Read + ?Sized, W: Write + ?Sized>(
    transport: &mut R,
    destination: &mut W,
    length: u64,
) -> Result<Receipt, TransferError> {
    let computed = copy_hashed(transport, destination, length)?;
    let mut transmitted = [0u8; DIGEST_BYTES];
    transport
        .read_exact(&mut transmitted)
        .map_err(|e| io_at(Stage::DigestRead, e))?;
    if transmitted != computed {
        return Err(TransferError::HashMismatch {
            transmitted,
            computed,
        });
    }
    destination.flush().map_err(|e| io_at(Stage::Flush, e))?;
    Ok(Receipt {
        length,
        sha256: computed,
    })
}

fn copy_hashed<R: Read + ?Sized, W: Write + ?Sized>(
    source: &mut R,
    destination: &mut W,
    length: u64,
) -> Result<[u8; DIGEST_BYTES], TransferError> {
    let mut buffer = [0u8; COPY_BUFFER_BYTES];
    let mut remaining = length;
    let mut hash = Sha256::new();
    while remaining != 0 {
        let requested = remaining.min(buffer.len() as u64) as usize;
        let read = match source.read(&mut buffer[..requested]) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result.map_err(|e| io_at(Stage::PayloadRead, e))?,
        };
        if read == 0 {
            return Err(io_at(
                Stage::PayloadRead,
                io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "payload ended before declared length",
                ),
            ));
        }
        destination
            .write_all(&buffer[..read])
            .map_err(|e| io_at(Stage::PayloadWrite, e))?;
        hash.update(&buffer[..read]);
        remaining -= read as u64;
    }
    Ok(hash.finalize().into())
}
