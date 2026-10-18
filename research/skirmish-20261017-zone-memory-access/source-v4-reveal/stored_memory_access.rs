//! Synchronous field loans shared by whole-store and active-Zone memory calls.
//! No actor, component, placement or memory allocation is extracted or copied.
use crate::actor_world::{
    ActorStore, ObjectFields, PlacementState, RuntimeComponent, Source, ThingCore,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::Components,
    loading::LoadKind,
};
use aa_mapworld::restore::RuntimeDefinition;
use std::collections::BTreeMap;
pub(crate) struct ActorRead<'a> {
    pub components: &'a Components<RuntimeComponent>,
    pub words: &'a BTreeMap<u32, u32>,
    pub string_dc_utf16: &'a [u16],
    pub string_e0_utf16: &'a [u16],
}
pub(crate) struct Read<'a> {
    pub definition: &'a RuntimeDefinition,
    pub kind: LoadKind,
    pub thing: &'a ThingCore,
    pub object: &'a ObjectFields,
    pub actor: Option<ActorRead<'a>>,
    pub source: &'a Source,
}
pub(crate) trait Access {
    fn read(&self, actor: ActorId) -> Result<Read<'_>>;
    fn object_mut(&mut self, actor: ActorId) -> Result<&mut ObjectFields>;
    fn placement(&self, actor: ActorId) -> Result<PlacementState>;
}
pub(crate) struct StoreAccess<'a> {
    pub actors: &'a mut ActorStore,
    pub placement: crate::load_store_memory_refresh::PlacementRead<'a>,
}
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
impl Access for StoreAccess<'_> {
    fn read(&self, id: ActorId) -> Result<Read<'_>> {
        let key = self
            .actors
            .key(id)
            .ok_or_else(|| error("memory current ActorStore generation absent"))?;
        let entity = self.actors.entity(key).map_err(error)?;
        let r = self.actors.object(key).map_err(error)?;
        Ok(Read {
            definition: &r.definition,
            kind: r.kind,
            thing: &r.thing,
            object: r
                .object
                .as_ref()
                .ok_or(Error::NullDereference("memory actual Object"))?,
            actor: r.actor.as_ref().map(|a| ActorRead {
                components: &a.components,
                words: &a.words,
                string_dc_utf16: &a.string_dc_utf16,
                string_e0_utf16: &a.string_e0_utf16,
            }),
            source: &entity.source,
        })
    }
    fn object_mut(&mut self, id: ActorId) -> Result<&mut ObjectFields> {
        let key = self
            .actors
            .key(id)
            .ok_or_else(|| error("memory current ActorStore generation absent"))?;
        self.actors
            .object_mut(key)
            .map_err(error)?
            .object
            .as_mut()
            .ok_or(Error::NullDereference("memory actual Object"))
    }
    fn placement(&self, id: ActorId) -> Result<PlacementState> {
        self.placement.read(self.actors, id)
    }
}
