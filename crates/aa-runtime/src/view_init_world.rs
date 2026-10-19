//! One retained VWorld composition. Bounded kernels borrow these SAME owners;
//! completion requires its concrete assets and all actual scene/tail owners.
//! No bounded child is an arbitrary ready/application-activation constructor.
use crate::{
    catalog::Catalog, live_world_frame::World, view_grid_state, view_heightfield, view_init_prefix,
    view_scene_init, view_startup_inputs::Inputs, world_revision::ConstructedWorldRevision,
};
use aa_numeric::Precision;
type Result<T> = std::result::Result<T, String>;

///Implemented by the concrete installed modern terrain backend. Its receipt
///must retain/revalidate the SAME World, source geometry, visual heightfield,
///renderer allocation and current epoch, never a caller-supplied success bit.
pub trait TerrainBinding {
    fn validate(&self, world: &World<'_>, height: &view_heightfield::State) -> Result<()>;
}
///Real diagnostics/profiling output at43FD94..FDDA. The implementation owns
///the current application timer and an actual diagnostic sink; it cannot
///substitute a successful ignored callback for either reached operation.
pub trait Diagnostics {
    fn generating_terrain_43fd94(&mut self) -> Result<()>;
}
///The concrete camera directory must retain the handle captured at43FB40,
///before7010D0. Re-querying the current camera here can select another owner.
///This bounded service does not itself mint a completed camera/view receipt.
pub trait TailCamera {
    fn captured_look_at_6ff130(&mut self,world:&World<'_>,target:[u32;2],pc:Precision)->Result<()>;
}
pub trait TailDiagnostics {
    fn camera_target_43fe80(&mut self,current:[u32;2])->Result<()>;
}
///Opaque direct loans for actual object Init. It cannot construct a Grid or
///replace AC with a caller-chosen bool; each getter borrows the same owner.
pub struct ObjectInitParts<'a,V>{
    grid:&'a mut view_grid_state::State<V>,
    ac:&'a std::sync::atomic::AtomicU8,
    world:crate::world_revision::Witness,
}
impl<V> ObjectInitParts<'_,V>{
    pub fn require_world(&self,world:&ConstructedWorldRevision)->Result<()>{
        if self.world.is_for(world){Ok(())}else{Err("object Init loans belong to another or destroyed World".into())}
    }
    pub fn grid_mut(&mut self)->&mut view_grid_state::State<V>{self.grid}
    ///Read lazily AFTER the current UVar/global-mode checks in45B530.
    pub fn request_mode_ac(&self)->bool{self.ac.load(std::sync::atomic::Ordering::Acquire)!=0}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Constructed,
    PrefixComplete,
    SceneComplete,
    InitializingTerrainInputs,
    AwaitingTerrainBackend,
    TerrainInstalled,
    InitializingTail,
    TailCpuComplete,
    Complete,
    Failed,
}

