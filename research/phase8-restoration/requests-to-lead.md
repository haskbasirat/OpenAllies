# Requests to lead (phase8 restoration)

1. **Mind saved Load (slot 4) is the next blocker of both tutorial maps.**
   Element Mind Load `0x536340` (base Mind Load `0x52C580`), Organization Mind
   `0x528C20`, Aircraft Mind `0x51C3C0`. The Mind state types live in aa-mind
   (not mine). Either assign the Mind saved-state port to the aa-mind owner
   (aa-mapworld side: I can add the payload decoder and the restore hook once
   aa-mind exposes `apply_saved(&mut Installed, &SavedMind)`), or transfer
   ownership for this step.
2. **Organization component Load `0x57D130` (slot 7)** — regiments/flocks in
   both tutorials. In my crates (aa-actors OrganizationState), next in my queue
   after Mind unless you split it.
3. **Definitions agent handoff facts** (also in unsupported-inventory.json):
   `0x4FAE80` skips records with a missing definition (logs via `0x5A1040`) and
   converts/skips class mismatches; `0x4FC340` seeks to the next
   DYNAMIC_THINGS_INDEX entry. aa-mapworld `restore.rs::restore_dynamic`
   currently stops instead; the skip must be wired there (my file) once the
   definitions agent exposes "definition missing" distinctly from other
   resolution errors.
4. **Not done this round (data items):** Kingdom recruit-singles (+0x214) —
   needs the `0x4E6C20`/`0x4E79F0`/`0x4E7A30` item construction and
   ActorData+0x2CC; no restoration record in the 80-map inventory reached a
   nonempty Kingdom recruitment list. `terrain_speed.rs` audit — not started.
5. **Economy kingdom-load wiring (your message, sections 13-16):** aa-mapworld
   has no aa-economy dependency. Either approve adding
   `aa-economy = { path = "../aa-economy" }` to crates/aa-mapworld/Cargo.toml
   (Cargo.lock entry for aa-mapworld gains one dependency line) so
   `special_state.rs` can call `Statistics::load_saved` /
   `RelationStatistics::load_saved` / `loaded_property_records` on the decoded
   +0x330/+0x344 words, or I apply them in aa-runtime restoration.rs (which
   already depends on aa-economy) after `restore_records` — tell me which, and
   where the economy agent wants the resulting `KingdomStatistics` stored.
6. **aa-rules test expectations broken by the definitions agent's 19:41 edits**
   (actors.rs/resolved.rs/native.rs): `phase3_local` now sees 1959 actors (I
   set 1646 after the parser fix) and `native_collision_definition` sees 1647
   catalog sources instead of 1847. Not my change; the definitions agent should
   confirm the new counts and update those two tests (my files) or tell me to.
7. No Cargo.toml / Cargo.lock changes were made.

# Round 2 (2026-10-15)

R2-a. **OrganizationLayoutData producer (blocks all 1,067 Organization
   records, both tutorials).** `RestoreDefinitions::organization_layout_info`
   (aa-mapworld restore.rs; catalog default = explicit error) must return
   `aa_actors::organization_state::LayoutInfo` for a layout IDS: +8 name, +18
   primary slot, +1C/+20 entries (Property handle + IDS, required/locked/
   morale_line), +28/+2C slot groups, +34/+38 formation handles, +44/+48,
   +50 property handle. aa-rules `native_organization_layout` lacks the
   formation objects and handles; the native KKC_OrganizationLayoutData
   LoadData (4C3xxx) needs a producer port and a handle convention shared
   with aa-movement formations. Please assign (definitions agent?).
R2-b. **Organization callees still unported** (explicit errors in
   organization_restore.rs): 4CF110/501C90 modifier, ActorData+2C0 morale
   contribution, 45EEF0, 5705A0, 5784C0, 4CEF10/4CEF70/4CEF80, 524B80,
   57C800, 579A00 (set leader over restore shells), 5788B0, 4D1080, 4D64F0,
   4D4CF0/4CF0F0/4D1350, 45EC60, 5147C0, recruit 4E13F0/4E2A60/4E1200,
   48D500 DamageType handle, formation facts, 4D49B0, 57BEF0, 4D1900/4B2830,
   855D30 dialog, 57DCF0. Bind them as their ports land.
