Phase 8 movement, pathfinding and formation port (rounds 1-3)
==============================================================

Images: original AA.exe sha256 5d2548a2...0d160d26, accepted v14 image
3bda77d7...efc5c1. Every executed in-image instruction is compared with the
v14 image and recorded in each fixture's "instructions" list. Original files,
saves and releases were not modified.

Inventory
---------
inventory.json (build_inventory.py) lists the Ghidra-recognized functions in
the movement/navigation source clusters (CAI.cpp, Mind/TAI, UAI.cpp,
FloodFill/LowPathSolver/PathSolver/ZoneSolver, Sim/Map, AirCraft.cpp, Mover,
Organization.cpp, Element tail, SAIRegion.cpp) plus helpers they call.
Status columns: "yes" = verified Rust port (manual), "cited" = an address is
cited in a crate (auto, read the citation before relying on it), "boundary"
= named as a required service, "no" = no citation. "proven_by" lists research
fixtures that executed >= 2 instructions inside the function (auto) or the
phase8 oracle files (manual). Cluster bounds are by address and approximate
(e.g. 0x53cb70/0x53cf40 fall inside the LowPathSolver range but belong to the
path solver). Ghidra's call table misses calls from undefined code regions
(0x55e62d -> 0x576b20, 0x55ed97 -> 0x5767f0, 0x55eda6 -> 0x576740,
0x4fe282 -> 0x576c70); callers are a lower bound.
Counts at the final rebuild (2026-10-15): 1233 functions; yes 229, cited 380,
boundary 12, no 612; 627 with fixture execution evidence ("cited" grows
as other agents add ports).

New native ports this round (all with original-x86 Unicorn oracles)
--------------------------------------------------------------------
Harness: harness.py (unchanged original code, explicit seams, v14 byte check,
x87 stack balance, nonvolatile register and stack checks, null-page access
raises NullDereference). PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages.

1. Mover component (crates/aa-movement/src/mover.rs, falling.rs)
   0x577e50 Mover updatable Tick dispatch          mover::tick
   0x5768a0 arrival stop (Mind +0x12C / deactivate) mover::arrival_stop
   0x5768f0 failure stop (Mind +0x128)             mover::failure_stop
   0x577330 stopped-near predicate (SVar+0x54)     mover::near_destination
   0x4d03d0 Properties+1C bit1 suspension          mover::properties_suspend_movement
   0x5767f0 stop turning                           mover::stop_turning
   0x576b20 requested speed/factor setter          mover::set_requested_speed
   0x576c70 maximum-speed refresh (+0x45d6f0/0x5014c0 modifier product)
                                                   mover::refresh_max_speed
   0x576850 turn radius / turn-rate refresh        mover::refresh_turn_rate
   0x576700 post-load falling fixup (vtable +8)    mover::restore_falling
   0x577380 ballistic launch speed helper          mover::launch_speed
   0x576740 begin unaimed fall                     falling::begin_fall
   0x5770a0 begin fall aimed at a ground point     falling::begin_aimed_fall
   0x577a70 falling step                           falling::fall_step
   0x576bb0 falling pose commit                    (inside fall_step)
   0x5766a0 map interior predicate                 falling::inside_map
   0x552640 map interior clamp                     falling::clamp_to_interior
   0x4453a0 vector3 normalize                      falling::normalize
   Oracle: native_mover.py -> native-mover.json, 1251 executions (417
   scenarios x PC24/53/64): tick 69, refresh_max_speed 882, fall 201,
   restore 36, interior 27, begin_aimed_fall 12, set_speed 12, begin_fall 9,
   stop_turning 3; plus 24 launch-speed rows compared as 80-bit results.
   Test: crates/aa-movement/tests/mover_native.rs (all Mover words/bytes,
   actor pose/flags, return AL, ordered callbacks with argument bits).
   Seams: 5774A0, 577A70 (from Tick), 4D67F0, AIRCRAFT virtual +38, Mind
   +118/+128/+12C, 4CFC10, 4DB090, 50AC00, 50B250, 4D64F0, 4D64A0, 4D04B0,
   4D7EA0, 4EE5C0, actor virtual +54, 4F0E50 (returns supplied binary64),
   57B250. The PlaneFallGravityTweak static is pre-initialized (first-time
   registry lookup not replayed); the resolved value is an explicit input
   (native registration default binary32 0x3ba3d70a at 0x508eed).

