Shared held Economy570020/5704C0 adapter
======================================
This slice binds the current held Actor and stored Kingdom arrays using the
existing economy_refresh, aa_actors::economy_values and borrowed_flows kernels.
It does not synthesize Organization context, initialize missing flow arrays,
or clone ActorStore/Kingdom ownership.

Production API: load_economy_refresh_world::refresh(Catalog, actual Inventory,
mutable live_object_init::Record, arithmetic Precision). The current registry
slot stays Loading. The shared SavedActions context can borrow the held Actor
with no temporary Organization; a real Organization remains in its actual slot.
Source layout/resource/recruit/definition reads use actions.source_precision(),
which authenticates the installed Bank identities and its source precision.
The caller's arithmetic precision still controls runtime arithmetic.

State reentry Context::new(actual Inventory) and the real OwnerAdd adapter both
call this same function. Actual Economy570020 allocation identity and owner
Kingdom storage are retained; missing/invalid source/storage and unsupported
held-Kingdom derived ownership fail before a replacement can be manufactured.

The external integration test uses the real aa-netcode Session implementation.
It performs an actual immediate load, chooses an actual reached Economy Actor
whose stored owner has actual initialized resource arrays, then refreshes that
same held allocation. It never resets flow arrays to create a passing case.
rust-external-initial.log records a successful compile followed by the correct
Area28 visibility refusal before any Economy Actor was reached. That attempt
is not a passing shared Economy test. The old unit-capture draft and duplicate
adapter draft are retained as abandoned attempts, not active implementation.

Native producer native_held_refresh.py independently runs both570020 and5704C0
in original/v14 with real vector and Kingdom arithmetic. Its pure getter and
predicate inputs must separately match original/v14 source-getter executions
over the captured actual source/Actor graph. Allocation/free and Properties
context lookup remain named seams; intercepted entry bytes are separately
reported from instructions that actually execute. Default mode verifies an
existing immutable fixture; --write is only initial author generation.

Final actual proof: Actor784/start_loc and stored Kingdom782, including their
genuine pre-existing flow arrays, at PC24/53/64. Both570020 and5704C0 match:
6 refresh rows/12 independent original-v14 refresh machines plus30 independent
source-getter machines;558 executed address pins and7 intercepted entry pins.
The fixture retains the exact hashes of the actual source graph and producer
dependencies. Fresh immutable native replay passes. The final external Rust
test also compares its new immediate-load input/output to both native rows.
Earlier failed attempts are retained and are not counted as passing tests.

Commands (PowerShell, from rust-rebuild):
  $env:CARGO_TARGET_DIR='target/s-graphics-reference'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-runtime --test load_economy_refresh_world --release --locked --offline -j4 -- --include-ignored --nocapture
  python research/skirmish-20261016-load-economy-refresh/native_held_refresh.py

This evidence does not establish complete object initialization, world loading,
playability, startup parity, or a visual improvement.
