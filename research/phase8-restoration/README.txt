PHASE8 SAVED-WORLD RESTORATION (IN PROGRESS) — 2026-10-14

Goal: original maps load completely, exactly as AA.exe loads them. This round
moved both tutorial maps past the Turret stop through eight more native
component ports, and wrote the all-maps inventory. Restoration is still NOT
complete: runtime_publication_ready stays false and tutorial01/02 now stop at
the saved Element Mind (slot 4) of the first corps.

---------------------------------------------------------------------------
ALL-MAPS INVENTORY (InspectPartial)
---------------------------------------------------------------------------
crates/aa-runtime/tests/restore_inventory.rs (ignored; env AA_GAME_ROOT,
optional AA_RESTORE_INVENTORY_OUT) restores all 80 original maps with
RestoreMode::InspectPartial (Attempt::load_with_mode, 4 map threads; maps
are independent, records stay in native order) and records every failed
record. build_inventory.py annotates it:
  unsupported-inventory.raw.json  per map: every failure (stage, ordinal,
                                  saved ID, definition, class, error)
  unsupported-inventory.json      groups by normalized error with component,
                                  Rust site, native addresses, first
                                  occurrence, top definitions; first stop per
                                  map
InspectPartial continues after failures, so later errors can be cascades.
The definition-resolution groups (class mismatch, missing definitions,
Resource ipcs, ambiguous template/Thing IDS, Construction without Economy)
moved to the definitions agent. Native facts for them recorded in the JSON:
4FC340 loops DYNAMIC_THINGS and seeks past a record that yields null; 4FAE80
logs and SKIPS a record whose definition is missing (427BD0 null), and
converts or skips a class mismatch (saved class 2 with caller context goes
through 4F26F0/4EA050/4F9840) — native does not abort the map for these.

---------------------------------------------------------------------------
PORTED THIS ROUND (all oracle-backed: Unicorn runs of the original x86)
---------------------------------------------------------------------------
1 ActorData Turret definitions — aa-rules/src/turrets.rs
  464BD0 Turret children -> 4C7660 ctor + 4C7820 LoadData (validation order,
  NaN passes native checks, every x87 store), 45DC30 (+4 read by 57F4D0),
  45DD50 (Ability turret_ids -> index, 4BF990 throw on -1), Bone registry
  (b36530+580, "Bone" domain) lookup 42C070.
  B279C0/B279C4: no instruction writes either absolute address (whole-image
  capstone scan); initialized .data zero -> NATIVE_ZERO_VECTOR_B279C0.
  FCOS field (+10): exact Q8.120 cosine, binary32 rounding, refusal window
  for hardware ambiguity; host x87 cross-check 760,007 inputs x 3 PC modes,
  0 refused, 0 mismatches.
  Fixture native-turrets.json: 2022 cases (1890 loaded bit-exact, 63 native
  data errors with identical read order, 69 NaN inputs refused by design).
  Tests: aa-rules/tests/native_turrets.rs.
  Performer constructor now takes the actual turret list (actor_world.rs
  performer_inputs); Ability loader binds turret_index (ability_load.rs).

2 Construction command contributor — aa-actors/src/construction_actions.rs
  565140 + 565090, GActor virtuals +34 4D1F90, +100 4CED60, +158 4CE980/
  588990, +15C 4CE9A0, 4CEDA0 settlement walk, 4D14C0/4D0820 group lookup.
  Fixture native-construction-actions.json: 640 cases, all bit-exact.
  Saved-factory 4D1350 now runs it, plus 4CFCC0 publication
  (action_publication::changed) with the owner word E8, Object+4C and
  Thing+C from the load shell (actor_world.rs ConstructionActions::Saved).
  can_sell/can_raze (+4E4/+4E5, raze cleared by sell, 467C16..467C63) and
  outgoing Upgrade count +4C0 (0 by the source census): aa-rules
  native_construction_flags. AllowSellUnderSiege: settings.rs (two-file Bool).

3 Supply (slot 11) — aa-actors/src/supply_state.rs
  ctor 58AA50 (supply_range ActorData+420 via 45D550(0)), owner Kingdom
  supply-source list 4E1F00 (+264 head insert, +268 tail) applied through
  restore OwnerEffect in construction order, Load 58ABD0 (relay stockpile =
  saved multiplier * stockpile_max +428). aa-rules native_supply_fields.
  Fixture native-supply.json: 540 cases.

