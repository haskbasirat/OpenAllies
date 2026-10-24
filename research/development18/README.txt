AXIS & ALLIES RUST - DEVELOPMENT 18 (BEVY) - TEST BUILD
DIAGNOSTIC / UNFINISHED

WHAT THIS IS
The Rust recreation of the original Axis & Allies, drawn with Bevy, using
the original menus, HUD, maps, models, animations, sounds and rules from
your installed game. It follows the original game's code and data closely
(not byte for byte).

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
- Animations snap from one to the next instead of blending; the landing
  and hit animations are not played.
- Some ability visuals (glows of enchantments, contrails), the radar blip
  on the minimap.
- The computer's early army is still a little smaller than the original's.

REQUIREMENTS
- Windows 10 or 11, 64-bit, Vulkan or DirectX 12 graphics driver.
- Your installed original Axis & Allies at E:\Games\Axis and Alies with the
  reconstruction folder beside rust-rebuild, as for earlier builds.
- Keep AxisAlliesBevy.exe and the assets-cache folder together.

Earlier Development builds are preserved unchanged in release\.
