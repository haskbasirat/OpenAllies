//!Actual camera process owner and short current-source loans on SimulationThread.
//!The UI RPC reads reached current cells once per closed numeric operation;
//!GPU devices, Bool references and property/source effects execute once.
use crate::{battle_ui_bridge, gpu};
use aa_netcode::local_bootstrap::LocalBootstrap;
use aa_numeric::Precision;
use aa_runtime::{
    catalog::Catalog,
    live_world_frame::World,
    native_camera::{
        self,
        directory::{self, RendererSource},
    },
    native_camera_current,
    simulation_execution::Execution,
    view_heightfield,
    view_startup_inputs::Inputs,
    view_visual_map,
};
use std::{cell::RefCell, collections::BTreeMap};
type Result<T> = std::result::Result<T, String>;
pub type Directory =
    directory::Directory<aa_ui::uvars::CameraRenderListSwitch, gpu::camera_device::Reference>;
///One process directory owns the SAME material/default Factory across missions.
pub struct Process {
    directory: Directory,
}
impl Process {
    pub fn from_application_constructor() -> Result<Self> {
        Ok(Self {
            directory: Directory::from_fresh_native_process()?,
        })
    }
    pub fn directory(&self) -> &Directory {
        &self.directory
    }
    pub fn directory_mut(&mut self) -> &mut Directory {
        &mut self.directory
    }
    fn ensure_initial_cameras(&mut self, op: &Operation<'_, '_, '_>) -> Result<()> {
        if let Some(failure)=self.directory.failed_camera(){return Err(failure.error.clone())}
        if let Some(error)=self.directory.publication_error(){return Err(error.to_owned())}
        if self.directory.camera_count() != 0 {
            return Ok(());
        }
        let current = op.current()?;
        let mut vars = Variables(&op.frontend);
        let mut device = Device(&op.frontend);
        let mut properties = Properties(op);
        //5C8650 initial UI constructor: source-selected render-list camera first,
        //then the separate non-render-list UI+38 camera. Modern creation is deferred
        //until these real source/UI/backend loans exist, never an inspector camera.
        let lists = op
            .frontend
            .call(|context| context.ui.uvars.renderer_use_render_lists_current())?;
        self.directory.construct_camera(
            lists,
            &current,
            &mut vars,
            &mut device,
            &mut properties,
            op.pc,
        )?;
        self.directory.construct_camera(
            false,
            &current,
            &mut vars,
            &mut device,
            &mut properties,
            op.pc,
        )?;
        Ok(())
    }
    pub fn reset_current_700e30(
        &mut self,
        op: &Operation<'_, '_, '_>,
    ) -> Result<directory::Handle> {
        self.ensure_initial_cameras(op)?;
        self.directory.reset_current_700e30(&op.current()?, op.pc)
    }
    pub fn reset_current_7010d0(
        &mut self,
        op: &Operation<'_, '_, '_>,
    ) -> Result<directory::Handle> {
        self.ensure_initial_cameras(op)?;
        self.directory
            .reset_current_7010d0(&op.current()?, &mut Zoom(&op.frontend), op.pc)
    }
    pub fn look_at_captured_6ff130(
        &mut self,
        handle: &directory::Handle,
        xy: [u32; 2],
        op: &Operation<'_, '_, '_>,
    ) -> Result<()> {
        self.directory
            .look_at_captured_6ff130(handle, xy, &mut op.current()?, op.pc)
    }
    pub fn scene_services<'a, 'b, 'source, 'execution>(
        &'a mut self,
        op: &'a Operation<'b, 'source, 'execution>,
        captured: &'a mut Option<directory::Handle>,
    ) -> Scene<'a, 'b, 'source, 'execution> {
        Scene {
            process: self,
            op,
            captured,
        }
    }
    pub fn target_services<'a, 'b, 'source, 'execution>(
        &'a mut self,
        op: &'a Operation<'b, 'source, 'execution>,
        captured: &'a directory::Handle,
    ) -> Target<'a, 'b, 'source, 'execution> {
        Target {
            process: self,
            op,
            captured,
        }
    }
    pub fn zero_pan_current_700500(
        &mut self,
        zero_pair: [u32; 2],
        op: &Operation<'_, '_, '_>,
    ) -> Result<()> {
        let h = self.directory.current_handle()?;
        self.directory.pan_captured_700500(
            &h,
            0,
            zero_pair,
            &mut Pan(&op.frontend),
            &mut op.current()?,
            op.pc,
        )
    }
}
///Source-only immutable World/Inputs/visual loans; execution remains its one
///actual mutable SimulationThread borrower inside the frontend RPC adapter.
pub struct Operation<'a, 'source, 'execution> {
    catalog: &'a Catalog<'source>,
    world: &'a World<'static>,
    inputs: &'a Inputs,
    visual_map: &'a view_visual_map::Global,
    height: Option<&'a view_heightfield::State>,
    frontend: Frontend<'a, 'execution>,
    pc: Precision,
}
impl<'a, 'source, 'execution> Operation<'a, 'source, 'execution> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        catalog: &'a Catalog<'source>,
        world: &'a World<'static>,
        inputs: &'a Inputs,
        visual_map: &'a view_visual_map::Global,
        height: Option<&'a view_heightfield::State>,
        host: &'a LocalBootstrap,
        bridge: &'a battle_ui_bridge::Client,
        execution: &'a mut Execution<'execution>,
        pc: Precision,
    ) -> Result<Self> {
        inputs.validate(catalog, world.presentation_parts()?.definitions)?;
        Ok(Self {
            catalog,
            world,
            inputs,
            visual_map,
            height,
            frontend: Frontend {
                host,
                bridge,
                execution: RefCell::new(execution),
                values: RefCell::new(BTreeMap::new()),
                authorization: RefCell::new(None),
            },
            pc,
        })
    }
    fn current(
        &self,
    ) -> Result<native_camera_current::Current<'_, 'source, Frontend<'a, 'execution>>> {
        native_camera_current::Current::bind(
            self.catalog,
            self.world,
            self.inputs,
            self.visual_map,
            self.height,
            self.frontend.host,
            &self.frontend,
            self.pc,
        )
    }
}
struct Frontend<'a, 'execution> {
    host: &'a LocalBootstrap,
    bridge: &'a battle_ui_bridge::Client,
    execution: RefCell<&'a mut Execution<'execution>>,
    values: RefCell<BTreeMap<u32, Result<u32>>>,
    authorization: RefCell<Option<Result<bool>>>,
}
impl Frontend<'_, '_> {
    fn call<T: Send + 'static>(
        &self,
        call: impl FnOnce(&mut battle_ui_bridge::Context<'_>) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let mut execution = self
            .execution
            .try_borrow_mut()
            .map_err(|_| "camera UI RPC has a reentrant execution loan")?;
        self.bridge.call(&mut **execution, call)
    }
}
impl native_camera_current::Frontend for Frontend<'_, '_> {
    fn can_process_40eaa0(&self) -> Result<bool> {
        if let Some(value) = self.authorization.borrow().as_ref() {
            return value.clone();
        }
        let result = self
            .host
            .can_process_40eaa0(|| self.call(|context| context.ui.uvars.dev_mode_current()));
        *self.authorization.borrow_mut() = Some(result.clone());
        result
    }
    fn current_uvar(&self, offset: u32) -> Result<u32> {
        if let Some(value) = self.values.borrow().get(&offset) {
            return value.clone();
        }
        let result = self.call(move |context| context.ui.uvars.camera_input_word(offset));
        self.values.borrow_mut().insert(offset, result.clone());
        result
    }
}
struct Variables<'a, 'b, 'c>(&'a Frontend<'b, 'c>);
impl native_camera::render_lists::Variables for Variables<'_, '_, '_> {
    type Reference = aa_ui::uvars::CameraRenderListSwitch;
    fn bool_reference(&mut self, name: &'static str) -> Result<Self::Reference> {
        self.0
            .call(move |context| context.ui.uvars.camera_render_list_switch(name))
    }
}
struct Device<'a, 'b, 'c>(&'a Frontend<'b, 'c>);
impl RendererSource for Device<'_, '_, '_> {
    type Reference = gpu::camera_device::Reference;
    fn current_device_40(&mut self) -> Result<Self::Reference> {
        self.0
            .call(|context| RendererSource::current_device_40(context.renderer))
    }
}
struct Properties<'a, 'b, 'c, 'd>(&'a Operation<'b, 'c, 'd>);
impl aa_runtime::native_render_properties::CameraInputs for Properties<'_, '_, '_, '_> {
    fn stencil_shadow_color_c0(&mut self) -> Result<[u32; 3]> {
        self.0.inputs.camera_render().current(
            self.0.catalog,
            self.0.world.presentation_parts()?.definitions,
        )
    }
    fn renderer_word_24c(&mut self) -> Result<u32> {
        self.0.frontend.call(|context| {
            context
                .renderer
                .camera_stencil_word_24c()
                .map_err(|e| e.to_string())
        })
    }
}
struct Zoom<'a, 'b, 'c>(&'a Frontend<'b, 'c>);
impl directory::ZoomVariable for Zoom<'_, '_, '_> {
    fn camera_scale_zoom_34(&mut self) -> Result<u32> {
        self.0
            .call(|context| context.ui.uvars.camera_input_word(0x34))
    }
    fn store_camera_scale_zoom_34(&mut self, value: u32) -> Result<()> {
        self.0
            .call(move |context| context.ui.uvars.store_camera_scale_zoom_34(value))
    }
}
struct Pan<'a, 'b, 'c>(&'a Frontend<'b, 'c>);
impl directory::PanVariables for Pan<'_, '_, '_> {
    fn camera_pan_word(&mut self, offset: u32) -> Result<u32> {
        native_camera_current::Frontend::current_uvar(self.0, offset)
    }
}

