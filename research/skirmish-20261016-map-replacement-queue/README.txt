Native map-replacement capture/queue -- 2026-10-16

Bounded implementation
----------------------
crates/aa-runtime/src/map_replacement_queue.rs captures4FD7F0 Object requests
and4FD8C0 Actor requests from actual ActorStore owners, preserving the native
creation payloads and distinct owned queue lists. It reuses the existing
ObjectCreationParams and complete ActorParams types. It does not execute
4FBA50 destruction, fresh creation, property attachment or world publication.
skirmish_start::require_replacement_drain continues to reject that boundary.

The original source GTID is retained separately from the requested creation
GTID. Position3 and direction2 are raw copied words, including NaNs/infinities;
there is no FPU calculation or RNG draw. Variant becomes0 and instance name
stays constructor-empty, rather than copying the source object's name/variant.
Actor owner is the actual ordinary4D1E80 owner. Actor mode is5; +3C carries
the source GTID; +30 carries it only when nonnull replacement ActorData+4E0
is identical to the current source definition. Other ActorParams constructor
fields remain proven defaults; inactive record halves/padding are not live
state and are not represented as duplicate mutable parameter blocks.

Properties503080 accepts kinds1/2/3. For those kinds a foreign nonzero source
emits the original replacement warning; only kind2 with source0/self transfers
the PropertyData pointer. Disabled/timing flags are not inspected. Duplicate
definitions and selected null definitions survive. A foreign null definition
is an error at the native logging dereference. A null Actor replacement is an
error after the property scan. Earlier diagnostics remain on later errors.

ActorStore adapter
------------------
Context::capture borrows current staged or spatial placement, ordinary owner,
the actual ordered Properties effects and canonical property_instances. Instance
lookup is lazy, so a missing later instance cannot erase earlier diagnostics.
Source and bank identities are checked; canonical definition IDs are never
translated twice. Required-object source binding reuses native_collision_definition
and its original source validation. Nonempty Upgrade inheritance, positive
Settlement wall-reference cases or shadowed ActorData remain explicit errors.
No mutable actor snapshot/default component is substituted. Nonempty saved
Properties reconstruction remains an earlier restore dependency; capture does
not fabricate those missing instances.

Queue ownership/order
---------------------
4FC500 appends a copy of the stack request before its destruction. Moving the
owned Rust request into Queue keeps that lifetime without retaining a reference
to temporary source storage. append_existing snapshots an aliased queue entry
before growth and owns a separate property vector. The object list and actor
list remain distinct: native4FC4A0 visits every object first, then every actor,
in insertion order within each list. native_order is inspection only; it is
not a successful implementation of the creation/drain callback. Vec grows on
demand; native initial8/minimum4/doubling allocations are not artificial caps.

Oracle
------
native_queue.py executes30 cases across PC24/PC53/PC64 and compares every
executed original instruction to accepted v14 bytes (1,025 instruction pins).
Native functions executed include4F4B90/4C8C20 constructors,4F5FD0 copy,
4F8C90/4F7C30/4F6050 assignments,4FC500 append,4F8CE0 queue growth,
503080 filtering,766DE0/4849F0 property-vector growth,4F3EA0 local destruction,
4012C0/405B80 CString refcount behavior, and4FC4A0 visit order. Original CRT
vector constructor/destructor loops execute. The4FBA50 seam returns false;
native4FC4A0 still visits every queued entry and keeps queue counts unchanged.

Explicit seams are successful allocator/delete, CString allocator identity/
empty-string/free virtuals, owner vf108, warning404F60, and4FBA50 visit-only.
Page0/import/unseamed allocator guards are inherited from mg_machine. Cases
cover empty, null object definition, null actor definition after warnings,
required-object match/mismatch, own/null/foreign property sources, unsupported
kinds, disabled properties, duplicate/null definitions, mixed-list growth beyond
initial capacity, and aliased appends both with and without reallocation.

Rust replay compares every meaningful captured field, ordered properties,
warnings, copied-queue data and stable visit order. It additionally checks
independent nonempty property-vector allocations for aliased appends.

Real-source test
----------------
map_replacement_queue tests also restore the original Firefight template and
capture a real placeholder_corpsunpacked using real gbr_corps source definitions,
its actual loaded owner, transform and bank. Capture leaves ActorStore and RNG
unchanged. The test then explicitly adds a source-derived Property fixture to
that actor (not claimed to be its natural saved effects) to verify filtering,
disabled handling, canonical identity, and a later missing-instance error that
retains an earlier diagnostic without appending a partial request.

Limits
------
No mission startup or playable map claim. The coordinator still requires full
replacement destruction/create/property attachment/publication. No internet,
multiplayer, UI, campaign, game EXE or release artifact changed. No native
allocation-failure/unwinding parity; malformed host identity guards may reject
earlier than native corrupt-pointer faults. Original game files remain untouched.
Only new runtime module/tests and its lib.rs registration were edited.

Backup: backups/map-replacement-queue-before-20261016-180347.zip (lib.rs).
Validation and final source/oracle pins are in findings.json, tests.log, check.log.
