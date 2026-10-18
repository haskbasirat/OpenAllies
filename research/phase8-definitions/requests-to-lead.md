# Requests to lead — phase 8 definitions (2026-10-14)

All requests are backed by native evidence in this folder
(`native-definition-registry.json`, produced by `native_definition_registry.py`,
replayed by `crates/aa-rules/tests/native_definition_registry.rs`).

## R1. actors.rs minimal hook — please confirm ownership (DONE, needs sign-off)

`crates/aa-rules/src/actors.rs` was not in my file list, but groups 4 and 5 of
the inventory point at it and nobody else lists it. I made three minimal edits
(full pre-edit copy: `actors.rs.before-native-load.txt`):

1. `Definition` gains `bound: Option<Vec<usize>>` (template declaration bound at
   parse time, 5BF0A0); `Resolver::layer_sequence` follows it when set, and
   otherwise keeps the old by-name lookup. New private `Resolver::node_at`.
2. `ActorCatalog::load_v14` now calls `native_load::load` (index-ordered native
   load). The previous every-member behaviour stays available as
   `ActorCatalog::load_v14_all_sources` (a syntax census, not the registry).
   `from_files` is unchanged.
3. `pub use resolved::{ResolvedNode, native_load};`

Please confirm or reassign. No other actors.rs logic changed.

## R2. Restoration agent: native missing-definition skip (group 2, 3,912 records)

`aa-mapworld/src/restore.rs restore_dynamic` / `bootstrap.rs resolve_definition`:
when the native Thing lookup returns null (`ActorCatalog::find(id)` → `None`; it
now applies the 427BD0 translation map), AA.exe does NOT fail the map:
4FAE80 logs `load_warning_unknown_thing_ids|s` (string A2CAF8 via 4B9F40/5A1040),
returns null, and 4FC340 seeks to the next DYNAMIC_THINGS index entry (858F50).
The record must be skipped (recorded as a native warning), not reported as an
error. Today bootstrap.rs returns `Unsupported("missing definition {id}")`.
All remaining 3,912 records of this group are IDs that the native registry does
not contain (their files are not in the index, or their translate.tgi entries
sit inside the `/*` opened at line 1292 that the original parser closes only
at line 5310 — the native parse of translate.tgi yields 182 Translate roots;
see README and fixture section `translate_file`).

## R3. Restoration agent: class-changed record conversion (group 1, 7,879 records)

The definitions are correct (native class 9 DynamicDoodad for the trees; 3 for
london_bridge). AA.exe converts these records instead of failing:
- 4FAE80: definition class `[[def+18]+1C]` != saved class AND saved class == 2
  AND caller context non-null (4FC600 always passes its vector, 4FCF0A..4FCF45)
  -> 4F26F0 builds a 0x30-byte record, 405B80 copies the IDS, 4EA050 reads the
  remaining saved Object fields from the stream (4EF0C0 + optional owner word at
  +2C), 4F9840 appends it; the record itself yields null (no registration).
  Any other mismatch logs and skips.
