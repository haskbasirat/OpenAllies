//! The client's side of film recording (research/byte-exact/FILM-PLAYBACK.md
//! section 9, items F11 and F12): the session's film at the battle's start and
//! the Save Film dialog of the score screen. The decree stream and the file are
//! `aa_parity::recording`.
//!
//! A battle always runs on the decree stream (`Battle::recorder`), so the world
//! and S are the same with a file and without one. What gets a file:
//! OnGWorldReadyForState 0x7757F0 in state 2 opens one for every launch but the
//! test and editor launches (Session+0xF8 7, 0xE); in this client that is a launch
//! from the staging screen (`PendingFilmSession`). A film being viewed records
//! nothing, and leaves Session+0x150 empty, so its score screen has no Save Film.
//! The films go to the client's own profile only (lead's decision 6), never to
//! the folder the original writes to.
use crate::session::{Battle, own_profile};
use crate::ui::front::Front;
use crate::ui::gamedata::GameData;
use aa_parity::playback::{FilmDir, FilmRow, list_films_named};
use aa_parity::recording::{self, BattleExtras, FilmOpened, FilmSession, FilmSource, FilmTarget, GameVersion, Recorder, RuleRow};
use aa_parity::registries::Registries;
use aa_play::mapgen::RandomMapParams;
use aa_play::SkirmishSetup;
use aa_ui::controls::ListCallback;
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What the launch knows that the world does not (the staging screen's state at
/// 0x7757F0). Its presence asks for a film of the coming battle.
#[derive(Resource, Clone, Debug)]
pub struct PendingFilmSession {
    pub setup: SkirmishSetup,
    pub random: Option<RandomMapParams>,
    /// The session's world rules (`Custom::film_rule_rows`).
    pub rules: Vec<RuleRow>,
    /// (player index = kingdom id, SAI profile).
    pub profiles: Vec<(usize, String)>,
    /// Kingdoms whose general staging left to the draw (`StartPick`).
    pub random_generals: Vec<usize>,
}

/// The film folder of the client's own profile: `<profile>/data/` + AVar
/// PathFilms, as `custom::film_dirs` lists it.
pub fn own_film_folder(ui: &Ui) -> Option<PathBuf> {
    let films = ui.avars.get("PathFilms").map(aa_ui::node::unquote).unwrap_or_else(|| "Films/".into());
    own_profile().map(|p| p.join("data").join(films))
}

fn settings_path() -> Option<PathBuf> {
    own_profile().map(|p| p.join("settings.json"))
}

/// UVar `LastFilmIndex`, kept with the options (natively in the profile's
/// `User/UVars.tgi`; it is registered by the code, not in `Data/UVars.tgi`).
fn load_last_film_index() -> i32 {
    settings_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.get("last_film_index")?.as_i64())
        .map_or(0, |v| v as i32)
}

fn store_last_film_index(index: i32) {
    let Some(path) = settings_path() else { return };
    let mut v: serde_json::Value = std::fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = v.as_object_mut() {
        obj.insert("last_film_index".into(), index.into());
    }
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
    if let Ok(text) = serde_json::to_string_pretty(&v) {
        let _ = std::fs::write(path, text);
    }
}

/// 0x76FE60's base: the localized `last_film_filename`, else AVars `LastFilmName`.
fn rotation_base(ui: &Ui) -> String {
    let localized = ui.strings.resolve("#last_film_filename");
    if !localized.is_empty() && !localized.starts_with('#') {
        return localized;
    }
    ui.avars.get("LastFilmName").map(aa_ui::node::unquote).unwrap_or_else(|| "LastFilm.RFM".into())
}

