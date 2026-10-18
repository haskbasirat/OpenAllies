//! Original588FB0 Structure constructor. Queue item restoration and execution
//! remain unavailable until their concrete native classes are bound.
use crate::{ActorId, ground_states::ComponentHeader};
use std::convert::Infallible;
#[derive(Clone, Debug)]
pub struct StructureState {
    pub header: ComponentHeader,
    pub object_handle_10: u32,
    pub queue_14: Vec<Infallible>,
    pub capacity_hint: u32,
    pub word_20: u32,
    pub word_24: u32,
    pub word_28: u32,
    pub word_2c: u32,
    pub byte_30: u8,
    pub word_34: u32,
}
impl StructureState {
    pub fn new(owner: ActorId) -> Self {
        Self {
            header: ComponentHeader::new(owner),
            object_handle_10: 0,
            queue_14: Vec::new(),
            capacity_hint: 6,
            word_20: 0,
            word_24: 0xbf80_0000,
            word_28: u32::MAX,
            word_2c: 0,
            byte_30: 0,
            word_34: 0,
        }
    }
}
