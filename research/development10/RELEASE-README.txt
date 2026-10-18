AXIS & ALLIES RUST â€” DEVELOPMENT 10
DIAGNOSTIC / UNFINISHED
BATTLE STATUS: UNAVAILABLE

This is a new Windows x64 preview of the Rust rewrite. It is not a completed
or playable battle build. Most work since Development-09 connects original
startup and simulation systems; it does not yet produce a complete match.

What to test
1. Run AxisAlliesRust.exe for the original front end and existing Options.
2. Open Single Player > Custom to test the new setup/startup preview.
   Choose a map, supply a player name and click Test skirmish startup.
   The real startup attempt stops and reports its first unfinished step.
   You can discard a stopped attempt and change setup. No battle is promised.
3. Run Start Asset Viewer.cmd to inspect original maps, models and animation.
   Its Check map loading button checks record restoration, a separate step
   from full skirmish initialization.
4. Run Start Live Preview.cmd for the isolated infantry animation preview.
5. F11 toggles fullscreen; WASD/arrows pan the viewer; mouse wheel zooms.

Changes included
- Real local skirmish setup and retained startup/activation plumbing.
- More original building creation, properties, terrain deformation, control
  zone registration and visibility bindings.
- Scheduled Regiment update routing, Sleep/arrival handling, formation member
  commands, and bounded ground movement code. These are source additions
  checked separately; complete live battle movement/combat is not accepted.
- More native material/model loading and original menu/resource integration.
  The viewer remains an asset preview, not proof of original visual parity.

Required files
Keep this EXE and assets-cache together in this installation's release tree.
The build reads the existing reconstruction directory and original Data.rwd,
including recovered maps, textures, models, UI, audio and v14 overrides.
This ZIP is NOT a standalone full-game copy for someone without those assets.
If moved, --assets must point to the existing reconstruction directory.

Not finished
A complete loaded/activated/rendered battle, native map picking and orders,
complete movement/combat/AI/victory, compatible whole-game saves,
multiplayer and campaign acceptance remain unfinished. No overall game
completion percentage has been established. No artificial memory, VRAM,
disk or bandwidth cap is added; hardware, OS and network limits still exist.

Original AA.exe, original saves/profiles and previous releases are unchanged.
Rewrite settings normally live at %APPDATA%/AxisAlliesRust/settings.json.
Read CHECKPOINT.txt and evidence/validation.json for observed tests and limits.
