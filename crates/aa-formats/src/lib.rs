//! Checked, read-only readers for the locally recovered game formats.
//!
//! This crate implements the **uncompressed TGCK version 2 RWD** container
//! and, in [`nif`], a port of AA.exe's NiStream NIF reader.

#![forbid(unsafe_code)]

pub mod nif;
pub mod rwd;

pub use rwd::{ArchiveError, Chunk, Entry, RwdArchive};
