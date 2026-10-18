Native `4EB6E0` blocking notification is now wired into the real Object Init
services in `load_object_init_world.rs` through `load_object_blocking.rs`.

The actual current bit10 gate precedes queries. The existing native query
constructor receives the real source; the caller then clears its source actor
(`descriptor+1C`) and sets ignore actor (`+20`) to that current logical key.
The same load index and concrete read-only placement policy collect actual
overlaps. Every returned generation resolves in discovery order. The actual
proved virtual+B4 is false for Object/Area/Camera/Doodad and true for
Actor/Kingdom. True results call the concrete `4D67F0` overlap-list refresh.
No empty discovery list, inactive state, friendly relation, or missing owner is
invented to complete the call.

An ordinary child is temporarily held outside its distinct matching Loading
entry and restored before every Result return. The original source remains in
its own actual allocation. `HeldShell`/`HeldView` and the new extra-held query
lease make both real shells available while both registry entries are Loading.
Both source identities/classes/storage layouts and generations are checked
before leasing. The closed query sees immutable shell/registry borrows and
can use only proved query/mark operations. It has no outward mutable registry
callback. All native candidate flag writes go back into both held owners and
every prior registry owner on success and error, then transient member fields
are retired. Logical key preparation does not insert native membership or
create a second resident pose/list owner.

The shared policy reads an optional extra held shell before consulting the
registry, retaining the same source/proof/class/layout guards. Existing
callers supply no extra shell. Structure34 and source-exclusion generation
resolution already count the real Loading registry entry as an existing GTID.
Derived Kingdom/Team payloads are still required explicitly when either held
owner's unavailable derived fields are reached; the new base-shell view does
not manufacture them.

The child reads the same Catalog.load_clock NativeClock reference lazily;
bit29/ineligible-state/bit10-clear children return before reading it. Positive
unbound collision policies and a missing reached clock remain explicit errors.
Native list expiration and all prior query/child effects survive later errors.

Evidence:

- `native-object-blocking.json`:54 paired rows/108 separate original-v14
  executions across all x87 precision controls. It executes actual4EB6E0,
  GTID generation lookup, actual vtable+B4 leaves, child4D67F0/4D10A0 and expiry
  removal.196 executed address pins; three separate seam-entry pins for query
  construction/discovery and allocator-free behavior. Query results are
  explicitly supplied fixture inputs. This control oracle does not prove
  collision geometry, a complete game-world binding or arbitrary child effects.
- `native_object_blocking.py` fresh read-only replay matches the frozen JSON.
  The earlier v1 artifact is retained separately: it counted the free entry as
  executed. Current evidence separates that seam entry; outputs are unchanged.
- The new two-held test runs on two actual tutorial Kingdom allocations. It
  confirms both actual flags commit after an injected post-write error, both
  allocations keep matching Loading entries during the query, duplicate logical
  owner admission is refused, and no transient member is available afterward.
- Immutable current-tree test executable:5/5 executed tests pass: extra-held
  query1, existing actual overlap1, and membership/lease/adoption regressions3.
  The overlap test explicitly controls active flags and uses an actual self-only
  index; it does not claim positive multi-actor collision coverage.
- A separate placement-author read-only source review found no blocker for this
  closed native-query ownership/order scope. It is not an independent native or
  Rust execution, geometry proof, full map acceptance, or battle-readiness claim.

Actual managed map strict/census tests belong to root integration QA. This
module alone does not certify that all27 former census failures succeed, that
any particular military actor finishes Init, or that a battle is playable.
