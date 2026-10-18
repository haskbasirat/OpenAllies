//! One authoritative actor arena, shared by source restoration and new creation.
//! Arena tokens are portable object addresses, not GTIDs or iteration order.
//! Native identity metadata is owned solely by IdentityRegistry. Components are
//! moved on import; no saved Body/Mover state is cloned or reinitialized.
use crate::catalog::{ActorBindings, Catalog};
use aa_actors::{
    ActorId, IdentityRegistry,
    components::{ComponentSlot, Components},
    creation::{self, ActorCreationRecord, DirectionTable},
    loading::{ActorShell, LoadKind, LoadShell, ObjectShell, ThingShell},
    owned,
};
use aa_body::F32;
use aa_mapworld::{
    actor_payload::ActorTail,
    bootstrap::DefinitionProof,
    inventory::Placement,
    kingdoms::{KingdomRecord, TeamRecord},
    restore::{
        DecodedSavedComponent, LoadCompletion, LoadedEntity, RestoredRecord, RuntimeDefinition,
    },
    special_state::SpecialShell,
};
use aa_numeric::Precision;
use aa_simulation::NativeRng;
use std::{collections::BTreeMap, sync::{Arc, Weak}};
#[path="actor_property_focus.rs"]mod property_focus;
pub(crate) use property_focus::PropertyActorAccess;

pub type RuntimeComponent = owned::Component<
    String,
    aa_mind::construct::Installed,
    aa_combat::constructors::OwnedPerformer,
