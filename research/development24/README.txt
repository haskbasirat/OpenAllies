AXIS & ALLIES RUST - DEVELOPMENT 24 (BEVY) - TEST BUILD
DIAGNOSTIC / UNFINISHED

WHAT THIS IS
The Rust recreation of the original Axis & Allies, drawn with Bevy, using
the original menus, HUD, maps, models, animations, sounds and rules from
your installed game. It follows the original game's code and data closely,
and more of it becomes byte-for-byte identical with each build.

NEW SINCE DEVELOPMENT 23
- Multiplayer you can really use: LAN with a list of games, Join Direct by IP address
  or host name, and online play through Epic Online Services from the Internet
  button. See MULTIPLAYER below for how to try each one.
- Watching a film no longer crashes partway through. Three films that stopped with an
  internal error now play to their end, and a film can be watched past its end.
- Deploy to Location: an Airborne HQ has the original's Deploy to Location button
  (hotkey L). Pick a regiment that comes by plane, then click open ground: a transport
  flies in, the paratroopers come down under parachutes, and the plane flies away.
  The ordinary Recruit page no longer lists the plane-dropped regiments, as in the
  original.
- Engineers sent to build somewhere away from where they stand now walk there and
  build it to full health. Before, the foundation stayed at 1 health.
- A truck no longer unpacks where the original refuses: the building needs 2 m of
  free ground around it (no water, impassable ground or map objects). A regiment's
  truck tries twice, then gives up and stands.
- Units no longer stay "Moving" without walking. In a test with about 430 orders on
  each of three maps, nobody stood still on a move.
- You can play against the computer properly now. In test games it bought its HQ at
  25 s, recruited, unpacked, took villages from about 2 to 5 minutes in, attacked,
  routed and regrouped, used its general's powers, and beat a player who did nothing.
- HQ banners show the HQ's own badge (the British one is the gold round), rivers and
  seas use the original's blending, and Tutorial 1 can be won, with Next Mission
  starting Tutorial 2.
- The Berlin 2v2 film is now exact for its first 140 of 266 checkpoints (it was 103).

NEW SINCE DEVELOPMENT 22
- Buildings sit on the ground like the original. When a truck unpacks, the ground
  under the building is flattened and blends out to the slope around it, and the
  trees, rocks and grass under it go. Buildings sit on the ground on gentle slopes.
  On a steep hillside one side of a building can still sink into the slope; whether
  the original does the same has not been compared yet.
- Right-drag a move to set which way the units face when they arrive.
- A right click is now the original's order, and each unit works out what it means.
  A left click on a soldier in front of a building's box selects the soldier.
- Selecting several units shows the original's group panel.
  These three are new and the last test could not reach them by clicking, so please
  say if they misbehave.
- Temperate ground has its detail textures, and menus have music and button sounds.
- Multiplayer: LAN between two copies of this game (see MULTIPLAYER below).
- The computer gives orders like the original's: it guards, retreats, explores, moves
  in groups and changes formation. Its decisions are still not the original's.
- Fights run on the original's own unit logic. Results are close but not the same,
  and battles can play out differently from Development 22.
- World War II: buying an army works (press the small square at the left of its row),
  rows have icons and tooltips, the selected army shows its arrow, and there is a
  politics display.
- Tutorial 1 goes on through its first fight, and its hint text is right.
- Real films stay exact for longer. One of your Market Garden games matches to its
  end, and one Berlin game stays exact for about 3.5 minutes. Most long games still
  drift. The Berlin 2v2 film is exact for its first 103 of 266 checkpoints, just
  under 2 minutes.

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
- How exact a replay is: films recorded by this build replayed exactly in testing
  (240 of 240 checkpoints with real fighting). Films from the original are a
  harder test. A few short random-map films play to the end with every
  checkpoint matching, but most films drift, some of them early, and the units
  then stop doing what really happened. When that happens the game says so in
  the message log and shows Out of Sync at the end, and keeps playing so you can
  still watch. Berlin stays exact for its first 93 of 266 checkpoints, about a
  third of the film.
- A film can open with the view on bare ground: click the minimap. A film from
  another version of the game, or one that needs a mod's data it cannot find,
  shows a message and returns to the list.
- Films that need a mod's data play on that data. Their art has to be converted
  first, and until then only stock art is drawn. Films of a saved game start
  from the save's world but drift within moments.
- Random maps are now the original's, exactly: terrain, heights, trees, rocks,
  villages and start positions were equal on all 12 maps compared. This holds
  for the standard data, not yet for mods with their own random map folder.
- Campaign missions: the mission list works and your progress is saved. Scripts
  run, scripted cameras and letterbox work, and movies play. A mission ends at
  the debriefing with Score, Replay and Next Mission. In Tutorial 1, nothing
  starts until you open the flashing Message Log (F3 or its button) and close it.
  The infantry regiment has a white banner and is not yours at first, as in the
  map's script: click the soldiers on the road, wait about 25 seconds until it is
  handed to you, then right click open ground to move it. Not
  every mission has been played through yet. The opening movies cannot be skipped.
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
1. Run Start.cmd.
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
   the mouse wheel zooms. If a click near a building selects the building, click
   the regiment's banner instead. To place a truck's building, click the ground: if
   nothing happens the spot is not allowed, even if the preview is not red.
