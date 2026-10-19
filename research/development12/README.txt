AXIS & ALLIES RUST - DEVELOPMENT 12 (BEVY ENGINE TEST)
DIAGNOSTIC / UNFINISHED
BATTLE STATUS: NOT PLAYABLE (display test of the Bevy engine only)

WHAT THIS IS
A test of drawing the original game with the Bevy engine instead of our own
renderer, so you can judge how it looks and how fast it runs. It loads the
original Fall of Berlin map from your installed game: terrain with the
original texture blending, all 7,963 placed objects (trees, rocks, hedges,
fences, roads, buildings, the Reichstag), the original models and textures.

For the test it also places, for display only:
- a Russian Corps HQ on the first start location,
- two rows of Russian infantry playing their original Idle and Attack
  animations, and a squad jogging around the HQ (original Jog animation),
- a T-34 driving a loop.
No gameplay runs: you cannot select or order anything, there is no HUD,
no menus, no sound and no computer opponent in this test.

HOW TO RUN
1. Run Start.cmd.
2. The battlefield opens in a few seconds (no menus in this test).

CONTROLS
- Arrow keys, WASD or the mouse at the screen edges: move the camera
- Mouse wheel or Page Up / Page Down: zoom
- Q / E: rotate        R / F: tilt
- Home: back to the HQ
- Close the window to quit.
The frame rate is shown at the top left (about 120 FPS on an RTX 3080 with
shadows on).

WHAT TO COMPARE
Development 11 is the faithful port with our own renderer. This build draws
the same original data through Bevy. If Bevy is the way to go, the gameplay
already ported (rules, economy, orders, movement, combat) stays as it is and
only the drawing moves to Bevy.

KNOWN DIFFERENCES FROM THE ORIGINAL LOOK
- Lighting is Bevy's lit material tuned to look like the original, not the
  original fixed-function lighting; colours are close, not identical.
- Water, fog of war, selection circles and effects are not drawn yet.
- Team colours are fixed (red for the placed Russian units).

REQUIREMENTS
- Windows 10 or 11, 64-bit, Vulkan or DirectX 12 graphics driver.
- Your installed original Axis & Allies at E:\Games\Axis and Alies with the
  reconstruction folder beside rust-rebuild, as for earlier builds.
- Keep AxisAlliesBevy.exe and the assets-cache folder together.

Development 11 and all earlier packages are preserved unchanged in release\.
