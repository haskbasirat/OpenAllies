Phase 8: original economy, production, research, construction, supply
========================================================================

Everything below executes the ORIGINAL AA.exe (sha256 5d2548a2...0d26)
instructions in isolated x86 memory (Unicorn); every executed instruction is
byte-checked against runtime-expanded-v14 (3bda77d7...c5c1). Intercepted calls
(seams) are recorded with their arguments and order; their bodies are not
executed. No game was launched, nothing original was written.

FPU policy
----------
The simulation runs at PC53 (Double53, CW 0x027F): TGC_NetNode::Update runs on
its own TGC_Thread with the Windows initial control word
(research/phase8-tick/fpu-precision.md). Double53 is the game default; every
port below is still proven at all three precisions (PC53 0x027F, PC24 0x007F,
PC64 0x037F) and every Rust API takes the precision from the caller (nothing is
defaulted). No pinned range executes FPATAN/FSIN/FCOS/FYL2X or another
transcendental x87 instruction (checked by disassembling every pin; the CRT
floor pin 0x96D5E0..0x96D7D4 overlaps acos bytes with an FPATAN at 0x96D7C4
that the visited ledger shows is never executed: floor takes its x87 path at
0x977575), so the Unicorn 2.1.4 binary64 transcendental shortcut does not
affect any PC64 fixture here. Status-word exception flags (PE) are not modelled.

Inventory
---------
inventory.json (build_inventory.py): 664 functions found through the Kingdom
stockpile/flow primitives' callers, economy gameplay strings, the GKingdom
vtable 0xA299D8 and economy-updatable vtable 0xA29B50, curated entry points,
their non-utility callees and every function start pinned by an oracle here.
Status: phase8-port 168, ported+oracle-referenced (earlier phases) 197, seam 28
(deliberate external callbacks), research-only 130, referenced-in-crates without
located oracle 5, missing 136 (most are "unclassified" call-graph neighbours
whose role was not analysed). Ghidra missed two construction function starts:
0x5667F0 (command dispatch) and 0x566880 (ConstructionComponent::Tick).

Round 1 ports (proven at PC53 (game), PC24, PC64)
-------------------------------------------------
1. GKingdom economy Tick, complete (crates/aa-economy/src/tick.rs)
   0x4E9C40 Tick: flow/overdraw prefix, then the upkeep-shortage ratio 1 - P/C
     (x87 accumulators over STOCKPILED rows plus min(provided, consumed) * cost
     of each UPKEEP row). Stored at +0x1F0. Event 3 kingdom_upkeep_shortage
     (percent = _ftol(PC(ratio * 100.0f))) only when ratio > 0 and (previous
     ratio == 0 or |ratio - announced(+0x1F4)| > 0.05f); without a new overdraw
     event 4 kingdom_upkeep_repaired if the previous ratio was nonzero.
     PC matters: ratio 0.29f reports 28% at PC53 (game) and PC64, 29% at PC24.
   0x4E12D0 recruit-single regeneration (ActorData+0x2CC rate, one charging
     item, index-0 completion quirk leaves charging = 0).
   0x4CDFD0 Commander experience income (kingdom+0x230; clamp to
     FactionData+0x3C when >= 0; score 44 for positive amounts).
   0x4E37D0 statistics snapshot timer, 0x58ED00 production totals.
   Victory checks 0x4E9960/0x4E9A40 are seams run only when world time >= 1.0f.
   Oracle native_tick.py -> native-tick-fixtures.json. Rust tests/native_tick.rs.
2. Research (crates/aa-production/src/research.rs): 0x584790 request,
   0x5844A0 blocked reason, 0x5845F0 affordability, 0x589F00/0x589830 queue item,
   0x4E2310/0x4E2360/0x4E2390 kingdom reservation, 0x583F30 refund (releases the
   reservation), 0x584120 completion. Oracle native_research.py.
3. Transfers (crates/aa-economy/src/transfers.rs): 0x4E2080/0x4E1630 row add
   with clamp, 0x4E16D0 subtract, 0x473520 primary resource, 0x4CA680 steal,
   0x509840 team command 3 give (display floor(PC(amount + 0.5)): 0.49999997f
   shows 0 at PC53/PC64, 1 at PC24), 0x4E24B0/0x4CDD60 faction ability
   affordability, 0x4E0FD0 XP multiplier. Oracle native_transfers.py.
