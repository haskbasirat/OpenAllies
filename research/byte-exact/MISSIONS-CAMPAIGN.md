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
| 4C9F10 trigger abilities | `special_ops::trigger_use`, with the FX event 4C8500 | built |
| 4F1E10 first statistics snapshot | `statistics::start_snapshots` | built, test `the_score_starts_with_a_snapshot` |

Not presented, logged once when they happen: the weather (2 uses: the client has no
weather at all), a VFX on an actor (no shipped use), team command flares' markers (10
uses; the flare's actor, with its own model, has no simulation counterpart).

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

## World War II

`campaign/` over `aa_metagame::driver`. Built: the chooser, the map with its props,
territories, armies, the interface (control panel, command buttons, player display,
message log, turn label), the menu, Start Battle with its army lists, purchasing,
history, upgrades, the object display's lists and portrait, the minimap rendition, saves
(`saves.rs`: TGCK with METAGAME, armies in AA_ARMIES: AA.exe cannot read them, and the
original's own saves are not read yet), the RTS launch and its return (`battle.rs`), the
help popups (`help.rs`, 94A5C0 with the triggers of 933C50, 9331A0, 94F2F0), the tooltips
of the army rows (Start Battle and purchasing) and of Quick Battle (`interface.rs`; its
column x is 100 reference pixels until the AVar at [B39990]+158 is identified), the
territory sounds of 953A20,
the double click, the drag (labelled: no ghost model, no cursor states), the arrows of the
selected army (`map.rs`, ribbon as a labelled rendition).

Not built, by what a player sees first:
1. The tooltip of the Play button (94E920), the technology rows' icons and tooltips (94CAE0).
2. The politics display. The territory focus and selection tints.
3. CycleSelection (933DB0: no shipped hotkey binds it in the MetaGame context).
4. The additional actors of a campaign battle are placed by rings around the start
   location, not by `KKC_WorldCreatorRandom`'s `_ki.additional_actors`.
5. The original's saved campaigns (their armies are in DYNAMIC_THINGS).

## Testing

Hidden-desktop scripts: `AA_UI_SCRIPT` steps; the step `{"mission_result": 1}` settles the
local kingdom's result so the mission's end can be checked without playing it. Main menu
click points (1600x1200): Single Player 0.878,0.306; then 0.878,0.306 is the scripted
campaigns, 0.878,0.432 World War II; a campaign's Play 0.5,0.958; the World War II chooser's
New 0.505,0.96; the loading page's Continue 0.871,0.928; the help popup's Close 0.64,0.613.

The original cannot be compared on the campaign map on a hidden desktop: posted mouse
clicks do not reach AA.exe there and no shipped hotkey selects an army.
