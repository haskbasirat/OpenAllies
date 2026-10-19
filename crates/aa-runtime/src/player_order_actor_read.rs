//! The read-only owners the 4CF430 permission and the 4DA570 guess read.
//!
//! Inside a frame the Mind callbacks lend [`ActorAccess`]; between frames
//! the UI's UIWorldState 0x436950 (CanSend) asks the same actor virtual
//! +0x150 over the retained [`ActorStore`] and the World's Bank, which it
//! may only read. Both forward to the same storage methods.
use super::{Error, Result};
use crate::actor_world::{self, ActorAccess, ActorStore, Entity, ObjectKey, PlacementState, RuntimeComponent, RuntimeObject, Spatial};
use crate::definitions::Bank;
use aa_actors::{ActorId, IdentityRegistry, components::ComponentSlot};
use std::cell::{Ref, RefCell, RefMut};

/// The actor storage reads of the permission/guess kernels.
pub trait ActorRead {
    fn identities(&self) -> &IdentityRegistry<ObjectKey>;
    fn key(&self, id: ActorId) -> Option<ObjectKey>;
    fn entity(&self, key: ObjectKey) -> actor_world::Result<&Entity>;
    fn object(&self, key: ObjectKey) -> actor_world::Result<&RuntimeObject>;
    fn component(&self, id: ActorId, slot: ComponentSlot) -> actor_world::Result<Option<&RuntimeComponent>>;
    fn placement(&self, key: ObjectKey, spatial: &impl Spatial) -> actor_world::Result<PlacementState>;
}
macro_rules! forward_actor_read {
    ($t:ty) => {
        impl ActorRead for $t {
            fn identities(&self) -> &IdentityRegistry<ObjectKey> {
                <$t>::identities(self)
            }
            fn key(&self, id: ActorId) -> Option<ObjectKey> {
                <$t>::key(self, id)
            }
            fn entity(&self, key: ObjectKey) -> actor_world::Result<&Entity> {
                <$t>::entity(self, key)
            }
            fn object(&self, key: ObjectKey) -> actor_world::Result<&RuntimeObject> {
                <$t>::object(self, key)
            }
            fn component(&self, id: ActorId, slot: ComponentSlot) -> actor_world::Result<Option<&RuntimeComponent>> {
                <$t>::component(self, id, slot)
            }
            fn placement(&self, key: ObjectKey, spatial: &impl Spatial) -> actor_world::Result<PlacementState> {
                <$t>::placement(self, key, spatial)
            }
        }
    };
}
forward_actor_read!(ActorAccess<'_>);
forward_actor_read!(ActorStore);

/// A read-only loan of the World's Bank (presentation parts between frames).
/// A path that would intern a definition (an explicit command) needs the
/// moved mutable Bank and is refused.
pub struct ReadBank<'b> {
    cell: RefCell<&'b Bank>,
}
impl<'b> ReadBank<'b> {
    pub fn new(bank: &'b Bank) -> Self {
        Self { cell: RefCell::new(bank) }
    }
}
impl super::Banks for ReadBank<'_> {
    fn borrow_bank(&self) -> Result<Ref<'_, Bank>> {
        self.cell
            .try_borrow()
            .map(|v| Ref::map(v, |bank| &**bank))
            .map_err(|e| Error::Required(format!("actual read-only Bank: {e}")))
    }
    fn borrow_bank_mut(&self) -> Result<RefMut<'_, Bank>> {
        Err(Error::Required("a Bank write (definition intern) on the read-only between-frames Bank loan".into()))
    }
}
