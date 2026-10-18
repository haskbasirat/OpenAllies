//! Owned native5030F0/581630 property allocations during saved map loading.
//! This owner moves into live ActorStore with the same handles and records.
//! Source definitions and live consequences are bound separately; an empty
//! registry never stands in for a nonempty actual Properties component.
use crate::catalog::Catalog;
use aa_body::{credit::Object, property_effects::Instances, property_flags::Definitions};
use aa_numeric::Precision;
use aa_rules::{
    abilities::EnchantmentModifier,
    actors::ResolvedNode,
    native::{TypedValue, layout_data::PropertyRef},
    resources::{Provenance, SourceIdentity},
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct DefinitionFacts {
    reference: PropertyRef,
    id: String,
    sources: Vec<Provenance>,
    required: Vec<Object>,
    pass_members: bool,
    pass_recruit: bool,
    pass_denizen: bool,
    cv_bias: aa_body::F32,
    fx_added: Vec<Vec<Provenance>>,
}
/// Actual5030F0 fields preceding its final World+E0 read. This allocation is
///not a completed Instance until the clock service succeeds.
#[derive(Debug)]
pub struct ConstructorPrefix {
    pub definition: Object,
    pub kind: u32,
    pub source_id: u32,
}
impl DefinitionFacts {
    pub fn reference(&self) -> PropertyRef {
        self.reference
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn required(&self) -> &[Object] {
        &self.required
    }
    pub fn performer_cv_bias(&self) -> aa_body::F32 {
        self.cv_bias
    }
    pub fn pass_down_members(&self) -> bool {
        self.pass_members
    }
    pub fn pass_down_recruit(&self) -> bool {
        self.pass_recruit
    }
    pub fn pass_down_denizen(&self) -> bool {
        self.pass_denizen
    }
}
#[derive(Debug)]
struct SourceReceipt {
    identities: Vec<SourceIdentity>,
    precision: Precision,
}

/// Portable ownership of native heap instances, their monotonic handles and
/// completed immutable definitions. No Clone/Default can duplicate instances.
#[derive(Debug)]
pub struct State {
    instances: Instances,
    next: u32,
    definitions: Definitions,
    sources: BTreeMap<Object, Vec<aa_rules::resources::Provenance>>,
    facts: BTreeMap<Object, DefinitionFacts>,
    references: BTreeMap<PropertyRef, Object>,
    receipt: Option<SourceReceipt>,
    source_failed: Option<String>,
    constructors: BTreeMap<Object, ConstructorPrefix>,
    detached: Vec<aa_body::property_effects::Instance>,
}
impl State {
    /// Read-only identity validation. A pristine constructor-empty owner has
    ///no definition identities to reinterpret; partial/failed source binding
    ///never gains that exemption. Completed allocations require their exact
    ///source universe, source PC and existing canonical Bank token mappings.
    pub fn require_catalog(&self, catalog: &Catalog<'_>) -> Result<(), String> {
        catalog.require_source_owner()?;
        if let Some(error) = &self.source_failed {
            return Err(error.clone());
        }
        let Some(receipt) = &self.receipt else {
            return if self.instances.is_empty()
                && self.definitions.is_empty()
                && self.facts.is_empty()
                && self.references.is_empty()
            {
                Ok(())
            } else {
                Err("property source owner is only partially bound".into())
            };
        };
        if receipt.identities != catalog.actors.sources
            || receipt.precision != source_precision(catalog)?
        {
            return Err("property allocation source universe/precision differs".into());
        }
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|_| "property source Bank is borrowed")?;
        for (reference, key) in &self.references {
            let facts = self.facts(*key)?;
            if bank.token("Property", &facts.id) != Some(key.0) || facts.reference != *reference {
                return Err("property allocation owner and canonical Bank token differ".into());
            }
        }
        Ok(())
    }
    /// A newly constructed World has not allocated any property instances.
    /// Individual5030F0 allocations remain registered immediately as they occur;
    /// PropertiesComponent.effects is always its authoritative membership list.
    pub fn from_world_constructor() -> Self {
        Self {
            instances: Instances::new(),
            next: 1,
            definitions: Definitions::new(),
            sources: BTreeMap::new(),
            facts: BTreeMap::new(),
            references: BTreeMap::new(),
            receipt: None,
            source_failed: None,
            constructors: BTreeMap::new(),
            detached: Vec::new(),
        }
    }
    pub fn instances(&self) -> &Instances {
        &self.instances
    }
    pub fn instances_mut(&mut self) -> &mut Instances {
        &mut self.instances
    }
    pub fn constructors(&self) -> &BTreeMap<Object, ConstructorPrefix> {
        &self.constructors
    }
    pub fn definitions(&self) -> &Definitions {
        &self.definitions
    }
    pub fn facts(&self, id: Object) -> Result<&DefinitionFacts, String> {
        self.facts
            .get(&id)
            .ok_or_else(|| "actual Property source facts are not bound".into())
    }
    pub fn definition_for_reference(&self, reference: PropertyRef) -> Result<Object, String> {
        self.references
            .get(&reference)
            .copied()
            .ok_or_else(|| "actual Property reference is not source-bound".into())
    }
    pub fn definition_sources(&self, id: Object) -> Option<&[aa_rules::resources::Provenance]> {
        self.sources.get(&id).map(Vec::as_slice)
    }
    /// Same monotonic portable-handle policy previously owned by ActorStore.
    /// Reserving a handle precedes5030F0 construction; later callback failure
    /// never reuses that handle for another instance.
    pub fn reserve_handle(&mut self) -> Result<Object, String> {
        let handle = Object(self.next);
        self.next = self
            .next
            .checked_add(1)
            .ok_or("property instance handle domain exhausted")?;
        if self.instances.contains_key(&handle) || self.constructors.contains_key(&handle) {
            return Err("property instance handle is already occupied".into());
        }
        Ok(handle)
    }
}

fn component<'a>(
    actor: &'a aa_actors::loading::ActorShell<aa_mapworld::restore::RestoredComponent>,
) -> Result<Option<&'a aa_body::properties::PropertiesState>, String> {
    use aa_mapworld::restore::RestoredComponent;
    match actor
        .components
        .get(aa_actors::components::ComponentSlot::Properties)
    {
        None => Ok(None),
        Some(RestoredComponent::Properties(p)) => Ok(Some(p)),
        _ => Err("actual Properties slot has another component payload".into()),
    }
}

