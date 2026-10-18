# Phase 8 combat: requests to the lead

Round 2 additions are at the end (section 7). Item 1 was applied by the lead.

No root `Cargo.toml`/`Cargo.lock` change and no new dependency is needed. All new
code uses the existing aa-combat (aa-body, aa-economy, aa-numeric, aa-spatial)
and aa-body (aa-numeric) dependencies. No `aa-runtime/src/lib.rs` line was added.

## 1. Two-line test patch in aa-mapworld (needed now)

Commander experience is now ONE store: `aa_combat::commander::State::experience`
is an `aa_economy::tick::CommanderExperience` (balance = Commander+0xC, maximum =
FactionData+0x3C). aa-mapworld's library and aa-runtime still compile unchanged;
only `crates/aa-mapworld/tests/commander_restore.rs` no longer compiles:

```diff
@@ line 60
-    assert_eq!(state.experience, F32(17f32.to_bits()));
+    assert_eq!(state.experience.balance.0, 17f32.to_bits());
@@ line 97
-    assert_eq!(state.experience, F32(17f32.to_bits()));
+    assert_eq!(state.experience.balance.0, 17f32.to_bits());
```

## 2. FactionData `experience_max` (FactionData+0x3C) must be loaded

The value is exposed through `commander::State::with_maximum` /
`commander::replace_with_maximum` (and lands in `state.experience.maximum` for
aa-economy). Nothing loads it yet: the FactionData loader (TGI key
`experience_max`, string 0xA0D7B8 referenced at 0x48BE55 inside the 0x48BAE0
loader) is not in `aa_rules::commander::CommanderConstructorInputs`. Until the
aa-rules owner adds it and aa-runtime `catalog.rs` / aa-mapworld
`commander_restore.rs` call `replace_with_maximum`, the legacy `replace` stores
`MAXIMUM_EXPERIENCE_UNBOUND` (a NaN marker) which aa-economy's 4CDFD0 rejects:
no cap is invented.

## 3. Runtime bindings (single stores)

- Economy tick: bind `aa_economy::tick::CommanderParts.experience` to
  `&mut commander::State::experience` of kingdom+0x230 (no copy in/out).
- Ability payment 4E2590/4CDD90: `aa_combat::payment::State` now BORROWS the
  kingdom `Economy` and that same `CommanderExperience`.
- Statistics (kingdom+0x32C): `aa_body::credit::World::add_statistic` (replaced
  the old `stat_mut`) and `aa_combat::payment::World::add_statistic` must call
  `aa_economy::tick::add_score` on the single aa-economy `Statistics`; aa-body and
  aa-combat keep no statistics storage type. Open item for the aa-economy owner:
  kill credit also updates the statistics object's RELATION map through 58D950
  (`aa_body::credit::RelationStat`, index 0x3C); aa-economy `Statistics` has no
  relation map yet. Please add it there so credit can bind to it and
  `RelationStat` can become a view.

## 4. Capture and ownership change (assigned to me; next round)

`capture::attempt` (4D5230) is proven; its ownership change is the actor
virtual +0x160 = 0x4D8450 `KKC_GActor::ChangeKingdom` (364 decompiled lines;
callees include 0x4E2A60, 0x4E2CB0, 0x4E44E0, 0x4E4C30, 0x4E6180, 0x4ECB90 kingdom
economy/registration, 0x589050/0x589CC0 research/queues, 0x565400/0x565700
construction, 0x5714D0 organization, 0x7A0A40 SAI). Porting it will need the
aa-economy kingdom add/remove-actor APIs for provided/consumed resources. Please
confirm the aa-economy owner exposes them (or that I may add an
`aa_economy`-independent World seam for them).

## 5. Seams that other owners must implement (no successful defaults exist)

- aa-mind (CAI/UAI): `engagement::World::add_engage_target` (524C10),
  `morale::World::{mind_declines (+5C), mind_exhausted (51F9F0), mind_wavering
  (51F920), engage_target (+90)}`, Mind +64/+7C/+6C/+188 virtuals. Unported
  target-acquisition states listed in inventory.json (535240 UAI Kill Think,
  5206A0, 523DD0/523EA0, 51EFB0/51F040, 527240, 52E830, 5340C0 which consumes
  the proven ROE rate 570E30).
