AXIS & ALLIES RUST - DEVELOPMENT 10 HOTFIX 02
DIAGNOSTIC / UNFINISHED
BATTLE STATUS: UNAVAILABLE

This preview addresses Berlin's reported missing GWorld+E0 clock binding
and Firefight's separate fresh-object classification refusal. It is not a
completed or playable battle build. Exact observed results and remaining
startup errors are recorded in evidence/validation.json.

Changes
- Visibility-memory callbacks borrow the actual loaded map's clock after
  the immediate loading scope ends. Fresh footprint/reveal, Actor/Object
  visibility, zone activation and global Fog forward the same clock owner.
  Timestamp reads keep their native position after record allocation.
- WorldStart recognizes an actual registered fresh Actor using the original
  constructor's proven virtual predicate. Saved objects and Groups preserve
  their previous classification. No health/phase/readiness shortcut is added.
- AI activation retains the proven map-load argument after resource loading.
  Actor effects separately read the same World's current map-mode owner,
  including its native restoration after loading; no mode is guessed.
- The loading notification uses the original executable's actual
  app_status_loading_resources key. The nonexistent loading_world key
  previously stopped view startup. Native missing-key failures stay strict.
- The responsive loader and elapsed timer from Hotfix-01 are retained.

Testing this preview
Run AxisAlliesRust.exe. Choose Single Player > Custom, select a map, enter
a player name and click Test skirmish startup. This runs genuine unfinished
startup, which may take minutes and stop at another unported callback.
An elapsed timer or a completed record-loading diagnostic is not a battle.
Start Asset Viewer.cmd opens the existing separate map/model viewer.
Start Live Preview.cmd opens the isolated infantry animation preview.
F11 toggles fullscreen. WASD/arrows and the mouse wheel control the viewer.

Required files
Keep this EXE and assets-cache together under this installation's release
tree. Original Data.rwd and the existing reconstruction directory, including
recovered assets and v14 overrides, are required. If moved, use --assets
with the existing reconstruction directory. This ZIP is NOT a standalone
whole-game copy. Windows x64, x64 Microsoft Visual C++ runtime and a
compatible Vulkan or DirectX 12 graphics driver are required.

Limitations
No complete playable battle, whole-game original save compatibility,
multiplayer or campaign acceptance. Static asset/animation previews do not
establish original rendering/gameplay parity. Native loader shutdown can
still wait for worker cleanup. No map-loading performance improvement,
completion percentage or finish date is promised by these fixes.

Original game files/profiles/saves and Development-08/09/10/Hotfix-01 are
preserved. Source predecessors were backed up before changes. Rewrite
preferences normally live at %APPDATA%/AxisAlliesRust/settings.json.
See CHECKPOINT.txt and evidence/validation.json for scope and continuation.