4. Construction payments (crates/aa-production/src/construction.rs): 0x562FD0,
   0x566110, 0x566360, 0x565700, 0x5656B0. Oracle native_construction.py.
5. Runtime binding (crates/aa-runtime/src/kingdom_economy.rs): tick_state /
   store_tick_state / recruit_singles over the restored GKingdom shell. Compiled
   and unit-tested only.

Round 2 ports (proven at PC53 (game), PC24, PC64)
-------------------------------------------------
6. Statistics, authoritative (crates/aa-economy/src/statistics.rs)
   One type per native object; no other crate may keep a mutable copy.
   * kingdom+0x32C `Statistics { production, scores }`: `Statistics::new(n)`,
     `score(index)`, `add_score(index, F32Bits, X87Precision)` = 0x58E870
     (binary32 store of PC(old + amount)); free fn `add_score(&mut Statistics,
     index, amount, precision)` (also re-exported as aa_economy::tick::add_score).
   * kingdom+0x340 `RelationStatistics { entries: [RelationStatistic; 10] }`,
     each `{ values: BTreeMap<key, F32Bits>, maximum, leader }`. Slots 0..4 are
     keyed by ActorData definition pointers and written by 0x58D8C0 for score
     indices 57..61 (slot = index - 57; callers 0x564F77, 0x570E0F,
     0x583562/0x583585/0x5835EB/0x583646); slots 5..9 are keyed by kingdom
     pointers and written by 0x58D950 (slot = index - 52; natively only index
     0x3C = 60 from kill credit 0x4D5B92/0x4D5BAF). API:
       RelationStatistics::add(index 52..=61, other: Option<u32>, amount, pc)
         = 0x58D950 (None = native null key: no-op)
       RelationStatistics::add_by_definition(index 57..=61, key, amount, pc)
         = 0x58D8C0
       entry(index) (slot index-52), definition_entry(index) (slot index-57).
     The value is stored binary32 (FST) and the unrounded sum is compared with
     the maximum (> replaces maximum and leader), so rounded ties can still
     change the leader. aa_body::credit::RelationStat should be a read-only view
     of `entries[index - 52]`; combat writes through `add` (as its add_statistic
     binding already documents) and ordinary scores through `add_score`.
   * Construction/upgrade completion statistics: `StructureFacts`,
     `structure_scored` (0x58EAB0), `structure_class` (0x58EE00),
     `construction_completed` (0x564ED0: score 19, class 1/2/3 -> 27/38/34,
     add_by_definition(59, definition, 1.0) unless actor+0x90),
     `upgrade_completed` (0x564E10: score 20, class 2/3 -> 39/35).
     The actor virtual +0x100 is called up to twice natively (58EAB0, 58EE00)
     and is modelled as a pure fact.
   Oracles: native_statistics.py (129 fixtures, 5829 executions incl. 58D8C0
   and the slot alias case), native_completion_statistics.py (180 fixtures,
   1590 executions). Rust: tests/native_statistics.rs,
   tests/native_completion_statistics.rs.

7. Supply (crates/aa-economy/src/supply.rs)
   0x58AE10 SupplyComponent Tick (updatable at component+8, vtable 0xA40098
   slot 5 = thunk 0x58B120): relay search over candidates not out of supply,
   0x58ADB0 relay chain (cycles reported as unsupported: the native loop would
   hang), 0x4D7CE0 state, relay stockpile drain (stock -= seconds while out of
   supply, floored at 0, economy refresh when depleted) or recharge
   (PC(rate * seconds + stock) clamped to definition+0x428, rate =
   SupplyStockpileRechargeRate), then zone activation via 0x58ACE0 /
   0x58AC70 (clear zone, or set zone at position/radius when dirty).
   0x4D0A80 in supply, 0x4D0C90 stockpile in supply, 0x56F2B0
   EconomyComponent::Tick (definition+0x348 off -> unschedule 0x50B1C0(1,0),
   else 0x4D7CE0). Spatial queries, zone rasters and state transitions are
   SupplyWorld callbacks. Oracle native_supply.py (270 fixtures, 2064
   executions). Rust tests/native_supply.rs.

