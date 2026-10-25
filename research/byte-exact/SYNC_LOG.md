# The original sync logs: every writer, and what the films' S checkpoints contain

Audit of 2026-10-25. Addresses are AA.exe virtual addresses. The binary analysed is
`reconstruction/research/native-aa/imported-0.bin` (SHA-256 cbf86377…); the QA films were
recorded with its v14 build (`runtime-expanded-v14/AA.exe`) — the patched v14 keeps these
code addresses (inferred from the decoder and the hashes below matching).

Evidence files (in this folder):
* `synclog_census.py` → `synclog-sites.json`: every direct `call` into the loggers, with
  its function, synchronizer instance, label and direct-call reachability (heuristic linear
  pass, see the script header). Rerun: `python -I synclog_census.py synclog-sites.json`.
* `crates/aa-parity`: `slog::account`, `slog::verify_zero_residual`, `chain::search`, the CLI
  (`aa-parity film <rfm> --game-root <root> --inferred-registries [--decrees a-b] [--chain n]`)
  and `tests/sync_log.rs` (5 tests, 3 on the QA films).

## 1. Mechanism

| item | native |
|---|---|
| synchronizer A (multiplayer, checked by TGC_SetSyncValueOrder 302/314) | `[0xB3655C]` |
| synchronizer S (films, checked by KKC_SetSimSyncValueOrder 299/311 on playback, 0x752F60 "FILM OUT OF SYNC") | `[0xB36560]` |
| loggers (thiscall, ecx = instance) | 0x77DA70 int, 0x77D930 float bits, 0x77D9D0 bool, 0x77DB10 text (one entry, every UTF-16 unit hashed), 0x77DC50 marker (hashes 0) |
| fixed-instance wrappers (cdecl) | 0x40C970 S int, 0x40F950 S text, 0x40F8D0 A text |
| hash per step | `h = (h * 37 + v) % 65536`, i32 wrapping, signed remainder (`aa_orders::net::sync::step`) |
| reset 0x77D900(index, hash) | session launch 0x775BF0 calls it at 0x775D4A with (0, 0) right after the world dump 0x4F8AE0 → 0x4F6280; also 0x74DF50, 0x75AEE0 |
| checkpoint | host admin tick: 0x74E670 sends {S end − 1, S hash} before the requests of that admin tick run (0x752E60) |

The world dump 0x4F6280 logs ~40 entries to **A** and draws once directly from GWorld+0xD4
(`m_rng.GetFRand()`, no getter, so no S entry); its A entries are then irrelevant to S, but its
draw advances the world stream before the S reset.

## 2. Census

1802 direct call sites (`synclog-sites.json`):

| logger | sites | instance |
|---|---|---|
| int 0x77DA70 | 893 | A 785, S 106, register-passed 2 (dump helpers, A) |
| float 0x77D930 | 37 | A 35, S 1, register-passed 1 (dump helper, A) |
| bool 0x77D9D0 | 23 | A |
| marker 0x77DC50 | 64 (42 functions) | A |
| text 0x77DB10 | 2 | A 1 (in 0x40F8D0), S 1 (in 0x40F950) |
| S int wrapper 0x40C970 | 100 | S |
| S text wrapper 0x40F950 | 201 | S |
| A text wrapper 0x40F8D0 | 377 | A |
| world-RNG getter 0x4F02B0 (logs to A then S) | 105 (58 functions + 10 sites outside Ghidra functions) | ecx = `[0xB36534]` GWorld at 100 sites; 5 pass ecx in a register (RMG 0x591D43/0x5933C4/0x539372, rout 0x5212C7, CreateGWorld 0x76AB2B) |

**Markers, bools and every dump (world 0x4F6280, things 0x4EE310/0x4F0DB0, rules 0x5118A0,
RMC settings 0x505EA0, creator 0x5128F0, …) go to A only.** They matter for multiplayer
interop, not for films.

## 3. Every S writer

The S log receives exactly four kinds of entries:

