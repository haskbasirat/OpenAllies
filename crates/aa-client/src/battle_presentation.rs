//! Draw transport for the same installed VWorld. A camera/render packet never
//! writes native loading, task, application-state or simulation-ready fields.
use crate::{
    battle_camera::{input as camera_input, orders_hud},
    battle_controller::{Controller, Status},
    battle_menus::hud_orders,
    battle_orders_input,
    gpu::{SceneRenderer, camera_device::Viewport},
    live_scene::camera_backend::Frame,
};
use aa_runtime::native_camera::input::Trace;
use eframe::egui;
use std::sync::{Arc, Mutex};
#[path = "battle_terrain_owner.rs"]
mod terrain_owner;

struct Packet {
    frame: Arc<Frame>,
    world_revision: u32,
    init_word_b8: u32,
    map_id: String,
    ///Native camera controls of the input frame run before this snapshot.
    camera_trace: Vec<Trace>,
    camera_pose: Option<[u32; 6]>,
    input_error: Option<String>,
    ///Native gate/state summary after the queued frames (diagnostics only).
    input_status: Option<String>,
    ///One entry per queued orders UI frame (battle_orders_input::Orders).
    orders: Vec<OrdersFrame>,
    ///The orders owners could not be bound for this job.
    orders_error: Option<String>,
    ///43EC60 VWorld+208 request segment refusal (the frame loop goes on).
    view_request_error: Option<String>,
    ///VWorld+AD for this job: 43EC60 ran exactly when it is set.
    vworld_ad: bool,
    ///43FE0F/43FE26 construction or 4312F0/725340 refusal.
    terrain_error: Option<String>,
    terrain_status: Option<String>,
}

///One native UI frame of selection/order input: the window messages and the
///HUD owner's UI+0x60 facts for the same messages (UI thread, Send).
struct OrdersInput {
    observed: battle_orders_input::Observed,
    hud: hud_orders::HudFrame,
}
impl OrdersInput {
    ///See `camera_input::Frame::coalesce`: messages and press picks in order;
    ///Input+0xC keeps the last reported cursor (WM_MOUSEMOVE writes it), the
    ///HUD facts are the latest; UI+0x60 before the first press stays.
    fn coalesce(&mut self, next: OrdersInput) {
        let (o, h) = (next.observed, next.hud);
        self.observed.messages.extend(o.messages);
        self.observed.cursor = o.cursor.or(self.observed.cursor);
        self.observed.client = o.client;
        self.observed.world_under_cursor = o.world_under_cursor;
        self.hud.presses.extend(h.presses);
        self.hud.focus_is_game = h.focus_is_game;
        self.hud.mouse_bindings = h.mouse_bindings;
        self.hud.economic_impact = h.economic_impact;
        self.hud.unit_acknowledgement = h.unit_acknowledgement;
    }
}

///What one native orders UI frame did, for the log (Send).
struct OrdersFrame {
    ///Button messages observed in this UI frame.
    buttons: usize,
    report: battle_orders_input::FrameReport,
    modal: Vec<&'static str>,
    children: Vec<orders_hud::Child>,
    apply_error: Option<String>,
    ///XY words of the box polygon's four vertices while it exists.
    box_xy: Option<[u32; 8]>,
    lock_214: Option<u8>,
    ///Voice/marker requests the HUD leaves queued (no presentation owner).
    presentation: Vec<String>,
}