2. Organization formation slots (crates/aa-movement/src/formation.rs)
   0x5797e0 ComputeSlotPosition                    formation::compute_slot_position
   0x578250 offset rotation by owner facing        formation::rotate_offset
   0x57bef0 all slots (+ reassignment if oriented) formation::compute_all_slot_positions
   0x57ba80 slot reassignment (pairwise swaps)     formation::reassign_slots
   0x579a00 set leader                             formation::set_leader
   0x444a90 random table direction                 formation::random_direction
   0x45d400 extended unit draw (no f32 store)      formation::unit_draw
   0x4cf290 slot world position                    formation::slot_world_position
   0x408ed0/0x408d50/0x4091f0/0x409230 2048-entry direction table
                                                   formation::direction (embedded)
   Direction table: native_direction_table.py -> direction-table.bin and
   native-direction-table.json. The static initializer 0x9c8680 runs after
   _cinit (0x96ec76) calls _fpmath (0x96bdb7) -> 0x972c6b
   _controlfp(0x10000 _PC_53, 0x30000 _MCW_PC), so the PC53 output is used.
   PC53 and PC64 outputs are identical; PC24 differs in 859 words. FSIN/FCOS
   are Unicorn's x87 emulation (same acceptance basis as phase4-trig).
   Oracle: native_formation.py -> native-formation.json, 477 executions
   (159 x 3 PC). 47 of 68 PC53 reassignment cases change positions. Three
   cases (set_leader to none with an attached old leader) read the null
   leader natively; Rust returns an explicit error before any write.
   Removed cap: 0x57ba80 tracks pending slots in a 32-bit mask (bit slot&31,
   assertion "e<32" Organization.cpp:0xAEC). Rust uses an exact growable
   per-slot set: identical for slot indices < 32; native aliasing above 31 is
   not reproduced. The swap budget keeps the native wrapping signed factorial.

3. Formation following (crates/aa-navigation/src/formation_follow.rs)
   0x52ff00 UAIStateMove formation speed controller (ActionGo mode-3 gate)
                                                   formation_follow::formation_speed
   0x52e400 slot lag along the path (corner taper) formation_follow::slot_lag
   0x52e300 organization minus member path length  formation_follow::path_length_difference
   0x52ce70 path length from a point               formation_follow::path_length
   0x46b000 vector inequality                      formation_follow::vectors_differ
   0x45d880 squared distance                       formation_follow::squared_distance
   0x576d20 Mover requested speed clamp            formation_follow::set_mover_speed
   0x577090 catch-up speed                         formation_follow::catchup_speed
   Mind virtual +0x44 implementations are pure accessors (Element 0x535e00
   lea +0xD4; Regiment 0x5247a0 lea +0x9C); paths are passed as views and
   their accessor events are filtered in the test.
   Oracle: native_follow.py -> native-follow.json, 498 executions (166 x 3
   PC) covering detached/no-organization, both lag formulas, the corner term,
   equal directions, the 36.0 radius gate, catch-up clamp, the 0.2 minimum
   with leader-moving stop+timer and finish-state branches, and the kind-0
   path-length blend. 3 cases (empty path) fault natively; Rust errors.

4. Collision separation (crates/aa-spatial/src/overlaps.rs)
   0x555620 all-overlaps query                     overlaps::all_overlaps
   0x554dd0 / 0x5541f0 shape tile iterator         (inside all_overlaps)
   0x4d67f0 actor overlapping-thing list refresh   overlaps::refresh_overlaps
   0x79e3c0 ordered list removal                   (inside refresh_overlaps)
   reuses 0x4efb10/0x4efb20/0x4efb50 marks, 0x552340 exclusion, 0x46ef30
   overlap, 0x5534f0/0x552550 rectangle and 0x554350 actor query.
   Oracle: native_overlaps.py (builds the native index with
   research/phase7-spatial/original_query.Query) -> native-overlaps.json:
   192 index scenarios (64 x 3 PC), 2304 queries (flag10 filter on/off,
   positions inside/outside the map), 576 refreshes (expiry, bit29/bit10
   gates, eligibility seam 0x4D10A0). Test: aa-spatial overlaps::tests (in
   crate, to use the existing pub(crate) intrusive-link accessors).

5. SAI goal base fields for the goal-priority port (lead request)
   crates/aa-navigation/src/sai_state.rs GoalBase (constructor 0x7A66E0,
   destructor 0x7A5C90, vtable 0xA9C9B0) now exposes the native fields used by
   aa_mind::sai::goal_priority (0x7A5200) and 0x7A3860, as public fields:
     +0x08 status (u32, engine compares signed; status_signed())
     +0x0C/+0x10/+0x14 assignment_words: actor list header {head, tail,
            count} (raw; actor_count() = +0x14 signed, actor_list_unlinked()
            = head and tail null, as after 0x7A66E0)
     +0x18 fuzzy (binary32 bits, constructor -1.0)
     +0x1C/+0x20 costs: owned Vec<u32> of binary32 bits (data/count)
     +0x24 cost_capacity_word (constructor 1, destructor 0)
     +0x28 inertia (applied inertia, binary32 bits)
     +0x2C byte2c
     +0x30 travel: average actor travel time (constructor -1.0)
     +0x34 current_priority: actor/travel adjusted priority (constructor -1.0)
     +0x38 priority: base priority (constructor -FLT_MAX)
   Native writers: status by the goal engine (0x7A5E10/0x7A67B0) and vtable
   +0x80 0x7A5600 (nonzero status -> 1 after 0x7A3D30); inertia and priority by
   0x7A5200; priority by goal virtual +0x78; costs by goal subclasses. Fields
   are public so those ported routines perform the same writes; no setter
   adds behavior. Serialization keeps the phase7 fixture names (category8,
   scalar18, words1c_20, word24, word28, scalars30_38); the cost data pointer
   is written as 0 with values in "costs" (omitted when empty). Tests:
   tests/goal_base_fields.rs plus the unchanged phase7 SAI fixture tests.
   Round 2 (lead request): +0x30/+0x34 are now the named fields travel and
   current_priority (they were scalars30_34; the serialized layout is
   unchanged). These are the words of aa_mind::sai::engine::GoalState
   {status +08, costs +1C/+20, current +34, base +38, travel +30} and of
   aa_mind::sai::assign::Roster goal_actors (+0C/+10/+14 list). aa-mind
   depends on aa-navigation, so the conversion lives in aa-mind (AI agent):
   GoalState <-> GoalBase is field-for-field; a Roster list can be produced
   from a GoalBase only while actor_list_unlinked() (empty list,
   ActorList::from_actors([], actor_count())), because head/tail are native
   node pointers whose nodes GoalBase does not own.

