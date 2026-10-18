PHASE 8 - METAGAME (World War II strategic mode) - native port, rounds 1-3
Crate: crates/aa-metagame      Research: research/phase8-metagame
Original image: E:\Games\Axis and Alies\AA.exe (sha256 5d2548a2...0d26),
every executed instruction byte-checked against the v14 AA.exe (3bda77d7...c5c1).
Rule: port from the machine code, prove with Unicorn, explicit Unsupported for
anything unproven (docs/NATIVE-PORT-RULES.md). Addresses are cited per item in
the Rust sources and in inventory.json.

FILES
inventory.json        1053 functions: the metagame code range 0x932000..0x96BB00
                      plus listed helpers; area, source .cpp (assert string or
                      contiguity), strings, role, ported (Rust file), proven
                      (fixture). Summary: 399 annotated, 377 ported, 341 proven.
                      Regenerate: python build_inventory.py
metagame-data.json    oracle input: MetaGameData exported from Data.rwd + v14
                      overlays (sha256 e90abf46...6b92, round 3: localized
                      fields). Every metagame native-*.json pins
                      it. tests/real_data.rs export_definitions_for_oracles
                      FAILS if the loader/aa-rules output drifted; rewrite only
                      with AA_METAGAME_EXPORT=1 and then regenerate the oracles.
mg_machine.py         Unicorn harness (image mapping, seams, per-call trampolines
                      capturing x87 ST0 as 80 bits, clean FPU per call).
mg_world.py           native world builders (CRT heap seams, trig tables built by
                      their static initializers, __wcsicmp C-locale seam, native
                      wide strings, kingdom actors 0x400 bytes).
mg_campaign.py        a native MGState over a shipped campaign (map defs, links,
                      alias partners, divisions, techs, things with MG data,
                      shops, objectives) via 9397E0/939B90.
native_<x>.py         oracle producers, each -> native-<x>.json:
  battle (round 1), qsort (round 1), init, victory, moves, commands, process,
  path, resolve, turn (+ native_resolve.py Harness), save, launch,
  mgai (+ native_mgai_world.py; per-campaign files native-mgai-c<N>.json),
  round 3: positions, chunks, mgai_controller (native-mgai-controller-c<N>.json),
  trig_tables_exact.py -> trig-tables-exact.json (exact sin/cos roundings).
rmc-data.json         round 3: every RMC node and the registries its loader
                      reads, exported from the original data, plus the loaded
                      RmcData (tests/rmc_data.rs; ignored export check, rewrite
                      only with AA_METAGAME_EXPORT=1).
disasm.py             Capstone listing helper (python disasm.py 95f480).
findings.json         machine-readable summary of this README.
requests-to-lead.md   requests to the lead / other crate owners.

RUN (PowerShell, rust-rebuild)
  $env:PYTHONPATH="E:\Games\Axis and Alies\reconstruction\tools\python-packages"
  python research\phase8-metagame\native_<x>.py        (save: native_save.py 6)
      wall time: init/victory/moves/commands/process 40-70 s each, turn 3 min,
      resolve 4 min, launch 5 min, save 5.5 min, path 10 min, positions 45 s
      (all can run in parallel); mgai: native_mgai.py <ci> 4 4 (35-45 min per
      campaign); controller: native_mgai_controller.py <ci> 2 3 (per campaign,
      see ROUND 3).
  $env:CARGO_TARGET_DIR="target/s-metagame"
  cargo test -p aa-metagame --release --locked --offline -j 4
  cargo test -p aa-metagame --release --locked --offline -j 4 -- --include-ignored
  cargo clippy -p aa-metagame --release --locked --offline -j 4 --tests --no-deps
  (results of the final run: see TEST RESULTS at the end)

PRECISION AND THREAD (round 2)
  GAME_PRECISION = Double53 (x87 PC53). Lead policy: the simulation thread
  (TGC_NetNode::Update 765410 on its own TGC_Thread, Windows initial CW 0x027F)
  runs at PC53; D3D's PC24 applies to the main/render thread. Metagame commands
  are TGC orders/decrees executed by 765410, and the MGAI yields through the
  SAI fiber scheduler 79DFF0 that KKC_SAI::Think runs inside 765410, so the
  authoritative metagame (KKC_MGState, commands, QuickBattleSolver, MGAI) is on
  the PC53 thread. This is an INFERENCE from the call structure, not a dynamic
  trace. Main-thread presentation (odds tooltip 94E610) is PC24. Every float
  rule takes the precision explicitly; oracles run PC53 first and also PC24 and
  PC64.
  Unicorn 2.1.4 computes FSIN/FCOS/FPATAN/FYL2X through binary64: the only
  transcendental use found is the 4096-entry sin/cos tables built by the
  static initializers 9C8620/9C8640 (used by the TerritoryPositionSolver,
  positions recorded in native-init.json but NOT ported or asserted). No
  ported/proven metagame rule depends on those tables.