/// Actual disjoint Actor storage while its Properties component is borrowed
///by the native mixed-modifier kernel. The focused component remains present
///in OtherComponents; no shell is removed, reconstructed or copied back.
pub struct Fields<'a> {
    ///Immutable source proof of this same held allocation; no source copy.
    pub definition: &'a aa_mapworld::restore::RuntimeDefinition,
    pub precision: Precision,
    pub owner: aa_actors::ActorId,
    pub words: &'a mut BTreeMap<u32, u32>,
    pub components:
        aa_actors::components::OtherComponents<'a, aa_mapworld::restore::RestoredComponent>,
    pub registry: &'a mut aa_actors::IdentityRegistry<aa_mapworld::restore::RestoredRecord>,
}
/// Reached live effects. Defaults identify exact unbound bodies; caller-side
///absence guards never use an invented successful callback.
pub trait Effects {
    ///Actual current cached SVar cell, read only by a reached aggregate that
    ///uses the clamp. Source defaults cannot replace runtime World.variables.
    fn minimum_multiplier(&mut self) -> aa_body::Result<aa_body::F32> {
        Err(aa_body::Error::Callback(
            "actual current ModifierAttributeClampAdd SVar owner is required".into(),
        ))
    }
    fn numeric_changed(
        &mut self,
        _catalog: &Catalog<'_>,
        _fields: &mut Fields<'_>,
        _modifier: &aa_body::property_effects::NumericModifier,
        _properties: &mut aa_body::properties::PropertiesState,
        _instances: &mut Instances,
    ) -> aa_body::Result<()> {
        Err(aa_body::Error::Callback(
            "numeric modifier virtual30 actor consequence is unbound".into(),
        ))
    }
    fn immediate(
        &mut self,
        _name: &'static str,
        _fields: &mut Fields<'_>,
        _value: bool,
        _properties: &mut aa_body::properties::PropertiesState,
        _instances: &mut Instances,
    ) -> aa_body::Result<()> {
        Err(aa_body::Error::Callback(format!(
            "actual property actor consequence {_name} is unbound"
        )))
    }
    fn has_character(&self, _fields: &Fields<'_>) -> aa_body::Result<bool> {
        Err(aa_body::Error::Callback(
            "property consequence actual Character predicate is unbound".into(),
        ))
    }
    fn organization_added(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut crate::live_object_init::Record<'_>,
        state: &mut State,
        instance: Object,
        _pc: Precision,
    ) -> Result<(), String> {
        organization_added(catalog, record, state, instance)
    }
    fn settlement_added(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut crate::live_object_init::Record<'_>,
        _state: &mut State,
        _instance: Object,
        _pc: Precision,
    ) -> Result<(), String> {
        Err("584F80 actual Settlement property propagation is unbound".into())
    }
    fn performer_added(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut crate::live_object_init::Record<'_>,
        _state: &mut State,
        _pc: Precision,
    ) -> Result<(), String> {
        Err("57F4C0/57F1E0 actual Performer property update is unbound".into())
    }
}
pub struct RequiredEffects;
impl Effects for RequiredEffects {}

