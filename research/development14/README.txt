AXIS & ALLIES RUST - DEVELOPMENT 14 (BEVY) - TEST BUILD
DIAGNOSTIC / UNFINISHED

WHAT THIS IS
The Rust recreation of the original Axis & Allies, drawn with Bevy, using
the original menus, HUD, maps, models, animations, sounds and rules from
your installed game. It follows the original game's code and data closely
(not byte for byte).

NEW SINCE DEVELOPMENT 13
- Fog of war like the original (Explored by default): unseen enemies are
  hidden, the minimap shows only what you know, with the camera frame and
  your supply border.
- Effects and sound: muzzle flashes, tracers, explosions, wrecks, smoke,
  building fires; positioned battle sound, the nation's music, map ambient
  sounds and unit voices.
- Selected-unit panel: portraits, "1st Infantry Regiment" names, status
  text, formation buttons, recruit tooltips with cost and upkeep.
- Unpack for supply trucks and HQ trucks (button or P, then click a spot).
- F1-F4 panels (Objectives, Technology, Message Log, Politics), the
  experience counter, idle truck/regiment buttons (F5/F6), game speed
  steps like the original (+ / -), tooltips.
- Original loading screen art and texts; the scenario list shows the 6
  official maps.
- Random maps: the starting bunkers and AA guns ring the Corps HQ like the
  original; every nation starts with its full income.
- Lighting and shadows matched to the original's brightness.
- The computer opponent builds its Infantry HQ first like the original and
  no longer spams orders.
- Trucks no longer get stuck on building corners.

HOW TO PLAY
1. Run Start.cmd.
2. Main menu -> Single Player -> Custom.
3. Random Map (map type, size, sides, terrain) or Scenario Map, set the
   players, nations, teams and computer generals, then Launch.
4. Click Continue on the loading screen when it has finished.
5. Left click selects, drag selects several, double click selects the same
   kind; right click moves, right click an enemy attacks. Select a building
   to recruit with the buttons on the right. Arrow keys or the screen edges
   scroll, the mouse wheel zooms.

STILL MISSING
- General abilities (special operations), campaigns, World War II,
  multiplayer, the editor, films and saved games.
- Some details still differ from the original; the computer opponent is
  slower than the original's on random maps.

REQUIREMENTS
- Windows 10 or 11, 64-bit, Vulkan or DirectX 12 graphics driver.
- Your installed original Axis & Allies at E:\Games\Axis and Alies with the
  reconstruction folder beside rust-rebuild, as for earlier builds.
- Keep AxisAlliesBevy.exe and the assets-cache folder together.

Earlier Development builds are preserved unchanged in release\.
