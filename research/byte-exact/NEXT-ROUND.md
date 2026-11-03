# Open items after Development 21

What each owner left open when the 2026-10-25 round stopped. Addresses are AA.exe.

## HUD (aa-bevy ui/, input.rs, aa-ui)

Built on 2026-11-02, from the original's code; none of it has a side-by-side check with the original yet, because mouse clicks do not reach it on a hidden desktop.

- **Right press** (0x620760): the default order, `Command::Hot`, with no command named; each unit guesses its own (0x4DA570) in the mapper, on every machine. A selected object with a hot location command that takes a direction (0x620A00) sends it to the facing state (GameInterface+0xE4): `facing_arrow_vfx` past the drag threshold (0x688AA0), the direction from the press point to the cursor (0x688990), sent at the release (0x6210B0). An action button's Move, Run, Guard and Explore go through the same state with the left button, naming their command (flags 0x5C).
- **What is under the cursor** (0x43D240): the model's box in model space, and of everything the ray meets the nearest by the distance from the eye to the object's **position**, not to where the ray enters the box. A single selected object the ray still meets stays hovered (0x43DC70); a click on it goes to the nearest behind it (0x437B20). `Unit`s in `Selecting` keep it. 0x5E05D0's preference for an object without a live +0x94 link is not read: aa-play has no such link.
- **The selection in a film** (0x436AD0, 0x436B90, 0x436C50): `Command::Select` when it changes, decrees 16, 17 and 18, no longer written with the order.
- **GroupDisplay** (0x62C3C0, 0x6296C0): up to six minis, the formations merged, the Name and Owner of the last. A mini's portrait selects that object alone (`DisplayObjectSelect`, 0x6277E0); RetreatRoutButton gives retreat or rout to that organization (0x62D6B0). Left out: the mini's double click (`CenterOnActor`, 0x627A80), SettlementMini, the DescriptionLabel, the candidates' highlight while a box is dragged (0x437DC0 -> 0x4371A0, view flag 0x8000).

Then:

- the Unpack impact preview (building minus truck, 0x602820);
- ghosts for organization layouts and city variants (component bits 7 / 29 in 0x688DC0);
- the hot construct command on unfinished structures (0x4DA570 type 4).

## Aircraft (core: combat/air*, minds.rs; movement: aa-movement aircraft*)

A transport flies since 2026-11-02 (`crate::transport`: its host of `aa_movement::aircraft` is the place to grow the others from; it refuses formations and abilities). The other aircraft do not: only a transport's Mover is on the queue, and `movement::fly_to` is a no-op for them. Air orders are still refused. B1 (the hosted aircraft Mind) is done. B2 (attack) is written behind the temporary `air-b2` feature in crates/aa-play/Cargo.toml.

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

1. **Actor-state effect lists** run since 2026-11-02 (`ActorFx::enter`, `leave`; states 0x4D64F0, event names 0x4985E0, field 0x42C1B0) for the states the world holds: `structure_packing`, `structure_unpacking`, `construction`, `active_construction` (read as `construction`: natively the state of a structure being worked on), `under_siege`, `out_of_supply`. So the pack and unpack sounds, the `building_pack` dust and the construction effects play. Not run: the motor pool's, transport unloading, plane crash on leaving `falling`: the world holds no such state yet.
2. **Tint modifiers** (`under_siege_vfx`, `out_of_supply_vfx`) are drawn: `ActorTint` is the timing of 0x449920, the colour lerp(white, colour, pct) of 0x717E40 multiplies the part's material colours. Only type 1 of 0x449920 is ported. `lirp` and `priority` are not read: of several the last created shows.
3. Grenade trails and contrails (VFXExtrusion) are not drawn.
4. **Not yet compared with the original:**
   - explosion size and duration, frame by frame;
   - acknowledgement voices;
   - ambient sounds;
   - weapon sound falloff with distance;
   - the tints and the state lists above.
