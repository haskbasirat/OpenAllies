//! Playing a film for the game client (research/byte-exact/FILM-PLAYBACK.md
//! sections 2 to 7, items F1, F4, F5).
//!
//! * [`list_films`] is the film mode of the file dialog: the rows of 0x68E340 and
//!   the preview of 0x6612E0.
//! * [`open_film`] is the load: the dialog's checks 0x59F570 and 0x777380, then
//!   the task 0x5A3980 with its version and build check, and the world
//!   (`replay::plan` and `build_with`), the film players (0x76E1D0, 0x76CD00) and
//!   the Orders chunk (0x7757F0). Its failures are the native ones ([`FilmFailure`]).
//! * [`FilmPlayer::step`] is one world tick (0x752AE0): record `r` right before
//!   update `r + 1` through [`replay::FilmDriver`], the decree loop the Berlin gate
//!   runs too, so the two cannot drift. After the film is gone the world keeps
//!   stepping. No computer player is attached: their decisions are the film's
//!   decrees, and natively the SAI main is gated off in film state (0x7A0080).
//!
//! What a film plays on: its folder decides ([`FilmData::for_film`]). The film's
//! definition references are decoded with the registries of that data set, and a
//! profile's data reaches the world's definitions, generals, random maps and maps
//! too (`aa_play::data::DataFolder`).
use crate::film::{self, Film, FilmHeader, FilmVersionCheck};
use crate::mapping::Mapped;
use crate::registries::Registries;
use crate::replay::{self, DecreeFlow, DecreeSignal, FilmDriver, IdTables};
use aa_mapworld::header::{DerivedCreator, MapHeader};
use aa_play::data::DataFolder;
use aa_play::mapgen::{GeneratedMap, RandomMapData, RandomMapParams, WorldKingdom, WorldSpec};
use aa_play::special_ops::GeneralData;
use aa_play::{GameData, GameMap, World};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// `[0xB27A80]`: the running build, 4129.7 in every AA.exe of the project.
pub const FILM_BUILD: f32 = 4129.7;
/// The running version when the data has no `Version` var to read
/// (`[0xB3888C]`, AVars_version.tgi).
const FALLBACK_VERSION: &str = "1.0.5.0";
/// The localized `last_film_filename` of the shipped string tables.
pub const LAST_FILM_FILENAME: &str = "LastFilm";

/// What a differing checkpoint does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SyncMode {
    /// The native stop: SetSimSyncValue 0x752F60 logs `FILM OUT OF SYNC(n)`,
    /// the stop 0x751EB0 drops the film and END_OF_FILM goes out with
    /// out-of-sync set.
    Strict,
    /// The client's default until films pass (FILM-PLAYBACK.md section 11):
    /// the first mismatch is reported once and the film keeps playing.
    #[default]
    Report,
}

/// A folder of films.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilmDir {
    pub path: PathBuf,
    /// A folder the original writes to: listed and played, never written.
    pub read_only: bool,
    /// The install, where a film's source map is read for the preview's `Source`.
    pub game_root: Option<PathBuf>,
}
impl FilmDir {
    /// The film folder of the client's own profile.
    pub fn own(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            read_only: false,
            game_root: None,
        }
    }
    /// The folder the original writes its films to.
    pub fn original(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            read_only: true,
            game_root: None,
        }
    }
    /// With the install, the preview's `Source` is the map's own name.
    pub fn with_game_root(mut self, game_root: impl Into<PathBuf>) -> Self {
        self.game_root = Some(game_root.into());
        self
    }
}

/// One kingdom row of the preview: its name and colour pip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewKingdom {
    /// The display key through the film's own strings (MAPSTRINGS): `Player`.
    pub name: String,
    /// The film's colour id ("blue"), resolved by the UI through
    /// `GameData::kingdom_colors`.
    pub color_id: String,
}

/// What the file dialog's right panel shows for a film (0x6612E0, mode 5).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FilmPreview {
    /// The film's name, without the `(Film)` line of 0x65F9D0: a film's name key
    /// is empty, and the running original shows the file's name.
    pub name: String,
    /// `Source` (0x660850): `#<rmc>_name` for a random-map film, a string table
    /// key; else the source map's own name, or its file name, the native fallback
    /// for a map that does not open.
    pub source_map: String,
    /// `File`: the file name without its folder (0x859B70).
    pub file: String,
    /// `Time`: the header's time (0x59E210) as 0x6FBF50 formats it.
    pub time: String,
    /// `Size`: "384x384" (0x65FB20, the creator's extents as integers).
    pub size: String,
    /// The rows of 0x65F880.
    pub kingdoms: Vec<PreviewKingdom>,
}

/// One row of the film dialog (0x68E340).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilmRow {
    pub path: PathBuf,
    /// The row text: the file name without extension, or `LastFilm <time> <date>`
    /// for the rotation films; ` [file]` is appended to equal texts.
    pub text: String,
    pub preview: FilmPreview,
    pub read_only: bool,
    /// The header's time, a FILETIME (0x59E210), which picks the preselected row.
    pub time: u64,
}

/// One native failure of opening a film. `key()` is the string table key and
/// `argument()` its `%` argument.
#[derive(Clone, Debug, PartialEq)]
pub enum FilmFailure {
    /// `load_error_missing_map|s` (0x645200, 0x59F570): the name as it was asked for.
    MissingFile { name: String },
    /// `unable_to_select_film_source_file_not_found|s` (0x777380).
    SourceFileNotFound { name: String },
    /// `failed_film_source_map_missing|s` (0x5A2B90).
    SourceMapMissing { source: String },
    /// `failed_film_wrong_version|s` (0x5A3980).
    WrongVersion { version: String },
    /// `failed_film_wrong_build|g` (0x5A3980).
    WrongBuild { build: f32 },
    /// The literal `unknown CreateGWorld failure` of 0x5A3980: the world could
    /// not be created. `detail` is for the log only.
    CreateWorld { detail: String },
}

/// The `%` argument of a [`FilmFailure`].
#[derive(Clone, Debug, PartialEq)]
pub enum FailureArgument {
    Text(String),
    Number(f32),
    None,
}

impl FilmFailure {
    /// The string table key; [`FilmFailure::CreateWorld`] has literal text.
    pub fn key(&self) -> &'static str {
        match self {
            Self::MissingFile { .. } => "load_error_missing_map|s",
            Self::SourceFileNotFound { .. } => "unable_to_select_film_source_file_not_found|s",
            Self::SourceMapMissing { .. } => "failed_film_source_map_missing|s",
            Self::WrongVersion { .. } => "failed_film_wrong_version|s",
            Self::WrongBuild { .. } => "failed_film_wrong_build|g",
            Self::CreateWorld { .. } => "unknown CreateGWorld failure",
        }
    }
    pub fn argument(&self) -> FailureArgument {
        match self {
            Self::MissingFile { name } | Self::SourceFileNotFound { name } => {
                FailureArgument::Text(name.clone())
            }
            Self::SourceMapMissing { source } => FailureArgument::Text(source.clone()),
            Self::WrongVersion { version } => FailureArgument::Text(version.clone()),
            Self::WrongBuild { build } => FailureArgument::Number(*build),
            Self::CreateWorld { .. } => FailureArgument::None,
        }
    }
}

