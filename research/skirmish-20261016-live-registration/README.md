# Saved object registration and publication

This ports the original AA `4F9510` registration coordinator and the insert
branch of `4F4C40` into one owned runtime state. It is a concrete bridge from the
held, in-progress `RestoredRecord` allocation to persistent world registration
storage. It does **not** make a partially restored actor ready for play.

The original executable is authoritative. No Gamebryo or Kohan implementation
is incorporated. The accepted v14 executable is independently executed too.

## Owned storage and API

`crates/aa-runtime/src/live_registration.rs` owns, without `Clone`:

- the existing proved `names::Table` for World+F0;
- head-first Area6 pointers at +188/+18C and Camera7 pointers at +178/+17C;
- the actual initialized Team5 slots at +16C;
- head-first, duplicate-preserving available-city GTIDs at +260/+264;
- the explicit current B3CC40 assertion latch.

`State::from_saved_team_count(count, latch)` follows the actual serialized
TEAMS count initialization. `from_world_constructor(initialized_slots, latch)`
also accepts an explicitly supplied current initialized array. Neither assumes
that native allocation capacity8 means eight loaded team slots.

`register_restored(state, id, held_record, registry, catalog, effects, pc)`
borrows the actual held allocation. Its registry entry may be `Loading` while
the loader owns the record. The saved ID must match that current registered
generation. Runtime flags, name, class, Team+38, City presence and Actor+158
come from that allocation, not a decoded inspector clone. Definition+34 is
resolved lazily through the current original Catalog owner and its canonical
source identity. It does not force unrelated ground/mesh/economy fields.

The required `Effects` methods have no successful fallback. Actor/Kingdom
registration calls actual state/grid service `4D62F0` at the native point;
duplicate-city assertions require an actual `855D30` result. The current test
adapter explicitly refuses the unbound actor service after the native preceding
flags/name effects. `SpatialWorld::publish_actor` alone is not that service.

The root loader integration must move this **same State** with the registry
into `Attempt`/`Staged` and route triggers through its name table. It must call
registration only at the real `4FAE80 -> 4F9510` point after preceding lifecycle
services succeed. It must not rebuild these lists from GTID traversal or create
another mutable name owner. `State::stored(&ActorStore)` gives a read-only view
after the owned registry move, resolving retained handles through current
generation-checked keys. It neither republishes nor changes completion status.

World+8 Object Init storage belongs to the separate `world_object_list` service.

## Original order

1. Capture the instance name (`4BEDF0`), then clear Thing+8 bit31.
2. For a nonempty captured name, remove the first matching name entry and
   prepend its current GTID through the existing proved native name table.
3. Test virtuals in order +BC, +C0, +AC, +B4. Area and Camera prepend their
   pointer lists. Team+38 indexes the actual array; the same pointer is accepted,
   another live pointer raises the original load assertion before replacement.
   Actor/Kingdom requires `4D62F0`.
4. `4F4C40` checks Object virtual+B0 and definition virtual+24 (ObjectData+34).
   Otherwise it requires Actor+B4 and City+D4, then generation-checks live
   Actor+158 and checks that referenced object's source definition+34.
5. A false B3CC40 latch permits duplicate search. A duplicate invokes the
   assertion and stores its returned latch. Insertion still happens afterward,
   even if the assertion returned false. The GTID is reread after the callback.

Ordered prior changes survive every later missing service. State entry/grid
effects may change live fields; the city guards read them afterward. The
concrete held-record API requires stable allocation/GTID/class/source identity
throughout callbacks, not arbitrary object destruction/replacement or reentry.

## Evidence

`native_registration.py` executes unchanged original and v14 instructions under
the page-zero/allocator-guarded phase8 movement harness. `native-registration.json`
contains:

- 414 registration rows =69 inputs x2 images x3 precision controls;
- 246 completed rows,162 dependency-stop prefixes and6 duplicate-Team exceptions;
- 36 executions of `430240` =6 counts x2 images x3 precision controls;
- 558 executed instruction pins, with actual class leaves, source byte+34 leaf,
  names/hash/compare/list code and `4E10B0/4E1AD0` generation checks executed;
- 40 original/v14 class-vtable leaf pins and constructor/dispatch byte regions.

Cases include all currently owned classes1..7/9; non-GTID publication order;
Area/Camera duplicates; Team same-pointer acceptance/conflict; City presence,
required reference null/stale/current/self; both source-space values; actor
callback field changes; both initial latch values and both assertion results;
and stopping before each required boundary in a complete actor/city sequence.

The explicit seams are string capture/ref ownership (stable ASCII UTF16 fixture
buffers), allocator/free, CRT thread-locale lookup (native C-locale comparison
executes), Actor4D62F0, assertion855D30 and LoadException4F0110. Successful
allocation is proven; portable allocation errors are explicit refusals rather
than claims about native null-write/SEH handling.

`native-construction-pins.json` additionally ties eight saved class constructor
vtable writes to the actual predicate leaves and pins all six empty class/city
list stores in original `4F7D00`. `4FCA6D..86` reads the serialized TEAMS count
and passes World+16C to `430240`, which sets +170 and nulls each initialized
slot. Native capacity growth8->16->17 is storage, not a gameplay cap.

B3CC40 is in PE zero-fill storage in both images: section VA B21000,
virtual size177604, raw size90112, offset113728. It starts zero in a fresh
process. The World constructor does not reset it; an existing process must keep
the actual assertion result instead of supplying a guessed false on every map.

Focused Rust tests replay every registration and team initialization row and
all predicate pins. An original-data test uses actual restored Kursk Actor3,
Kingdom4 and Team5 records: held shell address/flags/name ownership, required
actor-service stops, lazy source-owner validation, actual source+34/City/+158
reads, generation checks, and the actual registry move into ActorStore. It does
not promote saved completion states. That map's restored registry has no
unmodified Area6/Camera7 cases; those classes are covered by native fixture
inputs and static constructor/vtable pins, not an actual-map storage claim.

## Reproduction and boundaries

Run the producer only when deliberately regenerating its fixture:

```powershell
python research/skirmish-20261016-live-registration/native_registration.py
$env:CARGO_TARGET_DIR='target/s-graphics-review'
cargo test -p aa-runtime --test live_registration_native --release --locked --offline -j4
$env:AA_GAME_ROOT='E:\Games\Axis and Alies'
cargo test -p aa-runtime --lib live_registration::tests:: --release --locked --offline -j4 -- --include-ignored --nocapture
```

For independent immutable replay, import `native_registration`, select each
fixture's `image` through `h.P`, construct `Machine(row['input'],row['precision'])`
and compare `run()` to `row['output']`; do not call `main()` or overwrite fixtures.

This is not full Init/Create/PostLoad/startup, Actor4D62F0 state/property/FX/grid
binding, SAI/updatable registration, object deletion/list removal, class8 Group
restoration, multiplayer, skirmish completion, a complete engine or a parity
percentage. Names retain the established ASCII/no-NUL C-locale supported domain.
Pointer-list consumers require objects to remain alive; stale portable handles
are explicitly refused rather than silently resolving another generation.
