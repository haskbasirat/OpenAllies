Loading -> Game application state entry and the Game interface (HUD)
=====================================================================

Native flow after the map has loaded (addresses in AA.exe, original = v14 bytes)

Main loop KKC_Application frame 410E70, every frame:
  40FF70 UpdateDevUVars; GAMESTATE lock;
  40FBE0: pop every notification (785060) and dispatch to the CURRENT
          AppState node's +1C (Loading 682FD0, Game 414620, MainMenu 414D60 =
          jmp 414620);
  current node +10 tick (Loading 682EE0; Game and MainMenu 4B2830 = RET);
  5C92D0 UI update (interfaces' +0C updates, e.g. LoadingScreen 6E8C00,
          Game 612010); 5C7AC0 render.

Loading (AppState 6, node Application+E4, ctor 682DE0, vtable A6E6A4):
  +14 "start game"   set by 682FD0 when notification kind 9 arrives
  +15 "view started" set by 682FD0 for kind 1 (after the 773CE0 status) or by
                     the menu-finished callback 682DB0(0)
  +16 "user ready"   set by 682E10 when kind==0, multiplayer, or return state
                     7/5/14; otherwise only by the UserReady command 6E80F0
  +17 "resources"    set only by the VWorld resource task callback 5B3550
  The selected-map confirmation (local_selected_map_confirm.rs) launches
  kind 6 with +C (return state) = 4 and +10 (kind) = 1, so for a skirmish
  +16 is NOT preset: the player must press the ReadyButton.

  LoadingScreen update 6E8C00: ReadyButton hidden && +17 && !+16 ->
    status "app_status_finished" (773CE0) and ReadyButton shown.
  ReadyButton (UI/Loading/loading_screen.tgi, View L=802 T=690) command
    "UserReady" -> 6E80F0: App(B39690)->40D970 = Application+E4, +16 = 1.
    No AppState test (proven here in AppStates 0/2/4/6).
  Loading tick 682EE0: if +15 && +17 && +16: (VWorld+AD && Session+F4 ->
    "app_status_waiting_on_other_players"), 76ACA0(VWorld+AD) -> 74FF50:
    Node+90 = 3 (AD) or 4, then 74E970 sends the task status request (304);
    +17 = 0. Then if +14: 40CE10(Loading+C = 4, 0, 0).
  Host side: the 304 status reaches the ordering host; when every client
    reported, 315 runs; 775BF0 (all tasks complete) pushes notification kind
    9 and unfreezes the simulation (aa-netcode local_task_protocol.rs,
    local_all_tasks_complete.rs).
  So 6 -> 4 needs, in this order: VWorld init finished with AD = 1 (5C3000 /
    43F0A0, notification kind 1), the VWorld resource task completed (+17),
    the ReadyButton press (+16), the 304/315/775BF0 task protocol (kind 9 ->
    +14), then the next Loading tick.

40CE10 SetState(4) from 6: diagnostic "changing app_state 6 -> 4"; Loading
  OnLeave 40CAA0 clears +8; +110 = 6, +10C = 4; Game OnEnter 608C80 = jmp
  40CA80: explicit argument else node+4 (= B3974C, the Game interface,
  copied by the node ctor 608C60) -> Show 5C0440(1). Game OnLeave 5E8900 =
  jmp RET; Game menu-finished +28 943800 = ret 4.

Game interface (Application+BC = B3974C, ctor 61A650, vtable A5B0B0):
  KKC_Application::Init 41AA7A calls its Init (+C8 = 6182B0) once at
  application start: the code-built HUD tree (EconomyBar with one child per
  registered resource, message log, floaters, idle buttons, ControlPanel,
  ...). Show 5C0440 connects it; OnConnected (+48 = 60BAE0) adds ControlPanel
  and ControlPanelTop and places children. Per-frame update +0C = 612010.
  +E8 (the initialized callback 40CF00 calls after 5C3000) = 4B2830 RET.