///Short scene slice borrower: retain the true pre-7010D0 pointer for tail,
///then re-read the real current directory slot before lending root164.
pub struct Scene<'a, 'b, 'source, 'execution> {
    process: &'a mut Process,
    op: &'a Operation<'b, 'source, 'execution>,
    captured: &'a mut Option<directory::Handle>,
}
impl aa_runtime::view_scene_init::Camera for Scene<'_, '_, '_, '_> {
    fn reset_current_7010d0(&mut self) -> Result<()> {
        let handle = self.process.directory.current_handle()?;
        *self.captured = Some(handle);
        self.process.reset_current_7010d0(self.op).map(|_| ())
    }
    fn current_root_164(&mut self) -> Result<&mut aa_runtime::view_scene_init::Node> {
        let handle = self.process.directory.current_handle()?;
        self.process.directory.camera_mut(&handle)?.root_164()
    }
}
///Construct anew at the tail using its then-current actual height/visual map
///loans. It targets the retained capture; it never reselects current0.
pub struct Target<'a, 'b, 'source, 'execution> {
    process: &'a mut Process,
    op: &'a Operation<'b, 'source, 'execution>,
    captured: &'a directory::Handle,
}
impl aa_runtime::view_init_world::TailCamera for Target<'_, '_, '_, '_> {
    fn captured_look_at_6ff130(
        &mut self,
        world: &World<'_>,
        xy: [u32; 2],
        pc: Precision,
    ) -> Result<()> {
        if !world
            .presentation_parts()?
            .revision
            .witness()
            .is_for(self.op.world.presentation_parts()?.revision)
        {
            return Err("captured camera tail uses a different actual World".into());
        }
        if pc != self.op.pc {
            return Err("captured camera tail precision differs from its actual operation".into());
        }
        self.process
            .look_at_captured_6ff130(self.captured, xy, self.op)
    }
}
