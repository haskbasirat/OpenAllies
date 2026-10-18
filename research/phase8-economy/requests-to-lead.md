# Phase 8 economy: requests to the lead

No root `Cargo.toml`/`Cargo.lock` change and no new dependency is needed: all
work uses the existing aa-economy (serde, num-bigint, num-traits) and
aa-production (aa-numeric, aa-economy, serde, thiserror) dependencies. No shared
`aa-runtime/src/lib.rs` line was added. No file outside my ownership was edited.

## Round 2 status of earlier requests

1. **Statistics (+0x32C/+0x340): done, owned by aa-economy.**
   `aa_economy::statistics::{Statistics, RelationStatistics}` are the single
   types. Combat writes ordinary scores with `Statistics::add_score` (also
   re-exported as `aa_economy::tick::add_score`) and kill-credit relations with
   `RelationStatistics::add(index, Some(kingdom), amount, pc)` (0x58D950).
   `aa_body::credit::RelationStat` should become a read-only view of
   `RelationStatistics::entry(index)` (slot index-52). New: slots 0..4 are the
   by-definition maps of 0x58D8C0 (`add_by_definition`, slot index-57, used by
   construction completion, research and production counters); the two native
   adders share the 10-slot object, so no second relation store may exist.
2. **Commander XP.** kingdom+0x230 (Commander, aa-combat) stays the single
   balance; aa-economy reads/writes it only through `CommanderExperience`
   values the caller binds. Please confirm combat exposes FactionData+0x3C (the
   clamp maximum used by 0x4CDFD0) through its Commander definition API.
3. **Recruit singles restore** is with the restoration agent (no change).
4. **Scheduling.** Registration details are now recorded in README.txt
   ("Tick registration"): 0x50AC00(0) from GKingdom::Init into world+0x1AC,
   deferred 0x50AD80 via world+0x208, unschedule 0x50B1C0(1,0).

## New requests

5. **aa-body property effect API (needed by 0x4E3AD0/0x4E23D0 and the
   registry).** Please expose, in aa-body (I did not edit it):
   * `apply_property(actor, definition, source: u32, origin: Option<actor>)` =
     native `actor.0x4CEF10(definition, source, origin)`;
   * `remove_property(actor, definition, origin: Option<actor>)` =
     native `actor.0x4CEF70(definition, origin)`.
   `aa_production::research::{KingdomPropertyWorld::apply_property,
   KingdomPropertyRemoval::remove_property_effect}` and
   `aa_economy::kingdom_registry::RegistryWorld::{apply_property,
   remove_property_effect}` are the callbacks to bind to them.
6. **ChangeKingdom (combat).** The add/remove operations and the recruit-table
   steps with their native order and exact signatures are in README.txt section
   9. `RegistryWorld` callbacks the combat/world owner must bind: economy
   add/remove (aa-runtime `economy_refresh`, mine), statistics tracking
   0x58F010/0x58EFE0/0x58EA80 (still seams: please route them to aa-economy if
   combat ports them), city/tech tracking 0x4E3C70, property apply/remove (item
   5), organization refresh 0x4D1350, actor destruction.
7. **World seams for construction** (owners to decide): actor creation
   0x4F9C10, footprint iteration 0x554DD0/0x554330 with tile test 0x54EFC0,
   zone owner 0x4F0B70, site lookup 0x555830, grid snap 0x46EE00, kingdom
   restriction 0x4E1910, cost quote 0x45F4D0 (aa-actors phase 7), enable/disable
   structure 0x565400/0x564DC0, actor refresh 0x4D6660, settlement registration
   0x586B40/0x585500, OnConstructionFinished 0x565B00 / OnUpgradeFinished
   0x566400 (actor transformation; their statistics parts are ported here).
8. **Event/trigger/presentation seams** to bind in their owners' crates:
   0x42D4B0 kinds 3/4/5/6/0x1D/0x1E/0x1F/0x20/0x33, 0x42D490 kinds 0xB/0x17,
   0x40C230 messages, 0x404F30/0x404F60/0x404ED0 diagnostics, 0x54BFC0 /
   0x42A590 triggers, victory checks 0x4E9960/0x4E9A40.
