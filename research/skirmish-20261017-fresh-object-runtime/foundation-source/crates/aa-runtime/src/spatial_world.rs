//! ActorStore publication into one authoritative native spatial index. Only
//! ordinary class3 Actor shape virtuals are bound; derived classes are errors.
//! Source geometry is immutable definition data, never renderer mesh bounds.
use crate::{
    actor_world::{
        self, ActorAccess, ActorStore, ObjectKey, PlacementState, RuntimeObject, Spatial,
    },
    catalog::Catalog,
};
use aa_actors::{ActorId, components::ComponentSlot, loading::LoadKind};
use aa_numeric::Precision;
use aa_spatial::{
    collision::{Actor, Grid, Shape},
    index::{ActorKey, Member, SpatialIndex, TransformCallbacks},
    index_world::GroundSurface,
    range::ActorTransform,
    soft_range,
};
use std::{collections::BTreeMap, sync::{Arc, Weak}};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Actor(#[from] actor_world::Error),
    #[error(transparent)]
    Spatial(#[from] aa_spatial::Error),
    #[error("runtime spatial source binding: {0}")]
    Source(String),
    #[error("runtime spatial operation unavailable: {0}")]
    Missing(&'static str),
    #[error("portable spatial identity exhausted")]
    Exhausted,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub struct SourceGeometry {
    pub definition: u32,
    pub source: aa_rules::objects::ObjectDefinition,
    pub definition_flags: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    Blocking,
    Soft,
    Target,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeHandle {
    pub definition: u32,
    pub kind: ShapeKind,
}
#[derive(Clone, Copy, Debug)]
struct Prepared {
    object: ObjectKey,
    definition: u32,
    wall_present: bool,
    actor_category: bool,
    loaded_key: Option<ActorKey>,
}
fn shape(s: &aa_rules::objects::Shape) -> Shape {
    let w = s.words;
    Shape {
        kind: w[0],
        radius_bits: w[1],
        bounds: [w[2], w[3], w[4], w[5]],
    }
}
fn source_error(e: impl std::fmt::Display) -> actor_world::Error {
    actor_world::Error::Source(e.to_string())
}

/// Non-owning identity of one World, preserved through moves and protected
/// against allocator address reuse after that World is dropped.
pub struct SpatialWorldWitness(Weak<()>);
impl SpatialWorldWitness {
    pub fn is_for(&self, world: &SpatialWorld) -> bool {
        self.0.ptr_eq(&Arc::downgrade(&world.owner_identity))
    }
}
pub struct SpatialWorld {
    owner_identity: Arc<()>,
    index: SpatialIndex,
    prepared: BTreeMap<ActorId, Prepared>,
    definitions: BTreeMap<u32, Arc<SourceGeometry>>,
    objects: BTreeMap<ObjectKey, ActorKey>,
    keys: BTreeMap<ActorKey, ObjectKey>,
    actors: BTreeMap<ActorId, ActorKey>,
    next_key: ActorKey,
}
/// The world owns the original load-time cache before any fallible transfer.
/// On error, completed actors route to this world; pending actors retain their
/// staged placement. Committed native nodes/flags are never dropped/reinserted.
pub struct LoadAdoption {
    pub world: SpatialWorld,
    pub completed: Vec<ActorId>,
    pub pending: Vec<crate::load_spatial::Binding>,
    pub error: Option<Error>,
}
/// Immutable identity/source references split from the mutable index. Cloning
/// this view copies only references, never pose, flags, membership or geometry.
#[derive(Clone, Copy)]
pub struct SpatialBindings<'a> {
    objects: &'a BTreeMap<ObjectKey, ActorKey>,
    keys: &'a BTreeMap<ActorKey, ObjectKey>,
    definitions: &'a BTreeMap<u32, Arc<SourceGeometry>>,
}
impl SpatialBindings<'_> {
    pub fn key(&self, object: ObjectKey) -> Result<ActorKey> {
        self.objects
            .get(&object)
            .copied()
            .ok_or(Error::Missing("published object spatial identity"))
    }
    pub fn object(&self, key: ActorKey) -> Result<ObjectKey> {
        self.keys
            .get(&key)
            .copied()
            .ok_or(Error::Missing("spatial object identity"))
    }
    pub fn source_geometry(&self, definition: u32) -> Result<&SourceGeometry> {
        self.definitions
            .get(&definition)
            .map(Arc::as_ref)
            .ok_or(Error::Missing("source shape definition"))
    }
}
impl SpatialWorld {
    pub fn adopt_loaded(parts: crate::load_spatial::Parts, store: &mut ActorStore) -> LoadAdoption {
        let mut world = Self {
            owner_identity: Arc::new(()),
            index: parts.index,
            prepared: BTreeMap::new(),
            definitions: parts.definitions,
            objects: BTreeMap::new(),
            keys: BTreeMap::new(),
            actors: BTreeMap::new(),
            next_key: parts.next_key,
        };
        let bindings = parts.bindings;
        let mut completed = Vec::new();
        for (i, binding) in bindings.iter().enumerate() {
            let result = (|| -> Result<()> {
                let object = store.key(binding.id).ok_or(Error::Missing(
                    "loaded spatial GTID missing from staged store",
                ))?;
                let runtime = store.object(object)?;
                let geometry = world.source_geometry(binding.definition)?;
                if runtime.definition.native_class != binding.native_class
                    || runtime.definition.flags != geometry.definition_flags
                    || !runtime
                        .definition
                        .proof
                        .id
                        .eq_ignore_ascii_case(&geometry.source.id)
                    || runtime.definition.proof.source != geometry.source.source
                    || !crate::load_spatial::same_proof(
                        &runtime.thing.definition,
                        &runtime.definition.proof,
                    )
                    || !crate::load_spatial::same_proof(&binding.proof, &runtime.definition.proof)
                    || !crate::load_spatial::ordinary_layout(
                        binding.native_class,
                        runtime.kind,
                        runtime.object.is_some(),
                        runtime.actor.is_some(),
                    )
                    || world.index.resolve_reference(binding.id.raw()) != Some(binding.key)
                {
                    return Err(Error::Missing(
                        "loaded spatial adoption source/generation differs",
                    ));
                }
                let wall_present = runtime
                    .actor
                    .as_ref()
                    .is_some_and(|a| a.components.present(ComponentSlot::Wall));
                world.prepared.insert(
                    binding.id,
                    Prepared {
                        object,
                        definition: binding.definition,
                        wall_present,
                        actor_category: binding.native_class != 2,
                        loaded_key: Some(binding.key),
                    },
                );
                store.publish_placement(object, &mut world)?;
                Ok(())
            })();
            if let Err(error) = result {
                return LoadAdoption {
                    world,
                    completed,
                    pending: bindings[i..].to_vec(),
                    error: Some(error),
                };
            }
            completed.push(binding.id);
        }
        LoadAdoption {
            world,
            completed,
            pending: Vec::new(),
            error: None,
        }
    }
    pub fn new(grid: Grid, tile_flags: Vec<u32>, precision: Precision) -> Result<Self> {
        Ok(Self {
            owner_identity: Arc::new(()),
            index: SpatialIndex::new(grid, tile_flags, precision)?,
            prepared: BTreeMap::new(),
            definitions: BTreeMap::new(),
            objects: BTreeMap::new(),
            keys: BTreeMap::new(),
            actors: BTreeMap::new(),
            next_key: 1,
        })
    }
    pub fn index(&self) -> &SpatialIndex {
        &self.index
    }
    pub fn identity_witness(&self) -> SpatialWorldWitness {
        SpatialWorldWitness(Arc::downgrade(&self.owner_identity))
    }
    /// Immutable source lifetime only; no pose, flags or membership are copied.
    pub fn source_geometry_owner(&self, definition: u32) -> Result<Arc<SourceGeometry>> {
        self.definitions.get(&definition).cloned()
            .ok_or(Error::Missing("source shape definition"))
    }
    /// Callers must preserve this world's token/identity maps when operating on
    /// the index. Ordinary transform and membership methods below do so.
    pub fn index_mut(&mut self) -> &mut SpatialIndex {
        &mut self.index
    }
    /// Keep native removal bits in the same published placement allocation.
    pub fn or_removal_flags(&mut self, object: ObjectKey, bits: u32) -> Result<()> {
        let key = self.key(object)?;
        self.index.or_removal_flags(key, bits)?;
        Ok(())
    }
    ///Native4E44E0 flag stores keep the same published member allocation.
    pub fn clear_registration_flags(&mut self, object:ObjectKey, bits:u32)->Result<()> {
        let key=self.key(object)?;
        self.index.clear_registration_flags(key,bits)?;
        Ok(())
    }
    ///Actual4F9510 AND7FFFFFFF on the sole published Thing+8 owner.
    pub fn clear_loading_flag(&mut self, object:ObjectKey)->Result<()> {
        let key=self.key(object)?;
        self.index.clear_loading_flag(key)?;
        Ok(())
    }
    ///Native4E6180 keeps registered/counted stores in the published member.
    pub fn or_registration_flags(&mut self, object:ObjectKey, bits:u32)->Result<()> {
        let key=self.key(object)?;
        self.index.or_registration_flags(key,bits)?;
        Ok(())
    }
    /// Native4EE560 removes membership before4EFEC0 retires the GTID. Keep
    /// the detached field allocation alive for the later actual destructor.
    pub(crate) fn detach_for_destruction(
        &mut self, object:ObjectKey, id:ActorId, pc:Precision,
    )->Result<PlacementState> {
        let key=self.key(object)?;
        if self.keys.get(&key)!=Some(&object)
            || self.actors.get(&id)!=Some(&key)
            || self.index.member(key)?.reference!=id.raw()
            || self.index.resolve_reference(id.raw())!=Some(key)
            || self.prepared.contains_key(&id)
        {
            return Err(Error::Missing("destruction spatial identity/precision differs"));
        }
        if self.index.member(key)?.actor.flags & (1<<26)!=0 {
            if self.index.precision()!=pc {
                return Err(Error::Missing("destruction membership precision differs"));
            }
            self.index.remove(key)?;
        }
        // unregister refuses outstanding query/list ownership. If it fails,
        // membership mutations remain in this same index and maps remain live.
        let member=self.index.unregister(key)?;
        self.objects.remove(&object);
        self.keys.remove(&key);
        self.actors.remove(&id);
        Ok(PlacementState {
            flags:member.actor.flags,
            position_bits:[member.actor.position[0],member.actor.position[1],member.height_bits],
            facing_bits:[member.actor.facing[0],member.actor.facing[1],member.facing_z_bits],
            cached_tile:Some(member.cached_tile),
        })
    }
    #[cfg(test)]
    pub(crate) fn laboratory_destruction_member(object:ObjectKey,id:ActorId,flags:u32,pc:Precision)->Self {
        //Explicit storage-only fixture. No canonical source/shape getter or
        //native load/adoption completion is asserted by this constructor.
        let mut world=Self::new(Grid {dimensions:[4,4],coordinate_shift:0,cell_size_bits:1f32.to_bits()},vec![0;16],pc).unwrap();
        let s=Shape {kind:1,radius_bits:0.25f32.to_bits(),bounds:[0;4]};
        world.index.register(Member {key:17,reference:id.raw(),actor:Actor {definition:42,
            flags:flags&!(1<<26),position:[1.25f32.to_bits();2],facing:[1f32.to_bits(),0]},
            height_bits:3f32.to_bits(),facing_z_bits:0,cached_tile:[1,1],blocking:s.clone(),soft:s.clone(),target:s,
            actor_category:true,wall_present:false}).unwrap();
        if flags&(1<<26)!=0 {world.index.insert(17).unwrap();}
        world.objects.insert(object,17);world.keys.insert(17,object);world.actors.insert(id,17);
        world
    }
    pub fn split_index(&mut self) -> (&mut SpatialIndex, SpatialBindings<'_>) {
        (
            &mut self.index,
            SpatialBindings {
                objects: &self.objects,
                keys: &self.keys,
                definitions: &self.definitions,
            },
        )
    }
    pub fn key(&self, object: ObjectKey) -> Result<ActorKey> {
        self.objects
            .get(&object)
            .copied()
            .ok_or(Error::Missing("published object spatial identity"))
    }
    pub fn object(&self, key: ActorKey) -> Result<ObjectKey> {
        self.keys
            .get(&key)
            .copied()
            .ok_or(Error::Missing("spatial object identity"))
    }
    pub fn source_geometry(&self, definition: u32) -> Result<&SourceGeometry> {
        self.definitions
            .get(&definition)
            .map(Arc::as_ref)
            .ok_or(Error::Missing("source shape definition"))
    }
    /// Bind actual source identities and initialized component/cache fields.
    /// No mutable placement or runtime flag copy is retained here.
    pub fn prepare_actor(
        &mut self,
        store: &ActorStore,
        catalog: &Catalog<'_>,
        object: ObjectKey,
    ) -> Result<()> {
        let id = store
            .id(object)
            .ok_or(Error::Missing("GTID before spatial preparation"))?;
        if self.actors.contains_key(&id) || self.objects.contains_key(&object) {
            return Err(Error::Missing("object already published"));
        }
        let runtime = store.object(object)?;
        let definition = &runtime.definition;
        if runtime.kind != LoadKind::Actor
            || definition.native_class != 3
            || definition.proof.category.runtime_class != 3
            || definition.proof.category.native_kind != 2
        {
            return Err(Error::Missing(
                "derived/non-Actor shape virtuals are not implemented",
            ));
        }
        if !runtime
            .thing
            .definition
            .id
            .eq_ignore_ascii_case(&definition.proof.id)
        {
            return Err(Error::Missing("actor/source definition identity mismatch"));
        }
        let source = catalog
            .actors
            .native_object_definition(&definition.proof.id, self.index.precision())
            .map_err(|e| Error::Source(e.to_string()))?;
        // A different catalog with the same text ID must not silently replace
        // the actor's immutable source binding.
        if source.source != definition.proof.source {
            return Err(Error::Missing(
                "shape source provenance differs from actor source",
            ));
        }
        let token = catalog
            .definition_bank
            .try_borrow_mut()
            .map_err(|_| Error::Missing("nested definition bank mutation"))?
            .intern("ActorDefinition", &definition.proof.id)
            .map_err(Error::Source)?;
        if let Some(old) = self.definitions.get(&token) {
            if old.source.id != source.id
                || old.source.source != source.source
                || old.source.blocking != source.blocking
                || old.source.soft != source.soft
                || old.source.target != source.target
                || old.source.has_explicit_target != source.has_explicit_target
                || old.definition_flags != definition.flags
            {
                return Err(Error::Missing(
                    "spatial definition token conflicts with bound source",
                ));
            }
        } else {
            self.definitions.insert(
                token,
                Arc::new(SourceGeometry {
                    definition: token,
                    source,
                    definition_flags: definition.flags,
                }),
            );
        }
        let components = runtime.components()?;
        let wall_present = components.present(ComponentSlot::Wall);
        let mask = definition
            .component_mask
            .ok_or(Error::Missing("completed actor component mask"))?;
        if mask & (1 << ComponentSlot::Wall.index()) != 0 && !wall_present {
            return Err(Error::Missing(
                "Wall component must be constructed before spatial preparation",
            ));
        }
        self.prepared.insert(
            id,
            Prepared {
                object,
                definition: token,
                wall_present,
                actor_category: true,
                loaded_key: None,
            },
        );
        Ok(())
    }
    /// Fresh same-world source binding after the actual Bank moved to Staged.
    /// Source arithmetic uses its verified PC, independently of this live index.
    pub fn prepare_actor_bound(&mut self,store:&ActorStore,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,object:ObjectKey)->Result<()> {
        catalog.require_source_owner().map_err(Error::Source)?;
        let receipt=bank.combat_source().map_err(Error::Source)?;
        receipt.completed_means().map_err(Error::Source)?;
        if receipt.source_identities()!=catalog.actors.sources.as_slice(){return Err(Error::Missing("fresh spatial source Bank/Catalog universe differs"));}
        let pc=receipt.source_precision();
        let id=store.id(object).ok_or(Error::Missing("fresh spatial GTID absent"))?;
        if self.actors.contains_key(&id)||self.objects.contains_key(&object)||self.prepared.contains_key(&id){return Err(Error::Missing("fresh spatial identity already prepared/published"));}
        let runtime=store.object(object)?;let definition=&runtime.definition;
        if runtime.kind!=LoadKind::Actor||definition.native_class!=3||definition.proof.category.native_kind!=2||definition.proof.category.runtime_class!=3||runtime.object.is_none()||runtime.actor.is_none(){return Err(Error::Missing("fresh spatial requires actual ordinary Actor storage"));}
        crate::live_object_init::Source::bind_in_bank(catalog,bank,definition).map_err(|e|Error::Source(e.to_string()))?;
        if !crate::load_spatial::same_proof(&runtime.thing.definition,&definition.proof){return Err(Error::Missing("fresh Thing/source proof differs"));}
        let mask=catalog.canonical_component_mask(&definition.proof.id,pc).map_err(Error::Source)?;
        if definition.component_mask!=Some(mask){return Err(Error::Missing("fresh spatial actual source component mask differs"));}
        let source=catalog.actors.native_object_definition(&definition.proof.id,pc).map_err(|e|Error::Source(e.to_string()))?;
        let token=bank.token("ActorDefinition",&definition.proof.id).ok_or(Error::Missing("fresh spatial actual definition token absent"))?;
        bank.require_usable(token).map_err(Error::Source)?;
        let wall_present=runtime.components()?.present(ComponentSlot::Wall);
        if mask&(1<<ComponentSlot::Wall.index())!=0&&!wall_present{return Err(Error::Missing("fresh Wall component must precede spatial preparation"));}
        if let Some(old)=self.definitions.get(&token){
            if old.source.id!=source.id||old.source.source!=source.source||old.source.blocking!=source.blocking||old.source.soft!=source.soft||old.source.target!=source.target||old.source.has_explicit_target!=source.has_explicit_target||old.definition_flags!=definition.flags{return Err(Error::Missing("fresh spatial token conflicts with current source geometry"));}
        }else{self.definitions.insert(token,Arc::new(SourceGeometry{definition:token,source,definition_flags:definition.flags}));}
        self.prepared.insert(id,Prepared{object,definition:token,wall_present,actor_category:true,loaded_key:None});Ok(())
    }

    /// Fresh GObject/DynamicDoodad binding to the SAME canonical geometry and
    /// index. IsActor and Wall are native false, not an Actor-shell projection.
    pub(crate) fn prepare_object_bound(&mut self,store:&ActorStore,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,object:ObjectKey)->Result<()> {
        catalog.require_source_owner().map_err(Error::Source)?;
        let receipt=bank.combat_source().map_err(Error::Source)?;
        receipt.completed_means().map_err(Error::Source)?;
        if receipt.source_identities()!=catalog.actors.sources.as_slice(){return Err(Error::Missing("fresh Object spatial source Bank differs"));}
        let pc=receipt.source_precision();
        let id=store.id(object).ok_or(Error::Missing("fresh Object spatial GTID absent"))?;
        if self.actors.contains_key(&id)||self.objects.contains_key(&object)||self.prepared.contains_key(&id){return Err(Error::Missing("fresh Object spatial identity already prepared"));}
        let runtime=store.object(object)?;let definition=&runtime.definition;
        if runtime.kind!=LoadKind::Object||!matches!(definition.native_class,2|9)||runtime.object.is_none()||runtime.actor.is_some()||definition.component_mask.is_some(){return Err(Error::Missing("fresh Object actual constructor layout differs"));}
        crate::live_object_init::Source::bind_in_bank(catalog,bank,definition).map_err(|e|Error::Source(e.to_string()))?;
        if !crate::load_spatial::same_proof(&runtime.thing.definition,&definition.proof){return Err(Error::Missing("fresh Object Thing/source proof differs"));}
        let source=catalog.actors.native_object_definition(&definition.proof.id,pc).map_err(|e|Error::Source(e.to_string()))?;
        let token=bank.token("ActorDefinition",&definition.proof.id).ok_or(Error::Missing("fresh Object actual definition token absent"))?;
        bank.require_usable(token).map_err(Error::Source)?;
        if let Some(old)=self.definitions.get(&token){
            if old.source.id!=source.id||old.source.source!=source.source||old.source.blocking!=source.blocking||old.source.soft!=source.soft||old.source.target!=source.target||old.source.has_explicit_target!=source.has_explicit_target||old.definition_flags!=definition.flags{return Err(Error::Missing("fresh Object geometry conflicts with current source"));}
        }else{self.definitions.insert(token,Arc::new(SourceGeometry{definition:token,source,definition_flags:definition.flags}));}
        self.prepared.insert(id,Prepared{object,definition:token,wall_present:false,actor_category:false,loaded_key:None});Ok(())
    }
    /// Transfer placement ownership only. Native membership publication remains
    /// explicit through refresh_membership so failure after that step cannot
    /// leave ActorStore believing its old staged transform is authoritative.
    pub fn publish_actor(
        &mut self,
        store: &mut ActorStore,
        catalog: &Catalog<'_>,
        object: ObjectKey,
    ) -> Result<()> {
        self.prepare_actor(store, catalog, object)?;
        store.publish_placement(object, self)?;
        Ok(())
    }
    /// Actual4EE4C0 with ordinary Actor virtual+B0=true. Partial membership
    /// writes survive errors in the authoritative index, just as native.
    pub fn refresh_membership(&mut self, object: ObjectKey) -> Result<()> {
        Ok(self.index.refresh_membership(self.key(object)?, true)?)
    }
    /// Native4EBE50 writes Z/cache after membership without move notifications.
    pub fn set_initialization_scalar(&mut self,object:ObjectKey,offset:u32,value:u32)->Result<()> {
        let key=self.key(object)?;self.index.set_initialization_scalar(key,offset,value)?;Ok(())
    }
    ///Same published Thing+8 word at the native falling-state bit12 store.
    pub(crate) fn mark_falling(&mut self,object:ObjectKey)->Result<()> {
        let key=self.key(object)?;self.index.mark_falling(key)?;Ok(())
    }
    pub fn set_membership_request(&mut self, object: ObjectKey, requested: bool) -> Result<()> {
        Ok(self
            .index
            .set_ordinary_membership_request(self.key(object)?, requested)?)
    }
    pub fn commit_position(
        &mut self,
        object: ObjectKey,
        position: [u32; 3],
        callbacks: &mut impl TransformCallbacks,
    ) -> Result<()> {
        Ok(self
            .index
            .commit_position(self.key(object)?, position, callbacks)?)
    }
    pub fn set_facing(
        &mut self,
        object: ObjectKey,
        facing: [u32; 2],
        callbacks: &mut impl TransformCallbacks,
    ) -> Result<()> {
        Ok(self
            .index
            .set_facing(self.key(object)?, facing, callbacks)?)
    }
    pub fn move_ground(
        &mut self,
        object: ObjectKey,
        point: [u32; 2],
        surface: &GroundSurface<'_>,
        callbacks: &mut impl TransformCallbacks,
    ) -> Result<()> {
        Ok(self
            .index
            .move_ground(self.key(object)?, point, surface, callbacks)?)
    }
    pub fn soft_view<'a, A: ActorReader>(&'a self, actors: &'a A) -> SoftView<'a, A> {
        SoftView {
            actors,
            spatial: self,
        }
    }
    ///4EE800/4EE7C0 view accepts generation-bearing ActorId words, unlike
    ///SoftView's ObjectKey interface. It binds ordinary target virtual+14.
    pub fn target_view<'a, A: ActorReader>(&'a self, actors: &'a A) -> TargetView<'a, A> {
        TargetView {
            inner: self.soft_view(actors),
        }
    }
}
impl Spatial for SpatialWorld {
    fn placement(&self, id: ActorId) -> actor_world::Result<PlacementState> {
        let key = self
            .actors
            .get(&id)
            .copied()
            .ok_or(actor_world::Error::Missing("published spatial GTID"))?;
        let m = self.index.member(key).map_err(source_error)?;
        if self.index.resolve_reference(id.raw()) != Some(key) {
            return Err(actor_world::Error::Missing("retired spatial GTID"));
        }
        Ok(PlacementState {
            flags: m.actor.flags,
            position_bits: [m.actor.position[0], m.actor.position[1], m.height_bits],
            facing_bits: [m.actor.facing[0], m.actor.facing[1], m.facing_z_bits],
            cached_tile: Some(m.cached_tile),
        })
    }
    fn publish(&mut self, id: ActorId, p: PlacementState) -> actor_world::Result<()> {
        let prepared = *self
            .prepared
            .get(&id)
            .ok_or(actor_world::Error::Missing("prepared source shapes"))?;
        if self.actors.contains_key(&id) || self.objects.contains_key(&prepared.object) {
            return Err(actor_world::Error::Missing("duplicate spatial publication"));
        }
        if prepared.loaded_key.is_none() && p.flags & (1 << 26) != 0 {
            return Err(actor_world::Error::Missing(
                "unpublished actor claims existing tile membership",
            ));
        }
        let geometry = self
            .source_geometry(prepared.definition)
            .map_err(source_error)?;
        let key = prepared.loaded_key.unwrap_or(self.next_key);
        let next = if prepared.loaded_key.is_some() {
            self.next_key
        } else {
            key.checked_add(1).ok_or(actor_world::Error::Exhausted)?
        };
        let m = Member {
            key,
            reference: id.raw(),
            actor: Actor {
                definition: prepared.definition,
                flags: p.flags,
                position: [p.position_bits[0], p.position_bits[1]],
                facing: [p.facing_bits[0], p.facing_bits[1]],
            },
            height_bits: p.position_bits[2],
            facing_z_bits: p.facing_bits[2],
            cached_tile: p.cached_tile.ok_or(actor_world::Error::Missing(
                "ordinary Actor cached tile before publication",
            ))?,
            blocking: shape(&geometry.source.blocking),
            soft: shape(&geometry.source.soft),
            target: shape(if geometry.source.has_explicit_target {
                &geometry.source.target
            } else {
                &geometry.source.soft
            }),
            actor_category: prepared.actor_category,
            wall_present: prepared.wall_present,
        };
        // register validates before committing; no recoverable operation after
        // it succeeds may reject ownership transfer back to ActorStore.
        if prepared.loaded_key.is_some() {
            self.index.attach_loaded_fields(m).map_err(source_error)?;
        } else {
            self.index.register(m).map_err(source_error)?;
        }
        self.next_key = next;
        self.objects.insert(prepared.object, key);
        self.keys.insert(key, prepared.object);
        self.actors.insert(id, key);
        self.prepared.remove(&id);
        Ok(())
    }
}
/// Ordinary virtual+10 binding over real ActorStore/index state. Definition
/// snapshots remain immutable; derived shape getters are deliberately rejected.
pub trait ActorReader {
    fn read_object(&self, key: ObjectKey) -> actor_world::Result<&RuntimeObject>;
    fn actor_id(&self, key: ObjectKey) -> Option<ActorId>;
    fn actor_key(&self, id: ActorId) -> Option<ObjectKey>;
    fn read_placement(
        &self,
        key: ObjectKey,
        spatial: &SpatialWorld,
    ) -> actor_world::Result<PlacementState>;
}
impl ActorReader for ActorStore {
    fn read_object(&self, key: ObjectKey) -> actor_world::Result<&RuntimeObject> {
        self.object(key)
    }
    fn actor_id(&self, key: ObjectKey) -> Option<ActorId> {
        self.id(key)
    }
    fn actor_key(&self, id: ActorId) -> Option<ObjectKey> {
        self.key(id)
    }
    fn read_placement(
        &self,
        key: ObjectKey,
        spatial: &SpatialWorld,
    ) -> actor_world::Result<PlacementState> {
        self.placement(key, spatial)
    }
}
impl ActorReader for ActorAccess<'_> {
    fn read_object(&self, key: ObjectKey) -> actor_world::Result<&RuntimeObject> {
        self.object(key)
    }
    fn actor_id(&self, key: ObjectKey) -> Option<ActorId> {
        self.id(key)
    }
    fn actor_key(&self, id: ActorId) -> Option<ObjectKey> {
        self.key(id)
    }
    fn read_placement(
        &self,
        key: ObjectKey,
        spatial: &SpatialWorld,
    ) -> actor_world::Result<PlacementState> {
        self.placement(key, spatial)
    }
}
pub struct SoftView<'a, A: ActorReader = ActorStore> {
    actors: &'a A,
    spatial: &'a SpatialWorld,
}
impl<A: ActorReader> SoftView<'_, A> {
    fn validated_handle(
        &self,
        actor: ObjectKey,
        kind: ShapeKind,
    ) -> aa_spatial::Result<ShapeHandle> {
        let key = self
            .spatial
            .key(actor)
            .map_err(|e| aa_spatial::Error::World(e.to_string()))?;
        let member = self.spatial.index.member(key)?;
        let runtime = self
            .actors
            .read_object(actor)
            .map_err(|e| aa_spatial::Error::World(e.to_string()))?;
        let source = self
            .spatial
            .source_geometry(member.actor.definition)
            .map_err(|e| aa_spatial::Error::World(e.to_string()))?;
        if runtime.kind != LoadKind::Actor
            || runtime.definition.native_class != 3
            || !runtime
                .definition
                .proof
                .id
                .eq_ignore_ascii_case(&source.source.id)
        {
            return Err(aa_spatial::Error::World(
                "published ordinary shape binding changed".into(),
            ));
        }
        let id = self
            .actors
            .actor_id(actor)
            .ok_or_else(|| aa_spatial::Error::World("ActorStore GTID no longer live".into()))?;
        if member.reference != id.raw()
            || self.spatial.index.resolve_reference(id.raw()) != Some(key)
        {
            return Err(aa_spatial::Error::World(
                "ActorStore/index identity mismatch".into(),
            ));
        }
        Ok(ShapeHandle {
            definition: member.actor.definition,
            kind,
        })
    }
}
pub struct TargetView<'a, A: ActorReader = ActorStore> {
    inner: SoftView<'a, A>,
}
impl<A: ActorReader> TargetView<'_, A> {
    fn object(&self, id: u32) -> aa_spatial::Result<ObjectKey> {
        self.inner
            .actors
            .actor_key(ActorId::from_raw(id))
            .ok_or_else(|| aa_spatial::Error::World("target-range Actor GTID is stale".into()))
    }
}
impl<A: ActorReader> aa_spatial::range::ActorRangeWorld for TargetView<'_, A> {
    type ShapeHandle = ShapeHandle;
    fn snapshot_transform(&mut self, actor: u32) -> aa_spatial::Result<ActorTransform> {
        soft_range::World::snapshot_transform(&self.inner, self.object(actor)?)
    }
    fn range_shape(&mut self, actor: u32) -> aa_spatial::Result<ShapeHandle> {
        self.inner
            .validated_handle(self.object(actor)?, ShapeKind::Target)
    }
    fn read_shape(&self, handle: &ShapeHandle) -> aa_spatial::Result<Shape> {
        soft_range::World::read_shape(&self.inner, handle)
    }
}
impl<A: ActorReader> soft_range::World for SoftView<'_, A> {
    type Actor = ObjectKey;
    type ShapeHandle = ShapeHandle;
    fn snapshot_transform(&self, actor: ObjectKey) -> aa_spatial::Result<ActorTransform> {
        let p = self
            .actors
            .read_placement(actor, self.spatial)
            .map_err(|e| aa_spatial::Error::World(e.to_string()))?;
        Ok(ActorTransform {
            position: [p.position_bits[0], p.position_bits[1]],
            facing: [p.facing_bits[0], p.facing_bits[1]],
        })
    }
    fn soft_shape(&mut self, actor: ObjectKey) -> aa_spatial::Result<ShapeHandle> {
        self.validated_handle(actor, ShapeKind::Soft)
    }
    fn read_shape(&self, handle: &ShapeHandle) -> aa_spatial::Result<Shape> {
        let source = &self
            .spatial
            .source_geometry(handle.definition)
            .map_err(|e| aa_spatial::Error::World(e.to_string()))?
            .source;
        Ok(shape(match handle.kind {
            ShapeKind::Blocking => &source.blocking,
            ShapeKind::Soft => &source.soft,
            ShapeKind::Target => {
                if source.has_explicit_target {
                    &source.target
                } else {
                    &source.soft
                }
            }
        }))
    }
}