>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectKey(u64);
impl ObjectKey {
    pub fn raw(self) -> u64 {
        self.0
    }
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Creation(#[from] creation::CreationError),
    #[error(transparent)]
    Identity(#[from] aa_actors::registry::RegistryError),
    #[error(transparent)]
    Component(#[from] owned::Error),
    #[error("runtime actor source binding: {0}")]
    Source(String),
    #[error("runtime actor operation unavailable: {0}")]
    Missing(&'static str),
    #[error("runtime object identity exhausted")]
    Exhausted,
    #[error("actor already has a native identity")]
    AlreadyRegistered,
    #[error("component slot is already occupied")]
    Occupied,
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacementState {
    pub flags: u32,
    pub position_bits: [u32; 3],
    pub facing_bits: [u32; 3],
    pub cached_tile: Option<[i32; 2]>,
}
/// After publication there is no second transform/flag copy in the entity. The
/// Spatial storage owns the moved PlacementState and every actor getter routes
/// there. An import/new actor remains Staged until actual spatial publication.
#[derive(Debug)]
enum PlacementOwner {
    Staged(PlacementState),
    Spatial,
}
pub trait Spatial {
    fn placement(&self, id: ActorId) -> Result<PlacementState>;
    fn publish(&mut self, id: ActorId, placement: PlacementState) -> Result<()>;
}
#[derive(Debug)]
pub struct ThingCore {
    pub definition: DefinitionProof,
    ///Opaque portable handle owned by the current ViewDirectory. Zero is the
    ///native constructor's NULL; positive handles never encode GPU pointers.
    pub word_c: usize,
    pub word_10: u32,
    pub name_utf16: Vec<u16>,
    pub word_1c: Option<u32>,
}
/// Object fields excluding the live placement-cache words44/48. Those words
/// move into PlacementState, so published spatial changes cannot diverge from
/// a second mutable copy retained in the object shell.
#[derive(Debug)]
pub struct ObjectFields {
    pub associated_handle: u32,
    pub scale_bits: u32,
    pub variant: i32,
    pub byte_4c: u8,
    pub words_50_to_54: [u32; 2],
    pub memory: aa_actors::detection_memory::State,
}
impl From<ObjectShell> for ObjectFields {
    fn from(v: ObjectShell) -> Self {
        Self {
            associated_handle: v.associated_handle,
            scale_bits: v.scale_bits,
            variant: v.variant,
            byte_4c: v.byte_4c,
            words_50_to_54: v.words_50_to_54,
            memory: v.memory,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveComponent {
    None,
    Slot(ComponentSlot),
    UnresolvedNativeWord(u32),
}
#[derive(Debug)]
pub struct RuntimeObject {
    pub kind: LoadKind,
    pub active_component: ActiveComponent,
    pub definition: RuntimeDefinition,
    pub thing: ThingCore,
    placement: PlacementOwner,
    /// Old native Thing+14 after4EFEC0, retained only for its pending destructor.
    /// The live identity registry is still the sole source of resolvable GTIDs.
    retired_identity: Option<ActorId>,
    pub object: Option<ObjectFields>,
    pub actor: Option<ActorShell<RuntimeComponent>>,
}
impl RuntimeObject {
    fn from_shell(
        shell: LoadShell<DefinitionProof, RuntimeComponent>,
        definition: RuntimeDefinition,
    ) -> Self {
        let LoadShell {
            kind,
            thing,
            object,
            actor,
        } = shell;
        let ThingShell {
            definition: proof,
            flags,
            word_c,
            word_10,
            name_utf16,
            word_1c,
            position_bits,
            facing_bits,
        } = thing;
        let mut actor = actor;
        let active_component = match actor.as_mut().and_then(|a| a.words.remove(&0xe4)) {
            None | Some(0) => ActiveComponent::None,
            Some(word) => ActiveComponent::UnresolvedNativeWord(word),
        };
        let cached_tile = object
            .as_ref()
            .map(|o| [o.word_44 as i32, o.word_48 as i32]);
        Self {
            kind,
            active_component,
            definition,
            thing: ThingCore {
                definition: proof,
                word_c:word_c as usize,
                word_10,
                name_utf16,
                word_1c,
            },
            placement: PlacementOwner::Staged(PlacementState {
                flags,
                position_bits,
                facing_bits,
                cached_tile,
            }),
            retired_identity: None,
            object: object.map(ObjectFields::from),
            actor,
        }
    }
    pub fn components(&self) -> Result<&Components<RuntimeComponent>> {
        Ok(&self
            .actor
            .as_ref()
            .ok_or(Error::Missing("Actor component table"))?
            .components)
    }
    pub fn components_mut(&mut self) -> Result<&mut Components<RuntimeComponent>> {
        Ok(&mut self
            .actor
            .as_mut()
            .ok_or(Error::Missing("Actor component table"))?
            .components)
    }
}
#[derive(Debug)]
pub struct LoadedSource {
    pub doodad: Option<aa_mapworld::dynamic_doodad::DynamicDoodad>,
    pub special_object: Option<aa_mapworld::map_objects::SpecialObject>,
    pub placement: Placement,
    pub completion: LoadCompletion,
    pub owner_source_slot: Option<usize>,
    pub saved_components: Vec<DecodedSavedComponent>,
    pub actor_tail: Option<ActorTail>,
    pub failure: Option<String>,
}
#[cfg(test)]
#[path = "actor_destruction_ownership_tests.rs"]
mod destruction_ownership_tests;
#[derive(Debug)]
pub enum SpecialSource {
    Team(Box<TeamRecord>),
    Kingdom(Box<KingdomRecord>),
}
#[derive(Debug)]
pub struct LoadedSpecial {
    pub source: SpecialSource,
    pub completion: LoadCompletion,
    pub derived: Option<SpecialShell>,
    pub actor_tail: Option<ActorTail>,
    pub derived_payload_loaded: bool,
    pub failure: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewPhase {
    Constructed,
    IdentityRegistered,
    ObjectInitialized,
    ComponentsConstructing,
    AwaitingInitCreate,
    AwaitingPostCreate,
    AwaitingExistingPublication,
}
#[derive(Debug)]
pub enum Source {
    /// Real GObject/DynamicDoodad constructor storage. The component owner is
    /// the arena allocation, available before4EE460 assigns a native GTID.
    NewObject {
        record: Box<creation::CreationRecord<DefinitionProof>>,
        requested_variant: i32,
        doodad: Option<aa_mapworld::dynamic_doodad::DynamicDoodad<ObjectKey>>,
        phase: NewPhase,
        failure: Option<String>,
    },
    New {
        record: Box<ActorCreationRecord<DefinitionProof>>,
        phase: NewPhase,
        failure: Option<String>,
    },
    Loaded(Box<LoadedSource>),
    Special(Box<LoadedSpecial>),
}
#[derive(Debug)]
pub struct Entity {
    pub runtime: Option<RuntimeObject>,
    pub source: Source,
    pub bindings: Option<Arc<ActorBindings>>,
}
/// A non-owning witness for one store allocation's lifetime. The weak control
/// block prevents allocator address reuse from making a later store match.
pub struct ActorStoreWitness(Weak<()>);
impl ActorStoreWitness {
    pub fn is_for(&self, store: &ActorStore) -> bool {
        self.0.ptr_eq(&Arc::downgrade(&store.owner_identity))
    }
}
pub struct ActorStore {
    owner_identity: Arc<()>,
    identities: IdentityRegistry<ObjectKey>,
    entities: BTreeMap<ObjectKey, Box<Entity>>,
    next_key: u64,
    pub minds: aa_mind::Arena,
    /// Restored Element +D4 path nodes awaiting the typed Move owner
    /// (movement_world::Runtime::bind_element_restored), keyed by Mind.
    pub restored_element_paths: BTreeMap<aa_mind::MindId, Vec<aa_navigation::PathNode>>,
    /// Canonical loaded Regiment containers and references. The restore stage
    /// moves these owners once; detached Installed slots are never recreated.
    pub regiment_owned: BTreeMap<aa_mind::MindId, aa_mind::regiment_load::Owned>,
    /// Saved command owners await a one-time move into mind_world::Runtime.
    /// None for new stores; even an empty restored registry stages Some(empty).
    pub(crate) restored_commands: Option<crate::mind_world::Commands>,
    pub animations: AnimationBank,
    pub abilities: BTreeMap<u32, aa_combat::ability::Ability>,
    pub property_allocations: crate::load_property_instances::State,
    next_mind: u32,
    next_state: u32,
    next_ability: u32,
}
impl Default for ActorStore {
    fn default() -> Self {
        Self::new()
    }
}
impl ActorStore {
    pub fn new() -> Self {
        Self {
            owner_identity: Arc::new(()),
            identities: IdentityRegistry::new(),
            entities: BTreeMap::new(),
            next_key: 1,
            minds: aa_mind::Arena::default(),
            restored_element_paths: BTreeMap::new(),
            regiment_owned: BTreeMap::new(),
            restored_commands: None,
            animations: AnimationBank::default(),
            abilities: BTreeMap::new(),
            property_allocations: crate::load_property_instances::State::from_world_constructor(),
            next_mind: 1,
            next_state: 1,
            next_ability: 1,
        }
    }
    pub fn identity_witness(&self) -> ActorStoreWitness {
        ActorStoreWitness(Arc::downgrade(&self.owner_identity))
    }
    /// Validates a portable object token without interpreting it as a GTID.
    pub fn checked_key(&self, raw: u64) -> Option<ObjectKey> {
        let key = ObjectKey(raw);
        self.entities.contains_key(&key).then_some(key)
    }
    pub fn identities(&self) -> &IdentityRegistry<ObjectKey> {
        &self.identities
    }
    pub fn key(&self, id: ActorId) -> Option<ObjectKey> {
        self.identities.get(id).map(|e| e.value)
    }
    pub fn id(&self, key: ObjectKey) -> Option<ActorId> {
        self.identities
            .iter()
            .find(|e| e.value == key)
            .map(|e| e.id)
    }
    pub fn entity(&self, key: ObjectKey) -> Result<&Entity> {
        self.entities
            .get(&key)
            .map(Box::as_ref)
            .ok_or(Error::Missing("object arena identity"))
    }
    pub fn entity_mut(&mut self, key: ObjectKey) -> Result<&mut Entity> {
        self.entities
            .get_mut(&key)
            .map(Box::as_mut)
            .ok_or(Error::Missing("object arena identity"))
    }
    pub fn object(&self, key: ObjectKey) -> Result<&RuntimeObject> {
        self.entity(key)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("restored runtime constructor"))
    }
    pub fn object_mut(&mut self, key: ObjectKey) -> Result<&mut RuntimeObject> {
        self.entity_mut(key)?
            .runtime
            .as_mut()
            .ok_or(Error::Missing("restored runtime constructor"))
    }
    ///Borrow two distinct current arena allocations without removing either
    ///entry. The iterator proves disjointness; callbacks cannot replace the map.
    pub(crate) fn with_two_entities_mut<T>(
        &mut self, first:ObjectKey, second:ObjectKey,
        use_entities:impl FnOnce(&mut Entity,&mut Entity)->Result<T>,
    )->Result<T> {
        if first==second {return Err(Error::Missing("distinct arena allocations required"));}
        let (mut a,mut b)=(None,None);
        for (key,value) in self.entities.iter_mut() {
            if *key==first {a=Some(value.as_mut());}
            else if *key==second {b=Some(value.as_mut());}
        }
        use_entities(a.ok_or(Error::Missing("first arena allocation"))?,
            b.ok_or(Error::Missing("second arena allocation"))?)
    }
    pub fn component(&self, id: ActorId, slot: ComponentSlot) -> Result<Option<&RuntimeComponent>> {
        let key = self.key(id).ok_or(Error::Missing("live actor GTID"))?;
        Ok(self.object(key)?.components()?.get(slot))
    }
    pub fn component_mut(
        &mut self,
        id: ActorId,
        slot: ComponentSlot,
    ) -> Result<Option<&mut RuntimeComponent>> {
        let key = self.key(id).ok_or(Error::Missing("live actor GTID"))?;
        Ok(self.object_mut(key)?.components_mut()?.get_mut(slot))
    }
    pub fn placement(&self, key: ObjectKey, spatial: &impl Spatial) -> Result<PlacementState> {
        match self.object(key)?.placement {
            PlacementOwner::Staged(p) => Ok(p),
            PlacementOwner::Spatial => {
                spatial.placement(self.id(key).ok_or(Error::Missing("published GTID"))?)
            }
        }
    }
    /// Thing `+8` flags and transform before spatial publication.
    pub fn staged_placement(&self, key: ObjectKey) -> Result<PlacementState> {
        match self.object(key)?.placement {
            PlacementOwner::Staged(p) => Ok(p),
            PlacementOwner::Spatial => Err(Error::Missing(
                "published placement must be read through the spatial runtime",
            )),
        }
    }
    pub fn staged_placement_mut(&mut self, key: ObjectKey) -> Result<&mut PlacementState> {
        match &mut self.object_mut(key)?.placement {
            PlacementOwner::Staged(p) => Ok(p),
            PlacementOwner::Spatial => Err(Error::Missing(
                "published placement must mutate through spatial runtime",
            )),
        }
    }
    /// Detach the same spatial allocation before native GTID retirement. Both
    /// owners are preflighted before index mutation; the Entity stays in its slot.
    pub(crate) fn detach_spatial_for_destruction(
        &mut self, key:ObjectKey, spatial:&mut crate::spatial_world::SpatialWorld, pc:Precision,
    )->Result<()> {
        let id=self.id(key).ok_or(Error::Missing("destruction live GTID"))?;
        if self.key(id)!=Some(key) || self.object(key)?.retired_identity.is_some()
            || !matches!(self.object(key)?.placement,PlacementOwner::Spatial)
        {
            return Err(Error::Missing("destruction placement/identity owner differs"));
        }
        let object=self.object_mut(key)?;
        let fields=spatial.detach_for_destruction(key,id,pc)
            .map_err(|e|Error::Source(e.to_string()))?;
        object.placement=PlacementOwner::Staged(fields);
        Ok(())
    }
    /// Native4EFEC0 changes the registry immediately; native Thing+14 remains
    /// readable until E0. This label cannot resolve an object or rejoin iteration.
    pub(crate) fn retire_identity(&mut self,key:ObjectKey)->Result<ActorId> {
        let id=self.id(key).ok_or(Error::Missing("destruction live GTID"))?;
        let object=self.object(key)?;
        if self.key(id)!=Some(key) || object.retired_identity.is_some()
            || !matches!(object.placement,PlacementOwner::Staged(_))
        {
            return Err(Error::Missing("destruction spatial/GTID retirement order differs"));
        }
        self.identities.remove(id)?;
        // The preflighted allocation cannot disappear: no callback ran.
        self.entities.get_mut(&key).unwrap().runtime.as_mut().unwrap().retired_identity=Some(id);
        Ok(id)
    }
    pub(crate) fn native_identity_word(&self,key:ObjectKey)->Result<ActorId> {
        let retired=self.object(key)?.retired_identity;
        match (self.id(key),retired) {
            (Some(id),None)|(None,Some(id))=>Ok(id),
            _=>Err(Error::Missing("native Thing identity owner differs")),
        }
    }
    ///The capability is minted only after the supported actual E0 destructor
    ///finishes. Until then the retired allocation remains accessible by token.
    pub(crate) fn reclaim_retired(
        &mut self,completed:crate::map_replacement_drain::CompletedDestructor,
    )->Result<()> {
        let (key,id)=completed.into_parts(self).map_err(Error::Source)?;
        let object=self.object(key)?;
        if object.retired_identity!=Some(id) || self.id(key).is_some() || self.key(id).is_some()
            || !matches!(object.placement,PlacementOwner::Staged(_))
        {
            return Err(Error::Missing("completed destructor identity/placement differs"));
        }
        self.entities.remove(&key).expect("preflighted completed allocation");
        Ok(())
    }
    /// Spatial::publish must reject before ownership acceptance on failure. After
    /// success the arena relinquishes its transform and flags; future access routes
    /// solely to Spatial. World-specific insertion/membership follows its own proof.
    pub fn publish_placement(&mut self, key: ObjectKey, spatial: &mut impl Spatial) -> Result<()> {
        let id = self
            .id(key)
            .ok_or(Error::Missing("GTID before spatial publication"))?;
        let value = match self.object(key)?.placement {
            PlacementOwner::Staged(value) => value,
            PlacementOwner::Spatial => return Err(Error::Missing("placement already published")),
        };
        spatial.publish(id, value)?;
        self.object_mut(key)?.placement = PlacementOwner::Spatial;
        Ok(())
    }
    fn allocate_key(&mut self, entity: Entity) -> Result<ObjectKey> {
        let key = ObjectKey(self.next_key);
        self.next_key = self.next_key.checked_add(1).ok_or(Error::Exhausted)?;
        self.entities.insert(key, Box::new(entity));
        Ok(key)
    }
    /// Consume saved registry metadata unchanged, including loaded-generation tail,
    /// empty occupied slots, highest/cursor and actor-kind bits. Traversal is native
    /// ascending occupied-slot order, never inferred from future arena token values.
    /// Restored Minds move (not copy) into the single live Mind arena; new
    /// identities continue after the highest restored Mind/state identity.
    /// A record still being loaded, a Mind not built by the runtime restore
    /// adapter, repeated Mind/state/command identities, or mismatched saved
    /// command owners and values are explicit errors before anything moves.
    pub fn from_restored(registry: IdentityRegistry<RestoredRecord>) -> Result<Self> {
        validate_restored(&registry)?;
        let mut store = Self::new();
        let mut commands = crate::mind_world::Commands::default();
        store.identities = registry.map_values(|_id, _is_actor, record| {
            let entity = Entity::from_restored(
                record,
                &mut store.minds,
                &mut store.restored_element_paths,
                &mut store.regiment_owned,
                &mut commands,
            );
            let key = ObjectKey(store.next_key);
            store.next_key += 1;
            store.entities.insert(key, Box::new(entity));
            key
        });
        store.restored_commands = Some(commands);
        if let Some(last) = store.minds.minds.keys().next_back() {
            store.next_mind = store.next_mind.max(last.0.saturating_add(1));
        }
        if let Some(last) = store.minds.states.keys().next_back() {
            store.next_state = store.next_state.max(last.0.saturating_add(1));
        }
        Ok(store)
    }
    /// Moves the same saved action instances and allocator state into the live
    /// world. Existing Performer handle lists are retained without reconstruction.
    pub fn from_restored_with_abilities(
        registry: IdentityRegistry<RestoredRecord>,
        arena: crate::saved_performer::AbilityArena,
    ) -> Result<Self> {
        let mut store = Self::from_restored(registry)?;
        (store.abilities, store.next_ability) = arena.into_parts();
        Ok(store)
    }
    /// Runs the actual new GThing/GObject/GActor constructor chain using source
    /// flags, aspect weights and shared RNG. No GTID/components/InitCreate are
    /// silently performed at this earlier native construction stage.
    pub fn construct_new(
        &mut self,
        catalog: &Catalog<'_>,
        record: ActorCreationRecord<String>,
        world_begin: u32,
        rng: &mut NativeRng,
        startup_precision: Precision,
        pc: Precision,
    ) -> Result<ObjectKey> {
        let binding = catalog
            .actor_bindings(&record.base.definition.key, pc)
            .map_err(Error::Source)?;
        self.construct_new_bound(catalog, binding, record, world_begin, rng, startup_precision, pc)
    }
    /// Same constructor with immutable definition completion and live arithmetic
    /// precision separated. The Arc must be the actual Catalog cache allocation;
    /// a caller-authored field-equivalent ActorBindings cannot authorize creation.
    #[allow(clippy::too_many_arguments)]
    pub fn construct_new_bound(
        &mut self,
        catalog: &Catalog<'_>,
        binding: Arc<ActorBindings>,
        mut record: ActorCreationRecord<String>,
        world_begin: u32,
        rng: &mut NativeRng,
        startup_precision: Precision,
        pc: Precision,
    ) -> Result<ObjectKey> {
        catalog.require_source_owner().map_err(Error::Source)?;
        let actual = catalog.actor_bindings(&record.base.definition.key, binding.precision)
            .map_err(Error::Source)?;
        if !Arc::ptr_eq(&binding, &actual) {
            return Err(Error::Missing("creation binding must be the actual source Catalog allocation"));
        }
        let source = &binding.definition;
        if source.native_class != 3 {
            return Err(Error::Missing("ordinary Actor class3 construction"));
        }
        if source.flags & (1 << 23) != 0 && record.owner_word != 0 {
            let owner = self
                .key(ActorId::from_raw(record.owner_word))
                .ok_or(Error::Missing(
                    "creation owner must be registered Kingdom GTID",
                ))?;
            if self.object(owner)?.definition.native_class != 4 {
                return Err(Error::Missing("creation owner class must be Kingdom"));
            }
        }
        // Source definition authority replaces caller-supplied flag guesses.
        record.base.definition.flags = source.flags;
        let object = source
            .object
            .as_ref()
            .ok_or(Error::Missing("Actor ObjectDefinition"))?;
        let mut native = ActorCreationRecord {
            base: creation::CreationRecord {
                definition: creation::ThingDefinition {
                    key: source.proof.clone(),
                    flags: source.flags,
                },
                position_bits: record.base.position_bits,
                direction_bits: record.base.direction_bits,
                name_utf16: record.base.name_utf16,
                saved_id: record.base.saved_id,
            },
            variant: record.variant,
            owner_word: record.owner_word,
            word_30: record.word_30,
            request_nonblocking: record.request_nonblocking,
        };
        let shell = creation::construct_actor_with_components::<_, RuntimeComponent>(
            &mut native,
            object,
            world_begin,
            rng,
            DirectionTable::original(startup_precision),
            pc,
        )?;
        let runtime = RuntimeObject::from_shell(shell, source.clone());
        self.animations.bind(Arc::clone(&binding))?;
        self.allocate_key(Entity {
            runtime: Some(runtime),
            source: Source::New {
                record: Box::new(native),
                phase: NewPhase::Constructed,
                failure: None,
            },
            bindings: Some(binding),
        })
    }
    /// Native4EA5E0/4DCB30 common Object allocation. Source authority comes
    /// from the actual completed Bank; no Actor bindings/table are constructed.
    pub(crate) fn construct_new_object_bound(
        &mut self, catalog:&Catalog<'_>, bank:&crate::definitions::Bank,
        definition:RuntimeDefinition, mut record:creation::CreationRecord<String>,
        requested_variant:i32, world_begin:u32, rng:&mut NativeRng,
        startup:&crate::actor_creation_startup::StartupInputs, pc:Precision,
    )->Result<ObjectKey> {
        crate::live_object_init::Source::bind_in_bank(catalog,bank,&definition)
            .map_err(|e|Error::Source(e.to_string()))?;
        if !matches!(definition.native_class,2|9) || definition.component_mask.is_some()
            || !record.definition.key.eq_ignore_ascii_case(&definition.proof.id) {
            return Err(Error::Missing("fresh Object source/class differs"));
        }
        let object_definition=definition.object.as_ref().ok_or(Error::Missing("fresh ObjectDefinition"))?;
        record.definition.flags=definition.flags;
        let mut native=creation::CreationRecord {
            definition:creation::ThingDefinition { key:definition.proof.clone(), flags:definition.flags },
            position_bits:record.position_bits,direction_bits:record.direction_bits,
            name_utf16:record.name_utf16,saved_id:record.saved_id,
        };
        if definition.flags&(1<<17)==0 && definition.flags&(1<<15)!=0 {
            crate::configure_world::log_random_seed(catalog,rng).map_err(Error::Source)?;
        }
        let base=creation::construct(&mut native,world_begin,rng,startup.direction_table(),pc)
            .map_err(|e|Error::Creation(e.into()))?;
        let mut shell=aa_actors::loading::construct_shell_with_components::<_,RuntimeComponent>(LoadKind::Object,base.definition);
        shell.thing.flags=base.flags;shell.thing.word_c=base.word_c;shell.thing.word_10=base.word_10;
        shell.thing.name_utf16=base.name_utf16;shell.thing.word_1c=Some(base.created_at_bits);
        shell.thing.position_bits=base.position_bits;shell.thing.facing_bits=base.facing_bits;
        if object_definition.variations_enabled {
            shell.object.as_mut().ok_or(Error::Missing("fresh Object shell"))?.variant=if requested_variant == -1 {
                crate::configure_world::log_random_seed(catalog,rng).map_err(Error::Source)?;
                creation::select_variant(object_definition,rng,pc)?
            } else {requested_variant};
        }
        let doodad=(definition.native_class==9).then(aa_mapworld::dynamic_doodad::DynamicDoodad::default);
        self.allocate_key(Entity {runtime:Some(RuntimeObject::from_shell(shell,definition)),
            source:Source::NewObject {record:Box::new(native),requested_variant,doodad,
                phase:NewPhase::Constructed,failure:None},bindings:None})
    }
    /// Native identity-before-random-scale order of4E9FC0. A failed scale leaves
    /// the allocated GTID visible and the original component table intact.
    pub fn initialize_object(
        &mut self,
        key: ObjectKey,
        rng: &mut NativeRng,
        pc: Precision,
    ) -> Result<ActorId> {
        if self.id(key).is_some() {
            return Err(Error::AlreadyRegistered);
        }
        let (saved, definition, is_actor) = match &self.entity(key)?.source {
            Source::New { record, .. } => (
                record.base.saved_id,
                self.object(key)?
                    .definition
                    .object
                    .clone()
                    .ok_or(Error::Missing("ObjectDefinition"))?,
                true,
            ),
            Source::NewObject { record, .. } => (record.saved_id,
                self.object(key)?.definition.object.clone().ok_or(Error::Missing("ObjectDefinition"))?, false),
            _ => return Err(Error::Missing("fresh creation record")),
        };
        let id = creation::initialize_identity(&mut self.identities, saved, is_actor, key)?;
        self.set_phase(key, NewPhase::IdentityRegistered)?;
        match creation::random_scale(&definition, rng, pc) {
            Ok(scale) => {
                self.object_mut(key)?
                    .object
                    .as_mut()
                    .ok_or(Error::Missing("Object shell"))?
                    .scale_bits = scale;
                self.set_phase(key, NewPhase::ObjectInitialized)?;
                Ok(id)
            }
            Err(error) => {
                self.mark_failure(key, error.to_string());
                Err(error.into())
            }
        }
    }
    fn set_phase(&mut self, key: ObjectKey, next: NewPhase) -> Result<()> {
        match &mut self.entity_mut(key)?.source {
            Source::New { phase, .. } | Source::NewObject { phase, .. } => {
                *phase = next;
                Ok(())
            }
            _ => Err(Error::Missing("new actor lifecycle stage")),
        }
    }
    fn mark_failure(&mut self, key: ObjectKey, message: String) {
        if let Ok(entity) = self.entity_mut(key) {
            match &mut entity.source {
                Source::New { failure, .. } | Source::NewObject { failure, .. } => *failure = Some(message),
                _ => {},
            }
        }
    }
    /// Concrete common constructors. The Character set uses actual shell variant;
    /// initial -1 selects the proven base set. A state-view override must be bound
    /// separately and is rejected here until its source aspect mapping exists.
    pub fn construct_ground(
        &mut self,
        key: ObjectKey,
        slot: ComponentSlot,
        world_time: F32,
        pc: Precision,
    ) -> Result<()> {
        let id = self
            .id(key)
            .ok_or(Error::Missing("GTID before component construction"))?;
        if self.object(key)?.components()?.present(slot) {
            return Err(Error::Occupied);
        }
        let binding = Arc::clone(
            self.entity(key)?
                .bindings
                .as_ref()
                .ok_or(Error::Missing("completed source actor binding"))?,
        );
        let mut ground = binding.definition.ground.clone();
        if slot == ComponentSlot::Character {
            let object = self.object(key)?;
            let actor = object.actor.as_ref().ok_or(Error::Missing("Actor shell"))?;
            if actor.words.get(&0x15c).copied() != Some(u32::MAX) {
                return Err(Error::Missing("state-view aspect binding for Character"));
            }
            let variant = object
                .object
                .as_ref()
                .ok_or(Error::Missing("Object variant"))?
                .variant;
            let character = binding
                .character
                .as_ref()
                .ok_or(Error::Missing("source Character definition"))?;
            let set = if variant == -1 {
                u32::try_from(character.aspect_ids.len()).map_err(|_| Error::Exhausted)?
            } else {
                u32::try_from(variant).map_err(|_| Error::Missing("valid Object variant"))?
            };
            ground = binding
                .ground_for_animation_set(set)
                .map_err(Error::Source)?;
        }
        let position = match self.object(key)?.placement {
            PlacementOwner::Staged(p) => p.position_bits,
            PlacementOwner::Spatial => {
                return Err(Error::Missing(
                    "ground construction after spatial publication",
                ));
            }
        };
        let value = owned::construct(
            slot,
            &ground,
            owned::Context {
                owner: id,
                world_time,
                position: position.map(F32),
                precision: pc,
            },
        )?;
        self.object_mut(key)?.components_mut()?.insert(slot, value);
        self.set_phase(key, NewPhase::ComponentsConstructing)?;
        Ok(())
    }
}
/// Constructor inputs must come from the completed source Ability/Performer
/// loaders and exact native getters. Logical definition handles belong to Bank.
pub struct PerformerConstruction {
    pub abilities: Vec<aa_combat::constructors::LocalAbility>,
    pub energy: F32,
    pub turret_center_angles: Vec<aa_numeric::Binary>,
    pub startup_target: Option<[F32; 2]>,
}
impl ActorStore {
    ///Native567CD0: allocate each actual ActorData+21C local Ability in order,
    ///then publish the same Defender component. Source completion uses the
    ///retained binding PC; Ability construction copies those finalized values.
    pub fn construct_defender(
        &mut self,
        key:ObjectKey,
        catalog:&Catalog<'_>,
        bank:&mut crate::definitions::Bank,
    )->Result<()> {
        let owner=self.id(key).ok_or(Error::Missing("GTID before Defender construction"))?;
        if self.object(key)?.components()?.present(ComponentSlot::Defender){return Err(Error::Occupied);}
        let binding=Arc::clone(self.entity(key)?.bindings.as_ref().ok_or(Error::Missing("source Defender binding"))?);
        let actual=catalog.actor_bindings(&binding.definition.proof.id,binding.precision).map_err(Error::Source)?;
        if !Arc::ptr_eq(&binding,&actual){return Err(Error::Missing("Defender source binding belongs to another Catalog"));}
        let id=&binding.definition.proof.id;
        let loaded=catalog.actors.native_loaded_defender(id,binding.precision,
            binding.character.as_ref().map(|v|v as &dyn aa_rules::abilities::AnimationResolver))
            .map_err(|e|Error::Source(e.to_string()))?.ok_or(Error::Missing("DefenderComponent source"))?;
        let mut handles=Vec::new();
        for (index,ability) in loaded.abilities().iter().enumerate(){
            let local_name=format!("{id}/defender/{index}");
            let local=bank.intern("ActorLocalAbility",&local_name).map_err(Error::Source)?;
            let definition=match &ability.authored.shared_definition_id {
                Some(aa_rules::native::TypedValue::String{value,..})=>bank.intern("AbilityDefinition",value),
                Some(_)=>return Err(Error::Missing("Defender shared Ability IDS string")),
                None=>bank.intern("ActorLocalAbilityDefinition",&local_name),
            }.map_err(Error::Source)?;
            let handle=self.allocate_ability(aa_combat::ability::Ability::new(definition,Some(local),F32(ability.power_bits)))?;
            handles.try_reserve(1).map_err(|e|Error::Source(e.to_string()))?;
            handles.push(handle);
        }
        self.object_mut(key)?.components_mut()?.insert(ComponentSlot::Defender,
            RuntimeComponent::Defender(Box::new(aa_actors::defender_state::DefenderState::new(owner,handles))));
        self.set_phase(key,NewPhase::ComponentsConstructing)
    }
    /// Original46545C..465468 binds mana_max to ActorData+1E0; the turret
    /// list comes from the native Turret loader (464BD0/4C7820).
    pub fn source_performer(
        &self,
        key: ObjectKey,
        catalog: &Catalog<'_>,
        bank: &mut crate::definitions::Bank,
    ) -> Result<PerformerConstruction> {
        let binding = self
            .entity(key)?
            .bindings
            .as_ref()
            .ok_or(Error::Missing("source Actor binding"))?;
        performer_inputs(binding, catalog, bank)
    }
    pub fn allocate_ability(&mut self, ability: aa_combat::ability::Ability) -> Result<u32> {
        crate::saved_performer::AbilityArena::allocate_into(
            &mut self.abilities,
            &mut self.next_ability,
            ability,
        )
        .map_err(Error::Source)
    }
    pub fn construct_performer(
        &mut self,
        key: ObjectKey,
        input: &PerformerConstruction,
        pc: Precision,
        notify: &mut impl FnMut(&mut ActorStore, ActorId) -> Result<()>,
    ) -> Result<()> {
        let id = self
            .id(key)
            .ok_or(Error::Missing("GTID before Performer construction"))?;
        if self
            .object(key)?
            .components()?
            .present(ComponentSlot::Performer)
        {
            return Err(Error::Occupied);
        }
        let mut factory = PerformerFactory {
            store: self,
            id,
            input,
            notify,
        };
        let value = aa_combat::constructors::create(id.raw(), &mut factory, pc)
            .map_err(|e| Error::Source(e.to_string()))?;
        self.object_mut(key)?
            .components_mut()?
            .insert(ComponentSlot::Performer, RuntimeComponent::Performer(value));
        self.set_phase(key, NewPhase::ComponentsConstructing)?;
        Ok(())
    }
    /// The original factory chooses Organization before Aircraft before Element.
    /// Actual Element construction consumes shared world RNG and updates every
    /// already-created Performer ability's fuzzy-range field, in native order.
    pub fn construct_mind(
        &mut self,
        key: ObjectKey,
        rng: &mut NativeRng,
        pc: Precision,
    ) -> Result<()> {
        let id = self
            .id(key)
            .ok_or(Error::Missing("GTID before Mind construction"))?;
        let object = self.object(key)?;
        if object.components()?.present(ComponentSlot::Mind) {
            return Err(Error::Occupied);
        }
        let mask = object
            .definition
            .component_mask
            .ok_or(Error::Missing("completed native component mask"))?;
        let kind = match aa_actors::init_create::factory_kind(ComponentSlot::Mind, mask) {
            Some(aa_actors::init_create::FactoryKind::OrganizationMind) => {
                aa_mind::construct::Kind::Regiment
            }
            Some(aa_actors::init_create::FactoryKind::AircraftMind) => {
                aa_mind::construct::Kind::Aircraft
            }
            Some(aa_actors::init_create::FactoryKind::ElementMind) => {
                aa_mind::construct::Kind::Element
            }
            None => return Ok(()),
            _ => return Err(Error::Missing("derived Mind factory kind")),
        };
        let mut world = MindFactory { store: self, rng };
        let made = aa_mind::construct::construct(kind, id.raw(), &mut world, pc)
            .map_err(|e| Error::Source(e.to_string()))?;
        let mind = aa_mind::MindId(self.next_mind);
        self.next_mind = self.next_mind.checked_add(1).ok_or(Error::Exhausted)?;
        let mut states = BTreeMap::new();
        for state in &made.states {
            let id = aa_mind::StateId(self.next_state);
            self.next_state = self.next_state.checked_add(1).ok_or(Error::Exhausted)?;
            states.insert(state.offset, id);
        }
        let installed = made
            .install(mind, &states, &mut self.minds)
            .map_err(|e| Error::Source(e.to_string()))?;
        self.object_mut(key)?
            .components_mut()?
            .insert(ComponentSlot::Mind, RuntimeComponent::Mind(installed));
        self.set_phase(key, NewPhase::ComponentsConstructing)?;
        Ok(())
    }
}
struct MindFactory<'a> {
    store: &'a mut ActorStore,
    rng: &'a mut NativeRng,
}
impl aa_mind::construct::World for MindFactory<'_> {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        Ok(self.rng.next_u32())
    }
    fn performer_ability_count(&self, actor: u32) -> aa_mind::Result<usize> {
        match self
            .store
            .component(ActorId::from_raw(actor), ComponentSlot::Performer)
            .map_err(|e| aa_mind::Error::World(e.to_string()))?
        {
            Some(RuntimeComponent::Performer(p)) => Ok(p.state.abilities.len()),
            None => Ok(0),
            _ => Err(aa_mind::Error::World(
                "Performer slot payload mismatch".into(),
            )),
        }
    }
    fn set_ability_jitter(&mut self, actor: u32, index: usize, bits: u32) -> aa_mind::Result<()> {
        let ability = match self
            .store
            .component(ActorId::from_raw(actor), ComponentSlot::Performer)
            .map_err(|e| aa_mind::Error::World(e.to_string()))?
        {
            Some(RuntimeComponent::Performer(p)) => *p
                .state
                .abilities
                .get(index)
                .ok_or_else(|| aa_mind::Error::World("Performer action index changed".into()))?,
            _ => return Err(aa_mind::Error::World("required Performer missing".into())),
        };
        self.store
            .abilities
            .get_mut(&ability)
            .ok_or_else(|| aa_mind::Error::World("ability arena handle missing".into()))?
            .fuzzy_range = F32(bits);
        Ok(())
    }
}
/// Original globals B279C0/B279C4 copied into each new turret's target XY by
/// 57F4D0. Initialized .data bytes are zero and no instruction in the image
/// writes either absolute address (research/phase8-restoration/
/// b279c0-references.json); every reference is a read or a const pointer.
pub const NATIVE_ZERO_VECTOR_B279C0: [F32; 2] = [F32(0), F32(0)];
/// Shared new/restored Performer source binding; definition tokens come from
/// the caller's one world Bank, never a local temporary identity table.
pub fn performer_inputs(
    binding: &ActorBindings,
    catalog: &Catalog<'_>,
    bank: &mut crate::definitions::Bank,
) -> Result<PerformerConstruction> {
    let loaded = binding
        .performer
        .as_ref()
        .ok_or(Error::Missing("completed Performer definition"))?;
    let node = catalog
        .actors
        .resolved_actor(&binding.definition.proof.id)
        .map_err(|e| Error::Source(e.to_string()))?;
    node.child("PerformerComponent")
        .map_err(|e| Error::Source(e.to_string()))?
        .ok_or(Error::Missing("Performer source node"))?;
    // ActorData+1F8 turret list (464BD0 -> 4C7660/4C7820); 57F4D0 reads each
    // +4 yaw_start word through 45DC30.
    let turret_center_angles = catalog
        .actors
        .native_turrets(&binding.definition.proof.id, binding.precision)
        .map_err(|e| Error::Source(e.to_string()))?
        .iter()
        .map(|t| aa_numeric::Binary::from_f32_bits(t.yaw_start_bits))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| Error::Source(e.to_string()))?;
    let energy = loaded
        .authored
        .fields
        .get("mana_max")
        .ok_or(Error::Missing("mana_max source field"))?
        .fixed_bits()
        .map_err(|e| Error::Source(e.to_string()))?;
    let mut abilities = Vec::new();
    for (index, ability) in loaded.abilities().iter().enumerate() {
        let local_name = format!("{}/ability/{index}", binding.definition.proof.id);
        let local = bank
            .intern("ActorLocalAbility", &local_name)
            .map_err(Error::Source)?;
        let definition = if let Some(value) = &ability.authored.shared_definition_id {
            let aa_rules::native::TypedValue::String { value, .. } = value else {
                return Err(Error::Missing("shared Ability IDS string"));
            };
            bank.intern("AbilityDefinition", value)
                .map_err(Error::Source)?
        } else {
            bank.intern("ActorLocalAbilityDefinition", &local_name)
                .map_err(Error::Source)?
        };
        abilities.push(aa_combat::constructors::LocalAbility {
            definition,
            local_definition: local,
            main_power: F32(ability.power_bits),
        });
    }
    Ok(PerformerConstruction {
        abilities,
        energy: F32(energy),
        turret_center_angles,
        startup_target: Some(NATIVE_ZERO_VECTOR_B279C0),
    })
}
struct PerformerFactory<'a, F> {
    store: &'a mut ActorStore,
    id: ActorId,
    input: &'a PerformerConstruction,
    notify: &'a mut F,
}
impl<F: FnMut(&mut ActorStore, ActorId) -> Result<()>> aa_combat::constructors::Factory
    for PerformerFactory<'_, F>
{
    fn local_ability_count(&self, _: u32) -> aa_combat::Result<usize> {
        Ok(self.input.abilities.len())
    }
    fn local_ability(
        &self,
        _: u32,
        index: usize,
    ) -> aa_combat::Result<aa_combat::constructors::LocalAbility> {
        self.input
            .abilities
            .get(index)
            .copied()
            .ok_or_else(|| aa_combat::Error::World("resolved local Ability index".into()))
    }
    fn allocate_ability(&mut self, ability: aa_combat::ability::Ability) -> aa_combat::Result<u32> {
        self.store
            .allocate_ability(ability)
            .map_err(|e| aa_combat::Error::World(e.to_string()))
    }
    fn notify_actor_abilities(&mut self, _: u32) -> aa_combat::Result<()> {
        (self.notify)(self.store, self.id).map_err(|e| aa_combat::Error::World(e.to_string()))
    }
    fn energy(&self, _: u32) -> aa_combat::Result<F32> {
        Ok(self.input.energy)
    }
    fn turret_count(&self, _: u32) -> aa_combat::Result<usize> {
        Ok(self.input.turret_center_angles.len())
    }
    fn turret_center_angle(&self, _: u32, index: usize) -> aa_combat::Result<aa_numeric::Binary> {
        self.input
            .turret_center_angles
            .get(index)
            .cloned()
            .ok_or_else(|| aa_combat::Error::World("resolved turret index".into()))
    }
    fn startup_target(&self) -> aa_combat::Result<[F32; 2]> {
        self.input.startup_target.ok_or_else(|| {
            aa_combat::Error::World("original turret startup XY globals unavailable".into())
        })
    }
    fn properties_component(&self, _: u32) -> aa_combat::Result<Option<u32>> {
        Err(aa_combat::Error::World(
            "Performer create never queries properties; use actual InitCreate adapter".into(),
        ))
    }
    fn refresh_properties(&mut self, _: u32, _: bool) -> aa_combat::Result<()> {
        Err(aa_combat::Error::World(
            "Performer InitCreate requires concrete581420 property refresh".into(),
        ))
    }
}

impl Entity {
    fn from_restored(
        record: RestoredRecord,
        minds: &mut aa_mind::Arena,
        paths: &mut BTreeMap<aa_mind::MindId, Vec<aa_navigation::PathNode>>,
        regiments: &mut BTreeMap<aa_mind::MindId, aa_mind::regiment_load::Owned>,
        commands: &mut crate::mind_world::Commands,
    ) -> Self {
        match record {
            RestoredRecord::Thing(v) | RestoredRecord::Object(v) | RestoredRecord::Actor(v) => {
                let LoadedEntity {
                    source,
                    definition,
                    shell,
                    doodad,
                    special_object,
                    completion,
                    owner_source_slot,
                    saved_components,
                    actor_tail,
                    failure,
                    active_component,
                } = *v;
                let shell = shell.map_components(|_, c| {
                    c.map_specialized(|mind| stage_restored_mind(mind, minds, paths, regiments, commands), |performer| performer)
                });
                let mut runtime = RuntimeObject::from_shell(shell, definition);
                if let Some(slot) = active_component {
                    // Restored +E4 (4D9271): the component in that slot.
                    runtime.active_component = ActiveComponent::Slot(slot);
                }
                Self {
                    runtime: Some(runtime),
                    source: Source::Loaded(Box::new(LoadedSource {
                        placement: source,
                        doodad,
                        special_object,
                        completion,
                        owner_source_slot,
                        saved_components,
                        actor_tail,
                        failure,
                    })),
                    bindings: None,
                }
            }
            RestoredRecord::Team(v) => {
                Self::from_special(v.source.into(), v.completion, v.runtime, v.failure, minds, paths, regiments, commands)
            }
            RestoredRecord::Kingdom(v) => {
                Self::from_special(v.source.into(), v.completion, v.runtime, v.failure, minds, paths, regiments, commands)
            }
            RestoredRecord::Loading => {
                unreachable!("the record loader always replaces its Loading placeholder")
            }
        }
    }
    #[allow(clippy::too_many_arguments)] // Move each independent canonical Mind owner once.
    fn from_special(
        source: SpecialSource,
        completion: LoadCompletion,
        runtime: Option<aa_mapworld::restore::WorldEntityRuntime>,
        failure: Option<String>,
        minds: &mut aa_mind::Arena,
        paths: &mut BTreeMap<aa_mind::MindId, Vec<aa_navigation::PathNode>>,
        regiments: &mut BTreeMap<aa_mind::MindId, aa_mind::regiment_load::Owned>,
        commands: &mut crate::mind_world::Commands,
    ) -> Self {
        let (runtime, derived, actor_tail, derived_payload_loaded) = if let Some(v) = runtime {
            let shell = v.base.map_components(|_, c| {
                c.map_specialized(|mind| stage_restored_mind(mind, minds, paths, regiments, commands), |performer| performer)
            });
            (
                Some(RuntimeObject::from_shell(shell, v.definition)),
                v.derived,
                v.actor_tail,
                v.derived_payload_loaded,
            )
        } else {
            (None, None, None, false)
        };
        Self {
            runtime,
            source: Source::Special(Box::new(LoadedSpecial {
                source,
                completion,
                derived,
                actor_tail,
                derived_payload_loaded,
                failure,
            })),
            bindings: None,
        }
    }
}
/// Staging preconditions, checked before anything moves: no record is still
/// `Loading`, every restored Mind was built by this runtime's restore adapter
/// and Mind/state identities are unique. Every nonzero saved command identity
/// has exactly one pending/current owner and one stored value globally.
fn validate_restored(registry: &IdentityRegistry<RestoredRecord>) -> Result<()> {
    let mut minds = std::collections::BTreeSet::new();
    let mut states = std::collections::BTreeSet::new();
    let mut commands = std::collections::BTreeSet::new();
    for entry in registry.iter() {
        let shell = match &entry.value {
            RestoredRecord::Loading => {
                return Err(Error::Source("a restored record is still loading".into()));
            }
            RestoredRecord::Team(v) => v.runtime.as_ref().map(|r| &r.base),
            RestoredRecord::Kingdom(v) => v.runtime.as_ref().map(|r| &r.base),
            other => other.loaded().map(|v| &v.shell),
        };
        let Some(actor) = shell.and_then(|s| s.actor.as_ref()) else {
            continue;
        };
        for (_, component) in actor.components.iter() {
            let aa_actors::owned::Component::Mind(mind) = component else {
                continue;
            };
            let parts = mind
                .state
                .as_any()
                .downcast_ref::<crate::catalog::RestoredMindParts>()
                .ok_or_else(|| {
                    Error::Source("restored Mind was not built by the runtime restore adapter".into())
                })?;
            if !minds.insert(parts.installed.mind_id) {
                return Err(Error::Source("restored Mind identity repeats".into()));
            }
            if parts.regiment_owned.is_some() {
                if parts.installed.kind != aa_mind::construct::Kind::Regiment
                    || parts.element_path.is_some()
                    || [0x9c, 0xe8, 0x264].iter().any(|off| parts.installed.references.contains_key(off))
                    || [0xa0, 0xa4, 0xa8, 0xac, 0xb4, 0xb8, 0x148, 0x1dc, 0x1e0, 0x268, 0x26c, 0x2bc, 0x2c0]
                        .iter().any(|off| parts.installed.words.contains_key(off))
                    || parts.installed.bytes.contains_key(&0xb0)
                {
                    return Err(Error::Source("restored Regiment owned fields have duplicate scalar owners".into()));
                }
            } else if parts.installed.kind == aa_mind::construct::Kind::Regiment
                && [0x9c, 0xe8, 0x264].iter().any(|off| !parts.installed.references.contains_key(off))
            {
                return Err(Error::Source("restored Regiment detached containers have no owner".into()));
            }
            for id in parts.states.keys() {
                if !states.insert(*id) {
                    return Err(Error::Source("restored Mind state identity repeats".into()));
                }
            }
            let core = parts.mind.as_ref()
                .ok_or_else(|| Error::Source("restored Mind core is absent".into()))?;
            let mut referenced = std::collections::BTreeSet::new();
            for id in core.pending_commands.iter().copied().chain(core.current_command) {
                if id == 0 || !referenced.insert(id) {
                    return Err(Error::Source("restored Mind command reference is zero or repeats".into()));
                }
            }
            let mut owned = std::collections::BTreeSet::new();
            for (id, _) in &parts.commands {
                if *id == 0 || !commands.insert(*id) {
                    return Err(Error::Source("restored command identity is zero or repeats".into()));
                }
                owned.insert(*id);
            }
            if referenced != owned {
                return Err(Error::Source("restored Mind command owners and stored values disagree".into()));
            }
        }
    }
    Ok(())
}
/// Moves one restored Mind's core, embedded states and owned commands into
/// their live arenas. `validate_restored` proved the downcast and unique
/// identities, with exactly one pending/current owner for each command.
fn stage_restored_mind(
    mind: aa_mapworld::restore_components::RestoredMind,
    arena: &mut aa_mind::Arena,
    paths: &mut BTreeMap<aa_mind::MindId, Vec<aa_navigation::PathNode>>,
    regiments: &mut BTreeMap<aa_mind::MindId, aa_mind::regiment_load::Owned>,
    commands: &mut crate::mind_world::Commands,
) -> aa_mind::construct::Installed {
    let parts = mind
        .state
        .into_any()
        .downcast::<crate::catalog::RestoredMindParts>()
        .expect("restored Minds are built only by the runtime restore adapter");
    let crate::catalog::RestoredMindParts {
        installed,
        mind,
        states,
        element_path,
        regiment_owned,
        precision: _,
        commands: saved_commands,
    } = *parts;
    for (id, value) in saved_commands {
        commands.insert_restored(id, value)
            .expect("restored command identities and ownership were validated before transfer");
    }
    if let Some(nodes) = element_path {
        paths.insert(installed.mind_id, nodes);
    }
    if let Some(owned) = regiment_owned {
        let previous = regiments.insert(installed.mind_id, owned);
        assert!(previous.is_none(), "restore Regiment owner identities are unique");
    }
    let previous = arena.minds.insert(installed.mind_id, mind.expect("restored core was validated"));
    assert!(previous.is_none(), "restore Mind identities are unique");
    for (id, state) in states {
        let previous = arena.states.insert(id, state);
        assert!(previous.is_none(), "restore state identities are unique");
    }
    installed
}
impl From<TeamRecord> for SpecialSource {
    fn from(v: TeamRecord) -> Self {
        Self::Team(Box::new(v))
    }
}
impl From<KingdomRecord> for SpecialSource {
    fn from(v: KingdomRecord) -> Self {
        Self::Kingdom(Box::new(v))
    }
}

/// The first Performer constructor calls4D1350 before its slot, Properties,
/// Element/Organization and Mind exist. This adapter executes that real caller
/// and rejects any contributor or external publication branch it cannot fulfill.
/// It is deliberately not usable as a general live-world action adapter.
impl ActorStore {
    pub fn rebuild_construction_actions(&mut self, id: ActorId) -> Result<()> {
        aa_actors::ability_flags::rebuild(&mut ConstructionActions::Store(self), id)
            .map_err(|e| Error::Source(e.to_string()))
    }
    /// Source-aware565140 during fresh assembly, before Performer is installed.
    /// Reads Construction/Settings only when the native contributor reaches them.
    pub fn rebuild_source_construction_actions(&mut self, catalog:&Catalog<'_>, id:ActorId)->Result<()> {
        aa_actors::ability_flags::rebuild(&mut ConstructionActions::SourceStore{store:self,catalog},id)
            .map_err(|e|Error::Source(e.to_string()))
    }
}
enum ConstructionActions<'a,'source> {
    Store(&'a mut ActorStore),
    SourceStore { store:&'a mut ActorStore, catalog:&'a Catalog<'source> },
    Saved {
        owner: ActorId,
        components: &'a aa_actors::components::Components<aa_mapworld::restore::RestoredComponent>,
        flags: &'a mut u32,
        words: &'a mut BTreeMap<u32, u32>,
        context: SavedActionContext,
    },
}
/// Source and shell inputs of the saved factory's 4D1350 contributors and
/// 4CFCC0 publication. Built from the actual definition and load shell.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SavedActionContext {
    /// ActorData Construction inputs (565140), when the definition has one.
    pub construction: Option<aa_actors::construction_actions::Definition>,
    /// SVar AllowSellUnderSiege (two-file Bool override order).
    pub allow_sell_under_siege: Option<bool>,
    /// Object+4C and Thing+C as left by the base constructor and Load.
    pub object_byte_4c: u8,
    pub thing_word_c: u32,
}
/// Native4D1350 during saved factory construction, before later components exist.
/// The Construction contributor 565140 and 4CFCC0 publication run against the
/// owner's actual saved state; components constructed later (Structure, Wall,
/// Settlement, Properties, Element) are absent, and any other contributor or
/// publication branch that needs live services still fails explicitly.
pub(crate) fn rebuild_saved_construction_actions(
    owner: ActorId,
    components: &aa_actors::components::Components<aa_mapworld::restore::RestoredComponent>,
    flags: &mut u32,
    words: &mut BTreeMap<u32, u32>,
    context: SavedActionContext,
) -> Result<()> {
    aa_actors::ability_flags::rebuild(
        &mut ConstructionActions::Saved {
            owner,
            components,
            flags,
            words,
            context,
        },
        owner,
    )
    .map_err(|e| Error::Source(e.to_string()))
}
impl ConstructionActions<'_, '_> {
    /// Saved factory only: the owner itself, and a component that must not
    /// exist yet at this construction stage.
    fn saved_absent(
        &self,
        a: ActorId,
        slot: ComponentSlot,
        what: &'static str,
    ) -> aa_actors::ability_flags::Result<()> {
        match self {
            Self::Saved {
                owner, components, ..
            } => {
                if a != *owner {
                    return action_missing();
                }
                if components.present(slot) {
                    return Err(aa_actors::ability_flags::Error::Missing(what));
                }
                Ok(())
            }
            Self::SourceStore{store,..} => {
                if store.component(a,slot).map_err(action_world_error)?.is_some() {
                    return Err(aa_actors::ability_flags::Error::Missing(what));
                }
                Ok(())
            }
            Self::Store(_) => action_missing(),
        }
    }
}
fn action_world_error(e:impl std::fmt::Display)->aa_actors::ability_flags::Error {
    aa_actors::ability_flags::Error::World(e.to_string())
}
impl aa_actors::construction_actions::World for ConstructionActions<'_, '_> {
    fn fields(
        &self,
        a: ActorId,
    ) -> aa_actors::ability_flags::Result<aa_actors::ability_flags::ActorFields> {
        aa_actors::ability_flags::World::actor_fields(self, a)
    }
    fn set_action_bits(&mut self, a: ActorId, bits: u64) -> aa_actors::ability_flags::Result<()> {
        aa_actors::ability_flags::World::set_action_bits(self, a, bits)
    }
    fn definition(
        &self,
        a: ActorId,
    ) -> aa_actors::ability_flags::Result<aa_actors::construction_actions::Definition> {
        match self {
            Self::Saved { owner, context, .. } if a == *owner => context.construction.ok_or(
                aa_actors::ability_flags::Error::Missing(
                    "Construction contributor without ActorData Construction inputs",
                ),
            ),
            Self::SourceStore{store,catalog} => {
                let key=store.key(a).ok_or(aa_actors::ability_flags::Error::Missing("Construction owner GTID"))?;
                let definition=&store.object(key).map_err(action_world_error)?.definition;
                let binding=store.entity(key).map_err(action_world_error)?.bindings.as_ref()
                    .ok_or(aa_actors::ability_flags::Error::Missing("fresh completed source binding"))?;
                catalog.require_source_owner().map_err(action_world_error)?;
                let expected=catalog.actor_bindings(&definition.proof.id,binding.precision).map_err(action_world_error)?;
                if !Arc::ptr_eq(binding,&expected){return Err(aa_actors::ability_flags::Error::Missing("Construction actual source Catalog allocation"));}
                catalog.actors.native_construction_flags(&definition.proof.id,binding.precision)
                    .map_err(action_world_error)?.map(|f|aa_actors::construction_actions::Definition {
                        can_sell:f.can_sell,can_raze:f.can_raze,upgrade_count:f.upgrade_count,
                    }).ok_or(aa_actors::ability_flags::Error::Missing("actual Construction source"))
            }
            _ => action_missing(),
        }
    }
    fn structure_parent(
        &self,
        a: ActorId,
    ) -> aa_actors::ability_flags::Result<Option<Option<ActorId>>> {
        if let Self::SourceStore{store,..}=self {
            return match store.component(a,ComponentSlot::Structure).map_err(action_world_error)? {
                None=>Ok(None),Some(RuntimeComponent::Structure(v)) if v.header.owner==a => {
                    if v.object_handle_10==0 {Ok(Some(None))} else {
                        Err(aa_actors::ability_flags::Error::Missing("fresh Structure positive parent identity"))
                    }
                },Some(_)=>Err(aa_actors::ability_flags::Error::Missing("fresh Structure payload/owner")),
            };
        }
        self.saved_absent(a, ComponentSlot::Structure, "saved Structure parent word")?;
        Ok(None)
    }
    fn wall_reference(
        &self,
        a: ActorId,
    ) -> aa_actors::ability_flags::Result<Option<Option<ActorId>>> {
        self.saved_absent(a, ComponentSlot::Wall, "saved Wall reference word")?;
        Ok(None)
    }
    fn settlement_reference(
        &self,
        a: ActorId,
    ) -> aa_actors::ability_flags::Result<Option<Option<ActorId>>> {
        self.saved_absent(a, ComponentSlot::Settlement, "saved Settlement reference word")?;
        Ok(None)
    }
    fn properties_flags(&self, a: ActorId) -> aa_actors::ability_flags::Result<Option<u32>> {
        if let Self::SourceStore{store,..}=self {
            return match store.component(a,ComponentSlot::Properties).map_err(action_world_error)? {
                None=>Ok(None),Some(RuntimeComponent::Properties(v)) if v.owner.0==a.raw() =>Ok(Some(v.modifier.flags)),
                Some(_)=>Err(aa_actors::ability_flags::Error::Missing("fresh Properties payload/owner")),
            };
        }
        self.saved_absent(a, ComponentSlot::Properties, "saved Properties flags word")?;
        Ok(None)
    }
    fn allow_sell_under_siege(&mut self) -> aa_actors::ability_flags::Result<bool> {
        match self {
            Self::Saved { context, .. } => context.allow_sell_under_siege.ok_or(
                aa_actors::ability_flags::Error::Missing("SVar AllowSellUnderSiege"),
            ),
            Self::SourceStore{catalog,..}=>catalog.settings.as_ref()
                .ok_or(aa_actors::ability_flags::Error::Missing("actual Settings owner"))?
                .allow_sell_under_siege().map(|v|v.0).map_err(action_world_error),
            Self::Store(_) => action_missing(),
        }
    }
    fn settlement_upgrades_ready(&mut self, _: ActorId) -> aa_actors::ability_flags::Result<bool> {
        Err(aa_actors::ability_flags::Error::Missing(
            "585F00 settlement upgrade readiness",
        ))
    }
}
impl aa_actors::action_publication::World for ConstructionActions<'_, '_> {
    fn element_organization(
        &self,
        a: ActorId,
    ) -> aa_actors::ability_flags::Result<Option<ActorId>> {
        if let Self::SourceStore{store,..}=self {
            return match store.component(a,ComponentSlot::Element).map_err(action_world_error)? {
                None=>Ok(None),Some(RuntimeComponent::Element(v)) if v.header.owner==a=>Ok(v.organization),
                Some(_)=>Err(aa_actors::ability_flags::Error::Missing("fresh Element payload/owner")),
            };
        }
        self.saved_absent(a, ComponentSlot::Element, "saved Element organization word")?;
        Ok(None)
    }
    fn organization_owner(&self, _: ActorId) -> aa_actors::ability_flags::Result<ActorId> {
        action_missing()
    }
    fn rebuild_actions(&mut self, _: ActorId) -> aa_actors::ability_flags::Result<()> {
        action_missing()
    }
    fn owner(&mut self, a: ActorId) -> aa_actors::ability_flags::Result<Option<ActorId>> {
        match self {
            // GActor virtual +108 = 4D1E80 returns actor+E8, written before
            // component construction from the registry-checked saved owner.
            Self::Saved { owner, words, .. } if a == *owner => Ok(words
                .get(&0xe8)
                .copied()
                .filter(|raw| *raw != 0)
                .map(ActorId::from_raw)),
            Self::SourceStore{store,..}=>store.ordinary_owner(a).map_err(action_world_error),
            _ => action_missing(),
        }
    }
    fn object_byte_4c(&self, a: ActorId) -> aa_actors::ability_flags::Result<u8> {
        match self {
            Self::Saved { owner, context, .. } if a == *owner => Ok(context.object_byte_4c),
            Self::SourceStore{store,..}=>{
                let key=store.key(a).ok_or(aa_actors::ability_flags::Error::Missing("fresh Object GTID"))?;
                Ok(store.object(key).map_err(action_world_error)?.object.as_ref()
                    .ok_or(aa_actors::ability_flags::Error::Missing("fresh Object fields"))?.byte_4c)
            }
            _ => action_missing(),
        }
    }
    fn refresh_owner_views(&mut self, _: Option<ActorId>) -> aa_actors::ability_flags::Result<()> {
        Err(aa_actors::ability_flags::Error::Missing(
            "76C7F0 owner view refresh",
        ))
    }
    fn renderer_flags(&self, a: ActorId) -> aa_actors::ability_flags::Result<Option<u32>> {
        match self {
            Self::Saved { owner, context, .. } if a == *owner => {
                if context.thing_word_c != 0 {
                    return Err(aa_actors::ability_flags::Error::Missing(
                        "renderer link flags during saved construction",
                    ));
                }
                Ok(None)
            }
            Self::SourceStore{store,..}=>{
                let key=store.key(a).ok_or(aa_actors::ability_flags::Error::Missing("fresh Thing GTID"))?;
                if store.object(key).map_err(action_world_error)?.thing.word_c!=0 {
                    return Err(aa_actors::ability_flags::Error::Missing("fresh renderer link flags"));
                }
                Ok(None)
            }
            _ => action_missing(),
        }
    }
    fn refresh_selected_view(&mut self, _: ActorId) -> aa_actors::ability_flags::Result<()> {
        Err(aa_actors::ability_flags::Error::Missing(
            "50C8E0 selected view refresh",
        ))
    }
}
fn action_missing<T>() -> aa_actors::ability_flags::Result<T> {
    Err(aa_actors::ability_flags::Error::Missing(
        "live action contributor/publication after early construction",
    ))
}
impl aa_actors::ability_flags::World for ConstructionActions<'_, '_> {
    fn actor_fields(
        &self,
        a: ActorId,
    ) -> aa_actors::ability_flags::Result<aa_actors::ability_flags::ActorFields> {
        let (flags, state, words) = match self {
            Self::Saved {
                owner,
                components,
                flags,
                words,
                ..
            } => {
                if a != *owner {
                    return action_missing();
                }
                (**flags, components.state, &**words)
            }
            Self::Store(store) | Self::SourceStore{store,..} => {
                let key = store
                    .key(a)
                    .ok_or(aa_actors::ability_flags::Error::Missing("GTID"))?;
                let object = store
                    .object(key)
                    .map_err(|e| aa_actors::ability_flags::Error::World(e.to_string()))?;
                let PlacementOwner::Staged(p) = object.placement else {
                    return action_missing();
                };
                let actor = object
                    .actor
                    .as_ref()
                    .ok_or(aa_actors::ability_flags::Error::Missing("Actor"))?;
                (p.flags, actor.components.state, &actor.words)
            }
        };
        let low = *words
            .get(&0xf0)
            .ok_or(aa_actors::ability_flags::Error::Missing("actions low"))?;
        let high = *words
            .get(&0xf4)
            .ok_or(aa_actors::ability_flags::Error::Missing("actions high"))?;
        Ok(aa_actors::ability_flags::ActorFields {
            thing_flags: flags,
            state_bits: u64::from(state.low) | (u64::from(state.high) << 32),
            action_bits: u64::from(low) | (u64::from(high) << 32),
        })
    }
    fn set_action_bits(&mut self, a: ActorId, bits: u64) -> aa_actors::ability_flags::Result<()> {
        match self {
            Self::Store(store) | Self::SourceStore{store,..} => store
                .set_action_bits(a, bits)
                .map_err(|e| aa_actors::ability_flags::Error::World(e.to_string())),
            Self::Saved { owner, words, .. } => {
                if a != *owner {
                    return action_missing();
                }
                words.insert(0xf0, bits as u32);
                words.insert(0xf4, (bits >> 32) as u32);
                Ok(())
            }
        }
    }
    fn has_component(
        &self,
        a: ActorId,
        s: ComponentSlot,
    ) -> aa_actors::ability_flags::Result<bool> {
        match self {
            Self::Store(store) | Self::SourceStore{store,..} => store
                .component(a, s)
                .map(|c| c.is_some())
                .map_err(|e| aa_actors::ability_flags::Error::World(e.to_string())),
            Self::Saved {
                owner, components, ..
            } => {
                if a != *owner {
                    return action_missing();
                }
                Ok(components.present(s))
            }
        }
    }
    fn element(
        &self,
        _: ActorId,
    ) -> aa_actors::ability_flags::Result<aa_actors::ability_flags::ElementView> {
        action_missing()
    }
    fn organization_member_count(&self, _: ActorId) -> aa_actors::ability_flags::Result<usize> {
        action_missing()
    }
    fn organization_member(
        &self,
        _: ActorId,
        _: usize,
    ) -> aa_actors::ability_flags::Result<Option<ActorId>> {
        action_missing()
    }
    fn organization_member_44(
        &self,
        _: ActorId,
    ) -> aa_actors::ability_flags::Result<Option<ActorId>> {
        action_missing()
    }
    fn organization_reference_d0(&self, _: ActorId) -> aa_actors::ability_flags::Result<ActorId> {
        action_missing()
    }
    fn valid_reference(&self, id: ActorId) -> aa_actors::ability_flags::Result<bool> {
        match self {
            Self::Store(store) | Self::SourceStore{store,..} => Ok(store.key(id).is_some()),
            Self::Saved { .. } => action_missing(),
        }
    }
    fn organization_group_count(&self, _: ActorId) -> aa_actors::ability_flags::Result<usize> {
        action_missing()
    }
    fn organization_attack_group(
        &self,
        _: ActorId,
        _: usize,
    ) -> aa_actors::ability_flags::Result<aa_actors::ability_flags::AttackGroup> {
        action_missing()
    }
    fn owner(&mut self, _: ActorId) -> aa_actors::ability_flags::Result<Option<ActorId>> {
        action_missing()
    }
    fn kingdom_word_218(&self, _: ActorId) -> aa_actors::ability_flags::Result<u32> {
        action_missing()
    }
    fn state_resupply(&self, _: u8) -> aa_actors::ability_flags::Result<bool> {
        action_missing()
    }
    fn owner_can_recruit(
        &mut self,
        _: ActorId,
        _: ActorId,
    ) -> aa_actors::ability_flags::Result<bool> {
        action_missing()
    }
    fn owner_recruit_count(
        &mut self,
        _: ActorId,
        _: ActorId,
    ) -> aa_actors::ability_flags::Result<i32> {
        action_missing()
    }
    fn action_available(
        &mut self,
        a: ActorId,
        index: u32,
    ) -> aa_actors::ability_flags::Result<bool> {
        aa_actors::ability_flags::available(self.actor_fields(a)?.action_bits, index)
    }
    fn other_contributor(
        &mut self,
        a: ActorId,
        slot: ComponentSlot,
    ) -> aa_actors::ability_flags::Result<()> {
        if matches!(self, Self::Saved { .. }|Self::SourceStore{..}) && slot == ComponentSlot::Construction {
            return aa_actors::construction_actions::contribute(self, a);
        }
        action_missing()
    }
    fn mind_contributor(&mut self, _: ActorId) -> aa_actors::ability_flags::Result<()> {
        action_missing()
    }
    fn changed(&mut self, a: ActorId) -> aa_actors::ability_flags::Result<()> {
        if matches!(self, Self::Saved { .. }|Self::SourceStore{..}) {
            return aa_actors::action_publication::changed(a, self);
        }
        action_missing()
    }
}

/// Length-prefixed source ID plus the actual native set and row. Equal authored
/// names in different concrete sets remain distinct native animation identities.
/// Only a wholly absent aspect list shares the final/base set pointer45E690.
pub fn animation_key(actor: &str, set: u32, row: usize) -> String {
    let actor = actor.to_ascii_lowercase();
    format!("{}:{actor}:{set}:{row}", actor.len())
}
pub fn effective_animation_set(
    character: &aa_animation::definitions::CharacterDefinitions,
    set: u32,
) -> Result<u32> {
    let count = u32::try_from(character.aspect_ids.len()).map_err(|_| Error::Exhausted)?;
    if set == count {
        return Ok(count);
    }
    let aspect = character
        .aspect_ids
        .get(set as usize)
        .ok_or(Error::Missing("animation set outside source aspects"))?;
    Ok(
        if character.aspects.contains_key(&aspect.to_ascii_lowercase()) {
            set
        } else {
            count
        },
    )
}
#[derive(Clone, Debug)]
pub struct AnimationDefinition {
    pub actor_id: String,
    pub set: u32,
    pub row: usize,
    pub original_name: String,
    pub source: aa_animation::definitions::BoundAnimation,
}
#[derive(Default)]
pub struct AnimationBank {
    entries: BTreeMap<String, Arc<AnimationDefinition>>,
    owners: BTreeMap<String, Arc<ActorBindings>>,
}
impl AnimationBank {
    pub fn get(&self, key: &str) -> Result<&AnimationDefinition> {
        self.entries
            .get(key)
            .map(Arc::as_ref)
            .ok_or(Error::Missing("canonical Animation definition token"))
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn bind(&mut self, binding: Arc<ActorBindings>) -> Result<()> {
        let name = binding.definition.proof.id.to_ascii_lowercase();
        if let Some(previous) = self.owners.get(&name) {
            if Arc::ptr_eq(previous, &binding) {
                return Ok(());
            }
            return Err(Error::Missing(
                "Actor animations already bound to a different immutable definition snapshot",
            ));
        }
        if let Some(character) = &binding.character {
            for set in 0..character.set_count() {
                let set = u32::try_from(set).map_err(|_| Error::Exhausted)?;
                let effective = effective_animation_set(character, set)?;
                for (row, animation) in character
                    .for_variant(set)
                    .map_err(|e| Error::Source(e.to_string()))?
                    .iter()
                    .enumerate()
                {
                    let key = animation_key(&name, effective, row);
                    self.entries.entry(key).or_insert_with(|| {
                        Arc::new(AnimationDefinition {
                            actor_id: name.clone(),
                            set: effective,
                            row,
                            original_name: animation.id.clone(),
                            source: animation.clone(),
                        })
                    });
                }
            }
        }
        self.owners.insert(name, binding);
        Ok(())
    }
    /// Native name matching remains scoped to the current effective source set.
    pub fn find(&self, actor: &str, set: u32, name: &str) -> Result<Option<&AnimationDefinition>> {
        let binding = self
            .owners
            .get(&actor.to_ascii_lowercase())
            .ok_or(Error::Missing("actor Animation bank binding"))?;
        let character = binding
            .character
            .as_ref()
            .ok_or(Error::Missing("Character definition"))?;
        let effective = effective_animation_set(character, set)?;
        let rows = character
            .for_variant(set)
            .map_err(|e| Error::Source(e.to_string()))?;
        if let Some(row) = rows.iter().position(|a| a.id.eq_ignore_ascii_case(name)) {
            Ok(Some(self.get(&animation_key(actor, effective, row))?))
        } else {
            Ok(None)
        }
    }
}

/// Required owner/UI publication after the original nonempty assembly list.
/// A headless world must evaluate the actual owner/UI gates in its adapter;
/// there is no permissive default that silently declares publication complete.
pub trait AssemblyEffects {
    fn completed(
        &mut self,
        store: &mut ActorStore,
        actor: ActorId,
        created: &[ComponentSlot],
    ) -> Result<()>;
}
impl ActorStore {
    #[allow(clippy::too_many_arguments)] // Explicit shared world clock/RNG/Updatable bank.
    pub fn assemble_ground(
        &mut self,
        key: ObjectKey,
        catalog: &Catalog<'_>,
        time: F32,
        rng: &mut NativeRng,
        owners: &mut crate::scheduler_owners::Owners,
        definitions: &mut crate::definitions::Bank,
        pc: Precision,
        effects: &mut impl AssemblyEffects,
    ) -> Result<()> {
        let mask = self
            .object(key)?
            .definition
            .component_mask
            .ok_or(Error::Missing("completed component mask"))?;
        self.id(key)
            .ok_or(Error::Missing("identity initialized before assembly"))?;
        let mut factory = GroundAssembly {
            store: self,
            catalog,
            time,
            rng,
            owners,
            definitions,
            effects,
        };
        match aa_actors::init_create::assemble_with(key, mask, &mut factory, pc) {
            Ok(()) => {
                self.set_phase(key, NewPhase::AwaitingInitCreate)?;
                Ok(())
            }
            Err(e) => {
                self.mark_failure(key, e.to_string());
                Err(Error::Source(e.to_string()))
            }
        }
    }
}
struct GroundAssembly<'a, 'b, E> {
    store: &'a mut ActorStore,
    catalog: &'a Catalog<'b>,
    time: F32,
    rng: &'a mut NativeRng,
    owners: &'a mut crate::scheduler_owners::Owners,
    definitions: &'a mut crate::definitions::Bank,
    effects: &'a mut E,
}
fn factory_error(e: Error) -> aa_actors::init_create::Error {
    aa_actors::init_create::Error::World(e.to_string())
}
impl<E: AssemblyEffects> aa_actors::init_create::AssemblyWorld for GroundAssembly<'_, '_, E> {
    type Actor = ObjectKey;
    fn state_bits(
        &self,
        a: ObjectKey,
    ) -> aa_actors::init_create::Result<aa_actors::components::StateBits> {
        Ok(self
            .store
            .object(a)
            .map_err(factory_error)?
            .components()
            .map_err(factory_error)?
            .state)
    }
    fn has_component(
        &self,
        a: ObjectKey,
        s: ComponentSlot,
    ) -> aa_actors::init_create::Result<bool> {
        Ok(self
            .store
            .object(a)
            .map_err(factory_error)?
            .components()
            .map_err(factory_error)?
            .present(s))
    }
    fn construct_component(
        &mut self,
        a: ObjectKey,
        s: ComponentSlot,
        _: aa_actors::init_create::FactoryKind,
        p: Precision,
    ) -> aa_actors::init_create::Result<()> {
        match s {
            ComponentSlot::Defender=>self.store.construct_defender(a,self.catalog,self.definitions),
            ComponentSlot::Mind => self.store.construct_mind(a, self.rng, p),
            ComponentSlot::Performer => {
                let input = self
                    .store
                    .source_performer(a, self.catalog, self.definitions)
                    .map_err(factory_error)?;
                self.store
                    .construct_performer(a, &input, p, &mut |store, id| {
                        store.rebuild_source_construction_actions(self.catalog,id)
                    })
            }
            _ => self.store.construct_ground(a, s, self.time, p),
        }
        .map_err(factory_error)?;
        if s != ComponentSlot::Experience
            && self
                .store
                .object(a)
                .map_err(factory_error)?
                .components()
                .map_err(factory_error)?
                .present(s)
        {
            self.owners
                .register(crate::scheduler_owners::Target::Component {
                    object: a,
                    slot: s as u8,
                })
                .map_err(|e| aa_actors::init_create::Error::World(e.to_string()))?;
        }
        Ok(())
    }
    fn component_requirement(
        &mut self,
        a: ObjectKey,
        s: ComponentSlot,
    ) -> aa_actors::init_create::Result<Option<ComponentSlot>> {
        let mask = self
            .store
            .object(a)
            .map_err(factory_error)?
            .definition
            .component_mask
            .ok_or(aa_actors::init_create::Error::Unsupported(
                "completed native component mask",
            ))?;
        let k = aa_actors::init_create::factory_kind(s, mask).ok_or(
            aa_actors::init_create::Error::Unsupported("constructed Mind subtype"),
        )?;
        let structure_state=if s==ComponentSlot::Structure {
            let owner=self.store.id(a).ok_or(aa_actors::init_create::Error::Unsupported("Structure owner GTID"))?;
            let object=self.store.object(a).map_err(factory_error)?;
            let Some(RuntimeComponent::Structure(component))=object.components().map_err(factory_error)?.get(s)else {
                return Err(aa_actors::init_create::Error::Unsupported("actual Structure payload"));
            };
            if component.header.owner!=owner{return Err(aa_actors::init_create::Error::Unsupported("actual Structure owner differs"));}
            Some(object.components().map_err(factory_error)?.state)
        }else{None};
        aa_actors::leaf_init::ground_requirement_with_structure_state(k,structure_state)
            .map_err(|e| aa_actors::init_create::Error::World(e.to_string()))
    }
    fn city_allowed(&mut self, _: ObjectKey) -> aa_actors::init_create::Result<bool> {
        Err(aa_actors::init_create::Error::Unsupported(
            "City source predicate outside ground factory",
        ))
    }
    fn construction_ready(&mut self, a: ObjectKey) -> aa_actors::init_create::Result<()> {
        //4B2830 is a pure RET; still require the actual installed component.
        let id=self.store.id(a).ok_or(aa_actors::init_create::Error::Unsupported("Construction owner GTID"))?;
        match self.store.component(id,ComponentSlot::Construction).map_err(factory_error)? {
            Some(RuntimeComponent::Construction(v)) if v.header.owner==id=>Ok(()),
            _=>Err(aa_actors::init_create::Error::Unsupported("actual Construction ready payload/owner")),
        }
    }
    fn assembly_completed(
        &mut self,
        a: ObjectKey,
        created: &[ComponentSlot],
    ) -> aa_actors::init_create::Result<()> {
        let id = self
            .store
            .id(a)
            .ok_or(aa_actors::init_create::Error::Unsupported(
                "registered actor",
            ))?;
        self.effects
            .completed(self.store, id, created)
            .map_err(factory_error)
    }
}

/// Disjoint live storage view for Mind callbacks. `split_minds` borrows the
/// canonical Arena and this view simultaneously; no arena is removed/replaced,
/// and callback reads still reach the same entity/component allocations.
pub struct ActorAccess<'a> {
    identities: &'a IdentityRegistry<ObjectKey>,
    entities: &'a mut BTreeMap<ObjectKey, Box<Entity>>,
    pub abilities: &'a mut BTreeMap<u32, aa_combat::ability::Ability>,
    ///The same complete allocation owner is needed by synchronous state
    ///property grants; an Instances-only loan cannot allocate native5030F0.
    pub property_allocations: &'a mut crate::load_property_instances::State,
    pub animations: &'a AnimationBank,
    ///Same saved/fresh Regiment collection owner. State status stays in Arena;
    ///path nodes/formation references are never copied into Installed words.
    pub regiment_owned: &'a mut BTreeMap<aa_mind::MindId, aa_mind::regiment_load::Owned>,
}
impl ActorStore {
    pub fn split_minds(&mut self) -> (&mut aa_mind::Arena, ActorAccess<'_>) {
        (
            &mut self.minds,
            ActorAccess {
                identities: &self.identities,
                entities: &mut self.entities,
                abilities: &mut self.abilities,
                property_allocations: &mut self.property_allocations,
                animations: &self.animations,
                regiment_owned: &mut self.regiment_owned,
            },
        )
    }
}
impl ActorAccess<'_> {
    /// Shorter disjoint borrow for synchronous nested Mind callbacks. No arena
    /// or component is extracted, copied, or replaced while callbacks run.
    pub fn reborrow(&mut self) -> ActorAccess<'_> {
        ActorAccess {
            identities: self.identities,
            entities: &mut *self.entities,
            abilities: &mut *self.abilities,
            property_allocations: &mut *self.property_allocations,
            animations: self.animations,
            regiment_owned: &mut *self.regiment_owned,
        }
    }

    pub fn identities(&self) -> &IdentityRegistry<ObjectKey> {
        self.identities
    }
    pub fn checked_key(&self, raw: u64) -> Option<ObjectKey> {
        let key = ObjectKey(raw);
        self.entities.contains_key(&key).then_some(key)
    }
    pub fn key(&self, id: ActorId) -> Option<ObjectKey> {
        self.identities.get(id).map(|e| e.value)
    }
    pub fn id(&self, key: ObjectKey) -> Option<ActorId> {
        self.identities
            .iter()
            .find(|e| e.value == key)
            .map(|e| e.id)
    }
    pub fn entity(&self, key: ObjectKey) -> Result<&Entity> {
        self.entities
            .get(&key)
            .map(Box::as_ref)
            .ok_or(Error::Missing("object arena identity"))
    }
    pub fn entity_mut(&mut self, key: ObjectKey) -> Result<&mut Entity> {
        self.entities
            .get_mut(&key)
            .map(Box::as_mut)
            .ok_or(Error::Missing("object arena identity"))
    }
    pub fn object(&self, key: ObjectKey) -> Result<&RuntimeObject> {
        self.entity(key)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("restored runtime constructor"))
    }
    pub fn object_mut(&mut self, key: ObjectKey) -> Result<&mut RuntimeObject> {
        self.entity_mut(key)?
            .runtime
            .as_mut()
            .ok_or(Error::Missing("restored runtime constructor"))
    }
    pub fn component(&self, id: ActorId, slot: ComponentSlot) -> Result<Option<&RuntimeComponent>> {
        let key = self.key(id).ok_or(Error::Missing("live actor GTID"))?;
        Ok(self.object(key)?.components()?.get(slot))
    }
    pub fn component_mut(
        &mut self,
        id: ActorId,
        slot: ComponentSlot,
    ) -> Result<Option<&mut RuntimeComponent>> {
        let key = self.key(id).ok_or(Error::Missing("live actor GTID"))?;
        Ok(self.object_mut(key)?.components_mut()?.get_mut(slot))
    }
    pub fn placement(&self, key: ObjectKey, spatial: &impl Spatial) -> Result<PlacementState> {
        match self.object(key)?.placement {
            PlacementOwner::Staged(p) => Ok(p),
            PlacementOwner::Spatial => {
                spatial.placement(self.id(key).ok_or(Error::Missing("published GTID"))?)
            }
        }
    }
    pub fn staged_placement_mut(&mut self, key: ObjectKey) -> Result<&mut PlacementState> {
        match &mut self.object_mut(key)?.placement {
            PlacementOwner::Staged(p) => Ok(p),
            PlacementOwner::Spatial => Err(Error::Missing(
                "published placement must mutate through spatial runtime",
            )),
        }
    }
}

impl ActorStore {
    /// Executes the ordinary Character/Mover/Properties/Organization/Element/
    /// Economy/Experience/Detection InitCreate leaves against the shared original
    /// queues and RNG. Specialized Body/Mind/Performer methods remain explicit.
    /// Live scheduling belongs to Queues; no constructor head snapshot is reused.
    pub fn initialize_ground_support(
        &mut self,
        key: ObjectKey,
        slot: ComponentSlot,
        owners: &crate::scheduler_owners::Owners,
        queues: &mut aa_world::queues::Queues,
        rng: &mut NativeRng,
    ) -> Result<bool> {
        self.object(key)?
            .components()?
            .get(slot)
            .ok_or(Error::Missing("constructed InitCreate component"))?;
        if matches!(
            slot,
            ComponentSlot::Character | ComponentSlot::Mover | ComponentSlot::Experience
        ) {
            // Their exact native438AC0 returns true and does not read a scheduler owner.
            return aa_actors::leaf_init::ground_support(slot, false, 0, queues, rng)
                .map_err(|e| Error::Source(e.to_string()));
        }
        let supply = if slot == ComponentSlot::Economy {
            self.entity(key)?
                .bindings
                .as_ref()
                .ok_or(Error::Missing("source Actor binding"))?
                .source_ground
                .economy
                .as_ref()
                .ok_or(Error::Missing("source Economy"))?
                .supply_required
                .boolean()
                .map_err(|e| Error::Source(e.to_string()))?
        } else {
            false
        };
        let owner = owners
            .get(crate::scheduler_owners::Target::Component {
                object: key,
                slot: slot as u8,
            })
            .map_err(|e| Error::Source(e.to_string()))?;
        aa_actors::leaf_init::ground_support(slot, supply, owner, queues, rng)
            .map_err(|e| Error::Source(e.to_string()))
    }
}

impl ActorStore {
    /// Ordinary Body InitCreate when native state21 is clear. The other branch
    /// requires real health/capture/death callbacks and is rejected before effects.
    #[allow(clippy::too_many_arguments)] // Explicit native record, SVar and shared scheduler state.
    pub fn initialize_ordinary_body(
        &mut self,
        key: ObjectKey,
        record: Option<aa_actors::leaf_init::CreationRecord>,
        unit_supply_enable: bool,
        owners: &crate::scheduler_owners::Owners,
        queues: &mut aa_world::queues::Queues,
        rng: &mut NativeRng,
    ) -> Result<bool> {
        let object = self.object(key)?;
        let components = object.components()?;
        if !matches!(
            components.get(ComponentSlot::Body),
            Some(RuntimeComponent::Body(_))
        ) {
            return Err(Error::Missing("constructed Body"));
        }
        if components.state.construction() {
            return Err(Error::Missing("construction Body health callbacks"));
        }
        let has_organization = components.present(ComponentSlot::Organization);
        let rate = if record.is_some() && unit_supply_enable && !has_organization {
            F32(self
                .entity(key)?
                .bindings
                .as_ref()
                .ok_or(Error::Missing("source Actor binding"))?
                .source_ground
                .body_shortfall_damage_rate
                .as_ref()
                .ok_or(Error::Missing("Body unit_shortfall_damage_rate"))?
                .bits)
        } else {
            F32::ZERO
        }; // Unread source scalar under the exact native short-circuits.
        let owner = owners
            .get(crate::scheduler_owners::Target::Component {
                object: key,
                slot: ComponentSlot::Body as u8,
            })
            .map_err(|e| Error::Source(e.to_string()))?;
        aa_actors::leaf_init::body_queue_after_health(
            record,
            aa_actors::leaf_init::BodySupply {
                unit_supply_enable,
                has_organization,
                unit_shortfall_damage_rate: rate,
            },
            owner,
            queues,
            rng,
        )
        .map_err(|e| Error::Source(e.to_string()))
    }
}

impl ActorStore {
    /// Reserves one portable property-instance identity. Actual5030F0 construction
    /// and581630 insertion remain separate observable steps; failed insertion does
    /// not cause this identity to be reused for an unrelated instance.
    pub fn reserve_property_instance_handle(&mut self) -> Result<aa_body::credit::Object> {
        self.property_allocations.reserve_handle().map_err(Error::Source)
    }
}

impl ActorStore {
    ///57DB90 calls581420(attribute0) only when Properties exists. Immediately
    /// after new component assembly it is constructor-empty, before PostCreate
    /// adds initial properties. Nonempty live lists require the full definition
    /// and numeric-consequence adapter rather than being silently skipped.
    pub fn initialize_new_performer(&self, key: ObjectKey) -> Result<bool> {
        let object = self.object(key)?;
        let components = object.components()?;
        if !matches!(
            components.get(ComponentSlot::Performer),
            Some(RuntimeComponent::Performer(_))
        ) {
            return Err(Error::Missing("constructed Performer"));
        }
        if let Some(component) = components.get(ComponentSlot::Properties) {
            let RuntimeComponent::Properties(state) = component else {
                return Err(Error::Missing("Properties payload"));
            };
            aa_body::property_refresh::require_empty(state)
                .map_err(|e| Error::Source(e.to_string()))?;
        }
        Ok(true)
    }
}

#[path = "actor_economy_focus.rs"]
mod economy_focus;
pub use economy_focus::{ActorParts, EconomyAccess, EntityParts, RuntimeParts};

fn stored_owner_word(object: &RuntimeObject) -> Result<Option<ActorId>> {
    let actor = object
        .actor
        .as_ref()
        .ok_or(Error::Missing("ordinary GActor owner field"))?;
    let word = *actor
        .words
        .get(&0xe8)
        .ok_or(Error::Missing("initialized Actor owner reference"))?;
    Ok((word != 0).then_some(ActorId::from_raw(word)))
}
impl ActorStore {
    /// Mapworld stores a resolved portable GTID at E8, never a native pointer.
    ///4D1E80 itself has no callback; callers must prove that ordinary dispatch.
    pub fn ordinary_owner(&self, actor: ActorId) -> Result<Option<ActorId>> {
        let key = self
            .key(actor)
            .ok_or(Error::Missing("ordinary actor GTID"))?;
        let Some(owner) = stored_owner_word(self.object(key)?)? else {
            return Ok(None);
        };
        let owner_key = self
            .key(owner)
            .ok_or(Error::Missing("resolved owner GTID is stale"))?;
        if self.object(owner_key)?.definition.native_class != 4 {
            return Err(Error::Missing("ordinary owner must resolve Kingdom class4"));
        }
        Ok(Some(owner))
    }
    pub fn action_fields(
        &self,
        actor: ActorId,
        spatial: &impl Spatial,
    ) -> Result<aa_actors::ability_flags::ActorFields> {
        let key = self.key(actor).ok_or(Error::Missing("actor GTID"))?;
        let a = self
            .object(key)?
            .actor
            .as_ref()
            .ok_or(Error::Missing("Actor shell"))?;
        Ok(aa_actors::ability_flags::ActorFields {
            thing_flags: self.placement(key, spatial)?.flags,
            state_bits: u64::from(a.components.state.low)
                | (u64::from(a.components.state.high) << 32),
            action_bits: u64::from(
                *a.words
                    .get(&0xf0)
                    .ok_or(Error::Missing("action low word"))?,
            ) | (u64::from(
                *a.words
                    .get(&0xf4)
                    .ok_or(Error::Missing("action high word"))?,
            ) << 32),
        })
    }
    pub fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()> {
        let key = self.key(actor).ok_or(Error::Missing("actor GTID"))?;
        let a = self
            .object_mut(key)?
            .actor
            .as_mut()
            .ok_or(Error::Missing("Actor shell"))?;
        a.words.insert(0xf0, bits as u32);
        a.words.insert(0xf4, (bits >> 32) as u32);
        Ok(())
    }
}

impl ActorAccess<'_> {
    pub fn ordinary_owner(&self, actor: ActorId) -> Result<Option<ActorId>> {
        let key = self
            .key(actor)
            .ok_or(Error::Missing("ordinary actor GTID"))?;
        let Some(owner) = stored_owner_word(self.object(key)?)? else {
            return Ok(None);
        };
        let owner_key = self
            .key(owner)
            .ok_or(Error::Missing("resolved owner GTID is stale"))?;
        if self.object(owner_key)?.definition.native_class != 4 {
            return Err(Error::Missing("ordinary owner must resolve Kingdom class4"));
        }
        Ok(Some(owner))
    }
    pub fn action_fields(
        &self,
        actor: ActorId,
        spatial: &impl Spatial,
    ) -> Result<aa_actors::ability_flags::ActorFields> {
        let key = self.key(actor).ok_or(Error::Missing("actor GTID"))?;
        let a = self
            .object(key)?
            .actor
            .as_ref()
            .ok_or(Error::Missing("Actor shell"))?;
        Ok(aa_actors::ability_flags::ActorFields {
            thing_flags: self.placement(key, spatial)?.flags,
            state_bits: u64::from(a.components.state.low)
                | (u64::from(a.components.state.high) << 32),
            action_bits: u64::from(
                *a.words
                    .get(&0xf0)
                    .ok_or(Error::Missing("action low word"))?,
            ) | (u64::from(
                *a.words
                    .get(&0xf4)
                    .ok_or(Error::Missing("action high word"))?,
            ) << 32),
        })
    }
    pub fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()> {
        let key = self.key(actor).ok_or(Error::Missing("actor GTID"))?;
        let a = self
            .object_mut(key)?
            .actor
            .as_mut()
            .ok_or(Error::Missing("Actor shell"))?;
        a.words.insert(0xf0, bits as u32);
        a.words.insert(0xf4, (bits >> 32) as u32);
        Ok(())
    }
}