PROVEN BY ORACLE - ROUND 1 (unchanged)
  85D840 RNG; 95C7B0 destroy-behaviour enum; 93EB70/93EBD0 army strength;
  954FD0 defender strength; 955EB0 defending kingdom; 95F270 technology bonus;
  95F2F0 odds clamp; 95F370 attack odds; 95F480 SolveBattle (roll = draw *
  1/(2^32-1), binary32 store, attacker wins iff roll < odds); 954D80 + 93EC40/
  93ECA0/93F080/93F110 loser-army selection; 955420/9554F0/9564F0 n-th army and
  random destroy; 96CE20/96CDB0 + 961790 VS2003 qsort + shop comparator.

PROVEN BY ORACLE - ROUND 2 (fixture; Rust module; what is compared)
  InitCreate 939B90 (+939690 ctor, 95A100, 954A10/954A90/954AE0, 955D50,
    955B70, 956140, 940100, 93F9F0, 95A850, 954CF0): native-init.json, all 5
    campaigns; state.rs MgState::init_create. Order: kingdoms, owned records
    with their armies, unowned map territories, capital fallback; current
    kingdom 0; round 1; RNG 0x8E4B12A5. Army positions not compared.
  Owner change 955620 (removes the LAST match), victory 95A6B0 and capital
    lost 95A350 (+959F20): native-victory.json, 20 sequences / 240 steps (104
    with a winner); state.rs set_owner / has_won / capital_lost.
  Army relocation 940100 (956280 removal, 956140 first-gap slot, +0x1C count,
    path truncate-or-append 93F9F0): native-moves.json, 300 moves.
  Command validation 951D70 with the per-command validators (9512B0, 951480,
    951710, 951770, 951800, 951890, 9519D0, 9510F0, 950EF0, ...), 950CF0 alias
    -> partner, 93A320 pending battle, 93F970: native-commands.json, 1800
    commands, every error code 0..0x15 reached (952EC0 string table indices);
    commands.rs Validator. Also replayed with the ported path search.
  Command processing 954570 (dispatch by category: event 0x1F, battle 0x20,
    purchase 0x21, movement 0x22) with move/attack waypoints, start battle
    954440, play 954400, cancel 9543C0, purchase technology/actor (952B90,
    952D10, 952D90, 9537B0; actor placed at the capital, owner = capital
    owner), upgrade / upgrade all (951AA0 change type, waypoints cleared,
    93F7A0 list): native-process.json, 1000 command cases, 286 executed (52
    upgrades); commands.rs Processor.
  Pathfinder 93FDC0 / 93FCC0 / 93FAD0 / 93EAE0 and the search framework
    961550/961350/960FB0 (greedy best-first, parent fixed at first discovery,
    goal test before blocked test, key f32(link cost + f32(h)), h = (|dx|+|dy|)
    * 0x3D888889): native-path.json, 4900 searches + 2450 army queries + 1750
    heuristic ST0 rows at PC53/PC24/PC64 (one search differs PC24 vs PC53);
    path.rs.
  Battle results 954130 ResolveBattle / 954220 ProcessQuickBattle / 967530 /
    967390 / 967290 (liberation, capture 954F30, immobile cleanup 956380,
    defending destroy behaviour, owner change) / 9673F0 (attacking destroy
    behaviour, retreat 9671A0 / 967040) / 9670D0 / 956350 / 940020 deferred
    move step / 938E10 game over: native-resolve.json, 934 resolves (all 64
    behaviour pairs, 859 kills, 89 captures, 65 liberations, 305 retreats found,
    38 game overs), 173 quick battles; resolve.rs (MgContext, Engine seams,
    EngineCall log compared in order).
  History log: battle records 95EFD0 appended by 939410 (MGState+0x60),
    cleared by 938A80 at the next kingdom; proven in resolve/turn and saved /
    loaded by 95F000 / 95F100.
  End turn 952790 / 9391F0 / 938DA0 / 938C90 (records cleared, battles reset,
    auto-captures 9556D0, per-army reset 93FE10, round, income 959FB0, flags):
    native-turn.json, 379 end turns, 1731 auto-captures, 149 new rounds;
    turn.rs.
  Strategic save 938EE0 / load 93A3B0 (kingdom 95A470/95AAC0, territory
    955160/9565E0, army reference 954970/9549C0, snapshot 93F1A0/9409E0 captured
    by 9402E0, event state 95B020/95B0F0, battle record 95F000/95F100, army
    component record 93EEA0/940740): native-save.json, 35 saves (5 campaigns x
    7 worlds), bytes written equal, parsed state equal, re-save equal, 2275
    component round trips, 53 reader error cases + 10 writer cases + 40
    component edge cases; save.rs.
  Battle generation 948EF0 RequestGame (participants 947560, 946010, stockpile
    sums 9459F0, start properties 9466C0, additional actors 948BD0, RMC
    settings 506370) up to the Launch seam 40DF00, and the RTS result decision
    93A990: native-launch.json, 900 battles (585 launch with every order equal,
    315 native NULL dereferences -> Unsupported), 2700 RTS outcomes; launch.rs.
  MGAI: see MGAI below.

