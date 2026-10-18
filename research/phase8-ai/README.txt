PHASE 8 - ORIGINAL AA.EXE ARTIFICIAL INTELLIGENCE (rounds 1 to 3)
================================================================

Scope: port the original AI of AA.exe (image base 0x400000, SHA256 5d2548a2...).
Every Rust routine below was ported from the original machine code and replayed
against the ORIGINAL code executed under Unicorn (also byte-checked against the
accepted v14 image). Nothing here comes from the rejected aa-game simulation.

Files
-----
inventory.json          every identified native AI function (1737 rows) with
                        module, source file estimate, strings, vtable slots,
                        callers/callees, ported/executed/proven flags and a
                        reviewed "phase8" status for addresses touched here;
                        plus 22 SAI goal-class vtables and 41 Mind-state
                        vtables (name, metadata, Think slot +C).
build_inventory.py      regenerates inventory.json (read-only inputs).
ego-fields.json         248 SAI ego fields: ego offset -> TGI key/section/type,
ego_fields.py           recovered from the original ego loader 4B0560.
native_machine.py       shared Unicorn harness (original image, seams, visited
                        instruction capture, v14 byte check, null-page guard
                        that only admits FS-prefixed fs:[0] accesses).
native_*.py / native-*.json   oracle producers and fixtures (below).
record_evidence.py      regenerates all fixtures + inventory + findings.json.
findings.json           fixture counts/hashes, producer and Rust source pins.
disasm.py               capstone listing helper for original functions.
loader_listing.py,      call listing / getter-store lifting helpers for the
lift_loader.py          SAIData loaders (read-only).
native_data.py          SAIData oracle base: emulated ATL CStringW manager,
                        seamed data-node getters answering from recorded
                        answer trees, 48D500 registries, 410C40 data errors.
sai-shipped-inputs.json answers the SAI profile loader reads from the SHIPPED
sai-shipped-egos.json   data, and those of the ego loader (75 [Ego] roots),
                        recorded by crates/aa-mind/examples/dump_sai_inputs.rs
                        (aa-rules native-order load of ../Data.rwd + v14).
requests-to-lead.md     changes needed outside this agent's files.

Inventory summary (mechanical flags; see inventory.json "phase8" for review)
------------------------------------------------------------------------------
module            range             functions  bytes   vtable-only*  ported executed proven
SAIData           4A5000..4B2830       151     51949        1           5       4       1
Tai (AAI/CAI/Mind/UAI) 515A00..539C40  566    130677       55         146     140     108
SAI               79D300..7D0A70       799    202944      109         123     128      95
MGAIData          95FD70..960E00        21      3957        0           9       6       4
MGAI (campaign)   962700..96AD80       158     32414        1          53      85      13
MindStateMetadata 9CDF00..9D0100        42      7898        0           0      41       0
* code referenced only from vtables, absent from functions.tsv (no Ghidra
  pseudocode exported): disassemble with disasm.py or decompile with REA.