5. **Menus**: `audio::menu_sound` plays `interface_music` (0x5C0B50, `UI/Menus/main.tgi` main_title_music; a NULL keeps what plays) and the UI sounds, on the user's volumes (`[B39940]`, registration indices 0x8F..0x93, now read by the battle's manager too). The interface that plays it (the reader of Interface +0xB0) is not traced, and `win_interface_music` / `loss_interface_music` are not played.

## Ground (look.rs, original.wgsl)

`detail_texture` and `macro_texture` of a Biome are NiTextureEffects (0x723E00: filter 2, wrap 3, mode 0 projected light, world parallel, one repeat per `repetition` metres; 0 means 0.8 m and 80 m, and the shipped biomes give none), attached to the terrain's quads by 0x71D6A0 while UVar TerrainExtraQuality holds. Only temperate_grassland and temperate_forest name them. The stages are 0x8A57A0's for projected lights (0x8A5030 sorts them): the first effect alone, the second ADD, saturated, then the base texture MODULATE, then the diffuse: `base * min(detail + macro, 1) * light`, the textures being grey at 0.56. Not compared with the original yet on a registered picture: one film (Berlin2v2_1, temperate_forest, night) reads a mean level within 6% and a neighbouring-pixel contrast of 5.2 against the original's 5.7, on different parts of the same map at the same zoom. A `nightonly` VFX follows 0x4EE070 (SunNightThreshold, 1.6 in v14's SVars_AA), looked at when it is made where 0x4446F0 looks every update.

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
| mod 4 | 7 | 7 | no dump | 5 of 5 | 7 of 7 | 5 | 3 (`f3bd589503` 50 of 50, `d22932af71` 26 of 26, `47206c3262` 16 of 16) |
| mod 6 | 4 | 4 | no dump | 4 of 4 | 4 of 4 | 4 | 0 (to 3 of 4) |
| mod 3 | 4 | 4 | no dump | 3 of 3 | 4 of 4 | 1 | 0 |
| mod 2 | 8 | 8 | no dump | 1 of 1 | 8 of 8 | 1 | 0 |
| mod 1 | 5 | 5 | no dump | 4 of 4 | 5 of 5 | 0 | 0 |
| `widths` | 3 | 3 | no dump | 3 of 3 | refused: the films do not decode without their data | - | - |

