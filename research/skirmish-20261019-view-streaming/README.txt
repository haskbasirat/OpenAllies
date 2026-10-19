VGrid view streaming: On/Near lists and VThing proximity, 2026-10-19

Native (AA.exe, image base 0x400000)
  43EC60 (VWorld update) calls 4312F0 at 43ED0A: 42EED0 copies the camera
  frustum planes to VWorld+94 +A0..+FC, 430E50 rasterizes the terrain blocks,
  42FF50 (TerrainMap+BC set) walks the active blocks and calls 42FAD0(block)
  at 42FFA8 BEFORE 720080 updates the block's +B8, and 4312F0 tail-jumps to
  42F670 at 431323 on every update.

  42FAD0(block)   cells x in [col*A0 >> s, (col*A0 + A0) >> s), y likewise
                  (s = VGrid+24, A0 = [B36518]+A0, index VGrid+1C * y + x);
                  each node's view: radius = vt3C - vt40 (vt+FC 5A4ED0 ->
                  view+7C Model +74/+78) when greater than view+74; h =
                  43D050(view+38/+3C, 0x222) -> VWorld+BC 431550; d = view+40
                  - h; d > radius -> radius = d/2, z = view+40 - d/2, else
                  z = view+40. 42F4F0: outside when some plane gives
                  n.p - c <= -radius. Inside -> 45B2B0(2), 45B930 if on Near,
                  45B8F0 (On, +54 bit18) unless on a list; outside with block
                  +B8 != 0 -> 45B2B0(1), 45B910 (Near, bit19) unless listed.
  42F670          B39BB8 +24/+2C/+34/+3C rotate into +28/+30/+38/+40; Near
                  walk then On walk (head +4 first, next VThing+4): 45B2E0 ->
                  vt+14 455290 (451800 for VActor/VOrg) -> 45B330(+1C), +1C
                  = 0. Near: +18 == 1 counts +2C (and +3C when vt+A8, VOrg
                  only), else 45B930. On: 2 -> vt+0; 1 -> 45B930, 45B910,
                  the counters; else 45B930.
  45B330(p)       2: from 0 vt+D8(1); vt+E0. 1: from 0 vt+D8(0), else vt+E4.
                  0: from 2 vt+E4; vt+DC. Other p: no call. +18 = p.
                  vt+D8 45B3D0 -> vt+24 45B430(1, near): +58 += 1; at 1:
                  vt+D0(near), Data vt+14(1, near). vt+DC 45B3E0 -> vt+28
                  45BB50(1): +58 -= 1; at 0: +54 &= ~2, vt+D4, Data vt+18(1).
  455290          a non-NULL view+9C (attached VFX) runs 42E8A0 (not ported:
                  error); an unchanged proximity keeps +54 bit3 only when it
                  was clear, otherwise bit3 is cleared.
  Lists           {+0 tail, +4 head}; 45B4C0 appends after the tail; 45B0F0
                  unlinks; VThing +4 next, +8 prev, +C list. 43D240
                  (FindObjectAt) walks from the tail through +8.

Oracle: native_view_streaming.py (default compares the fixture; --write
regenerates). 252 rows (the plane-boundary arithmetic at PC24/53/64, the
rest PC53), 584 pinned instructions, byte-identical in original and v14.
Views carry copies of the real VObject/VActor/VOrg vtables; seams are vt+0,
vt+D0, vt+D4, vt+E0, vt+E4, Data vt+14/vt+18 and 431550, recorded in call
order. Replay: aa-runtime view_streaming::tests::native_view_streaming_replay
(fields, both lists, counters and the virtual call order of every row).

Rust
  crates/aa-runtime/src/view_streaming.rs     kernels over `Views`, the
      `Lists`/`Stats` owners and the `BlockObjects` hook for the 4312F0
      dispatcher (the terrain visibility owner's pass)
  crates/aa-runtime/src/view_grid_cell_views.rs   read-only cell walk
  crates/aa-client/src/view_streaming_objects.rs  `StreamingObjects` over
      the SAME directory (Fields +18/+1C/+54/+58, lists, counters), VGrid
      cells, VWorld+BC heightfield; `ModelLifecycle` (vt+0/+D0/+D4/+E0/+E4)
      and `DataLifecycle` (Data vt+14/vt+18 = pre_request_object /
      release_object, Model +74/+78) owners
  battle_view_startup::State::stream_views_4312f0 builds them for one update
  view_order_leaves.rs  on_list_100 (tail first) and proximity_18 read these
      owners once a 42F670 pass has run

Not yet bound (explicit errors where reached)
  Model+74/+78: Resources has no public read of those Model words.
  vt+0 (459E10/454780/44FD70), vt+D0 (4553D0: view+78 718AD0 model
  instance), vt+D4 (459690/44F660), vt+E0 (457D10/451A50), vt+E4
  (455470/451B30): the model/presentation owners' (UnportedModelLifecycle).
  42E8A0 for a non-NULL view+9C.