| # | entry | sites | value | when it runs | aa-play owner |
|---|---|---|---|---|---|
| 1 | `g_gworld->GetGameTimeBeginTick()` (float) | 0x76F835 in 0x76F790 | GWorld+0xE0 bits **before** the update (0, 0, 1/16, 2/16, …) | world tick 0x752AE0, after film record/playback, before GWorld update 0x4F9970 | `sim.rs` / `trace.rs` (done) |
| 2 | `(int)m_rng.GetSeed()` | 0x4F02E0 inside the getter 0x4F02B0 (105 call sites, §4) | world RNG state GWorld+0xD4 **before** the caller's draw | wherever a getter caller runs | every module that draws (§4) |
| 3 | `(int)m_rng.GetSeed()` inline | 0x4F4A77 in GWorld::AssignName 0x4F4A20; 0x4FDC15 and 0x4FDC7C in KKC_GWorld::ResolvePlaceholdersAndRandomThings 0x4FDA70 | same | actor naming at creation; map-start placeholder / random-thing resolution | `setup.rs`, creation in `world.rs` |
| 4 | decree `m_decree_type.m_name` (text) + `m_decree_type.m_id` (int) | 201 text sites (0x40F950), 100 int sites (0x40C970, request handlers) and 101 direct int sites (decree handlers) | the decree class name (UTF-16, every unit hashed) and its descriptor ordinal | host request execution 0x752E60 in the admin tick (recording), or film playback before 0x4F9970 | the replay/command layer (§6) |

Entry 4 is skipped for `TGC_SetPlayerOrderDecree` (every handler compares the type name with
0x9FD610 via 0x96CB83, e.g. 0x750DD0, 0x7509F0) and exists only for shapes with `s_log`
(100 decree shapes; `aa_orders::flow::shape`, already modelled by `flow::log_decree`;
`aa_parity::slog::decree_s_entries`). The timeline decrees (SetGameSpeed 73, SetSyncValue 314,
…) do not log to S.

**Validation on the films** (`slog::account`, `slog::verify_zero_residual`): every interval
between consecutive checkpoints is `clock (Δticks) + 2 × S-logging decrees (records r1..r2−1) +
residual`, the residual is never negative, and for every interval with residual 0 the hash is
reproduced exactly from clock and decree entries alone: **102 of 102 intervals** (film 0: 40,
film 1: 61, film 2: 1). Residual totals: film 0 33,397 entries over 10,710 ticks, film 1 9,523
over 5,025, film 2 51 over 255.

## 4. World-RNG getter sites (entry kind 2)

Every site is `getter; draw` with exactly one draw unless noted ("draws" = static calls to
0x85D840/0x40C8A0 in the function; 0 means the draw is in a callee, e.g. 0x45D400 unit
random). "Battle" sites run after the S reset and produce S entries in films; the others run
before it (world creation, RMG) and only move the RNG state.

