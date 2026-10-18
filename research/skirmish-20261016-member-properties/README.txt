SAVED ORGANIZATION MEMBER PROPERTY PROPAGATION -- 2026-10-16

Production
  saved_member_properties::member_added(Catalog, OrganizationAttachContext,
  member) binds native5784C0 while Load holds the actual Organization outside
  its owner's slot. The same mutable owner shell, Organization and identity
  registry are borrowed. No cloned components or success-only copyback exist.

  Native5784C0 captures owner Properties once, then reads its signed count and
  live +10[index] slot. The slot read precedes even the member-null branch.
  Instance fields are not demanded until a nonnull member has Properties.
  Reached order is instance+C, PropertyData+42 pass_down_members, member+A4
  Experience, conditional instance+8 kind7 exclusion, then instance+10 disabled.
  Before applying4CEF10 to the MEMBER, native rereads org+4 source owner and
  instance+8 kind. The loop tail rereads the captured Properties live count.
  Member+78 is Properties; Experience is member+A4. It is not the reverse.

  Actual saved Properties effects is the authoritative vector. The saved world
  currently has no completed property-instance arena; nonempty saved effect
  deserialization already stops explicitly before constructing those records.
  Therefore this integration can complete the actual empty-list branch, and
  can skip unresolved handles for null members or members without Properties.
  A reached instance+C returns the required saved-instance-arena error. It
  never becomes an empty list, default PropertyData or successful application.

  The complete guard/read coordinator is ported, including Experience/kind7,
  live count/slot rereads and fresh source owner. Later production services
  (PropertyData+42, actual instance fields,4CEF10 admission/insertion/effects)
  retain explicit errors. This does not claim real nonempty property grants.
  Captured component allocation identity is checked without unsafe pointers;
  replacement/detachment returns an ownership error. Arbitrary reentry that
  destroys that component is outside the valid captured-owner proof.

  saved_member_properties::find(context, property, source) also binds the
  native4CEF80/581280/5032D0 presence query. It captures the signed count once,
  compares each instance definition before resolving an optional source GTID,
  includes disabled instances, and returns the first matching list handle.
  A null source omits that filter entirely. Actual empty/absent state returns
  false; an actual nonempty unknown instance stays a required arena error.
  No property definition/source is demanded merely to query an empty list.
  The direct find_actual(LookupContext{owner,owner_actor,registry},property,
  source) API borrows only the actual read-only inputs native4CEF80 uses. The
  Organization-context wrapper forwards that same coordinator; tests compare
  both API results on every actual query. No held Organization is fabricated
  for a caller whose native property-presence callback does not possess one.

Proof
  native_member_properties.py separately executes original and v14 at
  Single24,Double53,Extended64. Page zero faults except legitimate FS access;
  executed original/v14 instructions are compared and pinned.

  294 native rows contain:
    144 coordinator rows (24 explicit inputs x3 PCs x2 images), with complete
      ordered field-read/callback trace replay in Rust. Cases cover owner
      Properties absence/empty, null/absent/present/Experience member, pass-down
      false, disabled, kind7 suppression, kinds8/10/full-width kind, required
      apply error, and explicit apply callbacks that truncate/append/replace a
      following list slot or change org+4 source owner. These callbacks are
      TEST SEAMS at4CEF10; real admission/insertion/effects are not executed or
      claimed. The mutations prove the coordinator does not snapshot the list,
      retain a stale count, or retain a stale source owner after callbacks.
    108 lookup rows (18 inputs x3 PCs x2 images), executing the actual4CEF80,
      581280 and5032D0 bodies without a callback seam. Ordered read traces prove
      absent/empty, definition mismatch/null, optional source filter, matching
      generation, stale generation, duplicate and disabled-instance behavior.
      Canonical definition aliases share native pointer identity in this oracle.
    42 actual saved-context rows (7 inputs x3 PCs x2 images): six unmodified
      tutorial02 rows plus36 labelled caller-input substitutions. Each also
      invokes both an unfiltered null-property query and a filtered unknown-
      property query over the same actual state, proving skipped source reads
      and the identical required-instance boundary on nonempty state.
  118 original/v14 instruction addresses are pinned. Native4CEF10 is recorded
  as a callback entry only in coordinator cases; its body is outside this proof.

Actual prefix
  The Rust ignored test restores the actual Campaign/tutorial02 RMP through
  managed StopOnError mode1 with actual settings/resources/localization and a
  stated375ms constructor-clock fixture. It finds owner4099 and member4100
  naturally attached in slot4. The current owner Properties.effects is empty.
  Under all3 PCs the public adapter returns without any source/instance read.
  Owner shell, held Organization and Properties allocation identity remain
  unchanged. Immediate Init/PostCreate and completed publication are omitted
  by this bounded record diagnostic and are not claimed as a playable battle.

  Labelled substitutions remove owner Properties, use a stale member, append
  an unresolved instance handle, use null member, or temporarily remove the
  actual member Properties. No PropertyData/Instance is fabricated. Native
  unknown-instance rows stop before5784F1's unavailable instance+C read; stale
  member rows stop before5784EA's unavailable member+78 read. Those are required
  source boundaries, not claims of equivalent native malformed-pointer faults.
  Lookup unknown-instance rows stop before5032F6's instance+C read. Its registry
  base pointer is captured by5032E3, but no invented registry entry is read.
  Original caller-owned fields/components are restored after each probe group.

Reproduce from rust-rebuild, PowerShell
  $env:CARGO_TARGET_DIR='target/s-owner-recruitment'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-runtime --lib saved_member_properties::tests --release --locked --offline -j4 -- --include-ignored --nocapture
  python research/skirmish-20261016-member-properties/native_member_properties.py

  Deliberate recapture only:
    $env:AA_MEMBER_PROPERTIES_CAPTURE='1'
    cargo test -p aa-runtime --lib actual_tutorial02_owner_properties_and_lazy_missing_instance_boundary_match_native --release --locked --offline -j4 -- --include-ignored --nocapture
    Remove-Item Env:AA_MEMBER_PROPERTIES_CAPTURE
    python research/skirmish-20261016-member-properties/native_member_properties.py --write
  Default native invocation only replays the frozen fixture; it never rewrites.

Limits
  Actual nonempty instance storage/restoration, canonical PropertyData loading,
  real4CEF10 admission/5030F0/581630 insertion/effects, native malformed-pointer
  exception parity and arbitrary component-destroying reentry remain outside
  this bounded integration. Existing snapshot PropagationWorld is not used:
  it cannot represent the native live count/slot callback behavior proven here.
  No original file, game asset, save, profile, Cargo manifest or release changes.