/// Where the game's data is for a film.
#[derive(Clone, Default)]
pub struct FilmData {
    /// The install: Data.rwd and the v14 overlay.
    pub game_root: PathBuf,
    /// The data folder of the profile the film lives in (`<profile>/data`): its
    /// loose files lie over the game's Data, and a film's sources are looked up
    /// there first.
    pub overlay: Option<PathBuf>,
    /// One more data folder a film's sources are looked up in, after the overlay,
    /// for a film played on another data set than its folder's.
    pub source_folder: Option<PathBuf>,
    /// UVar `FilmForcePlayback` (0x5A3980), default false.
    pub force_playback: bool,
    /// The caller draws the film: a map the extracted set does not hold has its
    /// terrain and scenery read for it (`FilmScene::generated`). A data set's random
    /// map tables are read for that.
    pub draws_scene: bool,
    /// What the client has loaded already, so a film does not load it twice.
    pub preloaded: Preloaded,
}

/// See [`FilmData::preloaded`]. The stock data set only.
#[derive(Clone, Default)]
pub struct Preloaded {
    pub game: Option<Arc<GameData>>,
    pub generals: Option<Arc<GeneralData>>,
    pub random_maps: Option<Arc<RandomMapData>>,
}

impl std::fmt::Debug for FilmData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilmData")
            .field("game_root", &self.game_root)
            .field("overlay", &self.overlay)
            .field("force_playback", &self.force_playback)
            .finish()
    }
}

impl FilmData {
    /// The game's own data, no overlay.
    pub fn new(game_root: impl Into<PathBuf>) -> Self {
        Self {
            game_root: game_root.into(),
            ..Self::default()
        }
    }
    /// The data a film at `path` plays on: a film living in `<profile>/data/Films`
    /// (or a folder below it) plays on that profile's data folder.
    pub fn for_film(game_root: impl Into<PathBuf>, path: &Path) -> Self {
        let mut out = Self::new(game_root);
        out.overlay = profile_data_folder(path);
        out
    }
    /// The overlay when it holds definitions of its own: an index, the root of
    /// the native load. Without one a profile's data holds only what the game
    /// wrote there (films, saves, screenshots) and the definitions stay the game's.
    pub fn definition_overlay(&self) -> Option<&Path> {
        let overlay = self.overlay.as_deref()?;
        std::fs::read_dir(overlay)
            .ok()?
            .flatten()
            .any(|e| {
                let name = e.file_name().to_string_lossy().to_ascii_lowercase();
                name.starts_with("index") && name.ends_with(".lst")
            })
            .then_some(overlay)
    }
}

/// `<profile>/data` when `path` lies in its `Films` folder or below it.
fn profile_data_folder(path: &Path) -> Option<PathBuf> {
    let mut dir = path.parent();
    while let Some(d) = dir {
        if d.file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("Films"))
        {
            return d.parent().map(Path::to_path_buf);
        }
        dir = d.parent();
    }
    None
}

/// What happened in one [`FilmPlayer::step`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilmEvent {
    /// The notification 0x13 END_OF_FILM. `out_of_sync` is +0x38: the box is
    /// `film_end_of_film_oos` when set, else the End of Film box. In
    /// [`SyncMode::Report`] the natural end sets it if any mismatch happened.
    EndOfFilm { out_of_sync: bool },
    /// The first checkpoint that differed: `index` is the film's S index, the
    /// argument of `FILM OUT OF SYNC(%d)`.
    FirstMismatch { index: u32 },
    /// A 298 named a player the film session does not have: the argument of
    /// `Film order with no ordering player %d` (0x75D710), once per player.
    NoOrderingPlayer { player: u32 },
    /// A 328 renamed a kingdom (`Rommel AI`); `World::kingdoms[k].name` has it.
    KingdomRenamed { kingdom: u32, name: String },
}

/// What a playback has done so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilmStats {
    /// World ticks stepped.
    pub ticks: u64,
    pub checkpoints_checked: u64,
    pub checkpoints_matched: u64,
    /// `(world tick, S index)` of the first checkpoint that differed.
    pub first_mismatch: Option<(u32, u32)>,
    /// Orders that did not resolve, by order and reason.
    pub unresolved: BTreeMap<String, u64>,
    /// Orders aa-play has no command for, by name.
    pub no_equivalent: BTreeMap<String, u64>,
    /// Commands aa-play refused, by reason.
    pub refused: BTreeMap<String, u64>,
    /// Order fields aa-play's `Command` cannot carry, by name.
    pub dropped: BTreeMap<String, u64>,
    /// Commands that reached the world.
    pub commands: u64,
    /// Orders under a player the film session does not have.
    pub no_ordering_player: BTreeMap<u32, u64>,
}

/// What the client's scene loader draws of the film's world: the terrain and
/// scenery of its map. (Added by the film-ui lane: open_film fills it from the
/// same plan and generator run it builds the world with.)
#[derive(Clone, Debug, Default)]
pub struct FilmScene {
    /// The file the world's map was read from when it is not a stock map: a saved
    /// game or a map of the profile's data. Its own terrain and scenery are drawn
    /// (`generated`) and its SUN chunk lights the battle. (Added by the film-sources
    /// lane.)
    pub source_file: Option<PathBuf>,
    /// The film's battle: `map_id` names a stock map's scene, the players
    /// decide which placements of the map become actors.
    pub setup: Option<aa_play::SkirmishSetup>,
    /// A random-map film: the generator's settings, the same ones its world
    /// was generated with.
    pub random: Option<RandomMapParams>,
    /// A random-map film: the generator's run its world's map comes from, for
    /// the scene's terrain and scenery ([`FilmPlayer::take_generated`]).
    pub generated: Option<GeneratedMap>,
}

/// The world's end of a data set.
struct DataSet {
    game: Arc<GameData>,
    generals: Arc<GeneralData>,
    random_maps: Mutex<Option<Arc<RandomMapData>>>,
    /// The loose folder a profile's data lies in, until its random maps are read.
    folder: Mutex<Option<DataFolder>>,
    registries: Arc<Registries>,
    /// `[0xB3888C]`: the `Version` var of this data.
    version: String,
    /// False when the world's definitions are not this data set's.
    world_on_this_data: bool,
}

impl DataSet {
    fn random_maps(&self, root: &Path, overlay: Option<&Path>) -> anyhow::Result<Arc<RandomMapData>> {
        let mut slot = self
            .random_maps
            .lock()
            .map_err(|_| anyhow::anyhow!("random map data lock"))?;
        if let Some(r) = slot.as_ref() {
            return Ok(r.clone());
        }
        let folder = self
            .folder
            .lock()
            .map_err(|_| anyhow::anyhow!("data folder lock"))?
            .take();
        let loaded = match (folder, overlay) {
            (Some(folder), _) => RandomMapData::load_on(root, &folder)?,
            (None, Some(overlay)) => RandomMapData::load_on(root, &DataFolder::new(overlay)?)?,
            (None, None) => RandomMapData::load(root)?,
        };
        let loaded = Arc::new(loaded);
        *slot = Some(loaded.clone());
        Ok(loaded)
    }
}

/// A film being played.
pub struct FilmPlayer {
    film: Film,
    set: Arc<DataSet>,
    driver: FilmDriver,
    sync: SyncMode,
    /// The record the next tick plays.
    next: u32,
    /// Admin+0x26C still set.
    running: bool,
    /// A checkpoint differed or a player was unknown, in [`SyncMode::Report`].
    mismatched: bool,
    stats: FilmStats,
    scene: FilmScene,
    notes: Vec<String>,
    view_locked: bool,
}

impl FilmPlayer {
    /// What the client's scene loader needs for this film.
    pub fn scene(&self) -> &FilmScene {
        &self.scene
    }

    /// The generator's run of a random-map film, for the scene to draw. It is
    /// large and only the loader needs it.
    pub fn take_generated(&mut self) -> Option<GeneratedMap> {
        self.scene.generated.take()
    }

