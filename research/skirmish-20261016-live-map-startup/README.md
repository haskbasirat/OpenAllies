# Immediate saved-map initialization: bounded native services

This research implements additional native services needed by the current
saved-record loader. It does **not** certify a complete initialized map, an
ordered simulation tick, a playable battle, or multiplayer. Original TimeGate
executable behavior is the authority; no Gamebryo SDK source was copied.

## Why map loading remains on the battle critical path

Native `4FAE80` completes each record's Load (`+D8`), then, when its current flag
bit 7 is clear, runs Init(NULL) (`+E8`, checks AL), PostCreate (`+EC`), and
`4F9510` publication before advancing to the next record. A decoder or an
inspection pass that loads all records before a later post-load pass cannot
stand in for that lifecycle. Later records can observe the initialized state,
lists, membership, relationships, and component effects of earlier records.

This folder also implements opt-in loader lifecycle/context integration under
the root agent's ownership. Other agents own creation-placement `4F4590 ->
4F3640`, `4F9510` registration, native
terrain ownership, and pre-staging spatial membership. This folder does not
claim their unfinished integration or independent review as complete.

The source-only capture `actual-first-records.json` identifies the first present
Team, first present Kingdom and first dynamic record in tutorial02, Firefight
and Berlin. It executes no Init/PostCreate/publication. Tutorial02 has no Team
record; its first Kingdom has canonical definition `kingdom`, class 4, ID 781,
and pre-Init flags `A0C46900`. Firefight and Berlin reach Team records first.
Map rule flags are not a substitute for the optional MapResource mode read by
`4F0460`, and that mode is not the Sim phase read by `76AC90`.

## Concrete additions

`crates/aa-runtime/src/world_object_list.rs` owns GWorld+8 and its intrusive
link semantics. The actual `4F7D00..4F7D47` constructor prefix establishes a
null head. `4F1F70` checks the current bit 8, allocates a node, prepends it,
and changes the previous head's backlink to the new node's next-link word.
Duplicate object IDs are allowed. The Rust owner is growable, has no Clone
implementation, and retains identities rather than mutable actor-state copies.
Removal, native allocation failure/unwind, and allocator reentrancy are outside
the proof.

`crates/aa-runtime/src/live_object_init.rs` binds the existing saved
Object Init(NULL) coordinator to the actual current held LoadShell and the
same identity registry. The registry entry must be the Loading placeholder.
Source binding checks canonical identity, source hashes/spans, template chain,
category, native definition flags, and the explicit x87 precision. Shape and
height come from the actual authored native definition, not renderer bounds.
It writes the current shell directly, including scaled height +38, byte +4C,
snapped XY, sampled Z, cached tile coordinates, and the world-list insertion.
Loaded +50..+5C words are retained unless an actual service changes them.

The adapter uses the existing proved `46EE00` snap and `4F0E50` GroundSurface
height query. It does not allocate its own terrain or infer missing terrain
values. Source geometry uses the actual completed source Bank's precision and
source-identity receipt; live Init/footprint arithmetic has its independent PC.
There is no source-PC/runtime-PC equality assumption. The source-only capture
cannot construct a live Source without that Bank owner. Earlier v2 capture
wording predates this stricter receipt requirement and is diagnostic only.
Its failure test confirms that earlier native-order writes and
mutations by a reached required callback survive a later service error.

Two additional native entry gates are concrete. Exact Object `A2A2A0`, Actor
`A273F0`, and Kingdom `A299D8` vtables all dispatch +B0 to `6A71D0`, whose bytes
are `B0 01 C3` (true). Therefore `4EE4C0` calls the required membership service
when bit 26 is clear; it returns without a membership change when bit 26 is
set. `4EB6E0` returns before its blocking query when current bit 10 is clear.
Insertion and a reached blocking query remain required services. An early
address-label mistake during investigation was corrected against these bytes
before any production gate used it; no false +B0 behavior was implemented.

The opt-in `memory_eligible` helper ports `4EB490 -> 4EA350`, including actual
class +A8 semantics: false for Object/Actor (`6A71F0`, `32 C0 C3`), true for
Kingdom (`6A71D0`, `B0 01 C3`). It preserves ordered/lazy reads of Sim+F8,
ActorData+174, current object flags, and late definition byte +16D. The phase
and +16D owner are mandatory `MemoryServices` inputs; an absent component mask
also errors when reached. This helper neither clears/creates detection memory
nor implements the full `4EBC70` visibility service.

The opt-in `occupancy_indices` helper uses the separately proved mode-one
`aa_spatial::creation_placement::occupancy_tiles` over actual blocking shape,
current pose, and MapGeometry. The global point shape must be supplied as an
explicit owner. It does not cache or invent tile zone sets.

`crates/aa-mapworld/src/restore_lifecycle.rs` now drives the existing proved
`record_lifecycle::finish` at the end of each successful Load, including source
Team/Kingdom stages and embedded records. The actual record stays outside its
registry slot as Loading through Load, ordered owner effects, and the immediate
tail. Required Init/PostCreate/registration/predicate/publication/warning/
destructor callbacks default to errors. Existing Catalog adapters retain
`DiagnosticOmitted`; this mode records omission and cannot indicate completion.
The real game-side services still need to opt in and bind these callbacks.

The private out slot supports checked ownership moves to another existing
generation and explicit nulling by returning the held allocation to its
registry slot. Destruction requires a receipt created inside the required
4F5490 callback after actual cleanup; the held out value is cleared only after
that succeeds. Invalid registry mutation preserves the displaced actual
allocation in report.lifecycle_detached, rather than cloning/dropping it or
overwriting another generation. A destroyed-slot replacement beyond this
bounded ownership domain errors explicitly. Nested return values carry the
actual output identity separately from consumed source bytes. Failed Load
never runs Init; the existing caught-Load cleanup stays before the tail.

Each reached record has a lifecycle report with omission, attempted/completed
state, actual output ID, successful/error callback prefix, and error. Runtime
publication readiness remains false. World versions below23 explicitly refuse
the unported legacy building-zone/pre-tail path in Immediate mode. This is not
a blanket certification of all native4FAE80 allocation/exception/legacy paths.

## Native evidence and verification scope

The three Python oracles remap original and v14 PE images independently and
assert every observed image instruction byte is identical. Every case runs at
PC24, PC53 and PC64. The original/v14 hashes are asserted by the reused
`research/phase8-conversion/native_machine.py` harness.

| Evidence | Paired rows | Image executions | Scope |
|---|---:|---:|---|
| native-world-object-list.json | 36 | 72 | Constructor null head; repeated insertion/backlinks, successful allocation seam only |
| native-object-init-gates.json | 360 | 720 | Actual +B0 leaves and membership/blocking entry gates; stops before reached child services |
| native-memory-eligibility.json | 1296 | 2592 | Full predicate, actual type/class virtual leaves and phase getter, no service seams; ordered scalar reads |

These denominators are test inputs, not a percentage of the game. A successful
leaf or coordinator test with mandatory service seams cannot establish the
children's behavior or an end-to-end live map. The owned-shell adapter test is
synthetic storage/ownership/error-order validation, not a native gameplay run.
See the test logs and the root checkpoint's final QA acceptance for integrated
results. Independent review, when present, is recorded separately rather than
implied by this author report.

The immediate-tail coordinator's existing independent native evidence is in
`research/skirmish-20261016-restore-lifecycle`; this folder reuses it without
editing the coordinator. New synthetic adapter tests verify ownership,
interleaving, partial failures, nested output replacement/null, required
destruction receipts, diagnostic omission, and unsupported-service refusals.

Reproduce the bounded verification from the Rust workspace:

```text
python research/skirmish-20261016-live-map-startup/native_world_object_list.py
python research/skirmish-20261016-live-map-startup/native_object_init_gates.py
python research/skirmish-20261016-live-map-startup/native_memory_eligibility.py
cargo test -p aa-runtime --lib live_object_init --release --locked --offline -j4 --target-dir target/s-skirmish-start-review
cargo test -p aa-runtime --test world_object_list --release --locked --offline -j4 --target-dir target/s-skirmish-start-review
```

`--write` creates a new native JSON once and refuses to overwrite it. The
ignored source-capture test additionally requires `AA_GAME_ROOT`; its optional
`AA_LIVE_STARTUP_OUT` must be absolute and absent. It writes only the selected
research output, never source game data.

## Actual next services and ownership boundaries

- Placement: `4F4590` must reach real `4F3640`, never movement's `4F3C40`.
  Required source-completion words such as +430 must have an actual producer.
- Membership: the first Kingdom requires `5536B0`. Existing class-3-only
  ActorStore SpatialWorld cannot be substituted for the held class-4 shell.
  A pre-staging owner must preserve native partial updates and ultimately
  transfer placement ownership without two mutable pose/flag copies.
- Ground: the actual MapHeader creator +3C/+40 extents, TERRAIN resolution and
  resolved native variation definitions feed the existing layout/terrain
  binders; actual HEIGHT_MAP and MOUNTAIN_HEIGHT_MAP feed height layers.
  BoundTerrain.cells[].flags20 supplies original index flags. Sampling cells
  alone do not carry tile flags, zone sets, or membership.
- Footprint: `4EA150` uses `4CF210`, then `554DD0` mode 1 and `554330`, and
  calls tile+4 zone-set virtual+58(actor). Mode 1 uses the global point-shape
  owner at B3A8C4; placement's mode-0 full-cell shape is not interchangeable.
  `aa_zones::detection::refresh_from_tiles` already implements zone-set visit
  behavior once real occupancy indices and an owned ZoneMap are provided.
  No empty ZoneMap is inferred from an absence of decoded zones.
- Visibility: +58/+5C detection-memory ownership and full `4EBC70` callbacks
  still need actual owners. The eligibility predicate alone is insufficient.
- Derived initialization: Object Init is only the base of Actor `4D7230` and
  Kingdom `4E5DF0`; component Init/AfterInit, required attachments, refreshes,
  recruitment, actual resource flow arrays, and PostCreate remain separate.
  Zero Kingdom flow arrays or a partial reset fragment cannot complete Init.

## Measurable battle milestones

1. An initialized world: a strict actual map load completes every immediate
   lifecycle step, with no missing required service or pending record; registry,
   world lists, spatial membership, source identities, relationships, terrain,
   components, and RNG ownership agree at their real publication boundaries.
2. Ordered simulation: the initialized world runs the native ordered update
   phases using those same owners, with command/property/AI/economy/movement/
   combat effects and deterministic state checks rather than isolated probes.
3. Playable skirmish: selection and orders lead to movement, attacks, damage,
   morale, recruitment/construction, an opposing AI, victory/defeat, and a
   save/reload of the same battle. Player-visible testing must exercise them.
4. Multiplayer: the same battle converges across host/client simulation,
   serialization, ordered commands and RNG, with actual joining, save transfer
   and internet tests. Separate-connection tests remain blocked unless the
   required machines/connections are available. Campaign follows skirmish and
   multiplayer, as the user requested.

No reliable completion percentage or finish date follows from the current
backend test totals. Remaining work includes unknown descendants of required
services; a multi-month project is a planning assessment, not a delivery date.

No original executable, RMP/NIF/TGI asset, released build, root Cargo manifest
or lock file was changed by the author of this folder. Shared integration and
release preservation are checked separately by the root checkpoint.
