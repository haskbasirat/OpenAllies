HUD side of the orders input (battle_orders_input::HudLeaves)
==============================================================

What the original does (AA.exe)
--------------------------------
Press pick, KKC_UI 0x5C3BC0 (left press 0x100+bit0, left double 0x108,
right press 0x101+bit0): pixel (int16) -> binary32 -> UI+0x74 root widget
+0x10 root group vt+0x38(x, y, 1).
  VE vt+0x38: GROUP (A50CB8) 0x5E2B50, every other class 0x5E1680 (A50BD8,
  A50C48, A50D28, A50E30, A515B8, A51628, A516A0, A51718, A52098).
  0x5E1680: rect = client ? VE+54 : VE+20; VE+88 bit2; L<=x<=R, T<=y<=B
  (fcomp, inclusive, unordered fails); owner VE+0C unless widget vt+7C
  (0x5EDD30 parent walk; tooltip class A508B0 0x6A71D0 true).
  0x5E2B50: own 0x5E1680 with client = flag && !+D0 (transparent; 1 in the
  widget root group ctor 0x5E3D80); miss NULL; owner intercept_pick
  (+24 bit21) is the result; else children tail->head (+CC, node +4).
  List order (0x5E37A0 -> 0x5EF900): own VEs key -FLT_MAX first, child
  widget groups by vt+A8 key.
  VE+88 bit2 is rewritten by vt+30 0x5E25C0 from the draw traversal
  (0x5E1700 / 0x5E1B70): 0x5EE2C0(owner) && (when & state) == when, the
  predicate aa-ui's draw list applies -> pickable == drawn.
  UI+0x60 = hit when 0x5EFA80 admits it: widget+14 (nearest TopLevel or
  Interface, 0x5F32F0 stores self, others copy the parent in 0x5EEE20) ==
  [UI+8C] focus head or UI+74. The focus list: TopLevel/Interface
  OnActivated (0x5EF290, Interface 0x5C08E0 -> 0x5EF290), removed by
  0x5EF2D0, sorted by -key, latest first (0x5C43C0).
VWorld: KKC_UI 0x5CAE30 -> 0x442C00("UI/Shared/VWorld.tgi:VWorld", 1)
  (UI+0xD0, DAT_00B36538); vworld.tgi: render_threshold 15 and a DUMMY
  [View] "so we can pick the world"; Game OnConnected 0x60BAE0 ->
  0x43B490: Game vt+0xD4 AddChild front (head of key group 0) -> picked
  after every other Game child.
Routing 0x5E0690 (default state +4): a left release with +4 NULL or not
  UI+0x60 is swallowed (+4 = 0, true); left press / 0x108 store +4 =
  UI+0x60; then UI+0x60 chain 0x5EFFF0 and Root's, or (UI+0x60 NULL) the
  focus head's and Root's. vt+0x34: VWorld 0x43B400 -> 0x969B10 false;
  Game A5B0B0 / Root A52C60 0x5EFC70 (widget hotkey map lookup); minimap
  0x5D77A0. Mouse key names are ButtonN / MouseWheel / Wheel / MouseZ
  (localization key_Button1..); the shipped hotkey files bind only
  +button3/-button3 and mousewheel.
vt+0x64: 0x5EDE60 (NULL) for every widget class but VWorld (0x43DC70) and
  the decal widgets (A52770, A52910, A7B858: 0x5EF4F0, the +0x70 handle
  through B364C0).
UI+0xBC: written non-NULL only by 0x5C1F90 <- 0x5E0510 (brush install)
  from 0x6200A0/0x6200C0/0x6201E0/0x620B20/0x621C23/0x689150 and the
  editor; constructor 0x5CAE30 NULL.
0x436850: Session+E4 == 3 or [B39940]+0x250 (UVar 0x94 UnitAcknowledgement)
  == 0 -> nothing; else the voice owner (actor +7C/+40/+A4 -> +0C) and
  0x42A590.
0x437EB0: clears UIWorldState+20..+2C; empty selection stops; else per
  actor vt+0x154 / +7C / +100.
0x436950: +40 static group (0x25/0x26 explicit commands) else each selected
  actor's vt+0x150(0x5C0E30 kingdom, &cmd, 1).
0x620640 tail: GameInterface+0x12C EconomicImpact (only when AVar
  UIUseEconomicImpact, true in AVars.tgi); show 0x603B50 -> 0x603A00 ->
  0x602640 stores the owner at +0xD4; hide 0x602060 only when +0xD4 is
  this state.
[[B36530]+0x298][i]: the 39 predefined ActorCommands; +0x2C behavior
  flags, +0x40 can_hot_command_location (48CF0A..48CF97).
VWorld+0x140: zero in 0x442C00 and 0x43E400; no other writer found.

Oracle
------
native_press_pick.py -> native-press-pick.json (sha256 389921b4...):
  260 laboratory trees x original/v14 x PC24/53/64 = 1560 runs, 3120 vt+0x38
  picks (523 hits; nested rects, equal edges, -0.0, NaN, invisible
  elements, transparent groups, intercept_pick, tooltip vt+7C) and 1560
  0x5EFA80 admissions; precision independent; 113 executed instructions
  pinned byte-identical in both images. No seams.
Rust: crates/aa-ui/tests/native_press_pick.rs replays every row
  (aa_ui::press_pick::Arena); crates/aa-ui/tests/press_pick.rs checks the
  real in-game HUD tree (VWorld the head Game child, world in the open
  screen, HUD on the control panel / economy bar, minimap MiniMap,
  hidden widgets transparent).

Ported / bound
--------------
aa-ui press_pick.rs: Arena (0x5E1680, 0x5E2B50, vt+0x38, 0x5EDD30,
  0x5EFA80, 0x5C3BC0 UI+0x60 update), Ui::press_pick / press_target_5c3bc0
  over the aa-ui tree; Hud builds VWorld at construct and connects it
  front in on_connected; Ui::attach_front (vt+0xD4); UVars
  unit_acknowledgement_current.
aa-client battle_hud_orders.rs (battle_menus::hud_orders):
  world_under_cursor (observe's verdict), PressTracker + HudFrame::observe
  (UI thread, per Observed), HudOrderState + HudOrderLeaves (HudLeaves):
  widget_route_5e0690 / pressed_widget_chain_5efff0 with the replayed
  UI+0x60 and +4 (VWorld/focus Game verdict false with the hotkey proof;
  HUD/minimap/dialog chains explicit errors), widget_pick_64,
  brush_installed_bc false, command_definition_298 from
  actor_command_ui::Source, brush_changed_42d4d0 / vworld_list_changed
  recorded, command_masks_437eb0 for an empty selection,
  cursor_feedback_620640 no-op cases, selection_sound_436850 /
  acknowledge_61eef0 early-outs then queued voices, marker_43db10 queued.
  Camera leaves are the camera owner's (explicit errors here).
Explicit errors (what is missing)
  0x436950 CanSend and 0x620640's show branch need the machine's selection
  (+8 list, +40 static group, 0x436510, 0x4363B0 hovered target): the
  HudLeaves signatures do not pass them. 0x437EB0 with a selection needs
  the actors' vt+0x154. 0x5D74B0 / B279C0: observe never marks minimap
  presses (the UI thread routes them). Decal widget vt+0x64 needs the
  +0x70 handle. Voices/marker need audio/VFX owners (queued, unplayed).
