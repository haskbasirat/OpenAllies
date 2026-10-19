Phase 8: in-game camera input (KKC_UI camera controls) - 2026-10-19

Native input path (AA.exe, base 0x400000)
- Hotkeys: Data/Localization/Hotkeys/hotkeys_core.txt (start.txt runs it first).
  map 5CA4B0 / key spec 5C46A0 (prefix 4098A0, modifiers 409CF0, names 409960
  over KKC_KeyNameList B27B20). Non-'+' down bindings get an automatic key-up
  binding: '+cmd' -> '-cmd' (5C9ED0/5C9D00) else DefaultKeyUpHandler (943800,
  RET 4). Events 5EFC70 (repeat ignored, key {code, flags&0xFFFFF07F}); wheel
  code 0x110 formats "deltazoom %f" with (float)(short)delta/1200.0 (A42868).
  '+name'/'-name' -> RegisterImpulse/UnregisterImpulse (426950).
- Camera bindings in the original file: arrows -> Begin/EndScroll{Left,Right,
  Up,Down}; mousewheel -> deltazoom; numpad-left/right/up/down -> +yawleft/
  +yawright/+pitchup/+pitchdown; numpad-5 DefaultAngles; +/-button3 (middle)
  -> Begin/EndCameraRotate; pageup/numpad-9 +zoomin, pagedown/numpad-3 +zoomout;
  home/end (context game) ZoomMin/ZoomMax.
- Scroll: UI commands 5C12C0..5C1360 -> UI+40 scroller (vtable A4DA84, ctor
  5CD2E0): 5CD560 key bit (L4 R8 U1 D2), 5CCD30 clear, 5CC7D0 direction,
  5CCC90 screen-edge bits against KKC_Input rect (0,0,W-1,H-1; 59AEB0 from
  40E210) with the UI+58 cursor copied by 5BF2B0 from the 59AD30 poll.
  Per-frame 5CD320: QPC microseconds (59AAD0) clamped [10000,100000] (401290),
  _ftol2(t*0.001) ms, 5CCC20 acceleration (accumulator +20, 1e-6*UVar1C
  CameraScrollAcceleration + UVar18 CameraScrollSpeed, capped by UVar20
  CameraScrollSpeedMax, divided by UVar18), then camera 700500 (UVar28
  CameraScalePan * UVar18 * ms * 0.001, rotated by basis 114/118, added to the
  target) -> 6FF130 -> 6FEDB0 (map rectangle minus UVar24 margin clamp and
  550210 heightfield) -> 6FE470 zoom -> 6FE050 NiCamera. 59AB60 resets after.
  Edge activation: game UI state update 5CD6B0 (vtable A73CD0, built by
  6B2CC0) -> 5CD420 (App+48 windowed && !App+49 coop -> no; VWorld hover
  5EFA80; rotate inactive). Scroller mouse-move 5CD410 recomputes bits.
- Zoom: DeltaZoom 43BC80 (426330 parses _wtof in [-1,1], else throws) * -1.0
  -> 7010F0 (zoom208 + delta) -> 7005F0 (6FE470 clamp to 20C/210, stores
  UVar34 CameraScaleZoom). Impulses ZoomIn/ZoomOut 5C29B0/5C2A10:
  u32(ms)*UVar34*UVar18*(-/+0.001). ZoomMin 43BC20 / ZoomMax 43BC50:
  1/(1/limit).
- Rotation/tilt: yaw 5C2930/5C2950 -> 5C2090 (UVar30 ScaleYaw) -> 700440;
  pitch 5C2970/5C2990 -> 5C20F0 (UVar2C ScalePitch, AVar50 range then AVar54
  CameraPitchMin, degrees) -> 7004A0; both -> 6FF220 whose 6FEA20 constraint
  forces yaw=AVar48 unless UVar B4 CameraRotate and pitch=AVar50 outside the
  developer camera, so they are no-ops in default gameplay. Middle-drag:
  5C21C0 (needs CameraRotate or developer camera) -> 5CCEB0 rate UVar38*DEG2RAD,
  5CCF00 mouse-move: yaw -= dx*rate -> 6FEBA0 (rotate about target) + 6FF200.
