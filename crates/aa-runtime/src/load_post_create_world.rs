//! Saved PostCreate(NULL,1) on the actual allocation held by LoadThing.
//!
//! Class dispatch follows each original constructor vtable +EC. Actor4CF300
//! uses the proved world_publication caller; source lists and current slots are
//! read from their real owners. Positive property/Kingdom/FX services remain
//! required effects, not fabricated empty lists or successful publication.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    ActorId,
    components::ComponentSlot,
    world_publication::{self, CreateWorld, Error},
};
use aa_mapworld::{
    restore::{RestoredRecord, RuntimeDefinition},
    restore_lifecycle::{Context, HeldRecord},
};
use aa_numeric::Precision;
use aa_rules::{
    native::layout_data::{self, PropertyRef},
    resources::Provenance,
};

#[cfg(test)]
#[path = "load_post_create_world_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;

/// A reached source property, using the canonical source registry identity.
/// This is not an allocated mutable property instance.
#[derive(Clone, Debug)]
pub struct InitialProperty {
    pub reference: PropertyRef,
    pub sources: Vec<Provenance>,
}

/// These bodies must use persistent application-owned instances, Kingdom lists
/// and presentation queues. The adapter never creates a second mutable arena.
pub trait Effects {
    /// Native5817A0(property,0,0): allocate5030F0 then insert581630 into the
    /// current Properties component, retaining any allocation/partial effects.
    fn initial_property(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut Record<'_>,
        _property: InitialProperty,
        _pc: Precision,
    ) -> Result<()> {
        Err("5817A0/581630 actual property-instance owner and effects are unavailable".into())
    }
    /// Native4E6180 on the exact current generation-checked stored Kingdom.
    fn owner_add_actor(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut Record<'_>,
        _owner: ActorId,
        _pc: Precision,
    ) -> Result<()> {
        Err("4E6180 actual Kingdom actor-list/effect owner is unavailable".into())
    }
    /// Native4296B0 after its nonempty+BC source-list guard. The complete
    /// world+130 mute check, FX allocation and actual thread-owned queue remain
    /// required here; the native actor argument is nonnull and flag is1.
    fn created_fx(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut Record<'_>,
        _sources: &[Vec<Provenance>],
        _pc: Precision,
    ) -> Result<()> {
        Err("4296B0 nonempty creation FX requires actual mute/queue/presentation ownership".into())
    }
}

fn parts(
    held: &mut HeldRecord,
) -> Result<(
    &mut aa_actors::loading::LoadShell<
        aa_mapworld::bootstrap::DefinitionProof,
        aa_mapworld::restore::RestoredComponent,
    >,
    &RuntimeDefinition,
)> {
    match held.record_mut() {
        RestoredRecord::Team(e) => e.runtime.as_mut().map(|r| (&mut r.base, &r.definition)),
        RestoredRecord::Kingdom(e) => e.runtime.as_mut().map(|r| (&mut r.base, &r.definition)),
        RestoredRecord::Thing(e) | RestoredRecord::Object(e) | RestoredRecord::Actor(e) => {
            Some((&mut e.shell, &e.definition))
        }
        RestoredRecord::Loading => None,
    }
    .ok_or_else(|| "PostCreate current allocation has no owned shell/definition".into())
}