R2-c. **pair_recruit_item (4E1200)**: aa-mapworld has no aa-economy
   dependency and the Kingdom +214 recruit table is not restored. Either
   approve `aa-economy` as an aa-mapworld dependency (no cycle: aa-economy
   does not depend on aa-mapworld) or move the binding behind a
   RestoreDefinitions method in aa-runtime; and assign the Kingdom +214
   recruit-single restore (round-1 open item).
R2-d. **Saved-component wiring done**: Transport (REQUIRED_COMPONENT used),
   Organization (re-entrant), Denizen slot list (legacy Definition removed;
   I updated the saved-components agent's test
   aa-actors/tests/native_saved_denizen_inventory.rs `legacy_definition_...`
   -> `empty_inventory_builds_no_slot` because the struct no longer exists).
R2-e. **Mind**: Regiment (528C20) and Aircraft (51C3C0) Loads are explicit
   errors — no original map saves either kind (all 2,194 distinct saved
   Minds are Element). A nonempty saved Element path needs a typed path
   hand-off in aa-runtime movement_world::bind_element (owner: movement);
   saved commands need the command store's 4D9C80 Load (commands agent).
R2-f. **Creation**: aa-actors world_creation::{create (4F9C10),
   spawn_member (4FBF40)} are the shared entry points (oracle-proven
   control flow and x87). Binding CreateWorld/SpawnWorld to the live
   ActorStore needs InitCreate 4D6F10 (aa-actors init_create::initialize
   World over ActorStore), InitExisting 4D7230 (also PostLoad), 4CF300,
   4F9510 world registration, 50C220 — these are also the PostLoad /
   publication path for loaded maps, so they should be one integration
   task; class constructors other than Actor 4D6BB0 (construct_new) are
   unported. The conversion agent can call `create` through its trait now.
R2-g. **Mind core 'unreachable!'/'assert!' at staging**: ActorStore
   from_restored treats a non-adapter RestoredMind or duplicate identity as
   an invariant violation (panic); all restored Minds come from
   catalog::Catalog::construct_mind.
R2-h. No Cargo.toml / Cargo.lock change was made. New files: aa-mind
   saved_load.rs (+ `pub mod` line), aa-mapworld organization_restore.rs (+
   mod line), aa-actors world_creation.rs (+ mod line), tests
   aa-mind/tests/saved_load_native.rs, aa-actors/tests/native_world_creation.rs,
   aa-actors/tests/native_spawn_member.rs, research native_mind_load.py,
   native_create_gthing.py, native_spawn_member.py and their fixtures.
R2-i. **Creation hand-over (ownership moved to the conversion agent).**
   What exists: `crates/aa-actors/src/world_creation.rs` (+ `pub mod` line in
   aa-actors lib.rs) with `create` = 4F9C10 control flow (instance-name
   check 4154D0 + IDMGR, class sizes 2..10, GACD/GKCD refusals with the
   404ED0 lines 0x7F6/0x821/0x82A, Actor mode/state/Body/Organization-layout
   steps incl. the B3CC42 assertion latch, E4 -> E8 -> EC(params,0) ->
   4F9510 -> 50C220 -> B0 -> 42A590, 4F5490 on failure; unknown class or a
   failed allocation = NullDereference error) and `spawn_member` = 4FBF40
   (nearest related recruit structure, binary32 squared distance at PC,
   4FACA0 + 57D860, else slot position + 578290 + 57D950). Oracles:
   research/phase8-restoration/native_create_gthing.py (123 cases) and
   native_spawn_member.py (240 cases at PC24/53/64); replays
   aa-actors/tests/native_world_creation.rs and native_spawn_member.rs
   (238 bit-exact, 2 refused non-finite operands). Nothing binds the
   CreateWorld/SpawnWorld traits to ActorStore; I stopped there. The
   conversion agent may take, move or rename these files.
