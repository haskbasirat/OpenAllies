# Berlin film parity census: everything between us and 266 of 266

Census of 2026-11-01 against main at c2b4099 plus the uncommitted working tree of that day (the
supply/fog rewrite, combat and scheduler edits were in it). Addresses are AA.exe. Line numbers are
of that tree and will drift; function names are given so they can be found again.

**Where it stands: 32 of 266 checkpoints, first difference at tick 561 (film (731, 52277), ours
(731, 28237)).** The S count is still right at 561: the getters run, one tick off.

The stream is the key to reading all of this. Every getter draws once, so both sides walk the same
LCG chain and the k-th getters have the same state whatever their tick. A checkpoint hash therefore
depends on *how many getters run before each clock entry*. 266 of 266 means: the same 6986 getters
in the same updates. Everything below matters to the film hash exactly as far as it moves a getter.

## 1. How to read and reproduce it

```
CARGO_TARGET_DIR=target/s-parity-census AA_GAME_ROOT=<game root> AA_PARITY_CENSUS_DIR=<dir> \
  cargo test -p aa-parity --features play --test berlin_native_state --release --offline -- --ignored --nocapture
```

`tests/berlin_native_state.rs` plays the film four ways and writes each report as JSON to the dir.
`AA_PARITY_WATCH=<native gtid hex>:<from>:<to>,...` prints things side by side;
`AA_PARITY_CENSUS_RUNS=free,forced-rng` selects runs. One run takes about 3 s.

| run | what is put back on the native values after every sampled tick |
|---|---|
| free | nothing: the replay as it stands. After the first difference it is mostly knock-on |
| forced-rng | GWorld+D4. Each getter window starts from the same state, so its first difference is exact |
| forced-rng-economy | also Kingdom +1C4. Gave the same getters as forced-rng: nothing here depends on the stock |
| forced-all | also position, direction, height, anchor, health. Mover speed and Mind state stay ours, so the movement rows there are only a hint |

What the harness does now (`crates/aa-parity/src`):

* `state_compare`: every lane runs to the end of the film and collapses into causes. A signature is
  the field, the kind of thing and how the two sides differ. `first-on` counts the things a cause is
  the first difference of, and a cause never first on anything is printed `KNOCK-ON`. `getter_census`
  compares the getters per sampled window and names each native getter by site and chain.
* `replay::Pairing`: native things created in battle are paired with ours by definition, in order.
  Without it one slot shift (cause K1) sends every later order and comparison to the wrong thing.
  The pairing is a diagnostic: the gate (`berlin_replay`) does not use it, so it still shows the
  shift's refusals.
* `mapping`: the fixes in section 2.

**What each claim rests on.** Native facts come from the sampler trace of the original (every tick
to 1200, then every 8th) and the pseudocode. "Proven" on a port means its own fixture replay; none of
these ports has run against this film. Each cause's size is an estimate. Causes M4 to M7 are read from
the first place they show: later movement causes can hide behind them, so **re-run this census after
each movement step**.

## 2. Fixed in the mapper (this round): 28 to 32, all 50 orders resolve

Every unresolved film order in the brief is explained. Only `group_behavior_enable` lacks an aa-play command.

| symptom in the gate | cause | now |
|---|---|---|
| "unpack truck actors, position and facing" (first at 482) | the film's unpacks have flags 0x44: position, no auxiliary words, so `has(3)` failed. And the selection is usually the HQ regiment, not the truck | position only. Delivery 0x4D9B60 passes a command to the organization's leader (+0x7C, +0x44) when the selected actor lacks the command's component |
| "recruit buildings and object definition" (first at 962) | the same delivery: selection is the HQ regiment. The native chain 0x4D9BE8, 0x4CE6BA, 0x583BF8 is identical to a recruit on the building | leader, as above |
| "formation organizations and FormationData" (first at 1217) | `PlayIds::formation` was a stub | slot through `Registries::formation_ids`. Slot 2 = `regiment_column` is confirmed by the measured speed (M6) |
| thing slots 13413, 13416, 13428, 13430, 13434, 13450, 13454, 13466 | knock-on: the recruit was unresolved or, later, the id shift K1 | resolve with the pairing; K1 removes the need for it |
| "not enough resources" at 1706 | knock-on of the unresolved recruits | gone |
| "not your building" at 3146, "not your truck" at 1226 and 1322, "missing requirements" at 3242, 3723, 3844 and 4325 | K1: the slot names another thing here | gone with the pairing, remain in the gate until K1 |
| "group_behavior_enable" no equivalent | real: X2 | stays `NoEquivalent` |
| "cannot attack an own unit" at 3394 and 4282 (new) | the target's owner differs here: X1 | still refused |

The Mapper now counts order fields aa-play cannot carry (`ReplayReport::dropped`): 11 recruits carry a
ParameterSet, cause K2.

**Already exact over the whole film, by this comparison:**

* Production lane: **0 differences in 1617 sampled ticks** (queue length, deadline, paused).
* Creation: in the forced-rng run, all 77 native creations happen in the same tick here, except the
  unpacks (M2), the later members of two regiments (from 4017 and 4402; a regeneration phase from a
  drawn bucket, inferred) and `group` (X2). The native list includes the buildings an unpack makes.
* The getter groups of creation (scale, mind jitter, InitCreate registrations, economy, countdown, HQ
  number) are right wherever the window is otherwise right (ledger, section 5).
* Economy lane: no rate or consumption of its own differs. Its differences are the tick of an unpack
  (M2) and of a city capture (F3).
* Combat: **the film has no damage and no death at all** (no health change at any sampled tick except
  the unpack swap, dead flag never set), and no native getter comes from combat or rout code. So they
  are not what stands between us and 266, except where they draw here: any draw from `combat.rs` or
  `morale.rs` would be an extra, and our getters carry no site to tell (open question 5). Morale itself
  is not in the sampler's trace, so that lane is unmeasured.

## 3. The causes, ordered

Dependencies decide the order. A lane can take its causes top-down; anything marked `needs` waits for
that cause. Numbers are the free run unless noted. Sizes: **S** at most one session, **M** 2 to 5,
**L** more than 5.

### Core lane

Files: `ids.rs`, `deploy.rs`, `creation.rs`, `production*.rs`, `setup.rs`, `map.rs`, `scheduler.rs`, `books.rs`, plus `world.rs` by agreement.

**State after the 2026-11-02 round** (the numbers are that tree's, with the other lanes' work in it):

| cause | state | measured |
|---|---|---|
| K1 | closed | The refusals "not your truck" at 1226 and 1322, "not your building" at 3146 and "missing requirements" at 3242 went with it, without the pairing. The slots that still differ start at 0x3465 and follow one late unpack (a movement difference) |
| K2 | carried, and **not the cause it was named** | Every ParameterSet of a v14 film names the definition's own layout and elements: 181 orders in the five QA films, 3697 in the corpus. The only 26 others are films of a mod read with v14's registries. So the 3992 row needs another cause: read it again once the recruits are on time (below) |
| K3 | closed for what the maps hold | Open question 1 is answered: see the table in K3. Getter group "disturbable delay": first tick 583 as native; the rest of the group waits on where the elements are (below) |
| K5 | closed | Static things have no height row any more. `height_at` equals the port on 60,000 points of each of 4 maps, the drawn corners are within 0.00001 m of it on the official maps. Moving things: Z lags one step (0x4EEB40 queries at the old XY), now in `move_on_ground` |
| K4 | written, its timing waits for M4 | In the forced-rng run the first node reached is at 1230 and the first fidget at 1278, both native's. Counts there against native: creation 34 of 48 (the rest of the regiments come later), node reached 43 of 91, fidget 21 of 63 |

**State after the second 2026-11-02 round.** Core's first differences are gone from Berlin: in its forced-rng run the first windows that differ are all regiment and low-level movement (M5, M7). What this round closed, by what the other films showed:

| film | at the start of the round | now | what moved it |
|---|---|---|---|
| Berlin2v2_0 | 85 of 266, first 1462 | 93 of 266, first 1598 | no core cause left before M5; the number moves with the movement lane |
| Comrades2v2_0 | 86 of 951, first 1479 | 85 of 951, first 1462 | |
| Farmhouses4FFA_0 | 821 of 1818, first 1105 | 85 of 1818, first 1462 | an unpack's assembly (below) |
| Berlin2v2_1 | 169 of 1187, first 2108 | not re-measured at the end | |
| Kursk2v2_0 | 0 of 1317, first 17 | 9 of 1317, first 170 | the map's saved Minds and the rampage (below) |

* **What an unpacking thing assembles.** State bit 26 keeps out everything but Construction, Body, Properties, Wall, Structure, Detection, Element, Economy, MobileDeployment and City (`aa_actors` `assemble_initial`); the rest comes with the deferred end, 0x576319 -> 0x4D21A0(1), in slot order. The code named only the Supply. A depot has a Lair too: 563FD0 at the end, not at creation, so two draws at the end (Kursk 1352, Farmhouses 1209). `creation::assembled_while_unpacking`, `creation::unpack_end`.
* **The map's saved Minds.** Every Campaign map, the trailer maps and the herd maps (Kursk, Moscow) save placed organizations whose members are records inside them, each with its Mind: 31 organizations and 151 Minds on allies01, 7 and 17 on (2v2)Kursk. `minds::install_saved` runs the Load over the constructed Mind (element `apply_with_commands`, regiment `regiment_load::apply_stores`, stack, stores, the current command by its definition's IDS), `element::restore` also takes the Move state's fields into the payload, the Mover and the Character (`actor+0xE4` from the record's current component). Nothing is scheduled there: the queue Load names the Updatables. **A saved state this host does not run** (the combat lane's kill, engage and the like) falls back to Sleep with a note, a labelled rendition; aircraft Minds keep a fresh Mind.
* **The rampage.** `KKC_CAIStateRampage` at Mind +0x54 is hosted: `aa_mind::cai_thinks::rampage` over the regiment (`movement/regiment/rampage.rs`), with a port of the wander point 0x525100 and the placement 0x4F46E0 through `gworld_placement::nearest_placement`. The Mind's arrival and failure arms for it: 0x52241E (leave the Move, think in 1e-4) and 0x5274B9 (leave, think in 1 s). A hunting config (`rampage_aggressive`) is an explicit error: it needs the kill state, and no ambient organization sets it. 0x525100 has no oracle of its own: it is checked by the Kursk trace only. A freshly created herd (a random map's) still starts in Sleep; 0x51E3A0's +0x310 branch is not taken.
* **Data cache.** `AA_PLAY_DATA_CACHE` holds a whole `GameData`: delete it after `data.rs` changes. A stale one gave a zero rampage config, which looks like a wander every tick.

Open: see `open_items` of the lane's report; the Kursk row above is where the herds' own movement differences begin.

**State after the third 2026-11-02 round.** The human films of the placed maps now read on the original's stream, and the herds walk. Native traces of the three films the corpus census named were taken this round (`fight-play.ps1`, 3000 ticks; F38 to its end). Numbers are that tree's, with the other lanes' work in it:

