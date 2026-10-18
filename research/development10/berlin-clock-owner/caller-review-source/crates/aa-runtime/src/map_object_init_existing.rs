//! Fresh GObject4EBE50 over its actual Object-only arena storage.
//! The numerical body is shared with Actor Init, while world services borrow
//! the SAME moved Bank, adopted index, lists and visibility owners.
use crate::{actor_world::{self,ObjectKey,PlacementState,Spatial},catalog::Catalog,
    map_actor_creation::Params,map_replacement_queue::DefinitionRef,restoration::Staged};
use aa_actors::{ActorId,actor_creation::{Error,Result,Shape},actor_init_existing::{self as native,Diagnostic}};
use aa_numeric::{Binary,Precision};
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
fn required(s:&'static str)->Error{Error::Unsupported(s)}
struct Source{pc:Precision,height:u32,shape:Shape}
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
struct Bound<'a,'s>{staged:&'a mut Staged,catalog:&'a Catalog<'s>,key:ObjectKey,source:Source,pc:Precision}
impl Bound<'_,'_>{
    fn check(&self,a:ObjectKey)->Result<()> {
        if a!=self.key||self.staged.actors.id(a).is_none(){return Err(required("fresh Object callback generation differs"))}
        let r=self.staged.actors.object(a).map_err(error)?;
        if !matches!(r.definition.native_class,2|9)||r.kind!=aa_actors::loading::LoadKind::Object||r.actor.is_some()||r.object.is_none()
            ||!matches!(&self.staged.actors.entity(a).map_err(error)?.source,actor_world::Source::NewObject{..}) {
            return Err(required("fresh Object actual source/storage differs"));
        }
        crate::live_object_init::Source::bind_in_bank(self.catalog,&self.staged.definitions,&r.definition)?;Ok(())
    }
    fn world(&self)->Result<&crate::load_world::World>{
        let w=self.staged.load_world.as_ref().ok_or(required("fresh Object actual World absent"))?;
        if w.terrain.source_precision()!=self.source.pc||w.source_fields.source_precision()!=self.source.pc||w.source_fields.receipt().sources!=self.catalog.actors.sources {
            return Err(required("fresh Object World/source precision differs"));
        }
        match &w.spatial {crate::load_world::SpatialOwner::Adopted(s)if s.error.is_none()&&s.pending.is_empty()&&s.world.index().precision()==self.pc=>{},
            _=>return Err(required("fresh Object actual spatial adoption/live precision differs"))};Ok(w)
    }
    fn pose(&self,a:ObjectKey)->Result<PlacementState>{self.check(a)?;self.staged.actors.placement(a,&ReadSpatial(self.world()?)).map_err(error)}
    fn id(&self,a:ObjectKey)->Result<ActorId>{self.check(a)?;self.staged.actors.id(a).ok_or(required("fresh Object GTID retired"))}
    fn set_pose_word(&mut self,a:ObjectKey,o:u32,v:u32)->Result<()> {
        self.check(a)?;
        if let Ok(p)=self.staged.actors.staged_placement_mut(a){
            match o {0x20|0x24|0x28=>p.position_bits[((o-0x20)/4)as usize]=v,
                0x44|0x48=>p.cached_tile.as_mut().ok_or(required("fresh Object constructor tile absent"))?[((o-0x44)/4)as usize]=v as i32,
                _=>return Err(required("fresh Object unsupported pose store"))};return Ok(())
        }
        let w=self.staged.load_world.as_mut().ok_or(required("fresh Object World absent"))?;
        match &mut w.spatial {crate::load_world::SpatialOwner::Adopted(s)=>s.world.set_initialization_scalar(a,o,v).map_err(error),
            _=>Err(required("fresh Object published spatial owner absent"))}
    }
}
pub(crate) fn initialize(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,params:&mut Params,pc:Precision)->Result<bool>{
    let r=staged.actors.object(key).map_err(error)?;
    let source=crate::live_object_init::Source::bind_in_bank(catalog,&staged.definitions,&r.definition)?;
    let geometry=catalog.actors.native_object_definition(&r.definition.proof.id,source.source_precision()).map_err(error)?;
    let w=geometry.blocking.words;
    let source=Source{pc:source.source_precision(),height:geometry.height.bits,shape:Shape{kind:w[0],radius_bits:w[1],extent_10_bits:w[4],extent_14_bits:w[5]}};
    let mut b=Bound{staged,catalog,key,source,pc};b.check(key)?;b.world()?;
    native::object_initialize_fields(key,params,&mut b,pc)
}
impl native::ObjectWorld<DefinitionRef,String> for Bound<'_,'_>{
    type Actor=ObjectKey;
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
            _ => Err(required("fresh Object scalar outside actual base fields")),
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
    fn placement_test(&mut self,a:ObjectKey,params:&Params)->Result<u32> {
        self.check(a)?;self.world()?;
        //4EF6A0's B4=false branch calls4F4590, which constructs its OWN
        //GACD descriptor. The caller's original mode/owner are not forwarded.
        let mut descriptor=Params::new(params.definition.clone());
        let pose=self.pose(a)?;descriptor.position_bits=pose.position_bits;
        descriptor.direction_bits=[pose.facing_bits[0],pose.facing_bits[1]];
        descriptor.byte_52=((!(pose.flags>>10))&1) as u8;
        crate::map_actor_placement::check(self.staged,self.catalog,a,&descriptor,self.pc).map_err(error)
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
                .prepare_object_bound(&s.actors, self.catalog, &s.definitions, a)
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
}