/// Native 0x58-byte collision descriptor (`0x552DD0` / `0x554350`) as used by
/// `aa_navigation::result_plan::CollisionContext`, mapped onto the portable
/// `aa_spatial::query::PreparedQuery`. Word index = byte offset / 4:
///
/// | words | offset | meaning | PreparedQuery |
/// |---|---|---|---|
/// | 0..=5 | +0x00 | embedded blocking shape (kind, radius, bounds) | `placement.shape` |
/// | 6 | +0x18 | source definition | `descriptor.source_definition` |
/// | 7 | +0x1C | source actor (virtual +0x30 exclusion) | `descriptor.source_actor` |
/// | 8 | +0x20 | ignore actor | `descriptor.ignore_actor` |
/// | 9 | +0x24 | ignore definition | `descriptor.ignore_definition` |
/// | 10 | +0x28 | collision owner | `descriptor.collision_owner` |
/// | 11 | +0x2C | hostility context | `descriptor.hostility_context` |
/// | 12 | +0x30 / +0x31 bytes | ignore all / ignore moving (`0x4CEDE0`) | `ignore_all` / `ignore_moving` |
/// | 13 | +0x34 | custom filter function | `custom_filter` (nonzero) |
/// | 14 | +0x38 | required actor flags | `required_actor_flags` |
/// | 15 | +0x3C | tile mask | `descriptor.mask` |
/// | 16 | +0x40 | self pointer to the embedded shape | none (host word 0) |
/// | 17, 18 | +0x44 / +0x48 | facing (`0x5522B0`) | `placement.facing` |
/// | 19, 20 | +0x4C / +0x50 | position (`0x552250`) | `placement.position` |
/// | 21 | +0x54 | orientation cache valid (cleared by `0x5522B0`) | none: computed on use |
///
/// Offsets are read from the original `0x552340` filter and the `0x552DD0`
/// constructor. Actor-bearing words hold `aa_spatial` [`ActorKey`]s (0 = null)
/// and definition words hold definition ids, so `aa_navigation` `Object(u32)`
/// values are spatial actor keys in this adapter.
pub mod low_path_world {
    use aa_navigation::low_path::{self, ActorPose, ActorShape, LowPathWorld, SpatialHit};
    use aa_navigation::result_plan::{CollisionContext, Object};
    use aa_spatial::{
        actor_context,
        collision::{self, Descriptor, Shape},
        geometry::Placement,
        gworld_placement::{
            self as gp, CrossingWorld, DescriptorWorld, EndpointWorld, GMapView,
            NearestPlacementWorld, PlacementWorld,
        },
        index::{ActorKey, SpatialIndex},
        index_world::CollisionView,
        overlaps::{self, OverlapEntry},
        query::{self, Hit, PreparedQuery, QueryGeometry, QueryPolicy},
        sweep,
    };