6. FPU precision. Lead correction (research/phase8-tick/fpu-precision.md):
   the simulation runs at PC53 (TGC_NetNode::Update on its own thread with
   the Windows initial control word 0x027F), and the x87 precision-exception
   (PE) flag is set on every tick (each frame's binary32 time store at
   0x76544F sets it; AA.exe has no FNINIT and FNCLEX only at CRT start).
   Precision stays an explicit input and every oracle still covers
   PC24/PC53/PC64. aa_movement::OriginalTurning / ground_step_original take
   PE explicitly; callers pass `true` from the tick. Under PC53 the CRT asin
   skips its PE-controlled qword round trip, so OriginalTurning53 and
   OriginalTurning{true} agree. Test: native_fixtures.rs replays the 180
   PC24/PC64 phase4 ground steps (74 turning) with the PE state recorded in
   those Unicorn runs (clear). aa-runtime MotionWorld::advance (PC53) and
   advance_original(.., pending) both exist.
   Fixture caveat: Unicorn 2.1.4 evaluates FPATAN/FSIN/FCOS/FYL2X through
   binary64 and never sets PE from inexact results, so 80-bit values from
   those instructions are not ground truth; binary32 stores are. The
   direction table (FCOS/FSIN, then binary32 stores and normalization) and
   phase4 turning rows rely only on binary32 stores of those instructions.
   The direction table is built by a static initializer after the CRT sets
   PC53, consistent with the simulation precision.
   Emulator-caveat audit (docs/NATIVE-PORT-RULES.md, 2026-10-15):
   - FS base 0: harness.py hooked page 0 only from address 4, so a plain
     null read of [0..3] silently returned the SEH word. It now hooks
     0..0xFFF and allows only FS-prefixed accesses to [0..3]
     (page_zero_access). native_topology_builder.py had the same gap; the
     phase7 Query machine used by native_placement.py mapped page 0 with no
     guard at all (now a Guarded class inserted before phase7 Index in the
     MRO, so the guard exists before any native code runs).
   - Allocator seam: executing 0x6F7810/0x6F7430/0x6F7700/0x6F7650 or CRT
     0x96E19F/0x96E173/0x96E0F8/0x96D331 without a seam raises
     UnseamedAllocator (never recorded as a native fault); same trap in the
     topology and placement machines. It never fired.
   - Regeneration under the hardened machines: mover, direction table,
     formation, follow, overlaps, company, company-think and placement
     fixtures are byte-identical; topology builder fixtures differ only in
     the recorded producer hash. native-aircraft.json changed in 6 rows
     (tick_no_mind and tick_formation_null_b x 3 precisions): the first
     null dereference is now reported at 0x0 instead of 0x1B0/0x14 (the old
     run had read [0] as 0 and faulted later); events and partial state are
     unchanged and the Rust replay (which checks the error, not the address)
     still passes. native-water-areas.json gained an emulator_caveat marker
     on its 3 signalling-NaN rows; no result changed.
   - Signalling NaNs: all fixtures were scanned for binary32 SNaN patterns.
     Only the water rows above are float SNaN inputs (neighbour height
     0x7F800001; only the unordered compare is observable, identical on
     hardware; Rust quiets as hardware does). Low-path hits are being
     checked by its producer (they look like integer ids/words).
   - Stale translations: every code stub is written at a fresh address
     (bump allocator / stub_next per Unicorn instance; placement uses two
     distinct table stubs).

Round 2 and 3 ports (all with original-x86 Unicorn oracles, PC24/PC53/PC64)
====================================================================

