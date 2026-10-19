//! Draw transport for the same installed VWorld. A camera/render packet never
//! writes native loading, task, application-state or simulation-ready fields.
use crate::{
    battle_camera::{input as camera_input, orders_hud},
    battle_controller::{Controller, Status},
    battle_orders_input,
    gpu::{SceneRenderer, camera_device::Viewport},
    live_scene::camera_backend::Frame,
};
use aa_runtime::native_camera::input::Trace;
use eframe::egui;
use std::sync::{Arc, Mutex};

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
    orders_input: Vec<battle_orders_input::Observed>,
    orders: Arc<Mutex<battle_orders_input::Orders>>,
    last_orders_error: Option<String>,
    last_view_request_error: Option<String>,
    last_vworld_ad: Option<bool>,
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
            orders_input: Vec::new(),
            orders: Arc::new(Mutex::new(battle_orders_input::Orders::new())),
            last_orders_error: None,
            last_view_request_error: None,
            last_vworld_ad: None,
        }
    }
    ///Collect this UI frame's real input for the native camera controls.
    ///Call once per update before `poll`; frames queue until a job runs.
    pub fn observe_input(&mut self, ctx: &egui::Context) {
        let Some(pixels) = self.pixels else { return };
        self.input.push(camera_input::observe(ctx, ctx.content_rect(), pixels, self.presented));
        //The HUD owner supplies the native UI+0x60 verdict later; until then
        //the world is under the cursor unless an egui area/widget has it.
        let world_under_cursor = !(ctx.is_pointer_over_area() || ctx.wants_pointer_input());
        //Native client pixels of the presented image (equal to the content
        //rect unless the window aspect differs from the fixed target).
        let image = self.image_rect(ctx.content_rect());
        self.orders_input.push(battle_orders_input::observe(ctx, &mut self.orders_window, image, pixels, world_under_cursor));
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
            let Some(viewport) = self.viewport.as_ref().map(Arc::clone) else { return };
            let map_id=controller.request().map(|r|r.map_id.clone()).unwrap_or_default();
            let input = std::mem::take(&mut self.input);
            let orders_input = std::mem::take(&mut self.orders_input);
            let orders = Arc::clone(&self.orders);
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
                //The same UI frames' selection/order input, after the camera
                //controls (see battle_orders_hud.rs for the ordering).
                let (orders, orders_error) = if orders_input.is_empty() { (Vec::new(), None) } else {
                    let startup = loans.view_startup.as_mut().expect("checked above");
                    match native_orders_frames(&orders_input, &orders, loans.source, loans.session,
                        loans.notifications, loans.application_views, startup, vworld_ad) {
                        Ok(frames) => (frames, None),
                        Err(error) => (Vec::new(), Some(error)),
                    }
                };
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
                    camera_trace,camera_pose,input_error,input_status,orders,orders_error,view_request_error,vworld_ad}))
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
                log::info!("native orders frame: buttons={} events={} handled={} selected={} modal={:?} children={:?} box={} lock214={:?} error={}",
                    frame.buttons, r.events, r.handled, r.selected, frame.modal, frame.children, box_xy.unwrap_or_else(|| "none".into()),
                    frame.lock_214, r.error.as_deref().unwrap_or("none"));
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
    frames: &[battle_orders_input::Observed],
    orders: &Mutex<battle_orders_input::Orders>,
    source: &Arc<aa_rules::actors::ActorCatalog>,
    session: &mut aa_netcode::local_bootstrap::world_ready::ActualActivatedSession,
    notifications: &aa_netcode::local_bootstrap::Notifications,
    views: &mut crate::live_scene::world_owner::ApplicationViews,
    startup: &mut crate::battle_view_startup::State,
    vworld_ad: bool,
) -> Result<Vec<OrdersFrame>, String> {
    use aa_runtime::native_camera::screen;
    let pc = aa_numeric::Precision::Double53;
    let mut catalog = aa_runtime::catalog::Catalog::new(source);
    let resources = session.world().evidence().resource_source.as_ref()
        .ok_or("orders input actual Resource source owner absent")?;
    catalog.bind_resource_catalog(Arc::clone(resources))?;
    catalog.settings = Some(session.world().presentation_parts()?.settings.clone());
    catalog.native_load_mode = session.world().evidence().native_load_mode;
    let (host, world, application) = session.parts();
    let parts = world.presentation_parts()?;
    let revision = parts.revision;
    let threshold = orders_hud::cursor_drag_threshold_b39990(&parts.settings);
    let seeds = {
        let app = application.view_startup_parts(&catalog, parts.definitions)?;
        battle_orders_input::seed_b43cf8(app.inputs.seeds(), &catalog)?
    };
    let terrain_ready = {
        let height = startup.native_view().native().heightfield();
        let (_, visual) = views.camera_and_visual_map();
        visual.dimensions_for_world(revision, height)?.is_some()
    };
    //Admin+2C0 and Node+8 of the SAME node whose AdminNode receives the requests.
    let (running_2c0, local_player) = {
        let s = host.snapshot();
        (s.node_active, s.local_player)
    };
    let application_kind = notifications.application_kind_b396c0();
    let admin = host.order_node_mut().map_err(|e| e.to_string())?;
    let mut orders = orders.lock().map_err(|_| "orders input owner poisoned")?;
    let mut out = Vec::with_capacity(frames.len());
    for observed in frames {
        let viewport = [observed.client[0] as i32, observed.client[1] as i32];
        let (camera, _) = views.camera_and_visual_map();
        let slot = camera.controls().and_then(|n| n.controls().active());
        let before = orders.machine().tracker();
        let mut facing = orders_hud::UnboundFacing;
        let mut hud = orders_hud::Leaves::new(slot, vworld_ad, threshold.clone(), &mut facing);
        let report = {
            let h = camera.directory().current_handle()?;
            let state = camera.directory().camera(&h)?.normalized_state()?;
            let mut leaves = startup.order_views(world)?;
            let mut live = battle_orders_input::Live {
                views: &mut leaves, hud: &mut hud, admin: &mut *admin, running_2c0, local_player,
                camera: Some(state), viewport, application_kind, terrain_ready,
                //The live TerrainMap (723E00/723960/71B8F0/71B610) is not bound yet.
                terrain: None, seed_b43cf8: Some(seeds), precision: pc,
            };
            orders.frame(observed, &mut live)
        };
        let after = orders.machine().tracker();
        let children = hud.children().to_vec();
        let tracker = screen::Tracker { current: after.current, left_press: before.left_press };
        let apply_error = camera.orders_children(&children, tracker, viewport, Some(seeds), pc).err();
        let box_xy = camera.screen_box().and_then(|p| p.lock().ok().map(|p| {
            let v = p.vertices();
            [v[0][0], v[0][1], v[1][0], v[1][1], v[2][0], v[2][1], v[3][0], v[3][1]]
        }));
        let lock_214 = camera.directory().current_handle().and_then(|h| camera.directory().byte_captured(&h, 0x214)).ok();
        let buttons = observed.messages.iter().filter(|m| matches!(m, battle_orders_input::Message::Button { .. })).count();
        out.push(OrdersFrame { buttons, report, modal: orders.machine().modal_names(), children, apply_error, box_xy, lock_214 });
    }
    Ok(out)
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
}