The chain check on the end state (FIGHT-FILMS.md's first getter method): strong on 2 films, weak on 23, refused on 18. Most films cannot say either way: too many placements in the first interval. The checkpoints above are the better oracle. 13 films' settings do not fit today's RMC (0x505B00), 8 of them on v14: natively they take the defaults and go out of sync, `SyncMode::Report` plays them on the stored settings, and there all 8 pass their first checkpoints.

Generation takes 0.6 to 4.6 s under load, `open_film` 1 to 5 s once the data set is read. The first film of a mod reads the mod: 12 to 66 s, and 590 s for mod 1 with nine lanes building.

**What the world needed besides the generator's own stream**, all found by `random_film_native_state`:

- A CityComponent exists only with a zone radius (0x45D6A0). A village's blocks have the node with radius 0 and no component, so `Def::city` is now the radius, and 0x4F6980 takes them.
- A city's PostCreate 0x564BE0 runs the layout 0x5644F0 with its list: the Structure actors it makes go on City+10 (0x5642A0) and change hands with the city. `GeneratedWorld::blocks`.
- A truck's trailer is a thing the generator makes (0x4D70F2), `Thing::Trailer` in aa-play and no actor. `GeneratedWorld::trailers`, `NativeIds::placed_trailer`.
- GThing +0x28 stays what it was at creation: the ground the later things changed is not read again. `replay_generated` puts it on the actors, and a generated map has `native_terrain`, so `height_at` is 0x4F0E50 there too. A start location takes its height at the position it leaves (0x4EEB40).
- ConfigureGWorld 0x50E8B0 destroys a kingdom with KingdomInfo +0x31 once the random settings are resolved, so a closed side draws its picks and keeps its GTID but the generator does not see it (`side05` and no `side03`).

**What the first differences are now.** On `1104a3b0f9` the truck is bit exact to tick 39, then the original's Think takes a turn recovery (`0x532B50`) that ours reaches at 41 (PARITY-CENSUS, fifth round). `ba445000bb` passes 32 of 32 checkpoints to the end of its trace since FindOpenPositionLineThenRadius `0x554BF0` is exact: it was the recruited organization's place. Neither is the generator: their lane is movement. Provided resources change one sample late at a capture (`supply::city_tick`, native 0x4D5230 does it at once); the stock does not differ.

**Left in the generator:**

- AddHeroesToKingdom 0x592E00: not ported, a world asking for heroes says so in `load_notes`. No film of the corpus has one and A&A hides the slider.
- Base terrain of mountain or water kind on a thing (0x551B40): not ported, says so in `load_notes`. No film reaches it.
- The mods with a `RandomMap` folder of their own (mod 2 7 films, mod 1 5, mod 3 3): the original's trace on mod 2's QA copy (film `2e04b717ec`: 183775 draws) now agrees with ours on every common draw, and the end state is the original's (`21db463a`). Two reads were missing: a required city's DenizenComponent with an inventory registers on queue 1 (0x568750), and `supply_required` lies on the EconomyComponent itself (0x464BD0, def +0x348; 0x56F250 registers on it), where the generator looked under a `[Resources]` child that no generated stock thing needed. The tools: `nat_mod.sh` of the lane, with `corpus_film_draws` (`AA_RMG_GATE_SETS`, `AA_RMG_GATE_TRACE`). The earlier "Breakpoint at 0x404F60" was the probe's own breakpoint on the log call, left behind when its 900 s timeout killed it during the mod's 16 minutes of loading. `(init time) main menu` is logged before the menu takes keys: wait for `connect end MainMenuInterface`. Two films of mod 4 were recorded on another data set (version RWE5.5.5.0) and cannot pass.

Note: SVars.tgi sets `RMGBalanceMarkerNudgeStepDelta`, but the game registers `RMGBalanceMarkerNudgeDelta`. The file value is ignored, so the object balancer's delta stays 4.0 and the kingdom balancer's is 2.

## Multiplayer (aa-netcode lan.rs, lan/orders.rs; client crates/aa-bevy/src/net)

What exists and how it is proven: research/phase8-net/README.txt section 13. The order of the next steps:

1. **A session with the original.** Section 13 says how far it goes today and what stopped it. Host on 127.0.0.1 with `tests/lan_original.rs`, let the QA copy join with `map <key> LobbyMenu "LobbyJoinDirectExecute 127.0.0.1 7290"` in its test profile's hotkeys. Our node never binds beyond loopback in a test (`AA_LAN_BIND`).
2. **The connect checks.** The client's `LanConfig` sends the depot string read from the archive headers (0x5AF810), the version and application strings, and the data checksum [0xB36530]+8 as a constant measured from the original on the stock data (`STOCK_DATA_CHECKSUM`). Port the function that computes it, and the mod string 0x5AF970, so that other data sets get the right refusal.
3. **The snapshot and the launch settings as the original writes them.** `GameSettings` of a session comes from `Data::header`: the name and the flags of each record, and only the fields staging needs. A native 0x5A1E90 header has the names, the pairs, the kingdom words and the file time. Native peers read all of it.
4. **Synchronizer A writers.** Without the 785 int sites, the world dump 0x4F6280 and the launch dump 0x76D6C0 (research/byte-exact/SYNC_LOG.md) an original peer's hash never matches ours, so the sync check would end a mixed session at the first report.
5. **Heartbeat 0x75B820 with the proven `crate::heartbeat`.** `LanNode::liveness` is the short rule. The real path needs the lag flags, the votes, backup mode (which stops the host's admin ticks) and DisconnectLostPlayers 0x75A2C0. Its dialog: `csd_*` strings, CSD panel. The owner of a lagging peer sees nothing today.
6. **Backup server and host migration** (0x759530, SetHost 0x757010, type 3, 6, 7 packets). When the host goes, a client ends its session.
7. **The LAN game list**: the original's comes from GameSpy's LAN browse. Only join by address exists. If it is built, it stays inside the person's own network and only when a person opens the lobby.
8. **Map and film file transfer** (`file.rs` is proven; status 2 in 0x7554B0 is refused). A client without the map has no way to get it yet, and the map check always answers "have it".
9. **Orders still refused by name** (`lan/orders.rs` `has_process`): swaps, teams, private and team chat, vote to kick, vote to resign, observer, keep playing, host ended the game, saved games.
10. **Native macro of 0x75EEB0 / 0x75F760.** The client sends what those host-only processes emit (91, 114, 115, 118). The orders 127 and 117 themselves are not sent.
11. `DefinitionSlots` (aa-parity registries.rs) could move to aa-rules so that aa-netcode stops hard-coding the 194 and 409 of `lan/registries.rs`.

## Computer players in a session (aa-play ai, aa-parity recording)

- **Where the SAI runs.** The original runs a computer player on the host, and its orders travel as decrees. Here every node runs every SAI and applies its orders locally, which works between our clients because the world is the same on each node. With an original peer the SAI's orders must become requests of the host's node and must run only there.
- **The SAI's seed.** `ai::set_seed` is never called in the client, so every node starts from seed 0. The original seeds from a clock (0x7A0AA0) on the host alone.
- **Random generals.** `Custom::launch_setup` draws a computer player's nation and general on every node from the map seed. The original draws them on the host (0x50E8B0, 0x79FAF0) and they travel.
- **MidgameCreateImplicitAIs 0x76EE10.** `Game::implicit_ai` hands the kingdom of a player who left to a computer player of the middle difficulty, with the profile the SAI picks by itself. The native path makes a new session player through 0x76EB30 and 0x76CB30, on the host only, so the name and profile follow from there.
- **Implicit AIs at launch 0x776580.** `Custom::net_fill_implicit` fills required kingdoms without a player. The record bytes +0x31, +0x32 and the phase 7 rule (every eligible kingdom) are not read.

## Water, clouds and what stands on the ground (2026-11-03; aa-bevy water.rs, look.rs, effects/)

Compared with the original on registered shots (same film, tick and camera), QA copy on a hidden desktop:

- **Water** was not drawn at all. `research/phase6-water/FINDINGS.txt` has the rules and what is open.
- **Cloud shadows** (Biome CloudInfo, the effect at Terrain +0x34, 0x723E00, 0x71E020): the original's ground shows them moving. The speed here is the middle of cloud_min_velocity and cloud_max_velocity; the original's wanders on the view's own random numbers.
- **Unpack dust**: `building_pack` loops and no data stops it. It ended with nothing in the client. What a state's WhenEnter attached now stops when the state ends and its particles fade, which reads as the original's frames do; the native call that ends it is not traced.
- **The building on a slope**: `research/phase8-ground/README.txt`, last section.
- **Shadows** use the shadow map's least filtering, for the stencil's hard edge. They are still a shadow map.
- **Refused spot**: the first ghost of a battle now takes its red tint. The original gives no message for a plain building: 0x6887C0 fires `building_invalid_placement` only for a thing with `required_object_ids`. That event is not fired yet, and no shipped building that a truck unpacks has the field.

Still open here: a truck or regiment ordered onto a building's footprint stops at its edge and stays Moving (right click on an own building gives a Move to its centre; the original's order there was not compared). Settings +0x1C on a construction order's flatten is still not measured: no QA film has one. Night lamp glow, the brown ground patch on some random seeds, animation blending, landing and hit animations, enchantment glows and contrails were not reached this round.
