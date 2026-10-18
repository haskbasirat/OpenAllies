//!4FC4A0/4FBA50 over the retained queue and same staged world. Positive
//!unbound children stop at their reached call; queues and prior writes remain.
use crate::{
    actor_world::{ObjectKey, PlacementState, RuntimeComponent, RuntimeObject, Spatial},
    catalog::Catalog,
    map_replacement_queue::{ActorRequest, DefinitionRef, ObjectRequest, Queue, RequestRef},
    restoration::Staged,
};
use aa_actors::{ActorId, components::ComponentSlot, loading::LoadKind};
use aa_numeric::Precision;
pub type Result<T> = std::result::Result<T, String>;

///Same-world effects. No default success exists for destruction, creation,
///property application or the actual failure diagnostic/publication children.
pub trait Effects {
    fn remove(
        &mut self,
        staged: &mut Staged,
        catalog: &Catalog<'_>,
        old: ObjectKey,
        pc: Precision,
    ) -> Result<()>;
    fn virtual_160(
        &mut self,
        staged: &mut Staged,
        catalog: &Catalog<'_>,
        old: ObjectKey,
        argument: u32,
        pc: Precision,
    ) -> Result<()>;
    fn create_object(
        &mut self,
        staged: &mut Staged,
        catalog: &Catalog<'_>,
        request: &ObjectRequest,
        pc: Precision,
    ) -> Result<Option<ObjectKey>>;
    fn create_actor(
        &mut self,
        staged: &mut Staged,
        catalog: &Catalog<'_>,
        request: &ActorRequest,
        pc: Precision,
    ) -> Result<Option<ObjectKey>>;
    fn property(
        &mut self,
        staged: &mut Staged,
        catalog: &Catalog<'_>,
        new: ObjectKey,
        definition: Option<&DefinitionRef>,
        pc: Precision,
    ) -> Result<()>;
    fn created_log(&mut self, staged: &mut Staged, request: RequestRef<'_>) -> Result<()>;
    fn failure_log(&mut self, staged: &mut Staged, request: RequestRef<'_>) -> Result<()>;
    fn failure_text_ui(
        &mut self,
        staged: &mut Staged,
        catalog: &Catalog<'_>,
        request: RequestRef<'_>,
        pc: Precision,
    ) -> Result<()>;
}
trait World {
    type Key: Copy;
    fn resolve(&mut self, id: ActorId) -> Result<Option<Self::Key>>;
    fn category(&mut self, key: Self::Key) -> Result<u32>;
    fn b4(&mut self, key: Self::Key) -> Result<bool>;
    fn announce(&mut self, key: Self::Key) -> Result<u8>;
    fn remove(&mut self, key: Self::Key) -> Result<()>;
    fn virtual_160(&mut self, key: Self::Key) -> Result<()>;
    fn create(&mut self, request: RequestRef<'_>) -> Result<Option<Self::Key>>;
    fn property(&mut self, key: Self::Key, definition: Option<&DefinitionRef>) -> Result<()>;
    fn created_log(&mut self, request: RequestRef<'_>) -> Result<()>;
    fn failure(&mut self, request: RequestRef<'_>) -> Result<()>;
}
fn source(request: RequestRef<'_>) -> ActorId {
    match request {
        RequestRef::Object(v) => v.source,
        RequestRef::Actor(v) => v.source,
    }
}
///4FBA50: a false creation result is returned only after its real failure
///diagnostic child. Required-child errors retain all earlier world mutations.
fn replace<W: World>(world: &mut W, request: RequestRef<'_>) -> Result<bool> {
    let id = source(request);
    if id.raw() != 0
        && let Some(old) = world.resolve(id)?
    {
        let category = world.category(old)?;
        if category == 31 {
            if world.b4(old)? && world.announce(old)? != 0 {
                world.virtual_160(old)?;
            } else {
                world.remove(old)?;
            }
        } else if matches!(category, 34 | 35) {
            world.remove(old)?;
        }
    }
    let Some(new) = world.create(request)? else {
        world.failure(request)?;
        return Ok(false);
    };
    if let RequestRef::Actor(actor) = request {
        for property in &actor.properties {
            world.property(new, property.as_ref())?;
        }
    }
    world.created_log(request)?;
    Ok(true)
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub visited: usize,
    pub created: usize,
    pub failed: usize,
}
pub fn drain(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    queue: &Queue,
    effects: &mut impl Effects,
    pc: Precision,
) -> Result<Report> {
    let mut world = Live {
        staged,
        catalog,
        effects,
        pc,
    };
    let mut report = Report::default();
    //Native4FC4A0 retains both vectors and ignores false4FBA50 returns.
    for request in queue.native_order() {
        let created = replace(&mut world, request)?;
        report.visited += 1;
        if created {
            report.created += 1
        } else {
            report.failed += 1
        }
    }
    Ok(report)
}
struct Live<'a, 'source, E> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    effects: &'a mut E,
    pc: Precision,
}
fn canonical<'a>(
    staged: &'a Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
) -> Result<&'a RuntimeObject> {
    catalog.require_source_owner()?;
    let source = staged.definitions.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources {
        return Err("replacement current source Bank/Catalog differs".into());
    }
    let runtime = staged.actors.object(key).map_err(|e| e.to_string())?;
    crate::live_object_init::Source::bind_in_bank(
        catalog,
        &staged.definitions,
        &runtime.definition,
    )
    .map_err(|e| e.to_string())?;
    let class = runtime.definition.native_class;
    if !crate::load_spatial::same_proof(&runtime.thing.definition, &runtime.definition.proof)
        || runtime.kind
            != if matches!(class, 3 | 4) {
                LoadKind::Actor
            } else {
                LoadKind::Object
            }
        || runtime.object.is_none()
        || runtime.actor.is_some() != matches!(class, 3 | 4)
        || matches!(class, 3 | 4)
            && runtime.definition.component_mask
                != Some(catalog.canonical_component_mask(
                    &runtime.definition.proof.id,
                    source.source_precision(),
                )?)
    {
        return Err("replacement current constructor/source/storage certificate differs".into());
    }
    Ok(runtime)
}
fn field(staged: &Staged, catalog: &Catalog<'_>, key: ObjectKey, offset: u32) -> Result<u8> {
    let runtime = canonical(staged, catalog, key)?;
    let source = staged.definitions.combat_source()?;
    let fields = &staged
        .load_world
        .as_ref()
        .ok_or("replacement actual world source fields absent")?
        .source_fields;
    let d = fields
        .definition(&runtime.definition.proof.id)
        .map_err(|e| e.to_string())?;
    if fields.receipt().sources != catalog.actors.sources
        || fields.source_precision() != source.source_precision()
        || d.token().source() != &runtime.definition.proof.source
        || d.native_kind() != runtime.definition.proof.category.native_kind
        || d.registry_index() != runtime.definition.proof.category.registry_index
    {
        return Err("replacement current source field receipt differs".into());
    }
    fields
        .byte(&runtime.definition.proof.id, offset)
        .map_err(|e| e.to_string())
}
impl<E: Effects> World for Live<'_, '_, E> {
    type Key = ObjectKey;
    fn resolve(&mut self, id: ActorId) -> Result<Option<ObjectKey>> {
        Ok(self.staged.actors.key(id))
    }
    fn category(&mut self, key: ObjectKey) -> Result<u32> {
        Ok(canonical(self.staged, self.catalog, key)?
            .definition
            .proof
            .category
            .registry_index)
    }
    fn b4(&mut self, key: ObjectKey) -> Result<bool> {
        crate::live_registration::class_predicate(
            canonical(self.staged, self.catalog, key)?
                .definition
                .native_class,
            0xb4,
        )
    }
    fn announce(&mut self, key: ObjectKey) -> Result<u8> {
        field(self.staged, self.catalog, key, 0x16d)
    }
    fn remove(&mut self, key: ObjectKey) -> Result<()> {
        self.effects.remove(self.staged, self.catalog, key, self.pc)
    }
    fn virtual_160(&mut self, key: ObjectKey) -> Result<()> {
        self.effects
            .virtual_160(self.staged, self.catalog, key, 0, self.pc)
    }
    fn create(&mut self, request: RequestRef<'_>) -> Result<Option<ObjectKey>> {
        match request {
            RequestRef::Object(v) => {
                self.effects
                    .create_object(self.staged, self.catalog, v, self.pc)
            }
            RequestRef::Actor(v) => {
                self.effects
                    .create_actor(self.staged, self.catalog, v, self.pc)
            }
        }
    }
    fn property(&mut self, key: ObjectKey, definition: Option<&DefinitionRef>) -> Result<()> {
        self.effects
            .property(self.staged, self.catalog, key, definition, self.pc)
    }
    fn created_log(&mut self, request: RequestRef<'_>) -> Result<()> {
        self.effects.created_log(self.staged, request)
    }
    fn failure(&mut self, request: RequestRef<'_>) -> Result<()> {
        self.effects.failure_log(self.staged, request)?;
        self.effects
            .failure_text_ui(self.staged, self.catalog, request, self.pc)
    }
}

///Concrete first removal prefix4F5370. Class/available-city list writes target
///the actual registration owner, never a copied aa_world laboratory table.
pub fn removal_prefix(staged: &mut Staged, catalog: &Catalog<'_>, key: ObjectKey) -> Result<()> {
    let runtime = canonical(staged, catalog, key)?;
    let class = runtime.definition.native_class;
    let id = staged
        .actors
        .id(key)
        .ok_or("replacement old object has no current GTID")?;
    let direct = crate::live_registration::class_predicate(class, 0xb0)?
        && field(staged, catalog, key, 0x34)? != 0;
    let mut available = direct;
    if !direct && crate::live_registration::class_predicate(class, 0xb4)? {
        let actor = runtime
            .actor
            .as_ref()
            .ok_or("4F4C40 current actualActor absent")?;
        if let Some(city) = actor.components.get(ComponentSlot::City) {
            if !matches!(city, RuntimeComponent::City(_)) {
                return Err("4F4C40 actual City slot contains another component type".into());
            }
            let handle = ActorId::from_raw(
                *actor
                    .words
                    .get(&0x158)
                    .ok_or("4E10B0 currentActor158 owner absent")?,
            );
            available = if let Some(other) = staged.actors.key(handle) {
                field(staged, catalog, other, 0x34)? == 0
            } else {
                true
            };
        }
    }
    let world = staged
        .load_world
        .as_mut()
        .ok_or("replacement actual world owner absent")?;
    if available {
        world.registration.remove_available_city(id);
    }
    if crate::live_registration::class_predicate(class, 0xbc)? {
        world
            .registration
            .remove_classification(aa_world::ListKind::Class188, id)?;
    } else if crate::live_registration::class_predicate(class, 0xc0)? {
        world
            .registration
            .remove_classification(aa_world::ListKind::Class178, id)?;
    }
    Ok(())
}

struct ReadSpatial<'a>(&'a Staged);
impl Spatial for ReadSpatial<'_> {
    fn placement(&self, id: ActorId) -> crate::actor_world::Result<PlacementState> {
        match self.0.load_world.as_ref().map(|world| &world.spatial) {
            Some(crate::load_world::SpatialOwner::Adopted(adopted)) => adopted.world.placement(id),
            _ => Err(crate::actor_world::Error::Missing(
                "replacement published placement needs the same adopted spatial owner",
            )),
        }
    }
    fn publish(&mut self, _: ActorId, _: PlacementState) -> crate::actor_world::Result<()> {
        Err(crate::actor_world::Error::Missing("replacement read-only placement loan"))
    }
}
struct MemoryRead<'a, 'source> {
    staged: &'a Staged,
    catalog: &'a Catalog<'source>,
    key: ObjectKey,
}
impl crate::live_object_init::MemoryFields for MemoryRead<'_, '_> {
    fn phase(&mut self) -> aa_actors::actor_creation::Result<u32> {
        Err(aa_actors::actor_creation::Error::Unsupported("4EA350 does not read Session phase"))
    }
    fn mask(&mut self) -> aa_actors::actor_creation::Result<u32> {
        canonical(self.staged, self.catalog, self.key)
            .map_err(aa_actors::actor_creation::Error::World)?
            .definition.component_mask
            .ok_or(aa_actors::actor_creation::Error::Unsupported("replacement actual Actor component mask"))
    }
    fn flags(&mut self) -> aa_actors::actor_creation::Result<u32> {
        self.staged.actors.placement(self.key, &ReadSpatial(self.staged))
            .map(|placement| placement.flags)
            .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))
    }
    fn announce(&mut self) -> aa_actors::actor_creation::Result<u8> {
        field(self.staged, self.catalog, self.key, 0x16d)
            .map_err(aa_actors::actor_creation::Error::World)
    }
}
fn removal_potentially_visible(staged: &Staged, catalog: &Catalog<'_>, key: ObjectKey) -> Result<bool> {
    let class = canonical(staged, catalog, key)?.definition.native_class;
    crate::live_object_init::potentially_visible(class, &mut MemoryRead { staged, catalog, key })
        .map_err(|e| e.to_string())
}