MGAI (mgai.rs; native_mgai.py <ci> 4 4 per campaign, ~45 min each, run in
parallel; fixtures native-mgai-c0..c4.json)
  KKC_MGAIPlayer constructor 963430 and PlayTurn 964D30 with every stage:
  DoMapAnalysis 962B70 (analysis 969CB0 / territory analysis 96A780, map
  record 95FDD0 / 95FF10, proximity BFS 96A230 + 96A1A0/96A090/969C20/969BC0/
  969B40/969B60 with _ftol2 96CC4C, influence 96A4C0/96A330/96A5A0 with the
  integer pow 4A6280 and its inlined copies, borders 969E60, importance
  9699B0/96A980/96A5D0, threat 96A9C0), goals (9635A0 SetGoalPriorities,
  9632E0, resets 969430/9697D0/969600, 9683B0, bisection insert 869D30 with
  comparator 9682A0, attack items 969180/969130, defend items 9690C0/968930,
  values 968720/968C50/968A50/968940, assignment 969010/968F70/969250/969380/
  9692B0/9682E0 and the iterators), purchasing (963B70, 962F00, 962DB0,
  963900, shuffle 962CB0, 938AC0/959F00, 959FF0, 93E9E0), attack decision
  963FD0 (955650, 955090), movement 9649A0/9649C0/963030, EndTurn 962A40.
  Reuses 954FD0, 93EBD0, 93F030, the ported path search (961550+961350),
  93A320, 95A350, 957910, 939B60.
  Oracle: 112 turns (68 at PC53, 18 at PC24, 26 at PC64) on perturbed
  InitCreate worlds of all 5 campaigns; 873 stage snapshots (analysis and
  goals after every PlayTurn stage: influence/priorities as binary32 bits,
  assigned armies, player bytes +0x28/+0x29) and 889 issued commands, every
  yield, busy query, 96A5D0 block and the app RNG word after each turn.
  Coverage: 10 attacks carried out (23 attack commands), 63 startbattle, 384
  moves, 120 attack steps, 90 armies + 100 technologies + 7 upgrades bought,
  17 turns with a lost capital, 28 with no upgrade possible, 339 scripted
  arrivals, early aborts. Findings: Attack and AttackUnoccupied goals both
  use attack items (969180); 968720 is the defence value of both item kinds;
  attack priority = f32(b*(1-f) + f*a) (a = f32(968720), b = 968C50
  unrounded, f = attack_defendability_factor); 968C50 uses
  no_attack_distance as a path budget: importance*(min-1)/(N-1); 938AC0/
  959F00 (territories owned by kingdoms regarded as enemies) gates army
  purchases; [[B36528]+0x2C] points to the app RNG word.
  Seams (MgaiHost): the kingdom actor army list (+0x284, order shuffled by the
  oracle), yield 944CE0 (may change the state: tick, MGState+0x46, aborts,
  arrivals through the original 940100), command queue 944F50 (issued
  commands are the output), tick [B36534]+0xDC, army busy (+0x70 vtable
  +0x64), handle resolution, app RNG, the uninitialised 96A5D0 block (an
  input; the oracle fills it with random bytes). Timer 59ABA0 and the logs
  404F60/952310 have no effect and are omitted.
  Round 2 left the controller (944F90 / 945090) and the processing of queued
  commands during yields to round 3 (see ROUND 3); stale handles never occur
  in native-mgai-c*.json. Unsupported: kingdoms without actor, missing MGAI
  data or map record, empty army paths, no_attack_distance 1, defence threat
  ratio 1, empty or zero-cost shops, NULL shop rows, non-finite x87 results.
  CRT pow is not used (integer pow by repeated multiplication), so
  aa_numeric::crt_pow is not needed here.

