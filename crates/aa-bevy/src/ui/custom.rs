//! Single Player -> Custom: the original custom-game staging screen
//! (StagingMenu, aa-ui `staging::Staging`) over an offline single-player
//! session, its launch-type dialogs (`dialogs.rs`) and Launch.
//!
//! The session follows KKC_Session as far as the staging screen shows it
//! (behaviour-faithful):
//! * entering (`RequestCustom` 0x64EF00 -> Host 0x75AEE0 ->
//!   RestoreLaunchSettings 0x77A000, fresh default): launch type RANDOM_MAP
//!   with the RMC `StagingSinglePlayerDefaultRMCIds` at
//!   `StagingSinglePlayerDefaultRMCWidth/Height` and the team format
//!   `StagingSinglePlayerDefaultRMCTeamConfig` ("0v0v0v0v1v1": two
//!   free-for-all seats); the local player "Player" (default_lan_name);
//! * the team configuration (session+0x14: 0x77E830 build, 0x781250 apply,
//!   0x77FAC0 validate, 0x780ED0 canonical string, 0x781C70 / 0x7806B0 /
//!   0x77DF00 the format list) and the AI fill / seating (0x7791F0: AIs
//!   up to the seat count, 0x7789D0 / 0x778B70 / 0x778D10 seating);
//! * `SelectLaunchType RANDOM_MAP / LOAD_MAP` opens the Random Map /
//!   Scenario Map dialog; their OK sets the session map (KKC_SetRMCOrder /
//!   KKC_HostSetLoadMapOrder -> 0x781DD0 reset, 0x7791F0);
//! * `TeamFormatDropdown %s` (0x65CC60 -> 0x779D80), `StagingSelectSAI %s`
//!   (0x69CDC0), the nation / faction / handicap functors (0x69E080 /
//!   0x69C1E0 / 0x69C2F0), `StagingConfirmExit` (ui_staging_confirm_exit);
//! * `StagingLaunch` (0x65DE80): the recommended-players check, then the
//!   battle setup for the loader (`session::PendingSetup`, plus
//!   `PendingRandomMap` for a random map). "Random" nations and computer
//!   generals are resolved here, as the original does when the world is
//!   configured (0x50E8B0, PickRandomProfile 0x79FAF0).
use super::dialogs::{self, FileDialog, Outcome, RandomDialog, RuleValue};
use super::gamedata::{GameData, MapInfo};
use aa_play::mapgen::RandomMapParams;
use aa_play::{Controller, PlayerSetup, SkirmishSetup};
use aa_ui::staging::{FactionInfo, KingdomSlot, LaunchType, NationInfo, Occupant, Staging, StagingState, TeamInfo};
use aa_ui::ui::{Ui, UiEvent};
use bevy::log::{info, warn};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub enum Action {
    None,
    Exit,
    Launch,
}

#[derive(Clone, Debug, PartialEq)]
enum Occ {
    Open,
    Human,
    /// `None` = random SAI.
    Ai(Option<String>),
}

/// A creator kingdom record (stride 0x6C) and its staging state.
#[derive(Clone, Debug)]
struct Record {
    /// The kingdom IDS of the map / RMC (aa-play's kingdom name for the
    /// independent ones).
    ids: String,
    /// The record's localized name ("Side 1"), shown in the map preview.
    side_name: String,
    color: [f32; 3],
    independent: bool,
    playable: bool,
    required: bool,
    /// The map team of the record, `None` = no team (pool / FFA).
    map_team: Option<usize>,
    fixed_nation: Option<String>,
    fixed_faction: Option<String>,
    // --- staging state
    /// Team the record seats on (map team index, or `map_teams.len()` for
    /// the no-team team), `None` = not in the format.
    team: Option<usize>,
    open: bool,
    occ: Occ,
    nation: Option<String>,
    faction: Option<String>,
    handicap: Option<String>,
}

#[derive(Clone, Debug)]
enum Source {
    Random { rmc: String, size: (f32, f32), terrain: Vec<f32> },
    Map(MapInfo),
}

#[derive(Clone, Debug)]
struct Session {
    source: Source,
    rules: Vec<RuleValue>,
    records: Vec<Record>,
    /// Map teams (ids, name) in map order.
    map_teams: Vec<(String, String)>,
    /// Some kingdom has no team: the no-team (FFA) team exists.
    has_ffa: bool,
    /// The applied format (+0x28).
    format: String,
}

