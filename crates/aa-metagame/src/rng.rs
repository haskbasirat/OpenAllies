//! Metagame random draws.
//!
//! Native: `0x85D840` (generic engine LCG), called by the metagame with the
//! state word at `KKC_MGState + 0x18` (`0x938990` returns `this + 0x18`).
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
    /// `0x85D8A0` (seed): `((seed << 16) | seed) ^ 0x8E4B12A5`, a zero
    /// result keeps the default word 0x8E4B12A5 (`0x85D870`). The SAI
    /// constructor 7A0AA0 seeds the app/SAI RNG `[[B36528]+0x2C]` this way
    /// with the milliseconds of the 59ABA0 timer.
    pub const fn from_seed(seed: u32) -> Self {
        let s = ((seed << 16) | seed) ^ 0x8e4b_12a5;
        Self { state: if s == 0 { 0x8e4b_12a5 } else { s } }
    }
    /// `0x85D840`: `wide = state * 0x41C64E6D + 0x3039` (64-bit),
    /// `state = low32(wide)`, returns `low32(wide >> 16)`.
    pub fn next_u32(&mut self) -> u32 {
        let wide = u64::from(self.state) * 0x41c6_4e6d + 0x3039;
        self.state = wide as u32;
        (wide >> 16) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::MetaGameRng;

    #[test]
    fn from_seed_is_85d8a0() {
        // (seed << 16 | seed) ^ 0x8E4B12A5
        assert_eq!(MetaGameRng::from_seed(0).state, 0x8e4b_12a5);
        assert_eq!(MetaGameRng::from_seed(0x1234).state, 0x1234_1234 ^ 0x8e4b_12a5);
        // the high half of the seed is shifted out, its low half ORed in
        assert_eq!(MetaGameRng::from_seed(0x0001_0000).state, 0x0001_0000 ^ 0x8e4b_12a5);
    }
}
