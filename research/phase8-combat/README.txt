PHASE 8: ORIGINAL COMBAT PIPELINE (rounds 1 to 3)
=================================================

Everything below executes the ORIGINAL AA.exe instructions (sha256 5d2548a2...0d26)
under Unicorn in isolated 32-bit memory; every executed instruction is byte-checked
against the accepted v14 executable (3bda77d7...c5c1). No game was launched and no
original file, save or profile was written. Rust replays every fixture.

Files
-----
inventory.json        220 native combat functions: address, role, area, status,
                      Rust location, oracle, Rust citations (build_inventory.py).
                      Counts after round 2: proven 152 (this crate pair),
                      proven-other 27 (aa-economy, aa-world, aa-mind, aa-spatial,
                      aa-actors), fragment 2, seam 24, not-ported 15 (12 of them
                      are AI states routed to the AI agent; 45E090/4BBF90 blocked;
                      543800 is the triggers agent's).
settings-index.json   The 132 game settings registered by 5065D0 with their
                      settings-cache offsets (B39AB0 table). Used to find every
                      consumer of the combat settings (Morale*, ROE*, Experience*,
                      AttackEfficiency*, CaptureHealthFactor, BombardTargetTimeout).
harness.py            Shared Unicorn harness (seams, f32/80-bit returns, byte pins).
dump_asm.py, asm/     Read-only instruction dumps used for the ports.
findings.json         Fixture/producer/harness/Rust source pins (record_findings.py).
requests-to-lead.md   Cross-owner requests (one 2-line aa-mapworld test patch).

New ports and their oracles (PC24 0x7F, PC53 0x27F, PC64 0x37F unless noted)
--------------------------------------------------------------------------
1. Organization morale = the engine's suppression mechanic
   (crates/aa-combat/src/morale.rs; native_morale.py, 711 fixtures)
   578140 state (0 exhausted, 1 wavering via Mind +5C, 2 steady: Mind busy,
   maximum <= 0 or morale > breakpoint); 5786D0 gain (f32 store, clamp to max,
   clear state 5 mind_exhausted unless exhausted); 579660 loss (gates: session
   mode 6, destructed, actor+104 bit2, modifier flag 8 except kinds 3/4, busy
   Mind; clamp < 0 to +0; transition calls 51F9F0/51F920 and state 5);
   579750 signed modify (nonpositive -> exact negation -> loss); 578F10 slayer
   (four ordered actor-value calls, f32 victim value, clamp, *SlayerBase, extended
   add) with 4CEB40 selection; 579010 breaker; 579AB0 retreat penalty; 57A0A0
   tick (replenish+supply bonus*dt f32, modifier +44; engagement loss
   ((FactorMax-1)*(1-living/slots)+1)*Loss*dt f32, modifier /+48, modifier
   re-queried, latch always rewritten); 5785D0 attack efficiency approach;
   4CA050 MORALE_GAIN/LOSS effect (real 4C8330/4CE6C0); 579600 engagement
   experience (real 5718B0/571820); 4CEBE0 capture reward (real 5719E0).
   Callbacks mutate morale/maximum/Mind to prove reread order.
2. Experience amounts (crates/aa-body/src/experience_amounts.rs;
   native_experience_amounts.py, 405 fixtures, results compared as full 80-bit)
   571820 ratio, 571960 kill, 5719E0 capture, 5718E0 rout (identical streams
   except the factor), 5718B0 engagement (extended, unclamped).
3. ROE primary damage history (crates/aa-combat/src/roe.rs; native_roe.py,
   1050 fixtures) 570F70 record (primary element only), 570A70 prune (f32
   cutoff, strict <), 570E30 rate (in-order extended sum / length), 570EA0 /
   5707E0 push (capacity is allocation detail).
4. Damage responses (native_damage_responses.py, 198 fixtures)
   capture.rs 4D1120 eligibility (segment owner delegation, flag21,
   NOT_CAPTURABLE 0x400, +94 live GTID, +D4, definition+290 byte);
   death.rs 4F1A90 death-queue push (LIFO head), 584E70 wall member death,
   4F65B0 phase change death drain (fragment: removal list = callback, proven
   empty), 584CD0 record copy; engagement.rs 579490 OnElementDamaged reactive
   engage (578E90, 578470 operating radius with unrounded differences, 4CE640,
   432ED0, 5790F0); shortfall.rs 578660 + 562D10 (one RNG draw per member,
   unsigned conversion, 1/(2^32-1) binary64 scale, -0.25, f*r+f, event kind 4).
5. Combat reactions (native_combat_reactions.py, 81 fixtures)
   reactions.rs 501770 defender abilities with 4C8300 context shared across
   4C9EB0 try-uses; 4D6190 attacked-by list (BombardTargetTimeout + weapon
   definition +20 when the victim Mind accepts it, refresh vs prepend, victim
   organization redirect, attacker organization/outer redirect) with real
   4D3A70/4D1B40; morale.rs 4D3FA0 rout handler (breaker + real 5718E0).
6. Capture attempt (capture.rs 4D5230; native_capture_attempt.py, 17 fixtures,
   integer-only so one precision) delegation, gates, new owner resolution,
   notifications, wall/component88 transfer, +160 ownership change, SAI, the
   0x1A/0x19, 0x2B/0x2A, 0x1D/0x1C statistics with 58F010, then 4CEBE0.

Round 2 ports (lead assignment, 2026-10-14)
-------------------------------------------
7. Organization maintenance (organization.rs; native_org_tick.py, 186 cases)
   57C8E0 TickMaintenance order and gates, 4D7DA0 state 40 targeted, 5788B0 +
   57DC40 current/maximum combat value, 579110 regeneration gate, 4D7CE0 state
   19 out_of_supply sync, 57C800 follow leader (578E90, 4515F0), 5147C0 marker,
   58B150 watched target, regroup timing (RegroupTime +BC), entrenchment
   (state 22, t <= now). Proven sub-steps are callbacks in native order.
8. Member removal (organization.rs remove_member; native_member_removal.py,
   114 cases) 57CD70 with 578540, 45EC60, 45D580(0), the MoraleDeathFactor
   loss (4CF110 result unused), living count, primary/leader reselection
   (first attached else first detached), state 37 missing_primary and the
   owner removal choice 4F5490 / 4D7960 / 4F6760.
9. Under siege (siege.rs; native_siege.py, 213 cases) 4D6660 state 20 and
   4D0440 recently-damaged (strict <, settings +40).
10. Denizen intruders (denizen.rs; native_denizen.py, 57 cases) 56AE70,
   569A80 (attached members via 4364C0; f32 squared distance vs extended
   squared range), 568D80 (517F30/4CF1B0/4D1970/4D0900).
11. Ownership change (ownership.rs; native_change_kingdom.py, 25 cases)
   4D8450 KKC_GActor::ChangeKingdom, including the duplicate recruit-single
   destroy path and the 32-bit visibility mask re-reveal. Kingdom add/remove
   actor and recruit singles are the aa-economy kingdom_registry functions.
12. Damage numbers (presentation.rs; native_damage_numbers.py, 18 cases)
   50D0B0/50D070.
One removal path: death.rs is now producer-only (4F1A90, 584E70, 584CD0);
the +100 drain 4F65B0 and actor removal 4F4C40/4F6760 are aa-world's (tick
agent, research/phase8-tick). The earlier drain fixtures stay as evidence.
Kill-credit relation statistics (58D950) now go through
credit::World::add_relation, bound to aa_economy RelationStatistics::add;
aa-body no longer has a RelationStat type.

