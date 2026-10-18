# Concrete saved Team and Kingdom initialization

`aa_runtime::load_kingdom_init_world` binds Team4E68C0 and Kingdom4E5DF0 to
the actual held `live_object_init::Record`, actual derived `SpecialShell`,
shared GTID registry, current canonical source Bank, Resource owner, managed
observer and persistent World registration owner. There is no cloned Actor or
Kingdom copyback. Root's immediate-world-02 observation also composes the real
Object/terrain/session services and reaches the later4F9510 registration phase;
that observation is separate from the bounded tail fixture below.

## Owners and order

- `KingdomRegistryState` owns World+144 count, all-Kingdom+148 slots,
  major+154 slots and color-count+160 allocation. Native constructor
  4F7E63..4F7EDF establishes zero count/lengths; source/saved loading then
  creates the initialized slot domain. `from_inventory` uses real KINGDOMS
  counts and the completed KingdomColor registry, not GTID reconstruction.
- Native4F8AF0 increments count before all-slot checks/stores, then major
  checks/stores, then Color+C lookup/increment. Occupied slots include a
  repeated same pointer. Later errors keep earlier writes. The narrow lazy
  color helper retains the loaded canonical reference and resolves its actual
  manager ordinal without a mutable mirror or a second Translate operation.
  Native47A6FD..47A71C stores manager+28 at Data+C before93D4E0 appends it.
- Actor Init uses the same Record and the existing concrete Actor adapter.
  The three flow arrays are cleared in native interleaved row order. Suffixes,
  stockpiles and unrelated fields survive. Their allocation metadata is kept.
- Team membership uses one typed head-first `SpecialShell.team_members`
  allocation, including duplicates. Team+68 is a separate wrapping scalar.
  Constructor-empty head/tail are projected as null only while actually empty;
  independently mutable raw44/48 mirrors were removed.
- Recruit traversal is UNIT19 then TRUCK42, current category-list order.
  Category nodes and Bank definition tokens are distinct handles. Source roots
  require exact registered provenance/template identity. Typed string getters
  include native quote removal. Element+2C8 primary is `required_properties`
  at+0; secondary is deferred `required_structures` at+C. Actual canonical
  property/Structure lookups and duplicate/unknown failures remain required.
  ActorData+17C is read only after both earlier heads are empty. Restriction
  source ownership is checked lazily at the actual Team/Kingdom profile use.
- Plain recruits use the actual typed table and current source tokens.
  Actual major slots remain nullable. Kingdom virtual+138 is4E5250
  (self or relation code1), distinct from+13C4E5280 (code2). Both relation
  masks write zero first and then each native bit update immediately.
  +22C is a WORD store preserving adjacent bytes.
- Team placement borrows the real Object placement adapter and actual Point
  owner. Native Team runtime/data vtables and their false/point-copy leaves
  are independently pinned. Terrain retains its source-PC receipt; live
  arithmetic precision is a separate input. Team B0=false preserves the
  reached4EE4C0 membership gate before any spatial service is requested.

## Native evidence

Every generator loads the two checksum-verified images separately at all three
x87 precision controls. Native instruction bytes are checked against both PEs.
Normal invocation is immutable; only `--write` regenerates its fixture.

- `native_team_leaves.py`:72 rows; actual Team constructor vtable store,
  TeamData point transport, AL-false leaves, full EAX-zero Data leaves, and
  real430240 zero initialization for Color count arrays. Arbitrary point
  inputs prove transport; they do not assert the global startup producer.
- `native_world_registry_ctor.py`:6 rows of unchanged constructor stores and
  successful allocation. Spare capacity remains unwritten, not zeroed slots.
- `native_color_ordinal.py`:48 rows; actual Data+C assignment and native
  vector append, with an explicitly supplied successful constructed Data.
- `native_registration.py`:48 paired cases/96 executions. Exact successful
  stores and failed prefixes include occupied same/other slots, signed invalid
  indexes, sparse growth, unavailable color storage and invalid color indexes.
  Unsafe/unbound cases stop before the native required read and are labeled
  bounded ownership refusals rather than native valid-input behavior.
- `native_relations_masks.py`:48 paired cases/96 executions. Real native
  Kingdom/Team relation lookups, virtual dispatch, nullable major iteration,
  both masks and WORD preservation; no relation or mask answer callbacks.
- `native_actual_kingdom.py`:full4E5DF0 using the actual all-PC tail capture.
  Actor Init and restriction eligibility are explicitly named service seams;
  no full native TGI source loader is asserted. Native category traversal,
  all flow stores, plain-item construction/hash insertion, world registration,
  real relation dispatch and masks execute unchanged. Unread source fields are
  poisoned and guarded. The Resource count comes from the pinned existing
  source-resolved Resource-domain evidence, independently of reset output.

The Rust focused tests replay registration and cross-table mask fixtures over
owned storage. Their synthetic relation-table fixture uses an original saved
Kingdom solely as an unread container; it makes no actual-world diplomacy
claim. The actual tutorial02 test creates a fresh original saved load for each
live PC at fixed source PC53, retains shell/derived allocation addresses, and
substitutes only the explicitly named Object Init service. Its source census
records the exact lazy source fields and restriction answers reached by Init.

## Required boundaries

This module does not claim complete battle/world/gameplay parity. Legacy
50AC00 scheduling for old versions, active79E170 SAI registration, Team
placement-failure diagnostics/map-warning owner, and changed Team spatial
membership still require their actual services. Sparse vector growth that
would create uninitialized native holes is refused, preserving earlier stores;
exact-end growth is supported. Negative or out-of-owner indexes are refused at
their reached read. No missing source, component, list, Resource owner,
restriction manager, observer, Point owner or registration allocation becomes
an empty/successful operation. Heap exhaustion/SEH/reentrant allocation and
exact native diagnostic sink internals are outside the successful-allocation
fixture scope. Existing native capacities are storage metadata, not Rust
runtime budgets. Original files, saves and release packages are untouched.

```powershell
$env:CARGO_TARGET_DIR='target/s-owner-recruitment'
$env:AA_GAME_ROOT='E:\Games\Axis and Alies'
cargo test -p aa-runtime --lib --release --locked --offline -j4 load_kingdom_init_world::tests -- --include-ignored
python research/skirmish-20261016-load-kingdom-init/native_team_leaves.py
python research/skirmish-20261016-load-kingdom-init/native_world_registry_ctor.py
python research/skirmish-20261016-load-kingdom-init/native_color_ordinal.py
python research/skirmish-20261016-load-kingdom-init/native_registration.py
python research/skirmish-20261016-load-kingdom-init/native_relations_masks.py
python research/skirmish-20261016-load-kingdom-init/native_actual_kingdom.py
```