/// The decree stream of a battle that is starting, and its film when the launch
/// asks for one (OnGWorldReadyForState 0x7757F0, after the generals are set).
pub fn start(mut commands: Commands, battle: Option<ResMut<Battle>>, session: Option<Res<PendingFilmSession>>, metagame: Option<Res<crate::campaign::battle::PendingBattle>>, front: Option<ResMut<Front>>) {
    let Some(mut b) = battle else { return };
    if !b.is_added() {
        return;
    }
    let mut front = front;
    // A film being viewed: Session+0x150 stays empty (0x658FF0), nothing records.
    if let Some(f) = front.as_deref_mut() {
        f.session_film = None;
    }
    if b.in_film() || b.recorder.is_some() {
        return;
    }
    let Some(root) = front.as_deref().map(|f| f.root.parent().map_or_else(|| f.root.clone(), Path::to_path_buf)) else { return };
    let registries = match Registries::cached(&root, None) {
        Ok(r) => Arc::new(r),
        Err(e) => {
            warn!("film: the registries did not load, the battle has no decree stream: {e:#}");
            return;
        }
    };
    let battle = &mut *b;
    let mut film = None;
    if let (Some(info), Some(f)) = (session.as_deref(), front.as_deref_mut()) {
        commands.remove_resource::<PendingFilmSession>();
        let extras = metagame.as_deref().map_or_else(Default::default, |p| BattleExtras { properties: p.properties.clone(), bonus: p.bonus.clone(), actors: p.actors.clone() });
        match open_session_film(&battle.world, info, extras, &root, &f.ui) {
            Ok((file, opened)) => {
                info!("film: recording {}", opened.path.display());
                store_last_film_index(opened.next_index);
                f.session_film = Some(opened.clone());
                film = Some((file, opened));
            }
            Err(FilmOpenError::File(e)) => {
                // 0x775A40: `failed_to_create_film|s`; the game goes on without one.
                warn!("film: {e}");
                let text = f.ui.strings.format("failed_to_create_film|s", &[&e]);
                let uv = f.ui.uvars.clone();
                f.ui.messages.send(&uv, "GENERIC_ERROR", &text, None);
            }
            // A battle whose header this client cannot write yet: no native text.
            Err(FilmOpenError::Header(e)) => warn!("film: no film of this battle: {e}"),
        }
    }
    let mut recorder = Recorder::new(&battle.world, registries, film);
    recorder.own_admin_clock = true;
    battle.recorder = Some(recorder);
}

enum FilmOpenError {
    /// The rotation found no file to write (0x775A40).
    File(String),
    /// The film header of this battle could not be made.
    Header(String),
}

fn open_session_film(world: &aa_play::World, info: &PendingFilmSession, extras: BattleExtras, root: &Path, ui: &Ui) -> Result<(aa_parity::film_writer::FilmFile, FilmOpened), FilmOpenError> {
    use FilmOpenError::{File, Header};
    let folder = own_film_folder(ui).ok_or_else(|| File("no profile folder".into()))?;
    let source = match &info.random {
        None => FilmSource::Map { id: info.setup.map_id.clone() },
        Some(params) => {
            let maps = crate::preload::random_maps(root).map_err(|e| Header(format!("{e:#}")))?;
            let parts = maps.creator_parts(params).map_err(|e| Header(format!("{e:#}")))?;
            FilmSource::Random { params: params.clone(), parts }
        }
    };
    let session = FilmSession {
        source,
        seed: info.setup.seed,
        local: world.kingdoms.iter().find(|k| matches!(k.controller, aa_play::Controller::Human)).map(|k| k.id),
        rules: info.rules.clone(),
        random_generals: info.random_generals.iter().map(|k| *k as u32).collect::<BTreeSet<_>>(),
        random_nations: BTreeSet::new(),
        sai_profiles: info.profiles.iter().map(|(k, p)| (*k as u32, p.clone())).collect(),
        player_names: info.setup.players.iter().enumerate().map(|(k, p)| (k as u32, p.name.clone())).collect::<BTreeMap<_, _>>(),
        version: GameVersion::load(root).map_err(|e| Header(format!("{e:#}")))?,
        filetime: recording::filetime_now(),
        extras,
    };
    let prelude = recording::prelude(world, root, &session).map_err(|e| Header(format!("{e:#}")))?;
    let queue = ui.avars.get("FilmRotationQueueSize").and_then(|v| aa_ui::node::unquote(v).trim().parse::<i32>().ok()).unwrap_or(5);
    let target = FilmTarget { folder, base: rotation_base(ui), last_film_index: load_last_film_index(), queue };
    recording::open_film_file(&prelude, &target).map_err(|e| File(format!("{}: {}", e.name, e.error)))
}

