//! The saved campaigns: the file dialogs of a World War II campaign, mode 3 of the
//! file dialog with its flag set (`0x68DB60(3, 1, ..)` from DynamicCampaignLoad
//! 932A00 and MetaGameLoadDialog, `0x68DCE0(3, ..)` from MetaGameSaveDialog; setup
//! `0x68BB70`: AVar PathDynamicCampaignSavedGames, extension RSG, newest first) and
//! the save itself (the MetaGameSave order 934FA0, 4F2DB0 with the METAGAME chunk,
//! 938EE0; load 4FC600 -> 93A3B0).
//!
//! Compare against the original: the file is the original's chunk file ("TGCK",
//! `aa_metagame::chunk_file`) with the original's METAGAME payload, but what the
//! native map world's other chunks hold of a campaign is not written: the armies
//! are the chunk `AA_ARMIES` here (each army's type, owner and the native army
//! component record 93EEA0), the local kingdom `AA_LOCAL`. AA.exe cannot load these
//! files, and the original's own saved campaigns are not read yet (their armies are
//! in DYNAMIC_THINGS). Labelled rendition; open item.
use super::Game;
use aa_metagame::battle::KingdomActor;
use aa_metagame::chunk_file::{ChunkFileReader, ChunkFileWriter};
use aa_metagame::save::{parse_metagame, write_metagame, ArmyComponentImage, EngineIdentities, SaveRegistries};
use aa_metagame::state::{ArmyId, MgArmy, MgState};
use aa_ui::controls::ListCallback;
use aa_ui::ui::Ui;
use aa_ui::widget::{Widget, WidgetClass as C};
use std::path::{Path, PathBuf};

const LOAD_DIALOG: &str = "UI/Shared/staging_file_dialog.tgi:StagingFileLoadSavedDialog";
const SAVE_DIALOG: &str = "UI/Shared/staging_file_saved_dialog.tgi:StagingFileSavedDialog";
const LOAD_ROOT: &str = "StagingFileLoadSavedDialog";
const SAVE_ROOT: &str = "StagingFileSavedDialog";
/// `0x68BB70` mode 3: `[A47734]`.
const EXTENSION: &str = "RSG";
const CHUNK_METAGAME: &str = "METAGAME";
const CHUNK_ARMIES: &str = "AA_ARMIES";
const CHUNK_LOCAL: &str = "AA_LOCAL";

/// The GTID handles the payload stores: the kingdom actors and armies have none in
/// this client, so a file numbers them.
fn identities(state: &MgState) -> EngineIdentities {
    EngineIdentities {
        kingdom_actors: state.kingdoms.iter().filter_map(|k| k.actor).map(|a| (a, 0x1000 + a.0)).collect(),
        armies: (0..state.armies.len() as u32).filter(|i| state.armies[*i as usize].is_some()).map(|i| (ArmyId(i), 0x10_0000 + i)).collect(),
    }
}

/// The registries 93A3B0's readers search: the factions, technologies (the shops'
/// and the divisions'), help popups and the armies' Things.
fn registries(game_data: &super::data::CampaignData, help_popups: Vec<String>) -> SaveRegistries {
    let d = &game_data.meta;
    let mut shared_properties: Vec<String> = Vec::new();
    for c in &d.campaigns {
        for k in &c.kingdoms {
            shared_properties.extend(k.shop.technologies.iter().filter_map(|t| t.ids.clone()));
        }
    }
    for x in &d.divisions {
        shared_properties.extend(x.technologies.iter().flatten().cloned());
    }
    SaveRegistries {
        factions: game_data.view.nations.values().flat_map(|n| n.factions.iter().map(|f| f.ids.clone())).collect(),
        shared_properties,
        help_popups,
        things: d.armies.iter().map(|a| (a.ids.clone(), true)).collect(),
    }
}

