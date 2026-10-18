//! Original4FD7F0/4FD8C0 replacement capture and4FC500 queue ownership.
//! Capturing never destroys/creates/publishes an actor and consumes no RNG.
//! Full4FC4A0/4FBA50 execution remains an explicit caller dependency.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Spatial},
    definitions::Bank,
    skirmish_start::Definitions,
};
use aa_actors::{ActorId, actor_creation::ActorParams, components::ComponentSlot};
use aa_body::credit::Object;
use aa_mapworld::class_conversion::ObjectCreationParams;
use aa_numeric::Precision;
use aa_rules::actors::ActorCatalog;
use serde::Serialize;

pub type Result<T> = std::result::Result<T, String>;

/// A portable immutable definition identity plus its canonical source IDS.
/// ActorStore capture validates it against the same loaded definition bank.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DefinitionRef {
    pub token: u32,
    pub canonical_ids: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ObjectRequest {
    /// Record+80: the old object's GTID, distinct from an arena address.
    pub source: ActorId,
    pub params: ObjectCreationParams<Option<DefinitionRef>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ActorRequest {
    pub source: ActorId,
    pub params: ActorParams<DefinitionRef, String>,
    /// Record+84: definition pointers only, copied in Properties+10 order.
    /// Repetitions and null pointers are retained, and instances are not moved.
    pub properties: Vec<Option<DefinitionRef>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Request {
    Object(ObjectRequest),
    Actor(ActorRequest),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Object,
    Actor,
}
#[derive(Clone, Copy, Debug)]
pub enum RequestRef<'a> {
    Object(&'a ObjectRequest),
    Actor(&'a ActorRequest),
}

/// The two native vectors. Storage grows with demand; the original initial8
/// entries and doubling policy are allocation details, never gameplay limits.
#[derive(Debug, Default)]
pub struct Queue {
    objects: Vec<ObjectRequest>,
    actors: Vec<ActorRequest>,
}
impl Queue {
    pub fn objects(&self) -> &[ObjectRequest] {
        &self.objects
    }
    pub fn actors(&self) -> &[ActorRequest] {
        &self.actors
    }
    /// Native4FC500 copies the temporary request before that temporary dies.
    /// Moving its owned Rust payload provides the same independent lifetime.
    pub fn append(&mut self, request: Request) {
        match request {
            Request::Object(v) => self.objects.push(v),
            Request::Actor(v) => self.actors.push(v),
        }
    }
    ///4FC500 protects an aliased input before growing the backing vector.
    /// The source remains intact and its property vector is independently owned.
    pub fn append_existing(&mut self, kind: Kind, index: usize) -> Result<()> {
        let request = match kind {
            Kind::Object => Request::Object(
                self.objects
                    .get(index)
                    .ok_or("replacement object index absent")?
                    .clone(),
            ),
            Kind::Actor => Request::Actor(
                self.actors
                    .get(index)
                    .ok_or("replacement actor index absent")?
                    .clone(),
            ),
        };
        self.append(request);
        Ok(())
    }
    /// Inspection of4FC4A0's stable queue order, not replacement execution.
    pub fn native_order(&self) -> impl Iterator<Item = RequestRef<'_>> {
        self.objects
            .iter()
            .map(RequestRef::Object)
            .chain(self.actors.iter().map(RequestRef::Actor))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub id: ActorId,
    pub position_bits: [u32; 3],
    pub direction_bits: [u32; 2],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropertyInput {
    pub kind: u32,
    pub source: ActorId,
    pub definition: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Warning {
    pub actor: ActorId,
    pub property: DefinitionRef,
}

///4FD7F0 copies raw transform words, chooses variant0, keeps the source GTID
/// as requested ID, and deliberately leaves the new name empty.
pub fn object_payload(snapshot: Snapshot, definition: Option<DefinitionRef>) -> ObjectRequest {
    ObjectRequest {
        source: snapshot.id,
        params: ObjectCreationParams {
            definition,
            position_bits: snapshot.position_bits,
            direction_bits: snapshot.direction_bits,
            name_utf16: Vec::new(),
            variant: 0,
            requested_id: snapshot.id.raw(),
        },
    }
}

///4FD8C0's successful-allocation capture. Required-object resolution runs only
/// after the ordered property scan. Warnings already emitted survive an error.
/// Unknown handles are errors; a selected native null property remains null.
#[allow(clippy::too_many_arguments)] // Native independent owners/ordered callbacks.
pub fn actor_payload(
    snapshot: Snapshot,
    owner: Option<ActorId>,
    current_definition: u32,
    replacement: Option<DefinitionRef>,
    properties: impl IntoIterator<Item = Result<PropertyInput>>,
    resolve_property: &mut impl FnMut(u32) -> Result<DefinitionRef>,
    required_object: impl FnOnce(&DefinitionRef) -> Result<Option<u32>>,
    warnings: &mut Vec<Warning>,
) -> Result<ActorRequest> {
    actor_payload_with_warning(
        snapshot,
        owner,
        current_definition,
        replacement,
        properties,
        resolve_property,
        required_object,
        &mut |warning| {
            warnings.push(warning);
            Ok(())
        },
    )
}
///Same capture algorithm, with the actual404F60 child evaluated immediately
///inside the property scan. A required diagnostic refusal must precede later
///property reads, required-object lookup and final queue append.
#[allow(clippy::too_many_arguments)]
pub fn actor_payload_with_warning(
    snapshot: Snapshot,
    owner: Option<ActorId>,
    current_definition: u32,
    replacement: Option<DefinitionRef>,
    properties: impl IntoIterator<Item = Result<PropertyInput>>,
    resolve_property: &mut impl FnMut(u32) -> Result<DefinitionRef>,
    required_object: impl FnOnce(&DefinitionRef) -> Result<Option<u32>>,
    warning: &mut impl FnMut(Warning) -> Result<()>,
) -> Result<ActorRequest> {
    let mut kept = Vec::new();
    for property in properties {
        let property = property?;
        //503080 checks kind only; disabled and timing words are not read.
        if !matches!(property.kind, 1..=3) {
            continue;
        }
        if property.source.raw() != 0 && property.source != snapshot.id {
            let token = property
                .definition
                .ok_or("4FD9B2 foreign property definition is null")?;
            warning(Warning {
                actor: snapshot.id,
                property: resolve_property(token)?,
            })?;
        } else if property.kind == 2 {
            kept.push(
                property
                    .definition
                    .map(&mut *resolve_property)
                    .transpose()?,
            );
        }
    }
    let replacement = replacement.ok_or("4FDA16 replacement ActorData is null")?;
    let required = required_object(&replacement)?;
    let mut params = ActorParams::new(replacement);
    params.position_bits = snapshot.position_bits;
    params.direction_bits = snapshot.direction_bits;
    params.variant = 0;
    params.requested_id = snapshot.id.raw();
    params.owner = owner.map_or(0, ActorId::raw);
    params.mode = 5;
    params.word_3c = snapshot.id.raw();
    if required.is_some_and(|token| token != 0 && token == current_definition) {
        params.word_30 = snapshot.id.raw();
    }
    Ok(ActorRequest {
        source: snapshot.id,
        params,
        properties: kept,
    })
}

/// Read-only owners used by the real staged/published ActorStore adapter.
pub struct Context<'a, S> {
    pub actors: &'a ActorStore,
    pub spatial: &'a S,
    pub definitions: &'a Definitions,
    pub bank: &'a Bank,
    pub source: &'a ActorCatalog,
    pub precision: Precision,
}
impl<S: Spatial> Context<'_, S> {
    fn thing(&self, token: u32) -> Result<DefinitionRef> {
        let entry = self.bank.entry(token)?;
        let definition = self
            .definitions
            .things
            .get(&token)
            .ok_or("replacement ThingData token missing")?;
        if entry.key.domain != "actordefinition"
            || !entry.key.name.eq_ignore_ascii_case(&definition.ids)
        {
            return Err("replacement definition bank and source owner disagree".into());
        }
        Ok(DefinitionRef {
            token,
            canonical_ids: definition.ids.clone(),
        })
    }
    fn canonical_thing(&self, ids: &str) -> Result<DefinitionRef> {
        let (&token, _) = self
            .definitions
            .things
            .iter()
            .find(|(_, v)| v.ids.eq_ignore_ascii_case(ids))
            .ok_or("canonical replacement ThingData identity missing")?;
        self.thing(token)
    }
    fn property(&self, token: u32) -> Result<DefinitionRef> {
        let entry = self.bank.entry(token)?;
        if entry.key.domain != "property" {
            return Err("replacement PropertyData identity has wrong definition domain".into());
        }
        // The live instance already points at the canonical immutable Property
        // definition; never reapply a mutable textual alias lookup here.
        Ok(DefinitionRef {
            token,
            canonical_ids: entry.key.name.clone(),
        })
    }
    fn required_object(&self, replacement: &DefinitionRef) -> Result<Option<u32>> {
        let canonical = &replacement.canonical_ids;
        let actual = self
            .source
            .find(canonical)
            .ok_or("replacement ActorData source absent")?;
        if !actual.id.eq_ignore_ascii_case(canonical) {
            return Err("canonical replacement ActorData is shadowed by translation".into());
        }
        let bound = self
            .source
            .native_collision_definition(canonical, self.precision)
            .map_err(|e| e.to_string())?;
        bound
            .required_object
            .map(|v| self.canonical_thing(&v.id).map(|r| r.token))
            .transpose()
    }
    /// Capture native parameters now from canonical live owners. For an actor
    /// this uses the proved ordinary4D1E80 owner dispatch; derived/unavailable
    /// owner classes, missing property instances and unresolved source graphs
    /// remain errors. No actor/component snapshot is published as live state.
    pub fn capture(
        &self,
        queue: &mut Queue,
        key: ObjectKey,
        replacement: Option<u32>,
        kind: Kind,
        warnings: &mut Vec<Warning>,
    ) -> Result<()> {
        self.capture_with_warning(queue, key, replacement, kind, &mut |warning| {
            warnings.push(warning);
            Ok(())
        })
    }
    ///Concrete immediate diagnostic child. The legacy capture API retains its
    ///observational Vec sink, while live callers may require the real service.
    pub fn capture_with_warning(
        &self,
        queue: &mut Queue,
        key: ObjectKey,
        replacement: Option<u32>,
        kind: Kind,
        warning: &mut impl FnMut(Warning) -> Result<()>,
    ) -> Result<()> {
        let id = self
            .actors
            .id(key)
            .ok_or("replacement source has no registered GTID")?;
        let replacement = replacement.map(|id| self.thing(id)).transpose()?;
        let owner = if kind == Kind::Actor {
            self.actors.ordinary_owner(id).map_err(|e| e.to_string())?
        } else {
            None
        };
        let placement = self
            .actors
            .placement(key, self.spatial)
            .map_err(|e| e.to_string())?;
        let snapshot = Snapshot {
            id,
            position_bits: placement.position_bits,
            direction_bits: [placement.facing_bits[0], placement.facing_bits[1]],
        };
        if kind == Kind::Object {
            queue.append(Request::Object(object_payload(snapshot, replacement)));
            return Ok(());
        }
        let object = self.actors.object(key).map_err(|e| e.to_string())?;
        let current = self.canonical_thing(&object.definition.proof.id)?;
        let state = if let Some(component) = self
            .actors
            .component(id, ComponentSlot::Properties)
            .map_err(|e| e.to_string())?
        {
            let RuntimeComponent::Properties(state) = component else {
                return Err("Properties slot payload mismatch".into());
            };
            if state.owner != Object(id.raw()) {
                return Err("replacement Properties owner mismatch".into());
            }
            Some(state.as_ref())
        } else {
            None
        };
        let properties = state
            .into_iter()
            .flat_map(|state| state.effects.iter())
            .map(|handle| {
                let instance = self
                    .actors
                    .property_allocations
                    .instances()
                    .get(handle)
                    .ok_or("replacement property instance missing")?;
                if instance.handle != *handle {
                    return Err("replacement property instance handle mismatch".into());
                }
                Ok(PropertyInput {
                    kind: instance.kind,
                    source: ActorId::from_raw(instance.source_id),
                    definition: (instance.definition.0 != 0).then_some(instance.definition.0),
                })
            });
        let request = actor_payload_with_warning(
            snapshot,
            owner,
            current.token,
            replacement,
            properties,
            &mut |token| self.property(token),
            |definition| self.required_object(definition),
            warning,
        )?;
        queue.append(Request::Actor(request));
        Ok(())
    }
}
