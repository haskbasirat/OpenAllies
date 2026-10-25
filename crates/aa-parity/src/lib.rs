//! Film parity harness (research/byte-exact/ROADMAP.md, items H1–H3 and §5.3).
//!
//! * [`film`]: decodes an original AA.exe film (`.RFM`, a TGCK chunk file).
//!   It reads the MAP_HEADER with its creator class (`KKC_WorldCreator`,
//!   `KKC_WorldCreatorLoad`, `KKC_WorldCreatorRandom`) and seed, the
//!   SessionPlayers roster, and the Orders chunk, one record per world tick
//!   (0x74D640). Every record is decoded through aa-orders (the native
//!   descriptor registry and payload schemas) into typed [`decree::Decree`]s.
//!   The `KKC_SetSimSyncValueOrderDecree` (311) checkpoints are collected.
//! * [`mapping`]: maps film decrees onto `aa_play::Command`s through the
//!   [`mapping::NativeIds`] table that aa-play provides (native id slots,
//!   players, definition registry indices).
//! * [`slog`]: recomputes the original simulation sync log S (`[0xB36560]`,
//!   `aa_orders::net::sync::Synchronizer`) from an entry stream and compares
//!   it with the film's checkpoints.
//! * [`registries`]: the native definition-registry counts payload decoding
//!   needs ([`aa_orders::payload::SchemaContext`]).
//! * [`replay`]: the H4 replay driver: a film's decrees into aa-play on the
//!   film's own stock map, checkpoint by checkpoint.
//!
//! Nothing here reproduces a simulation; it measures one.
pub mod chain;
pub mod classify;
pub mod decree;
pub mod film;
#[cfg(feature = "play")]
pub mod mapping;
pub mod registries;
#[cfg(feature = "play")]
pub mod replay;
pub mod slog;

pub use decree::Decree;
pub use film::{Checkpoint, Film, FilmDecree, FilmHeader, SessionPlayer, TickRecord};
#[cfg(feature = "play")]
pub use mapping::{Mapped, Mapper, NativeIds};
pub use registries::Registries;
pub use slog::{SEntryKind, SLog};

/// Errors of the film container and header. Per-record decree decode
/// failures are not errors: they are kept in [`TickRecord::error`].
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("film file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("film container: {0}")]
    Container(#[from] aa_mapworld::Error),
    #[error("film chunk {chunk}: {message}")]
    Chunk {
        chunk: &'static str,
        message: String,
    },
    #[error("order registry: {0}")]
    Orders(#[from] aa_orders::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