| film | at the start of the round | now | what moved it |
|---|---|---|---|
| F38, (2v2)Market_Garden, 259 ticks, a human film | 0 of 15, first 17 | 15 of 15, to its end | the cows |
| F18, (2v2)Berlin with one player, a human film, 13859 ticks | 8 of 271, first 153 | 94 of 271, first 3380; its native trace ends at 3000 | the closed sides |
| F43, (2v2)Market_Garden, 7107 ticks, a human film | 0 of 176, first 17 (trace of 3000 ticks) | 110 of 176, first 1887 | the cows; then a low path, the movement lane's |
| Kursk2v2_0 | 9 of 1317, first 170 | 110 of 1317, first 1887 | the slot table, the overlap, the Sleep rounds |
| Berlin2v2_0 | 93 of 266, first 1598 | 103 of 266, first 1768 | the movement lane's; no core cause first |

The corpus slot census on the Load films of v14, to 40000 ticks: every slot the human and QA films of the placed maps name is live on Berlin (F05, F18), Market Garden (F38) and Mountain Pass (F33, 176 of 732, first 3009); Farmhouses still has its shift, 2257 of 13509 orders named a slot that is empty here, after the fighting begins early. The films of saved games (F06, F14, F15, F36) fail in their first ticks, a saved world's matter.

What this round closed:

* **The sides no player took.** WorldParameters 0x50E8B0 resolves every kingdom of the creator, with a player or not, then destroys the closed ones: F18, (2v2)Berlin with one player, was six draws short at its first getter. `creation::ClosedSide`, `World::new_skirmish_closed`, `ReplayPlan::closed`; a random map's picks stay on the generator's stream, so `playback` clears them there.
* **The cows of (2v2)Market_Garden.** A record on its own that saved a Mind is an actor, whatever its definition says, and its Mind constructs at the Load (0x4D8290 -> 0x4D833E -> 0x530E50, eight draws). They were scenery, so their queue 3 and 4 entries had no owner: the first checkpoint failed at 17 on both Market Garden films. `setup.rs`, `creation::restored_thing`.
* **The slot table of a formation that is not organized.** `0x57BEF0` over `aa_movement::formation::compute_slot_position`: an attached member's offset is where it stands from the leader. The code drew for the empty slots and kept the layout's places. `creation::organization_scatter`.
* **The element's Sleep overlap.** Sleep's Think with Actor +0x110: `0x4D1820`, `0x4D04D0`, the first clear place at the other's blocking radius plus its own soft radius, and the Move in mode 2. `space::overlap_point`. Proof: the horse `0x3bb6` at 237.
* **The regiment's Sleep rounds.** `0x522F60` ends by setting +0x10, and the next Think's `0x523000` waits while a member moves (`0x5200B0`), clears when each stands within 0.5 m of its place (`0x520190`) or after two rounds, else runs it again. The host never set the byte. Proof: Kursk's first difference from 289 to 1887, and Berlin's did not fall.
* **The regeneration gate.** `0x579110` reads the reported Mind state's `resupply` (Game/actor_states.tgi) and the engagers' list; what a member shoots at is no input. `0x57B5A0`'s loop passes over a damaged member that is detached. Read from the code; the order_gone rows have not been re-read against it.
* **Transports.** A summon's (0x4CB376) and a recruit at a location's (0x582426, `special_ops::recruit_by_air`) take their id and draws (`creation::transport`), and a dropped thing's fall draws one getter (`creation::cargo_fall`). A rendition still: no actor flies, and a regiment lands whole at its arrival where natively a member drops at each of the transport's thinks, 9 or 10 ticks apart. The engineers' bunkers 0x5634D6 were never missing: `construct.rs`, reached only when the task is.

What the first differences are now: Berlin, F43 and Kursk read as movement and, on Kursk and the two developer maps, the Kill state of a `move` on a hostile thing (attack efficiency desired from 1840 on Kursk, member speed from 1448 on Comrades, the parity-fights lane's). On Comrades ours fires its first shot at 2282, the original at 6625, which is what makes most of its slot shift there: 496 projectile ids against 79.

**State after the fourth 2026-11-02 round.** Full native traces of F18 (13859 ticks) and F33 (12445) were taken this round (`fight-play.ps1`; a film for it has to lie at the top of `Films/`: the dialog does not open a subfolder). The all-films slot census ran first, with the tree before these fixes, to 20000 ticks a film: every slot the human and QA films of the placed maps name is live on Berlin, Mountain Pass, Market Garden; the empty slots are Farmhouses (675 of 6264), Kursk (549 of 1854), the saved games and the random maps past their first difference. Numbers are that tree's, with the other lanes' work in it:

| film | at the start of the round | now | what moved it |
|---|---|---|---|
| Berlin2v2_0 | 103 of 266, first 1768 | 140 of 266, first 2397 | the movement lane's |
| Comrades2v2_0 | 85 of 951, first 1462 | 128 of 951, first 2193 | the Move at a thing |
| Kursk2v2_0 | 110 of 1317, first 1887 | 126 of 1317, first 2159 | with the movement lane's |
| Farmhouses4FFA_0 | 86 of 1818, first 1479 | 97 of 1818, first 1666 | the Move at a thing |
| Berlin2v2_1 | 169 of 1187, first 2108 (the second round's) | 138 of 1187, first 2363 | |
| F33, (1v1)MountianPass | 176 of 732, first 3009 | 177 of 731, first 3026 (full trace) | the retreat |
| F18, (2v2)Berlin with one player | 94 of 271, first 3380 | 94 of 271, first 3380 (full trace) | |
| F43, (2v2)Market_Garden | 110 of 176, first 1887 | 146 of 176, first 2499 (its 3000-tick trace) | with the movement lane's |
| F38, (2v2)Market_Garden | 15 of 15 | 15 of 15 | |

What this round closed:

* **The Move of a regiment at a thing ignored the wrong actor.** `0x51EA20` hands Solve the target as the descriptor's ignored actor. State +0x10 already is the key (id + 1), and the host added one again, so the building a regiment was sent at blocked its own goal and `0x53D340` walked the goal back along the line: on Comrades the leader went for (130.93, 137.19), the edge, by way of (67, 205), where the original goes (65, 207), (122, 133), then the centre (136, 131). `regiment.rs` `solve`. This was the "member speed from 1448" of the last round: Comrades' first difference goes from 1462 to 2193.
* **A retreat to where the regiment stands.** The Move ends in its first Think; the Sleep that follows in the same update took the RETREAT formation, because `orders::retreating` held until the next `orders::tick`, and walked every member to the column's places (F33's 3011). `0x520250` reports for the Move state only: `required_formation` asks `Some(MOVE)`, `retreating` compares the order.
* **Transports.** `crate::transport`. A recruit at a location (`0x582190`) and a summon (`0x4CB110`) make a real plane now: a thing with its GTID, its Mind in `aai_queued`, released at once by the Transport's +0x4C (`0x519F70`, `0x518550`), `aai_transport` hosted in `combat::air` (`aa_mind::uai_thinks::aai_transport`, `0x517B60`, `0x5179C0`, `0x5179E0`, `0x516D10`), its flight on `aa_movement::aircraft` (`move_tick`, `fly_to`). The regiment appears at the first think, `0x554BF0` from the plane (Berlin2v2_1's anchor (147.27492, 91.36623), native (147.27493, 91.36622)). Each think (0.6 s) drops one member under the plane at `[Size] height` below it (`0x58BCB0`), aimed along the plane's way by `0x577380` (`fall_drag`, ActorData +0x1A4), falling on `aa_movement::falling` (`Owner::Fall`: the Mind's stops do not touch it). The landing is `0x4D64A0` -> `0x563D50`: the Character's kind 5, one getter, the Mind thinks at its end; the first member lands at (149.36240, 92.30676), native (149.36134, 92.30629). The plane flies its way out and leaves (`0x58B130`). `special_ops::recruit_by_air`, `Pending::Recruit`, `Pending::Drop` and `creation::transport` are gone. **Renditions, labelled in the module:** the structure's own aircraft (`0x56DC50`) is not taken first, `0x554850`'s search around a taken landing point is not ported, a member's place after its landing goes through `movement::member_added`.

Named from the traces, not fixed:

* **F18's 3344, F43's, Berlin's:** member-level avoidance. On F18 a member takes a new node at `0x537005` -> `0x52FD80` (a late Reattach of a moving regiment) one window before ours. Movement lane.
* **Comrades' 2192:** `character 563BA0 from 52fc45`, an element's stop as its regiment reaches the building's zone: ours holds the Kill's walk a few ticks longer; ours fired first at 2282 because it stood at the wrong goal, and with the goal right that moved past this window. The fights lane's.
* **Farmhouses' `gbr_hq_mechanized` at (289, 34):** not the snap. 23 of the 24 native unpacks on that film round the Move's destination as `deploy::placement` does (`0x46EE00`). At 2116 the original's truck stops 0.19 m short of (288.927, 34.530), at about the anchor's (288.933, 34.340), against something in its way, and rounds to 34; ours arrives 58 ticks later on a clear road and rounds to 35. So: movement, a knock-on of the earlier first difference.
* **Efficiency desired +88:** reads the state one update late at a formation change (first-on, no getter behind it). Not fixed.

#### K1. An unpack takes a new id and swaps it (S, do first)

* **Shows as:** 60 things sit on another slot from tick 1002. First: native `ger_truck_artillery` 0x3462, ours 0x3460. The native slots never named are 0x3460, 0x3461, 0x3464, 0x346B, 0x346F, 0x3479: one per unpack.
* **Native:** 0x575C80 creates with record +0x24 = 0, so 0x4EE460 takes the next-fit id (0x4EFDB0). Then 0x4D6130 runs 0x4D5EA0, whose 0x4EFB90 swaps the two things' GTIDs, then 0x4F6760 destroys the old thing, which now holds the new id. Result: the building keeps the truck's id, the cursor has moved one, and the spare slot is free with its generation raised.
* **Port:** `aa_actors::registry::IdentityRegistry::swap_identities` (0x4EFB90), replayed by `crates/aa-actors/tests/native_registry.rs` on `research/phase4-actors/trace_registry.json`; outside its tests only a mechanics check calls it.
* **aa-play:** `ids.rs:294 replace_actor` removes the id and re-registers the same one, so the cursor stays. Called from `deploy.rs:156`.
* **Takes:** allocate, swap, remove in `replace_actor`. `Thing::Trailer` goes with the old thing as today.
* **Proof:** the replay needs no pairing and the gate's refusals go. Assert in `berlin_replay` that no order is unresolved except X2.

#### K2. A recruit order chooses its layout and elements (M)

**Done, without moving a number:** `Command::RecruitWith` carries the set to the queue item, the quotes and the spawn read it. A set on another layout is refused: `Def` holds only the definition's own expansion, and no film orders one.

* **Shows as:** 11 of 13 recruit orders carry `ParameterSet { definition, objects }`, for example `[713, 713, 713, 717, 714, 716]`. Ours builds the definition's own elements. First visible at 3992: three `ger_infantry_mortar` missing, regiment 9 members against 8 (order object 856, elements `[562, 562, 566, 566, 567, 565]`).
* **Native:** 0x61EBA0 flag 5, command +0x34; 0x583A70 takes it, 0x5836E0, 0x589E00 stores it on the item, and 0x582E80 / 0x4FBF40 spawn by it. The layout index is OrganizationLayout `[0xB3AB00]`, the objects are ThingData slots.
* **Port:** `aa_production::RecruitRequest` has `organization_layout` but no objects (`crates/aa-production/src/lib.rs:134`). Spawn is `aa_actors::organization_spawn`, `organization_recruitment`.
* **aa-play:** `Command::Recruit` (`world.rs:134`) has only `def`; `production.rs:467 recruit` passes layout 0; `production.rs:249 spawn_member` reads `Def::slots`.
* **Takes:** a new variant or field on `Command::Recruit` (shared file, aa-bevy and the AI build it, so additive), carried on the queue item, read by `spawn_member`. `mapping.rs` passes it and drops the `dropped` counter. Check the cost quote against the parameters.

