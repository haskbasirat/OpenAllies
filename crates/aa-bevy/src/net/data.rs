//! The game data a LAN session reads ([`aa_netcode::lan::LanData`]) and the
//! kingdom records a session stages (the creator records, stride 0x6C), both
//! from what the staging screen has parsed (`ui/gamedata.rs`).
use crate::ui::gamedata::{GameData, MapInfo, Rmc};
use aa_netcode::lan::registries::{self, NativeRegistries};
use aa_netcode::lan::LanData;
use aa_netcode::lobby::{self, KingdomSetup, MapHeader, Session};
use aa_runtime::simulation::SpeedSettings;
use aa_simulation::FloatBits;
use aa_ui::resources::StringTable;
use std::collections::HashMap;
use std::sync::Arc;

pub fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
pub fn text(u: &[u16]) -> String {
    let end = u.iter().position(|&c| c == 0).unwrap_or(u.len());
    String::from_utf16_lossy(&u[..end])
}

/// A record of the map or RMC as a session stages it.
#[derive(Clone, Debug)]
pub struct RecordInfo {
    pub ids: String,
    pub independent: bool,
    pub playable: bool,
    pub required: bool,
    pub team: Option<String>,
    pub nation: Option<String>,
    pub faction: Option<String>,
    pub color: String,
}

/// What a session reads of the staging's parsed data.
pub struct GameParts {
    rmcs: HashMap<String, Vec<RecordInfo>>,
    rmc_teams: HashMap<String, usize>,
    factions: HashMap<String, Vec<String>>,
}
impl GameParts {
    pub fn of(game: &GameData) -> Self {
        Self {
            rmcs: game.rmcs.iter().map(|r| (r.ids.to_ascii_lowercase(), rmc_records(r))).collect(),
            rmc_teams: game.rmcs.iter().map(|r| (r.ids.to_ascii_lowercase(), r.teams.len())).collect(),
            factions: game.nations.iter().map(|n| (n.ids.clone(), n.factions.iter().map(|f| f.ids.clone()).collect())).collect(),
        }
    }
}

/// What the session's data view holds, cheap to clone into a node.
pub struct Data {
    pub registries: NativeRegistries,
    pub strings: StringTable,
    pub rmcs: HashMap<String, Vec<RecordInfo>>,
    pub rmc_teams: HashMap<String, usize>,
    pub maps: HashMap<String, (Vec<RecordInfo>, usize)>,
    /// Nation IDS -> Faction IDS.
    pub factions: HashMap<String, Vec<String>>,
    pub countdown: i32,
    pub speed: SpeedSettings,
}

fn lower(u: &[u16]) -> String {
    text(u).to_ascii_lowercase()
}

/// The records of a map, as the creator 0x5144A0 takes them from its header.
pub fn map_records(m: &MapInfo) -> Vec<RecordInfo> {
    m.kingdoms
        .iter()
        .map(|k| RecordInfo {
            ids: k.ids.clone(),
            independent: k.independent,
            playable: k.playable,
            required: k.required,
            team: (!k.team.is_empty()).then(|| k.team.clone()),
            nation: k.nation.clone(),
            faction: k.faction.clone(),
            color: k.color.clone(),
        })
        .collect()
}

/// The records of an RMC (its kingdoms in file order).
pub fn rmc_records(r: &Rmc) -> Vec<RecordInfo> {
    r.kingdoms
        .iter()
        .map(|k| RecordInfo {
            ids: k.ids.clone(),
            independent: k.independent,
            playable: k.playable,
            required: k.required,
            team: k.team.clone(),
            nation: None,
            faction: None,
            color: k.color.clone(),
        })
        .collect()
}

impl Data {
    pub fn new(reg: NativeRegistries, game: GameParts, maps: &[MapInfo], strings: StringTable, countdown: i32, rule: impl Fn(&str, f32) -> f32) -> Arc<Self> {
        let speed = SpeedSettings {
            game_speed_min: FloatBits::from_f32(rule("GameSpeedMin", -2.0)),
            game_speed_max: FloatBits::from_f32(rule("GameSpeedMax", 2.0)),
            film_min: FloatBits::from_f32(rule("GameSpeedFilmMin", -4.0)),
            film_max: FloatBits::from_f32(rule("GameSpeedFilmMax", 4.0)),
            admin_ticks_per_second: FloatBits::from_f32(rule("AdminTicksPerSecond", 16.0)),
        };
        Arc::new(Self {
            registries: reg,
            strings,
            rmcs: game.rmcs,
            rmc_teams: game.rmc_teams,
            maps: maps.iter().map(|m| (m.title.to_ascii_lowercase(), (map_records(m), m.teams.len()))).collect(),
            factions: game.factions,
            countdown,
            speed,
        })
    }

    /// The registry index of an IDS, case-insensitive.
    fn index(&self, registry: aa_orders::payload::Registry, ids: &str) -> Option<u32> {
        self.registries.slots.get(&registry.0)?.iter().position(|s| s.eq_ignore_ascii_case(ids)).map(|i| i as u32)
    }

