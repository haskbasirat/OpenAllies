# Open items after Development 21

What each owner left open when the 2026-10-25 round stopped. Addresses are AA.exe.

## HUD (aa-bevy ui/, input.rs, aa-ui)

GroupDisplay for multiple selections is the next visible step. Our panel shows only the head object.

- **Where it lives:** ObjectDisplay 0x62CBA0 owns four sub-displays:
  - SimpleActorDisplay 0x62CA40
  - CompanyDisplay 0x62A5D0
  - SettlementDisplay 0x62A740
  - GroupDisplay 0x62CAF0 (constructor 0x62C3C0, vtable 0xA5CEB8)
- **Template:** `UI/Game/object_display.tgi:ObjectDisplay/GroupDisplay`, from UnitDisplay. It holds:
  - Name and Owner labels (UIGroupDisplayUseNameLabel = true in AVars_AA);
  - a FormationButtons panel ("GroupFormationButtons");
  - pools of six SimpleActorMini (ids 0-5), six CompanyMini (6-11) and six SettlementMini (12-17), each renamed "%s_%i" (0x5F0F40).
- **SetSelection** (vtable +0xE0, 0x6296C0) handles up to six objects in list order:
  - Each takes a mini from its pool (0x625670): an organization gets a CompanyMini, a settlement goes through 0x7A32B0, anything else gets a SimpleActorMini.
  - Then SetActor (+0xEC) is called and the mini is added as a child.
  - Position: x = i * floor((width - formation panel width) / 6), y = mini_display_offset (6).
  - Each controllable organization with set_formation (command 0x21) merges into up to three "FormationChange <id> " buttons (0x628490).
  - The last actor processed feeds the Name and Owner labels (0x626310).
- **The minis** derive from MiniDisplay 0x62AD70, which builds PortraitButton (Portrait / ModelPortrait / Icon) and DescriptionLabel (0x628940).
  - SimpleActorMini 0x62AE90 adds a Health bar (0x628D40; refresh 0x628DD0).
  - CompanyMini 0x62FF20 adds:
    - PrimaryHealth (only with a leader);
    - Morale and AttackEfficiency, each with a target value;
    - HealthBars, two per row (0x62EB90);
    - the banner model (0x62F1B0);
    - RetreatRoutButton (0x62D6B0; refresh 0x62E9C0).
- **Clicking a mini:** "DisplayObjectSelect <gtid> 0" (0x6277E0, handler 0x610680), and "CenterOnActor <gtid>" (0x627A80).
- **Candidate highlight while dragging:** 0x437DC0 -> 0x4371A0 rebuilds the box candidates each frame. Candidates get view flag 0x8000.

Then:

- the Unpack impact preview (building minus truck, 0x602820);
- ghosts for organization layouts and city variants (component bits 7 / 29 in 0x688DC0);
- the hot construct command on unfinished structures (0x4DA570 type 4).

## Aircraft (core: combat/air*, minds.rs; movement: aa-movement aircraft*)

Air orders are still refused. B1 (the hosted aircraft Mind) is done. B2 (attack) is written behind the temporary `air-b2` feature in crates/aa-play/Cargo.toml.

1. **Wire the flight ports into aa-play** (movement owner). The ports in aa-movement are exact and fixture-proven: fly_to 0x560DB0, replace_path 0x560ED0, fly_formation 0x560F10, join/leave_formation 0x55EB30 / 0x55EDC0. aa-play still has no-op stubs. Use an adapter over `books.actor(a).aircraft.flight` and `books.groups`. Then:
   - put MoveTick 0x5612F0 on the queue-0 Mover;
   - start that Mover at actor init: 0x4D7287 calls the aircraft's vf+8, 0x55E600, which calls 0x576B20. This runs before the Mind's InitExisting 0x517C60;
   - on death, run 0x55ED00 (vf+0x34): leader handover, path clear, then the fall;
   - at map edges, call 0x4D04B0 / 0x4D7EA0 from the pose commit, which also set and clear `AircraftBooks::hidden`;
   - at the path end, call `combat::air::path_finished` (0x561747).
