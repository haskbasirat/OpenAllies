Phase 8: original map objects -> native definitions (definition registry port)

WHAT CHANGED
The definition catalog used by restoration (ActorCatalog::load_v14) no longer
reads every TGI member in archive order with by-name template lookup. It now
reproduces the original AA.exe data load:

1. Which files, in which order (crates/aa-rules/src/native_index.rs,
   aa_rules::native::index):
   48DB60 enumerates root `index*.LST` (4901DC..49024C, 85BD30 full-path,
   non-recursive) -> 48D970 LoadIndex -> 5AB590 KKC_IndexFileResource::LoadIndex.
   5AB3C0 stores nonempty lines (right-trim CR/LF only, 425310 "\r\n");
   lines whose first unit is ';' are skipped; 858850 strips a leading '/' or
   prefixes the index's directory; a line containing '?' or '*' is enumerated
   through the resource manager (5AF720/5AF7F0 -> 5AEF60): whole-path
   wildcard 859830 AND the per-'/' segment rule (no descent into deeper
   directories); matches are sorted by qsort/859A10 (ordinal UTF-16); a name
   whose FIRST case-insensitive "LST" ends it (8562B0) is loaded as a nested
   index in place (5AB350); everything else is appended without dedup.
   Result on the v14 sources: index_aa.lst, templates.lst, templates_AA.lst,
   mg_resources.lst -> 1,647 data files. 200 TGI members are never loaded
   by this chain: UI/, MetaGame/UI/, Templates/UI/ (UI lists), Maps/ (map
   string tables), root AVars*/UVars* (var files), Cutscene/cutscene.tgi
   and the three Buildings/Villages/EasternEurope/<subdir>/*.tgi files.

2. Parse-time template binding (crates/aa-rules/src/native_load.rs,
   aa_rules::actors::native_load): the global template registry ([B39BC8]) is
   simulated file by file. A `Template=`/`inherit=` reference binds to the
   declaration registered when the node is read (5BF0A0 -> 4247C0,
   case-insensitive on the node's CURRENT name). Forward references fail
   ("Template definition not found."), a second registration of a live name
   fails ("Template was already defined.", 5BACE4/5BCA42 -> 5BF0D0). Sibling
   name collisions rename both nodes to `?%07d_<name>` (5B5C80/42BC40), which
   makes an already registered template unreachable. The bound declaration is
   stored on the node (Definition::bound) and the resolver follows it.

3. Root processing and file lifetime: 48D660 visits non-template roots in
   order and dispatches by name (renamed roots by their label) through the
   GameData+834 handler map (48DB60 tags + the MetaGameData map merged at
   49006C..4900BD). Any TGC exception is caught by 48D660's own catch funclets
   (48D824/48D886/48D8F0, FuncInfo AE0BC0) and the REST OF THAT FILE is not
   processed. 48D970 releases a file whose 48D660 returned true (>=1
   non-template root, no exception): 5AA550 -> ... -> 5BCDD0 Unload -> 5B3FE0
   -> 5BF080 unregisters every template the file declared. Template-only
   files and files that threw stay loaded (their templates stay live).

4. Thing registry: roots Thing/Unit/Projectile/VFX go to the Thing
   DataManager (ctor 4BD9C0, vtable A21260, global [B39F58]). 4BDC50 reads IDS
   and category (48D500; unknown -> "Unknown thing category"); 4BCA00 throws
   "ThingData name already used: %s" for a registered IDS, so the FIRST
   declaration wins and the rest of that file is abandoned.
   NativeThingRegistry ports 4BCA00/427BD0/48D2D0 for the Thing manager.

5. Translations: translate.tgi (last index line) Translate roots go to
   48D2D0: DataManager by `node` (case-insensitive; unknown -> "does not
   exist", vf8 must be true), duplicate ids_old throws before insertion, the
   pair is inserted (478E10), then the target is checked with the manager's
   Find (Thing: vf18 4BDD90 -> 427BD0) and a missing target throws AFTER
   insertion. 427BD0: empty IDS -> null; a translated IDS is looked up by its
   target only (no fallback). ActorCatalog::find applies this, so for example
   `eastern_europe_city02` -> eeurope_cityblock12x12_02 and the registered
   `chaparral_wall` is hidden by its translation to temperate_walls.
   The original parser comments out translate.tgi lines 1292..5310 (a `/*`
   at 1292 closes at the `*/` on 5310; non-nested), leaving 182 Translate
   roots, 114 of them for Thing; this is why temperate_dynamic_hedge,
   tropical_bamboostand, desert_palmtrees, ... stay missing natively.

6. ActorCatalog::native_named_definition(kind, id, pc) returns Ok(None) for
   an IDS with no declaration, as the generic DataManager Find 48D500 returns
   null (used by the Kingdom resource reader 4CDBB0/4CDC70).

On the shipped v14 data the Rust model predicts zero native exceptions
(no parse error, no template error, no duplicate Thing, no translation error,
no unknown root tag). This matches the recorded native v14 runs: every
48D660 catch logs "KKC_Data::LoadDataFile" at level 0 on channel B38B70
(404EA0), the channel that records the level-10 "loading index" line, and no
such line exists in reconstruction/native-game/qa/expanded-runtime-log-0-ok.log
or Logs/log-2*.log.

INVENTORY (crates/aa-runtime/tests/restore_inventory.rs, InspectPartial,
Double53, 80 maps; counted by inventory_groups.py over the raw outputs).
"Before" is the inventory at the start of this round (18,389 records, as in
the brief); research/phase8-restoration/unsupported-inventory*.json was
regenerated by the restoration agent at 21:10 with this loader in place and
now equals "after". Final rerun 2026-10-14 ~21:30: identical counts.
  group                                         before (recs/maps)  after
  1 source class # differs from definition       7,781 / 27     7,879 / 27
  2 missing definition <id>                      4,751 / 47     3,912 / 39
  3 Resource IDS ipcs has # matching decls         106 / 32       106 / 32
  4 ambiguous template ...Bombard                   41 / 13         0 / 0
  5 ambiguous Thing IDS ...city_center#_small       27 / 12         0 / 0
  6 Construction without Economy data                1 / 1          1 / 1
  total failed records                          18,389         16,486
  maps without failures                              0              0
Group 1 grew by 98 because translated IDs now resolve to their (class 9)
definitions. Groups 1/2/3/6 now carry the CORRECT native definition; what
remains is restoration/adapter behaviour outside my files (requests R2..R5).
Records that now resolve proceed to later stops (e.g. "Actor name
collision/action-table validation is unavailable" 1 -> 213).

PER GROUP
1. Definitions correct (DynamicDoodad class 9 for the trees, verified by the
   native categories table). AA.exe converts saved class-2 Objects whose
   definition class changed: 4FAE80 -> 4F26F0/405B80/4EA050/4F9840, then
   4FC600 4FD47C..4FD781 creates them fresh via 4F9C10 with the saved ID,
   position, direction and owner (log load_warning_replaced_changed_gclass /
   load_warning_cant_replace_changed_gclass). Blocker: fresh-creation port in
   aa-mapworld (request R3).
2. Resolved natively: 839 records via the Thing translation map. Remaining
   3,912: the native registry has no such IDS; AA.exe logs
   load_warning_unknown_thing_ids|s and skips the record (4FAE80 null ->
   4FC340 858F50 seek). Blocker: restore.rs skip (request R2).
3. Native: unknown resource IDS is skipped with its value consumed and a
   warning flag (4CDC70 catch funclet 4CDD04; oracle section `kingdom`).
   Definition API provided (native_named_definition). Blocker: catalog.rs
   adapter still uses the erroring resolved_definition (request R4).
4. Resolved natively (0 records): usa_battleship.tgi and
   hires_usa_battleship_texas.tgi each declare AAUSAAttackBattleShipBombard;
   the first file is released before the second is parsed, so each file's
   references bind to its own declaration.
5. Resolved natively (0 records): the duplicate declaration lives in
   Buildings/Villages/EasternEurope/EasternEuropeCityCenter02/, which the
   non-recursive index never loads.
6. Native: Construction cost entries are built only when the ActorData mask
   +174 has bit 9 (565FA0 -> 565F20 -> 5657B0); none otherwise (oracle
   section `construction`). Blocker: construction.rs (request R5).

EVIDENCE
native_definition_registry.py -> native-definition-registry.json (original
AA.exe under Unicorn; every executed window pinned and checked identical in
the v14 executable). Sections and seams:
  wildcard     450 859830 cases.
  find         18 5AF720/5AF7F0/5AEF60/859B70 cases (fixture resource
               manager map with one resource per case).
  sort         132 index patterns, 2,389 real names through 96CE20 qsort +
               859A10 + 85BE40.
  lookup       4 427BD0 scenarios (registry entries fixture-inserted,
               translations inserted by the original 478E10).
  translate    6 48D2D0 scenarios on Translate nodes from the original parser,
               original Thing-manager vtable A21260; a throw stops the case.
  dispatch     6 48D660 scenarios with fixture handler stubs; handler
               exceptions are raised by the original 410C40; C++ EH dispatch
               seam runs the original catch funclet (inner-frame destructors
               skipped).
  templates    14 parser/registry scenarios incl. release via the original
               5BCDD0 Unload (pool free 85C2B0 is a leaf).
  thing        4 4BCA00 scenarios (native kind 0, empty fixture maps).
  kingdom      4 4CDC70 scenarios (stream read leaf, Resource manager in the
               generic 48D500 layout, EH dispatch seam to 4CDD04).
  construction 4 5657B0 masks without bit 9.
  translate_file  original parser over the shipped translate.tgi: 182 roots.
Common seams (native_machine.py): restoration-agent parser harness (line
source, heap, pool mutex, empty registry buckets), widened fixture heap,
logging calls 404EA0..404F90 are recorded leaves, KKC_Resource path
formatter 5AB1D0 and node-pool release 85C2B0 are leaves, template
registry object reallocated full-size. Thing/Resource DataManagers in the
lookup/translate/thing/kingdom sections are built by the original
constructor 4BD9C0 (argument 0x200 as at 48A95C).

EMULATOR CAVEATS (docs/NATIVE-PORT-RULES.md) CHECKED
- FS base 0: page-0 accesses fault unless FS-prefixed and within [0..3]
  (hook + check_null() after every call/parse). This exposed one silent
  null read in the first `thing` run (4BCF71, per-category list +5C of a
  hand-built manager); the manager is now built by 4BD9C0 and every
  section reruns with zero page-0 violations. All case results were
  identical before and after the fix.
- Allocator seams: 6F7810/CRT heap/85C680/85C2B0 are seams; every case runs
  to its stop address or to a recorded throw; no fault is accepted.
- x87: only f32 copies of normal values (kingdom); no transcendental,
  no NaN inputs.
- No code is written into emulated memory (stubs are hook-implemented at
  fresh addresses), so stale translations cannot occur.

Static (not executed) evidence: 48D970 keep/release branch; 48DB60 tag table
and MetaGame merge; 5AB3C0/5AB590 line handling; 4FAE80/4FC600 conversion
path; 4CDC70/48D660 FuncInfo try maps (parsed by xref.py funcinfo).

TESTS
  CARGO_TARGET_DIR=target/s-definitions cargo test -p aa-rules --release
    --locked --offline -j 4 --test native_definition_registry
    -> 10 passed (replays every fixture section above).
  ... --test native_definition_load -- --ignored --nocapture
    -> real data: 1,647 files, 1,959 Thing-manager registrations, 182
       translations, 0 predicted native exceptions; translate_file roots
       match the Rust load in order; translated lookups resolve.
  AA_GAME_ROOT=... AA_RESTORE_INVENTORY_OUT=... cargo test -p aa-runtime
    --release --locked --offline -j 4 --test restore_inventory -- --ignored
    -> inventory-after.raw.json (counts above).

PROVEN / COMPILED / UNTESTED
Proven by oracle: wildcard matcher, segment rule, enumeration order, 427BD0,
48D2D0 (Thing manager), 48D660 abandon-rest-of-file and return value, parser
template rules and Unload unregistration, 4BCA00 first-wins, 4CDC70 skip,
5657B0 no-Economy guard, shipped translate.tgi roots.
Compiled and exercised on real data only: whole index expansion (composition
of proven pieces + static line rules), the full native-order catalog.
Untested / not modeled: per-category LoadData validation exceptions
(4BB310, 49DC40, 464BD0, ...; absent per native logs), non-Thing translation
managers' registries (68 Variation/Biome/Faction/Weather pairs recorded as
UnmodeledTranslation; no error in native logs), generic DataManager
re-LoadData of duplicate non-Thing IDS (4BD1B0 -> 47D960), dangling registry
entries of renamed templates in released files (never occurs in shipped data).

REMOVED ENGINE CAP
5AB3C0 reads index lines through a 0x800-unit buffer (857210 arg 0x800);
native_index has no line-length limit (original lines are far shorter).
A cyclic .lst chain overflows the original stack; the port reports it.

FILES
crates/aa-rules/src/native_index.rs (new), native_load.rs (new),
resolved.rs (module + native_named_definition), native.rs (module line),
actors.rs (minimal hook; see requests-to-lead.md R1), tests
native_definition_registry.rs and native_definition_load.rs (new).
research/phase8-definitions: README.txt, findings.json, requests-to-lead.md,
native_definition_registry.py, native_machine.py, xref.py,
native-definition-registry.json, native-load-report.json,
inventory-after.raw.json, inventory_groups.py, actors.rs.before-native-load.txt.

==========================================================================
ROUND 2 (2026-10-15): producer-side resolution for groups (a) and (b),
generic DataManager translations (c) and duplicate handling (d)
==========================================================================

(a) "component Character requires completed source input resolved
    Character view animations" (794 records / 32 maps, first: cow)
  Native: Character ctor 563670 binds the first animation of the list that
  4D0720 returns: Actor+15C stance != -1 -> 45E5E0 (ActorData+188 stance ->
  +8 -> +8 aspect index), else Object+40 variant; 45E690 returns
  ActorData+1BC[index], the last (base) set for -1 or a null entry, null when
  +1C0 == 0. The saved-load ctor 4D6D70 sets Actor+15C = -1 (the actor tail
  keeps -1), so a restored Character selects by the loaded Object variant.
  Producer: aa_rules::native::character_view::native_character_view_set and
  ActorCatalog::native_character_sets_present (ActorData+1BC presence:
  creation aspects with an AspectAnimationSet, then the base set). The
  per-set animation keys already exist in aa-runtime catalog.rs
  (ActorBindings::ground_for_animation_set). Oracle section character_view
  (40 rows; out-of-range indices read past the native array and are
  explicit Rust errors). Blocker: consumer (R8) — restore passes the
  definition-wide ground whose character_animations is None.

(b) "Denizen Inventory requires resolved target definitions and behavior
    flags" (168 records / 40 maps, first: usa_carrier)
  Native: ActorData loader 464BD0 Inventory children (467350..4674C7): a
  case-sensitive wcscmp search; repeated actor_ids become a NEW entry after
  the equal run (462E60), nothing merges; count via 5B4ED0; target is a
  deferred Thing reference (461770). Organization children (467165..)
  fill +34C {organization_ids, behavior flags 463520: worker 2,
  militia_offensive 4, militia_defensive 8, air_offensive 10, air_defensive
  20, air_recon 40, air_escort 80}. Denizen ctor 56B720 prepends `count`
  slots per +35C entry with flags 568880(target, 2) and 568880(target, 2C);
  568880 needs target Body+Properties and an owner +34C organization whose
  layout accepts the target (4C1D60/4C1CF0/45EEF0). No shipped actor has an
  Organization child, so every slot's flags are 0 (oracle section denizen).
  Producer: ActorCatalog::native_denizen_inventory (aa_rules::native::
  denizen). Blocker: consumer (R9) — denizen_state::Definition only carries
  inventory_is_empty.

(c) The 68 non-Thing translations (67 Variation, 1 Biome)
  48D2D0 reaches generic DataManagers whose vf8 is 6A71D0 (true) and vf18 a
  48D500-family Find (static join tag-managers.json over the GameData ctor
  48A8E0 and the 48DB60 tag table). The native load now builds every
  generic registry: 47 root tags plus Variation from Terrain children
  (TerrainData::LoadData 4B87A0 -> [GameData+658] vf1C). 410 Variations,
  8 Biomes; all 68 translations load with zero native exceptions (the
  "tropical_river03 -> tropical_river02" target check passes because Find
  translates river02 -> river01; a Find of river03 itself returns null —
  no chaining). API: ActorCatalog::native_data_find / native_data_registry;
  native_named_definition uses the registries automatically. Oracle: lookup
  section now runs 48D500 beside 427BD0 (identical results).

(d) Generic DataManager duplicate IDS
  vf1C (47D030, 479080, ... same shape as 4BD1B0) depends on the manager
  mode word +8 (47D880's argument; GameData ctor passes 0 for most, 1 for
  KingdomColor/GameType/Cursor, 2 for ActorCommand/ActorState/
  TriggerElement/MessageType/GameEvent):
    mode 0: no lookup; the create path throws "Data IDS already used" ->
            first wins, rest of file skipped;
    mode 1: reuse + re-LoadData; KKC_Data base 47D960 throws "loading
            duplicate node with ids" when the stored name (localized text;
            unquoted null is the text "null") is nonempty, else the object
            is re-loaded (overlay; reported, field merge not modeled);
    mode 2: reuse; a new IDS throws "Unknown data cannot be added" (only
            code-predefined entries; the Rust model records
            PredefinedAssumed for the 297 shipped roots of these managers).
  Oracle section generic: 15 cases (5 scenarios x modes 0/1/2) on the
  original DataManager<ThingData> path 483A40 -> 4BD1B0 -> 4BD270 ->
  4BB310 -> 47D960. Shipped data has no duplicate non-Thing IDS among
  natively loaded roots and Variations.

Also fixed: the original section-header scan ignores stray '=' tokens
("[DenizenType = Template=DenizenType]", "[A =]"), while a bare word stays
"Malformed section name." (oracle section headers); actors.rs definition()
now follows it, so the two DenizenType roots load (DenizenType registry 2).

Round-2 evidence sections in native-definition-registry.json: lookup
(+48D500), generic, headers, denizen, character_view. Rust replay:
crates/aa-rules/tests/native_definition_registry.rs (14 tests) and the
real-data checks in native_definition_load.rs.

ROUND-2 INVENTORY (inventory-round2.raw.json, 2026-10-15, InspectPartial,
Double53, 80 maps; the restoration agent had meanwhile applied R2/R4/R5):
  group                                         round-1 after   round-2
  1 source class # differs (conversion, R3)      7,879 / 27     7,879 / 27
  2 missing definition <id>                      3,912 / 39         1 / 1
  3 Resource IDS ipcs                              106 / 32         0 / 0
  4 ambiguous template                               0              0
  5 ambiguous Thing IDS                              0              0
  6 Construction without Economy                    1 / 1          0 / 0
  a Character view animations (R8)                 794 / 32       901 / 32
  b Denizen Inventory (R9)                         168 / 40       168 / 40
  total failed records                          16,486         11,602
  maps without failures                              0              1
Group (a) grew because records skipped as missing before now reach the
Character stage. The single remaining group-2 record is a Kingdom-stage
reference (DeveloperMaps/(4v4)NorthernFrance, kingdom saved ID 5) to
regiment_usa_tank_destroyer, which has no native definition (its only
translations sit inside the commented translate.tgi block); its native
handling belongs to the Kingdom loader (restoration).
