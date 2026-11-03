# The computer player against the original's

What the SAI of `crates/aa-play/src/ai*` does, read against the original's code. Addresses are AA.exe. Three things decide how close a computer's orders can get:

- The SAI's RNG is seeded from a millisecond clock (0x7A0AA0), so two games of the original on the same map do not give the same orders. A real film's orders can be compared in kind and in reasoning, not one by one, until the whole SAI is exact and the seed searched.
- Its decisions leave only as requests (0x7AF220). The film never needs the SAI: the original plays a film with its SAI off (0x7A0080 on Session+0xE4), so **any** orders our computers give must play the same in the original if the world simulation is exact. A film of ours going out of sync is a world difference shown up by those orders.
- The goal engine's selection (`goals.rs`) is a rendition that follows the original's values. The order building below it is what this page is about.

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

The order was the cause: 0x4ED360 delivers to the selection, and a member ordered past its organization takes another path in the original. What stops the last film at tick 1326 has not been traced.

## Tools

- `crates/aa-parity/tests/sai_shadow.rs`: the SAI thinks on a real film's world without its orders being applied, and the census and timeline of both are printed.
- `crates/aa-parity/tests/film_round_trip.rs`, `order_shapes_against_the_original_s`: every shape a computer of ours sends against the real films.
