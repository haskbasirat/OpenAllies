//! Original MobileDeployment component (slot 19, 0x18 bytes): constructor
//! 575940 (4DB0C0 base; +10 = 1.0, +14 = -1.0), saved Load 575890, writer
//! 575840. Oracle: research/phase8-restoration/native-mobile-deployment.json.
use crate::{ActorId, ground_states::ComponentHeader};
use aa_body::F32;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobileDeploymentState {
    pub header: ComponentHeader,
    pub word_10: F32,
    pub word_14: F32,
}
impl MobileDeploymentState {
    pub fn new(owner: ActorId) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            word_10: F32(0x3f80_0000),
            word_14: F32(0xbf80_0000),
        }
    }
    /// 575890 after the Updatable record and version: raw +10, then +14
    /// for version > 0 (version 2's temporary object is rejected upstream).
    pub fn load(&mut self, word_10: u32, word_14: Option<u32>) {
        self.header.scheduled_head = None;
        self.word_10 = F32(word_10);
        if let Some(word) = word_14 {
            self.word_14 = F32(word);
        }
    }
}