    fn key(word: u32) -> Option<u32> {
        (word != 0).then_some(word)
    }
    fn world(e: impl std::fmt::Display) -> low_path::Error {
        low_path::Error::World(e.to_string())
    }

    /// Descriptor words to the portable query. Words 16 and 21 carry no
    /// portable state (self pointer, orientation cache).
    pub fn query_from_context(context: &CollisionContext) -> PreparedQuery {
        let w = &context.words;
        PreparedQuery {
            placement: Placement {
                shape: Shape {
                    kind: w[0],
                    radius_bits: w[1],
                    bounds: [w[2], w[3], w[4], w[5]],
                },
                facing: [w[17], w[18]],
                position: [w[19], w[20]],
            },
            descriptor: Descriptor {
                source_definition: key(w[6]),
                source_actor: key(w[7]),
                ignore_actor: key(w[8]),
                ignore_definition: key(w[9]),
                collision_owner: key(w[10]),
                hostility_context: key(w[11]),
                ignore_all: w[12] & 0xff != 0,
                ignore_moving: (w[12] >> 8) & 0xff != 0,
                custom_filter: w[13] != 0,
                mask: w[15],
            },
            required_actor_flags: w[14],
        }
    }

    /// The portable query as descriptor words (for `collision_context`
    /// providers built on `aa_spatial::actor_context::prepare`, 0x554350).
    /// Bytes +0x30/+0x31 are written as 0/1, word 13 as 1 when a custom
    /// filter is present, and words 16/21 as 0.
    pub fn context_from_query(query: &PreparedQuery) -> CollisionContext {
        let d = &query.descriptor;
        let s = &query.placement.shape;
        let mut w = [0u32; 22];
        w[0] = s.kind;
        w[1] = s.radius_bits;
        w[2..6].copy_from_slice(&s.bounds);
        w[6] = d.source_definition.unwrap_or(0);
        w[7] = d.source_actor.unwrap_or(0);
        w[8] = d.ignore_actor.unwrap_or(0);
        w[9] = d.ignore_definition.unwrap_or(0);
        w[10] = d.collision_owner.unwrap_or(0);
        w[11] = d.hostility_context.unwrap_or(0);
        w[12] = u32::from(d.ignore_all) | (u32::from(d.ignore_moving) << 8);
        w[13] = u32::from(d.custom_filter);
        w[14] = query.required_actor_flags;
        w[15] = d.mask;
        w[17] = query.placement.facing[0];
        w[18] = query.placement.facing[1];
        w[19] = query.placement.position[0];
        w[20] = query.placement.position[1];
        CollisionContext { words: w }
    }

