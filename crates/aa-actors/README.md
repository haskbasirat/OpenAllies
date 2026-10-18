# aa-actors

Original/v14 object identities and the actor component lifecycle boundary for
the independent Rust runtime. This crate advances real registry and component
state. It does **not** yet instantiate a complete playable actor or implement
the entire `GWorld::CreateGThing`/deployment path.

`ActorId` is the native 32-bit GTID stored at object+0x14: low16 slot, high16
generation. The same identity manager also contains non-actor game objects.
`IdentityRegistry<T>` owns caller-supplied state behind stable boxed entries.
Normal allocation, saved-ID registration, removal, identity exchange and
ascending-slot enumeration follow the original manager. `get_actor` adds the
explicit native IsActor classification. It does not parse a name or infer a
type from a definition string.

Normal Add permits allocation while the current live count is below32,768 (so
it can reach32,768) and scans slots1..65,535 from the current cursor, wrapping
to1. That cap and the 16-bit slots are original
format/algorithm behavior, not a chosen host-memory budget. Saved-ID registration
is a different original path and does not apply the normal Add cap. Removal
increments the u16 generation with native wrapping and leaves the returned
object's old ID intact. After a complete generation wrap an ancient ID can alias
again; this is not a promise of permanently unique identities. A Rust checkpoint
preserves cursor, generations, occupied entries and highest slot; it is not an
RSG save decoder. Restore validates these registry invariants, not game-world
relationships or component payloads.

`Components<T>` owns the 31 original component slots and caller-supplied raw
actor state words. Body is0, Mover is3, and Mind is4. The API accepts a proven
definition+0x174 mask; it does not derive that mask or runtime values from TGI.

* `assemble_initial` reproduces the original initial-mode constructor selection
  order and bit conditions, then validates required components and invokes the
  explicit construction notification. Construction15 is considered first and
  Mind4 last. Mind selection deliberately does not require definition bit4.
  The factory callback must implement the actual constructor/subclass rules;
  null factory results are retained where native permits them.
* `validate` obtains each present component's native virtual+0x20 requirement,
  checks its slot, and stops at the first missing dependency.
* `init_create` is the ascending component virtual+4 loop after original base,
  owner, stance and nested-actor setup have succeeded.
* `init_existing` initializes every present component except Mind ascending,
  initializes Mind last, then invokes virtual+0x14 ascending. It starts after
  the original base spatial and required-object phases and stops before later
  owner/state/ability work.
* `post_create` invokes the component virtual+0xC loop with the unchanged record
  and flag, between the original base and owner/city notification phases.

Callbacks can mutate the table/state. Subsequent conditions and loops reread
that state as native does. Failure preserves prior effects: do not automatically
retry or assume an atomic spawn/rollback. The assembly return records installed
slots in creation order; it is not a replacement for native raw-pointer lifetime
or the separate runtime-reassembly mode's newly-created component list.

The caller can put `Components<MyComponent>` in `IdentityRegistry`, connect Body
and Mover constructor callbacks to their separately verified implementations,
and drive these phases at the proven world boundary. No owner, position, facing,
random seed, health, defense, speed, AI or component default is invented here.
Native object IDs may be read from a verified creation/save record; map thing
definition IDs and renderer placement indices are not GTIDs.

`creation::construct` now implements the bounded native base `GThing`
constructor, given an explicit verified definition identity/flags, creation
record, world time, world RNG and arithmetic precision. It preserves original
axis-lock precedence, random-facing draw count, input-direction validation and
mutation, and 2D/3D normalization. Random directions come from the complete
original 2,048-entry tables, selected by explicit startup precision; startup
precision is independent of current simulation precision. `initialize_identity`
then applies the native new/saved GTID branch. No derived `GActor`, spatial
placement, owner, component constructor, world publication or rollback is
implied. See `research/phase5-actors/README.txt` for exact boundaries and evidence.

The order crate's movement selector remains a UTF-16 CString. Native0x4F8E90
first resolves aliases and otherwise uses raw CString pointer bits before GTID
lookup. Decimal/hex string parsing is not an equivalent implementation. Alias
construction/publication, selector decoding and permission stay outside this
crate. `ActorId::from_raw/raw` bridges only already-proven native handles.

Run `cargo test -p aa-actors` and
`cargo clippy -p aa-actors --all-targets -- -D warnings`.
The nine Rust tests compare 33 actual registry transitions, 85 original component
scenarios, the 31-entry native enum, 282 original base constructor/identity cases
and all 6,144 startup direction vectors. See `research/phase4-actors/README.txt`
for original instruction ranges, oracle seams and missing lifecycle systems.

The safe API rejects stale handles, invalid saved slots/checkpoints and invalid
requirement enum values rather than reproducing dangling pointers/assertions.
Constructor allocation success is a parity precondition; Rust allocations are
not a whole-process OOM recovery layer. Original files and profiles are read-only.
