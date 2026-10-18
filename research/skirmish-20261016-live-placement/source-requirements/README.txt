Native source fields for creation placement (2026-10-16)

Production files
  crates/aa-rules/src/native_source_requirements.rs
  crates/aa-rules/tests/native_source_requirements.rs
  aa-rules/src/lib.rs: one new module export. No Bank/Catalog storage edits.

Scope
  A source-owned, canonical/provenance-bound and source-PC-bound immutable
  dependency slice for ObjectData+34 and ActorData+430/+444/+1A8/+290/+4E0.
  This does not claim full LoadData/deferred callback or 4BD800 completion.
  It does not supply shapes, component masks, final ThingFlags, combat values,
  game profile/settings/CSV context, or full live ObjectInit.

  1959 registered Things; 1084 ActorData, including 56 RandomActorData, were
  inspected by their exact registration provenance, rather than by current Find.
  The shipped resolved source has no SettlementComponent in those registrations,
  no Upgrade instances in the complete conservative whole-input census, no
  nonempty resolved required_object_ids, and zero Settlement-category objects.
  The source producer refuses positive versions of these writer graphs. It
  does not replace unknown references or missing Body bools with zero.
  Before publication, the full private-resolver Thing/Unit/Projectile/VFX root
  set must agree one-to-one with the public actor projection: root count,
  registered provenance, root IDS and template chain. Root/template source
  identities must also exist uniquely. Public truncation, rename, duplication
  or source-list clearing cannot hide a writer. The guarded slice has no
  order-dependent source mutation; arena_ordinal is explicitly a portable
  logical slot, NEVER a native registration ordinal/address.

Native needs_space_for_settlement_zone virtual+2C (45DB10)
  1. Calls own virtual+24 via49B490, actual byte+34 getter462E50.
  2. If false, reads component mask+174 dword and tests Settlement bit14.
  3. If clear, reads Settlement owner+430 and tests nonnull.
  4. If null, reads required object+4E0; if nonnull calls its virtual+24.
  +430 is Settlement owner. The Construction required object is +4E0.
  First tutorial02 Kingdom781 uses canonical `kingdom` and reaches all four
  reads: own+34, mask+174, owner+430, required+4E0. All are false/clear/null,
  so its actual native result is false. The separately supplied ThingFlags
  0x20C46900 must NEVER be substituted for the component mask.

Typed writers and exclusions (instructions pinned in JSON)
  4624B0 constructor zero stores:
    46257A -> byte+1A8; 4627CB -> byte+290;
    462B25 -> word+430; 462B43 -> word+444; 462C85 -> word+4E0.
    EBX is zero from4624E1. Individual store execution is deliberately labelled
    isolated; no allocator-heavy full constructor execution is claimed.
    The same constructor also initializes +45C centers and +4C0 upgrades empty.
  469980 RandomActor constructor calls4624B0; 468D20 loader calls464BD0.
    Neither wrapper writes the requested fields after its parent.
  49DC40 root field needs_space_for_settlement_zone writes byte+34 at49DC7E.
    It is a root Object field, not Size.needs_space_for_settlement_zone.
  464BD0 Body.captureable GetBoolean return writes byte+290 at466309.
    Body absence retains constructor zero; a present but missing/invalid bool
    fails explicitly. This is independent of current target aliases.
  4640F0 Settlement writes a resolved town center's +430 at464558 and shared
    placeholder +1A8 at4648DF/4648EC/464902. It also populates owner+45C centers.
    Resolved absent SettlementComponent across ALL registered kind2 and19,
  plus hash-verified omitted-source guards, excludes every such source call.
  This producer additionally refuses ANY opaque omitted file: excluding its
  Settlement/required-ref names would not exclude a reload of byte34/290.
  The current shipped receipt has no opaque files, so that refusal changes no
  current source fields or captured/native fixture values.
  464BD0 Construction registers Upgrade callbacks and registers a required
    object callback only when the authored string length is nonzero
    (467BCA..467BD7). Its callback function pointer is A03480 ->461850,
    which dispatches4606D0 with its native +10 selector. This is a one-slot
    callback table, not an ordinary destructor/vf4 vtable. Selector1 writes
    +4E0 BEFORE checking shape/tile_locked/MARKER conditions; a later failure
    would retain that store. Positive required references are refused here.
  4606D0 selector0 prepends +4C0 upgrade refs. 4BB480 invokes4612E0/45FAE0
    only for nonempty outgoing upgrades, then makes affiliations for nonnull
    Building+430 or Settlement+45C centers. 45FAE0 can inherit +4E0.
    Empty complete Upgrade census excludes those +4C0 refs; absent Settlement
    excludes owner/centers. Therefore 4BB480 cannot reach45FBB0, which is the
    positive inverse-affiliation writer of otherActor+444 (45FC1C).
    This is a branch/dataflow proof, not merely a count of category21 entries.

