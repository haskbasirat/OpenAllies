ACTORDATA +2D8 MEMBER SUPPORT PROPERTIES -- 2026-10-16

Production scope
  aa_rules::native_member_property_data reads the ActorData support-property
  list used by45EC60. This is distinct from initial PropertiesComponent+1D4
  and from StructureComponent support/technology lists.

  Constructor4624B0 stores zero at2D8/2DC (462830/462836). The ElementComponent
  block of464BD0 reads SupportProperty children at466A79 and calls4C4CF0 at
  466AA2 with destination ActorData+2D8. Each child property_IDS is resolved
  immediately through Property Find463FA0; it is not deferred via473AD0.
  Native4C4CF0 captures the signed child count, reads each selected child only
  when reached, throws Unknown property '%s' for a null lookup, then prepends
  the resolved definition and initializes the tail only if it was null.
  Thus duplicates are retained and completed values are reverse authored order.
  45EC60 returns a cursor containing the list sentinel address and current head.

  ActorCatalog::native_member_property_data(layout, canonical_owner) returns
  MemberPropertyData { id, element_present, properties, sources }. Properties
  are shared native_layout_data::PropertyRef identities, never independent
  string-derived IDs or process pointers. The caller must supply the same
  catalog's NativeLayoutData registry used by its other property consumers.

  Canonical owners are selected from native registrations, then matched to
  their actual root provenance. No Thing Find/Translate is performed again.
  Both standard ActorData and derived RandomActorData are supported; other
  classes and catalogs without native registration order are refused.
  Property lookups use RootPosition of the owner's declaration, preserving
  immediate visibility and translations that existed at that point. Plain
  source strings/identity operations are precision-independent. No restore
  precision is silently substituted for source loading.

  Public load_list(Source, &mut Vec<PropertyRef>) is the partial-state core:
  earlier prepends survive a later source/lookup failure. Its allocation domain
  is successful growable allocation; native allocator reentry/null faults are
  not claimed. The per-owner completed API returns an error for incomplete
  loading instead of presenting a partial list as completed data.

Evidence
  native_member_property_data.py separately executes unchanged original/v14
  4C4CF0 and45EC60 at PC24/53/64:72 rows,100 byte/address pins (92 executed
  instruction addresses and8 intercepted leaf-entry addresses). The shared
  movement harness guards page zero except FS access and rejects missing heap
  seams. Native list nodes, head/tail stores and cursor writes execute. Stops
  on missing source fields/unknown properties retain prior list stores.

  Explicit seams: typed child count/selection/string getter, CString ownership
  helpers, Property Find, and successful allocation. Their ordered reads/calls
  are recorded. This proof does not newly execute parser/registry internals or
  464BD0's unrelated fields; existing layout registry/native source-loader
  evidence remains separate. native-source-instructions.log records original/
  v14 identical windows for the call site/reader/list and constructor stores.

  Focused Rust tests replay every native row; synthetic native-order source
  files check reverse non-palindromic order, duplicate shared identity, canonical
  owner shadowing, late translation invisibility, later-property refusal and
  non-Actor/unordered-source errors. The actual source census traverses1084
  canonical ActorData definitions at each precision and retains provenance;
  every installed original/v14+2D8 source list is empty. actual-source-data.json
  pins that source census and all input source hashes. These real source rows
  are Rust source projections, not new native parser executions or initialized
  gameplay observations.

Reproduce (PowerShell from rust-rebuild)
  $env:CARGO_TARGET_DIR='target/s-state-binding-review'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  $env:AA_METAGAME_EXPORT='0'
  Remove-Item Env:AA_MEMBER_PROPERTY_DATA_CAPTURE -ErrorAction SilentlyContinue
  cargo test -p aa-rules --test native_member_property_data --release --locked --offline -j4 -- --include-ignored
  python research/skirmish-20261016-member-property-data/native_member_property_data.py

  Native replay is immutable by default; --write deliberately regenerates.
  Actual census regeneration uses AA_MEMBER_PROPERTY_DATA_CAPTURE set to an
  absolute JSON path; never set that variable during ordinary replay.

Limits
  This is only the2D8 source subsequence. It does not certify whole ActorData
  loading, global deferred completion, runtime property instances/admission/
  insertion/effects, original save completion or playable battle initialization.
  No original file, asset, save, profile, release or manifest was changed.
