COMPLETED ACTORDATA+17C RECRUITMENT BACKREFERENCES -- 2026-10-16

Purpose
  Native501730 checks ActorData+17C when45E5A0 says non-recruitable.
  +17C is an incoming recruitment list. It is NOT transport_actor+184.
  The new lazy source API supplies the completed real list/presence; no proxy,
  default false, altered gameplay, parser/definition-loader edit, or dependency.

Production API (aa_rules::native::recruit_backrefs)
  ActorCatalog::native_recruit_backrefs(Precision)
    -> native::Result<NativeRecruitBackrefs>
  NativeRecruitBackrefs::definition(canonical_id) -> Result<&RecruitBackrefs>
    RecruitBackrefs { id, ids: Vec<String> (head first), sources }
  NativeRecruitBackrefs::present(canonical_id) -> Result<bool>
  actor_recruit_backref_present(&ActorCatalog, canonical_id, Precision)
    -> native::Result<bool>
  The caller should invoke/cache the arena only when native501730 reaches
  this read. Independent scalar getters need no arena. Invalid/unbound data
  remains an error, including failure to complete the global producer graph.
  Inputs/ActorInput/LayoutInput expose typed source facts for oracle replay.
  These are not runtime actors and do not create a playable world.

Native producer trace
  ActorData constructor4624B0 zeros+17C/+180.
  ActorData464BD0 creates Organization element callbacks at466Fxx before
  Recruit callbacks at467C89..467DB6. Sources are native Thing registration
  order, with each list in authored child order.473AD0 prepends callbacks to
  GameData+824;475230 walks that list head first after all files/Translate roots.
  Recruit callback461810 invokes460310; Element callback461750 invokes460000.

  460310 first prepends target to recruiter+4EC and recruiter to target+17C.
  For an Organization target, it walks layout+28/+2C expanded slot-group
  indices, including repeats. Current nonnull default-layout parameters receive
  the recruiter. The native4603F0 search compares each list value to the
  ELEMENT ITSELF, not the recruiter. This can preserve duplicate recruiters;
  presence of an element-self entry suppresses that propagation. Port retained.
  Non-Organization tail only emits unit-limit diagnostics; it cannot write17C.

  460000 stores the layout parameter through4C2310/45EEF0, validates Element
  flag8/property/packable-slot rule, then walks current Organization+17C head
  first and prepends each missing recruiter to the Element+17C. HQ/research
  validation follows the merge. Native ownership order is preserved; no
  transitive-closure approximation or precompleted parameter substitution.

Canonical identity / Translate timing
  Registered owners and source nodes are selected by actual source provenance
  and registration order. A canonical owner IDS is never passed through Find
  again (an alias could redirect even an already registered owner name).
  Property/Layout lookups use RootPosition at ActorData LoadData time.
  Authored deferred actor reference strings use the catalog's final native
  Thing Find only at resolution. Thus a later Translate changes the referenced
  target without changing callback owner identity. Getter definition(id) accepts
  only canonical keys, never an authored alias. Embedded NUL ends a Find string.

Validation
  native_recruit_backrefs.py executes original/v14 code at PC24/53/64.
  Actual473AD0 prepend constructor,475230 dispatch,461810/461750 callbacks,
  460310/460000,4C2310 and45EEF0 execute. The deferred queue is empty afterward.
  Explicit seams: typed final target-registry4BDBA0; operator new6F7810;
  CString buffer retain4012C0; deferred cleanup9454B0/6F7430; diagnostic throw;
  warning-only logging. Native resource count0 bypasses warning iteration,
  which has no recruitment-list writes. Page-zero access guard is retained.
  Native executed instruction bytes must equal original/v14; windows, original
  EXE hashes, generator hash, and all source facts are pinned in the JSON.

  native-recruit-backrefs.json:270 synthetic runs,45 graphs x2images x3PC:
  direct duplicate targets, actor ordering permutations, repeated slot groups,
  element-self suppression, deferred aliases, missing/wrong Element/Property,
  mobile-slot and HQ Research validation boundaries. Rust replays ordered lists
  and callbacks for successful graphs, explicit errors for invalid graphs.

  actual-source-inputs.json captures all1,028 registered native ActorData with
  actual source/provenance, properties/layouts/refs and final translated IDs.
  Native full source graph has906 deferred callbacks and327 nonempty17C lists.
  native-real-recruit-backrefs.json executes that entire graph six times:
  5,436 producer callbacks plus their native property/layout helpers. Every
  completed ordered list and callback order matches Rust. This does not run
  the complete original data loader end to end; source/TGI/layout projections
  reuse their existing native producer proofs. Target registry Find is the
  explicit identity seam described above.

  crates/aa-rules/tests/native_recruit_backrefs.rs:3 passed,0 failed:
  synthetic replay, final Translate versus canonical owner/source handling,
  and actual source/native full graph comparison (include-ignored required).
  Actual source fact projection is also equal underPC24/PC53/PC64; every
  native full-graph row must explicitly report success, not only matching lists.
  all-tests.log; isolated target/s-recruit-backrefs.
  Strict Clippy initially stops on pre-existing native_load.rs:878 manual_map
  and native_display_name.rs doc_lazy_continuation. clippy-scoped.log passes
  with only those two named lint allowances; no other source was modified.
  Full aa-rules/aa-metagame compatibility is coordinated separately by root.

Commands (workspace root, PowerShell environment variables as appropriate)
  CARGO_TARGET_DIR=target/s-recruit-backrefs cargo test -p aa-rules
    --test native_recruit_backrefs --release --locked --offline -j4
    -- --include-ignored --nocapture
  AA_GAME_ROOT=E:/Games/Axis and Alies is required for the actual-source test.
  python research/skirmish-20261016-recruit-backrefs/native_recruit_backrefs.py
  python research/skirmish-20261016-recruit-backrefs/native_recruit_backrefs.py --real
  To explicitly recapture source only, set AA_RECRUIT_BACKREF_CAPTURE to an
  ABSOLUTE path for actual-source-inputs.json, run actual source test, unset
  the variable, rerun --real, and rerun mandatory native comparison.

Limits / preservation
  Completed-data API does not expose partial load/fault side effects. Native
  allocation failure, invalid raw pointers, malformed storage read order and
  unit-limit diagnostics are not claimed by this field projection. The source
  must have a native-order registry; a generic from_files catalog errors.
  Lists are growable owned Vec values; no fixed native engine cap occurs here.
  No original game/release EXE/profile/save/network service was modified.
  This does not supply full ActorData loading, recruitment execution, a battle,
  multiplayer, campaign, or a new release. Root owns live/saved Economy binding.

Backup
  backups/before-recruit-backrefs-20261016-180254/native.rs is the pre-export
  shared-file copy. New sources/tests/evidence are pinned in findings.json.
