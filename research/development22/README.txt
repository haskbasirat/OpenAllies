AXIS & ALLIES RUST - DEVELOPMENT 22 (BEVY) - TEST BUILD
DIAGNOSTIC / UNFINISHED

WHAT THIS IS
The Rust recreation of the original Axis & Allies, drawn with Bevy, using
the original menus, HUD, maps, models, animations, sounds and rules from
your installed game. It follows the original game's code and data closely,
and more of it becomes byte-for-byte identical with each build.

NEW SINCE DEVELOPMENT 21
- Films. Main menu -> Film, or the Film button in staging. The list shows the
  films this game recorded and the films your original game recorded (those
  are read-only here). Pick one, Load, click Continue on the loading screen,
  and watch.
    + and -        speed (the film range, as in the original)
    0              see everything, no fog
    1 to 9, q, w   see the battle through one side: its fog, resources and
                   supply. The binocular buttons do the same.
    Game menu      End Film
  At the end you get End of Film (Exit to Scoring or Keep Watching), then the
  score screen with Replay.
- Your battles now record a film by themselves, like the original (LastFilm).
  Save Film on the score or debriefing screen keeps one under your own name.
  The original game has opened films recorded this way.
- How exact a replay is: short random-map films now play to the end with every
  checkpoint matching the original. Longer films drift partway through, and
  the units then stop doing what really happened. When that happens the game
  says so in the message log and shows Out of Sync at the end, and keeps
  playing so you can still watch. Berlin stays exact for its first 93 of 266
  checkpoints, about a third of the film.
- Films that need a mod's data play on that data. Their art has to be converted
  first, and until then only stock art is drawn. Films of a saved game start
  from the save's world but drift within moments.
- Random maps are now the original's, exactly: terrain, heights, trees, rocks,
  villages and start positions were equal on all 12 maps compared. This holds
  for the standard data, not yet for mods with their own random map folder.
- Campaign missions: the mission list works and your progress is saved. Scripts
  run, scripted cameras and letterbox work, and movies play. A mission ends at
  the debriefing with Score, Replay and Next Mission. In Tutorial 1 the infantry
  regiment has a white banner and is not yours at first, as in the map's script:
  select it, wait about 24 seconds until it is handed to you, then move it. Not
  every mission has been played through yet.
- World War II: a battle can now be fought rather than only Quick Resolved, and
  returns to the map. Also new: saving and loading a campaign, help popups,
  tooltips, upgrade buttons, a minimap and the army's portrait. To buy an army,
  press the small square at the left of its row; this has been tested only as
  far as the confirm box.
- The score screen shows real numbers and a graph.
- Units follow the original's own logic much more closely: regiments walk the
  original's routes, formations and groups behave like the original's, and
  orders like guard, retreat and explore work. Battles can play out differently
  from Development 21.
- Right click on an enemy now walks to it and attacks, as in the original. This
  replaces the following described under Development 18. A plain move mostly
  stays in a fight: use Retreat to leave one.
- Combat: damage now reads technologies and terrain, grenades hurt buildings,
  and fighting starts when an enemy enters a unit's zone of control. Morale
  follows what the regiment is doing. Results are close to the original's, not
  yet the same.
- Fog and supply areas come from the original's own rules for every building,
  regiment and city. Corps HQs and bunkers now capture and hold villages.
- The computer plays closer to the original: its thinking, recruiting and
  general powers follow the original's code. Its early army is still smaller.
- The look: the original's lighting, sun and shadows, the start view on your
  own start location, the camera at your saved zoom, and map colours fixed on
  developer and campaign maps.
- "You have captured" a village in the first seconds of a battle is right: the
  original does the same.

NEW SINCE DEVELOPMENT 20
- Box selection works like the original: a box now also takes ammo
  trucks, supply drop trucks and other vehicles, together with regiments.
  It selects up to 6 objects, keeping the ones nearest the middle, and a
  soldier inside the box selects his whole regiment once. Buildings are
  only taken when the box holds no units. Shift or Ctrl adds to the
  selection, and a double click adds the same kind.
- Engineers: a Construct button shows a see-through building that follows
  the mouse (drag to turn it), as Unpack does. Several engineers build
  together. Right click on one of your damaged or unfinished buildings
  with engineers selected and they repair it.
- Mortars and artillery show the original's "cannot bombard" cursor over
  ground that is out of range.
- Dead soldiers sink into the ground before they disappear, like the
  original.
- Random maps come out closer to the original: for the same settings the
  terrain, the start positions, the required objects and the villages
  are placed exactly where the original puts them. Doodads and heights
  still differ.
- Units move on the original's own timing more closely: a recorded game
  of the original stays identical for longer before the first difference.
- Airfields and carriers now keep their aircraft in hangars, and the
  original's aircraft orders and flight paths are being ported. Air
  attacks and air recon are still not available in this build.

NEW SINCE DEVELOPMENT 19
- Byte-for-byte work has started: the random numbers, the game clock, unit
  numbering, fog of war, supply areas, the economy, the production queue,
  research, pathfinding and formations now run the original game's exact
  code, on the original's own timing. A recorded game of the original
  (Berlin) replays identically in this build for its first 28 seconds.
  Battles can play out a little differently from Development 19: closer
  to the original.
- Recruiting follows the original's rules: units that need a building or
  a technology are disabled until you have it, and their tooltip lists
  what is required. Pressing a recruit you cannot afford shows the
  original's "Insufficient funds" message.