    /// One world tick (0x752AE0): the next record (0x751A40, 0x74D710), then the
    /// update. Past the last record the read throws, its catch 0x751B07 pushes
    /// END_OF_FILM and drops the film, and the update of that tick still runs;
    /// from then on only the world runs.
    pub fn step(&mut self, world: &mut World) -> Vec<FilmEvent> {
        let mut events = Vec::new();
        world.events.clear();
        if self.running {
            let record = self.next;
            match self.film.ticks.get(record as usize) {
                None => {
                    self.running = false;
                    events.push(FilmEvent::EndOfFilm {
                        out_of_sync: self.mismatched,
                    });
                }
                Some(tick) => {
                    self.next += 1;
                    let Self {
                        set,
                        driver,
                        sync,
                        running,
                        mismatched,
                        stats,
                        ..
                    } = self;
                    let tables = IdTables {
                        object_ids: &set.registries.object_ids,
                        property_ids: &set.registries.property_ids,
                        formation_ids: &set.registries.formation_ids,
                        ability_ids: &set.registries.ability_ids,
                        layout_ids: &set.registries.layout_ids,
                    };
                    let strict = *sync == SyncMode::Strict;
                    let mut sink = |signal: DecreeSignal<'_>, _: &mut World| {
                        take_signal(signal, record, strict, running, mismatched, stats, &mut events)
                    };
                    driver.run_record(
                        &tick.decrees,
                        world,
                        tables,
                        &replay::Aliases::new(),
                        None,
                        strict,
                        &mut sink,
                    );
                }
            }
        }
        replay::update_world(world);
        self.driver.sim.feed_and_forget(world);
        self.stats.ticks += 1;
        for e in &world.events {
            if let aa_play::Event::Refused { reason, .. } = e {
                *self.stats.refused.entry(reason.clone()).or_default() += 1;
            }
        }
        for (name, count) in &self.driver.mapper.dropped {
            self.stats.dropped.insert((*name).to_owned(), u64::from(*count));
        }
        events
    }

    pub fn stats(&self) -> &FilmStats {
        &self.stats
    }
    /// True until END_OF_FILM (Admin+0x26C still set).
    pub fn film_running(&self) -> bool {
        self.running
    }
    /// World ticks the film holds.
    pub fn film_ticks(&self) -> u32 {
        self.film.ticks.len() as u32
    }
    /// The film as it was read.
    pub fn film(&self) -> &Film {
        &self.film
    }
    /// For tests of the native stop: the records it is about to play.
    #[doc(hidden)]
    pub fn film_mut(&mut self) -> &mut Film {
        &mut self.film
    }
    /// The record the next tick plays.
    pub fn next_record(&self) -> u32 {
        self.next
    }
    /// Session+0x168 (0x7757F0): the header's +0x3C string is set, so the viewer
    /// may not switch kingdoms (0x5C0E90, 0x5F8E00; FILM-PLAYBACK.md 8.3).
    pub fn view_locked(&self) -> bool {
        self.view_locked
    }
    /// The film session's players (0x76E1D0, and those a 329 created).
    pub fn players(&self) -> &[crate::film::SessionPlayer] {
        &self.driver.roster
    }
    /// Where this film's world departs from the film's battle.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
    /// True when the world's definitions are the film's own data set's.
    pub fn world_on_film_data(&self) -> bool {
        self.set.world_on_this_data
    }
}

/// What a decree's signal does to the player.
#[allow(clippy::too_many_arguments)]
fn take_signal(
    signal: DecreeSignal<'_>,
    record: u32,
    strict: bool,
    running: &mut bool,
    mismatched: &mut bool,
    stats: &mut FilmStats,
    events: &mut Vec<FilmEvent>,
) -> DecreeFlow {
    match signal {
        DecreeSignal::Mapped { decree, mapped, .. } => {
            let name = || {
                crate::classify::actor_command(&decree.decree)
                    .map_or_else(|| decree.name.to_owned(), |(_, n, _)| n.to_owned())
            };
            match mapped {
                Mapped::Commands(c) => stats.commands += c.len() as u64,
                Mapped::NoEquivalent(why) => *stats.no_equivalent.entry(why.clone()).or_default() += 1,
                Mapped::Unresolved(why) => {
                    let why = why.trim_end_matches(|c: char| c.is_ascii_digit()).trim_end();
                    *stats.unresolved.entry(format!("{}: {why}", name())).or_default() += 1;
                }
                Mapped::State | Mapped::NoEffect(_) => {}
            }
            DecreeFlow::Continue
        }
        // 0x752F60: `FILM OUT OF SYNC(%d)` and the stop 0x751EB0; the rest of the
        // record's decrees still run, the packet is in memory.
        DecreeSignal::Checkpoint { index, passed, .. } => {
            stats.checkpoints_checked += 1;
            if passed {
                stats.checkpoints_matched += 1;
                return DecreeFlow::Continue;
            }
            if stats.first_mismatch.is_none() {
                stats.first_mismatch = Some((record, index as u32));
                events.push(FilmEvent::FirstMismatch {
                    index: index as u32,
                });
            }
            if strict {
                if *running {
                    *running = false;
                    events.push(FilmEvent::EndOfFilm { out_of_sync: true });
                }
            } else {
                *mismatched = true;
            }
            DecreeFlow::Continue
        }
        // 0x75D710: the same stop, and Admin+0x2E5 ends the decree loop.
        DecreeSignal::NoOrderingPlayer { player } => {
            let count = stats.no_ordering_player.entry(player).or_default();
            if *count == 0 {
                events.push(FilmEvent::NoOrderingPlayer { player });
            }
            *count += 1;
            if strict {
                if *running {
                    *running = false;
                    events.push(FilmEvent::EndOfFilm { out_of_sync: true });
                }
                DecreeFlow::Stop
            } else {
                *mismatched = true;
                DecreeFlow::Continue
            }
        }
        DecreeSignal::KingdomRenamed { kingdom, name } => {
            events.push(FilmEvent::KingdomRenamed { kingdom, name });
            DecreeFlow::Continue
        }
    }
}

// ---------------------------------------------------------------------------
// The dialog's rows.
// ---------------------------------------------------------------------------

/// List the films of `dirs` (0x68E340). Unreadable or locked files are skipped
/// silently, as natively. The folders are not searched below (+0xDC = 0).
pub fn list_films(dirs: &[FilmDir]) -> Vec<FilmRow> {
    list_films_named(dirs, LAST_FILM_FILENAME)
}

/// [`list_films`] with the localized `last_film_filename`.
pub fn list_films_named(dirs: &[FilmDir], last_film_filename: &str) -> Vec<FilmRow> {
    let mut rows = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir.path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_film = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case(crate::film_writer::FILM_EXTENSION));
            if !is_film || !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            if let Some(row) = film_row(&path, dir, last_film_filename) {
                rows.push(row);
            }
        }
    }
    // The list widget keeps its rows sorted (0x5D32F0, compare 0x5D32A0).
    rows.sort_by(|a, b| wcsicmp(&a.text, &b.text));
    // Rows with equal text get ` [file]` (`%s [%s]`), every one of them.
    let same: Vec<bool> = (0..rows.len())
        .map(|i| {
            (i > 0 && rows[i - 1].text == rows[i].text)
                || (i + 1 < rows.len() && rows[i + 1].text == rows[i].text)
        })
        .collect();
    for (row, same) in rows.iter_mut().zip(same) {
        if same {
            let file = file_title(&row.path);
            row.text = format!("{} [{file}]", row.text);
        }
    }
    rows
}