/// The file of a running campaign.
pub fn encode(game: &Game) -> Result<Vec<u8>, String> {
    let state = &game.campaign.state;
    let data = &*game.campaign.data;
    let ids = identities(state);
    let payload = write_metagame(state, data, &ids).map_err(|e| e.to_string())?;
    let mut armies: Vec<u8> = Vec::new();
    armies.extend((state.armies.len() as u32).to_le_bytes());
    for a in state.armies.iter() {
        match a {
            None => armies.extend(u32::MAX.to_le_bytes()),
            Some(a) => {
                armies.extend((a.definition as u32).to_le_bytes());
                armies.extend(a.owner.0.to_le_bytes());
                let record = ArmyComponentImage::from_army(a, -1).write(data).map_err(|e| e.to_string())?;
                armies.extend((record.len() as u32).to_le_bytes());
                armies.extend(record);
            }
        }
    }
    let local = game.campaign.local.map_or(u32::MAX, |l| l as u32);
    let mut w = ChunkFileWriter::create("campaign");
    for (name, bytes) in [(CHUNK_METAGAME, payload.as_slice()), (CHUNK_ARMIES, armies.as_slice()), (CHUNK_LOCAL, &local.to_le_bytes()[..])] {
        w.open_chunk(name, false).map_err(|e| e.to_string())?;
        w.write(bytes).map_err(|e| e.to_string())?;
    }
    w.close().map_err(|e| e.to_string())
}

fn take_u32(b: &[u8], at: &mut usize) -> Result<u32, String> {
    let v = b.get(*at..*at + 4).ok_or("the saved campaign's armies end early")?;
    *at += 4;
    Ok(u32::from_le_bytes(v.try_into().map_err(|_| "a word")?))
}

/// What a file holds: the state and the local kingdom.
pub fn decode(bytes: &[u8], game_data: &super::data::CampaignData, help_popups: Vec<String>) -> Result<(MgState, Option<usize>), String> {
    let data = &game_data.meta;
    let mut r = ChunkFileReader::open("campaign", bytes).map_err(|e| e.to_string())?;
    let payload = r.chunk(CHUNK_METAGAME).map_err(|e| e.to_string())?;
    let parsed = parse_metagame(payload, data, &registries(game_data, help_popups)).map_err(|e| e.to_string())?;
    let territories = parsed.image.territories.len();
    let campaign = parsed.image.campaign.ok_or("a saved campaign without its campaign")?;
    let c = data.campaigns.get(campaign).ok_or("no such campaign")?;
    let armies_bytes = r.chunk(CHUNK_ARMIES).map_err(|e| e.to_string())?;
    let mut at = 0usize;
    let count = take_u32(armies_bytes, &mut at)? as usize;
    let mut armies: Vec<Option<MgArmy>> = Vec::with_capacity(count);
    for _ in 0..count {
        let definition = take_u32(armies_bytes, &mut at)?;
        if definition == u32::MAX {
            armies.push(None);
            continue;
        }
        let owner = KingdomActor(take_u32(armies_bytes, &mut at)?);
        let len = take_u32(armies_bytes, &mut at)? as usize;
        let record = armies_bytes.get(at..at + len).ok_or("a saved army ends early")?;
        at += len;
        let (image, _) = ArmyComponentImage::parse(record, data, territories).map_err(|e| e.to_string())?;
        let mut army = MgArmy { definition: definition as usize, owner, divisions: Vec::new(), fought: 0, territory: None, path: Vec::new(), waypoints: Vec::new() };
        image.apply_to(&mut army).map_err(|e| e.to_string())?;
        armies.push(Some(army));
    }
    let ids = EngineIdentities {
        kingdom_actors: (0..c.kingdoms.len() as u32).map(|k| (KingdomActor(k), 0x1000 + k)).collect(),
        armies: (0..count as u32).filter(|i| armies[*i as usize].is_some()).map(|i| (ArmyId(i), 0x10_0000 + i)).collect(),
    };
    let names = c.kingdoms.iter().enumerate().map(|(k, x)| (KingdomActor(k as u32), x.ids.clone())).collect();
    let state = parsed.into_state(&ids, armies, names).map_err(|e| e.to_string())?.state;
    let local = r.chunk(CHUNK_LOCAL).map_err(|e| e.to_string())?;
    let local = u32::from_le_bytes(local.try_into().map_err(|_| "the saved campaign's local kingdom")?);
    Ok((state, (local != u32::MAX).then_some(local as usize)))
}

/// The folder: the profile's PathDynamicCampaignSavedGames.
fn folder(ui: &Ui) -> Option<PathBuf> {
    let path = ui.avars.get("PathDynamicCampaignSavedGames").map(aa_ui::node::unquote).unwrap_or_else(|| "DynamicCampaignSave/".into());
    crate::session::own_profile().map(|p| p.join("data").join(path))
}