    /// Actor and world services the spatial index does not hold. Each has a
    /// native meaning and no default.
    /// Leaves of 0x4F3C40 / 0x4F22C0 / 0x53CE40 / 0x4F46E0 that the spatial
    /// index does not hold (aa_spatial::gworld_placement). No defaults.
    pub trait PlacementHost {
        /// 0x4F2220(GWorld, point): the structure whose footprint holds it.
        fn structure_at(&mut self, point: [u32; 2]) -> gp::Result<Option<ActorKey>>;
        /// 0x4D36F0(actor, structure).
        fn actor_4d36f0(&mut self, actor: ActorKey, structure: ActorKey) -> gp::Result<bool>;
        /// Structure virtual +0x168(actor).
        fn structure_virtual_168(
            &mut self,
            structure: ActorKey,
            actor: ActorKey,
        ) -> gp::Result<bool>;
        /// 0x585280 on structure word +0x98 with the actor.
        fn footprint_585280(&mut self, structure: ActorKey, actor: ActorKey) -> gp::Result<bool>;
        /// Object virtual +0x100: an object pointer (None = null). The
        /// low-path goal displacement gates on the same virtual being nonzero.
        fn actor_virtual_100(&mut self, actor: ActorKey) -> gp::Result<Option<ActorKey>>;
        /// 0x585160 on structure word +0x98.
        fn footprint_585160(&mut self, structure: ActorKey) -> gp::Result<bool>;
        /// Actor virtual +0x164(structure).
        fn actor_virtual_164(&mut self, actor: ActorKey, structure: ActorKey) -> gp::Result<bool>;
        /// GMap tile word +0x10 (SAI region node; None = null).
        fn tile_region_node(&mut self, tile: usize) -> gp::Result<Option<u32>>;
        /// Region node word +8 (None = 0).
        fn region_node_region(&mut self, node: u32) -> gp::Result<Option<u32>>;
        /// Actor word +0x7C (0x4F46E0 replacement object; None = null).
        fn actor_word_7c(&mut self, actor: ActorKey) -> gp::Result<Option<u32>>;
        /// Word +0x40 of that object (the replacement actor).
        fn object_word_40(&mut self, object: u32) -> gp::Result<Option<ActorKey>>;
    }

