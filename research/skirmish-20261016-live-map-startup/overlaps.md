# Held Actor overlap ownership and ordinary query composition

The native Actor +110/+114 list now has one typed owner, `ActorShell.overlap_list_110`, moved with the actual shell through staging. Raw +110/+114 constructor mirrors were removed. Source exclusion reads this same list; supplied obsolete raw pointers are refused. Node IDs are allocation identities, duplicates remain separate nodes, and storage grows without an inline twenty-entry cap. This is not a claim of unlimited physical allocation or allocation-failure parity.

`load_actor_overlaps::refresh` is the concrete held-record 4D67F0 bridge. It validates the actual source/Loading owner, reads the authoritative `components.state` for 4D10A0, and takes bit29/ineligible/bit10-clear returns before reading the actual clock. Expiration mutates the actual list before querying. The spatial policy then borrows that post-expiration list immutably. Only a completed query's discoveries are prepended with native expiry +0. No detached overlap Vec or copied pose supplies a second mutable owner.

The load index leases current/prior Object fields synchronously for proven read-only query operations. Every record generation/variant/source destination is checked first. Query flag changes to all candidates are collected and committed to the actual current or registry shell on both success and error, then transient fields are retired. GTID keys, native membership nodes, tile flags and query marks remain in the same persistent index. Logical identity can be bound before Object membership; this does not insert native tile nodes. The callable contract permits only immutable actual-world policy access and native query operations that change candidate flags, not arbitrary pose/cache/reference mutations through the index.

The new original/v14 fixtures execute independently at all three x87 PCs:

- `native-actor-overlap-list.json`: 342 paired cases /684 native executions /179 instruction pins. Actual 4D10A0, expiration/removal79E3C0, successful native prepend and temporary-list cleanup run. Required554350/555620 queries are explicitly supplied reference-list seams; this is not a new spatial or collision proof. Stop-before-query cases prove expiration survives the required boundary.
- `native-actor-overlap-denormals.json`: 48 additional paired cases /96 executions /176 pins, including subnormals, signed zero, infinities and NaNs. Expiry uses integer binary32 ordering so host SIMD DAZ/FTZ cannot change the result.
- The typed-list Rust replay covers all390 paired rows. A separate list lifetime test exceeds1024 nodes, preserves duplicate references, retires removed identities and does not reuse a stale node ID.

Constructor and shell-mapping tests check actual typed null ownership, omission of obsolete raw authorities and movement of the same node storage. The old frozen constructor fixture is unchanged: only its test projection reconstructs the already-proved native null pointers.

`load_spatial_query_tests` uses actual source-completed tutorial02 records and terrain. It proves logical current identity before membership leaves tile lists/flags untouched, then exercises real flag writes to both prior/current records, an injected error retaining both writes, and retirement of transient field copies. It is an ownership/partial-effect test, not a whole initialized map or battle claim.

`load_actor_overlaps_tests` additionally uses a real saved Kingdom allocation with explicitly synthetic active flags to exercise a real self-only ordinary query, expiration, lazy clock gates and retained expiration before a deliberately mismatched map-PC error. It must not be described as an actual military-unit or full Actor initialization test. Its test result is recorded separately after final compilation.

The shared placement policy uses actual source precision/provenance and same held/registry owners. Reached positive relations, Wall/parent collision and other unbound services remain explicit errors. Generic future low-path Vec-host traits have no concrete ActorStore host here; their existence is not an activated live-navigation claim.

# Area, Camera and Doodad base admission

Source/base Object and load-index admission is limited to the proven runtime classes2/3/4/6/7/9. Classes6/7/9 have Object storage and no Actor storage; their actual +B4 is false. Their immutable radius/blocking/soft getter targets match the previously proved base Object getters. Exact derived constructors and virtuals are evidenced separately in `../skirmish-20261016-special-object-init/native-special-classes.json`; derived Init behavior remains that module's responsibility.

Memory eligibility is class-specific. Only Actor/Kingdom Data+38 reaches the component mask; Area/Camera/Doodad do not. DoodadData+40 returns full EAX1, so potential visibility returns false before flags or announce reads. Treating Doodad as plain Object would be wrong. `native-special-object-memory.json` adds1296 paired cases /2592 independent original-v14 executions /86 pins, verifying actual memory results and ordered reads across flags/masks/announce/phase/all PCs. The Rust replay checks both result and read order. Geometry still comes from the existing canonical native source loader; no missing geometry is replaced with a default.

No original executable, assets, release archive, root manifest or lockfile is changed by this work. Final author test/source pins are in the separately sealed overlap findings; parent integration and independent acceptance are separate evidence.