| site / function | system (evidence) | getters / draws | when | aa-play module |
|---|---|---|---|---|
| 0x50B2DF in periodic queue add 0x50B2C0 | Updatable registration on a periodic queue (0x50AC00 queue +1AC, 0x50AC60 +1C0, …); bucket = draw % buckets — even the 1-bucket queue0 draws | 1 / 1 | every registration: e.g. production start head 0x589420 → 0x50AC00(0) when the queue is not scheduled | `world.rs` scheduler (F4), `production.rs` (recruit start) |
| 0x5300A0 ActionGo | element move: re-think delay `(0.3 \| 1.0) + 0.3 u` s via 0x45D400 | 1 / (callee) | UAI/Element think on its deferred queue | `movement.rs` (`aa_navigation::action_go`) |
| 0x530E50 | Element Mind constructor jitter `2u − 1` → +0x1B8 | 1 / 1 | Element mind creation | `movement.rs` / creation |
| 0x52E830 | UAI engage reaction delay (`EngageReactionTimeRange`) | 1 / 1 | engage think | `combat.rs` |
| 0x5340C0 | UAI kill decision | 1 / 1 | kill think | `combat.rs` |
| 0x5233F0 | CAI scatter formation (company_move) | 3 / 3 | regiment move dispatch | `movement.rs` / `formation.rs` |
| 0x5797E0 (Organization.cpp) | formation slot positions | 2 / (callee) | formation turn / slot assignment | `formation.rs` |
| 0x521240, 0x5213E0, 0x524170, 0x5242C0, 0x525100, 0x5267E0 | CAI rout think, explore ("explored"), explore candidates, rampage | 1, 1, 2, 1, 1, 1 | CAI thinks | `morale.rs`, `ai/*` (CAI side) |
| 0x51A260 | AAI GetValidAbility (aircraft) | 1 / 1 | aircraft think | (no aircraft in aa-play) |
| 0x4C8170 | performer GetRandomAnimation | 1 / (callee) | ability start | `combat.rs` (+ presentation) |
| 0x4CCA70 (AbilityEffect.cpp) | summon | 1 / (callee) | special operation | `special_ops.rs` |
| 0x4D3E20 (GActor.cpp) | KKC_GActor::OnKilledLair | 1 / (callee) | death of a lair | — |
| 0x562D10 | shortfall attrition | 1 / 1 per member | economy shortfall | `economy.rs` |
| 0x544CF0, 0x544ED0 | trigger elements (random) | 1 / 1 | trigger manager 0x54C280 | (campaigns) |
| 0x4EF2A0 GThing ctor, 0x4EA5E0 GObject ctor, 0x4E9FC0 InitCreate, 0x4EC7A0 GObject::Load | object creation (facing draw in 0x4EF2A0) | 1 each | creation / load | `world.rs::spawn_actor`, `setup.rs` |
| 0x563BA0, 0x563CA0, 0x563D50, 0x5644F0, 0x563B3E | Character component | 1 each | character init / animation | creation (+ presentation) |
| 0x575150, 0x575320, 0x575715, 0x575747; 0x4E2640 | HQ component / HQ | 1 each | HQ init / think | `supply.rs`, creation |
| 0x584CF0, 0x5853E0, 0x586FC1, 0x586FDC, 0x588817, 0x588893, 0x5888AE | recruit / structure / settlement components (0x5888xx near structure production 0x588D60) | 1 each (0x584CF0: 2 draws) | component thinks | `production.rs`, `deploy.rs` |
| 0x5902C0, 0x5904A0 | doodad / disturbable component delay | 1 / 1 | doodad think | — (scenery) |
| 0x50AB10 | Updatable placement ctor | 1 / 1 | creation | scheduler |
| 0x76AA90 (Session.cpp), 0x513410, 0x512F10 (WorldSettings.cpp), 0x50E370, 0x50E8B0 (WorldParameters.cpp) | world creation, seeding, random nation/colour/faction | 1–2 | before the reset | `world.rs` (`create_rng`), `setup.rs` |
| 0x5925E0 and stages 0x590E10, 0x591A30, 0x592E00, 0x5931C0, 0x593290, 0x593AA0, 0x594260, 0x594590, 0x595940, 0x5969D0, 0x596AE0, 0x596D20, 0x597090, 0x597530, 0x598B00; 0x54FB40, 0x551360, 0x539372 | random map generator (RandomMapGenerator.cpp, RMG*Stage.cpp), PlaceAutoDoodads, AddTerrainNoise, blob solver | 1–7 per function | before the reset | `mapgen/**` |