    pub trait LowPathHost<P>: PlacementHost {
        /// Source actor virtual +0 (binary32, stored by 0x53AB90).
        fn source_virtual_0(&mut self, source: ActorKey) -> low_path::Result<u32>;
        /// Actor +0x110/+0x114 overlapping-thing list (head first).
        fn overlap_entries(&mut self, actor: ActorKey) -> low_path::Result<&mut Vec<OverlapEntry>>;
        /// GWorld+0xE0 simulation time read by 0x4D67F0.
        fn world_time_bits(&self) -> u32;
        /// 0x4D10A0 actor eligibility predicate of 0x4D67F0.
        fn overlap_eligible(
            &mut self,
            index: &mut SpatialIndex,
            policy: &mut P,
            actor: ActorKey,
        ) -> low_path::Result<bool>;
    }

    /// `aa_navigation::low_path::LowPathWorld` over the authoritative spatial
    /// index (GWorld+0x30 lists), its native query geometry and collision
    /// policy. Spatial services are the ported originals:
    /// 0x54F370 `CollisionView::blocked`, 0x552250
    /// `PreparedQuery::set_position`, 0x553BA0 `query::first_hit`, 0x553CE0
    /// `sweep::sweep`, 0x552340 `collision::excluded`, 0x4D67F0
    /// `overlaps::refresh_overlaps`; actor virtual +0xB4, word +0x90, pose
    /// and blocking shape (+0xC) come from the index member.
    pub struct SpatialLowPathWorld<'a, P, H> {
        pub index: &'a mut SpatialIndex,
        pub geometry: &'a QueryGeometry,
        pub policy: &'a mut P,
        pub host: &'a mut H,
    }

    impl<P: QueryPolicy, H: LowPathHost<P>> SpatialLowPathWorld<'_, P, H> {
        fn member(&self, actor: Object) -> low_path::Result<&aa_spatial::index::Member> {
            self.index.member(actor.0).map_err(world)
        }
    }

    impl<P: QueryPolicy, H: LowPathHost<P>> LowPathWorld for SpatialLowPathWorld<'_, P, H> {
        fn tile_blocked(
            &mut self,
            tile: [i32; 2],
            context: &CollisionContext,
        ) -> low_path::Result<bool> {
            let mut descriptor = query_from_context(context).descriptor;
            CollisionView {
                index: &mut *self.index,
                policy: &mut *self.policy,
            }
            .blocked(tile, &mut descriptor)
            .map_err(world)
        }
        fn set_context_position(
            &mut self,
            context: &mut CollisionContext,
            position: [u32; 2],
        ) -> low_path::Result<()> {
            let mut query = query_from_context(context);
            query
                .set_position(self.index, position, &*self.policy)
                .map_err(world)?;
            context.words[19] = query.placement.position[0];
            context.words[20] = query.placement.position[1];
            Ok(())
        }
        fn first_hit(&mut self, context: &CollisionContext) -> low_path::Result<SpatialHit> {
            let query = query_from_context(context);
            let hit =
                query::first_hit(self.index, self.geometry, &query, self.policy).map_err(world)?;
            Ok(SpatialHit {
                kind: hit.kind,
                reference: hit.reference,
            })
        }
        fn sweep(
            &mut self,
            context: &CollisionContext,
            goal: [u32; 2],
        ) -> low_path::Result<SpatialHit> {
            let mut query = query_from_context(context);
            let hit = sweep::sweep(self.index, self.geometry, &mut query, goal, self.policy)
                .map_err(world)?;
            Ok(SpatialHit {
                kind: hit.kind,
                reference: hit.reference,
            })
        }
        fn placement_result(
            &mut self,
            source: Option<Object>,
            context: &CollisionContext,
        ) -> low_path::Result<u32> {
            let query = query_from_context(context);
            let mut bridge = SpatialPlacement {
                index: &mut *self.index,
                geometry: self.geometry,
                policy: &mut *self.policy,
                host: &mut *self.host,
            };
            Ok(bridge
                .verdict(source.map(|o| o.0), &query)
                .map_err(world)?
                .code())
        }
        fn resolve_actor(&mut self, reference: u32) -> Option<Object> {
            self.index.resolve_reference(reference).map(Object)
        }
        fn actor_virtual_b4(&mut self, actor: Object) -> low_path::Result<bool> {
            Ok(self.member(actor)?.actor_category)
        }
        fn actor_word_90_present(&mut self, actor: Object) -> low_path::Result<bool> {
            Ok(self.member(actor)?.wall_present)
        }
        fn actor_virtual_100(&mut self, actor: Object) -> low_path::Result<u32> {
            Ok(self
                .host
                .actor_virtual_100(actor.0)
                .map_err(world)?
                .unwrap_or(0))
        }
        fn context_excludes(
            &mut self,
            context: &CollisionContext,
            actor: Object,
        ) -> low_path::Result<bool> {
            let mut descriptor = query_from_context(context).descriptor;
            collision::excluded(
                actor.0,
                &mut descriptor,
                &mut CollisionView {
                    index: &mut *self.index,
                    policy: &mut *self.policy,
                },
            )
            .map_err(world)
        }
        fn actor_pose(&mut self, actor: Object) -> low_path::Result<ActorPose> {
            let m = self.member(actor)?;
            Ok(ActorPose {
                position_bits: m.actor.position,
                facing_bits: m.actor.facing,
            })
        }
        fn source_virtual_0(&mut self, source: Object) -> low_path::Result<u32> {
            self.host.source_virtual_0(source.0)
        }
        fn actor_shape(&mut self, actor: Object) -> low_path::Result<ActorShape> {
            let s = &self.member(actor)?.blocking;
            Ok(ActorShape {
                kind: s.kind,
                word1_bits: s.radius_bits,
                word5_bits: s.bounds[3],
            })
        }
        fn refresh_overlaps(&mut self, source: Object) -> low_path::Result<()> {
            let now = self.host.world_time_bits();
            let mut entries = std::mem::take(self.host.overlap_entries(source.0)?);
            let host = &mut *self.host;
            let mut failure = None;
            let result = overlaps::refresh_overlaps(
                self.index,
                self.geometry,
                source.0,
                &mut entries,
                now,
                &mut |index, policy| match host.overlap_eligible(index, policy, source.0) {
                    Ok(v) => Ok(v),
                    Err(e) => {
                        let text = e.to_string();
                        failure = Some(e);
                        Err(aa_spatial::Error::World(text))
                    }
                },
                self.policy,
            );
            *self.host.overlap_entries(source.0)? = entries;
            if let Some(e) = failure {
                return Err(e);
            }
            result.map(|_| ()).map_err(world)
        }
    }

    /// GMap view of `[GWorld+0x30]` for 0x42A530 / 0x4F3C40 / 0x53CE40 /
    /// 0x4F46E0: tile counts, shift and cell size from the spatial grid, the
    /// +0xC/+0x10 extents from the native query geometry.
    pub fn gmap_view(index: &SpatialIndex, geometry: &QueryGeometry) -> GMapView {
        let grid = index.grid();
        GMapView {
            width: grid.dimensions[0],
            height: grid.dimensions[1],
            coordinate_shift: grid.coordinate_shift,
            cell_size_bits: grid.cell_size_bits,
            extent_bits: geometry.extent_bits,
        }
    }

    /// `aa_spatial::gworld_placement` worlds over the authoritative index:
    /// actor position / virtual +0xB4 / word +0x90 / facing from the index
    /// member, 0x553BA0 `query::first_hit`, 0x552250
    /// `PreparedQuery::set_position`, 0x554350 `actor_context::prepare`,
    /// tile byte +0x20 from the index tile flags, 0x54F370
    /// `CollisionView::blocked`; every other leaf from the [`PlacementHost`].
    /// Used for the low-path reachability test 0x53A6F0, and available to
    /// the region prepass host (`region::World::blocked_here`, 0x53DC00), the
    /// endpoint host (`endpoint::World::acceptable` = [`Self::endpoint`],
    /// 0x53CE40) and aa-mind's `world_4f46e0` ([`Self::nearest`], 0x4F46E0).
    pub struct SpatialPlacement<'a, P, H> {
        pub index: &'a mut SpatialIndex,
        pub geometry: &'a QueryGeometry,
        pub policy: &'a mut P,
        pub host: &'a mut H,
    }

    impl<P: QueryPolicy, H: PlacementHost> SpatialPlacement<'_, P, H> {
        fn view(&self) -> GMapView {
            gmap_view(self.index, self.geometry)
        }
        fn member(&self, actor: ActorKey) -> gp::Result<&aa_spatial::index::Member> {
            Ok(self.index.member(actor)?)
        }
        fn cell(&self, tile: usize) -> gp::Result<[i32; 2]> {
            let width = self.index.grid().dimensions[0];
            let tile = i32::try_from(tile).map_err(|_| gp::Error::World(format!("tile {tile}")))?;
            Ok([tile % width, tile / width])
        }
        /// Complete 0x4F3C40.
        pub fn verdict(
            &mut self,
            source: Option<ActorKey>,
            query: &PreparedQuery,
        ) -> gp::Result<gp::Verdict> {
            let view = self.view();
            gp::placement_result(&view, source, query, self)
        }
        /// Complete 0x53CE40 (`endpoint::World::acceptable`).
        pub fn endpoint(
            &mut self,
            point: [u32; 2],
            descriptor: &mut PreparedQuery,
        ) -> gp::Result<bool> {
            let view = self.view();
            gp::endpoint_acceptable(&view, point, descriptor, self)
        }
        /// Complete 0x4F46E0 (aa-mind `world_4f46e0`).
        pub fn nearest(
            &mut self,
            actor: Option<ActorKey>,
            point: &mut [u32; 2],
            radius_bits: u32,
        ) -> gp::Result<bool> {
            let view = self.view();
            let p = self.index.precision();
            gp::nearest_placement(&view, actor, point, radius_bits, p, self)
        }
    }

    impl<P: QueryPolicy, H: PlacementHost> CrossingWorld for SpatialPlacement<'_, P, H> {
        fn actor_virtual_b4(&mut self, actor: ActorKey) -> gp::Result<bool> {
            Ok(self.member(actor)?.actor_category)
        }
        fn structure_at(&mut self, point: [u32; 2]) -> gp::Result<Option<ActorKey>> {
            self.host.structure_at(point)
        }
        fn actor_4d36f0(&mut self, actor: ActorKey, structure: ActorKey) -> gp::Result<bool> {
            self.host.actor_4d36f0(actor, structure)
        }
        fn structure_virtual_168(
            &mut self,
            structure: ActorKey,
            actor: ActorKey,
        ) -> gp::Result<bool> {
            self.host.structure_virtual_168(structure, actor)
        }
        fn footprint_585280(&mut self, structure: ActorKey, actor: ActorKey) -> gp::Result<bool> {
            self.host.footprint_585280(structure, actor)
        }
        fn actor_word_90_present(&mut self, actor: ActorKey) -> gp::Result<bool> {
            Ok(self.member(actor)?.wall_present)
        }
        fn actor_virtual_100(&mut self, actor: ActorKey) -> gp::Result<Option<ActorKey>> {
            self.host.actor_virtual_100(actor)
        }
        fn footprint_585160(&mut self, structure: ActorKey) -> gp::Result<bool> {
            self.host.footprint_585160(structure)
        }
        fn actor_virtual_164(&mut self, actor: ActorKey, structure: ActorKey) -> gp::Result<bool> {
            self.host.actor_virtual_164(actor, structure)
        }
    }

    impl<P: QueryPolicy, H: PlacementHost> PlacementWorld for SpatialPlacement<'_, P, H> {
        fn actor_position(&mut self, actor: ActorKey) -> gp::Result<[u32; 2]> {
            Ok(self.member(actor)?.actor.position)
        }
        fn tile_region_node(&mut self, tile: usize) -> gp::Result<Option<u32>> {
            self.host.tile_region_node(tile)
        }
        fn region_node_region(&mut self, node: u32) -> gp::Result<Option<u32>> {
            self.host.region_node_region(node)
        }
        fn first_hit(&mut self, query: &PreparedQuery) -> gp::Result<Hit> {
            Ok(query::first_hit(
                self.index,
                self.geometry,
                query,
                self.policy,
            )?)
        }
    }

    impl<P: QueryPolicy, H: PlacementHost> DescriptorWorld for SpatialPlacement<'_, P, H> {
        fn set_descriptor_position(
            &mut self,
            descriptor: &mut PreparedQuery,
            point: [u32; 2],
        ) -> gp::Result<()> {
            Ok(descriptor.set_position(self.index, point, &*self.policy)?)
        }
        fn placement_code(
            &mut self,
            source: Option<ActorKey>,
            descriptor: &PreparedQuery,
        ) -> gp::Result<u32> {
            Ok(self.verdict(source, descriptor)?.code())
        }
    }

    impl<P: QueryPolicy, H: PlacementHost> EndpointWorld for SpatialPlacement<'_, P, H> {
        fn tile_byte_20(&mut self, tile: usize) -> gp::Result<u8> {
            let cell = self.cell(tile)?;
            Ok((self.index.tile(cell)?.flags20 & 0xff) as u8)
        }
        fn tile_blocked(
            &mut self,
            tile: usize,
            descriptor: &mut PreparedQuery,
        ) -> gp::Result<bool> {
            let cell = self.cell(tile)?;
            CollisionView {
                index: &mut *self.index,
                policy: &mut *self.policy,
            }
            .blocked(cell, &mut descriptor.descriptor)
            .map_err(|e| gp::Error::World(e.to_string()))
        }
    }

    impl<P: QueryPolicy, H: PlacementHost> NearestPlacementWorld for SpatialPlacement<'_, P, H> {
        fn actor_virtual_b4(&mut self, actor: ActorKey) -> gp::Result<bool> {
            Ok(self.member(actor)?.actor_category)
        }
        fn actor_word_7c(&mut self, actor: ActorKey) -> gp::Result<Option<u32>> {
            self.host.actor_word_7c(actor)
        }
        fn object_word_40(&mut self, object: u32) -> gp::Result<Option<ActorKey>> {
            self.host.object_word_40(object)
        }
        fn prepare_descriptor(&mut self, actor: ActorKey) -> gp::Result<PreparedQuery> {
            Ok(actor_context::prepare(
                self.index,
                actor,
                false,
                &*self.policy,
            )?)
        }
        fn actor_facing(&mut self, actor: ActorKey) -> gp::Result<[u32; 2]> {
            Ok(self.member(actor)?.actor.facing)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::Value;

        /// Every descriptor the original 0x554350 / 0x5522B0 produced in the
        /// phase7 context oracle (raw +0x18..+0x54 words decoded with the same
        /// offsets as this table) survives words -> query -> words unchanged.
        #[test]
        fn native_context_descriptors_round_trip_through_words() {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase7-spatial-context/native-context.json");
            let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            let mut checked = 0;
            for case in doc["cases"].as_array().unwrap() {
                for key in ["context", "after_facing"] {
                    let Some(v) = case["output"].get(key) else {
                        continue;
                    };
                    let query: PreparedQuery = serde_json::from_value(v.clone()).unwrap();
                    let words = context_from_query(&query);
                    let back = query_from_context(&words);
                    assert_eq!(serde_json::to_value(&back).unwrap(), *v);
                    assert_eq!(words.words[16], 0);
                    assert_eq!(words.words[21], 0);
                    checked += 1;
                }
            }
            assert!(checked >= 384, "{checked}");
        }

        #[test]
        fn gmap_view_reads_grid_and_native_extents() {
            let grid = aa_spatial::collision::Grid {
                dimensions: [8, 7],
                coordinate_shift: 1,
                cell_size_bits: 0x4000_0000,
            };
            let index =
                SpatialIndex::new(grid, vec![0; 56], aa_numeric::Precision::Double53).unwrap();
            let geometry = QueryGeometry::from_native_extents(
                &index,
                [0x4180_0000, 0x4160_0000],
                aa_numeric::Precision::Double53,
            )
            .unwrap();
            let view = gmap_view(&index, &geometry);
            assert_eq!((view.width, view.height, view.coordinate_shift), (8, 7, 1));
            assert_eq!(view.cell_size_bits, 0x4000_0000);
            assert_eq!(view.extent_bits, [0x4180_0000, 0x4160_0000]);
            // 0x42A530: (15.9, 13.9) -> cell (7, 6) -> 6 * 8 + 7.
            assert_eq!(view.tile_at_point([0x417e_6666, 0x415e_6666]).unwrap(), 55);
        }

        #[test]
        fn descriptor_bytes_follow_native_offsets() {
            let mut words = [0u32; 22];
            words[12] = 0x0100; // +0x31 ignore moving only
            words[13] = 7;
            words[7] = 5;
            let q = query_from_context(&CollisionContext { words });
            assert!(
                !q.descriptor.ignore_all
                    && q.descriptor.ignore_moving
                    && q.descriptor.custom_filter
            );
            assert_eq!(q.descriptor.source_actor, Some(5));
            assert_eq!(q.descriptor.source_definition, None);
            words[12] = 0x0001; // +0x30 ignore all only
            let q = query_from_context(&CollisionContext { words });
            assert!(q.descriptor.ignore_all && !q.descriptor.ignore_moving);
        }
    }
}

