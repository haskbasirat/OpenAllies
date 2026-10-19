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
#[path = "battle_camera_input.rs"]
pub mod input;
#[path = "battle_orders_hud.rs"]
pub mod orders_hud;
///One process directory owns the SAME material/default Factory across missions.
///The KKC_UI camera controls (UI+40/+48, KKC_Input impulses) are retained on
///this same owner because every control reads/mutates *B40048 directly.
pub struct Process {
    directory: Directory,
    controls: Option<input::Native>,
    ///The KKC_UI box state's +4 screen polygon (5CD850..5CCA30). Camera+168
    ///holds the same allocation while it is attached.
    screen_box: Option<native_camera::screen::Polygon>,
}
impl Process {
    pub fn from_application_constructor() -> Result<Self> {
        Ok(Self {
            directory: Directory::from_fresh_native_process()?,
            controls: None,
            screen_box: None,
        })
    }
    pub fn controls(&self) -> Option<&input::Native> {
        self.controls.as_ref()
    }
    ///One observed UI frame through the ported native camera controls. The
    ///hotkey file is the actual Data/Localization/Hotkeys/hotkeys_core.txt.
    pub fn input_frame(
        &mut self,
        op: &Operation<'_, '_, '_>,
        frame: &input::Frame,
        gates: &mut input::Gates<'_>,
    ) -> Result<Vec<aa_runtime::native_camera::input::Trace>> {
        if self.controls.is_none() {
            let text = op.frontend.call(|context| {
                let bytes = context
                    .ui
                    .store
                    .root
                    .read("Localization/Hotkeys/hotkeys_core.txt")
                    .map_err(|e| format!("hotkeys_core.txt: {e}"))?;
                Ok(aa_ui::data::decode(&bytes))
            })?;
            self.controls = Some(input::Native::new(&text, frame.client)?);
        }
        let handle = self.directory.current_handle()?;
        self.directory.camera(&handle)?.normalized_state()?;
        let mut camera = CameraOps {
            directory: &mut self.directory,
            handle,
            op,
        };
        let mut vars = InputVars { op };
        self.controls
            .as_mut()
            .expect("constructed above")
            .frame(frame, gates, &mut vars, &mut camera, op.pc)
    }
    ///Current target/zoom words for diagnostics (24C/250/254, 208, 2BC, 2B8).
    pub fn current_pose_words(&self) -> Result<[u32; 6]> {
        let h = self.directory.current_handle()?;
        let mut out = [0; 6];
        for (i, o) in [0x24c, 0x250, 0x254, 0x208, 0x2bc, 0x2b8].into_iter().enumerate() {
            out[i] = self.directory.word_captured(&h, o)?;
        }
        Ok(out)
    }
    pub fn screen_box(&self) -> Option<&native_camera::screen::Polygon> {
        self.screen_box.as_ref()
    }
    ///The box/facing camera children one orders frame reached, in order, on
    ///`*B40048`. `tracker` is the UI+58 tracker as 5CCA80 read it (both box
    ///children are reached in the frame's tracker step: the cursor after
    ///5BF2B0 and the left press before this frame's events).
    pub fn orders_children(
        &mut self,
        children: &[orders_hud::Child],
        tracker: native_camera::screen::Tracker,
        viewport: [i32; 2],
        seeds: Option<[u32; 3]>,
        pc: Precision,
    ) -> Result<()> {
        use orders_hud::Child;
        for child in children {
            match child {
                Child::BoxBegin => {
                    let h = self.directory.current_handle()?;
                    self.directory.set_byte_214_captured(&h, 1)?;
                    let seeds = seeds.ok_or("5CD850 B43CF8 seed vector is not bound")?;
                    let polygon = self.directory.new_screen_box_5cd850(seeds, tracker, viewport, pc)?;
                    //5CD9A8..5CD9D0: +4 replaces (releases) the previous polygon.
                    self.screen_box = Some(std::sync::Arc::clone(&polygon));
                    self.directory.attach_screen_polygon_captured_5cd7e0(&h, &polygon)?;
                }
                Child::BoxFrame => {
                    let polygon = self.screen_box.as_ref().ok_or("5CCA80 box state +4 polygon is NULL")?;
                    let mut polygon = polygon.lock().map_err(|_| "screen box owner poisoned")?;
                    native_camera::screen::update_5cca80(&mut polygon, tracker, viewport, pc)?;
                }
                Child::BoxEnd => {
                    let h = self.directory.current_handle()?;
                    self.directory.set_byte_214_captured(&h, 0)?;
                    //874350(NULL) finds no attached NULL slot to clear.
                    if let Some(polygon) = self.screen_box.take() {
                        self.directory.detach_screen_polygon_captured_874350(&h, &polygon)?;
                    }
                }
                Child::FacingLock | Child::FacingUnlock => {
                    let h = self.directory.current_handle()?;
                    self.directory.set_byte_214_captured(&h, u8::from(*child == Child::FacingLock))?;
                }
            }
        }
        Ok(())
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

///*B40048 camera operations for the native controls: the SAME current
///directory camera, its retained Current services and UI RPC adapters.
struct CameraOps<'a, 'b, 'source, 'execution> {
    directory: &'a mut Directory,
    handle: directory::Handle,
    op: &'a Operation<'b, 'source, 'execution>,
}
impl aa_runtime::native_camera::input::Camera for CameraOps<'_, '_, '_, '_> {
    fn word(&mut self, offset: u32) -> Result<u32> {
        self.directory.word_captured(&self.handle, offset)
    }
    fn byte_214(&mut self) -> Result<u8> {
        self.directory.byte_captured(&self.handle, 0x214)
    }
    fn developer_6fe9c0(&mut self) -> Result<bool> {
        let current = self.op.current()?;
        self.directory.developer_captured_6fe9c0(&self.handle, &current)
    }
    fn pan_700500(&mut self, millis: u32, direction: [u32; 2]) -> Result<()> {
        //Reached UVar cells are captured before the first camera store, so
        //an unbound cell refuses the input without failing the camera.
        for offset in [0x28, 0x18, 0x24, 0xa8, 0xac] {
            native_camera_current::Frontend::current_uvar(&self.op.frontend, offset)?;
        }
        let mut current = self.op.current()?;
        self.directory.pan_captured_700500(
            &self.handle,
            millis,
            direction,
            &mut Pan(&self.op.frontend),
            &mut current,
            self.op.pc,
        )
    }
    fn zoom_7005f0(&mut self, value: u32) -> Result<()> {
        let result = self.directory.set_zoom_captured_7005f0(
            &self.handle,
            value,
            &mut Zoom(&self.op.frontend),
            self.op.pc,
        );
        //40E0A0 changed the SAME UVar34 cell; later reads must see it.
        self.op.frontend.values.borrow_mut().remove(&0x34);
        result
    }
    fn pose_6ff220(&mut self, center: [u32; 3], yaw: u32, pitch: u32, zoom: u32) -> Result<()> {
        let current = self.op.current()?;
        self.directory
            .pose_captured_6ff220(&self.handle, &current, center, yaw, pitch, zoom, self.op.pc)
    }
    fn rotate_6feba0(&mut self, yaw: u32, pitch: u32) -> Result<()> {
        self.directory
            .rotate_about_target_captured_6feba0(&self.handle, yaw, pitch, self.op.pc)
    }
    fn look_at_6ff200(&mut self, xy: [u32; 2]) -> Result<()> {
        let mut current = self.op.current()?;
        self.directory
            .look_at_captured_6ff130(&self.handle, xy, &mut current, self.op.pc)
    }
}
///UVar fast-table B39940 / AVars B39990 reads for the controls.
struct InputVars<'a, 'b, 'source, 'execution> {
    op: &'a Operation<'b, 'source, 'execution>,
}
impl aa_runtime::native_camera::input::Variables for InputVars<'_, '_, '_, '_> {
    fn uvar(&mut self, offset: u32) -> Result<u32> {
        let name = match offset {
            0x1c => "CameraScrollAcceleration",
            0x20 => "CameraScrollSpeedMax",
            0x2c => "CameraScalePitch",
            0x30 => "CameraScaleYaw",
            0x38 => "CameraRotateSpeed",
            _ => return native_camera_current::Frontend::current_uvar(&self.op.frontend, offset),
        };
        //Not in aa-ui's strict camera fast table yet: the current UVars.tgi
        //overlay value of the SAME UI owner, refused outside its 41F9D0 range.
        self.op.frontend.call(move |context| {
            let vars = &context.ui.uvars;
            let def = vars
                .def(name)
                .ok_or_else(|| format!("UVar {name} is not registered in the current UI owner"))?;
            if !matches!(def.kind, aa_ui::uvars::Kind::Float) {
                return Err(format!("UVar {name} is not a Float"));
            }
            let raw = vars.get_str(name);
            let value: f32 = raw
                .trim()
                .parse()
                .map_err(|_| format!("UVar {name} current value `{raw}` is not a Float"))?;
            if !value.is_finite()
                || def.range.is_some_and(|(lo, hi)| f64::from(value) < lo || f64::from(value) > hi)
            {
                return Err(format!("UVar {name} current value is outside its native range"));
            }
            Ok(value.to_bits())
        })
    }
    fn avar(&mut self, offset: u32) -> Result<u32> {
        let current = self.op.current()?;
        directory::ReadOnlyInputs::avar(&current, offset)
    }
    fn avar_range(&mut self, offset: u32) -> Result<[u32; 2]> {
        Err(format!(
            "AVar{offset:X} registered range (+14/+18) has no ported source owner"
        ))
    }
    fn metagame_40cee0(&mut self) -> Result<bool> {
        let current = self.op.current()?;
        directory::ReadOnlyInputs::metagame_40cee0(&current)
    }
}