EMULATOR GUARDS (round 2, lead caveats in docs/NATIVE-PORT-RULES.md)
  mg_machine.install_guards (Machine and native_mgai_world.FastMachine):
  - page 0: any access below 0x1000 except an FS-prefixed access to 0..3
    (the SEH link) stops the run with NullAccess (pc, address, size).
  - allocators: entries of __heap_alloc, __nh_malloc, malloc, free, realloc,
    calloc, _msize, the aligned mallocs, engine operator new/delete 6F7810/
    6F7430 and the engine pool 6F7700 trap unless seamed.
  - imports: all 324 IAT slots point to an unmapped trap range (the image IAT
    holds unbound RVAs, e.g. HeapAlloc -> 0x720368, which is engine code), so
    an unseamed import call faults with its name.
  - code rewriting: trampolines are written once per (target, store) at fresh
    addresses; seam stubs are 'ret' fills written once. No harness rewrites
    executed code.
  - SNaN: no generator produces NaN/Inf inputs (launch's raw float patterns
    use exponents 90..160); Rust treats non-finite x87 results as Unsupported.
  Result of the re-run with the guards: every fixture is unchanged except
  native_resolve/native_turn, whose setup "territory owner NULL" called
  955620(NULL), which dereferences NULL in 95A850 (page 0 had been silently
  readable). The setup now builds the unowned state synthetically (959F20,
  owner 0, 954CF0; Rust MgState::clear_owner) and MgState::set_owner takes
  a required owner. The resulting worlds and all recorded outcomes are
  identical; only the visited-instruction list lost the 95A850/766DE0 path.
  Launch faults: all 315 accepted native faults are page-0 accesses in
  metagame code (947616, 9476F6, 938BE6, 4A0906, 947607), none in an
  allocator or import.

SEAMS (engine side, explicit inputs; listed per producer script)
  Kingdom relations vtable+0x140 (enemy) / +0x138 (ally-like), army owner
  vtable+0x108, controllable vtable+0x134, set owner vtable+0x160 (assumed to
  change only what +0x108 returns), kill 4D7960, battle messages 953A20 /
  95A8D0, kingdom victory 4E9720, game over 50D370, army visuals 9626D0/962880,
  the army busy virtual (+0x70 vtable+0x64), the kingdom actor army list
  (+0x284, field-read seam 938DD8), world kingdom by name 4F8F00, 4D5EA0,
  4F6760 (respawn), army creation 4FA030, visuals 4F48E0/4F9C10, 939230, the
  save stream (ctx+0x3C), _CxxThrowException 96BEF5 (message captured), the 16
  TGC order senders and Launch 40DF00, RMC game settings (5A2210 family,
  511DF0), wall clock 6FB1F0, CRT heap (96E19F/96D331/6F7810/6F7430, realloc
  97117C), wide printf 96C85A/96C7EC (C locale), __wcsicmp 96CAD6 (C locale,
  A-Z folding), atexit 96CAB0.

REMOVED NATIVE LIMITS
  All native vectors (kingdoms, territories, links, armies, shop lists, path
  node store and open heap, goal items, participants, records) are Vec. The
  qsort stack (30 entries) is a Vec. The territory hash bucket count only
  affects lookup speed. The heap maximum (53E7B0, 0 = unlimited) stays
  unlimited.

KEY FINDINGS (round 2)
  - 955620 set owner removes the LAST matching entry from the old owner's list.
  - The defender iterator 955E80 and the immobile cleanup 956380 use
    vtable+0x138; the n-th defender 9554F0 and defender counts use !+0x140.
  - 954F30 captures every capturable army in the territory, attackers too.
  - Retreat target 967040: linked territory of a friend of the old owner with
    the strictly largest binary32 strength.
  - 9673F0 gives a NULL retreat waypoint to an attacker whose path has <= 1
    entry (natively reachable; 940020 would dereference it): Unsupported.
  - Unknown IDS in the save reader throws ("Unknown IDS %s for class %hs",
    41DFB0); event states are read and dropped; the territory record (+08) is
    not restored; each kingdom's territory list is rebuilt in territory order;
    re-saving a loaded state differs in 25 of 35 rounds (Rust reproduces it).
  - RMC settings +0x1C is an INTEGER midpoint (max-min)/2+min (Ghidra shows a
    float). Attackers look up their session player by kingdom actor name with
    no NULL check (native crash), defenders by campaign kingdom IDS with one.
  - 93A990 with an empty winner name: local-side rule, else one SolveBattle.
  - Pathfinder: budget 0 needs start == goal; neutral/background territories
    always fail; 93FCC0 budget may be negative (-1 = unlimited).

