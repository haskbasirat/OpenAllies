//! Native runtime ability identity and fields. Definition evaluation is separate.
use aa_body::F32;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ability {
    pub definition: u32,
    pub local_definition: Option<u32>,
    pub use_count: u32,
    pub last_use: F32,
    pub word14: u32,
    pub enabled: bool,
    pub recharge_bypass: bool,
    pub fuzzy_range: F32,
    pub power: F32,
    pub summoned_ids: Vec<u32>,
    pub word28: u32,
}
impl Ability {
    ///4C8650/4C86D0. main_power is the definition's native4BE3F0(0) result
    ///at its binary32 store, not a guessed weapon strength.
    pub fn new(definition: u32, local_definition: Option<u32>, main_power: F32) -> Self {
        Self {
            definition,
            local_definition,
            use_count: 0,
            last_use: F32(0xff7f_ffff),
            word14: 0,
            enabled: true,
            recharge_bypass: false,
            fuzzy_range: F32::ZERO,
            power: main_power,
            summoned_ids: Vec::new(),
            word28: 0,
        }
    }
}
