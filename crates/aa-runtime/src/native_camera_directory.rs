//! Closed camera7016A0 ->702380 ->UI reset ownership.
//!
//! The original constructor reads its unwritten204 index. This deliberate UB
//! normalization retains both zero/nonzero *local numeric alternatives* until
//! real publication and UI reset converge bit for bit. It does not claim the
//! original first branch, permit mutable callback replay, or grant view-ready.
use super::{base, ni, pose, quad, render_lists, screen, ResetServices, Result, State};
use crate::{
    native_render_defaults::SharedSet,
    native_render_properties::CameraInputs,
    view_grid_state::Globals,
    view_scene_init::{Node, Transform},
};
use aa_numeric::{Binary, Precision};
use std::sync::{Arc, Weak};

///Actual stable source/configuration and immutable map/surface loans during
///one closed operation. Implementations MUST NOT draw RNG, enqueue requests,
///mutate UI/resources/World, or change values between alternative reads.
///A UI channel must capture its read-only current cells once before entry.
pub trait ReadOnlyInputs {
    fn metagame_40cee0(&self) -> Result<bool>;
    fn free_camera_without_directory_role(&self) -> Result<bool>;
    fn avar(&self, offset: u32) -> Result<u32>;
    fn map_dimensions_74_78(&self) -> Result<Option<[i32; 2]>>;
    fn fallback_dimensions_uvar_a8_ac(&self) -> Result<[i32; 2]>;
    fn visual_height_43d050(&self, xy: [u32; 2], flags: u32, pc: Precision) -> Result<Binary>;
    fn seed_word(&self, address: u32) -> Result<u32>;
    fn free_yaw_uvar_b4(&self) -> Result<bool>;
    fn world_editor_275(&self) -> Result<bool>;
    fn session_state_e4(&self) -> Result<u32>;
    fn map_height_550210(&self, xy: [u32; 2], pc: Precision) -> Result<Binary>;
}
///The actual UVar34 CameraScaleZoom allocation. Reads/writes happen only after
///the camera's native numeric update; failed writes preserve that prefix.
pub trait ZoomVariable {
    fn camera_scale_zoom_34(&mut self) -> Result<u32>;
    fn store_camera_scale_zoom_34(&mut self, value: u32) -> Result<()>;
}
pub trait PanVariables {
    fn camera_pan_word(&mut self, offset: u32) -> Result<u32>;
}
///A retained reference to the actual renderer device at Renderer+40, read at
///the reached native point. It is not an inspector renderer or a copied device.
pub trait RendererSource {
    type Reference;
    fn current_device_40(&mut self) -> Result<Self::Reference>;
}

struct Read<'a, I> {
    input: &'a I,
    role_nonzero: bool,
    matrix: &'a pose::Globals,
    pc: Precision,
}
impl<I: ReadOnlyInputs> Globals for Read<'_, I> {
    fn word(&mut self, a: u32) -> Result<u32> {
        self.input.seed_word(a)
    }
}
impl<I: ReadOnlyInputs> ResetServices for Read<'_, I> {
    fn metagame_40cee0(&mut self) -> Result<bool> {
        self.input.metagame_40cee0()
    }
    fn developer_camera_6fe9c0(&mut self, _: &State) -> Result<bool> {
        Ok(self.input.free_camera_without_directory_role()? || self.role_nonzero)
    }
    fn avar(&mut self, o: u32) -> Result<u32> {
        self.input.avar(o)
    }
    fn map_dimensions_74_78(&mut self) -> Result<Option<[i32; 2]>> {
        self.input.map_dimensions_74_78()
    }
    fn fallback_dimensions_uvar_a8_ac(&mut self) -> Result<[i32; 2]> {
        self.input.fallback_dimensions_uvar_a8_ac()
    }
    fn visual_height_43d050(&mut self, xy: [u32; 2], f: u32) -> Result<Binary> {
        self.input.visual_height_43d050(xy, f, self.pc)
    }
    fn seed_word(&mut self, a: u32) -> Result<u32> {
        self.input.seed_word(a)
    }
    fn dirty_planes_873d90(&mut self, c: &mut State) -> Result<()> {
        c.require_ni_base_constructor()?;
        if c.word(0x174)? != 0 {
            return Err("actual nonempty NiCamera plane-list dirtying is unbound".into());
        }
        Ok(())
    }
    fn set_pose_6ff220(
        &mut self,
        c: &mut State,
        center: [u32; 3],
        yaw: u32,
        pitch: u32,
        zoom: u32,
        p: Precision,
    ) -> Result<()> {
        let matrix = self.matrix;
        pose::set_pose_6ff220(c, matrix, self, center, yaw, pitch, zoom, p)
    }
}
impl<I: ReadOnlyInputs> pose::Services for Read<'_, I> {
    fn metagame_40cee0(&mut self) -> Result<bool> {
        self.input.metagame_40cee0()
    }
    fn developer_camera_6fe9c0(&mut self, _: &State) -> Result<bool> {
        Ok(self.input.free_camera_without_directory_role()? || self.role_nonzero)
    }
    fn avar(&mut self, o: u32) -> Result<u32> {
        self.input.avar(o)
    }
    fn free_yaw_uvar_b4(&mut self) -> Result<bool> {
        self.input.free_yaw_uvar_b4()
    }
    fn world_editor_275(&mut self) -> Result<bool> {
        self.input.world_editor_275()
    }
    fn session_state_e4(&mut self) -> Result<u32> {
        self.input.session_state_e4()
    }
    fn map_height_550210(&mut self, xy: [u32; 2]) -> Result<Binary> {
        self.input.map_height_550210(xy, self.pc)
    }
    fn update_camera_6fe050(&mut self, c: &mut State, p: Precision) -> Result<()> {
        ni::update_6fe050(c, &mut ni::RequiredChildren, p)
    }
}

