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

///No public constructor: this is produced only at the completed E0 boundary.
pub(crate) struct CompletedDestructor {
    owner:crate::actor_world::ActorStoreWitness,
    key:ObjectKey,
    id:ActorId,
}
impl CompletedDestructor {
    pub(crate) fn into_parts(self,store:&crate::actor_world::ActorStore)->Result<(ObjectKey,ActorId)> {
        if !self.owner.is_for(store) {return Err("completed destructor belongs to another ActorStore".into());}
        Ok((self.key,self.id))
    }
}

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
    if !matches!(class, 2 | 3 | 4 | 6 | 7 | 9)
        || !crate::load_spatial::same_proof(&runtime.thing.definition, &runtime.definition.proof)
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
        Err(crate::actor_world::Error::Missing(
            "replacement read-only placement loan",
        ))
    }
}
struct MemoryRead<'a, 'source> {
    staged: &'a Staged,
    catalog: &'a Catalog<'source>,
    key: ObjectKey,
}
impl crate::live_object_init::MemoryFields for MemoryRead<'_, '_> {
    fn phase(&mut self) -> aa_actors::actor_creation::Result<u32> {
        Err(aa_actors::actor_creation::Error::Unsupported(
            "4EA350 does not read Session phase",
        ))
    }
    fn mask(&mut self) -> aa_actors::actor_creation::Result<u32> {
        canonical(self.staged, self.catalog, self.key)
            .map_err(aa_actors::actor_creation::Error::World)?
            .definition
            .component_mask
            .ok_or(aa_actors::actor_creation::Error::Unsupported(
                "replacement actual Actor component mask",
            ))
    }
    fn flags(&mut self) -> aa_actors::actor_creation::Result<u32> {
        self.staged
            .actors
            .placement(self.key, &ReadSpatial(self.staged))
            .map(|placement| placement.flags)
            .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))
    }
    fn announce(&mut self) -> aa_actors::actor_creation::Result<u8> {
        field(self.staged, self.catalog, self.key, 0x16d)
            .map_err(aa_actors::actor_creation::Error::World)
    }
}
fn removal_potentially_visible(
    staged: &Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
) -> Result<bool> {
    let class = canonical(staged, catalog, key)?.definition.native_class;
    crate::live_object_init::potentially_visible(
        class,
        &mut MemoryRead {
            staged,
            catalog,
            key,
        },
    )
    .map_err(|e| e.to_string())
}
fn clear_registration_flags(staged: &mut Staged, key: ObjectKey, bits: u32) -> Result<()> {
    if !matches!(bits, 0x08000000 | 0x10000000 | 0x18000000) {
        return Err("4E44E0 unexpected registration bit clear".into());
    }
    if staged.actors.staged_placement(key).is_ok() {
        staged
            .actors
            .staged_placement_mut(key)
            .map_err(|v| v.to_string())?
            .flags &= !bits;
    } else {
        let world = staged
            .load_world
            .as_mut()
            .ok_or("4E44E0 actual World absent")?;
        let crate::load_world::SpatialOwner::Adopted(adopted) = &mut world.spatial else {
            return Err("4E44E0 published flags require same adopted spatial".into());
        };
        adopted
            .world
            .clear_registration_flags(key, bits)
            .map_err(|v| v.to_string())?;
    }
    Ok(())
}
fn or_removal_flags(staged: &mut Staged, key: ObjectKey, bits: u32) -> Result<()> {
    if !matches!(bits, 1 | 2 | 0x81) {
        return Err("4D7580 unexpected removal flag mask".into());
    }
    if staged.actors.staged_placement(key).is_ok() {
        staged
            .actors
            .staged_placement_mut(key)
            .map_err(|v| v.to_string())?
            .flags |= bits;
    } else {
        let world = staged
            .load_world
            .as_mut()
            .ok_or("4D7580 actual World absent")?;
        let crate::load_world::SpatialOwner::Adopted(adopted) = &mut world.spatial else {
            return Err("4D7580 published flags require same adopted spatial".into());
        };
        adopted
            .world
            .or_removal_flags(key, bits)
            .map_err(|v| v.to_string())?;
    }
    Ok(())
}
fn actor_word(staged: &Staged, catalog: &Catalog<'_>, key: ObjectKey, offset: u32) -> Result<u32> {
    canonical(staged, catalog, key)?
        .actor
        .as_ref()
        .ok_or("4D7580 actual Actor absent")?
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| format!("4D7580 actual Actor+{offset:X} unbound"))
}
fn properties_cleanup(staged: &mut Staged, catalog: &Catalog<'_>, key: ObjectKey) -> Result<()> {
    let id = staged.actors.id(key).ok_or("5812A0 current GTID absent")?;
    canonical(staged, catalog, key)?;
    let target = crate::scheduler_owners::Target::Component {
        object: key,
        slot: ComponentSlot::Properties as u8,
    };
    let owner = match staged.owners.get(target) {
        Ok(owner) => Some(owner),
        //Portable tokens are allocated when this actual instance first enters
        //the saved-ID or scheduling owner. Without a token it has no queue links.
        Err(crate::scheduler_owners::Error::Missing) => None,
        Err(v) => return Err(v.to_string()),
    };
    let actor = staged
        .actors
        .object_mut(key)
        .map_err(|v| v.to_string())?
        .actor
        .as_mut()
        .ok_or("5812A0 actual Actor absent")?;
    let Some(RuntimeComponent::Properties(properties)) =
        actor.components.get_mut(ComponentSlot::Properties)
    else {
        return Err("5812A0 actual Properties slot/class differs".into());
    };
    if properties.owner.0 != id.raw() {
        return Err("5812A0 actual Properties owner differs".into());
    }
    let lifetime = &staged
        .load_world
        .as_ref()
        .ok_or("50B7A0 actual World absent")?
        .updatable_lifetime;
    properties_fields_cleanup(
        properties,
        owner,
        target,
        &mut staged.owners,
        &mut staged.queues,
        &mut staged.updatables,
        lifetime,
    )?;
    actor.components.remove(ComponentSlot::Properties);
    Ok(())
}
fn properties_fields_cleanup(
    properties: &mut aa_body::properties::PropertiesState,
    owner: Option<aa_world::queues::Owner>,
    target: crate::scheduler_owners::Target,
    owners: &mut crate::scheduler_owners::Owners,
    queues: &mut aa_world::queues::Queues,
    table: &mut crate::updatables::Table<aa_world::queues::Owner>,
    lifetime: &crate::updatable_lifetime::State,
) -> Result<()> {
    if properties.scheduler_link.is_some() {
        return Err(
            "5812A0 unbound raw Properties scheduler head must not replace owned queues".into(),
        );
    }
    if let Some(owner) = owner {
        if owners.target(owner).map_err(|v| v.to_string())? != target {
            return Err("5812A0 current Updatable identity inverse differs".into());
        }
    }
    if let Some(owner) = owner {
        queues.cancel_owner(owner).map_err(|v| v.to_string())?;
    }
    properties.scheduler_link = None;
    if !properties.effects.is_empty() {
        return Err("5812A0 positive Property heap-instance cleanup required".into());
    }
    //502850's positive reverse Ability allocations have their own live arena.
    //Do not discard those handles through a plain Vec drop.
    if !properties.modifier.reverse_abilities.is_empty() {
        return Err("502850 positive reverse Ability allocation cleanup required".into());
    }
    properties.modifier.periodic_damage.clear();
    properties.modifier.periodic_heals.clear();
    properties.modifier.damage_factors.clear();
    properties.modifier.production.clear();
    properties.modifier.ability_effects.clear();
    properties.modifier.immunity_types.clear();
    if lifetime.saved_table_present()
        && let Some(owner) = owner
    {
        lifetime.require_live_table_count(table.len())?;
        table.remove(&owner);
    }
    if lifetime.save_hash_present() {
        return Err("50B7A0 positive save-enumeration hash removal required".into());
    }
    if owner.is_some() {
        owners.retire(target, queues).map_err(|v| v.to_string())?;
    }
    Ok(())
}
fn base_object_uninit_prefix(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    retain: bool,
    pc: Precision,
) -> Result<()> {
    canonical(staged, catalog, key)?;
    let id = staged.actors.id(key).ok_or("4EA6F0 current GTID absent")?;
    if !retain {
        let flags = staged
            .actors
            .placement(key, &ReadSpatial(staged))
            .map_err(|v| v.to_string())?
            .flags;
        staged
            .load_world
            .as_mut()
            .ok_or("4F3F10 actual World list absent")?
            .objects
            .remove_first(id, flags)
            .map_err(|v| v.to_string())?;
    }
    let object = canonical(staged, catalog, key)?
        .object
        .as_ref()
        .ok_or("4EA6F0 actual Object absent")?;
    if staged
        .actors
        .key(ActorId::from_raw(object.words_50_to_54[0]))
        .is_some()
    {
        return Err("4EA6F0 liveObject50 requires4D3DD0/recursive4F5490".into());
    }
    if !retain && (object.byte_4c as i8) > 0 {
        return Err("4EA6F0 positive signedObject4C requires4F0390 zone-grid removal".into());
    }
    let flags = staged
        .actors
        .placement(key, &ReadSpatial(staged))
        .map_err(|v| v.to_string())?
        .flags;
    if flags & 2 == 0 {
        return Err("4EA6F0 actual42A640 destroy FX required before bit1 store".into());
    }
    if retain {
        return Ok(());
    }
    let world=staged.load_world.as_mut().ok_or("4EE560 actual World absent")?;
    let crate::load_world::SpatialOwner::Adopted(adoption)=&mut world.spatial else {
        return Err("4EE560 same load index must be adopted before destruction".into());
    };
    if adoption.error.is_some() || !adoption.pending.is_empty() {
        return Err("4EE560 actual spatial adoption is incomplete".into());
    }
    staged.actors.detach_spatial_for_destruction(key,&mut adoption.world,pc)
        .map_err(|e|e.to_string())?;
    staged.actors.retire_identity(key).map_err(|e|e.to_string())?;
    Ok(())
}
fn actor_uninit(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    retain: bool,
    pc: Precision,
) -> Result<()> {
    if canonical(staged, catalog, key)?.definition.native_class != 3 {
        return Err("4D7580 actual ordinary Actor vtable required".into());
    }
    or_removal_flags(staged, key, 1)?;
    //Native4D37E0 traverses no node when the actual constructor/list head isNULL.
    if actor_word(staged, catalog, key, 0x130)? != 0 {
        return Err("4D37E0 positive timed-list owner/unlink required".into());
    }
    if actor_word(staged, catalog, key, 0x144)? != 0 {
        return Err("4D7580 positive144 allocation cleanup required".into());
    }
    let active = staged
        .inactive_load
        .as_ref()
        .ok_or("4D7580 actual SAI observer owner absent")?
        .observer_active()?;
    if active {
        return Err("4D7580 active7A0A40 category transition required".into());
    }
    *staged
        .actors
        .object_mut(key)
        .map_err(|v| v.to_string())?
        .actor
        .as_mut()
        .unwrap()
        .words
        .get_mut(&0x140)
        .ok_or("4D7580 actual140 owner absent")? = 5;
    let companion = ActorId::from_raw(actor_word(staged, catalog, key, 0x158)?);
    if staged.actors.key(companion).is_some() {
        return Err("4D7580 live158 companion virtual12C required".into());
    }
    for zone in canonical(staged, catalog, key)?
        .actor
        .as_ref()
        .unwrap()
        .zones_148
        .zones()
        .slots
        .iter()
        .flatten()
    {
        if zone.radius_bits != aa_zones::zone::INACTIVE {
            return Err("4D7580 active Zone514CE0 uninit required".into());
        }
    }
    if canonical(staged, catalog, key)?
        .actor
        .as_ref()
        .unwrap()
        .components
        .state
        .low
        & (1 << 25)
        != 0
    {
        or_removal_flags(staged, key, 2)?;
    }
    if actor_word(staged, catalog, key, 0x160)? != 0 {
        return Err("4D19F0 actual160 owner removal required".into());
    }
    if staged
        .actors
        .key(ActorId::from_raw(actor_word(staged, catalog, key, 0x164)?))
        .is_some()
    {
        return Err("4DFF30 actual164 owner removal required".into());
    }
    let components = &canonical(staged, catalog, key)?
        .actor
        .as_ref()
        .unwrap()
        .components;
    if let Some(component) = components.get(ComponentSlot::Mind) {
        let RuntimeComponent::Mind(installed) = component else {
            return Err("4D7580 actual Mind slot/class differs".into());
        };
        let mind = staged
            .actors
            .minds
            .minds
            .get(&installed.mind_id)
            .ok_or("4D7580 actual Mind allocation absent")?;
        if mind.actor
            != staged
                .actors
                .id(key)
                .ok_or("4D7580 current GTID absent")?
                .raw()
        {
            return Err("4D7580 actual Mind owner differs".into());
        }
        if let Some(linked) = staged.actors.key(ActorId::from_raw(mind.words_24_30[1])) {
            let linked = canonical(staged, catalog, linked)?;
            if linked
                .actor
                .as_ref()
                .ok_or("4D7580 Mind+28 linked actual Actor absent")?
                .components
                .present(ComponentSlot::Economy)
            {
                return Err("4D7580 actual Mind+28 linked Economy removal568B10 required".into());
            }
        }
    }
    if let Some(component) = components.get(ComponentSlot::Element) {
        let RuntimeComponent::Element(element) = component else {
            return Err("4D7580 actual Element slot/class differs".into());
        };
        if element.header.owner != staged.actors.id(key).ok_or("4D7580 current GTID absent")? {
            return Err("4D7580 actual Element owner differs".into());
        }
        if element.organization.is_some() {
            return Err("4D7580 actual Element organization detach57CD70 required".into());
        }
    }
    if actor_word(staged, catalog, key, 0xe8)? != 0 {
        owner_remove::remove(staged, catalog, key, pc)?;
    }
    if staged
        .actors
        .key(ActorId::from_raw(actor_word(staged, catalog, key, 0x168)?))
        .is_some()
    {
        return Err("4D7580 live168 linked Actor removal requires actual SessionE4".into());
    }
    let components = &canonical(staged, catalog, key)?
        .actor
        .as_ref()
        .unwrap()
        .components;
    if components.present(ComponentSlot::Structure) {
        return Err("589CC0 actual Structure uninit required".into());
    }
    //4CF390 visits Mind first, then slots30..0; the pointer is cleared only
    //after its actual Uninit and destructor have both completed.
    for slot in std::iter::once(ComponentSlot::Mind)
        .chain(aa_actors::components::ALL_SLOTS.into_iter().rev())
    {
        if canonical(staged, catalog, key)?
            .actor
            .as_ref()
            .unwrap()
            .components
            .present(slot)
        {
            if slot == ComponentSlot::Properties {
                properties_cleanup(staged, catalog, key)?;
            } else {
                return Err(format!("4CF390 actual {slot:?} Uninit/destructor required"));
            }
        }
    }
    or_removal_flags(staged, key, 2)?;
    base_object_uninit_prefix(staged, catalog, key, retain, pc)
}
///Actual4D55E0→4D1B80 empty-owned-child path. Errors preserve all earlier
///writes and keep the allocation alive after its already retired live GTID.
fn actor_destructor(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey)->Result<CompletedDestructor> {
    if canonical(staged,catalog,key)?.definition.native_class!=3 {
        return Err("4D55E0 requires actual ordinary Actor storage".into());
    }
    let id=staged.actors.native_identity_word(key).map_err(|e|e.to_string())?;
    if staged.actors.id(key).is_some() {
        return Err("4D55E0 actor must have reached4EFEC0 before actual E0".into());
    }
    //B3652C is the same SAIMap owner, not a sound/presentation preference.
    let world=staged.load_world.as_ref().ok_or("4D1B80 actual World/SAIMap absent")?;
    if world.sai_map.map_source()!=&staged.inventory.map_source {
        return Err("4D1B80 SAIMap belongs to another map source".into());
    }
    actor_destructor_fields(staged.actors.object_mut(key).map_err(|e|e.to_string())?,world.sai_map.lifecycle())?;
    Ok(CompletedDestructor {owner:staged.actors.identity_witness(),key,id})
}
pub(crate) fn actor_destructor_fields(runtime:&mut RuntimeObject,map:&aa_mind::sai::manager::SaiMapLifecycle)->Result<()> {
    let actor=runtime.actor.as_mut().ok_or("4D1B80 actual Actor allocation absent")?;
    let word=|offset|actor.words.get(&offset).copied().ok_or_else(||format!("4D1B80 actual Actor{offset:X} owner absent"));
    if map.analysed && map.dynamic && word(0xe8)?!=0
    {
        return Err("4D1B80 actual4282A0 analysed/dynamic map notification required".into());
    }
    if word(0x154)?!=0 {
        return Err("4D1B80 positive SAIActor154 validation/release required".into());
    }
    actor.zones_148.release_null_for_destructor().map_err(str::to_owned)?;
    for head in [0x130,0x128,0x120,0x118] {
        if actor.words.get(&head).copied().ok_or_else(||format!("4D1B80 actual Actor{head:X} owner absent"))?!=0 {
            return Err(format!("4D1B80 positive Actor{head:X} node cleanup required"));
        }
        //Native does not read/write the tail when the actual head wasNULL.
    }
    if actor.overlap_list_110.head().is_some() || actor.overlap_list_110.allocated_nodes()!=0 {
        return Err("4D1B80 positive overlap110 node cleanup required".into());
    }
    if actor.actor_list_108.head().is_some() || actor.actor_list_108.allocated_nodes()!=0 {
        return Err("4D1B80 positive build-target108 node cleanup required".into());
    }
    actor.string_e0_utf16.clear();
    actor.string_dc_utf16.clear();
    let memory=&runtime.object.as_ref().unwrap().memory;
    if memory.count()>0 {
        return Err("4EBDD0 positive detection-memory4EBB80 cleanup required".into());
    }
    if memory.allocated_records()!=0 {
        return Err("4EBDD0 count/storage owner differs".into());
    }
    runtime.thing.name_utf16.clear();
    Ok(())
}
#[path = "map_replacement_remove_owner.rs"]
mod owner_remove;

