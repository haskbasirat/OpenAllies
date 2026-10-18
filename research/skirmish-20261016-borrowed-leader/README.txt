Borrowed Organization leader and slot operations (2026-10-16)

Production:
  crates/aa-movement/src/borrowed_formation.rs
  crates/aa-movement/tests/borrowed_formation_native.rs
  One module export appended to aa-movement/src/lib.rs after rereading it.
  Original shared lib.rs before that append is saved as lib.rs.before here.
  No dependencies/manifests, original assets, saves, binaries or releases changed.

Native scope:
  00579A00 Organization SetLeader, unchanged complete routine.
  005797E0 ComputeSlotPosition, unchanged complete routine.
  00578250 rotation and native random-vector/unit-draw arithmetic execute.
  Native instruction listing is native-instructions.txt.
  The old formation::set_leader remains unchanged for its existing callers.

API and reason:
  World must read/write authoritative storage rather than copying an
  Organization and Members and committing only after success. Handles capture
  native pointer identity; positions handles retain allocation identity across
  intervening services. Each axis store happens immediately. Missing dependencies
  return an error with all earlier writes and service effects retained.
  Native SetLeader stores +40 before any old Element lookup, computes old slot,
  rereads current +40, computes new slot, then reads the current owner Mind.
  There is NO same-pointer shortcut: assigning the existing leader recomputes
  twice and still notifies the Mind. No invented shortcut was added.
  Negative ComputeSlotPosition slots perform no reads. Native primary-slot read
  is skipped when the nonnull current member equals the leader. Current
  formation is checked, then reread in 578250 after the facing read. Scattering
  captures its initial output array before RNG, writes heading there, then
  recaptures +78 after the second RNG draw and scales values currently there.
  Attached member relative reads and detached diagnostic argument reads retain
  the original instruction ordering. No whole-state rollback/copyback exists.

Proof:
  native_borrowed_leader.py creates 678 rows: 113 inputs x 2 executable images x
  3 explicit x87 PCs (24/53/64). Both ORIGINAL and accepted v14 images execute
  separately; every original instruction visited must match accepted v14 bytes.
  All 339 original/v14 output pairs match exactly.
  420 rows execute SetLeader; 258 directly execute ComputeSlotPosition.
  Includes 162 mandatory-dependency stops across the scattering SetLeader path,
  30 rows with explicit boundary/service mutations, 12 actual native null faults.
  All 678 rows replay in Rust, comparing full read/store/service order and raw
  state/error prefixes, allocation selection, draw consumption and service logs.
  Twelve native null rows replace unsafe pointer dereferences with Rust errors;
  the final attempted actor_position(null) trace event is omitted in typed Rust.
  No behavior beyond those explicit inputs is certified by the case count.

Required explicit service seams:
  85D840: supplied authoritative raw RNG draws (4F02B0,444A90,45D400 execute).
  404F90: detached diagnostic callback; no logger implementation claim.
  Owner Mind virtual +114: callback event, not native Mind implementation.
  Native sync-log objects at B3655C/B36560 have byte zero (disabled).
  Direction table initialized from existing phase8 native PC53 direction-table.bin.
  Ordinary native field boundaries are instruction observers used for mutation
  and dependency-stop experiments, not claims they are actual virtual services.

Harness correction:
  tests.log records a rejected first run: throwing in Unicorn's memory-write
  observer still allowed its pending store to commit. Final oracle emits store
  boundaries from instruction hooks BEFORE each store instruction, permitting
  accurate pre-store dependency stops. No replay rows from the rejected oracle
  are retained in the final fixture. native-generation.log is historical;
  native-generation-final.log pins the accepted fixture.

Checks:
  cargo test -p aa-movement --test borrowed_formation_native --test formation_native
    --release --locked --offline -j4: 4 tests passed, including 678 new replay rows
    and existing native formation regressions/direction-table checks.
  cargo clippy -p aa-movement --lib --test borrowed_formation_native --release
    --locked --offline -j4 --no-deps -- -D warnings: passed with no allowances.
  Own build directory target/s-borrowed-leader; all child processes reaped.

Limitations:
  Finite numeric inputs within aa_numeric::Binary support only. No NaN/infinity
  arithmetic claim. No transcendental instructions used by the tested core;
  random directions use the separately established static native table.
  This module has no complete saved/live actor/world adapter or Mind+114 port.
  Layout/formation, membership, identity resolution, RNG, diagnostics and Mind
  callbacks are mandatory requirements, never permissive success defaults.
  No complete Actor Init/PostCreate, battle startup, simulation, skirmish,
  multiplayer or complete-game compatibility claim. No artificial cap is added;
  actual adapters must provide growable state and checked native identity access.
  No vendor SDK implementation was consulted or integrated.

Reproduce from rust-rebuild (PowerShell):
  python research/skirmish-20261016-borrowed-leader/native_borrowed_leader.py
  $env:CARGO_TARGET_DIR='target/s-borrowed-leader'
  cargo test -p aa-movement --test borrowed_formation_native --test formation_native --release --locked --offline -j4