/// 0x772BB0: Session+0x150 is set and `<PathFilms><name>.RFM` exists.
pub fn film_exists(session_film: Option<&FilmOpened>) -> bool {
    session_film.is_some_and(|o| o.path.is_file())
}

// ------------------------------------------------------------ Save Film

const DIALOG: &str = "UI/Shared/staging_file_saved_dialog.tgi:StagingFileSavedDialog";
const ROOT: &str = "StagingFileSavedDialog";
const FD: &str = "Root/StagingFileSavedDialog";
const LIST: &str = "Root/StagingFileSavedDialog/SelectPanel/SelectListBox";
const EDIT: &str = "Root/StagingFileSavedDialog/DescriptionEditBox";

/// `StagingFileSavedDialog` in mode 5 (ScoringSaveFilm 0x658F90 ->
/// 0x68DCE0(5, 0, 0), Init 0x68BB70, Activate 0x690E80): the films of the client's
/// own folder with their preview, the edit box with a name from the map and the
/// time, Save, Delete and Cancel. Natively it is the file dialog's class; this
/// client's film dialog is in `ui/dialogs.rs`, for the load mode.
pub struct SaveFilmDialog {
    folder: PathBuf,
    source: PathBuf,
    rows: Vec<FilmRow>,
    /// The name waiting for the overwrite answer.
    confirming: Option<PathBuf>,
    /// Where the film went (0x690D75 remembers it in the preferences).
    pub saved: Option<PathBuf>,
}

/// What a dialog command came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Open,
    Closed,
}

fn w(class: C, name: &str) -> Widget {
    Widget::composite(class, name, false)
}

impl SaveFilmDialog {
    /// 0x68DCE0(5, 0, 0).
    pub fn open(ui: &mut Ui, session: &FilmOpened, map_name: &str) -> Self {
        let folder = session.path.parent().map(Path::to_path_buf).unwrap_or_default();
        let mut d = Widget::new(C::TopLevel, DIALOG, true);
        let mut sp = w(C::Panel, "SelectPanel");
        let mut list = Widget::composite(C::Listbox, "SelectListBox", false);
        if let Some(l) = list.listbox_mut() {
            l.on_select = ListCallback::Command("Select \"%s\"".into());
        }
        sp.add_child(list);
        d.add_child(sp);
        d.add_child(crate::ui::dialogs::label_button("SelectButton", "SaveFilmCommit"));
        d.add_child(Widget::push_button("CancelButton", false, "FilmCancel"));
        let mut mp = w(C::Panel, "MapPreviewPanel");
        mp.add_child(Widget::composite(C::Viewport, "MapPreviewViewport", false));
        d.add_child(mp);
        d.add_child(Widget::push_button("DeleteButton", false, "SaveFilmDelete"));
        d.add_child(Widget::composite(C::Editbox, "DescriptionEditBox", false));
        let mut notice = w(C::Panel, "NoticeDialog");
        notice.add_child(w(C::Label, "StatLabel"));
        notice.set_visible(false);
        d.add_child(notice);
        aa_ui::screens::show(ui, d);
        let save = ui.strings.resolve("#staging_file_save_button");
        ui.set_label(&format!("{ROOT}/SelectButton/Client"), &save);
        let mut label = w(C::Label, "MapPreviewLabel");
        crate::ui::dialogs::load_item(ui, &mut label, &format!("{DIALOG}/MapPreviewPanel/MapPreviewViewport/ItemPanel"));
        ui.viewport_add(&format!("{FD}/MapPreviewPanel/MapPreviewViewport"), label);
        let mut dlg = Self { folder, source: session.path.clone(), rows: Vec::new(), confirming: None, saved: None };
        dlg.fill(ui);
        // 0x690E80: the default name is the map's name and the time (0x6FBF50).
        let time = aa_parity::playback::film_time_text(now_filetime());
        ui.editbox_set_text(EDIT, &format!("{map_name} {time}"));
        ui.focus_editbox(EDIT);
        ui.layout();
        dlg
    }