///The first actual window target remains fixed until a native camera resize
///operation is available. Window changes scale its image with aspect preserved;
///they cannot silently attach the old native projection to a new target ratio.
pub struct State {
    pixels: Option<[u32; 2]>,
    viewport: Option<Arc<Viewport>>,
    frame: Option<Packet>,
    #[cfg(windows)]
    pending: Option<aa_runtime::simulation_execution::JobTicket<Option<Packet>>>,
    error: Option<String>,
    presented: bool,
    ///UI frames observed since the last submitted camera job, in order.
    input: Vec<camera_input::Frame>,
    last_input_error: Option<String>,
    last_camera_log: Option<std::time::Instant>,
    last_status_log: Option<std::time::Instant>,
    ///The last image actually rendered from a current native snapshot. A
    ///snapshot superseded by a later native camera store (input processed
    ///by the next camera job) is never rendered; this image stays shown
    ///until that job's fresh snapshot arrives.
    last_texture: Option<egui::TextureId>,
    ///Selection/order input (research/phase8-orders-input): window message
    ///tracking, UI frames queued like `input`, and the per-battle machine.
    orders_window: battle_orders_input::Window,
    ///UI+0x60 across frames on the UI thread (HUD owner's press pick).
    press_tracker: hud_orders::PressTracker,
    ///At most one pending native frame (see OrdersInput::coalesce).
    orders_input: Vec<OrdersInput>,
    orders: Arc<Mutex<battle_orders_input::Orders>>,
    ///The HUD owner's per-battle simulation-thread state.
    hud_orders: Arc<Mutex<hud_orders::HudOrderState>>,
    ///observe_input ran without the HUD tree, so no orders frame was queued.
    orders_unrouted: bool,
    last_orders_error: Option<String>,
    last_view_request_error: Option<String>,
    last_vworld_ad: Option<bool>,
    ///B36518 TerrainMap + VWorld+94 visibility (simulation thread).
    terrain: Arc<Mutex<terrain_owner::Slot>>,
    last_terrain_error: Option<String>,
    last_terrain_status: Option<String>,
}
impl State {
    pub fn empty() -> Self {
        Self {
            pixels: None,
            viewport: None,
            frame: None,
            #[cfg(windows)]
            pending: None,
            error: None,
            presented: false,
            input: Vec::new(),
            last_input_error: None,
            last_camera_log: None,
            last_status_log: None,
            last_texture: None,
            orders_window: battle_orders_input::Window::default(),
            press_tracker: hud_orders::PressTracker::default(),
            orders_input: Vec::new(),
            orders: Arc::new(Mutex::new(battle_orders_input::Orders::new())),
            hud_orders: Arc::new(Mutex::new(hud_orders::HudOrderState::default())),
            orders_unrouted: false,
            last_orders_error: None,
            last_view_request_error: None,
            last_vworld_ad: None,
            terrain: Arc::new(Mutex::new(terrain_owner::Slot::Pending)),
            last_terrain_error: None,
            last_terrain_status: None,
        }
    }
    ///Collect this UI frame's real input for the native camera controls only.
    ///Selection/order input needs the HUD tree's press verdict: without it no
    ///orders frame is queued (see observe_input_hud).
    pub fn observe_input(&mut self, ctx: &egui::Context) {
        self.observe(ctx, None);
    }
    ///Collect this UI frame's camera and selection/order input. `ui` is the
    ///aa-ui HUD drawn over the battle image (0x5C3BC0 press pick). Call once
    ///per update before `poll`; frames observed while a job is pending join
    ///the next native frame (Frame::coalesce), so the queue stays one frame.
    pub fn observe_input_hud(&mut self, ctx: &egui::Context, ui: &aa_ui::ui::Ui) {
        self.observe(ctx, Some(ui));
    }
    fn observe(&mut self, ctx: &egui::Context, ui: Option<&aa_ui::ui::Ui>) {
        let Some(pixels) = self.pixels else { return };
        let frame = camera_input::observe(ctx, ctx.content_rect(), pixels, self.presented);
        match self.input.last_mut() {
            Some(last) => last.coalesce(frame),
            None => self.input.push(frame),
        }
        let Some(ui) = ui else {
            self.orders_unrouted = true;
            return;
        };
        //Native client pixels of the presented image (equal to the content
        //rect unless the window aspect differs from the fixed target).
        let image = self.image_rect(ctx.content_rect());
        //0x5C3BC0's UI+0x60 for each button message (VWorld, minimap or the
        //HUD widget's identity) from the aa-ui press pick.
        let observed = battle_orders_input::observe_with(ctx, &mut self.orders_window, image, pixels,
            &mut |p| battle_orders_input::widget_for_press_target(&ui.press_target_5c3bc0(p[0] as i16, p[1] as i16)));
        let hud = hud_orders::HudFrame::observe(ui, &mut self.press_tracker, &observed);
        let next = OrdersInput { observed, hud };
        match self.orders_input.last_mut() {
            Some(last) => last.coalesce(next),
            None => self.orders_input.push(next),
        }
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    ///Run before pumping the actual camera's frontend RPC. In particular the
    ///first Renderer+40 reference must see this real allocated window target.
    pub fn prepare(&mut self, renderer: &mut SceneRenderer, pixels: [u32; 2]) -> Result<(), String> {
        let pixels = *self.pixels.get_or_insert(pixels);
        //Upload changes the renderer epoch. Reacquire its real target receipt
        //at the same dimensions; no projection or camera state is replaced.
        if self.viewport.as_ref().is_none_or(|v| v.require_current().is_err()) {
            self.frame = None;
            self.last_texture = None;
            self.viewport = Some(Arc::new(
                renderer.prepare_camera_viewport(pixels).map_err(|e| e.to_string())?,
            ));
        }
        Ok(())
    }
    ///Only Send camera words/weak witnesses return from SimulationThread.
    ///The native VWorld/Directory/World stay on their retained thread.
    pub fn poll(&mut self, controller: &mut Controller) {
        #[cfg(windows)]
        {
            if let Some(ticket) = self.pending.as_mut() {
                match ticket.try_result() {
                    Ok(None) => return,
                    Ok(Some(frame)) => {
                        let fresh = frame.is_some();
                        if let Some(packet) = &frame { self.camera_diagnostics(packet); }
                        self.frame = frame; self.pending = None;
                        //Let draw() render this fresh snapshot before the next
                        //job can apply further native camera input.
                        if fresh { return; }
                    }
                    Err(error) => { log::error!("battle presentation camera job failed: {error}"); self.error = Some(error); self.pending = None; }
                }
            }
            if self.error.is_some() || controller.status() != &Status::AwaitingNativeView {
                return;
            }
            if std::mem::take(&mut self.orders_unrouted) && self.last_orders_error.is_none() {
                let error = "frontend calls observe_input without the HUD tree (observe_input_hud): no selection/order frame is queued";
                log::error!("native orders input not routed: {error}");
                self.last_orders_error = Some(error.into());
            }
            let Some(viewport) = self.viewport.as_ref().map(Arc::clone) else { return };
            let map_id=controller.request().map(|r|r.map_id.clone()).unwrap_or_default();
            let input = std::mem::take(&mut self.input);
            let orders_input = std::mem::take(&mut self.orders_input);
            let orders = Arc::clone(&self.orders);
            let hud_orders = Arc::clone(&self.hud_orders);
            let terrain = Arc::clone(&self.terrain);
            let bridge = controller.ui_backend();
            self.pending = match controller.submit_activated(move |loans, execution| {
                //A queued upload/target change can legitimately supersede a
                //previous UI request. Defer only that packet; never infer AD.
                if viewport.require_current().is_err() { return Ok(None) }
                if loans.view_startup.is_none() { return Ok(None) }
                {
                    let startup = loans.view_startup.as_ref().expect("checked above");
                    let world = loans.session.world();
                    let view = startup.native_view();
                    view.directory().require_world(world).map_err(|e| e.to_string())?;
                    let native = view.native();
                    use aa_runtime::view_init_world::Phase;
                    if matches!(native.phase(), Phase::Constructed | Phase::PrefixComplete |
                        Phase::SceneComplete | Phase::InitializingTerrainInputs | Phase::AwaitingTerrainBackend) {
                        return Ok(None);
                    }
                    native.scene().require_renderer(world.presentation_parts()?.revision)?;
                }
                //Native UI frame camera controls on the SAME camera before
                //this frame's snapshot. Their errors are reported, not hidden.
                let events: usize = input.iter().map(|f| f.events.len()).sum();
                let (camera_trace, input_error) = if input.is_empty() { (Vec::new(), None) } else {
                    let startup = loans.view_startup.as_ref().expect("checked above");
                    match native_camera_frames(&input, loans.source, loans.session,
                        loans.focus, loans.application_views, startup, &bridge, execution) {
                        Ok(trace) => (trace, None),
                        Err(error) => (Vec::new(), Some(error)),
                    }
                };
                let input_status = (events > 0).then(|| {
                    let (camera, _) = loans.application_views.camera_and_visual_map();
                    let flags = loans.focus.flags_b396c4().map(|f| format!("{f:#x}")).unwrap_or_else(|e| e);
                    let last = input.last();
                    format!("frames={} events={events} app34={flags} cursor={:?} windowed={:?} {}", input.len(),
                        last.and_then(|f| f.cursor), last.map(|f| f.windowed),
                        camera.controls().map(|c| c.status()).unwrap_or_default())
                });
                let vworld_ad = {
                    let startup = loans.view_startup.as_ref().expect("checked above");
                    let revision = loans.session.world().presentation_parts()?.revision;
                    startup.native_view().native().prefix().initialized_for(revision)?
                };
                let mut terrain = terrain.lock().map_err(|_| "battle TerrainMap owner poisoned")?;
                //43FE0F/43FE26: the B36518 TerrainMap and VWorld+94, built once
                //right after the visual heightfield publication.
                let mut terrain_error = {
                    let startup = loans.view_startup.as_ref().expect("checked above");
                    build_terrain_43fe0f(&mut terrain, loans.source, loans.session, loans.application_views, startup, &bridge, execution)
                };
                //The same UI frames' selection/order input, after the camera
                //controls (see battle_orders_hud.rs for the ordering). 0x71D840
                //sees the previous update's block residency, as natively.
                let (orders, orders_error) = if orders_input.is_empty() { (Vec::new(), None) } else {
                    let startup = loans.view_startup.as_mut().expect("checked above");
                    let owner = match &*terrain { terrain_owner::Slot::Built(o) => Some(&**o), _ => None };
                    match native_orders_frames(&orders_input, &orders, &hud_orders, loans.source, loans.session,
                        loans.notifications, loans.application_views, startup, vworld_ad, owner, &bridge, execution) {
                        Ok(frames) => (frames, None),
                        Err(error) => (Vec::new(), Some(error)),
                    }
                };
                //0x4312F0 (0x43ED0A) and 0x725340 (0x43ED15) once per VWorld
                //update while VWorld+AD, right before the 43EC60 request segment.
                //They move into the full 43EC60 driver (aa-runtime/src/
                //vworld_update.rs) when it lands, so they never run twice.
                let mut terrain_status = None;
                if vworld_ad {
                    if let terrain_owner::Slot::Built(owner) = &mut *terrain {
                        let startup = loans.view_startup.as_ref().expect("checked above");
                        match terrain_visibility_4312f0(owner, loans.session, loans.application_views, startup) {
                            Ok(()) => terrain_status = Some(owner.status()),
                            Err(error) => terrain_error = terrain_error.or(Some(error)),
                        }
                    }
                }
                drop(terrain);
                //43FF60 -> 43EC60 (VWorld+208 requests) once per UI frame while
                //VWorld+AD, after the camera children. A refusal is reported
                //and the frame loop continues.
                let view_request_error = if vworld_ad {
                    let startup = loans.view_startup.as_mut().expect("checked above");
                    startup.update_view_requests_43ec60(loans.source, loans.session, &bridge, execution, aa_numeric::Precision::Double53).err()
                } else { None };
                let startup = loans.view_startup.as_ref().expect("checked above");
                let world = loans.session.world();
                let native = startup.native_view().native();
                let revision = world.presentation_parts()?.revision;
                let (camera, _) = loans.application_views.camera_and_visual_map();
                let camera_pose = camera.current_pose_words().ok();
                let snapshot = camera.directory().snapshot_current_for_scene(
                    revision, native.scene(), viewport,
                )?;
                let frame=Frame::from_current(snapshot, revision).map_err(|e|e.to_string())?;
                Ok(Some(Packet{frame,world_revision:revision.value(),init_word_b8:native.prefix().init_word_b8(),map_id,
                    camera_trace,camera_pose,input_error,input_status,orders,orders_error,view_request_error,vworld_ad,
                    terrain_error,terrain_status}))
            }) {
                Ok(ticket) => Some(ticket),
                Err(error) => { log::error!("battle presentation camera job not submitted: {error}"); self.error = Some(error); None }
            };
        }
        #[cfg(not(windows))]
        let _ = controller;
    }
    ///SceneRenderer validates the exact native camera, actual scene and target
    ///again at draw time. A stale camera is discarded and queried afresh.
    pub fn draw(&mut self, renderer: &mut SceneRenderer) -> Option<egui::TextureId> {
        let Some(packet) = self.frame.as_ref() else { return self.last_texture };
        let frame = &packet.frame;
        let pixels = frame.viewport_pixels();
        if frame.require_current(pixels).is_err() {
            self.frame = None;
            return self.last_texture;
        }
        match renderer.render_native(frame, pixels) {
            Ok((texture, _)) => {
                if !self.presented {
                    log::info!("actual battle native frame presented: selected_map={} world_revision={} init_word_b8={} camera_pixels={}x{}",
                        packet.map_id,packet.world_revision,packet.init_word_b8,pixels[0],pixels[1]);
                    self.presented=true;
                }
                self.last_texture = Some(texture);
                Some(texture)
            },
            //A native camera store between the check above and the renderer's
            //own validation supersedes the snapshot: discard it, not an error.
            Err(_) if frame.require_current(pixels).is_err() => { self.frame = None; self.last_texture }
            Err(error) => { log::error!("battle presentation render failed: {error}"); self.error = Some(error.to_string()); self.frame = None; None }
        }
    }
    pub fn image_rect(&self, available: egui::Rect) -> egui::Rect {
        aspect_fit(available, self.pixels.unwrap_or([1, 1]))
    }
    fn orders_diagnostics(&mut self, packet: &Packet) {
        let once = |slot: &mut Option<String>, what: &str, error: &str| {
            if slot.as_deref() != Some(error) {
                log::error!("{what}: {error}");
                *slot = Some(error.to_owned());
            }
        };
        if self.last_vworld_ad != Some(packet.vworld_ad) {
            self.last_vworld_ad = Some(packet.vworld_ad);
            log::info!("native VWorld+AD = {}: 43EC60 view request segment {}", packet.vworld_ad,
                if packet.vworld_ad { "runs once per presentation job" } else { "skipped" });
        }
        if let Some(error) = &packet.terrain_error {
            if self.last_terrain_error.as_deref() != Some(error) {
                log::error!("native battle TerrainMap/visibility refused: {error}");
                self.last_terrain_error = Some(error.clone());
            }
        }
        if packet.terrain_status.is_some() && packet.terrain_status != self.last_terrain_status {
            log::info!("native terrain visibility 4312F0: {}", packet.terrain_status.as_deref().unwrap_or(""));
            self.last_terrain_status = packet.terrain_status.clone();
        }
        if let Some(error) = &packet.view_request_error {
            once(&mut self.last_view_request_error, "native view request queue 43EC60 refused", error);
        }
        if let Some(error) = &packet.orders_error {
            once(&mut self.last_orders_error, "native orders input not bound", error);
        }
        for frame in &packet.orders {
            if let Some(error) = &frame.report.error {
                once(&mut self.last_orders_error, "native orders input refused", error);
            }
            if let Some(error) = &frame.apply_error {
                once(&mut self.last_orders_error, "native orders camera child refused", error);
            }
            let r = &frame.report;
            if frame.buttons > 0 || r.events > 0 || !frame.children.is_empty() {
                let f = |w: u32| f32::from_bits(w);
                let box_xy = frame.box_xy.map(|w| format!("[{:.4},{:.4}]-[{:.4},{:.4}]", f(w[0]), f(w[1]), f(w[4]), f(w[5])));
                log::info!("native orders frame: buttons={} events={} handled={} selected={} modal={:?} children={:?} box={} lock214={:?} unpresented={:?} error={}",
                    frame.buttons, r.events, r.handled, r.selected, frame.modal, frame.children, box_xy.unwrap_or_else(|| "none".into()),
                    frame.lock_214, frame.presentation, r.error.as_deref().unwrap_or("none"));
            }
        }
    }
    fn camera_diagnostics(&mut self, packet: &Packet) {
        self.orders_diagnostics(packet);
        if let Some(error) = &packet.input_error {
            if self.last_input_error.as_ref() != Some(error) {
                log::error!("native camera input refused: {error}");
                self.last_input_error = Some(error.clone());
            }
        }
        if let Some(status) = &packet.input_status {
            let now = std::time::Instant::now();
            if self.last_status_log.is_none_or(|t| now.duration_since(t).as_millis() >= 1000) {
                self.last_status_log = Some(now);
                log::info!("native camera input frames: {status}");
            }
        }
        if packet.camera_trace.is_empty() {
            return;
        }
        let now = std::time::Instant::now();
        if self.last_camera_log.is_some_and(|t| now.duration_since(t).as_millis() < 250) {
            return;
        }
        self.last_camera_log = Some(now);
        let f = |w: u32| f32::from_bits(w);
        let pose = packet.camera_pose.map(|p| format!(
            "target=({:.4},{:.4},{:.4}) zoom={:.4} yaw={:.4} pitch={:.4}", f(p[0]), f(p[1]), f(p[2]), f(p[3]), f(p[4]), f(p[5])));
        log::info!("native camera input: ops={:?} {}", packet.camera_trace, pose.unwrap_or_default());
    }
}

///The queued UI frames, each one native UI frame, through the ported controls
///(battle_camera::Process) on the SAME camera, in observation order.
#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn native_camera_frames(
    frames: &[camera_input::Frame],
    source: &Arc<aa_rules::actors::ActorCatalog>,
    session: &mut aa_netcode::local_bootstrap::world_ready::ActualActivatedSession,
    focus: &aa_runtime::application_window_state::Reader,
    views: &mut crate::live_scene::world_owner::ApplicationViews,
    startup: &crate::battle_view_startup::State,
    bridge: &crate::battle_ui_bridge::Client,
    execution: &mut aa_runtime::simulation_execution::Execution<'_>,
) -> Result<Vec<Trace>, String> {
    let mut catalog = aa_runtime::catalog::Catalog::new(source);
    let resources = session.world().evidence().resource_source.as_ref()
        .ok_or("camera input actual Resource source owner absent")?;
    catalog.bind_resource_catalog(Arc::clone(resources))?;
    catalog.settings = Some(session.world().presentation_parts()?.settings.clone());
    catalog.native_load_mode = session.world().evidence().native_load_mode;
    let (host, world, application) = session.parts();
    let parts = world.presentation_parts()?;
    let app = application.view_startup_parts(&catalog, parts.definitions)?;
    let native = startup.native_view().native();
    let height = native.heightfield();
    let revision = parts.revision;
    let vworld_ad = native.prefix().initialized_for(revision)?;
    let (camera, visual) = views.camera_and_visual_map();
    let visual_map = visual.dimensions_for_world(revision, height)?.is_some();
    let op = crate::battle_camera::Operation::new(&catalog, world, &*app.inputs, visual, height, host,
        bridge, execution, aa_numeric::Precision::Double53)?;
    let mut trace = Vec::new();
    for frame in frames {
        let mut gates = camera_input::Gates { focus_flags: focus, vworld_ad, visual_map, frame };
        trace.extend(camera.input_frame(&op, frame, &mut gates)?);
    }
    Ok(trace)
}

