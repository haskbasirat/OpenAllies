# Scripted missions and the World War II campaign

What the client does of the original's missions and campaign, and what it does not
yet. Owner: the missions lane. Sources: pseudocode of AA.exe, cited by address in
the code's doc comments.

## Scripted missions

The triggers run on the exact aa-triggers port over aa-play (`aa-play/src/triggers.rs`,
its module doc lists every seam). Readers:

| Native | Where | State |
|---|---|---|
| Kingdom+0x20 SAI off (62 / 79E2F0), unit SAI off | `ai.rs` skips them | built, test `a_kingdom_the_script_disabled_does_not_think` |
| Kingdom+238 default victory (0x4E9960) | `victory.rs` team_check | built, test `default_victory_off_keeps_an_eliminated_kingdom_in_the_game` |
| 0x633A50 objectives | `ui/panels.rs` refresh_objectives | built |
| 4F3FC0 / 54C170 the script hears UI commands | `ui/objectives.rs` (native_text) | built, test `the_tutorial_goes_on_when_the_script_hears_its_command` |
| *(GameVars+104) difficulty | `triggers::set_difficulty_ids`, chooser dropdown | built |
| MoveCamera, CameraFollow, ShakeCamera, letterbox, fade, cutscene speed | `ui/objectives.rs`, `camera.rs` | built, hidden-desktop runs |
| PlayVideo (545C60 -> 0x411070) | `campaign/cinematic.rs` | built |
| 543BA0 transported creation | sim: a timed drop; view: `effects` trigger_transport | labelled rendition, no transport actor in the sim |
| 4C9F10 trigger abilities | `special_ops::trigger_use`, with the FX event 4C8500; axis08's `v_weapons` is a general's, and the note "not modelled" came from test worlds without the generals' data | built |
| 4D7960 object_destroyed | `combat::script_kill`: the damage record stands, so the Destroyed event names the last attacker (54BE10); element 3 asks for one, and Tutorial 1's T 02.070 waits on it | built |
| 0x725340 -> 0x7389B0 the weather (World+290) | `effects/weather.rs` | labelled rendition: block states from the view, no wind |
| 4F1E10 first statistics snapshot | `statistics::start_snapshots` | built, test `the_score_starts_with_a_snapshot` |

Not presented, logged once when they happen: a VFX on an actor (no shipped use), team
command flares' markers (10 uses; the flare's actor, with its own model, has no simulation
counterpart). A trigger's Music(None) stops what the script started (747630).

The flash of a UI button (50C880) reaches every widget that sends the command, so the
formation buttons (`FormationChange 0 %i`), the floater's `ShowFloater 2` and the ability's
button flash as the action panel's do.

Music after a battle: `victory_music` and `defeat_music` are `null` and 747630 of NULL does
nothing; the game's music is LOCAL, so the world's end (0x43E400 -> 0x7459F0) leaves it; it
goes on behind the score, the debriefing, staging and the chooser until the title music or the
next game's replaces it (`audio::CarriedMusic`, labelled: it starts its track again).
`win_interface_music` and `loss_interface_music` are defined in `Audio/music_aa.tgi`, but no
data file and no code names them: the original never plays them.

A click on a soldier during a scripted camera pan (a question left open last round): a click
that follows the actor (`"track": true`) selects it, one aimed at where the actor was two
frames before misses, as a person's hand on a moving target would. It is the script's stale
position, not the pick. One sample each, on a loaded machine.

A generated world has no map file and so no TRIGGERS chunk: `triggers::tick` no longer
looks for one, which opened the whole archive for 1.5 s on the first tick of a random-map
battle.

Seams still labelled in the module doc of `triggers.rs`: the Area member iteration walks
GTID order instead of the spatial query's order, 554B30 uses `special_ops::spot_open`
instead of the spatial index port. They matter to a film recorded on a scripted map, not
to playing one.

## The scripted campaigns

`campaign/missions.rs` is ChooseCampaign (0x642280). `KKC_CampaignState` is
`CampaignState`, in the original's file format: `<profile>/data/CampaignSave/<ids>.rcs`
(0x46F7F0 writes: u32 0, u16 length and UTF-16 IDS, u32 +4, per mission u32 0 and the
state 0 open / 1 locked / 2 complete, u32 difficulty index; checked against files the
original wrote, unit test `the_record_reads_and_writes_the_originals_file`). The
client reads and writes its own profile's copy, never the original's.

