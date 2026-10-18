SAVED OWNER ACTION REBUILD -- 2026-10-16

Scope
  saved_owner_actions::rebuild(&Catalog, OrganizationAttachContext) directly
  constructs the existing SavedActions adapter and calls the native-backed
  aa_actors::ability_flags::rebuild(owner), native4D1350. Root connects that
  callback from the Organization loader using the same actual mutable Actor,
  held Organization, identity registry and source/lifecycle bindings.

  This is the owner-call integration, not a new command-availability model.
  No contributor, publication service, source value or gameplay behavior was
  invented. Existing Element, Organization, contributor and Mind dispatch run
  in their native order and retain their existing required errors. Active Mind
  starts None for this saved Organization Load caller. No Mind payload is moved
  or replaced by the wrapper; existing contributor arena ownership stays intact.

Actual storage/order
  Native4D1350 saves the previous F0/F4 mask, clears both actual words before
  any guard or contributor lookup, performs the reached dispatch and calls
  changed4CFCC0 only when the final mask differs. Existing SavedActions writes
  the actual ActorShell words immediately. No Actor/Organization snapshot,
  local recomputation followed by success-only copyback or default contributor
  is introduced. Typed build-list storage retains its actual owner/allocation.

  The owner's Organization is temporarily absent from its component slot while
  Load runs. It remains logically present through the supplied actual context,
  as SavedActions::has_component states explicitly. The native oracle always
  installs Actor+7C for this held Organization; captured component-slot lists
  intentionally omit that temporary slot and do not imply native absence.

  Publication is the actual existing action_publication coordinator: Element
  parent rebuilding, virtual owner lookup, signed Object4C>0 gate, second owner
  lookup and required76C7F0, then CURRENT renderer link/flags and selected view.
  A missing service stays an error after preceding stores. The wrapper does
  not suppress UI work or treat that work as completed because a load is stopped.

Evidence
  Frozen proof contains90 native rows:36 unmodified saved rows (six owners x
  three precisions x two images) and54 labelled probe rows (nine substitutions
  x three precisions x two images).78 rows return; six stop at required76C7F0
  and six stop before renderer+54.578 executed instruction addresses are pinned.

  Captures use six actual partial (2v2)Kursk saved Organization owners at all
  three explicit precision modes, with the same owned components and member
  masks. They are stopped managed mode1 diagnostics with explicit elapsed
  clock fixture0, not completed native Init/PostCreate or published battles.

  Labelled probes substitute only caller-owned test inputs: previous F0/F4,
  dead flag, the outer state26/36/25 guards, an actual saved Kingdom ID with
  empty actual HQ vector, signed Object4C, or a nonnull unavailable renderer
  link. Original fields are restored afterwards. These are not observed
  initialized actors or claims about native screen/renderer objects.

  Unchanged cleared mask plus a nonnull renderer link proves4CFCC0 is skipped.
  Changed-mask owner-view and renderer probes preserve actual clears before
  explicit required-service errors. Signed-negative Object4C skips owner views.
  Member masks and the actual Organization contents remain unchanged in these
  bounded shapes; the held typed-list allocation address is checked too.

  native_owner_actions.py separately loads pinned original and v14 images and
  executes4D1350,57B390, native ground scans/member availability, actual401060
  generation checking, empty-HQ4E1430/4E1E40 and4CFCC0 where reached. It does
  not seam a complete contributor or answer an invented command mask.
  Existing native ActorState registration/masks populate the descriptor table;
  actual captured resupply source fields fill valid entries. Unavailable entries
  are guarded against reads, never supplied as false. Actual saved HQ vectors
  are captured and asserted empty for this bounded oracle; unsupported nonempty
  HQ or extra contributor/Wall shapes are explicitly rejected by the generator.

  Required76C7F0 interrupts before its body. Renderer-required interrupts at
  4CFD0E before the unavailable+54 read; it does not fabricate renderer flags.
  Original/v14 instructions are pinned and compared. Page zero faults except
  legitimate FS access. Precision remains explicit at PC24/53/64. The script
  replays frozen evidence by default and regenerates only with --write.

Reproduce from rust-rebuild, PowerShell
  $env:CARGO_TARGET_DIR='target/s-owner-recruitment'
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  cargo test -p aa-runtime --lib saved_owner_actions::tests --release --locked --offline -j4 -- --include-ignored --nocapture
  python research/skirmish-20261016-owner-actions/native_owner_actions.py

  Deliberate regeneration:
    $env:AA_OWNER_ACTIONS_CAPTURE='1'
    [same cargo test command]
    Remove-Item Env:AA_OWNER_ACTIONS_CAPTURE
    python research/skirmish-20261016-owner-actions/native_owner_actions.py --write

Limits
  Actual Mind/special contributor execution in these owner shapes, nonempty HQ
  eligibility, real owner-view/renderer/selected-view publication, malformed
  native pointer exceptions, arbitrary callback reentry and full initialization
  remain outside this integration proof. Their existing separate proofs and
  explicit unsupported boundaries are preserved. No original file, asset, save,
  profile, release or Cargo manifest is modified by this scoped wrapper.
