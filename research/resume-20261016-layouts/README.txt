RESUMED ORGANIZATION RESTORATION INTEGRATION — 2026-10-16

Scope
  Connect the existing phase8 LayoutData producer to the native saved-map
  restoration path, then connect the next morale-source/Properties callbacks.
  This does not publish a playable world or substitute invented defaults.

Files
  crates/aa-runtime/src/catalog.rs
    Per-catalog/per-precision NativeLayoutData cache, original type-manager
    lookup, canonical LayoutInfo, default Organization pointers/parameters,
    Formation facts, ActorData property and morale source bindings, and
    the source ModifierAttributeClampAdd setting.
  crates/aa-mapworld/src/restore.rs
    RestoreDefinitions contracts for those definition/setting facts.
  crates/aa-mapworld/src/organization_restore.rs
    Forwarding to those facts. 4CF110 Properties context lookup using the
    loading owner's shell or a member's full generation-checked GTID.
    501C90 morale modification using the existing proved primitive.
  crates/aa-runtime/tests/organization_layout_bindings.rs
    Layout fields/identities against saved original-code fixtures; partial
    registrations remain errors; one-hop translation regression; source
    settings, all 192 shipped Organization definitions and their properties.

Original code and existing proof reused
  LayoutData 4C3610, LayoutFormation 4C32B0, FormationData 4C34A0,
  validation 4C1BE0 and ActorData Organization binding 466C4F..46707C,
  element links 460000, property membership 45EEF0:
    research/phase8-layouts/native-layout-data.json
    research/phase8-layouts/native-organization-component.json
    crates/aa-rules/tests/native_layout_data.rs
  Native DataManager Find 48D500 applies one translation and returns the
  resulting definition pointer. A canonical layout identity must not be
  sent through translation a second time just to read its fields:
    research/phase8-definitions/native-definition-registry.json
    research/phase8-definitions/native_definition_registry.py section_lookup
  ActorData+2C0 / 45D580(NULL): producer and original-code tests provided
  separately by the organization_morale_source task in native_actor_morale.rs:
    research/resume-20261016-morale/native-morale.json
  4CF110 returns null if Actor+78 Properties is absent, otherwise its +1C
  modifier context. The Rust full Actor ID represents that context and is
  validated on every use, including the Properties owner identity.
  501C90 adds ModifierState+20, performs the explicit binary32 store, then
  floors at min(base, ModifierAttributeClampAdd) and zero:
    research/phase7-properties/00501c90.asm.txt
    aa_body::properties::ModifierState::adjust_with_floor
  The existing binary32-store primitive is reused and its result becomes
  Binary without another rounding operation. 578300 still owns native
  member-order accumulation and its stores in organization_state.rs.

Identity and error boundaries
  Layout lookup may return a registered partial object, as the native
  manager registers before running LoadData. LayoutInfo refuses to treat
  a partially loaded object as usable. Missing/malformed references do
  not receive synthesized definitions. The same cached arena provides
  layout formation and embedded/registered Property handles.
  The older ActorData source APIs still accept lookup IDs rather than
  registered-object identities. A canonical Actor ID shadowed by a Thing
  translation is explicitly refused before property/morale/default-layout
  use, rather than silently reading the translated actor instead.
  The existing Organization warning/exception catch remains in place;
  missing saved ActorData parameters are dropped through the native path,
  not replaced with invented modern units.
  Properties absent is the native null context. Stale IDs, wrong component
  payloads, wrong owners and missing settings remain explicit errors.

Checkpoints
  BEFORE source census: qa-output/resume-20261016/before-inventory.json
    80 maps; 18 without record failures, all 18 queue/PostLoad passes;
    1,069 failures (1,067 LayoutInfo and 2 encoded localization strings).
  Preliminary layout-only census:
    qa-output/resume-20261016/after-layout-inventory.json
    80 maps; 19 without record failures; 924 failures, all at the then
    missing ActorData morale contribution. The 143 missing saved ActorData
    references took the native warning/drop path. Two localization errors
    also disappeared during concurrent work; that change is not credited
    to this layout integration. This is an intermediate build, preceding
    canonical identity review fixes and the morale callbacks.
  FINAL combined census: qa-output/resume-20261016/after-inventory.json
    80 maps; 20 without record failures, all 20 queue/PostLoad passes;
    796 failed records: 647 saved Mind current command objects
    (4D9C80 virtual+8 Load), 149 Element set-organization (5705A0).
    Tutorial01 reaches saved command objects; Tutorial02 reaches 5705A0.
    The native caught-warning count is now 399: the additional 128 over
    the layout-only run are 109 layouts no longer adhering to defaults
    and 19 saved 6-slot layouts whose current layout has 7 slots.
    This preserves the existing native warning/drop path; it is not
    counted as successfully restoring those dropped old actors.
    Original class-conversion records remain 7,879 and unknown Thing
    skips remain 3,912. No conversion/publication bypass was added.
    Test completed in 131.71 seconds. Every pinned source hash matched
    before and after the run.
  Tests: 2 mapworld Properties-context tests, 5 new runtime adapter/source
    tests (including all192 source organizations at PC24/53/64), 1 existing
    tutorial catalog/staging test, and the final all80 census passed.
    The translation regression initially had a wrongly ordered template
    file; fixing that test input yielded the final 6/6 runtime pass.
  Independent review accepted the final source and identity fixes:
    independent-review.json (includes source hashes and evidence scope).
  Final source hashes, checks, and census are recorded in findings.json.
  InspectPartial is diagnostic only: later failures may be cascades.
  Clean record restoration/PostLoad does not mean InitForLoadedMap,
  conversions, publication, battle simulation, AI or multiplayer is ready.

Preservation
  Source backup before this task:
    backups/source-before-resume-20261016-152607.zip
  Original game, saves, profiles, releases, parser, and LayoutData producer
  were not modified by this task. No new engine cap or concurrency change.