fn validate(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<()> {
    catalog.require_source_owner()?;
    if record.owner.is_null()
        || !record
            .registry
            .get(record.owner)
            .is_some_and(|v| matches!(&v.value, RestoredRecord::Loading))
    {
        return Err("PostCreate current generation must retain its Loading registry owner".into());
    }
    let proof = &record.definition.proof;
    let canonical = catalog
        .actors
        .actors
        .iter()
        .find(|a| a.id.eq_ignore_ascii_case(&proof.id))
        .ok_or("PostCreate canonical source identity is absent")?;
    let found = catalog
        .actors
        .find(&proof.id)
        .ok_or("PostCreate canonical lookup is absent")?;
    if !found.id.eq_ignore_ascii_case(&canonical.id) || found.source != canonical.source {
        return Err("PostCreate canonical source is shadowed by Thing translation".into());
    }
    if canonical.source != proof.source
        || canonical.template_chain != proof.template_chain
        || catalog
            .actors
            .native_category(&proof.id)
            .map_err(|e| e.to_string())?
            != proof.category
        || proof.category.runtime_class != record.definition.native_class
    {
        return Err("PostCreate current canonical definition/source differs".into());
    }
    Ok(())
}

fn source_precision(catalog: &Catalog<'_>) -> Result<Precision> {
    let bank = catalog
        .definition_bank
        .try_borrow()
        .map_err(|_| "nested PostCreate source Bank ownership")?;
    let state = bank.combat_source()?;
    if state.source_identities() != catalog.actors.sources.as_slice() {
        return Err("PostCreate source list has another source owner".into());
    }
    Ok(state.source_precision())
}

/// The current record stays held outside its exact Loading registry slot.
/// No next record is constructed here and no world publication is performed.
pub fn post_create(
    catalog: &Catalog<'_>,
    context: &mut Context<'_>,
    effects: &mut impl Effects,
) -> Result<()> {
    let pc = context.precision;
    let (held, registry) = context.split_current()?;
    let owner = held.id();
    let (shell, definition) = parts(held)?;
    post_create_record(
        catalog,
        &mut Record {
            owner,
            shell,
            definition,
            registry,
        },
        effects,
        pc,
    )
}

/// Concrete saved-call dispatch. Flag1 skips ground paint and city parameter
/// reads before any null creation-block dereference, just as native4EBA40/
/// 4CF300/564BE0. Unknown native classes are refused, never treated as Thing.
pub fn post_create_record(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    effects: &mut impl Effects,
    pc: Precision,
) -> Result<()> {
    validate(catalog, record)?;
    match record.definition.native_class {
        1 | 5 => Ok(()), // original vtable+EC =4D1E10 (RET8)
        2 | 6 | 7 | 9 => {
            if record.shell.object.is_none() {
                return Err("4EBA40 requires actual Object shell".into());
            }
            Ok(()) // actual flag1:4D1E10 then no ground deformation
        }
        3 | 4 => {
            if record.shell.object.is_none() || record.shell.actor.is_none() {
                return Err("4CF300 requires actual Actor shell".into());
            }
            let owner = record.owner;
            world_publication::post_create(
                &mut Borrowed {
                    catalog,
                    record,
                    effects,
                    pc,
                },
                owner,
                &(),
                1,
            )
            .map_err(|e| e.to_string())
        }
        _ => Err("saved PostCreate class vtable is not bound".into()),
    }
}

/// The required4296B0(current,1) call after native4F9510. +BC is the first
/// ObjectData FX-list pair (49C760 constructor;49DC40 WhenCreated load).
/// Its true null head returns before the native world+130 mute dereference.
pub fn publish_definition_created_fx(
    catalog: &Catalog<'_>,
    context: &mut Context<'_>,
    effects: &mut impl Effects,
) -> Result<()> {
    let pc = context.precision;
    let (held, registry) = context.split_current()?;
    let owner = held.id();
    let (shell, definition) = parts(held)?;
    publish_created_fx_record(
        catalog,
        &mut Record {
            owner,
            shell,
            definition,
            registry,
        },
        effects,
        pc,
    )
}
pub fn publish_created_fx_record(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    effects: &mut impl Effects,
    pc: Precision,
) -> Result<()> {
    validate(catalog, record)?;
    if !matches!(record.definition.native_class, 2 | 3 | 4 | 6 | 7 | 9)
        || record.shell.object.is_none()
    {
        return Err("4296B0 requires proved Object virtual+B0 and source+BC".into());
    }
    let source = crate::definition_fx::Source::bind(catalog, record.definition)?;
    let list = source.list(crate::definition_fx::ListKind::Created);
    if list.is_certified_empty() {
        return Ok(());
    }
    effects.created_fx(catalog, record, list.authored_entries(), pc)
}

struct Borrowed<'a, 'source, 'record, E> {
    catalog: &'a Catalog<'source>,
    record: &'a mut Record<'record>,
    effects: &'a mut E,
    pc: Precision,
}
impl<E: Effects> CreateWorld for Borrowed<'_, '_, '_, E> {
    type Record = ();
    fn has_component(
        &self,
        actor: ActorId,
        slot: ComponentSlot,
    ) -> world_publication::Result<bool> {
        if actor != self.record.owner {
            return Err(Error::World(
                "PostCreate requested another current Actor".into(),
            ));
        }
        Ok(self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or(Error::Unsupported("actual Actor shell"))?
            .components
            .get(slot)
            .is_some())
    }
    fn component_post_create(
        &mut self,
        actor: ActorId,
        slot: ComponentSlot,
        _: &(),
        flag: u32,
    ) -> world_publication::Result<()> {
        if actor != self.record.owner || flag != 1 {
            return Err(Error::Unsupported("only saved PostCreate(NULL,1)"));
        }
        let component = self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or(Error::Unsupported("actual Actor shell"))?
            .components
            .get(slot)
            .ok_or(Error::Unsupported("current component vanished"))?;
        if component.slot() != slot {
            return Err(Error::Unsupported(
                "PostCreate actual component payload/slot mismatch",
            ));
        }
        if slot == ComponentSlot::Properties {
            let aa_mapworld::restore::RestoredComponent::Properties(properties) = component else {
                return Err(Error::Unsupported("actual Properties payload"));
            };
            if properties.owner.0 != actor.raw() {
                return Err(Error::Unsupported("Properties actual owner differs"));
            }
            let pc = source_precision(self.catalog).map_err(Error::World)?;
            let data = self.catalog.layout_data(pc).map_err(Error::World)?;
            let list = layout_data::native_actor_properties(
                self.catalog.actors,
                &data,
                &self.record.definition.proof.id,
                pc,
            )
            .map_err(|e| Error::World(e.to_string()))?;
            for reference in list {
                let sources = match reference {
                    PropertyRef::Registered(i) => data
                        .properties
                        .get(i as usize)
                        .ok_or(Error::Unsupported("registered initial property source"))?
                        .sources
                        .clone(),
                    PropertyRef::Embedded(_) => {
                        return Err(Error::Unsupported(
                            "ActorData initial property must be a registered source",
                        ));
                    }
                };
                self.effects
                    .initial_property(
                        self.catalog,
                        self.record,
                        InitialProperty { reference, sources },
                        self.pc,
                    )
                    .map_err(Error::World)?;
            }
        }
        // All other concrete component +C leaves are RET8; City564BE0 also
        // returns without reading params+34 because the saved flag low byte1.
        Ok(())
    }
    fn stored_owner(&self, actor: ActorId) -> world_publication::Result<Option<ActorId>> {
        if actor != self.record.owner {
            return Err(Error::Unsupported("another stored Actor owner"));
        }
        let word = *self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or(Error::Unsupported("actual Actor shell"))?
            .words
            .get(&0xe8)
            .ok_or(Error::Unsupported("actual Actor+E8 owner word"))?;
        Ok((word != 0).then(|| ActorId::from_raw(word)))
    }
    fn owner_post_create_4e6180(
        &mut self,
        owner: ActorId,
        actor: ActorId,
    ) -> world_publication::Result<()> {
        if actor != self.record.owner
            || !self
                .record
                .registry
                .get(owner)
                .is_some_and(|v| matches!(&v.value, RestoredRecord::Kingdom(_)))
        {
            return Err(Error::Unsupported(
                "4E6180 current stored Kingdom generation is absent",
            ));
        }
        self.effects
            .owner_add_actor(self.catalog, self.record, owner, self.pc)
            .map_err(Error::World)
    }
    fn map_mode_a8(&self) -> world_publication::Result<u32> {
        Err(Error::Unsupported("saved flag1 does not read map mode"))
    }
    fn object_ground_post_create(&mut self, _: ActorId, _: &()) -> world_publication::Result<()> {
        Err(Error::Unsupported("saved flag1 does not paint ground"))
    }
    fn definition_mask(&self, _: ActorId) -> world_publication::Result<u32> {
        Err(Error::Unsupported("saved flag1 does not read city mask"))
    }
    fn city_predicate_45d6a0(&mut self, _: ActorId) -> world_publication::Result<bool> {
        Err(Error::Unsupported("saved flag1 does not query city"))
    }
    fn city_argument_34(&self, _: &()) -> world_publication::Result<u32> {
        Err(Error::Unsupported("null saved creation block"))
    }
    fn city_post_create_5644f0(&mut self, _: ActorId, _: u32) -> world_publication::Result<()> {
        Err(Error::Unsupported("saved flag1 does not create city"))
    }
}
