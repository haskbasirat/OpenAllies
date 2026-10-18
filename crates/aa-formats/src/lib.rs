//! Checked, read-only readers for the locally recovered game formats.
//!
//! This crate currently implements the **uncompressed TGCK version 2 RWD**
//! container, not the game engine or the formats inside its members.

#![forbid(unsafe_code)]

pub mod rwd;

pub use rwd::{ArchiveError, Chunk, Entry, RwdArchive};
