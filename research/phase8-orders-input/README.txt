In-battle selection and ordering input (phase8 orders-input)
============================================================

What the original does (AA.exe, image base 0x400000)
------------------------------------------------------
Window -> event
  WndProc 0x6FD4A0: WM_L/R/MBUTTONDOWN/UP/DBLCLK (class CS_DBLCLKS), releases
  through 0x6FD3D0 (ReleaseCapture, client clamp 0x40D280).
  KKC_Input::AddKeyboardEvent 0x59A5C0: VK_MENU/SHIFT/CONTROL -> Input+0x28
  bits 0/1/2.
  KKC_Input::AddMouseEvent 0x59A6E0(code, flags, x, y, delta): dropped while
  B396C0 == 3; flags |= 0x80 (+0x10 Alt, +0x20 Shift, +0x40 Ctrl); release of
  0x100..0x107 clears slot code+8; Input+0x44[code] = flags; 0x5C2770 builds
  the camera ray (eye camera+0x90; pixel -> binary32 -> _ftol, Y/B396D4 then
  X/B396D0 stored binary32, 0x6FE590); world = B43CF8 seed, then 0x71D840
  on B36518 (+0xBC ready, NULL secondary, margin 0); 0x5997F0 event
  {x,y +0, flags +4, code +8, delta +A, world +C, origin +18, direction +24};
  queue 0x59A500. Codes 0x100 L, 0x101 R, 0x102 M, +8 double, 0x110 wheel.

UI manager DAT_00B3653C
  per frame 0x5C92F0: tracker 0x5BF2B0 (+ states vt+0x10 0x5C1AF0 when the
  cursor moved), then per event 0x5C3BC0 (press: widget pick -> UI+0x60,
  0x436650 hover at the press pixel) -> 0x5C1B80 (tracker 0x5BF2F0, root
  widget chain 0x5EFFF0 incl. hotkeys 0x5EFC70, VWorld pointer 0x43C550,
  modal stack +0xB4, +0xBC brush, +0xC0 camera, +0xC4 default), then every
  slot's vt+8. Push 0x5C35A0 (vt+0x2C false pops all first), pop 0x5E0890,
  modal base 0x5E0910 (Esc, UI+0x60 chain, swallow mouse).

Game states (KKC_GameInterface 0x61A650 / B3974C)
  +0xD8 A57530 default: vt+0x14 0x620760 right-click orders, base 0x5CD190
       (left press on the world pushes +0xDC).
  +0xDC A57560 click: 0x5CC960 (release -> UIWorldState vt+0x3C 0x640CF0,
       Shift/Ctrl vt+0x40 0x640E10), vt+0x10 0x5CDBE0 Manhattan threshold
       *[B39990+0x60] -> 0x5CD850 box.
  +0xE0 A575A4 box: 0x5CCB70 (release -> 0x436460 replace / vt+0x48 0x4377B0
       add), vt+8 0x5CC770 -> 0x437DC0 -> 0x4371A0 candidate preview,
       vt+4 0x5CCA30 clears candidates.
  +0xE4 A5C580 facing drag: 0x621230 begin, 0x688AA0 threshold from the right
       press, 0x688990 facing = 0x44B600(pointer+0x184 - press point),
       0x6886B0 release -> 0x6210B0 (flag 8, facing or 0) -> send.

Right click 0x620760
  UI+0x60 must be VWorld (B36538) or a widget whose vt+0x90 is true (the
  minimap); right press; non-empty selection. Command 0x4D9C80 (0x40 bytes),
  +0x3C = Shift|Ctrl, 0x4D9B30(Alt) flag 0x400, flag 0x10 scalar 0.
  Target 0x4363B0 + 0x45B7A0 (vt+0xB4) on the world -> 0x4D9BF0 (flag 1,
  +0x10 = actor+0x14), 0x61E150 CanSend then 0x61EEF0 Send and 0x459600
  flash. Else location flag 4 = VWorld+0x184 (minimap: 0x5D74B0 and flag 8
  aux B279C0, send + marker). Any selected actor with vt+0x34(i) and
  def+0x40 && def+0x2C bit5 opens the facing state (GameInterface+0xE4
  vt+0x58); else send + marker 0x43DB10. Send = TellSelectedCommandOrder
  request 84; the simulation guesses the command (GuessCommandData
  0x4DA570, guess-4da570/).

Selection manager = game UIWorldState (0x640640/0x438E10, vtables A61E40 /
  9FD648): see crates/aa-runtime/src/player_order_input/ui_world.rs.
  Requests SelectionAdd 12 / Remove 13 / Clear 14 / KKC_SaveGroupOrder 15.
Picking: FindObjectAt 0x43D240 (0x42D750, VGrid+0x100 "On" list, 0x454AC0
  -> 0x715C10 / 0x875700, distance 0x436480, chooser 0x5C1970); screen pick
  0x43DBF0 / 0x43DC70.
Box: 0x436DD0 (release planes), 0x4371A0 (drag planes), 0x436310, gather
  0x4367A0, 0x454B50 -> 0x715D40 (sphere/plane, 0x86E8D0, 0x715850).

Oracles (Unicorn, research/phase8-ai/native_machine.py; original AA.exe and
the pinned v14 image; every executed instruction byte-identical in both)
--------------------------------------------------------------------------
  pick-math/native_pick_math.py      825 rows (275 x PC24/53/64), no seams:
      0x875700 0x8707E0 0x870720 0x715430 0x436480 0x42D750 0x44B600 0x640A90
  selection/native_selection.py      12 cases x 3 precisions, 702 operations
      of the ORIGINAL UIWorldState (select/add/deselect/clear/click/box/
      double click/hover/static groups/SaveGroup 0x439E90/SelectGroup
      0x4376C0) with the request stream
  box/native_box.py                  1170 rows (390 x 3): 0x436310 54/54,
      0x436DD0 271/271, 0x4371A0 208/208, 0x715850 118/118, 0x715D40 115/116
      (the one left is alignment padding)
  states/native_right_click.py       400 cases of 0x620760 (284/285, padding
      left): base / target attack (CanSend true/false, flash) / location /
      minimap / facing; command words at every order call
  event/native_event.py              60 sequences x 8 events x 3 precisions:
      0x59A6E0 126/126, 0x5C2770 41/41 (drop, log, Input+0x44 table, ray,
      terrain call and the queued 0x30-byte event)
  terrain-ray/  (0x71D840)           v3 480 + v4 3276 rows (own README)
  guess-4da570/ (0x4DA570)           757 cases (own README)

Rust replays (cargo test -p aa-runtime --lib player_order_input): 9 tests,
all rows, all precisions, no mismatch:
  pick::tests::pick_math_matches_native_rows
  pick::box_tests::box_selection_matches_native_rows
  ui_world::tests::selection_manager_matches_native_rows
  states::tests::right_click_matches_native_rows
  event::tests::add_mouse_event_matches_native_rows
  terrain_ray::tests::* (4)

Not ported (explicit errors where reached)
  +0xBC brush/targeting states (0x5CD220 family), CycleSelection
  (0x6101C0/0x60B7C0/0x635D60), SelectGroup double-tap LookAt, the
  TellActorCommandOrder facing branch (+0x3C), presentation children
  (box rectangle, facing arrow, cursor feedback 0x620640, voices, markers)
  which are HUD/audio owners' leaves; live TerrainMap construction
  (0x723E00/0x723960/0x71B8F0/0x71B610) for 0x71D840.