/// `__wcsicmp` on the rows' UTF-16 text (the CRT's "C" locale folds only A to Z).
fn wcsicmp(a: &str, b: &str) -> std::cmp::Ordering {
    let fold = |u: u16| if (0x41..=0x5A).contains(&u) { u + 0x20 } else { u };
    a.encode_utf16().map(fold).cmp(b.encode_utf16().map(fold))
}

/// 0x859D30: the file name without its folder and extension.
fn file_title(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One film's row, `None` when its header does not read (0x59F570).
fn film_row(path: &Path, dir: &FilmDir, last_film_filename: &str) -> Option<FilmRow> {
    let bytes = std::fs::read(path).ok()?;
    let head = film::read_header(bytes).ok()?;
    let title = file_title(path);
    let map = &head.header.map_header;
    let time = film_time(map);
    let time_text = film_time_text(time);
    Some(FilmRow {
        path: path.to_path_buf(),
        text: row_text(&title, &time_text, last_film_filename),
        read_only: dir.read_only,
        time,
        preview: FilmPreview {
            name: if map.name_key.is_empty() {
                title
            } else {
                head.map_string(&utf16(&map.name_key))
            },
            source_map: preview_source(&head.header, path, dir),
            file: path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            time: time_text,
            size: {
                let [w, h] = map.creator.world_extents_bits.map(f32::from_bits);
                format!("{}x{}", w as i32, h as i32)
            },
            kingdoms: preview_kingdoms(&head),
        },
    })
}

/// The row text rule of 0x68E340: when the name's first `len(last_film_filename)`
/// units equal it, ignoring case (0x41D2F0, `__wcsicmp`), the row is that prefix,
/// a space and the film's time.
fn row_text(title: &str, time: &str, last_film_filename: &str) -> String {
    let units: Vec<u16> = title.encode_utf16().collect();
    let base: Vec<u16> = last_film_filename.encode_utf16().collect();
    if base.is_empty() || units.len() < base.len() {
        return title.to_owned();
    }
    let prefix = String::from_utf16_lossy(&units[..base.len()]);
    if prefix.to_lowercase() == last_film_filename.to_lowercase() {
        format!("{prefix} {time}")
    } else {
        title.to_owned()
    }
}

fn utf16(v: &[u16]) -> String {
    String::from_utf16_lossy(v)
}

/// The header's time (+0x18, 0x59E210): a FILETIME from the header's constructor.
fn film_time(map: &MapHeader) -> u64 {
    u64::from(map.words_18_1c[0]) | u64::from(map.words_18_1c[1]) << 32
}

/// `Source` of the preview (0x660850 over the header's +0x38): an RMC's name, the
/// name in the source map's own header (+0x24), or the file's name when it does
/// not open (0x859B70). A saved game's +0x38 is its original map or RMC too.
fn preview_source(header: &FilmHeader, film: &Path, dir: &FilmDir) -> String {
    let source = utf16(&header.map_header.string_38).replace('\\', "/");
    if let Some(rmc) = &header.rmc {
        return format!("#{rmc}_name");
    }
    let lower = source.to_ascii_lowercase();
    if lower.starts_with("rmc_") {
        return format!("#{lower}_name");
    }
    let candidates = profile_data_folder(film)
        .into_iter()
        .chain(dir.game_root.iter().map(|r| r.join("reconstruction/assets/original/Data")))
        .map(|data| data.join(&source));
    for path in candidates {
        if let Some(name) = source_map_name(&path) {
            return name;
        }
    }
    source.rsplit('/').next().unwrap_or("").to_owned()
}

/// The name a map file gives itself, read once per file.
fn source_map_name(path: &Path) -> Option<String> {
    static NAMES: OnceLock<Mutex<BTreeMap<PathBuf, Option<String>>>> = OnceLock::new();
    let names = NAMES.get_or_init(Default::default);
    if let Some(known) = names.lock().ok()?.get(path) {
        return known.clone();
    }
    let name = std::fs::read(path).ok().and_then(|bytes| {
        let head = film::read_header(bytes).ok()?;
        let key = utf16(&head.header.map_header.name_key);
        (!key.is_empty()).then(|| head.map_string(&key))
    });
    names.lock().ok()?.insert(path.to_path_buf(), name.clone());
    name
}

/// The kingdom rows of 0x65F880 over the header's own parameters: not
/// independent, +0x28 set, and +0x29 set or a colour.
fn preview_kingdoms(head: &film::HeaderOnly) -> Vec<PreviewKingdom> {
    head.header
        .map_header
        .parameters
        .kingdoms
        .iter()
        .filter(|k| k.byte_10 == 0 && k.byte_28 != 0 && (k.byte_29 != 0 || !k.color.is_empty()))
        .map(|k| PreviewKingdom {
            name: head.map_string(&utf16(&k.display_key)),
            color_id: utf16(&k.color),
        })
        .collect()
}

/// 0x6FBF50: FileTimeToSystemTime, GetTimeFormatW (user default, no flags), a
/// space, GetDateFormatW (user default, short date): `3:39:08 PM 10/24/2026`.
pub fn film_time_text(filetime: u64) -> String {
    let t = system_time(filetime);
    format!("{} {}", native_time(&t), native_date(&t))
}

/// SYSTEMTIME's fields of a FILETIME, FileTimeToSystemTime's arithmetic.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

fn system_time(filetime: u64) -> SystemTime {
    const TICKS_PER_MS: u64 = 10_000;
    let ms = filetime / TICKS_PER_MS;
    let days = (ms / 86_400_000) as i64;
    let rest = ms % 86_400_000;
    // Civil date from days since 1601-01-01.
    let z = days - 134_774 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    SystemTime {
        year: year as u16,
        month: month as u16,
        // 1601-01-01 was a Monday.
        day_of_week: ((days + 1).rem_euclid(7)) as u16,
        day: day as u16,
        hour: (rest / 3_600_000) as u16,
        minute: (rest / 60_000 % 60) as u16,
        second: (rest / 1000 % 60) as u16,
        milliseconds: (rest % 1000) as u16,
    }
}

#[cfg(windows)]
mod nls {
    use super::SystemTime;
    const LOCALE_USER_DEFAULT: u32 = 0x400;
    const DATE_SHORTDATE: u32 = 1;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetTimeFormatW(
            locale: u32,
            flags: u32,
            time: *const SystemTime,
            format: *const u16,
            out: *mut u16,
            size: i32,
        ) -> i32;
        fn GetDateFormatW(
            locale: u32,
            flags: u32,
            date: *const SystemTime,
            format: *const u16,
            out: *mut u16,
            size: i32,
        ) -> i32;
    }
    pub fn time(t: &SystemTime) -> Option<String> {
        let mut out = [0u16; 0x80];
        // SAFETY: `t` is a valid SYSTEMTIME, `out` holds the size passed, and the
        // format pointer may be null.
        let n = unsafe {
            GetTimeFormatW(
                LOCALE_USER_DEFAULT,
                0,
                t,
                std::ptr::null(),
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        (n > 0).then(|| String::from_utf16_lossy(&out[..n as usize - 1]))
    }
    pub fn date(t: &SystemTime) -> Option<String> {
        let mut out = [0u16; 0x80];
        // SAFETY: as in `time`.
        let n = unsafe {
            GetDateFormatW(
                LOCALE_USER_DEFAULT,
                DATE_SHORTDATE,
                t,
                std::ptr::null(),
                out.as_mut_ptr(),
                out.len() as i32,
            )
        };
        (n > 0).then(|| String::from_utf16_lossy(&out[..n as usize - 1]))
    }
}

#[cfg(not(windows))]
mod nls {
    use super::SystemTime;
    pub fn time(_: &SystemTime) -> Option<String> {
        None
    }
    pub fn date(_: &SystemTime) -> Option<String> {
        None
    }
}

/// The user's locale's time; without the Windows call, the en-US form.
fn native_time(t: &SystemTime) -> String {
    nls::time(t).unwrap_or_else(|| {
        let (h, p) = match t.hour {
            0 => (12, "AM"),
            1..=11 => (t.hour, "AM"),
            12 => (12, "PM"),
            _ => (t.hour - 12, "PM"),
        };
        format!("{h}:{:02}:{:02} {p}", t.minute, t.second)
    })
}

/// The user's locale's short date; without the Windows call, the en-US form.
fn native_date(t: &SystemTime) -> String {
    nls::date(t).unwrap_or_else(|| format!("{}/{}/{}", t.month, t.day, t.year))
}

// ---------------------------------------------------------------------------
// The data sets.
// ---------------------------------------------------------------------------

/// One process keeps one of each data set: a film after the first costs nothing.
fn data_sets() -> &'static Mutex<BTreeMap<(PathBuf, Option<PathBuf>), Arc<DataSet>>> {
    static SETS: OnceLock<Mutex<BTreeMap<(PathBuf, Option<PathBuf>), Arc<DataSet>>>> =
        OnceLock::new();
    SETS.get_or_init(Default::default)
}

/// The data set of `data` if a film has loaded it.
fn loaded_data_set(data: &FilmData) -> Option<Arc<DataSet>> {
    let key = (data.game_root.clone(), data.definition_overlay().map(Path::to_path_buf));
    data_sets().lock().ok()?.get(&key).cloned()
}

/// The data set of `data`, loaded once.
fn data_set(data: &FilmData) -> anyhow::Result<Arc<DataSet>> {
    let overlay = data.definition_overlay().map(Path::to_path_buf);
    let key = (data.game_root.clone(), overlay.clone());
    if let Some(set) = data_sets()
        .lock()
        .map_err(|_| anyhow::anyhow!("data set lock"))?
        .get(&key)
    {
        return Ok(set.clone());
    }
    let root = &data.game_root;
    // A profile's definitions over the archive (lead's decision 6). The stock
    // path below is the one every other film takes, unchanged.
    let folder = overlay.as_deref().map(DataFolder::new).transpose()?;
    let (game, generals, random_maps) = match &folder {
        Some(folder) => {
            let game = Arc::new(GameData::load_on(root, folder)?);
            let generals = Arc::new(GeneralData::load_on(root, folder, &game)?);
            (game, generals, None)
        }
        None => {
            let game = match &data.preloaded.game {
                Some(g) => g.clone(),
                None => Arc::new(GameData::load(root)?),
            };
            let generals = match &data.preloaded.generals {
                Some(g) => g.clone(),
                None => Arc::new(GeneralData::load(root, &game)?),
            };
            (game, generals, data.preloaded.random_maps.clone())
        }
    };
    let registries = Arc::new(Registries::cached(root, overlay.as_deref())?);
    let version = version_var(root, overlay.as_deref());
    let set = Arc::new(DataSet {
        game,
        generals,
        random_maps: Mutex::new(random_maps),
        folder: Mutex::new(folder),
        registries,
        version,
        world_on_this_data: true,
    });
    data_sets()
        .lock()
        .map_err(|_| anyhow::anyhow!("data set lock"))?
        .insert(key, set.clone());
    Ok(set)
}

/// The `Version` var the original compares with (`[0xB3888C]`, AVars_version.tgi):
/// the overlay's, else the game's.
fn version_var(root: &Path, overlay: Option<&Path>) -> String {
    let v14 = root.join("reconstruction/native-game/runtime-expanded-v14/Data");
    let stock = root.join("reconstruction/assets/original/Data");
    overlay
        .into_iter()
        .chain([v14.as_path(), stock.as_path()])
        .find_map(|dir| {
            let text = std::fs::read(dir.join("AVars_version.tgi")).ok()?;
            let text = String::from_utf8_lossy(&text).into_owned();
            text.lines().find_map(|line| {
                let line = line.trim();
                let rest = line.strip_prefix("string")?;
                let (name, value) = rest.split_once('=')?;
                name.trim()
                    .eq_ignore_ascii_case("Version")
                    .then(|| value.trim().to_owned())
            })
        })
        .unwrap_or_else(|| FALLBACK_VERSION.to_owned())
}

// ---------------------------------------------------------------------------
// The source.
// ---------------------------------------------------------------------------

/// What a film's world is created from.
enum FilmSource {
    /// `Maps/<id>.RMP`: the game's, or `file` from a profile's data or the film's
    /// folder when one of those holds it (a loose file replaces the archive's).
    Scenario { map_id: String, file: Option<PathBuf> },
    /// A random map from the film's settings.
    Random,
    /// A saved game (the film started from one).
    SavedGame { path: PathBuf },
    /// The base `KKC_WorldCreator`: 0x59F600 takes the film itself for the source.
    /// No film of the corpus has it.
    Plain,
}

/// Where a resource name resolves, the profile's data first: the file, or `None`.
fn find_resource(name: &str, data: &FilmData) -> Option<PathBuf> {
    let relative = name.replace('\\', "/");
    let candidates = data
        .overlay
        .iter()
        .chain(&data.source_folder)
        .map(|o| o.join(&relative))
        .chain(std::iter::once(
            data.game_root
                .join("reconstruction/assets/original/Data")
                .join(&relative),
        ));
    for path in candidates {
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// The game's own resources (Data.rwd and the v14 overlay), read once: the
/// extracted folder may not hold them all.
fn in_game_data(name: &str, root: &Path) -> bool {
    static RESOURCES: OnceLock<Mutex<BTreeMap<PathBuf, Arc<std::collections::BTreeSet<String>>>>> =
        OnceLock::new();
    let Ok(mut known) = RESOURCES.get_or_init(Default::default).lock() else {
        return false;
    };
    let set = match known.get(root) {
        Some(set) => set.clone(),
        None => {
            let set = Arc::new(crate::registries::v14_resources(root).unwrap_or_default());
            known.insert(root.to_path_buf(), set.clone());
            set
        }
    };
    set.contains(&name.replace('\\', "/").to_ascii_lowercase())
}

/// The map id of a resource `Maps/<id>.RMP`.
fn map_id_of(name: &str) -> Option<String> {
    let name = name.replace('\\', "/");
    let lower = name.to_ascii_lowercase();
    if !lower.starts_with("maps/") || !lower.ends_with(".rmp") {
        return None;
    }
    Some(name["maps/".len()..name.len() - ".rmp".len()].to_owned())
}

/// 0x777380 -> 0x59F600: walk the film's creators down to a source that exists
/// (a map, or the RMC of a random one). `None`: the source is missing. A saved
/// game's own source must be there too, the walk goes through it. `rmcs`: the
/// data set's RMCs (0x41EEC0), when it is loaded.
fn find_source(header: &FilmHeader, data: &FilmData, rmcs: Option<&[String]>) -> Option<FilmSource> {
    let rmc_known = |rmc: &str| {
        rmcs.is_none_or(|known| known.is_empty() || known.iter().any(|r| r.eq_ignore_ascii_case(rmc)))
    };
    match &header.creator {
        DerivedCreator::Base => Some(FilmSource::Plain),
        DerivedCreator::Random(settings) => {
            rmc_known(&utf16(&settings.rmc)).then_some(FilmSource::Random)
        }
        DerivedCreator::Load {
            source_resource_ids,
            ..
        } => {
            let name = utf16(source_resource_ids);
            if name.is_empty() {
                return None;
            }
            let path = find_resource(&name, data);
            if let Some(map_id) = map_id_of(&name) {
                let there = path.is_some() || in_game_data(&name, &data.game_root);
                let stock = data.game_root.join("reconstruction/assets/original/Data");
                let file = path.filter(|p| !p.starts_with(&stock));
                return there.then_some(FilmSource::Scenario { map_id, file });
            }
            // A saved game: its header names its own source.
            let path = path?;
            let bytes = std::fs::read(&path).ok()?;
            if let Ok(head) = film::read_header(bytes) {
                match head.header.creator {
                    DerivedCreator::Base => {}
                    _ => {
                        find_source(&head.header, data, rmcs)?;
                    }
                }
            }
            Some(FilmSource::SavedGame { path })
        }
    }
}

// ---------------------------------------------------------------------------
// Opening a film.
// ---------------------------------------------------------------------------

/// The generator's settings of a random-map film: the header's kingdoms give the
/// sides and their teams, the creator its m_rmc, m_width/m_height, terrain weights,
/// object counts and heroes (0x505EA0), the seed the world's seed.
pub fn random_params(
    film: &Film,
    rmg: &RandomMapData,
) -> anyhow::Result<RandomMapParams> {
    random_params_by(film, rmg, false).map(|p| p.0)
}

/// [`random_params`] with the settings as the film stored them, for a film whose
/// settings do not fit this data's RMC: natively it gets the RMC's defaults and
/// then cannot sync, because it was recorded on other data (the world the
/// recording came from is the stored settings' one). The second value says whether
/// they fit.
pub fn recorded_params(
    film: &Film,
    rmg: &RandomMapData,
) -> anyhow::Result<(RandomMapParams, bool)> {
    random_params_by(film, rmg, true)
}

/// The world creator's lists of a film (`KKC_WorldCreatorRandom`, the header's
/// parameters): what the world makes before the generator runs.
pub fn world_spec(film: &Film) -> WorldSpec {
    WorldSpec {
        teams: film.header.map_header.creator.parameters.teams.len() as u32,
        kingdoms: film
            .header
            .creator_kingdoms
            .iter()
            .map(|k| WorldKingdom { ids: k.id.clone(), independent: k.independent, nation: k.nation.clone(), general: k.faction.clone() })
            .collect(),
        present: film.header.header_kingdoms.iter().filter(|k| !k.independent).map(|k| k.id.clone()).collect(),
    }
}

fn random_params_by(
    film: &Film,
    rmg: &RandomMapData,
    recorded: bool,
) -> anyhow::Result<(RandomMapParams, bool)> {
    let DerivedCreator::Random(settings) = &film.header.creator else {
        anyhow::bail!("not a random-map film");
    };
    let rmc = utf16(&settings.rmc);
    let info = rmg
        .catalog()
        .rmcs
        .into_iter()
        .find(|r| r.ids.eq_ignore_ascii_case(&rmc))
        .ok_or_else(|| anyhow::anyhow!("unknown RMC {rmc}"))?;
    let map = &film.header.map_header;
    let [width, height] = map.creator.world_extents_bits.map(f32::from_bits);
    let sides: Vec<&film::KingdomInfo> = film
        .header
        .header_kingdoms
        .iter()
        .filter(|k| !k.independent)
        .collect();
    let team_names: Vec<String> = map.creator.parameters.teams.iter().map(|t| utf16(&t.id)).collect();
    let mut own_team = team_names.len() as u32;
    let teams: Vec<u32> = sides
        .iter()
        .map(|k| {
            team_names
                .iter()
                .position(|t| t.eq_ignore_ascii_case(&k.team))
                .map_or_else(
                    || {
                        own_team += 1;
                        own_team - 1
                    },
                    |t| t as u32,
                )
        })
        .collect();
    // 0x513A30: what fits the RMC, else its defaults.
    let bits = |list: &[(Vec<u16>, u32)]| -> Vec<(String, f32)> { list.iter().map(|(id, bits)| (utf16(id), f32::from_bits(*bits))).collect() };
    let stored = (bits(&settings.terrain_weights), bits(&settings.object_counts), settings.num_heroes);
    let settings = rmg.film_settings(&info.ids, &stored.0, &stored.1, stored.2)?;
    let fits = settings.fits;
    let (terrain_weights, object_counts, heroes) = if recorded && !fits {
        // By brush: a terrain used by two brushes is stored twice, in order.
        let mut used = BTreeMap::<String, usize>::new();
        let weights = info
            .terrain_brushes
            .iter()
            .map(|b| {
                let nth = used.entry(b.ids.to_ascii_lowercase()).or_default();
                let found = stored.0.iter().filter(|(id, _)| id.eq_ignore_ascii_case(&b.ids)).nth(*nth).map(|t| t.1);
                *nth += 1;
                found.unwrap_or(b.default)
            })
            .collect();
        (weights, stored.1.into_iter().collect(), stored.2)
    } else {
        (settings.terrain_weights, settings.object_counts, settings.heroes)
    };
    Ok((
        RandomMapParams {
            rmc: info.ids.clone(),
            width,
            height,
            sides: sides.len() as u32,
            teams,
            terrain_weights,
            object_counts,
            heroes,
            seed: u64::from(film.header.seed),
            additional_actors: film
                .header
                .map_header
                .creator
                .parameters
                .kingdoms
                .iter()
                .flat_map(|row| row.definitions.iter().map(move |a| (String::from_utf16_lossy(&row.id), String::from_utf16_lossy(a))))
                .collect(),
        },
        fits,
    ))
}

/// A random film's map: generated with the film's settings, its kingdom slots
/// carrying the +0x188 index of the world they make: the header's kingdom order
/// (GWorld+0x148, WP::CreateKingdomsAndTeams).
fn generated_map(
    film: &Film,
    data: &FilmData,
    set: &DataSet,
    sync: SyncMode,
    notes: &mut Vec<String>,
) -> anyhow::Result<(GameMap, GeneratedMap)> {
    let rmg = set.random_maps(&data.game_root, data.definition_overlay())?;
    // The native stop is the code path: on data whose RMC lists other groups the
    // original takes the RMC's defaults and the film does not sync. Report mode
    // shows the world the film was recorded on (the lead's decision 2).
    let params = match (sync, recorded_params(film, &rmg)?) {
        (SyncMode::Report, (recorded, false)) => {
            notes.push("the film's settings do not fit this data's RMC (0x505B00): the original would use the RMC's defaults".to_owned());
            recorded
        }
        (_, (recorded, true)) => recorded,
        _ => random_params(film, &rmg)?,
    };
    let generated = rmg.generate_world(&params, &world_spec(film), &set.game)?;
    let mut map = generated.map.clone();
    // The slots carry +0x188, the position in the RMC's list, which is the header's
    // order only without closed sides. The film's players name theirs.
    for p in &film.players {
        if let (Some(index), Some(slot)) = (p.kingdom_index, map.slots.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&p.name))) {
            slot.kingdom_index = Some(index);
        }
    }
    Ok((map, generated))
}

/// The check of the film's choice (0x59F570 and 0x777380 in `FilmOK` 0x645200
/// and `KKC_HostSetLoadFilmOrder` 0x75F470), before anything else happens: the
/// file reads and its source can be found. (Added by the film-ui lane.)
pub fn check_film_choice(path: &Path, data: &FilmData) -> Result<(), FilmFailure> {
    let name = file_title(path);
    let missing = || FilmFailure::MissingFile { name: name.clone() };
    let bytes = std::fs::read(path).map_err(|_| missing())?;
    let head = film::read_header(bytes).map_err(|_| missing())?;
    // Called on the UI's thread: the RMC is checked only if the data set is loaded
    // already. `open_film` checks it in any case.
    let set = loaded_data_set(data);
    let rmcs = set.as_ref().map(|s| s.registries.rmc_ids.as_slice());
    find_source(&head.header, data, rmcs)
        .map(|_| ())
        .ok_or_else(|| FilmFailure::SourceFileNotFound { name: name.clone() })
}

/// Open a film: builds the world and its player (0x5A3980, 0x7757F0). Blocking,
/// called from the loading thread.
///
/// The checks come in the native order of the task: the file (0x59F570), its
/// version, then its build (0x5A3980), then its source (0x5A2B90, from the
/// creator's CreateGWorld: the dialog's check 0x777380 has passed by then, so
/// this one sees a source gone since) and the world.
pub fn open_film(
    path: &Path,
    data: &FilmData,
    sync: SyncMode,
) -> Result<(World, FilmPlayer), FilmFailure> {
    let name = file_title(path);
    let missing = || FilmFailure::MissingFile { name: name.clone() };
    let bytes = std::fs::read(path).map_err(|_| missing())?;
    let head = film::read_header(bytes.clone()).map_err(|_| missing())?;
    let create_world = |detail: String| FilmFailure::CreateWorld { detail };
    let set = data_set(data).map_err(|e| create_world(format!("the data set: {e:#}")))?;
    if !data.force_playback {
        match film::film_version_check(&head.header.map_header, &set.version, FILM_BUILD) {
            FilmVersionCheck::Accepted => {}
            FilmVersionCheck::WrongVersion { film } => {
                return Err(FilmFailure::WrongVersion { version: film });
            }
            FilmVersionCheck::WrongBuild { film } => {
                return Err(FilmFailure::WrongBuild { build: film });
            }
        }
    }
    let source = find_source(&head.header, data, Some(&set.registries.rmc_ids)).ok_or_else(|| {
        FilmFailure::SourceMapMissing {
            source: utf16(&head.header.map_header.string_38).replace('\\', "/"),
        }
    })?;
    let film = Film::from_bytes(bytes, &set.registries)
        .map_err(|e| create_world(format!("the film's records: {e}")))?;
    // A record that does not decode on this data has no meaning here, and the
    // original has no catch for it (0x751B07 catches only the end of the file):
    // the film was recorded on other data.
    if let Some(t) = film.failures().next() {
        return Err(create_world(format!(
            "record {} does not decode on this data: {}",
            t.tick,
            t.error.as_ref().map_or("", |e| e.message.as_str())
        )));
    }
    let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        create_film_world(&film, &head, &source, data, &set, sync)
    }));
    let (world, scene, notes) = match built {
        Ok(Ok(built)) => built,
        Ok(Err(e)) => return Err(e),
        Err(panic) => {
            let text = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("panic");
            return Err(create_world(format!("the world's creation panicked: {text}")));
        }
    };
    let driver = FilmDriver::new(&film, &world);
    let view_locked = !head.header.map_header.string_3c.is_empty();
    let player = FilmPlayer {
        film,
        set,
        driver,
        sync,
        next: 0,
        running: true,
        mismatched: false,
        stats: FilmStats::default(),
        scene,
        notes,
        view_locked,
    };
    Ok((world, player))
}