The end of a mission (0x60A140, 0x651BD0): the debriefing (`campaign/debriefing.rs`,
0x6014B0 / 0x600D40), the chooser after a won final mission. Scoring, Replay, Play Next,
Exit, Save Film as the original's commands. Leaving a mission asks `campaign_end_confirm`.

Cinematic missions and the trigger's PlayVideo: `campaign/cinematic.rs`, from the
flipbooks of `research/phase8-ui/bake_campaign_video.py`; without a bake the movie ends at
once, as its end does (0x641400). The bake is a local conversion of the original's movies
and stays out of the repository (`assets-cache/`). The movies are in Video.rwd; the MP4s it
reads are the reconstruction's.

Refused: `CampaignLoadGame` (0x68DB60 mode 4: aa-play has no saved game).

The actor states a script tests (`actor_has_state`, names 0x477FC0) were answered for 3 of
42: `Host::state_bits` now reads engaged, routing, retreating, moving, attacking, exhausted,
entrenched, dead, packing, unpacking, under construction, out of supply and attached to an
HQ from aa-play. The rest stay off (no shipped script reads them).

### Tutorial 1, played as a person

A scripted client run plays the whole of it from the move to the supply drop and wins: Mission Won,
End Mission, the debriefing with every objective ticked (hidden desktop, at eight times the speed).
Next Mission starts Tutorial 2. It stood still twice on engine causes, both found by following the
script's waits:

- T 02.070 Germans Destroyed: its event (element 3) needs a killer. `object_destroyed` (4D7960)
  went through the sourceless zero-health pass, which clears the damage record. Now
  `combat::script_kill`; 4D57C0's KillCreditTimeout applies to the event's attacker.
- T 03.130, T 05.060 "finishes construction" of what a truck unpacks: the end of the unpacking
  raises 54C050 itself (0x57634D, in code the decompiled listing cuts off at a function boundary),
  not through 565B00. `Event::Deployed` now queues it.

### How far each mission's script goes

`mission_person_census` with `AA_TRIGGER_WALK=1`, 300 game seconds each: what the script asks of the
fight simply happens, so this measures the script and not the battle. Won within the time: allies01,
allies03, allies05, axis02, axis04, axis08, axis09, axis10, axis11. Still running at 300 s with
objectives open, waiting on captures or areas (no script stop): allies02, allies04, axis01, axis06,
axis07, axis12, the tutorials (their buttons and placements need a person; the client run above plays
Tutorial 1). axis03 reads lost at 28 s and axis05 won at 1 s: artefacts of the walk, which kills what
the script names, there the side's own HQs and the enemy's whole kingdom. allies06 to allies12 did not
finish in this round's time on a loaded machine. No mission stopped on a script failure.

### How far each mission gets with computers on every side

`mission_progress_census`, 600 game seconds each, on the state of 2026-11-02 before the actor
states above: every mission runs without a stop in the script. These are not a person's
games: a computer player does not answer a mission's script.