4 Recruit (16) 581E40/583C80 (legacy v1/v2 queue records; object flag set
  is rejected), MobileDeployment (19) 575940/575890, Research (21)
  584210/5849E0 (45EE00 lookup; the Property+54 FX list 42A590 is recorded in
  RestoreReport.presentation_effects, not executed), Lair (25) 574FA0/
  5751F0, City (29) 564160/564CE0 (4D1AF0 list; ActorData+5B4 zone_radius
  must be > 0 or the whole map load is canceled through 5A0910).
  Fixtures native-recruit.json (28), native-mobile-deployment.json (18),
  native-research.json (17), native-lair-city.json (24).

5 Defender (slot 20) — aa-actors/src/defender_state.rs
  ctor 567CD0 (one 4C8650 Ability per ActorData+21C entry, shared arena),
  Load 567020 (records into min(i, n-1), +24 entry list: Ability+14 below
  version 2, saved pairs at version 2). aa-rules native_loaded_defender
  (464BD0 4657A0..465937: control_range > 0, SPELL required, no BOMBARD,
  no animations). Fixture native-defender.json: 51 cases.

6 Component requirement virtuals (+20) for the new slots: Defender ->
  Performer (6B2450), Supply/Recruit/MobileDeployment/Research/Lair/City ->
  none (580F70). actor_finish.rs.

---------------------------------------------------------------------------
LEAD DATA/LOADER ITEMS
---------------------------------------------------------------------------
* TGI parser (aa-rules/src/tgi.rs): native delayed body (an unbraced node's
  properties followed by `{` continue that node; `}` returns to the parent)
  and the 5BDF80 value-quote rule (only a value whose first char is `"` is
  quoted, the quote ends at the next `"` on the line, later quotes are text),
  plus the parse-time 5BEE60/string/localized declaration checks. Oracle:
  native_tgi_parser.py runs the whole original TGC_DataFileParser (5BD080,
  handler table 5BCC80, lexer table 5BEFD0) — native-tgi-parser.json, 44
  snippet cases (both shipped constructs verbatim). All 1847 v14 TGI files
  now parse (was 1845); usa_carrier becomes a definition.
* ResolvedNode::localized (resolved.rs, before it moved): stored native text
  of a `localized` field = all `"` removed, one leading `@` dropped (5BDC00,
  5BA3C0); keys are not resolved (the getter does that natively).
  tgi::native_localized_text. 494000 markup substitution is inactive when
  registry b36530+3BC is empty (seam, documented).
* FactionData experience_max: declared int -> 5B70B0 fild + fstp dword;
  CommanderConstructorInputs.experience_max_bits; restore uses
  commander::replace_with_maximum (commander_restore.rs, special_state.rs,
  catalog.rs). Shipped value 5000 for the tested commanders.
* 4D49B0 / 4D64F0: already ported as aa_actors::state::{group, set} with the
  phase7-actors/actor_states.py oracle (native-actor-states.json); combat
  can call them directly.
* Recruit-singles (+214) and terrain_speed.rs: NOT done this round (see
  requests-to-lead.md). No restoration record reached nonempty Kingdom
  recruitment in this inventory.

---------------------------------------------------------------------------
CURRENT STOPS (StopOnError)
---------------------------------------------------------------------------
tutorial01: 134 registered; DynamicThings ordinal 130, saved ID 1220
  (rus_corps): "unported saved component slot 4" — Element Mind saved Load
  536340 (base Mind Load 52C580). Mind belongs to aa-mind (other owner).
tutorial02: 437 registered; ordinal 433, saved ID 4097 (usa_corps): same.
Remaining InspectPartial failures (latest inventory): tutorial01 12 records
  (Mind slot 4: rus_corps, ger_corps, 3 depots; Organization slot 7:
  4 regiments + 3 owl flocks); tutorial02 8 records (Mind: 2 corps + depot;
  Organization: 2 regiments; usa_battleship Ability effect kinds (combat
  agent); usa_airfield nonempty Denizen Inventory (56B720); city center
  Actor name collision/action-table validation (actor_finish.rs)).
  Organization component Load 57D130 and Organization/Aircraft Minds
  528C20/51C3C0 are unported.