/// The scene of a map file (`RandomMapData::scene_of_file`) with what its world left
/// as scenery.
fn file_scene(world: &World, bytes: Vec<u8>, data: &FilmData, set: &DataSet) -> anyhow::Result<GeneratedMap> {
    let rmg = set.random_maps(&data.game_root, data.definition_overlay())?;
    rmg.scene_of_file(&world.map, bytes, &world.books.scenery)
}

/// The film's FogOfWar world rule (enum index, 0 NONE, 1 EXPLORED, 2 FULL), before
/// the first tick. (Added by the film-ui lane.)
fn film_fog(film: &Film, world: &mut World) {
    if let Some(mode) = film
        .header
        .map_header
        .creator
        .rules
        .rules
        .iter()
        .find(|r| utf16(&r.key).eq_ignore_ascii_case("FogOfWar"))
        .and_then(|r| aa_play::fog::FogMode::from_ids(&(r.value as i32).to_string()))
    {
        aa_play::fog::set_mode(world, mode);
    }
}

/// A film whose creator mode is 2 (0x4FC600 on the saved stream): the saved game's
/// world, the film's computer players (no SAI is attached in film state) and fog.
fn restore_saved_world(
    film: &Film,
    head: &film::HeaderOnly,
    path: &Path,
    set: &DataSet,
) -> anyhow::Result<(World, Vec<String>)> {
    let bytes = std::fs::read(path)?;
    let name = file_title(path);
    let map = GameMap::load_file(&format!("saved/{name}"), bytes.clone(), &set.game, Some(name))?;
    let computers = replay::computer_kingdoms(film);
    let controller = |slot: &str| {
        if computers.iter().any(|c| c.eq_ignore_ascii_case(slot)) {
            aa_play::Controller::Computer(1)
        } else {
            aa_play::Controller::Human
        }
    };
    let restored = aa_play::saved_world::restore(&bytes, set.game.clone(), set.generals.clone(), Arc::new(map), &controller)?;
    let mut world = restored.world;
    world.trace.record(true);
    film_fog(film, &mut world);
    // The names the session starts with (the map strings of 0x50E760).
    for k in 0..world.kingdoms.len() {
        let slot = world.ids.kingdom_slot(k as u32).and_then(|s| world.map.slots.get(s)).map(|s| s.id.clone());
        if let Some(info) = slot.and_then(|id| film.header.header_kingdoms.iter().find(|i| i.id.eq_ignore_ascii_case(&id))) {
            let name = head.map_string(&info.display);
            if !name.is_empty() {
                world.kingdoms[k].name = name;
            }
        }
    }
    Ok((world, restored.notes))
}

