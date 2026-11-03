# The computer player against the original's

What the SAI of `crates/aa-play/src/ai*` does, read against the original's code. Addresses are AA.exe. Three things decide how close a computer's orders can get:

- The SAI's RNG is seeded from a millisecond clock (0x7A0AA0), so two games of the original on the same map do not give the same orders. A real film's orders can be compared in kind and in reasoning, not one by one, until the whole SAI is exact and the seed searched.
- Its decisions leave only as requests (0x7AF220). The film never needs the SAI: the original plays a film with its SAI off (0x7A0080 on Session+0xE4), so **any** orders our computers give must play the same in the original if the world simulation is exact. A film of ours going out of sync is a world difference shown up by those orders.
- The goal engine's selection is the original's own, through the exact ports (`ai/selection.rs`). The candidates' class priorities (`goals.rs`, `value.rs`) follow the original's values; the goal log below is what checks them.

## The original's goal log, the oracle for decisions

The original writes every goal of every think when run with `SAIDebug=1 GoalPriorityCSV=<file>` on its command line (0x7A5BF0 opens the file, 0x7A5200 writes a row): goal type, label, the class base priority (+38), the ego's multiplier, bonus and minimum, the fuzz, the inertia, the final base. A row is written when the priority before fuzz is above zero and at its minimum.

One such game is kept beside its film (Chuikov, a Southern Europe random map, an idle person): 23 thinks over 660 s. The film's map is the log's map, so `sai_shadow` with `AA_SHADOW_LOG` writes the same rows from the same state, region numbers included. What the log has shown, and what is now the code:

- **Selection is the original's own.** `ai/selection.rs` runs 0x7A93B0 through the exact ports in `aa_mind::sai` (`engine::select_goals`, `assign::evaluate`, `assign::assign`, `goal_base`). A goal is selected when its current priority is above zero, with actors or not: every ATTACK_REGION and DEFEND_REGION goal is selected from the first think (+250 inertia in the log from the second on), an EXPLORE goal without an actor is not. A BOMBARD or AIR_ATTACK goal reads its region's attack goal as selected, which is what puts the air attack rows in the log from the second think and the x10 on bombard values.
- **Goals hold their actors** from think to think while they stay selected, and the others are in pools by category (0x7A1150).
- **A new actor with a RECOVER need is that goal's at once** (0x7A83A0): selected, its fuzz never drawn (-1 in the log), the record in the recover state. An organization's health here is 0x4D0230(1), the members' fractions over the layout's slots. The log's 456.1 on every fresh regiment is (1 - (4/9)^3) * 500, above `max_hp`, so natively a regiment registers as its first member stands and its record is in that state from then on. Yet the same think its attack goal has it: the first regiment's Recover row at 90.31 s, its move at the village at 90.4 s. How it leaves RECOVER there is not read (0x7A3740 asks for more than the attack goals give; with the state played the first attack comes a think late, at 120 s). Here it registers at the think that first sees it, so the regiment at 4 of 9 goes straight to the attack goals, as in the film, and those Recover rows are missing from our log.
- **Flares replace flares** (0x7B0610): whatever their target, a new one takes out those that last and are worth no more, and is left out when one lasts and is worth at least as much. The log's +500 on Vranje to 120 s, Prevenza from 150 to 180 s, Pisciotta at 210 s alone, Jajce from 240 to 390 s are one slot. The start's city has one from the first think.
- **RECOVER +58** (0x7C5740) sends its order every think it stays selected. It was not in the export; read from the instructions.
- **REPAIR** goals exist (0x7A83A0, base 0x7A4230, class 0x7A31A0, execution 0x7A40F0, order 0x7A5630). A rendition: only structures are targets, the +9C component is not read.

Goals that are not there and why:

| goal | what makes it | here |
|---|---|---|
| ATTACK_ACTOR, DEFEND_ACTOR (0x7C6E40, 0x7C7780) | only 0x572A10: a flare on an actor from another kingdom of the side (0x572E50 team flare, 0x573440). No row in any of the four goal logs | not built; `ai::team_flare` makes region flares |
| CONSTRUCT (0x7C1DF0, 0x7C1950) | an actor with a builder, per region and per building | not built, no row in the early logs. Real computers send 263 construct orders in the corpus |

Kill and move-at orders of a real computer name the thing itself: in the Berlin film they are city blocks (the organization is not named).

