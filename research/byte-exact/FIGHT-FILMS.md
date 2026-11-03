# Fight films: the original fighting on non-random maps (2026-11-01)

The Berlin film `Berlin2v2_0` has 4539 ticks, no kill order and no death, so it cannot prove combat, morale, retreats or abilities. The random-map films can't be used until the map generator is exact. These four films fill that gap.

## The films

Recorded on the QA copy. One slot is an idle human; the other three are computer players on Normal (all the rules panel allows without a mouse). All four decode with no failure. Creator class is `KKC_WorldCreatorLoad` for all.

| film | map | shipped as | ticks | checkpoints | decrees | money / experience | first kill order | first unit at zero health | actors at zero health |
|---|---|---|---|---|---|---|---|---|---|
| `Comrades2v2_0` | (2v2)Comrades, 128×128 tiles | developer map | 16179 | 951 | 7528 | 500 / 200 | 6169 | 6712 | 27 |
| `Farmhouses4FFA_0` | (4FFA)Farmhouses, 160×160 tiles, free for all | developer map | 30920 | 1818 | 45100 | 2000 / 200 | 4593 | 5000 | 107 |
| `Kursk2v2_0` | (2v2)Kursk, 224×224 tiles | official | 22413 | 1318 | 9932 | 500 / 200 | 2649 | 4856 | 119 |
| `Berlin2v2_1` | (2v2)Berlin, 224×224 tiles | official | 20195 | 1187 | 11101 | 500 / 200 | 2585 | 5760 | 133 |

`Comrades2v2_0` is the short small-map film. `Kursk2v2_0` and `Berlin2v2_1` are the large 2v2 films (23 and 21 game minutes). `Berlin2v2_1` is the first one to reach for: its start is already solved (see the replay table).

A (1v1)MountianPass film was rejected: 12445 ticks with no kill order and no death before it was stopped. It is kept under `rejected/`.

Files, all in the QA copy and none tracked:

- films: `reconstruction/qa-runs/compare-20261022/game/Profile/data/Films/archive-qa-20261101/`
- traces: `reconstruction/qa-runs/compare-20261022/traces/fight-*-native.txt` (about 1 GB in all)
- aggregates: `crates/aa-parity/tests/fixtures/qa_films_fight_20261101.json`

### Orders, against the old Berlin film

Counts of ActorCommands (86/87) and of decrees 70 and 23. A dash means none.

| order | Berlin2v2_0 | Comrades | Farmhouses | Kursk | Berlin2v2_1 |
|---|---|---|---|---|---|
| kill | - | 28 | 69 | 294 | 242 |
| guard | - | 1066 | 11585 | 785 | 1447 |
| retreat | - | 181 | 842 | 271 | 391 |
| rout | - | - | 7 | - | - |
| bombard | - | - | 4 | - | 15 |
| explore | - | 9 | 46 | 2 | 18 |
| stop | - | - | 313 | - | - |
| repair | - | - | - | 4 | - |
| construct | - | - | - | 5 | - |
| recruit_location | - | - | 1 | 1 | 1 |
| group_behavior_enable | - | 35 | 30 | 74 | 37 |
| research | - | 4 | 23 | 6 | 6 |
| move | 5 | 113 | 44 | 190 | 76 |
| recruit | 15 | 29 | 83 | 49 | 42 |
| unpack | 11 | 8 | 42 | 21 | 15 |
| set_formation | 33 | 66 | 324 | 274 | 280 |
| 70 FactionUseAbility | - | 4 | 54 | 14 | 11 |
| 23 TeamCommand (give resources) | - | 1 | - | 2 | 3 |

**No film has an air order** (`air_attack`, `air_recon`, escort). Aircraft appear only as ability transports (`aai_transport`) and bombs (`carpet_bomb_projectile`). No computer player ordered an air attack in 16 to 32 game minutes, so air parity still needs its own film.

### What the traces show of the fighting