struct Kernel<'a, 'source, E> {
    catalog: &'a Catalog<'source>,
    fields: Fields<'a>,
    facts: &'a BTreeMap<Object, DefinitionFacts>,
    effects: &'a mut E,
    tail: Option<Object>,
    detached: &'a mut Vec<aa_body::property_effects::Instance>,
}
fn body_error(error: impl std::fmt::Display) -> aa_body::Error {
    aa_body::Error::Callback(error.to_string())
}
impl<E: Effects> aa_body::property_effects::Effects for Kernel<'_, '_, E> {
    fn resolve_source(&self, id: u32) -> aa_body::Result<Option<Object>> {
        let id = aa_actors::ActorId::from_raw(id);
        if id.is_null() || self.fields.registry.get(id).is_none() {
            return Ok(None);
        }
        //5031E0/5033E0 inspect the authoritative generation/slot only;
        //the source allocation's Actor/component/derived payload is not read.
        //A Loading entry still denotes the same registered held allocation.
        Ok(Some(Object(id.raw())))
    }
    fn allegiance(
        &mut self,
        _: &mut aa_body::properties::PropertiesState,
        _: &mut Instances,
    ) -> aa_body::Result<Option<Object>> {
        let id = *self
            .fields
            .words
            .get(&0xe8)
            .ok_or_else(|| body_error("581630 actual owner virtual108 E8 is absent"))?;
        self.resolve_source(id)
    }
    fn numeric_changed(
        &mut self,
        m: &aa_body::property_effects::NumericModifier,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects.numeric_changed(self.catalog, &mut self.fields, m, p, i)
    }
    fn property_added(
        &mut self,
        handle: Object,
        _: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        let instance = i
            .get(&handle)
            .ok_or_else(|| body_error("property added instance disappeared"))?;
        let facts = self
            .facts
            .get(&instance.definition)
            .ok_or_else(|| body_error("property added source facts are absent"))?;
        if !facts.fx_added.is_empty() {
            return Err(body_error(
                "503100 nonempty Property WhenAdded FX graph/publication is required",
            ));
        }
        Ok(())
    }
    fn actor_added(
        &mut self,
        handle: Object,
        _: &mut aa_body::properties::PropertiesState,
        _: &mut Instances,
    ) -> aa_body::Result<()> {
        //4CEAC0 is the final581630 call. Transport its exact tail request out
        //of the component borrow; add() MUST execute it before returning.
        self.tail = Some(handle);
        Ok(())
    }
    fn actor_removed(
        &mut self,
        _: Object,
        _: &mut aa_body::properties::PropertiesState,
        _: &mut Instances,
    ) -> aa_body::Result<()> {
        Err(body_error(
            "actual actor property removal callback is required",
        ))
    }
    fn property_removed(
        &mut self,
        _: Object,
        _: &mut aa_body::properties::PropertiesState,
        _: &mut Instances,
    ) -> aa_body::Result<()> {
        Err(body_error(
            "actual Property WhenRemoved callback is required",
        ))
    }
    fn released(&mut self, instance: aa_body::property_effects::Instance) -> aa_body::Result<()> {
        self.detached.push(instance);
        Err(body_error(
            "actual property destructor is required; detached allocation retained",
        ))
    }
}
impl<E: Effects> aa_body::property_flags::Effects for Kernel<'_, '_, E> {
    fn minimum_multiplier(&mut self) -> aa_body::Result<aa_body::F32> {
        self.effects.minimum_multiplier()
    }
    fn has_mover(&self, _: Object) -> aa_body::Result<bool> {
        Ok(self
            .fields
            .components
            .present(aa_actors::components::ComponentSlot::Mover))
    }
    fn set_mover_byte_4c(
        &mut self,
        _: Object,
        v: bool,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects
            .immediate("Mover+4C", &mut self.fields, v, p, i)
    }
    fn actor_stop_4d4c60(
        &mut self,
        _: Object,
        v: bool,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects.immediate("4D4C60", &mut self.fields, v, p, i)
    }
    fn has_character(&self, _: Object) -> aa_body::Result<bool> {
        self.effects.has_character(&self.fields)
    }
    fn character_pause_4daf20(
        &mut self,
        _: Object,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects
            .immediate("4DAF20", &mut self.fields, true, p, i)
    }
    fn character_resume_4daf40(
        &mut self,
        _: Object,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects
            .immediate("4DAF40", &mut self.fields, false, p, i)
    }
    fn mover_576800(
        &mut self,
        _: Object,
        v: bool,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects.immediate("576800", &mut self.fields, v, p, i)
    }
    fn has_economy(&self, _: Object) -> aa_body::Result<bool> {
        Ok(self
            .fields
            .components
            .present(aa_actors::components::ComponentSlot::Economy))
    }
    fn economy_570020(
        &mut self,
        _: Object,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects
            .immediate("570020", &mut self.fields, false, p, i)
    }
    fn rebuild_actions_4d1350(
        &mut self,
        _: Object,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects
            .immediate("4D1350", &mut self.fields, false, p, i)
    }
    fn resource_count(&self) -> aa_body::Result<usize> {
        use aa_mapworld::restore::RestoreDefinitions;
        usize::try_from(
            self.catalog
                .resource_domain_count(source_precision(self.catalog).map_err(body_error)?)
                .map_err(body_error)?,
        )
        .map_err(body_error)
    }
    fn refresh_actor_4cf0f0(
        &mut self,
        _: Object,
        p: &mut aa_body::properties::PropertiesState,
        i: &mut Instances,
    ) -> aa_body::Result<()> {
        self.effects
            .immediate("4CF0F0", &mut self.fields, false, p, i)
    }
    fn enchantment_type(&self, definition: Object) -> aa_body::Result<Option<Object>> {
        self.facts
            .get(&definition)
            .ok_or_else(|| body_error("actual Property type metadata is absent"))?;
        //All definitions in this source owner are4C48C0 plain PropertyData;
        //EnchantmentProperty registrations require their own producer.
        Ok(None)
    }
}
fn notify<E: Effects>(
    catalog: &Catalog<'_>,
    record: &mut crate::live_object_init::Record<'_>,
    state: &mut State,
    handle: Object,
    pc: Precision,
    effects: &mut E,
) -> Result<(), String> {
    use aa_actors::components::ComponentSlot;
    let present = |record: &crate::live_object_init::Record<'_>, slot| {
        record
            .shell
            .actor
            .as_ref()
            .ok_or("property-added actual Actor allocation is absent")
            .map(|actor| actor.components.present(slot))
    };
    if present(record, ComponentSlot::Organization)? {
        effects.organization_added(catalog, record, state, handle, pc)?;
    }
    if present(record, ComponentSlot::Settlement)? {
        effects.settlement_added(catalog, record, state, handle, pc)?;
    }
    if present(record, ComponentSlot::Performer)? {
        effects.performer_added(catalog, record, state, pc)?;
    }
    Ok(())
}
///5787F0's current member traversal. The definition gate is lazy: a null
///member or absent member Properties never reads the instance/source facts.
///Actual positive4CEF10 application remains a required child service.
pub fn organization_added(
    catalog: &Catalog<'_>,
    record: &crate::live_object_init::Record<'_>,
    state: &State,
    instance: Object,
) -> Result<(), String> {
    use aa_actors::components::ComponentSlot;
    use aa_mapworld::restore::RestoredComponent;
    let source_pc = source_precision(catalog)?;
    let world = Applicable {
        catalog,
        record,
        state,
        source_pc,
    };
    let mut index = 0usize;
    loop {
        let Some(org) = world
            .organization(record.owner)
            .map_err(|e| e.to_string())?
        else {
            return Err("5787F0 actual current Organization allocation is absent".into());
        };
        if index >= org.members.len() {
            return Ok(());
        }
        let member = org.members[index];
        if !member.is_null() {
            let actor = world.actor(member).map_err(|e| e.to_string())?;
            if let Some(component) = actor.components.get(ComponentSlot::Properties) {
                if !matches!(component,RestoredComponent::Properties(p)if p.owner.0==member.raw()) {
                    return Err("5787F0 actual member Properties payload/owner differs".into());
                }
                state.require_catalog(catalog)?;
                let value = state
                    .instances
                    .get(&instance)
                    .ok_or("5787F0 actual current instance+C is absent")?;
                if state.facts(value.definition)?.pass_members
                    && !(actor.components.present(ComponentSlot::Experience) && value.kind == 7)
                    && !value.disabled
                {
                    return Err(format!(
                        "5787F0 reached positive4CEF10 member {:#010x}, Property {}, kind {}, source {:#010x}",
                        member.raw(),
                        value.definition.0,
                        value.kind,
                        record.owner.raw()
                    ));
                }
            }
        }
        index = index.checked_add(1).ok_or("5787F0 member index overflow")?;
    }
}
///The saved component owner currently has no Settlement payload variant.
///Do not replace its reached target/list reads with a false source flag.
pub fn settlement_added(
    _catalog: &Catalog<'_>,
    _record: &crate::live_object_init::Record<'_>,
    _state: &State,
    _instance: Object,
) -> Result<(), String> {
    Err("584F80 actual Settlement+14/+18/+1C/+24/+28 allocation is required before its lazy property gate".into())
}
/// Shared5030F0 allocation prefix. The completed allocation is returned by
/// value to its immediate insertion caller; no callback occurs between return
/// and registration in the same State.instances directory. A failed clock read
/// leaves the constructor prefix and reserved identity in this State.
pub(crate) fn construct_instance(
    state: &mut State,
    definition: Object,
    kind: u32,
    source: Option<aa_actors::ActorId>,
    now: impl FnOnce() -> Result<u32, String>,
) -> Result<aa_body::property_effects::Instance, String> {
    let handle = state.reserve_handle()?;
    state.constructors.insert(handle, ConstructorPrefix {
        definition,
        kind,
        source_id: source.map_or(0, aa_actors::ActorId::raw),
    });
    let timestamp = now()?;
    let prefix = state.constructors.remove(&handle)
        .expect("new5030F0 allocation remains owned");
    Ok(aa_body::property_effects::Instance::new(
        handle, prefix.definition, prefix.kind, aa_body::F32(timestamp),
        source.map(aa_actors::ActorId::raw),
    ))
}
///Direct5817A0 after any caller-owned4CEF10/5035E0 admission. All failures
///retain the same constructor/attachment/aggregate/callback prefix.
#[allow(clippy::too_many_arguments)]
pub fn add<E: Effects>(
    catalog: &Catalog<'_>,
    record: &mut crate::live_object_init::Record<'_>,
    state: &mut State,
    property: u32,
    kind: u32,
    source: Option<aa_actors::ActorId>,
    now: impl FnOnce() -> Result<u32, String>,
    pc: Precision,
    effects: &mut E,
) -> Result<(), String> {
    use aa_actors::components::ComponentSlot;
    use aa_mapworld::restore::RestoredComponent;
    crate::load_actor_init_world::validate(catalog, record).map_err(|e| e.to_string())?;
    let properties = record
        .shell
        .actor
        .as_ref()
        .ok_or("5817A0 actual Actor allocation is absent")?
        .components
        .get(ComponentSlot::Properties)
        .ok_or("5817A0 actual Properties component is absent")?;
    if !matches!(properties,RestoredComponent::Properties(p)if p.owner.0==record.owner.raw()) {
        return Err("5817A0 Properties payload/owner differs".into());
    }
    state.bind_registered_sources(catalog)?;
    let reference =
        PropertyRef::from_handle(property).ok_or("5817A0 Property reference is null/invalid")?;
    let definition = state.definition_for_reference(reference)?;
    let instance = construct_instance(state, definition, kind, source, now)?;
    let tail;
    {
        let actor = record.shell.actor.as_mut().unwrap();
        let (properties, components) = actor.components.split_slot(ComponentSlot::Properties);
        let Some(RestoredComponent::Properties(properties)) = properties else {
            return Err("5817A0 Properties allocation changed".into());
        };
        let mut kernel = Kernel {
            catalog,
            fields: Fields {
                definition: record.definition,
                precision: pc,
                owner: record.owner,
                words: &mut actor.words,
                components,
                registry: record.registry,
            },
            facts: &state.facts,
            effects,
            tail: None,
            detached: &mut state.detached,
        };
        aa_body::property_effects::insert_with(
            instance,
            properties,
            &mut state.instances,
            &state.definitions,
            &mut kernel,
            |handle, p, i, definitions, effects| {
                aa_body::property_flags::add_modifiers(handle, p, i, definitions, pc, effects)
            },
        )
        .map_err(|e| e.to_string())?;
        tail = kernel.tail;
    }
    if let Some(handle) = tail {
        notify(catalog, record, state, handle, pc, effects)?;
    }
    Ok(())
}
pub fn initial<E: Effects>(
    catalog: &Catalog<'_>,
    record: &mut crate::live_object_init::Record<'_>,
    state: &mut State,
    property: &crate::load_post_create_world::InitialProperty,
    now: impl FnOnce() -> Result<u32, String>,
    pc: Precision,
    effects: &mut E,
) -> Result<(), String> {
    state.bind_registered_sources(catalog)?;
    let definition = state.definition_for_reference(property.reference)?;
    if state.facts(definition)?.sources != property.sources {
        return Err("initial Property source owner differs".into());
    }
    add(
        catalog,
        record,
        state,
        property.reference.handle().map_err(|e| e.to_string())?,
        0,
        None,
        now,
        pc,
        effects,
    )
}
struct Applicable<'a, 'source, 'record> {
    catalog: &'a Catalog<'source>,
    record: &'a crate::live_object_init::Record<'record>,
    state: &'a State,
    source_pc: Precision,
}
impl Applicable<'_, '_, '_> {
    fn actor(
        &self,
        id: aa_actors::ActorId,
    ) -> aa_actors::state_effects::Result<
        &aa_actors::loading::ActorShell<aa_mapworld::restore::RestoredComponent>,
    > {
        use aa_mapworld::restore::RestoredRecord;
        let shell = if id == self.record.owner {
            &*self.record.shell
        } else {
            let entry = self.record.registry.get(id).ok_or_else(|| {
                app_error("property applicability actor generation is not current")
            })?;
            match &entry.value {
                RestoredRecord::Actor(e) => &e.shell,
                RestoredRecord::Kingdom(e) => {
                    &e.runtime
                        .as_ref()
                        .ok_or_else(|| {
                            app_error("property applicability Kingdom allocation absent")
                        })?
                        .base
                }
                _ => {
                    return Err(app_error(
                        "property applicability current record is not Actor storage",
                    ));
                }
            }
        };
        shell
            .actor
            .as_ref()
            .ok_or_else(|| app_error("property applicability actual Actor allocation absent"))
    }
    fn organization(
        &self,
        id: aa_actors::ActorId,
    ) -> aa_actors::state_effects::Result<
        Option<&aa_actors::ground_states::OrganizationState<String>>,
    > {
        use aa_mapworld::restore::RestoredComponent;
        match self
            .actor(id)?
            .components
            .get(aa_actors::components::ComponentSlot::Organization)
        {
            None => Ok(None),
            Some(RestoredComponent::Organization(org)) if org.header.owner == id => Ok(Some(org)),
            _ => Err(app_error(
                "property applicability actual Organization allocation/owner differs",
            )),
        }
    }
    fn definition_id(&self, key: Object) -> aa_actors::state_effects::Result<String> {
        let bank = self
            .catalog
            .definition_bank
            .try_borrow()
            .map_err(app_error)?;
        let entry = bank.entry(key.0).map_err(app_error)?;
        if entry.key.domain != "actordata" {
            return Err(app_error(
                "property applicability ActorData token is in another domain",
            ));
        }
        Ok(entry.key.name.clone())
    }
}
fn app_error(error: impl std::fmt::Display) -> aa_actors::state_effects::Error {
    aa_actors::state_effects::Error::World(error.to_string())
}
impl aa_actors::state_effects::Applicability for Applicable<'_, '_, '_> {
    fn has_properties(&self, actor: aa_actors::ActorId) -> aa_actors::state_effects::Result<bool> {
        Ok(component(self.actor(actor)?).map_err(app_error)?.is_some())
    }
    fn property_requirements(
        &self,
        property: Object,
    ) -> aa_actors::state_effects::Result<Vec<Object>> {
        Ok(self
            .state
            .facts(property)
            .map_err(app_error)?
            .required
            .clone())
    }
    fn actor_has_property(
        &self,
        actor: aa_actors::ActorId,
        property: Object,
    ) -> aa_actors::state_effects::Result<bool> {
        let Some(properties) = component(self.actor(actor)?).map_err(app_error)? else {
            return Ok(false);
        };
        aa_actors::state_effects::property_present(properties, &self.state.instances, property)
    }
    fn organization_count(
        &self,
        actor: aa_actors::ActorId,
    ) -> aa_actors::state_effects::Result<Option<usize>> {
        Ok(self.organization(actor)?.map(|org| org.members.len()))
    }
    fn organization_member(
        &self,
        actor: aa_actors::ActorId,
        index: usize,
    ) -> aa_actors::state_effects::Result<Option<aa_actors::ActorId>> {
        let org = self
            .organization(actor)?
            .ok_or_else(|| app_error("property applicability Organization disappeared"))?;
        let id = *org.members.get(index).ok_or_else(|| {
            app_error("property applicability member slot is outside current array")
        })?;
        Ok((!id.is_null()).then_some(id))
    }
    fn vacant_member_definition(
        &self,
        actor: aa_actors::ActorId,
        index: usize,
    ) -> aa_actors::state_effects::Result<Option<Object>> {
        let org = self
            .organization(actor)?
            .ok_or_else(|| app_error("property applicability Organization disappeared"))?;
        let data = self
            .catalog
            .layout_data(self.source_pc)
            .map_err(app_error)?;
        let layout = data
            .find_layout(&org.layout.definition)
            .ok_or_else(|| app_error("property applicability current layout source is absent"))?;
        let group = *layout
            .slot_groups
            .get(index)
            .ok_or_else(|| app_error("property applicability slot-group index is absent"))?;
        let id = org
            .layout
            .parameters
            .get(group as usize)
            .ok_or_else(|| app_error("property applicability expanded parameter is absent"))?;
        id.as_ref()
            .map(|id| token(self.catalog, "ActorData", id).map_err(app_error))
            .transpose()
    }
    fn definition_component_mask(
        &self,
        definition: Object,
    ) -> aa_actors::state_effects::Result<u32> {
        self.catalog
            .canonical_component_mask(&self.definition_id(definition)?, self.source_pc)
            .map_err(app_error)
    }
    fn definition_properties(
        &self,
        definition: Object,
    ) -> aa_actors::state_effects::Result<Vec<Object>> {
        let data = self
            .catalog
            .layout_data(self.source_pc)
            .map_err(app_error)?;
        let properties = aa_rules::native::layout_data::native_actor_properties(
            self.catalog.actors,
            &data,
            &self.definition_id(definition)?,
            self.source_pc,
        )
        .map_err(app_error)?;
        properties
            .into_iter()
            .map(|reference| {
                self.state
                    .definition_for_reference(reference)
                    .map_err(app_error)
            })
            .collect()
    }
}
///5035E0 over actual current Organization/member allocations and source-bound
///requirements. Disabled property duplicates still satisfy required properties.
pub fn applicable(
    catalog: &Catalog<'_>,
    record: &crate::live_object_init::Record<'_>,
    state: &mut State,
    property: u32,
) -> Result<bool, String> {
    crate::load_actor_init_world::validate(catalog, record).map_err(|e| e.to_string())?;
    state.bind_registered_sources(catalog)?;
    let reference = PropertyRef::from_handle(property)
        .ok_or("property applicability received null/invalid Property reference")?;
    let property = state.definition_for_reference(reference)?;
    aa_actors::state_effects::applicable(
        &Applicable {
            catalog,
            record,
            state,
            source_pc: source_precision(catalog)?,
        },
        record.owner,
        property,
    )
    .map_err(|e| e.to_string())
}

