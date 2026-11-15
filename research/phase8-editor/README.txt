PHASE 8 - THE MAP EDITOR
========================

The original game has a full scenario editor. Our client has none yet: the
main menu's Editor button is listed as not built (aa-bevy ui/front.rs,
NOT_BUILT "RequestEditor"; aa-client tgi_frontend.rs shows the asset viewer
for it). This file says what the original's editor is, from its own data
and the strings and layouts it ships, and how to port it in small proven
steps. Nothing here is invented: where the original's rule is not found yet,
it is listed under UNKNOWN.


1. HOW THE ORIGINAL ENTERS THE EDITOR
-------------------------------------
- Main menu button MainMenuButtonEditor ("Editor", UI/Menus/main.tgi), command
  RequestEditor (also compiled into the main menu panel, aa-ui screens.rs).
  The main menu registers it in 650A80 with handler 64F3D0, which calls
  64EF70 on the current application state: a local session (7513F0 with 1,
  3), then the new-state record (the 0x28-byte state of aa-metagame
  launch.rs NewGameState) set by 784160(2, 0, 0) (+0x10 = 2, +0x04 = 1,
  +0x14..+0x16 = 0) and 784130(5, 1) (kind 6, +0x18 = 5, +0x1C = 1), sent by
  75D2D0. The battle launch sends the same kind 6 with +0x18 = 4, which
  suggests +0x18 = 5 selects the editor's application state (not yet traced
  through 75D2D0). Compare the Film button: 64F3F0 -> 64EFF0.
- QuickStartEditor in the main menu is registered with 943800, an empty
  function. The in-game interface (61A650) registers RequestEditor,
  RequestEditorConfirm and RequestQuickEditor (RequestFinishedQuickEditor
  leaves the quick editor).
- From a map being tested: confirm "Exit to Editor?" (editor_confirm,
  strings_rtse.tgi), command RequestEditorConfirm.
- Editor app state: EditorAppState (ProceedToTestAfterSave, 6A78D0). The
  session has a SetSessionEditor decree order (65F0A0, handler 65FD00);
  when it is sent is not traced yet.
- Leaving: RequestFinishedConfirm (Exit Editor), QuitConfirm.
- Seen in the original on a hidden desktop (round 4: the QA copy's test
  profile ran RequestEditor from the main menu): the log shows
  Session::Reset(), KKC_Session::SetNewState(3), Launch(),
  CreateBlankMapGenerateTask(), ChangeToState(3,6,1), application state
  3 -> 6 -> 5, "VWorld::Init, set camera to (96,96)" and "connect Editor".
  The first screen is the top bar (File, Map, Edit, Options, Layer, Scenario,
  Tools, first letters yellow) and the minimap at the bottom left over a
  blank grass map with doodads. No layer panel is open.