///V is the real ViewDirectory's opaque view handle; A is the concrete asset
///field/resource owner. Neither owner is cloned or rebuilt from a snapshot.
///The client VWorldOwner wraps this ONE allocation plus its directory.
pub struct State<V, A> {
    prefix: view_init_prefix::State<view_grid_state::State<V>>,
    scene: view_scene_init::State,
    assets: A,
    fx_commands: crate::view_fx_commands::State,
    height: Option<view_heightfield::State>,
    terrain_slots: crate::view_terrain_slots::State,
    terrain: Option<Box<dyn TerrainBinding>>,
    auxiliary_12c: Option<crate::view_init_globals::Auxiliary12c>,
    helper_10c: Option<crate::view_init_globals::Helper>,
    trigger_vfx: crate::view_trigger_vfx::List,
    ///43FF60/43EC60 words (+98, +140, +174..+18C) and their statics.
    update: crate::vworld_update::Fields<V>,
    world: Option<crate::world_revision::Witness>,
    phase: Phase,
}
impl<V, A> State<V, A> {
    ///Only the documented bounded native constructor fields are represented.
    ///A is moved from its actual constructor owner, never silently defaulted.
    pub fn from_constructor_fields(assets: A) -> Self {
        Self {
            prefix: view_init_prefix::State::from_constructor_fields(),
            scene: view_scene_init::State::from_constructor_fields(),
            assets,
            fx_commands: crate::view_fx_commands::State::from_native_constructor(),
            height: None,
            terrain_slots: crate::view_terrain_slots::State::constructed(),
            terrain: None,
            auxiliary_12c: None,
            helper_10c: None,
            trigger_vfx: crate::view_trigger_vfx::List::from_constructor_fields(),
            update: crate::vworld_update::Fields::from_native_constructor(),
            world: None,
            phase: Phase::Constructed,
        }
    }
    pub fn prefix(&self) -> &view_init_prefix::State<view_grid_state::State<V>> {
        &self.prefix
    }
    pub fn object_init_parts_for(&mut self,world:&ConstructedWorldRevision)->Result<ObjectInitParts<'_,V>>{
        self.require_world(world)?;
        let(grid,ac)=self.prefix.object_init_parts()?;
        Ok(ObjectInitParts{grid,ac,world:world.witness()})
    }
    pub fn scene(&self) -> &view_scene_init::State {
        &self.scene
    }
    pub fn assets(&self) -> &A {
        &self.assets
    }
    pub fn fx_commands(&self) -> &crate::view_fx_commands::State { &self.fx_commands }
    ///Attachment43B870 precedes completed graphics initialization. Require
    ///the same retained World allocation, without manufacturing AD/readiness.
    pub fn fx_commands_mut_for(&mut self, world: &ConstructedWorldRevision)
        -> Result<&mut crate::view_fx_commands::State> {
        self.require_world(world)?;
        Ok(&mut self.fx_commands)
    }
    ///The same asset owner is lent for its real ordered43F1D4..F948 load.
    ///This mutable loan does not itself certify that load completed.
    pub fn assets_mut(&mut self) -> &mut A {
        &mut self.assets
    }
    ///Same installed terrain and visual samples at the modern visual-map
    ///publication point. No raw dimensions or GPU acknowledgement can replace
    ///this actual retained terrain binding.
    pub(crate) fn terrain_for_visual_map(&self,world:&World<'_>)->Result<&view_heightfield::State>{
        if self.phase!=Phase::TerrainInstalled{return Err("visual-map publication is outside actual installed terrain stage".into())}
        self.require_world(world.presentation_parts()?.revision)?;
        let height=self.height.as_ref().ok_or("visual-map heightfield allocation is absent")?;
        self.terrain.as_ref().ok_or("visual-map terrain GPU binding is absent")?.validate(world,height)?;
        Ok(height)
    }
    pub fn heightfield(&self) -> Option<&view_heightfield::State> {
        self.height.as_ref()
    }
    ///The 43FF60/43EC60 words of this SAME VWorld.
    pub fn update_fields(&self) -> &crate::vworld_update::Fields<V> {
        &self.update
    }
    pub fn update_fields_mut_for(&mut self, world: &ConstructedWorldRevision)
        -> Result<&mut crate::vworld_update::Fields<V>> {
        self.require_world(world)?;
        Ok(&mut self.update)
    }
    ///5C92F0's 43AB30 on this VWorld (reached only while AD is set): the
    ///76B120(0) render time, then World+E0/+E4, into +84/+88/+8C/+90.
    pub fn update_view_time_43ab30(&mut self, world: &ConstructedWorldRevision,
        sim: &mut impl crate::vworld_update::SimTime, pc: Precision) -> Result<()> {
        self.require_world(world)?;
        if !self.prefix.initialized_for(world)? {
            return Err("5C92F0 reaches 43AB30 only while VWorld+AD is set".into());
        }
        let time = crate::vworld_update::sim_time_76b120(sim, 0, pc)?;
        let e0 = sim.world_e0()?;
        let e4 = sim.world_e4()?;
        crate::vworld_update::view_time_43ab30(self.prefix.view_time_words_mut_for(world)?, time, e0, e4, pc)
    }
    ///43EC60's 4318E0 on VWorld+BC over the SAME World's ground.
    pub fn update_heightfield_rects_4318e0(&mut self, world: &World<'_>, pc: Precision) -> Result<usize> {
        self.require_world(world.presentation_parts()?.revision)?;
        self.height.as_mut().ok_or("4318E0 VWorld+BC heightfield is null")?.update_rects_4318e0(world, pc)
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn auxiliary_12c(&self)->Option<&crate::view_init_globals::Auxiliary12c>{self.auxiliary_12c.as_ref()}
    pub fn helper_10c(&self)->Option<&crate::view_init_globals::Helper>{self.helper_10c.as_ref()}
    pub fn trigger_vfx(&self)->&crate::view_trigger_vfx::List{&self.trigger_vfx}
    pub fn terrain_slot_count(&self) -> usize {
        self.terrain_slots.count()
    }
    pub fn terrain_slot_capacity(&self) -> usize {
        self.terrain_slots.capacity()
    }
    pub fn initialize_prefix(
        &mut self,
        world: &ConstructedWorldRevision,
        services: &mut impl view_init_prefix::Services<Grid = view_grid_state::State<V>>,
    ) -> Result<()> {
        //Historical standalone fixture adapter. Actual event entry below
        //must receive notification+38 independently of the World identity.
        self.initialize_prefix_with_argument(world,world.value(),services)
    }
    pub fn initialize_prefix_with_argument(
        &mut self,world:&ConstructedWorldRevision,init_word_b8:u32,
        services:&mut impl view_init_prefix::Services<Grid=view_grid_state::State<V>>,
    )->Result<()>{
        if self.phase != Phase::Constructed {
            return Err("retained VWorld prefix was already attempted".into());
        }
        self.world = Some(world.witness());
        let result = self.prefix.initialize_prefix_with_argument(world,init_word_b8, services);
        self.phase = if result.is_ok() {
            Phase::PrefixComplete
        } else {
            Phase::Failed
        };
        result
    }
    ///Bounded scene entry. The final concrete full-Init facade must first
    ///validate its actual A asset owner; this entry alone never writes AD.
    pub fn initialize_scene(
        &mut self,
        world: &ConstructedWorldRevision,
        g: &mut impl view_grid_state::Globals,
        camera: &mut impl view_scene_init::Camera,
        pc: Precision,
    ) -> Result<()> {
        if self.phase != Phase::PrefixComplete {
            return Err("scene requires the same successful VWorld prefix".into());
        }
        self.require_world(world)?;
        self.prefix.require_prefix_complete()?;
        let result = self.scene.initialize(world, g, camera, pc);
        self.phase = if result.is_ok() {
            Phase::SceneComplete
        } else {
            Phase::Failed
        };
        result
    }
    fn require_world(&self, world: &ConstructedWorldRevision) -> Result<()> {
        if self.world.as_ref().is_some_and(|w| w.is_for(world)) {
            Ok(())
        } else {
            Err("VWorld composition belongs to another or destroyed World".into())
        }
    }
    ///Concrete43FCA4..43FD94 prefix: source registry count, current AVars and
    ///UVar reads, SAME VGrid/World3C+40, then retained431A90/431BC0 samples.
    ///The actual modern terrain installation must follow; it is never inferred
    ///from successfully computing these CPU inputs.
    pub fn initialize_grid_and_heightfield(
        &mut self,
        world: &World<'_>,
        catalog: &Catalog<'_>,
        inputs: &Inputs,
        mut terrain_quad_current: impl FnMut() -> Result<i32>,
        diagnostics: &mut impl Diagnostics,
        pc: Precision,
    ) -> Result<()> {
        if self.phase != Phase::SceneComplete {
            return Err("terrain inputs require the same completed scene CPU segment".into());
        }
        let parts = world.presentation_parts()?;
        self.require_world(parts.revision)?;
        self.scene.require_cpu_complete(parts.revision)?;
        inputs.validate(catalog, parts.definitions)?;
        self.phase = Phase::InitializingTerrainInputs;
        let result = (|| {
            let registry = catalog
                .actors
                .native_data_registry("Terrain")
                .ok_or("canonical native Terrain registry is absent")?;
            self.terrain_slots.initialize(registry.entries.len())?;
            let metrics = inputs.grid().metrics(catalog, || terrain_quad_current())?;
            let extents = parts
                .creator_extents
                .ok_or("same World3C/40 creator extents are absent")?;
            self.prefix
                .grid_mut()
                .ok_or("native VWorld94 VGrid constructor returned null")?
                .initialize_from_world(extents, metrics.grid_shift, pc)?;
            self.height = Some(view_heightfield::State::from_constructor());
            self.height.as_mut().unwrap().initialize_for_world(
                world,
                || terrain_quad_current(),
                pc,
            )?;
            diagnostics.generating_terrain_43fd94()?;
            Ok(())
        })();
        self.phase = if result.is_ok() {
            Phase::AwaitingTerrainBackend
        } else {
            Phase::Failed
        };
        result
    }
    pub fn install_terrain(
        &mut self,
        world: &World<'_>,
        binding: Box<dyn TerrainBinding>,
    ) -> std::result::Result<(), (Box<dyn TerrainBinding>, String)> {
        let result = (|| {
            if self.phase != Phase::AwaitingTerrainBackend {
                return Err(
                    "terrain GPU receipt arrived outside its actual initialization interval".into(),
                );
            }
            self.require_world(world.presentation_parts()?.revision)?;
            binding.validate(
                world,
                self.height.as_ref().ok_or("visual heightfield is absent")?,
            )
        })();
        if let Err(e) = result {
            return Err((binding, e));
        }
        self.terrain = Some(binding);
        self.phase = Phase::TerrainInstalled;
        Ok(())
    }
    ///Install actual same-renderer scene uniforms only after the initial
    ///static terrain upload has established its genuine nonzero scene epoch.
    pub fn install_scene_renderer(
        &mut self,
        world: &World<'_>,
        binding: Box<dyn view_scene_init::RendererBinding>,
    ) -> std::result::Result<(), (Box<dyn view_scene_init::RendererBinding>, String)> {
        let result = (|| {
            if self.phase != Phase::TerrainInstalled {
                return Err(
                    "scene GPU binding requires the actual installed terrain backend".into(),
                );
            }
            let revision = world.presentation_parts()?.revision;
            self.require_world(revision)?;
            self.terrain
                .as_ref()
                .ok_or("actual terrain receipt is absent")?
                .validate(
                    world,
                    self.height.as_ref().ok_or("visual heightfield absent")?,
                )?;
            Ok(())
        })();
        if let Err(e) = result {
            return Err((binding, e));
        }
        let revision = match world.presentation_parts() {
            Ok(p) => p.revision,
            Err(e) => return Err((binding, e)),
        };
        self.scene.install_renderer(revision, binding)
    }
    ///Actual43FE2B..43FF31 children in native order. The camera remains a
    ///required concrete producer; no direct look-at assignment substitutes for
    ///6FF130's tracking/clamp/terrain behavior. This bounded entry NEVER writes
    ///AD: the concrete assets/camera/full-Init facade must own that later step.
    pub fn initialize_tail_children<'world>(
        &mut self,world:&World<'world>,catalog:&Catalog<'_>,inputs:&mut Inputs,
        application:&crate::resource_depot::Application,
        camera:&mut impl TailCamera,diagnostics:&mut impl TailDiagnostics,pc:Precision,
    )->Result<()>{
        self.initialize_tail_children_with_inputs(world,catalog,inputs,application,
            |world,_inputs,_height,target,pc|camera.captured_look_at_6ff130(world,target,pc),diagnostics,pc)
    }
    ///The actual camera reads the SAME current source and visual samples in a
    ///short immutable loan. It ends before42D0D0 mutates Inputs.helper_globals.
    ///The legacy entry delegates here; native ordering has one coordinator.
    #[allow(clippy::too_many_arguments)]
    pub fn initialize_tail_children_with_inputs<'world>(
        &mut self,world:&World<'world>,catalog:&Catalog<'_>,inputs:&mut Inputs,
        application:&crate::resource_depot::Application,
        mut camera:impl FnMut(&World<'world>,&Inputs,&view_heightfield::State,[u32;2],Precision)->Result<()>,
        diagnostics:&mut impl TailDiagnostics,pc:Precision,
    )->Result<()>{
        if self.phase!=Phase::TerrainInstalled{return Err("VWorld tail requires the actual installed terrain/scene interval".into())}
        let parts=world.presentation_parts()?;
        self.require_world(parts.revision)?;
        inputs.validate(catalog,parts.definitions)?;
        self.scene.require_renderer(parts.revision)?;
        self.terrain.as_ref().ok_or("actual terrain receipt absent")?.validate(world,self.height.as_ref().ok_or("visual heightfield absent")?)?;
        self.phase=Phase::InitializingTail;
        let result=(||{
            let target=world.presentation_parts()?.camera_target.look_at.ok_or("actual World22C/230 target has no reached writer")?;
            camera(world,inputs,self.height.as_ref().ok_or("visual heightfield absent")?,target,pc)?;
            //43FE58/5A reread the SAME World fields AFTER the camera call.
            let current=world.presentation_parts()?.camera_target.look_at.ok_or("actual World22C/230 target is unavailable after camera call")?;
            diagnostics.camera_target_43fe80(current)?;
            self.auxiliary_12c=Some(crate::view_init_globals::Auxiliary12c::from_native_constructor());
            self.helper_10c=Some(inputs.helper_globals_mut().construct_helper_42d0d0()?);
            let parts=world.presentation_parts()?;
            let triggers=parts.triggers.ok_or("same World TriggerManager is absent at54BDE0")?;
            self.trigger_vfx.collect(catalog,parts.definitions,triggers)?;
            self.trigger_vfx.request(catalog,parts.definitions,application)?;
            Ok(())
        })();
        self.phase=if result.is_ok(){Phase::TailCpuComplete}else{Phase::Failed};
        result
    }
}