///The queued UI frames' selection/order input (battle_orders_input) on the
///SAME activated AdminNode, view directory and current camera, each frame
///followed by its box/facing camera children (battle_orders_hud.rs).
#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn native_orders_frames(
    frames: &[OrdersInput],
    orders: &Mutex<battle_orders_input::Orders>,
    hud_orders: &Mutex<hud_orders::HudOrderState>,
    source: &Arc<aa_rules::actors::ActorCatalog>,
    session: &mut aa_netcode::local_bootstrap::world_ready::ActualActivatedSession,
    notifications: &aa_netcode::local_bootstrap::Notifications,
    views: &mut crate::live_scene::world_owner::ApplicationViews,
    startup: &mut crate::battle_view_startup::State,
    vworld_ad: bool,
    terrain: Option<&terrain_owner::Owner>,
    bridge: &crate::battle_ui_bridge::Client,
    execution: &mut aa_runtime::simulation_execution::Execution<'_>,
) -> Result<Vec<OrdersFrame>, String> {
    use aa_runtime::native_camera::screen;
    let pc = aa_numeric::Precision::Double53;
    let mut catalog = aa_runtime::catalog::Catalog::new(source);
    let resources = session.world().evidence().resource_source.as_ref()
        .ok_or("orders input actual Resource source owner absent")?;
    catalog.bind_resource_catalog(Arc::clone(resources))?;
    catalog.settings = Some(session.world().presentation_parts()?.settings.clone());
    catalog.native_load_mode = session.world().evidence().native_load_mode;
    let session_e4 = session.session_state_e4();
    //Session/UI words of the view leaves, read before the AdminNode loan.
    let context = startup.order_context(session, bridge, execution);
    let (host, world, application) = session.parts();
    let parts = world.presentation_parts()?;
    let revision = parts.revision;
    let threshold = orders_hud::cursor_drag_threshold_b39990(&parts.settings);
    let seeds = {
        let app = application.view_startup_parts(&catalog, parts.definitions)?;
        battle_orders_input::seed_b43cf8(app.inputs.seeds(), &catalog)?
    };
    //B36518 != 0 && +BC. Before the map exists (or after its refused
    //construction) the published visual map stands for B36518.
    let terrain_ready = match terrain {
        Some(owner) => owner.loaded_bc()?,
        None => {
            let height = startup.native_view().native().heightfield();
            let (_, visual) = views.camera_and_visual_map();
            visual.dimensions_for_world(revision, height)?.is_some()
        }
    };
    //Admin+2C0 and Node+8 of the SAME node whose AdminNode receives the requests.
    let (running_2c0, local_player) = {
        let s = host.snapshot();
        (s.node_active, s.local_player)
    };
    let application_kind = notifications.application_kind_b396c0();
    let admin = host.order_node_mut().map_err(|e| e.to_string())?;
    let mut orders = orders.lock().map_err(|_| "orders input owner poisoned")?;
    let mut hud_state = hud_orders.lock().map_err(|_| "HUD orders owner poisoned")?;
    let mut out = Vec::with_capacity(frames.len());
    for OrdersInput { observed, hud: hud_frame } in frames {
        let viewport = [observed.client[0] as i32, observed.client[1] as i32];
        let (camera, _) = views.camera_and_visual_map();
        let slot = camera.controls().and_then(|n| n.controls().active());
        let before = orders.machine().tracker();
        let mut facing = orders_hud::UnboundFacing;
        let mut hud_leaves = hud_orders::HudOrderLeaves::new(&mut hud_state, hud_frame, observed,
            application_kind, Ok(session_e4), Some(&catalog));
        let mut hud = orders_hud::Leaves::new(slot, vworld_ad, threshold.clone(), &mut facing, &mut hud_leaves);
        let report = {
            let h = camera.directory().current_handle()?;
            let state = camera.directory().camera(&h)?.normalized_state()?;
            let mut leaves = startup.order_views_with(world, &context)?;
            let mut ray = terrain.map(|o| battle_orders_input::TerrainRay(&o.map));
            let mut live = battle_orders_input::Live {
                views: &mut leaves, hud: &mut hud, admin: &mut *admin, running_2c0, local_player,
                camera: Some(state), viewport, application_kind, terrain_ready,
                //0x71D840 on B36518 (margin 0).
                terrain: ray.as_mut().map(|r| r as &mut dyn battle_orders_input::TerrainQuery),
                seed_b43cf8: Some(seeds), precision: pc,
            };
            orders.frame(observed, &mut live)
        };
        let after = orders.machine().tracker();
        let children = hud.children().to_vec();
        drop(hud);
        let presentation = hud_state.take_presentation().iter().map(|p| format!("{p:?}")).collect();
        let tracker = screen::Tracker { current: after.current, left_press: before.left_press };
        let apply_error = camera.orders_children(&children, tracker, viewport, Some(seeds), pc).err();
        let box_xy = camera.screen_box().and_then(|p| p.lock().ok().map(|p| {
            let v = p.vertices();
            [v[0][0], v[0][1], v[1][0], v[1][1], v[2][0], v[2][1], v[3][0], v[3][1]]
        }));
        let lock_214 = camera.directory().current_handle().and_then(|h| camera.directory().byte_captured(&h, 0x214)).ok();
        let buttons = observed.messages.iter().filter(|m| matches!(m, battle_orders_input::Message::Button { .. })).count();
        out.push(OrdersFrame { buttons, report, modal: orders.machine().modal_names(), children, apply_error, box_xy, lock_214, presentation });
    }
    Ok(out)
}