| Mission | Objectives at 600 s, all open unless said |
|---|---|
| allies01 El Alamein | 3 |
| allies02 Guadalcanal | 2 |
| allies03 Stalingrad | 4 |
| allies04 Kursk | 3 |
| allies05 Monte Cassino | 2 |
| allies06 Normandy | 5 |
| allies07 Saint Lo | 3 |
| allies08 Market Garden | 1 |
| allies09 Bulge | 1 |
| allies10 Berlin | 1 |
| allies11 Iwo Jima | 4 |
| allies12 Okinawa | 3 |
| axis01 Crete | 4 |
| axis02 El Alamein | 2 |
| axis03 Suez Canal | 6, ended at 431 s: the local kingdom lost |
| axis04 Stalingrad | 3 |
| axis05 Australia | 1 |
| axis06 Kursk | 1 |
| axis07 Normandy | 6 |
| axis08 Sea Lion | 3 (the test world has no generals' data, so its rockets were noted as not modelled; the client has them) |
| axis09 Burma | 1 |
| axis10 Moscow | 3 |
| axis11 Midway | 3 |
| axis12 Hawaii | 5 |

## World War II

`campaign/` over `aa_metagame::driver`. The tooltip of the Play button is 94E920 (`interface.rs` `rts_tip`, 63C6B0's rows over
`launch::battle_review`). The territories' View takes `territory_focus_vfx` under the pointer,
`territory_select_vfx` when selected and, with UVar MetaGameViewShowVFXMovement (0x324, off by
default), the friendly or enemy tint on the selected army's reach (933840, 93ABF0). The drag of an
army shows its ghost (956F40, 693AE0), red where it could not go (9571D0, of which only no
territory, out of range and another battle are read). A campaign battle's additional actors are
placed by the generator (KKC_RMCPlaceWorldParametersActorsStage 0x594930,
`RandomMapParams::additional_actors`), a random film's from its kingdom rows.

Built: the chooser, the map with its props,
territories, armies, the interface (control panel, command buttons, player display,
message log, turn label), the menu, Start Battle with its army lists, purchasing,
history, upgrades, the object display's lists and portrait, the minimap rendition, saves
(`saves.rs`: TGCK with METAGAME, armies in AA_ARMIES: AA.exe cannot read them, and the
original's own saves are not read yet), the RTS launch and its return (`battle.rs`), the
help popups (`help.rs`, 94A5C0 with the triggers of 933C50, 9331A0, 94F2F0), the tooltips
of the army rows (94DFE0 at Start Battle, 94BC50 at purchasing), the technology rows (94BE50)
and Quick Battle (`interface.rs`, the builder 638690 as `TipBody`; the column x of 638B70 is
the AVar UITabWidth, `[B39990]+0x158` = index 0x56, registered at 0x403B1E, times 5C1DB0),
anchored as the interface's data says (5F2D60, `ui/tooltip.rs`), the territory sounds of
953A20, the double click, the drag, the arrows
of the selected army (`map.rs`, ribbon as a labelled rendition), the politics display
(932930, 950290).

A purchase row (965830, 9655C0): only the 33 by 33 PushItem is a button, the row's name is a
plain label, so clicking the name does nothing; the icon is the Decal under the PushItem
(964F90), the Thing's +0xF0 or the Property's `action_button_icon` (+0x30); a technology the
kingdom owns has its row disabled (959FF0).

Not built, by what a player sees first:
1. CycleSelection (933DB0: no shipped hotkey binds it in the MetaGame context).
2. The original's saved campaigns (their armies are in DYNAMIC_THINGS). No sample file from
   the original exists to test against.

## Testing

Hidden-desktop scripts: `AA_UI_SCRIPT` steps; the step `{"mission_result": 1}` settles the
local kingdom's result so the mission's end can be checked without playing it. Main menu
click points (1600x1200): Single Player 0.878,0.306; then 0.878,0.306 is the scripted
campaigns, 0.878,0.432 World War II; a campaign's Play 0.5,0.958; the World War II chooser's
New 0.505,0.96; the loading page's Continue 0.871,0.928; the help popup's Close 0.64,0.613.

Following a mission's script from a test: `{"until_listen": "<command>"}`,
`{"until_trigger": "<name>", "on": false}`, the actor kind `slot:<GTID slot>`,
`{"click_command": "<command>"}`, `{"rclick_world": [x, y]}`, `{"destroy": "enemy"}`.
`cargo test -p aa-play --test triggers -- --ignored --nocapture mission_progress_census` plays
every mission with computer players on all sides and prints how far each gets.
`mission_person_census` follows what the enabled triggers wait on: it selects what the script
asks, sends the UI commands it listens to, walks its regiments into the areas it watches, attacks
what it waits to see dead, recruits what it counts. With `AA_TRIGGER_WALK=1` what the script asks
of the fight simply happens, which shows how far the script goes whatever the battle does.
`def:<IDS>` and `name:<name>` are further actor kinds of the UI script, `"track": true` keeps a
click on its actor.

The original cannot be compared on the campaign map on a hidden desktop: posted mouse
clicks do not reach AA.exe there and no shipped hotkey selects an army.