Oracle
  native_game_state.py runs the unchanged code (both images, PC24/53/64,
  1584 executions, 146 executed instruction pins): 682EE0 -> real 40CE10 ->
  real 40CAA0 -> real 608C80/40CA80 with the real Loading and Game nodes in
  the Application table, seams only at 401B00/773CE0/76ACA0/404F60/5C0440;
  and 6E80F0 in four AppStates. It also pins the node and interface vtable
  slots above and the 40CB40/41AA7A instructions. native-game-state.json.
  Rust: aa-client battle_notifications::game_state_tests replays every row
  through the production node slot table and application_state::set_state.

Rust wiring (crates/aa-client, crates/aa-ui)
  battle_notifications.rs: node slot table (+14/+18/+1C/+28) drives
    OnEnter/OnLeave/event prefix/menu-finished; UserReady has no AppState
    test; the Game interface is built from the actual native Resource
    registry; current AppState exported.
  battle_menus.rs: Game interface built with aa_ui::hud::Hud::construct
    (Init 6182B0) at menu bind; Show runs Hud::on_connected (60BAE0); Game
    +E8 is RET.
  aa-ui hud.rs: Hud::show split into construct (Init) / on_connected
    (OnConnected) / bind_resources; behaviour of Hud::show unchanged.
  battle_controller.rs: AppState exported to the shell, every ReadyButton
    press queued as its own 6E80F0 command (a second press no longer errors).
  frontend.rs: the development status box moves to the top-right corner in
    AppState 4 so it does not cover the economy bar.

Not bound (explicit)
  Game update 612010 and its children's live-World reads (local kingdom
  5C1760 -> Session 76AC70 -> player 765E00 economy +1A0/+1AC/+1B8/+1C4,
  faction experience player+230, idle buttons, selection, action buttons):
  the HUD shows its Init/OnConnected state; nothing is filled with invented
  values. The 60BAE0 world reads (5C1760+1F0 visibility) are not ported.

Also not bound, outside these files: the LoadingScreen OnConnected 6E8730 ->
refresh 6E83A0 (map name Session+68+24, description +2C, objectives, nation
art, status 6E8300), so the native-menus Loading page still shows its TGI
default texts; and the nation UI skin swap KKC_UI::Reskin 5C3DC0 (UI+13E).

Offline evidence of the Game forest
  The ignored production test battle_menus::tests::actual_four_menu_
  allocations_and_ready_widget_stay_in_same_ui binds all four menus from the
  installed data (zero load diagnostics), shows Game and, with
  AA_GAME_INTERFACE_DUMP set, writes drawlists/game_interface_1440x900.json
  (+ _widgets.json, 128 widgets, 0 diagnostics), rendered by
  research/phase8-ui/render_drawlist.py to drawlists/game_interface_1440x900.png.
  Without the unbound 612010 update some labels keep their TGI default text
  (e.g. TimeMessage), which the native update would hide or fill.

Real run hud01 (run_gui_ready.py berlin hud01, 2700 s, ReadyButton pressed
every 30 s after 600 s): qa-output/development10/game-state-entry/hud01/berlin
  04:13:40 SetState 0->2 (menus bound, Game interface Init built from the
           native Resource registry), 04:13:41 SetState 2->6, InitVWorld
  04:13:49 World view creation begin
  04:31:18 World view creation end: existing=0, attempted missing=7970
  04:31:18 Native view initialization stopped: selected Model requires exact
           source-bound property/effect field projection
           (aa-client model_graph_fields.rs; assets-cache/model-native-fields/
           index.json lists only 3 sources, the GBR corps models)
  The battle frame is drawn behind the Loading page (after-2280s.png,
  after-2640s.png). The ReadyButton never appears because 5C3000/VWorld init
  did not finish, so +17/+16/kind 9 and SetState 6->4 are not reached.

Continuation (2026-10-19): HUD update, Loading page refresh, UI frame cost
==========================================================================