| | Comrades | Farmhouses | Kursk | Berlin2v2_1 |
|---|---|---|---|---|
| actors damaged | 42 | 217 | 190 | 233 |
| organizations below maximum morale | 13 | 37 | 31 | 23 |
| at or under their breakpoint (wavering) | 4 | 10 | 10 | 10 |
| at zero morale (exhausted) | 0 | 0 | 2 | 4 |
| `cai_rout` samples | 80 | 286 | 327 | 452 |
| `cai_engage` samples | 308 | 2769 | 2319 | 2521 |
| `cai_kill` samples | 1214 | 2097 | 18001 | 6920 |
| `cai_bombard` samples | - | 47 | - | 89 |
| `cai_rampage` samples | - | - | 21441 | - |
| `cai_repair` / `cai_construct` samples | - | - | 332 / 22 | - |
| `cai_reform` samples | 10 | 82 | 70 | 155 |

More than one kingdom loses units in every film, and structures die too (`eeurope_cityblock*`, `rus_corps`). Deaths per owner are in the fixture.

A sample is one actor in one sampled tick. Actors are sampled every 8th tick, so one that dies and vanishes between samples counts only under `vanished` in the fixture. The zero-health counts also include a few things that aren't units, such as smoke grenades.

## Native traces

`tools/FightSampler.cs` is a copy of `NativeSampler.cs` with three additions, all read-only:

- `own=`: the owner kingdom index (Actor+E8 → +188).
- `ms=`: the Mind's vtable and its current state's vtable (0x529F70).
- `mor=`: Organization morale +5C, maximum +60, breakpoint +B8, living +64, combat value +68/+6C, attack efficiency +84/+88, watched target +D4.

`aa_parity::native_trace` ignores the new keys, so the existing reader and `state_compare` work on these traces. Cadence is every tick up to 300 (1200 for Berlin2v2_1), then every 8th.

Proven on the original:

- **Each trace belongs to its film.** At every checkpoint, the film's S entries that are neither clocks nor decrees equal the getters the sampler saw: 951/951, 1818/1818, 1317/1318 and 1187/1187. The one Kursk checkpoint left out is past the sampler's last whole tick.
- **Playback is deterministic.** `Comrades2v2_0` was captured twice, once at game speed +2 and once at normal speed with other jobs running. `tools/trace_compare.py` finds them equal through tick 16168: 94543 draws, 94227 getters, 218559 actor lines and 9136 kingdom lines. Heap pointers, thread ids and sequence numbers are left out of the comparison. Only this film was captured twice.
- **The Berlin start is map-determined.** `Berlin2v2_1` has a different seed from `Berlin2v2_0`, yet the same 6231 draws and first getter state 0x4F1E05E7 (SYNC_LOG section 10).

## The gate

`crates/aa-parity/tests/fight_films.rs`, all ignored. It reads the films and traces through `AA_GAME_ROOT`.

| test | what it does |
|---|---|
| `fight_films_decode_to_their_recorded_aggregates` | every film decodes to its fixture row and has kill orders |
| `fight_films_replay_into_aa_play` | replays like `berlin_replay.rs`; prints the plan, matched checkpoints, first difference, refusals |
| `fight_film_traces_account_for_their_checkpoints` | the getter check above, and the fixture's trace row |
| `fight_films_native_state_first_divergence_per_lane` | first divergence per lane, as in `berlin_native_state.rs` |
| `fight_films_refresh_fixture` | rewrites the fixture, only with `AA_FIGHT_FILMS_WRITE=1` |

All five pass. The replay and lane tests report differences without failing on them, because nothing about the fighting matches yet.

### Replay against the 2026-11-01 working tree

This is where aa-play stands today. It proves nothing about aa-play's fighting, because every film diverges before its first battle.

| film | checkpoints matched | first difference | film / aa-play S count | cause |
|---|---|---|---|---|
| `Berlin2v2_1` | 26 of 1187 | tick 459 | 594 / 592 | 2 getters at tick 458 are missing: a Mover registration (site 50B2E4, chain 577469 ← 530379) and ActionGo (530384), i.e. a unit starting to move |
| `Comrades2v2_0` | 0 of 951 | tick 17 | 38 / 38 | RNG state only |
| `Farmhouses4FFA_0` | 0 of 1818 | tick 17 | 38 / 38 | RNG state only |
| `Kursk2v2_0` | 0 of 1318 | tick 17 | 59 / 38 | RNG state, and 21 more getters in the first 17 ticks |

