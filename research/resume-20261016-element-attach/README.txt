ELEMENT ATTACHMENT PREFIX IN RESTORED OWNED COMPONENTS — 2026-10-16

Scope
  Connect the existing native5705A0 attach primitive to real restored
  Element and Actor fields. Execute native4D1350/571640 action rebuilding
  through the first unsupported contributor. Do not skip any callback to
  make a map look complete. This does NOT complete the149 blocked records.

Implementation
  crates/aa-runtime/src/saved_organization_actions.rs
    5705A0 preserves slot+18,organization+10,attached+1C write order, then
    dispatches the existing4D1350 coordinator on the member's real fields.
    Full GTIDs access the real registry; the currently loading parent and
    its Organization remain borrowed from the active57D130 call.
    No cloned/fake Mind or actor arena, no inferred default world values.
    Ground action reads and4CFCC0 publication use existing native primitives;
    specialized contributors, live Economy and SAI services still fail
    explicitly at the first reached dependency.
  aa-actors::organization_state::OrganizationWorld::element_set_organization
    now receives the active Organization reference. This preserves parent
    visibility while that component is temporarily outside its owner's slot.
    The existing saved-Organization oracle adapter accepts that reference.
  aa-mapworld::restore::OrganizationAttachContext and the Catalog's narrow
    organization_attach_member forwarder connect that live borrowed state.

Native evidence
  Existing complete outer5705A0 call order/field proof:
    research/phase8-combat/native-member-extras-fixtures.json
  Existing4D1350/571640 ground coordinator proof:
    research/phase7-actor-live/native-ability-flags.json
  New native_attach_prefix.py / native-attach-prefix.json:
    Eight cases execute original5705A0→4D1350→571640, then STOP at entry to
    Recruit582C40. Four varied input cases run separately against original
    and v14 PE images;92 executed instruction sites match both images.
    The actual tutorial02 input is included. No behavior seams run in the
    measured prefix; page zero is unmapped. The stop is instrumentation,
    not a claim that native Recruit throws or its behavior was executed.
  observer/ contains separate read-only lifecycle investigation and42 native
    7A0C20 inactive-branch cases. It supports an explicit inactive observer
    during the normal managed load sequence. This prefix does not pretend
    an observer exists or is absent: the unbound runtime service stays an
    explicit error if reached. Active SAI is not implemented here.

Real tutorial02 regression
  crates/aa-runtime/tests/element_attach_restore.rs loads the original map
  and verifies the new exact boundary. It also replays all eight native
  prefix cases through the production adapter on real owned components.
  Owner4099 regiment_usa_hq_infantry_unpacked; nested member4100
  usa_hq_infantry, slot4. Before: no Element membership, attachedfalse,
  actionF0=1048576,F4=0. After: organization4099,slot4,attachedtrue,
  F0/F4=0, matching the original executed prefix.
  The next failure is the actual missing Recruit582C40 action contributor,
  before Research,Structure,Denizen,Construction,MobileDeployment,Mind,
  Economy refresh and observer callbacks. These were not skipped.
  The member's Element definition pointer is genuinely null in this map;
  571640 therefore performs its native no-definition branch.

Validation and preservation
  Commands use target/s-restore,release,--locked,--offline,-j4.
  Logs/output: qa-output/resume-20261016/element-attach-{before,after,final}*
    and organization-callback-signature-test.log.
  Exact final test results/source hashes are recorded in findings.json.
  Final checks: one tutorial02 regression (including eight native-prefix
    replays) passed, plus both existing Organization constructor/Load and
    out-of-range-slot oracle tests. Production source compiled successfully.
  No further all80-map census was run by this task; the lead owns the final
    combined census and Development08 packaging.
  Snapshot: backups/source-before-element-attach-20261016.zip.
  Original game, assets, saves, profiles and previous releases untouched.
  Earlier accepted layout/morale evidence remains in its own dated folder.

Remaining limitations
  This is a bounded native prefix with preserved partial side effects.
  General saved action contributors, economy/SAI services and full map
  restoration, publication, battle simulation and multiplayer remain open.
  All eight new prefix rows use a null Element definition and stop at the
  first Recruit call. Active-parent traversal and later action publication,
  Economy/observer branches are not covered by those execution fixtures;
  they remain outside this task's new parity claim.
