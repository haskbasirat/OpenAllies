# How the original plays films: specification and build brief

Written 2026-11-01 against AA.exe (image base 0x400000), tree at c2b4099. Addresses are AA.exe. Nothing here is copied pseudocode; it describes behaviour and points at the native code.

The brief for whoever builds this comes last: [section 12](#12-ordered-build-list). What happens on a sync mismatch is [section 6](#6-sync-checks-what-happens-natively-on-a-mismatch). What our client can do meanwhile is [section 11](#11-options-for-our-client-while-it-does-not-match-a-whole-film).

## 1. How to read this document

Every claim carries one of these labels.

| label | meaning |
|---|---|
| **[O]** | Observed in the running original: QA copy, hidden desktop, posted keys, 2026-11-01. The log or screenshot exists (section 10). |
| **[P]** | Observed in the running original after I deliberately changed one value in the memory of my own QA process. |
| **[C]** | Read from the pseudocode or instructions. Not exercised live. |
| **[I]** | Inference. Check it before relying on it. |

Posted mouse clicks do not reach the engine, so everything that needs a mouse is [C] only: the Keep Watching button, the End Film button, Replay, the observer buttons, clicking on units.

Field names used below:

| field | meaning |
|---|---|
| Session = `[0xB36550]` | KKC_Session |
| Session+0x54 | pending session state; 4 = film |
| Session+0xE4 | current session state; 2 = game, 4 = film, 5 = metagame battle |
| Session+0x58 | map type; 2 = load |
| Session+0x60 | launch type (names 0x4982D0): NONE, RANDOM_MAP, LOAD_MAP, LOAD_SAVED_GAME, LOAD_SAVED_MISSION, LOAD_FILM = 5, EDITOR |
| Session+0xF4 | networked game [I] |
| Admin = `[0xB36558]` | TGC_AdminNode |
| Admin+0x268 | film writer stream; null when not recording |
| Admin+0x26C | film reader stream; null when not playing |
| Admin+0x284 | current ordering player |
| Admin+0x288 | film ordering player kept between records |
| GWorld = `[0xB36534]` | the world |
| S = `[0xB36560]` | the simulation synchronizer (`SYNC_LOG.md`) |

The film state 4, the film launch type 5 and the map type 2 are three different enums. Do not mix them.

## 2. Summary

1. A film is a small file (a few KB in the QA copy) in `Films/`: the map header and strings, the session players, and one order record per world tick. It holds no world. The world is rebuilt at load: a random map by running the generator again [O], a scenario map from its source file (`SYNC_LOG.md` section 9).
2. You reach a film from staging (Film button, then a file dialog, then Launch) or from the main menu (Film button, file dialog, straight to loading [C]).
3. Playing is a normal simulation with one difference. At the start of each world tick, one record is read from the file and its decrees run with the film flag set. After the world update nothing else changes. [C]
4. The viewer is an observer: no kingdom, whole map visible, camera at the map centre. Keys 1 to 9 and the binocular buttons switch the view to one kingdom's fog and HUD. Nothing else the viewer does reaches the simulation. [O] for the view, [C] for the order gate.
5. Speed goes from 1/16 to 16 in a film, against 1/4 to 4 in a game. Pause, chat and Save stay available. [O] for speed and pause down to 18%, [C] for the rest.
6. Natural end: the read past the last record throws TGC_FileException, which is caught and shows "End of Film" with Exit to Scoring and Keep Watching. [O]
7. Sync mismatch: at the first checkpoint that differs, the original logs `FILM OUT OF SYNC(n)`, drops the film, and shows "Out of Sync with Film" with one Ok that goes to Scoring. The world keeps running behind the box, and it keeps running after any film end. [P]

## 3. Film files on disk

- **Folder.** AVars `PathFilms = Films/`, resolved under the profile data root: `Profile/data/Films/` in the QA copy. [O]
- **Which profile, 0x85B040** (from 0x85B690 in KKC_Application::Init 0x4196A0). [C] SHGetFolderPathW through shfolder.dll (0x85A9D0) with CSIDL_PERSONAL gives `%USERDATA%`. Empty: `<exe folder>/Profile` (0x85A910, 0x85ACA0). Otherwise `<personal>/<ApplicationIDS>` ("A&A"), backslashes turned to slashes; the resources are scanned at `<that>/data/`. [O: the log's `Personal Folder` line] The QA copy's AA.exe has byte 0x85B0AA patched from `74` to `eb`, which forces `Profile`.
- **Our client** lists two folders (`aa_bevy::ui::custom::film_dirs`): its own `<own profile>/data/Films`, where it records and may delete, and the folder the rule above gives the original, read-only: Delete is off for its rows and nothing is ever written there. A film under some `<profile>/data/Films` plays on that profile's data overlay.
- **Names.** `<name>.RFM` (extension constant 0xA4772C, appended by 0x85AB30) plus `<name>.TGI` beside it. The TGI is a text file holding the `map_strings` table (`File Films/LastFilm_1.TGI TextData 'map_strings' loaded 33 strings`). Its timestamp is the film's start; the RFM's is its end. [O]
- **Automatic names (rotation), 0x76FE60.** The base is the localized `last_film_filename` ("LastFilm"); if that is empty, AVars `LastFilmName = LastFilm.RFM`. The name is base + `_%d` (format 0xA93F98) with UVar `LastFilmIndex` clamped to 0..queue-1. Then `LastFilmIndex = (index + 1) % FilmRotationQueueSize` (AVars: 5). The result is kept at Session+0x150. Called again without the "next" flag, it returns Session+0x150. [C]; the 5 files `LastFilm_0..4` are on disk. [O]
- **Layout.** TGCK chunks `MAP`, `MAP_HEADER`, `MAPSTRINGS`, `SessionPlayers`, `Orders`, documented in `ROADMAP.md` 5.1 and read by `aa_parity::Film`.
- **A locked film is skipped silently.** While another game was recording `LastFilm_2.RFM`, opening the dialog logged `FileException ... Sharing Violation` and the entry was missing from the list, with no message. [O]

## 4. Reaching a film

### 4.1 Main menu [C]

The main menu has a Film button. [O: `00-main-menu.png`]

- `MainMenuButtonFilm` (0x64F730, `main.tgi`) runs `RequestFilm`, registered in 0x650A80 with handler 0x64F3F0, which calls 0x64EFF0.
- 0x64EFF0 sets up the local session (0x7513F0(3, 1)) and switches to menu mode 3 (0x7840B0(3, 0, 0), 0x75D2D0, 0x74EEC0). That shows the ChooseFilm interface. [I] for what the calls mean.
- `QuickStartFilm` is registered in the same place with 0x943800, an empty function. It does nothing.
- ChooseFilm (`UI/Menus/choose_film.tgi:ChooseFilm`, class 0x6455B0, built in 0x652D80) draws no list of its own. The template says so: "we don't actually want an interface as we use the staging file dialog". The `FilmListbox`, `FilmInfoPanel` and `FilmButton*` widgets in that file are not used by this route [I]. It opens the same file dialog through 0x68DB60 in mode 5, with the staging flag off (0x644F5C).
- `FilmOK "<file>"` runs 0x645200:
  1. It requires menu mode 12 (0x40CEB0 / 0x40D8E0) and a non-empty name.
  2. 0x59F570 opens the file. A missing or unreadable file shows `load_error_missing_map|s` ("File missing: '%s'", 0x66F390).
  3. 0x777380 finds the source map. If none is found it shows `unable_to_select_film_source_file_not_found|s` ("Source map for film '%s' missing.", 0x66F4F0).
  4. Otherwise it builds and sends KKC_SetSessionLoadFilmOrder (0x644C90 / 0x644AA0 / 0x644A20), calls 0x652B10 and closes the dialog (0x644F70).
- 0x652B10 issues the state change to 4 and the launch orders. So this route skips the staging screen and goes straight to loading. [I from the calls: 0x4157A0(4), 0x40E790, 0x40DF00]
- `FilmCancel` 0x645540 closes the dialog and returns to where it came from.

### 4.2 Staging

- `FilmButton` (0x65D350; `staging.tgi` L 794, T 564, R +181, B +44; tooltip "View a Film"; label "Film") runs `SelectLaunchType LOAD_FILM`. [O: `02-staging-default.png`, log `(SelectLaunchType LOAD_FILM)`]
- The handler is 0x65C1C0, which calls 0x68DB60(5, staging flag 1) (call site 0x65BEDD), which opens `StagingFileDialog`. The main-menu route passes flag 0.

### 4.3 The file dialog (`UI/Shared/staging_file_dialog.tgi:StagingFileLoadSavedDialog`)

Same class as the scenario-map dialog our client already has (`crates/aa-bevy/src/ui/dialogs.rs`): ctor 0x68B0D0, Init 0x68D280, Activate 0x68B6B0, list 0x68E340, preview 0x68A820 then 0x6612E0, Commit 0x68EF50. The widget comes from 0x68DB60: mode 1 `RandomMapDialog`, mode 2 `StagingFileDialog`, the others (3, 4 and this mode 5) `StagingFileLoadSavedDialog`. [C] [O: its layout, Load, Delete, Cancel under the list]

**Mode 5 setup, 0x68BB70.** [C]
- Directory PathFilms, subfolders off (+0xDC = 0), extension RFM.
- +0x11C = 1 (pick the newest when nothing is remembered).
- Remembered name from the preferences block `[0xB39D04]+0x18` into +0x120 [I]. Maps use +0xC, saved games +0x10, RMC +0x14.

**List, 0x68E340.** [C] unless noted.
- Every file of that extension in the directory is opened with 0x59F570 and listed if its header reads.
- Row text is the file's name without extension. If its first `len(last_film_filename)` units (0x41D2F0 is a left-substring with that length, read from the instructions at 0x68E827) equal `last_film_filename` case-insensitively, the row becomes that prefix, a space and the film's time, so `LastFilm_1 (2)` is a LastFilm row too. The time is the map header's +0x18, a FILETIME set by the header's constructor (0x59E210, 0x5A14C0), not the file's; 0x6FBF50 formats it with GetTimeFormatW and GetDateFormatW in the user's locale, without converting a time zone. [O: `LastFilm 3:39:08 PM 10/24/2026`]
- Rows appear in alphabetical order of their text, so `12:53:07 AM` sorts before `3:39:08 PM`. [O]
- Rows with equal text get ` [file]` appended (`%s [%s]`).
- Preselected row: the remembered name (+0x120) if it is in the list, else the newest by 0x59E210 if +0x11C is set, else the first row. [C] [O: `LastFilm_2` selected while readable; `LastFilm_1`, the newest, once `LastFilm_2` was locked]

**Preview on the right, 0x6612E0.** [O: `03-film-file-dialog.png`] `<name> (Film)` (`staging_map_preview_append_film|s`, 0x65F9D0), then Source (map name), File, Time, Size (`384x384`), then one row per kingdom with a colour pip and the name (`Player`, `Random`). A film's name key is empty and the name shown is the file's [O], how it gets there is not read [I]. Source is header+0x38: an RMC's name, or the map's name, or the file's name when the map does not open (0x660850). The kingdoms are the header's own parameters that are not independent, have +0x28 set and +0x29 or a colour (0x65F880). Their names are the display keys through the film's own `MAPSTRINGS` (`side01 = "Player"`), which is why a computer slot reads `Random`; the colour and glyph are the kingdom's +0xC string (0x50F970).

**Edit box.** Shows the selected row's text, fully selected, with focus. [O]

**Buttons.** Load, Delete, Cancel. [O] Hotkeys `enter StagingFileDialog Commit`, `esc StagingFileDialog Cancel`.

**Delete** (around 0x68C420). [C] A box `delete_confirm|s` with Ok and Cancel; Ok runs the command `FileDelete "<name>"`, 0x68C5B0, which removes the file, or shows `ui_file_dialog_file_delete_failed|s`, and refills the list. Our client also removes the `.TGI` beside the film and offers Delete only on rows of its own folder. [client: proven on the hidden desktop; native: not run]

**Commit, 0x68EF50.** Opens the name in +0xE4 (0x59F570). Then:
- Staging flag on and not in a running game (Session+0xE4 not 2 or 5): mode 5 builds KKC_HostSetLoadFilmOrder (0x61E930, 0x68A370), sends it and closes.
- Otherwise, mode 5 runs the command `FilmOK "<name>"` (the main-menu route).

**Typing into the edit box.** [O] These are native quirks; the list needs no typing.
- A bare name such as `LastFilm_3` or `9` shows "File missing: 'LastFilm_3'" with Ok. Enter closes it and keyboard focus is then lost: typed characters no longer reach the edit box until the dialog is reopened.
- A name with a folder, `Films/LastFilm_3.RFM`, **crashes the original**: `thread MAIN caught unhandled exception ... Not a KKC_MapResource: 'Films'`. Our client must not copy the crash; see section 13.

### 4.4 Staging with a film selected [O: `05-staging-film-loaded.png`]

- The team format dropdown and the teams are gone. The list shows only `Observer` and one row, `Player`. No slot, nation or colour controls.
- The right panel shows `LastFilm_1 (Film)`, `320x320`, and the film's kingdoms with colour pips.
- `launch_film_text` ("Play Film") is loaded into +0xF8 by LoadData (0x68DA35) and nothing reads that member: the button stays `Launch`. [C] [O: `05-staging-film-loaded.png`]

Session work on the way (0x68DFD0 ProcessRequest, 0x75F470 Process, then 0x644FE0 / 0x75DE80, then 0x779B30). [O] in the log: `HostSetLoadMapOrder::Process() - ComputeChecksum`, `GetSourceFilename: looking at source_res Films/LastFilm_1.RFM`, `KKC_Session::SetSessionLoadFilm(Films/LastFilm_1.RFM)`, then the AI players are created and destroyed, `SetNewState(4)`.

- 0x75F470 refuses during a launch countdown (`session_order_rejected_currently_counting_down`) and acts only on the host. With no source map it writes the same `unable_to_select_film_source_file_not_found|s`, here to the message log (channel 0x11), not a box. [C]
- 0x779B30 refuses non-hosts (`session_order_rejected`) and countdowns. It then sets: Session+0x54 = 4, +0x58 = 2, +0x5C = film name, +0x60 = 5, +0xC8 = +0xC9 = 1, +0xFC = film CRC, +0x108 = source file name, +0x104 = source CRC, +0x10C = source flag. It copies +0x30 / +0x34 of the order's creator block into the load creator's +0x3C / +0x40, then refreshes the staging state (0x781DD0, 0x7791F0, 0x770280). A client checks the transfer (0x770B70, 0x7770B0); those are network paths. [C]
- The film is not re-read for the roster: the staging panel only reads the header. [I]

### 4.5 Launch and loading [O unless noted]

Log, in order:

`StagingLaunch` → `KKC_Session::Launch()` → `NET: FinalizeLaunchSettings - Loading File: <Films/LastFilm_1.RFM>` → `NET: CreateLaunchTask - Launching with maptype = 2` → `CreateLoadFilmTask()` → `BeginTask() vworld_init_req = 1` → `RMG: rmc_southerneurope Try #1` (random-map film only) → `KKC_Session::ChangeToState(4,6,1)` → `g_sim->SetFrozen(true)` → `APP: changing app_state 12 -> 6` → `creating map size 320x320` → `[4610] VWorldState::StartMission, observer, lookat=(160,160)` → `NET: Task complete` → `Creating film player id=1..4` → `Finished Loading`.

Pressing Continue (Enter, `UserReady`): `ALL_TASKS_SUCCEEDED` → `g_sim->SetFrozen(false)` → `APP: changing app_state 6 -> 8`. The world does not tick until then.

- **Loading screen.** Title `LastFilm_1`, an empty preview rectangle, the usual art, `Finished Loading`, Continue. [`10-loading-after-60s.png`]
- **FinalizeLaunchSettings 0x776B20** with +0x54 == 4: opens the film as a resource (0x5AEA90, 0x59F720) into Session+0x20; `m_replaying_film` asserts it is non-null. [C]
- **CreateLaunchTask 0x775EE0**, map type 2 and state 4: calls CreateLoadFilmTask 0x76C0A0, which makes KKC_ResourceManagerTaskLoadFilm (ctor 0x59E690, Run 0x5A3980), queues it (0x5AED10), then 0x74DAF0(1). [C]
- **Task Run 0x5A3980.** [C]
  1. Open the header (0x59F330 raises if unreadable). 0x5A28B0 prepares the creation.
  2. Version check, unless UVar `FilmForcePlayback` (default false) is set: the header's version string (+0x58) against the running version (`[0xB3888C]`), wcscmp; a difference fails with `failed_film_wrong_version|s` ("Film is from a different version (%s)."). Then the header's build (+0x50, a binary32) against `[0xB27A80]`, compared as floats; a difference fails with `failed_film_wrong_build|g`.
  3. The creator's virtual +0x24 creates the world (the failure text is `unknown CreateGWorld failure`).
  4. If GWorld+0x48 == 0, StartMission 0x4FDCE0 (`StartMission failure`).
  5. Finish via 0x4FBCE0, 0x5A3D30.
- **Source map check, 0x5A2B90.** The load creator's source resource (creator+0x50) is null: `failed_film_source_map_missing|s` ("Source map missing (%s)."). Only outside film state (Session+0xE4 != 4) does it run the campaign checks. [C]
- **Camera, 0x4F61A0.** Viewer has no kingdom: look-at = half the map width and height. [O: (160,160) on 320x320; `VWorld::Init, set camera to (160,160)`]
- **Camera zoom, 0x7010D0** (from 0x43FB44). Reset 0x700E30, then 0x7005F0 with UVar `CameraScaleZoom`, clamped to 0.7..2.1. So a battle opens at the profile's last zoom. [C] The QA profile holds 2.1, which is why the film shots show a wider view than a fresh profile; `Data/UVars.tgi` has 0.5, so a fresh profile opens at 0.7.
- **Failure of the load.** The task's text travels on session event 8 GWORLD_CREATE_FAILED (0x773AE0, `failed_to_create_map` when it has none, after event 10 SOME_TASK_FAILED). TGC_AppState 0x414620, case 8, queues it as a message (0x414520, 0x40F580) and leaves the state (slot +0x24). [C] Our client shows that text in a box with Ok on the page the launch came from. [client: proven on the hidden desktop with a film of another version, both routes; native: not run]
- **OnGWorldReadyForState 0x7757F0**, state 4: [C]
  1. Calls 0x76E1D0, which opens chunk `SessionPlayers`, reads the count, and for each row reads id (4 bytes), name length (2), UTF-16 name, kingdom index (4) and calls CreateFilmPlayer 0x76CD00.
  2. Opens chunk `Orders` and hands the stream to 0x74B320, which sets Admin+0x26C = stream and Admin+0x288 = 0.
  3. 0x7A0D40.
  4. Session+0x168 = (header+0x3C string non-empty). See section 8.
- **CreateFilmPlayer 0x76CD00.** A new session player from 0x7661D0 with ai = 0, implicit = 0 and **film flag +0xE = 1**, its id and name, and its kingdom from GWorld+0x148[index] when index >= 0 and in range. Added to the session list; Session+0xC8 = 1. Only a player with +0xE gets sync validation (section 6). [C]

## 5. The playback loop

### 5.1 Frame and tick

**The frame, 0x75CA70** (called from 0x765410). [C] It runs 0x776540 and 0x75C9A0, and leaves if Admin+0x2C0 is clear or there is no clock (Admin+0x158). When the clock reaches the earlier of the next admin tick (Admin+0x28C) and the next world tick (0x76AF90):

- World tick first: 0x752AE0, always (the guard 0x6A71F0 returns 0).
- Otherwise, the admin tick 0x755290, which adds 0.0625 to Admin+0x28C.

`SCHEDULER.md` covers this path for ordinary games. Films use it unchanged.

**The world tick, 0x752AE0, in this order.** [C]
1. Admin+0x268 set: record 0x74D640.
2. Admin+0x26C set: play 0x751A40.
3. 0x76F790: log `g_gworld->GetGameTimeBeginTick()`, 0x41C180, GWorld update 0x4F9970, 0x76F600.

So record r runs right before update r + 1, the rule `aa_parity::replay` already follows.

**One record, 0x751A40.** [C]
1. Read 4 bytes, the length.
2. Read that many bytes.
3. Admin+0x284 = Admin+0x288. Session+0x24 = 1 (the film flag).
4. 0x74D710 runs the packet's decrees.
5. Session+0x24 = 0. Admin+0x288 = Admin+0x284. Admin+0x284 = 0.

`aa_orders::net::admin::AdminNode::play_film_record*` is the port of this function.

**Decree loop, 0x74D710.** [C] Decodes each decree (0x766F20) and runs it (0x766CE0). While Session+0x24 is set and Admin+0x2E5 is set, it stops. 

Film decrees do **not** go through the request path, so the film gate of section 8 does not apply to them.

**The ordering player.** `TGC_SetPlayerOrder::Process` 0x75D710 looks the id up with 0x76B8F0(id, film flag) and stores it in Admin+0x284. An unknown id during a film logs `Film order with no ordering player %d`, calls the stop 0x751EB0 and sets Admin+0x2E5 = 1, which ends the loop above. So a bad player id produces the same box as a sync mismatch. [C]

### 5.2 The natural end [O]

The read at step 1 or 2 of the record throws when the chunk has no more bytes. The handler in 0x751A40 is at 0x751B07; its FuncInfo (0xB0F9C8) shows it catches **only TGC_FileException**. It pushes the notification type 0x13 END_OF_FILM with +0x38 = 0, then sets Admin+0x26C = 0.

Execution resumes after the record step, so the world update of that tick still runs. Any other exception type is not caught there. [C]

Observed: `LastFilm_1` has its last record at tick 272 (`aa-parity film`). `ALL_TASKS_SUCCEEDED` at 162.7 s, `END_OF_FILM` at 179.8 s: 17.1 s at 16 ticks per second. 272 / 16 = 17.0.

### 5.3 After the film is gone, the world runs on [P]

Admin+0x26C = 0 only stops the reading. Nothing freezes the simulation.

- With the "Out of Sync" box up, S kept growing: 1222, then 1959, then 2089 entries. [P]
- After the natural end, the log still shows sim events (`Swapping ID gbr_truck_supply (31) with gbr_depot_supply (2033)`). [O]
- No computer player acts: the SAI main is gated on Session+0xE4 != 4 (0x7A0080). The kingdoms' running production and orders simply continue. [C]/[I]

## 6. Sync checks: what happens natively on a mismatch

**Where the checkpoints come from.** The admin tick 0x755290 counts admin ticks and on the 17th (counter > 0x10), when Session+0xE4 == 2 and on the host, calls 0x573580, 0x77CBE0 and 0x74E670(0). That sends a decree which by every sign is 299/311 `KKC_SetSimSyncValueOrder` with {S last index, S hash}. [C], the identity [I]. Film state never emits them. Films have one every 17 ticks. [O: decoded, `ROADMAP.md` 5.1]

**Where they are checked.** The decree's Process 0x75D7F0 calls SetSimSyncValue 0x752F60(index, hash): [C]
- It checks only if the current ordering player has the film flag (player+0xE). Live players are never checked.
- 0x77D050 (`aa_orders::net::sync::Synchronizer::validate`) compares S's entry at that index with the hash:
  - S disabled: passes.
  - Index beyond what S has seen: **fails** (`getting a value beyond what we've currently seen`).
  - Index older than the retained window: passes (`value already forgotten`).
  - Otherwise: pass only on equal hash.

**On failure.** 0x752F60 logs `FILM OUT OF SYNC(%d)` with the index and calls the stop 0x751EB0: Admin+0x26C = 0, push END_OF_FILM with +0x38 = 1. The rest of the record's decrees still run, because the packet is already in memory. [C]

**Proof in the running original.** [P] In a film at normal speed I XORed S's hash (S+8) in my own QA process. The next checkpoint produced:

```
FILM OUT OF SYNC(414)
[time 642.1] NetEvent Push: END_OF_FILM
[time 642.1] NetEvent Pop: END_OF_FILM
MessageBox::_Display(Out of Sync with Film)
```

The box has one button, Ok. Enter pressed it and the game went to the Scoring screen. S went on growing behind the box. There is no retry, no resync, no second chance. [`31-out-of-sync-dialog.png`, `32-oos-ok-result.png`, `log-film-run2-oos.log`]

**Dump files.** UVars `SyncLogSimPath` and `SyncLogSimRecordingPath` (`synclogs/synclog_film_record_`, `synclogs/synclog_film_replay_`, 0x41F9D0) name dump files; the path is 0x75BBC0 to 0x75B8D0 to 0x77D0D0. Not triggered in these runs. [C]

## 7. The end-of-film handler and leaving

### 7.1 The film state

The film runs in **app state 8**, class vtable 0xA6559C (ctor 0x651640, slot at +0xEC of the state list in 0x40CB40). It shares its id base `[0xB3974C]` with the normal game's state 7 (0x608C90, vtable 0xA56C2C). It differs in three slots: [C]

| slot | film | game | meaning |
|---|---|---|---|
| +0xC | 0x651E00, returns `film_viewing` | 0x60A110 | state name |
| +0x1C | **0x652020** | 0x414620 | notification handler |
| +0x24 | 0x652880 | 0x60A8D0 | finish path |

0x652020 and 0x651E00 are not in `functions.tsv` and have no pseudocode file: the decompiler missed them. Read them with a disassembler.

### 7.2 Handler 0x652020 (type 0x13 only) [O] [P] [C]

| event | on the host | on a non-host |
|---|---|---|
| +0x38 = 1, out of sync | Box via 0x66F4F0: `film_end_of_film_oos` ("Out of Sync with Film"), Ok runs `RequestFinished`. Enter works. | Box via 0x66F390: same text, Ok with no command. |
| +0x38 = 0, natural | Box built with 0x66D360 / 0x405B80: `film_end_of_film2` ("End of Film"). Button `film_exit_to_scoring` runs `RequestFinished`. If GWorld+0x275 == 0, a second button `film_keep_watching` with an empty command. Shown by 0x66E790. Enter and Esc do nothing: the box has neither an accept nor a cancel button. | Box via 0x66F390: `film_end_of_film2`, Ok with no command. |

- Layout [O: `14-end-of-film-dialog.png`]: the shared message box, text centred at the top, Exit to Scoring on the left, Keep Watching on the right.
- The box does not pause the world. [P]
- It is the only effect of the event. No text in the log panel. [O]
- Keep Watching only closes the box: the viewer stays in app state 8 with the world running and the film gone. [C]
- GWorld+0x275 is set by 0x4F1080 and read by the speed limit (0x76B3A0) and the game-over paths: the world's cutscene mode (`aa_play::triggers::cutscene`). [I]

### 7.3 Leaving

- `RequestFinished` in the film: handler 0x6125A0 accepts app states 4, 7, 8, calls 0x774C80, which returns 3 for a film host, and runs 0x652790. That issues mode 6 of the 0xC menu event (0x7840F0(6), 0x7840E0), KKC_SetNewStateOrder(1) (0x4157A0, 0x40E790), 0x40E6E0(4), 0x40DF00. [C] Slot +0x24 (0x652880) is the same sequence without mode 6 and with 0x40E6E0(3). [C]
- Result [O: the `numpad-8 game RequestFinished` test binding]: `KKC_Session::SetNewState(1)`, `ChangeToState(1,12,0)`, `CreateLaunchTask - Launching with maptype = 4`, `SetFrozen(true)`, VWorld torn down, `APP: changing app_state 8 -> 12`, `connect Scoring`. [`26-request-finished-in-film.png`]
- **Game menu** (Esc, `ButtonMenu`) [O: `21-game-menu-end-film.png`]: Options, Load (disabled, 0x76AD40), Save (enabled, 0x76AD10), Connection Status (disabled, Session+0xF4), a bar, **End Film** (`ui_game_menu_leave_game_host_film`, 0x774C80 case film host), Quit Axis & Allies, Return To Game. Opening it pauses a non-networked game (0x61D260). A non-host in a film gets `ui_game_menu_leave_game_non_host_observing`. The End Film click is untested.
- **Scoring in a film** [O]: same screen. Scores from the world as it stood. Kingdoms are named as the film renamed them (`Wingate AI`, `Rommel AI`, `Konev AI`). Buttons Replay and Exit. **No Save Film** (0x658FF0 shows it only if 0x772BB0 finds a film, and Session+0x150 is empty when viewing). [C] for the reason.
- **Replay** (0x6590B0): with +0x54 == 4 it calls 0x652B10, restarting the film. Untested. [C]
- **Exit** (`ScoringExit`): back to staging, film still selected. [O: `27-after-scoring-exit.png`] Launch runs it again from a fresh load; speed is back to 100% (`Game speed changed to 100%`). [O]

## 8. What the viewer can do

### 8.1 The order gate

`Order %s rejected order that would interfere with film` has 100 sites, one per instantiation of `TGC_RequestDecreeOrder<T>::ProcessRequest`. The rule is one line in the template (Orders.h line 0x11C): after the two common checks (a known ordering client, versioning), if 0x766C70 (Session+0xE4 == 4), log and drop the request. It applies only to live requests, never to film decrees. [C]

There are 178 instantiations in total, so 78 have no such check. The full census from `string-references.tsv` is below.

**Rejected (100).**
- Every `KKC_Editor*` decree that has this check (68 of them).
- Cheats: CheatExperience, CheatFog, CheatLose, CheatMoney, CheatNoWait, CheatWin.
- World commands: TellActorAbility, TellActorAttack, TellActorCommand, TellActorMove, TellSelectedCommand, TeamCommand, FactionUseAbility, TellMGCommand, TellMGSwapPlayerKingdom.
- Selection: SelectionSet, SelectionAdd, SelectionRemove, SelectionClear, SaveGroup.
- Identity: GamePlayerKingdom, GamePlayerObserver, TGC_SetPlayer, UpdateKingdomName, MidgameCreateAIsAndSessionPlayers.
- Other: SetSimSyncValue, HeardScriptCommand, NotifyTriggerManagerOfVideoCompletion, VoteToResign, MGHelpPopupShown, MGPurchaseSelectionSet, MGSetFastAITurn.

**Allowed during a film (78).**
- Speed: SetGameSpeed, SpeedUp, SpeedDown.
- Pause: SetPaused, TogglePaused.
- Chat: PublicChat, PrivateChat, TeamChat.
- Saving: GameSave, MetaGameSave.
- SkipCutscene, NotifySelectedKeepPlaying, NotifyHostHasEndedTheGame, NotifyNonHostIsNowHost.
- All session, lobby and staging setup: SetNewState, Launch, countdown, SetSession*, HostSet*, SetKingdom*, SetPlayer*, SetMap*, Kick, Ban, VoteToKick, join and leave, Challenge, sync and status orders.

The census is reproducible by comparing the `ProcessRequest` strings with the `interfere with film` strings in that file.

### 8.2 Whose view and fog

- **Local player.** Stays an observer, no kingdom (`[4610] ... observer`). The film players are other session players. [O]
- **View kingdom.** 0x5C0E50 returns the local player's kingdom if it has one, else interface+0x1E0. All HUD, fog and display code asks this function (90 call sites). [C]
- **No view kingdom (the start).** No fog: 0x444EF0 and friends treat a null kingdom as everything visible. All units on the minimap. No resource bar, no experience counter, no floaters. The objectives and technology panels open empty. [O: `12-ingame-3s.png`, `22-f1-objectives.png`, `23-f2-technology.png`]
- **With a view kingdom** (keys, below): that kingdom's fog on terrain and minimap, its resource bar (`507(+40)  +20  +10  1/100  0/0`), experience (`211/5000` top left), its supply border, its idle-truck floater. [O: `40-observe-kingdom-1.png`, `41-observe-kingdom-2.png`] Two different kingdoms gave two different views.

### 8.3 The observer buttons and keys

- When the local player has no kingdom and nothing is selected, the action panel update 0x5F8E00 calls 0x5F7E50, unless Session+0x168 is set.
- 0x5F7E50 walks the major kingdoms (GWorld+0x154, count +0x158), shows up to 10 per page from page start +0x1D4, skipping the slot in +0x178, each by 0x5F7150: binoculars tinted with the kingdom's colour, command `SelectObservingKingdom <i>`. Then a page arrow `SelectObservingKingdomPage <next>` and the globe `SelectObservingKingdom` with no argument (the no-kingdom view, button definition `SelectObservingKingdomDefault`). [C]; 4 binoculars, arrow and globe [O: every in-game shot]
- Handlers: 0x5F4780 (only if the local player has no kingdom; no argument clears, else sets the kingdom through 0x5C0E90), 0x5F4750 (page). Registered in 0x5F9520; button names in 0x5F60D0. [C]
- **Keys**, `Localization/Hotkeys/Actions/hotkeys_standard_aa.txt` lines 531 to 542: `0` no kingdom, `1`..`9` kingdoms 0..8, `q` 9, `w` 10, context `game|action|verify|1`. Keys 1 and 2 worked. [O: log `(SelectObservingKingdom 0)`, `(SelectObservingKingdom 1)`] The modal box blocked my test of `0`.
- 0x5C0E90 forces no kingdom when Session+0x168 is set, and 0x5F8E00 then clears the panel (0x5F5300). Session+0x168 is set only in 0x7757F0, from the film header's +0x3C string. That string goes with +0x40 in the campaign checks of 0x5A2B90, so this looks like "a film of a campaign mission: no kingdom switching". [I] Not exercised.

### 8.4 Speed

- SVars: `GameSpeedMin -2`, `GameSpeedMax +2`, `GameSpeedFilmMin -4`, `GameSpeedFilmMax +4` (registered in 0x5065D0; pointers at `[0xB39AB0]`+0x60 / +0x64 / +0x68 / +0x6C). The value is the base-2 exponent of the rate.
- Limits: 0x76B360 (minimum) uses the film value when Session+0xE4 == 4 and Session+0xF4 is clear. 0x76B3A0 (maximum) also uses the film value when GWorld+0x275 is set. [C]
- Keys: `minus`, `numpad-minus`, `plus`, `numpad-plus` (`vworld SpeedDown/SpeedUp`), `backslash` (`GameSpeed`), and the options slider (0x675F80, 0x6A2670 take the same limits). [C]
- A step, 0x76F530: host only, not when GWorld+0x275 is set; round the current value to the nearest half, then add or subtract 0.5. The set function 0x76F370 clamps, stores Session+0x20, recomputes the rate (0x76B230) and, when the value changed, writes `networking_change_game_speed|f` ("Game speed changed to %.0f%%") to the message log. [C]
- Observed: 71%, 50%, 35%, 25%, **18%** going down (a game would stop at 25%), and 141%, 200% going up. [O: `20-film-speed-minimum.png`, log] The bottom (6%) and top (1600%) were not reached.
- The KKC_SetGameSpeed order applies only through 0x75D810 (0x76AD90 gate). [C]

### 8.5 Other

| thing | behaviour | evidence |
|---|---|---|
| Pause | `pause` key: `Paused` in red in the view, world held, pause button lit. | [O: `28-paused.png`] |
| Game over boxes | The result box 0x440190 returns in a film, after its first branch (GWorld+0x10 == 1 runs `RequestFinished`); the observer box 0x4408A0 is skipped by its first check. | [C] |
| Computer players | SAI main gated off by 0x7A0080. | [C] |
| Selection | SelectionSet and friends rejected, so the local player never holds a selection from clicks. Tab ran `CycleSelection 0`: nothing selected. | [O] for Tab, [C] for the rest; clicking untested |
| Unit commands | Rejected. | [C] |
| Camera | Free: `vworld` camera hotkeys have no film check. | [C]; not moved in these runs |
| Minimap | Same widget; follows the view kingdom. | [O] |
| Panels | F1 objectives, F2 technology, F3 message log open normally. The pause button is lit while the menu and panels are open. | [O: `22`..`24`] |
| Music | With no kingdom: AVars `ObserverGameMusic = menu_background_music` (0x4F96D0). | [C]; the QA runs are muted |
| Chat | Orders allowed, so the chat bubble works. | [C] |
| Save | Save enabled in the menu; 0x776200 accepts state 2 or 4. | [C] for the save itself |
| Film decree effects | SelectionSet decrees change the film player's selection, not the viewer's. | [C] |
| Sync | Neither speed changes nor view switches disturb the film. 5 speed changes, then checkpoints kept passing for the rest of that film's run; keys 1 and 2, then a natural End of Film. | [O: no `OUT OF SYNC` in those runs] |
| Scripts/triggers during a film | Not traced. Open item. | |

## 9. Recording and saving

### 9.1 Recording [C], except the files on disk [O]

**When.** OnGWorldReadyForState 0x7757F0, state 2 or 5, and Session+0xF8 not 7 or 0xE (test and editor launches [I]).

**Open.**
1. Close any old writer (0x74B310, stream +0x28), then 0x773E30.
2. Up to FilmRotationQueueSize tries, each with a fresh name from 0x76FE60(1) (`opening film %s`): name + RFM under PathFilms; KKC_FilmResource::Create 0x5A3250 into Session+0x1C; 0x5A00E0; OpenFilmForRecording 0x76C310.
3. A failed try moves to the next index. All failures report `failed_to_create_film|s` (0x775A40).

**OpenFilmForRecording 0x76C310.**
- 0x5A01E0: header and strings.
- Chunk `SessionPlayers`: count, then per player id, name length, UTF-16 name, kingdom index or -1, with the log `Writing film header - session player ai=%d implicit=%d id=%d`.
- Flush, then chunk `Orders`, then 0x74D5F0: Admin+0x268 = stream, mirror zeroed.

**Per tick.** 0x74D640, at the start of 0x752AE0: finalize the mirror stream (0x74D0A0), write length and bytes, zero the mirror (+0x278 = 0, +0x27C = 3). Checkpoints come from section 6.

**Close.** 0x74B310 and the stream's close happen in 0x76ACE0 (called on saving, 0x776200), 0x76E0F0, 0x770500 (leaving states 2 and 5), and 0x7757F0.

### 9.2 Saving a film [C]

- **Scoring.** `SaveFilm` is visible only when 0x772BB0 is true, command `ScoringSaveFilm` 0x658F90, which runs 0x68DCE0(5, 0, 0): `StagingFileSavedDialog` in mode 5. `FilmCancel` 0x658F80 runs 0x68C190.
- **Debriefing.** `SaveFilmButton` / `DebriefingSaveFilm` (0x6014B0, 0x600FE0, visibility 0x600CE0, 0x600D40).
- **0x772BB0.** Session+0x150 non-empty and `<PathFilms><name>.RFM` exists.
- **Commit 0x6911F0, mode 5,** calls 0x690A00: source = current film path, target = PathFilms + typed name; 0x8597B0 moves it (MoveFileW; a film at the target is deleted first by 0x5AD860, with a failure message), then runs `UpdateSaveFilmButton`, which finds no film any more. The `.TGI` stays: the saved films of a real profile have none. Before that, 0x6901D0 turns every character 0x859710 refuses into `_`, so the default name's `:` and `/` come out as `3_15_05 PM 3_30_2024`, as in a real profile. 0x7729E0 tests "target is the current film".
- **Default name** in 0x690E80 (mode 5) comes from the current time. [I]

### 9.3 In our client

Code: `crates/aa-parity/src/film_writer.rs` (the bytes), `crates/aa-parity/src/recording.rs` (the decree stream, the checkpoints, the header, the rotation), `crates/aa-bevy/src/film_record.rs` (the session's film, Save Film). Tests: `crates/aa-parity/tests/film_writer.rs`, `crates/aa-parity/tests/film_round_trip.rs`.

**One path.** A battle always runs on the decree stream, film or no film: `Recorder::step` is the admin tick and the world tick over `aa_play::World`, and every decree runs the way a film's does (its S entries, then what `aa_parity::Mapper` makes of it, applied at once). A test launch has a recorder with no file. So the world and S do not depend on recording, and a film replays the same world by construction. The plain `World::step` remains for tools and tests; its orders do not pass the Mind handlers' shaping (a move at a hostile actor as a kill, 0x527E30), so it is another battle as soon as anyone orders such a thing.

**The computer players.** `World::update` no longer applies the SAI's commands: they wait in `World::sai_orders` for the next tick's decrees, as the requests 0x7AF220 emits wait for the admin tick 0x752E60. The SAI's general powers go the same way (`special_ops::use_from_sai`). This moves every computer order one tick later than before, which is the native order.

**What is written.**
- A human's selection is 18 and 16, sent when it changes; a computer's is 333 before every order, 16 for further units, and 86 `group_behavior_enable` first with more than one unit (0x7AF220). Every order is 86 or 70: the request form 85 for a single building is written as a selection and an 86, because no real film holds an 87.
- 298 as 0x7509F0 and 0x74B360 write it: one for the node's player when a tick has requests, one when the requesting player changes. Record 0 holds the two the version acknowledgements leave. The stream of a real film has exactly this shape.
- 311 every 17th admin tick that ran (0x755290 with 0x76AEE0, so not while paused), the first in record 17, with {S last index, S hash} captured before the tick's decrees; 328 `<name> AI` for the computers in record 1.
- Not the pause or the speed. [O] A game of the original with the pause key twice and three speed steps left a film with none of 73, 75, 76, 80, 81: the 311s stay 17 records apart across the pause and go to 24 at 141% and 34 at 200%, because the admin ticks are real time and stop while paused. A 76 in a film crashes the original's playback (0x754880 asks 0x74B3C0 for the ordering client, which a film player does not have, and 0x74B080 reads through the null).
- The header: the map's own MAP_HEADER and text, the session's rules, the world's kingdoms (the header keeps what the world has, the creator every kingdom of the map), the version from AVars and the build 4129.7. MAP_HEADER and SessionPlayers come out byte for byte as the original's on 4 stock and 5 random real films (`film_header_*` tests).
- The order shapes are the real films': a census over every real film (`order_shapes_against_the_original_s`) shows that flag 5 goes with organizations (recruit 0x62 for a regiment, 0x42 for anything else, for people and computers alike), and that the two write differently, so `player_orders::native::OrderForm` says whose request it is. A person: a move on the ground is the default order 0x1C (position and direction: 0x620760 and its facing drag, the direction zero from the minimap; real films n=5705; 0x14, n=183, has no direction and 0x4DA570 only guesses a rally point from it), the placement drag sends a direction and a scalar of -1 (unpack 0x5C, construct 0x5E, recruit_location 0x76), an aimed order a scalar of 0 (0x51, 0x54). A computer: no scalar, no direction on unpack and construct (0x44, 0x46), hq_attach 0x41, repair 0x241, its orders in the shapes of the original's goals and reactive AI (guard 0x49, retreat 0x44, move at a thing 0x59, kill 0x41, set_formation 0xC0, explore 0x44; `COMPUTER-PLAYER.md`), and a member of an organization ordered through the organization: a real film's unpack selects the truck's organization.

**Not native, on purpose.**
- Since 2026-11-02 a right press is `Command::Hot`, the default order of 0x620760 (0x11 on a thing, 0x1C on the ground, 0x14 for a lone structure, with 0x400 for Alt and the drag's direction in the auxiliary words): no command named, the guess runs in the mapper on every machine. A person's selection is `Command::Select`, decrees 16, 17 and 18 as it changes.
- `Command::Attach` names its HQ, which `hq_attach` (31) cannot say (0x57B0A0 reads no field). It is written with the HQ in the word and the scalar flag set, which no real film has (`film_writer.rs` test), and only this port's decoders read it. In the original that decree cycles to another HQ.
- A metagame battle's start technologies, bonus stockpile and additional actors go into the kingdom rows' properties, resources and definitions (`recording::BattleExtras`), and a random film's rows are applied at its start. Which field is which is read from their names: there is no metagame film of the original to compare with. The placement of the additional actors stays the labelled rendition of `campaign/battle.rs`.

**A battle on a random map** starts as a film of it starts: `recording::random_battle` gives `RandomMapData::generate_world` the creator's lists as staging left them and the settings the creator keeps, so the kingdoms' picks are the first draws of the one stream (0x513410, 0x50E8B0, `GeneratedMap::world_picks`), and the generals the staging left open are those picks. `playback::generated_map` runs the same on the film's rows.

**The original opens our films** (2026-11-02, QA copy, hidden desktop). A film of the headless test on Berlin, computers only: staging shows `RecLane0 (Film)`, the loading screen, `Creating film player id=1..4`, and it plays. Every checkpoint through tick 391 passes on the original's own S, with our SAI's orders in it. With the recruits in their real shape it goes on to tick 476, 28 checkpoints, then `FILM OUT OF SYNC(647)` with `getting a value beyond what we've currently seen`: the first order that differs from what the original's SAI gave in a real film of this map is the unpack at tick 482 (another truck, another place), and from there the two battles are not the same battle. That first reading was wrong: the order named the truck where a real film names its organization. With the computers' orders in the real form the same headless film passes 77 of its 88 checkpoints, to tick 1309, and stops at tick 1326, index 1787 (`COMPUTER-PLAYER.md`).
A film of the headless test on a random map: `RMG: rmc_southerneurope Try #1`, the generator's log, it plays, and every checkpoint through tick 408 (24 of them, S index 638) passes on the original's own S, so the world it generated from our header is ours, with our SAI's unpacks and recruits in it. `FILM OUT OF SYNC(663)` at tick 425: same index, another hash. Before the recruits had their ParameterSet the same film went out at tick 34. A film the client itself recorded on Berlin, with a person's recruit, rally point and moves in it: it opens, the staging screen lists Player and the computers' names, it plays through tick 475, then `FILM OUT OF SYNC(658)`, beyond, at the same unpack as above.

**Save Film.** `Scoring/SaveFilm` and `DebriefingInterface/SaveFilmButton` show when 0x772BB0 holds and open `film_record::SaveFilmDialog` (`StagingFileSavedDialog`), Commit moves the film (0x690A00), after the name's characters are cleaned (0x6901D0). Only the client's own folder is listed, deleted from or written to.

## 10. QA log: what I ran and saw

**Setup.** QA copy `reconstruction/qa-runs/compare-20261022/game`; my own desktop `AAFILMSPEC`; my own state files; `QaDesk.exe` for posted keys; muted with `MuteProcess`, unmuted before each close. Two other agents had the QA copy running at the same time.

| step | key | saw |
|---|---|---|
| main menu to single player | F11 (0x7A) | |
| to staging | F12 (0x7B) | `02-staging-default.png` |
| open film dialog | numpad-8 (0x68), the existing test binding `StagingMenu "SelectLaunchType LOAD_FILM"` | `03-film-file-dialog.png` |
| commit the preselected film | Enter | `05-staging-film-loaded.png` |
| launch | F10 (0x79) | |
| continue when `Finished Loading` | Enter | `11`, `12` |
| speed | numpad-minus (0x6D), numpad-plus (0x6B) | log, `20`, `29` |
| menu | Esc | `21` |
| panels | F1, F2, F3 | `22`, `23`, `24` |
| cycle selection | Tab | `25`, nothing selected |
| pause | Pause (0x13) | `28` |
| finish | numpad-8 in game, the test binding `game RequestFinished` | `26` |
| leave scoring | numpad-3, test binding `Scoring ScoringExit` | `27` |
| view kingdoms | `1` (0x31), `2` (0x32) | `40`, `41` |
| out of sync | memory change of S+8 in my own process, Enter on Ok | `31`, `32` |

**Screenshots**, all at 1920x1080, in `qa-output/original-ui/film/`, plus my three logs (`log-film-run1.log`, `log-film-run2-oos.log`, `log-film-run3-observe.log`):

`00-main-menu`, `01-singleplayer`, `02-staging-default`, `03-film-file-dialog` (list and preview), `04-film-list-default-selection`, `05-file-missing-message`, `05-staging-film-loaded`, `07-typed-name-file-missing`, `10-loading-after-60s`, `11-ingame-start`, `12-ingame-3s` (viewer HUD, no fog), `14-end-of-film-dialog`, `20-film-speed-minimum`, `21-game-menu-end-film`, `22-f1-objectives`, `23-f2-technology`, `24-f3-messagelog`, `25-tab-cycle-selection`, `26-request-finished-in-film` (Scoring), `27-after-scoring-exit`, `28-paused`, `29-speed-up-keys`, `30-before-oos`, `31-out-of-sync-dialog`, `32-oos-ok-result`, `40-observe-kingdom-1`, `41-observe-kingdom-2`, `42-end-of-film-while-viewing-kingdom-2`.

**Pitfalls for the next person.**
- `Select Films/...` on a numpad key does not work in this dialog: the numpad also types its digit into the focused edit box. F2, F3, F4 did not select either: the preselection stayed. So I only played the preselected film, `LastFilm_1` (a random-map film, 272 ticks, `rmc_southerneurope`).
- After a "File missing" box, keyboard focus is lost; restart the game rather than recover.
- The Screenshots folder is shared: read each capture before trusting it. My numbers jumped when another agent saved one.
- Logs buffer in 64 KB blocks, so a recent line may not be on disk yet. Logs rotate to `-ok` names, and only the last few stay.
- Under load (three games at once), early keys are lost and some repeats are dropped. Wait for each screen.

**Not tested live.** Everything needing a mouse: the Keep Watching and Exit to Scoring buttons, End Film, Replay, Delete, the observer buttons, clicks on units, Save Film, the main-menu route. Also: the 6% and 1600% limits, version and source-map failures (the `failed_film_*` messages), sound.

## 11. Options for our client while it does not match a whole film

**The facts.**
- Natively, the first differing checkpoint ends the film. Checkpoints come every 17 ticks, about once a second.
- Our best current result, the stock-map film `Berlin2v2_0`, first differs at tick 493 (`NEXT-ROUND.md`), which is 31 seconds into a 4.7 minute film.
- `LastFilm_0` of the earlier set has 10,711 ticks, about 11 minutes.
- The three films of `ROADMAP.md` 5.1 and `LastFilm_1` are random-map films. The generator is exact on the stock data (12 of 12 world dumps, `NEXT-ROUND.md`), so they start in sync and differ where the simulation does.
- Even with hashes ignored, decrees name native GTID slots. Once ids shift, `aa_parity::Mapper` reports them as unresolved and nothing happens for those orders.

The native rule is not in doubt; what follows is a decision for the lead.

| option | what the player sees | what it costs | verdict |
|---|---|---|---|
| **A. Native: stop at the first mismatch.** Port 0x752F60 as it is. | "Out of Sync with Film" after about 30 seconds on Berlin, Ok to Scoring. | Nothing. Honest. | This is the end state. Required path for tests. Unusable as the default until parity. |
| **B. Report and continue.** Run the same check; on the first mismatch log `FILM OUT OF SYNC(n)` once, keep a count, do not stop. Decrees that no longer resolve are logged and skipped. The natural end still shows "End of Film". | The film plays to the end, but the battle diverges more and more: units idle, orders fail. No sign on screen. | A deliberate, temporary departure from the original, and an invisible one to a player. | **Recommended as the interim default, behind one explicit setting whose other value is A.** Needs the lead's sign-off, since it is not native behaviour. Remove it when the stock films pass. |
| **C. Report, continue, and re-base the hash.** B, but after each mismatch set S to the film's {index, hash}, so each interval is judged alone. | Same as B. | A little harness code. | Useful in `aa-parity` for counting bad intervals. Adds nothing for players. Not needed in the client. |
| **D. Refuse.** Show a refusal when a film cannot match, such as any random-map film until the generator is exact. | The film cannot be played; a message says why. | No native string says this, so the text would be ours; the lead should choose it. | Fits the "explicit refusal" rule, and should apply to random-map films if B is rejected. Under B they load but show a wrong map, so D is still the better default for them. |
| **E. Replay recorded state.** Drive the view from a native state trace. | Correct picture. | Needs a trace per film, which players do not have. | Reject. It is not the game. |

**My recommendation.** Build A as the code path, with a single `film_sync` mode next to it (`Strict` / `Report`). Default `Report` for scenario-map films, refusal (D) for random-map films until the generator passes, and flip the default to `Strict` film by film as checkpoints pass. Tests and `aa-parity` always run `Strict`. Log the first mismatch and the end-of-film mismatch count, so progress can be read from any run.

`FilmForcePlayback` is **not** this switch: natively it only skips the version check.

**The lead's decision, as built (`aa_parity::playback::SyncMode`).** The native stop is the code path: `Strict` is 0x752F60, 0x751EB0 and END_OF_FILM with out-of-sync set. The client's default is `Report` until films pass, a deliberate, temporary departure from the original:

- The first mismatch gives `FilmEvent::FirstMismatch`: the client logs `FILM OUT OF SYNC(n)` once and writes the native text `film_end_of_film_oos` to the in-game message log, so the player is not misled. The film keeps playing.
- At the natural end the event is `EndOfFilm { out_of_sync: true }` if any mismatch happened (the box `film_end_of_film_oos`, Ok, to Scoring), otherwise the native End of Film box with Exit to Scoring and Keep Watching.
- An unknown ordering player (0x75D710) in `Report` is told once (`FilmEvent::NoOrderingPlayer`), counts as a mismatch and the film plays on; in `Strict` it is the native stop and ends the record's decree loop.
- `AA_FILM_SYNC=strict` gives the native behaviour. Tests run `Strict` unless they measure `Report`.
- Random-map films are not refused (option D is not taken): they load through the generator with the film's own settings, and `Report` covers the divergence until the generator is exact.

Remove `Report` when the stock films pass.

## 12. Ordered build list

Each item is playable on its own. Numbers are the order to build in.

**F1. Move the film reader where the client can use it.** Needed by everything below.
- Natively: 0x5A28B0, 0x76E1D0, 0x751A40.
- Data: `.RFM` bytes.
- Today: `aa_parity::film` depends only on aa-mapworld, aa-orders and aa-rules, and `crates/aa-bevy` does not depend on aa-parity. Add the dependency, or move `film`, `decree` and `registries` to a library. Your call (see the requests to the lead).
- Also needs: the SharedProperty registry count, still missing for one record of film 1 (`ROADMAP.md` 5.1).
- **Built:** `aa_parity::playback`, reached by a path dependency of aa-bevy on aa-parity (feature `play`). The registries come from the data set in use, cached under `assets-cache/aa-parity`: 6.6 s to compute for v14 under load, under a millisecond to read back, about 3 minutes once for a large mod.

**F2. Film mode of the file dialog.**
- Natively: 0x68DB60(5), 0x68BB70, 0x68E340, 0x6612E0, 0x65F9D0, 0x68EF50.
- Data: list of `Films/*.RFM`; for each, header name, size, kingdoms, source name; time from 0x59E210; remembered name.
- Build: `SelectLaunchType LOAD_FILM` in `crates/aa-bevy/src/ui/custom.rs` (now a log line) opens `FileDialog` in a second mode: no rules panel, the row-text rule, `Load` / `Delete` / `Cancel`, the preview.
- Refusals: a locked or unreadable file is skipped silently; "File missing" as the original; a typed folder path shows "File missing" instead of crashing.

**F3. Film session in staging.**
- Natively: 0x75F470, 0x777380, 0x779B30.
- Data: source map lookup and CRC.
- Build: `Source::Film` in `custom.rs`. `aa_ui::staging` already has `LaunchType::LoadFilm`, the no-teams roster and the `(Film)` preview line (`staging.rs:488, 937`).
- Messages: `unable_to_select_film_source_file_not_found|s`.

**F4. Load the battle.**
- Natively: 0x776B20, 0x775EE0, 0x76C0A0, 0x5A3980, 0x5A2B90, 0x76E1D0, 0x76CD00.
- Data: header version and build (`aa_parity::film::film_version_check` exists), `FilmForcePlayback` UVar, source map, SessionPlayers.
- Build: `aa_parity::replay::plan` and `build` already give an aa-play world from a film on a stock map, including kingdoms, nations, generals, colours and starting rules. Lift them into the client's load path.
- Random-map films: refuse (option D) or generate (`aa-play` mapgen), by the lead's decision.
- Failures: the three `failed_film_*` messages on the loading screen's failure path. The world stays frozen until Continue.
- **Built:** `playback::open_film`: the file, the source (the saved game's chain too), the version, the build, then the world. A scenario map and a random map from the film's settings open; what aa-play cannot create is the literal `unknown CreateGWorld failure` of 0x5A3980, with the reason for the log. A film in `<profile>/data/Films` plays on that profile's data, and a film of a saved game starts from the save's restored world: see `FILM-CORPUS.md`, "Sources".

**F5. The tick driver and end of film.**
- Natively: 0x752AE0, 0x751A40, 0x75D710, 0x752F60, 0x751EB0, the catch at 0x751B07.
- Build: in `step_battle` (`crates/aa-bevy/src/session.rs`), feed record r through `World::step_decrees` before update r + 1, as `aa_parity::replay::run_observed` does, using `Mapper` and `PlayIds`. Keep stepping after the film is gone. No SAI.
- Sync: S from `aa_play::trace` and `aa_parity::slog`; checkpoint check per section 6; mode per section 11.
- Add: the unknown-player stop; unresolved decrees logged, never fatal.
- Commands: 26 of the 39 ActorCommands have no aa-play equivalent yet (`classify::ACTOR_COMMAND_MAPPING`: guard, retreat, bombard and so on). Guard and retreat are the most common in film 0. Each one missing is a visible divergence.
- **Built:** `playback::FilmPlayer::step` over `replay::FilmDriver`, the decree loop the Berlin gate runs too. Each decree logs its S entries and takes effect in record order; a 311 is checked by the native rule 0x77D050 on S's entry at its index, through `aa_orders::net::sync::Synchronizer`. `tests/playback.rs` proves the two agree, the stops, and every corpus film (`FILM-CORPUS.md`).

**F6. The viewer.**
- Natively: 0x4F61A0, 0x5C0E50, 0x5C0E90, 0x5F8E00, 0x5F7E50, 0x5F7150, 0x5F4780, 0x5F4750, 0x4F96D0.
- Data: kingdom colours, `Actions/hotkeys_standard_aa.txt` lines 531 to 542, `ObserverGameMusic`.
- Build: `Battle.observer = true` already exists (`session.rs:83`) and clears selection and commands. Add `view_kingdom: Option<KingdomId>` and use it where `ui/fog.rs`, `ui/minimap.rs` and `ui/hud.rs` now pass `Some(b.local)`; `aa_play::fog::shade` and `sighting` already take an Option. Hide the resource bar and experience when None. Action panel buttons. Camera at the map centre.
- `Session+0x168` rule for campaign films: read and port with the [I] above verified.

**F7. Speed and pause.**
- Natively: 0x76B360, 0x76B3A0, 0x76F530, 0x76F370.
- Data: `Game/SVars.tgi`.
- Build: `ui/front.rs:258` hard-codes -2..2 for the slider; `hud.rs` keeps speed. Use the film range in film state. The rounding step from 0x76F530. Check the message at each step against the observed 71, 50, 35, 25, 18.

**F8. End-of-film box and leaving.**
- Natively: 0x652020, 0x66D360, 0x66F4F0, 0x66F390, 0x6125A0, 0x774C80, 0x652790, 0x6590B0.
- Data: strings `film_end_of_film2`, `film_end_of_film_oos`, `film_exit_to_scoring`, `film_keep_watching`, `ui_game_menu_leave_game_host_film`.
- Build: `Front` handles a film end event; `aa_ui::message_box` already shows these boxes. Menu label from `hud.rs:951`. `RequestFinished` to Scoring exists (`front.rs:539`); Scoring needs the film's Replay branch and Exit back to staging with the film kept. No mission result boxes (0x440190, 0x4408A0).

**F9. The order gate in the client.**
- Natively: the 100 sites; rule 0x766C70.
- Build: one check where `crates/aa-bevy/src/input.rs` and `hud.rs` issue commands, listing the same classes as section 8.1. Keep speed, pause, chat, panels.

**F10. Main menu route.**
- Natively: 0x64F3F0, 0x64EFF0, 0x6455B0, 0x645200, 0x645540, 0x652B10.
- Build: remove `RequestFilm` from `NOT_BUILT` in `ui/front.rs:143`. Depends on F2 to F4.

**F11. Recording in the client.**
- Natively: 0x7757F0, 0x76FE60, 0x76C310, 0x74D640, 0x755290.
- Data: the mirror stream of decrees for the client's commands.
- Built: section 9.3.

**F12. Save Film.**
- Natively: 0x658F90, 0x68DCE0(5), 0x6911F0, 0x690A00, 0x772BB0, 0x7729E0, 0x6014B0.
- Built: section 9.3.

**F13. Save during a film, delete, network.** Last, and only if wanted: GameSave from film state (0x776200), Delete in the dialog, and the network film paths (`map_will_not_transfer_host_film|ss`, 0x7554B0, 0x7725A0, 0x770B70, 0x7770B0).

**Comparison with what exists.**

| piece | exists | proof level | missing for playback |
|---|---|---|---|
| `crates/aa-parity/src/film.rs` | film reader, header, players, records, checkpoints, version check | decodes every record of the QA films; I ran it on `LastFilm_1`: last tick 272, 16 checkpoints | not reachable from aa-bevy; one registry count |
| `crates/aa-parity/src/replay.rs`, `mapping.rs`, `slog.rs` | plan, build, run, decree to Command, S log | 28 of 266 Berlin checkpoints | production home; unmapped commands; RMG films refused only implicitly |
| `aa_orders::net::admin`, `sync` | 0x751A40, 0x74D640, 0x77D050 ports, with `stop_film` in the host traits | ported and tested in aa-orders; not used by the Bevy client | client wiring |
| `crates/aa-runtime/src/film_*.rs` | full writer | native fixtures (`research/skirmish-20261017-film`) | a stream of decrees from the client |
| `crates/aa-play` `step_decrees`, `trace` | S entries, decree stepping | S matches Berlin up to the first difference | exactness |
| `aa-bevy/src/ui/front.rs` | `RequestFilm` listed as not built; `RequestFinished` to Scoring | runs | film routes |
| `aa-bevy/src/ui/custom.rs` | LOAD_MAP and RANDOM_MAP; LOAD_FILM logs "not built yet" | runs | F2, F3 |
| `aa-bevy/src/ui/dialogs.rs` | scenario map dialog, same native class | runs | film mode |
| `aa-bevy/src/ui/scoring.rs` | SaveFilm hidden, command names, `film` flag | runs | F8, F12 |
| `aa-bevy/src/session.rs` | `observer` flag, fixed 16 Hz step | runs | film feed, view kingdom |
| `aa-ui/src/staging.rs` | `LoadFilm`, no-teams roster, `(Film)` line | compiled; not exercised with a film | a `Source::Film` to drive it |

## 13. Open items

1. **Typed folder path crash.** The original crashes; the client shows "File missing" instead, by the lead's decision. A deliberate difference.
2. **Session+0x168 and header+0x3C** (campaign films lock the view): inferred, not exercised. No campaign film on hand.
3. **Scripts and triggers during a film** were not traced. KKC_HeardScriptCommand and KKC_NotifyTriggerManagerOfVideoCompletion are rejected as live requests; whether their decree forms carry the whole story in the film is unchecked. Needs a film of a scripted map.
4. **Speed reset.** Speed returns to 100% at the next film launch [O]; I did not find which call does it (candidates 0x4F1080, 0x61E620, the world setup).
5. **The main-menu route skips staging**: read from the calls, untested in the original. The client follows it (0x64F3F0, 0x645200, Cancel 0x645540, Scoring Exit 0x659290 to slot 9), proven on the hidden desktop.
6. **Mouse-only paths** in section 10 stay unrun in the original; all but Save Film and clicks on units on a real film are built and run in the client.
7. **Slot +0x24 of the film state (0x652880)**: who calls it against 0x652790. The observed exit used 0x652790's route. [I]
8. **Hotkey `w`.** The file says `map w`, my log printed `map alt-w` from the merged tables. Harmless, but check when binding.
9. **A saved world's Minds.** `aa_play::saved_world::apply_saved_minds` applies them: the hosted Minds take the saved stack, commands, Move state and path, with the Mover and the Character as saved. A guard, a retreat, an explore and a lone actor's kill have no hosted state yet: their saved command is given again as a new order, labelled in the film's notes. The aircraft Mind Load 51C3C0 is not ported, so aircraft in flight are not created.
10. **Stale modifiers after a load.** On the original, a loaded world's Detection and Supply radii do not hold the kingdom technologies' adds until something refreshes them (18, not 23). The restore keeps them so (`ActorBooks::saved_techs`). The refresh point used is a change of the kingdom's technologies; a level change of the thing itself is not followed yet. The morale maximum is copied (`OrgCombat::saved_morale_living`).
11. **Art of a profile's data.** `aa_assets::convert::FolderArt` converts a model or a texture of a data folder when the battle first asks for it, into `<own profile>/art/<key>`; a file equal to the stock data's stays the stock conversion. Effects, sounds and interface text read the folder too (`effects::bones::folder`, `FILM-CORPUS.md`).
12. **Sides a session leaves out.** Read and settled by the films. ConfigureGWorld 0x50E8B0 walks every kingdom the creator made: one without a KingdomInfo, or whose KingdomInfo has +0x31 set, is listed for destruction (`Destroying kingdom because cannot find KingdomInfo in WorldParams`), and only after the random settings of all of them are resolved does the list go. So a closed side makes its world-start draws. It never reaches the generator's placement: the seven films with closed sides stand on the stream, and `d22932af71` passes all 26 of its checkpoints with only the session's sides placed. Had the others been generated and removed, their placement's draws would break the first checkpoint.
9. **Recording, what is still not the original's** (section 9.3). A nation left to Random is drawn by staging, so the creator row holds it where the original's is empty and draws 0x50ECA6. The HUD's hq_attach is the cycle, 0x40 (`ActorOrder::HqCycle`); `Command::Attach` is left for the scripts, in a shape only this port reads.

## 14. The client sweep

The real client (`AxisAlliesBevy`, fast release build) on a hidden desktop, scripted with `AA_UI_SCRIPT`, muted, in a scratch profile: pick the film in the dialog, launch, Continue, objectives, view kingdom 1 and back, zoom 2.1 and look at a building, speed to the top, look at units twice, the end box, Scoring, Exit to staging. Nine other builds ran on the machine throughout, so the frame rates are what a machine with no room left gives, not what the client can do. The previous round's runs, with fewer builds going, read 11 to 33 FPS on the same films.

`AA_BEVY_TERRAIN_CHECK=1` compares every drawn terrain corner with the height the world keeps. Checkpoints are the film's own 311s, matched in `Report`. "Not reached" means the run's time limit came first: at top speed the client takes at most 8 ticks a frame, so at 5 FPS a film plays at about 40 ticks a second, not 256.

| film | map | end tick | checkpoints | first mismatch tick | corners differ | objects drawn / placed | panics | median FPS | end box | back at staging |
|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| Market Garden, short | 448 scenario | 263 | 0/15 | 40 | 0 | 5644/5683 | 0 | 8 | yes | yes |
| Market Garden, 7107 ticks | 448 scenario | 7114 | 0/418 | 38 | 0 | 5644/5683 | 0 | 15 | yes | yes |
| Berlin, fight | 448 scenario | 20203 | 123/1187 | 2726 | 0 | 7930/7971 | 0 | 9 | yes | yes |
| Kursk, fight | 448 scenario | 22415 | 0/1318 | 59 | 0 | 4623/4683 | 0 | 7 | yes | yes |
| Comrades, fight | 256 scenario | 16182 | 86/951 | 2014 | 0 | 1123/1139 | 0 | 8 | yes | yes |
| Mountain Pass | 256 developer | 12452 | 171/732 | 3482 | 0 | 1378/1429 | 0 | 7 | yes | yes |
| Farmhouses, fight | 320 developer | 30921 | 64/1818 | 1534 | 0 | 1783/1831 | 0 | 9 | yes | yes |
| Tutorial (view locked) | 256 campaign | 2313 | 0/135 | 30 | 0 | 2679/2765 | 0 | 8 | yes | yes |
| Southern Europe | 384 random | 260 | **15/15** | none | 0 | 2319/2371 | 0 | 7 | yes, End of Film | yes |
| Southern Europe | 384 random | 10718 | 5/630 | 140 | 0 | 2023/2082 | 0 | 10 | yes | yes |
| Southern Europe, long | 384 random | not reached | | 33499 | 0 | 2182/2231 | 0 | 6 | no | no |
| Southern Europe | 320 random | 277 | 3/16 | 135 | 0 | 1639/1701 | 0 | 5 | yes | yes |
| Southern Europe, the longest film | 640 random | not reached | | 188 | 0 | 3965/4134 | 0 | 5 | no | no |
| Alaska | 640 random | 4275 | 13/88 | 283 | 0 | 6236/6330 | 0 | 6 | yes | yes |
| Germany | 640 random | 90452 | 1/2636 | 133 | 0 | 6177/6390 | 0 | 7 | yes | yes |
| India | 512 random | 83659 | 9/2475 | 270 | 0 | 2822/2904 | 0 | 7 | yes | run's limit |

No panic, no stall and no error line in any run. The gap between objects drawn and placed is the world's invisible things.

**Against the original.** The first Southern Europe film above played in both, same keys, at the QA profile's zoom 2.1. The view registers to one pixel (correlation search over the shots). The resource bar at view kingdom 2 reads the same (252(+35), +20, +10, 2/100); at kingdom 1 the shots, not taken on the same tick, read 502 and 501 (+35), +25, +20, 1/100. The fog's shape and its 0.50 on the terrain are the same, and so are the minimap and its rectangle. Exit to Scoring and Keep Watching are the same box.

**Causes found and closed by the sweep.**
- The camera's target stood on the drawn ground. Natively 0x6FEDB0 -> 0x6FF288 reads HEIGHT_MAP through 0x550210, no overlay, so on a mountain our view sat 182 px low at zoom 2.1. Every battle with a mountain under the camera had this.
- Developer, trailer and campaign maps drew flat colour on 20 to 80% of their tiles and disagreed with the world on height: their files and recovered palettes name ids `Data/translate.tgi` renames. `AssetStore::fill_palette` is now 0x48D500: translation first, its target only, the entry takes the variation it comes to; a tile whose id comes to nothing keeps the previous tile's terrain (0x557D4B), the same rule the world uses. The stripes this leaves on Mountain Pass are in the original's minimap too.
- 11 mountain variations of chaparral and desert were drawn with the template's noise, up to 1.45 m off the world (`TerrainMetadata::bundled`).
- The film's own Sun world rule is applied to scene and world (0x4FDCE0: only when the rule differs from the default). Checked with a film whose rule was edited to sun_dusk.

**Failure paths, proven on the client:** source map missing shows `Source map for film 'x' missing.` in the log and stays on the session; a wrong build and a wrong version show their boxes from staging and from the main menu.

**Settled against the native code, shown on the client.**
- A view-locked film (the tutorial's, header +0x3C set; `FilmPlayer::view_locked`, Session+0x168). The empty action panel is native (0x5F8E00 -> 0x5F5300), and so is the flashing book button (0x50C880 is not gated on film).
- The message log clears at a launch from state 1 (0x77A950 -> 0x770500 case 1 -> 0x40B450) and stays through Scoring and Exit, as `27-after-scoring-exit.png` shows. Launch again and Replay start with no old messages and at 100%.
- The remembered film persists as preferences block +0x18 does (written at 0x75F6E7 and 0x690D75, saved by 0x41D570): `last_film` in the client's settings, chosen again by a second process.
- The first Esc closes the film dialog even with the edit box focused (0x5D17E0 case 0x1B returns 0).
- A battle opens at the profile's zoom (0x43FB44 -> 0x7010D0 -> 0x7005F0 with CameraScaleZoom): 0.7 on a fresh profile, 2.1 on the QA profile.
- A film on a mod's own data opens from its profile folder (14 s) and plays with that data's definitions; its art is converted as the battle asks for it (`aa_assets::convert`).

**Not settled.**
- The biome's `detail_texture` and `macro_texture` (0x4B8E80: Biome +0x88, +0x8C, repetitions +0x90, +0x94) are not drawn. It is why the original's ground is grainy and ours smoother on temperate maps. `look.rs` and `original.wgsl` own it; the native reader of those fields is not yet found.
- Tree shadows are Bevy's cascades, the original's are stencil volumes: thinner and darker. Trees under fog read 0.50 in ours, 0.53 in the original.
- The map's edge in the viewer with no fog is hard in ours. No shot of the original at an edge in film mode yet: posted keys do not scroll it. From the code: `AVars_AA` turns TerrainSkirt on (0x71FB20 -> 0x71E2C0: a strip on each map side from the ground down to the world's lowest height), and its texture `Special/SkirtTexture.dds` is black (2,2,2), as our clear colour is, so nothing should show. Not built, inferred, no picture to prove it.
- Top speed at a low frame rate: since 2026-11-02 `step_battle` drops no tick. Natively the world has its own thread and owes the time it has not run (0x755290, 0x76B780); here a frame runs eight ticks and then what 50 ms hold, keeping up to two seconds of play for the next.
- A film opening on bare rock is native when the middle of the map is rock: 0x4F61A0's observer branch looks at half the map's width and height.
- `ObserverGameMusic`: the original writes no sound log. Its log only names missing resources, and the UVar `AudioDebugIDS` would only silence the others (and saves into the profile). The client plays what 0x4F96D0 reads, `menu_background_music`, on the first world tick. [C] only.
- A film copied into a profile whose data differs fails to decode there. Corpus film 0c01c516f8 decodes on v14 but sits in a folder whose data it does not fit: `record 13 does not decode on this data`, with the failure box. That follows the lead's decision 6; the native result on that data is untested.

10. **A scripted map does not yet round-trip.** `scripted_map_round_trip` on `Campaign/allies01_el_alamein`: 39 of 40 checkpoints, then at tick 678 the live battle has 12 S entries the film's replay does not, the world digest the same and the RNG on. The stock and random maps have run 64000 and 16000 ticks without it, so it comes with the script. Not traced yet: what draws there in the battle and not in the film.