runtime_publication_ready=false; PostLoad/PostExisting/publication (HANDOFF
steps 4-5) not started — they need every record restored first.

Commands (PowerShell or bash, AA_GAME_ROOT='E:\Games\Axis and Alies'):
  CARGO_TARGET_DIR=target/s-restore cargo test -p aa-runtime --release --locked --offline -j 4 --test catalog_restoration real_infantry_definitions_and_tutorial_restore_share_owned_state -- --ignored --nocapture
  ... --test restore_inventory -- --ignored --nocapture   (inventory)
  cargo test -p aa-rules / aa-actors / aa-mapworld --release --locked --offline -j 4 -- --include-ignored
  Python oracles: PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
    python research/phase8-restoration/native_<name>.py
    (native_tgi_parser.py --fixture regenerates the parser fixture)

Emulator caveats (docs/NATIVE-PORT-RULES.md): every phase8 harness now
installs null_guard.py (page-0 access faults unless FS-prefixed); the TGI
parser harness got explicit B39940+340 / B36530+3BC seams instead of null
reads. Re-runs: construction-actions, supply, recruit, mobile-deployment,
defender, lair-city and tgi-parser case data unchanged; research changed in
one case (unregistered Property: now "native null dereference" — the native
access violation — instead of a not-found message produced by the emulator's
zero page; Rust still reports an error). Turrets: 51 sampled cases re-run
with the guard, identical (full 18-minute regeneration not repeated). All
allocations go through 6F7810/CRT malloc seams; no SNaN inputs are used.

City zone finding: 18 records in 2 developer maps stop with the native
"City no longer has a city zone" map-load cancel (564CE0 -> 5A0910): their
saved IDs translate to non-center village blocks whose template zone_radius
is 0. This is the original game's behaviour, not a port gap.

Inventory after this round: 16,486 failed records (was 18,389); groups in
unsupported-inventory.json. Ownership changes during the round: definitions
agent owns aa-runtime definitions.rs and aa-rules resolved/sources/objects/
doodads/native_categories/native.rs; combat agent owns aa-rules abilities.rs
and ability_load.rs (my edits there — turret binding, Defender loader — were
finished before the hand-over).

Not claimed: physical-x87 parity of anything except the Turret FCOS field
(host cross-check); Unicorn FCOS/FSIN/FPATAN are binary64-based. Fixtures use
PC53 except where cases enumerate PC24/53/64 (Turret, Supply).
No original game file, save, profile or release was modified.

===========================================================================
ROUND 2 (2026-10-15)
===========================================================================
Mind saved Load (slot 4) — aa-mind/src/saved_load.rs (new)
  Element Mind 536340 with base Mind 52C580 (Updatable 50BCD0, version <= 3,
  +24/+28, signed command count, +2C, state-name stack resolved through
  Element virtual +19C = 531C30 in list order 48,68,7C,F8,10C,148,12C,164,
  178,188 (uai_unload is NOT in the list), names compared as NUL-terminated
  wide strings, an empty name becomes "cai_rampage" (_wcsicmp vs L""), no
  match -> 5A0910 "Map Load Canceled"; current-command flag at v3) and the
  owned path at +D4 (52D2C0: version 0, records 8+8+4+4(+4 discarded)+8
  pushed to the FRONT by 52D000, kind 1/2 counts, dirty byte, +18 word).
  All Element version gates 0..14 (legacy discards, the v<10 +A8==2 fix-up).
  Decode is pure; apply validates every unported branch before mutating.
  Unported (explicit errors, absent from all 80 maps): saved command objects
  (4D9C80 Load), a nonempty path (needs a typed owner in movement_world),
  nonempty uai_bombard text, Regiment 528C20 and Aircraft 51C3C0.
  Oracle: native_mind_load.py -> native-mind-load.json (2,340 cases: all
  2,194 distinct saved Mind payloads of the 80 maps — every one is an
  Element Mind, version 10/12/14, stack [uai_sleep] — plus synthetic
  branches incl. 5 cancels, 5 bad versions, 1 truncation; constructor
  535BA0 then 536340, 530E50 jitter seamed). Replay:
  aa-mind/tests/saved_load_native.rs compares every constructed word/byte,
  every embedded state status, the Mind core words, stack, path nodes, text
  and the Updatable id with the native object image (a deliberate store
  swap makes it fail).
  Restore plumbing: RestoredComponent's Mind type is RestoredMind
  {required (Mind virtual +20: Element 5B6830 -> Element, Regiment 7C53E0 ->
  itself, Aircraft 55EA50 -> 0x17), state}; the catalog constructs (4CF480
  case 4 precedence, Element RNG draw + Performer ability jitter in the
  restore Ability arena), decodes and loads; staging moves the Mind core and
  states into ActorStore.minds and continues identities after the highest.