9. **FPU policy acknowledged.** Double53 is documented as the game default in
   README.txt and findings.json; all corpora still cover PC24/PC53/PC64 and no
   fixture depends on Unicorn transcendental instructions.

## Round 3

10. **Lead answers recorded:** FactionData+0x3C comes from
    `aa_combat::commander::State::with_maximum / replace_with_maximum`; the
    aa-body property effect API (0x4CEF10/0x4CEF70) and the statistics
    tracking seams (0x58F010/0x58EFE0/0x58EA80) are with the combat agent.
11. **aa-mapworld naming (restoration agent).** Its Kingdom "strategic" state
    (0x58EB60/0x58EE80) and "tactical" state (0x58D560/0x58DBE0) are the
    kingdom statistics objects (+0x330/+0x344 while loading). The runtime should
    feed the decoded words into `aa_economy::statistics::Statistics::load_saved`
    / `RelationStatistics::load_saved` (keys resolved by the owner of
    definition/GTID lookup) and call `KingdomStatistics::start_world` at world
    start (0x4F1E10). Its raw Properties list should go through
    `aa_economy::kingdom_load::loaded_property_records` before becoming
    `aa_production::research::KingdomProperties`. No aa-mapworld edit was made.
12. **Restriction profiles (aa-rules / world owner).** `aa_economy::
    restrictions::RestrictionProfiles` needs the world+0x12C profile data
    (loader 0x505760) and the kingdom +0x348 / team +0x64 names; Construction::
    kingdom_allows and KingdomPropertyWorld::technology_allowed should be bound
    to `actor_allowed` / `technology_allowed`.
13. **World seams of construction completion** (owners to assign): actor
    creation/replacement 0x4F9C10, CreateComponents 0x4D21A0, settlement join
    0x586470 / attach 0x585F60 / refresh 0x584B40, component hand-offs
    0x588970/0x58CAB0/0x588AE0/0x58CB80/0x5698C0/0x5814A0, state transfer
    0x4D5EA0, groups 0x4D3DD0/0x4D0000, AI 0x79EA90/0x7B2350, triggers
    0x54C050/0x42A590, enable/disable structure 0x565400/0x564DC0 (actor state;
    0x565400 is also used by ChangeKingdom). The economy pieces are bound to
    existing ports (kingdom_registry::actor_completed = 0x4E4470, statistics
    construction_completed/upgrade_completed, economy refresh, cancel_upgrade).
14. **x87 duplication acknowledged.** `aa_economy::x87` was not extended in
    round 3 (start mission uses exact comparisons only); it can be folded into
    aa-numeric with `aa_mind::x87` whenever the lead decides.

## Round 4

15. **ChangeKingdom (combat).** `aa_production::structure_state::
    enable_structure(&mut impl StructureState) -> Result<()>` is 0x565400
    (state 0x1B, command recompute 0x565140, settlement refresh 0x584E10).
    ChangeKingdom 0x4D8450 calls it for a disabled structure (actor+0x100 bit
    30) after clearing the queue (0x589CC0). Actor-state writes go through
    `Construction::construction_actor_state` (0x4D64F0, aa-actors owner) and
    the command mask through `StructureState::set_command_mask` (actor+0xF0).
    `ConstructionLifecycle::{enable_structure, disable_structure}` should be
    bound to `structure_state::{enable_structure, disable_structure}`.
16. **World start (world integration).** `aa_economy::world_start::world_start`
    covers the economy parts of 0x4F1E10; it needs the world kingdom list
    (+0x154, null entries kept), the GTID-table walk, actor+0x13C, owners and
    kingdom +0x190, statistics tracking 0x58F010 (combat is routing it into
    aa-economy) and the snapshot object 0x58F530.
17. **Emulator caveats.** The shared harness now faults on non-FS page-0
    accesses. Three silent null reads were found and fixed in the oracle
    setups; only the construction-finish fixture changed (57 out-of-domain
    executions removed; `upgrade_finished` now rejects a null upgrade
    definition). No SNaN appears in any fixture; the harness writes no code.
