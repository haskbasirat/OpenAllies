Retained Sim / shared Driver kernel / constructor publication

Production scope
- live_sim::Sim owns native76F240 scalar fields once, without Clone/Default. +10/+14/+18/+1C are explicitly absent until real76DCE0 writes, because the allocator is not zero-filling them. Fresh LocalBootstrap now uses this same owner.
- Constructor rate binds real registered AdminTicksPerSecond via the existing source receipt and an explicit constructor PC. Source/restore PC is not substituted. Finite arithmetic/stores are required; native nonfinite/trap semantics are outside this slice.
- simulation::borrowed::Kernel is the one frame algorithm. Existing Driver is a laboratory adapter over it; live Host implements direct getters/stores on actual Node, OrderAdmin and current Session/UVar fields. No Node snapshot/copyback is used.
- Whole74FD10 client replay includes its own+20C increment. The old laboratory NetworkSeams prefix adapter appends the increment itself; do not pass an already-complete actual client callback through that adapter. New live Host uses replay_client_tick.
- Native76B780 logs IsFrozen BEFORE timeline writes. A log failure retains the already incremented admin time and the old timeline; the older pre-extraction Driver incorrectly committed all timeline writes before logging. The extracted kernel fixes this observable failure prefix while preserving every existing success fixture.
- SharedSim lends the same Sim to driver fields and synchronous order callbacks. Each RefCell loan ends before the next callback; no whole Host or cloned Timeline is wrapped. Invalid reentrancy retaining a loan is outside this sequential native domain, never disguised as false/null/default.
- FrameControl is constructed once in the actual fresh restore_lifecycle application owner and moves unchanged into StartupOwner. B40904/B40908 are zero in the PE loader virtual zero-fill tail (not raw initialized bytes); the frame error latch is Rust's explicit stop policy. Never recreate counters per mission/frame.
- load_sim_world::WorldConstructorSink stores the actual typed World revision at76AB1C, after constructor return and before RNG/Init. Fresh-null only. A missing/late sink does not establish ownership. Root must retain partial constructor qualification and call it at actual timing, not infer a World from actor counts at activation.

Native fixtures
native_live_sim.py:72 constructor +30 ResetAdminTime rows,116 executed nonseam instruction pins, original/v14 and PC24/53/64. Constructor allocation fills0/A5/FF; four time words stay unchanged. CString/callback registration/Sync constructors/allocator are explicit seams, so this does not certify full constructor/lifecycle.
native_borrowed_driver.py:90 frame/error rows,331 pins, both images/all PCs. Eleven host failure positions, client replay failure, host/client success and an explicit host-record callback mutation of SAME admin+28C. Existing tick machine's network/presentation/clock leaves are named seams. Stops record fields at real instruction prefixes, not full C++ exception unwinding.
native_created_world.py:30 rows,39 pins, both images/all PCs. Actual76AA90 entry through76AB1F, before second diagnostic/seed/Init. Diagnostic/allocation/World ctor seams, null and error cutoffs. No claim of complete World Init, map load or active battle.

Validation
Native scripts replay readonly by default; --write regenerates only their own evidence JSON. PYTHONPATH points to reconstruction/tools/python-packages. Run cargo test -p aa-runtime --release --locked --offline -j4 --test load_sim_world --test shared_sim --test live_sim --test borrowed_driver --test simulation_native --test simulation_round2 with CARGO_TARGET_DIR=target/s-settlement-completion.
The shared boundary test uses one actual aa_orders::ClientState/History and consume_next: no second processed index exists and no duplicate increment occurs. It also checks callback failure latches and native prefix state.

Open integration
No actual ActivatedMission has been manufactured. live_battle.rs remains an unexported draft pending the real activation guard. Native constructor PC/rate, actual focus B396C4 flags and current UVars, remaining Node fields/Host services, Sim pause/speed producers, SAI/BeginPlaying activation and actual audio playback must be bound. Same-Sim world publication must happen before Init through the real worker/LocalBootstrap loan. This report does not claim a playable battle or live transport test.

Independent correction and final freeze
Native76B7A3/76B7A6 capture both End words before either Begin store. The independent review found and corrected that required-error prefix; a new6-row/15-pin read cutoff fixture and missing-EndAdmin regression preserve both Begin words. review-freeze-v2.json retains corrected source pins and all previous evidence. Final tests-final-v12.log passes23 invocations across6targets (17 distinct test bodies; two private-source harnesses intentionally repeat6). Independent-leader-review.json accepts228 fresh native rows and the corrected source; it does not claim an independent Rust rerun. Findings-final-v2 records all limits.