struct Root {
    node: Node,
    controller: Weak<std::sync::atomic::AtomicU64>,
    properties: Vec<Option<crate::native_render_properties::Reference>>,
}
enum Phase {
    Constructing,
    Published,
    Normalized,
    Failed,
}
pub struct Camera<K, R> {
    states: Vec<State>,
    root: Option<Root>,
    renderer: Option<Arc<R>>,
    lists: Option<render_lists::State<K>>,
    quad: Option<quad::Quad>,
    ///Camera+168 NiScreenPolygon array (NiCamera constructor 874B84).
    screen_168: screen::List,
    phase: Phase,
}
impl<K, R> Camera<K, R> {
    fn pending() -> Self {
        Self {
            states: vec![State::unbound_allocation(), State::unbound_allocation()],
            root: None,
            renderer: None,
            lists: None,
            quad: None,
            screen_168: screen::List::default(),
            phase: Phase::Constructing,
        }
    }
    pub fn normalized_state(&self) -> Result<&State> {
        if !matches!(self.phase, Phase::Normalized) {
            return Err(
                "camera numeric alternatives have not converged after publication/UI reset".into(),
            );
        }
        Ok(&self.states[0])
    }
    pub fn render_lists(&self) -> Option<&render_lists::State<K>> {
        self.lists.as_ref()
    }
    pub fn auxiliary_quad(&self) -> Option<&quad::Quad> {
        self.quad.as_ref()
    }
    ///Camera+168 as the 6FF63E draw loop walks it.
    pub fn screen_polygons(&self) -> &screen::List {
        &self.screen_168
    }
    pub fn root_164(&mut self) -> Result<&mut Node> {
        self.normalized_state()?;
        let r = self
            .root
            .as_mut()
            .ok_or("native camera root164 constructor is incomplete")?;
        if r.controller.upgrade().is_none() {
            return Err("native root camera-controller allocation was destroyed".into());
        }
        Ok(&mut r.node)
    }
}
///Opaque retained capture of the actual directory allocation and ordinal.
///Native43FB40 may keep this while later operations reread the current slot.
#[derive(Clone)]
pub struct Handle {
    directory: Weak<()>,
    camera: Weak<std::sync::atomic::AtomicU64>,
    index: usize,
}
pub struct Failure<K, R> {
    pub camera: Camera<K, R>,
    pub error: String,
}
///One application retains this directory plus SAME material/default owner.
///There is no Clone, Default, raw-camera insertion or initialized-view setter.
pub struct Directory<K, R> {
    identity: Arc<()>,
    cameras: Vec<Camera<K, R>>,
    failed: Option<Failure<K, R>>,
    defaults: SharedSet,
    matrix: pose::Globals,
    effects_b45f18: Option<ProcessEffect>,
    publication_error: Option<String>,
}
//No source-proven positive process effect producer is bound yet. The actual
//PE image initializes B45F18 NULL; it is retained once across all cameras.
enum ProcessEffect {}
impl<K, R> Directory<K, R> {
    pub fn from_fresh_native_process() -> Result<Self> {
        let mut defaults = SharedSet::from_fresh_native_process()?;
        defaults.initialize_properties_prefix_8c3ed0()?;
        Ok(Self {
            identity: Arc::new(()),
            cameras: Vec::new(),
            failed: None,
            defaults,
            matrix: pose::Globals::from_static_constructor_9e3520(),
            effects_b45f18: None,
            publication_error: None,
        })
    }
    pub fn failed_camera(&self) -> Option<&Failure<K, R>> {
        self.failed.as_ref()
    }
    pub fn publication_error(&self) -> Option<&str> {
        self.publication_error.as_deref()
    }
    pub fn camera_count(&self) -> usize {
        self.cameras.len()
    }
    pub fn current_handle(&self) -> Result<Handle> {
        self.handle(0)
    }
    fn handle(&self, index: usize) -> Result<Handle> {
        let c = self
            .cameras
            .get(index)
            .ok_or("native camera directory slot absent")?;
        Ok(Handle {
            directory: Arc::downgrade(&self.identity),
            camera: Arc::downgrade(&c.states[0].revision),
            index,
        })
    }
    fn require(&self, h: &Handle) -> Result<usize> {
        if !h
            .directory
            .upgrade()
            .is_some_and(|p| Arc::ptr_eq(&p, &self.identity))
        {
            return Err("camera handle belongs to another/dead directory".into());
        }
        let c = self
            .cameras
            .get(h.index)
            .ok_or("captured camera directory ordinal is no longer present")?;
        if !h
            .camera
            .upgrade()
            .is_some_and(|p| Arc::ptr_eq(&p, &c.states[0].revision))
        {
            return Err("captured camera allocation was replaced/destroyed".into());
        }
        Ok(h.index)
    }
    pub fn camera(&self, h: &Handle) -> Result<&Camera<K, R>> {
        Ok(&self.cameras[self.require(h)?])
    }
    pub fn camera_mut(&mut self, h: &Handle) -> Result<&mut Camera<K, R>> {
        let i = self.require(h)?;
        Ok(&mut self.cameras[i])
    }
    ///Only local numeric operations are evaluated for both unknown204 classes.
    ///Renderer, variable references, property factories and node graphs execute
    ///ONCE, after the first reset and in their original constructor order.
    pub fn construct_camera<I: ReadOnlyInputs>(
        &mut self,
        use_lists: bool,
        input: &I,
        vars: &mut impl render_lists::Variables<Reference = K>,
        renderer: &mut impl RendererSource<Reference = R>,
        properties: &mut impl CameraInputs,
        pc: Precision,
    ) -> Result<Handle> {
        if self.failed.is_some() || self.publication_error.is_some() {
            return Err("camera directory retains a previous failed construction".into());
        }
        let mut c = Camera::pending();
        let built = (|| -> Result<()> {
            for index in 0..2 {
                let mut read = Read {
                    input,
                    role_nonzero: index != 0,
                    matrix: &self.matrix,
                    pc,
                };
                base::initialize_874b40(&mut c.states[index], &mut read, pc)?;
                prefix_7016a0(&mut c.states[index], pc)?;
                super::reset_700e30(&mut c.states[index], &mut read, pc)?;
            }
            c.renderer = Some(Arc::new(renderer.current_device_40()?));
            for s in &mut c.states {
                typed_pointer(s, 0x198);
            }
            if use_lists {
                c.lists = Some(match render_lists::State::from_native_constructor(vars) {
                    Ok(s) => s,
                    Err(f) => {
                        c.lists = Some(f.state);
                        return Err(f.error);
                    }
                });
                for s in &mut c.states {
                    s.ground_tracking_701847();
                    typed_pointer(s, 0x374)
                }
            }
            if !use_lists {
                for s in &mut c.states {
                    s.put(0x374, 0);
                }
            }
            let mut g = Read {
                input,
                role_nonzero: false,
                matrix: &self.matrix,
                pc,
            };
            let mut node = Node::from_native_constructor(&mut g)?;
            //866CE0 captures this camera as controller of the newly empty root.
            //Local translation is then explicitly cleared before86E8D0 update.
            let mut transform = *node.local_transform().words();
            transform[9..12].fill(0);
            node.set_local_transform(Transform::from_words(transform)?);
            node.update_world(pc)?;
            let root_properties = self.defaults.new_property_set_895550()?;
            c.root = Some(Root {
                node,
                controller: Arc::downgrade(&c.states[0].revision),
                properties: root_properties,
            });
            for s in &mut c.states {
                typed_pointer(s, 0x164);
            }
            //86F090 clears the SAME process effect scratch slot. A fresh root
            //has constructor-null parent24, so its inherited effect list is NULL.
            drop(self.effects_b45f18.take());
            for s in &mut c.states {
                for (i, w) in [
                    0, 0, 0, 0x3f800000, 0, 0, 0x3f800000, 0x3f800000, 0, 0, 0x3f800000, 0,
                ]
                .into_iter()
                .enumerate()
                {
                    s.put(0x340 + 4 * i as u32, w)
                }
            }
            let vertices = std::array::from_fn(|i| {
                std::array::from_fn(|j| {
                    c.states[0]
                        .word(0x340 + 4 * (i * 3 + j) as u32)
                        .expect("reached quad vertex store")
                })
            });
            c.quad = Some(
                match quad::Quad::from_camera_vertices_887f40(
                    vertices,
                    &mut self.defaults,
                    properties,
                ) {
                    Ok(q) => q,
                    Err(f) => {
                        c.quad = Some(f.quad);
                        return Err(f.error);
                    }
                },
            );
            for s in &mut c.states {
                typed_pointer(s, 0x370);
                for o in (0x378..=0x38c).step_by(4) {
                    s.put(o, 0)
                }
                s.put(0x390, 0x3f800000)
            }
            Ok(())
        })();
        if let Err(error) = built {
            c.phase = Phase::Failed;
            self.failed = Some(Failure {
                camera: c,
                error: error.clone(),
            });
            return Err(error);
        }
        //7022B0 append, retained-reference publication, then actual ordinal204.
        let ordinal = self.cameras.len();
        for s in &mut c.states {
            typed_pointer(s, 4);
            if let Ok(index) = u32::try_from(ordinal) {
                s.directory_index_702380(index as i32)
            }
        }
        c.phase = Phase::Published;
        self.cameras.push(c);
        //70DC60 rereads the current device after directory publication.
        let device = match renderer.current_device_40() {
            Ok(device) => device,
            Err(error) => {
                self.cameras[ordinal].phase = Phase::Failed;
                self.publication_error = Some(error.clone());
                return Err(error);
            }
        };
        self.cameras[ordinal].renderer = Some(Arc::new(device));
        self.handle(ordinal)
    }
    ///Render data belongs to the SAME actual scene only when its retained
    ///root is a child of this normalized CURRENT camera root. Equal numeric
    ///World IDs, transforms or a foreign camera handle do not establish that.
    pub fn snapshot_current_for_scene<V: super::backend::ViewportFor<R>>(
        &self,
        world: &crate::world_revision::ConstructedWorldRevision,
        scene: &crate::view_scene_init::State,
        viewport: Arc<V>,
    ) -> Result<super::backend::Snapshot> {
        scene.require_cpu_complete(world)?;
        let h = self.current_handle()?;
        let camera = self.camera(&h)?;
        let state = camera.normalized_state()?;
        let root = camera
            .root
            .as_ref()
            .ok_or("native camera root164 is absent")?;
        let scene_root = scene.root().ok_or("actual scene root is absent")?;
        if !root.node.is_parent_of(scene_root) {
            return Err("current camera root is not the actual World scene parent".into());
        }
        let device = camera
            .renderer
            .as_ref()
            .ok_or("native camera renderer198 is absent")?;
        viewport.require_device(device)?;
        super::backend::Snapshot::from_normalized_current(state, world, scene.witness()?, viewport)
    }
    ///7010D0 captures the current pointer, resets it, then reads SAME UVar34
    ///and invokes7005F0. This does not confer scene/view readiness.
    pub fn reset_current_7010d0(
        &mut self,
        input: &impl ReadOnlyInputs,
        zoom: &mut impl ZoomVariable,
        pc: Precision,
    ) -> Result<Handle> {
        let h = self.reset_current_700e30(input, pc)?;
        let value = match zoom.camera_scale_zoom_34() {
            Ok(v) => v,
            Err(error) => {
                let i = self.require(&h)?;
                self.cameras[i].phase = Phase::Failed;
                return Err(error);
            }
        };
        self.set_zoom_captured_7005f0(&h, value, zoom, pc)?;
        Ok(h)
    }
    pub fn set_zoom_captured_7005f0(
        &mut self,
        h: &Handle,
        value: u32,
        zoom: &mut impl ZoomVariable,
        pc: Precision,
    ) -> Result<()> {
        let index = self.require(h)?;
        let c = &mut self.cameras[index];
        c.normalized_state()?;
        let result = apply_zoom_7005f0(&mut c.states[0], index != 0, value, zoom, pc);
        if result.is_err() {
            c.phase = Phase::Failed;
        }
        result
    }
    ///700500 computes local planar pan arguments, then tail-calls6FF130 on
    ///the SAME camera. Even a zero delta preserves actual UVar/basis reads.
    pub fn pan_captured_700500(
        &mut self,
        h: &Handle,
        millis: u32,
        direction: [u32; 2],
        vars: &mut impl PanVariables,
        services: &mut impl super::target::Services,
        pc: Precision,
    ) -> Result<()> {
        let index = self.require(h)?;
        let c = &mut self.cameras[index];
        c.normalized_state()?;
        let result = (|| {
            let target = pan_target_700500(&c.states[0], millis, direction, vars, pc)?;
            super::target::set_look_at_6ff130(&mut c.states[0], services, target, pc)
        })();
        if result.is_err() {
            c.phase = Phase::Failed;
        }
        result
    }
    ///Tail43FE51 targets the SAME camera handle captured at43FB40, never a
    ///new current lookup. Required terrain/Session/UVar children stay explicit.
    pub fn look_at_captured_6ff130(
        &mut self,
        h: &Handle,
        xy: [u32; 2],
        services: &mut impl super::target::Services,
        pc: Precision,
    ) -> Result<()> {
        let index = self.require(h)?;
        let c = &mut self.cameras[index];
        c.normalized_state()?;
        let result = super::target::set_look_at_6ff130(&mut c.states[0], services, xy, pc);
        if result.is_err() {
            c.phase = Phase::Failed;
        }
        result
    }
    ///A field of the normalized captured camera as UI code reads *B40048.
    pub fn word_captured(&self, h: &Handle, offset: u32) -> Result<u32> {
        self.camera(h)?.normalized_state()?.word(offset)
    }
    pub fn byte_captured(&self, h: &Handle, offset: u32) -> Result<u8> {
        self.camera(h)?.normalized_state()?.byte(offset)
    }
    ///`[B40048]+214` stores of the KKC_UI box and facing states: 5CD88E 1,
    ///5CCA3A 0 (box), 5CCD70 1, 5CCDB3 0 (facing). Camera input reads it.
    pub fn set_byte_214_captured(&mut self, h: &Handle, value: u8) -> Result<()> {
        let index = self.require(h)?;
        let c = &mut self.cameras[index];
        c.normalized_state()?;
        c.states[0].put_byte(0x214, value);
        Ok(())
    }
    ///5CD850's polygon after its +214 store, with a new 895550 state of the
    ///SAME shared defaults and the SAME material Factory (B323C0).
    pub fn new_screen_box_5cd850(
        &mut self,
        seeds: [u32; 3],
        tracker: screen::Tracker,
        viewport: [i32; 2],
        pc: Precision,
    ) -> Result<screen::Polygon> {
        let state = self.defaults.new_property_set_895550()?;
        let (factory, globals) = self.defaults.factory_and_globals();
        let polygon = screen::box_5cd850(seeds, state, factory, globals, tracker, viewport, pc)?;
        Ok(Arc::new(std::sync::Mutex::new(polygon)))
    }
    ///5CD7E0 on the captured camera's +168 array.
    pub fn attach_screen_polygon_captured_5cd7e0(&mut self, h: &Handle, polygon: &screen::Polygon) -> Result<usize> {
        let index = self.require(h)?;
        let c = &mut self.cameras[index];
        c.normalized_state()?;
        c.screen_168.attach_5cd7e0(polygon)
    }
    ///874350 on the captured camera's +168 array.
    pub fn detach_screen_polygon_captured_874350(&mut self, h: &Handle, polygon: &screen::Polygon) -> Result<()> {
        let index = self.require(h)?;
        let c = &mut self.cameras[index];
        c.normalized_state()?;
        c.screen_168.detach_874350(polygon);
        Ok(())
    }
    ///6FE9C0 with the SAME directory-ordinal +204 role as the reset reads.
    pub fn developer_captured_6fe9c0(
        &self,
        h: &Handle,
        input: &impl ReadOnlyInputs,
    ) -> Result<bool> {
        let index = self.require(h)?;
        self.cameras[index].normalized_state()?;
        Ok(input.free_camera_without_directory_role()? || index != 0)
    }
    ///700440/7004A0/700A90 tail: 6FF220 on the SAME normalized camera.
    #[allow(clippy::too_many_arguments)]
    pub fn pose_captured_6ff220(
        &mut self,
        h: &Handle,
        input: &impl ReadOnlyInputs,
        center: [u32; 3],
        yaw: u32,
        pitch: u32,
        zoom: u32,
        pc: Precision,
    ) -> Result<()> {
        let index = self.require(h)?;
        let matrix = &self.matrix;
        let c = &mut self.cameras[index];
        c.normalized_state()?;
        let mut read = Read {
            input,
            role_nonzero: index != 0,
            matrix,
            pc,
        };
        let result =
            pose::set_pose_6ff220(&mut c.states[0], matrix, &mut read, center, yaw, pitch, zoom, pc);
        if result.is_err() {
            c.phase = Phase::Failed;
        }
        result
    }
    ///5CCF00 -> 6FEBA0 on the SAME normalized camera.
    pub fn rotate_about_target_captured_6feba0(
        &mut self,
        h: &Handle,
        yaw: u32,
        pitch: u32,
        pc: Precision,
    ) -> Result<()> {
        let index = self.require(h)?;
        let matrix = &self.matrix;
        let c = &mut self.cameras[index];
        c.normalized_state()?;
        let result = pose::rotate_about_target_6feba0(&mut c.states[0], matrix, yaw, pitch, pc);
        if result.is_err() {
            c.phase = Phase::Failed;
        }
        result
    }
    pub fn reset_current_700e30(
        &mut self,
        input: &impl ReadOnlyInputs,
        pc: Precision,
    ) -> Result<Handle> {
        let h = self.current_handle()?;
        self.reset_captured_700e30(&h, input, pc)?;
        Ok(h)
    }
    pub fn reset_captured_700e30(
        &mut self,
        h: &Handle,
        input: &impl ReadOnlyInputs,
        pc: Precision,
    ) -> Result<()> {
        let index = self.require(h)?;
        let c = &mut self.cameras[index];
        if matches!(c.phase, Phase::Failed) {
            return Err("camera retains an earlier failed native operation".into());
        }
        if matches!(c.phase, Phase::Normalized) {
            let mut read = Read {
                input,
                role_nonzero: index != 0,
                matrix: &self.matrix,
                pc,
            };
            let result = super::reset_700e30(&mut c.states[0], &mut read, pc);
            if result.is_err() {
                c.phase = Phase::Failed;
            }
            return result;
        }
        for state in &mut c.states {
            let mut read = Read {
                input,
                role_nonzero: index != 0,
                matrix: &self.matrix,
                pc,
            };
            if let Err(error) = super::reset_700e30(state, &mut read, pc) {
                c.phase = Phase::Failed;
                return Err(error);
            }
        }
        if !equivalent(&c.states[0], &c.states[1]) {
            c.phase = Phase::Failed;
            return Err("native constructor204 alternatives did not converge after actual publication/UI reset".into());
        }
        c.states.truncate(1);
        c.phase = Phase::Normalized;
        Ok(())
    }
}
fn pan_target_700500(
    s: &State,
    millis: u32,
    direction: [u32; 2],
    vars: &mut impl PanVariables,
    p: Precision,
) -> Result<[u32; 2]> {
    use super::{b, f, mul};
    let scale = vars.camera_pan_word(0x28)?;
    let speed = vars.camera_pan_word(0x18)?;
    let mut time = Binary::from_i32(millis as i32);
    if (millis as i32) < 0 {
        time = time
            .add(
                &Binary::from_f32_bits(0x4f800000).map_err(|e| e.to_string())?,
                p,
            )
            .map_err(|e| e.to_string())?;
    }
    let scale = mul(
        &mul(&mul(&b(scale)?, &b(speed)?, p)?, &time, p)?,
        &b(0x3a83126f)?,
        p,
    )?;
    let x = f(mul(&b(direction[0])?, &scale, p)?)?;
    let y = f(mul(&b(direction[1])?, &scale, p)?)?;
    let bx = s.word(0x114)?;
    let by = s.word(0x118)?;
    let dx = f(mul(&b(x)?, &b(by)?, p)?
        .add(&mul(&b(y)?, &b(bx)?, p)?, p)
        .map_err(|e| e.to_string())?)?;
    let dy = f(mul(&b(y)?, &b(by)?, p)?
        .sub(&mul(&b(x)?, &b(bx)?, p)?, p)
        .map_err(|e| e.to_string())?)?;
    Ok([
        f(b(dx)?
            .add(&b(s.word(0x24c)?)?, p)
            .map_err(|e| e.to_string())?)?,
        f(b(dy)?
            .add(&b(s.word(0x250)?)?, p)
            .map_err(|e| e.to_string())?)?,
    ])
}
fn apply_zoom_7005f0(
    s: &mut State,
    role_nonzero: bool,
    value: u32,
    zoom: &mut impl ZoomVariable,
    pc: Precision,
) -> Result<()> {
    if s.byte(0x214)? != 0 {
        return Ok(());
    }
    let direction = [s.word(0x114)?, s.word(0x118)?, s.word(0x11c)?];
    pose::zoom_6fe470(s, value, direction, pc)?;
    ni::update_6fe050(s, &mut ni::RequiredChildren, pc)?;
    if !role_nonzero {
        zoom.store_camera_scale_zoom_34(value)?;
    }
    Ok(())
}
fn typed_pointer(s: &mut State, offset: u32) {
    s.words[(offset / 4) as usize].known = 0;
    s.revision
        .fetch_add(1, std::sync::atomic::Ordering::Release);
}
fn equivalent(a: &State, b: &State) -> bool {
    a.require_ni_base_constructor().is_ok()
        && b.require_ni_base_constructor().is_ok()
        && (0..a.words.len()).all(|i| {
            let x = a.words[i];
            let y = b.words[i];
            x.known == y.known
                && (x.bits & x.known) == (y.bits & y.known)
                && a.word(4 * i as u32).ok() == b.word(4 * i as u32).ok()
        })
        && a.target_rectangle_alias_present() == b.target_rectangle_alias_present()
        && a.target_point_alias_present() == b.target_point_alias_present()
}
fn prefix_7016a0(s: &mut State, pc: Precision) -> Result<()> {
    for o in [0x21c, 0x220, 0x224, 0x228] {
        s.put(o, 0)
    }
    let rectangle =
        aa_navigation::uai_collision::swept_shape(0, 0, pc).map_err(|e| e.to_string())?;
    for (i, w) in rectangle.words().into_iter().enumerate() {
        s.put(0x258 + 4 * i as u32, w)
    }
    s.bind_target_rectangle_270_after_native_writer();
    for (o, w) in [
        (0x274, 0),
        (0x278, 0xbf800000),
        (0x27c, 0),
        (0x280, 0),
        (0x284, 0),
    ] {
        s.put(o, w)
    }
    for o in (0x288..=0x29c).step_by(4) {
        s.put(o, 0)
    }
    s.bind_target_point_2a0_after_native_writer();
    for (o, w) in [
        (0x2a4, 0),
        (0x2a8, 0xbf800000),
        (0x2ac, 0),
        (0x2b0, 0),
        (0x2b4, 0),
    ] {
        s.put(o, w)
    }
    for (o, w) in [
        (0x2d0, 0),
        (0x308, 0),
        (0x30c, 0),
        (0x304, 0x3f800000),
        (0x310, 0),
        (0x314, 0),
        (0x318, 0),
        (0x320, 0x3f800000),
        (0x324, 0),
        (0x328, 0),
        (0x32c, 0),
        (0x330, 0),
        (0x334, 0),
    ] {
        s.put(o, w)
    }
    for o in [0x31c, 0x338, 0x33c] {
        s.put_byte(o, 0)
    }
    for (o, w) in [
        (0x21c, 0xbf800000),
        (0x228, 0xbf800000),
        (0x220, 0x3f800000),
        (0x224, 0x3f800000),
    ] {
        s.put(o, w)
    }
    Ok(())
}

#[cfg(test)]
#[path = "native_camera_directory_tests.rs"]
mod tests;