Reachability flags per site are in `synclog-sites.json` (direct calls only; most battle sites
are reached through virtual Updatable calls, so "unreachable" there means "not by a direct
call").

## 5. The residual is getter + one draw, nothing else

`aa_parity::chain` uses two facts: the hash steps are congruent mod 65536 to `37 h + v`, and
the low 16 bits of the LCG 0x85D840 form their own LCG mod 2^16 (`x → 0x4E6D x + 0x3039`). So a
getter entry contributes only its state's low half, and for a fixed placement of the getters a
checkpoint hash is affine in the low half `x` of the first getter's state. The search tries all
65536 values of `x` and every placement of each interval's residual getters into the
interval's slots (after a decree's entries, or after an update's clock), under "each getter is
followed by one draw and nothing else draws", chaining the intervals.

| film | intervals chained | surviving x | first-interval placement |
|---|---|---|---|
| LastFilm_0 | 16 (272 ticks) | **0x1551** only | getters in update 7, update 7, and while the 86 of record 7 runs |
| LastFilm_1 | 16 (272 ticks) | **0x7EE1** only | same: update 7, update 7, 86 of record 7 |
| LastFilm_2 | 6 (102 ticks) | 0x1501, 0x9501 (bit 15) | for 0x1501: update 8, update 8, 86 of record 8 (one of 3 placements; the only one matching films 0 and 1); 0x9501 has only (3, 333 of record 7, 17) |

With ~17–30 slots per interval and one 16-bit check per interval, a false survivor over 16
intervals is improbable (expected ≪ 1), so the draw model holds for the first ~270 ticks and
fixes the low half of the world RNG state at the first battle getter. The later pairs are
unique: film 2 has two getters in each of updates 29, 49, 69 and 90; film 0 in 28, 45, 62, 80;
film 1 in 24, 42, 61 — every 17–21 ticks, matching ActionGo's `1.0 + 0.3u` s re-think delay
(16–21 ticks) while the computer player's truck drives to its unpack spot (inferred from the
timing; the second getter of each pair is not identified).

## 6. Film 2's first checkpoint, entry by entry (30 entries at tick 17, hash 2327)

Order of the log: before update k come the decrees of film record k − 1 (their admin tick);
update k logs its clock, then the getters of the update. Record 17's decrees follow the
checkpoint. Reproduced exactly by `tests/sync_log.rs::film_2_first_checkpoint_entry_by_entry`.

| S index | tick | entry | source |
|---|---|---|---|
| 1 | update 1 | clock 0.0 | 0x76F835 |
| 2–3 | record 1 | `KKC_UpdateKingdomNameOrderDecree` name + id 328 | AI player's kingdom name (decree) |
| 4–9 | updates 2–7 | clock 0, 1/16, …, 5/16 | |
| 10–11 | record 7 | `SelectionSetOrderDecree` + 333 | AI selects slot 29 (its truck) |
| 12–13 | record 7 | `KKC_TellSelectedCommandOrderDecree` + 86 | AI orders unpack (ActorCommand 11) at a point |
| 14 | update 8 | clock 6/16 | |
| 15 | update 8 | **getter**, state low half 0x1501, one draw | the truck starts its move (inferred: ActionGo-type) |
| 16 | update 8 | **getter**, low half 0x1501 → lcg16, one draw | second getter of the move start (site not identified) |
| 17–18 | record 8 | `SelectionSetOrderDecree` + 333 | AI selects slot 28 (its HQ building) |
| 19–20 | record 8 | `KKC_TellSelectedCommandOrderDecree` + 86 | AI recruits (ActorCommand 15) |
| 21 | record 8 (admin tick) | **getter**, one draw | recruit → production start head 0x589420 → 0x50AC00(0) → periodic add 0x50B2C0 getter 0x50B2DF + draw 0x50B2E5 (verified in code) |
| 22–30 | updates 9–17 | clock 7/16 … 15/16 | |

So the 13 non-clock entries are 10 decree entries (UpdateKingdomName, two SelectionSet and two
TellSelectedCommand) and 3 world-RNG getters. Films 0 and 1 have the same structure one record
earlier (unpack at record 6, recruit at record 7).

## 7. What aa-play must emit to reach the film checkpoints

For tick 17 (count 30, hash ≡ 2327 mod 65536), in order:

1. Clock: one `Clock` per update (done).
2. Decree entries: for every executed decree with `s_log` (not SetPlayer), a text entry with its
   class name (every UTF-16 unit hashed) and an int entry with its descriptor, **before** the
   decree's effect runs, in the admin tick (i.e. before the next update's clock). In a film
   replay these come from the film (the harness knows every decree); in live play aa-play must
   emit them for each command, including the selection decrees the original sends with every
   command (SelectionSet 333 + TellSelectedCommand 86 for the AI).