- aa-actors: `morale::World::state_group` (4D49B0) and `set_state` (4D64F0).
- World/runtime owner: `death::PhaseWorld::process_removals` (4F65B0 removal
  list), Organization TickMaintenance 57C8E0 scheduling (calls the proven
  57A0A0, 579600, 5785D0, 578660), and `shortfall::World::draw_random`, which
  must draw from the single authoritative world RNG (4F02B0 + 85D840).
- Presentation/events: 42D490 messages (rout 0xC, capture 0x10/0xF, engaged
  0x24/0x25), 50D0B0 damage numbers.
- aa-triggers: 543800 EffectActorModifyMorale should call `morale::modify`.

## 6. Shared dependency hiccup observed

During this round `aa-spatial` briefly failed to compile (`mod overlaps` file
missing in crates/aa-spatial/src/lib.rs:362); it recovered later. No action
taken by me.

## 7. Round 2 (2026-10-14)

1. **No Cargo change.** aa-combat/aa-body dependencies unchanged.
2. **One removal path.** `aa_combat::death` is now producer-only: `DeathQueue`
   is a trait (`push_front`) the runtime binds to
   `aa_world::World::enqueue_front(ListKind::Pending100, actor)`; the drain
   4F65B0 and removal 4F4C40/4F6760 are aa-world's. Member removal's owner
   removal calls `RemovalWorld::request_owner_removal` = `World::request_removal`.
3. **Economy bindings for ownership change (4D8450)**, `aa_combat::ownership::World`:
   `kingdom_remove_actor(kingdom, actor)` -> `aa_economy::kingdom_registry::remove_actor` (0x4E44E0);
   `kingdom_add_actor(kingdom, actor)` -> `add_actor` (0x4E6180);
   `find_single(kingdom, actor)` -> `RecruitItemTable::find_single(definition, false)` (0x4E2A60, kingdom+0x1FC);
   `remove_single(kingdom, single)` -> `RecruitItemTable::remove_single(single, force=true)` (0x4E2CB0);
   `add_single(kingdom, single)` -> `RecruitItemTable::add_single` (0x4E4C30);
   `free_single(single)` -> drop of the removed single. No new economy API is needed.
4. **Relation statistics.** `aa_body::credit::World::add_relation(statistics, index, other, amount, precision)`
   replaces `relation_mut`; bind it to `aa_economy::statistics::RelationStatistics::add`.
   `aa_body::credit::RelationStat`/`increment_relation` were removed (no external users).
5. **AI agent (aa-mind)**: combat-side services for the routed CAI/UAI states
   are exposed: `roe::rate` (5340C0 input), `engagement::element_damaged` (calls
   AddTarget 524C10 through `add_engage_target`), `morale::{state, lose, ...}`
   (51F9F0/51F920 reactions are `mind_exhausted`/`mind_wavering` callbacks),
   `organization::{tick_maintenance, follow_leader, remove_member}` (Mind
   +64/+7C/+110/+114/+150 callbacks), `reactions::attacked_by`.
6. **Numeric owner**: 45E090/4BBF90 (combat value, AI input) need an exact
   MSVC CRT `pow` (0x96D890 dispatches to the SSE2 path 0x977A10 when SSE2 and
   default MXCSR/x87 masks, else the x87 path 0x96D8ED). Unicorn's x87
   transcendental emulation is not ground truth, so an oracle needs either the
   SSE2 path (deterministic integer/SSE arithmetic) or real hardware traces.
7. **Still seams (next round, same owner or yours to assign)**: 57B5A0
   regeneration, 57AEB0 regroup attempt, 4D0A80 in-supply, 45D520 entrench
   delay, 57A640, 5797E0, 4D4CF0, 4CF0F0, 570670 membership detach, 4CEF70 ->
   581810 property removal by source (aa-body property owner), 515F40 flare,
   57C680, 585160/569540/586370 wall/denizen helpers, and the ChangeKingdom
   component callees (589CC0/589050, 565400/565700, 4D19F0, 4DFF30, 574590,
   584F10, 579F10, 56ADA0, 5643A0, 5714D0, 50C2B0, 4D5DE0/4CFD30, 4ECB90,
   585FF0, 7A0A40).
8. **Observed**: `cargo check -p aa-runtime --all-targets` currently fails in
   aa-runtime's own `tests/saved_performer.rs` (10 vs 9 arguments to its own
   `src/saved_performer.rs:83`); not caused by aa-combat/aa-body.

## 8. Round 3 (2026-10-14)