7. Regiment movement (KKC_CAIStateMove, vtable 0xA33804, Regiment Mind +0x68)
   crates/aa-navigation/src/company_move.rs
   0x520320 / 0x520440 activate move to actor / location   activate_to_actor / activate_to_location
   0x51e9b0 destination (+0x51d840 group offset, 0x5525d0)  set_destination / clamp_to_extent
   0x523080 Enter, 0x51d7b0 Leave, 0x5205e0 speed broadcast enter / leave / set_speed
   0x526940 move tries + CAIMoveMinSecondsBeforeFail        increment_tries
   0x5276b0 Think: target pursuit re-path (squared distance ratio 0.14645929), path
            region check, arrival (0x52a1f0, 0.25), planning 0x51ea20 (Solve from
            the organization leader), leader-moving pending-member drain,
            GroupSpeedMinimum hold, move-try failure        think / plan_path / arrived
   0x525740 member dispatch by formation: 0x523130 (slot path 0x52cbd0 with
            0x445280), 0x523240 (translated path 0x52d230 / detached order),
            0x5233f0 (scatter: 3 world RNG draws, 0x409230, placement 0x554ad0,
            push-front 0x52ca90/0x52d180), 0x579a90, 0x57bf30
                                                             dispatch_member_paths and send_*
   0x52cd90 path reset, 0x4364c0 member iterator semantics.
   Element return to slot 0x52e9d0 / 0x52e900 (activate_move to center + slot,
   restart when the destination or mode differs)         return_to_slot / move_to_slot
   Oracles: native_company.py -> native-company.json (756 executions) and
   native_company_think.py -> native-company-think.json (660 Think executions,
   39 of them native null reads with no organization leader, which Rust
   rejects with an error after the same events). Tests:
   aa-navigation tests/company_move_native.rs, tests/company_think_native.rs.
   Remaining services are callbacks: the Mind state framework (0x529c00,
   0x52a810, 0x52a000), group objects (0x4ddee0/0x4de6c0), 0x51fbc0 target in
   range, 0x52cae0 region test, 0x4cede0, 0x44dea0, 0x76b9a0, Solve 0x53e1f0
   (crate::solver::solve), 0x57bef0 (aa_movement::formation), member Mind
   +0x100/+0x10c (crate::move_path_event), member orders 0x4ca760/0x4d9b60.

8. Free placement search (aa-spatial/src/placement.rs)
   0x554850 rings x 8 rotations with table sin/cos(-pi/4) (0xbf3504ef /
   0x3f3504f7; cross-checked against aa_trig in aa-movement
   tests/trig_constants.rs), 0x552f70 shape query, 0x554ad0 radius floor,
   0x554b30 distance wrapper (rings clamped 1..=20, a native search rule).
   Oracle native_placement.py -> native-placement.json (864 searches + 432
   distance searches on native-built tile indexes). Note: definition +0x3C is
   the blocking shape's own radius word (+0x38 shape +4).

9. Static SAI region-topology builder 0x7ACB00 (aa-navigation/src/topology_builder.rs)
   Ported by a helper session; see native-topology-builder-findings.json.
   Squares/rectangles/sides/regions/islands/merge/centers/distances
   (0x7A9B00, 0x7A9DF0, 0x7A9EA0, 0x7ABD40, 0x7BAC90, 0x7AACD0, 0x7B9B80,
   0x7BA4F0, 0x7B99B0, 0x7AB7A0/0x7AA8B0, 0x7AA2A0, 0x7AB320, 0x7B9900,
   0x7B8E50, 0x7BC590, 0x7BA3C0, 0x606750). Evidence: synthetic grids x 3
   precisions + a 4224-rectangle case, 2 real 96x96 maps, and the original
   builder reproducing the topology shipped in all 80 maps at PC53/PC64 (not
   at PC24), with the Rust builder reproducing the same 80 maps (ignored
   test, needs local RMP files). Not ported: 0x7AA180 max rectangle width
   (CRT pow via FYL2X/F2XM1; explicit integer input; 10 older maps need a
   historical 27/28), 0x7AC9A0 clear (precondition). Removed cap: island
   flood recursion (main-thread stack 0x100000) is an explicit heap stack;
   the 0x1000-rectangle warning stays a warning.

10. Aircraft movement (aa-movement/src/aircraft.rs)
   Ported by a helper session. Aircraft vtable 0xA3AF78 (+0x38 = MoveTick
   0x5612F0, reached from Mover Tick via TickWorld::aircraft_step). Ported:
   0x5612F0, 0x55E2F0 path advance, 0x55DCA0 arc length, 0x55DA10 cubic
   evaluation, 0x560050/0x55FE60 holding circle, 0x561080/0x5685E0/0x4CF1B0
   formation following, 0x55EBE0, 0x55E1B0, 0x5685B0/0x567FF0, 0x516B10,
   0x576D20. Oracle native_aircraft.py -> native-aircraft.json (1281 cases).
   No trig instructions are involved. Callbacks: leg builder 0x55F230, follow
   planner 0x560A70, formation teardown 0x55EDC0, combat getters, pose commit
   0x576BB0 (now public as aa_movement::falling::commit_pose), terrain height.
   Unbounded native recursion in 0x55DCA0 on degenerate curves and
   out-of-range formation slots are explicit Rust errors.

