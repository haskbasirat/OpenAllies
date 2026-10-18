SAVED ORGANIZATION LEADER -- 2026-10-16

Scope
  Native579A00 SetLeader and5797E0 ComputeSlotPosition are now bound to the
  actual saved OrganizationState and owned saved Actor/Element/Thing fields.
  The arithmetic/read/store sequence is separately provided by the borrowed
  aa-movement::borrowed_formation core. No Organization snapshot, cloned world,
  commit-only-on-success path, invented Actor Init or substitute RNG is used.

  Root-owned integration adds OrganizationLeaderContext and the required
  RestoreDefinitions::organization_set_leader callback. The restore loader
  threads the owner's actual ThingShell alongside its actual ActorShell;
  Catalog forwards that context to saved_organization_leader::set_leader.
  Existing context implementations still stop explicitly by default.

Owned storage and services
  +40 is stored before looking up the previous leader's Element. ActorId
  preserves the full generation-bearing identity. The active owner is outside
  the registry while loading and reads its supplied actual shell. Other actors
  must resolve to the current RestoredRecord::Actor with a real ActorShell.
  Required Elements validate their component header owner at their first use.

  +78 offsets are written one axis at a time into the actual vector. Errors
  preserve all prior leader/offset stores. The Organization allocation remains
  unchanged. Positions=() is an identity token only because the exclusively
  borrowed Organization and readonly registry cannot replace its vector during
  any implemented callback. The borrowed core separately proves reentrant
  pointer recapture order and failures using a mutable laboratory world.

  Canonical LayoutData primary_slot is read lazily. Current Formation+74 is a
  native-layout source handle; the adapter reads LayoutFormation+C's actual
  FormationData.organized byte and LayoutFormation.positions offset_bits.
  578250 rereads the current handle at its native use. Owner facing and member
  positions are original native XY Thing fields, with no renderer conversion.
  Catalog::layout_data verifies the original source owner at actual source use.

  Scattering advances the same LoadContext NativeRng already owned by restore;
  it does not seed, clone or reset production RNG. The tests compare the whole
  resulting RNG state. Native404F90 detached-member diagnostics append to the
  actual native_messages sink with source ordinal, function404F90 and lineAB0.
  The body uses signed32 IDs and six fractional digits (%d/%f finite-domain
  formatting). The fixture proves diagnostic fields/order, not CRT byte-string
  formatting for every number, locale or nonfinite input.

  A present actual owner Mind reaches an explicit error naming virtual+114.
  Its concrete execution is not fabricated or treated as a no-op. Earlier
  leader/offset stores survive this boundary. Missing/stale/loading/non-Actor
  IDs, absent required Elements and invalid source fields refuse explicitly.

Evidence and reproduction
  Frozen evidence has78 native rows:36 unmodified saved rows (six actual
  Organizations x three precisions x two images) and42 explicitly substituted
  storage rows (seven probes x three precisions x two images), plus12 portable
  identity/source checks. This Kursk restore prefix has no nonempty Organization owner
  with an actual Mind. capture-final.log preserves the failed exploration that
  established this absence; the unsupported expectation was removed. No fake
  initialized Mind was added. Actual saved Mind+114 retained-store behavior is
  therefore not an adapter oracle claim; the pure core proves its required
  callback/store order separately, and the runtime refusal was source-audited.

  actual-saved-inputs.json captures actual (2v2)Kursk.RMP InspectPartial saved
  allocations and separately named explicit adapter-storage probes. Captures
  use a fresh stopped managed application interval, native load mode1 and an
  explicit elapsed-clock fixture0. These do not certify the omitted immediate
  Actor Init/PostCreate or a live battle. All three x87 precisions are explicit.

  native_saved_leader.py executes original and v14 native579A00/5797E0,
  native578250 and native85D840 for every captured input. The two PE images are
  loaded separately; dynamic globals are rebound after image loading. Executed
  in-image instructions are pinned/equality-checked. Static direction-table
  entries use the previously proven PC53 native startup table. No trigonometric
  transcendental is emulated here. Page zero faults except valid FS access.
  No original executable, assets, save or user profile is modified.

  Mind+114 is an explicit test interruption before its unported body, never a
  native exception/success claim. In absent-Element rows the original null read
  faults; Rust returns its portable required-Element error. Only the retained
  store prefix is compared, not native exception/dialog behavior.

  The explicitly substituted probes use two actual saved Actor allocations
  with test Element payloads, two-slot member/offset contents, a captured raw
  RNG state, optional canonical formation selection and a temporarily absent
  owner Mind. Original components and Organization fields are put back after
  the tests. These test inputs are never claimed as native initialized state.
  Cases cover new/same leader, relative/detached old slot, scattering, canonical
  oriented offset rotation and failure at the new Element after the old slot.

  Portable identity/source checks are outside native malformed-pointer parity:
  wrong active Organization owner, stale generation, origin-slot source lookup
  bypass, and a later required source-owner failure. They verify actual retained
  stores and unchanged offsets/RNG at each missing service boundary.

  From rust-rebuild, PowerShell:
    $env:CARGO_TARGET_DIR='target/s-saved-leader'
    $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
    cargo test -p aa-runtime --lib saved_organization_leader::tests --release --locked --offline -j4 -- --include-ignored --nocapture

  Deliberate capture regeneration is separate from frozen fixture replay:
    $env:AA_SAVED_LEADER_CAPTURE='1'
    [same cargo test command]
    Remove-Item Env:AA_SAVED_LEADER_CAPTURE
    python research/skirmish-20261016-saved-leader/native_saved_leader.py

Limits
  Concrete Mind+114, native C++ exception/log encoding equivalence, arbitrary
  callback reentry in the saved adapter, nonfinite arithmetic, actual native
  immediate record Init/PostCreate and full map publication remain unproven.
  This binding does not supply57C800 follow-leader,524B80 member-added or later
  gameplay callbacks. Advancing a stopped diagnostic is not a playable battle.
  Original files, previous releases and root manifests are unchanged.