"proven" = cited in crates/*/src, executed by an oracle fixture that a Rust
test replays (all agents' crates are scanned: MGAI rows are the metagame
agent's work; several Tai/SAI rows are phase7 or movement-agent work).

Architecture facts established from the original code
-----------------------------------------------------
* KKC_SAI::Think 7A0080 runs the goal engine (79F660), RAI (79EF60) and MGAI
  (79E830) as Win32 fibers with wall-clock budgets from QueryPerformanceCounter
  (59ABA0). The SAI constructor 7A0AA0 seeds the SAI RNG ([B36528]+2C) with that
  counter (85D8A0). SAI output leaves only as orders (KKC_SAICommandData::Send
  7AF220). => SAI evaluation is host-local, non-lockstep input; it may run on
  its own thread in Rust as long as orders enter the simulation like native
  orders. Its RNG is NOT the world RNG. The fiber yield 79DF70 is a service.
* Tai (unit/regiment/aircraft Mind states) runs inside the simulation and uses
  the world RNG ([B36534]+D4 via 4F02B0): it must stay deterministic.
* FPU: the simulation runs at PC53 (TGC_NetNode::Update on its own thread, CW
  0x027F; research/phase8-tick/fpu-precision.md) - the production precision
  for every Tai routine here. The SAI think runs in fibers of the thread that
  calls KKC_SAI::Think; its CW is not separately proven. Every oracle case is
  still produced and replayed at PC24, PC53 and PC64 (Single24/Double53/
  Extended64), so the routines are proven at whichever precision applies.
* Native constant 0x9FDA98 used by 45D400 and its inline copies is the binary64
  nearest 1/(2^32-1) (= 2^-32 + 2^-64), not 2^-32 (caught by the Double53
  rout oracle; aa_mind::x87::unit_random).
* Ego GoalPriority records are inline at ego+4C+type*18 for the 23 goal types
  (7BDB70 enum) and end exactly at ego+274 (ResourceManagement).
* Difficulty handicaps are data-driven kingdom properties (Data/Game/
  handicaps.tgi, Properties/properties_handicaps.tgi: RECRUIT_COST and health
  multipliers -/+25% for easy/hard) applied through the generic property/
  modifier system (KKC_HandicapData 48D040/48DB60, SetAIHandicap order
  699B70/69B820). SAI profiles carry difficulty_level easy/hard (7BDCF0).
* SAI goals and actors: a goal is a {status +8, actor list +C/+10/+14, avg
  travel +30, current priority +34, base priority +38} object with 34 virtual
  slots; 4 military classes override +24/+28/+2C/+64/+68/+6C, the rest keep
  the base bodies ported in sai::goal_base. Actors are intrusive refcounted
  objects (+4) with owner goal +10, cached travel +14, category mask +18/+1C
  and route +30; the player keeps 17 available-actor pools (+54) and a
  withheld list (+38).

Ported and oracle-proven (Rust tests replay every fixture case)
----------------------------------------------------------------
native            Rust                                        fixture (cases)
Round 1
7BDDD0/7BDB70     aa_mind::sai::tables::GOAL_TYPES             native-sai-tables.json (29 entries)
7BDCF0            aa_mind::sai::tables::DIFFICULTY_LEVELS      (same)
7BDEA0            aa_mind::sai::tables::GOAL_ACTOR_PROXIMITY   (same)
7A5200            aa_mind::sai::goal_priority::finalize        native-goal-priority.json (1356 incl.)
7A3860            aa_mind::sai::goal_priority::affordable      (same; 660 direct calls)
45D400 (inline)   aa_mind::x87::unit_random                    (via 7A5200/52A930/52B530)
52A930            aa_mind::rout_score::score                   native-rout-score.json (318)
52B530            aa_mind::rout_select::select                 native-rout-select.json (165)
52BB40, 58AC70    aa_mind::recover_select::{select, SupplyComponent::active}
                                                               native-recover-select.json (150)
521240            aa_mind::cai_rout::think                     native-rout-think.json (228)
51E110 52DF30 51DE10 51E260 523930
                  aa_mind::cai_states::{retreat, deploy, regroup, exhausted, chasing}
                                                               native-state-thinks.json (67)
52A340 529E60 52A000 52A1F0
                  aa_mind::{pop, think_base, is_current, within_arrival}
                                                               native-mind-primitives.json (108)
Round 2 - SAI goal engine
96CE20/96CDB0     aa_mind::sai::crt_qsort::qsort (VS2003)      native-goal-qsort.json (52, 13115 compares)
7A30B0            aa_mind::sai::engine::base_priority_order    (same)
7A7700            aa_mind::sai::engine::reevaluate_goals       native-goal-engine.json (255, 6 faults)
7A93B0            aa_mind::sai::engine::select_goals           (same)
7A9150 7A7E30     aa_mind::sai::assign::{evaluate, assign}     native-goal-assign.json (606, 18 faults)
7A35D0 7A3740     aa_mind::sai::assign::{evaluate_required, assign_required}
7A7EB0 7A6C70     aa_mind::sai::assign::{gather, assign_from_pool}
7A3E10 7A3F30     aa_mind::sai::assign::{take_from_lower, take_from_higher}
7A50E0 7A33E0     aa_mind::sai::assign::{trade, swap}
7AFA40 7A1120     aa_mind::sai::assign::release_from_pools
56A1F0 7A6480     aa_mind::sai::assign::ActorList::{remove_first, remove_null}
                  (+7A2FB0/7A10E0 predicates, 473300, 7A1030 inline)
7A3B00            aa_mind::sai::goal_base::priority_with_actors native-goal-base.json (640, 36 faults)
4A7130 (+96D890)  aa_mind::sai::goal_base::travel_weight (CRT pow through
                  aa_numeric::crt_pow, SSE2 body executed natively)
7A4FB0 7A5020 7A5080  aa_mind::sai::goal_base::{swap_value, value, held_value}
7A6930 7A6A90 7A67B0  aa_mind::sai::goal_base::{add_actor, remove_actor, reevaluate}
7A5600 7A3D30 7AFB00  aa_mind::sai::goal_base::{deselect, release_all, return_to_pool}
7A0F80 7ACF50 59E240  aa_mind::sai::goal_base::{set_owner, actor_count}
Round 2 - combat-facing Mind/company states (combat agent seams)
521060 523DD0 524C10 51EFB0 51F040 523EA0 51FE60
                  aa_mind::cai_engage::{can_add, add_target, mind_add_target,
                  remove_target, clear_targets, refresh_targets, insert_sorted}
                                                               native-cai-engage.json (150, 5 faults)
51F9F0 51F920 5206A0 52E830 527240
                  aa_mind::cai_reactions::{exhausted, wavering, kill_on_enter,
                  engage_reaction_delay, target_priority}      native-cai-reactions.json (248)
5340C0 535240     aa_mind::uai_kill::{choose, think}           native-uai-kill.json (840, 96 faults)
Round 2 - Mind-state Thinks
51F190 (+52C0D0)  aa_mind::cai_rout::enter (rout OnEnter)       native-rout-enter.json (80)
524D70            aa_mind::cai_sleep::think                    native-sleep-think.json (400, 35 faults)
526A70 (+4364C0)  aa_mind::cai_engage_think::think             native-engage-think.json (150, 3 faults)
5294F0 (+529C20)  aa_mind::cai_guard::think                    native-guard-think.json (500, 10 faults)

Round 3 - SAIData loaders on the shipped data
4AA430, 4AE3E0 (+47D960, 473AD0, 497220)
                  aa_mind::sai::profile::{SaiProfile::new, load, base_load}
                                                               native-saidata-profile.json (175, 84 faults)
4AB9A0 4A8E00 4A5C60 4B0560 and every sub-loader: 4AD980 Filters, 4AF500
GoalPriority (+4A5D60/4A5D80), 4AF230 ResourceNeed, 4AAD80 ComponentValue,
4AF8E0 ActorPriority, 4A66E0 exploration goals, 4ADF50 RecruitPreference,
4AB090 SpecificBuildingBonus, 4AAEB0/4A7C70 SpecificRecruitRequest,
4AFD50/4AFB60/4AA1D0 general powers, 4A8700, 4AEDC0 formations, 4AF0C0
GoalFormation; containers 4A86D0/7BA3C0/606750/7A6430 (hash), 45F180,
4A6C40, 766DE0; CRT qsort 96CE20 with 4A5C20
                  aa_mind::sai::{ego, ego_lists, native_hash}  native-saidata-ego.json (495, 125 faults)
(GoalBase bridge for the lead: aa_mind::sai::bridge, tests/sai_bridge.rs.)
7A1C10, 7A1B30    aa_mind::sai::{travel::travel_time, travel_cost}
                  (route block = aa_navigation::route_cache::route_cost via
                  travel::RouteServices)                       native-travel-time.json (600)
The ego fixture holds all 75 shipped egos at the three precisions (full
decoded ego for Double53) plus 270 answer-patch mutations covering every
validation and lookup error of 4B0560 and its sub-loaders and the value
paths (binary32 rounding before/after the range checks, ms conversion, the
binary32 1/60 scale of min_seconds_solvency, the EconomicExcess sort, hash
growth). Events (every getter, registry, Thing and Property lookup and data
error, in native order) and decoded results are compared as sha256 digests
of canonical JSON. Ego facts: the "Invalid actor value" check compares the
unrounded distance_mult_min with the stored max and 0; Teamwork/congruency
multipliers check the unrounded value against [0, 1], Recover fractions
check the STORED binary32 against 1; WallStrength fills new levels with -1
and treats a positive (or zero above level 0) entry as a duplicate;
GeneralPower ActorValue reads consider_friendly/enemy/owned from the CV
section (native quirk, kept); GoalPriority ActorPreference lookups store
null for unknown Things/Properties without error; RecruitPreference looks
Things up through [[B36530]+0C], everything else through [B39F58] (the same
Thing DataManager, constructor 4BD850); 47D960 sets the DataItem +14 byte.
Shipped data: every one of the 25 profiles and 75 egos loads (no data error).

Round 3 - SAI dynamic map analysis and Mind member-added
7AC0D0 (+7AA5B0, 7A9C60/7A9C80, 7B9180, 7BB480, 7BAC00, 7AE350, 7AD890,
7ADDA0, 4A5E40, 96D890, 7ABA70, 7AB940, 7AA620, 45D720, 7B9050, 7BB780,
7BB820, 5C37E0)   aa_mind::sai::map_analysis::dynamic_map_analysis
                                                               native-map-analysis.json (400, 54 faults)
524B80 (+4D0900)  aa_mind::cai_member_added::member_added      native-member-added.json (200)
7A0C20            aa_mind::sai::actor_notify::notify (the SAI hook of
                  organization attach 5705A0 and the combat special-operation
                  / summon path)                               native-actor-notify.json (680, 104/104)
Map analysis facts: per region, the static lists (+34) spread every live
actor's combat value (scaled per SAI player by the 7AE350 learning
multiplier, which prunes and erases the player's +170 learning lists) into
entry +14 within its settlement range (+B0 +2C, else the +148 slot 5 +8;
times settlement_cv_range_fudge_factor) and into entry +20 within the ego
border_influence_range; the mobile lists (+28) spread into entry +8 within
speed (capped by DynamicMapAnalysisSpeedCap; the cap log reads the
definition +8 name) * ego cv_influence_max_travel_time. 7AA620 recurses over
the neighbour hash (+5C) through land regions (first rectangle terrain 0)
with factor max(1 - d/range*(1-min_cv_propogation_factor), min) ^
cv_influence_exponent. The fiber yields (79DF70) after each collection and
each processed actor; 7AE350 uses the ego's recruit_duration (+940) as the
learning window. Coverage: 7AC0D0 627/642 (the rest: allocation failure,
alignment padding, a GTID going stale between yields), callees 100% except
the unreachable min-factor clamp of 7AA620 and erase-miss returns of 7ADDA0.
UAI collision results: element_move_continuation::{collision_plan,
stop_distance} bind 534A40/536AD0 to aa_navigation::uai_collision
(movement-proven; this glue is compiled, not separately replayed);
Payload::word78 is UAIStateMove +78 (last low-path solve time).

Round 3 - remaining Mind-state Thinks (ported by the two helpers, integrated)
Regiment (aa_mind::cai_thinks::*, harness native_cai_regiment.py):
520730 kill 400 cases 50/50; 51DB80 construct 200 56/56; 51DE70 reform 360
81/81; 5267E0 rampage 600 (2 faults) 133/133; 527090 pack 160 133/133;
526E60 unpack 240 (7) 185/185; 521850 join 400 (4) 216/220; 525930 siege 900
(1) 288/288; 5239A0 repair 700 275/276; 5207F0 bombard 800 334/334; 525D70
explore 900 (4) 382/382 (+ siege_target, stack_action, unit_order helpers).
Element (aa_mind::uai_thinks::uai_*, harness native_uai_harness.py):
536E10 engage 600 99/99; 531AD0 join 240 (4) 54/54; 534000 reattach 160 (19)
56/56; 52F440 unload 330 (29) 88/88; 5359D0 repair 714 (23) 158/158; 52F950
sleep 762 201/201; 5353E0 bombard 909 (62) 402/402; 5372B0 unpack 540 (17)
211/211.
Aircraft (aa_mind::uai_thinks::aai_*): 5180B0 recon 40 30/30; 519F70 queued
120 84/84; 5185D0 transport 120 (8) 99/99; 51C900 guard 906 252/253; 51CC10
escort 900 (26) 254/255; 518D60 join 909 285/294; 519150 takeoff 1020 (22)
450/451; 519750 landing 1020 590/592; 51B8C0 attack 1200 (32) 589/592
(ported by this agent after the helpers stopped). Tests: cai_<name>_native,
uai_<name>_native, aai_<name>_native. Each module lists its seams.

"faults" are cases where the ORIGINAL code dereferences null (or reaches an
assertion handler); the Rust port returns an Err at the same point after the
same recorded calls. Instruction coverage of the round-2 bodies: 7A9150/7A7E30
100%, assignment callees 89-100% (missing only destructor-at-zero paths and
already-asserted latches), 7A3B00 193/194, 524D70 293/293, 526A70 278/311
(the SVar static initialiser is pre-initialised), 5294F0 496/530 (static
initialisers pre-initialised, one removed-object fallback).

Combat-facing APIs (for the combat agent's seams)
* 579490 damage-driven target acquisition -> aa_mind::cai_engage::mind_add_target
  (524C10; returns false after adding to an active engage list).
* morale -> aa_mind::cai_reactions::{exhausted (51F9F0), wavering (51F920)}
  (both activate the rout state with 52A810 mode 2 unless blocked).
* 5206A0 kill OnEnter, 527240 target priority (1..6), 52E830 engage reaction
  delay (min 1e-4) in cai_reactions; 5340C0/535240 UAI kill in uai_kill
  (ChooseWorld::roe_rate binds aa_combat::roe::rate).

Behaviour summary (round 2)
* 7A93B0 sorts candidate+selected goals by base priority (CRT qsort,
  comparator 7A30B0), evaluates each (7A9150) with a running resource copy,
  reserves costs of selected goals and re-positions by current priority, then
  assigns actors (7A7E30) in a second pass with a fresh copy.
* 7A9150: expired -> deselect; a goal naming a required actor claims it
  (7A35D0: free -> pools released, else taken from an owner valuing it below
  this base priority); else 7A7EB0 gathers positive-value actors from its pool,
  then steals from lower goals offering spares (7A3E10) and from higher goals
  while each actor is worth more here (7A3F30, first refusal stops); then
  current = affordable ? (float)7A3B00(0,0) : 0 when base > 0; selected iff
  current > 0 (unordered counts as positive).
* 7A7E30: required-actor goals hand the actor to a selected goal valuing it
  above this base priority (7A3740); pooled goals take positive-value pool
  actors (7A6C70), refresh current, then trade both ways with every later
  selected goal (7A50E0 moves, else 7A33E0 swaps the pair with the largest
  positive combined +24 swap value, x87 sum rounded per PC) and deselect
  goals whose current is not positive or whose +44 holds.
* 7A3B00: f32(+7C(add, remove)), then (sum + v) * product over the goal
  type's SAIData modifiers applied to every live non-flag4 actor of the
  hypothetical set, times 4A7130(hypothetical average travel):
  pow(clamp(1 - t/norm*0.8, 0.2 (unstored test), 1 (stored test)), exp).
* 524D70 sleep Think: schedule 2 s; exhausted reaction; follow target (+190 /
  +188 / follow state 51D920 mode 2 + think); home (4D7960 / 56AEC0 / +F4);
  arrival list joins (522890); sleep point to the organization (57BF30, reset
  to the null point; fld/fstp quiets an SNaN); resume pending commands
  (52BF30) or activate Mind+54 (mode 2) on flag 310; 522F60/523000 by +1C/+10.
* 526A70 engage Think: wavering reaction in status 1 with +118; lost tail
  target -> +140; new attacker (+4C) recorded in +20 and, with
  KillTargetIsHighestPriority, added in range (524C10) or members +17C;
  chasing state 520D90 may hand over to +F8; members +F0(target); +1C = every
  member Mind attacks a followed target; schedule 1 s.
* 5294F0 guard Think: see the module header (threat list sources, binary32
  squared distances, current-target bias max(sqrt(d2)-GuardOldTargetBias,0)^2,
  engagement via 527CE0 or 51D920, home handling, sleep fallback).

Emulator findings (relevant to every agent's oracles)
* FS base is 0 in the harness, so a read of [0..3] (a null pointer's first
  field) silently returned the SEH chain pointer. native_machine.py now faults
  every page-0 access whose instruction lacks the FS (0x64) prefix; all 13
  round-1/2 fixtures were regenerated byte-identical after the fix.
* Unicorn 2.1.4 does not quiet a signalling NaN on fld m32 (hardware raises
  masked #IA and loads the QNaN, Intel SDM). The sleep oracle avoids SNaN
  inputs; Rust follows the hardware (cai_sleep::think, x87_copy).
* Unicorn computes FPATAN/FSIN/FCOS/FYL2X through binary64 and never sets
  PE: the 522 captured 445400 values in native-rout-score.json are not 80-bit
  ground truth. The production binding is aa_mind::rout_score::native_angle
  (aa_trig::signed_angle_extended, the real 80-bit register value); the test
  checks it within two binary64 ulps of the capture with the same binary32
  store. The Double53/Extended64 rout fixtures embed Unicorn's FPATAN value.
* The CRT pow SSE2 body is integer/binary64 arithmetic which Unicorn computes
  exactly; native-goal-base.json runs it with [B4B340] = 1, MXCSR 0x1F80, and
  the Rust replay uses aa_numeric::crt_pow::ci_pow.

Seams / required services (bound by callers; not ported here)
* Goal virtuals overridden by goal classes (+4 +8 +C +14 +20 +24 +28 +2C +30
  +38 +44 +64 +68 +6C +7C +80), actor virtuals (+4 +8 +C +10 +20, +10
  object's +20, cached cost +14), 7A10F0/79F390 unit check,
  SAIData modifier match 4A7CD0, 7A5200/7A3860 bound to goal_priority.
* Tai services: 445400 (see above), 54EFC0/54F8B0 zones, 52A690, 4F46E0,
  520440/520320 (movement agent), 520D90 chase predicate, 578140 organization
  status, 578470 engage range, 4D3FA0 organization halt, 4D7960, 4D4FF0
  relation, 51D920 follow activation, 527CE0/51EE20 guard helpers, 522890,
  522F60, 523000, 520120, 56AEC0 denizen home, 569300 component threat list,
  585160, GTID registry, SVars (KillTargetIsHighestPriority,
  GuardOldTargetBias, RoutingTime), Mind/organization/member virtuals.
* SAIData services: aa_mind::sai::data::DataNode (getters), profile::
  DataManagers (48D500) and ego::EgoServices (4BDBA0 Things, 463FA0
  Properties, AbilityDefinition/Resource registry sizes, AbilityDefinition
  `area` at +38). rules_data::RulesData implements them over the aa-rules
  native-order catalog; definition handles are opaque words (native: heap
  pointers, hashed by the ego tables). Queued DataRefs (vtables A17038,
  A1703C, A17040, A17044, A17048, A1704C) are resolved later by the data
  manager's pending-list pass (not ported; only the profile's 4AE340 is).
* Actor reference counts are not modelled (Rust ownership); the oracles start
  every actor with world-held references and check the native counts equal
  1000 + list memberships at the end with no destructor run.

Commands (bash, from rust-rebuild; PYTHONPATH=../reconstruction/tools/python-packages)
  python research/phase8-ai/record_evidence.py         (~35 min, regenerates all)
  CARGO_TARGET_DIR=target/s-ai cargo test -p aa-mind --release --locked --offline -j 4
  CARGO_TARGET_DIR=target/s-ai cargo clippy -p aa-mind --release --locked --offline -j 4 --tests --no-deps -- -D warnings
  CARGO_TARGET_DIR=target/s-ai cargo run -p aa-mind --release --locked --offline --example dump_sai_inputs -- research/phase8-ai/sai-shipped-inputs.json research/phase8-ai/sai-shipped-egos.json
Results 2026-10-15 (round 3): record_evidence regenerated all 52 fixtures
byte-identically (24393 cases; the ego producer alone runs ~15 min on 4
processes); 70 aa-mind test binaries, 85 tests passed, 0 failed, before
sai_actor_notify_native (+1, passes); strict clippy (--tests --examples)
clean.

Proven vs compiled vs untested
* Proven (native oracle replay at three precisions): all functions in the
  "Ported" table, under the stated seams.
* Compiled only: element_move_continuation::{collision_plan, stop_distance}
  (glue to the movement-proven aa_navigation::uai_collision),
  sai::travel::RouteServices (forwarding to route_cost), the RulesData
  services (exercised by the shipped dump, whose answers the oracles replay).
* Not bound to the live runtime: none of the new modules is called by
  aa-runtime yet (needs the services above); no end-to-end AI match play.
* Untested: SAI/Tai behaviour in a running map; NaN/Inf arithmetic inputs
  (explicit errors, as the arithmetic layer is finite-only); CRT pow libm
  error paths (explicit error).

Ownership boundaries
* Movement agent: CAI move orders 520320/520440/5276B0, member
  return-to-slot 52E9D0/52E900, static region topology 7ACB00, water flood
  fill 539C40 (services here). SAI map analysis 7AC0D0 is ported here on its
  own region model (index, neighbour hash, first-rectangle terrain service);
  binding it to the movement Topology is the SAI manager owner's job.
* Metagame agent: MGAI (not ported here).
* aa_mind::sai stays in aa-mind (no aa-sai crate).

Gaps (largest first; addresses in inventory.json)
* SAI goal classes: base priorities (+78) for the 22 classes (ATTACK_REGION
  7C5AE0, DEFEND_REGION 7C5F80, DEFEND_ACTOR 7C62D0, AIR_ATTACK 7C7E90,
  RECOVER 7C6640, CONSTRUCT 7C0120, EXPLORE 7C29F0, BOMBARD 7C8750, RESEARCH
  7D0680, ATTACH 7BE260 ...; they read the SAI map/threat model 7B9050,
  7BD360, 572040/572940/571C60, [B36528]+80), the military overrides
  (7C5240/7C57F0/7C5870/7C59F0/7C5A60/7C9E90/7C9080), class +7C hooks, goal
  execution (+58, e.g. 7C9710 -> 7A5E10 order issuing), recruit/build/
  general-power goals, attack waves.
* SAIData modifier 4A7CD0, SAI think
  scheduling 7A0080, orders 7AF220.
* Tai: the Think helpers left as seams (51D920, 527CE0, 51EE20, 522890, 522F60,
  523000, 520120, 520D90, and those listed in each cai_thinks / uai_thinks
  module).
* Handicaps: property application (property/modifier system owner).