11. Water-area flood fill (aa-spatial/src/water_areas.rs)
   Ported by a helper session; see native-water-areas-findings.json.
   0x539E30 (goal 0x53C570, vtable 0xA36058) on the generic solver core
   0x4F4050 with solver 0x539D60 (vtable 0xA36048: 0x53F8D0, 0x53F890,
   0x4F0550, 0x4F0500), FIFO queue 0xA3602C (0x539AC0, 0x539DB0/0x539D20,
   0x53F5D0, 0x539DF0; link tile+0x24, so breadth-first), neighbour iterator
   0x554ED0/0x4F2000/0x4F2760/0x4F14A0 (direction table 0xB322E0, arc tables
   0xAB7804..0xAB7870, reverse 0xAB7894), reach 0x539B10/0x53C630 and
   0x53C430, base height 0x539B70/0x53C410/0x552440 (visit stamp restart at
   0x7FFFFFFF), IsIndexBlocked 0x539C40 (terrain class 2 = water; a tile of
   another nonzero area blocks only if |h - base| > 0.1f, NaN never blocks),
   paint 0x539C00 -> 0x552150/0x552110, vertex area bytes 0x550F20/0x5507C0
   (heightfield +0x44, 0xFF = shared), 0x54EDC0, 0x550190, 0x552C90 first
   free id, 0x42A530, stroke 0x5559F0 and paint_terrain 0x555C10.
   Oracle native_water_areas.py -> native-water-areas.json (387 generated
   runs + 30 floods on 6 original maps, each at PC24/53/64; 9 native null
   dereferences rejected with errors after the same partial state). Tests:
   aa-spatial water_areas::tests (8), tests/water_areas_native.rs (2; the
   original-map test reads reconstruction/assets/original/Data and is not
   ignored). Kept caps (data semantics, not engine buffers): 254 water areas
   (one byte id, 0xFF = shared, 255-entry table) and the visit-stamp restart.
   Callbacks: stroke stamp 0x5526C0 / batch commit 0x559FB0 (StrokeWorld),
   brush stages 0x5528F0 / 0x551140 / 0x551B40 (PaintTerrainStages).
   Not executed: random-map callers 0x5972F0 / 0x597530. Unknown: where the
   original restores GMap+0x50 area record counts at map load (input here).

12. Low-level pather KKC_LowPathSolver (aa-navigation/src/low_path/)
   Ported by a helper session; see native-low-path-findings.json. The
   moving-obstacle avoidance solver in a 64x64 sub-cell window around the
   actor, and the UAIStateMove continuation that drives it.
   0x53BB90 constructor (LowPathSolver::new), 0x53A490 window (Window::new),
   0x53C170 Solve (low_path::solve), 0x53BE30 core loop (template shared with
   the tile search 0x53D8D0; reuses search::NativeQueue), solver vtable
   0xA36460 (0x53B0C0 expand, 0x7C7840 examine, 0x4F0550 mark start,
   0x53A790 exhausted), neighbour iterator 0x53BC80/0x53BB40/0x53B3C0, limit
   0x53A660, window placement 0x53B1F0, sub-cell centre 0x53AAC0, world to
   sub-cell 0x53A570, bounds 0x539E90/0x53A6F0, goal clipping
   0x53A990/0x53A820/0x53A3A0, octile heuristic 0x53ADB0, cost 0x53AD00,
   cell position 0x53AF10, neighbour line test 0x53AF70 (sweep below
   AccuracyDepth, end-point query otherwise; descriptor byte +0x31 =
   ignore moving), goal vtable 0xA3634C (13 entries: 0x53A5D0, 0x53A5F0,
   OnSolveStarted 0x53B720 with goal adjust 0x53B490 and displacement
   0x53AB90/0x53A930/0x43AA00/0x4325A0, 0x539F00, OnSolved 0x53B960/0x53AE60,
   0x53A610, 0x539EE0, 0x539F10 (0x54F370), 0x4C9FA0, 0x4DB6B0, 0x53A5C0,
   0x6A71F0), 0x409230, 0x5522B0, LowPathStats 0x539F60, path list
   operations 0x52CF40/0x52D180/0x52D000/0x52CA90/0x52CAD0/0x52D1E0/0x52D080,
   and the continuation 0x531520 (low_path::continue_current; ResultPlan's
   World::continue_current now points to it).
   Oracle native_low_path.py -> native-low-path-solve.json (321),
   native-low-path-continuation.json (69), native-low-path-result-plan.json
   (21 ResultPlan runs reaching 0x531520): 411 executions at PC24/53/64,
   byte-identical under the hardened harness. Tests: aa-navigation
   tests/low_path_native.rs (3 replay tests).
   Settings are explicit inputs (no executable default exists; 0x4B2E60
   looks them up lazily and a missing Var dereferences null):
   LowPathingExaminedMax 512, LowPathingIgnoreMovingDepth 3,
   LowPathingAccuracyDepth 5 (LowPathSettings::ORIGINAL_DATA) and
   LowPathingWaitMinimum 2.0 (ORIGINAL_DATA_WAIT_MINIMUM_BITS), all from
   Data/Game/SVars.tgi lines 95-98. Caps: queue limit 0x1F9C (0x53CDF0) is
   Options::queue_limit with growable storage (None = unbounded);
   16x16-tile window with sub shift 2 (64x64 cells) stays explicit
   (WindowConfig::NATIVE); depth 32 (0x53A6AA) is a search rule
   (DEPTH_LIMIT). Wall-clock: the solver+0xC8 QueryPerformanceCounter is
   write-only and no limit depends on time. Quirks kept: both window
   extents use the width; window stamps share the world epoch with the tile
   search; a nonzero status with an empty path makes 0x531520 read a null
   tail (0x531711), which Rust reports as Error::EmptySolvedPath.
   World services (LowPathWorld / ContinuationWorld): 0x54F370,
   0x552250, 0x553BA0, 0x553CE0, 0x4F3C40, 0x552340, actor virtuals and
   0x4D67F0. UAIStateMove +0x78 (last low-path solve time) belongs to the
   AI agent's UAI host state (lead backlog).