    /// The creator's records: name, +0x10 independent, +0x28 playable, +0x29
    /// required, and the nation / faction / color definitions of the record.
    pub fn header(&self, records: &[RecordInfo]) -> MapHeader {
        let kingdoms = records
            .iter()
            .map(|r| KingdomSetup {
                name: units(&r.ids),
                flag_10: r.independent,
                flag_28: r.playable,
                flag_29: r.required,
                nation: r.nation.as_deref().and_then(|n| self.nation_index(n)),
                faction: r.faction.as_deref().and_then(|f| self.index(registries::FACTIONS, f)),
                color: self.index(registries::KINGDOM_COLORS, &r.color),
                ..Default::default()
            })
            .collect();
        MapHeader { names: Vec::new(), kingdoms }
    }

    /// A map's nation is its IDS or the short form ("usa" -> "nation_usa").
    fn nation_index(&self, nation: &str) -> Option<u32> {
        self.index(registries::NATIONS, nation).or_else(|| self.index(registries::NATIONS, &format!("nation_{nation}")))
    }

    /// The records of what `session` stages.
    pub fn session_records(&self, session: &Session) -> Option<(&[RecordInfo], usize)> {
        let (records, teams) = match session.launch.mode {
            0 => {
                let key = lower(session.launch.random_map.as_deref()?);
                (self.rmcs.get(&key)?, *self.rmc_teams.get(&key)?)
            }
            _ => {
                let (r, t) = self.maps.get(&lower(&session.launch.file))?;
                (r, *t)
            }
        };
        Some((records.as_slice(), teams))
    }
}

/// `format` is the team format of the session (`4v4`, `0v0v1v1`): numbers per
/// map team then, for the no-team team, one seat per kingdom. Which records it
/// seats: 0x781250 (the same rule as `Custom`'s `apply`).
pub fn format_open(records: &[RecordInfo], map_teams: usize, format: &str) -> Vec<bool> {
    let mut open = vec![false; records.len()];
    let eligible = |r: &RecordInfo| r.playable && !r.independent;
    let parts: Vec<usize> = format.split('v').map(|t| t.parse().unwrap_or(0)).collect();
    let team_ids: Vec<Option<String>> = {
        let mut ids: Vec<Option<String>> = Vec::new();
        for r in records.iter().filter(|r| eligible(r)) {
            if r.team.is_some() && !ids.iter().flatten().any(|i| Some(i) == r.team.as_ref()) {
                ids.push(r.team.clone());
            }
        }
        ids
    };
    let mut pool: Vec<usize> = (0..records.len()).filter(|&i| eligible(&records[i]) && records[i].team.is_none() && !records[i].required).collect();
    for (t, want) in parts.iter().enumerate() {
        if t < map_teams {
            let id = team_ids.get(t).cloned().flatten();
            let mut left = *want;
            for i in 0..records.len() {
                if left > 0 && eligible(&records[i]) && records[i].team == id && id.is_some() {
                    open[i] = true;
                    left -= 1;
                }
            }
            while left > 0 && !pool.is_empty() {
                open[pool.remove(0)] = true;
                left -= 1;
            }
        } else if *want > 0 {
            for i in 0..records.len() {
                if eligible(&records[i]) && records[i].team.is_none() && records[i].required {
                    open[i] = true;
                }
            }
            if !pool.is_empty() {
                open[pool.remove(0)] = true;
            }
        }
    }
    open
}

impl LanData for Data {
    fn registries(&self) -> &NativeRegistries {
        &self.registries
    }
    fn random_map_config(&self, key: &[u16]) -> Option<Vec<u16>> {
        self.rmcs.contains_key(&lower(key)).then(|| key.to_vec())
    }
    fn scenario(&self, key: &[u16]) -> Option<Vec<u16>> {
        self.maps.contains_key(&lower(key)).then(|| key.to_vec())
    }
    fn format(&self, key: &str, args: &[Vec<u16>]) -> Vec<u16> {
        let a: Vec<String> = args.iter().map(|x| text(x)).collect();
        let b: Vec<&str> = a.iter().map(String::as_str).collect();
        units(&self.strings.format(key, &b))
    }
    fn countdown_time(&self) -> i32 {
        self.countdown
    }
    fn nation_factions(&self, nation: &[u16]) -> aa_netcode::Result<Vec<Vec<u16>>> {
        let ids = text(nation);
        self.factions
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&ids))
            .map(|(_, v)| v.iter().map(|f| units(f)).collect())
            .ok_or_else(|| aa_netcode::Error::Protocol(format!("no nation {ids}")))
    }
    fn speed_settings(&self) -> SpeedSettings {
        self.speed
    }
    fn kingdom_open(&self, session: &lobby::Session, record: usize) -> Option<bool> {
        let format = text(&session.info_text);
        if format.is_empty() {
            return None;
        }
        let (records, teams) = self.session_records(session)?;
        format_open(records, teams, &format).get(record).copied()
    }
    fn rmc_header(&self, rmc: &[u16]) -> Option<MapHeader> {
        Some(self.header(self.rmcs.get(&lower(rmc))?))
    }
}