R2-j. **ActorData+10 display name** (actor_names NameWorld::display_name):
   RestoreDefinitions::actor_display_name needs the GetLocalized("name")
   producer (definitions agent, R3-2 of the saved-components requests);
   273 records wait on it.
R2-k. **PostLoad / publication (round-2 item 9) not started**: no tutorial
   restores completely (Organization LayoutData producer, then the
   unported Organization callees); the replacement step of the conversion
   queue and KingdomStatistics::start_world belong to that same step.

# Round 3 (2026-10-15)
R3-a. **Numbered names use saved Kingdom state.** 4E4980 increments the
   owner Kingdom's +35C table, which GKingdom::Load restores from the save
   (4E910C..4E9171). `aa_actors::actor_names::NumberedNameCounters` (per-load,
   "not saved") does not match the binary; restoration now uses
   `aa_mapworld::definition_counts::DefinitionCounts::assign` on the
   Kingdom's table. The saved-components agent may want to align its doc;
   the owner-change path 4D6F10 should use the same Kingdom table.
R3-b. **Round-1 statement corrected**: the 18 "City no longer has a city
   zone" records are dropped by 4FAE80's TGC_LoadCancelException catch; the
   map continues.
R3-c. **Still blocked on other areas**: LayoutData producer (1,067),
   display name producer (470), PostLoad/InitExisting hooks (tutorials and
   the conversion replacement step C4 / FreshWorld), saved Mind commands
   (command store Load 4DACE0), DisplayString decoding for 2 developer-map
   strings (localization owner), Element+14 recruit link field in
   ElementState (pair_single link_element is an explicit error; no shipped
   data reaches it).
R3-d. Signature changes: `ActorStore::from_restored[_with_abilities]` now
   return `Result`; `Attempt::into_staged` returns `restoration::Error`;
   `aa_mind::saved_load::apply` returns `Applied`. I updated the call sites
   in aa-runtime tests (actor_world, trigger_world, saved_performer).

# Round 4 (2026-10-15)
R4-a. **R4-1 done as far as native order allows.** `Staged::post_load`
   (aa-runtime restoration.rs) installs the saved UPDATABLES queues, runs
   `aa_actors::post_load::post_load_pass` over the new
   `aa_runtime::post_load_world::World` and marks objects `PostLoaded`.
   All 18 maps that load completely pass it. New: `RestoreReport::
   world_version` (final GWorld+2C), `LoadCompletion::PostLoaded`,
   `ActorStore::staged_placement`, `Attempt`/`Staged` keep `settings`,
   `definition_text`, `updatables_chunk`. The binding follows post_load.rs
   as of 07:20 (it changed twice while I bound it; a later trait change
   breaks aa-runtime's build until the binding follows).
R4-b. **Next native steps unported**: `Staged::init_for_loaded_map` is an
   explicit error: 4F1E10 (map settings +A8 in {1,4} and 59E520),
   VWorldState::InitForLoadedMap 4F5290 / 4F1C20, then the conversion
   replacement tail (C4 / FreshWorld; 4F9C10 per queued record), which
   natively runs after them — so I did not call replace_converted yet.
   SUN and MISC_DATA world sections are also unrestored (not PostLoad
   inputs).
R4-c. **Hq (slot 30) wiring** needs a ThingData +530 (Hq slot count)
   producer in aa-rules for `HqState::new`; with it I can add
   `owned::Component::Hq`, the slot-30 SavedComponent decoder,
   `HqWorld` and the `hq_*` PostLoad bindings. tutorial02 stops earlier
   (LayoutData) anyway.
R4-d. Remaining restore stops: 1,067 LayoutData producer, 2 DisplayString
   font escapes.