8. Research requirements and kingdom properties
   (crates/aa-production/src/research.rs)
   0x5842F0 research_requirements(item, kingdom, settlement, actor, message):
   0 met, 1 property requirements met but actors missing (description of the
   missing actors then properties when requested), 2 not allowed (0x4E1890) or
   property requirements missing; 0x4C5250/0x4C5630 evaluate_requirements.
   0x4E3AD0 add_kingdom_property(properties, kingdom, definition, source,
   origin_gtid, notify): allowed-list gate for restricted technologies (log
   only), record always appended (+0x318), first record of a definition
   applies 0x4CEF10 to every kingdom actor (+0x284 order) and raises event 5.
   0x4E23D0 + 0x5034F0 remove_kingdom_property(properties, kingdom, definition,
   origin): first record matching definition and resolved origin; removing the
   active record promotes the next one (effects stay), otherwise 0x4CEF70 on
   every actor and event 6. Oracles native_requirements.py (450 fixtures, 1350
   executions), native_properties.py (360 fixtures, 1857 executions).

9. Kingdom actor membership for ChangeKingdom 0x4D8450
   (crates/aa-economy/src/kingdom_registry.rs). Signatures for the combat port:
     aa_economy::kingdom_registry::add_actor(&mut KingdomRegistry, actor: u32,
         &mut impl RegistryWorld) -> Result<(), EconomyError>        = 0x4E6180
         (with 0x4E4470 counted/statistics part)
     aa_economy::kingdom_registry::remove_actor(&mut KingdomRegistry, actor,
         &mut impl RegistryWorld) -> Result<(), EconomyError>        = 0x4E44E0
         (an actor missing from a list it is classified into is an error: the
         native would corrupt the vector)
     RecruitItemTable::find_single(&self, definition, available_only)
         -> Option<usize>                                            = 0x4E2A60
     RecruitItemTable::remove_single(&mut self, index, force, &mut impl
         RecruitAi) -> Result<Option<RecruitSingle>, EconomyError>  = 0x4E2CB0
     RecruitItemTable::add_single(&mut self, RecruitSingle, &mut impl
         RecruitAi) -> Result<(), EconomyError> / add_plain(definition)
                                                                      = 0x4E4C30
     RecruitItemTable::pair_single(&mut self, index, member, &mut impl
         RecruitPairing) -> Result<(), EconomyError>                = 0x4E1200
         (Organization load 0x4CEA60: item+0xC = member+0x14 GTID, then a
         member Element (+0x80) gets Element+0x14 = item; oracle
         native_recruit_pairing.py, 120 fixtures)
   ChangeKingdom order (native): old.find_single(def, false) -> if found
   old.remove_single(item, force = true); old remove_actor; actor+0xE8 = new;
   if new and an item was taken: new.find_single(def, false) (a duplicate makes
   the native destroy the actor) else new.add_single(item); new add_actor.
   RegistryWorld callbacks: economy add/remove (0x570020 / 0x56F4D0 =
   aa-runtime economy_refresh), statistics tracking (0x58F010 / 0x58EFE0,
   0x58EA80 + score 14), city/tech tracking 0x4E3C70, property apply/remove
   (0x4CEF10 / 0x4CEF70, aa-body), structure properties via 0x4E3AD0/0x4E23D0,
   organization refresh 0x4D1350, counted 0x4E1540, flares (max/oldest 0x4E1ED0).
   Oracle native_registry.py (360 fixtures, 2625 executions).

10. Purchases (crates/aa-production/src/purchases.rs)
   Denizen 0x56D5A0 (air attack on target), 0x56D7C0 (at location), 0x56DA00
   (air recon): ActorData cost copy, 0x4E20A0, message when unaffordable,
   create 0x56CC10, debit only when created, then use. HQ attach cost 0x5735D0
   (organization cost * *(cache+0xC0) per row), gate 0x574A10, attach 0x574C10
   (second quote debited without another check). Transport unload refund
   0x58BBA0. Oracle native_purchases.py (300 fixtures, 1596 executions).