fn source_precision(catalog: &Catalog<'_>) -> Result<Precision, String> {
    catalog.require_source_owner()?;
    let bank = catalog
        .definition_bank
        .try_borrow()
        .map_err(|_| "property source Bank is already borrowed")?;
    let source = bank.combat_source()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("property source Bank belongs to another source universe".into());
    }
    Ok(source.source_precision())
}
///4D734A..59 reads the real Property manager's registration array slot0.
///An empty manager does not justify NULL: the native allocation slot was not
///initialized by47EFD0, so the first loaded definition is required here.
pub fn global(catalog: &Catalog<'_>) -> Result<Option<u32>, String> {
    let data = catalog.layout_data(source_precision(catalog)?)?;
    let first = data
        .properties
        .first()
        .ok_or("global Property manager has no initialized registration slot0")?;
    if first.ordinal != 0 || first.is_technology.is_none() {
        return Err("global Property registration slot0 is not completed".into());
    }
    Ok(Some(
        PropertyRef::Registered(0)
            .handle()
            .map_err(|e| e.to_string())?,
    ))
}
fn field(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedValue, String> {
    node.field(name, pc)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("PropertyData.{name} source is absent"))
}
fn text(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<String, String> {
    match field(node, name, pc)? {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(format!("PropertyData.{name} is not a string")),
    }
}
fn token(catalog: &Catalog<'_>, kind: &str, name: &str) -> Result<Object, String> {
    catalog
        .definition_bank
        .try_borrow_mut()
        .map_err(|_| "property reference Bank is already borrowed")?
        .intern(kind, name)
        .map(Object)
}
fn named_token(
    catalog: &Catalog<'_>,
    definition: &aa_rules::abilities::DefinitionIdentity,
) -> Result<Object, String> {
    let registry = catalog
        .actors
        .native_data_registry(&definition.kind)
        .ok_or_else(|| format!("actual {} registry is unavailable", definition.kind))?;
    let origin = definition
        .sources
        .first()
        .ok_or("property reference source origin is absent")?;
    let entries = registry
        .entries
        .iter()
        .filter(|entry| entry.source == *origin)
        .collect::<Vec<_>>();
    let [entry] = entries.as_slice() else {
        return Err("property reference canonical registration source is absent/ambiguous".into());
    };
    catalog
        .actors
        .native_data_node(entry)
        .map_err(|e| e.to_string())?;
    token(catalog, &definition.kind, &entry.ids)
}
fn modifiers(
    catalog: &Catalog<'_>,
    node: &ResolvedNode<'_>,
    pc: Precision,
) -> Result<Vec<aa_body::property_flags::Modifier>, String> {
    use aa_body::{
        F32,
        modifier_tables::TableModifier,
        property_effects::{NumericKind, NumericModifier},
        property_flags::{FlagModifier, Modifier},
    };
    let domain = catalog
        .actors
        .native_target_domain(pc)
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for child in node.children("Modifier").map_err(|e| e.to_string())? {
        let Some(input) = catalog
            .actors
            .native_property_modifier(&child, &domain, pc)
            .map_err(|e| e.to_string())?
        else {
            continue;
        };
        let converted = match input {
            EnchantmentModifier::Flag(flag) => Modifier::Flag(FlagModifier { index: flag.index }),
            EnchantmentModifier::Add {
                attribute,
                amount_bits,
                repair,
                ..
            } => Modifier::Numeric(NumericModifier {
                kind: NumericKind::Add,
                index: attribute,
                amount: F32(amount_bits),
                repair,
            }),
            //The existing source loader already performed the native18/23 negation.
            EnchantmentModifier::Multiply {
                attribute,
                amount_bits,
                ..
            } => Modifier::Numeric(NumericModifier {
                kind: NumericKind::Multiply,
                index: attribute,
                amount: F32(amount_bits),
                repair: false,
            }),
            EnchantmentModifier::ResistDamage {
                amount_bits,
                damage_type,
                ..
            } => Modifier::Resistance(aa_body::property_resistance::ResistDamage {
                damage_type: named_token(catalog, &damage_type)?,
                amount: F32(amount_bits),
            }),
            EnchantmentModifier::PeriodicDamage {
                damage_type,
                amount_bits,
                ..
            } => Modifier::Table(TableModifier::PeriodicDamage {
                damage_type: named_token(catalog, &damage_type)?,
                amount: F32(amount_bits),
            }),
            EnchantmentModifier::PeriodicHeal { amount_bits, .. } => {
                Modifier::Table(TableModifier::PeriodicHeal {
                    amount: F32(amount_bits),
                })
            }
            EnchantmentModifier::EnchantmentImmunity {
                enchantment_type, ..
            } => Modifier::Table(TableModifier::Immunity {
                enchantment_type: enchantment_type
                    .as_ref()
                    .map(|d| named_token(catalog, d))
                    .transpose()?,
            }),
            EnchantmentModifier::ResourceProduction { bits, .. } => {
                Modifier::Table(TableModifier::ResourceProduction {
                    values: bits.into_iter().map(F32).collect(),
                })
            }
            EnchantmentModifier::ReverseAbility { ability, .. } => {
                Modifier::Table(TableModifier::ReverseAbility {
                    definition: named_token(catalog, &ability)?,
                })
            }
            EnchantmentModifier::AbilityEffect { .. } => {
                return Err(
                    "Property ABILITY_EFFECT requires its actual loaded effect-object owner".into(),
                );
            }
        };
        result.push(converted);
    }
    Ok(result)
}
impl State {
    /// Source completion is immutable and bound to the SAME layout Property
    /// namespace and Bank. Registered reloads require their native cumulative
    ///writer; none are present in the current165-definition source census.
    pub fn bind_registered_sources(&mut self, catalog: &Catalog<'_>) -> Result<(), String> {
        if let Some(error) = &self.source_failed {
            return Err(error.clone());
        }
        let result = self.bind_registered_sources_inner(catalog);
        if let Err(error) = &result {
            self.source_failed = Some(error.clone());
        }
        result
    }
    fn bind_registered_sources_inner(&mut self, catalog: &Catalog<'_>) -> Result<(), String> {
        let pc = source_precision(catalog)?;
        if let Some(receipt) = &self.receipt {
            if receipt.identities != catalog.actors.sources || receipt.precision != pc {
                return Err(
                    "property allocation owner belongs to another source universe/precision".into(),
                );
            }
            for (reference, key) in &self.references {
                let facts = self.facts(*key)?;
                let bank = catalog
                    .definition_bank
                    .try_borrow()
                    .map_err(|_| "property source Bank is borrowed")?;
                if bank.token("Property", &facts.id) != Some(key.0) || facts.reference != *reference
                {
                    return Err("property allocation owner and canonical Bank token differ".into());
                }
            }
            return Ok(());
        }
        let data = catalog.layout_data(pc)?;
        let roots = catalog
            .actors
            .resolved_definitions("Property")
            .map_err(|e| e.to_string())?;
        for property in &data.properties {
            let reference = PropertyRef::Registered(property.ordinal);
            if property.reloads != 0 || property.is_technology.is_none() {
                return Err(
                    "reloaded/incomplete PropertyData effect source owner is unavailable".into(),
                );
            }
            let candidates = roots
                .iter()
                .filter(|node| node.sources() == property.sources)
                .collect::<Vec<_>>();
            let [node] = candidates.as_slice() else {
                return Err(format!(
                    "Property {} canonical source node is absent/ambiguous",
                    property.ids
                ));
            };
            let key = token(catalog, "Property", &property.ids)?;
            let mods = modifiers(catalog, node, pc)?;
            let flags = catalog
                .actors
                .native_target_domain(pc)
                .map_err(|e| e.to_string())?
                .flag_word(&field(node, "enabled_target_flags", pc)?)
                .map_err(|e| e.to_string())?;
            let mut required = Vec::new();
            for name in aa_rules::abilities::required_property_tokens(&text(
                node,
                "required_properties",
                pc,
            )?)
            .map_err(|e| e.to_string())?
            {
                let found = data
                    .find_property(&name, None)
                    .ok_or_else(|| format!("Unknown required Property '{name}'"))?;
                let PropertyRef::Registered(index) = found else {
                    return Err("required registered Property resolved to embedded source".into());
                };
                let target = data
                    .properties
                    .get(index as usize)
                    .ok_or("required Property registration ordinal is absent")?;
                required.insert(0, token(catalog, "Property", &target.ids)?);
            }
            let facts = DefinitionFacts {
                reference,
                id: property.ids.clone(),
                sources: property.sources.clone(),
                required,
                pass_members: field(node, "pass_down_members", pc)?
                    .boolean()
                    .map_err(|e| e.to_string())?,
                pass_recruit: field(node, "pass_down_recruit", pc)?
                    .boolean()
                    .map_err(|e| e.to_string())?,
                pass_denizen: field(node, "pass_down_denizen", pc)?
                    .boolean()
                    .map_err(|e| e.to_string())?,
                cv_bias: aa_body::F32(
                    field(node, "performer_cv_bias", pc)?
                        .fixed_bits()
                        .map_err(|e| e.to_string())?,
                ),
                fx_added: node
                    .children("WhenAdded")
                    .map_err(|e| e.to_string())?
                    .iter()
                    .map(|n| n.sources())
                    .collect(),
            };
            self.sources.insert(key, property.sources.clone());
            self.definitions.insert(
                key,
                aa_body::property_effects::Definition {
                    flags,
                    modifiers: mods,
                },
            );
            self.facts.insert(key, facts);
            self.references.insert(reference, key);
        }
        self.receipt = Some(SourceReceipt {
            identities: catalog.actors.sources.clone(),
            precision: pc,
        });
        Ok(())
    }
}