impl Session {
    fn ffa_index(&self) -> usize {
        self.map_teams.len()
    }
    fn eligible(&self, i: usize) -> bool {
        let r = &self.records[i];
        r.playable && !r.independent
    }
    /// The pool (+0x10): kingdoms with no map team, not required.
    fn pool(&self) -> Vec<usize> {
        (0..self.records.len()).filter(|&i| self.eligible(i) && self.records[i].map_team.is_none() && !self.records[i].required).collect()
    }
    /// Team members of the map itself (fixed) and required no-team ones.
    fn fixed_members(&self, team: usize) -> Vec<usize> {
        (0..self.records.len())
            .filter(|&i| self.eligible(i))
            .filter(|&i| {
                let r = &self.records[i];
                if team == self.ffa_index() { r.map_team.is_none() && r.required } else { r.map_team == Some(team) }
            })
            .collect()
    }
    fn parse(&self, format: &str) -> Option<(Vec<usize>, usize)> {
        let mut teams = vec![0usize; self.map_teams.len()];
        let mut ffa = 0usize;
        for (i, t) in format.split('v').enumerate() {
            let v: usize = if t.is_empty() { 0 } else { t.parse().ok()? };
            if i < teams.len() {
                teams[i] = v;
            } else {
                if v > 1 || !self.has_ffa {
                    return None;
                }
                ffa += v;
            }
        }
        Some((teams, ffa))
    }
    /// 0x77FAC0.
    fn valid(&self, format: &str) -> bool {
        let Some((teams, ffa)) = self.parse(format) else { return false };
        let mut pool = self.pool().len();
        for (t, &v) in teams.iter().enumerate() {
            let fixed = self.fixed_members(t);
            let required = fixed.iter().filter(|&&i| self.records[i].required).count();
            if v < required || v > fixed.len() + pool {
                return false;
            }
            pool -= v.saturating_sub(fixed.len());
        }
        if self.has_ffa {
            let fixed = self.fixed_members(self.ffa_index());
            if ffa < fixed.len() || ffa > fixed.len() + pool {
                return false;
            }
        }
        true
    }
    /// 0x781250: seat the pool into the teams, open / close the members.
    fn apply(&mut self, format: &str) -> bool {
        if !self.valid(format) {
            return false;
        }
        let (teams, ffa) = self.parse(format).unwrap();
        for r in &mut self.records {
            r.team = None;
            r.open = false;
        }
        let mut pool = self.pool().into_iter();
        let mut targets: Vec<(usize, usize)> = teams.iter().copied().enumerate().collect();
        if self.has_ffa {
            targets.push((self.ffa_index(), ffa));
        }
        for (t, v) in targets {
            let mut members = self.fixed_members(t);
            while members.len() < v {
                match pool.next() {
                    Some(i) => members.push(i),
                    None => break,
                }
            }
            let required = members.iter().filter(|&&i| self.records[i].required).count();
            let mut optional_open = v.saturating_sub(required);
            for i in members {
                let r = &mut self.records[i];
                r.team = Some(t);
                r.open = if r.required {
                    true
                } else if optional_open > 0 {
                    optional_open -= 1;
                    true
                } else {
                    false
                };
            }
        }
        self.format = self.canonical();
        true
    }
    /// 0x780ED0: "%d"/"v%d" per team, "1"/"v1" per FFA seat, trailing "v0"
    /// removed.
    fn canonical(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for t in 0..self.map_teams.len() {
            parts.push(self.records.iter().filter(|r| r.team == Some(t) && r.open).count().to_string());
        }
        if self.has_ffa {
            for _ in self.records.iter().filter(|r| r.team == Some(self.ffa_index()) && r.open) {
                parts.push("1".into());
            }
        }
        let mut s = parts.join("v");
        while s.ends_with("v0") {
            s.truncate(s.len() - 2);
        }
        s
    }
    fn seats(&self) -> usize {
        self.records.iter().filter(|r| r.team.is_some() && r.open).count()
    }
    /// 0x7791F0: computer opponents up to the seat count, then the seating
    /// (previous seat, else the first open unused record in creator order);
    /// opponents left without a seat are removed.
    fn fill_and_seat(&mut self) {
        // players: the human first, then the AIs in seat order
        let mut players: Vec<(Occ, Option<usize>)> = Vec::new();
        let mut human = (Occ::Human, None);
        for (i, r) in self.records.iter().enumerate() {
            match &r.occ {
                Occ::Human => human = (Occ::Human, Some(i)),
                Occ::Ai(_) => players.push((r.occ.clone(), Some(i))),
                Occ::Open => {}
            }
        }
        players.insert(0, human);
        let seats = self.seats();
        while players.len() < seats {
            players.push((Occ::Ai(None), None));
        }
        // clear every kingdom (0x765DD0)
        let old: Vec<Record> = self.records.clone();
        for r in &mut self.records {
            r.occ = Occ::Open;
        }
        let n = self.records.len();
        let mut taken = vec![false; n];
        let mut placed: Vec<Option<usize>> = vec![None; players.len()];
        let seat_ok = |s: &Session, i: usize| s.records[i].team.is_some() && s.records[i].open && s.eligible(i);
        // 0x7789D0: the previous seat when it is still an open seat
        for (p, (_, prev)) in players.iter().enumerate() {
            if let Some(i) = *prev {
                if i < n && seat_ok(self, i) && !taken[i] {
                    taken[i] = true;
                    placed[p] = Some(i);
                }
            }
        }
        // 0x778D10: the first open unused record in creator order
        for p in 0..players.len() {
            if placed[p].is_some() {
                continue;
            }
            if let Some(i) = (0..n).find(|&i| !taken[i] && seat_ok(self, i)) {
                taken[i] = true;
                placed[p] = Some(i);
            }
        }
        for (p, (occ, prev)) in players.into_iter().enumerate() {
            let Some(i) = placed[p] else { continue };
            let o = prev.and_then(|j| old.get(j)).cloned();
            let r = &mut self.records[i];
            // a seated player keeps his choices (0x7789D0 reapplies the
            // preferred nation / faction); humans have no handicap
            r.nation = r.fixed_nation.clone().or(o.as_ref().and_then(|o| o.nation.clone()));
            r.faction = r.fixed_faction.clone().or(o.as_ref().and_then(|o| o.faction.clone()));
            r.handicap = if occ == Occ::Human { None } else { o.and_then(|o| o.handicap) };
            r.occ = occ;
        }
        // an unused record resets its nation and faction (0x777F00)
        for (i, r) in self.records.iter_mut().enumerate() {
            if !taken[i] {
                r.nation = r.fixed_nation.clone();
                r.faction = r.fixed_faction.clone();
                r.handicap = None;
            }
        }
    }
}

/// One TeamConfig list entry: (format id, display, imbalance).
fn format_entry(ui: &Ui, s: &Session, id: &str) -> Option<(String, String, u32)> {
    let (teams, ffa) = s.parse(id)?;
    let mut tokens: Vec<usize> = teams.into_iter().filter(|&v| v > 0).collect();
    tokens.extend(std::iter::repeat_n(1, ffa));
    if tokens.is_empty() {
        return None;
    }
    // 0x77F530: sum of |v - ceil(average)|
    let avg = (tokens.iter().sum::<usize>() as f32 / tokens.len() as f32).ceil() as i64;
    let imbalance = tokens.iter().map(|&v| (v as i64 - avg).unsigned_abs() as u32).sum();
    // 0x77F770: all ones -> "%d Free For All", else the compact string
    let display = if tokens.iter().all(|&v| v == 1) {
        ui.strings.format("team_management_ffa_label|d", &[&tokens.len().to_string()])
    } else {
        tokens.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("v")
    };
    Some((id.to_string(), display, imbalance))
}