11. Team commands (crates/aa-economy/src/transfers.rs)
   0x509840 type 3 give and type 4 tribute (0x509440 tribute_actor; events
   0x1F/0x20 even when the receiver is defeated) are ported. Other types are
   not economy: 0/1/2/7 flares, 5 resource request (event 0x21 + SAI 0x7A3240),
   6 event 0x22 + SAI 0x7A4B10, 9 chat/report 0x40BF70.

12. Construction completion of round 2 (crates/aa-production/src/construction.rs)
   * BuildActor 0x563160 build_actor(definition, location, facing, tag, world):
     0x6939B0 adjust_placement (definition virtual +0x30 then grid snap
     0x46EE00 when ActorData+0x1C bit 16; facing (0,-1) when bit 17), owner
     read, 0x562FD0 (no owner -> affordable), 0x4E1910 when owned, enemy
     footprint when ActorData+0x430 (message sent whatever the caller), site
     lookup 0x555830 when ActorData+0x4E0 (missing site only logged; creation
     proceeds), creation 0x4F9C10 with BuildParams (+0x10 = builder +0x28,
     +0x34 tag, +0x38 = 1, +0x3C site GTID), and only then the cost quote and
     the debit of the ENTRY owner. The paid cost is not stored.
     Oracle native_build_actor.py (300 fixtures, 1653 executions).
   * Settlement structures: 0x5879B0 settlement_affordable, 0x587B60
     can_build_structure (state 0xD, ActorData+0x174 bit 13, count +0x1C <
     ActorData+0x464[level +0x10] else log, 0x4E1910, enemies (always
     notified) and own zone 0x4F0B70 when checking location, buildable list
     ActorData+0x45C, 0x585500, optional cost), 0x587FD0 build_structure
     (quote BEFORE creation, debit the reread owner, 0x5657A0 -> 0x5655C0
     record_paid(cost, replace = 1), 0x586B40). Oracle native_settlement.py
     (300 fixtures, 1662 executions).
   * Lifecycle: 0x565500 salvage(out, paid, fraction, accumulate):
     out[i] (+)= floor(double(PC(PC(fraction * paid[i]) + 0.5f))) stored
     binary32, fraction = *(cache+0x13C); 0x5658C0 refund_salvage (credit the
     reread owner, zero paid, event 0x17); 0x565A20 sell_structure (cancel an
     upgrade in progress, then unless destroyed refund + destroy); 0x565860
     cancel_command (constructing -> 0x5656B0, upgrading -> 0x565700, disabled
     -> 0x565400 seam); 0x5667F0 construction_command (0x13 cancel, 0x14 sell,
     0x15 upgrade with command+0x14, 0x16 disable 0x564DC0 seam, else false);
     0x566880 construction_tick: dissolving owner (kingdom+0x239, set by
     0x4E98C0 with status 2 / event 0x33) services state 0x16 (disable) or 0x14
     (sell); otherwise 0x4D6660, auto-upgrade when the 0x4CF110 token exists,
     0x501570 auto_upgrade_due (interval token+0x30 >= 0 and PC(world+0xE0 -
     actor+0x1C) > interval), the upgrade list is non-empty and no upgrade runs:
     first definition passing 0x566110(def, 1, 0) gets the unmodified base quote
     (0x45F4D0(out, 0), diagnostic 0x404ED0 when a STOCKPILED row > 0) and
     0x566360; finally state 0x20 = (constructing or upgrading) and
     PC(world+0xE0 - (actor+0x60)+0x20) < *(cache+0x1CC). Oracle
     native_construction_lifecycle.py (273 fixtures, 1467 executions).

