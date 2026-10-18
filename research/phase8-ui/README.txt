PHASE 8 - ORIGINAL USER-INTERFACE ENGINE (crates/aa-ui)
=========================================================

Goal: every menu, dialog and HUD screen is built from the original
Data/UI/*.tgi files by a port of AA.exe's UI engine (TGC_Widget /
TGC_ViewElement, .\Source\Ui\System\*.cpp), laid out exactly where the
original puts it, and emitted as a renderer-neutral draw list.

Status legend used below
  NATIVE-PROVEN  original x86 executed under Unicorn on generated inputs,
                 Rust replays every fixture bit-exactly (both x87 PC24/PC53)
  PORTED         translated from the instructions/pseudocode (addresses
                 cited in the code), not executed under an oracle
  SCREENSHOT     verified against qa-output/original-ui/*.png (2560x1440)
  MISSING        not ported; explicit gap

Files
-----
  native_layout.py      Unicorn oracle: bounds parse/resolve/clip, VE layout,
                        state/visibility, when/attribute keyword tables
  native-layout.json    its fixtures (23k resolve cases, 1k parse cases,
                        96 layout chains over 8 screen sizes, 6.9k states)
  native_decal.py       Unicorn oracle: 9-slice piece rects, directional
                        piece rects, decal aspect fit
  native-decal.json     its fixtures
  native_text.py        Unicorn oracle: GetTextExtent 0x7352C0, ref/world
                        wrappers 0x735C00/0x735D20, ellipsis 0x735D50
  native-text.json      its fixtures (18328 cases, PC24/PC53)
  native_draw.py        Unicorn oracle: label rebuild 0x735B90
                        (PreProcessText, GetRowStartingXPixels, Parse, quad
                        builder, AddQuad); `--compact` rewrites
                        native-draw.json from native-draw.json.gz
  native-draw.json(.gz) its fixtures (14604 cases; the .gz keeps every quad,
                        the .json per-case records + FNV-1a digests)
  gdi_metrics.py        bakes crates/aa-ui/data/gdi-metrics.json (GDI Arial
                        cells, see Text)
  screenshot_match.py   rasterises draw-list quads (point sampling, as the
                        original's samplers) and finds the pixel offset that
                        best matches the original screenshots; measures the
                        white label ink boxes
  screenshot-match.json its results (consumed by crates/aa-ui tests)
  render_drawlist.py    composites a draw list to PNG for visual comparison
  drawlists/            draw lists + widget rects written by
                        `cargo run -p aa-ui --example dump_screens`
  aadis.py, findref.py  Capstone helpers (read-only)
  scratch-*/            subagent research scratch (read-only tools)
  requests-to-lead.md   requests and notes for the lead
  scripts/              --ui-script runs for aa-client (rounds 2, 3)
  scratch-hud/lead/     round-3 HUD research notes (notes.txt) and listings

Commands
--------
  set PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-ui/native_layout.py
  python research/phase8-ui/native_decal.py
  python research/phase8-ui/native_text.py
  python research/phase8-ui/native_draw.py            (~8.5 min)
  set CARGO_TARGET_DIR=target/s-ui
  cargo run  -p aa-ui --release --locked --offline -j 4 --example dump_screens
  python research/phase8-ui/screenshot_match.py
  cargo test -p aa-ui --release --locked --offline -j 4

Native facts and their status
-----------------------------
Layout units and arithmetic (NATIVE-PROVEN)
  0x5E3150 bounds parse: L/T/R/B strings; flags L_from_right 1, T_from_bottom
    2, R_from_right 4, B_from_bottom 8, L% 0x10, T% 0x20, R% 0x40, B% 0x80,
    R size 0x100, B size 0x200; rejects '+'L/T, 'b' on L/R, 'r' on T/B,
    '+' with r/b, empty; partial side effects kept. CRT __wtof (0x96D3E8) is
    a seam (MSVC atof prefix rules; it needs the thread locale).
  0x5E3370 resolve: x = sx*v + pL, right = pR - sx*v, % = w*v*0.01f + pL,
    size R = sx*v + L (L re-read as float), size B = sy*v + T (T kept on the
    x87 stack - observable under PC53); 0x5E2460 clamp R>=L, B>=T and clip
    test; 0x5E1580 clip to parent recording per-side cut amounts.
  0x5E34B0 VE layout: outer rect against the parent VE's OUTER rect (screen
    (0,0,W,H) for roots), clip amounts += parent's, client rect against the
    own outer rect, EMPTY flag 0x400 on zero width/height, flags |= 0x21.
  0x5C1DB0/0x5C1DD0 scale = screen / InterfacePositioningWidth|Height (AVar
    slots 0x22/0x23, defaults 1024/768 at 0x402C5A; Data/AVars.tgi).
  x87 precision: PC24 is the default for the UI (the UI runs on the main
    thread, which Direct3D 9 switched to single precision - confirmed by the
    lead, research/phase8-tick/fpu-precision.md); PC53 is supported; 758 of
    11520 resolve cases differ between the two. No transcendental x87
    instructions occur in the oracled UI code (Unicorn computes those through
    binary64).
