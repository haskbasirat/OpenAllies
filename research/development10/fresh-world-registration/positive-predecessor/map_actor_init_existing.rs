//! Fresh nonNULL4D7230 over the SAME split ActorStore/World/Bank owners.
//! No saved shell, copied pose, Init(NULL) or scratch world is constructed.
use crate::{
    actor_world::{self, ObjectKey, PlacementState, RuntimeComponent, Spatial},
    catalog::{ActorBindings, Catalog},
    map_replacement_queue::DefinitionRef,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    actor_creation::{ActorParams, Error, Result, Shape, TrackInfo},
    actor_init_existing::{self as native, Diagnostic},
    components::ComponentSlot as Slot,
    loading::LoadKind,
};
use aa_numeric::{Binary, Precision};
use std::sync::Arc;
#[path = "map_component_constant_leaf.rs"]
mod constant_leaf;
pub type Params = ActorParams<DefinitionRef, String>;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required(s: &'static str) -> Error {
    Error::Unsupported(s)
}
struct Source {
    binding: Arc<ActorBindings>,
    pc: Precision,
    height: u32,
    shape: Shape,
}
impl Source {
    fn bind(
        staged: &Staged,
        catalog: &Catalog<'_>,
        key: ObjectKey,
        definition: &DefinitionRef,
    ) -> Result<Self> {
        catalog.require_source_owner().map_err(error)?;
        let bank = &staged.definitions;
        let proof = bank.combat_source().map_err(error)?;
        proof.completed_means().map_err(error)?;
        if proof.source_identities() != catalog.actors.sources.as_slice() {
            return Err(required("fresh4D7230 actual source universe differs"));
        }
        let entry = bank.require_usable(definition.token).map_err(error)?;
        if entry.key.domain != "actordefinition"
            || !entry
                .key
                .name
                .eq_ignore_ascii_case(&definition.canonical_ids)
        {
            return Err(required("fresh4D7230 request/Bank token differs"));
        }
        let runtime = staged.actors.object(key).map_err(error)?;
        let pc = proof.source_precision();
        let binding = catalog
            .actor_bindings(&definition.canonical_ids, pc)
            .map_err(error)?;
        if staged.actors.id(key).is_none()
            || runtime.kind != LoadKind::Actor
            || runtime.actor.is_none()
            || runtime.object.is_none()
            || runtime.definition.native_class != 3
            || runtime.definition.proof.category.runtime_class != 3
            || runtime.definition.proof.category.native_kind != 2
            || !crate::load_spatial::same_proof(
                &runtime.thing.definition,
                &runtime.definition.proof,
            )
            || !crate::load_spatial::same_proof(
                &runtime.definition.proof,
                &binding.definition.proof,
            )
            || runtime.definition.flags != binding.definition.flags
            || runtime.definition.component_mask != binding.definition.component_mask
        {
            return Err(required(
                "fresh4D7230 actual allocation/source/factory differs",
            ));
        }
        crate::live_object_init::Source::bind_in_bank(catalog, bank, &runtime.definition)?;
        let geometry = catalog
            .actors
            .native_object_definition(&definition.canonical_ids, pc)
            .map_err(error)?;
        let w = geometry.blocking.words;
        Ok(Self {
            binding,
            pc,
            height: geometry.height.bits,
            shape: Shape {
                kind: w[0],
                radius_bits: w[1],
                extent_10_bits: w[4],
                extent_14_bits: w[5],
            },
        })
    }
}
struct ReadSpatial<'a>(&'a crate::load_world::World);
impl Spatial for ReadSpatial<'_> {
    fn placement(&self, id: ActorId) -> actor_world::Result<PlacementState> {
        match &self.0.spatial {
            crate::load_world::SpatialOwner::Adopted(s) => s.world.placement(id),
            _ => Err(actor_world::Error::Missing(
                "fresh placement requires same adopted index",
            )),
        }
    }
    fn publish(&mut self, _: ActorId, _: PlacementState) -> actor_world::Result<()> {
        Err(actor_world::Error::Missing(
            "readonly placement loan cannot publish",
        ))
    }
}
///The missing binding remains an error only if the native memory predicate
///actually reads Session state. This is a loan of the application owner.
struct MemoryPhase<'a>(Option<&'a dyn crate::load_world_globals::SessionPhase>);
impl crate::load_world_globals::SessionPhase for MemoryPhase<'_> {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        self.0
            .ok_or_else(|| "fresh memory requires actual application Session+F8 owner".to_string())?
            .current_phase()
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        self.0
            .ok_or_else(|| {
                "fresh memory requires actual application Session state owner".to_string()
            })?
            .application_state()
    }
}
struct Bound<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    key: ObjectKey,
    source: Source,
    pc: Precision,
}
impl Bound<'_, '_> {
    fn check(&self, a: ObjectKey) -> Result<()> {
        if a != self.key || self.staged.actors.id(a).is_none() {
            return Err(required(
                "fresh4D7230 callback current allocation/generation differs",
            ));
        }
        Ok(())
    }
    fn world(&self) -> Result<&crate::load_world::World> {
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or(required("fresh4D7230 actual World owner absent"))?;
        if world.terrain.source_precision() != self.source.pc
            || world.source_fields.source_precision() != self.source.pc
            || world.source_fields.receipt().sources != self.catalog.actors.sources
        {
            return Err(required(
                "fresh4D7230 World terrain/source-fields differ from actual Bank PC/universe",
            ));
        }
        match &world.spatial {
            crate::load_world::SpatialOwner::Adopted(s)
                if s.error.is_none() && s.pending.is_empty() && s.world.index().precision() == self.pc =>
            {
                ()
            }
            _ => {
                return Err(required(
                    "fresh4D7230 same adopted index/live precision required",
                ));
            }
        }
        Ok(world)
    }
    fn pose(&self, a: ObjectKey) -> Result<PlacementState> {
        self.check(a)?;
        self.staged
            .actors
            .placement(a, &ReadSpatial(self.world()?))
            .map_err(error)
    }
    fn id(&self, a: ObjectKey) -> Result<ActorId> {
        self.check(a)?;
        self.staged
            .actors
            .id(a)
            .ok_or(required("fresh4D7230 actual GTID retired"))
    }
    fn set_pose_word(&mut self, a: ObjectKey, offset: u32, value: u32) -> Result<()> {
        self.check(a)?;
        if let Ok(p) = self.staged.actors.staged_placement_mut(a) {
            match offset {
                0x20 | 0x24 | 0x28 => p.position_bits[((offset - 0x20) / 4) as usize] = value,
                0x44 | 0x48 => {
                    let tile = p
                        .cached_tile
                        .as_mut()
                        .ok_or(required("fresh actual constructor cached tile absent"))?;
                    tile[((offset - 0x44) / 4) as usize] = value as i32;
                }
                _ => return Err(required("fresh unsupported initialization pose word")),
            };
            return Ok(());
        }
        let w = self
            .staged
            .load_world
            .as_mut()
            .ok_or(required("fresh actual World owner absent"))?;
        match &mut w.spatial {
            crate::load_world::SpatialOwner::Adopted(s) => s
                .world
                .set_initialization_scalar(a, offset, value)
                .map_err(error),
            _ => Err(required("fresh published placement owner absent")),
        }
    }
    fn mask(&self) -> Result<u32> {
        self.source
            .binding
            .definition
            .component_mask
            .ok_or(required("fresh actual source component mask absent"))
    }
    fn source_bool(&self, name: &str) -> Result<bool> {
        self.catalog
            .actors
            .native_field(
                &self.source.binding.definition.proof.id,
                name,
                self.source.pc,
            )
            .map_err(error)?
            .ok_or(required("fresh actual typed source Boolean absent"))?
            .boolean()
            .map_err(error)
    }
    fn require_slot(&self, a: ObjectKey, s: Slot) -> Result<&RuntimeComponent> {
        self.check(a)?;
        if self.mask()? & (1 << s.index()) == 0 {
            return Err(required("fresh component outside actual source factory"));
        }
        let value = self
            .staged
            .actors
            .object(a)
            .map_err(error)?
            .components()
            .map_err(error)?
            .get(s)
            .ok_or(required("fresh actual typed component absent"))?;
        if value.slot() != s {
            return Err(required("fresh component payload differs"));
        }
        Ok(value)
    }
}
/// Invoked after the SAME actor's real4D6F10. Reached failures preserve earlier
/// native fields, parameter writebacks and authoritative index mutations.
pub fn initialize(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    params: &mut Params,
    pc: Precision,
) -> Result<bool> {
    let source = Source::bind(staged, catalog, key, &params.definition)?;
    let mut bound = Bound {
        staged,
        catalog,
        key,
        source,
        pc,
    };
    bound.world()?; //Explicit portable owner/PC preflight, before native writes.
    native::initialize(key, params, &mut bound, pc)
}
///4F9FC7 uses42A590 with the actual ObjectData+BC list, byte58=0.
///The null-list certificate is the same inherited source projection used by
///4D7556 below, whose distinct list is ObjectData+E4 (WhenConstructed).
pub(crate) fn publish_created_fx(staged:&Staged,catalog:&Catalog<'_>,key:ObjectKey)->Result<()> {
    let object=staged.actors.object(key).map_err(error)?;
    if object.definition.native_class!=3 || object.actor.is_none() || staged.actors.id(key).is_none() {
        return Err(required("fresh42A590 actual ordinary Actor owner absent"));
    }
    if certified_null_fx(staged,catalog,key,crate::definition_fx::ListKind::Created)? { return Ok(()) }
    Err(required("fresh42A590 positive Created FX/interleaved execution owner required"))
}
fn certified_null_fx(staged:&Staged,catalog:&Catalog<'_>,key:ObjectKey,kind:crate::definition_fx::ListKind)->Result<bool> {
    let definition=&staged.actors.object(key).map_err(error)?.definition;
    let source=crate::definition_fx::Source::bind_in_bank(catalog,&staged.definitions,definition).map_err(error)?;
    if !source.list(kind).is_certified_empty() { return Ok(false) }
    let world=staged.load_world.as_ref().ok_or(required("fresh42A590 actual position World absent"))?;
    //42A590 copies the current XYZ twice before429080's null-list return.
    //These reads use the sole staged/published placement owner each time.
    for _ in 0..2 {
        let _position=staged.actors.placement(key,world.live_spatial().map_err(error)?).map_err(error)?.position_bits;
    }
    Ok(true)
}
impl native::World<DefinitionRef, String> for Bound<'_, '_> {
    type Actor = ObjectKey;
    fn word(&self, a: ObjectKey, o: u32) -> Result<u32> {
        self.check(a)?;
        let r = self.staged.actors.object(a).map_err(error)?;
        match o {
            8 => Ok(self.pose(a)?.flags),
            0x20 | 0x24 | 0x28 => Ok(self.pose(a)?.position_bits[((o - 0x20) / 4) as usize]),
            0x2c | 0x30 => Ok(self.pose(a)?.facing_bits[((o - 0x2c) / 4) as usize]),
            0x44 | 0x48 => Ok(self
                .pose(a)?
                .cached_tile
                .ok_or(required("fresh actual cached tile absent"))?[((o - 0x44) / 4) as usize]
                as u32),
            0x38 => Ok(r
                .object
                .as_ref()
                .ok_or(required("fresh Object fields absent"))?
                .associated_handle),
            0x3c => Ok(r
                .object
                .as_ref()
                .ok_or(required("fresh Object fields absent"))?
                .scale_bits),
            0x50 => Ok(r
                .object
                .as_ref()
                .ok_or(required("fresh Object fields absent"))?
                .words_50_to_54[0]),
            _ => r
                .actor
                .as_ref()
                .and_then(|a| a.words.get(&o))
                .copied()
                .ok_or_else(|| error(format!("fresh current Actor+{o:x} absent"))),
        }
    }
    fn set_word(&mut self, a: ObjectKey, o: u32, v: u32) -> Result<()> {
        self.check(a)?;
        if matches!(o, 0x20 | 0x24 | 0x28 | 0x44 | 0x48) {
            return self.set_pose_word(a, o, v);
        }
        let r = self.staged.actors.object_mut(a).map_err(error)?;
        match o {
            0x38 => {
                r.object
                    .as_mut()
                    .ok_or(required("fresh Object fields absent"))?
                    .associated_handle = v
            }
            0x138 | 0x158 => {
                let slot = r
                    .actor
                    .as_mut()
                    .and_then(|a| a.words.get_mut(&o))
                    .ok_or(required("fresh native initialized Actor word absent"))?;
                *slot = v;
            }
            _ => return Err(required("fresh unsupported initialized scalar write")),
        };
        Ok(())
    }
    fn clear_byte_4c(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        self.staged
            .actors
            .object_mut(a)
            .map_err(error)?
            .object
            .as_mut()
            .ok_or(required("fresh Object fields absent"))?
            .byte_4c = 0;
        Ok(())
    }
    fn definition_height_bits(&self, a: ObjectKey) -> Result<u32> {
        self.check(a)?;
        Ok(self.source.height)
    }
    fn shape_a(&self, a: ObjectKey) -> Result<Shape> {
        self.check(a)?;
        Ok(self.source.shape)
    }
    fn tile_size_bits(&self) -> Result<u32> {
        Ok(self.world()?.terrain.grid.cell_size)
    }
    fn tile_shift(&self) -> Result<u32> {
        Ok(self.world()?.terrain.grid.coordinate_shift)
    }
    fn diagnostic(&mut self, _: ObjectKey, _: Diagnostic) -> Result<()> {
        Err(required("fresh4D7230 actual runtime diagnostic/UI sink"))
    }
    fn snap(
        &mut self,
        a: ObjectKey,
        _shape: &Shape,
        mut xy: [u32; 2],
        tile: u32,
    ) -> Result<[u32; 2]> {
        self.check(a)?;
        aa_spatial::snap_half_cell(&mut xy, tile, self.pc).map_err(error)?;
        Ok(xy)
    }
    fn placement_test(&mut self, a: ObjectKey, params: &Params) -> Result<u32> {
        self.check(a)?;
        self.world()?;
        crate::map_actor_placement::check(self.staged, self.catalog, a, params, self.pc)
            .map_err(error)
    }
    fn map_mode(&self) -> Result<u32> {
        self.world()?
            .map_mode
            .as_ref()
            .ok_or(required("fresh actual MapResource mode absent"))
    }
    fn spatial_membership(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        let s = &mut *self.staged;
        let w = s
            .load_world
            .as_mut()
            .ok_or(required("fresh actual World owner absent"))?;
        let crate::load_world::SpatialOwner::Adopted(spatial) = &mut w.spatial else {
            return Err(required(
                "fresh native membership requires adopted actual index",
            ));
        };
        if s.actors.staged_placement(a).is_ok() {
            spatial
                .world
                .prepare_actor_bound(&s.actors, self.catalog, &s.definitions, a)
                .map_err(error)?;
            s.actors
                .publish_placement(a, &mut spatial.world)
                .map_err(error)?;
        }
        spatial.world.refresh_membership(a).map_err(error)
    }
    fn terrain_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary> {
        self.world()?
            .terrain
            .surface()
            .map_err(error)?
            .sample(xy, flags, self.pc)
            .map_err(error)
    }
    fn blocking_notify(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        self.world()?;
        crate::map_actor_overlaps::notify(self.staged, self.catalog, a, self.pc)
    }
    fn tile_footprint(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        self.world()?;
        crate::map_actor_footprint::refresh(self.staged, self.catalog, a, self.pc)
    }
    fn world_list_insert(&mut self, a: ObjectKey) -> Result<()> {
        let flags = self.pose(a)?.flags;
        let id = self.id(a)?;
        self.staged
            .load_world
            .as_mut()
            .ok_or(required("fresh actual World owner absent"))?
            .objects
            .insert(id, flags)
            .map_err(error)?;
        Ok(())
    }
    fn memory_refresh(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        if self
            .staged
            .actors
            .object(a)
            .map_err(error)?
            .object
            .as_ref()
            .ok_or(required("fresh Object storage absent"))?
            .memory
            .count()
            > 0
        {
            return Err(required(
                "fresh4EB490/4EBB80 real positive memory allocation services",
            ));
        }
        Ok(())
    }
    fn visibility_refresh(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        self.world()?;
        let id = self.id(a)?;
        let s = &mut *self.staged;
        let world = s
            .load_world
            .as_ref()
            .ok_or(required("fresh visibility actual World absent"))?;
        let spatial = world.live_spatial().map_err(error)?;
        let phase = MemoryPhase(self.catalog.load_session);
        let services = crate::load_zone_services::Context::with_kingdoms(
            &world.point,
            &phase,
            &world.kingdoms,
        )
        .with_source_fields(self.catalog, &world.source_fields);
        crate::load_store_fog::refresh_memory_published(
            self.catalog,
            &s.definitions,
            &mut s.actors,
            spatial,
            &services,
            &world.kingdoms,
            id,
        )
    }
    fn required_object_enabled(&self, a: ObjectKey) -> Result<bool> {
        self.check(a)?;
        self.world()?
            .source_fields
            .reference(&self.source.binding.definition.proof.id, 0x4e0)
            .map(|x| x.is_some())
            .map_err(error)
    }
    fn find_required_object(&mut self, _: ObjectKey, _: [u32; 2]) -> Result<Option<u32>> {
        Err(required("fresh555830 actual required-object query"))
    }
    fn gtid_live(&self, id: u32) -> Result<bool> {
        Ok(self.staged.actors.key(ActorId::from_raw(id)).is_some())
    }
    fn required_placement_blocked(&mut self, _: u32, _: ObjectKey) -> Result<bool> {
        Err(required("fresh46B000 required-object blocking query"))
    }
    fn required_object_attach(&mut self, _: u32, _: ObjectKey) -> Result<()> {
        Err(required(
            "fresh actual required-object virtual128 attachment",
        ))
    }
    fn has_component(&self, a: ObjectKey, s: Slot) -> Result<bool> {
        self.check(a)?;
        Ok(self
            .staged
            .actors
            .object(a)
            .map_err(error)?
            .components()
            .map_err(error)?
            .present(s))
    }
    fn component_init_existing(&mut self, a: ObjectKey, s: Slot) -> Result<bool> {
        if crate::load_detection_init_world::supports(s){
            self.check(a)?;
            return crate::stored_detection_init_world::init_existing(self.catalog,self.staged,a,s,self.pc)
        }
        if s==Slot::Mind{
            self.check(a)?;
            return crate::stored_mind_init_world::init_existing(self.catalog,self.staged,a,self.pc).map_err(error)
        }
        if s==Slot::Mover{
            self.check(a)?;
            return crate::stored_mover_init_world::init_existing(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::Element{
            self.check(a)?;
            return crate::stored_element_init_world::init_existing(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::Body{
            self.check(a)?;
            return crate::stored_body_init_world::init_existing(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::Performer{
            self.check(a)?;
            let world=self.staged.load_world.as_ref().ok_or(required("fresh57E000 actual World/Spatial owner"))?;
            return crate::stored_performer_init_existing::init_existing_with_variables(self.catalog,&mut self.staged.actors,
                &mut self.staged.definitions,a,&self.staged.inventory,world.live_spatial().map_err(error)?,Some(&world.variables),self.pc).map_err(error)
        }
        if s==Slot::Economy{
            self.check(a)?;
            return crate::stored_economy_init_world::init_existing(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::Supply{
            self.check(a)?;
            return crate::stored_supply_init_world::init_existing(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::Recruit{
            self.check(a)?;
            return crate::stored_recruit_mobile_init_world::recruit(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::MobileDeployment{
            self.check(a)?;
            return crate::stored_recruit_mobile_init_world::mobile_deployment(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::Denizen{
            self.check(a)?;
            return crate::stored_denizen_init_world::init_existing(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::Defender{
            self.check(a)?;
            return crate::stored_defender_init_world::init_existing(self.catalog,self.staged,a,self.pc)
        }
        let id = self.id(a)?;
        let leaf = constant_leaf::checked(self.require_slot(a,s)?,id,s,self.mask()?).map_err(error)?;
        //6A71D0 is MOV AL,1; RET and reads no component/world fields.
        if leaf.init_existing == 0x6a71d0 {
            return Ok(true);
        }
        Err(error(format!(
            "fresh actual component{s:?} InitExisting{:x} owner is required",
            leaf.init_existing
        )))
    }
    fn component_after_init(&mut self, a: ObjectKey, s: Slot) -> Result<()> {
        if s==Slot::Detection{
            self.check(a)?;
            return crate::stored_detection_init_world::after_init(self.catalog,self.staged,a,self.pc)
        }
        if s==Slot::Mind{
            self.check(a)?;
            return crate::stored_mind_init_world::after_init(self.catalog,&self.staged.actors,&self.staged.definitions,a).map_err(error)
        }
        if s==Slot::Mover{
            self.check(a)?;
            return crate::stored_mover_init_world::after_init(self.catalog,&self.staged.actors,&self.staged.definitions,a)
        }
        if s==Slot::Element{
            self.check(a)?;
            return crate::stored_element_init_world::after_init(self.catalog,&self.staged.actors,&self.staged.definitions,a)
        }
        if s==Slot::Body{
            self.check(a)?;
            return crate::stored_body_init_world::after_init(self.catalog,&self.staged.actors,&self.staged.definitions,a)
        }
        if s==Slot::Performer{
            self.check(a)?;
            return crate::stored_performer_init_existing::after_init(self.catalog,&self.staged.actors,&self.staged.definitions,a).map_err(error)
        }
        if s==Slot::Denizen{
            self.check(a)?;
            return crate::stored_denizen_init_world::after_init(self.catalog,self.staged,a,self.pc)
        }
        let id = self.id(a)?;
        let leaf = constant_leaf::checked(self.require_slot(a,s)?,id,s,self.mask()?).map_err(error)?;
        //4B2830 is a bare RET for this actual constructed class. Nontrivial
        //Denizen/Detection/City and other +14 children keep their required gate.
        if leaf.vf14 == 0x4b2830 {
            return Ok(());
        }
        Err(error(format!(
            "fresh actual component{s:?} AfterInit{:x} owner is required",
            leaf.vf14
        )))
    }
    fn global_property(&self) -> Result<Option<u32>> {
        crate::load_property_instances::global_in_bank(self.catalog,&self.staged.definitions).map_err(error)
    }
    fn property_applies(&mut self, a: ObjectKey, property: u32) -> Result<bool> {
        self.check(a)?;
        let reference=aa_rules::native::layout_data::PropertyRef::from_handle(property).ok_or(required("fresh5035E0 actual global Property reference differs"))?;
        crate::staged_property_admission::applicable_on_owners(self.catalog,&mut self.staged.actors,&mut self.staged.definitions,a,reference).map_err(error)
    }
    fn properties_add(&mut self, a: ObjectKey, property: u32) -> Result<()> {
        let source=self.id(a)?;
        let reference=aa_rules::native::layout_data::PropertyRef::from_handle(property).ok_or(required("fresh5817A0 actual global Property reference differs"))?;
        //Actual4D7377..37B pushes current Actor,kind0,Property.
        crate::staged_property_instances::add(self.staged,self.catalog,a,reference,0,Some(source),self.pc).map_err(error)
    }
    fn needs_settlement_zone(&self, a: ObjectKey) -> Result<bool> {
        self.check(a)?;
        self.source_bool("needs_space_for_settlement_zone")
    }
    fn create_zone2(&mut self, a: ObjectKey) -> Result<bool> {
        //4D73F3's4D69A0(2) uses the same Actor zones and native identity
        //allocator as the component-created zones. No replacement Actor is
        //detached to perform a callback on the currently initializing owner.
        self.check(a)?;
        let owner = self.id(a)?;
        let world = self.staged.load_world.as_mut()
            .ok_or(required("fresh4D69A0 actual persistent ZoneMap World absent"))?;
        let flags = self.staged.actors.placement(a, world.live_spatial().map_err(error)?)
            .map_err(error)?.flags;
        let actor = self.staged.actors.object_mut(a).map_err(error)?.actor.as_mut()
            .ok_or(Error::NullDereference("fresh4D69A0 actual Actor"))?;
        crate::load_detection_activation::create_slot_fields(
            owner, flags, actor.zones_148.zones_mut(), &mut world.zone_identities, 2,
        ).map_err(error)
    }
    fn discovery_radius(&self, pc: Precision) -> Result<u32> {
        let maximum = self
            .staged
            .definitions
            .combat_source()
            .map_err(error)?
            .completed_maximum_74()
            .map_err(error)?
            .0;
        aa_actors::component_virtuals::discovery_zone_radius(self.tile_size_bits()?, maximum, pc)
            .map_err(error)
    }
    fn zone_count(&self, a: ObjectKey) -> Result<i32> {
        self.check(a)?;
        i32::try_from(
            self.staged
                .actors
                .object(a)
                .map_err(error)?
                .actor
                .as_ref()
                .ok_or(required("fresh Actor fields absent"))?
                .zones_148
                .len(),
        )
        .map_err(error)
    }
    fn move_zone2(&mut self, a: ObjectKey, position: [u32; 2], radius: u32) -> Result<()> {
        //4D7435 enters515F40 only after the real discovery radius/position
        //reads and the parent null-zone guard. All partial activation writes
        //remain on this same stored zone if a later child refuses execution.
        self.check(a)?;
        let staged = &mut *self.staged;
        let world = staged.load_world.as_mut()
            .ok_or(required("fresh515F40 actual persistent ZoneMap World absent"))?;
        crate::stored_zone_activation::operate_slot(
            self.catalog, &staged.definitions, &mut staged.actors, world, a, 2,
            Some((position, radius)), staged.inactive_load.as_ref(), self.pc,
        ).map_err(error)
    }
    fn sai_present(&self) -> Result<bool> {
        self.staged
            .inactive_load
            .as_ref()
            .ok_or(required("fresh actual SAI lifetime interval absent"))?
            .observer_present()
            .map_err(error)
    }
    fn sai_notify(&mut self, _: ObjectKey) -> Result<()> {
        let scope = self
            .staged
            .inactive_load
            .as_ref()
            .ok_or(required("fresh actual SAI lifetime interval absent"))?;
        if !scope.observer_active().map_err(error)? {
            return Ok(());
        }
        Err(required("fresh7A0C20 current SAI notification owner"))
    }
    fn tracks(&self, a: ObjectKey) -> Result<Vec<TrackInfo>> {
        self.check(a)?;
        let node = self
            .catalog
            .actors
            .resolved_actor(&self.source.binding.definition.proof.id)
            .map_err(error)?;
        let Some(mover) = node.child("MoverComponent").map_err(error)? else {
            return Ok(vec![]);
        };
        let mut out = vec![];
        for track in mover.children("TrackInfo").map_err(error)? {
            let kind = match track.field("track_type", self.source.pc).map_err(error)? {
                Some(aa_rules::native::TypedValue::String { value, .. })
                    if value.eq_ignore_ascii_case("CONTINUOUS") =>
                {
                    1
                }
                Some(aa_rules::native::TypedValue::String { value, .. })
                    if value.eq_ignore_ascii_case("DISCRETE") =>
                {
                    0
                }
                _ => return Err(required("fresh TrackInfo actual type absent/unsupported")),
            };
            let length = match track.field("fade_sec", self.source.pc).map_err(error)? {
                Some(aa_rules::native::TypedValue::Scalar(s)) => s.bits,
                _ => return Err(required("fresh TrackInfo fade_sec scalar absent")),
            };
            out.push(TrackInfo {
                kind,
                length_bits: length,
            });
        }
        Ok(out)
    }
    fn track_settings(&self) -> Result<(u32, i32)> {
        Err(required(
            "fresh actual TrackInfo source poll-period settings",
        ))
    }
    fn track_updatable(&mut self, _: ObjectKey, _: i32) -> Result<()> {
        Err(required(
            "fresh50AB10 actual TrackInfo allocation/queue owner",
        ))
    }
    fn state_low(&self, a: ObjectKey) -> Result<u32> {
        self.check(a)?;
        Ok(self
            .staged
            .actors
            .object(a)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state
            .low)
    }
    fn existing_fx(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        if certified_null_fx(self.staged,self.catalog,a,crate::definition_fx::ListKind::Constructed)? { return Ok(()) }
        Err(required(
            "fresh42A590 actual Existing FX/interleaved worker owner",
        ))
    }
    fn refresh_4d4cf0(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        crate::stored_actor_properties::refresh(self.catalog,&mut self.staged.actors,&self.staged.definitions,a).map_err(error)
    }
    fn refresh_4d1350(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        let world=self.staged.load_world.as_ref().ok_or(required("fresh4D1350 actual World/Spatial owner"))?;
        crate::stored_actor_actions::rebuild_with_variables(self.catalog,&mut self.staged.actors,&mut self.staged.definitions,
            a,&self.staged.inventory,world.live_spatial().map_err(error)?,Some(&world.variables),self.pc).map_err(error)
    }
    fn refresh_4d67f0(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        self.world()?;
        crate::map_actor_overlaps::refresh(self.staged, self.catalog, a, self.pc).map(|_| ())
    }
    fn refresh_4d4130(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        self.world()?;
        crate::stored_actor_terrain_refresh::refresh(self.catalog,self.staged,a,self.pc)
    }
    fn organization_refresh(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        crate::stored_actor_value_refresh::refresh_organization(self.catalog,&mut self.staged.actors,&self.staged.definitions,a,self.pc)
    }
    fn settlement_refresh(&mut self, _: ObjectKey) -> Result<()> {
        Err(required("fresh585650 actual Settlement values owner"))
    }
    fn performer_refresh(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        let world=self.staged.load_world.as_mut().ok_or(required("fresh57E430 actual World SVar owner"))?;
        let spatial=match &world.spatial{
            crate::load_world::SpatialOwner::Adopted(adoption)if adoption.error.is_none()&&adoption.pending.is_empty()=>Some(&adoption.world),
            _=>None,
        };
        fn required_environment()->std::result::Result<aa_numeric::crt_pow::CrtEnvironment,String>{Err("fresh57E430 actual current-thread CRT sampler required".into())}
        crate::stored_performer_refresh::refresh_value_on_owners(self.catalog,&mut self.staged.actors,
            &mut self.staged.definitions,spatial,a,&mut world.variables,&self.staged.inventory,
            self.catalog.runtime_environment.unwrap_or(required_environment),self.pc).map_err(error)
    }
    fn denizen_refresh(&mut self, a: ObjectKey) -> Result<()> {
        self.check(a)?;
        crate::stored_actor_value_refresh::refresh_denizen(self.catalog,&mut self.staged.actors,&self.staged.definitions,a,self.pc)
    }
}