#### K3. The map's saved deferred queues, and doodad timers (M)

**Done.** Queues 3 and 4 are restored (`scheduler::restore_deferred`), doodad owners resolve through their two-slot Load 0x4DCD10, and 0x5902C0 and 0x58FA30 have a port with an oracle (`aa_play::doodad`, research/phase8-doodad, 240 + 480 cases). What the maps save on the deferred queues:

| map | queue 3 | queue 4 | owners |
|---|---|---|---|
| (2v2)Berlin | 796 in 16 of 97 slots | 0 | two events for each of 398 Disturbables (600 temperate_high_grass, 196 temperate_high_dry_grass) |
| (2v2)Kursk | 277 in 18 of 97 slots | 17 | 256 Disturbables, 1 Breakable, 37 of the 7 placed things whose components do not walk yet |
| (2v2)Comrades | 0 | 0 | none: its doodads are re-created at load (below) |
| (4FFA)Farmhouses | 16 in 1 of 80 slots | 0 | Detection (slot 22) of 7 village things |

The scan is over targetable things (flag 0x4000 is Thing `targetable`). Organizations are in the tile lists too (Thing flag bit 26, 0x4EE4C0), and their 4CEDE0 is `movement::org_moving`. **That test holds only its first half, on evidence and without an explanation:** by the code a banner off its goal (`!0x578E10`) is moving, yet regiment 0x3457 walks within 0.5 m of grass 6788 from 585 to 601 and native draws nothing between 583 and 1525. The trace puts a moving element (`act=3`) in reach at every one of the first 24 draws. Without that half the getter group stays at 8 of 165 in the free and forced-rng runs, 124 in forced-all: the elements are in the wrong place, a movement matter. **Not bound:** Break 0x58FF70 (nothing starts state 2), the action lists run only as `Notice::DoodadDisturbed`.

The world-start branch of the developer maps is the same subject. A record saved as an Object whose definition is now another class is re-created at the end of the map load (`aa_mapworld::class_conversion`, `creation::converted`): facing, scale, then each component's 0x5904A0 twice, InitCreate and InitExisting. A Breakable cancels itself each time; a Disturbable keeps both events.

Three more things decide those branches, and with them the first battle getter state now equals the film's on Comrades (0x34B31E6A), Farmhouses (0xA8A8A8DC) and both Berlin films:

* **Old ids.** A placed thing may be named by an id Data/translate.tgi has renamed (0x427BD0): `data.find` reads the translation first. 7 things on Comrades and 4 on Farmhouses were missing.
* **Facing.** The constructor's 0x4EF2A0 draws only for `random_orientation` without `axis_locked`, and a saved variant is kept (0x4EA5E0).
* **PostLoad.** The pass 0x4FD3B3 between the queue Load and the conversions runs each actor's component +30 in slot order (`aa_actors::post_load`). 0x581260, the +30 of Properties and Supply, adds the component to queue 1 when no queue holds it, one draw each. The count from the maps is the traces': 0 on Berlin and Kursk, 4 on Comrades, 16 on Farmhouses (`tests/map_queues.rs`).

| film | checkpoints before | now | first difference now |
|---|---|---|---|
| Comrades2v2_0 | 0 of 951 | 34 | tick 595: ours has 8 getters in the window, native 2. Native's next thing is an unpack at 598 and a mover order at 594: a timing of the movement lane |
| Farmhouses4FFA_0 | 0 of 1818 | 38 | tick 663, 2 entries more on ours |
| Berlin2v2_1 | 26 of 1187 | 47 | tick 816 |
| Kursk2v2_0 | 0 of 1318 | 0 | tick 17: open question 7 |

* **Shows as:** the first missing getter: tick 583, 165 in the film range (group "disturbable delay"). Every one shifts the whole chain by one, so in the free run this is what turns timing differences into value differences.
* **Native:** the queue Load replaces all five queues with the map's saved lists (0x50BF10 / 0x50BFF0); `UPDATABLES` holds them in the order 3, 4, 0, 1, 2. **Berlin saves 796 events on queue 3, in 97 slots, and none on queue 4** (measured by this test), so on this reading of the code (open question 1) the 6190 draws of InitExisting 5904A0 are real but their events do not survive. Queue 3 delivers a Disturbable's 0x5902C0 (Updatable +0x18): 0x5901F0 scans a circle of `affect_radius` for a moving thing (0x4CEDE0); only when disturbed does it check its time, run the FX (vf +0xC), and draw (getter 0x590323, one draw) `now + min + u * (max - min)`. Then it re-arms with `update_time` through 0x50ACC0. Breakable 0x58FA30 likewise.
* **Port:** `aa-runtime/src/live_doodad_tick.rs` is a binding of 0x5902C0 with the disturbed branch as an explicit error, and 0x58FA30 unbound; `aa_spatial::circle_actors`, `moving_state`. **No oracle:** a port and fixture for 0x5902C0 and 0x58FA30 are part of this cause.
* **aa-play:** `map.rs:673 saved_periodic` reads queues 3 and 4 and throws them away; `scheduler.rs:239 restore_saved` takes 3. `creation.rs:151 restored` draws for 5904A0 / 58F960 and registers nothing, which is right as long as the Load replaces them. `deliver` (`scheduler.rs:354`) has no doodad leaf.
* **Takes:** restore queue 3 (deferred kind; `aa_world::queues::restoration::restore_saved` keeps the saved ring length and zeroes the last-end time, as 0x50BF10 does); `Owner::Placed` delivery to a doodad leaf that needs data on `Def` (`update_time`, `affect_radius`, min, max) and a spatial query over moving actors.
* **Watch:** the 796 owners have not been listed: not all need be doodads, and each kind needs a leaf or an explicit refusal. Doodad FX need no simulation effect, but check the Action list (0x42A590) for anything that does.
* **needs** M4 for exact `4CEDE0` moving and positions before the getter count can match; it can land first and be checked in the windows before 583.

#### K4. The Character component (M to L)

**Written:** `crate::character`, `Def::character`, the creation draw, the queue-3 delivery; movement calls `character::mover_started` and `character::mover_stopped`. What stands in, labelled in the module: Performer +0x34 (`Actor::combat.target` and the first weapon's idle), 0x4D03D0 (no stun flag yet), 0x563D50 (no recruit in a film sets record +0x51).

* **Shows as:** 256 getters in the film range, first at 1202 (`rus_infantry_co` creation): 48 at creation (0x4D6FE0), 91 at node reached (0x530DA9), 100 in the Character's deferred update, the rest from Mind states.
* **Native:** 0x563BA0 (default stance) draws idle repetitions (0x49B4D0) and arms 0x50ACC0 with `repetitions * animation length`; 0x563CA0 draws a fidget (0x45EB70) and arms with its length; 0x563E10 is the deferred update (+0x18) with its data cases. Reached through 0x4CFC80 when the Mover stops, and from the Minds.
* **Port:** `aa_animation::runtime` (`default_stance`, `actor_default_stance`, `idle_repetitions`, `start_move`), fixture `native-runtime.json` 720 cases, bindings 65; `aa_actors::ground_states::CharacterState`; 0x563CA0 and 0x563E10 need checking against it. `crates/aa-bevy/src/battle_view.rs:484` has a view-only copy, which must stay view-only.
* **aa-play:** nothing in the simulation. `creation.rs` omits the InitCreate draw.
* **Takes:** new `character.rs`: per-actor `CharacterState`, the animation sets and lengths from data **in the simulation** (a length decides a re-arm tick, so it cannot live in the renderer; aa-bevy can read the state), deferred delivery in `deliver`, the draw in `creation.rs`, and a call from movement when a Mover stops (see M4). The interface to agree first is `character::mover_stopped(world, actor)`.
* **needs** M4 for the node-reached and stop calls to fall on the right tick; the creation and timer parts can be checked earlier.

#### K5. Heights (S)

**Done:** `GameMap::native_terrain`, HEIGHT_MAP and MOUNTAIN_HEIGHT_MAP in exact words, `NativeTerrain::ground` the query 0x4F0E50 with flags 0x122, proven equal to the port by `tests/heights_native.rs`. A generated map has the same query over its own layers, and its things keep the height they were made at. On the developer maps the drawn corners differ by up to 3 m on tiles naming a terrain variation that no longer exists: the query takes the previous tile's (557D4B), the drawn terrain does not.

* **Shows as:** all film long on all things: 1.9543319 against 1.95433, 2.2136512 against 2.21365. `map.rs:382` reads heights from the renderer's JSON, whose text is rounded, and `map.rs:216 height_at` interpolates in f32.
* **Native / port:** 0x4F0E50, `aa_spatial` (crates/aa-spatial/src/lib.rs:162), `research/phase5-heightfield` fixtures, `aa_mapworld::terrain`. ROADMAP lists this gap.
* **Takes:** heights from the RMP terrain through the port. No getter depends on it directly as far as the film shows (height does not enter a Mover step), so it can go any time. The renderer shares `heights`: check it still lines up.

**The ground under a created thing (2026-11-02).** The base map was never written in a running battle; the original writes it at every unpack. `crate::ground`: GThing PostCreate 0x4EBA40 -> 0x4EB500, the brush 0x551140 on HEIGHT_MAP (one code path with the generator through `mapgen::height::HeightGrid`), ClearDoodads 0x4F5560. Settings +0x1C is an unwritten stack word: the thing's Z before the session starts, 0 on an unpack (sampled on four films). `tests/ground_native.rs` holds the whole base map's hash and the cleared GTIDs equal to the original after every brush of a Kursk film. The drawn terrain follows (`scene.rs follow_ground`); no tile changes its variation and no static thing changes its Z. Notes in `research/phase8-ground/README.txt`.

### Movement lane

Files: `movement.rs`, `path.rs`, `formation.rs`, `minds.rs`, `aa-movement`, `aa-navigation`. Today `movement.rs` is a plain f32 reimplementation that follows the ports but calls none of them except the solver and `formation`. This lane owns **6200 of the 6986 getters** (the Mover registration and ActionGo pairs).

The order is M1, then M2 and M3, then M4, then the rest. In forced-rng every window up to tick 551 is equal (76 getters), so the structure holds that far and geometry is what breaks it first.

**Status 2026-11-02 (parity-movement lane), first round; the next round's is below the table.** M1, M2, M3, M4 and M6 are in; M5 and M7 are not. The element Mind runs on the exact ports (`crates/aa-play/src/movement/element.rs`, `HostedKind::Element`, queue 4). In the free run the trucks now match the original bit for bit, position and direction, up to the first collision query that goes wrong. What each step gave, read from this census:

| step | gate | first difference | what it showed |
|---|---|---:|---|
| before | 32 of 266 | 561 | |
| M1 | 32 | 561 | truck direction and position bit-exact through the first turn (484 to 521) |
| M4, M2, M3 | 58 | 1003 | every getter window to 991 equal; the first unpack on tick 587, building and truck exact |
| a lone recruit walks out through the Deploy state, thinking in its creation update; Unpack Enter's 0.1 m test (`0x52A240`) | 70 | 1207 | the expectation of section 4, every checkpoint to 1190 |

Read against the original while doing this, and not as M2 first described it: the regiment's unpack does run its own Move first. The truck gets its path from the regiment (`0x5379F7` at 483 is a continuation, so the path exists before the Think), and the unpack action goes to the unit when that Move is over. The order's point is unsnapped; the building snaps when it is made. Mind +0x12C is `0x530AE0` called directly inside the Mover's delivery, not an event for the Think. A Move that ends in a regiment sets the member to the organization's facing (`0x52DA60` -> `0x4EEBE0`). A changed Mover maximum reaches the state and the Mover's request at once through Mind +0x118 (`0x52D620`), which is what makes M6's 3.125 appear within the tick.

**Still renditions in the hosted Mind, each marked in the code:** the collision queries `0x553BA0` / `0x553CE0` are `movement::collision` (terrain flags, placed shapes of structures, circles of movers; no doodad shapes); a blocked result is `Host::collision_rendition`, not `0x536840`, `0x536AD0`, `0x534A40`, `0x53C170`; `0x531520` is a direct node; the region pre-pass `0x53DC00` is absent, so collection +18 stays -1; Element +1C reads as set (F2); Sleep only re-arms (no overlap escape, no follow-the-leader posture, `0x52EC20` still not ported); the regiment layer (`order_think`, `dispatch`, `reform`) is still the earlier code, M7.

**A finding for K3:** `movement::org_moving` keeps only the first half of `0x4CEDE0`'s organization branch (the regiment's Move with a moving leader). With the second half, the banner away from its goal, a Disturbable near the first HQ draws at 599 where the original does not (35 of 266, first difference 612); the banner does still walk then, so the code as read says it should. Unexplained: either the organization is not in that scan or something else differs.