States (NATIVE-PROVEN)
  0x5E4D00 when table ALWAYS 0, ENABLED 1, DISABLED 2, FOCUSED 4,
    UNFOCUSED 8, PRESSED 0x10, UNPRESSED 0x20, SELECTED 0x40, UNSELECTED
    0x80, OPEN 0x100, CLOSED 0x200, TARGETED 0x400, UNTARGETED 0x800;
    0x5E5210 label attributes NONE 0, TEXT 1, FONT 2, JUSTIFY 4, COLOR 8,
    BACKGROUND_COLOR 0x10.
  0x5E25C0 state bits from widget flags (bit19 enabled, bit3 focused, bit8
    pressed, bit7 selected, bit9 open, bit22 targeted) and visibility
    shown && (when & state) == when; 0x5EE2C0 shown = widget flag bit1 and
    render_threshold >= UI render level (KKC_UI+0xDC = 0 at 0x5CAE30).
  0x5E54A0 when parse: tokenizer 0x412480 with "+ " delimiters,
    case-insensitive (PORTED; unknown tokens OR the previous match).
Decal geometry (NATIVE-PROVEN for rects/UV fit)
  0x5E6AB0 9-slice piece rects (corner half-sizes and border sizes in raw
    texture pixels, not scaled); 0x5E7790/0x5E6D40/0x5E6DC0 directional
    pieces (end half-size = ends texture WIDTH, also when vertical);
    0x5E7000 decal fit for stretch = false.
  PORTED: 0x5E7480 9-slice UV/tile table, 0x5E7EA0 directional UVs (vertical
    end caps swapped), UV field order (u_left, v_bottom, u_right, v_top) ->
    sprite (u0,v0,u1,v1) = (in0,in3,in2,in1); 0x740900 nudge of integral
    coordinates by -0.1 and no half-pixel offset (draw-list rects use the
    i+0.5 pixel-centre convention: x' = adj(x) + 0.5); point sampling with
    clamp (0x742200); tiling grid 0x73F750/0x73FA20/0x73F1B0/0x73F290
    (span in design pixels, texture >= 32 px, q+1 uniform cells, last cell
    partial UV) - NOT oracle-proven.
  Render lists: non-blended sprites (0x742CC0, blend off, z-write on)
    and blended ones (0x742630, z-write off); only alpha_blending matters
    (0x743750); non-blended texels with alpha 0 are discarded (ALPHATEST >
    0). One pre-order layer counter runs over the whole tree from Root
    (0x5C2420 -> vtable+0x40, 0x5E17E0/0x5E1BA0); sprites sit at z = far -
    layer*far*2^-24 (0x744677, 0x744BB0). Evidence: the original options
    dialog hides the main menu completely and its drop-down fields cover the
    earlier label bars (reconstruction/native-game/qa/
    controls-v12-original-default-video.png), i.e. later layers cover earlier
    ones whatever their list = tree (painter's) order, used for menus and
    dialogs; the loading screen (03-launch.png) shows its earlier blended
    briefing box over the later opaque art, i.e. the two lists in order
    without depth rejection. INFERRED: the loading screen is presented by
    the loader's own path without the depth test (DepthMode::TwoLists).
    All screenshot matches are unchanged under this model.
Paths (PORTED)
  0x490ED0 '%s' = screen width ("%d"), falling back to removing '%s/' when
    the file is absent (no UI/2560 folder: 2560 wide uses UI/Default...).
  Skins: in-game art comes from Skins/<NATION> (KKC_UI::Reskin 0x5C3DC0);
    the layer order skin > v14 loose overrides > Data.rwd is INFERRED (the
    04-ingame screenshot shows Skins/USA's control panel).
TGI semantics (PORTED, from instructions; 0 diagnostics over all UI lists)
  lexer 0x5BE9D0 (key ends at '=', ';' or '/', one char skipped:
    `texture /UI/x` is texture = UI/x; `string justify ; LEFT, ...` has the
    value "LEFT, CENTER, or RIGHT"), header 0x5BC0A0 (Template/Inherit/
    List/Optional/Language, implicit template = scope template's child,
    same-template rule), close-time CopyValuesAndSubNodesTo 0x5B57C0, child
    merge 0x5B6190, clone 0x5B5FC0, groups/AddChild 0x5B5C80 (template list
    groups of >= 2 entries are front-inserted before the instance's own
    entries; a derived template always holds its cloned list declaration
    at index 0, so its entries come first; a lone list-member entry without
    a declaration would be appended after them), list declarations fill
    every entry and are never cloned into instances, global template
    registry (duplicates are errors), data nodes per file, `file:node`
    paths. UI template lists load in the order of 0x5C5060: shared, editor,
    game, menus (AVars Interface*Templates). Block comments inside headers
    (template_game.tgi:1613) are removed - INFERRED (the in-game panels load).
Widgets (PORTED)
  0x5F1D00 ctor (flags 0x80007, threshold 10, root group VE flags 0x1006),
  0x5F0080 AddChild (stable sort by key vt+0xA8 = 0.0; MenuBackground
  -FLT_MAX 0x652C20), 0x5EFA10 attach root VE to parent root VE,
  0x5F2070 LoadData (node lookup, tooltip_name/ToolTip, intercept_pick,
  RenderThreshold, View -> root bounds 0x5E29B0/0x5EEB80, visible, children),
  panel 0x5F2C10, top-level 0x5F2D60, interface 0x5C0B50, button 0x5F2DF0
  (pressed_client bounds replace the client's bounds while pressed),
  label 0x5F29B0, decal 0x5F28B0, cinematic (VideoResource list).
  Code-defined hierarchies: main menu 0x64F730/0x650A80/0x650400, backdrop
  update 0x5C92D0, single player 0x65B9D0/0x65B340, loading 0x6E87C0/
  0x6E88E0/0x6E8580, staging 0x661BC0/0x65D350 (panels 0x69E240, 0x6673A0,
  0x692A70, 0x6897F0), game HUD 0x61A650/0x6182B0/0x60BAE0, control panel
  0x615FB0. TGI child widgets that code does not create are ignored.
Text (NATIVE-PROVEN measure and draw layout; glyph cells PORTED)
  font size px = floor(size * screen_h / 768 + 0.5) (0x708C03..0x708C99),
  backup face when px < use_backup_font_when_smaller_than; TTF faces via
  GDI+ UnitPoint (em = px*4/3 at 96 dpi), others GDI (em = px).
  Measure (rtext.rs, NATIVE-PROVEN by native-text.json): GetTextExtent
  0x7352C0 with its markup tokens (Color/RC/Font/SetX/SetY/h_on/h_off),
  remapped 0x11-0x15, stand-ins for 10/13/14, missing-glyph substitution
  (character sets), two rows per wrap, the width including the partial
  word; 0x735C00 / 0x735D20 wrappers; ellipsis 0x735D50 (grammar_ellipsis,
  visible length 0x732C60, Left 0x732E60). The native endless loop when a
  glyph is wider than the wrap width is reported as Measured::Hang.
  Draw layout (rdraw.rs, NATIVE-PROVEN by native-draw.json): label rebuild
  0x735B90 = PreProcessText 0x734120 (rows, 0x0D/0x0E wrap insertion with
  the native index drift, the rest of a token dropped when one glyph cannot
  fit), GetRowStartingXPixels 0x730130 (LEFT = -clipL, CENTER floor((W -
  row_w)*0.5 + 0.5), RIGHT W - row_w, trailing spaces excluded), Parse
  0x733B10 (colour/RC/h_on markup, <RC> before any Color = transparent,
  mixed fonts bottom-aligned after a taller font, use_own_color special
  glyphs), quad builder 0x730F00 (quads are the advance wide: the shadow
  column is cut; rows advance by the largest 'A' height of the row) and
  AddQuad 0x72FA00 (screen clamp, clip to the area, floor, highlight quad
  in the label background_color, overflow tint). The view element sets the
  area to the clipped outer rect and the origin to (-clipL - edit box
  scroll, -clipT) (0x5E2C90); no vertical alignment. The draw list carries
  the glyph quads; aa-client paints each glyph clipped to its quad.
  Glyph cells (fonts.rs, PORTED): GDI+ TTF cells w = ceil(f32(adv*em) /
  upem), h = ceil((hhea asc - desc + gap) * em / upem) (checked against live
  GDI+ for 32..383 at 8..48 px: one difference, U+00AD); GDI Arial cells
  are hinted GetTextExtentPoint32W values baked on the build machine
  (gdi_metrics.py -> data/gdi-metrics.json, 4..64 px; a Windows 11 Arial,
  INFERRED to equal the XP-era hinting at these sizes); padding +2 outline
  else +1 shadow, advance = cell - 1 with shadow, line height = 'A' cell,
  fixed = widest + 2; special glyph images Fonts/<ids>/<screen
  width>/<name>.png else Fonts/<ids>/<name>.png (use_own_color default
  true). Glyph shapes are drawn by the client's font rasteriser.
  SCREENSHOT: label ink starts within 0-2 px of the computed pen in x and
  0-8 px below the cell top for every checked label.
  Localization: [Text] tables, case-insensitive keys, '#key' lookups,
  missing -> error_missing_string.
Input (PORTED behaviour, partly INFERRED)
  pick walks children tail -> head (0x5E2B50) and returns the topmost
  widget taking input (buttons, slider grips/tracks, owned list items,
  drop-downs, edit boxes; a drop-down's SelectedItem has no owner and its
  clicks reach the drop-down); hover focuses it, press sets PRESSED (+
  pressed client bounds) and plays set_sound (UiEvent::Sound), release over
  the same button fires its command string. Toggles in a radio group (the
  options tabs, 0x5F0420) stay selected. Callbacks are queued (0x5C3EB0)
  and run after the event. Hotkeys: hotkeys_ui.txt entries, tried from the
  topmost top-level down (dispatcher INFERRED, hotkeys.rs); the modal
  message box takes MessageBoxAccept/Cancel (first/last button).
  Tooltips and keyboard focus order are MISSING.
Control internals (PORTED, controls.rs / control_load.rs / ui_controls.rs)
  slider 0x5DE360/0x5DE800 (usage area 0x5DD2E0/0x5DD340/0x5DD420 with the
  pixel-width quirk, UpdateGrip 0x5DD960, background 0x5DD670, snap/clamp
  0x5DD760, SetValue 0x5DDAC0, SetMin/SetMax 0x5DDB20/0x5DDB90, notify
  0x5DDCA0, track click 0x5DDFB0 (scroll bars page), grip drag
  0x5DDC00/0x5DDEB0, Less/More 0x5DDD20); advanced slider 0x5DEB20/
  0x5DE940 labels with formatter 0x5DEA20 and the options formatters;
  scroll bar 0x5DE560 (key 0.2, vertical); list box 0x5D4C80/0x5D58E0
  (height B = n*h + 2*off, Activate 0x5D4F30, item widgets 0x5D48E0 named
  ItemLook, per-frame bind 0x5D4A30/0x5D3E10, scroll range 0x5D31B0, top =
  trunc(v + 0.5) 0x5D2CC0, sorted insert 0x5D4250/0x5D32A0, Select
  0x5D4030, click 0x5D55A0, command 0x5D50A0 fired even for a redundant
  select); drop-down 0x5D5C30/0x5D6200 (Activate 0x5D4430 takes the list
  box out of the tree, Place 0x5D3450 in absolute reference units, open
  0x5D3740 / close 0x5D2DD0 through the UI overlay, UpdateSelected 0x5D4390,
  command 0x5D51C0 with SelectedItem.param; OpenButton is never created);
  edit box 0x5D1000/0x5D23E0 (focus + select-all on first click, insert
  0x5D11F0 with integer_only/max_characters/integer_max, commit 0x5D0D60,
  change 0x5D0DA0, caret 0x5CF9B0 with the ch_insert/ch_overwrite glyph and
  500 ms blink); viewport 0x5EDAA0 (item height 0x5ED150, Place 0x5ED1C0,
  top-down layout 0x5ED510, scroll bar update 0x5ED560, AddItem 0x5ED730,
  Snap 0x5ED2C0, Step 0x5ED450, wheel 0x5ED940).
  List text truncation uses the native ellipsis 0x735D50 (round 3); edit
  text scrolling 0x5CF770 is ported (round 3). INFERRED: a press outside an
  open drop-down closes it.

Screenshot verification (SCREENSHOT, 2560x1440)
-----------------------------------------------
  draw-list quads matched against qa-output/original-ui (screenshot_match.py;
  "verified" = offset (0,0) with error <= 30, enforced by
  tests/screenshot_layout.rs). In-game screens use PLACEHOLDER world data
  (hud.rs PlaceholderWorld: values read off the screenshots, stand-in unit /
  action / ability textures and no 3D models), so their unmatched quads are
  mostly the stand-in textures.
  screen                    quads  matched  offset 0  verified
  main menu      (00)          21       13        13        13
  single player  (01)          17        9         9         9
  staging        (02)         397      338       267       267
  loading        (03)         190       43        43        43
  game HUD       (04)          39       37        32        29
  regiment       (05)          95       90        66        60
  corps HQ       (06)          71       67        50        44
  recruit+impact (07)          77       72        53        45
  game menu      (13, 08)     110      106        74        65
  quit box       (13)         162      154       111       103
  Label text: 15+ labels within the tolerances above; the economy bar,
  object display and message log texts match the screenshots character for
  character (qa-output/native-port/r3-compare-*.png).

Removed caps / limits
---------------------
  max_child_count (widget template field) is a performance hint; children
  are growable vectors. The native font atlas packs glyph x/y in 9 bits and
  w/h in 7 bits (0x705020) - text is laid out without any atlas here. The
  aa-rules TGI syntax limits are not used (aa-ui has its own native lexer).

Round 2: aa-client integration
------------------------------
  Options dialog (PORTED, options.rs; research by the options subagent,
  scratch-options/): ctor 0x67B110, Init 0x6788E0 (tab toggles with
  "OptionsPanelSelectTab <name>", pages' CreateChildren, AddTabPage
  0x5DF6C0, SelectTab(GamePlay) 0x5DF250, OK/Cancel/Defaults buttons),
  Show 0x6792A0 (last tab persists, activation 0x6743E0 runs every page's
  ReadSettings), OK 0x6768D0 (Apply all, close, save), Cancel 0x674430,
  Defaults 0x6793B0 -> confirm box -> 0x674480. Pages: GamePlay (0x679C70,
  ReadSettings 0x675F80, Apply 0x6763E0 incl. the camera scroll ratio,
  Defaults 0x674C20), Video (0x67A830, resolution list 0x6767B0/0x675110,
  presets 0x67C8B0 + match 0x67BE90, preset apply 0x67BB80, CanLeave
  0x67CFF0), Audio (0x677740, live channels 0x673F40..0x674010, Cancel
  quirk: channel 1 from the FX slider), Network (0x677D40, inverted browser
  filters, AFK/refresh ranges from AVars), MetaGame (0x943F20; its TGI
  hides the message panel and two toggles). UVar defaults/ranges
  (uvars.rs) from the registration table and Data/UVars.tgi. The user
  values persist in %APPDATA%\AxisAlliesRust\settings.json ("uvars"), never
  in the original User/UVars.tgi. Engine inputs this port does not have:
  display modes (the window size is the only mode; a different mode takes
  the native failure path failed_to_change_resolution), camera zoom limits
  (no camera: the data range is kept, diagnostic), game speed (disabled as
  outside a game).
  Credits (PORTED, credits.rs; research by the credits subagent):
  CreditsInterface 0x646150, LoadData 0x645F70, nodes 0x645BD0/0x471180
  ('@' literal 0x5BA3C0), labels pooled per label_ids (0x645D10/0x646020/
  0x646670, AddChild flag 1 = below the gradients), OnConnected 0x646B50,
  Update 0x646C10 (50 units/s real time, two fade steps, spawn 0x646A90,
  release, CreditsEndCredits 0x645800), crossfade 0x645730/0x645890. Esc
  posts CreditsEndCredits (hotkeys_ui.txt). The label height uses the
  approximate text measure (see the message box note).
  Loading screen refresh (PORTED, screens.rs): 0x6E83A0 (labels, objectives
  list for kind 0, art choice mission > dynamic campaign (kind 4) > local
  player's nation 0x48C050 > interface default; the dynamic art gate
  0x40D0F0(0xB) is not resolved), 0x6E8C00 (app_status_finished + Ready
  button), status 0x6E8300. The asset viewer has no world, so it shows the
  default art; the frame clears to black (03-launch.png).
  Emulator caveats re-check (docs/NATIVE-PORT-RULES.md): native_layout.py
  now maps page 0 only for the FS:[0] SEH word and stops on any other page-0
  access, and stops if operator new/delete or the CString allocator is
  reached; native_decal.py uses the same machine. Both fixture files were
  regenerated and are byte-identical to the earlier ones (no fixture
  depended on a page-0 read or the heap; no NaN inputs; the only code stub
  is written once).
  crates/aa-client/src/tgi_frontend.rs paints aa-ui draw lists in egui
  (point sampling, alpha-test for non-blended quads, A&A / Arial faces as
  chosen by the font TGIs, the original 1-px drop shadow) and feeds pointer
  input; frontend.rs uses it for the main menu, the single player menu
  (gameplay-starting buttons disabled with the native DISABLED look), the
  quit/error message box (0x66DF90 port, message_box.rs) and the original
  LoadingScreen while the asset viewer builds its map. The MiniMenuBackground
  globe plays from assets-cache/video/MainMenuGlobe (bake_globe.py).
  Scripted runs (1440x900 logical window):
    AxisAlliesRust.exe --ui-script research/phase8-ui/scripts/round2-menus.json
    AxisAlliesRust.exe --ui-script research/phase8-ui/scripts/round2-quit.json
    AxisAlliesRust.exe --ui-script research/phase8-ui/scripts/round2-loading.json
    AxisAlliesRust.exe --ui-script research/phase8-ui/scripts/round2-options.json
    AxisAlliesRust.exe --ui-script research/phase8-ui/scripts/round2-credits.json
  `cargo run -p aa-ui --example dump_round2` writes the options/credits
  draw lists (drawlists/r2_*.json).
  Shots: qa-output/native-port/tgi-*.png.
  Message box check: the in-game quit box over the game menu (13-quit.png)
  matches with every 9-slice piece at error 0 but 1 px lower than the
  original and its buttons 1 px higher: the text measure (label
  vtable+0xCC) is approximated from glyph metrics and is ~1 reference unit
  short for the tiny (Arial) font.

Round 3
-------
  Text: see Text above (measure + draw layout NATIVE-PROVEN; the
  quit box is now at its original position: 9-slice pieces at (0,0) error
  0, game_quit 103 verified quads).
  Staging rows (staging.rs, PORTED; research scratch-staging/): pools
  PlayerSlot / ObserverSlot / TeamSlot cloned from Slots/ItemPanel, the
  viewport fill order (0x69F950), slot modes, action panels, map preview
  text 0x660E90 ("<color=%g,%g,%g>%c <rc>name"), colour wrap 0x731980
  ("<Color=%g,%g,%g>text<RC>"), label fit height 0x5EF5C0; the slot Ready
  toggle does not flip on click (+0xE8 cleared, 0x69F6AC). Markup labels
  (KingdomColorIcon, map preview) now render through rdraw.rs. Slot data
  comes from staging::StagingState (placeholder_state() for screenshots).
  HUD (hud.rs, PORTED; notes scratch-hud/lead/notes.txt, object display
  notes scratch-hud/notes-objdisp.txt): the code-built trees and runtime
  layout of EconomyBar / EconomicImpact (texts, impact projection,
  hidden-offset slide with the UI movers 0x5C3260/0x5C2800, sort keys 1.0 /
  0.5 at vtable+0xA8), FactionExperience, Group / FactionAbilities floaters
  (ShowFloater, end caps, group button x = i*W*0.1, ability items with
  recharge bars), idle buttons, TimeMessage / camera label visibility
  (UVars), ObjectDisplay -> CompanyDisplay (regiment and HQ: name, owner
  markup with the KingdomGlyphInfo glyph, portrait / banner model rects,
  health / morale / attack-efficiency / supply status bars with target
  markers, economy label 0x5C4CB0, state, company units layout 0x62F690,
  HQ attachment grid 0x62D010, formation buttons, properties) and the
  ActionButtons 4 x 3 grid. Game data enters through hud::HudWorld (an
  explicit trait the native world implements); StatusBar view elements
  (type 7, 0x5E3DE0/0x5E61C0/0x5E2FE0/0x5E2100) and StatusBarWidget
  (0x5DEF90/0x5DF0F0, TargetPercentage 0x5DF060) are ported.
  Message log / minimap / numeric edit box / edit box (message_log.rs,
  minimap.rs, ui_controls.rs; research scratch-internals/): message
  system send/filter 0x40B5C0, log update 0x671230 with row pooling and
  fading (a = 1 - d*0.001), GameMessageLog = 0x5FD6D0(slot 0x47
  UIMessageLogMaximumLines, fades, auto height); minimap geometry
  0x5E9270/0x5E9030/0x46DD10, Point/Inverse 0x5D72C0/0x5D73E0, input
  0x5D77A0/0x5D7830 (UiEvent::Minimap); numeric edit box 0x5D7B80 (Up/Down
  0x5D79F0/0x5D7AC0, SetStep 0x5D7910); edit box scrolling 0x5CF770 over
  the rebuilt glyph records, caret 0x5CF9B0, selection highlight 0x5D0550
  ("<h_on>"/"<h_off>" markup, drawn in background_color), commands
  SelectAll/Copy/Cut/Paste/Cycle (0x5C13E0, 0x5C4190, 0x5C1400, 0x5C1420,
  0x5C26F0/0x5C2730, FindNext 0x5C2500; no EditboxUndo exists); list double
  clicks (Windows CS_DBLCLKS -> 0x108, list item select then double
  command). aa-client: clipboard (egui Copy/Cut/Paste events, ctrl-a,
  tab), double clicks (500 ms / 4 px Windows defaults).
  Loading screen: objectives 0x673D40 fill then ClearItems (the list is
  always empty); the dynamic-campaign art gate 0x40D0F0(0xB) = the
  MetaGame/UI/choose_dynamic_campaign.tgi menu exists.
  Client shots (round3-text.json): qa-output/native-port/tgi-r3-*.png
  (main menu text, options, edit box selection highlight). HUD / staging
  previews: qa-output/native-port/r3-*.png and r3-compare-NN(-panel).png
  (original above, port below; render_drawlist.py, glyph shapes from PIL).

Open gaps (MISSING / unresolved)
--------------------------------
  * HUD: SimpleActor / Settlement / Group displays (HudSelection::
    Unsupported), the recruit item floater (disabled in AA), tooltips
    (the 07-recruit anchored tooltip), the pinned impact modes (+0xD9 /
    +0xDA), company unit border decals and AltBackground, the action
    button state machine (the world supplies each slot), the
    FavoritesDisplay, KingdomDropDown visibility (hidden), side panels,
    which code hides the SupplyStockpile bar of regiments (INFERRED from
    05/06), when the action button decal takes the pressed inset
    (INFERRED: while selected), the owner relation colour source
    0x457CD0 (world-supplied).
  * Staging: SelectablePanel release/drag semantics, the ActionPanel
    ReadyToggle auto-flip flag (not traced).
  * Highlight list (<h_on>) rendering as a solid background: INFERRED (the
    batch's second list renderer was not traced).
  * NIF / model view elements are drawn by the caller (External with the
    model id).
  * Native hotkey dispatcher (the rules in hotkeys.rs are inferred).
  * Sprite tiling grid not executed under an oracle; widget transitions
    (WTFX colour/alpha), animated decals timing, colour modes 1-3.
  * Tooltips, keyboard focus order. UI sounds are reported
    (UiEvent::Sound) and played by the client when the IDS is known.
  * Recorded by the lead, not ported: 0x76F600 "no human opponents
    remaining" message; 0x41C180 KKC_TimingStats (wall-clock smoothing for
    on-screen diagnostics).