1. **aa-numeric**: new `crates/aa-numeric/src/crt_pow.rs` plus the single
   appended line `pub mod crt_pow;` in `aa-numeric/src/lib.rs` (no other
   aa-numeric file touched, no Cargo change; it uses the existing
   `thiserror` dependency). The file is generated by
   `research/phase8-combat/lift_crt_pow.py`; regenerate rather than edit.
   It reads `Binary`'s private fields through the parent module for
   `store_binary64` (x87 `fstp qword`).
2. **Tick agent / numeric users**: `aa_numeric::crt_pow::ci_pow_x87(&x, &y,
   CrtEnvironment::SSE2_DOUBLE53)` is the general `_CIpow`; the base-2
   specialisation in aa-runtime `speed.rs` can be cross-checked against it.
3. **Runtime bindings to add (no defaults exist)**:
   - `aa_economy::kingdom_registry::RegistryWorld::statistics_track(actor, add)`
     -> `aa_combat::statistics_tracking::track` / `untrack` on the kingdom's
     `Statistics` and the actor's class word (+0x13C, initial 0x38);
     `statistics_classified` -> `statistics_tracking::classified`.
     `capture::AttemptWorld::capture_statistics` -> `track` likewise.
   - `RegistryWorld::apply_property` / `remove_property_effect` ->
     `aa_body::property_api::apply_property` (0x4CEF10) /
     `remove_property` (0x4CEF70).
   - `organization::World::regenerate` / `regroup_check` ->
     `aa_combat::regeneration::{regenerate, regroup}`; `entrench_time` ->
     `organization::entrench_delay`; `RemovalWorld::recompute_6c` ->
     `organization::recompute_speed`.
   - Combat value: whoever owns ThingDataManager (`[0xB39F58]`, definition
     loading) calls `combat_value::mean_values` (0x4BBF90) then
     `actor_data_values` (0x4BB590); performers call `performer_value`
     (0x57E430). `combat_value::World` needs the two SVars and the modifier
     getters (aa-body `ModifierState::{power, recharge, projectile_range,
     bombard_range}`).
4. **Spawn seam**: `regeneration::World::spawn` is 0x57AD30 (actor creation
   through 0x4FBF40 with a creation descriptor); the world/creation owner
   should provide it.
5. **Emulator caveat for every oracle owner**: Unicorn 2.1.4 does not quiet
   signalling NaNs in x87 m64 conversions and picks between two distinct NaN
   operands of SSE arithmetic with the x87 rule; also, rewriting code at an
   address already executed reuses the stale translated block (write each
   stub once). Details in README round 3.
6. **Still seams** (listed in README "Round 3 gaps"): 4D7960 BeginDeath,
   570670, 515F40, 57DD90, 54BEA0, 56ACB0, 585A20, 569540/585160/586370.

## 9. Round 4 (2026-10-15)

1. **aa-rules (now owned here: abilities.rs, ability_load.rs only)**: no
   other aa-rules file was touched. The restoration agent should re-run
   `research/phase8-restoration/unsupported-inventory.json`. The groups
   "completed ground Ability currently requires MAIN_DAMAGE effects" and
   "Ability effect input kind ENCHANTMENT not implemented" now load: the aa-rules
   lib test `abilities::loading::original_data::original_ability_effects_resolve`
   loads 363 actors' abilities from Data.rwd plus the v14 catalog with no
   failures. Effect kinds seen: MORALE_LOSS 96, HEAL 12, MAIN_DAMAGE 327,
   ADD_DAMAGE 7, ENCHANTMENT 11.
   - The aa-rules lib briefly failed to build (`NativeDataRegistry` /
     `native_data` in actors.rs/native_load.rs, another agent's in-progress
     edit). It is fixed now; nothing was changed here for it.
2. **Explicit Unsupported in ability loading (owners)**:
   - SUMMON effects (4CC890; deferred resolution 473AD0 -> 4CC730 against
     the object catalog): definition-resolution owner. No actor ability uses
     SUMMON in the shipped data.
   - Enchantment modifier kinds other than FLAG/ADD/RESIST_DAMAGE/
     PERIODIC_DAMAGE (MULTIPLY, ENCHANTMENT_IMMUNITY, ABILITY_EFFECT,
     PERIODIC_HEAL, REVERSE_ABILITY, RESOURCE_PRODUCTION): not used by
     enchantment properties in the data. They would be ported from 4C4030 on
     demand.
   - Non-empty EnchantmentProperty `required_properties`, and quoted
     required-property tokens (tokenizer 424FA0/412480): not used in the data.
   - Validation of FXC kinds other than CREATE_VFX (only the permanent-VFX
     facts 428A30/42C5E0/494F30 are ported): FX loader owner.