/// Spatial leaves of `aa_navigation::uai_collision::UaiCollisionWorld`
/// (0x534A40 / 0x536AD0) over the authoritative aa-spatial ports. A host's
/// `UaiCollisionWorld` implementation calls these; the actor, Mind and
/// organization virtuals stay host services.
pub mod uai_collision_world {
    use aa_navigation::low_path;
    use aa_navigation::uai_collision::{self, IgnoredThing};
    use aa_numeric::Precision;
    use aa_spatial::{
        actor_context,
        collision::Shape,
        geometry::Placement,
        index::{ActorKey, SpatialIndex},
        moving_state,
        overlaps::OverlapEntry,
        query::{QueryGeometry, QueryPolicy},
        range, shape_overlap, sweep,
    };

    fn world(e: impl std::fmt::Display) -> low_path::Error {
        low_path::Error::World(e.to_string())
    }

    /// A placed-shape descriptor as an aa-spatial placement. A null shape
    /// pointer is the native access violation of 0x46EF30 / 0x46F100.
    pub fn placement(d: &uai_collision::Descriptor) -> low_path::Result<Placement> {
        let s = d
            .shape
            .ok_or_else(|| world("null placed-shape pointer (native access violation)"))?;
        Ok(Placement {
            shape: Shape {
                kind: s.kind,
                radius_bits: s.radius_bits,
                bounds: s.bounds_bits,
            },
            facing: d.facing_bits,
            position: d.position_bits,
        })
    }