ROUND 3 (2026-10-15)
  Army display positions (positions.rs; native-positions.json, 5 tests):
    TerritoryPositionSolver 959420 (+ctor 959220), layouts SIMPLE 959390 /
    CIRCLE 959560 / SQUARE 959240, spacing enum 959B20/959A50, first free slot
    954A10, army slot 954CC0/954C60, actor slots 9578C0, territory position
    9552A0 -> 957DD0 (alias x from bounds by partner, z from the territory
    visual +0x28 = engine input), placement 955B70 (from 955D50 InitCreate,
    952A00 purchase, 95B220 event add_actor), respawn 955820, realign 9626D0,
    move waypoints 962880. 360 layout rows, 6960 solves, 1740 territory
    positions, 600 realigns, 600 waypoint rows (PC53/PC24/PC64), 75 955B70
    placements (PC53); InitCreate positions of native-init.json all match.
    Positions are engine data (4FA030 / 4EE5C0 / 9627D0) and are returned,
    not stored in MgState. Sin/cos 85DAA0/85DA80 = aa_trig.
  Trig tables (trig_tables_exact.py; trig-tables-exact.json): the tables of
    9C8620/9C8640 (408DE0: x_i = f32(i * f32(2pi) / 4096), fsin/fcos, binary32
    store) equal the exact binary32 roundings of sin/cos (mpmath, 256 bits) in
    all 8192 entries; the smallest distance to a rounding boundary is 282x the
    bound E = 2 ulp53 + 2^-62 (Intel FSIN/FCOS < 1 ulp64, 66-bit pi reduction,
    Unicorn's binary64 path), so neither Unicorn nor a conforming x87 changes
    any entry. aa-trig embeds the same bytes. (The bound is Intel's published
    accuracy, not a CPU measurement.)
  MgState gaps closed (state.rs): faction (+0x0C), rts_winner (+0x48),
    events (+0x54, EventState 95AD60/95AD90: event, triggered +0x0C, kingdom
    +0x10; InitCreate builds one per campaign event via 95BE70), kingdom help
    popups (+0x2C), territory visual (+0x20), army-reference snapshots
    (9402E0, captured by enter_territory / 956140), kingdom actor names
    (4BEDF0, engine input for snapshots), init_create_with(InitEngine).
    save.rs and launch.rs no longer carry private copies.
  MGState +0x48 writer: 60A140 (game type 4, callers 6125A0 / 61E620) sets
    the 4BEDF0 name of the last world kingdom whose +0x19C == 1, else empty.
    Engine side; MgState::rts_winner is its input.
  Save container (chunk_file.rs; native-chunks.json): the TGCK chunk file
    (header 'TGCK' v2, chunk data, 0x60-byte directory records; open 859690,
    859380, write 859150/858E40/858DB0, read 859030/858C90, seek 858F50/
    858ED0, close 859330/859090, records 8582B0/8580E0, error 858BA0): 24
    written files, 206 chunks and 9 crafted reader cases equal natively.
    Compressed chunks (zlib, MapSaveCompressed) are Unsupported.
  Load order (4FC600): RULES, TERRAIN, HEIGHT_MAP, MOUNTAIN_HEIGHT_MAP, IDMGR,
    UPDATABLES, DENIZEN_GROUPS, RESTRICTION_GROUPS, TEAMS, OBJECTIVES,
    KINGDOMS, METAGAME (939930 + Load; game type 4 and chunk present), then
    USED_REQUIRED_OBJECTS, STATIC_THINGS, DYNAMIC_THINGS (4FC340), TRIGGERS,
    SUN, UPDATABLES, MISC_DATA. So 940740 resolves component territory indices
    against the MGState just loaded (closes the round-2 question).
  Localized fields (data.rs, fields.rs): 5B5620 (required localized getter;
    a #key is replaced by the string table 4B9F40 at load time) and the name
    read of the data base loader 47D960, at their native positions: territory
    name (958AE0, first field), map name, division name/tooltip/
    upgrade_tooltip, army component tooltip (93B250), MGAI name, command
    name/tooltip (949B60), campaign name/description, kingdom name (95EB00),
    event name/message (968000), effect message (967C10). Values are the
    authored keys (the string tables are not part of this crate); background
    territories carry the unquoted null. metagame-data.json re-exported once
    (e90abf46...); every oracle regenerated: outcomes identical, only the
    pinned data sha changed.
  Campaign events at runtime (turn.rs; native-turn.json): roll 95ADD0 (new
    round; fixed round or one x87 draw vs round_probability; kingdom_turn or
    one draw over the kingdoms), fire 95BE20 -> 95BC80 effects in order
    (95B630 add_tech, 95B220 add_actor, 95B7F0 remove_tech, 95B400
    remove_actor, 95B9B0 change_territory; given or drawn territory 95AED0,
    army type 95AE80, technology 95AF40), event / effect messages as engine
    seams with their formatting arguments. Synthetic events (no shipped
    campaign has any): 524 rolls (288 probability draws, 89 random kingdoms),
    326 fires, 758 effects of all five kinds.
  MGAI controller (mgai/controller.rs; native-mgai-controller-c*.json):
    CONTROLLER_RESULTS
  RMC loader (rmc.rs; rmc-data.json, tests/rmc_data.rs): KKC_RMCData::Load
    4A5420 with LoadRules 4A19D0, terrain sliders 4A4410, noise 4A1DB0,
    kingdoms 4A4EB0 (major +0xF8 / independent +0x104 slots, relations,
    KingdomsMax), object groups 4A4600 (node order, apply_to_all_major_kingdoms
    expansion), restrictions 4A2150, teams 4A1CD0, [Heroes] (+0x80/+0x84).
    All 63 shipped RMCs load. Two loader rules were completed from the
    listing while checking the shipped data: a group-restriction prefix with
    no loaded match is compared with the group being loaded (4A2150; 8
    shipped RMCs need it), and KingdomsMax is the SVars_AA.tgi value (loaded
    after SVars.tgi; 16 in v14). The metagame view (data.rs RmcDefinition)
    agrees with the loader on every metagame RMC's kingdom slots, and
    launch::RmcSliders::from_rmc feeds 506370 from it (LaunchEnv keeps the
    input for the oracle's synthetic sliders). NOT oracle-proven: the loader
    has no Unicorn replay (it needs native TGI node objects and the engine
    registries); ported from the listing and checked on all shipped data.

GAPS / NOT DONE (explicit)
  - RMC loader: no native oracle (above); WorldRule registry, [Stockpile]
    (473B10), Biome loader 479080, localization registration 49AD00/49AE00
    are not ported (rmc.rs module doc).
  - String tables: 5B5620 / 4B9F40 translation of #keys is not ported; data
    and order payloads carry the keys (CreateExplicitAI name included).
  - NULL retreat order (9673F0 -> 940020): Unsupported instead of a modelled
    crash.
  - Save: compressed chunks, the engine stream's short reads, duplicate
    registry IDS (Unsupported).
  - Launch: RMC game settings (+0x90) carried by RMC id; the order-stream gate
    766CD0; process_rts_result (93A990 + 954130) has no end-to-end oracle.
  - MGAI controller engine context (SAI think 7A0080, fiber 79E2C0, order
    transport 950770/9506D0 -> 9507D0, issuer resolution 74B3B0/765E00) are
    engine seams.
  - Kingdom relation virtuals are inputs (KingdomRelations), backlog.
  - Map-table props 939230 in InitCreate are not ported (seamed in every oracle).
  - Native crash / hang paths return Unsupported; launch's 315 NULL faults
    are oracle-proven, the others are ported from the listing.

TEST RESULTS (2026-10-15, after the guarded re-run of every oracle)
  cargo test -p aa-metagame ...                     -> 19 passed, 10 ignored
  cargo test -p aa-metagame ... -- --include-ignored -> 29 passed
    loader_validation 8, native_battle 4, native_commands 1, native_init 1,
    native_launch 2, native_mgai 1, native_moves 1, native_path 1,
    native_process 1, native_qsort 1, native_resolve 1, native_save 3,
    native_turn 1, native_victory 1, real_data 2 (loader_validation and
    real_data are ignored by default: they need the local Data.rwd;
    export_definitions_for_oracles now checks that
    metagame-data.json is current instead of rewriting it)
  cargo clippy -p aa-metagame ... --tests --no-deps -> clean
  Fixture sha256: findings.json "fixtures" (data 96a66de6...703b).
