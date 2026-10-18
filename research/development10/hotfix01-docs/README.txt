AXIS & ALLIES RUST - DEVELOPMENT 10 HOTFIX 01
DIAGNOSTIC / UNFINISHED
BATTLE STATUS: UNAVAILABLE

This Windows x64 preview addresses the unresponsive skirmish loading screen
reported in Development-10. It does not complete a playable battle.

What changed
Source preparation and the existing resource worker are polled without
waiting inside a GUI frame. The original application thread still owns
startup and registration; the same mission owners and native task body are
used. The window shows an elapsed loading timer and reports the first
unfinished startup step. No readiness flags or gameplay are invented.
Global Fog now reads the same published spatial owner after map adoption.

How to test
Run AxisAlliesRust.exe, choose Single Player > Custom, select a map, enter
a player name, and click Test skirmish startup. Loading is diagnostic and
can take minutes. The attempt can stop at an unfinished callback; this is
not a successful battle. The exact observed tests and limitations accompany
this build in evidence/validation.json. Start Asset Viewer.cmd and Start
Live Preview.cmd open the existing separate asset/animation previews.

Required files
Keep this EXE and assets-cache together under this installation's release
tree. Existing original Data.rwd and the reconstruction directory, including
recovered original assets and v14 overrides, are required. If moved, use
--assets with the existing reconstruction directory. This ZIP is NOT a
standalone full-game package. Windows x64, the x64 Microsoft Visual C++
runtime and a compatible Vulkan or DirectX 12 driver are required.

Still unfinished
No map is accepted as a complete playable battle. Complete World/View
activation, native selection/orders, movement/combat/AI/victory, original
whole-game saves, multiplayer and campaign remain unaccepted. Static
previews and bounded native tests do not establish whole-game parity.

The original game, original saves/profiles and Development-08/09/10 are
preserved. Rewrite preferences normally live at
%APPDATA%/AxisAlliesRust/settings.json. See CHECKPOINT.txt for continuation.