    /// 0x46EF30(this = a, b): `aa_spatial::shape_overlap::overlaps`.
    pub fn shapes_overlap(
        a: &uai_collision::Descriptor,
        b: &uai_collision::Descriptor,
        p: Precision,
    ) -> low_path::Result<bool> {
        shape_overlap::overlaps(&placement(a)?, &placement(b)?, p).map_err(world)
    }

    /// 0x46F100(this = a, range, b, inverted): `aa_spatial::range::shapes_in_range`.
    pub fn shapes_in_range(
        a: &uai_collision::Descriptor,
        range_bits: u32,
        b: &uai_collision::Descriptor,
        inverted: bool,
        p: Precision,
    ) -> low_path::Result<bool> {
        range::shapes_in_range(&placement(a)?, &placement(b)?, range_bits, inverted, p)
            .map_err(world)
    }

    /// GMap+0x30 tile shape used by 0x52F3E0 (`uai_collision::tile_descriptor`):
    /// the square built by the GMap constructor through 0x46EB60, which is
    /// `QueryGeometry::cell_shape` (`from_native_extents`).
    pub fn tile_shape(geometry: &QueryGeometry) -> uai_collision::Shape {
        let s = &geometry.cell_shape;
        uai_collision::Shape {
            kind: s.kind,
            radius_bits: s.radius_bits,
            bounds_bits: s.bounds,
        }
    }

