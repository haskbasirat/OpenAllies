//! Original Builder component (slot 18, 0x10 bytes, vtable A3B2A8):
//! constructor 562F10 is only the component base 4DB0C0 (owner +4 and the
//! Updatable at +8, vtable A3B27C); saved Load 562ED0 is the Updatable
//! record (50BCD0) and a version word (8589B0, max 0); requirement virtual
//! +20 is 7C53E0 (`mov eax,4`: the Mind).
use crate::{ActorId, components::ComponentSlot, ground_states::ComponentHeader};

pub const REQUIRED_COMPONENT: ComponentSlot = ComponentSlot::Mind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuilderState {
    pub header: ComponentHeader,
}
impl BuilderState {
    pub fn new(owner: ActorId) -> Self {
        Self {
            header: ComponentHeader::new(owner),
        }
    }
}