13. Low-path world adapter over aa-spatial (aa-runtime/src/spatial_world.rs,
   module low_path_world)
   aa-navigation and aa-spatial do not depend on each other, so the
   LowPathWorld implementation lives in aa-runtime: SpatialLowPathWorld
   {index, geometry, policy, host}. Descriptor layout (from 0x552340 and the
   0x552DD0 constructor, the same offsets the phase7 context producer
   decodes): words 0..5 embedded shape, 6 +0x18 source definition, 7 +0x1C
   source actor, 8 +0x20 ignore actor, 9 +0x24 ignore definition, 10 +0x28
   collision owner, 11 +0x2C hostility context, 12 bytes +0x30 ignore-all /
   +0x31 ignore-moving, 13 +0x34 custom filter, 14 +0x38 required actor
   flags, 15 +0x3C tile mask, 16 +0x40 self pointer (no portable state),
   17/18 +0x44/+0x48 facing, 19/20 +0x4C/+0x50 position, 21 +0x54
   orientation cache (computed on use). query_from_context /
   context_from_query convert; actor words are aa-spatial ActorKeys, so
   aa-navigation Object(u32) = ActorKey here.
   Wiring: tile_blocked = CollisionView::blocked (0x54F370);
   set_context_position = PreparedQuery::set_position (0x552250, writes
   words 19/20 only); first_hit = query::first_hit (0x553BA0); sweep =
   sweep::sweep (0x553CE0); context_excludes = collision::excluded
   (0x552340); refresh_overlaps = overlaps::refresh_overlaps (0x4D67F0) on
   the host's actor +0x110 list with GWorld+0xE0 time and 0x4D10A0 from
   the host; resolve_actor = index generation-checked GTID lookup; virtual
   +0xB4 / word +0x90 / pose / blocking shape (+0xC) from the index member.
   Host trait LowPathHost (no defaults): actor virtual +0x100, source
   virtual +0, the actor overlap list, world time, 0x4D10A0 and the
   0x4F3C40 placement result.
   Tests: spatial_world::low_path_world::tests (descriptor round trip over
   every 0x554350 / 0x5522B0 descriptor in phase7-spatial-context, byte
   offsets). The adapter itself is compiled and delegation-only; no
   end-to-end oracle runs 0x53C170 with the real spatial services yet.

14. SAI route cache and path cost (aa-navigation/src/route_cache.rs)
   Ported by a helper session; see native-route-cache-findings.json.
   0x53F320 flag 1 ("cached route") runs the shared 0x53EDE0 engine
   (region_search::run) with the cached early test 0x53E610, the
   cache-writing mark 0x53F230 -> record 0x53F120 (hop 0x53EAC0, adjacency
   slot 0x7B90C0) and the finaliser 0x53EBD0 -> rebuild 0x53E920. Cache
   object [[0xB3652C]+0x40]: count-1 rows of next-hop bytes for pairs
   i<j (0xFF unknown, 0xFE no route); created by 0x7ABC30 (via
   0x53F0A0/0x53ED50; only caller 0x7A0D40, the SAI ready object), reset by
   the ResetPathPlanner command 0x5D8560, destroyed in 0x7AC9A0 (from the
   destructor 0x7ACE50 and the topology build/load 0x7ACB00). No other
   invalidation exists; only 0x53F120 writes it. 0x53E5B0 sums 0x7BB420
   distances along the region list at the explicit precision with a
   binary32 store per step; an empty list returns -1.0 (0x9FDC48).
   API (for the AI agent's 0x7A1B30): route_cache::route_cost(graph,
   request, state, cache) -> RouteCost {outcome, path, cost_bits}, built
   from search_cached + path_cost; RouteCache::build / reset /
   reset_path_planner for the lifecycle. Flag-1 callers: 0x5728D4,
   0x5D88BF, 0x7A1BD1, 0x7A274C, 0x7AA872, 0x7B8045, 0x7C9F8F; flag 0 only
   the region prepass 0x53DC96. The busy byte 0xB3D43D is shared with the
   water flood fill, the tile search and the low-path core (one flag).
   Oracle native_route_cache.py -> native-route-cache.json: 84 cases,
   1092 searches (843 flag 1, 249 flag 0), 1080 path costs, 12 native
   faults mapped to CacheError variants; PC24/53/64. Tests:
   tests/route_cache_native.rs (3) plus the unchanged region_search tests.
   Not proven: the per-region finalisers inside 0x7ABC30 (seamed), the rest
   of 0x7AC9A0 and 0x79FF10 (static evidence), the runtime event order
   that calls 0x7A0D40/0x7ACB00, and the 0x7A1B30/0x7A1C10 actor parts
   (AI owner). Flag-1 searches mutate the cache, epoch and busy byte, so
   they run in native order (no parallel path queries).