FPU note: the game simulation runs at PC53 (Double53, lead correction). All
floating fixtures here cover PC24/PC53/PC64; none of the ported routines uses
FPATAN/FSIN/FCOS/FYL2X/F2XM1, so no fixture depends on Unicorn's non-exact
transcendental emulation. Round 3: 45E090/4BBF90 (combat value) are ported on
the exact SSE2 CRT pow (see ROUND 3 below).

Single stores (lead coordination, 2026-10-14)
---------------------------------------------
commander::State::experience is now an aa_economy::tick::CommanderExperience:
the one store of Commander+0xC (and the FactionData+0x3C cap, exposed through
State::with_maximum / replace_with_maximum). payment::State borrows it and the
kingdom Economy. Statistics (+0x32C) are written only through world callbacks
(credit::World::add_statistic, payment::World::add_statistic) bound to
aa_economy::tick::add_score. The phase6/phase7 commander, payment and credit
fixtures still pass unchanged.

Commands and results (2026-10-14)
---------------------------------
  set PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-combat/native_morale.py               711 fixtures
  python research/phase8-combat/native_experience_amounts.py    405 fixtures
  python research/phase8-combat/native_roe.py                  1050 fixtures
  python research/phase8-combat/native_damage_responses.py      198 fixtures
  python research/phase8-combat/native_combat_reactions.py       81 fixtures
  python research/phase8-combat/native_capture_attempt.py        17 fixtures
  python research/phase8-combat/native_org_tick.py              186 fixtures
  python research/phase8-combat/native_member_removal.py        114 fixtures
  python research/phase8-combat/native_siege.py                 213 fixtures
  python research/phase8-combat/native_denizen.py                57 fixtures
  python research/phase8-combat/native_change_kingdom.py         25 fixtures
  python research/phase8-combat/native_damage_numbers.py         18 fixtures
  python research/phase8-combat/build_inventory.py
  python research/phase8-combat/record_findings.py
  CARGO_TARGET_DIR=target/s-combat cargo test -p aa-combat -p aa-body --release --locked --offline -j 4
     73 passed, 0 failed (aa-combat 40, aa-body 33; all earlier tests included)
  CARGO_TARGET_DIR=target/s-combat cargo clippy -p aa-combat -p aa-body --release --locked --offline -j 4 --all-targets --no-deps -- -D warnings
     exit 0
  Dependents: aa-runtime and aa-mapworld libraries, aa-mind and aa-actors (all
  targets) compile; aa-mapworld test commander_restore.rs needs the 2-line patch
  in requests-to-lead.md.

