//! Evidence-backed arithmetic and deterministic scheduling foundations.
//!
//! This crate is not a complete Axis & Allies simulation. The scheduler is a
//! parity laboratory: no movement, combat, AI, full economy or native save codec
//! is implemented. See `docs/SPEC.txt` and `docs/PARITY.txt` in the workspace.

mod arithmetic;
mod random;
mod scheduler;

pub use arithmetic::{FloatBits, NativeClock, NativeRng, ProviderState, RuleError};
pub use random::{RandomError, native_unit_float};
pub use scheduler::{Batch, BatchReceipt, Command, CommandResult, Core, CoreError, Snapshot};

/// Shipped Data/Game/SVars.tgi, not the compiled fallback of ten.
pub const STOCK_GAME_TICKS_PER_SECOND: u32 = 16;
