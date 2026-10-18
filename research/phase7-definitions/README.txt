Phase 7: original definition binding for ground actor runtime

Implemented and source-bound:
- ResolvedNode supports ordered inherited list instances, descendant child templates,
  unique non-Thing root lookup, and Fixed/Float/Boolean/String/Integer/Point2/Point3
  scalar inputs with value and declaration provenance. NULL strings become empty.
- ObjectData collision shapes preserve native point/circle zero bounds and rectangle
  x87 rounding. Soft NULL keeps the blocking kind but uses soft dimensions; target
  NULL copies the soft shape. Visual scale is not applied to collision getters.
- View creation data orders AspectNormal first when variations are disabled, then
  authored Aspect entries; positive weights accumulate through native f32 stores.
- Ground inputs cover Body/Mover fields, economy supply_required, final Detection
  and remote Detection ranges, Organization morale/layout member capacity and
  default element parameters. Group Property IDs and default Actor IDs are distinct.
- Denizen guard-range projection now binds source Inventory presence, original
  guard+2 default and validation, then the empty-list reset of guard_range only.
  Empty original rus_corps Inventory yields guard0/operational12. Other Denizen
  component/AI/runway/deployment/reference callbacks are not completed by this API.
- Construction constructor inputs expose original Economy Cost and certify an
  absent Upgrade graph across1847 supplied TGI files:1list declaration,0instances.
  Two existing parse omissions are separately hash-checked non-NUL files without
  the ASCII Upgrade name anywhere; they cannot add that node, but are not claimed
  fully parsed. Any potentially relevant omission/nonempty graph rejects. Native
  ctor/predecessor/callback paths are pinned; nonempty upgrade ordering is unported.
- Checked Ability gameplay binding validates each original Character animation set,
  equal attack lengths, source identities, target filters, native resource costs,
  flags/main damage, area geometry, spawn XY sign and recharge arithmetic. Five
  infantry factions bind two ordered abilities each through the actual aa-animation
  metadata provider; US rifle/grenade recharge are3/15seconds. Count comes from the
  materialized private list, without a caller-supplied count.
- Resource/Damage native ordinals are assigned only when all authored entries are
  in the verified single indexed source file. TargetType begins with16 compiled
  entries, reuses those IDs, then appends the17 A&A source entries. Native u32 shift
  wrapping (index32 aliasesbit0) is preserved. Bone IDs retain source identities;
  this does not assert rendered skeleton binding.
- ActorState inputs expose typed priority/type/Mind flags and ordered Property IDs.
  Runtime descriptor group/ordinal selection belongs to aa-actors.
- Commander inputs are explicitly constructor projections: native ordered Faction
  local/shared identities, main effect power, and the max-range overwrite. Special
  operation effects remain gated by aa-runtime's constructor-only definition Bank.
- NationData+3C/+40 is the Badge texture array/count, not technology data. Ordered
  authored texture expressions and counts are bound for all five original nations;
  resource formatting, texture existence and decoding are separate concerns.
- DynamicDoodad native kind21 inherits Object flags and shapes. Its distinct mask
  and two component slots expose typed Disturbable/Breakable source inputs. Native
  repeated component types overwrite the previous slot. All65 original definitions
  bind. Actions, trampled visual-aspect resolution and gameplay updates stay gated.
- CameraData kind20 also retains Object flags/shapes. Its typed Float distance and
  VFX reference are constructor inputs; resolving and rendering that VFX remains
  outside the source projection. Original camera definition passes the source check.
- Actor InitCreate snapshots resolve numbering/name-list, trailer category/Bone/
  point3 hitch and reversed initial Property references. Ground Performer constructor
  energy is bound to mana_max; this API explicitly requires an empty Turret list.
  These projections do not imply complete Actor InitCreate or post-create execution.
- Shared Property inputs retain the entire supported Flag/ResistDamage list and
  typed metadata. Other modifier kinds reject, never disappear. Original indispensable
  has one NOT_DISPOSABLE(index11) modifier, technology byte true, target flags empty,
  CV bias zero and no FX. Technology+5C is distinct from target-flags word+24.