- Buildings with research items have a Research page, like the original.
- The minimap flashes your HQs red while they are under attack and green
  while they are out of supply.
- The original's messages and sounds for events: recruited, lost,
  captured, construction complete, promoted, research complete, upkeep
  shortage and more.
- Every side starts with all its resources at the original's starting
  value; neutral cities give no supply until they are captured.

NEW SINCE DEVELOPMENT 18
- Right click on any unit (yours or the enemy's) and the selected units
  keep following it, like the original. Right click on the ground still
  moves there; attack with the Attack button, as in the original.
- Buildings rise out of the ground when they unpack: the parts that come
  up are no longer visible from the start. Units behind hills are now
  hidden by the hill.
- The supply border is drawn on the ground in the 3D world like the
  original, and on the minimap as the original's dotted green line.
- Captured cities show only their flag on the minimap, like the original.

NEW SINCE DEVELOPMENT 17
- Tanks aim: turrets turn toward their target at the original turn speed
  (heavy tanks slowest) and fire only once lined up; idle turrets turn
  back forward after a while. Shells leave from the barrel and land
  scattered around the target like the original.
- Recruiting like the original: a new regiment appears with its leader,
  then the other soldiers come out of the HQ door one at a time and walk
  to their places. Wounded soldiers heal one at a time.
- Unpack shows a see-through preview of the building that follows the
  mouse (red where it cannot go); drag to turn it. Camps now face the
  same way as the Corps HQ unless you turn them.
- A move order shows the original green arrows on the ground, and selected
  units show a flag at their destination.
- Right click on the minimap sends the selected units there.
- The minimap shows the original icons for HQs, flags and other objects,
  standing upright, in the original colours.
- An HQ with nothing queued no longer shows a progress line.

NEW SINCE DEVELOPMENT 16
- The minimap fog of war looks like the original: explored ground is a
  lighter grey, the edges of what your units see are soft, and the green
  supply border follows the ground instead of a perfect circle.
- Supply areas spread like the original: they stop at water, mountains and
  other impassable ground, so a unit can be out of supply across a river
  even when it is close to its HQ.

NEW SINCE DEVELOPMENT 15
- Infantry move and fight like the original: they walk or jog by speed,
  kneel when engaged (machine guns set up on their tripods), go prone when
  hurt, fidget when idle and use the original death animations. Mortar
  teams walk instead of sliding. Kneeling soldiers move 20% slower and
  prone ones 60% slower, as in the original rules.
- Trucks drive like the original: they turn on arcs instead of on the
  spot, keep their facing when they deploy, wait behind slower trucks, and
  lean with the ground on slopes (tanks and trailers too).
- Wheels and tank treads turn, trucks tow their trailers, and buildings
  play their packing, unpacking, damaged and destroyed looks.
- Snow and desert maps use the original Arctic and Desert textures
  (for example Stalingrad).
- Recruiting: the Corps HQ and other buildings show what they are building
  with its progress bar and the waiting queue (click an item to cancel
  it). Division HQs can be selected and recruit their regiments; queued
  regiments show in their slots with their progress.
- Cities show their names on the map and in the panel, and captures are
  announced.
- Game speed can be changed during a battle (Options -> Game Speed, or the
  + and - keys).
- The Custom button and the start of a battle no longer freeze.

HOW TO PLAY
1. Run Start.cmd. The first start of a new build can take a few minutes while
   it reads your game's data.
2. Main menu -> Single Player -> Custom for a battle, Campaign for missions,
   World War II for the strategic game, or Film from the main menu to watch a
   recording.
3. Random Map (map type, size, sides, terrain) or Scenario Map, set the
   players, nations, teams and computer generals, then Launch.
4. Click Continue on the loading screen when it has finished.
5. Left click selects, drag selects several, double click selects the same
   kind; right click moves, right click an enemy attacks. Select a building
   to recruit with the buttons on the right. Use your general's abilities
   with the buttons on the left. Arrow keys or the screen edges scroll,
   the mouse wheel zooms.

STILL MISSING
- Multiplayer, the editor, and saved games for battles. The Save button in a
  battle's game menu does nothing yet.
- Air attacks and air recon.
- Right-drag to set the facing of a move.
- Exact replays of long films (see above), and of most films with real fighting.
- Selecting several units shows only the first one in the unit panel;
  the original shows a panel with each of them.
- World War II: the selected army's arrow ribbon does not appear, the purchase
  rows' icons are empty, and the politics display and a few tooltips are not
  built.
- Scripted missions: weather and team flare markers are not shown.
- The ground's detail and macro textures, so ground looks smoother than the
  original on temperate maps.
- Menus have no sound or music yet.
- Some building sounds and effects: packing, unpacking and construction
  sounds and dust, and the red pulse of a building under attack.
- Animations snap from one to the next instead of blending; the landing
  and hit animations are not played.
- Some ability visuals (glows of enchantments, contrails), the radar blip
  on the minimap.
- The computer's early army is still a little smaller than the original's.

REQUIREMENTS
- Windows 10 or 11, 64-bit, Vulkan or DirectX 12 graphics driver.
- Your installed original Axis & Allies at <game root> with the
  reconstruction folder beside rust-rebuild, as for earlier builds.
- Keep AxisAlliesBevy.exe and the assets-cache folder together.

Earlier Development builds are preserved unchanged in release\.