World-RNG draws from map restore to the first battle getter (`tools/trace_start.py`), which is the first thing the three maps other than Berlin need:

| caller | what it is | Berlin2v2_1 | Comrades | Farmhouses | Kursk |
|---|---|---|---|---|---|
| 5904C4 | Disturbable delay | 6190 | 1660 | 1102 | 4054 |
| 49B458 | scale in `CreateGThing` | 4 | 541 | 124 | 4 |
| 4EF40E | GThing facing | - | 537 | 108 | - |
| 50B2EA | queue registration | 24 | 24 | 73 | 25 |
| 530E65 | Element Mind jitter | 4 | 4 | 5 | 14 |
| 50ECA6, 474C14 | nation and faction picks | 4 + 4 | 4 + 4 | 4 + 4 | 4 + 4 |
| 444AA1, 5798D4 | in 444A90 and 5797E0, not identified yet | - | - | - | 3 + 3 |
| 4F62B8 | sync dump | 1 | 1 | 1 | 1 |
| total | | 6231 | 2775 | 1421 | 4112 |
| first battle getter state | | 4F1E05E7 | 34B31E6A | A8A8A8DC | 0ADB7526 |
| things created | | 4 | 541 | 124 | 4 |
| placeholders replaced | | 4 | 4 | 16 | 4 |

The facing draws are the branch SYNC_LOG section 10 lists as not reached on Berlin. Kursk also has placed units active from the first tick: its 24 getters in ticks 1 to 17 are 8 at 50B2E4, 7 at 52691F (in 5267E0), 5 at 530384, 3 at 563C59 and 1 at 525132. aa-play notes 44 saved periodic queue entries on that map it cannot resolve.

### First divergence per lane

Tick of each lane's first difference from the native trace. Berlin2v2_1 keeps the RNG until 458. Most other lanes first differ where the first truck and trailer appear, or at tick 1 for zones and heights.

| lane | Comrades | Farmhouses | Kursk | Berlin2v2_1 |
|---|---|---|---|---|
| rng | 1 | 1 | 1 | 458 |
| economy | 592 | 744 | 408 | 480 |
| existence | 592 | 744 | 1 | 686 |
| combat | 592 | 744 | 680 | 686 |
| movement | 408 | 1 | 1 | 1 |
| production | 10480 | 6256 | 3760 | 3608 |
| supply | 592 | 752 | 680 | 686 |
| fog | 1 | 1 | 1 | 1 |

The combat lane's first difference is a truck's health (500 against 2000) on the tick the first truck and trailer appear, not a fight. Every lane first differs before any battle, so none of this says anything yet about aa-play's fighting.

### What the orders do against the traces (2026-11-02)

Every ActorCommand now maps, and the corpus report has no `NO EQUIVALENT` line. Still refused in aa-play, with zero orders in the corpus unless noted: `build_structure`, `commission`, `upgrade`, `attach`, `detach`, `cancel_flare`, and the four flare team commands (no host). `air_attack` and `air_recon` map and stay refused until the aircraft fly (2,344 orders in the corpus, none in these four films).

`order_fights.rs` reads the orders against the native traces. These runs are renditions compared with the original, and each film has drifted from native long before its fighting, so a difference here is not by itself a handler fault:

| test | what it shows |
|---|---|
| `the_fight_films_fight` | deaths, routs, refusals, unresolved orders |
| `mind_state_confusion` | every organization sample: native Mind state against ours |
| `order_effects` | the state 16 ticks after each order, by order and by what the target is. `AA_EFFECTS_SHOW=order,native,ours;...` lists the cases |
| `order_gone` | why an order's thing is missing: never made here, made later, or dead here |
| `order_rows` | one film's orders in order. `AA_ORDER_ORG=org:from:to` follows one organization by tick |