///43FE0F/43FE26 once the visual heightfield is published: returns the
///construction refusal the first time it happens.
#[cfg(windows)]
fn build_terrain_43fe0f(
    slot: &mut terrain_owner::Slot,
    source: &Arc<aa_rules::actors::ActorCatalog>,
    session: &mut aa_netcode::local_bootstrap::world_ready::ActualActivatedSession,
    views: &mut crate::live_scene::world_owner::ApplicationViews,
    startup: &crate::battle_view_startup::State,
    bridge: &crate::battle_ui_bridge::Client,
    execution: &mut aa_runtime::simulation_execution::Execution<'_>,
) -> Option<String> {
    if !matches!(slot, terrain_owner::Slot::Pending) {
        return None;
    }
    let built = (|| -> Result<Option<terrain_owner::Owner>, String> {
        let native = startup.native_view().native();
        let height = native.heightfield();
        let revision = session.world().presentation_parts()?.revision;
        let (_, visual) = views.camera_and_visual_map();
        if visual.dimensions_for_world(revision, height)?.is_none() {
            return Ok(None);
        }
        let height = height.ok_or("43FE0F VWorld+BC heightfield absent")?;
        let mut catalog = aa_runtime::catalog::Catalog::new(source);
        let resources = session.world().evidence().resource_source.as_ref()
            .ok_or("43FE26 actual Resource source owner absent")?;
        catalog.bind_resource_catalog(Arc::clone(resources))?;
        catalog.settings = Some(session.world().presentation_parts()?.settings.clone());
        catalog.native_load_mode = session.world().evidence().native_load_mode;
        let (_, world, application) = session.parts();
        let parts = world.presentation_parts()?;
        let app = application.view_startup_parts(&catalog, parts.definitions)?;
        let quad = || bridge.call(&mut *execution, |c| c.ui.uvars.terrain_quad_size_current());
        terrain_owner::build_43fe0f(world, height, app.inputs.grid(), quad, aa_numeric::Precision::Double53).map(Some)
    })();
    match built {
        Ok(Some(owner)) => {
            log::info!("native battle TerrainMap 43FE26 built: {}", owner.status());
            *slot = terrain_owner::Slot::Built(Box::new(owner));
            None
        }
        Ok(None) => None,
        Err(error) => {
            *slot = terrain_owner::Slot::Failed(error.clone());
            Some(format!("43FE0F/43FE26 TerrainMap construction: {error}"))
        }
    }
}

