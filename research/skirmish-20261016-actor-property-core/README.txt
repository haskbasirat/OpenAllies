Native Actor +108/+10C list rebuild, 2026-10-16

Scope and integration
  aa_body::actor_property_list::{State<T>,World,refresh,refresh_parent}
  Original4D4CF0,4D4E50 and member iterator4364C0. This is NOT the existing
  PropertiesComponent581420 property_refresh module, which remains unchanged.
  +108/+10C is an Actor linked list whose values are native definition pointer
  identities. Builder+ A8 gates the ActorData+504 BuildActor source list. No
  numeric property aggregates or scheduler are read/written by these functions.
  Runtime source/identity resolution is delegated through mandatory World calls.

Authoritative state
  State<T> is a growable node arena with current head/tail and stable logical
  NodeId(usize). It derives Clone/Debug but is not a success-only projection.
  Native constructor head/tail are null; the root session wires actual State
  into ActorShell. Node allocation leaves value/next explicitly unwritten.
  Value and next stores, head/tail stores, allocation and free are independent
  operations. Earlier effects stay visible if a later dependency is unavailable.
  Reading missing/freed/unwritten fields fails; no fabricated null or empty
  canonical source is produced. NodeId is local to the owning Actor State;
  a World adapter must include actor identity in its cross-actor node handles.
  Free removes the node only; it does not silently change head/tail. Head/tail
  changes occur at their exact native store points. There is no fixed list cap.

Exact native order
  Clear existing list first: read current head, capture next, store head, clear
  tail only for a null next, free the old node; repeat current head after free.
  Only then test Builder+A8 and canonical ActorData+174 bit18. Capture canonical
  ActorData+504 head and prepend EACH source node, including duplicate values.
  Before allocation capture current destination head; after allocator returns
  reread source value, store new value then next, read current tail, store head,
  and set tail only if the captured prior tail was null. Read source next last.
  Capture Organization+7C once. Native4364C0 scans current signed member count
  and current member slot, skipping null slots; after it selects a slot the
  coordinator reads that slot AGAIN. For each member source node, search the
  current destination values by identity. A found value is not added. If absent,
  prepend in the same order as above. Own BuildActor duplicates remain intact.
  Finally4D4E50 reads current Element+80, then its current parentActor+10 and
  recursively rebuilds that Actor. No Organization+4 owner indirection exists.
  The Rust tail loop preserves these effects without consuming Rust call stack.
  No invented cycle-suppression or successful default callback is present.

Source correction before freeze
  Initial pseudocode interpretation mistook Element+10 for an Organization
  pointer. Actual4D4E50 instructions directly move it into ECX and jump4D4CF0.
  The new API was corrected before proof/freeze: element_parent returns Actor.
  Independent reviewer confirmed the correction from the original instructions.

Native proof
  native_actor_property_core.py executes original AA.exe and accepted v14 in
  separate mappings at PC24, PC53 and PC64. All visited native instructions are
  byte-pinned to both images and the original/v14 outputs must match exactly.
  314 inputs x3PC =942 stored rows, representing1884 actual native executions.
  843 rows stop at explicit dependency boundaries across every boundary and
  occurrence of full member-union and parent-propagation cases. Nine rows have
  explicit allocation/free mutations. Cases cover constructor-empty lists,
  clear/free order, Builder/flag gates, duplicate own sources, member dedup,
  nullable slots/current iterator counts, direct parent recursion, allocation
  source mutation, free mutation of current head and membership count mutation.
  Full lazy read/write/service traces and every allocated/freed/unwritten node,
  current per-Actor head/tail and error prefix are compared by Rust replay.
  Allocator6F7810(8) and free6F7430 are the only native service seams. Allocation
  fills new native memory0xA5 so no zero-filled node fields are fabricated.
  Ordinary field observers for dependency/mutation probes are instruction hooks,
  not claims that native field reads are virtual services. Stops occur BEFORE
  store instructions. Page-zero access faults except genuine FS:[0] references.
  The wrapper performs no floating point arithmetic; all PCs are still executed.

Validation
  native_actor_property_list integration:2tests passed (942 native replay rows
  and explicit authoritative state unwritten/freed identity behavior).
  Existing native_property_refresh integration was also rerun unchanged.
  Scoped strict Clippy passed, no-deps, -Dwarnings, no lint suppressions.
  Own target/s-borrowed-leader; logs retained alongside this report.

Required remaining services / limitations
  This core does not produce canonical BuildActor504 data, resolve definitions,
  mutate a saved registry or run live action publication. Those adapters have
  separate source evidence and must bind actual list storage. The Value equality
  operation must compare canonical pointer/definition identity, not guessed
  names. A missing source/actor/node/service remains an explicit error after
  its true earlier native effects. No complete world startup, PostCreate,
  Actor Init, battle, skirmish or multiplayer completion is claimed.
  Invalid native cyclic linked lists/parent graphs do not gain fake completion;
  the host may explicitly reject an invalid graph. Native allocator-null and
  out-of-memory exception/unwind behavior are outside this bounded proof.
  No SDK implementation was consulted, copied or integrated. No original game
  files, saves, profiles, dependency manifests or release binaries were changed.

Reproduce from rust-rebuild (PowerShell)
  python research/skirmish-20261016-actor-property-core/native_actor_property_core.py
  $env:CARGO_TARGET_DIR='target/s-borrowed-leader'
  cargo test -p aa-body --test native_actor_property_list --test native_property_refresh --release --locked --offline -j4
  cargo clippy -p aa-body --lib --test native_actor_property_list --release --locked --offline -j4 --no-deps -- -D warnings
  Normal Python execution replays and asserts existing fixture equality. It
  writes the fixture only with the deliberate --write regeneration argument.