/// CreateGWorld for the film's source, and the viewer's kingdom names.
fn create_film_world(
    film: &Film,
    head: &film::HeaderOnly,
    source: &FilmSource,
    data: &FilmData,
    set: &DataSet,
    sync: SyncMode,
) -> Result<(World, FilmScene, Vec<String>), FilmFailure> {
    let fail = |detail: String| FilmFailure::CreateWorld { detail };
    let mut notes = Vec::new();
    let (map, random) = match source {
        FilmSource::Scenario { map_id, file: Some(file) } => {
            let bytes = std::fs::read(file).map_err(|e| fail(format!("map {map_id}: {e}")))?;
            // The name the file gives itself, in its own text table.
            let name = film::read_header(bytes.clone()).ok().and_then(|h| {
                let key = utf16(&h.header.map_header.name_key);
                (!key.is_empty()).then(|| h.map_string(&key))
            });
            let map = GameMap::load_file(map_id, bytes, &set.game, name)
                .map_err(|e| fail(format!("map {map_id}: {e:#}")))?;
            (map, None)
        }
        FilmSource::Scenario { map_id, file: None } => {
            let map = GameMap::load_with(&data.game_root, map_id, &set.game)
                .map_err(|e| fail(format!("map {map_id}: {e:#}")))?;
            (map, None)
        }
        FilmSource::Random => {
            let (map, generated) = generated_map(film, data, set, sync, &mut notes)
                .map_err(|e| fail(format!("random map: {e:#}")))?;
            notes.push("a random map from the film's settings: aa-play's generator".to_owned());
            (map, Some(generated))
        }
        // A saved game's world: every kingdom and thing as it stood, as far as
        // aa-play has a place for it (the notes say what it has not).
        FilmSource::SavedGame { path } => {
            let (world, saved_notes) = restore_saved_world(film, head, path, set).map_err(|e| fail(format!("a film of a saved game: {e:#}")))?;
            notes.extend(saved_notes);
            let mut scene = FilmScene {
                source_file: Some(path.clone()),
                // Who plays is the world's: no setup player makes anything.
                setup: Some(aa_play::SkirmishSetup { map_id: world.map.id.clone(), players: Vec::new(), starting_resources: None, seed: 0 }),
                ..FilmScene::default()
            };
            if data.draws_scene {
                let bytes = std::fs::read(path).map_err(|e| fail(format!("a film of a saved game: {e}")))?;
                scene.generated = Some(file_scene(&world, bytes, data, set).map_err(|e| fail(format!("a film of a saved game's scene: {e:#}")))?);
            }
            return Ok((world, scene, notes));
        }
        FilmSource::Plain => {
            return Err(fail("a film whose creator is the base KKC_WorldCreator".to_owned()));
        }
    };
    let mut plan = replay::plan(film, &set.game, &map).map_err(|e| fail(format!("{e:#}")))?;
    notes.extend(plan.notes.iter().cloned());
    // Natively a random-map creator's picks draw before the generator runs (the
    // draws 50ECA6 and 474C14), not at the map's restore. The world keeps the
    // film's picks here: `generate_world` drew them on its stream already.
    if random.is_some() {
        for p in &mut plan.players {
            p.random_nation = false;
            p.random_general = false;
        }
        plan.closed.clear();
    }
    let map = Arc::new(map);
    let mut world = replay::build_with(&plan, set.generals.clone(), set.game.clone(), map)
        .map_err(|e| fail(format!("{e:#}")))?;
    film_fog(film, &mut world);
    // A metagame battle's launch orders, which the film's rows hold.
    if random.is_some() {
        let slots: Vec<String> = plan.players.iter().map(|p| p.slot_id.clone()).collect();
        crate::recording::apply_battle_extras(&mut world, &crate::recording::film_extras(&film.header.map_header, &slots));
    }
    notes.extend(
        replay::pick_mismatches(&plan, &world)
            .into_iter()
            .map(|m| format!("random pick: {m}")),
    );
    // The names the session starts with (the map strings of 0x50E760).
    for (k, p) in plan.players.iter().enumerate() {
        if let (Some(kingdom), Some(info)) = (
            world.kingdoms.get_mut(k),
            film.header.header_kingdoms.iter().find(|i| i.id.eq_ignore_ascii_case(&p.slot_id)),
        ) {
            kingdom.name = head.map_string(&info.display);
        }
    }
    // A map file the extracted set does not hold keeps its own triggers (the lookup
    // by map id of `triggers::attach` would find none, or the archive's) and is drawn
    // from the file.
    let mut drawn = None;
    if let FilmSource::Scenario { file: Some(file), map_id } = source {
        let bytes = std::fs::read(file).map_err(|e| fail(format!("map {map_id}: {e}")))?;
        aa_play::triggers::attach_bytes(&mut world, bytes.clone(), &data.game_root);
        if data.draws_scene {
            drawn = Some(file_scene(&world, bytes, data, set).map_err(|e| fail(format!("map {map_id}'s scene: {e:#}")))?);
        }
    }
    let scene = FilmScene {
        source_file: match source {
            FilmSource::Scenario { file, .. } => file.clone(),
            _ => None,
        },
        setup: Some(plan.setup.clone()),
        random: random.as_ref().map(|g| g.params.clone()),
        generated: random.or(drawn),
    };
    Ok((world, scene, notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two films' header times against the original's dialog, which shows them as
    /// `3:39:08 PM 10/24/2026` and `12:53:07 AM 10/25/2026`
    /// (qa-output/original-ui/film/03-film-file-dialog.png).
    #[test]
    fn the_header_time_is_read_as_the_dialog_shows_it() {
        let t = system_time(0x01dd_63cd_cf27_1245);
        assert_eq!((t.year, t.month, t.day), (2026, 10, 24));
        assert_eq!((t.hour, t.minute, t.second), (15, 39, 8));
        let t = system_time(0x01dd_641b_32dd_2258);
        assert_eq!((t.year, t.month, t.day), (2026, 10, 25));
        assert_eq!((t.hour, t.minute, t.second), (0, 53, 7));
        // 2026-10-24 was a Saturday.
        assert_eq!(system_time(0x01dd_63cd_cf27_1245).day_of_week, 6);
    }

    /// 0x68E340: the first units of the name against `last_film_filename`,
    /// whatever follows them and in any case.
    #[test]
    fn the_rotation_films_show_their_time() {
        let time = "3:39:08 PM 10/24/2026";
        assert_eq!(row_text("LastFilm_2", time, "LastFilm"), "LastFilm 3:39:08 PM 10/24/2026");
        assert_eq!(row_text("lastfilm_0 (2)", time, "LastFilm"), "lastfilm 3:39:08 PM 10/24/2026");
        assert_eq!(row_text("Southern Europe 3_06_42 PM", time, "LastFilm"), "Southern Europe 3_06_42 PM");
        assert_eq!(row_text("Last", time, "LastFilm"), "Last");
        assert_eq!(row_text("LastFilm", time, ""), "LastFilm");
    }

    #[test]
    fn a_film_in_a_profile_plays_on_its_data_folder() {
        let inside = Path::new("x/Profile/data/Films/LastFilm_1.RFM");
        assert_eq!(profile_data_folder(inside), Some(PathBuf::from("x/Profile/data")));
        let below = Path::new("x/Profile/data/Films/archive/LastFilm_1.RFM");
        assert_eq!(profile_data_folder(below), Some(PathBuf::from("x/Profile/data")));
        assert_eq!(profile_data_folder(Path::new("elsewhere/LastFilm_1.RFM")), None);
    }

    #[test]
    fn a_map_resource_names_its_map_id() {
        assert_eq!(map_id_of("Maps/(2v2)Berlin.RMP").as_deref(), Some("(2v2)Berlin"));
        assert_eq!(
            map_id_of(r"maps\DeveloperMaps\(4FFA)Farmhouses.rmp").as_deref(),
            Some("DeveloperMaps/(4FFA)Farmhouses")
        );
        assert_eq!(map_id_of("Save/for.RSG"), None);
    }

    #[test]
    fn every_failure_is_its_native_text() {
        let name = || "x".to_owned();
        let keys = [
            (FilmFailure::MissingFile { name: name() }, "load_error_missing_map|s"),
            (FilmFailure::SourceFileNotFound { name: name() }, "unable_to_select_film_source_file_not_found|s"),
            (FilmFailure::SourceMapMissing { source: name() }, "failed_film_source_map_missing|s"),
            (FilmFailure::WrongVersion { version: name() }, "failed_film_wrong_version|s"),
        ];
        for (failure, key) in keys {
            assert_eq!(failure.key(), key);
            assert_eq!(failure.argument(), FailureArgument::Text(name()));
        }
        let build = FilmFailure::WrongBuild { build: 4129.7 };
        assert_eq!(build.key(), "failed_film_wrong_build|g");
        assert_eq!(build.argument(), FailureArgument::Number(4129.7));
    }
}