- Impulse list: KKC_Input B39B18 +2C/+30, timer +38 (since construction);
  tick 599940 -> 5995F0 dt (now<last -> 10000; extended diff > 100000 ->
  100000; binary32 diff < 10000 -> 10000) then tail 59AD30 cursor poll.
  40DA30 OnDeactivate: 59AF40 releases held keys/buttons, 599930 clears list.

Ported (Rust)
- crates/aa-runtime/src/native_camera_input.rs: Controls (scroller, rotate,
  impulses, UI+C0 slot for the two camera states, KKC_Input cursor), 5CCC20,
  5995F0, 5C2090/5C20F0/5C29B0, 700440/7004A0/7010F0, 43BC80/43BC20/43BC50,
  5EFCCC wheel argument, QPC timer 59AB60/59AAD0, frame 5C92F0 subset.
- crates/aa-runtime/src/native_camera_hotkeys.rs: hotkeys_core.txt camera
  bindings, key table B27B20, auto key-up rule, command parsing.
- native_camera_pose.rs: 6FEBA0. native_camera_directory.rs: captured
  word/byte reads, 6FE9C0, 6FF220 and 6FEBA0 on the SAME normalized camera.
- aa-client battle_camera.rs/battle_camera_input.rs/battle_presentation.rs:
  egui -> KKC_Input events, hotkeys and one native UI frame per presentation
  job on the SimulationThread camera owner before the frame snapshot.

Proof
- native_camera_input.py runs the ORIGINAL code under Unicorn 2.1.4 at
  PC24/PC53/PC64 (fixture native-camera-input.json, 1002 rows, 986 executed
  instruction pins byte-checked against the v14 image): 399 acceleration rows,
  36 impulse-dt rows, 30 scroller state-machine sequences (key holds, opposite
  keys, camera lock, cursor outside, edge scroll at all edges, hover/windowed/
  coop/visual-map/VWorld gates, accumulator reset), 384 impulse/setter rows,
  114 zoom command rows, 30 wheel rows, 9 drag-rotation sequences.
  Seams: QueryPerformanceCounter IAT 9F021C (scripted counters), 700500,
  6FF220, 7005F0, 6FEBA0, 6FF130, 426330 (returns the parsed binary32),
  40A610/426FB0 (wheel), VWorld+13C hover pick. All other bodies, including
  the native vtables/state slot 5E0570/5E0590/5C1FD0, 401290 and _ftol2, run.