2. **Write the fire service** 0x57E030: mana debit, recharge, the bomb projectile.
3. **Combat** (combat.rs owner): skip actors where `combat::air::is_aircraft` in lone_think and fire.
4. **Finish B2:**
   - turn `air-b2` on and remove the feature;
   - port the dog-fight branches of 0x51A920;
   - do a real run: a bomber takes off, bombs, returns and lands. Only then stop refusing AirAttack / AirRecon.
5. **Readings to check against the original:**
   - slot health at InitCreate ignores creation mode 4;
   - recon on an actor uses its position;
   - join's object +0x10 radius is taken from the blocking footprint;
   - Mind+0x1D0 is read as projectile_type != ANTIGRAV;
   - 0x4C8F20 is reduced to filter / range / mana / recharge;
   - actor vt+0x104 and 0x45D6F0(0) are taken as speed_max.
6. **FPS:** repeat the airmind FPS run on a quiet machine. It showed about 7 FPS while 18 builds were running.

Movement also left: OrgMind on queue 4, and an idle re-measure of the planning worst tick.

## Effects and sound (effects owner: aa-bevy effects/, audio/, battle_view.rs)

Corpses now sink as in the original (0x459E10 -> 0x457DF0, 0x5617E0). The rest of the audit comparison is in qa-output/fx-audit/original/.

1. **Actor-state effect lists are never run.** These are the root-level WhenEnter / WhenLeave lists keyed by `actor_state` (FXC field 0x42C1B0, event names 0x4985E0, state names 0x477FC0, states set by 0x4D64F0). That leaves out:
   - pack and unpack sounds and the `building_pack` dust;
   - construction effects and the construction-finished sounds;
   - the motor pool sounds;
   - transport unloading;
   - plane crash on leaving `falling`.
2. **Tint modifiers** (`under_siege_vfx` red, `out_of_supply_vfx` grey-green) are not drawn. The timing is in 0x449920, and the colour comes from lerp(white, colour, pct) via 0x717E40. "Under siege" also needs the sim state 0x4D6660 / 0x586370, which is core's.
3. Grenade trails and contrails (VFXExtrusion) are not drawn.
4. **Not yet compared with the original:**
   - explosion size and duration, frame by frame;
   - acknowledgement voices;
   - ambient sounds;
   - weapon sound falloff with distance.

## Random maps (parity owner: aa-play mapgen*, aa-parity)

The generator (`crates/aa-play/src/mapgen*`) is exact on the stock data. Gate: `crates/aa-parity/tests/rmg_gate.rs`, all tests ignored, all read the corpus in `target/film-corpus`.

```
CARGO_TARGET_DIR=target/n-rmg AA_GAME_ROOT=<game root> AA_RMG_GATE_SETS=v14 cargo test -p aa-parity --features play --test rmg_gate --release --offline -- --ignored --nocapture <test>
```

| test | what it says |
|---|---|
| `every_random_film_generates_the_original_world` | Per film: generation runs, draws and end state, the first getter chain, the header's random picks, and against a native world dump (`traces/rmg-gate-<first 10 of the sha>.txt`) things by definition, position, height and facing, base and overlay heights, tiles |
| `open_film_starts_on_the_generators_stream` | `open_film` on each film on its own data: the stream after the world's sync dump, then its checkpoints until the first mismatch |
| `random_film_native_state` | One film against a FightSampler trace of the original (`traces/rmg-state-<first 10 of the sha>.txt`): every lane's causes, `AA_RMG_GATE_WATCH` prints things side by side |
| `corpus_film_draws` | One film against the original's draw trace (`AA_RMG_GATE_TRACE`): the first draw that differs |

Numbers of 2026-11-02, 43 random-map films of the corpus:

| data set | films | generation runs | native dump equal (end state, things, heights, tiles) | header's random picks from the stream | `open_film` on the stream | first checkpoint passes | no mismatch (cap 4000 ticks) |
|---|---:|---:|---:|---:|---:|---:|---:|
| v14 | 12 | 12 | 12 of 12 | 11 of 11 | 12 of 12 | 12 | 2 (`1d25a501b7` 15 of 15, `fd4649a138` 235 of 235 to the cap) |
| mod `A&A gg` | 7 | 7 | no dump | 5 of 5 | 7 of 7 | 5 | 3 (`f3bd589503` 50 of 50, `d22932af71` 26 of 26, `47206c3262` 16 of 16) |
| mod `player` | 4 | 4 | no dump | 4 of 4 | 4 of 4 | 4 | 0 (to 3 of 4) |
| mod `A&A edit` | 4 | 4 | no dump | 3 of 3 | 4 of 4 | 1 | 0 |
| mod `A&A Use` | 8 | 8 | no dump | 1 of 1 | 8 of 8 | 1 | 0 |
| mod `A&A - Copy` | 5 | 5 | no dump | 4 of 4 | 5 of 5 | 0 | 0 |
| `widths` | 3 | 3 | no dump | 3 of 3 | refused: the films do not decode without their data | - | - |

The chain check on the end state (FIGHT-FILMS.md's first getter method): strong on 2 films, weak on 23, refused on 18. Most films cannot say either way: too many placements in the first interval. The checkpoints above are the better oracle. 13 films' settings do not fit today's RMC (0x505B00), 8 of them on v14: natively they take the defaults and go out of sync, `SyncMode::Report` plays them on the stored settings, and there all 8 pass their first checkpoints.

Generation takes 0.6 to 4.6 s under load, `open_film` 1 to 5 s once the data set is read. The first film of a mod reads the mod: 12 to 66 s, and 590 s for `A&A - Copy` with nine lanes building.

**What the world needed besides the generator's own stream**, all found by `random_film_native_state`:

- A CityComponent exists only with a zone radius (0x45D6A0). A village's blocks have the node with radius 0 and no component, so `Def::city` is now the radius, and 0x4F6980 takes them.
- A city's PostCreate 0x564BE0 runs the layout 0x5644F0 with its list: the Structure actors it makes go on City+10 (0x5642A0) and change hands with the city. `GeneratedWorld::blocks`.
- A truck's trailer is a thing the generator makes (0x4D70F2), `Thing::Trailer` in aa-play and no actor. `GeneratedWorld::trailers`, `NativeIds::placed_trailer`.
- GThing +0x28 stays what it was at creation: the ground the later things changed is not read again. `replay_generated` puts it on the actors, and a generated map has `native_terrain`, so `height_at` is 0x4F0E50 there too. A start location takes its height at the position it leaves (0x4EEB40).
- ConfigureGWorld 0x50E8B0 destroys a kingdom with KingdomInfo +0x31 once the random settings are resolved, so a closed side draws its picks and keeps its GTID but the generator does not see it (`side05` and no `side03`).

**What the first differences are now.** On `1104a3b0f9` the truck is bit exact to tick 39, then the original replans around a bunker two ticks before ours (M5, `collision` has no real avoidance). On `ba445000bb` everything is equal to tick 455, then a recruited truck stops short of the original (M7). Neither is the generator: their lane is movement. Provided resources change one sample late at a capture (`supply::city_tick`, native 0x4D5230 does it at once); the stock does not differ.

**Left in the generator:**

- AddHeroesToKingdom 0x592E00: not ported, a world asking for heroes says so in `load_notes`. No film of the corpus has one and A&A hides the slider.
- Base terrain of mountain or water kind on a thing (0x551B40): not ported, says so in `load_notes`. No film reaches it.
- The mods with a `RandomMap` folder of their own (`A&A Use` 7 films, `A&A - Copy` 5, `A&A edit` 3): the first checkpoint does not pass, while `A&A gg` and `player`, on the stock RMCs, pass. Their RMCs ask for things the stock data never does (required cities with layouts of their own such as `rome`). Not known yet whether it is a cause in the generator or data changed since the film. The way to tell: the original's world dump on the mod. The tools are `mkmod.sh` and `nat_mod.sh` in the QA copy's manner; the one try on `A&A Use` stopped at 968 s while loading its SAI (unhandled Breakpoint at 0x404F60, the log's error call, with no message written), so it needs that looked at first. Two films of `A&A gg` were recorded on another data set (version RWE5.5.5.0) and cannot pass.

Note: SVars.tgi sets `RMGBalanceMarkerNudgeStepDelta`, but the game registers `RMGBalanceMarkerNudgeDelta`. The file value is ignored, so the object balancer's delta stays 4.0 and the kingdom balancer's is 2.