    /// 0x531170 (`origin` None) and 0x5312C0 (`origin` Some): 0x554350
    /// non-strict context of the state's actor, 0x552250 at the actor
    /// position (0x531170) or `origin` (0x5312C0), 0x5522B0 facing =
    /// `direction`, descriptor +0x20 = `ignore` (the 0x4CE6E0 result for the
    /// generation-checked state target, resolved by the host; None when the
    /// target is stale), then 0x553CE0 to the goal computed as the original
    /// does (`actor_context::current_actor_goal` / `explicit_origin_goal`).
    /// Returns the four native result words (kind, reference, cell x, cell y).
    #[allow(clippy::too_many_arguments)]
    pub fn probe<P: QueryPolicy>(
        index: &mut SpatialIndex,
        geometry: &QueryGeometry,
        policy: &mut P,
        actor: ActorKey,
        origin: Option<[u32; 2]>,
        direction: [u32; 2],
        distance_bits: u32,
        ignore: Option<ActorKey>,
    ) -> low_path::Result<[u32; 4]> {
        let p = index.precision();
        let mut query = actor_context::prepare(index, actor, false, policy).map_err(world)?;
        let position = index.member(actor).map_err(world)?.actor.position;
        let (at, goal) = match origin {
            None => (
                position,
                actor_context::current_actor_goal(position, direction, distance_bits, p)
                    .map_err(world)?,
            ),
            Some(origin) => (
                origin,
                actor_context::explicit_origin_goal(origin, direction, distance_bits, p)
                    .map_err(world)?,
            ),
        };
        query.set_position(index, at, &*policy).map_err(world)?;
        actor_context::set_facing(&mut query, direction);
        query.descriptor.ignore_actor = ignore;
        let hit = sweep::sweep(index, geometry, &mut query, goal, policy).map_err(world)?;
        Ok([
            hit.kind,
            hit.reference,
            hit.cell[0] as u32,
            hit.cell[1] as u32,
        ])
    }

    /// 0x4D6940: append `{gtid, until}` at the tail of the actor's
    /// +0x110/+0x114 list (the same list 0x4D67F0 refreshes; head first).
    pub fn ignore_thing(
        entries: &mut Vec<OverlapEntry>,
        entry: IgnoredThing,
    ) -> low_path::Result<()> {
        entries.try_reserve(1)?;
        entries.push(OverlapEntry {
            reference: entry.gtid,
            expiry_bits: entry.until_bits,
        });
        Ok(())
    }

    /// 0x4CEDE0: `aa_spatial::moving_state::excluded` over the host's actor
    /// and component snapshots.
    pub fn actor_moving<W: moving_state::World>(
        actor: ActorKey,
        snapshot: &W,
    ) -> low_path::Result<bool> {
        moving_state::excluded(actor, snapshot).map_err(world)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use aa_spatial::collision::Grid;

        /// The GMap+0x30 tile shape from aa-spatial's 0x556570/0x46EB60 port
        /// equals aa-navigation's independent 0x46EB60 port.
        #[test]
        fn gmap_tile_shape_matches_square_shape() {
            for p in [
                Precision::Single24,
                Precision::Double53,
                Precision::Extended64,
            ] {
                for (cell, shift, extent) in [
                    (0x4000_0000u32, 1u32, 0x4100_0000u32),
                    (0x4080_0000, 2, 0x4180_0000),
                    (0x3f80_0000, 0, 0x4080_0000),
                ] {
                    let grid = Grid {
                        dimensions: [4, 4],
                        coordinate_shift: shift,
                        cell_size_bits: cell,
                    };
                    let index = SpatialIndex::new(grid, vec![0; 16], p).unwrap();
                    let geometry =
                        QueryGeometry::from_native_extents(&index, [extent, extent], p).unwrap();
                    assert_eq!(
                        tile_shape(&geometry),
                        uai_collision::square_shape(cell, p).unwrap()
                    );
                }
            }
        }

        #[test]
        fn ignored_things_append_at_the_tail() {
            let mut list = vec![OverlapEntry {
                reference: 1,
                expiry_bits: 0,
            }];
            ignore_thing(
                &mut list,
                IgnoredThing {
                    gtid: 9,
                    until_bits: 0x4000_0000,
                },
            )
            .unwrap();
            assert_eq!(
                list[1],
                OverlapEntry {
                    reference: 9,
                    expiry_bits: 0x4000_0000
                }
            );
        }
    }
}
