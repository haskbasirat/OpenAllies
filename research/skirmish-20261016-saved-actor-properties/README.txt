SAVED ACTOR BUILD-TARGET LIST -- 2026-10-16

Scope
  saved_actor_properties connects native4D4CF0 and direct Element-parent
  propagation4D4E50 to the actual saved Actor+108/+10C owner. Despite the
  historical module name these are Builder build-target ActorData identities,
  not PropertiesComponent numeric modifiers/effects. Existing581420 refresh
  is unrelated and is not substituted for this operation.

  Root adds ActorPropertiesContext and actor_refresh_build_options forwarding.
  The current owner's mutable ActorShell, canonical definition and temporarily
  held readonly Organization membership are supplied directly. Other Actors
  resolve from the same mutable generation-checked IdentityRegistry. The owner
  remains outside that registry during its in-progress Load. No Actor/world or
  Organization snapshot is committed only after success.

  Root migrates constructor-zero raw108/10C words to authoritative
  ActorShell.actor_list_108: aa_body::actor_property_list::State<String>.
  Component mapping preserves that owned field. Related boolean consumers read
  actual typed-head presence rather than an invented raw pointer or zero.

Bindings and partial order
  The separately proven borrowed core clears/frees old nodes first, then checks
  ACTUAL Builder component+A8 presence. Only then is canonical174 read through
  Catalog::canonical_component_mask, retaining source-owner validation at its
  actual native use. Bit18 independently gates the canonical504 list read.
  A present live Builder is never assumed equivalent to that definition bit.

  Completed Builder504 targets come from source-owned NativeBuilderData, cached
  by Catalog. The source producer is a separate proof/contract: it resolves
  typed BuildActor references and preserves canonical registered object identity,
  authored final list order, duplicates and source completion refusals. It does
  not certify unrelated global LoadData/deferred callbacks. This adapter never
  substitutes empty for a failed source lookup. Its source-node handles contain
  the actual definition owner and current arena index, without copying a list.

  Actor node handles contain the complete ActorId and per-owner NodeId. Head,
  tail, allocated nodes and unwritten fields belong to the actual State owner.
  Allocation/free and individual node/head/tail stores occur immediately, with
  current values reread at core boundaries. A foreign owner/source-node store
  or free refuses instead of aliasing another allocation. Canonical strings are
  identity handles from the source registry, not display names or guessed IDs.

  Current Organization members retain slot order, duplicates and nulls. Member
  derived lists merge with native deduplication/prepending. Detached members are
  not filtered: this call's iterator attached-only byte is zero. Actual Element
  header ownership is validated at use. Element+10 is a DIRECT PARENT ACTOR,
  as the original4D4E50 instructions show; there is no extra Organization+4 read.

  A per-call growable visited-parent set refuses invalid Element-parent cycles
  only after completed earlier refresh stores. It is a portable invalid-graph
  diagnostic, not native malformed-pointer/exception equivalence. There is no
  arbitrary recursion/depth cap and no successful return for a cycle.

Tests and evidence
  actual-saved-inputs.json captures six unmodified partial Kursk Organization
  records at all three explicit precisions and clearly labelled storage probes.
  The current source load uses a fresh managed stopped application interval,
  native mode1 and explicit elapsed-clock fixture0. The in-progress record has
  not completed native immediate Init/PostCreate, publication or battle startup.

  Explicit storage probes seed canonical build-target IDs in two ACTUAL saved
  Actor list allocations, replace the temporary member list with duplicates and
  nulls, add a test Builder, or substitute the canonical USA engineer definition
  identity for that test call. A direct-parent probe temporarily substitutes
  Element/Organization presence. All original owned fields/components are put
  back. These probes are not observed initialized native actors or Init receipts.

  native_saved_actor_properties.py imports the independently proved original/
  v14 core harness and executes unchanged4D4CF0/4D4E50/4364C0 for each captured
  context. Canonical identities map to opaque numeric labels while preserving
  equality. Outputs compare list order, tail value and live allocation count;
  the pure core separately proves complete read/store/allocation/free traces,
  partial dependency stores and native instruction pins at PC24/53/64.
  Default generator mode replays immutably; only --write regenerates fixtures.

  Portable source-owner, stale-generation and cyclic-parent errors are tested
  separately from native valid-context rows. They retain clear/free and prior
  member-union stores. Original files/assets/saves/profiles/releases and root
  Cargo manifests are not changed by this scoped adapter.

Reproduce from rust-rebuild, PowerShell
  $env:CARGO_TARGET_DIR='target/s-owner-recruitment'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-runtime --lib saved_actor_properties::tests --release --locked --offline -j4 -- --include-ignored --nocapture
  python research/skirmish-20261016-saved-actor-properties/native_saved_actor_properties.py

  Explicit capture regeneration:
    $env:AA_ACTOR_PROPERTIES_CAPTURE='1'
    [same cargo test command]
    Remove-Item Env:AA_ACTOR_PROPERTIES_CAPTURE
    python research/skirmish-20261016-saved-actor-properties/native_saved_actor_properties.py --write

Limits
  Allocation failure/reentry, native corrupt-pointer exceptions and full source
  lifecycle are outside this adapter proof. Actor Init/PostCreate/publication,
  complete startup and live battle steps remain separate required work.