///Required children are concrete native boundaries. This adapter performs the
///real removal prefix and supported ordinary-Actor Uninit fields. Required
///destructor children still stop after any earlier identity retirement.
pub struct Required;
impl Effects for Required {
    fn remove(
        &mut self,
        staged: &mut Staged,
        catalog: &Catalog<'_>,
        old: ObjectKey,
        pc: Precision,
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
                return Err(
                    "4EC290 requires actual SessionF8 before detection allocation/refresh/clear"
                        .into(),
                );
            }
            //4EA3D0 is signed count>0, not allocation-pointer presence.
            let count = canonical(staged, catalog, old)?
                .object
                .as_ref()
                .ok_or("4EA3D0 actual Object owner absent")?
                .memory
                .count();
            if count > 0 {
                actor_uninit(staged, catalog, old, true, pc)?;
                or_removal_flags(staged, old, 0x81)?;
                return Err("4F5430 actual retained World110/114 node insertion required after F0(1)/flags81".into());
            }
        }
        actor_uninit(staged, catalog, old, false, pc)?;
        //World+130 is the retained50CE00 presentation manager. Native50C1B0
        //returns immediately only while that actual manager's view+4 isNULL.
        if staged.load_world.as_ref().ok_or("50C1B0 actual World absent")?
            .presentation.attached_view().is_some() {
            return Err("50C1B0 actual attached-view43DB80 unlink required".into());
        }
        let completed=actor_destructor(staged,catalog,old)?;
        staged.actors.reclaim_retired(completed).map_err(|e|e.to_string())
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