3. **Runtime bindings to add (no defaults exist)**:
   - Actor BeginDeath 0x4D7960 -> `aa_combat::begin_death::begin_death`, for
     aa-body `Event::BeginDeath`, `organization::RemovalWorld::begin_owner_death`,
     `walls::TransferWorld::begin_death` and projectile deactivation. Its
     `begin_death::World` leaves are listed there:
     - credit 4D57C0 is aa-body `credit::credit_kill`;
     - zone release 514CE0 belongs to the zone owner;
     - announcements 42D490 belong to the UI;
     - component virtual +34 deaths belong to each component's owner;
     - 568B70, 5859C0/584D50 and StartDying 562380 (aa-body `dying::start_dying`);
     - removal 4F6760 is `aa_world::World::request_removal`.
   - `capture::AttemptWorld::transfer_wall` / `transfer_component88` ->
     `walls::transfer_wall` / `transfer_component_88`.
   - `siege::World::wall_predicate` / `component88_predicate` / `wall_update`
     -> `walls::wall_complete` / `intruder_near_wall` / `propagate_under_siege`.
     The 585280 within-wall geometry (`IntruderWorld::within_wall`) needs the
     wall/geometry owner.
   - `organization::RemovalWorld::detach` -> `organization::detach_membership`.
   - `regeneration::World::spawn` -> `regeneration::spawn_member(o, slot,
     false, ..)`. The actor-creation owner provides
     `SpawnWorld::create_actor` (0x4FBF40 with the `CreationDescriptor`).
   - `performer::World::track_target` -> `performer::track_target` on the
     owning `OwnedPerformer`; Performer `+8` Updatable activation 50AC00 is
     `TrackWorld::activate_updates`. **AI agent (aa-mind)**: `uai_kill.rs`
     (`57DD90(1, target, &scratch)` / `57DEA0`) and `element_move.rs` can bind
     `performer::track_target` / `clear_turret_intentions`.
4. **Identified, not combat (owners)**:
   - 0x54BEA0 (capture notification) is the trigger manager B36520's +50
     queue prepend of a 54B820 Captured payload. Owner: aa-triggers
     (`aa_triggers::predicates::queue_captured`, the proven 54BEA0/54B820 port).
     - +4: captured actor +14.
     - +8: source +14.
     - +C: the source if its virtual +A8 is true, else virtual +108's +14.
   - 0x515F40 (the org-tick "flare") is `KKC_Zone::Move` by world position:
     552640 clamp on [B36534]+30, `_ftol`, then a shift by the map +4 into
     515970 (.\Source\Sim\Zones.cpp). Owner: zone/detection solver
     (phase8-movement cluster "Solver/ZoneSolver.cpp").
5. **aa-rules clippy (`-D warnings`)**: abilities.rs/ability_load.rs are clean (four needless borrows and one collapsible `if` fixed this round). The remaining lint is `native_load.rs:878` (manual `Option::map`), owned by another agent.

## 10. Round 5 (2026-10-15): special operations

1. **Runtime bindings (no defaults exist)**:
   - The orders owner's `KKC_FactionUseAbilityOrder` execute (61E190) ->
     `aa_combat::special_ops::use_ability` on the kingdom's
     `commander::State`. `FactionWorld::can_use` / `use_ability` bind
     `admission::can_use` (check_range true) / `execution::use_ability`.
   - `execution::World::other_effect` per kind:
     - HEAL `special_ops::apply_heal`;
     - ENCHANTMENT `apply_enchantment`;
     - CAPTURE `apply_capture`;
     - DESCRIBE_ECONOMY `apply_describe_economy`;
     - SUMMON `summon::apply`;
     - MORALE `morale::{effect_organization, apply_effect}`;
     - STEAL_RESOURCES `aa_economy::transfers::steal_resources` after the
       target kingdom (virtual +108).
   - `periodic::World::expired` -> `special_ops::enchantment_expired` for
     enchantment instances (vtable A2EBE0). Plain instances never expire.
   - `walls::IntruderWorld::within_wall` -> `walls::intruder_within_wall`
     with `intruder_radius`.
   - New membership/performer services: `organization::attach_membership`
     (5705A0), `organization::propagate_properties` (5784C0),
     `performer::gate_by_rank` (57DCF0; its caller 57E010).
   - `aa_body::property_api::find_actor_property` (4CEF80).
