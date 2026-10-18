//! Opt-in immediate saved-record lifecycle, over the actual held allocation.
//!
//! The proved4FAE80 tail coordinator owns native call order. This adapter owns
//! the current out slot and registry handoff; it supplies no successful Init,
//! PostCreate, registration, warning or destructor implementation.
use crate::{
    WorldInventory, record_lifecycle,
    restore::{NativeMessage, RestoreDefinitions, RestoreStage, RestoredRecord},
};
use aa_actors::{ActorId, IdentityRegistry, loading::LoadShell};
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use serde::Serialize;
#[cfg(test)]
#[path = "restore_lifecycle_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum Mode {
    /// Existing read-only prefix diagnostics explicitly omit the entire tail.
    #[default]
    DiagnosticOmitted,
    Immediate,
}

#[derive(Clone, Debug, Serialize)]
pub struct Call {
    pub service: &'static str,
    pub record: Option<u32>,
    pub completed: bool,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub stage: RestoreStage,
    pub source_ordinal: usize,
    pub saved_id: u32,
    pub output_id: Option<u32>,
    pub nested: bool,
    pub mode: Mode,
    pub attempted: bool,
    pub completed: bool,
    pub omitted: bool,
    pub error: Option<String>,
    pub calls: Vec<Call>,
}

/// An owned allocation temporarily outside the identity registry. No Clone:
/// moving the out slot must never clone the current record's mutable state.
#[derive(Debug)]
pub struct HeldRecord {
    id: ActorId,
    record: RestoredRecord,
}
impl HeldRecord {
    pub fn id(&self) -> ActorId {
        self.id
    }
    pub fn record(&self) -> &RestoredRecord {
        &self.record
    }
    pub fn record_mut(&mut self) -> &mut RestoredRecord {
        &mut self.record
    }
}

fn source_id(record: &RestoredRecord) -> Result<ActorId> {
    Ok(ActorId::from_raw(match record {
        RestoredRecord::Team(e) => e.source.serialized_id,
        RestoredRecord::Kingdom(e) => e.source.serialized_id,
        RestoredRecord::Thing(e) | RestoredRecord::Object(e) | RestoredRecord::Actor(e) => {
            e.source.serialized_id
        }
        RestoredRecord::Loading => {
            return Err("lifecycle held record is another Loading placeholder".into());
        }
    }))
}

pub fn shell(
    record: &RestoredRecord,
) -> Result<&LoadShell<crate::bootstrap::DefinitionProof, crate::restore::RestoredComponent>> {
    match record {
        RestoredRecord::Team(e) => e.runtime.as_ref().map(|r| &r.base),
        RestoredRecord::Kingdom(e) => e.runtime.as_ref().map(|r| &r.base),
        RestoredRecord::Thing(e) | RestoredRecord::Object(e) | RestoredRecord::Actor(e) => {
            Some(&e.shell)
        }
        RestoredRecord::Loading => None,
    }
    .ok_or_else(|| "lifecycle current record has no owned base shell".into())
}
pub fn shell_mut(
    record: &mut RestoredRecord,
) -> Result<&mut LoadShell<crate::bootstrap::DefinitionProof, crate::restore::RestoredComponent>> {
    match record {
        RestoredRecord::Team(e) => e.runtime.as_mut().map(|r| &mut r.base),
        RestoredRecord::Kingdom(e) => e.runtime.as_mut().map(|r| &mut r.base),
        RestoredRecord::Thing(e) | RestoredRecord::Object(e) | RestoredRecord::Actor(e) => {
            Some(&mut e.shell)
        }
        RestoredRecord::Loading => None,
    }
    .ok_or_else(|| "lifecycle current record has no owned base shell".into())
}

/// Required native services receive this exact owner, not a reconstructed pose.
/// The out slot is private: replacements/nulls use checked ownership moves.
pub struct Context<'a> {
    current: Option<HeldRecord>,
    destroyed: Option<ActorId>,
    pub registry: &'a mut IdentityRegistry<RestoredRecord>,
    pub inventory: &'a WorldInventory,
    pub rng: &'a mut NativeRng,
    pub native_messages: &'a mut Vec<NativeMessage>,
    pub presentation_effects: &'a mut Vec<crate::restore_components::OwnerEffect>,
    pub stage: RestoreStage,
    pub source_ordinal: usize,
    pub nested: bool,
    pub precision: Precision,
    /// The actual shared GWorld+2C word, including earlier Actor Load raises.
    /// A lifecycle callback must not reconstruct it from immutable inventory.
    pub world_version: &'a mut u32,
    calls: Vec<Call>,
    active_service: Option<&'static str>,
}