Proven vs compiled vs untested
------------------------------
Proven by oracle: every inventory entry with status "proven" (124), within the
finite binary32 domain and the fixture shapes, with every seam's arguments and
order checked. "fragment" entries are proven only for their documented part.
Compiled only: none of the new modules; all are fixture-replayed.
Untested: live runtime composition (no adapter binds these World traits yet), the
not-ported entries below, NaN/infinity propagation, native division by zero
(returned as errors: zero member slots in 57A0A0, zero killer value in 578F10's
fourth call), native null dereferences (returned as errors: morale state with no
Mind, capture delegation to a null outer actor), list mutation during the rout
walk.

Gaps (see inventory.json)
-------------------------
- Combat value 45E090/4BBF90: blocked on an exact CRT pow port (numeric owner).
- Seams still to port (owners/next round): 57B5A0 regeneration, 57AEB0 regroup
  attempt, 4D0A80 in-supply, 45D520 entrench delay (definition +2EC divided by
  modifier +40 via 5014D0), 57A640, 5797E0, 4D4CF0, 4CF0F0, 570670, 515F40,
  57C680, 585160/569540/586370 and the component callees of 4D8450.
- AI states (routed to the AI agent): 535240, 5340C0, 5206A0, 523DD0/523EA0,
  51EFB0/51F040, 527240, 52E830, 521E10/521240; the combat-side services they
  call are exposed (roe::rate, engagement, reactions, morale, organization).
- 543800 trigger effect (triggers agent) should call morale::modify.
Weapon selection (5307A0, 52ED30, 52D820, 52EF40), moving target (533C30) and
range/soft range are proven in aa-mind / aa-spatial by their owners.

Removed native caps
-------------------
None introduced. ROE history, attacked-by/engagement lists, death queue and
defender-ability loops use growable Rust collections; native capacity doubling
(5707E0) is not replicated as a limit.


ROUND 3 (2026-10-14): CRT pow, combat value, statistics, regeneration
=====================================================================
Inventory after round 3 (build_inventory.py): proven 183, proven-other 36,
seam 14, not-ported 10 (9 AI states routed to the AI agent + 543800 triggers),
fragment 2; 245 entries.

