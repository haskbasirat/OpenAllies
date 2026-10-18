GKingdom::Init phase coordinator -- 2026-10-16
===========================================

Implementation
  crates/aa-economy/src/kingdom_init.rs
  aa_economy::kingdom_init::initialize(kingdom, argument, precision, world)
  crates/aa-economy/tests/kingdom_init.rs

This is the whole control/order coordinator for original4E5DF0. Required
runtime effects are host callbacks, with no default implementations. It is not
a completed runtime host and does not authorize post-init publication.

Native sequence
1. Actor::Init4D7230 receives the incoming argument and precision. False
   returns immediately; no diagnostic, scheduling, reset or registration follows.
2. Init diagnostic4BEDF0/404F60.
3. Read current global GWorld+2C. If its unsigned value is below22 and the
   current kingdom+184 is zero, invoke50AC00 on kingdom+180 with argument0.
   The original file version cannot substitute for this current state read.
4. For each live ResourceCount index, store +0 to1A0[i],1AC[i],1B8[i] in that
   order. Grow each vector only as needed; preserve existing suffix and all
   stockpiles. Reread GameData+38 after each complete row.
5. If kingdom+1EC Team exists, prepend kingdom to Team+44; initialize tail+48
   only if empty. Increment Team+68 with the original32-bit wrapping behavior.
6. Original4BC6C0 filter iterates UNIT(category19) then TRUCK(category42).
   Read each current node/definition; require component mask bit8, then lazily
   short-circuit the heads at *(ActorData+2C8)+0, +C, and ActorData+17C.
   If any exists, call4E1810 using actual restriction profiles. If accepted,
   construct a plain available recruit item (vtable A29650, definition+4,
   byte+8=1) and pass ownership to4E4C30. This is NOT RecruitSingle A29694.
   Read next-node links after the callbacks; read the next category head lazily.
7. If kingdom+190==0 and current global SAI+20 is nonzero, call79E170.
   Native79E170 is a thiscall with ret4; the oracle seam uses that exact ABI.
8. Call world4F8AF0(kingdom), including actual world registration side effects.
9. Traverse the live world+154/+158 major vector with4F0680/4F06B0 null skipping.
   For each selected other kingdom, call virtual+138(new kingdom). If true,
   refresh other+228 via54EBA0(other), then other+22C via559600(other).
   Count/slot mutations inside callbacks affect later traversal exactly as native.
10. Return true after all required phases complete.

Timing and ownership
LoadThing4FAE80 calls the Kingdom virtual+E8=4E5DF0 synchronously after its Load,
before the next record, unless Thing flag bit7 skips Init and queues the object.
The outer caller must honor that gate; initialize itself is the virtual body.
The related independent lifecycle audit is in
  research/skirmish-20261016-saved-flows/lifecycle-proof/
Runtime reset_flow_vectors is a proven leaf, not an implementation of this
whole phase. No existing runtime or parser source was modified here.

World API
Host methods cover Actor Init, diagnostic, legacy scheduling, current facts,
per-cell flow mutation, Team membership/count, definition category nodes and
backreferences, restrictions, plain recruit ownership, SAI registration, world
registration and relation masks. IDs refer to actual host owners. List nodes
remain distinct from contained ActorData IDs. Read callbacks use current state;
they must not return an earlier snapshot or fabricate absent source links.
Every callback error propagates immediately while preserving earlier changes.
The host must finish real Actor Init and scheduling before reaching flow reset.

Native evidence
native_kingdom_init.py / native-kingdom-init.json execute the full original
and accepted v14 body at PC24, PC53, PC64:33 scenarios,99 compared rows,
198 executions. All executed instruction bytes match original and v14.
The coordinator itself performs integer/pointer operations and zero stores;
the three precision modes establish that no caller FP mode is invented.

Unmodified native behavior in the oracle includes:
- all outer branches and legacy-version gate;
- actual vector length updates and three flow stores per resource;
- Team head/tail linkage and counter writes;
-4BC6C0/4BB3E0/46C8C0/4BB3F0 category iterator;
- raw recruit-item construction before ownership callback;
-4F0680/4F06B0 major-vector iterator;
- final return and normal temporary filter-vector cleanup.

Declared seams:
- Actor Init, legacy scheduling,4E1810 restriction outcome,4E4C30 ownership,
 79E170 SAI,4F8AF0 registration, other kingdom virtual+138,54EBA0/559600.
 These record invocation order and apply explicit fixture effects; they do not
 prove those complete runtime implementations.
- operator new/free and vector capacity reallocation services;
- diagnostic CString/name and log formatting.
No instruction or branch in4E5DF0 itself is replaced. Protected native_machine
rejects non-FS null accesses, and unexpected native faults fail the generator.

Cases include Actor Init false, arguments0/9/77, versions21/22/23/FFFFFFFF,
legacy scheduler already present, negative/zero resource count, unequal/empty
flow lengths, preserved suffix/stockpiles, resource count growing/shrinking in
the middle of a row, Team empty/nonempty/count wrap, active/inactive SAI,
minor-kingdom SAI skip, Actor Init changing the subsequent legacy gate,
short-circuit recruit predicates, category link insertion during restriction,
null first/second category heads, missing required recruit backreference,
major-vector growth/shrink/null mutation, and ten interrupted callback points.

Two native fault boundaries become explicit Rust errors:
-4BC6C0 initially selects the first UNIT head without skipping a null head.
 46C8C0 would dereference it. Later empty TRUCK categories are skipped normally.
- With Element bit8 set, a null ActorData+2C8 owner cannot be treated as empty.
 With that bit clear the pointer is not read, and the fixture proves the skip.

Validation and limits
tests.log: targeted aa-economy kingdom_init test replays every native row,
including exact event order and final flow/stock/Team/recruit/major state.
clippy.log: strict scoped Clippy result.
Callback-failure cases deliberately stop the native emulator at the callback
entry and compare the prefix. They do NOT claim C++ exception-unwind, allocator
failure, or destructor side-effect parity. The native temporary category-filter
allocation becomes Rust local state. Its native43-slot initial capacity was
growable, not a gameplay cap; the original two-category filter is preserved.
Runtime owners, Actor Init, scheduling, recruitment and world/relation bindings
still need actual implementations before this can initialize a real kingdom.
No claim of a playable skirmish, multiplayer match or campaign is made.

Backup
lib.rs.before is the economy export file before the one-line module addition.
No original executable, asset, save, profile or release was changed.