- 4FC600 after all sections (4FD47C..4FD781): for each converted record a
  creation-parameter block (vtable A25B1C) is filled (definition +4, saved ID
  +8, position/direction words +14..+24, owner +2C checked against the
  definition's +108/+110 and vf+30), the name string is copied, and the world
  creation function 4F9C10 is called; null -> log
  `load_warning_cant_replace_changed_gclass|s` (A2CE90), else
  `load_warning_replaced_changed_gclass|s` (A2CE40). This is a FRESH creation
  path (4F9C10), so it needs the creation/RNG port, not the saved-load path.

## R4. catalog.rs owner: Kingdom resource lookup returns null (group 3, 106 records)

`aa-runtime/src/catalog.rs resolve_named_definition` uses
`ActorCatalog::resolved_definition`, which errors on 0 matches. Native 4CDC70
reads each saved resource record through 4CDBB0: empty IDS -> null (no lookup);
otherwise 48D500 (generic DataManager Find); a null result throws
"Unknown IDS %s for class %hs" (41DFB0), which 4CDC70's own catch funclet
(4CDD04, FuncInfo AE4B10) swallows after consuming the record's 4-byte value
(41CD00); the loop continues and the function returns 1, so 4E8646 logs
"unrecognized resource IDS loading kingdom" (A2A1A0). Proven by the `kingdom`
fixture section (EH dispatch seam). Please use the new
`ActorCatalog::native_named_definition(kind, id, pc) -> Result<Option<_>>`
(resolved.rs) and return `Ok(None)` for `ipcs`; special_state.rs already skips
`None`. Recording the warning flag is optional bookkeeping.

## R5. construction.rs owner: no-Economy zero-cost path (group 6, 1 record)

`aa-rules/src/construction.rs` errors with "Construction without Economy data
requires separate native zero-cost path binding". Native: the Construction
constructor 565FA0 -> 565F20 -> 5657B0 builds/appends a cost entry (45F4D0,
5655C0) only when the ActorData component mask (+174) has bit 9
(EconomyComponent). Without it no cost entry is appended (fixture section
`construction`, masks without bit 9). The definition-side answer is "empty
cost list", not an error. hires_desert_horizon (cutscene_short_03) is the only
record.

## R6. aa-rules real-data test pins that encoded the old all-files catalog

Both are `#[ignore]` real-data tests (not in my file list):
- `crates/aa-rules/tests/native_collision_definition.rs:72`
  `wall_reference_census.sources.len() == 1847` -> 1647 (native index files).
- `crates/aa-rules/tests/phase3_local.rs:41` `actors.len() == 1646` -> 1959
  (Thing/Unit/Projectile/VFX roots registered by the Thing DataManager), and the
  final assertion that `eastern_europe_city_center02_large` is ambiguous no
  longer holds: natively it resolves uniquely from
  `Buildings/Villages/EasternEurope/eeurope_citycenter_02.tgi` (the duplicate
  in the `EasternEuropeCityCenter02/` subdirectory is never loaded).
Please update or let me update them.

## R7. Restoration harness note (no action required by me)

`research/phase8-restoration/native_tgi_parser.py` inherits a 16-byte
template-registry object at [B39BC8] from `probe_lists.Machine`, but its
5BF0D0 seam writes the registry hash map at +24/+28, overwriting the next
fixture allocation. It is harmless for files without templates; my
`native_machine.py` reallocates a full-size registry block. Tell the
restoration agent in case their template cases depend on it.

# Round 2 (2026-10-15)

## R8. Restoration agent: Character view set at saved construction (group a, 794 records)

Producer side is complete and proven:
- Native rule (oracle section `character_view`): Character ctor 563670 takes
  the first animation of 4D0720(owner): Actor+15C stance != -1 -> 45E5E0,
  else Object+40 variant; 45E690 returns ActorData+1BC[index], the last
  (base) set for -1 or a null entry, null when +1C0 == 0. The saved-load
  constructor 4D6D70 sets Actor+15C = -1 and the tail keeps it -1, so a
  restored Character selects by the LOADED Object variant.
- `aa_rules::native::character_view::native_character_view_set(present,
  stance_aspects, -1, variant)` ports that; `ActorCatalog::
  native_character_sets_present(id, pc)` gives the +1BC presence vector
  (verified on cow/donkey/sheep/infantry). The existing
  `aa_runtime::catalog::ActorBindings::ground_for_animation_set(set)` already
  builds the per-set animation keys (its `for_variant` fallback matches 45E690).
Consumer change: `RuntimeDefinition.ground.character_animations` is always
None, so `aa_actors::owned::construct(Character)` fails during restore. In
`aa-mapworld restore.rs` Character construction (and the RestoreDefinitions
trait / `aa-runtime catalog.rs` impl) select the set from the restored Object
variant (`variant == -1` -> last set) and pass `ground_for_animation_set(set)`
instead of the definition-wide ground.

## R9. Restoration agent: Denizen inventory slots (group b, 168 records)

Producer: `ActorCatalog::native_denizen_inventory(id, pc)` (aa-rules
`native::denizen`) returns the ActorData+35C Inventory list in native order
(case-sensitive grouping 4673C6/462E60, nothing merged; targets resolved
through 427BD0 with translations), the +34C Organization behaviors (empty
for every shipped actor) and the Denizen+18 slot list exactly as 56B720
builds it: `count` slots per entry, each PREPENDED, flags 568880(target, 2)
and 568880(target, 0x2C) = false/false when +34C is empty (oracle section
`denizen`). usa_carrier: 6 x usa_fighter_corsair then 2 x usa_bomber.
Consumer: `aa_actors::denizen_state::Definition { inventory_is_empty }`
cannot carry this; replace it with the slot list ({target definition,
flag +C, flag +D}, head first) and build `DenizenState` from it; catalog.rs
mask bit 10 should fill it from `native_denizen_inventory`. A nonempty
Organization list (none in shipped data) stays an explicit producer error.

## R10. Generic DataManagers (c, d) — FYI for terrain/map consumers

`ActorCatalog::native_data_find(manager, id)` / `native_data_registry` now
expose every generic DataManager registry of the native load (47 tags +
Variation from Terrain children), with the 48D500 Find (translation first,
no chaining) and the native duplicate rule per manager mode (+8). All 68
Variation/Biome translations load without native exceptions. Map/terrain
code that resolves saved Variation/Biome IDS should use `native_data_find`
(e.g. tropical_shoredirttrans01 -> tropical_shore01; tropical_river03 -> null).
`native_named_definition` uses these registries automatically.