Read from `order_effects`: most remaining differences are knock-on. "Dead here" and "never made here" account for most missing selections, and retreat `sleep/move` cases are regiments standing somewhere else. A city's owner also differs here, which turns a native kill into a Move at a thing that is no longer hostile.

**Refusals against the original's own log.** 0x4CDE00 logs `Faction '%s' unable to use ability '%s'` when 0x4C8F20 refuses, and the logs of two of these recordings were kept: Comrades holds 4 such lines (Field Marshal Montgomery, Corps Reorganization), Berlin2v2_1 holds 10 (Marshal Zhukov 2, Marshal Konev 8, Corps Reorganization). Our replay refuses 4 on Comrades (`special_operation_unique`) and 10 on Berlin2v2_1 (`special_operation_unique` 7, `special_operation_not_enough_experience` 3). The counts agree; the log gives no reason, so the reasons are not checked, and a replay that has left the original can agree by chance on the later ones. Farmhouses and Kursk have no log. On Farmhouses all 8 `special_operation_invalid_target` are `resupply` orders whose target in the film is a member of a regiment, not the regiment: `admit_with` gives the member's organization only on the HUD path. Whether the original refuses them too needs a log of that recording.

Found and fixed through it: delivery checks the command mask (0x4ED360 vt+150, 0x4D9B60 vt+38), so a retreat from an engaged regiment too low in morale is not given; an unpack to a group gives the members without the command a Move (0x4DE7F0).

The mask is `orders::mask`: the exact contributors of `aa_actors` (0x571640, 0x57B390, 0x582C40, 0x583FC0, 0x589A60, 0x569C70, 0x565140, 0x575FB0) in the order of 0x4D1350, and the Mind's 0x521E10, 0x530510, 0x518720. Every command is gated by it, with the owner and 0x4CF120 (vt+150), except 37 and 38, which never reach a thing. Not proven: the actor states it reads are derived from aa-play's state, not stored (`orders::mask::state_bits`), and it is rebuilt when asked where the original keeps a word rebuilt at each state change. Against the traces of the four fight films, an organization's states 4, 6, 8, 12 and 19 mostly agree (the rest follows the replay's own differences), state 22 is never set here, and a lone actor's Mind states (4, 8, 9, 12 in the trace) are not derived at all.

### The fight against the traces, by the numbers

Measured by `tests/order_fights.rs` (`the_fight_films_fight`) against counts read from the native traces: deaths are actors reaching zero health, routs are organizations entering `cai_rout` (the earlier "far fewer routs" compared events with samples, which is no comparison), shots are the original's `0x4C8190` draws (one per ability started) against ours. Free replay: each film has left the original long before its fighting, so a number close to the original's is not parity.

| film | deaths ours / native | first death ours / native | routs ours / native | shots ours / native |
|---|---:|---:|---:|---:|
| `Comrades2v2_0` | 8 / 27 | 7341 / 6712 | 0 / 4 | 6890 / 349 |
| `Farmhouses4FFA_0` | 82 / 107 | 3026 / 3032 | 32 / 15 | 1510 / 2666 |
| `Kursk2v2_0` | 94 / 119 | 4841 / 4584 | 28 / 15 | 1808 / 3541 |
| `Berlin2v2_1` | 129 / 133 | 4949 / 5760 | 41 / 25 | 2147 / 2929 |

Before the hosting: deaths 26, 137, 75, 88; first death 9096, 3039, 4794, 4957; routs 7, 20, 15, 46; shots 1538, 3274, 2150, 2349. The shots were about four times the original's on Comrades, because an ability's animation and the rule that the Mind waits for it were not there.

### The fifth round (2026-11-03)

Every order state of a regiment is on its Mind now; `PARITY-CENSUS.md` has each cause and its proof. Same measure as above:

Measured by `tests/order_fights.rs` (`the_fight_films_fight`) on the tree of 2026-11-03, with the other lanes' work in it (the Berlin gate there: 140 of 266, first 2397). Free replay: each film has left the original before its fighting, so a number close to the original's is not parity. Native shots are the `0x4C8190` draws; 901 of Kursk's are the engineers' Repair (`0x5359D0`), so 2640 fight there.