- All14 initial Property IDs for five infantry factions and the US infantry regiment
  bind. Only infantry contains a modifier: RESIST_DAMAGE armor_piercing, amount0.2
  stored unchanged. Garand's source Add examples are commented; it has no modifiers.
  Property target-word parsing uses literal comma/space-delimited IDs, without the
  Ability parser's prefix/default rules. Infantry sets bit20, org_infantry bit31.
  Runtime Resistance aggregation/interpretation belongs to aa-body.
- Native index wildcard sorting and source-created Settings Boolean overwrite order
  are proved for SVars.tgi followed by SVars_AA.tgi. This is a bounded Settings case,
  not blanket last-wins semantics for arbitrary TGI registries or templates.

Native evidence:
Original AA.exe and accepted-v14 byte ranges are pinned in each fixture. Native
instructions execute through Unicorn, with successful allocation/string pool,
source getter or registry-lookup seams documented per fixture. These are bounded
function/fragment proofs, not a claim of whole engine execution.
  native-shapes.json:480 shape cases plus enum/FPU startup observations
  native-strings.json:8 native plain-string assignments
  native-lists.json:64 ordered-copy cases,21 descendant compatibility branches
  native-creation-definitions.json:36 native View iteration/weight append cases
  native-values.json:7 integer and30 point assignments
  native-ground-definitions.json:20 layout expansion and486 Detection finalizers
  native-remote-detection.json:20 remote range condition cases
  native-layout-parameters.json:16 original element/property/post-link cases
  native-ability-derived.json:28 target parses,21 target defaults,15 effect finalizers
  native-ability-arithmetic.json:24 recharge cases,7 dense cost vectors,3 resource enums
  native-ability-geometry.json:33 area geometry/validation cases
  native-faction-constructors.json:6 Faction list cases and10 effect enum registrations
  native-settings-order.json:4 Boolean overwrite sequences and11 native sort cases
  native-nation-badges.json:6 original Badge list/count/append cases
  native-doodads.json:21 derived/component loader cases and2 native type registrations
  native-init-lists.json:6 initial Property prepend cases and2 technology-byte cases
  native-camera-definition.json:12 original CameraData derived-loader cases
  native-property-definitions.json:22 target-word and36 Resistance loader cases,
    plus10 native modifier enum registrations
  native-denizen-guards.json:180 guard default/validation/empty-inventory cases
  construction-source-evidence.json:whole-source no-Upgrade certificate,
    native caller/constructor pins and4 original empty-recursion cases
  registry-order-evidence.json: source-index lines plus peers' native registry proofs

Validation:
record_evidence.py runs final aa-rules tests, original/v14 asset regression, and
strict Clippy, storing exact commands and source/fixture digests in evidence.json.
Actual animation + Ability integration is independently exercised by
  aa-animation/tests/native_definitions.rs
  original_character_sets_preserve_authored_timing_and_aspect_fallback
and research/phase7-animation-runtime/original-actor-animation-sets.json.

Explicit remaining scope:
- LoadedPerformer is the bounded ground simulation definition loader. It does not
  execute all UI icon, FX/audio, generated global AbilityDefinition IDs or live
  projectile construction side effects of4BF990. It is not a complete engine load.
  Turrets, required-property filters and effect kinds beyond MAIN_DAMAGE reject;
  tank metadata is valid but its completed turret Ability is unsupported.
- Shared general/faction ability execution, SUMMON, general Property expressions,
  live combat/AI/animation rendering are owned by their runtime subsystems.
- Denizen runtime components and deferred references remain unsupported beyond the
  explicit guard snapshot. Organization HQ/research/category propagation effects are
  outside the default element parameter projection.
- General TGI expressions, scientific-notation point inputs, localized string
  evaluation and arbitrary registry file ordering remain unsupported. Finite x87
  arithmetic explicitly receives precision; startup PC53 evidence alone does not
  prove every gameplay thread's FPU lifecycle.
- ActorCatalog still reads all catalogue TGI members. The wider Body/Mover audit
  resolves1627/1645;18 duplicate-template/Thing ambiguities remain explicit until
  original index selection/replacement semantics are proven. Current source-path
  guards prevent unknown multi-file native ordinal assumptions.
- Original assets/executables were read only. Changes are in rust-rebuild; baseline
  source archive is backups/phase7-aa-rules-before-definition-binding.zip.