/// The format list (0x781D70): every free-for-all and equal-team format
/// the pool can seat when every kingdom is in the pool (0x781C70), else
/// only the map's own format; sorted by 0x77DF00. The balanced ones are
/// shown, plus the current one (0x7806B0 with flag 1).
fn format_list(ui: &Ui, s: &Session) -> Vec<(String, String)> {
    let eligible = (0..s.records.len()).filter(|&i| s.eligible(i)).count();
    let pool = s.pool().len();
    let mut ids: Vec<String> = Vec::new();
    if pool == eligible && pool >= 2 {
        if s.has_ffa {
            for n in 2..=pool {
                let mut t: Vec<String> = vec!["0".to_string(); s.map_teams.len()];
                t.extend(std::iter::repeat_n("1".to_string(), n));
                ids.push(t.join("v"));
            }
        }
        for k in 2..=s.map_teams.len() {
            for size in 2..=pool / k {
                ids.push(vec![size.to_string(); k].join("v"));
            }
        }
    }
    let mut entries: Vec<(String, String, u32)> = ids.iter().filter(|id| s.valid(id)).filter_map(|id| format_entry(ui, s, id)).collect();
    let balanced: Vec<_> = entries.iter().filter(|e| e.2 == 0).cloned().collect();
    if !balanced.is_empty() {
        entries = balanced;
    }
    if !entries.iter().any(|e| e.0 == s.format) {
        if let Some(e) = format_entry(ui, s, &s.format) {
            entries.push(e);
        }
    }
    entries.sort_by(|a, b| {
        let one = |d: &str| d.chars().count() == 1;
        one(&a.1)
            .cmp(&one(&b.1))
            .then(a.2.cmp(&b.2))
            .then(a.1.chars().count().cmp(&b.1.chars().count()))
            .then(a.1.contains('v').cmp(&b.1.contains('v')))
            .then(a.1.cmp(&b.1))
    });
    entries.into_iter().map(|e| (e.0, e.1)).collect()
}

enum Dialog {
    File(FileDialog),
    Random(RandomDialog),
}

pub struct Custom {
    root: PathBuf,
    data: Option<GameData>,
    session: Option<Session>,
    pub state: StagingState,
    staging: Option<Staging>,
    dialog: Option<Dialog>,
    /// profile+0x2C: default_lan_name.
    player_name: Option<String>,
    seed: u64,
    rmc_prefs: HashMap<String, dialogs::RmcPrefs>,
    /// The local player's nation chosen at launch (for the loading art).
    launched_nation: Option<String>,
    /// The computer generals of the last launch: (player index, SAI ids).
    pub launched_profiles: Vec<(usize, String)>,
    /// The faction (general) of each launched player: the staging Faction
    /// choice, a computer's SAI profile faction; None = random for the
    /// nation (appended by the combat owner, for ui/abilities.rs).
    pub launched_factions: Vec<(usize, Option<String>)>,
}

/// The staging screen's game data (nations, SAIs, rules, RMCs), parsed on a
/// background thread as soon as the front end starts so the Custom button
/// does not stall a frame on it (about 0.2 s of TGI parsing).
static WARM: std::sync::Mutex<Option<std::thread::JoinHandle<GameData>>> = std::sync::Mutex::new(None);

/// Start parsing the staging data in the background (once, at startup).
pub fn warm(root: aa_ui::fs::DataRoot) {
    if let Ok(mut g) = WARM.lock() {
        if g.is_none() {
            *g = Some(std::thread::spawn(move || GameData::load(&root)));
        }
    }
}