R2 (missing definitions) — restore.rs: 4FAE80's null results before
  construction (generation check, 427BD0 Thing lookup, class mismatch for a
  non-convertible record) skip the record with SkipCause and the native
  warning key (load_warning_unknown_thing_ids|s, load_warning_changed_gclass|s);
  report.skipped. 3,912 records skipped across the 80 maps.

Organization (slot 7) / Transport (slot 24) wiring
  Transport: Component::Transport, 58B9A0 constructor (Resource count),
  58C810 Load (decoded once on a constructor copy for its length, then
  loaded on the real component), requirement Mover (7C7840).
  Organization: 579BD0 constructed at its stage, then 57D130 runs on it
  (organization_restore.rs). Records load re-entrantly: the record loader
  takes the owner out of the registry (RestoredRecord::Loading) during its
  Load, embedded members (4FC200 -> 4FAE80(stream, &out, owner, 0)) are
  decoded from the remaining stream (inventory::decode_embedded_record),
  registered, loaded with Actor+138 = owner, and report their consumed
  bytes. OrganizationWorld binds Updatable registration, member facts,
  diagnostics, owner kingdom, Body 561760 / experience 4CED10 / rank
  4CEBB0 (saved-components agent's organization_callees); every other
  callee is an explicit "unported" error. All 1,067 Organization records
  stop at organization_layout_info (LayoutData producer missing).
  GWorld+2C is tracked per restore (upgraded 14 -> 15 by Actor version > 7).

R8 (Character) — construction picks ActorData+1BC[set] from the LOADED
  Object variant (native_character_view_set, stance -1) and the kind-0
  array's first animation (563670 -> 4DAEF0), and the saved Load 563930 is
  ported (animation (kind, index) through the per-kind arrays, Animator Load
  4DAFC0, the not-found branch 404F60 + 4DB120 with GWorld+E0, +2C at v2,
  42A590 on animation+38 recorded as presentation).
Denizen switch-over (R2-1): Definitions.denizen = 56B720 slot list and
  target health map; DenizenPayload::apply applies occupants; the legacy
  Definition { inventory_is_empty } is removed.
Kingdom economy load: Attempt/Staged.kingdom_economy per Kingdom
  (Statistics::load_saved into +330, RelationStatistics::load_saved into
  +344 — empty tables only —, loaded_property_records); start_world is for
  the world-start step.
Map triggers: Attempt::load decodes TRIGGERS with load_map_triggers and the
  LanguageIDS AVar (Settings.app_variables from AVars*.tgi; "English").
Fresh creation entry points (aa-actors/src/world_creation.rs):
  create (4F9C10 CreateGThing): instance-name check, class dispatch and
  sizes, GACD/GKCD checks, actor mode/state/Body/Organization-layout steps
  with the construction assertion latch, E4/E8/EC, 4F9510, 50C220, B0 ->
  42A590, destroy on failure; oracle native_create_gthing.py (123 cases).
  spawn_member (4FBF40): nearest related recruit structure by binary32
  squared distance (x87 at PC), 4FACA0 + 57D860, else the slot position,
  578290 and 57D950; oracle native_spawn_member.py (240 cases, PC24/53/64,
  238 bit-exact, 2 refused for non-finite operands). The callees are the
  CreateWorld / SpawnWorld traits; binding them to ActorStore (InitCreate
  4D6F10 World, InitExisting 4D7230, registration 4F9510) is next.

Inventory (80 maps, InspectPartial, PC53): 11,580 failed records (round 1
  end: 16,486), 3,912 skipped natively, 1 clean map. Groups:
    7,879 class-changed records (R3, conversion agent)
    1,308 detection-memory refresh 4D67F0 (zones)
    1,067 Organization LayoutData producer
      861 Actor saved current-component pointer
      273 Actor name collision (saved-components agent)
      110 saved slot absent from the current definition (4D8290 -> 858A00)
       40 Builder factory; 18 City cancel (native); 15 strategic entries;
        8 MobileDeployment v2; 1 embedded missing definition.
  Tutorials 01/02 stop at the Organization LayoutData producer.

Commands (AA_GAME_ROOT='E:\Games\Axis and Alies'):
  cargo test -p aa-mind --test saved_load_native
  cargo test -p aa-actors --test native_world_creation --test native_spawn_member
  cargo test -p aa-mapworld ; cargo test -p aa-actors
  cargo test -p aa-runtime --test catalog_restoration --test restore_inventory -- --ignored
  python research/phase8-restoration/native_mind_load.py <mind_corpus.jsonl>
    (corpus from aa-runtime/tests/saved_payload_probe.rs dump_saved_mind_corpus)
  python research/phase8-restoration/native_create_gthing.py
  python research/phase8-restoration/native_spawn_member.py

Round 2, final state (supersedes the inventory numbers above)
  Conversion (R3, conversion agent's class_conversion.rs) is wired at the
  4FAFA8 class compare: one ConversionQueue per load in
  RestoreReport::conversions, converted records (7,879) are reported in
  RestoreReport::converted and not registered; null_record_cause uses
  class_conversion::check_class. The replacement step
  (class_conversion::replace_converted after PostLoad) is not reached yet:
  PostLoad/publication is not implemented and fresh creation 4F9C10 /
  4FBF40 / 4DCB30 moved to the conversion agent. Removed native cap
  (C3): 4FC340 reads a section index into a fixed 0x40004-byte block (more
  than 65,536 entries overflow natively); the inventory decoder keeps the
  index in growable storage.
  Name tail (R3-1): finish_unlinked_actor runs aa_actors::actor_names::finish
  (+158 required object, MapString key/text, numbered names) with one
  NumberedNameCounters per Kingdom (RestoreReport::numbered_names, kept for
  owner changes). ActorData+10 GetLocalized("name") has no producer yet
  (RestoreDefinitions::actor_display_name is an explicit error), so the 273
  actors with a saved name key stop there; 59E520 is false for RMP maps
  (MAPSTRINGS is external).
  Inventory: 3,701 failed records, 7,879 converted, 3,912 skipped, 5 clean
  maps of 80:
    1,308 detection-memory refresh 4D67F0 (zones agent)
    1,067 Organization LayoutData producer
      861 Actor saved current-component pointer
      273 ActorData+10 display name producer
      110 saved slot absent from the current definition
       40 Builder factory, 18 City cancel (native), 15 strategic entries,
        8 MobileDeployment v2, 1 embedded missing definition.

===========================================================================
ROUND 3 (2026-10-15)
===========================================================================
4FAE80 Load catch (FuncInfo AE6F98, try states 13..15 around virtual D8):
  TGC_LoadWarningException (every 41DFB0 throw: 858A00 "File Warning",
  858AD0 "Fatal File Error", 8589B0 "Bad data version", "Unknown IDS") and
  TGC_LoadCancelException (5A0910 "Map Load Canceled") thrown by a record's
  Load are caught: warning `load_warning_thing_init_failed_for_reason|sds`
  (5A1040; none for a cancel), 4F5490 destroy, null, next index entry.
  4F0110 (TGC_LoadingException) is not caught. restore.rs destroy_caught
  undoes the record's Updatable table entries (50B120), its arena Abilities,
  its deferred Supply source (dtor 58A990 -> 4E48B0/4E38B0 removes it) and
  its GTID. Embedded/nested drops stay explicit errors. 128 records: 110
  "Component %d removed from actor" (4D831C -> 858A00) and 18 City "no
  longer has a city zone" cancels — the latter is a record drop, not a
  whole-map cancel as round 1 stated.
4D67F0 during Load: Thing+8 bit 29, 4D10A0 (state bits 25/26/34/36) and
  bit 10 decide; for the querying case the expiry +0.0 entries are always
  discarded by the InitExisting refresh (4D7230 -> 4D67F0) that every
  surviving record runs, because the +110 list is not saved and GWorld+E0
  is >= 0; a negative/NaN world time stays an explicit error.
Actor +E4 (4D9271): current component restored as ActiveComponent::Slot.
Builder (slot 18): ctor 562F10, Load 562ED0, requirement 7C53E0 (Mind).
Component requirement failure (4CFB00 -> 858AD0) is a caught Load warning.
MobileDeployment v2: temporary ResourceVector read and discarded (4CD3F0).
Kingdom +338 statistics snapshots: 58F360 + 58F2E0 into
  KingdomEconomyLoad::snapshots (version 0 = uninitialized heap -> error).
Kingdom +35C: Find without a definition is the null key (0), as the loader
  expects. Numbered names (4E4980) increment the owner Kingdom's RESTORED
  +35C table (the same table GKingdom::Load fills at 4E910C..4E9171), via
  DefinitionCounts::assign — not a fresh per-load counter.
Kingdom +1FC/+214 recruit table: SpecialShell::recruit_items (empty in
  every shipped map); Organization recruit callbacks bound to it
  (count/at/find_single/item words/pair_single).
Staging: ActorStore::from_restored validates first and returns errors
  (Loading record, foreign Mind state, repeated Mind/state identity).
Mind: a nonempty saved Element path is applied; its nodes go to
  ActorStore::restored_element_paths and
  movement_world::Runtime::bind_element_restored (new). Saved Mind command
  objects (4D9C80/4DACE0 Load) remain explicit errors (none saved).
Inventory: 1,539 failed records, 13 clean maps; 7,879 converted,
  3,912 + 128 skipped natively.
    1,067 Organization LayoutData producer (layouts agent)
      470 ActorData+10 display name producer (saved-components agent)
        2 developer-map strings with font escapes (DisplayString decoding)

===========================================================================
ROUND 4 (2026-10-15): display name, queue Load and PostLoad pass
===========================================================================
R4-7 display name: RestoreDefinitions::actor_display_name =
  ActorCatalog::native_display_name over the definition-time text table
  (Catalog::definition_text, cloned before MAPSTRINGS; 4B9F40 missing key ->
  empty text). The 470 display-name stops are gone.
R4-1 GWorld::Load tail (aa-runtime restoration::Staged::post_load):
  1. UPDATABLES queue Load (saved_queues::decode + install onto the owners
     bound by Owners::bind_saved_table), gated on the GWORLD version 4FC600
     read at its start (not the raised GWorld+2C).
  2. aa_actors::post_load::post_load_pass over
     aa-runtime post_load_world::World (GTID table, staged objects, component
     Updatables through Owners/Queues, the world NativeRng, the final
     GWorld+2C in RestoreReport::world_version, ActorCatalog for ThingData
     +2A8 / Nation badges / display names, SVar UnitSupplyEnabled).
  3. every object's LoadCompletion becomes PostLoaded.
  Preconditions: no stopped section, no failed record, every object
  AwaitingPostLoad; otherwise Error::PostLoadState. On a pass error the
  earlier objects' effects stay (native has no rollback): discard the world.
  Explicit errors (not reached by any map that loads completely): virtual
  +12C helpers beyond +50/+8 writes (ThingData +1C, set_blocking, GWorld
  +154, detection memory records, UI agent, settlement zone terms), Wall
  +42C (only for world version < 26), Hq slot 30 (no staged component yet:
  ThingData +530 producer missing), Kingdom +34C group objects, Settlement
  and GGroup PostLoads, Denizen command objects (refused by the decoder).
  Actor +118/+120/+128 lists have no staged storage (Load requires them
  empty), so the pruning sees empty lists.
  SUN and MISC_DATA (between UPDATABLES and the pass natively) are not
  restored; no ported PostLoad reads them and they draw no RNG.
  Staged::init_for_loaded_map (4F1E10 when settings +A8 is 1 or 4,
  InitForLoadedMap 4F5290/4F1C20, then the conversion replacement tail C4)
  is an explicit Unported error.
Inventory (restore_inventory.rs now also reloads each clean map with
  StopOnError and runs Staged::post_load): 1,069 failed records, 18 clean
  maps; every clean map completes the queue Load and the PostLoad pass
  (discarded queue entries = entries of natively skipped records).
    1,067 Organization LayoutData producer (layouts agent)
        2 developer-map strings with font escapes (DisplayString decoding)
