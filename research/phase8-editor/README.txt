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
User variables of the editor (aa-ui view_variable_registration.rs 158-163):
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
- Writes: .RMP maps. The map save (4F2DB0) writes the chunks in this order
  (the order of all 80 shipped maps; 68 have the bracketed pair):
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


5. DONE IN THIS ROUND
---------------------
crates/aa-editor (new workspace member):
- deflate.rs: the zlib 1.1.4 deflater, ported (the lazy matcher used by
  levels 4 to 9, the Huffman trees and block output, the zlib header and
  Adler-32 trailer). Streaming like the original: write = deflate with no
  flush, finish = deflate with finish.
- map_file.rs: the map file writer, the exact inverse of
  aa_mapworld::MapArchive: header, chunks in order from 0x0C, raw or level-6
  compressed payloads, 0x60-byte records. The stale flag bits are taken as
  data so an existing map re-encodes to itself; a new map writes 0 there.
- Proof: tests/shipped_maps.rs reads every .RMP in the original Data.rwd
  with our reader, writes it again and compares: 80 maps, 1736 chunks (1656
  compressed) are identical byte for byte. A second test feeds every
  compressed chunk to the compressor in small pieces (1 to 300 bytes, as the
  original's save writes field by field): all 1656 are identical too. Run
  with cargo test -p aa-editor -- --include-ignored (needs AA_GAME_ROOT).
Also useful elsewhere: films and saved games use the same chunk file. Our
aa-metagame chunk_file writer (used by the film writers in aa-parity and
aa-runtime and by the campaign saves) compresses with another zlib, format
compatible but not byte identical, and aa-play saved_write.rs writes its
chunks uncompressed. Both can use aa_editor::deflate (no dependencies) to
write the original's bytes.


6. PORT PLAN (smallest provable step first)
-------------------------------------------
1. Done: the container and the compressor (section 5).
2. Chunk payload writers, one chunk at a time, each the inverse of our
   reader for it and proven the same way (decode every shipped map, encode,
   compare the chunk's bytes): first the self-contained ones (MAP,
   MAP_HEADER, HEIGHT_MAP, MOUNTAIN_HEIGHT_MAP, TERRAIN, SUN, MAPSTRINGS),
   then the world chunks (IDMGR, RULES, TEAMS, KINGDOMS, OBJECTIVES,
   RESTRICTION_GROUPS, TRIGGERS, DENIZEN_GROUPS, the thing lists and their
   indexes, UPDATABLES, MISC_DATA). Then 4F2DB0 itself: the order, the
   optional pair, and MapSaveCompressed. At the end a whole map loaded with
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
- What RequestEditor opens first (a blank map, the New Map dialog, or the
  last map), and the File menu items the code adds.
- Where the editor saves and loads by default (the file dialog's folder).
- The file EditorMapStringsSeparate writes the map strings to.
- The editor's hotkeys, and which orders Undo and Redo cover (the strings
  undo_place, undo_paste, undo_drag and their redo_ forms exist, 6A86A0 to
  6A89C0).
- The exact write sizes of the original's map save. One write and many small
  writes give the same stream except, rarely, when the window slides at one
  exact input position; both forms match all shipped chunks.