Measured on the film's world, all 23 thinks (`sai_shadow`, `AA_SHADOW_GOALS`): the types of goal agree in count nearly everywhere, RESEARCH, RECRUIT, EXPLORE and DEFEND_REGION in every think. Of the combat goals 264 are in both, 6 the original's alone, 1 ours alone; the inertia differs on 3, where before this work it differed on 135 of 150; the class base is off by 4.8% at the median and 19% at the 90th percentile (ATTACK_REGION by 0.1%). DEFEND_REGION is the bigger part: a constant 3 to 7 on each, which looks like `enemy_border_influence` (0.2 times a border influence ours has too small) and is not traced. Rows still short: the fresh regiments' RECOVER below, and the air and bombard rows of a goal log's target ours does not see at that think.

### The same map, played

`AA_SHADOW_LIVE` lets our computer play the goal log's map by itself, against the same idle person, with its orders applied:

| | the original's game | ours, SAI seeds 1, 2, 3, 4 |
|---|---:|---:|
| first regiment ready | 76 s | 75, 75, 165, 75 s |
| first attack order | 90.4 s | 90.1, 90.1, 180.1, 90.1 s |
| first city taken | by 150 s | 114, 140, 208, 117 s |
| third city taken | by 240 s | 495, 214, none in 660 s, 196 s |
| recruits in 660 s | 9 | 8, 8, 7, 9 |

Whether the first money goes to a regiment or to the saving for a Mechanized HQ is the fuzz of those two goals (seed 3, and seed 0, which saves for six thinks). The original's log has the same choice at thinks 3 to 7: the regiment on top at 60, 120 and 150 s, the HQ at 90 and 180 s. So the seed matters: `KKC_SAI` seeds from a millisecond clock (0x7A0AA0), and the client now seeds from the launch's own seed, which is new each launch and the same on every LAN node. Before, every game a person played drew the same fuzz.

What the live run showed and is now right: 0x7B4FE0 compares with the organization's +70, what was last ordered, not +74, what its Mind state asks now, so a regiment on the move is not told its formation again every 2 s.

## Where the orders come from

Goal classes (vtable, type at +4, +58 execute, +5C one actor's order, +78 base priority):

| type | class | vtable | +58 | +5C | +78 |
|---|---|---:|---:|---:|---:|
| 0 | ATTACK_REGION | A9CAC8 | 7C9710 | 7C6C20 | 7C5AE0 |
| 1 | ATTACK_ACTOR | A9EDD8 | 7C6FC0 | 7C7330 | |
| 2 | DEFEND_REGION | A9CB58 | 7CA1D0 | 7C5C60 | 7C5F80 |
| 3 | DEFEND_ACTOR | A9EE68 | 7CA450 | 7C61A0 | |
| 4 | AIR_ATTACK | A9ECC8 | 7C79C0 | 7C7C30 | |
| 5 | RECOVER | A9CC70 | 7C5740 | 7C9E20 | 7C6640 |
| 6 | REPAIR | A9CBE8 | 7A40F0 | 7A5630 | |
| 7 | EXPLORE | A9E950 | 7C3310 | 7C3440 | 7C29F0 |
| 9 | CONSTRUCT | A9CD80 | 7C1DF0 | 7C1330 | |
| 10 | PACK | A9CE90 | 7CB760 | 7CB450 | |
| 11 | UNPACK | A9CF18 | 7CB940 | 7CB540 | 7CBC40 |
| 20 | BOMBARD | A9ED50 | 7C8340 | 7C84F0 | |

The command constructors and the shapes of the real films' computers:

| constructor | ordinal | flags | n in the corpus |
|---|---|---:|---:|
| 4DD520 guard a thing, the zero direction of B279C0 | guard 2 | 0x49 | 53793 |
| 7C57C0 retreat at a place | retreat 7 | 0x44 | 34203 |
| 4CA330 kill a thing | kill 0 | 0x41 | 20299 |
| 7B4FE0's set_formation with a FormationData | 33 | 0xC0 | 17534 |
| 4DDE60 move at a thing, zero direction, scalar 0 | move 5 | 0x59 | 4129 |
| 7AF220 before an order to several actors | group_behavior_enable 37 | 0x240 | 1968 |
| 4D9E20(6) stop | stop 6 | 0x40 | 934 |
| 7C29C0 explore at a region's centre | explore 9 | 0x44 | 290 |
| 542A30 guard at a place | guard 2 | 0x4C | 66 |
| 4D9E20(8) rout | rout 8 | 0x40 | 40 |

## The order state of a SAI record

Every actor record (actor+154) has +10 its goal, +20 pending (7B4C50 until 7AF220 sends), +3C its RECOVER goal, +30 an order state (7A1070): +0 kind, +4 a thing, +8 a region, +C the place retreated to. Kinds 0 attack, 1 defend, 2 recover, 5 the unpack goal's. `rai::OrderState` keeps the same.

## Goal execution (each goal-engine think, 30 s)

- **ATTACK** (7C9710, 7C6FC0): 7A5E10 sends off those its RECOVER goal wants; the rest get state 0 and one group order, move at the target thing (0x59), or at the region's centre (0x5C) when the goal has none.
- **DEFEND_REGION** (7CA1D0): 7C9CE0 with the actor's worth to the goal; the rest hold (state 1, the goal's region) and get guard at the goal's +4C, the region's most valuable own structure (7BD360, 7C5F80).
- **RECOVER** (7C9E20 -> 7C8AF0): the record keeps its place while it stays friendly supply and the actor is not under attack; an actor at rest in supply stays; else 52BB40 draws places in the allied supply things. Retreat there; stop when there; with no place an actor under attack routs (`+118`), another stops.
- **EXPLORE** (7C3440): explore at the chosen region's centre.