impl Custom {
    pub fn new(root: &Path) -> Self {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1) | 1;
        Self {
            root: root.to_path_buf(),
            data: None,
            session: None,
            state: StagingState::default(),
            staging: None,
            dialog: None,
            player_name: None,
            seed,
            rmc_prefs: Default::default(),
            launched_nation: None,
            launched_profiles: Vec::new(),
            launched_factions: Vec::new(),
        }
    }

    fn data(&mut self, ui: &Ui) -> &GameData {
        if self.data.is_none() {
            let t = std::time::Instant::now();
            // the staging data warmed at startup ([`warm`]), else loaded now
            let warmed = WARM.lock().ok().and_then(|mut g| g.take()).and_then(|h| h.join().ok());
            self.data = Some(warmed.unwrap_or_else(|| GameData::load(&ui.store.root)));
            let d = self.data.as_ref().unwrap();
            info!(
                "staging data: {} nations, {} SAIs, {} handicaps, {} rules, {} RMCs in {:.2} s",
                d.nations.len(),
                d.sais.len(),
                d.handicaps.len(),
                d.rules.len(),
                d.rmcs.len(),
                t.elapsed().as_secs_f32()
            );
        }
        self.data.as_ref().unwrap()
    }

    fn player_name(&mut self, ui: &Ui) -> String {
        if self.player_name.is_none() {
            // the profile defaults 0x41E160: Format(default_lan_name)
            self.player_name = Some(ui.strings.resolve("#default_lan_name"));
        }
        self.player_name.clone().unwrap()
    }

    fn next_random(&mut self) -> u64 {
        self.seed ^= self.seed >> 12;
        self.seed ^= self.seed << 25;
        self.seed ^= self.seed >> 27;
        self.seed.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// The session records, teams and default rules of an RMC.
    fn rmc_session(&mut self, ui: &Ui, rmc_ids: &str, size: Option<(f32, f32)>, terrain: Option<Vec<f32>>, rules: Option<Vec<RuleValue>>) -> Option<Session> {
        let data = self.data(ui);
        let rmc = data.rmcs.iter().find(|r| r.ids.eq_ignore_ascii_case(rmc_ids)).or(data.rmcs.first()).cloned()?;
        let size = size.or_else(|| rmc.sizes.first().map(|s| (s.0, s.1))).unwrap_or((384.0, 384.0));
        let terrain = terrain.unwrap_or_else(|| rmc.terrains.iter().map(|t| t.range.0 + 0.5 * (t.range.1 - t.range.0)).collect());
        let rules = rules.unwrap_or_else(|| {
            let mut r = dialogs::default_rules(data);
            for (ids, default, _) in &rmc.rules {
                if let Some(v) = r.iter_mut().find(|v| v.ids.eq_ignore_ascii_case(ids)) {
                    v.value = default.clone();
                }
            }
            r
        });
        let map_teams: Vec<(String, String)> = rmc.teams.iter().map(|(i, n)| (i.clone(), ui.strings.resolve(n))).collect();
        let records: Vec<Record> = rmc
            .kingdoms
            .iter()
            .map(|k| Record {
                ids: k.ids.clone(),
                side_name: ui.strings.resolve(&k.name),
                color: data.color(&k.color),
                independent: k.independent,
                playable: k.playable,
                required: k.required,
                map_team: k.team.as_ref().and_then(|t| map_teams.iter().position(|(i, _)| i.eq_ignore_ascii_case(t))),
                fixed_nation: None,
                fixed_faction: None,
                team: None,
                open: false,
                occ: Occ::Open,
                nation: None,
                faction: None,
                handicap: None,
            })
            .collect();
        let has_ffa = records.iter().any(|r| r.playable && !r.independent && r.map_team.is_none());
        Some(Session { source: Source::Random { rmc: rmc.ids.clone(), size, terrain }, rules, records, map_teams, has_ffa, format: String::new() })
    }

    /// A scenario map's records and teams (session 0x5144A0 from the
    /// header).
    fn map_session(&mut self, ui: &Ui, m: &MapInfo, rules: Vec<RuleValue>) -> Session {
        let data = self.data(ui);
        let map_teams = m.teams.clone();
        let records: Vec<Record> = m
            .kingdoms
            .iter()
            .map(|k| {
                let nation = k.nation.as_ref().and_then(|n| data.nation(n)).map(|n| n.ids.clone());
                Record {
                    ids: k.ids.clone(),
                    side_name: k.name.clone(),
                    color: data.color(&k.color),
                    independent: k.independent,
                    playable: k.playable,
                    required: k.required,
                    map_team: map_teams.iter().position(|(id, _)| !k.team.is_empty() && id.eq_ignore_ascii_case(&k.team)),
                    fixed_nation: nation,
                    fixed_faction: k.faction.clone(),
                    team: None,
                    open: false,
                    occ: Occ::Open,
                    nation: None,
                    faction: None,
                    handicap: None,
                }
            })
            .collect();
        let has_ffa = records.iter().any(|r| r.playable && !r.independent && r.map_team.is_none());
        Session { source: Source::Map(m.clone()), rules, records, map_teams, has_ffa, format: String::new() }
    }

    /// 0x781DD0 + 0x7819A0 + 0x7791F0: a new map keeps the previous format
    /// when it is valid, else the largest balanced one (0x77E160), else the
    /// map's own; then the people sit down again and AIs fill the seats.
    fn reset_teams(&mut self, ui: &Ui, mut s: Session, previous: &str, people: Vec<Occ>) -> Session {
        let mut chosen = (!previous.is_empty() && s.valid(previous)).then(|| previous.to_string());
        if chosen.is_none() {
            let list = format_list(ui, &s);
            let mut best: Option<(String, usize)> = None;
            for (id, _) in &list {
                let total = s.parse(id).map(|(t, f)| t.iter().sum::<usize>() + f).unwrap_or(0);
                match &best {
                    Some((_, b)) if total < *b => break,
                    Some((_, b)) if total <= *b => {}
                    _ => best = Some((id.clone(), total)),
                }
            }
            chosen = best.map(|b| b.0);
        }
        // the map's own format: every map team with its kingdoms, the FFA
        // kingdoms as seats
        let own = {
            let mut t = s.clone();
            let ffa = t.map_teams.len();
            let has_ffa = t.has_ffa;
            for r in &mut t.records {
                r.team = if has_ffa && r.map_team.is_none() && r.playable && !r.independent { Some(ffa) } else { r.map_team };
                r.open = r.playable && !r.independent;
            }
            t.canonical()
        };
        let format = chosen.filter(|f| s.valid(f)).unwrap_or(own);
        if !s.apply(&format) {
            warn!("staging: team format {format} does not apply");
        }
        // the people already in the session sit down again
        for (i, occ) in people.into_iter().enumerate() {
            let seat = (0..s.records.len()).filter(|&j| s.records[j].team.is_some() && s.records[j].open && s.eligible(j)).nth(i);
            if let Some(j) = seat {
                s.records[j].occ = occ;
            }
        }
        s.fill_and_seat();
        s
    }

    /// `RequestCustom`: a fresh single-player session on the default RMC.
    pub fn enter(&mut self, ui: &mut Ui) {
        let rmc_ids = ui.avars.get("StagingSinglePlayerDefaultRMCIds").map(aa_ui::node::unquote).unwrap_or_default();
        let w = ui.avars.float("StagingSinglePlayerDefaultRMCWidth").unwrap_or(384.0);
        let h = ui.avars.float("StagingSinglePlayerDefaultRMCHeight").unwrap_or(384.0);
        let config = ui.avars.get("StagingSinglePlayerDefaultRMCTeamConfig").map(aa_ui::node::unquote).filter(|s| !s.is_empty()).unwrap_or_else(|| "4v4".into());
        // the size only when the RMC has it
        let size = {
            let data = self.data(ui);
            data.rmcs.iter().find(|r| r.ids.eq_ignore_ascii_case(&rmc_ids)).and_then(|r| r.sizes.iter().find(|s| s.0 == w && s.1 == h)).map(|s| (s.0, s.1))
        };
        let Some(s) = self.rmc_session(ui, &rmc_ids, size, None, None) else {
            warn!("staging: no RMC data");
            self.session = None;
            return;
        };
        let s = self.reset_teams(ui, s, &config, vec![Occ::Human]);
        self.session = Some(s);
        self.launched_nation = None;
    }

    /// The staging screen's view of the session.
    fn build_state(&mut self, ui: &Ui) -> StagingState {
        let player = self.player_name(ui);
        let Some(s) = self.session.clone() else {
            return StagingState { networked: false, local_is_host: true, ..Default::default() };
        };
        let formats = format_list(ui, &s);
        let data = self.data(ui);
        let nations: Vec<NationInfo> = data
            .nations
            .iter()
            .map(|n| NationInfo {
                id: n.ids.clone(),
                text: ui.strings.resolve(&n.name),
                playable: n.playable,
                flag_icon: n.flag_icon.clone(),
                factions: n.factions.iter().map(|f| FactionInfo { id: f.ids.clone(), text: ui.strings.resolve(&f.name), selectable: f.selectable }).collect(),
            })
            .collect();
        let sais: Vec<(String, String, bool)> = data.sais.iter().map(|x| (x.ids.clone(), ui.strings.resolve(&x.name), x.selectable)).collect();
        let handicaps: Vec<(String, String)> = data.handicaps.iter().map(|(i, n)| (i.clone(), ui.strings.resolve(n))).collect();
        let random = ui.strings.resolve("#staging_and_session_random_sai_name");
        let mut teams: Vec<TeamInfo> = Vec::new();
        let team_names: Vec<Option<String>> = s.map_teams.iter().map(|(_, n)| Some(n.clone())).chain(std::iter::once(None)).collect();
        for (t, name) in team_names.into_iter().enumerate() {
            let members: Vec<&Record> = s.records.iter().filter(|r| r.team == Some(t)).collect();
            if members.is_empty() {
                continue;
            }
            let mut kingdoms = Vec::new();
            for r in members {
                let occupant = match &r.occ {
                    Occ::Open => Occupant::None,
                    Occ::Human => Occupant::Human { name: player.clone(), local: true, on_host: true, ready: false, ping_ms: 0, download: None, swap_from: false, swap_to: false },
                    Occ::Ai(sai) => Occupant::Ai {
                        name: sai.as_ref().and_then(|id| sais.iter().find(|x| &x.0 == id)).map(|x| x.1.clone()).unwrap_or_else(|| random.clone()),
                        client: String::new(),
                        sai: sai.clone(),
                        local: true,
                    },
                };
                kingdoms.push(KingdomSlot {
                    closed: !r.open,
                    required: r.required,
                    color: r.color,
                    record_name: r.side_name.clone(),
                    nation: r.nation.clone(),
                    faction: r.faction.clone(),
                    handicap: r.handicap.clone(),
                    fixed_nation: r.fixed_nation.clone(),
                    fixed_faction: r.fixed_faction.clone(),
                    occupant,
                });
            }
            teams.push(TeamInfo { name, kingdoms });
        }
        let (launch_type, map_name, map_size) = match &s.source {
            Source::Random { rmc, size, .. } => {
                let name = data.rmcs.iter().find(|r| &r.ids == rmc).map(|r| ui.strings.resolve(&r.name)).unwrap_or_default();
                (LaunchType::RandomMap, name, (size.0 as i32, size.1 as i32))
            }
            Source::Map(m) => (LaunchType::LoadMap, m.name.clone(), m.size),
        };
        let humans = s.records.iter().filter(|r| r.occ == Occ::Human).count() as i32;
        let can_join = vec![false; teams.len()];
        StagingState {
            networked: false,
            local_is_host: true,
            online: false,
            launch_type,
            locked: false,
            countdown: false,
            game_name: String::new(),
            password: String::new(),
            players: (humans, s.seats() as i32),
            team_formats: formats,
            team_format: Some(s.format.clone()),
            scoring_available: false,
            observers_allowed: true,
            map_name,
            map_size,
            rules: String::new(),
            local_has_kingdom: humans > 0,
            local_ready: false,
            teams,
            observers: Vec::new(),
            nations,
            handicaps,
            sais,
            can_join_team: can_join,
        }
    }

    /// (Re)build the staging menu for the current screen size.
    pub fn show(&mut self, ui: &mut Ui) {
        if self.session.is_none() {
            self.enter(ui);
        }
        self.state = self.build_state(ui);
        self.staging = Some(Staging::show(ui, self.state.clone()));
        self.dialog = None;
        self.after_update(ui);
    }

    fn refresh(&mut self, ui: &mut Ui) {
        self.state = self.build_state(ui);
        if let Some(st) = &mut self.staging {
            st.set_state(ui, self.state.clone());
        }
        self.after_update(ui);
    }

    /// What the staging update leaves for the session to write: the map
    /// preview of a random map (expanded-map-options.png: the title,
    /// "WxH,", "R Recommended.", the sides), the empty chat log's scroll bar.
    fn after_update(&mut self, ui: &mut Ui) {
        if let Some(Session { source: Source::Random { rmc, size, .. }, records, .. }) = self.session.clone() {
            let data = self.data(ui);
            if let Some(r) = data.rmcs.iter().find(|r| r.ids == rmc).cloned() {
                let mut lines = vec![ui.strings.resolve(&r.name)];
                lines.push(format!("{},", ui.strings.format("staging_map_preview_map_size_format_string|dd", &[&(size.0 as i32).to_string(), &(size.1 as i32).to_string()])));
                if let Some(s) = r.sizes.iter().find(|s| s.0 == size.0 && s.1 == size.1).filter(|s| s.2 >= 0) {
                    let range = if s.2 == s.3 { s.2.to_string() } else { ui.strings.format("staging_map_preview_recommended_range|dd", &[&s.2.to_string(), &s.3.to_string()]) };
                    lines.push(format!("{}.", ui.strings.format("staging_map_preview_recommended_mini|s", &[&range])));
                }
                lines.push(String::new());
                let gem = ui.strings.glyphs.get(&ui.avars.get("KingdomGlyphInfo").unwrap_or("ch_gem").to_ascii_lowercase()).copied().unwrap_or('#');
                let g = |v: f32| aa_ui::staging::fmt_g(v as f64);
                // 0x65F880: not independent, playable, open
                for k in records.iter().filter(|k| k.playable && !k.independent && k.team.is_some() && k.open) {
                    lines.push(format!("<color={},{},{}>{} <rc>{}", g(k.color[0]), g(k.color[1]), g(k.color[2]), gem, k.side_name));
                }
                let lp = "Root/StagingMenu/MapPreviewPanel/MapPreviewViewport/ItemPanel/MapPreviewLabel";
                ui.set_label(&lp["Root/".len()..], &lines.join("\n"));
                aa_ui::staging::label_fit_height(ui, lp);
                ui.viewport_refresh("Root/StagingMenu/MapPreviewPanel/MapPreviewViewport");
            }
        }
        // the chat log of a single-player session stays empty: its scroll
        // bar is not shown (02-custom.png; INFERRED from the viewport rule
        // 0x5ED560, the message log's own scroll update is not ported)
        ui.with_widget("StagingMenu/StagingChatPanel/MessageLogPanel/MessageLog/Scrollbar", |w| w.set_visible(false));
        ui.layout();
    }

    fn people(&self) -> Vec<Occ> {
        let Some(s) = &self.session else { return vec![Occ::Human] };
        let mut v: Vec<Occ> = s.records.iter().filter(|r| r.occ != Occ::Open).map(|r| r.occ.clone()).collect();
        v.sort_by_key(|o| *o != Occ::Human);
        v
    }

    pub fn command(&mut self, ui: &mut Ui, cmd: &str, widget: &str) -> Option<Action> {
        // an open dialog takes its own commands first
        if let Some(d) = self.dialog.take() {
            self.data(ui);
            let data = self.data.as_ref().unwrap();
            let (d, out) = match d {
                Dialog::File(mut f) => {
                    let o = f.command(ui, data, cmd);
                    (Dialog::File(f), o)
                }
                Dialog::Random(mut r) => {
                    let o = r.command(ui, data, cmd);
                    (Dialog::Random(r), o)
                }
            };
            match out {
                Some(Outcome::Open) => {
                    self.dialog = Some(d);
                    return Some(Action::None);
                }
                Some(Outcome::Cancelled) => return Some(Action::None),
                Some(Outcome::Map { map, rules }) => {
                    let previous = self.session.as_ref().map(|s| s.format.clone()).unwrap_or_default();
                    let people = self.people();
                    let s = self.map_session(ui, &map, rules);
                    let s = self.reset_teams(ui, s, &previous, people);
                    self.session = Some(s);
                    self.refresh(ui);
                    return Some(Action::None);
                }
                Some(Outcome::Random { rmc, size, terrain, rules, prefs }) => {
                    self.rmc_prefs = prefs;
                    let previous = self.session.as_ref().map(|s| s.format.clone()).unwrap_or_default();
                    let people = self.people();
                    if let Some(s) = self.rmc_session(ui, &rmc, Some(size), Some(terrain), Some(rules)) {
                        let s = self.reset_teams(ui, s, &previous, people);
                        self.session = Some(s);
                    }
                    self.refresh(ui);
                    return Some(Action::None);
                }
                None => self.dialog = Some(d),
            }
        }
        let current_map = match self.session.as_ref().map(|s| &s.source) {
            Some(Source::Map(m)) => Some(m.path.clone()),
            _ => None,
        };
        match cmd {
            "SelectLaunchType RANDOM_MAP" => {
                let cur = match self.session.as_ref().map(|s| &s.source) {
                    Some(Source::Random { rmc, .. }) => Some(rmc.clone()),
                    _ => None,
                };
                let rules = self.session.as_ref().map(|s| s.rules.clone()).unwrap_or_default();
                self.data(ui);
                let data = self.data.as_ref().unwrap();
                let d = RandomDialog::open(ui, data, cur.as_deref(), self.rmc_prefs.clone(), rules);
                self.dialog = Some(Dialog::Random(d));
                Some(Action::None)
            }
            "SelectLaunchType LOAD_MAP" => {
                let rules = self.session.as_ref().map(|s| s.rules.clone()).unwrap_or_default();
                self.data(ui);
                let data = self.data.as_ref().unwrap();
                let d = FileDialog::open(ui, data, current_map.as_deref(), rules);
                self.dialog = Some(Dialog::File(d));
                Some(Action::None)
            }
            c if c.starts_with("SelectLaunchType") => {
                info!("staging: {c}: the saved game / film dialog is not built yet");
                Some(Action::None)
            }
            // the exit confirmation (ui_staging_confirm_exit)
            "StagingConfirmExit" => {
                let text = ui.strings.resolve("#ui_staging_confirm_exit");
                let ok = ui.strings.resolve("#button_ok");
                let cancel = ui.strings.resolve("#button_cancel");
                ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(ok, "StagingExit".into()), (cancel, String::new())] });
                Some(Action::None)
            }
            "StagingExit" => {
                self.session = None;
                Some(Action::Exit)
            }
            "StagingLaunch" => {
                // 0x65DE80: a random map with more sides than the size
                // recommends asks first
                if let Some(Session { source: Source::Random { rmc, size, .. }, records, .. }) = &self.session {
                    let (rmc, size) = (rmc.clone(), *size);
                    let count = records.iter().filter(|r| r.playable && !r.independent && (r.required || r.occ != Occ::Open)).count() as i32;
                    let data = self.data(ui);
                    let max = data.rmcs.iter().find(|r| r.ids == rmc).and_then(|r| r.sizes.iter().find(|s| s.0 == size.0 && s.1 == size.1)).map(|s| s.3).unwrap_or(-1);
                    if max >= 0 && count > max {
                        let text = ui.strings.resolve("#ui_staging_recommended_num_players_message_box");
                        let go = ui.strings.resolve("#ui_staging_proceed_anyway_text");
                        let cancel = ui.strings.resolve("#button_cancel");
                        ui.show_message_box(aa_ui::message_box::SHARED, aa_ui::message_box::MessageBoxRequest { text, buttons: vec![(go, "StagingReallyLaunch".into()), (cancel, String::new())] });
                        return Some(Action::None);
                    }
                }
                Some(Action::Launch)
            }
            "StagingReallyLaunch" => Some(Action::Launch),
            c if c.starts_with("StagingSelectSAI") => {
                let arg = c.split_once(' ').map(|(_, a)| a.trim().to_string()).unwrap_or_default();
                self.select_sai(ui, &arg, widget);
                Some(Action::None)
            }
            c if c.starts_with("TeamFormatDropdown") => {
                let id = c.split_once(' ').map(|(_, a)| a.trim().to_string()).unwrap_or_default();
                if id == "team_format_custom" {
                    info!("staging: the team management dialog (TeamManagementDialog) is not built yet");
                } else if let Some(mut s) = self.session.clone() {
                    // 0x779D80: apply, then 0x7791F0
                    if s.apply(&id) {
                        s.fill_and_seat();
                        self.session = Some(s);
                    }
                }
                self.refresh(ui);
                Some(Action::None)
            }
            _ => None,
        }
    }

    /// Which record the PlayerSlot holding `path` shows: the slots follow
    /// the staging fill order (teams, then their kingdoms; 0x69F950).
    fn slot_record(&self, ui: &Ui, path: &str) -> Option<usize> {
        let key = "/Slots/ItemPanel/";
        let at = path.find(key)? + key.len();
        let seg = path[at..].split('/').next()?;
        let slots = ui.widget("Root/StagingMenu/WorldParamsPanel/Slots/ItemPanel")?;
        let order = self.record_order();
        let mut seen = 0usize;
        for (c, s) in slots.children.iter().zip(aa_ui::ui::child_segments(slots)) {
            if c.name == "PlayerSlot" {
                if s == seg {
                    return order.get(seen).copied();
                }
                seen += 1;
            }
        }
        None
    }

    /// Record indices in slot order (team by team).
    fn record_order(&self) -> Vec<usize> {
        let Some(s) = &self.session else { return Vec::new() };
        let mut out = Vec::new();
        for t in 0..=s.map_teams.len() {
            for (i, r) in s.records.iter().enumerate() {
                if r.team == Some(t) {
                    out.push(i);
                }
            }
        }
        out
    }

    /// `StagingSelectSAI %s` (0x69CDC0) on the slot whose AddAI drop-down
    /// fired: `_open`, `_random` or an SAI (which sets nation and faction).
    fn select_sai(&mut self, ui: &mut Ui, arg: &str, widget: &str) {
        let Some(k) = self.slot_record(ui, widget) else { return };
        let sai = self.data(ui).sais.iter().find(|s| s.ids == arg).cloned();
        let nation_of = |n: &str| self.data.as_ref().and_then(|d| d.nation(n)).map(|n| n.ids.clone());
        let Some(s) = self.session.as_mut() else { return };
        let Some(r) = s.records.get_mut(k) else { return };
        match arg {
            "_open" => r.occ = Occ::Open,
            "_random" => {
                r.occ = Occ::Ai(None);
                r.nation = r.fixed_nation.clone();
                r.faction = r.fixed_faction.clone();
            }
            id => {
                r.occ = Occ::Ai(Some(id.to_string()));
                if let Some(sai) = sai {
                    if r.fixed_nation.is_none() {
                        r.nation = sai.nation.as_deref().and_then(nation_of);
                    }
                    if r.fixed_faction.is_none() {
                        r.faction = sai.faction.clone();
                    }
                }
            }
        }
        self.refresh(ui);
    }

    /// The Nation / Faction / Handicap functors (0x69E080 / 0x69C1E0 /
    /// 0x69C2F0).
    pub fn select(&mut self, ui: &mut Ui, widget: &str, _index: usize, param: &str) {
        let which = widget.rsplit('/').find(|s| matches!(*s, "Nation" | "Faction" | "Handicap")).unwrap_or("");
        let Some(k) = self.slot_record(ui, widget) else { return };
        let Some(s) = self.session.as_mut() else { return };
        let Some(r) = s.records.get_mut(k) else { return };
        let v = (!param.starts_with('_')).then(|| param.to_string());
        match which {
            "Nation" => {
                if r.nation != v {
                    r.nation = v;
                    r.faction = None;
                }
            }
            "Faction" => r.faction = v,
            "Handicap" => r.handicap = v,
            _ => return,
        }
        self.refresh(ui);
    }

    pub fn event(&mut self, ui: &mut Ui, e: &UiEvent) {
        if let Some(s) = &mut self.staging {
            s.event(ui, e);
        }
    }

    pub fn update(&mut self, _ui: &mut Ui) {}

    /// The battle the loader builds (and the random map parameters). The
    /// computer generals' names stay localization keys until
    /// [`Custom::localize_setup`].
    pub fn launch_setup(&mut self) -> Option<(SkirmishSetup, Option<RandomMapParams>)> {
        let s = self.session.clone()?;
        let player = self.player_name.clone().unwrap_or_else(|| "Player".into());
        let data = self.data.take()?;
        let mut players = Vec::new();
        let playable: Vec<&super::gamedata::Nation> = data.nations.iter().filter(|n| n.playable).collect();
        let mut start = 0usize;
        let mut local_nation = None;
        let mut profiles = Vec::new();
        let mut factions = Vec::new();
        for r in s.records.iter() {
            if !(r.playable && !r.independent) {
                continue;
            }
            let index = start;
            start += 1;
            let controller = match &r.occ {
                Occ::Open => continue,
                Occ::Human => Controller::Human,
                // handicap order easy, medium, hard; none = the middle
                Occ::Ai(_) => Controller::Computer(r.handicap.as_ref().and_then(|h| data.handicaps.iter().position(|(i, _)| i == h)).unwrap_or(data.handicaps.len() / 2) as u8),
            };
            // "Random" nation: uniform among the selectable ones (0x50E8B0)
            let nation = match r.nation.as_ref().and_then(|n| data.nation(n)) {
                Some(n) => Some(n),
                None if !playable.is_empty() => {
                    let i = (self.next_random() % playable.len() as u64) as usize;
                    Some(playable[i])
                }
                None => None,
            };
            let nation_short = nation.map(|n| n.short.clone()).unwrap_or_else(|| "usa".into());
            // a random computer general: an SAI profile for the nation
            // (PickRandomProfile 0x79FAF0); its name is the player name
            let name = match &r.occ {
                Occ::Human => {
                    // the loading screen reads the staging record's nation
                    // (0x6E83A0 -> 0x50DC00 +0x18): none for "Random"
                    local_nation = r.nation.as_ref().and_then(|n| data.nation(n)).map(|n| n.short.clone());
                    factions.push((players.len(), r.faction.clone()));
                    player.clone()
                }
                _ => {
                    let wanted = nation.map(|n| n.ids.clone());
                    let chosen = match &r.occ {
                        Occ::Ai(Some(id)) => Some(id.clone()),
                        _ => {
                            let cands: Vec<String> = data.sais.iter().filter(|x| x.selectable && x.nation == wanted).map(|x| x.ids.clone()).collect();
                            (!cands.is_empty()).then(|| cands[(self.next_random() % cands.len() as u64) as usize].clone())
                        }
                    };
                    if let Some(id) = &chosen {
                        profiles.push((players.len(), id.clone()));
                    }
                    let sai_faction = chosen.as_ref().and_then(|id| data.sais.iter().find(|x| &x.ids == id)).and_then(|x| x.faction.clone());
                    factions.push((players.len(), r.faction.clone().or(sai_faction)));
                    chosen.and_then(|id| data.sais.iter().find(|x| x.ids == id)).map(|x| x.name.clone()).unwrap_or_else(|| r.side_name.clone())
                }
            };
            let c = r.color;
            let team = match r.team {
                // the no-team team: every kingdom on its own side
                Some(t) if t == s.map_teams.len() => (s.map_teams.len() + index) as u32,
                Some(t) => t as u32,
                None => (s.map_teams.len() + index) as u32,
            };
            players.push(PlayerSetup {
                name,
                nation: nation_short,
                team,
                color: [(c[0] * 255.0).round() as u8, (c[1] * 255.0).round() as u8, (c[2] * 255.0).round() as u8],
                controller,
                start: index,
            });
        }
        let money = s.rules.iter().find(|r| r.ids.eq_ignore_ascii_case("ResourceStockpileDefault")).and_then(|r| r.value.parse::<f32>().ok());
        let default_money = data.rules.iter().find(|r| r.ids.eq_ignore_ascii_case("ResourceStockpileDefault")).and_then(|r| r.default.parse::<f32>().ok());
        self.data = Some(data);
        let starting_resources = money.filter(|m| Some(*m) != default_money);
        let game_seed = self.next_random();
        let (map_id, random) = match &s.source {
            Source::Map(m) => (m.title.clone(), None),
            Source::Random { rmc, size, terrain } => {
                let params = RandomMapParams {
                    rmc: rmc.clone(),
                    width: size.0,
                    height: size.1,
                    sides: players.len() as u32,
                    teams: players.iter().map(|p| p.team).collect(),
                    terrain_weights: terrain.clone(),
                    object_counts: Default::default(),
                    heroes: 0,
                    seed: game_seed,
                };
                (rmc.clone(), Some(params))
            }
        };
        self.launched_nation = local_nation;
        self.launched_profiles = profiles;
        self.launched_factions = factions;
        Some((SkirmishSetup { map_id, players, starting_resources, seed: game_seed }, random))
    }

    /// The display names of the session's independent kingdoms (kingdom
    /// IDS lower case -> the record's name: RMC "Enemies", a map's
    /// "Civilian (Aggressive)").
    pub fn kingdom_names(&self) -> std::collections::HashMap<String, String> {
        self.session
            .as_ref()
            .map(|s| s.records.iter().filter(|r| r.independent && !r.side_name.is_empty()).map(|r| (r.ids.to_ascii_lowercase(), r.side_name.clone())).collect())
            .unwrap_or_default()
    }

    /// The value of a world rule (IDS) of the session being launched.
    pub fn rule_value(&self, ids: &str) -> Option<String> {
        self.session.as_ref()?.rules.iter().find(|r| r.ids.eq_ignore_ascii_case(ids)).map(|r| r.value.clone())
    }

    /// Localize the computer generals' names chosen for a launch.
    pub fn localize_setup(&self, ui: &Ui, setup: &mut SkirmishSetup) {
        for p in &mut setup.players {
            if p.name.starts_with('#') {
                p.name = ui.strings.resolve(&p.name);
            }
        }
    }

    /// Map name, description and the local player's nation for the loading
    /// screen.
    pub fn loading_info(&self) -> (String, String, Option<String>) {
        let Some(s) = &self.session else { return (String::new(), String::new(), None) };
        let nation = self.launched_nation.clone();
        match &s.source {
            Source::Map(m) => (m.name.clone(), m.description.clone(), nation),
            // the generated world's description: the RMC's (a '#' key)
            Source::Random { rmc, .. } => {
                let desc = self.data.as_ref().and_then(|d| d.rmcs.iter().find(|r| r.ids.eq_ignore_ascii_case(rmc))).map(|r| r.description.clone()).unwrap_or_default();
                (self.state.map_name.clone(), desc, nation)
            }
        }
    }

    pub fn load_failed(&mut self, _ui: &mut Ui, err: &str) {
        warn!("battle load failed: {err}");
    }

    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        &self.root
    }
}