- Rust replay: native_camera_input_tests.rs (8 tests incl. hotkey parse of the
  original file) all pass; a fixture mutation makes 3 of them fail. First run
  in target/agent-camera/isolated-input (the unchanged production files via
  #[path]) while the shared aa-runtime lib test target was broken by other
  sessions; later `cargo test -p aa-runtime --lib native_camera` in
  target/agent-camera: 35 passed, 0 failed, 1 ignored (all camera tests,
  including the input replays, the 6FEBA0 replay and the directory test).

- native_camera_rotate.py: 6FEBA0 with its real 6FE3B0/870720/FSIN/6FE470
  children (6FE050 seamed, FSIN/FCOS from this CPU's x87 via hardware_trig),
  84 rows (14 cases x original/v14 x PC24/53/64), 406 pins, 780 transports
  (fixture native-camera-rotate.json); Rust replay native_camera_rotate_tests.rs.
- Directory integration test (native_camera_directory_tests.rs
  controls_on_directory): the controls move a real lab Directory camera by the
  native pan, the 6FEDB0 rectangle bounds a 60 s right+down hold and wheel
  DeltaZoom clamps at the gameplay minimum. The clamp/ground tracking is
  gated by Camera+33C (render-list camera, 701847): a 60 s right+down hold
  stops at x=251 of the 5..251 rectangle.

Real GUI run (qa-output/phase8-camera-input/camera-run04/berlin, summary
real-run-04.json; private mirror build = real sources + the proposed
frontend-routing.patch line; run_camera_gui.py drives --ui-script)
- Frame presented at +5 min. Sequence 0 (+400 s, cursor in the window):
  60 ArrowRight presses -> 60 native 10 ms pans (each press frame clamps dt
  to 10000 us), target (52.11,376.11) -> diagonal +x+y (yaw -45 deg);
  60 ArrowDown -> +x-y; 60 PageDown (+zoomout impulse) zoom 0.70 -> 2.10
  (clamped at gameplay max AVar40); F11 fullscreen + cursor on the left edge
  for 3 s -> continuous accelerated edge scroll, target slides until the
  6FEDB0 rectangle clamps x=5.0, then along the edge to y=142.4; Z follows
  550210 heights (2.0..2.99). Screenshots cam00-*.png show the move, the
  zoom-out and the map's west border.
- Sequence 1 (+560 s): the OS cursor had left the window after F11 back, so
  egui reported no pointer -> KKC_Input+24 inside=0 -> 5CD320 issues no pan
  for the 120 arrow presses (the native gate); zoom impulses at the 2.10
  limit still run; edge scroll drives the target into the corner (5,5).
- No camera/presentation errors; the only ERROR line is the unrelated
  battle_controller view-initialization stop.

Real-run integration findings (fixed in battle_presentation/battle_camera_input)
- Input frames must stay separate native UI frames: merging a key press and
  its release (the --ui-script hook sends them in consecutive frames) into one
  job made BeginScroll/EndScroll cancel before the scroller update. Each queued
  frame now runs as its own 5C92F0 frame with its observed QPC counter.
- The 59AD30 rectangle is the whole client window; mapping the cursor onto the
  letterboxed image made the fullscreen screen edge fall outside it.
- A snapshot superseded by the next camera job's store was a fatal renderer
  error; it is now discarded and the last rendered image kept until the fresh
  snapshot arrives, and a received snapshot is drawn before the next job.

Not proven / open
- _wtof/vswprintf %f of the wheel argument use Rust's correctly rounded
  decimal conversion (assumed equal for these short fixed-point strings).
- AVar50 range (+14/+18) has no ported owner: pitch impulses return an error.
- UVars 1C/20/2C/30/38 are not in aa-ui's strict camera fast table; the client
  reads 1C/20 from the same UI owner's UVars.tgi overlay (refused outside the
  41F9D0 range); 2C/30/38 are unregistered there (explicit errors).
- Frontend facts without a ported owner use documented provisional mappings
  in battle_camera_input.rs (VWorld widget enabled/hover, contexts, App+48/49).
- UI+C0 contention with non-camera UI states, UI+BC/+C4 states, the VWorld+13C
  hover pick, editbox-context hotkey priority and numpad keys are not ported.

Orders integration: camera/presentation side of HudLeaves (2026-10-19)
=====================================================================
Native paths (AA.exe)
- UI+0xC0 slot as the orders machine sees it: 5E0570/5E0590 install the UI+40
  scroller (A4DA84) or the UI+48 rotate state (A4D968). Their vt+0x10
  (5CD410 / 5CCF00) return 0 and run, with vt+8 (5CD320), in the camera frame
  (Controls::frame_5c92f0), so HudLeaves::camera_frame_10 is Ok(false) and
  camera_frame_8 Ok(()); camera_slot_c0 is Generic exactly while
  Controls::active() holds one of them.
- *[B39990+0x60]: AVar 24 "CursorDragThreshold", 401BD0 at 402894..4028C2
  (401650 int, default 10, flags 0; 4B3470 index 0x18). Flags 0 = every
  AVars*.tgi declaration assigns it in load order (v14: AVars.tgi line 33,
  "int CursorDragThreshold = 10"). Read from the SAME Settings rows; only plain
  decimal literals (other spellings need the unported native int parser).
- Drag box (GameInterface+0xE0, A575A4):
  5CD850 enter: 5E0780 push, 599870 press copy (+8), [B40048]+214 = 1, four
    vertices = B43CF8/B43CFC/B43D00, colors (0,0,0.5,0.3) (5CD903..5CD927),
    new(2C) 887F40(4, vertices, NULL, colors) -> +4, 5CCA80, then into the
    polygon's 895550 property state by vt+34 class: NiVertexColorProperty
    A44EFC (+24 source = 1, +28 lighting = 0), NiAlphaProperty A4D92C (+20
    0xEC|1 = 0xED: blend, SRCALPHA/INVSRCALPHA, +22 = 0) and
    NiMaterialProperty 87D280 (+58 alpha 0.5, +5C revision + 1, one process
    material id B323C0), then 5CD7E0 on *B40048 (RTTI names from the static
    RTTI initializers: AB8F90 "NiVertexColorProperty", AB91D0 "NiAlphaProperty").
  5CCA80 (also 5CCC10 = box vt+0x10): tracker UI+58 (+4/+8 cursor, +C/+10
    left press), A = f32(min_x/W), B = max_x/W (FIDIVR, extended until its
    store), H stored as binary32 first, C = 1 - min_y/H, D = 1 - max_y/H;
    vertices (A,C) (A,D) (B,D) (B,C), Z untouched, polygon+20 |= 1.
  5CD7E0: 5CD730 = first NULL slot of the Camera+168 NiTArray, else append at
    +C (grow 5CCFE0 by +14 = 5; NiCamera 874B7D..874BC9 starts capacity 5),
    then polygon+20 |= 1.
  5CCA30 exit: [B40048]+214 = 0, 874350 (first equal slot -> 882300 NULL,
    size kept) on the CURRENT camera, release +4, tail VWorld+13C vt+44
    (the selection manager's 6406B0, run by the orders machine).
  Draw: the camera render 6FF63E..6FF672 walks +168 below +174 and calls
    887E60 (renderer+10 = polygon+C, +14 = 0) -> renderer vt+D8
    (NiDX9Renderer screen-polygon path). NOT PORTED: the GPU raster of
    camera screen polygons belongs to the renderer owner (see open gaps).
- Facing drag (GameInterface+0xE4, A5C580): 689490 = 689380 (ghosts per
  selected view) + 5CCD60 (5E0780, [B40048]+214 = 1, +8 = 0, embedded arrow
  +18 vt+2C(0), +C..+14 = 0); 688AE2..688B25 drag start (arrow vt+2C of
  43CCA0(VWorld+1CC, pointer +19C/+1B4), state vt+44/48); per frame 688A43
  (+1C vt+6C) and 693D10 (ghosts); 6885F0 = 693EB0 (ghost teardown) + 5CCDA0
  (arrow vt+2C(0), +214 = 0). The ghosts and the arrow are VWorld scene
  objects (view owner): explicit FacingScene errors, reached BEFORE the camera
  lock exactly as 689380 precedes 5CCD60.

Ported (Rust)
- crates/aa-runtime/src/native_camera_screen.rs: ScreenPolygon (887F40),
  update_5cca80, box_5cd850 (with the SAME shared defaults/Factory), the
  Camera+168 List (5CD7E0/5CD730, 874350). Installed properties keep their
  later stores beside the SAME allocation (the shared property owner only
  exposes reads).
- native_camera_directory.rs: Camera+168 per camera, set_byte_214_captured,
  new_screen_box_5cd850, attach/detach on the captured camera.
- crates/aa-client/src/battle_orders_hud.rs (module battle_camera::orders_hud):
  HudLeaves for the camera slot, VWorld+AD, drag threshold, box and facing
  children; HUD widget routing, minimap, sounds, markers, command definitions
  delegate to UnboundHud (HUD owner).
- battle_camera.rs Process: the box state's +4 polygon and orders_children
  (5CD850/5CCA80/5CCA30/5CCD60/5CCDA0 on *B40048, in the order reached).
- battle_presentation.rs: State.orders_window/orders_input/orders; observe
  queues battle_orders_input::observe per UI frame (world under cursor unless
  an egui area/widget has the pointer; native pixels of the presented image);
  the camera job runs, after native_camera_frames, one Orders::frame per
  queued UI frame with Live = DirectoryViews (startup.order_views), the
  activated AdminNode (order_node_mut, Admin+2C0 = node_active, Node+8 =
  local_player), the current normalized camera, B396D0/D4 = native target
  pixels, B396C0 = Notifications::application_kind_b396c0, terrain ready =
  B36518 published, terrain None, B43CF8 from the startup seeds; then its
  camera children; then (VWorld+AD) the 43EC60 view request segment once per
  job. Errors are logged once; frames with button messages log the result.
- Ordering: the camera frame (5C92F0 subset) runs before the orders frame of
  the same UI frame, so a +214 store by the box/facing state takes effect on
  the camera one UI frame later than native (5C92F0 interleaves the C0 slot,
  impulses and states). The children are applied right after their orders
  frame with the tracker 5CCA80 read (cursor after 5BF2B0, left press before
  this frame's events: both box children are reached in the tracker step).

Proof
- native_camera_screen.py (ORIGINAL code, Unicorn, PC24/53/64, fixture
  native-camera-screen.json, 1015 instruction pins checked against v14):
  360 5CCA80 rows (10 viewports incl. 1x1, 3x7 and H > 2^24, 12 tracker
  pairs each incl. negative/INT_MIN/INT_MAX), 21 5CD850+5CCA30 rows (seven
  Camera+168 layouts: empty, holes, full at capacity 5 and 7 -> 5CCFE0 grow;
  exit on the same camera and after *B40048 moved: then the old camera keeps
  the polygon, only the current camera's +214 is cleared and the polygon is
  not destroyed), 6 5CCD60/5CCDA0 rows. Seams: 6F7810 new (0xCD fill),
  6F7430 free, 5E0780, the AB990C destructor, VWorld+13C vt+44, arrow vt+2C.
- Rust replays (native_camera_screen_tests.rs): every vertex word, color,
  flag, installed-property byte (unreadable Rust bytes must be the 0xCD
  fill), the relative material id, list placement/count, detach and the
  destroy-or-retained result; directory test screen_on_directory on a real
  lab Directory camera; battle_orders_hud.rs unit tests.

Open gaps (orders integration)
- GPU draw of Camera+168 screen polygons (renderer vt+D8 NiDX9Renderer and
  KKC RScreenGeometryManager 743320) is not ported: the box polygon exists
  with its exact words and is logged, but is not rasterized. The final blend
  (vertex alpha 0.3 vs material alpha 0.5 under VertexColor source 1 /
  lighting 0) is a renderer fact that must be proven there, not guessed.
- Facing ghosts/arrow (689380, 688AE2, 688A43, 693D10, 693EB0, arrow vt+2C):
  view owner.
- Live.terrain is None until the live TerrainMap (723E00/723960/71B8F0/
  71B610) lands; with B36518 published, every orders frame stops at its
  0x43C550 pointer update (0x43C4B0 -> 0x71D840), before any event.
- UI+0xBC/+0xC4 interplay, the HUD's UI+0x60 verdict, and a single 5C92F0
  driver for camera and orders (one-frame +214 latency above).
- 43EC60 runs once per presentation job (one or more queued UI frames).

Real GUI runs, orders integration (run_orders_gui.py; summary real-run-orders.json)
- orders-run01 (Berlin 2v2, 968 s, exit 0) and orders-run02 (431 s, exit 0),
  real tree build (no mirror: frontend.rs already calls observe_input). Each
  sequence: left click (640,360), left drag (480,260)-(820,480) (one pointer
  event per UI frame), shift drag, right click.
- Every button message arrives as its own native UI frame: 8 orders frames per
  sequence (32 in run01, 16 in run02). Each one logs
  "native orders frame: buttons=1 events=0 handled=0 selected=0 modal=[]
  children=[] box=none lock214=Some(0) error=0x71D840 battle TerrainMap query
  is not bound": with B36518 published (terrain ready), the frame's first
  native step, the 0x43C550 pointer update (0x43C4B0 -> 0x71D840), reaches the
  unbound TerrainMap before 0x5BF2B0 and any event, so nothing is selected and
  no box child is reached. The refusal is logged once as an ERROR.
- run02: "native VWorld+AD = true: 43EC60 view request segment runs once per
  presentation job"; 43EC60 returned Ok on every job of both runs (no 5AA550
  release refusal appeared in these runs).
- The only other ERROR is the battle_controller view-initialization stop
  (5A94F0 BoundingSphere fallback), unrelated to this path; presentation and
  the frame loop continue.