/// Simultaneous borrows of the actual initialization owners. None of these
/// references points to a reconstructed shell, RNG, version or message queue.
pub struct InitializeParts<'a> {
    pub held: &'a mut HeldRecord,
    pub registry: &'a mut IdentityRegistry<RestoredRecord>,
    pub world_version: &'a mut u32,
    pub rng: &'a mut NativeRng,
    pub messages: &'a mut Vec<NativeMessage>,
    pub presentation: &'a mut Vec<crate::restore_components::OwnerEffect>,
    pub inventory: &'a WorldInventory,
}

impl Context<'_> {
    pub fn current(&self) -> Option<&HeldRecord> {
        self.current.as_ref()
    }
    pub fn current_mut(&mut self) -> Option<&mut HeldRecord> {
        self.current.as_mut()
    }
    pub fn split_current(
        &mut self,
    ) -> Result<(&mut HeldRecord, &mut IdentityRegistry<RestoredRecord>)> {
        self.validate()?;
        Ok((
            self.current
                .as_mut()
                .ok_or("lifecycle current out slot is null")?,
            &mut *self.registry,
        ))
    }
    /// Split the same held allocation and authoritative mutable GWorld+2C.
    /// Kingdom Init can raise this word before the next record is loaded.
    pub fn split_current_with_world_version(
        &mut self,
    ) -> Result<(
        &mut HeldRecord,
        &mut IdentityRegistry<RestoredRecord>,
        &mut u32,
    )> {
        self.validate()?;
        Ok((
            self.current
                .as_mut()
                .ok_or("lifecycle current out slot is null")?,
            &mut *self.registry,
            &mut *self.world_version,
        ))
    }
    /// Validate current generation/Loading ownership before exposing disjoint
    /// borrows needed by Actor/Kingdom Init and their nested concrete services.
    pub fn split_initialize_parts(&mut self) -> Result<InitializeParts<'_>> {
        self.validate()?;
        Ok(InitializeParts {
            held: self
                .current
                .as_mut()
                .ok_or("lifecycle current out slot is null")?,
            registry: &mut *self.registry,
            world_version: &mut *self.world_version,
            rng: &mut *self.rng,
            messages: &mut *self.native_messages,
            presentation: &mut *self.presentation_effects,
            inventory: self.inventory,
        })
    }
    fn require(&self, id: ActorId) -> Result<&HeldRecord> {
        self.current
            .as_ref()
            .filter(|r| r.id == id)
            .ok_or_else(|| "lifecycle captured record is not the current owned out slot".into())
    }
    fn validate(&self) -> Result<()> {
        let Some(current) = &self.current else {
            return Ok(());
        };
        if current.id.is_null() || source_id(&current.record)? != current.id {
            return Err("lifecycle held record/source identity differs".into());
        }
        if self.destroyed == Some(current.id) {
            if self.registry.get(current.id).is_some() {
                return Err("destroyed lifecycle identity is still registered".into());
            }
            return Ok(());
        }
        if !self
            .registry
            .get(current.id)
            .is_some_and(|e| matches!(&e.value, RestoredRecord::Loading))
        {
            return Err("lifecycle current generation/Loading ownership changed".into());
        }
        Ok(())
    }
    /// Return the actual allocation to its same generation slot and clear only
    /// this out pointer. This is not destruction or an implicit native cleanup.
    pub fn return_current_to_registry_and_clear(&mut self) -> Result<()> {
        self.validate()?;
        let Some(current) = self.current.as_ref() else {
            return Ok(());
        };
        if self.destroyed.is_some() {
            return Err("cannot return a destroyed lifecycle allocation".into());
        }
        let id = current.id;
        let slot = self
            .registry
            .value_mut(id)
            .ok_or("lifecycle identity disappeared before return")?;
        if !matches!(slot, RestoredRecord::Loading) {
            return Err("lifecycle return would overwrite another owner".into());
        }
        let current = self.current.take().unwrap();
        *slot = current.record;
        Ok(())
    }
    /// A changed out slot transfers another existing allocation from the same
    /// generation-checked registry, and returns the old allocation there first.
    pub fn replace_current_from_registry(&mut self, replacement: ActorId) -> Result<()> {
        self.validate()?;
        if self.current.as_ref().is_some_and(|r| r.id == replacement) {
            return Ok(());
        }
        let candidate = self
            .registry
            .get(replacement)
            .ok_or("lifecycle replacement generation is absent")?;
        if source_id(&candidate.value)? != replacement {
            return Err("lifecycle replacement source identity differs".into());
        }
        self.return_current_to_registry_and_clear()?;
        let slot = self
            .registry
            .value_mut(replacement)
            .ok_or("lifecycle replacement disappeared")?;
        let record = std::mem::replace(slot, RestoredRecord::Loading);
        self.current = Some(HeldRecord {
            id: replacement,
            record,
        });
        self.destroyed = None;
        self.validate()
    }
    /// Call only after the required4F5490 implementation completed its actual
    /// detach/component/world cleanup. Removing a GTID alone is not destruction.
    /// The coordinator clears/drops the held out value only after this receipt.
    pub fn retire_current_after_destroy(&mut self, id: ActorId) -> Result<()> {
        if self.active_service != Some("destroy_record_4F5490") {
            return Err(
                "GTID retirement receipt is only valid inside the required4F5490 callback".into(),
            );
        }
        self.validate()?;
        self.require(id)?;
        if self.destroyed.is_some() {
            return Err("lifecycle allocation already destroyed".into());
        }
        self.registry.remove(id).map_err(|e| e.to_string())?;
        self.destroyed = Some(id);
        Ok(())
    }
}

