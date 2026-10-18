//! One retained VWorld composition. Bounded kernels borrow these SAME owners;
//! this type currently stops at the concrete remaining tail services and has
//! no AD writer, arbitrary ready setter or application activation constructor.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Constructed,
    PrefixComplete,
    SceneComplete,
    InitializingTerrainInputs,
    AwaitingTerrainBackend,
    TerrainInstalled,
    Failed,
}

///V is the real ViewDirectory's opaque view handle; A is the concrete asset
///field/resource owner. Neither owner is cloned or rebuilt from a snapshot.
///The client VWorldOwner wraps this ONE allocation plus its directory.
pub struct State<V, A> {
    prefix: view_init_prefix::State<view_grid_state::State<V>>,
    scene: view_scene_init::State,
    assets: A,
    height: Option<view_heightfield::State>,
    terrain_slots: crate::view_terrain_slots::State,
    terrain: Option<Box<dyn TerrainBinding>>,
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
            height: None,
            terrain_slots: crate::view_terrain_slots::State::constructed(),
            terrain: None,
            world: None,
            phase: Phase::Constructed,
        }
    }
    pub fn prefix(&self) -> &view_init_prefix::State<view_grid_state::State<V>> {
        &self.prefix
    }
    pub fn scene(&self) -> &view_scene_init::State {
        &self.scene
    }
    pub fn assets(&self) -> &A {
        &self.assets
    }
    ///The same asset owner is lent for its real ordered43F1D4..F948 load.
    ///This mutable loan does not itself certify that load completed.
    pub fn assets_mut(&mut self) -> &mut A {
        &mut self.assets
    }
    pub fn heightfield(&self) -> Option<&view_heightfield::State> {
        self.height.as_ref()
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
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
        if self.phase != Phase::Constructed {
            return Err("retained VWorld prefix was already attempted".into());
        }
        self.world = Some(world.witness());
        let result = self.prefix.initialize_prefix(world, services);
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
}