| film | deaths ours / native | first death ours / native | routs ours / native | shots ours / native | checkpoints, first difference |
|---|---:|---:|---:|---:|---:|
| `Comrades2v2_0` | 21 / 27 | 6820 / 6712 | 3 / 4 | 222 / 349 | 128 of 951, 2193 |
| `Farmhouses4FFA_0` | 108 / 107 | 3034 / 3032 | 12 / 15 | 1810 / 2666 | 97 of 1818, 1666 |
| `Kursk2v2_0` | 75 / 119 | 4856 / 4584 | 32 / 15 | 1731 / 3541 | 126 of 1318, 2159 |
| `Berlin2v2_1` | 125 / 133 | 6092 / 5760 | 31 / 25 | 2487 / 2929 | 138 of 1187, 2363 |

Before this round: deaths 8, 82, 94, 129; first death 7341, 3026, 4841, 4949; routs 0, 32, 28, 41; shots 6890, 1510, 1808, 2147.

The first-death column counts anything that reaches zero health, so two of its native figures are no fight: Farmhouses' 3032 (and ours 3034) is a `smoke_grenade` running out, its first hit on a soldier is at 4864 and its first soldier dies at 5000; Kursk's 4584 is the city block `eeurope_cityblock12x20_02`, its first soldier dies at 4856. In the client, `Farmhouses4FFA_0` at tick 5710 has one thing dead beyond that grenade where the original has lost nine soldiers by 5456: the totals agree (108 against 107) while the fight does not come at the same time.

The same round's trace of the original on a random map: `traces/film-rmg-ba44-native.txt` (film `ba445000bb`, 2400 ticks).

## Recording recipe

Everything is in `reconstruction/qa-runs/compare-20261022/tools`, on hidden desktops `AAFIGHT<tag>`, with posted keys only:

- `fight.ps1`: the driver, with `$env:FIGHT_TAG` selecting the desktop and state files.
- `fight-record.ps1 -Map -Label -MaxTicks`: records and archives a film.
- `fight-play.ps1 -Film -Trace -MaxTick -Dense -Every -Speed`: plays a film under the sampler.
- `FightWatch.exe`: a read-only monitor of ticks and deaths during a recording.
- `trace_compare.py`, `trace_start.py`.

What was learned:

- **Choosing a map.** The profile's `quote` key runs `ExecHere Localization/Hotkeys/qa_fight.txt`. `Select Maps/DeveloperMaps/(4FFA)Farmhouses.RMP` works even though that map is not in the dialog's list. A file name with a space, such as `(1v1)Golden Fields`, was avoided and not tried.
- **Choosing a film.** The film dialog keeps focus in its name box, and tried hotkeys do not fire: `quote` and numpad-1 typed characters, while F12 and Pause did nothing. Typing a listed name and pressing Enter does work. A path into a subfolder gives "File missing", so the film must be copied to the top of `Films/` as `FightX.RFM` and `.TGI`, then `FightX` typed. These copies have been removed again.
- **Rules cannot be changed from the keyboard.** The rules panel ignored the keys tried (typing in Money, Tab, Down), so difficulty stays Normal and money is the map's.
- **Film file timing.** A film is written only when the game exits, and it stays locked while recording. The slot comes from `LastFilmIndex`, read when the game starts: two games started close together were given the same slot. So record one film at a time and copy it out at once. Other games running from the same profile log sharing violations on the locked slot.
- **Developer maps warn at load.** Farmhouses reports 25 unknown things and an unrecognized terrain variation; Comrades 2 and 4. The game carries on after Enter, and aa-play loads both with terrain notes. Berlin has no warnings.
- **`NativeSampler` can deadlock.** When two threads hit breakpoints together, each suspends the other and every thread stays suspended. This happened while a film was loading. `FightSampler` puts the second thread back on its INT3 instead. `NativeSampler.cs` is unchanged.
