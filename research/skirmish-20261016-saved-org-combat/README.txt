SAVED ORGANIZATION COMBAT VALUE -- 2026-10-16

Scope
  The saved-restore host now supplies native Organization 5788B0 through a narrow
  required callback. aa-runtime::saved_organization_combat borrows the actual
  in-progress OrganizationState<String> and the actual saved Actor shells.
  It calls the separately proven aa-combat::organization_combat_value routine;
  it does not project or clone an Organization, copy back only on success, or
  invent a completed Performer/source combat value.

  RestoreDefinitions::organization_combat_value(OrganizationCombatContext)
  is the only new restore host requirement. The default remains an explicit
  unavailable error. Catalog binds it; organization_restore forwards the actual
  owner, owner Actor, Organization, registry and explicit arithmetic precision.
  Backups of shared files are in backups/before-saved-org-combat-20261016-183833.
  Catalog edits and its backup are owned by the root coordinator.

Bindings and store order
  Initial native count+2C is read before clearing+68/+6C. Each subsequent count
  read occurs at the native boundary. The actual member vector retains null
  entries and duplicates. Nonzero IDs resolve through generation-checked GTIDs;
  the active owner uses owner_actor because it is outside the registry during
  its own Load. Organization.header.owner must match that actual context.

  A member reads its actual Performer slot (+74). Absence contributes nothing,
  without falling through to vacant-parameter handling. A present typed
  OwnedPerformer contributes its actual word60 bits. Its state.owner must match
  the member, and the Body read follows that owner (+4 -> Actor+60); the typed
  Body header is checked too. Body health+10 and maximum+14 are actual stored
  values, not definition values. Absent Body has the native exact ratio 1.
  Current+68 is stored before the second member/Performer/value read for max+6C.

  A vacancy resolves CURRENT canonical layout.definition, its native slot-group
  index and CURRENT layout.parameters[group]. Null parameter reads no source.
  Nonnull parameter reads canonical completed component mask+174 through the
  narrow Catalog::canonical_component_mask getter. Only bit5 reaches+1E4.
  Completed canonical ActorData+1E4 is not bound: that exact dependency returns
  an explicit error, preserving all earlier stores. Constructor zero and -1 are
  never used to substitute for missing completed source values.

  The exclusive Organization borrow and readonly registry prevent any callback
  in this adapter from replacing member/parameter/component containers. Thus
  stable IDs, indices and Members=() preserve the captured native pointer
  identities within this no-reentry scope. The borrowed arithmetic API itself
  also proves the more general native recapture and second-read order.

Native/actual-source evidence
  actual-saved-inputs.json contains 33 inputs across PC24/PC53/PC64. The generator
  executes every input against both original and v14 images, producing 66 rows:
    36 rows: six unmodified saved Organization records, all three precisions,
             both images, from actual (2v2)Kursk.RMP InspectPartial restoration.
    30 rows: five explicitly substituted adapter-storage probes, all three
             precisions and both images. These insert a test OwnedPerformer
             with word60=10 into an actual saved Actor slot, use the same actual
             Organization allocation, and supply explicit Body 3/4 or no Body.
             They cover nonzero weighted values, absent Body, late missing+1E4,
             null vacancy and bit5-clear vacancy. Original slots are restored.

  The unmodified captured restore prefix has NO concrete Performer slots.
  Accordingly the nonzero probes are test inputs, not observed initialized
  saved Performers and not a native Init/property-refresh completion receipt.
  In the late-source probe, native and Rust retain current 7.5 and maximum 10.

  Native5788B0/57DC40 arithmetic executes unmodified. For an unavailable+1E4,
  the harness interrupts at 57891A, BEFORE its FLD. This is an explicit required
  source-service interruption proving a retained-store prefix, not an original
  C++ exception or pointer-fault claim. No source value is supplied there.
  The test also checks 12 portable identity refusals (four per precision): later
  stale-generation member after a nonzero contribution, wrong Performer owner,
  wrong Body owner, and wrong active Organization owner. They are structural
  diagnostics, not native malformed-pointer error/exception equivalence.
  Organization allocation and substituted Performer slot identity are checked.

  independent-saved-replay.json re-executes all 66 frozen native rows without
  rewriting the fixtures. The borrowed arithmetic has its own independent 306
  case proof in ../skirmish-20261016-borrowed-combat-value/.

Reproduce
  From rust-rebuild, PowerShell:
    $env:CARGO_TARGET_DIR='target/s-saved-org-combat'
    $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
    cargo test -p aa-runtime --lib saved_organization_combat::tests --release --locked --offline -j4 -- --include-ignored --nocapture

  Rebuilding observed input evidence is deliberate and separate from replay:
    $env:AA_ORG_COMBAT_CAPTURE='1'
    [same cargo test command]
    Remove-Item Env:AA_ORG_COMBAT_CAPTURE
    $env:PYTHONPATH='E:\Games\Axis and Alies\reconstruction\tools\python-packages'
    python research/skirmish-20261016-saved-org-combat/native_saved_org_combat.py

Limits
  No true per-record Actor Init/PostCreate/publication is proven by this binding.
  The test uses an explicitly fresh stopped managed SAI/application context,
  mode 1 and clock 0 diagnostic scope; that is independent of the missing native
  immediate record lifecycle. Constructor/restored word60 may be observed as-is,
  but its actual native refresh producer is not recreated by this adapter.
  Completing ActorData+1E4 needs its own source-phase means and precision proof.
  Native corrupt-pointer/exception behavior, arbitrary reentry, nonfinite values
  and zero Body divisor are outside this supported numeric/storage domain.
  Advancement of InspectPartial map diagnostics is not a battle or complete
  native map-load claim. No original files, saves, profiles or releases changed.