**Status of the next round, same day (parity-movement lane).** M7's regiment Mind, M5's queries and the route solver are on the exact ports. `crates/aa-play/src/movement/`: `regiment.rs` (`company_move`, `cai_sleep`, unpack), `space.rs` and `uai.rs` (`SpatialIndex`, `first_hit`, `sweep`, `0x534A40`, `0x536AD0`, `0x531520`, `0x53C170`), `route.rs` (`0x53E1F0` with `0x53DC00`, `0x53D8D0`, `0x53D500`, the map's saved regions). Measured on the census (fast profile, gate-equivalent `free` run):

| step | gate | first difference | what it showed |
|---|---|---:|---|
| regiment Mind (`company_move`, Sleep, Reattach, element commands) | 85 of 266 | 1462 | every window to 1456 equal |
| route solver on the ports, with the region pre-pass and the kind-2 hops; `0x52CED0` counts from the regiment path's tail | 86 | 1479 | the whole regiment bit-exact at 1448, 1456, 1464 but one member's speed |
| `0x57D8A0` -> `0x57A640`: a member joining reaches the others through Mind +0x118, so the Mover's request is factor * speed | 86 | 1479 | 1472 bit-exact for all members |
| Mind +0x44 of the leader's Mind (`0x535E00`) in `0x534A40`; a refused Move keeps its fields (`0x52DF60`) | 93 | 1598 | 1480, 1488 bit-exact for what walks; ours has 6945 getters of 6986 |
| Mind +0xA4 `0x52D780` (Reattach current) gates the catch-up speed of `0x5300A0` | 93 | 1598 | the recruit that left its place at 1590 is bit-exact at 1592, 1600 |
| `0x537750` (Mind +0x124) ends with the next Think in 3 s with a Reattach on the stack, else in 1 s; ours had no schedule, so a recruit's refused Move put its Sleep on the wrong phase | 96 | 1649 | the second recruit's Sleep at 1502 + 32k as native, plan at 1598 |
| `0x4FBF40`'s scan is `0x558040` mode 0: every actor in the tile rectangle of the control range, with no circle test (`production::member_structure`, `movement::actors_in_tile_rectangle`, `aa_spatial::rectangle_actors`) | 97 | 1666 | a recruit of a regiment that has walked away still comes out of the building |
| `[0xB3D300]` is the SVar `MoverDetachedSecondsUntilReattach`, 2; ours read another name and took 5. Organization +0x148[4] and `0x54EF40` are bound in the Move Think `0x537B90` | 103 | 1768 | members detached while the regiment moves reattach at 1651 and 1659 as native |

What these rested on: the sampler trace of the original, side by side (`AA_PARITY_WATCH`), and the pseudocode. Nothing here has an oracle of its own beyond the ports' fixtures.

**Status of the fifth round, 2026-11-03 (parity-movement lane).** Measured on the census's free run, fast profile, the other lanes' work in the tree:

| step | gate | first difference | what it showed |
|---|---|---:|---|
| the review's F2, F3, F4, F5, F7, F9, F10 (below); `0x556070`'s radius is the *organization* definition's +0x2FC, so the rings of a Reattach are 0.9 m apart, not 0 | 106 of 266 | 1819 | 0x3470 and 0x3471 head for the organization's position + 0.9 x facing as native (`0x4F3640` with no source: the leader stands on the point itself) |
| `0x533C30` (F16), `0x5317A0` with its box branch, thing virtual 0 is the soft radius | 106 | 1819 | 0x3472 bit exact to 1775, replanning with the target as native; `member speed +20` at 1536 is gone with it |
| `0x570560` tests Element +0x1C: a detached member takes its own tile's speed cap, not its organization's | 106 | 1819 | 0x3472 bit exact to 1804 |
| `0x522F60`, `0x5200B0`, `0x520190` walk the attached members only (`0x4364C0` with its third word 1) | 140 | 2397 | a member still reattaching got a slot's Move from the Sleep at 1805; ours has 6977 getters of 6986 now, the first tick difference at getter 1668 (native 2384, ours 2382) |
| FindOpenPositionLineThenRadius `0x554BF0` exact (`space::open_line_then_radius`, `0x553FA0`, `0x554B30`), for a member walking out and a recruit's organization (`0x582E80`) | 106 before the row above | 1819 | **`ba445000bb`, a random-map film, 32 of 32 checkpoints to the end of its trace, no movement row left** (before: 28 of 32, first 493); the organization stood one step short of a bunker because the old test was a box |

The other QA films on the same build (free run): Kursk2v2_0 126 of 1317, first 2159 (was 110, first 1887); Comrades2v2_0 128 of 951, first 2193 (was 85, first 1462); Farmhouses4FFA_0 97 of 1818, first 1666 (was 85, first 1462); Berlin2v2_1 138 of 1187, first 2363.

**What the review's defects became.** F2: `0x52F260` compares with 9.0, `0x52E130` is ported (`node_or_failed_52e130`). F3: `0x52DA60` schedules 1 s only with Sleep current, and `0x5324A0`'s Move arm has `0x52FD00` (a live target with a Mover: think in 1 s, follow at its speed). F4: `0x52D9C0` is its own function. F5: `set_speed` has the 0.1 floor and the clamp of `0x576D20`; the group's floor is not bound because no element keeps Actor +0x164 (X2). F7: the unpack error only for an actor with no organization; the text is still ours, not `tai_error_unpack|s`. F9: `E::DeactivateMover` runs `0x4CFC80`. F10: `0x5273D0` has no member loop, the Engage (priority 4) stays and thinks at once. F15, F16: above. F8: `sleep_523000` is right against `cai_sleep_fields` once it walks the attached members.

**A map's objects are in the tile lists now** (`Space::add_scenery`): the records of `Books::scenery` whose definition blocks, with Thing +8 from `native_final_flags`, not of the actor category. Keys above `0x40008000` name them; `cont::Actor` and `Uai` read them from the lists. No gate number moved by it on Berlin.

**A lone actor's Move and Follow are the Move command on its Mind** (`element::deliver_lone_move`: `0x4CA760`, `0x52C320`, `0x538340`), so its route is `0x5336B0` with the region hops. A lone actor's kill order is still `order_think`, labelled.

Next causes, read 2026-11-03: Berlin's first position difference is now 0x346A at 2336, `member speed +20` (native 2.1181774, ours 2.6477218), then the first tick difference at 2382 (native 2384). On `1104a3b0f9` the truck is bit exact to 39, where the original's Think takes `0x532B50`'s turn recovery; ours thinks this truck at 21 and then 41 where the original's window has 21, 22, 39, 41 (the stream stays equal to 53, so one of those is another truck's): read the Mind's schedule of a lone truck first.

**Renditions left, each marked at its place:** walls (`0x4F2220`, `0x53CA30`, `0x53CB70`'s wall cost, Mind +0x88): no wall thing is hosted; a map's scenery objects that are no actors and the trailers are not in the tile lists (`Space`); `0x554AD0` of a scatter formation keeps the point; a regiment's formation counts as oriented; the kill order of a regiment is still `order_think` (X1); `0x536840` stands aside for the combat states.

**Not done:** `0x52EC20` and `0x52EBF0` in `0x530440` (M2's placement for the other members). A thing's 564300 draw still lands at the first queue-1 delivery; `observe_key` right after the Mover's position write has not been retried. Trailers are not in the tile lists: they have a Mover natively and aa-play keeps no trailer thing. Regiment state virtual +4 is `Ok(true)`: `0x51D730`, `0x51D650`, `0x51D610`, `0x51DF80` are read (priorities: Sleep and Rampage 0, Regroup 3, Engage 4, Rout 6, Exhausted 7, the others 2; Organization +0x8C is FormationData `aggressive`) and not bound: a Move to a regiment engaged in an aggressive formation should be refused. Multithreaded movers: not started. Open in `tests/movement.rs` since this round: `building_footprints_block_cells` (a lone truck on a test map, whose tile grid goes to `crate::path`, enters the building's box) and the ignored `real_random_map_orders_as_a_person_gives_them` ("still following" after a regiment reached its target on a generated map); neither is explained yet.

#### M1. The Mover ground step on the exact port (S to M): done

* **Shows as:** first difference in the lane, tick 484, the truck's first turn: native direction (0.99802, -0.06285), ours (0.99805, -0.06246). Position differs from the same tick (< 1 mm, growing).
* **Native:** 0x5774A0, turning through 0x445400 and 0x4454A0 / 0x4454E0, which are quantized and CRT-based.
* **Port:** `aa_movement::ground_step_original` with `OriginalTurning`, `mover::{tick, arrival_stop, failure_stop}`; fixtures 186 + 297 ground steps, Mover 1251 (`research/phase4-movement`, `phase8-movement/native-mover.json`).
* **aa-play:** `movement.rs:2521 ground_step` with `signed_angle` / `rotate` at `:927` / `:930` (host `atan2`, `sin_cos`, renormalize).
* **Takes:** an adapter implementing `aa_movement::GroundWorld` over `ActorMove` + `collision()`. PC53, PE pending = true. Per-tick cost: `aa_numeric::Fast` is what `interpolate_native` already uses. Keep `collision()` as the seam until M5.

#### M2. Unpack as the native states do it (M): done for the unit itself; 0x52EC20 for the other members is not ported

* **Shows as:** the first move's destination: native (160.67825, 366.33093), the order's point; ours (161, 366), snapped. That changes the path from tick 483 (the node at 525 differs, the arrival at 552 comes a tick early), then the unpack tick: native 587, one update after the Mover's arrival at 586; ours in the arrival update. The same offset moves every knock-on: the 8 creation getters per unpack, the Supply registration at the end of the unpacking (715 against 714), the trailer, the HQ's health and ZOD at the swap, and the economy (`provided`, then the money stock, 1031 sampled ticks).
* **Native:** regiment `KKC_CAIStateUnpack` 0x526E60 (action 0x51E800, ordinal 0xB, delivered to the leader, then the 1.0 s re-think); element `KKC_UAIStateUnpack` 0x5372B0, placement 0x52EC20 (0x554500 in rings, **not ported**), 0x52EBF0 (point = definition +0x538/+0x53C + L), move state 0x52DF60; then 0x575C80 and 0x4F3640, which is where the building snaps (`Thing` tile_locked, axis_locked). All on queue 4.
* **Port:** `aa_mind::cai_thinks::unpack` 240 cases, `aa_mind::uai_thinks::uai_unpack` 540 cases.
* **aa-play:** `deploy.rs:38` calls `placement` at command time and gives the mover the snapped point; `deploy.rs:39` and `movement.rs:3394` refuse spots by rules not found natively (`spot_clear`, `deploy_spot_problem`); `deploy.rs:190 tick` is a pass that starts the unpack from `arrived()` (`:139`).
* **Takes:** `Order::Deploy` carries the order's point and the snapped placement is computed when the building is made; the Think on `mind_tick` (with M4's Mind host or, as a first step, an equivalent think that keeps the queue-4 timing). Port 0x52EC20. Touches `deploy.rs` (core) and `movement.rs`: **one agent does both**.
* Facing: every Berlin unpack is axis_locked. See the note in `mapping.rs` on the zero direction.

#### M3. The organization during and after the unpack (S): done

* **Shows as:** native 0x3457 keeps walking to the building at the catch-up speed after 587 (160.69958, 160.72137, ...), ours jumps. The group "organization goal 57C680" (4 getters at 587, 888, 1066, 1289) is never produced here.
* **Native:** 0x57C680 gates on Mind +0x64 / +0x7C; the regiment's unpack state is still active, so the goal moves and 0x50AC00(1) registers (0x4D6174, 0x57D8A0, 0x57CC8B, 0x57C800).
* **Port:** `aa_actors::organization_interpolation`, `element_position::organization_moved`; already used by `movement.rs:3187`.
* **aa-play:** `deploy.rs:170` sets the organization `Order::Idle` *before* `member_replaced` (`:174`), so `org_active` (`movement.rs:3141`) is false and `set_goal` returns.
* **Takes:** keep the deploy order until the regiment's unpack state ends. Goes with M2.

#### M4. The element Mind on the exact ports (L, the centre of everything): in, with the renditions listed above

* **Shows as:** the windows. In forced-rng 437 of 1617 windows differ; the first are ours one tick before native (552 / 554, 570 / 573, ...). Every ActionGo pair is 0x5773C0 registration + ActionGo's draw: 3081 + 2173 + 508 + 100 + ... getters.
* **Native:** Mind update 0x531D30 on queue 4; move state Think 0x537B90 / 0x537AF0, continuation 0x5379C0, forward check 0x537150, 0x536F40 collision result, 0x532B50 get back to path, 0x530AE0 node reached, 0x52FD90 turn offset, ActionGo 0x5300A0, Mover order 0x5773C0, formation follow 0x52FF00.
* **Port:** `aa_mind::element_move` 384 cases (37 trait methods), `element_move_continuation` 219 + 348 (24), `node_reached` 276 (14), `element_update` compiled, **no fixture** (15), `aa_navigation::action_go` (`crates/aa-navigation/src/lib.rs:274`), `formation_follow` 498, `result_plan` 249 (16). Reference bindings: `aa-runtime/src/live_element_mind.rs`, `movement_world.rs`.
* **aa-play:** `movement.rs:1982 element_think`, `:2076 action_go`, `:2264 get_back_to_path`, `:2341 node_reached`, `:2143 formation_speed`; `minds.rs` hosts only the aircraft Mind (`HostedKind`).
* **Takes:** a `HostedKind::Element` Mind per element, Mind arena states, and the trait impls over `MovementState` (about 100 methods; many are thin). `Owner::Mind` delivery runs `element_update`. Do it behind the same `movement::tick` façade: aa-bevy reads `Motion`.
* **Check after:** that forced-rng windows are equal up to the first regiment move (1442), with M1 to M3 in. Then re-run this census.

#### M5. Low-level avoidance (M): in on the ports, see the status above

* **Shows as:** ActionGo groups from ResultPlan 0x5336B0 (124), 0x533140 (107), 0x536F40 (27), 0x536AD0 (13), 0x531520 (11), first at 403 and 1469. Members stand or move differently from 1224.
* **Native:** low-path solver 0x53C170, `ResultPlan` 0x5336B0, `ResultAvoidStatic` 0x534A40, `ResultCollision` 0x536F40, 0x536AD0, continuation 0x531520; collision scan 0x4D5600 / 0x552340, overlap list 0x4D67F0.
* **Port:** `aa_navigation::low_path` 411, `uai_collision` 636, `result_plan` 249; `aa_spatial::overlaps` 192 + 2304 + 576. Host helpers in `aa-runtime/src/spatial_world.rs`.
* **aa-play:** `movement.rs:2657 blocked` and `:2600 collision`: the module header says "Simplified": a sidestep node or a tile re-route.
* **Takes:** tile actor lists in native order, overlap lists, the solver over them. **needs** M4 (these are its callbacks).

#### M6. Formation choice: property and turn (S to M): done

* **Shows as:** order 1217, formation slot 2: native speed 2.5 to 3.125 by tick 1224 (`regiment_column`, `SPEED_MAX` +0.25), ours stays 2.5. 5079 speed fields. The data: `regiment_aggressive` `SPEED_MAX` -0.25, `regiment_defensive` `ATTACK_EFFICIENCY_DESIRED` -0.25, `regiment_column` `SPEED_MAX` +0.25 and `ATTACK_EFFICIENCY_DESIRED` -0.50, the others none.
* **Native:** command 0x21 on the Organization component: 0x57BFF0 / 0x577F30 / 0x57BF50 (Organization +0x70, remove the old property, apply the Formation's +0x24), then Mover refresh 0x576C70 / 0x576850.
* **Port:** `aa_actors::organization_formation_fields::set_formation_fields`; `aa_movement::mover::refresh_max_speed` and `refresh_turn_rate` (882 of the Mover set); `aa_movement::formation` 477. `aa_rules::native::layout_data` records the property, whose modifier load 0x4C42D0 is **not ported** there.
* **aa-play:** `player_orders.rs:97` records the index, and on a standing regiment issues a `Move` to the anchor (`:108`), an extra that should be read against 0x57BFF0: it is not in the fields port; `movement.rs:507 max_speed` has no formation term; `data.rs:1720` loads positions only.
* **Takes:** formation modifiers in `Def::formations`, a term in `max_speed`. The `player_orders.rs` part is the UI owner's file: same agent, by agreement.

#### M7. Recruits walking out; regiment orders (M): in on the ports, see the status above; the second recruit's Sleep phase is open

* **Shows as:** `rus_infantry_co` destination: native (159, 360), ours (159, 359.5). Second member: native (159, 359), ours (158.29, 359.29). The first regiment's move at 1442 runs on: at 3296 ours is 23 m ahead of native.
* **Native:** 0x4FACA0, Mind vf +0x198 0x52F320, `KKC_UAIStateDeploy` 0x52DE40 / 0x52F560 / 0x52F7A0; regiment `KKC_CAIStateMove`: 0x520320, 0x520440, 0x5276B0, 0x525740, 0x51F7A0; slots 0x5797E0.
* **Port:** `aa_navigation::company_move` 756 + 660 (49 trait methods), `aa_movement::formation` 477; `aa_navigation::solver` in use.
* **aa-play:** `movement.rs:2948 walk_member`, `:2971 begin_walk`, `:1744 order_think`, `:1880 dispatch`, `:1200 start`.
* **Takes:** M4's host for the regiment Mind too. **needs** M4. Size stands only after M4: this is where hidden causes are most likely.

### Fog and supply lane

Files: `zones.rs`, `fog.rs`, `supply.rs`. These lanes move getters only through F3 and the supply end of an unpack. They can run in parallel with everything, and they were being edited during this census.

Status of 2026-11-02, third round, free run on `Berlin2v2_0`: fog first difference 1600, and the one cause first on anything is a member's zone 6 tile, which follows its position; supply first difference 1784, a position too; combat none. The zone 1 row on a city is gone: `combat.rs` no longer scans the control range, the ZOC's effects `0x514DA0` -> `0x514E40` -> `0x57AB70` / `0x567840` run from tile changes and a ZOC moving (`Engine::take_entered`, `combat::zone_entered`, `combat::defender`). The numbers in section 6 are from before this and are no longer current for this table.

Still not bound, measured as absent from all five fight and Berlin traces: zone types 2, 3, 5, 7 and walls (tile `+0x1C`). The Element Tick's detached teleport is bound (`element/fight.rs`, its free spot a rendition); actor `+0x100` bit 24 (falling) is not.

| id | size | shows as | native | port | aa-play | takes |
|---|---|---|---|---|---|---|
| **F1** ZOC zones | closed | zone 4 on buildings and organizations: gone from the table | Defender InitExisting 0x566DB0 (+0x218, +0x2FC for an organization), Organization AfterInit 0x579DD0 | aa-zones `component::create_zone`, `zone::move_to_position` | `zones.rs` `zoc_init`, at creation and, for a building, at the end of its unpacking (`0x4D21A0(1)`) | radius, tile and flags equal wherever the thing stands where the original's does. The enemy-entered effects 0x514DA0 -> 0x57AB70 / 0x567840 are not bound (bit 14 of actor +8 answers clear): `combat.rs` still scans the control range |
| **F2** membership | closed | zone 6 on trucks, members and leaders: first-on causes gone, what remains follows positions | 0x57CBA0 -> 0x5705A0 sets byte +0x1C when a member joins (why the truck has no zone at creation). 0x4D7230 +0x138 = the creation block's +0x40, the organization: 0x56EC90 makes no ZOD and asks for a deferred tick (0x50ACC0, 1e-4). 0x5709D0: leader, or a tile under it holds the organization's zone 4 (0x4EA2C0) | aa-zones `component::activate`, `query::contains` | `zones.rs` `element_update`; 0x5709D0 runs in the Element's queue-1 Tick 0x570B90 (`fog::element_pass` reads the bucket until the scheduler delivers it: `fog::element_delivery`) and in 0x579DD0 | the Element Tick's teleport is bound (`element/fight.rs`); actor +0x100 bit 24 is not |
| **F3** city zone and capture | zone done, tick waits for movement | zone 8 equal from tick 1. **Creator found: City AfterInit 0x563FE0** (slot 29, virtual +0x14, reached only through the vtable): 0x4D69A0(8), 0x515F40 with ActorData +0x5B4. 0x563E10 was wrong: that is the Character's | Entered through 0x4D6350 -> 0x4CFEA0 -> 0x515620 -> 0x564300 (the film's three getters carry this chain) | `zones.rs` `city_init`, `entered`; `supply.rs` `city_entered`, the City Tick's 0x564C10 now asks the zone (0x4EA2C0) | proof of the tiles: the film enters on tile (67, 199) at 1767, having walked (67, 198) from about 1745 and passed under the radius well before. Water stops the flood there; ours has (67, 199) in and (67, 198) out (`zones_parity::berlin_city_zone_stops_where_the_film_enters_it`). The capture now follows the regiment. In the forced-all run, where the positions are put on the native ones every 8th tick from 1200, ours makes the same 3 entries from 1769 against native 3 from 1767, and the provided money differs only at samples 587, 888, 1063..1065 and 3600, so the native steps at 1784, 3600 and 4288 are met to within a sample. City +0x10, the blocks that go with a city (0x5643A0), is still a radius scan |
| **F4** supply at unpack | closed | zone 0 absent until the end of the unpacking | `assemble_initial`: selection_special (state 25, 26) leaves Supply and Defender out; 0x576319 -> 0x4D21A0(1) -> 0x4D3068 -> 0x563FD0, 0x58B0E0 | aa-economy `supply` | `supply.rs` `tick` skips an unpacking thing, `supplier_tick` makes the slot on its first run | what remains in the supply table is the unpack tick (M2) and the bucket phase |
| **F5** ZOD of a built thing | closed | zone 6 radius 16 natively, 12 here | The unpacking building is state 26 (st 0x4001000), not state 21: `0x56F140` takes the normal branch, ActorData +0x1CC. The old `under_construction` took it for 21 and gave +0x1D0 | aa-zones `component::tick` | `zones.rs` `under_construction` reads `constructing` only; `supply.rs` state 21 likewise | |
| **F6** reverse RZOD | closed on our side | SVar: EngagementShowReverseRZOD = true (SVars_AA.tgi). No zone 1 natively: the kill orders (3297 on) never reach +0x118 or +0x1DC. | Writers of +0x118: 0x523DD0, 0x523EA0, 0x524C10 (the engage list) and the Defender 0x567140; a member's weapon target is not one, and a Mind-less thing has no reverse source | `zones.rs` `engagements`: the invented "actors whose target it is" went, a kill target held in `engage` does not count, +0x130 is bound (`combat.rs` `attacked_by`, 0x4D6190, event kind 3 only) | the film's `move` on a city becomes `Command::Attack` in `mapping.rs`, which is what put a regiment on the city at 1443. Natively the regiment walks there and stops. To be read against 0x527E30 by whoever owns the mapper |

Not causes: "zone 6 tile (position differs too)" and "supply stock +18" (a one-second phase from the drawn bucket: forced-rng moves its first tick from 718 to 1023) are knock-ons of movement and the chain.

### Combat lane

Files: `combat.rs`, `morale.rs`. `Berlin2v2_0` has no health or death difference and no combat row at all in the free, forced-rng and forced-all runs. The fighting is measured on `Berlin2v2_1` through the census, with the fight trace's morale and Mind state: `AA_PARITY_BERLIN_FILM=<film> AA_PARITY_BERLIN_NATIVE=<trace> AA_PARITY_CENSUS_RUNS=forced-all`. Forced-all puts health and position back on the native values at every sample, so a health row there is the damage of one 8-tick window: it says who fired and for how much, but one shot a tick off shows as two rows.

**Status 2026-11-03, fifth round: every order state of a regiment is on its Mind.** What stood between the fights and the original, found by reading the original's getter sites against ours in `Berlin2v2_1` (the census's getter table, forced-all) and the first fights of `Comrades2v2_0`:

| cause | what it was | proof |
|---|---|---|
| block `e81` on Comrades | `0x4D8290`: a component the record neither saved nor had in its default mask, that the definition has now, is made at the Load. The record is the old `eastern_europe_city02`, masks `0x40A241` against the definition's `0x2040A241`: it has a City, so `+0xD4` and the refusal `0x522CC0`, and a zone 8 of radius 0. Berlin's blocks saved the bit in the default mask: none. `PlacedThing::saved_city`, `combat::has_city` | the masks read from the map; `map_queues::a_record_older_than_the_city_node_gets_the_component_at_load`; the zone row. Shots on Comrades 6890 to about 130 |
| the rout's draws | `0x52B530` takes one getter (`0x5212CC`) and draws from the object it returned (`0x52B4E4`, `0x52B629`, `0x52B699`, `0x52B8E8`, `0x52B9F5`): 25 getters and 2132 draws in `Berlin2v2_1`. Ours took one getter per draw | the trace's `D` lines against its `G` lines |
| rout and exhausted | now the Mind's: `aa_mind::cai_rout`, `rout_select`, `rout_score`, Mind +0x20C and +0x270 (`0x521AF0`, `0x51E260`, `0x51DDF0`, `0x51FAE0`), the arrival arm `0x522567`, the time arm of `0x522B90`, the failure arm `0x527658`, `0x578140` with Mind +0x5C, the Sleep's `0x51F9F0`. The rout's end asks the ZOC's effects at the next update (`0x51E130` -> `0x4F3D90`, `combat::zoc_rescans`). The ZOC and ZOG terms of the score now count | with this alone `Berlin2v2_1` had 25 routs against 25 |
| the current formation | Organization +0x74 follows the Mind state: `0x57C400` -> `0x57BFF0` with the state's `formation_required`: DESIRED +0x70, BROKEN none, COMBAT the layout's +0x44, RETREAT its +0x48. `0x57BF50` swaps the property (the layout's BrokenProperty: SPEED_MAX +25%), recomputes the slots, and with no formation or a loose one `0x5797E0` draws two for each empty slot: 760 getters in `Berlin2v2_1` that ours did not have. `player_orders::formation` is now +0x74, `desired_formation` +0x70. `FormationKind::None` and `Plain` are told apart (`0x525740`) | getter sites `0x5798BB`, `0x5798CD`: native 760, ours 506 in forced-all |
| guard | now the Mind's: `aa_mind::cai_guard`, Mind +0x17C, with the embedded Kill, Move and Sleep (`0x529420`, `0x529490`, `0x527CE0`, `0x51EE20`, `0x51DC80`, `0x51DD30`, the arms of `0x521F90`, `0x52243C`, `0x527440`). Its Bombard stands for a task of `combat::tasks` | code; `tests/orders.rs` |
| explore | now the Mind's: `aa_mind::cai_thinks::explore`, Mind +0x220, `0x524100`, `0x5249A0`, `0x5213A0`, `0x51E180`, `0x51E1B0`, `0x524170`, `0x5213E0`. **Rendition:** `0x5242C0`'s walk is its bisection; `ExploreFinishAreaBonus` is not added | getter sites `0x5241B3`, `0x5241E7`, `0x52430B`, `0x5216D7` by name |
| a target held by its members | `0x4D4900`: an organization asked `+0x130(zone, 1)` answers by its attached members' tiles alone. Ours asked the banner's tile, so a regiment an enemy's members had walked into emptied its Engage list (`0x523EA0`) at once and went back to Sleep: on Comrades no fight began until the banners stood close. `0x51FBC0` asks with 0 | the order 6694 (zone entered, Engage, Sleep two ticks later) of the Comrades window |
| `0x51FBC0` | the thing's own position, an organization's and not a member's, and its zone test | code |
| state 20 | taken at a hit before that hit's record (a first hit does not set it, `Berlin2v2_1` `0x3478` at 5680 and 5688), at a death and at the Element Tick: it falls 7 to 8 s after the last hit (208 falls measured). A thing with no Element Tick still answers when asked | the trace |
| projectile flight | `0x4C8CB0` -> `0x580BC0`: from the ability's spawn position to the target's hit position with both height ratios, `projectile_life`, the kinds; `0x580920` on queue 0, newest first, before the Performer's events, strictly below zero. The target's place follows it. `combat::flight_seconds`, `combat::impacts`. **Rendition:** no actor flies; kind 2 takes the source's speed as it stands | compiled; shots of `0x580BE1` are counted, no landing tick has been compared |
| turrets | Performer +0x5C: `0x57DD90`, the aim of `0x57EF00`, and its registrations `0x57E8A3`, `0x57DE7B`, `0x57DC7E`, `0x57DCE2`, `0x57F443`: 615, 615 and 540 getters on Farmhouses that ours did not have | getter sites by name; no window compared yet |

**The home village, established.** The original on a v14 random-map film (`ba445000bb`, `rmc_southerneurope`, under the sampler to 2400 ticks): the village 37.5 m from the Corps HQ of kingdom 1 has owner 1 at tick 1, with no `0x564300` before tick 2150. The others are owner 4 until one is taken at 2160. So it comes with the world's start, not through a later City Tick. The replay of that film in aa-play now gives the same: owner 1 at step 1, and the second village at 2162 against 2160, so the 115 s of the Chuikov comparison was another build's.

Measured by `tests/order_fights.rs` (`the_fight_films_fight`) on the tree of 2026-11-03, with the other lanes' work in it (the Berlin gate there: 140 of 266, first 2397). Free replay: each film has left the original before its fighting, so a number close to the original's is not parity. Native shots are the `0x4C8190` draws; 901 of Kursk's are the engineers' Repair (`0x5359D0`), so 2640 fight there.

| film | deaths ours / native | first death ours / native | routs ours / native | shots ours / native | checkpoints, first difference |
|---|---:|---:|---:|---:|---:|
| `Comrades2v2_0` | 21 / 27 | 6820 / 6712 | 3 / 4 | 222 / 349 | 128 of 951, 2193 |
| `Farmhouses4FFA_0` | 108 / 107 | 3034 / 3032 | 12 / 15 | 1810 / 2666 | 97 of 1818, 1666 |
| `Kursk2v2_0` | 75 / 119 | 4856 / 4584 | 32 / 15 | 1731 / 3541 | 126 of 1318, 2159 |
| `Berlin2v2_1` | 125 / 133 | 6092 / 5760 | 31 / 25 | 2487 / 2929 | 138 of 1187, 2363 |

Before this round: deaths 8, 82, 94, 129; first death 7341, 3026, 4841, 4949; routs 0, 32, 28, 41; shots 6890, 1510, 1808, 2147.

The first-death column counts anything that reaches zero health, so two of its native figures are no fight: Farmhouses' 3032 (and ours 3034) is a `smoke_grenade` running out, its first hit on a soldier is at 4864 and its first soldier dies at 5000; Kursk's 4584 is the city block `eeurope_cityblock12x20_02`, its first soldier dies at 4856. In the client, `Farmhouses4FFA_0` at tick 5710 has one thing dead beyond that grenade where the original has lost nine soldiers by 5456: the totals agree (108 against 107) while the fight does not come at the same time.

**Renditions left:** `combat::decide` and `combat::choose` are `0x5340C0` and `0x534700` in f32 with the original's stores, not the ports' traits, and the candidates are the engage list, not the cached list of `0x57A780`; a Bombard, Repair and Construct are `combat::tasks`, on no Mind state; the Siege state has no site (no wall thing is hosted); a lone actor fires from `combat::fire`; the denizen's threat list is read as who stands in its range; a model's animation set is the first; group following (Organization +0x164) in Explore has no host.

**Status 2026-11-02, fourth round: the fight is hosted on the Minds.** The states the combat code stood in for are now the original's, on the stack of the hosted Mind, through the ports:

| what | where | port |
|---|---|---|
| regiment Kill (+0xC0), Chasing (+0x120), Reform (+0x1B8), Engage (+0x1CC), `0x524C10`, `0x524A00`, `0x524AE0`, `EventTargetGone` `0x521F90`, the Kill arm of `0x5222E0` | `movement/regiment/fight.rs` | `aa_mind::cai_thinks::kill`, `cai_thinks::reform`, `cai_engage_think`, `cai_engage` |
| element Engage (+0x12C), Kill (+0xF8), `0x532070`, `0x5351C0`, `0x533D90`, `0x5307A0`, `0x52D820`, `0x52EF40`, the Element Tick's teleport | `movement/element/fight.rs` | `aa_mind::uai_thinks::uai_engage`, `uai_kill` (its Think's steps, by hand) |
| the Performer: start, animation draw, use frame, end, reload, `0x57DC90` | `combat/performer.rs` | `aa_combat::performer` |

What this changes that the old pass could not do: the kill walk to a city is the Kill state and ends as the original's does (`0x520730` -> Mind +0x140); a new Attack replaces the Engage state and empties its list (`0x51D920` -> `0x52A810(2)`); a fight that ends with no chase goes through the Reform state; a target leaving the ZOC is dropped unless it or the regiment is shooting (`0x523EA0`); a chase runs when the only target flees (`0x520D90`); every shot takes its animation's length and its reload, and draws (`0x4C8170`).

`Order` is now what the Mind shows (`fight::reflect_order`, at Mind +0x1A4): the rest of the game still reads it, and `OrgCombat::auto_order` / `saved_order` hold what the Engage, Chasing and Reform states stand over.

**The next cause, Comrades.** Block `e81` holds the film down (8 deaths of 27, 6890 shots, nearly all at it): the film's `move` onto it at 1443 is a Kill state, the arrival makes an Engage state, and it never ends, because a body at zero health stays a target (`0x4C8740` has no health test). The original refuses it at `0x522CC0`: the trace has a zone 8 of radius 0 on that block and on no other block of the five films, so it has `+0xD4`. What gives it that is not found: `PlacedThing::saved_city` (the record's slot 29) and a converted record with the node both left the numbers unchanged. Print `map.placed` for that record first.