    /// The film rows of the folder (0x68E340): only the client's own.
    fn fill(&mut self, ui: &mut Ui) {
        self.rows = list_films_named(&[FilmDir::own(self.folder.clone())], &ui.strings.resolve("#last_film_filename"));
        ui.listbox_clear(LIST);
        for r in &self.rows {
            ui.listbox_add(LIST, &r.text, &r.path.to_string_lossy());
        }
    }

    /// A row picked: its preview (0x68A820 -> 0x6612E0) and its file's name in
    /// the edit box.
    fn select_row(&self, ui: &mut Ui, data: &GameData, row: &FilmRow) {
        let lp = format!("{FD}/MapPreviewPanel/MapPreviewViewport/ItemPanel/MapPreviewLabel");
        let text = crate::ui::dialogs::film_preview_text(ui, data, row);
        ui.set_label(&lp["Root/".len()..], &text);
        aa_ui::staging::label_fit_height(ui, &lp);
        ui.viewport_refresh(&format!("{FD}/MapPreviewPanel/MapPreviewViewport"));
        let name = row.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        ui.editbox_set_text(EDIT, &name);
        ui.focus_editbox(EDIT);
        ui.layout();
    }

    fn close(ui: &mut Ui) {
        ui.disconnect(ROOT);
        ui.layout();
    }