3. Getters: no world-RNG getter at all before the AI's truck moves — aa-play's current getters
   at ticks 1 and 2 must go (they have no native counterpart there). Then:
   * the update after the unpack order: two getter + one-draw pairs (truck move start);
   * the recruit command (when the building's production queue is not yet scheduled): one
     getter + one draw during command processing, after the decree entries (queue0
     registration 0x50AC00 → 0x50B2C0, bucket `draw % 1`).
4. RNG state: the low 16 bits of the state at the first battle getter must be 0x1501 (film 2;
   0x1551 film 0, 0x7EE1 film 1). The hash depends only on the low halves, so until world
   creation and the RMG are exact, setting `state = (any << 16) | low` at battle start reaches
   the checkpoint; the high half decides the drawn values (delays) and so the later timing.

For tick 34 (count 51, hash 51390), additionally: the 311 decree of record 17 (name + id), and
two getter + draw pairs in update 29 (the truck's first re-think, `1.0 + 0.3u` s after update
8; the tick depends on the drawn `u`, i.e. on the high half of the state).

## 8. Open items

* ~~The second getter of each movement pair~~ (identified 2026-10-25, now in aa-play
  `movement.rs`): every ActionGo 0x5300A0 move-path call orders the Mover (0x530374 →
  0x5773C0), and 0x5773C0 ends on both branches with 0x50AC00(0) on the Mover's Updatable at
  Mover+8. That Updatable's vtable is 0xA3CE60 and its slot +8 is 0x6A71D0, which always
  returns true, so there is no "already registered" guard. The registration goes to GWorld+0x1AC
  (periodic queue 0, built by 0x50ABA0(0) in the GWorld ctor at 0x4F7F58, vtable 0xA30BE0)
  through its vtable +0xC, 0x50B2C0: getter 0x50B2DF, draw 0x50B2E5, bucket = draw %
  buckets. Only then does ActionGo run its own getter, 0x53037F + 0x45D400. So the pair is
  **registration first, re-think delay second**. 0x5773C0 skips the registration when
  [Mover+0x56] != 0 (0x5770A0's mode) or when the Mover's vtbl+0x38 refuses. Arrival and stop
  draw nothing (0x52D660 → 0x5767C0 → 0x50B250 is the unregister loop), and the Mover
  unregisters in its update/step (0x5768A0/0x5768F0, 0x5774A0), so every re-think registers
  it again.
* The high 16 bits of the battle-start state: the re-think ticks (17–21 tick gaps) constrain
  `u` and could recover them; or make world creation and the RMG exact.
* The A log (multiplayer interop): markers in 42 functions (list in `synclog-sites.json`),
  the tick-driver entries, `o.GetBitSize()`, decree logs, the launch dumps.
* The heuristic instance tracking covers direct calls; 3 register-passed sites were checked by
  hand (they belong to A dumps).


## 9. H4 replay: the stock-map film (2v2)Berlin (2026-10-25)

Recording (QA copy, hidden desktop, posted keys only). Single Player (F11), Custom (F12), then
Scenario Map (F7, `SelectLaunchType LOAD_MAP`). Then the profile hotkey `map numpad-3
StagingFileDialog "Select Maps/(2v2)Berlin.RMP"`: the bare file name and `QuickStartLoadMap`
do not resolve, because the latter gets a leading space ("Maps/ (2v2)Berlin.RMP"). Then Enter
(Commit) and Launch (F10). Loading a scenario map fills the other three slots with "Random"
computer players (Normal), so no mouse step was needed. The film ran ~4 min:
`Films/archive-qa-20261025/Berlin2v2_0.RFM` (QA copy only; aggregates in
`crates/aa-parity/tests/fixtures/qa_film_berlin_20261025.json`). It has 4539 ticks, 266
checkpoints and 752 decrees: three AIs recruit (15), unpack (11), move (5), kill (0) and
set_formation (33).

Initial world, film against aa-play (`aa_parity::replay::plan`/`build`):

| item | original (film) | aa-play replay |
|---|---|---|
| players | SessionPlayers 1, 2, 4, 5 = kingdom_0/1/3/4 (Kingdom+0x188 indices 0, 1, 3, 4; independent_2 and independent_5 are 2 and 5) | same slots; aa-play numbers its kingdoms 0–3 for the players, then 4–5 for the independents |
| teams | kingdom_0 + kingdom_1 = team_1, kingdom_3 + kingdom_4 = team_0 (map teams) | same |
| nation / general | creator kingdoms keep the staging input (empty = Random); the header kingdoms carry the picks: rus chuikov (human), rus zhukov, ger rundstedt, rus konev | taken from the header (aa-play does not draw them) |
| colours | blue, red, green, black (the map's) | same |
| rules | DifficultyLevel 1, ResourceStockpileDefault 500.0, StartingExperience 200.0 (kind 2 = binary32) | Computer(1) handicap, 500, 200; SAI not attached (the AI's decisions are the film's decrees) |
| start positions | each slot's start_loc / placeholder_corpsunpacked | same |
| HQ ids (creation order) | the replacement keeps the placeholder's saved GTID: kingdom_1 0x344D, kingdom_3 0x344F, kingdom_0 0x3450, kingdom_4 0x3451; next-fit stays at IDMGR 13399 (the first recruit) | placeholder destroyed, corps on next-fit 0x3457–0x345A (13399–13402) |
| RNG state at the first battle getter | 0x4F1E05E7: exactly 6231 draws after the map's saved GWORLD state 0xB846CE0C (measured, §10; the creator-seeded hypothesis is ruled out) | 0x9355: one draw (begin_battle) after the saved state |
| HQ ids, after 21a4d6d | — | the replacement keeps the placeholder's saved GTID (core fix) |

Replay (`tests/berlin_replay.rs`): record `r` runs before update `r + 1`, through `Mapper` +
`PlayIds`.

1. **First differing checkpoint: tick 17**, film (38, 48181) against aa-play (35, 19977).
   aa-play logs the 17 clocks and the 18 decree entries but **0 of the 3 getters**. The
   three record-2 recruits select 13389/13391/13393, which are empty in aa-play (HQ id row
   above), so no Recruit command runs, no production start, and no queue-0 getter. The same
   id shift later sends selections of 13399/13402 to aa-play's corps, giving 4 refused
   recruits.
2. Since the core HQ-id fix (21a4d6d), no alias is needed: the record-2 selections 13389,
   13391 and 13393 resolve to each player's own Corps HQ. Tick 17 then has count 38, with the
   getters exactly where the film needs them (after each recruit's 86); only the hash
   differs, by the RNG state row above. (Before the fix this needed hand aliases.)
3. Starting from the measured state 0x4F1E05E7 (the gate's diagnostic; §10), checkpoints
   17–391 match (23). **The next difference is tick 408**: (523, 60855) against
   (487, 15865). The film logs 48 getters in 391..408, aa-play 12 (6 in update 402, 6 in 403).
   These are the recruits completing; §10 lists the 48 draws.

## 10. Every world-RNG draw, measured live: Berlin battle start and first recruits (2026-10-25)

Tool: `reconstruction/qa-runs/compare-20261022/tools/RngTrace.cs`, built x86 with csc (QA copy only,
not in the repo). It is a debugger attached to the QA AA.exe while it plays the Berlin film
(hidden desktop, keys only: F11, F12, `SelectLaunchType LOAD_FILM`, Enter, F10, Enter). It sets
INT3s on 85D840 (draw, logged only when the state pointer is GWorld+0xD4), 4F02B0 (getter),
85D880 (seed) and the creation markers (4DC730, 58B0E0, 4F9C10, 4FA030, 4FBF40, 4FBA50, ...),
logging each with a return-address stack scan. Two runs gave identical sequences with no state
discontinuity, so 85D840 is the only writer of GWorld+0xD4 after map restore. The film's
checkpoints confirm both runs: the first battle getter's state is **0x4F1E05E7**, and its low
half 0x05E7 is the value solved from the film.

**Battle start: 6231 draws, from the map's saved GWORLD state 0xB846CE0C to the first battle getter.**

| # | phase / native | draws (Berlin) | what draws |
|---|---|---|---|
| 0 | CreateGWorld 76AA90 (getter, seed 85D880), GWorld::Init 4F27E0 | 0 | MapRestore 4FC600 then overwrites the state with the saved 0xB846CE0C |
| 1 | MapRestore → 4FCE31 → WorldParameters 50E8B0, per player kingdom in creator order | 8 | nation for an empty ("Random") creator nation: `do {getter 50EC92; draw 50ECA1; n = table[draw % count]} while (!n+0x68)`; then the faction: getter 50ECE5, 474B40 = eligible factions (+0x18) and, when more than one, draw 474C0F `% eligible`. Interleaved per kingdom: kingdom_0, _1, _3, _4 → rus chuikov, rus zhukov, ger rundstedt, rus konev |
| 2 | restore of saved things in ascending GTID slot order: DynamicDoodad InitExisting 4DC730 → component +4 | 6190 | one draw per DynamicDoodad: Disturbable 5904A0 (398) or Breakable 58F960 → 5904A0 (5792); delay = unit × component def+0x10 into deferred 50ACC0. Every Berlin doodad has exactly one component |
| 2 | (same walk) Supply InitExisting 58B0E0 with comp+0xC == 0 → 50AC60(0) → 50B2C0 | 4 | queue-1 registration of the city supply components: GTIDs 0xF68, 0xF72, 0xF78 (city_center01_small) and 0x2FA1 (berlin_city_center) |
| 3 | StartMission 4E6DB0 × 6 | 0 | |
| 4 | ResolvePlaceholders 4FDA70 → 4FC4A0 → Replace 4FBA50 × 4 (ascending placeholder slot 0x344D, 0x344F, 0x3450, 0x3451) → CreateGActor 4FA030 → CreateGThing 4F9C10 | 28 | per corps: random scale 49B440 (InitCreate 4E9FC0), Element Mind ctor jitter 530E50, then the component InitCreate loop 4D6F10: 5 × 563FD0 (queue-1 add; aa-actors `component_virtuals` slots Properties/Organization/Element/Supply/Construction/Detection/Lair/City). Supply InitExisting 58B0E0 then skips (already registered) |
| 5 | SessionLaunch 775BF0 → SyncDump 4F8AE0 → 4F6280 | 1 | the dump draw (aa-play `begin_battle`), then S reset 77D900(0,0) |

Not reached on Berlin: AssignName 4F4A20, CreateOrganization 4FA150, the GThing facing draw
(4EF2A0 flag bit 15), the variant draw (49B900) and the random-thing tables.

**First recruits (film interval 391..408: 48 getters, one draw each).** Production completion
58A414 runs for both rus HQ regiments in update 402 and for the ger one in update 403. Each
recruit makes 14 draws:

1. 5830AC → CreateGThing `regiment_*_hq_infantry` (the organization), 6 draws: scale 49B440;
   then component InitCreates in slot order: 563FD0 × 2, Economy 56F250 (queue 1, when
   element+0x348 is set), 563FD0, Hq 574610 → 4E2640.
2. SpawnMember 4FBF40 → CreateGThing `*_truck_infantry`, 8 draws: scale 49B440; Element Mind
   ctor 530E50. Then, inside the truck's GActor InitCreate (4D70F2), CreateGActor →
   `*_trailer_infantry`: scale 49B440 and 50AB10 (4D7520 ← 4F9F88). Then the truck's
   563FD0 × 3, then the truck's 50AB10.

Then each truck starts to move one update after creation (403, 403, 404): the Mover order
5773C0 queue-0 registration, then ActionGo 53037F, two draws each.

Total: 3 × 14 + 3 × 2 = 48.