**Renditions left, each marked at its place:** `combat::decide` and `combat::choose` are `0x5340C0` and `0x534700` in f32 with the original's stores, not the ports' traits; the Siege state has no site (no wall thing is hosted); `0x57DD90` (other turrets aiming) is not bound; a model's animation set is the first; the rout and exhausted states are still `crate::morale`; guard and explore are `crate::orders` and enter the Kill state by writing `Order::Attack`; a lone actor still fires from `combat::fire` at once; the teleport's free spot is `0x554AD0` for `0x556070`.

Measured by `tests/order_fights.rs` (`the_fight_films_fight`) against counts read from the native traces: deaths are actors reaching zero health, routs are organizations entering `cai_rout` (the earlier "far fewer routs" compared events with samples, which is no comparison), shots are the original's `0x4C8190` draws (one per ability started) against ours. Free replay: each film has left the original long before its fighting, so a number close to the original's is not parity.

| film | deaths ours / native | first death ours / native | routs ours / native | shots ours / native |
|---|---:|---:|---:|---:|
| `Comrades2v2_0` | 8 / 27 | 7341 / 6712 | 0 / 4 | 6890 / 349 |
| `Farmhouses4FFA_0` | 82 / 107 | 3026 / 3032 | 32 / 15 | 1510 / 2666 |
| `Kursk2v2_0` | 94 / 119 | 4841 / 4584 | 28 / 15 | 1808 / 3541 |
| `Berlin2v2_1` | 129 / 133 | 4949 / 5760 | 41 / 25 | 2147 / 2929 |