pub(crate) struct Inputs<'a> {
    pub registry: &'a mut IdentityRegistry<RestoredRecord>,
    pub inventory: &'a WorldInventory,
    pub rng: &'a mut NativeRng,
    pub native_messages: &'a mut Vec<NativeMessage>,
    pub presentation_effects: &'a mut Vec<crate::restore_components::OwnerEffect>,
    pub events: &'a mut Vec<Event>,
    pub detached: &'a mut Vec<HeldRecord>,
    pub stage: RestoreStage,
    pub source_ordinal: usize,
    pub nested: bool,
    pub precision: Precision,
    pub world_version: &'a mut u32,
}

pub(crate) fn return_held(
    id: ActorId,
    record: RestoredRecord,
    registry: &mut IdentityRegistry<RestoredRecord>,
    detached: &mut Vec<HeldRecord>,
) -> Result<()> {
    let matches = source_id(&record).is_ok_and(|actual| actual == id)
        && registry
            .get(id)
            .is_some_and(|entry| matches!(&entry.value, RestoredRecord::Loading));
    if !matches {
        detached.push(HeldRecord { id, record });
        return Err("record ownership changed before returning the actual Load allocation".into());
    }
    *registry.value_mut(id).unwrap() = record;
    Ok(())
}

pub(crate) fn load_failed(
    stage: RestoreStage,
    source_ordinal: usize,
    id: ActorId,
    nested: bool,
    mode: Mode,
    error: &str,
) -> Event {
    Event {
        stage,
        source_ordinal,
        saved_id: id.raw(),
        output_id: Some(id.raw()),
        nested,
        mode,
        attempted: false,
        completed: false,
        omitted: false,
        error: Some(format!(
            "Load did not complete; immediate lifecycle not reached: {error}"
        )),
        calls: Vec::new(),
    }
}