6. To leave a battle: Esc, Abort Mission, Ok.

MULTIPLAYER
Your nickname: the first time you open the LAN or Internet lobby the game asks for it.

LAN (two PCs on the same network)
1. Both players: Multiplayer, LAN, and give a nickname.
2. The host presses Create, may set a game name and a password, and presses Create.
3. The other player presses Refresh, selects the host's game and presses Join. If the
   list stays empty, use Join Direct instead and type the host's address (the IPv4
   address that ipconfig shows on the host's PC, or its computer name). Port 7290.
4. In staging the host picks the map and can add computer players from an open seat's
   list. The other player presses Ready, and the host presses Launch.

DIRECT ADDRESS (over the Internet, without Epic)
- The host forwards UDP port 7290 on their router to their PC, allows the game through
  Windows Firewall, and presses Create as above.
- The other player uses Join Direct with the host's public IP address (or a name that
  points to it), port 7290. The joining player needs nothing forwarded.
- A countdown of 60 seconds runs while joining. "Connection to game failed" means the
  port is not reaching the host's game.

ONLINE (Epic Online Services: no port forwarding, no Epic account)
1. Both players: Multiplayer, Internet. The game signs in to Epic with an anonymous
   ID for your PC and your nickname. The first sign-in takes a few seconds.
2. The host presses Create. The other player sees the game in the list, selects it
   and presses Join. Then staging, Ready and Launch as on a LAN.
- Online play needs EOSSDK-Win64-Shipping.dll and the eos folder beside the program;
  they are in this build. EPIC-ONLINE-SERVICES-NOTICE.txt is the notice about Epic's
  software that Epic's agreement asks for.

TRYING IT ON ONE PC
- Run Start.cmd for the first player, and Start-Second-Player.cmd for the second.
  The second player has its own nickname and settings, kept in the second-player
  folder of this build.
- LAN: the first copy presses Create; the second presses Join Direct, types 127.0.0.1
  and keeps port 7290.
- Online: start the first copy and press Internet FIRST, then start the second copy
  and press Internet. Epic keeps one online player per Windows user; the second copy
  makes itself a new one when it starts, and the first copy keeps the one it already
  signed in with. If the second copy is started first, the two copies are the same
  player and cannot see each other's games.

WHAT WAS AND WAS NOT TESTED
- LAN and Join Direct: a full battle with a computer player, to its end, between two
  copies on one PC, over the PC's own LAN address, in sync for the whole battle. Both
  films of that battle replay with every checkpoint matching.
- Online: two copies on one PC signed in to Epic's live service, hosted, found and
  joined a game, launched it, and stayed in sync.
- This build itself, over LAN and online: each player recruited an HQ regiment and
  moved it, both screens showed both regiments walking to the same places, and the two
  worlds stayed identical on every tick (935 ticks online, 943 over LAN).
- Nothing has been tried between two PCs, through a router, or over the Internet yet.
  That is what this build is for: please say what happens.
- Known problems: maps are not transferred, so both players need the map (the stock
  maps are in every install). There is no host migration: when the host leaves, the
  game ends for everyone. The LAN list shows each game as one line of text; the online
  list shows the map's file name and no ping. A crashed host's online game can stay in
  the list for a while, and joining it then times out. The host can launch before the
  other player presses Ready. Playing against the original 2004 game is not supported.

STILL MISSING
- The editor, and saved games for battles (Save and Load are grey in the game menu).
- Multiplayer: map transfer, host migration, the connection-status dialog, and play
  with the original game.
- Air attacks and air recon.
- Exact replays of long films, and of most films with real fighting.
- World War II: reading the original's saved campaigns, and some tooltips.
- Scripted missions: weather and team flare markers are not shown.
- Unpack dust runs at the wrong time, and building shadows on the ground are softer
  than the original's hard-edged ones. Construction dust and the parachute closing
  effect are not shown.
- A move ordered into a lake, onto impassable ground or a city block stops short on
  the near side. A move to a point on a cliff can leave regiments where they are.
- An HQ can still unpack closer than 2 m to another building.
- Films recorded with a mod can fail to open ("unknown CreateGWorld failure"), and
  films from a saved game say "Out of Sync with Film" at the start, then play on.
- Animations snap from one to the next instead of blending; the landing
  and hit animations are not played.
- Some ability visuals (glows of enchantments, contrails).
- Some computer generals start slowly, and the computer's early army is still a
  little smaller than the original's.

REQUIREMENTS
- Windows 10 or 11, 64-bit, Vulkan or DirectX 12 graphics driver.
- Your installed original Axis & Allies at <game root> with the
  reconstruction folder beside rust-rebuild, as for earlier builds.
- Keep AxisAlliesBevy.exe, EOSSDK-Win64-Shipping.dll and the assets-cache and eos
  folders together.
- Multiplayer: Windows may ask to allow the game through the firewall. Allow it.

Earlier Development builds are preserved unchanged in release\.
