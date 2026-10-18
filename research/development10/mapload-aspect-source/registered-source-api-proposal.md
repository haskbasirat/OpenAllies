# Registered Object source API proposal (research only)

Current production APIs are unchanged. The first source ticket is being probed with the existing APIs. This proposal is not a completion or parity receipt.

The native loader returns the newly registered allocation from 4BCA00 to 4BDC50, which calls that allocation's vf0 LoadData. Later 427BD0 is a different operation: it consults the current translation map first. Current source helpers frequently take an IDS and call that final Find, so they cannot select an original registered declaration shadowed by a later translation.

The historical native-load-report has one actual canonical shadow: `chaparral_wall` from `Doodads/Chaparral/chaparral_wall.tgi`, with final translation to `temperate_walls`. The `ActorCatalog.actors` vector itself preserves `thing_owner` first-registration order (native_load.rs771..805), but resolving each vector ID through final Find loses the original allocation. The product's exact ID/source guard must remain until a real registered-source API is available.

Proposed front door: `ActorCatalog::registered_thing(ordinal)` returns a sealed borrowed `RegisteredThing` tied to the SAME catalog/registered slot, exact provenance and parsed root. It does not create a second ActorCatalog, alter lookup tables or suppress a translation. Ordinary `find`, `resolved_actor`, category and definition APIs retain translated Find semantics.

The source capability should call shared existing field/path-merge, category/base flags, ObjectDefinition and CreationDefinition kernels. The Find APIs first select their existing translated root, then call those same bodies. The registered API selects the actual registered root and calls the same bodies. No independent flag/geometry algorithm, copied Bank, dynamic source override or permissive caller-provided source index is acceptable.

Initial useful domain is non-Actor ObjectData kind1 and existing Object-derived kinds20/21; Actor2 keeps the current canonical binding path. Unsupported kinds remain explicit. The real registered `chaparral_wall` source is the concrete test, not an invented fixture whose old ID is absent.

Runtime source construction needs a sealed source-only registered Object capability (same source Bank universe/PC and exact canonical definition proof). Existing source loader should delegate one common retained-state algorithm. Source validation and Object FX source binding must use the registered capability only on this front door; legacy runtime-instance `live_object_init::Source` Find validation is unchanged. Graph source compilation must take the exact root from its validated ObjectFxSource rather than re-resolving its IDS through final Find.

Proposed owned files after parent unfreeze/approval:
- aa-rules actors.rs and resolved.rs: sealed slot/root selection and shared field-path/root access.
- aa-rules native.rs: extract unchanged category/base-flag source bodies.
- aa-rules objects.rs: extract unchanged Object/Creation source bodies.
- aa-runtime new registered_object_source.rs (or equivalent sealed type), plus data_pre_request.rs, data_object_request.rs, data_aspect.rs narrow common-source delegation.
- aa-runtime definition_fx.rs, fx_graph.rs and fx_graph_texture.rs: validated source/root front door and unchanged common compiler.
- focused tests for actual registered shadow, ordinary Find still returning the translated target, exact source/PC/foreign-owner rejection and preserved partial Resource/Graph ownership.

Native proof before code: directly observe 4BCA00 return and the ECX passed to vf0 by actual 4BDC50, then contrast final 427BD0 after translation. The existing phase8 registry harness separately executes registration and Find but does not yet prove the 4BDC50 vf0 call. Globals is preparing the minimal original/v14 pointer trace under registered-source/. All allocator, typed getter and vf0 loader seams must be named; the proof must not claim complete ObjectData LoadData.

Checks after any implementation: scoped aa-rules tests and mandatory aa-metagame actual-data export check if definition/loading APIs change, then the scoped actual-source ownership tests. No full client/workspace link by this agent.