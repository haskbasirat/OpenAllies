# Requests from the aa-ui (phase8-ui) session to the lead

## 1. aa-client dependency on aa-ui - DONE (thank you)

`aa-ui = { path = "../aa-ui" }` is in crates/aa-client/Cargo.toml; round 2 is
wired: `mod tgi_frontend;` in main.rs (one line), frontend.rs uses
`TgiFrontend` for the main menu, single player menu, quit/error message box
and the loading screen before the asset viewer.

## 2. assets-cache/video/MainMenuGlobe (FYI)

`research/phase8-ui/bake_globe.py` bakes the MiniMenuBackground cinematic
(AAGlobeBig, from the reconstruction's MP4) into
`assets-cache/video/MainMenuGlobe/` in the same flipbook format as
`research/bake_menu_video.py`; the shell plays it behind the logo when the
folder exists. Please include it in packaging if you package assets-cache.

## 3. Emulator caveats re-check (answer)

native_layout.py now faults on page-0 access unless FS-prefixed (the SEH
word only) and fails a case that reaches operator new/delete or the CString
allocator; native_decal.py uses the same machine. Both fixtures regenerated
byte-identical: nothing changed.

## 4. Render order model changed (FYI)

The original options dialog screenshot in
reconstruction/native-game/qa/controls-v12-original-default-video.png shows
that later UI layers cover earlier ones regardless of blending (the main
menu is fully hidden; drop-down fields cover the label bars), so aa-ui now
emits draw lists in tree order for menus and dialogs; only the loading
screen keeps the opaque-then-blended order (03-launch.png). Every earlier
screenshot match is unchanged.

## 5. Settings file (FYI)

%APPDATA%\AxisAlliesRust\settings.json gains "uvars" (user variables changed
in the options dialog). The volume fields are now mirrors of the audio
UVars; the old invented defaults (0.8/0.5/0.8/0.9, "Commander") are gone.

## 6. Round 3: game-data seams for the native world (FYI / request)

The in-game HUD reads everything through `aa_ui::hud::HudWorld` (resources
in registry order with type / icon / limited_by / limit provider / purchase
costs; the local kingdom economy arrays +0x1A0/+0x1AC/+0x1B8/+0x1C4 and the
SVar limited resource maximum; the hovered action's economic impact;
faction experience; faction abilities; group textures; idle builder /
detached organization; game time; the selection as `HudSelection`; the 12
action button slots). Staging reads `aa_ui::staging::StagingState`. Both
ship PLACEHOLDER implementations only for layout checks and screenshots
(`hud::PlaceholderWorld`, `staging::placeholder_state()`); please route the
native world / session owners to implement them. `HudSelection::Unsupported`
exists for the SimpleActor / Settlement / Group displays not ported yet.

## 7. Round 3: text (FYI)

Labels are now laid out by the native-proven rebuild 0x735B90
(`aa_ui::rdraw`); `draw::Text` carries per-glyph quads (`glyphs`, `faces`)
instead of rows. GDI Arial cells come from `crates/aa-ui/data/gdi-metrics.json`
(baked on this Windows 11 machine with `research/phase8-ui/gdi_metrics.py`);
if you have an XP-era machine/VM, re-baking there would settle the one
INFERRED step.