2. **Seams that need their owners** (all named on `summon::SummonWorld`,
   `special_ops::DescribeWorld`, `FactionWorld`):
   - world creation 4F9C10 / 4FA030 / 4FA150 (restoration agent);
   - transport payload 58BA90 on transport +C0 with 4C2290 (transport owner);
   - commands 4CA330/4D9E20 + 4D9B60 (orders/AI owner);
   - open position 554B30 and map clamp 552640 (placement/map owner);
   - world RNG 4F02B0 + direction table [B39AF8] (world owner);
   - SAI 7A0C20;
   - UI messages 4CB5B0 (0x11) and 40BF70 (0xF) with 4E7200 / 4B9F40
     (UI/economy owners);
   - effect tooltip virtuals (+C of each effect class, 4CB050..4CBFB0)
     (UI owner).
3. **aa-runtime (not mine)**:
   - Commander abilities are still marked constructor-only
     (`catalog.rs` "special-operation definition execution and world effects
     remain incomplete"). Loading now resolves for all 24 Factions through
     `ActorCatalog::native_faction_ability_effects`, and the execution path
     above is ported, so the gate can be lifted once the bindings exist.
   - `economy_values_world.rs` reads `ModifierState.other_tables[0]` as the
     +80 production table. It should move to a typed table when the
     ModifierState non-numeric tables are ported (next item).
4. **Next combat work (not ported this round)**: ModifierState
   Add/RemovePropertyModifiers for the non-numeric kinds:
   - 502B30 cases 3/4/6/7/8/9 and 502C60;
   - immunity purge 4FE6F0;
   - 502AD0 ability-effect lists;
   - 502560/5015C0 periodic damage;
   - +A4 heals;
   - 502640 reverse abilities;
   - 501F60 production.
   I can take it next round; it changes `ModifierState`'s table types, which
   aa-runtime reads.
5. **aa-rules notes**:
   - The round-4 required_properties tokenizer was wrong (it split on
     whitespace). It is now the exact 424FA0/412480 port.
   - The CREATE_VFX lookup now uses the Thing manager (4BDB70), because
     special-op VFX are `[Thing Inherit=VFX...]` roots.
   - 4BF990 resolves required properties immediately against the
     registration-ordered Property registry; Rust resolves against the
     complete registry. This is the same for shipped data but differs if a
     mod references a property registered later.
   - The remaining aa-rules clippy lint is still `native_load.rs:878`
     (another owner).
6. **Triggers request done**: capture.rs, the inventory and section 9 now
   name `aa_triggers::predicates::queue_captured`.

## 11. Round 6 (2026-10-15): ModifierState table kinds

1. **aa-runtime edit (approved)**: `economy_values_world.rs` `production_delta`
   now reads `ModifierState.production`. Only that function changed; the
   economy agent may want to know.
2. **Bindings for whoever implements `aa_body::property_flags::Effects`**
   (the Properties/actor runtime):
   - `resource_count` = `[B36530]+38`;
   - `refresh_actor_4cf0f0` = 4CF0F0;
   - `create_reverse_ability` = `aa_combat::ability::Ability::new(definition,
     None, 4BE3F0(0))` in the ability arena;
   - `destroy_reverse_ability` = 4CE080 and drop;
   - `enchantment_type` = property definition `+64` (loaded as
     `aa_rules::abilities::EnchantmentProperty::enchantment_type`);
   - `allied` = actor virtual `+138(allegiance)`.
   Enchantment instances must be created with `Instance::enchantment`
   (503140; `special_ops::EnchantmentWorld::insert_enchantment`).
3. **Readers of the new tables**:
   - the 5818D0 tick uses `aa_combat::periodic::periodic_from_tables`;
   - `admission::World::effects_for` (501920) reads `ability_effects`;
   - +B0 reverse abilities are available to whoever ports their consumers
     (performer/admission users of ModifierState+B0).
4. Seams you routed (creation, transport payload, commands, SAI, UI) stay as
   named `SummonWorld` / `DescribeWorld` methods. The placement, clamp and
   direction-table seams now point at their existing ports.