/// `0x68E340`: the files of the folder, newest first.
fn list(dir: &Path) -> Vec<(String, PathBuf, std::time::SystemTime)> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x.eq_ignore_ascii_case(EXTENSION)) {
                let name = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let time = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                out.push((name, p, time));
            }
        }
    }
    out.sort_by(|a, b| b.2.cmp(&a.2));
    out
}

/// What a dialog's command came to.
#[derive(Debug)]
pub enum Outcome {
    Open,
    Closed,
    /// The file to load.
    Load(PathBuf),
    /// Save under this name.
    Save(PathBuf),
}

/// A saved-campaign dialog on the screen.
pub struct Dialog {
    save: bool,
    dir: PathBuf,
    rows: Vec<(String, PathBuf)>,
    selected: Option<usize>,
    confirming: Option<PathBuf>,
}

fn root(save: bool) -> &'static str {
    if save { SAVE_ROOT } else { LOAD_ROOT }
}

impl Dialog {
    /// Init `0x68D280` and Activate of the mode: the list, the preview panel, the
    /// edit box and the three buttons.
    pub fn open(ui: &mut Ui, save: bool, default_name: &str) -> Option<Self> {
        let dir = folder(ui)?;
        let mut d = Widget::new(C::TopLevel, if save { SAVE_DIALOG } else { LOAD_DIALOG }, true);
        let mut sp = Widget::composite(C::Panel, "SelectPanel", false);
        let mut lb = Widget::composite(C::Listbox, "SelectListBox", false);
        if let Some(l) = lb.listbox_mut() {
            l.on_select = ListCallback::Command("Select \"%s\"".into());
            l.on_double = ListCallback::Command("SelectFromDouble \"%s\"".into());
        }
        sp.add_child(lb);
        d.add_child(sp);
        d.add_child(crate::ui::dialogs::label_button("SelectButton", "Commit"));
        d.add_child(Widget::push_button("CancelButton", false, "Cancel"));
        let mut mp = Widget::composite(C::Panel, "MapPreviewPanel", false);
        mp.add_child(Widget::composite(C::Viewport, "MapPreviewViewport", false));
        d.add_child(mp);
        d.add_child(Widget::push_button("DeleteButton", false, "Delete"));
        d.add_child(Widget::composite(C::Editbox, "DescriptionEditBox", false));
        let mut notice = Widget::composite(C::Panel, "NoticeDialog", false);
        notice.add_child(Widget::composite(C::Label, "StatLabel", false));
        notice.set_visible(false);
        d.add_child(notice);
        aa_ui::screens::show(ui, d);
        let name = root(save);
        let key = if save { "#staging_file_save_button" } else { "#staging_file_load_button" };
        let label = ui.strings.resolve(key);
        ui.set_label(&format!("{name}/SelectButton/Client"), &label);
        let mut this = Self { save, dir, rows: Vec::new(), selected: None, confirming: None };
        this.fill(ui);
        if save {
            ui.editbox_set_text(&format!("Root/{name}/DescriptionEditBox"), default_name);
            ui.focus_editbox(&format!("Root/{name}/DescriptionEditBox"));
        }
        ui.layout();
        Some(this)
    }

    fn list_path(&self) -> String {
        format!("Root/{}/SelectPanel/SelectListBox", root(self.save))
    }

    fn fill(&mut self, ui: &mut Ui) {
        let rows = list(&self.dir);
        ui.listbox_clear(&self.list_path());
        self.rows = rows.iter().map(|(n, p, _)| (n.clone(), p.clone())).collect();
        for (n, p) in &self.rows {
            ui.listbox_add(&self.list_path(), n, &p.to_string_lossy());
        }
        // the load mode selects the newest (+0x11C set in mode 3)
        self.selected = None;
        if !self.save && !self.rows.is_empty() {
            ui.listbox_select(&self.list_path(), 0, false);
            self.select(ui, 0);
        }
    }

    fn select(&mut self, ui: &mut Ui, i: usize) {
        self.selected = Some(i);
        if let Some((n, _)) = self.rows.get(i) {
            let edit = format!("Root/{}/DescriptionEditBox", root(self.save));
            ui.editbox_set_text(&edit, n);
            ui.focus_editbox(&edit);
        }
    }

    fn close(&self, ui: &mut Ui) {
        ui.disconnect(root(self.save));
        ui.layout();
    }