///0x4312F0 + 0x725340 on the current camera, the SAME World, VWorld+BC and
///the VWorld+94 VGrid.
#[cfg(windows)]
fn terrain_visibility_4312f0(
    owner: &mut terrain_owner::Owner,
    session: &aa_netcode::local_bootstrap::world_ready::ActualActivatedSession,
    views: &mut crate::live_scene::world_owner::ApplicationViews,
    startup: &crate::battle_view_startup::State,
) -> Result<(), String> {
    let native = startup.native_view().native();
    let height = native.heightfield().ok_or("4312F0 VWorld+BC heightfield absent")?;
    let grid = native.prefix().grid().ok_or("4312F0 VWorld+94 VGrid absent")?;
    let (camera, _) = views.camera_and_visual_map();
    let words = aa_runtime::terrain_visibility::CameraWords::current(camera.directory())?;
    owner.update_4312f0_725340(session.world(), height, grid, &words, aa_numeric::Precision::Double53)
}

fn aspect_fit(available: egui::Rect, pixels: [u32; 2]) -> egui::Rect {
    let source = egui::vec2(pixels[0] as f32, pixels[1] as f32);
    let scale = (available.width() / source.x).min(available.height() / source.y);
    egui::Rect::from_center_size(available.center(), source * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn window_resize_preserves_the_actual_native_target_aspect() {
        let wide = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(1600.0, 900.0));
        let fit = aspect_fit(wide, [800, 600]);
        assert_eq!(fit.size(), egui::vec2(1200.0, 900.0));
        assert_eq!(fit.center(), wide.center());
        let tall = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 1000.0));
        assert_eq!(aspect_fit(tall, [1600, 900]).size(), egui::vec2(600.0, 337.5));
    }
    #[test]
    fn frames_queued_behind_a_job_join_one_native_frame_in_order() {
        use camera_input::{Event, Frame};
        let key = |code, flags| Event::Key { code, flags };
        let mut f = Frame { events: vec![key(0x27, 1)], cursor: Some([1, 2]), client: [800, 600], presented: false, windowed: true, counter: Some(10) };
        f.coalesce(Frame { events: vec![key(0x27, 2), Event::Wheel { delta: 120, flags: 0x81 }], cursor: None, client: [800, 600], presented: true, windowed: false, counter: Some(25) });
        assert_eq!(f.events, [key(0x27, 1), key(0x27, 2), Event::Wheel { delta: 120, flags: 0x81 }]);
        assert_eq!((f.cursor, f.presented, f.windowed, f.counter), (None, true, false, Some(25)));
        use aa_runtime::player_order_input::states::Widget;
        use aa_ui::press_pick::PressTarget;
        let button = |code, flags, x| battle_orders_input::Message::Button { code, flags, x, y: 5, widget: Widget::World };
        let pick = |code, x| hud_orders::PressPick { code, flags: 1, x, y: 5, target: PressTarget::VWorld };
        let hud = |presses| hud_orders::HudFrame { start_60: PressTarget::None, presses, focus_is_game: true,
            mouse_bindings: Ok(()), economic_impact: false, unit_acknowledgement: Ok(true) };
        let mut o = OrdersInput {
            observed: battle_orders_input::Observed { messages: vec![button(0x100, 1, 3)], cursor: Some([3, 5]), client: [800, 600], world_under_cursor: true },
            hud: hud(vec![pick(0x100, 3)]),
        };
        let mut next_hud = hud(vec![pick(0x101, 9)]);
        next_hud.start_60 = PressTarget::VWorld;
        next_hud.economic_impact = true;
        o.coalesce(OrdersInput {
            observed: battle_orders_input::Observed { messages: vec![button(0x100, 2, 4), button(0x101, 1, 9)], cursor: None, client: [800, 600], world_under_cursor: false },
            hud: next_hud,
        });
        assert_eq!(o.observed.messages, [button(0x100, 1, 3), button(0x100, 2, 4), button(0x101, 1, 9)]);
        assert_eq!((o.observed.cursor, o.observed.world_under_cursor), (Some([3, 5]), false));
        assert_eq!(o.hud.presses, [pick(0x100, 3), pick(0x101, 9)]);
        assert_eq!((o.hud.start_60.clone(), o.hud.economic_impact), (PressTarget::None, true));
    }
}