15. GWorld placement tests (aa-spatial/src/gworld_placement.rs) and their
   runtime wiring
   Ported by a helper session; see native-gworld-placement.json and
   native-gworld-placement-callers.json ("findings").
   0x4F3C40 placement_result (0 clear, 1 region change, 2 structure
   crossing, 3 outside, 4 first-hit blocked; NaN bounds are outside; the
   first argument of 0x553BA0 is the 16-byte result buffer), 0x4F22C0
   structure_crossing (thiscall GWorld, ret 0x18), 0x54ECF0 tile_region,
   0x42A530 via GMapView::tile_at_point, 0x53CE40 endpoint_acceptable
   (first argument never read; the tile comes from the caller's point
   re-read after 0x552250) and 0x4F46E0 nearest_placement (8 table
   rotations per ring; the step compare uses the unstored sum, so PC24 can
   try an extra ring). Evidence: 1170 seamed 0x4F3C40/0x4F22C0 templates x
   3 precisions + 48 composed cases with the real 0x553BA0; 1404 0x53CE40
   rows and 1017 0x4F46E0 rows (214 precision-dependent templates).
   Tests: aa-spatial tests/gworld_placement_native.rs (5).
   Runtime wiring (aa-runtime spatial_world.rs, low_path_world):
   gmap_view(index, geometry) builds the GMap view from the spatial grid
   and QueryGeometry.extent_bits. SpatialPlacement {index, geometry,
   policy, host} implements CrossingWorld / PlacementWorld /
   DescriptorWorld / EndpointWorld / NearestPlacementWorld: actor position,
   +0xB4, +0x90 and facing from the index member, 0x553BA0 query::first_hit,
   0x552250 PreparedQuery::set_position, 0x554350 actor_context::prepare,
   tile byte +0x20 from the tile flags, 0x54F370 CollisionView::blocked,
   and PlacementHost for 0x4F2220, 0x4D36F0, +0x168, 0x585280, +0x100,
   0x585160, +0x164, tile +0x10 / node +8 and actor +0x7C / object +0x40.
   SpatialLowPathWorld::placement_result (0x53A6F0) now calls
   SpatialPlacement::verdict(...).code(); LowPathHost extends
   PlacementHost (its actor_virtual_100 is the same virtual).
   SpatialPlacement::endpoint is the body for endpoint::World::acceptable
   (0x53D340) and SpatialPlacement::nearest for aa-mind's world_4f46e0;
   region::World::blocked_here (0x53DC00) hosts call verdict with their
   probe descriptor. No runtime region::World / endpoint::World host exists
   yet, so those three are available, not called. Tests: gmap_view and the
   descriptor tests in spatial_world::low_path_world::tests; the bridge
   itself is compiled and delegation-only (no end-to-end oracle).

16. UAI collision results (aa-navigation/src/uai_collision.rs) and host
   helpers
   Ported by a helper session; see native-uai-collision.json "findings".
   0x534A40 ResultAvoidStatic (avoid_static; aa-mind collision_plan),
   option attempt 0x533140, option distance 0x52E210, descriptor alignment
   0x46DAB0/0x46DC50/0x46DCB0, closest point 0x46E4F0/0x46E150, Manhattan
   test 0x51D510, leader test 0x4CEFC0, 0x536AD0 (avoid_thing; aa-mind
   stop_distance), swept shape 0x46EB30/0x46E0E0, square 0x46EB60,
   descriptors 0x4CF210 / 0x4EA0F0 / 0x52F3E0. 0x536840 (avoid_actor) does
   not reach 0x536AD0. Evidence: 441 static + 126 thing + 69 tile cases x
   PC24/53/64, with 0x531520 executing natively inside. Tests:
   tests/uai_collision_native.rs (3).
   Host helpers (aa-runtime spatial_world.rs, uai_collision_world):
   shapes_overlap (0x46EF30 -> shape_overlap::overlaps), shapes_in_range
   (0x46F100 -> range::shapes_in_range), tile_shape (GMap+0x30 =
   QueryGeometry::cell_shape; tested equal to uai_collision::square_shape
   at all precisions), probe (0x531170 / 0x5312C0: 0x554350 context,
   0x552250, 0x5522B0, ignore actor, 0x553CE0 to current_actor_goal /
   explicit_origin_goal, composed from the disassembly), ignore_thing
   (0x4D6940 tail append to the +0x110 list, from the disassembly) and
   actor_moving (0x4CEDE0 -> moving_state::excluded). The probe and
   ignore-list compositions are compiled, not oracle-run end to end.

