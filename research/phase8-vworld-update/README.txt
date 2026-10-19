Phase 8: VWorld per-frame update driver (package 1 of the 2026-10-19 gap plan)
=============================================================================

Native functions (AA.exe, image base 0x400000)
----------------------------------------------
43FF60  VWorld vtable 9FDFF8+C update (not in the Ghidra export; decoded from
        the AA.exe bytes): 5EEC70; return while +AD clear; lazy +140 =
        6F7810(0x88)+4341A0; +174 RandomLookAt (B39F70 guard, B39F68 timer
        59AA70/59AB60/96CAB0/59ABA0, two 85D840(B39714), [GWorld+30]+C/+10,
        6FF130 on *[B40048], 59AB60); 701590; 43DA00; 43C4B0 into +184 (+178
        normal) while [B36518]; 41C0E0(5); 43EC60; 41C110(5); 43A1C0([+13C]);
        71FAC0([B36518]); 4361B0([+140]).
43EC60  WorldView update: IsGWorldValid() (855D30 under the B39F64 latch);
        50C750(World+130); 43A400(+108); 4318E0(+BC); 4312F0(+94);
        725340(B36518); +208 request loop; E0/E4/E8 FX loop (count read once).
43AB30  VWorld view time from 76B120(0) and World+E0/E4 into +84/+88/+8C/+90.
76B120  KKC_Sim render time +34 (timeline gate, paused/frozen, RenderDumpAVI
        76B080, 74B180 admin clock, World+E0..E4 clamp).
50C750  World+130 broadcast walk; 50C600 dispatch (type 1 = 427950 Eval).
431E10  VWorld+BC rect append (only native caller: TerrainBlock 71CC00).
4318E0  VWorld+BC rect drain; 431390 resample (CRT floor/ceil + 96CC4C,
        4F0E50 at 0x122, +20 min / +24 max).
43DA00  VWorld+98 deferred-destroy prune (producer 43D190, removal 43CF00).
41C0E0/41C110  profiler enter/leave: argument 5 is outside 1..4, no store.

Oracle
------
native_vworld_update.py (Unicorn 2.1.4 through research/phase8-movement/
harness.py; original and accepted v14 bytes compared for every executed
in-image instruction; PC24/PC53/PC64; page-0 null guard; allocator seams).
python native_vworld_update.py --write regenerates; without --write it replays
and asserts equality with native-vworld-update.json.

Sections (rows): view_time_43ab30 432, sim_time_76b120 1842,
messages_50c750 42, rects_4318e0 270, deferred_43da00 42, update_43ff60 512,
world_view_43ec60 42. 1029 executed instructions pinned.

Seams per section are listed in the script docstring. Notable:
- 4318E0 runs both CRT paths ([B4B340] = 0: x87 977575; = 1: SSE2 96D620);
  results identical for the generated rects.
- 43FF60 runs 41C0E0/41C110 natively (argument 5): the profiler object is
  byte-identical afterwards and 866060 is never reached.
- 43EC60 runs 50C510 natively over the allocator seam.
- 50C750 rows with kinds 0/2/3/4/5 are native reference for future
  producers; Rust has only the type-1 producer 50D120.
- 43FF60 rows with a null 6F7810 result store +140 = 0 and pass null to
  4361B0 (a native fault): native reference only.

Rust
----
crates/aa-runtime/src/vworld_update.rs: drivers update_43ff60 /
  world_view_43ec60 / prune_43da00 over a Services trait, kernels
  view_time_43ab30 and sim_time_76b120, Fields (+98, +140, +174..+18C, B39F64,
  B39F70), Refusals (product: first refusal stops; look-ahead: diagnostic).
crates/aa-runtime/src/vworld_update_tests.rs: replays every section above
  except messages/rects (fx_manager.rs and view_heightfield_tests.rs).
crates/aa-runtime/src/fx_manager.rs: deliver_broadcasts_50c750.
crates/aa-runtime/src/view_heightfield.rs: append_rect_431e10,
  update_rects_4318e0 (431390).
crates/aa-runtime/src/view_init_world.rs: update fields, 43AB30 and 4318E0
  entries on the same VWorld; view_init_prefix.rs: +84..+90 word access.
crates/aa-client/src/vworld_frame.rs: the client Services/SimTime binding.
crates/aa-client/src/battle_view_startup.rs: the per-UI-frame hook runs the
  whole frame (update_view_requests_43ec60 -> update_vworld_frame).

Open (explicit refusals in the client binding, see findings.json)
-----------------------------------------------------------------
5EEC70 (aa-ui VWorld widget), 4341A0/4361B0 (minimap), 701590 (camera),
43C4B0 (camera + TerrainMap query), 43A1C0 (UIWorldState), 71FAC0/725340/
4312F0 (no TerrainMap/TerrainVisibility bound in the client), 43A400 (scene
light), 427950 (script processor), FX loop (package 4), 855D30, 43CF00,
RandomLookAt owners, 76B120's unfrozen path (74B180 and the Sim+34 store) and
its B396C4 read when the hook is not given the window reader.