Before the hosting: deaths 26, 137, 75, 88; first death 9096, 3039, 4794, 4957; routs 7, 20, 15, 46; shots 1538, 3274, 2150, 2349. The shots were about four times the original's on Comrades, because an ability's animation and the rule that the Mind waits for it were not there.

**Status 2026-11-02, third round (`Berlin2v2_1`, forced-all).** The census has a new row, `regiment Mind state`, the fight trace's `ms=` against `morale::regiment_state`. What each cause was, and what proved it:

| cause | what it was | proof from the recording |
|---|---|---|
| morale of a regiment in its Unpack state | it replenished from the swap on. The Unpack state lasts until the building has unpacked (`0x4D1050` true while constructing, `0x4D1080`), and `mind_unpacking` does not replenish | HQ regiment `0x345a`: 74.25 from 688, Sleep at 816, first gain at 826, all equal now. The row's first difference moved from 698 to 5584 |
| attack efficiency +88 from the state | `formation_required` of the state the Mind reports (`0x51D8F0`, `0x520250`, `0x51DC20`, `0x51E2C0`, `0x51D9E0`). A `move` command on a hostile thing is the Kill state with Mind +0x20's kind 5, which reports mind_moving: desired formation and behaviour (`ActorOrder::MoveOnto`, from the mapper) | `0x346f`: 0.5 and 0.75 in the Kill state from `move` commands, 1.0 from a `kill` in the same state |
| a Kill walk to a city never ended | the state ends when the city stops being an enemy, by its capture (`0x520730` -> Mind +0x140) | `0x346f` Sleep at 2472. Ours now ends 2 samples early (2456) in forced-all |
| area damage stopped at centres | the query is `0x558040` mode 1: `(soft radius + area)^2 >= d^2`; attenuation subtracts the target's soft radius, not DefaultRadius. Buildings never took splash | `0x2fa1` city center: native takes only multiples of 27, from grenades. Ours took no damage at all before this |
| technologies in damage | `shooter_power` and `target_defense` read only the level and enchantments. AV, DV and RESIST_DAMAGE now go through the thing's properties and its owner's technologies | 25 + 2 = 27, the native number on the city center: the Blast grenade of a kingdom that researched at 1681. Ours was 25 |
| terrain in damage | `0x4D4130`: a Mover takes the properties of the terrain under it (infantry in city or forest DV +4, armor_piercing resist). Rendition: the tile is read at the time of asking, not kept from tile change to tile change | code only: no window has been isolated to prove it |
| recon morale | a regiment's own properties are in its members' modifier state too: a member adds MORALE +3 | `0x35be`: 28 + 3 per living member, 31 to 43 over the whole film. The row is gone |

