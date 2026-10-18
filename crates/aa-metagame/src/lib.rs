//! Port of the original AA.exe World War II strategic layer (metagame),
//! reconstructed from native code with Unicorn oracle tests.
//!
//! Scope of this crate (see `research/phase8-metagame/README.txt` for the
//! address inventory, proofs and open gaps):
//! - [`rng`]: the metagame draw (85D840 on `MGState+0x18`).
//! - [`battle`]: quick battle odds/solve and loser-army selection.
//! - [`data`]: the original metagame definitions (campaigns, kingdoms, teams,
//!   territories and links, armies, divisions, shops, AI parameters) loaded
//!   from the original TGI sources with the native loader validations.
//! - [`state`]: `KKC_MGState` InitCreate (939B90), owners, relocation, victory.
//! - [`commands`]: command validation (951D70) and processing (954570).
//! - [`path`]: the strategic pathfinder (93FDC0, 961550/961350).
//! - [`resolve`] / [`turn`]: battle results (954130, 967290/9673F0) and the
//!   end-turn flow (952790, 9391F0, 938C90).
//! - [`save`]: the strategic save format (938EE0 / 93A3B0).
//! - [`launch`]: battle generation up to the RTS seam (948EF0) and the RTS
//!   result decision (93A990).
//! - [`mgai`]: the metagame AI player (963430, PlayTurn 964D30).
//!
//! Independent of any UI. No fixed-size native tables are reproduced: all
//! native arrays are growable `Vec`s.
#![forbid(unsafe_code)]

pub mod battle;
pub mod commands;
pub mod data;
pub mod path;
pub mod qsort;
pub mod rng;
pub mod state;
pub mod resolve;
pub mod turn;
pub mod save;
pub mod chunk_file;
pub mod launch;
pub mod mgai;
pub mod positions;
pub mod rmc;

use thiserror::Error;

/// x87 precision control of the thread that runs the authoritative metagame
/// (lead policy, round 2 correction; research/phase8-tick/fpu-precision.md):
/// PC53. Metagame commands are TGC orders/decrees executed by the net node
/// update (`TGC_NetNode::Update` 765410 on its own TGC_Thread, Windows initial
/// CW 0x027F), and the MGAI yields through the SAI fiber scheduler 79DFF0 that
/// `KKC_SAI::Think` runs inside 765410 (INFERENCE from the call structure, not
/// a dynamic trace). Main-thread presentation (e.g. the odds tooltip 94E610)
/// runs at PC24. Every float rule still takes the precision explicitly and the
/// oracles cover PC24, PC53 and PC64.
pub const GAME_PRECISION: aa_numeric::Precision = aa_numeric::Precision::Double53;

#[derive(Debug, Error)]
pub enum Error {
    /// Behaviour that is not ported/proven yet, or input outside the proven domain.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// A native fatal data error (`FUN_00410C40` throws); message is the native format.
    #[error("native data error: {0}")]
    NativeData(String),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Rules(#[from] aa_rules::native::Error),
    #[error(transparent)]
    Source(#[from] aa_rules::sources::SourceError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