Read accessors (additive)
  aa-netcode src/local_hud_reads.rs (module hud_reads in local_bootstrap.rs):
    ActualActivatedSession::local_player_kingdom_76ac70 (Session+E4 == 3 ->
    NULL; Node+8 local player; 765E00 player+28, requiring the 765E10
    BeginPlaying binding), session_state_e4, session_multiplayer_f4,
    sim_paused_28 (KKC_Sim+28), session_phase_f8, session_launch_mode_58,
    session_status_148, local_player_present_76ac60, game_settings_68_texts
    (Session+68 +24/+2C), local_player_creator_nation_50dc00 (76C700 creator
    by launch mode: 0 Random +74, 2 Load +70, 5 Metagame +7C, else Blank +6C;
    Launch ctor 76E540 order), local_player_selection_empty (Player+50).
  aa-runtime src/hud_world_reads.rs (one appended lib.rs line): Kingdom
    vectors +1A0/+1AC/+1B8/+1C4, Kingdom+1F0 shortage ratio, Kingdom+230
    Commander experience, VWorld+8C (view_init_prefix message word), current
    SVar EconomyLimitedResourceMax.

Game interface update 612010 (aa-ui src/game_interface_update.rs)
  Oracle native_game_update.py -> native-game-update.json: unchanged
  612010, 5EFB00, 5F0910/5F0940/5F01B0/5F0210, 40CEB0, 40D8E0 on a
  laboratory interface whose fields hold the Init 6182B0 widgets; 308
  inputs x 2 images x PC24/53/64 = 1848 executions, 452 pins; 26 inputs
  have precision-dependent banner alpha. aa-ui tests/native_game_update.rs
  replays all 924 original rows exactly. Field map from the Init
  disassembly: +190 TimeMessage (attached while UVar ViewShowElapsedGameTime
  +8C), +144/+148 Group/FactionAbilities floaters (attached while 5C0E30
  has a local Kingdom), +170 DeveloperModeLabel (40EAA0), +174
  CameraInformationLabel (AppState node 7 current and UVar CameraInformation
  +B0), ControlPanel+A0 ButtonSupplyBorder selected as UVar
  ViewShowZoneFriendlySupply +270 (notify 1), +178/+17C/+180 banner fade
  over AVar BannerFadeDurationMS, +154 GameMenuDialog, +184 DisplayPause
  blink every 1000 ms while KKC_Sim+28, +198 GameChatPanel / ControlPanel+A8
  ButtonChat (notify 0), +128 UpkeepShortageIcon while Kingdom+1F0 > 0.0.
  Client binding aa-client src/battle_game_hud.rs: LiveFrame read on the
  simulation thread every drain in AppState 4, Services over the installed
  Game forest on the UI thread, then Hud::update_children with a fallible
  LiveWorld (EconomyBar 602D10 from the local Kingdom vectors, faction
  experience from Kingdom+230, TimeMessage 5D27B0 from VWorld+8C, no
  selection -> ObjectDisplay None / no EconomicImpact).
  Not bound (named, logged once, widget left as is): 612010 children
  60BA50, 427E60, 60B9D0; the camera text (AppState 7 only, explicit error);
  banner colours (no banner producer); 5DC780 functor selection events;
  faction abilities, groups, idle-button scan, action buttons; the 5C1760
  UI+1E0/VWorld+13C and film 6AA370 fallbacks (explicit errors).

LoadingScreen OnConnected 6E8730 / refresh 6E83A0 / status 6E8300
  (aa-ui src/loading_refresh.rs). Oracle native_loading_refresh.py ->
  native-loading-refresh.json: 212 inputs x 2 images x 3 precisions = 1272
  executions, 314 pins, precision independent; aa-ui
  tests/native_loading_refresh.rs replays all rows. Bound in battle_menus
  show_connected from the SetState(6) transition's LoadingFrame: map name
  and description (Session+68), objectives hidden/cleared for kind != 0,
  nation art (Creator KingdomParameter nation -> Nations/<nation>.tgi), the
  status (Session+148), ReadyButton hidden, the 745580 hold count.
  Campaign kind 0 (mission art 411790, objectives 673D40) and dynamic art
  (kind 4) are explicit errors.

