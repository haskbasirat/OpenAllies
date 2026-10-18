//! Disjoint borrows of the actual held allocation, excluding Actor zone storage.
use crate::live_object_init::{MemoryRecord, Record};
use aa_actors::{
    ActorId, IdentityRegistry,
    actor_creation::{Error, Result},
    components::{ComponentSlot, Components},
    loading::{LoadKind, ObjectShell, ThingShell},
};
use aa_mapworld::{
    bootstrap::DefinitionProof,
    restore::{RestoredComponent, RestoredRecord, RuntimeDefinition},
};
use std::collections::BTreeMap;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

pub(crate) struct ActorFields<'a> {
    pub components: &'a Components<RestoredComponent>,
    pub words: &'a BTreeMap<u32, u32>,
}
pub struct Fields<'a> {
    pub(crate) owner: ActorId,
    pub(crate) definition: &'a RuntimeDefinition,
    pub(crate) kind: LoadKind,
    pub(crate) thing: &'a ThingShell<DefinitionProof>,
    pub(crate) object: &'a mut ObjectShell,
    pub(crate) actor: Option<ActorFields<'a>>,
    pub(crate) registry: &'a IdentityRegistry<RestoredRecord>,
}
impl Fields<'_> {
    pub(crate) fn actor_container(&self) -> Result<Option<ActorId>> {
        let actor = self
            .actor
            .as_ref()
            .ok_or(Error::Unsupported("4CED60 actual Actor allocation"))?;
        actor_container(self.owner, actor.components)
    }

    pub(crate) fn from_record<'a>(record: &'a mut Record<'_>) -> Result<Fields<'a>> {
        Ok(Fields {
            owner: record.owner,
            definition: record.definition,
            kind: record.shell.kind,
            thing: &record.shell.thing,
            object: record
                .shell
                .object
                .as_mut()
                .ok_or(Error::NullDereference("memory Object allocation"))?,
            actor: record.shell.actor.as_ref().map(|a| ActorFields {
                components: &a.components,
                words: &a.words,
            }),
            registry: &*record.registry,
        })
    }
    pub(crate) fn memory_record(&self) -> MemoryRecord<'_> {
        MemoryRecord {
            owner: self.owner,
            definition: self.definition,
            kind: self.kind,
            proof: &self.thing.definition,
            flags: &self.thing.flags,
            object_present: true,
            actor_present: self.actor.is_some(),
            registry: self.registry,
        }
    }
    pub(crate) fn word(&self, offset: u32) -> Result<u32> {
        match offset {
            8 => Ok(self.thing.flags),
            0x54 => Ok(self.object.words_50_to_54[1]),
            _ => Err(Error::Unsupported(
                "disjoint memory field is not represented",
            )),
        }
    }
}

pub(crate) fn actor_container(
    owner: ActorId,
    components: &Components<RestoredComponent>,
) -> Result<Option<ActorId>> {
    if let Some(component) = components.get(ComponentSlot::Structure) {
        let RestoredComponent::Structure(component) = component else {
            return Err(error("4CED60 Structure payload differs"));
        };
        if component.header.owner != owner {
            return Err(error("4CED60 Structure owner differs"));
        }
        if component.object_handle_10 != 0 {
            return Ok(Some(ActorId::from_raw(component.object_handle_10)));
        }
    }
    if components.present(ComponentSlot::Wall) {
        return Err(Error::Unsupported("4CED60 current Wall58CB30 container"));
    }
    Ok(None)
}