All later 4BD800 postpasses audited
  4BB130 selection completion: 492C20/492B90/4C2290/4C21E0 clone a layout;
    45F7B0/45F240 append only owner+490/+494. They do not populate +444.
  4BB730:471A10/4B6BA0 append group+18/+1C weighted refs, a different typed
    receiver. They do not write ActorData placement fields.
  4BBF90/4BB590 and recursive4618B0/461EE0 compute manager means and CV
    storage+1E4/+30C/+36C; they do not write these six source fields.
    Their full source lifecycle/numerics are owned by combat_source_completion,
    not this receipt. No default combat values are introduced here.
  45F7A0/45EE40 reads +44C and predecessor +4D0 (when mask bit15), deduplicates
    and prepends into +454/+458. A positive child-list case is replayed; it is
    not necessary to pretend all ConstructionComponents are absent.
  4BD440 UpdateEditorNames sorts pointers, changes only editor CString+170 and
    a temporary placement-group bitset, and logs/times. Its helpers construct
    and refcount strings. No requested typed source field is written. This is
    a static typed instruction/callee audit, not a full editor-name oracle.
  4BC7F0 replacement checks call4BC5E0, which verifies containment and can
    repair ThingFlags+1C using its replacement mask. Flags+1C is NOT +174 and
    does not overlap any field in this receipt. It then calls virtual+8C for
    every registered Thing. All24 final constructor vtables were checked:
      kind2/19 ->461340 (Actor Verify);
      Object-derived1/3/18/20/21 ->49C460 (Object Verify);
      other kinds ->4B2830 (empty).
    49C460 can clear ownable ThingFlags+1C and set ObjectData byte+16D when
    referenced as a required object. 461340 calls49C460, checks deployment,
    Marker ownability, Denizen reach, wall size and HQ flags; its only remaining
    persistent write is assertion guard B3A670. Called radius/range/shape
    getters read source fields. No requested field is written by these paths.
    Diagnostic/error logging and allocation failure/reentrant external code
    remain excluded, rather than being recast as a successful whole startup.
  Source destructors (including468D50) clear lists while destroying an owner;
    they are outside the lifetime of the held immutable source arena. Reload
    or arbitrary later source mutation is not certified by this receipt.

Native evidence
  native_source_requirements.py -> native-source-requirements.json:
    144 original/v14 x allPC full45DB10 synthetic branch cases;
    6504 original/v14 x allPC registered ActorData source-field projections;
    30 exact isolated constructor stores;
    6 whole guarded4BB480/4BB130 runs and positive45F7A0 child propagation;
    6 complete4BC7F0 virtual dispatch passes over actual24 vtables with valid
      empty FX and empty replacement heads (NOT the actual full source graph);
    54 positive prepass leaves, including actual+444 affiliation writes, with
      the other placement fields protected. The prior independent leaf oracle
      is imported under a unique name; only its isolated Machine cases run.
  Every executed original instruction must equal v14. Static function windows
  and96 vtable slots are byte-pinned. The full direct memory-store/call inventory
  supports the typed static audit above; unrelated equal offsets are not called
  ActorData writers. Null-page accesses are guarded except genuine FS:[0].
  The native source-projection rows intentionally initialize only mask bit14;
  they make no claim to the other full masks (including Detection bit22).

Tests and reproduction
  Actual sources are in actual-source-requirements.json (all3 source PCs).
  Set AA_GAME_ROOT to E:\Games\Axis and Alies, then:
    cargo test -p aa-rules --test native_source_requirements --release --locked
      --offline -j4 --target-dir target/s-builder-source-review
      -- --include-ignored --nocapture
  The focused real-source test recomputes the exact receipt, compares every
  native source-projection result and checks token/source/PC identity. Synthetic
  tests cover canonical shadowing, RandomActor, inherited Settlement, Upgrade,
  required-ref refusal, missing bools, missing native registry loading, opaque
  inputs and public-projection tampering. The final focused run passed6/6.
  Broader rules/metagame compatibility is coordinated by root after all writers
  freeze; this subtask does not run redundant concurrent full-suite link jobs.

Runtime contract
  Own the arena once beside the actual source Catalog and its actual source PC.
  Use a canonical runtime definition and verify token.source against that held
  definition's provenance. Never call current Find again for a canonical owner.
  +430/+444 word() results are documented 0/1 null-presence predicates, NEVER
  native addresses. +444 is a list-node pointer, not a definition pointer.
  reference(+4E0) keeps a canonical ID when implemented positively; this slice
  only certifies null. No mutable state is accepted from arbitrary callers.
  Reach-dependent live placement must still require shapes, final flags, masks,
  registry state and settings only when the native branch needs them.