Tests (2026-10-15, CARGO_TARGET_DIR=target/s-movement)
------------------------------------------------------
cargo test -p aa-movement -p aa-navigation -p aa-spatial -p aa-heightfield --release --locked --offline -j 4
cargo clippy (same crates and flags) --all-targets --no-deps -- -D warnings
  -> rust-tests.log / clippy.log; counts and hashes in findings.json
     "validation" (record_findings.py).
Asset tests (#[ignore], read-only original files): run with
AA_GAME_ROOT=<game folder> and -- --include-ignored; all 7 passed:
aa-spatial water_areas_original_map_floods, live_maps; aa-navigation
topology_builder (80 shipped maps), topology_native (2),
preparation_native; aa-heightfield original_maps.
aa-runtime adapter tests: cargo test -p aa-runtime --lib spatial_world (5).
Replay tests by area: aa-movement mover_native, formation_native,
native_fixtures, aircraft_native, trig_constants; aa-navigation
formation_follow_native, goal_base_fields, company_move_native,
company_think_native, topology_builder_native, low_path_native,
route_cache_native, uai_collision_native; aa-spatial overlaps,
placement, water_areas(_native), gworld_placement_native.
Regenerate fixtures: python research/phase8-movement/native_*.py (every
producer reruns byte-identically under the hardened harness; see section
6); inventory: build_inventory.py; findings: record_findings.py.

Proven vs compiled vs untested
------------------------------
Proven by oracle replay (within the fixture domains, explicit
PC24/PC53/PC64): sections 1-5 and 7-12, 14, the gworld_placement and
uai_collision ports of sections 15-16. Compiled and unit-tested only (no
end-to-end oracle): the aa-runtime adapters (low_path_world
SpatialLowPathWorld / SpatialPlacement, uai_collision_world helpers;
descriptor mapping is checked against every phase7 0x554350 descriptor).
Not integrated: no live caller yet for company_move, aircraft,
topology_builder, water_areas, low_path, route_cache, uai_collision or
the adapters; aa-runtime still calls the ground step through
MotionWorld::advance (requests-to-lead.md). Untested: live game sessions,
80-bit FPATAN/FSIN/FCOS/FYL2X results (Unicorn binary64), x87 exception
domains beyond the recorded NaN/infinity rows.

Threading rule (path queries)
-----------------------------
The original simulation is single-threaded and path queries mutate shared
state: the region-search traversal epoch byte (wraps with a predecessor
reset), the tile-search epoch, intrusive spatial query links, strategy and
collision-word references and the Element path. A path query may run on a
worker thread only if (1) it reads an immutable snapshot of the tile grid,
region topology and spatial index taken at the native call point; (2) its
scratch state (epochs, predecessors, queue, marks) is private and its
starting epoch is assigned in native call order (base + call index, with the
native wrap/reset applied); (3) results and epoch/link effects are committed
strictly in native call order; and (4) any query whose inputs are changed by
an earlier commit or by intervening movement in the same tick is discarded
and recomputed sequentially. No parallel path code exists yet.

Gaps (largest first)
--------------------
- Runtime integration: hosts for LowPathHost / PlacementHost /
  ContinuationWorld / UaiCollisionWorld / CompanyThinkWorld / region::World
  / endpoint::World, and the Mover tick (requests-to-lead.md items 3, 9-14).
- Aircraft callbacks 0x55F230 / 0x560A70 / 0x55EDC0 (leg builder, follow
  planner, formation teardown), Aircraft constructor/destructor.
- Topology builder inputs/teardown: 0x7AA180 max rectangle width (CRT pow),
  0x7AC9A0 clear, 0x79FF10 / 0x5D85C0; route-cache per-region finalisers
  inside 0x7ABC30 (seamed) and the runtime event order of 0x7A0D40/0x7ACB00.
- Water brush stages 0x5526C0 / 0x559FB0 / 0x5528F0 / 0x551140 /
  0x551B40; random-map callers 0x5972F0 / 0x597530; where map load restores
  GMap+0x50 area counts.
- UAIStateMove 0x536840 (avoid_actor, engage/kill decision) and the rest of
  0x536F40's leaves owned by aa-mind; GetBackToPath / CheckWaypoint review.
- Dynamic region topology edits; Mover save/load 0x576D50 / 0x576E90
  (restoration owner); terrain speed producer (aa-rules owner).