Round 3 ports (proven at PC53 (game), PC24, PC64)
-------------------------------------------------
13. Construction completion (crates/aa-production/src/construction.rs)
   0x566AC0 construction_complete(builder, payments, world): bit 21 ->
   0x565B00 construction_finished, bit 29 -> 0x566400 upgrade_finished, else
   completion event 8 unless the actor has component +0x90.
   0x565B00: 0x4D18C0 founds_settlement (ActorData+0x430 set, virtual +0x100
   false, under construction) is read, state 0x15 off; a founding structure
   creates the settlement (0x4F9C10, ReplaceParams mode 0) and joins it
   (created+0x98, 0x586470) or logs and destroys itself; CreateComponents
   0x4D21A0(1) failure logs, sets state 0x15 and destroys; then 0x4E4470 on
   the owner (bind: aa_economy::kingdom_registry::actor_completed), economy
   refresh 0x4CF0F0, AI notice (0xB36528 -> 0x79EA90 -> 0x7B2350), settlement
   refresh 0x584B40, 0x4D1350, 0x564ED0 statistics (bind:
   aa_economy::statistics::construction_completed), event 1 unless +0x90,
   trigger 0x54C050(owner, ActorData), component completion 0x588970 (+0x94) /
   0x58CAB0 (+0x90), builder controller notice (0x4CE6C0, 0x4D11A0, +0x70
   virtual +0x40 when in a settlement) and 0x42A590 WhenBuilt.
   0x566400: 0x4D04B0, group (actor+0x158 GTID) leave, replacement creation
   (mode 4, source = old GTID, definition = ConstructionComponent+0x28); on
   failure rejoin the group, 0x4D7EA0, construct_error_creating and the
   0x565700 refund (cancel_upgrade); on success state 0x1B, created state 0x14 =
   old bit 20, log, 0x564E10 statistics, 0x4D5EA0 state transfer, name copy
   unless 0x4D1930, trigger with the created definition, component hand-offs
   0x588AE0/0x58CB80/0x5698C0/0x5814A0, created 0x4D1350, 0x4F6760(actor, 1).
   Everything outside the economy is a ConstructionFinish callback (actor
   creation/replacement, components, AI, groups, names, triggers, events).
   Oracle native_construction_finish.py (270 fixtures, 1278 executions; a
   null upgrade definition is outside the domain, see section 19).
14. Start mission 0x4E6DB0 (crates/aa-economy/src/kingdom_start.rs)
   start_mission(economy, StartMission, commander, world): stockpile reset to
   *(cache+0x110) (+0 unless > 0) clamped to the maximum when kingdom+0x23A;
   Commander definition+0x30 technologies; credit kingdom+0x258; start profile
   +0x244 (credit +0x20, technology 0x48D040 unless owned); start technologies
   +0x24C (null and owned skipped); nation faction check 0x4730D0 (diagnostic
   0x4E4BC0 only); 0x4CDF10 set_commander_experience(*(cache+0x134)) with the
   [0, max>=0] clamp; status +0x19C = 0, +0x23B = 1. Technologies go through
   StartMissionWorld::add_property = 0x4E3AD0(def, 2, kingdom, 1) (bind to
   aa_production::research::add_kingdom_property). Only exact x87 comparisons
   occur (NaN/infinite inputs are proven too). Oracle native_start_mission.py
   (366 fixtures).
15. Economy-side world seams identified (round-2 list)
   * 0x4E1910 / 0x4E1810 / 0x4E1890: kingdom/team restriction lists - ported
     (crates/aa-economy/src/restrictions.rs: RestrictionProfiles::find,
     actor_allowed, technology_allowed). Profiles are named in world+0x12C
     (team name +0x64, kingdom name +0x348) with +0x34 actor and +0x44
     technology maps keyed by ActorData+8 IDs, ASCII case-insensitive
     (0x405030 hash, 0x96CAD6 _wcsicmp C-locale path), last inserted entry wins,
     null values do not restrict. Bind Construction::kingdom_allows and
     KingdomPropertyWorld::technology_allowed to them. Profile data comes from
     the rules loader 0x505760 (aa-rules owner). Oracle native_restrictions.py
     (360 fixtures, 2046 executions).
   * 0x585500 / 0x5890C0 / 0x586B40: SettlementComponent structure slots -
     ported (find_settlement_structure, structure_matches,
     add_settlement_structure; SettlementBuild::settlement_has_structure =
     find_settlement_structure(def, false, true); settlement_add_structure =
     add_settlement_structure). Attach 0x585F60 stays a seam (component links
     and refreshes). Oracle native_settlement_slots.py (360 fixtures, 2022).
   * 0x565400 enable / 0x564DC0 disable: ported in round 4 (section 17) with
     the actor-state writes as seams.