**The four films in free replay** (`tests/order_fights.rs`, deaths against native actors at zero health, first death against native first): Comrades 22 of 27, 7264 against 6712; Farmhouses 113 of 107, 3027 against 3032; Kursk 85 of 119, 4861 against 4584. They are no proof: the replays had left the original long before, and that build also carried the scatter draws at 483 (`creation::organization_scatter`).

**Not closed from this round's list.** The Chuikov comparison: the original's log (`sai_goals_run1.csv`) has Research at x1.75 from its second think at 30.31 s, so it is in the main-game ego by then; ours changes at 60.1 s, income 25 until 50, and takes the home village at 115 s where the earlier radius scan took it at 1.1 s. Whether the corps enters that zone at the start is not established. Fog cost in the official profile, under load: 807 us a tick (budget 1000), and with all 2000 members detached 4801 us, of which observe 3971 us.

**What was left there, first on anything, before the fourth round:** `organization living +64` is a member arriving one sample off in single ticks every 64 (the regeneration's drawn bucket); attack efficiency and desired, read in the same Mind-state column as everything after it; building and truck health, which are shot timing.

#### X1. Kill orders (M)

**Done 2026-11-02, fourth round:** the Kill state is hosted on `aa_mind::cai_thinks::kill` (`movement/regiment/fight.rs`); `order_think` no longer runs for a regiment. It walks by its embedded Move, and at the target the arrival `0x5222E0` starts the Engage state or, where `0x521060` refuses (a city, a block that goes with one), ends through `0x521F90`. The text below is what the first census saw.

* **Shows as:** five `kill` orders (3297, 3393, 3929, 4217, 4281). 3393 and 4281 are refused here "cannot attack an own unit" (`movement.rs:1119`). With positions forced to native the 3393 refusal goes, so that one is a knock-on: ours is 23 m ahead by then and takes the city first (M7, F3). 4281 stays. The target block is at full health natively at 3296 and 3392. Whether native refuses such a target at all is not established: that check is aa-play's.
* **Native:** `KKC_CAIStateKill` and the element's approach; the kill decision 0x5340C0 does not draw in this film.
* **Port:** `aa_mind::cai_thinks::kill` 400, `aa_mind::uai_kill` 840, `cai_engage` and `aa_combat` (183 proven).
* **aa-play:** `movement.rs:1115`, `combat.rs`.
* **needs** M4 and M7.

#### X2. Group behaviour (M)

**The arithmetic is fixture-proven since the fifth round** (`research/phase8-orders-input/group`: the centre, the places, a place at a point, the list order, the speed loop of the commanded Update). A selection is a list built at its head (0x4ECFF0) and a group sorts it by 0x4D0780: the mapper and `group::form` now do both. **Written, 2026-11-02:** `aa_play::orders::group` (the GGroup thing, the selection path 0x4ED360, delivery 0x4DF840 -> 0x4DE7F0, the three Update states, the formation placement 0x4DF600). There is no oracle: the checks are the film's `group` things and `order_fights`. What stays a rendition is listed at the head of that file.

* **Shows as:** order 4201 `group_behavior_enable` then a `move` on two regiments. Native creates a `group` thing at 4201 (one id, one scale draw) and moves them as a group. No equivalent here.
* **Native:** selection command 0x4ED360: cases 0x25 / 0x26 set selection group +0x18, then `KKC_GWorld::CreateGGroup` 0x4FBE10, 0x4E0940, 0x4E2940, 0x4DE740, 0x4DFE20. `group_formations.tgi`.
* **Port:** only `aa-runtime/src/player_order_guess.rs` (the guess kernel of 0x4DE740). **No port** of the group thing or its Mind: oracle and port are part of this.
* **Takes:** new selection state in `Mapper` for the decree, a new `Command`, a group Mind. **needs** M4. Last: it affects the last 338 ticks only.

## 4. Who can work in parallel

```
core:      K1 ─ K2 ──────────────┐
           K3 (map, scheduler, doodad leaf)
           K5
           K4 ── character.rs ── agreed call with movement
movement:  M1 ─ M2 + M3 ─ M4 ─ M5 ─ M7 ─ X1 ─ X2
                          └ M6 (with UI owner)
fog:       F1 ─ F2      F4   F5   F6       F3 (after M4)
```

* **First round (no two touch one file):** core K1 + K2; core K3; movement M1 + M2 + M3 (which takes `deploy.rs` for that step: tell core); fog F1, F2, F4, F5. K5 fits any free slot.
* **Shared files, by agreement:** `world.rs` (K2's `Command`), `scheduler.rs` (K3 and K4 add `deliver` arms: additive), `creation.rs` (K4's draw).
* **The check for each step** is this census, not a new tool: the getter window table for that group must rise, and the gate's matched count should move. Expect the gate's number to stay low until M1 to M4, K3 and K4 are all in: the first missing doodad draw is at 583 and the first Character draw at 1202, and the windows before 1202 only need K3 and M1 to M3. **The realistic next gate, as an expectation: every checkpoint to 1190 (70 of 266).**
* **Timing tests** in aa-movement and aa-play are known flakes.

## 5. The getter ledger

Every native getter of the film range by group: how many, first tick, how many fell in equal windows, and how many windows it was the first difference of, free and forced-rng. In the free run the first difference is mostly a later getter than the real cause. Forced-rng is the exact one.

| native group | native | first | cause | equal free | first diff free / from | equal forced-rng | first diff forced-rng / from |
|---|---:|---:|---|---:|---:|---:|---:|
| production start 589420 -> queue 0 | 15 | 2 | done | 4 | 4 / 747 | 8 | 0 |
| HQ number 4E2640 | 4 | 402 | done | 3 | 0 | 4 | 0 |
| component InitCreate 563FD0 -> queue 1 | 216 | 402 | done | 18 | 0 | 42 | 3 / 1392 |
| economy InitCreate 56F250 -> queue 1 | 8 | 402 | done | 3 | 0 | 4 | 0 |
| element mind jitter 530E50 | 60 | 402 | done | 3 | 0 | 9 | 0 |
| mover countdown phase 50AB10 | 12 | 402 | done | 6 | 0 | 10 | 0 |
| random scale InitCreate 4E9FC0 | 77 | 402 | done; unpack tick M2 | 9 | 21 / 587 | 19 | 3 / 587 |
| mover order 5773C0 -> queue 0 | 3081 | 403 | M4 | 15 | 352 / 553 | 145 | 114 / 554 |
| ActionGo from 537150 | 2173 | 421 | M4 | 9 | 0 | 95 | 21 / 1408 |
| ActionGo from continuation 5379C0 | 508 | 483 | M4 | 1 | 0 | 24 | 12 / 1536 |
| ActionGo from get-back-to-path 532B50 | 100 | 525 | M4 | 1 | 0 | 7 | 0 |
| ActionGo from low path 5336B0 | 124 | 403 | M5 | 3 | 0 | 10 | 0 |
| ActionGo from 533140 | 107 | 1469 | M5 | 0 | 0 | 7 | 1 / 4224 |
| ActionGo from 536F40 | 27 | 1452 | M5 | 0 | 0 | 0 | 0 |
| ActionGo from queue wait 536AD0 | 13 | 1801 | M5 | 0 | 0 | 0 | 0 |
| ActionGo from 531520 | 11 | 1801 | M5 | 0 | 0 | 0 | 0 |
| ActionGo from other callers (0x4EABEE, 0x52DC7A, 0x4CEE78, 0x5350AA, 0x50AF65, 0x50ADAC, 0x5314FE, 0x6F77EF) | 18 | 505 | M4, X1 | 1 | 0 | 2 | 0 |
| disturbable delay 5902C0 | 165 | 583 | K3 | 0 | 26 / 583 | 6 | 6 / 583 |
| organization goal 57C680 -> queue 0 | 4 | 587 | M3 | 0 | 0 | 0 | 0 |
| unpack end 4D21A0 -> component 563FD0 -> queue 1 | 4 | 715 | M2 | 0 | 4 / 715 | 0 | 3 / 715 |
| character 563BA0 at creation | 48 | 1202 | K4 | 0 | 0 | 4 | 0 |
| character 563BA0 from node reached 530AE0 | 91 | 1230 | K4, M4 | 0 | 18 / 1312 | 1 | 7 / 1232 |
| character 563CA0 queue 1 tick 563E10 | 63 | 1278 | K4 | 0 | 10 / 1584 | 0 | 4 / 3552 |
| character 563BA0 queue 1 tick 563E10 | 37 | 1390 | K4 | 0 | 9 / 1904 | 0 | 0 |
| character 563BA0 from Mind states (0x531EF1, 0x5324AF, 0x52FC45, 0x52DC68) | 17 | 1651 | K4, M4 | 0 | 2 / 1696 | 1 | 1 / 1808 |
| 564300 -> queue 1 | 3 | 1767 | F3 | 0 | 0 | 0 | 0 |
| (aa-play draws where native stops) | 0 | | M4 | | 44 / 552 | | 262 / 552 |

Totals, film range: native 6986. Free run ours 7959, 1127 of 1617 windows equal, 176 short, 270 long, first getter in another tick at index 76 (native 553, ours 552). Forced-rng: ours 7686, 1180 equal. S counts: film 12174, ours 13105; 198 of 266 intervals differ in count; the first are at 595 (28 / 26), 714, 731. The native trace's getter chains are stack scans, so a group name can be wrong for a few entries; the site is exact.

## 6. Every divergence by lane

Free run; `forced` columns show first tick / fields in the forced-rng and forced-all runs. Movement signatures are folded by thing and field; the motion classes (who moves, how far) are in the JSON. `first-on` 0 means knock-on.

### Existence: first 586, 90 of 1617 sampled ticks

| signature | first | last | ticks | things | first-on | forced-rng | forced-all | cause |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| missing / extra `*_trailer_*` | 586 | 1504 | 6 | 4 | 4 | 586..1504 | 587..1288 | M2 tick |
| extra `rus_infantry`, `_mg`, `ger_infantry`, `_mg`, `_mortar`, `rus_infantry_at` | 1256 | 4528 | 35 | 28 | 28 | gone | 4136/14 | one sample early: the org's regeneration phase from a drawn bucket, so K3 and the chain |
| organization members: aa-play has more | 1256 | 4528 | 31 | 4 | 0 | gone | 4136/14 | same |
| missing `ger_infantry_mortar`; members: native has more | 3992 | 4536 | 69 | 5 | 3 | 3992/69 | 3992/69 | K2 |
| missing `rus_infantry` | 4408 | 4536 | 3 | 3 | 3 | gone | gone | chain, K2 |

Native `group` at 4201 is X2: it has no `A` line, as it is not class 3, so only the creation list shows it.

### Economy: first 587, 1031 sampled ticks

| signature | first | last | ticks | kingdoms | forced-rng | forced-all | cause |
|---|---:|---:|---:|---:|---:|---:|---|
| stockpiled +1C4 money | 587 | 4536 | 1031 | 3 | 587/2311 | 588/43 | M2 tick, then integrated: a one-off delta of 0.0135 |
| provided +1A0, effective +1AC: money, ammo, oil | 888 | 4280 | 83 | 3 | 888 | 587 | M2 tick; spans 1664..1776, 3384..3592, 4104..4280: F3 capture |

Forced-all shrinks the stock difference to 43 fields, so no rate is wrong: no economy cause of its own. Shortage +1F0, consumed +1B8 and the stocks of the other six resources never differ.

### Production: none. 0 of 1617 sampled ticks.

### Movement: first 1, 1617 sampled ticks

| signature | first | last | ticks | fields | things | first-on | forced-rng | forced-all | cause |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| building, other, member, truck height z | 1 | 4536 | 1617 | 32,498 | 31 | 25 | same | same | K5 |
| truck facing | 484 | 3472 | 406 | 448 | 6 | 0 | 484/446 | 484/443 | M1 |
| truck position | 484 | 3472 | 360 | 508 | 6 | 0 | 484/513 | 484/450 | M1, M2 |
| organization position | 484 | 4536 | 718 | 2980 | 10 | 1 | 484/2950 | artefact | M3, M4 |
| truck speed +20 | 552 | 1392 | 9 | 19 | 5 | 0 | 552/20 | 552/11 | M4 |
| building facing, position, speed | 586 | 1504 | 5 | 13 | 3 | 0 | 586 | 886 | M2 tick |
| member position | 1224 | 4536 | 325 | 5891 | 46 | 10 | 1224/5886 | 1224/5566 | M6, M7 |
| member speed +20 | 1224 | 4536 | 385 | 5079 | 45 | 8 | 1224/5063 | 1224/5091 | M6 |
| member facing | 1264 | 4536 | 410 | 5727 | 45 | 4 | 1272/5703 | 1272/5100 | M7 |

### Combat: first 586, 6 sampled ticks

Building and truck health and maximum at 586, 888 and on to 1504: the HQ swap one tick off (500 against 2000). M2.

### Supply: first 586, 623 sampled ticks

| signature | first | last | ticks | fields | forced-rng | forced-all | cause |
|---|---:|---:|---:|---:|---:|---:|---|
| building zone 0: aa-play has it, native none | 586 | 1416 | 403 | 403 | 586/398 | 588/401 | F4 |
| building supply stock +18 | 718 | 2848 | 288 | 426 | 1023/179 | 717/440 | chain (bucket phase) |
| building zone 0: active differs | 1016 | 3600 | 43 | 43 | 1016/47 | 715/14 | F4, chain |

### Fog: first 1, 1617 sampled ticks

| signature | first | last | ticks | fields | things | first-on | cause |
|---|---:|---:|---:|---:|---:|---:|---|
| building zone 4: native has it | 1 | 4536 | 1617 | 8787 | 8 | 4 | F1 |
| building zone 8: native has it | 1 | 4536 | 1617 | 6468 | 4 | 4 | F3 |
| organization zone 4: native has it | 402 | 4536 | 1216 | 4965 | 10 | 10 | F1 |
| truck zone 6: aa-play has it | 402 | 1288 | 727 | 1397 | 4 | 4 | F2 |
| organization zone 6: tile, position differs too | 530 | 4536 | 478 | 753 | 10 | 0 | knock-on |
| building, truck detection ZOD radius +14 | 586 | 1504 | 6 | 6 | 3 | 0 | M2 tick |
| building zone 6: aa-play has it | 586 | 4536 | 1032 | 2722 | 4 | 0 | F2 |
| member zone 6: aa-play has it | 1208 | 4536 | 417 | 4083 | 46 | 14 | F2 |
| truck, member zone 6: tile, active, position differs too | 1384 | 4536 | 365 | 1908 | 12 | 0 | knock-on |
| building zone 6: radius | 1504 | 3600 | 30 | 31 | 2 | 0 | F5 |
| building, organization zone 1: aa-play has it | 3304 | 4536 | 155 | 308 | 4 | 2 | F6 |

### RNG: first 552, 964 sampled ticks

GWorld+D4. Section 5 has it by getter. Before 552 every window is equal; state compare differs only after the chain is off by a getter (583, K3).

## 7. Open questions

1. **Queue 3 owners.** K3 rests on reading the code: 0x50BF10 clears and loads, and `saved_periodic` passes over those lists. The 796 entries are measured; that they include the doodads that draw at 583 is the first thing to confirm (resolve the owners, or dump queue 3 with the sampler).
2. **The creator of zone 8.** Answered: City AfterInit 0x563FE0. See F3.
3. **What X1 will show** once a regiment arrives on time: whether an attack on the block starts, whether the 4281 refusal goes, and whether the zone 1 row (F6) goes.
4. **M7's size.** Every movement row after 1442 is read from a regiment that is by then 23 m off. Expect new causes.
5. ~~`Trace` has no site.~~ `SKind::RngGetter` carries one, spelled as `getter_group` spells the native's (`trace::sites`); a getter nobody named carries its source file. The census prints our getters by site and the first differing windows side by side.
6. **The 3992 row.** K2 was its named cause and is not. `missing ger_infantry_mortar` is the end of a longer row: a regiment's members come from the regeneration 0x57B5A0, one every 64 ticks here, and the first regiment's stop in ours after 1392 and start again at 1792 where native goes on (1456, 1520, ... 1712). The gate 0x579110 is what stops them. The likely reading, not proven: the supply state, because the truck that unpacks at 1417 natively is late in ours (the supply rows at 1194 and 1424). Read it again when the trucks are on time.
7. **Kursk.** Answered in the core lane's state of the second 2026-11-02 round: the herds are restored with their Minds and the rampage runs. What follows is how it read before. Its 7 placed things' saved components do not walk (`map.rs`), which is the 44 periodic and 37 deferred entries without an owner, and aa-play spawns a placed organization's members itself where the map saves them as records. Its 16 draws at load, 10 Mind jitters of members embedded in those records and 3 + 3 of 5798BB / 5798CD (0x57BF08), and the regiment states of ticks 1 to 17 need that restoration first. `aa_mapworld::organization_restore` is the port to bind.