- The editor's Load (OpenLoadDialog) opens UI/Menus/menu_file_dialog.tgi
  MenuFileDialog: the list, the name box, Load (its command is FileOK, as the
  log shows), Delete, Cancel, and the refresh and new-folder icons. It lists
  the profile's Profile/data/Maps folder (AVar PathMaps = Maps/): empty when
  the folder is missing, "(2v2)Berlin.RMP" once that file is there. FileOK
  asks ui_file_dialog_load_map_confirmation ("Any unsaved changes to the
  current map will be lost.  Continue anyway?", Ok / Cancel) first.


2. SCREENS (Data/UI/Editor/*.tgi, art under UI/<res>/Editor)
------------------------------------------------------------
- editor_interface.tgi: the Editor interface (Template=SilentInterface, with
  no view of its own). editor_templates.tgi: its widget templates (loaded
  from the InterfaceEditorTemplates list, aa-ui tgi.rs load_ui_templates).
- menu_panel.tgi: the top bar with menus. Labels from strings_ui_AA.tgi:
    File      (its items are not in the layout; the layout's own list, commented
              out, is Load, Save, Save As, Options, Test, Exit Editor, Quit
              Application with commands OpenLoadDialog, EditorSave,
              OpenSaveDialog, OpenOptionsMenu, RequestTestConfirm,
              RequestFinishedConfirm, QuitConfirm)
    Map       New Map (OpenNewMapDialog), Random Map (OpenRandomMapDialog)
    Edit, Options
    Layer     Terrain, Actors, Doodads, Triggers, Cameras, Areas
              (ActivateTerrainLayer ... ActivateAreaLayer)
    Scenario  Map Properties, Sides And Teams, Map Restrictions, Objectives
    Tools     Clear Map, Clear Doodads, Clear Actors, Clear Triggers,
              Verify Map (EditorVerifyMap), Prepare Map (EditorPrepareMap),
              Add Noise (AddTerrainNoise), Beautify Map (BeautifyMap)
- Layer panels: layer_terrain.tgi (brushes: circle, square, raise, lower,
  plateau, smooth, paint - see the Brush*.tga art), layer_actors.tgi (actor
  groups: unit, building, organization, truck, settlement, structure,
  special; AVars EditorDefaultActorGroup, EditorDefaultActorType),
  layer_doodads.tgi (biome, previous/next aspect), layer_triggers.tgi
  (trigger list, events, conditions, effects, Active, Repeats, OR
  conditions), layer_cameras.tgi, layer_areas.tgi (box and circle areas,
  width and height).
- Dialogs: new_map_dialog.tgi (New Map: Size, Biome, Weather, Sides, Doodads;
  Generate, Cancel), map_properties.tgi (Name, Description, Game Type,
  Cutscene Mission), kingdom_editing.tgi (Sides And Teams) with
  relations_viewport.tgi, restriction_groups.tgi (groups of unit, building
  and technology restrictions applied to sides), objectives_dialog.tgi,
  properties_dialog.tgi, message_box.tgi, message_log.tgi, minimap.tgi.
- Shared with the game: UI/Shared/random_map_dialog.tgi (EditorRandomMapDialog),
  UI/Shared/organization_editor.tgi (regiment editor), the file dialog
  (ui_file_dialog_* messages, "new folder" via ui_editor_ask_new_folder_name).


3. TOOLS: THE EDITOR'S ORDERS
-----------------------------
The editor's actions are decree orders sent through the session, the same
order pipeline the game uses (the exe names a request-decree handler for
the orders below; AssignSettlement, Create and Place are commands of the
editor interface):
- Terrain: PaintTerrain, PaintHeightmap, AddTerrainNoise, BeautifyMap.
- Objects: CreateGThing, CreateActor, Destroy, SelectedDestroy, Copy, Paste,
  Drag, RotateSelection (EditorRotateRadians), OrientSelection, ThingOrient,
  OrientSelectOrganizationElements, HQAttach, HQDetach, DoodadAutoPlacement.
- Areas and cameras: CreateGArea, GAreaProperties, CreateGCamera,
  GCameraProperties.
- Triggers: AddTrigger, ReplaceTrigger, RemoveTrigger.
- Scenario: ChangeMapProperties, ChangeMapType, SetBlankSettings,
  SetRandomMapMaxPlayersInEditor, world rules (ChangeWorldRule Locked, Bool,
  Int, Fixed, Enum).
- Sides and teams: CreateGKingdom, change faction, nation, resource
  stockpile, colour, name, open or computer, computer general; uses default
  resources; add or remove a hero; add a property; CreateGTeam, team name,
  add or remove a side, side and team relations.
- Restrictions: create, delete and name a group, add or remove technology
  and actor restrictions, set a thing's group.
- Objectives: add, remove, change description, toggle required.
- Clear: ClearMap, ClearObjects, ClearActors, ClearTriggers. Undo, Redo.
- Session: EditorPrepareMap, EditorVerifyMap, EditorSave.
User variables of the editor (aa-ui view_variable_registration.rs, indices 158-163):
EditorMapStringsSeparate, EditorNewMapAutoDoodify, EditorAutoVerifyOnSave,
EditorSAIActiveInTest, EditorCameraUnlocked, MapSaveCompressed (default
true); also TestMapTemporaryFile, TestMapTemporaryFilePrefix,
CameraEditorPreviewTime, EditorShowCustomizeButton.


4. FILES IT READS AND WRITES
----------------------------
- Reads: maps (.RMP chunk files) through the map loader in its editor mode
  (ResourceManagerTaskLoadGame LoadForEditor; "loading test map in editor -
  fixing MapResource"; failed_to_load_savegame_in_editor), the UI layouts and
  strings above, and the game data the map refers to.
- Writes: .RMP maps. All 80 shipped maps have their chunks in this order
  (68 have the bracketed pair). The map save (4F2DB0) also has branches the
  shipped maps do not take: METAGAME after KINGDOMS (a campaign save) and
  STATIC_THINGS (aa-metagame chunk_file.rs, aa-mapworld campaign_save.rs):
    MAP (stored raw) MAP_HEADER GWORLD RULES TERRAIN HEIGHT_MAP
    MOUNTAIN_HEIGHT_MAP IDMGR RESTRICTION_GROUPS TEAMS OBJECTIVES KINGDOMS
    [USED_REQUIRED_OBJECTS USED_REQUIRED_OBJECTS_INDEX] DYNAMIC_THINGS
    DYNAMIC_THINGS_INDEX TRIGGERS DENIZEN_GROUPS SUN UPDATABLES MISC_DATA
    MAPSTRINGS
  through the chunk file writer (859150 open chunk, 858E40 write, 859330 /
  859090 close). A compressed chunk goes through a zlib 1.1.4 stream at level
  6 (8B72E0 calls deflateInit_ with 6 and "1.1.4"; the exe carries the zlib
  1.1.4 strings). Test play (EditorAppState ProceedToTestAfterSave, 6A78D0)
  saves to TestMapTemporaryFile, then runs LoadHeader, ComputeChecksum,
  LoadMapOrder and LaunchOrder; a failed save shows
  failed_to_save_temporary_map_to_test_it (6A7F60).
- The record's flags word: bit 0 is the compression flag; bits 1..63 are
  never written by the original. 859090 builds each record in one 0x60-byte
  stack buffer (8582B0 clears bit 0 and sets it only for a compressed chunk;
  it never copies the descriptor's +0x50), so bits 1..63 are stale stack
  bytes, one value shared by every record of a file. All 80 shipped maps
  have such bits, each map its own single value. Our reader calls this word
  source_timestamp.


5. DONE SO FAR
--------------
Round 3, crates/aa-editor (new workspace member):
- The zlib 1.1.4 deflater, ported (the lazy matcher used by levels 4 to 9,
  the Huffman trees and block output, the zlib header and Adler-32
  trailer). Streaming like the original's chunk compressor: write = deflate
  with no flush (8B7520), finish = deflate with finish (8B76B0).
- map_file.rs: the map file writer, the exact inverse of
  aa_mapworld::MapArchive.
- Proof: tests/shipped_maps.rs reads every .RMP in the original Data.rwd
  with our reader, writes it again and compares: 80 maps, 1736 chunks (1656
  compressed) are identical byte for byte. A second test feeds every
  compressed chunk to the compressor in small pieces (1 to 300 bytes; the
  original save's write sizes are not traced, so this covers that
  assumption): all 1656 are identical too. Run with
  cargo test -p aa-editor -- --include-ignored (needs AA_GAME_ROOT).
Round 4, one writer and one deflater:
- The deflater moved to aa-formats (zlib_deflate.rs), the lowest crate,
  with zlib 1.1.4's sync flush (not used by the chunk compressor, whose
  methods call deflate with 0 and 4 only; AA.exe's sync flushes are on the
  stream at +0x74 of another class: 8D8830, 915D00, 917E10, 917FF0) and a
  ZlibEncoder stream type shaped like flate2's.
- aa-metagame chunk_file.rs (films, campaign saves) now compresses through
  it: its one import line names aa_formats::zlib_deflate instead of flate2,
  so its compressed chunks are the original's bytes. Its flush publishes the
  bytes emitted so far and does not touch the stream (the chunk compressor
  has no flush; 8B7520 hands every output byte to the file after each
  deflate call).
- aa-editor map_file.rs is now built on that chunk file writer. The
  differences are settled: names follow chunk_file (an empty name is the
  original's error; a zero unit or 32 units or more is refused, as wcsncpy
  would store it unterminated); flag bits 1..63 are one value per file
  (stale_flag_bits / set_stale_flag_bits), which the shipped-map test now
  checks on all 80 maps. rewrite_map(path, bytes) reads a map and writes it
  back.
- The client's editor entry (crates/aa-bevy/src/editor/): RequestEditor
  shows the Editor interface and the top bar; OpenLoadDialog opens
  MenuFileDialog over our profile's data/Maps (AVar PathMaps); FileOK asks
  the load confirmation in the editor's message box; EditorSave and
  EditorSave "<file>" write the loaded map through rewrite_map. The blank
  map, the world view, the minimap, the MenuWidget class (labels and
  drop-downs of the top bar) and every tool are not built; the UI script's
  editor_command step sends the File menu's commands for now.


6. PORT PLAN (smallest provable step first)
-------------------------------------------
1. Done: the container and the compressor (section 5).
2. Chunk payload writers, one chunk at a time, each the inverse of our
   reader for it and proven the same way (decode every shipped map, encode,
   compare the chunk's bytes): first the self-contained ones (MAP,
   MAP_HEADER, HEIGHT_MAP, MOUNTAIN_HEIGHT_MAP, TERRAIN, SUN, MAPSTRINGS),
   then the world chunks (IDMGR, RULES, TEAMS, KINGDOMS, OBJECTIVES,
   RESTRICTION_GROUPS, TRIGGERS, DENIZEN_GROUPS, the thing lists and their
   indexes, UPDATABLES, MISC_DATA). Then 4F2DB0 itself: the order, its
   branches (the USED_REQUIRED_OBJECTS pair, METAGAME after KINGDOMS,
   STATIC_THINGS) and MapSaveCompressed. At the end a whole map loaded with
   our loader and saved with our save is the shipped file.
3. The editor session: SetSessionEditor and the editor decree orders, one
   group at a time (scenario and sides first, they only change data;
   terrain brushes and object placement next). Proof: drive the original
   editor on a hidden desktop to make one change, save, and compare its map
   with ours after the same order (mask flag bits 1..63 of the records).
4. The screens in the client, from the layouts in section 2 with the aa-ui
   TGI engine: main menu RequestEditor (replace the NOT_BUILT entry in
   aa-bevy ui/front.rs), the editor interface and top bar, the layer panels,
   then the dialogs. Compare with screenshots of the original editor.
5. Test play (File, Test): save to the temporary map, play it, "Exit to
   Editor?" back.
6. New map and random map in the editor (the random map generator is
   already ported exactly), Verify Map, Prepare Map, Add Noise, Beautify.


7. UNKNOWN (not yet found in the original)
------------------------------------------
- Found in round 4: RequestEditor opens a blank generated map
  (CreateBlankMapGenerateTask); the load dialog lists Profile/data/Maps; the
  save dialog (6ADE70 -> 66AB30 in save mode over PathMaps) commits
  EditorSave "%s". Still unknown: the blank map's generator and settings,
  and the File menu items the code adds (6ACC00 builds the top bar).
- The original's own save is a world save, not a container round trip.
  Loading (2v2)Berlin.RMP from Profile/data/Maps and running EditorSave
  rewrote that file (439,158 bytes; the shipped one has 439,309) and wrote
  (2v2)Berlin.TGI (the map strings, 818 bytes, beside it; its text differs
  from the shipped .TGI from byte 107). 18 of the 22 chunks are identical to
  the shipped ones after decoding and as stored; MAP_HEADER (from decoded
  byte 2658), GWORLD (from byte 4), TERRAIN (from byte 420505) and KINGDOMS
  (from byte 845) differ, and the record flags carry a new stale value. Our
  client's EditorSave writes the loaded chunks back unchanged, so it gives
  the loaded file's bytes, not the original's: the world save (4F2DB0) and
  the strings file are the next port.
- The file EditorMapStringsSeparate writes the map strings to.
- The editor's hotkeys, and which orders Undo and Redo cover (the strings
  undo_place, undo_paste, undo_drag and their redo_ forms exist, 6A86A0 to
  6A89C0).
- The exact write sizes of the original's map save. One write and many small
  writes give the same stream except, rarely, when the window slides at one
  exact input position; both forms match all shipped chunks.