Not portable from proven code: KKC_UI::Reskin 5C3DC0 runs only when
  UI+13E != 0, and a scan of every byte/word/dword/SIB/setcc store found no
  writer of UI+13E other than the constructor clear (5CB0B5) and 5C9314.
  UI+13F has one setter (5C10F0, called from 40E260). The nation skin path
  was not located; aa-ui keeps the default skin.
60BAE0's world read (5C1760+1F0 -> UpkeepShortageIcon) is the same value
  612010 writes on the first Game frame; not separately bound.

UI frame cost / egui staging-buffer panic
  tgi_frontend frame_growth_tests (headless egui, real data, 1440x900):
  400 frames of the pre-bind Loading page, the native LoadingScreen and the
  Game interface: identical index/vertex/command counts every frame (1422,
  1614, 330 indices). No geometry accumulates.
  Cost: Ui::draw_list resolved every texture path through the file system
  (0x490ED0 resolution, exists/canonical) on every frame and rebuilt every
  label. Now cached per raw path (cleared with screen/skin) and per label
  input key (two-frame cache): draw_list 1.85 ms -> 0.12 ms; the cached list
  equals a fresh rebuild (asserted).
  Real run hud02 (45 min, sample_memory.py): working set 1.77 GB, private
  1.96 GB, GPU dedicated 207 MB, all flat from 146 s to 2705 s.
  The deploy534-01 panic (index count 3678, buffer 49152 bytes) followed
  "Acquiring a texture failed with a generic error": the wgpu device was
  lost; the staging-buffer failure is a symptom, not UI growth.

Continuation 2 (2026-10-19)
---------------------------
Real run hud03 (final build, 30.5 min Berlin, sample_memory.py every 30 s):
  SetState 0->2, 2->6, World view creation reached; the Loading page shows
  "Fall of Berlin" and its description (6E8730). Stopped at the model
  owner's "selected NIF actual Object03 controller update requires its real
  key/controller graph". Working set 1.76-1.79 GB, private 1.95-1.97 GB,
  GPU 213 MB, flat from 292 s to 1750 s; 0 panics. CPU time grows about
  one core-second per second while the view init runs.
  qa-output/development10/game-state-entry/hud03/berlin/ (after-0480s.png,
  memory.jsonl, result.json).

Native frame loop 410E70: GAMESTATE_LOCK ([B36524]+8 vt+4), 40FBE0
  notifications + node vt+0x10 ("NETEVENTS"), 41C0E0(4), UI update 5C92D0
  ("UIUPDATE"), unlock vt+8 ("UNLOCK"), render 5C7AC0 ("RENDER"); the
  NetNode Update 765410 -> AdminNode 75CA70 product frame runs between the
  locked sections. The notification job therefore runs drain (incl. Game
  612010 in AppState 4) and then run_frame on the SAME simulation thread
  (battle_controller start_notifications, already wired); the frame now
  uses aa-netcode run_native_frame_bound (local_live_frame.rs, additive):
  the caller's Catalog validates the Application loan and a FrameCatalog
  builder (battle_notifications::FrameCatalog: same source, Resource
  catalog, settings, load mode) builds the frame Catalog inside that loan
  with Catalog::bind_renderer_orientation(parts.orientation) (AVar25,
  552DD0). A missing view-inputs owner is an explicit error.

UVars from 4DA570: SwapRightClickWithAltRightClick (kind 1) and
  HotkeyAltRightClickCommand (kind 4) have no code registration; aa-ui
  uvars.rs creates their records only from a Data/UVars.tgi declaration
  (bool / string; values false / retreat), User overrides on top; strict
  getters error when the record is absent or of another kind. Bound in
  battle_notifications CurrentAudio (Frontend) through the UI bridge.
  Test uvars::tests::right_click_order_variables_are_source_created.

HudLeaves (HUD side): research/development10/hud-orders/README.txt.
