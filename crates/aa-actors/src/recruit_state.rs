//! Original Recruit component (slot 16, 0x20 bytes): constructor 581E40
//! (4DB0C0 base), saved Load 583C80, writer 581DE0. Oracle:
//! research/phase8-restoration/native-recruit.json.
use crate::{ActorId, ground_states::ComponentHeader};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecruitState {
    pub header: ComponentHeader,
    /// +10/+11 constructor-zero bytes (not serialized).
    pub byte_10: u8,
    pub byte_11: u8,
    /// +14/+18 words, loaded only when the +1C byte is nonzero.
    pub words_14: [u32; 2],
    /// +1C byte (written for every saved version above 0).
    pub byte_1c: u8,
}
impl RecruitState {
    pub fn new(owner: ActorId) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            byte_10: 0,
            byte_11: 0,
            words_14: [0; 2],
            byte_1c: 0,
        }
    }
    /// 583C80 field application after the Updatable record and version.
    pub fn load(&mut self, byte_1c: Option<u8>, words_14: Option<[u32; 2]>) {
        self.header.scheduled_head = None;
        if let Some(value) = byte_1c {
            self.byte_1c = value;
            if let Some(words) = words_14 {
                self.words_14 = words;
            }
        }
    }
}
