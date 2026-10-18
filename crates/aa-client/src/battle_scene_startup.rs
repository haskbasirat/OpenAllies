//!Concrete retained43F948 scene/terrain/tail children on SimulationThread.
//!Only immutable render packets cross UiBridge. These children do not write
//!VWorld AD or complete the loading/resource-task protocol.
use crate::{
    battle_camera, battle_ui_bridge,
    live_scene::world_owner::ApplicationViews,
    loading::{self, LoadSettings},
};
use aa_assets::AssetStore;
use aa_netcode::local_bootstrap::LocalBootstrap;
use aa_numeric::Precision;
use aa_runtime::{
    catalog::Catalog, live_world_frame::World, native_camera::directory::Handle,
    simulation_execution::Execution, view_assets, view_init_world, view_startup_inputs::Inputs,
};
use std::sync::Arc;
type Result<T> = std::result::Result<T, String>;
type Native =
    view_init_world::State<crate::live_scene::view_directory::ViewHandle, view_assets::State>;

///Separate UI-owned battle scene; inspection geometry cannot stand in for it.
pub struct UiScene {
    scene: Option<crate::gpu::CpuScene>,
}
impl UiScene {
    pub fn empty() -> Self {
        Self { scene: None }
    }
    pub fn scene(&self) -> Option<&crate::gpu::CpuScene> {
        self.scene.as_ref()
    }
    fn require_empty(&self) -> Result<()> {
        if self.scene.is_none() {
            Ok(())
        } else {
            Err("battle UI scene is already installed; actual renderer/mission teardown is required".into())
        }
    }
}
#[derive(Debug)]
pub enum Diagnostic {
    GeneratingTerrain { milliseconds: u32 },
    CameraTarget { words: [u32; 2] },
}
///Captured camera and backend receipts stay beside the SAME VWorld on error.
pub struct State {
    attempted: bool,
    captured: Option<Handle>,
    static_map: Option<loading::battle_map::Bound>,
    failed_terrain: Option<Box<dyn view_init_world::TerrainBinding>>,
    failed_scene: Option<Box<dyn aa_runtime::view_scene_init::RendererBinding>>,
    diagnostics: Vec<Diagnostic>,
}
impl State {
    pub fn not_attempted() -> Self {
        Self {
            attempted: false,
            captured: None,
            static_map: None,
            failed_terrain: None,
            failed_scene: None,
            diagnostics: Vec::new(),
        }
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        native: &mut Native,
        world: &World<'static>,
        catalog: &Catalog<'_>,
        inputs: &mut Inputs,
        depot: &mut aa_runtime::resource_depot::Application,
        views: &mut ApplicationViews,
        host: &LocalBootstrap,
        bridge: &battle_ui_bridge::Client,
        execution: &mut Execution<'_>,
        map_id: &str,
        milliseconds: &mut impl FnMut() -> Result<u32>,
        pc: Precision,
    ) -> Result<()> {
        if self.attempted {
            return Err("same VWorld scene startup was already attempted; retained failure requires teardown".into());
        }
        let parts = world.presentation_parts()?;
        native
            .assets()
            .require_initialized(catalog, parts.definitions, parts.revision)?;
        inputs.validate(catalog, parts.definitions)?;
        self.attempted = true;
        {
            let mut globals = inputs.seeds().bind(catalog)?;
            let (camera, visual) = views.camera_and_visual_map();
            //431A90 has not run at this native point; no height allocation is
            //substituted from the simulation map or inspection scene.
            let op = battle_camera::Operation::new(
                catalog, world, inputs, visual, None, host, bridge, execution, pc,
            )?;
            let mut camera = camera.scene_services(&op, &mut self.captured);
            native.initialize_scene_from_assets(world, catalog, &mut globals, &mut camera, pc)?;
        }
        {
            let mut diagnostic = TerrainDiagnostic {
                events: &mut self.diagnostics,
                milliseconds,
            };
            native.initialize_grid_and_heightfield(
                world,
                catalog,
                inputs,
                || bridge.terrain_quad_current(execution),
                &mut diagnostic,
                pc,
            )?;
        }
        //One current frontend read lends the SAME immutable AssetStore and
        //configured modern rendering policy. No second source store is opened.
        let (store, settings): (Arc<AssetStore>, LoadSettings) = bridge
            .call(execution, |context| {
                Ok((Arc::clone(context.store), context.load_settings.clone()))
            })?;
        let terrain = crate::view_terrain_backend::Prepared::from_world(
            world,
            catalog,
            native
                .heightfield()
                .ok_or("same sampled visual heightfield is absent")?,
            &store,
            map_id,
            &settings,
        )
        .map_err(|e| e.to_string())?;
        let (pending, request) =
            loading::battle_map::Pending::request(world, map_id).map_err(|e| e.to_string())?;
        let (static_receipt, terrain_receipt) = bridge.call(execution, move |context| {
            context.battle_scene.require_empty()?;
            let prepared = request.load(&store, settings).map_err(|e| e.to_string())?;
            let (scene, receipt, installed) = prepared
                .upload_with_terrain(terrain, context.renderer)
                .map_err(|e| e.to_string())?;
            context.battle_scene.scene = Some(scene);
            Ok((receipt, installed))
        })?;
        self.static_map = Some(
            pending
                .accept(world, static_receipt)
                .map_err(|e| e.to_string())?,
        );
        if let Err((binding, error)) = native.install_terrain(world, Box::new(terrain_receipt)) {
            self.failed_terrain = Some(binding);
            return Err(error);
        }
        let request =
            crate::live_scene::scene_backend::Request::from_current(native.scene(), parts.revision)
                .map_err(|e| e.to_string())?;
        let scene_receipt = bridge.call(execution, move |context| {
            request.install(context.renderer).map_err(|e| e.to_string())
        })?;
        if let Err((binding, error)) = native.install_scene_renderer(world, Box::new(scene_receipt))
        {
            self.failed_scene = Some(binding);
            return Err(error);
        }
        views
            .visual_map_mut()
            .publish_installed_terrain(world, native)?;
        let captured = self
            .captured
            .as_ref()
            .ok_or("native43FB40 current-camera capture is absent")?;
        let mut diagnostics = TailDiagnostic(&mut self.diagnostics);
        //The short readonly Inputs borrow ends before native42D0D0 mutates
        //its actual helper owner in the common runtime coordinator.
        native.initialize_tail_children_with_inputs(
            world,
            catalog,
            inputs,
            depot,
            |same_world, current_inputs, height, target, precision| {
                let (camera, visual) = views.camera_and_visual_map();
                let op = battle_camera::Operation::new(
                    catalog,
                    same_world,
                    current_inputs,
                    visual,
                    Some(height),
                    host,
                    bridge,
                    execution,
                    precision,
                )?;
                camera.look_at_captured_6ff130(captured, target, &op)
            },
            &mut diagnostics,
            pc,
        )?;
        self.static_map
            .as_ref()
            .ok_or("battle static-map receipt is absent")?
            .validate(world)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
struct TerrainDiagnostic<'a, F> {
    events: &'a mut Vec<Diagnostic>,
    milliseconds: &'a mut F,
}
impl<F: FnMut() -> Result<u32>> view_init_world::Diagnostics for TerrainDiagnostic<'_, F> {
    fn generating_terrain_43fd94(&mut self) -> Result<()> {
        let milliseconds = (self.milliseconds)()?;
        log::info!("native VWorld generating terrain, actual Node timer={milliseconds}ms");
        self.events
            .push(Diagnostic::GeneratingTerrain { milliseconds });
        Ok(())
    }
}
struct TailDiagnostic<'a>(&'a mut Vec<Diagnostic>);
impl view_init_world::TailDiagnostics for TailDiagnostic<'_> {
    fn camera_target_43fe80(&mut self, words: [u32; 2]) -> Result<()> {
        log::info!(
            "native VWorld target x={} y={}",
            f32::from_bits(words[0]),
            f32::from_bits(words[1])
        );
        self.0.push(Diagnostic::CameraTarget { words });
        Ok(())
    }
}