struct Adapter<'a, 'cx, D> {
    definitions: &'a D,
    context: &'a mut Context<'cx>,
}
impl<D: RestoreDefinitions> Adapter<'_, '_, D> {
    fn invoke<T>(
        &mut self,
        service: &'static str,
        id: Option<ActorId>,
        f: impl FnOnce(&D, &mut Context<'_>) -> Result<T>,
    ) -> Result<T> {
        let index = self.context.calls.len();
        self.context.calls.push(Call {
            service,
            record: id.map(ActorId::raw),
            completed: false,
            error: None,
        });
        self.context.active_service = Some(service);
        let result =
            f(self.definitions, self.context).and_then(|v| self.context.validate().map(|_| v));
        self.context.active_service = None;
        self.context.calls[index].completed = result.is_ok();
        self.context.calls[index].error = result.as_ref().err().cloned();
        result
    }
}
impl<D: RestoreDefinitions> record_lifecycle::World for Adapter<'_, '_, D> {
    type Record = ActorId;
    fn current_record(&mut self) -> Result<Option<ActorId>> {
        self.context.validate()?;
        Ok(self.context.current.as_ref().map(|r| r.id))
    }
    fn flags(&mut self, id: ActorId) -> Result<u32> {
        Ok(shell(&self.context.require(id)?.record)?.thing.flags)
    }
    fn clear_loading_flag(&mut self, id: ActorId) -> Result<()> {
        self.invoke("clear_loading_flag_bit31", Some(id), |_, c| {
            c.require(id)?;
            shell_mut(&mut c.current.as_mut().unwrap().record)?
                .thing
                .flags &= !(1 << 31);
            Ok(())
        })
    }
    fn init_null(&mut self, id: ActorId) -> Result<u8> {
        self.invoke("init_null", Some(id), |d, c| d.lifecycle_init_null(c, id))
    }
    fn editor_insert_out_slot(&mut self) -> Result<()> {
        self.invoke("editor_insert_out_slot", None, |d, c| {
            d.lifecycle_editor_insert_out_slot(c)
        })
    }
    fn post_create_null_one(&mut self, id: ActorId) -> Result<()> {
        self.invoke("post_create_null_one", Some(id), |d, c| {
            d.lifecycle_post_create_null_one(c, id)
        })
    }
    fn register_record(&mut self, id: ActorId) -> Result<()> {
        self.invoke("register_record_4F9510", Some(id), |d, c| {
            d.lifecycle_register_record(c, id)
        })
    }
    fn object_predicate(&mut self, id: ActorId) -> Result<u8> {
        self.invoke("object_predicate_B0", Some(id), |d, c| {
            d.lifecycle_object_predicate(c, id)
        })
    }
    fn definition_publish_one(&mut self, id: ActorId) -> Result<()> {
        self.invoke("definition_publish_one_4296B0", Some(id), |d, c| {
            d.lifecycle_definition_publish_one(c, id)
        })
    }
    fn init_failed_warning(&mut self, id: ActorId, pc: Precision) -> Result<()> {
        self.invoke("init_failed_warning_5A1040", Some(id), |d, c| {
            d.lifecycle_init_failed_warning(c, id, pc)
        })
    }
    fn destroy_record(&mut self, id: ActorId) -> Result<()> {
        self.invoke("destroy_record_4F5490", Some(id), |d, c| {
            d.lifecycle_destroy_record(c, id)?;
            if c.destroyed != Some(id) {
                return Err(
                    "4F5490 callback returned without actual cleanup/GTID retirement receipt"
                        .into(),
                );
            }
            Ok(())
        })
    }
    fn clear_out_slot(&mut self) -> Result<()> {
        self.invoke("clear_out_slot_after_destroy", None, |_, c| {
            let id = c
                .current
                .as_ref()
                .ok_or("destroyed out slot was already cleared")?
                .id;
            if c.destroyed != Some(id) || c.registry.get(id).is_some() {
                return Err("out slot has no completed destruction receipt".into());
            }
            c.current.take();
            c.destroyed = None;
            Ok(())
        })
    }
}

/// Called only after successful Load and ordered owner effects. The held record
/// is returned to the registry even when a later lifecycle service fails. On an
/// invalid external registry mutation it is retained in detached, never cloned,
/// silently dropped, or written over another generation's allocation.
pub(crate) fn finish(
    id: ActorId,
    record: RestoredRecord,
    definitions: &impl RestoreDefinitions,
    inputs: Inputs<'_>,
) -> Result<Option<ActorId>> {
    let mode = definitions.record_lifecycle_mode();
    let mut context = Context {
        current: Some(HeldRecord { id, record }),
        destroyed: None,
        registry: inputs.registry,
        inventory: inputs.inventory,
        rng: inputs.rng,
        native_messages: inputs.native_messages,
        presentation_effects: inputs.presentation_effects,
        stage: inputs.stage,
        source_ordinal: inputs.source_ordinal,
        nested: inputs.nested,
        precision: inputs.precision,
        world_version: inputs.world_version,
        calls: Vec::new(),
        active_service: None,
    };
    let mut result = context.validate();
    let attempted = result.is_ok() && mode == Mode::Immediate;
    if attempted {
        result = if *context.world_version < 23 {
            Err("immediate lifecycle requires unported pre-v23 building-zone/legacy pre-tail checks".into())
        } else {
            record_lifecycle::finish(
                &mut Adapter {
                    definitions,
                    context: &mut context,
                },
                inputs.precision,
            )
        };
    }
    let output_id = context.current.as_ref().map(|r| r.id);
    if let Some(current) = context.current.as_ref() {
        if context.destroyed == Some(current.id) {
            // A required later error after destruction keeps ownership evidence;
            // no second destructor or fabricated successful out-slot clear.
            inputs.detached.push(context.current.take().unwrap());
            if result.is_ok() {
                result = Err("destroyed allocation remained in lifecycle out slot".into());
            }
        } else if let Err(error) = context.return_current_to_registry_and_clear() {
            inputs.detached.push(context.current.take().unwrap());
            result = Err(match result {
                Err(prior) => format!("{prior}; ownership return: {error}"),
                Ok(()) => error,
            });
        }
    }
    inputs.events.push(Event {
        stage: inputs.stage,
        source_ordinal: inputs.source_ordinal,
        saved_id: id.raw(),
        output_id: output_id.map(ActorId::raw),
        nested: inputs.nested,
        mode,
        attempted,
        completed: attempted && result.is_ok(),
        omitted: mode == Mode::DiagnosticOmitted,
        error: result.as_ref().err().cloned(),
        calls: context.calls,
    });
    result.map(|_| output_id)
}
