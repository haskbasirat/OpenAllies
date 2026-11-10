//! The original's map editor, ported piece by piece
//! (`research/phase8-editor/README.txt` has the plan).
//!
//! * [`deflate`]: the zlib 1.1.4 deflater AA.exe links, which compresses
//!   map chunks.
//! * [`map_file`]: the map file writer, the inverse of the map reader
//!   (`aa_mapworld::MapArchive`), proven on every shipped map
//!   (`tests/shipped_maps.rs`).
#![forbid(unsafe_code)]
pub mod deflate;
pub mod map_file;