13. CRT pow (crates/aa-numeric/src/crt_pow.rs; native_crt_pow.py, 11450 pow
    cases + 510 ___libm_error_support cases + 84 dispatch cases)
    _CIpow 0x96D890 dispatch ([0xB4B340] SSE2 flag, MXCSR & 0x1F80, x87 CW &
    0x7F) -> SSE2 entry 0x977A10 -> body 0x977A29 (644 instructions, 81
    blocks). The Rust body is GENERATED by lift_crt_pow.py from the original
    bytes (crt_pow_template.rs holds the hand-written frame): one match arm per
    basic block, every instruction translated to its binary64/integer effect
    under MXCSR 0x1F80, tables 0xAD0BA0..0xAD44C8 copied verbatim (SHA-256 pins
    in the file header). NaN results follow the SSE rules explicitly (first NaN
    operand quieted, default NaN 0xFFF8000000000000), never host NaN payloads.
    ___libm_error_support 0x977111 is ported for every code it dispatches
    (result rewrite 1.0 / x / unchanged, errno 0x21/0x22) with the image's
    default _matherr 0x84D110 (returns 0; [0xB359A4] is never written).
    API: pow_sse2(x, y), pow_sse2_bits, ci_pow(x, y, CrtEnvironment),
    select_path, ci_pow_x87(&Binary, &Binary, env) (fstp qword rounding via
    store_binary64), libm_error_support. The x87 body 0x96D8ED is reported as
    CrtPowError::X87PathUnsupported; non-default MXCSR rounding/FTZ/DAZ as
    UnsupportedMxcsr.
    Oracle: original code, MXCSR 0x1F80, CW 0x27F, both entries (_CIpow via
    exact fld tbyte operands, and the body with 0x977A10's stack layout);
    incoming XMM registers randomised twice per case (results identical).
    Inputs: 66x66 special-value cross product (zeros, denormals, normals near
    1, integers odd/even up to 2^64, inf, quiet/signalling NaNs with payloads)
    plus 3000 random pairs (uniform bit patterns, x near 1 with huge y,
    negative bases with integral y, overflow/underflow/denormal-result
    boundaries, binary32 game-range values). 79 of 81 blocks executed; the two
    unexecuted blocks (0x97811A/0x978139, x == -1.0 inside 0x9780E8) are
    unreachable (that block is entered only with a positive normal x).
    Emulator deviations, detected per case rather than trusted:
    * Unicorn 2.1.4 (QEMU 5) does not quiet a signalling NaN in x87 m64
      conversions (probed: fld/fstp m64 keep 0x7FF0000000000001). Every
      executed fld/fst/fstp m64 is checked; an SNaN operand is corrected to the
      hardware QNaN (Intel SDM masked #IA response) and the row flagged x87q
      (762 pow rows, 40 libm rows; only direct SNaN inputs, impossible in play
      because x87 registers never hold SNaNs).
    * QEMU picks between two distinct NaN operands of an SSE arithmetic
      instruction with the x87 rule; hardware returns the first operand. The
      10 rows where this happens (flag nan2) are compared for NaN-ness and
      quietness only.
    * cvtsd2si stays in range in every execution (checked).
    Rust: 4 tests in crt_pow.rs (cases, libm, dispatch, binary64 store).

14. Combat value (crates/aa-combat/src/combat_value.rs; native_combat_value.py,
    all precisions; the CRT pow runs NATIVELY in this oracle, no hook)
    - 45E090 ability_value (480 cases incl. 121 diagnostic CSV rows): range
      4BE910, power 4BE3F0 (501A30/501AA0), recharge 4BE4B0 (501560), first
      entry 4BE680, SVars CombatValueRangedMinimum /
      CombatValueDefenseWeightRanged, settings 41 and 81..86, manager means;
      two _CIpow calls (range^RangePower, (area*0.5+1)^AreaPower). With the
      manager +4C stream enabled the 17-double CSV row (0x856F00) is returned.
    - 4BBF90 mean_values (108 cases, 16 with the mean assertion): descending
      bisection insertion (869D30 + comparator 4BB020 + 4BB850), trimmed f32
      sums with setting 87 (CombatValueMeanValuesDiscardPercent, _ftol2),
      +74 maximum of 45E6D0, offense/lifetime; the assertion 855D30 and the
      seven 404F60 log lines are reported to the caller. Empty trimmed ranges
      (native NaN means) are rejected explicitly (2 fixture rows).
    - 57E430 performer_value (180 cases): property +28 sum (extended, one
      store), body +18/+14, +6C +28, actor definition +210, both ability
      tables (+14 and actor +B0 +10), modifier 4CF110, maximum.
    - 4618B0 definition_value and 4BB590 actor_data_values (180 cases, 90
      through 4BB590): recursion over uncomputed (-1.0) children, +30C sum,
      +36C integer-weighted sum; CSV prefix/open/close are DefinitionWorld
      seams (diagnostic output).
    - aa-body ModifierState::power (501A30 melee +8 / 501AA0 ranged +C, +4,
      clamp ModifierAttributeClampAdd +1A4): 2412 cases.

15. Statistics class tracking (crates/aa-combat/src/statistics_tracking.rs;
    native_statistics_tracking.py, 1320 cases, all precisions, no seams)
    58F010 track, 58EFE0 untrack, 58EBA0 leave, 58E8B0 enter, 58E890 score-33
    maximum, 58EA80 predicate; 58EE00 is aa-economy statistics::structure_class
    (executed natively here). Every score change is Statistics::add_score
    (aa-economy, the single +0x32C store). capture::AttemptWorld and
    aa-economy RegistryWorld::statistics_track bind to these functions.

16. Organization regeneration and regroup (crates/aa-combat/src/regeneration.rs;
    native_regeneration.py, 480 cases, all precisions)
    57B5A0 slot choice (damaged primary, attached damaged member, absent
    primary, regenerable empty slot), allegiance shortage factor, heal amount
    (562BA0 seam) or respawn progress +50 with 45D5B0/5014E0 and 45D730 and
    spawn 57AD30 (seam); 57AEB0 spawn then RegroupStealHealthRatio transfer
    (561E50 seam) with the 1e-4 minimum.

17. Smaller ports: 45D520 entrench_delay (360 cases), 57A640 recompute_speed
    (+D8 minimum member +6C +28, owner +70 virtual +118; 450 cases),
    aa-body property_api::apply_property (4CEF10) / remove_property (4CEF70,
    12 wrapper cases) for aa-economy's property callbacks (lead request).

Also identified as already ported by other owners (proven-other): 5797E0
(aa-movement formation), 57C680 (aa-actors element_position), 4D0A80
(aa-economy supply), 4D49B0 (aa-actors state::group), 521240/51F920/51F9F0
(aa-mind), 521E10 (aa-mind action_mask::regiment).

Round 3 commands and results
----------------------------
  set PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-combat/lift_crt_pow.py            (regenerates crt_pow.rs)
  python research/phase8-combat/native_crt_pow.py           ~1 minute
  python research/phase8-combat/native_combat_value.py
  python research/phase8-combat/native_statistics_tracking.py
  python research/phase8-combat/native_regeneration.py
  python research/phase8-combat/native_entrench_delay.py
  python research/phase8-combat/native_org_speed.py
  python research/phase8-combat/native_property_wrappers.py
  python research/phase8-combat/build_inventory.py
  python research/phase8-combat/record_findings.py
  CARGO_TARGET_DIR=target/s-combat cargo test -p aa-combat -p aa-body -p aa-numeric --release --locked --offline -j 4
  CARGO_TARGET_DIR=target/s-combat cargo clippy -p aa-combat -p aa-body -p aa-numeric --release --locked --offline -j 4 --all-targets --no-deps -- -D warnings
     (results in findings.json "validation")

Round 3 gaps
------------
- _CIpow x87 body 0x96D8ED (only taken without SSE2 or with unmasked
  exceptions): explicit Unsupported.
- Seams kept: 4D7960 BeginDeath (886 bytes; component +34 dispatch over 31
  components, announcements, zone rasters, removal), 570670 membership detach
  (567650/4CF0F0/7A0C20), 515F40 flare, 57DD90, 54BEA0, 56ACB0, 585A20,
  569540/585160/586370 (denizen/wall predicates and updates), 57AD30 member
  spawn (world actor creation 4FBF40), 50C380 presentation.
- Diagnostic text output (CSV rows via 856F00/405C90, 404F60 logs, 855D30
  assertion dialog) is returned as data, not formatted.

Round 4 (lead assignment 2026-10-15)
====================================

18. Ability definitions (aa-rules abilities.rs / ability_load.rs, transferred
    to this owner). native_ability_effects.py runs the original loaders
    (fixtures native-ability-effect-fixtures.json):
    - Effect dispatch 4BEF80 and construction 4CA880.
    - Every effect kind's LoadData:
      - morale 4CA010;
      - heal 4CAB50;
      - damage 4CCC70;
      - enchantment 4CBBF0;
      - base 4C9FD0;
      - steal-resources 4CA190.
    - Flag/power 4BEE20.
    - Enum registrations: 497AF0, 497A40, 4C1060, 4C11E0, 4CC1D0. All are
      ASCII case-insensitive (405030/96CAD6).
    - Modifier loading 4C4030 (ADD 4FEC70 with its amount-0 rule,
      PERIODIC_DAMAGE 501350, FLAG, RESIST_DAMAGE).
    - EnchantmentProperty LoadData 4C4F90, with the permanent-VFX search
      428A30/428360, VFX lifetime 494F30 and CreateVFX 42C5E0's +64 byte.
    Required properties (4BF990 at 4C00AB) tokenize on "," and look each
    token up immediately (463FA0).
    Tests: aa-rules lib `abilities::loading::native_fixtures::*` (4) and
    `original_data::original_ability_effects_resolve` (363 actors from
    Data.rwd + v14; kinds MORALE_LOSS 96, HEAL 12, MAIN_DAMAGE 327,
    ADD_DAMAGE 7, ENCHANTMENT 11). Both restoration groups now load.

19. BeginDeath 4D7960 (886 bytes): aa-combat begin_death::begin_death, the
    exact orchestration over begin_death::World leaves.
    native_begin_death.py: 160 cases.

20. Walls/denizens (aa-combat walls.rs) and membership detach.
    native_walls.py covers:
    - wall_complete 585160: 24 cases.
    - propagate_under_siege 586370 with refresh_flag13 585AE0: 40 cases.
    - transfer_wall 585A20 / transfer_component_88 56ACB0: 30 cases,
      including link refcounts and node frees.
    - intruder_near_wall 569540 with the 569300 front cleanup: 30 cases.
    - organization::detach_membership 570670: 16 cases.

21. Member spawn 57AD30 up to actor creation 4FBF40:
    regeneration::spawn_member, which builds the CreationDescriptor (vtable
    A25B28). native_member_spawn.py: 40 cases.

22. Performer turret intention 57DD90 / clear 57DEA0:
    performer::track_target / clear_turret_intentions, on OwnedPerformer
    turrets (+4C records) and turret_update_pending (+5C).
    - For each non-active ability whose turret byte +31 is set (4C81E0,
      45DD30), it runs a fresh 4C8300 context (kind, owner, target, target
      +20 or point) through 4C8F20 without range.
    - On success it marks the turret pending at the point, then activates
      updates once.
    native_turret_intention.py: 60 cases, every branch covered.

Identified, not combat:
- 54BEA0: trigger Captured event push; owner aa-triggers.
- 515F40: KKC_Zone::Move by world position; owner zone/detection solver.
Both are documented at their seams and in requests-to-lead.md section 9.

Round 4 commands and results
----------------------------
  set PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-combat/native_ability_effects.py
  python research/phase8-combat/native_begin_death.py
  python research/phase8-combat/native_walls.py
  python research/phase8-combat/native_member_spawn.py
  python research/phase8-combat/native_turret_intention.py
  python research/phase8-combat/build_inventory.py
  python research/phase8-combat/record_findings.py
  CARGO_TARGET_DIR=target/s-combat cargo test -p aa-combat -p aa-body -p aa-numeric --release --locked --offline -j 4
  CARGO_TARGET_DIR=target/s-combat cargo test -p aa-rules --release --locked --offline -j 4 --lib abilities
  (clippy with -D warnings on the same crates; results in findings.json "validation")

Round 4 gaps
------------
- SUMMON effect deferred resolution (4CC890 -> 473AD0/4CC730): explicit
  Unsupported. No shipped actor ability uses it.
- Enchantment modifier kinds beyond FLAG/ADD/RESIST_DAMAGE/PERIODIC_DAMAGE,
  non-empty enchantment required_properties and quoted tokens: explicit
  Unsupported (unused in the shipped data).
- Leaves that stay world services, with owners in requests-to-lead.md
  section 9:
  - from BeginDeath: zone release 514CE0, announcements, component +34
    deaths, 568B70, StartDying binding, removal;
  - 585280 within-wall geometry;
  - 4FBF40 actor creation;
  - 50AC00 Updatable activation.

Round 5 (lead assignment 2026-10-15): special operations
========================================================

Inventory first: special_ops_census.py writes special-ops-census.json, a
text census of the shipped Data:
- 24 generals, 57 AASpecialOp templates and 36 SpecialOps files.
- Effect templates: SUMMON 35, ENCHANTMENT 18, damage 12, morale 4,
  describe/steal 2.
- Modifier templates: MULTIPLY 15, ENCHANTMENT_IMMUNITY 6, ADD 6, FLAG 4,
  RESIST 3, PERIODIC_DAMAGE 2, ABILITY_EFFECT 2.
- Flags: target_location, enchantment, summon_multiple, bombard,
  primary_island_only, summon_unique.
build_inventory.py has a "special operations" area. Its addresses were found
through the 4CA880 jump table (effect_classes.py), the 4FE840 jump table
(modifier_classes.py), calls.tsv and string references.

Native path:
- KKC_FactionUseAbilityOrder 61E190 (orders owner, seam) calls
  KKC_Faction::UseAbility 4CDE00.
- UseAbility admits through 4C8F20 (admission, proven). Cost is 4E24B0
  (aa-economy) and cooldown is the recharge check 4C8A40 (proven).
- It then uses through 4C9EF0 -> 4C9550 at world time (execution, proven).
  Payment is 4E2590 (payment, proven).
- Effect Apply is vtable +8 of each class:
  - morale 4CA050 (proven, round 2);
  - damage 4CA400 (proven);
  - heal 4CA0B0 with 501B70;
  - enchantment 4CA130 with 581280/5032D0, 581700, 501610 and 503140;
  - capture 4CA260 (-> 4D5230);
  - steal resources 4CA680 (aa-economy);
  - describe economy 4CC050;
  - summon 4CCA70 with 4CC2C0 / 4CC470 / 4CB110, 4C89A0, 4CA7B0 and
    409230.
- Duration and expiry: enchantment instances (vtable A2EBE0) expire in the
  Properties tick 5818D0 once world time - created >= property +60
  (5031B0). Plain instances never expire (6A71F0).

23. aa-rules loading (native_special_ops_loading.py,
    native-special-ops-loading-fixtures.json):
    - The six other modifier kinds through the real 4C4030: MULTIPLY 4FED70,
      ENCHANTMENT_IMMUNITY 5010A0, ABILITY_EFFECT 501150 (nested effect),
      PERIODIC_HEAL 4FEE90, REVERSE_ABILITY 501410, RESOURCE_PRODUCTION
      4FEEC0.
    - SummonEffect 4CC890 with the real deferral 4CAA00/473AD0, then its
      resolver 4CCD60 -> 4CC730.
    - required_properties: 4C4E10 and 4BF990's own block 4C00AB..4C023D,
      run in place, on 283 inputs. They agree with each other.
      - The tokenizer 424FA0/412480 is ported exactly:
        - iswspace below 0x100: 09..0D, 20, A0;
        - plain tokens end only at ',';
        - quoted tokens use either quote character;
        - the walk stops at the first empty token.
      - This replaced a wrong round-4 whitespace splitter.
    - CREATE_VFX VFX lookup is now 4BDB70: the Thing manager plus VFX data
      (virtual +48). Special-op VFX are [Thing] roots.
    - Summon objects use 427BD0 on the Thing manager plus ThingData virtual
      +38 (ActorData 4F0500 = 1, ObjectData/VFXData 4C9FA0 = 0).
    - Original data: all 24 Factions load their 88 abilities through the
      native path (test original_special_operations_resolve).
24. aa-combat::special_ops (native_special_ops_effects.py):
    - Faction use: 40 cases.
    - 501B70: 198 cases.
    - Heal: 30 cases.
    - Enchantment and 581700: 40 cases.
    - Capture record: 3 cases.
    - Describe economy gate: 3 cases.
    - 5032D0/4CEF80: 40+2 cases.
    - Expiry 5031B0: 250 cases.
25. aa-combat::summon (native_summon.py): 160 cases, every branch.
    - target_location, source facing, and random facing (world RNG plus the
      [B39AF8] table).
    - Campaign object choice, ethereal skip, and the open-position search.
    - Object, actor, organization and transported creation; summoned list,
      property, FX event 4 over members, attack orders.
    - Transport far-edge entry in all four directions.
26. aa-combat::walls 585280/5851A0 (native_wall_geometry.py): 400 cases.
    The point is offset by the intruder radius, then tested against the
    convex polygon with a 0.01 cross threshold.
27. Lead additions (native_member_extras.py):
    - membership attach 5705A0;
    - property propagation 5784C0;
    - Performer rank gating 57DCF0 (4BE4D0/4BE4F0);
    - aa-body find_actor_property 4CEF80 / find_instance 5032D0.

Round 5 commands and results
----------------------------
  set PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-combat/special_ops_census.py
  python research/phase8-combat/native_special_ops_loading.py
  python research/phase8-combat/native_special_ops_effects.py
  python research/phase8-combat/native_summon.py
  python research/phase8-combat/native_wall_geometry.py
  python research/phase8-combat/native_member_extras.py
  python research/phase8-combat/build_inventory.py
  python research/phase8-combat/record_findings.py
  CARGO_TARGET_DIR=target/s-combat cargo test -p aa-combat -p aa-body -p aa-numeric --release --locked --offline -j 4
  CARGO_TARGET_DIR=target/s-combat cargo test -p aa-rules --release --locked --offline -j 4 --lib abilities

Round 5 gaps
------------
- ModifierState tables for the non-numeric kinds are not ported. These are
  502B30 cases 3/4/6/7/8/9, 502C60 removal, and 4FE6F0 (an added immunity
  purges hostile enchantments of its type). Also not ported: 502AD0 ability
  effects (+8C), 502560/5015C0 periodic damage (+98), periodic heals (+A4),
  502640 reverse abilities (+B0) and 501F60 resource production (+80).
  Loading is complete; the numeric/flag/resist kinds already apply through
  502A70. aa-runtime reads ModifierState.other_tables[0] as the production
  table (see requests).
- 4CC730: a recruit actor with a non-flying transport depends on the
  deferred-list order and is rejected explicitly.
- Seams with owners in requests-to-lead.md section 10.

Round 6 (lead approval 2026-10-15): ModifierState table kinds
=============================================================

28. aa-body modifier_tables (native_modifier_tables.py,
    native-modifier-tables-fixtures.json): 120 random add/remove sequences.
    The real 502B30 / 502C60 run on modifier objects with their real
    vtables. What executes natively:
    - 483A20 immunity push, and 4FE6F0, the immunity's +20 hook. It purges
      hostile enchantments of the immune type: virtual +4 == 1, definition
      +64 equal, actor virtual +138(allegiance) false, then 581810. The scan
      restarts from 0 after each removal.
    - 502AD0 / 502700 / 5021E0 and 501EF0 (ability-effect lists).
    - 502560 / 5015C0 and 501820 (periodic damage records).
    - 766DE0 and 5018C0 (heals).
    - 502640 / 4C86D0 and 5023B0 / 4CE080 (reverse abilities).
    - 501F60 / 50B530 / 45F180 (production). The table is zero-filled
      whenever its size differs from the registry count; add is mod + prod
      and subtract is prod - mod, both binary32.
    Tests:
    - tests/native_modifier_tables.rs pins every table snapshot, purge
      event and the remaining actor list per step.
    - tests/property_tables_integration.rs drives the real property_flags
      insert/remove. The purge recursively removes through 581810's path,
      and removal unwinds every table.
    Changes:
    - ModifierState.other_tables is replaced by typed tables: production
      (+80), ability_effects (+8C by AbilityType index), periodic_damage
      (+98), periodic_heals (+A4), reverse_abilities (+B0). Immunity stays
      in immunity_types (+68).
    - property_flags::Modifier::Table carries the six kinds.
    - property_flags::Effects gained resource_count, refresh_actor_4cf0f0,
      create/destroy_reverse_ability, enchantment_type and allied. Their
      defaults are explicit errors.
    - Instance gained enchantment_allegiance (503140, Instance::enchantment).
      It is not serialized for plain instances.
    - aa-combat periodic::periodic_from_tables projects +A4 / +98 for the
      5818D0 tick (getters 501690..501710).
    - tests/native_properties.rs compares the 502750 constructor fixture
      through the typed tables, mapping their five empty vectors to the
      fixture's former other_tables.
    - aa-runtime/src/economy_values_world.rs (economy agent's file, edited
      with the lead's approval): production_delta now reads
      ModifierState.production through aa_body::economy_modifiers::
      production_delta (5019D0) instead of other_tables[0]. Minimal edit,
      no other change.
29. Summon seams now name their existing ports:
    - 554B30: aa_spatial::placement::find_placement_within.
    - 552640: aa_movement::falling::clamp_to_interior.
    - Random direction: aa_movement::formation::random_direction with the
      GWorld+0xD4 draw.

Round 6 commands and results
----------------------------
  python research/phase8-combat/native_modifier_tables.py
  python research/phase8-combat/build_inventory.py
  python research/phase8-combat/record_findings.py
  CARGO_TARGET_DIR=target/s-combat cargo test -p aa-combat -p aa-body -p aa-numeric --release --locked --offline -j 4
  CARGO_TARGET_DIR=target/s-combat cargo test -p aa-runtime --release --locked --offline -j 4 --test economy_values_world
