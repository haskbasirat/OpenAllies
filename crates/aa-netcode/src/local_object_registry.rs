//! The Thing DataManager [B39F58] for live order payloads: its `+28` count
//! (each command 76A880/76A960 reference has count+1 possibilities, the count
//! being NULL) and 76A960's resolution (index < count, entry present, its
//! vt+38 = ActorData). Both read the SAME current Bank/source universe as
//! the ActorCommand registry; the definitions are the actual native-order
//! registration (aa_runtime::player_order_decree::object_reference).
use super::*;
use aa_runtime::player_order_decree::object_reference as objects;

pub(super) const OBJECTS: Registry = Registry(0xb39f58);

impl Current<'_, '_, '_, '_> {
    pub(super) fn object_count_b39f58(&self) -> OrderResult<usize> {
        self.with(|_, _| objects::object_registry_count_b39f58(self.catalog))
    }
    pub(super) fn object_definition_76a960(&self, index: u32) -> OrderResult<bool> {
        self.with(|_, _| objects::resolve_object_76a960(self.catalog, index).map(|d| d.is_some()))
    }
}