///Required children are concrete native boundaries. This adapter performs the
///real removal-list prefix before refusing an unbound object virtual, and never
///retires a live GTID or fabricates a replacement allocation.
pub struct Required;
impl Effects for Required {
    fn remove(
        &mut self,
        staged: &mut Staged,
        catalog: &Catalog<'_>,
        old: ObjectKey,
        _: Precision,
    ) -> Result<()> {
        removal_prefix(staged, catalog, old)?;
        let phase = staged
            .load_world
            .as_ref()
            .ok_or("4F53D0 actual world absent")?
            .dispatcher
            .as_ref()
            .ok_or("4F53D0 actual WorldD8 owner absent")?
            .phase;
        if phase != 6 {
            //4EC290 evaluates4EA350 before touching actual Session+F8.
            if removal_potentially_visible(staged, catalog, old)? {
                return Err("4EC290 requires actual SessionF8 before detection allocation/refresh/clear".into());
            }
            //4EA3D0 is signed count>0, not allocation-pointer presence.
            let count = canonical(staged, catalog, old)?.object.as_ref()
                .ok_or("4EA3D0 actual Object owner absent")?.memory.count();
            if count > 0 {
                return Err("4F53D0 requires actual virtualF0(1) before retained-object flags/list stores".into());
            }
        }
        Err("4F53D0 requires actual old object virtualF0(0) before dispatch unlink/destructor".into())
    }
    fn virtual_160(
        &mut self,
        _: &mut Staged,
        _: &Catalog<'_>,
        _: ObjectKey,
        _: u32,
        _: Precision,
    ) -> Result<()> {
        Err("4FBB68 requires actual announced old Actor virtual160(0)".into())
    }
    fn create_object(
        &mut self,
        _: &mut Staged,
        _: &Catalog<'_>,
        _: &ObjectRequest,
        _: Precision,
    ) -> Result<Option<ObjectKey>> {
        Err("4FBB74 requires actual4F9C10 fresh object construction/Init/publication".into())
    }
    fn create_actor(
        &mut self,
        _: &mut Staged,
        _: &Catalog<'_>,
        _: &ActorRequest,
        _: Precision,
    ) -> Result<Option<ObjectKey>> {
        Err(
            "4FBB00 requires actual4FA590 fresh Actor creation/organization/linked object services"
                .into(),
        )
    }
    fn property(
        &mut self,
        _: &mut Staged,
        _: &Catalog<'_>,
        _: ObjectKey,
        _: Option<&DefinitionRef>,
        _: Precision,
    ) -> Result<()> {
        Err("4FBB32 requires actual4CEF10(newActor,Property,2,newActor)".into())
    }
    fn created_log(&mut self, _: &mut Staged, _: RequestRef<'_>) -> Result<()> {
        Err("4FBBAD requires actual404F90 created-replacement diagnostic owner".into())
    }
    fn failure_log(&mut self, _: &mut Staged, _: RequestRef<'_>) -> Result<()> {
        Err("4FBBFB requires actual404F30 failed-replacement diagnostic owner".into())
    }
    fn failure_text_ui(
        &mut self,
        _: &mut Staged,
        _: &Catalog<'_>,
        _: RequestRef<'_>,
        _: Precision,
    ) -> Result<()> {
        Err("4FBC43/4FBC97 require actual localized failure text/UI owner".into())
    }
}

#[cfg(test)]
#[path = "map_replacement_drain_tests.rs"]
mod tests;