7A5E10 (also from 7B7D30 while goal +6C is set, set by 7C5380 when a recovering actor joins): the actors its RECOVER goal wants (7C9CE0) leave unless committed (7C5290: state 0 within attack_commit_distance of the goal's region); uncommitted and the stayers' value under cv_advantage_target (ego +304) times the region's hostile influence, all leave.

7C9CE0: R's base above zero, R valid (7C8AA0: morale under critical_morale, or a supply thing of a friend exists), and no order state, state 2 (R's +40, 7C5470) or the worth to the goal below R's base.

## The reactive AI (7B8AC0, every 2 s)

1. 7B7250, unit retreat. With attackers or targets, under unit_retreat_max_health, its side and the other's within unit_retreat_depth steps of who fights whom (even and odd steps); under unit_retreat_cv_ratio and unit_retreat_probability: 7B5210, to its RECOVER goal. The record's +38 countdown is only ever decremented.
2. 7B64F0 over all RECOVER goals. In state 2 (or held): a supply thing gone: back to the region's DEFEND_REGION goal when morale and health allow, else the order again; standing away from the place: the order again.
3. 7B7750 over the selected DEFEND_REGION goals. Free actors (no attacker) and busy ones; hostile things within attack_distance (7B01F0). Outnumbered under min_attack_advantage: only the hostile in a fight (7AEDE0). 7B5300 up to min, then max; with none free the busy up to min. The rest stay in state 1, and one standing outside its region gets the guard of that region's goal.
4. 7B7D30 over the selected ATTACK goals. Lists by disassembly (0x7B7FE7..0x7B80E3): engaged or flagged; bombard capable (7A22A0); within attack_distance; farther. The target thing first up to max; the hostile around up to min and max; bombarders 7B5970; a settlement target last, as a move at it.
5. 7B8540 (CONSTRUCT) and 7B8800 (UNPACK), not played: the goals here have no such actor lists.
6. 7B6810, not played: the sally_forth and recall of garrisons (no shape in the corpus).
7. 7B4FE0, formations: by goal type and 7BAFD0's class (3 with a hostile of value in or next to the region, 1 while hostile over own influence is under threatened_region_influence_ratio, else 2), from the ego's GoalFormation and DefaultFormation; template_sai_aa gives regiment_defensive, regiment_defensive, regiment_aggressive for the four combat goals.
8. 7AD550 spaces the queue, capped by max_rai_commands.

7B5300: each hostile in turn takes the nearest actors until they hold its value times the advantage. An actor already on it leaves the list with no order; one that cannot attack it (4D4FF0) waits for the next. Kill (0x41), or move at it when it has +D4.

`region_retreat_*` and `halt_retreat_cv_ratio` in the template have no reader: 4B0560 does not load them.

## Trigger calls

- 79F5D0 (`ai::set_profile`) -> 744EA0 writes only the player's +18. Goals, records and think times stay; 7AD180 picks the ego from the new profile at the next think.
- 572E50 (`ai::team_flare`): 7C5F80 and the attack goal read the flares; here they are internal flares by region. A rendition.

## The place of an unpack (7CBC40 -> 557010 -> 556940)