    fn message(ui: &mut Ui, text: String) {
        let ok = ui.strings.resolve("#button_ok");
        ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, String::new())] });
    }

    /// A command of the dialog; `None` when it is not one.
    pub fn command(&mut self, ui: &mut Ui, data: &GameData, cmd: &str) -> Option<SaveOutcome> {
        let arg = cmd.split_once(' ').map(|(_, a)| a.trim().trim_matches('"').to_string());
        if cmd.starts_with("Select ") {
            if let Some(row) = arg.and_then(|p| self.rows.iter().find(|r| r.path.to_string_lossy() == p).cloned()) {
                self.select_row(ui, data, &row);
            }
            return Some(SaveOutcome::Open);
        }
        // `delete_confirm|s` with the name, Ok runs `FileDelete` (0x68C5B0).
        if cmd.starts_with("SaveFilmFileDelete") {
            if let Some(path) = arg {
                self.delete(ui, Path::new(&path));
            }
            return Some(SaveOutcome::Open);
        }
        match cmd {
            "FilmCancel" | "Cancel" => {
                Self::close(ui);
                Some(SaveOutcome::Closed)
            }
            "SaveFilmCommit" | "Commit" => Some(self.commit(ui)),
            "SaveFilmOverwrite" => {
                let Some(target) = self.confirming.take() else { return Some(SaveOutcome::Open) };
                Some(self.save(ui, &target))
            }
            "SaveFilmDelete" | "Delete" => {
                let name = ui.editbox_text(EDIT).unwrap_or_default().trim().to_owned();
                let path = recording::film_path(&self.folder, &name);
                if path.is_file() && path != self.source {
                    let text = ui.strings.resolve("#delete_confirm|s").replacen("%s", &name, 1);
                    let ok = ui.strings.resolve("#button_ok");
                    let cancel = ui.strings.resolve("#button_cancel");
                    let command = format!("SaveFilmFileDelete \"{}\"", path.to_string_lossy());
                    ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, command), (cancel, String::new())] });
                }
                Some(SaveOutcome::Open)
            }
            _ => None,
        }
    }

    /// 0x6901D0 before 0x6911F0: a character 0x859710 refuses becomes `_`, so
    /// the default name's `:` and `/` do (a real profile's saved films read
    /// `<map> 3_15_05 PM 3_30_2024`); `..` or a folder's name gets a box; an
    /// existing film is overwritten only when confirmed.
    fn commit(&mut self, ui: &mut Ui) -> SaveOutcome {
        let text = ui.editbox_text(EDIT).unwrap_or_default();
        let name: String = text.trim().chars().map(|c| if film_name_char(c) { c } else { '_' }).collect();
        if name.is_empty() {
            return SaveOutcome::Open;
        }
        if name == ".." || self.folder.join(&name).is_dir() {
            let text = ui.strings.resolve("#ui_file_dialog_file_folders_not_allowed|s").replacen("%s", &name, 1);
            Self::message(ui, text);
            return SaveOutcome::Open;
        }
        // 0x85A660: the dialog's extension goes on unless the name has it.
        let extension = format!(".{}", aa_parity::film_writer::FILM_EXTENSION);
        let stem = match name.len().checked_sub(extension.len()) {
            Some(at) if name.is_char_boundary(at) && name[at..].eq_ignore_ascii_case(&extension) => &name[..at],
            _ => name.as_str(),
        };
        let name = stem.to_owned();
        let target = recording::film_path(&self.folder, &name);
        if target.exists() {
            let text = ui.strings.resolve("#ui_file_dialog_file_overwrite|s").replacen("%s", &name, 1);
            let ok = ui.strings.resolve("#button_ok");
            let cancel = ui.strings.resolve("#button_cancel");
            self.confirming = Some(target);
            ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, "SaveFilmOverwrite".into()), (cancel, String::new())] });
            return SaveOutcome::Open;
        }
        self.save(ui, &target)
    }

    /// 0x690A00: the current film becomes the target. MoveFileW (0x8597B0): the
    /// `.RFM` goes, its `.TGI` stays where it was, as in a real profile, and the
    /// caller's `UpdateSaveFilmButton` then finds no film (0x772BB0). A film
    /// already there is deleted first (0x5AD860); one that cannot be gets the
    /// box. No current film is nothing to do, and so is the current film onto
    /// itself (0x7729E0).
    fn save(&mut self, ui: &mut Ui, target: &Path) -> SaveOutcome {
        let same = target.to_string_lossy().eq_ignore_ascii_case(&self.source.to_string_lossy());
        if !same && self.source.is_file() {
            if target.exists() && let Err(e) = std::fs::remove_file(target) {
                warn!("film save {}: {e}", target.display());
                let name = target.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let text = ui.strings.resolve("#ui_file_dialog_file_delete_failed|s").replacen("%s", &name, 1);
                Self::message(ui, text);
                return SaveOutcome::Open;
            }
            if let Err(e) = std::fs::rename(&self.source, target) {
                warn!("film save {}: {e}", target.display());
            }
        }
        info!("film: saved as {}", target.display());
        self.saved = Some(target.to_path_buf());
        Self::close(ui);
        SaveOutcome::Closed
    }

    /// 0x68C5B0: only ever a film of the client's own folder.
    fn delete(&mut self, ui: &mut Ui, path: &Path) {
        if path.parent() != Some(self.folder.as_path()) || path == self.source {
            return;
        }
        if let Err(e) = std::fs::remove_file(path) {
            warn!("film delete {}: {e}", path.display());
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let text = ui.strings.resolve("#ui_file_dialog_file_delete_failed|s").replacen("%s", &name, 1);
            Self::message(ui, text);
            return;
        }
        let _ = std::fs::remove_file(path.with_extension("TGI"));
        self.fill(ui);
    }
}

/// 0x859710: what a saved film's name may hold.
pub(crate) fn film_name_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, ' ' | '!' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '+' | ',' | '-' | '.' | ';' | '=' | '@' | '^' | '_' | '`' | '~')
}

/// The local time as FILETIME ticks, what 6FB1F0 gives 0x6FBF50.
fn now_filetime() -> u64 {
    let [low, high] = recording::filetime_now();
    u64::from(high) << 32 | u64::from(low)
}