16. Kingdom load 0x4E8550, economy semantics only (aa-economy, pure)
   aa-mapworld decodes the wire (phase 6 full suffix, phase 7 prefix
   0x4E85C1..0x4E86C3 and suffix 0x4E8EF8..0x4E93A0; its "strategic" and
   "tactical" states ARE the statistics objects +0x330/+0x344). Not applied
   there and ported here:
   * Statistics::load_saved(version, words) = 0x58EE80 (v0: 60 words, scores
     42..46 zeroed, 55 kept; v1: 66 words; returns production rows beyond the
     registry) and RelationStatistics::load_saved(version, by_definition,
     by_kingdom) = 0x58DBE0 (v0 discarded; v1/v2 4/5 groups; unresolved keys
     skipped; leader when value > maximum).
   * KingdomStatistics { active +0x32C, loaded +0x330, relations +0x340,
     loaded_relations +0x344 }: promote_loaded = 0x4E3690, start_world =
     0x4F1E10 per kingdom (0x4E3690 + 0x58EA00 reset_mission_scores 47..55).
   * kingdom_load::loaded_property_records(world_version, sources) = the v7+
     record loop 0x4E8A09..0x4E8C2F: source outside 1..3 (record version 0
     stores -1) ignored with a log, world version < 0x15 discards every valid
     record (first one logs), else kept in order.
   Oracles native_statistics_load.py (180 fixtures, 720 executions) and
   native_kingdom_load.py (360 fixtures, the loop entered with its native
   frame). Recruit items, Commander, nation and named-state blocks of the load
   are wire/restoration (aa-mapworld) and were not touched.

