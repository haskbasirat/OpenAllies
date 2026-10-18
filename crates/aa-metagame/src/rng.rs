//! Metagame random draws.
//!
//! Native: `FUN_0085D840` (generic engine LCG), called by the metagame with the
//! state word at `KKC_MGState + 0x18` (`FUN_00938990` returns `this + 0x18`).
//! Consumers ported so far: `KKC_QuickBattleSolver::SolveBattle` (95F480) and
//! the random destroy selection (9564F0).
//! Proven by `research/phase8-metagame/native-battle.json` (`rng` rows).
use serde::{Deserialize, Serialize};

/// The single authoritative metagame draw sequence (state word `MGState+0x18`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetaGameRng {
    pub state: u32,
}

impl MetaGameRng {
    pub const fn new(state: u32) -> Self {
        Self { state }
    }
    /// `FUN_0085D840`: `wide = state * 0x41C64E6D + 0x3039` (64-bit),
    /// `state = low32(wide)`, returns `low32(wide >> 16)`.
    pub fn next_u32(&mut self) -> u32 {
        let wide = u64::from(self.state) * 0x41c6_4e6d + 0x3039;
        self.state = wide as u32;
        (wide >> 16) as u32
    }
}