///Synchronous disjoint zone loan. Every actor read resolves its SAME fields;
///the current runtime allocation is never detached or replaced by an empty one.
pub(crate) struct ZoneFields<'a> {
    pub owner: ActorId,
    pub key: ObjectKey,
    pub definition: &'a RuntimeDefinition,
    pub kind: LoadKind,
    pub thing: &'a ThingCore,
    pub object: &'a mut ObjectFields,
    pub components: &'a mut Components<RuntimeComponent>,
    pub words: &'a BTreeMap<u32, u32>,
    string_dc_utf16: &'a [u16],
    string_e0_utf16: &'a [u16],
    placement: &'a PlacementOwner,
    source: &'a Source,
    identities: &'a IdentityRegistry<ObjectKey>,
    others: BTreeMap<ObjectKey, &'a mut Entity>,
    zone_index: usize,
    zone_key: u32,
    zone_before: &'a [Option<aa_zones::zone::Zone>],
    zone_after: &'a [Option<aa_zones::zone::Zone>],
}
impl ZoneFields<'_> {
    fn prior(&self, id: ActorId) -> Result<&Entity> {
        let key = self
            .identities
            .get(id)
            .ok_or(Error::Missing("zone current Actor generation"))?
            .value;
        self.others
            .get(&key)
            .map(|v| &**v)
            .ok_or(Error::Missing("zone actual other Actor allocation"))
    }
    pub fn present(&self, id: ActorId) -> bool {
        if id == self.owner {
            self.identities.get(id).is_some_and(|v| v.value == self.key)
        } else {
            self.prior(id).is_ok()
        }
    }
    pub fn definition(&self, id: ActorId) -> Result<&RuntimeDefinition> {
        if id == self.owner {
            return Ok(self.definition);
        }
        Ok(&self
            .prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?
            .definition)
    }
    pub fn source(&self, id: ActorId) -> Result<&Source> {
        if id == self.owner {
            Ok(self.source)
        } else {
            Ok(&self.prior(id)?.source)
        }
    }
    pub fn same_definition_proof(&self, id: ActorId) -> Result<bool> {
        if id == self.owner {
            return Ok(crate::load_spatial::same_proof(
                &self.thing.definition,
                &self.definition.proof,
            ));
        }
        let runtime = self.prior(id)?.runtime.as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?;
        Ok(crate::load_spatial::same_proof(
            &runtime.thing.definition,
            &runtime.definition.proof,
        ))
    }
    pub fn components(&self, id: ActorId) -> Result<&Components<RuntimeComponent>> {
        if id == self.owner {
            return Ok(self.components);
        }
        self.prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?
            .components()
    }
    pub fn words(&self, id: ActorId) -> Result<&BTreeMap<u32, u32>> {
        if id == self.owner {
            return Ok(self.words);
        }
        Ok(&self
            .prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?
            .actor
            .as_ref()
            .ok_or(Error::Missing("zone actual Actor storage"))?
            .words)
    }
    pub fn kind(&self, id: ActorId) -> Result<(LoadKind, bool, bool)> {
        if id == self.owner {
            return Ok((self.kind, true, true));
        }
        let r = self
            .prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?;
        Ok((r.kind, r.object.is_some(), r.actor.is_some()))
    }
    pub fn placement(&self, id: ActorId, spatial: &impl Spatial) -> Result<PlacementState> {
        let p = if id == self.owner {
            self.placement
        } else {
            &self
                .prior(id)?
                .runtime
                .as_ref()
                .ok_or(Error::Missing("zone actual Object storage"))?
                .placement
        };
        match p {
            PlacementOwner::Staged(p) => Ok(*p),
            PlacementOwner::Spatial => spatial.placement(id),
        }
    }
    pub fn zone_key(&self, id: ActorId, index: usize) -> Result<u32> {
        if id == self.owner {
            return Ok(if index == self.zone_index {
                self.zone_key
            } else if index < self.zone_index {
                self.zone_before
                    .get(index)
                    .and_then(Option::as_ref)
                    .map_or(0, |z| z.key)
            } else {
                self.zone_after
                    .get(index - self.zone_index - 1)
                    .and_then(Option::as_ref)
                    .map_or(0, |z| z.key)
            });
        }
        Ok(self
            .prior(id)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("zone actual Object storage"))?
            .actor
            .as_ref()
            .ok_or(Error::Missing("zone actual Actor storage"))?
            .zones_148
            .zones()
            .slots
            .get(index)
            .and_then(Option::as_ref)
            .map_or(0, |z| z.key))
    }
}
impl ActorStore {
    pub(crate) fn with_zone_fields<R>(
        &mut self,
        key: ObjectKey,
        index: usize,
        call: impl FnOnce(&mut ZoneFields<'_>, &mut aa_zones::zone::Zone) -> R,
    ) -> Result<R> {
        let owner = self
            .id(key)
            .ok_or(Error::Missing("zone current Actor generation"))?;
        let mut current = None;
        let mut others = BTreeMap::new();
        for (k, e) in self.entities.iter_mut() {
            if *k == key {
                current = Some(&mut **e)
            } else {
                others.insert(*k, &mut **e);
            }
        }
        let entity = current.ok_or(Error::Missing("zone actual current Entity"))?;
        let r = entity
            .runtime
            .as_mut()
            .ok_or(Error::Missing("zone actual current runtime"))?;
        let actor = r
            .actor
            .as_mut()
            .ok_or(Error::Missing("zone actual current Actor"))?;
        let slots = &mut actor.zones_148.zones_mut().slots;
        if index >= slots.len() {
            return Err(Error::Missing("zone actual current slot"));
        }
        let (before, tail) = slots.split_at_mut(index);
        let (slot, after) = tail.split_first_mut().expect("checked current slot");
        let zone = slot
            .as_mut()
            .ok_or(Error::Missing("zone actual current allocation"))?;
        let mut fields = ZoneFields {
            owner,
            key,
            definition: &r.definition,
            kind: r.kind,
            thing: &r.thing,
            object: r
                .object
                .as_mut()
                .ok_or(Error::Missing("zone actual current Object"))?,
            components: &mut actor.components,
            words: &actor.words,
            string_dc_utf16: &actor.string_dc_utf16,
            string_e0_utf16: &actor.string_e0_utf16,
            placement: &r.placement,
            source: &entity.source,
            identities: &self.identities,
            others,
            zone_index: index,
            zone_key: zone.key,
            zone_before: before,
            zone_after: after,
        };
        Ok(call(&mut fields, zone))
    }
}

/// Memory/reveal callbacks borrow the split actor fields while its Zone stays
/// exclusively borrowed by the solver. Other actors remain in the same map.
pub(crate) struct ZoneMemoryAccess<'a,'fields>{
    pub fields:&'a mut ZoneFields<'fields>,
    pub spatial:&'a crate::spatial_world::SpatialWorld,
}
impl crate::stored_memory_access::Access for ZoneMemoryAccess<'_,'_>{
    fn read(&self,id:ActorId)->aa_actors::actor_creation::Result<crate::stored_memory_access::Read<'_>>{
        use crate::stored_memory_access::{ActorRead,Read};
        let fail=|e:Error|aa_actors::actor_creation::Error::World(e.to_string());
        if !self.fields.present(id){return Err(aa_actors::actor_creation::Error::World("zone memory actual Actor generation absent".into()))}
        if id==self.fields.owner{
            return Ok(Read{definition:self.fields.definition,kind:self.fields.kind,thing:self.fields.thing,
                object:self.fields.object,actor:Some(ActorRead{components:self.fields.components,
                    words:self.fields.words,string_dc_utf16:self.fields.string_dc_utf16,
                    string_e0_utf16:self.fields.string_e0_utf16}),source:self.fields.source});
        }
        let e=self.fields.prior(id).map_err(fail)?;
        let r=e.runtime.as_ref().ok_or_else(||aa_actors::actor_creation::Error::World("zone memory actual runtime absent".into()))?;
        Ok(Read{definition:&r.definition,kind:r.kind,thing:&r.thing,
            object:r.object.as_ref().ok_or(aa_actors::actor_creation::Error::NullDereference("zone memory Object"))?,
            actor:r.actor.as_ref().map(|a|ActorRead{components:&a.components,words:&a.words,
                string_dc_utf16:&a.string_dc_utf16,string_e0_utf16:&a.string_e0_utf16}),source:&e.source})
    }
    fn object_mut(&mut self,id:ActorId)->aa_actors::actor_creation::Result<&mut ObjectFields>{
        if id==self.fields.owner{
            if !self.fields.present(id){return Err(aa_actors::actor_creation::Error::World("zone memory current generation differs".into()))}
            return Ok(self.fields.object);
        }
        let key=self.fields.identities.get(id).ok_or_else(||aa_actors::actor_creation::Error::World("zone memory other generation absent".into()))?.value;
        self.fields.others.get_mut(&key).and_then(|e|e.runtime.as_mut()).and_then(|r|r.object.as_mut())
            .ok_or(aa_actors::actor_creation::Error::NullDereference("zone memory other Object"))
    }
    fn placement(&self,id:ActorId)->aa_actors::actor_creation::Result<PlacementState>{
        let fail=|e:Error|aa_actors::actor_creation::Error::World(e.to_string());
        let key=self.fields.identities.get(id).ok_or_else(||aa_actors::actor_creation::Error::World("zone memory placement generation absent".into()))?.value;
        let placement=if id==self.fields.owner{self.fields.placement}else{&self.fields.prior(id).map_err(fail)?.runtime.as_ref().ok_or_else(||aa_actors::actor_creation::Error::World("zone memory runtime absent".into()))?.placement};
        if matches!(placement,PlacementOwner::Spatial){
            let k=self.spatial.key(key).map_err(|e|aa_actors::actor_creation::Error::World(e.to_string()))?;
            if self.spatial.object(k).map_err(|e|aa_actors::actor_creation::Error::World(e.to_string()))?!=key
                ||self.spatial.index().reference_for_key(k).map_err(|e|aa_actors::actor_creation::Error::World(e.to_string()))?!=id.raw()
                ||self.spatial.index().resolve_reference(id.raw())!=Some(k){return Err(aa_actors::actor_creation::Error::World("zone memory paired spatial identity differs".into()))}
        }
        self.fields.placement(id,self.spatial).map_err(fail)
    }
}
