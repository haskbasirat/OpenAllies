AXIS & ALLIES RUST - DEVELOPMENT 15 (BEVY) - TEST BUILD
DIAGNOSTIC / UNFINISHED

WHAT THIS IS
The Rust recreation of the original Axis & Allies, drawn with Bevy, using
the original menus, HUD, maps, models, animations, sounds and rules from
your installed game. It follows the original game's code and data closely
(not byte for byte).

NEW SINCE DEVELOPMENT 14
- Fast loading: a custom battle now starts about 0.3 s after Launch
  (Fall of Berlin 0.4 s; the original takes about 0.9 s). The shared game
  data loads in the background while you are in the menus.
- Generals' special abilities: the left column of the HUD shows your
  general's 4 original abilities (carpet bombs, V-weapons, air drops, radar
  scan, espionage...). They cost experience and recharge; hotkeys Y, H, N,
  J; area abilities show their target circle under the cursor. Bombers,
  rockets and transport planes fly in with their original effects and
  sounds. The computer uses its general's powers too.
- Radar Scan reveals the whole minimap while it lasts; Espionage reports
  the target's economy in the message log.
- Animated flags, flying tank debris and tank gun recoil.
- Random maps: villages can lie next to a starting HQ like in the original,
  and are then taken by that side at the start.
- Economy and city capture follow the original more closely (packed
  trucks, cut-off HQs, which units can take a city).
- The computer opponent scouts and saves money like the original.

HOW TO PLAY
1. Run Start.cmd.
2. Main menu -> Single Player -> Custom.
3. Random Map (map type, size, sides, terrain) or Scenario Map, set the
   players, nations, teams and computer generals, then Launch.
4. Click Continue on the loading screen when it has finished.
5. Left click selects, drag selects several, double click selects the same
   kind; right click moves, right click an enemy attacks. Select a building
   to recruit with the buttons on the right. Use your general's abilities
   with the buttons on the left. Arrow keys or the screen edges scroll,
   the mouse wheel zooms.

STILL MISSING
- Campaigns, World War II, multiplayer, the editor, films and saved games.
- Some ability visuals (glows of enchantments, contrails), the radar blip
  on the minimap.
- The computer's early army is still a little smaller than the original's.

REQUIREMENTS
- Windows 10 or 11, 64-bit, Vulkan or DirectX 12 graphics driver.
- Your installed original Axis & Allies at E:\Games\Axis and Alies with the
  reconstruction folder beside rust-rebuild, as for earlier builds.
- Keep AxisAlliesBevy.exe and the assets-cache folder together.

Earlier Development builds are preserved unchanged in release\.