impl<V> State<V,crate::view_assets::State>{
    ///The actual43FF36 completion store after ALL retained children return.
    ///This grants neither queued-resource completion nor application gameplay
    ///activation: native AC is cleared only by the later real5B355D callback.
    pub fn complete_init_after_children(&mut self,world:&World<'_>,catalog:&Catalog<'_>)->Result<()>{
        if self.phase!=Phase::TailCpuComplete{return Err("VWorld completion requires the actual successful full tail".into())}
        let parts=world.presentation_parts()?;
        self.require_world(parts.revision)?;
        self.prefix.require_prefix_complete()?;
        self.assets.require_initialized(catalog,parts.definitions,parts.revision)?;
        self.scene.require_renderer(parts.revision)?;
        let height=self.height.as_ref().ok_or("VWorld completion actual visual heightfield is absent")?;
        self.terrain.as_ref().ok_or("VWorld completion actual terrain allocation is absent")?.validate(world,height)?;
        if self.auxiliary_12c.is_none()||self.helper_10c.is_none()||!self.trigger_vfx.is_complete(){
            return Err("VWorld completion actual auxiliary/helper/trigger request children are incomplete".into())
        }
        self.prefix.complete_full_init_43ff36(parts.revision)?;
        self.phase=Phase::Complete;
        Ok(())
    }
    ///The concrete asset owner is checked before the scene's first child. The
    ///generic bounded scene entry cannot forge this actual source completion.
    pub fn initialize_scene_from_assets(
        &mut self,world:&World<'_>,catalog:&Catalog<'_>,
        globals:&mut impl view_grid_state::Globals,camera:&mut impl view_scene_init::Camera,pc:Precision,
    )->Result<()>{
        let parts=world.presentation_parts()?;
        self.assets.require_initialized(catalog,parts.definitions,parts.revision)?;
        self.initialize_scene(parts.revision,globals,camera,pc)
    }
}