Round 4 ports (proven at PC53 (game), PC24, PC64)
-------------------------------------------------
17. Structure enable/disable (crates/aa-production/src/structure_state.rs)
   enable_structure (0x565400; the API for ChangeKingdom 0x4D8450, which calls
   it on a disabled structure's ConstructionComponent): state 0x1B on,
   recompute_commands (0x565140), then refresh_settlement (0x584E10) when in a
   settlement. disable_structure (0x564DC0, command 0x16): state 0x1E on,
   0x4D1350, then in a settlement clear every structure queue (0x584DE0 ->
   0x589CC0) and refresh_settlement.
   recompute_commands (0x565140) rewrites the command bits of the 64-bit
   actor+0xF0 mask (0x517FA0): under construction cancel(0x13) on and sell
   (0x14)/upgrade(0x15)/disable(0x16) off; otherwise upgrade =
   upgrade_available(true) (0x565090: no +0x90 component, not state 20/21,
   mode 0x1B, ActorData+0x4C0 non-empty, in a settlement only when
   settlement_ready_for_upgrade 0x585F00: count == limit and nothing under
   construction), disable = ActorData+0x4E5 and not besieged (state 20) and
   (no virtual +0x100 parent or in a settlement) and mode 0x1B, cancel = mode
   0x1D without +0x90 or mode 0x1E (no parent or in a settlement), sell =
   ActorData+0x4E4 and not out of supply (state 0x13) and (not besieged or
   AllowSellUnderSiege); a modifier token with bit 0x800 blocks sell and
   disable; a disabled settlement center (0x4CEDA0, its state 30) forces mode
   0x1E. The mode is state_in_group(states, group 0xD mask) (0x4D14C0/
   0x4D0820, lowest set state). Actor-state writes (0x4D64F0), the command
   mask, the state-group table, virtuals, 0x4CEDA0, the option and the queue
   clearing are StructureState callbacks.
   Oracle native_structure_state.py (900 fixtures, the 0x4D64F0 test double
   sets/clears the state bit and is mirrored by the Rust test world).
18. World start economy parts (crates/aa-economy/src/world_start.rs)
   world_start (0x4F1E10, once per world+0x274): statistics promotion and
   mission score reset per kingdom; for every active GTID object in index order
   (0x4EFBE0, virtual +0xB4) actor+0x13C = 0x38 when different and owner
   statistics tracking 0x58F010 when the owner's +0x190 is 0; then a forced
   initial snapshot (0x4E37D0(0, 1): timer +0x334 = +0, 0x58F530 + 0x45F240)
   for every kingdom without snapshots. Oracle native_world_start.py
   (360 fixtures).
19. Emulator caveats check (docs/NATIVE-PORT-RULES.md)
   * FS base 0: harness.py now maps page 0 only for the SEH link and stops the
     run on any page-0 access that is not FS-prefixed to bytes 0..3
     (guard_null_page / check_null_accesses; native_research.py and the
     mid-function native_kingdom_load.py install the same guard). Rerunning
     every oracle found three silent null reads: 0x509A8C (team command give:
     resource display object +0x28 was null in the fixture memory; now set),
     0x6939D8 (BuildActor placement read world+0x30 map; now set) and
     0x5666A9 (OnUpgradeFinished with a null upgrade definition: 0x4F9C10
     dereferences it natively, so those 57 executions are outside the domain
     and upgrade_finished now rejects definition 0 before any effect).
     Fixture comparison before/after: 19 of 20 fixture files are unchanged
     (research's producer hash aside); native-construction-finish lost the 57
     out-of-domain executions (1335 -> 1278).
   * Allocator seams: every allocating path is seamed (0x6F7810/0x6F7430,
     0x856370 string allocator); no native fault was ever accepted as
     behaviour (the out-of-domain exclusions are predicted from inputs).
   * Transcendental x87: none executed (see FPU policy).
   * Signalling NaNs: no fixture contains an SNaN pattern (scanned); the
     NaN inputs used are quiet NaNs moved or compared, never loaded/stored as
     SNaN.
   * Stale code: the harness never writes code; seams are hook redirections
     at distinct stub addresses, so no translation is ever stale.

Tick registration (for the world-integration owner)
---------------------------------------------------
GKingdom::Init 0x4E5EA6: when world+0x2C < 0x16 and kingdom+0x184 == 0 it calls
0x50AC00(0) on the economy updatable at kingdom+0x180 (vtable 0xA29B50, slot 5 =
Tick 0x4E9C40), which inserts (updatable, 0) into the periodic queue at
world+0x1AC through its vtable +0xC; 0x50AD80 (vtable slot 1) is the deferred
variant through world+0x208 (delay, argument). The supply updatable (component
+8, vtable 0xA40098) and EconomyComponent::Tick 0x56F2B0 (component+8) use the
same updatable base; 0x50B1C0(1,0) unschedules. ConstructionComponent::Tick
0x566880 is likewise called on component+8 with two unused arguments.

Commands and results (2026-10-14)
---------------------------------
  set PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-economy/native_tick.py                 414 fixtures, 1008 executions
  python research/phase8-economy/native_research.py             147 case runs, 561 steps
  python research/phase8-economy/native_transfers.py            273 fixtures, 1545 executions
  python research/phase8-economy/native_construction.py         243 fixtures, 1257 executions
  python research/phase8-economy/native_statistics.py           129 fixtures, 5829 executions
  python research/phase8-economy/native_supply.py               270 fixtures, 2064 executions
  python research/phase8-economy/native_requirements.py         450 fixtures, 1350 executions
  python research/phase8-economy/native_purchases.py            300 fixtures, 1596 executions
  python research/phase8-economy/native_properties.py           360 fixtures, 1857 executions
  python research/phase8-economy/native_registry.py             360 fixtures, 2625 executions
  python research/phase8-economy/native_settlement.py           300 fixtures, 1662 executions
  python research/phase8-economy/native_build_actor.py          300 fixtures, 1653 executions
  python research/phase8-economy/native_construction_lifecycle.py  273 fixtures, 1467 executions
  python research/phase8-economy/native_completion_statistics.py   180 fixtures, 1590 executions
  python research/phase8-economy/native_construction_finish.py     270 fixtures, 1278 executions
  python research/phase8-economy/native_start_mission.py          366 fixtures, 366 executions
  python research/phase8-economy/native_restrictions.py           360 fixtures, 2046 executions
  python research/phase8-economy/native_settlement_slots.py       360 fixtures, 2022 executions
  python research/phase8-economy/native_statistics_load.py        180 fixtures, 720 executions
  python research/phase8-economy/native_kingdom_load.py           360 fixtures, 360 executions
  python research/phase8-economy/native_structure_state.py        900 fixtures, 900 executions
  python research/phase8-economy/native_world_start.py            360 fixtures, 360 executions
  python research/phase8-economy/native_recruit_pairing.py        120 fixtures, 120 executions
  python research/phase8-economy/build_inventory.py             664 functions
  python research/phase8-economy/record_findings.py
  CARGO_TARGET_DIR=target/s-economy cargo test -p aa-economy -p aa-production --release --locked --offline -j 4
     aa-economy 18 passed (4 parity + 3 tick + transfers, statistics,
     completion statistics, supply, registry, start mission, restrictions,
     statistics load, kingdom load, world start, recruit pairing); aa-production 21 passed
     (4 queue + 3 research + 2 construction + requirements, purchases,
     properties, 2 settlement, settlement slots, build actor, construction
     lifecycle, 2 construction finish, 2 structure state)
  CARGO_TARGET_DIR=target/s-economy cargo test -p aa-runtime --release --locked --offline -j 4 --lib tick_binding
     1 passed (round 4; the owned aa-runtime files were not changed in rounds 3-4)
  CARGO_TARGET_DIR=target/s-economy cargo clippy -p aa-economy -p aa-production --release --locked --offline -j 4 --all-targets -- -D warnings   clean
Mutation checks: removing the round-1 index-0 regeneration quirk fails the tick
corpus; swapping BuildActor placement bits fails the BuildActor corpus (both
reverted). The FSTP qword step of 0x565500 is modelled but cannot change a
result in the finite binary32 domain (products of two binary32 values plus 0.5
fit in 53 bits), so no fixture distinguishes it.

Proven vs compiled vs untested
------------------------------
Proven by oracle: functions marked phase8-port in inventory.json, within the
finite binary32 domain and the fixture shapes (1..6 resources), with every
seam's arguments/order checked by the Rust replays.
Compiled only: aa-runtime kingdom_economy tick bindings (unit test only); the
new traits are not bound to the live world by any runtime code yet.
Untested / not ported: see gaps.

Removed native cap
------------------
0x48BA89: the stat data manager handles at most 10 resources because production
totals are an inline float[10] at statistics+0. Statistics.production is a Vec
sized to the registry; the cap is recorded, not enforced.

Unsupported domains (explicit errors, never invented results)
-------------------------------------------------------------
Nonfinite inputs; ratio denominator zero; _ftol outside i32; completing a
kingdom-level research record without an owner; upgrade gate or settlement
CanBuildStructure reaching 0x4E1910 without an owner (native null dereference);
supply relay cycles (native infinite loop); removing an actor absent from a
registry list it is classified into; accumulating paid cost across a resize;
statistics loads for a registry larger than the 10 saved production rows or
with versions the native rejects (0x8589B0); a listed settlement structure
without a StructureComponent (native dereference in 0x585500).
Errors after callbacks leave earlier effects applied: no rollback is implied.

Gaps (identified, not ported)
-----------------------------
* World seams (other owners): actor creation/replacement 0x4F9C10, component
  hand-offs 0x588970/0x58CAB0/0x588AE0/0x58CB80/0x5698C0/0x5814A0, settlement
  attach 0x585F60 and refreshes 0x584B40/0x584E10, CreateComponents 0x4D21A0,
  state transfer 0x4D5EA0, groups 0x4D3DD0/0x4D0000, AI 0x79EA90/0x7B2350,
  triggers 0x54C050/0x42A590, actor state 0x4D64F0 (aa-actors), 0x4CEDA0,
  queue clearing 0x589CC0, actor refresh 0x4D6660/0x4D1350, footprint/zone/site/grid seams
  (0x554DD0/0x554330/0x54EFC0, 0x4F0B70, 0x555830, 0x46EE00), cost quote
  0x45F4D0 (aa-actors phase 7).
* Kingdom Load wire blocks (recruit items, Commander, nation, named states)
  belong to aa-mapworld; statistics tracking 0x58F010 itself (combat routes it
  into aa-economy).
* Flare/request/chat team command types (presentation/AI), victory checks.
* Capture (aa-combat), ability cooldowns (aa-combat), recruit-singles restore
  (restoration agent).
