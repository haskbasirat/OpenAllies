//! Original Defender component (slot 20, 0x34 bytes): constructor 567CD0
//! (4DB020 base; one 4C8650 Ability per ActorData+21C local ability, in
//! order), saved Load 567020, writer 566EA0. Oracle:
//! research/phase8-restoration/native-defender.json (51 whole cases).
use crate::{ActorId, ground_states::ComponentHeader};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefenderState {
    pub header: ComponentHeader,
    /// +10/+14 Ability instances (shared world arena handles), source order.
    pub abilities: Vec<u32>,
    pub word_1c: u32,
    pub word_20: u32,
    /// +24/+28 entry list {key, value}, head first (566FE0 head insert; the
    /// +28 tail is the oldest entry).
    pub entries: Vec<[u32; 2]>,
    pub word_2c: u32,
    pub byte_30: u8,
}
impl DefenderState {
    pub fn new(owner: ActorId, abilities: Vec<u32>) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            abilities,
            word_1c: 0,
            word_20: 0,
            entries: Vec::new(),
            word_2c: 0,
            byte_30: 0,
        }
    }
    /// 567074..56708A: saved record `index` loads into Ability
    /// min(index, count - 1). With no abilities the native reads before the
    /// vector (no defined result), reported as `None`.
    pub fn load_target(&self, index: usize) -> Option<u32> {
        let last = self.abilities.len().checked_sub(1)?;
        Some(self.abilities[index.min(last)])
    }
    /// 567098..: version < 2 appends {Ability+14, 0} per Ability in order;
    /// version 2 appends each saved pair in read order (all head inserts).
    pub fn load_entries(
        &mut self,
        saved_pairs: Option<&[[u32; 2]]>,
        mut ability_word14: impl FnMut(u32) -> Result<u32, String>,
    ) -> Result<(), String> {
        self.header.scheduled_head = None;
        match saved_pairs {
            Some(pairs) => {
                for pair in pairs {
                    self.entries.insert(0, *pair);
                }
            }
            None => {
                for handle in self.abilities.clone() {
                    let key = ability_word14(handle)?;
                    self.entries.insert(0, [key, 0]);
                }
            }
        }
        Ok(())
    }
}