Each candidate region (7BB1C0 within adjacent_region_depth, num_random_regions drawn from the SAI's RNG, the goal's last region) gets a place first and the value of that place after (4A9AE0: the friendly bonus measures from the place). The search starts at the region's centre with the definition's larger side as its spacing, at least a tile, and `ceil(0.75 * 7AA180 / spacing)` rings: a direction of the B39AF8 table and a first ring from the SAI's RNG, `6 * ring` places a ring, rotating by -2pi / places, twice over. The real films' sites are not round numbers because they are region centres. What this port does not have of it: 5553E0 is `deploy::spot_clear` and the movement owner's test, 556480 (no hostile zones under the footprint) is the hostile supply radii, and it asks that the truck can drive there.

## What a film shows against this

`sai_shadow` on Berlin2v2_0 over 4000 ticks, the SAI thinking on the film's world without its orders applied:

- The goal engine's thinks fall on the film's ticks: the first orders of each kingdom at tick 2, then 482 and 483, 962, 1442 for the first player; 746, 3626 and 842, 3722 for the others. The other two come one tick after the film's. 7AD550 stamps its due times with the millisecond clock 0x59ABA0, so a tick either way is the original's own jitter.
- What they order at those ticks agrees often in kind and sometimes to the thing: move at 0xF72 at 1442, move at 0xF78 at 3625 and 3721, a research at 483, a recruit where the film recruits. Where it differs it is the fuzz of the priorities, which is the SAI's clock-seeded RNG, or a value not yet exact.
- A real computer's order names the organization where a member acts (the unpack's truck), which is what the original's delivery expects.

## The original on our computers' film

A headless Berlin battle of four computers, its film played by the QA copy of the original on its own S:

| film | stops at | checkpoints passed |
|---|---:|---:|
| before this page: the truck itself selected for its unpack | index 647, tick 493 | 28 |
| the same with the orders of the goals and the reactive AI in their shapes | index 647, tick 493 | 28 |
| the truck's organization selected, as a real film does | index 1787, tick 1326 | 77 of 88 |
| the same, on the tree of 2026-11-02 before the bound selection | index 1787, tick 1326 | 77 of 88 |
| with the bound selection, other unpack sites | index 921, tick 697 | 40 of 88 |

The order was the cause: 0x4ED360 delivers to the selection, and a member ordered past its organization takes another path in the original.

The stop at tick 1326, traced with the sampler on a film of the final tree (`our_film_against_the_original_s_trace`): every lane of the world is equal to tick 1340 but the stream. Our replay has one getter pair more at tick 1316, a mover order and an ActionGo from 0x537150, with no change of any actor's next node, and the same pair comes 3 ticks early once more (1337 against 1340); the original has 249 getters to our 251. The only mover then is the truck `gbr_truck_ammo` (0x3464), which at 1315 left its node (369.0, 183.677) with the same pair as the original. Which branch of 0x537150 sends the second one is not read yet: its calls to 0x5300A0 are a mover on its destination and 0x531170 finding nothing. It is `movement/element.rs`, not the SAI.

A film of the tree with the bound selection stops earlier, at index 921, tick 697, and for another cause: an unpack the original refuses at the site. The first player's HQ truck (0x345B) is told to unpack at (181.36, 413.24) at tick 482. In the original it drives there, stops at (181.42, 413.21) at tick 685 and stands, its Mind idle, to the end of the trace. Ours deploys at 689, and from there on nothing agrees. So `deploy::spot_clear` lets a place through that the original's unpack does not (0x5553E0, 0x4F3640 at arrival), and our `find_site` picks such places; a person's order at that place would differ the same way.

## Tools

- `crates/aa-parity/tests/sai_shadow.rs`: the SAI thinks on a real film's world without its orders being applied, and the census and timeline of both are printed.
- `sai_shadow` with `AA_SHADOW_LOG` and `AA_SHADOW_GOALS=<the goal log>`: the goals of every think against the original's log of the film's game. `AA_SHADOW_LIVE` lets the computers play the film's map with their orders applied, `AA_SHADOW_SEED` sets their RNG.
- `sai_shadow`, `our_film_against_the_original_s_trace`: a film of ours against a FightSampler trace of the original playing it, first difference by lane and the getters by window. `AA_SHADOW_WATCH` and `AA_SHADOW_PATHS` print things around a tick.
- `crates/aa-parity/tests/film_round_trip.rs`, `order_shapes_against_the_original_s`: every shape a computer of ours sends against the real films.
