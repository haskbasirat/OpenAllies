COMBAT VALUE SOURCE CONTEXT SUPPORT -- 2026-10-16

Scope
  Read-only native/source investigation supporting world_start_stats. No
  production code, Catalog integration, original files, profiles or releases
  changed. This does not produce a completed ActorData combat-value arena.

New executed proof
  native_source_context.py / native-source-context.json:
  - 18 original/v14 cases execute4BCA00's actual post-registration insertion
    suffix4BCF6B..4BCFC5. Allocator success and already-created definitions are
    explicit inputs. Two linked lists are prepended: category descriptor+C
    indexes manager+5C; descriptor+18 native-kind indexes manager+68.
    ActorData is native-kind2, so its list head is [manager+68]+8. It is reverse
    successful ActorData creation order, independent of IDS alphabetization.
  - 18 cases execute full4BD800 with explicit child-operation seams. Exact
    order:4BB480,4BB130,4BB730,4BBF90(means),4BB590(values),45F7A0 per native
    Actor-list entry,4BD440,4BC7F0. Every child sees unchanged caller FPCW.
  - 6 cases execute full4BB590 with an empty native Actor list and empty CSV
    UVar twice.4B2E60 lookup uses B39938, nameCombatValueCSV, requiredtype4.
    It caches the actual variable pointer inB3B4F0 with guardbit1B3B4F4.
    A second call reuses that pointer. Typed lookup and unopened stream query
    are seams. No CSV file operation or nonempty debug branch is claimed.
  - All three x87 PCs, both pinned PEs,108 executed instruction pins and no
    unguarded page-zero accesses. Eight additional instruction windows pin
    the static application caller chain, renderer flags and extent getter.

Precision provenance
  Native Application::Init419FB5 calls renderer initialization7101A0 at41A200.
  Later in this same synchronous function,41A561 calls GameData::Load48DB60.
  That calls ThingDataManager completion4BD800 at490525. Net/simulation thread
  startup is later at41ABC5..41ABEE. Means and values are source-load work on
  the main application thread, not world restore/simulation callbacks.

  Existing research/phase8-tick/fpu-precision.md establishes that renderer
  creation requests no D3DCREATE_FPU_PRESERVE flag. D3D9 normally changes this
  thread toPC24. Hence PC24 is a STATIC post-D3D expectation, not an observed
  native control word here. Our oracle proves inheritance at all three PCs;
  it does not run Windows/D3D or establish the actual hardware/source-loader
  control word. Keep source-load precision explicit, separate from restorePC,
  and state any native startup/D3D assumption. Do not label a suppliedPC24 or
  restorePC53 as a captured native context. CRT environment/SSE2 dispatch and
  FP exception state are likewise explicit caller context, not sourced from
  a TGI setting. Existing numeric fixture executes the SSE2 pow branch.

Source ordering/identity
  ActorCatalog's native_load.rs projects thing_owner in successful registration
  order. Reverse/filter native_kind2 yields the manager Actor list only for
  that native-order catalog. ActorCatalog::from_files is a different syntax
  projection, not a native registry-order certificate. Require proven native
  source order/identity and account for diagnostics where native load would
  stop. Canonical references must not be translated again.

Effective variable dependencies
  source-dependencies.json pins all8 cached float registrations and original/
  v14 source declarations. Seven formula floats are:
    offset A4 index41 CombatDamageMinimum
    offset144 index81 CombatValueRangePower
    offset148 index82 CombatValueAreaPower
    offset14C index83 CombatValueOffenseWeight
    offset150 index84 CombatValueDefenseWeight
    offset154 index85 CombatValueMovementWeight
    offset158 index86 CombatValueMovementDefault
  Means additionally reads offset15C index87 CombatValueMeanValuesDiscardPercent.
  45E090 additionally lazily looks up typedfloat2 CombatValueRangedMinimum and
  CombatValueDefenseWeightRanged in SVar objectB39AA8, caching the variable
  pointers inB3A668/B3A664 with B3A66C bits1/2. The IGNORE_FOR_CV definition
  flagbit7 returns before both lookups. Range minimum lookup occurs before the
  positive-power gate; ranged defense lookup occurs later even when its value
  is not selected. Their variable identity/value remains shared, not copied
  from arbitrary runtime unit settings.

  These are9 formula SVars, plus1 mean-discard SVar. ModifierAttributeClampAdd
  at1A4 belongs to actual modifier getters when a modifier context is present;
  source4618B0 passes no modifier context and does not need this as a fabricated
  source dependency. Existing native fixture does also cover performer context.

  Game/SVars.tgi declares all10 fixed values. Registered defaults are not the
  effective source values: range0.1 is overwritten by0.15, offense/defense1 by0.5.
  Use the existing proven scalar parser with explicit source-load precision,
  exact source identity/hash, typed declaration and no ambiguous assignments.
  Never fill missing values with the constructor defaults.

  48DB60 also calls474C50 to apply registered WorldRule default values before
  copying sourceDefaults4B3580 and before4BD800. If any registered WorldRule
  targets one of these names, plain SVar declarations are not enough. Prove
  their absence for a bounded source set, or implement exact default application.
  The same distinction already exists in saved_action_settings.rs.

  CombatValueCSV is a UVar, not an SVar; both original UVars.tgi/UVars_AA.tgi
  explicitly set stringnull. The source audit scans original root UVars*.tgi
  and v14 loose overlays; the current overlay has no further assignment.
  Existing user-profile/command-line mutations are outside this source-default
  observation. Only a proven empty effective CString permits skipping debug
  stream behavior. Nonempty must remain explicit until implemented.

Mean-value ThingInput mapping
  Use the completed ActorData words, not arbitrary templates or live actors:
  +174 completed component mask; +1EC/+1F0 ordered local Performer AbilityData;
  AbilityData+0 definition+64 power, local+4 reload_time, local+18 recharge,
  first animation set/first attack+10 length; +288 defense, +228 mover speed,
  +284 health. Eligibility is Performerbit5 AND Bodybit0 AND Moverbit3 AND
  NOT(Experiencebit17 AND Elementbit8). The existing mean_values code owns
  sorting, trimmed sums, comparisons, explicit spills and assertion outcome.

  Settlementbit14 independently reaches45E6D0 for maximum74. That getter reads
  signedcount+468. Count<1 returns completedbase+478. Positivecount follows
  first12-byteLevel+8 -> WallSet+1C -> wall-cornerActorData+3C, then adds+478
  and literalA03448(2.0000999f) in native x87 order.

  Actual source producers:4640F0 Levels/Level records include wallset_ids;
  47DF59..47DFD1 resolves wall_corner_ids and stores WallSet+1C. Shape's
  wall_diameter initially becomesbase+478 via*0.5, BUT4640F0 then460940
  DetermineWallPieces replaces+478 using actual segment/gate/corner geometry,
  side/gate counts, FSIN/FCOS, length fitting and float stores. It also mutates
  shared wall/placeholder definitions. Raw wall_diameter is NOT the completed
  radius. Until this producer is ported and numerically proved, a reached
  positive Settlement must remain unsupported. A source-proven clearbit14
  avoids this dependency; do not fabricate a default extent and call it proven.

Existing numeric proof reused, not rerun
  phase8-combat/native-combat-value-fixtures.json (hash pinned):108 mean,
  180 definition/list,480 ability cases, plus modifier/performer coverage.
  This proves arithmetic on supplied inputs, not the complete source producer.
  No Cargo tests were needed because this task adds evidence only.
