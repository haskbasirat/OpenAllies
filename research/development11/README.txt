AXIS & ALLIES RUST - DEVELOPMENT 11
DIAGNOSTIC / UNFINISHED
BATTLE STATUS: NOT PLAYABLE YET

The battlefield now loads quickly and is drawn, but HQs, units and the
in-game HUD do not appear yet, and you cannot play a battle. This build is
for testing how far the faithful port of the original game has come.

WHAT IS NEW SINCE DEVELOPMENT 10
- Much faster startup. Berlin reaches its battlefield about 25 seconds after
  you click Start (it took over 16 minutes earlier in Development 10).
  Firefight takes about 20 seconds.
- Every one of the game's 1,959 rules definitions now loads (133 failed at
  the start of Development 10).
- Every object view is created: 7,970 on Berlin, 2,361 on Firefight.
- The loading screen shows the real map title and briefing text.
- All 1,900 original 3D models load through a port of the original model
  reader, including the older file versions.
- Ported and tested against the original code, but not visible yet because
  startup stops before the game screen: camera controls, unit selection and
  orders, the in-game HUD and its Ready button, the battle clock (economy,
  unit minds, triggers, sun), terrain visibility for mouse clicks.

HOW TO TEST
1. Run Start.cmd.
2. In the main menu choose Single Player, then Custom.
3. In "Skirmish startup preview" pick a map. "Fall of Berlin" and
   "DeveloperMaps/(3v3)Firefight" are the two tested maps. Type any player
   name and click "Test skirmish startup".
4. The original loading screen appears with the map briefing. Within about
   half a minute the battlefield terrain is drawn around it.
5. Startup then stops. The panel at the top left names the next piece of the
   original game that is not ported yet:
   - Berlin: "5A5410 actual Sequence root owner is required"
     (animation sequence files, being ported now).
   - Firefight: "87AA10 has-skin unique texture/clone path requires concrete
     skin graph" (skinned models, being ported now).
   This stop is expected in this build; it is not a crash.
6. While that screen is up, the arrow keys, Page Up/Page Down and the mouse at
   the screen edges move the ported camera over the battlefield you can see
   around the loading screen.
7. Use "Main menu" in the panel to go back, or close the window.

DIAGNOSTIC LOOK-AHEAD
"Start Diagnostic Look-Ahead.cmd" keeps loading past unported pieces instead
of stopping at the first one, and writes what it found to
%TEMP%\AxisAlliesRust-startup-census.json. It shows how far startup can get;
the game state it reaches is not correct, so use it only for testing.

REQUIREMENTS
- Windows 10 or 11, 64-bit, with a DirectX 12 or Vulkan graphics driver and
  the x64 MSVC runtime.
- Your installed original Axis & Allies at E:\Games\Axis and Alies, with the
  reconstruction folder beside rust-rebuild, as for earlier builds. This is
  not a standalone game.
- Keep AxisAlliesRust.exe and the assets-cache folder together.

Earlier Development packages are preserved unchanged in release\.
