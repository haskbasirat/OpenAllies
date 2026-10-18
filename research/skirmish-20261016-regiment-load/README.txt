Regiment Mind saved Load foundation -- 2026-10-16

Implemented
-----------
crates/aa-mind/src/regiment_load.rs decodes original528C20 versions0..21.
Saved::Regiment is additive to saved_load::Saved; Saved::base() gives the
shared52C580 fields without subtype destructuring. Existing updatable()
semantics are unchanged. Saved commands use the proven4DACE0 decoder.

The decoder preserves every serialized scalar bit, raw boolean byte,
discarded legacy byte, named ActorData/CString reference, desired-formation
index, path record, tracked word, engage target/priority and join-vector word.
The native20-entry state list521C30 is exact, including cai_chasing.
Engage targets reuse proven cai_engage::insert_sorted(..., from_tail=true),
which matches51FE60 and preserves order between equal signed priorities.

Existing apply/apply_with_commands supports versions0/1: version0 returns
without loading the Base; version1 loads52C580 and the words2B4/2B8. StateIDs
come from the actual constructed Mind. Current/queued commands require
distinct already-owned IDs, exactly as the Element saved-load adapter.

Versions2..21 intentionally fail application before any mutation. They need
real runtime inputs and owners listed below. They must never load with a
fabricated zero clock, raw32-bit pointers, guessed formation definitions,
or dropped collections. No runnable skirmish/multiplayer claim follows from
this reader foundation.

Native layout/dependencies
--------------------------
526260: concrete Regiment constructor (0x2C4 bytes), already proven separately.
A335A0+19C ->521C30: state list,20 entries in constructor order.
528C20: version reader max21. v0 returns. v1 base52C580 then8 bytes2B4.
v2+:
  +2BC linked words, native append order.
  +9C owned path52D2C0, constructor vtableA34738; stored order reverses on
       head insertion. Counts+ A8/AC, dirtybyteB0, finalwordB4.
  World B36534+E0 ->Mind+BC, exact copied word, not serialized.
  v3+ desired Formation index via4C2D90 ->+B8. This is a signed index into
       the owner's Organization+10 parameter-set LayoutData formations.
       Negative indices resolve null. A nonnegative out-of-range index is
       an original error; it is not a Performer/Ability reference.
  +148 ActorData CString via4C2E60/4BDBA0 category2B.
  v6+ +E8 bombard CString and +EC/F0 scalar pair.
  +1DC engage targets: read priority then GTID;51FE60 sorted insertion from
       tail; each node's GTID written after insertion.
  +264 join word vector: clear size, v7+ populate in source order. Rust keeps
       growable vectors, no fixed native capacity/allocation policy.
  +110 siege target list clears at v4+;118/119=1,11C=0.
  Numerous version-gated status/scalar fields, documented by their explicit
       offsets in regiment_load.rs and original load-instructions.txt.

API for completing v2+ application
---------------------------------
Regiment::needs_world_time indicates an actual world clock is mandatory.
desired_formation_index needs the current owner's Organization layout table,
not an authored enum ordinal or global FormationData fallback.
construct_definition needs typed ActorData lookup; empty serialized CString
means null, nonempty unknown name remains an error.
bombard_text needs a real CString/string owner for nonempty text.
tracked_words, path, engage_records/engage_targets(), join_words need concrete
Mind-owned containers transferred into runtime. Native pointer slots in the
constructed remainder cannot stand in for these owners.
Until these are wired, the explicit Unsupported error is deliberate.

Proof and tests
---------------
native_regiment.py runs original526260/528C20/521C30, path52D2C0, formation
reader4C2D90, engage insertion51FE60, native linked-list insertion and join
vector growth522E60. Native instructions match the preserved v14 executable.
Page-zero accesses are guarded. Explicit seams are byte I/O, CString storage,
allocation/free, named registry lookup and stream-mode(+14) false to select
saved-input mode. Worldtime and three concrete Formation pointers are supplied
to real native data structures; no read-layout/reference-index seam is used.

266 native cases: all22 versions, PC24/PC53/PC64, sparse/rich collections,
baseversion0/3, queued/current commands, all20 saved state names, positive/null
formation references, ActorData references, nonempty text, unsorted/equal
engage priorities, join-vector growth and two invalid outer versions.
No real saved Regiment records were required or claimed for this checkpoint.

regiment_saved_load_native.rs:2 tests passed. Decoder byte boundaries/scalar
stores/containers/states match original outputs; v0/v1 application compares
every constructed field/status; v2+ rejection leaves the Mind untouched.
Every proper prefix of a richv21 payload is rejected as a short read.
Existing saved_commands_native4/4 and saved_load_native1/1 also passed.
See tests.log and findings.json. Runtime/publication was not tested here.

Reproduce
---------
Set PYTHONPATH to reconstruction/tools/python-packages, run native_regiment.py.
Set CARGO_TARGET_DIR=target/s-ai-uai:
 cargo test -p aa-mind --test regiment_saved_load_native
   --test saved_commands_native --test saved_load_native
   --release --locked --offline -j4