    fn message(ui: &mut Ui, text: String, buttons: Vec<(String, String)>) {
        ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons });
    }

    /// A command of the dialog; `None` when it is not one.
    pub fn command(&mut self, ui: &mut Ui, cmd: &str) -> Option<Outcome> {
        let arg = cmd.split_once(' ').map(|(_, a)| a.trim().trim_matches('"').to_string());
        let row_of = |rows: &[(String, PathBuf)], p: &str| rows.iter().position(|r| r.1.to_string_lossy() == p);
        if cmd.starts_with("SelectFromDouble") {
            if let Some(i) = arg.and_then(|p| row_of(&self.rows, &p)) {
                self.select(ui, i);
            }
            return Some(self.commit(ui));
        }
        if cmd.starts_with("Select ") {
            if let Some(i) = arg.and_then(|p| row_of(&self.rows, &p)) {
                self.select(ui, i);
            }
            return Some(Outcome::Open);
        }
        if cmd.starts_with("CampaignFileDelete") {
            if let Some(p) = arg {
                let path = PathBuf::from(p);
                if path.parent() == Some(self.dir.as_path()) {
                    if let Err(e) = std::fs::remove_file(&path) {
                        bevy::log::warn!("saved campaign delete {}: {e}", path.display());
                        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                        let text = ui.strings.resolve("#ui_file_dialog_file_delete_failed|s").replacen("%s", &name, 1);
                        let ok = ui.strings.resolve("#button_ok");
                        Self::message(ui, text, vec![(ok, String::new())]);
                    }
                    self.fill(ui);
                }
            }
            return Some(Outcome::Open);
        }
        match cmd {
            "Commit" => Some(self.commit(ui)),
            "Cancel" => {
                self.close(ui);
                Some(Outcome::Closed)
            }
            "CampaignSaveOverwrite" => {
                let target = self.confirming.take()?;
                self.close(ui);
                Some(Outcome::Save(target))
            }
            "Delete" => {
                if let Some((n, p)) = self.selected.and_then(|i| self.rows.get(i)).cloned() {
                    let text = ui.strings.resolve("#delete_confirm|s").replacen("%s", &n, 1);
                    let ok = ui.strings.resolve("#button_ok");
                    let cancel = ui.strings.resolve("#button_cancel");
                    Self::message(ui, text, vec![(ok, format!("CampaignFileDelete \"{}\"", p.to_string_lossy())), (cancel, String::new())]);
                }
                Some(Outcome::Open)
            }
            "EditBoxChanged" => Some(Outcome::Open),
            _ => None,
        }
    }

    /// Commit `0x68EF50` / `0x6901D0`.
    fn commit(&mut self, ui: &mut Ui) -> Outcome {
        if !self.save {
            let Some((_, p)) = self.selected.and_then(|i| self.rows.get(i)).cloned() else { return Outcome::Open };
            self.close(ui);
            return Outcome::Load(p);
        }
        let text = ui.editbox_text(&format!("Root/{SAVE_ROOT}/DescriptionEditBox")).unwrap_or_default();
        let name: String = text.trim().chars().map(|c| if crate::film_record::film_name_char(c) { c } else { '_' }).collect();
        if name.is_empty() {
            return Outcome::Open;
        }
        let extension = format!(".{EXTENSION}");
        let stem = match name.len().checked_sub(extension.len()) {
            Some(at) if name.is_char_boundary(at) && name[at..].eq_ignore_ascii_case(&extension) => name[..at].to_string(),
            _ => name.clone(),
        };
        let target = self.dir.join(format!("{stem}.{EXTENSION}"));
        if target.exists() {
            let text = ui.strings.resolve("#ui_file_dialog_file_overwrite|s").replacen("%s", &stem, 1);
            let ok = ui.strings.resolve("#button_ok");
            let cancel = ui.strings.resolve("#button_cancel");
            self.confirming = Some(target);
            Self::message(ui, text, vec![(ok, "CampaignSaveOverwrite".into()), (cancel, String::new())]);
            return Outcome::Open;
        }
        self.close(ui);
        Outcome::Save(target)
    }
}

/// Write a campaign's file.
pub fn save(game: &Game, target: &Path) -> Result<(), String> {
    let bytes = encode(game)?;
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(target, bytes).map_err(|e| e.to_string())
}
