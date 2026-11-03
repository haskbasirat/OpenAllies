//! Recording a film of a running aa-play battle (FILM-PLAYBACK.md section 9,
//! item F11): the decree stream the client's `Command`s stand for, the
//! checkpoints, and the file.
//!
//! Natively every order is a decree. The client issues a request (the SAI's
//! `KKC_SAICommandData::Send` 0x7AF220 too), the next admin tick 0x755290 seals
//! the requests (0x752DA0) and executes them (0x752E60): each runs as a decree,
//! is logged to the sync logs and copied to the mirror stream, and the next
//! world tick 0x752AE0 writes that mirror as one film record (0x74D640) before
//! the update 0x4F9970. A film therefore holds the computer players' orders,
//! which is why it can play with the SAI off (0x7A0080).
//!
//! aa-play runs on `Command`s, so [`Recorder::step`] is that admin and world
//! tick over [`World::step_decrees`]:
//!
//! * every `Command` (`World::sai_orders` first, they were requested during the
//!   last update) becomes its decrees with native ids, through the request forms
//!   of `aa_play::player_orders::native` (the same translation the LAN session
//!   sends): selection decrees, then 86 KKC_TellSelectedCommand or 70
//!   KKC_FactionUseAbility;
//! * the ordering player's 298 decrees come as 0x7509F0 and the request stream
//!   (0x74B360) write them: one for the node's player when a tick has any
//!   request, one whenever the requesting player changes;
//! * every 17th admin tick that the game runs (0x755290, counter > 0x10, and
//!   0x76AEE0) a 311 carries {S last index, S hash} (0x74E670), captured before
//!   the tick's decrees run, as the request is emitted first of all;
//! * the computer players' kingdoms are renamed `<name> AI` by 328 at the first
//!   record they can act in, which is also how the reader tells them apart.
//!
//! A decree runs exactly as a film's does: its S entries, then what [`Mapper`]
//! makes of it, applied at once ([`crate::replay::FilmDriver`] is the same loop),
//! so a film of this stream replays the same world by construction, with the Mind
//! handlers' shaping of an order (a move at a hostile actor is a kill, 0x527E30) on
//! both sides. A command the stream cannot carry still reaches the world and is
//! counted ([`RecordingStats::unmapped`]): that film will go out of sync.
//!
//! The decree stream runs whether or not a file is being written, so the world
//! and S are the same with recording on and off: a test launch (0x7757F0, launch
//! type 7 or 0xE) has a [`Recorder`] with no [`FilmFile`].
//!
//! Selection forms: a human's selection is 18 SelectionClear, 17 SelectionRemove and
//! 16 SelectionAdd and is sent only when it changes (the films' humans); a computer's is 333
//! SelectionSet before every order, 16 for further units, and with more than one
//! unit 86 group_behavior_enable first (0x7AF220). The commands go to the world
//! on the 86, so a replay applies them at the same place in the stream.
//!
//! A person's right click is `Command::Hot`, the default order of 0x620760 with no
//! ActorCommand named: 0x4DA570 guesses on every machine. Their selection comes as
//! `Command::Select` when it changes. The HUD's aimed orders name theirs, as natively.
use crate::decree::{Decree, id};
use crate::film::{FilmDecree, SessionPlayer};
use crate::film_writer::{self, FilmFile, OrdersWriter, WireDecree};
use crate::mapping::{Mapped, Mapper, PlayIds};
use crate::registries::Registries;
use crate::slog::decree_s_entries;
use aa_mapworld::header::{
    DerivedCreator, HeaderEntry, KingdomParameter, MapHeader, RandomCreatorSettings, Rule,
    TeamParameter, WorldCreator, WorldParameters, WorldRules,
};
use aa_orders::payload::{Field, Registry, ThingCheck};
use aa_play::mapgen::{GeneratedMap, RandomMapData, RandomMapParams, WorldKingdom, WorldSpec};
use aa_play::orders::ActorOrder;
use aa_play::player_orders::native::{self as wire, Outbox, Tables};
use aa_play::{Command, Controller, GameData, KingdomId, SkirmishSetup, Unit, World};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// `[0xB27A80]`: the build number of every AA.exe of the project, 4129.7.
pub const BUILD_BITS: u32 = 0x4581_0D9A;
/// The Version check of header 5A14C0's numeric word: two decimal digits per
/// component (0x410D80), `1.0.5.0` is 1000500.
pub fn numeric_version(text: &str) -> u32 {
    text.split('.')
        .fold(0u32, |n, part| n.wrapping_mul(100).wrapping_add(part.parse().unwrap_or(0)))
}

/// What the header constructor 0x5A14C0 copies from the application's globals:
/// AVars `Version` (`[0xB3888C]`), `ApplicationIDS` and `SKU` + " AA".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameVersion {
    pub version: String,
    pub application: String,
    pub release: String,
}
impl GameVersion {
    /// Read from the data the game runs on.
    pub fn load(game_root: &Path) -> anyhow::Result<Self> {
        let mut sources = aa_play::data::Sources::open(game_root)?;
        let mut read = |file: &str, key: &str| -> anyhow::Result<String> {
            let bytes = sources
                .files
                .read(file)
                .map_err(|e| anyhow::anyhow!("{file}: {e}"))?
                .bytes;
            avar(&bytes, key).ok_or_else(|| anyhow::anyhow!("{file}: no {key}"))
        };
        let version = read("AVars_version.tgi", "Version")?;
        let application = read("AVars_AA.tgi", "ApplicationIDS")?;
        let sku = read("AVars_Locale.tgi", "SKU")?;
        Ok(Self {
            version,
            application,
            release: format!("{sku} AA"),
        })
    }
}

/// `string <key> = <value>` of an AVars file (`;;` comments skipped).
fn avar(bytes: &[u8], key: &str) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with(';') {
            continue;
        }
        let mut words = line.split_whitespace();
        if !words.next().is_some_and(|w| w.eq_ignore_ascii_case("string")) {
            continue;
        }
        if !words.next().is_some_and(|w| w.eq_ignore_ascii_case(key)) {
            continue;
        }
        let (_, value) = line.split_once('=')?;
        return Some(value.trim().trim_matches('"').to_owned());
    }
    None
}

/// The world rules a session runs with: IDS, native kind (0 bool, 1 int, 2 fixed
/// as binary32 bits, 3 enum value index) and raw value (WorldRules 0x512420).
pub type RuleRow = (String, u32, u32);

/// The world rules whose definition says `always_editable` (`Game/world_rules*.tgi`):
/// what a rule row's flag holds where no map gives it, a random map's.
fn always_editable_rules(game_root: &Path) -> Vec<Rule> {
    let Ok(mut sources) = aa_play::data::Sources::open(game_root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for file in ["Game/world_rules.tgi", "Game/world_rules_aa.tgi"] {
        let Ok(found) = sources.files.read(file) else {
            continue;
        };
        let text = String::from_utf8_lossy(&found.bytes).into_owned();
        let mut id = None;
        for line in text.lines() {
            let line = line.split(";;").next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim().to_ascii_lowercase().as_str() {
                "ids" => id = Some(value.trim().to_owned()),
                "always_editable" if value.trim().eq_ignore_ascii_case("true") => {
                    if let Some(id) = id.take() {
                        out.push(Rule { key: utf16(&id), enabled: 1, kind: 0, value: 0 });
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// The nine rules of the v14 world rule registry, in slot order, with the
/// defaults of `Game/world_rules*.tgi` a random map starts from.
fn default_rules(world: &World) -> Vec<RuleRow> {
    let fixed = |name: &str, default: f32| world.data.rule_ci(name).unwrap_or(default).to_bits();
    vec![
        ("Multiplayer".into(), 0, 0),
        ("DynamicCampaign".into(), 0, 0),
        ("Rated".into(), 0, 0),
        ("CityVictoryRatio".into(), 1, 0),
        ("DifficultyLevel".into(), 3, 1),
        ("FogOfWar".into(), 3, 1),
        ("ResourceStockpileDefault".into(), 2, fixed("ResourceStockpileDefault", 500.0)),
        ("StartingExperience".into(), 2, fixed("StartingExperience", 200.0)),
        ("Sun".into(), 3, 0),
    ]
}

/// What the battle was created from (the creator of 5A3250's header).
#[derive(Clone, Debug)]
pub enum FilmSource {
    /// `KKC_WorldCreatorLoad`: a stock map by aa-play's map id, "(2v2)Berlin".
    Map { id: String },
    /// `KKC_WorldCreatorRandom`: the generator's settings.
    Random {
        params: aa_play::mapgen::RandomMapParams,
        parts: aa_play::mapgen::CreatorParts,
    },
}

/// The start of a battle on a random map, the way a film of it starts
/// (`KKC_WorldCreatorRandom::CreateGWorld` 0x513410, the film's
/// `playback::generated_map`): the creator's lists as staging left them, the teams
/// and kingdoms and their picks on the world stream, then the generator on the
/// settings the creator keeps (0x505EA0).
pub struct RandomBattle {
    pub generated: GeneratedMap,
    /// What 0x474C14 drew, by player: `None` for a general staging chose.
    pub generals: Vec<Option<String>>,
}

/// `factions`: (player index, the general staging chose; `None`: left to the draw).
pub fn random_battle(
    maps: &RandomMapData,
    game: &GameData,
    params: &RandomMapParams,
    setup: &SkirmishSetup,
    factions: &[(usize, Option<String>)],
) -> anyhow::Result<RandomBattle> {
    let parts = maps.creator_parts(params)?;
    let majors: Vec<usize> = parts.kingdoms.iter().enumerate().filter(|(_, k)| !k.independent).map(|(i, _)| i).collect();
    let player_of = |row: usize| {
        let major = majors.iter().position(|m| *m == row)?;
        setup.players.iter().position(|p| p.start == major)
    };
    let kingdoms = parts
        .kingdoms
        .iter()
        .enumerate()
        .map(|(row, k)| {
            let player = if k.independent { None } else { player_of(row) };
            let general = player.and_then(|p| factions.iter().find(|f| f.0 == p)?.1.clone());
            WorldKingdom {
                ids: k.ids.clone(),
                independent: k.independent,
                nation: player.map_or_else(String::new, |p| format!("nation_{}", setup.players[p].nation)),
                general: general.unwrap_or_default(),
            }
        })
        .collect();
    let spec = WorldSpec { teams: parts.teams.len() as u32, kingdoms, present: Vec::new() };
    let mut read = params.clone();
    read.terrain_weights = parts.terrain_weights.iter().map(|t| t.1).collect();
    read.object_counts = parts.object_counts.iter().cloned().collect();
    let mut generated = maps.generate_world(&read, &spec, game)?;
    let generals = setup
        .players
        .iter()
        .map(|p| generated.world_picks.get(*majors.get(p.start)?)?.general.clone())
        .collect();
    // The world's kingdom order (GWorld+0x148): the header's rows, the independents
    // and the kingdoms that play (510EB0).
    let mut index = 0;
    for (row, k) in parts.kingdoms.iter().enumerate() {
        if !k.independent && player_of(row).is_none() {
            continue;
        }
        if let Some(slot) = generated.map.slots.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&k.ids)) {
            slot.kingdom_index = Some(index);
        }
        index += 1;
    }
    Ok(RandomBattle { generated, generals })
}

/// What the session knows that the world does not (the staging screen's state,
/// `KKC_Session` at OnGWorldReadyForState 0x7757F0).
#[derive(Clone, Debug)]
pub struct FilmSession {
    pub source: FilmSource,
    /// The creator seed, `SkirmishSetup::seed` (0x76AA90).
    pub seed: u64,
    /// The local player's kingdom; none for an observer.
    pub local: Option<KingdomId>,
    /// The world rules; empty: the map's own (a random map's defaults).
    pub rules: Vec<RuleRow>,
    /// Kingdoms whose general the creator leaves to 0x50E8B0's draw: their
    /// creator row has no faction, the header row has the pick. They must have
    /// been created with `StartPick::random_general`.
    pub random_generals: BTreeSet<KingdomId>,
    /// The same for nations.
    pub random_nations: BTreeSet<KingdomId>,
    /// The computer players' SAI profiles (KingdomParameter +2C).
    pub sai_profiles: BTreeMap<KingdomId, String>,
    /// Display names of the session players (`Player`, `Random`).
    pub player_names: BTreeMap<KingdomId, String>,
    /// The running game's version.
    pub version: GameVersion,
    /// The current local FILETIME (6FB1F0): [low, high].
    pub filetime: [u32; 2],
    /// What a metagame battle adds to its kingdoms.
    pub extras: BattleExtras,
}

/// What a metagame battle's launch orders add to the kingdoms of the session
/// (`TGC_AddKingdomStartPropertyOrder`, `TGC_SetKingdomBonusStockpileOrder`,
/// `TGC_AddKingdomAdditionalActorOrder`), by player index = kingdom id. A film keeps
/// them in the kingdom rows, properties, resources and definitions (which row field
/// is which is read from their names: no metagame film of the original is on hand).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BattleExtras {
    pub properties: Vec<(usize, String)>,
    /// Resource IDS and binary32 amount.
    pub bonus: Vec<(usize, String, u32)>,
    pub actors: Vec<(usize, String)>,
}

/// The extras a random-map film's kingdom rows hold, `slots` being the slot id of
/// each player. A scenario's rows are the map's own, which its restore has read.
pub fn film_extras(header: &MapHeader, slots: &[String]) -> BattleExtras {
    let mut out = BattleExtras::default();
    for (k, slot) in slots.iter().enumerate() {
        let Some(row) = header.creator.parameters.kingdoms.iter().find(|r| String::from_utf16_lossy(&r.id).eq_ignore_ascii_case(slot)) else {
            continue;
        };
        out.properties.extend(row.properties.iter().map(|p| (k, String::from_utf16_lossy(p))));
        out.bonus.extend(row.resources.iter().filter(|r| r.1 != 0).map(|r| (k, String::from_utf16_lossy(&r.0), r.1)));
        out.actors.extend(row.definitions.iter().map(|a| (k, String::from_utf16_lossy(a))));
    }
    out
}

/// Battle start of a metagame battle (4E6DB0: the start technologies 4E3AD0 and the
/// bonus stockpile 4E39D0: the sum at least 0, at most EconomyStockpileResourceMax).
/// The additional actors stand in rings around the start location on free ground: a
/// labelled rendition, `KKC_WorldCreatorRandom`'s `_ki.additional_actors` is not
/// ported.
pub fn apply_battle_extras(world: &mut World, extras: &BattleExtras) {
    let resources = world.data.resources.len();
    for (k, property) in &extras.properties {
        let id = world.kingdoms.get(*k).map(|x| x.id);
        if let Some(id) = id {
            let book = world.books.kingdom_mut(id, resources);
            if !book.technologies.iter().any(|t| t.eq_ignore_ascii_case(property)) {
                book.technologies.push(property.clone());
            }
        }
    }
    let max = world.data.rule_ci("EconomyStockpileResourceMax").unwrap_or(f32::MAX);
    for (k, resource, bits) in &extras.bonus {
        let row = world.data.resources.iter().position(|r| r.id.eq_ignore_ascii_case(resource));
        let id = world.kingdoms.get(*k).map(|x| x.id);
        if let (Some(row), Some(id)) = (row, id) {
            let book = world.books.kingdom_mut(id, resources);
            if let Some(s) = book.stockpiled.get_mut(row) {
                *s = (*s + f32::from_bits(*bits)).max(0.0).min(max);
            }
        }
    }
    let mut placed = BTreeMap::<usize, u32>::new();
    let map = world.map.clone();
    for (k, thing) in &extras.actors {
        let Some(id) = world.kingdoms.get(*k).map(|x| x.id) else { continue };
        let Some(def) = world.data.find(thing) else {
            eprintln!("the additional actor {thing} is not in the data");
            continue;
        };
        let start = world.ids.kingdom_slot(id).and_then(|slot| map.player_slots.iter().position(|&s| s == slot)).and_then(|i| map.start_locations.get(i).copied());
        let Some(at) = start else { continue };
        let n = placed.entry(*k).or_insert(0);
        let spot = ring_spot(&map, at, *n);
        *n += 1;
        let d = world.data.def(def);
        if d.class == Some(aa_play::DefClass::Organization) || !d.slots.is_empty() {
            aa_play::production::spawn_organization(world, def, id, spot, -std::f32::consts::FRAC_PI_2, None);
        } else {
            aa_play::production::spawn_single(world, def, Some(id), spot, -std::f32::consts::FRAC_PI_2);
        }
    }
}

/// The `n`th free spot of the rings of 16 m around the point.
fn ring_spot(map: &aa_play::GameMap, at: glam::Vec2, n: u32) -> glam::Vec2 {
    const SPACING: f32 = 16.0;
    let free = |q: glam::Vec2| {
        let f = map.cell_flags(q);
        f & aa_play::map::BLOCKED == 0 && f & aa_play::map::NOT_LAND_PASSABLE == 0
    };
    let mut left = n;
    for ring in 1..64u32 {
        let steps = 8 * ring;
        for i in 0..steps {
            let a = i as f32 / steps as f32 * std::f32::consts::TAU;
            let q = at + glam::Vec2::new(a.cos(), a.sin()) * (ring as f32 * SPACING);
            if !free(q) {
                continue;
            }
            if left == 0 {
                return q;
            }
            left -= 1;
        }
    }
    at
}

/// The current local time as a FILETIME, the way 6FB1F0 reads it:
/// GetSystemTimeAsFileTime, then FileTimeToLocalFileTime.
pub fn filetime_now() -> [u32; 2] {
    const TO_1601: u64 = 116_444_736_000_000_000;
    let since = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() / 100) as u64;
    let t = since + TO_1601;
    to_local([t as u32, (t >> 32) as u32])
}

#[cfg(windows)]
fn to_local(utc: [u32; 2]) -> [u32; 2] {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn FileTimeToLocalFileTime(utc: *const [u32; 2], local: *mut [u32; 2]) -> i32;
    }
    let mut local = utc;
    // SAFETY: both point at a valid FILETIME, two 32-bit words low first.
    let done = unsafe { FileTimeToLocalFileTime(&utc, &mut local) };
    if done != 0 { local } else { utc }
}
#[cfg(not(windows))]
fn to_local(utc: [u32; 2]) -> [u32; 2] {
    utc
}

fn utf16(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// See `prelude`: the biome of a `KKC_WorldCreatorLoad` in a film.
const LOAD_CREATOR_BIOME: &str = "temperate_grassland";

/// The kingdom glyph staging writes before a player's colour text.
const KINGDOM_GLYPH: u16 = 0x84;

/// The session player of one kingdom (or the observer host).
#[derive(Clone, Debug)]
struct RosterPlayer {
    id: u32,
    kingdom: Option<KingdomId>,
    computer: bool,
}

fn roster(world: &World) -> Vec<RosterPlayer> {
    // The host's player first (id 1), then the computer players (0x76D970 gives
    // ids in creation order; the films' humans have the low ones).
    let majors = world.kingdoms.iter().filter(|k| !k.independent);
    let mut humans: Vec<KingdomId> = Vec::new();
    let mut computers: Vec<KingdomId> = Vec::new();
    for k in majors {
        match k.controller {
            Controller::Computer(_) => computers.push(k.id),
            _ => humans.push(k.id),
        }
    }
    let mut out = Vec::new();
    let mut id = 1;
    if humans.is_empty() {
        // Everyone is a computer: the host watches, with no kingdom.
        out.push(RosterPlayer {
            id,
            kingdom: None,
            computer: false,
        });
        id += 1;
    }
    for (list, computer) in [(humans, false), (computers, true)] {
        for k in list {
            out.push(RosterPlayer {
                id,
                kingdom: Some(k),
                computer,
            });
            id += 1;
        }
    }
    out
}

/// The map kingdom row of an aa-play kingdom.
fn slot_of(world: &World, kingdom: KingdomId) -> Option<&aa_play::map::MapSlot> {
    world.map.slots.get(world.ids.kingdom_slot(kingdom)?)
}

fn nation_id(nation: &str) -> Vec<u16> {
    if nation.is_empty() {
        Vec::new()
    } else {
        utf16(&format!("nation_{nation}"))
    }
}

fn general_id(world: &World, kingdom: KingdomId) -> Vec<u16> {
    aa_play::special_ops::general(world, kingdom).map_or(Vec::new(), |g| utf16(&g.id))
}

/// The kingdom's colour id: the map's unless the player took another one.
fn color_id(world: &World, kingdom: KingdomId, base: &[u16]) -> Vec<u16> {
    let Some(k) = world.kingdoms.iter().find(|k| k.id == kingdom) else {
        return base.to_vec();
    };
    let rgb = k.color;
    world
        .data
        .kingdom_colors
        .iter()
        .find(|(_, c)| c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8) == rgb)
        .map_or_else(|| base.to_vec(), |(id, _)| utf16(id))
}

/// `<color=r,g,b>` + glyph: the name prefix the session stores for a player, the
/// kingdom colour's own numbers (0x50E760), the shortest form as %g prints them.
fn color_text(world: &World, kingdom: KingdomId) -> Vec<u16> {
    let k = world.kingdoms.iter().find(|k| k.id == kingdom);
    let rgb = k.map_or([255, 255, 255], |k| k.color);
    let exact = world
        .data
        .kingdom_colors
        .iter()
        .find(|(_, c)| c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8) == rgb)
        .map(|(_, c)| *c);
    let c = exact.unwrap_or(rgb.map(|v| f32::from(v) / 255.0));
    let mut out = utf16(&format!("<color={},{},{}>", c[0], c[1], c[2]));
    out.push(KINGDOM_GLYPH);
    out
}

/// The team id a kingdom plays on: `teams[team]` of the header's team list, none
/// for the no-team team (the client's team numbers past the map's).
fn team_id(world: &World, kingdom: KingdomId, teams: &[TeamParameter]) -> Vec<u16> {
    let Some(k) = world.kingdoms.iter().find(|k| k.id == kingdom) else {
        return Vec::new();
    };
    teams.get(k.team as usize).map_or(Vec::new(), |t| t.id.clone())
}

/// The handicap id of a computer player whose level is not the DifficultyLevel
/// rule's (the rule covers the rest, as in the films).
fn handicap_id(world: &World, kingdom: KingdomId, difficulty: u32) -> Vec<u16> {
    let Some(k) = world.kingdoms.iter().find(|k| k.id == kingdom) else {
        return Vec::new();
    };
    match k.controller {
        Controller::Computer(level) if u32::from(level) != difficulty => world
            .data
            .handicaps
            .get(level as usize)
            .map_or(Vec::new(), |h| utf16(&h.0)),
        _ => Vec::new(),
    }
}

/// One kingdom row of the creator or the header, on a template row.
#[allow(clippy::too_many_arguments)]
fn kingdom_row(
    world: &World,
    session: &FilmSession,
    template: &KingdomParameter,
    played: Option<KingdomId>,
    in_header: bool,
    difficulty: u32,
    teams: &[TeamParameter],
) -> KingdomParameter {
    let mut row = template.clone();
    row.string_0c = Some(Vec::new());
    row.string_2c = Vec::new();
    row.nation = Vec::new();
    row.faction = Vec::new();
    row.byte_32 = 1;
    // A map saved before parameter version 13 has no resource rows (50D730 skips
    // them); the session's row was made fresh, with every resource at zero.
    if row.resources.is_empty() {
        row.resources = resource_rows(world);
    }
    let independent = template.byte_10 != 0;
    if independent {
        row.byte_29 = 1;
        if in_header && row.color.is_empty() {
            row.color = utf16("default");
        }
        return row;
    }
    let Some(kingdom) = played else {
        row.byte_29 = 0;
        row.byte_31 = 1;
        row.string_14 = Vec::new();
        return row;
    };
    let k = world.kingdoms.iter().find(|k| k.id == kingdom);
    row.byte_31 = 0;
    // 0 for a person (Session player of a human), 1 for a computer player.
    row.byte_32 = u8::from(session.local != Some(kingdom) && k.is_some_and(|k| matches!(k.controller, Controller::Computer(_))));
    row.color = color_id(world, kingdom, &template.color);
    row.string_14 = team_id(world, kingdom, teams);
    row.handicap = handicap_id(world, kingdom, difficulty);
    // The launch orders of a metagame battle.
    let k_index = kingdom as usize;
    row.properties.extend(session.extras.properties.iter().filter(|e| e.0 == k_index).map(|e| utf16(&e.1)));
    row.definitions.extend(session.extras.actors.iter().filter(|e| e.0 == k_index).map(|e| utf16(&e.1)));
    for (_, resource, bits) in session.extras.bonus.iter().filter(|e| e.0 == k_index) {
        let id = utf16(resource);
        if let Some(r) = row.resources.iter_mut().find(|r| r.0 == id) {
            r.1 = *bits;
        }
    }
    let nation = nation_id(k.map_or("", |k| k.nation.as_str()));
    let general = general_id(world, kingdom);
    if in_header {
        row.byte_29 = 1;
        row.nation = nation;
        row.faction = general;
        row.string_0c = Some(color_text(world, kingdom));
        row.string_2c = session
            .sai_profiles
            .get(&kingdom)
            .map_or(Vec::new(), |p| utf16(p));
    } else {
        if !session.random_nations.contains(&kingdom) {
            row.nation = nation;
        }
        if !session.random_generals.contains(&kingdom) {
            row.faction = general;
        }
        row.string_0c = Some(vec![KINGDOM_GLYPH]);
    }
    row
}

/// `(id, played kingdom)` for the template rows.
fn played_of(world: &World, id: &[u16]) -> Option<KingdomId> {
    let id = String::from_utf16_lossy(id);
    world
        .kingdoms
        .iter()
        .find(|k| slot_of(world, k.id).is_some_and(|s| s.id.eq_ignore_ascii_case(&id)))
        .map(|k| k.id)
}

fn rule_rows(rows: &[RuleRow], enabled: &[Rule]) -> Vec<Rule> {
    rows.iter()
        .map(|(key, kind, value)| {
            let units = utf16(key);
            Rule {
                enabled: enabled.iter().find(|r| r.key == units).map_or(0, |r| r.enabled),
                key: units,
                kind: *kind,
                value: *value,
            }
        })
        .collect()
}

/// What 0x76C310 writes before the Orders chunk.
pub struct FilmPrelude {
    pub header: MapHeader,
    pub creator: DerivedCreator,
    pub map_strings: Vec<u8>,
    pub players: Vec<SessionPlayer>,
}

/// The map's text table as `(key, text)` rows, in file order, with the header's
/// `name` apart.
fn parse_text_table(bytes: &[u8]) -> (String, Vec<(String, String)>) {
    let text: String = bytes.iter().map(|b| *b as char).collect();
    let (mut name, mut rows) = (String::new(), Vec::new());
    for line in text.lines() {
        let line = line.trim();
        let Some((key, value)) = line.split_once(" = ") else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_owned();
        match key {
            "ids" => {}
            "name" => name = value,
            _ => rows.push((key.to_owned(), value)),
        }
    }
    (name, rows)
}

/// The `SessionPlayers` rows of the roster (0x76C310).
fn session_players(world: &World, roster: &[RosterPlayer]) -> Vec<SessionPlayer> {
    roster
        .iter()
        .map(|p| {
            let slot = p.kingdom.and_then(|k| slot_of(world, k));
            SessionPlayer {
                id: p.id,
                name: slot.map_or_else(|| "observer".to_owned(), |s| s.id.clone()),
                kingdom_index: slot.and_then(|s| s.kingdom_index),
            }
        })
        .collect()
}

/// 5A23F0 / 5A26E0 with the creator of the session (the film header), the map
/// text 59FDA0 and the session players of 0x76C310.
pub fn prelude(world: &World, game_root: &Path, session: &FilmSession) -> anyhow::Result<FilmPrelude> {
    let roster = roster(world);
    let players = session_players(world, &roster);
    let entries: Vec<HeaderEntry> = roster
        .iter()
        .filter_map(|p| {
            let k = p.kingdom?;
            let slot = slot_of(world, k)?;
            Some(HeaderEntry {
                flag: u8::from(p.computer),
                string_20: utf16(&slot.id),
                string_18: utf16(session.player_names.get(&k).map_or_else(
                    || if p.computer { "Random" } else { "Player" },
                    String::as_str,
                )),
                flag_v5: Some(u8::from(session.local == Some(k))),
            })
        })
        .collect();
    let v = &session.version;
    let seed = session.seed as u32;
    let filetime = session.filetime;
    let rename_players = |rows: &mut Vec<(String, String)>| {
        for (key, value) in rows.iter_mut() {
            if let Some(e) = entries
                .iter()
                .find(|e| String::from_utf16_lossy(&e.string_20).eq_ignore_ascii_case(key))
            {
                *value = String::from_utf16_lossy(&e.string_18);
            }
        }
    };
    let source = aa_mapworld::SourceIdentity::of(&[]);
    match &session.source {
        FilmSource::Map { id } => {
            let bytes = aa_play::map::read_rmp(game_root, id)?;
            let archive = aa_mapworld::MapArchive::from_bytes(bytes)?;
            let chunk = archive.decode_chunk("MAP_HEADER", 1 << 30)?;
            let (base, _) = MapHeader::decode_any_creator(&chunk)?;
            let rows = if session.rules.is_empty() {
                base.rules
                    .rules
                    .iter()
                    .map(|r| (String::from_utf16_lossy(&r.key), r.kind, r.value))
                    .collect()
            } else {
                session.rules.clone()
            };
            let difficulty = rows
                .iter()
                .find(|r| r.0.eq_ignore_ascii_case("DifficultyLevel"))
                .map_or(1, |r| r.2);
            let rules = rule_rows(&rows, &base.rules.rules);
            let name = utf16(&format!("Maps/{id}.RMP"));
            let teams = base.parameters.teams.clone();
            // The creator keeps every kingdom of the map, the header what the
            // world has (510EB0): the kingdoms of unused slots are gone.
            let creator_parameters = WorldParameters {
                version: 13,
                teams: base.creator.parameters.teams.clone(),
                kingdoms: base
                    .creator
                    .parameters
                    .kingdoms
                    .iter()
                    .map(|t| kingdom_row(world, session, t, played_of(world, &t.id), false, difficulty, &teams))
                    .collect(),
            };
            let header_parameters = WorldParameters {
                version: 13,
                teams: base.parameters.teams.clone(),
                kingdoms: base
                    .parameters
                    .kingdoms
                    .iter()
                    .filter_map(|t| {
                        let played = played_of(world, &t.id);
                        (t.byte_10 != 0 || played.is_some())
                            .then(|| kingdom_row(world, session, t, played, true, difficulty, &teams))
                    })
                    .collect(),
            };
            let header = MapHeader {
                source,
                version: 7,
                rules: WorldRules {
                    version: 4,
                    mode: 3,
                    game_type: base.rules.game_type.clone(),
                    rules: rules.clone(),
                },
                parameters: header_parameters,
                words_0c_10: [7, 0],
                name_key: Vec::new(),
                description_key: Vec::new(),
                display_extents_bits: base.display_extents_bits.or(Some(base.creator.world_extents_bits)),
                word_50: BUILD_BITS,
                word_54: numeric_version(&v.version),
                string_58: utf16(&v.version),
                string_5c: utf16(&v.application),
                string_60: utf16(&v.release),
                byte_64: 0,
                string_38: name.clone(),
                string_3c: Vec::new(),
                word_40: Some(u32::MAX),
                words_18_1c: filetime,
                entries,
                creator_offset: 0,
                creator: WorldCreator {
                    class_name: utf16("KKC_WorldCreatorLoad"),
                    version: 3,
                    rules: WorldRules {
                        version: 4,
                        mode: 0,
                        game_type: base.creator.rules.game_type.clone(),
                        rules,
                    },
                    parameters: creator_parameters,
                    world_extents_bits: base.creator.world_extents_bits,
                    // A load creator keeps its constructor's biome: the world's
                    // comes from the source map. Every Load film of the corpus
                    // has this one, whatever its map has.
                    biome: utf16(LOAD_CREATOR_BIOME),
                    weather: base.creator.weather.clone(),
                    rng_seed: seed,
                },
            };
            let strings = archive
                .decode_chunk("MAPSTRINGS", 1 << 30)
                .map(|c| c.bytes)
                .unwrap_or_default();
            let (title, mut text) = parse_text_table(&strings);
            let entries_for_text = &header.entries;
            for (key, value) in text.iter_mut() {
                if let Some(e) = entries_for_text
                    .iter()
                    .find(|e| String::from_utf16_lossy(&e.string_20).eq_ignore_ascii_case(key))
                {
                    *value = String::from_utf16_lossy(&e.string_18);
                }
            }
            Ok(FilmPrelude {
                creator: DerivedCreator::Load {
                    derived_version: 0,
                    source_resource_ids: name,
                },
                map_strings: film_writer::map_strings_bytes(&title, &text),
                header,
                players,
            })
        }
        FilmSource::Random { params, parts } => {
            let rows = if session.rules.is_empty() {
                default_rules(world)
            } else {
                session.rules.clone()
            };
            let difficulty = rows
                .iter()
                .find(|r| r.0.eq_ignore_ascii_case("DifficultyLevel"))
                .map_or(1, |r| r.2);
            let rules = rule_rows(&rows, &always_editable_rules(game_root));
            let templates: Vec<KingdomParameter> = parts
                .kingdoms
                .iter()
                .map(|k| KingdomParameter {
                    id: utf16(&k.ids),
                    byte_10: u8::from(k.independent),
                    nation: Vec::new(),
                    faction: Vec::new(),
                    color: k.color.as_deref().map_or(Vec::new(), utf16),
                    handicap: Vec::new(),
                    display_key: utf16(&k.ids),
                    string_0c: Some(Vec::new()),
                    string_2c: Vec::new(),
                    byte_29: 0,
                    byte_28: 1,
                    string_14: k.team.as_deref().map_or(Vec::new(), utf16),
                    localized_entries: Vec::new(),
                    definitions: Vec::new(),
                    string_58: Vec::new(),
                    properties: Vec::new(),
                    byte_31: 0,
                    byte_32: 1,
                    resources: resource_rows(world),
                })
                .collect();
            let teams: Vec<TeamParameter> = parts
                .teams
                .iter()
                .enumerate()
                .map(|(i, id)| TeamParameter {
                    id: utf16(id),
                    display_key: utf16(&format!("team_{i}")),
                })
                .collect();
            // The creator's teams are named by their ids; the header's, the world's,
            // by the map text (`team_<i>`).
            let creator_teams = teams.iter().map(|t| TeamParameter { id: t.id.clone(), display_key: t.id.clone() }).collect();
            let creator_parameters = WorldParameters {
                version: 13,
                teams: creator_teams,
                kingdoms: templates
                    .iter()
                    .map(|t| kingdom_row(world, session, t, played_of(world, &t.id), false, difficulty, &teams))
                    .collect(),
            };
            let header_parameters = WorldParameters {
                version: 13,
                teams: teams.clone(),
                kingdoms: templates
                    .iter()
                    .filter_map(|t| {
                        let played = played_of(world, &t.id);
                        (t.byte_10 != 0 || played.is_some())
                            .then(|| kingdom_row(world, session, t, played, true, difficulty, &teams))
                    })
                    .collect(),
            };
            let extents = [params.width.to_bits(), params.height.to_bits()];
            let mut text: Vec<(String, String)> = Vec::new();
            for (i, _) in parts.teams.iter().enumerate() {
                text.push((format!("team_{i}"), format!("Team {}", i + 1)));
            }
            for k in &parts.kingdoms {
                text.push((k.ids.clone(), k.name.clone()));
            }
            text.push(("map_description".into(), "Map Description".into()));
            text.push(("map_name".into(), "New Map".into()));
            rename_players(&mut text);
            let settings = RandomCreatorSettings {
                version: 1,
                rmc: utf16(&params.rmc),
                terrain_weights: parts
                    .terrain_weights
                    .iter()
                    .map(|(id, w)| (utf16(id), w.to_bits()))
                    .collect(),
                total_terrain_weight_bits: parts.total_terrain_weight.to_bits(),
                object_counts: parts
                    .object_counts
                    .iter()
                    .map(|(id, c)| (utf16(id), c.to_bits()))
                    .collect(),
                num_heroes: params.heroes,
            };
            let header = MapHeader {
                source,
                version: 7,
                rules: WorldRules {
                    version: 4,
                    mode: 3,
                    game_type: utf16("NORMAL"),
                    rules: rules.clone(),
                },
                parameters: header_parameters,
                words_0c_10: [7, 0],
                name_key: Vec::new(),
                description_key: Vec::new(),
                display_extents_bits: Some(extents),
                word_50: BUILD_BITS,
                word_54: numeric_version(&v.version),
                string_58: utf16(&v.version),
                string_5c: utf16(&v.application),
                string_60: utf16(&v.release),
                byte_64: 0,
                string_38: utf16(&params.rmc),
                string_3c: Vec::new(),
                word_40: Some(u32::MAX),
                words_18_1c: filetime,
                entries,
                creator_offset: 0,
                creator: WorldCreator {
                    class_name: utf16("KKC_WorldCreatorRandom"),
                    version: 3,
                    rules: WorldRules {
                        version: 4,
                        mode: 0,
                        game_type: utf16("NORMAL"),
                        rules,
                    },
                    parameters: creator_parameters,
                    world_extents_bits: extents,
                    biome: utf16(&world.map.biome),
                    weather: utf16(&world.map.weather),
                    rng_seed: seed,
                },
            };
            Ok(FilmPrelude {
                creator: DerivedCreator::Random(settings),
                map_strings: film_writer::map_strings_bytes("New Map", &text),
                header,
                players,
            })
        }
    }
}

/// A kingdom row's resource list: every resource at zero (50D730 over a fresh
/// parameter row).
fn resource_rows(world: &World) -> Vec<(Vec<u16>, u32)> {
    world.data.resources.iter().map(|r| (utf16(&r.id), 0)).collect()
}

/// The film name rotation 0x76FE60 over UVar `LastFilmIndex`: the name for the
/// index clamped to the queue, and the next index. `base` is the localized
/// `last_film_filename`, or AVars `LastFilmName`.
pub fn rotate_film_name(base: &str, last_film_index: i32, queue: i32) -> (String, i32) {
    let queue = queue.max(1);
    let index = last_film_index.max(0).min(queue - 1);
    (format!("{base}_{index}"), (index + 1) % queue)
}

/// `<folder>/<name>.RFM`: 0x7757F0 appends the extension to the rotation name,
/// whatever it ends in.
pub fn film_path(folder: &Path, name: &str) -> PathBuf {
    folder.join(format!("{name}.{}", film_writer::FILM_EXTENSION))
}

/// A recording that goes to a file, as OnGWorldReadyForState 0x7757F0 opens it.
pub struct FilmTarget {
    /// The client's own film folder (never the original's).
    pub folder: PathBuf,
    /// The rotation base name (0x76FE60).
    pub base: String,
    /// UVar `LastFilmIndex` now; [`FilmOpened::next_index`] is what it becomes.
    pub last_film_index: i32,
    /// AVars `FilmRotationQueueSize`.
    pub queue: i32,
}

/// What 0x7757F0 leaves after a successful open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilmOpened {
    /// Session+0x150: the rotation name, without extension.
    pub name: String,
    pub path: PathBuf,
    /// The value of `LastFilmIndex` after every try.
    pub next_index: i32,
}

/// `failed_to_create_film|s` (0x775A40): every try of the rotation failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilmCreateFailed {
    pub name: String,
    pub next_index: i32,
    pub error: String,
}

/// Open a film file (0x7757F0): up to `queue` tries, each with a fresh rotation
/// name; a name that cannot be created (the file is locked) moves on to the next.
pub fn open_film_file(
    prelude: &FilmPrelude,
    target: &FilmTarget,
) -> Result<(FilmFile, FilmOpened), FilmCreateFailed> {
    let header = film_writer::map_header_bytes(&prelude.header, &prelude.creator);
    let players = film_writer::session_players_bytes(&prelude.players);
    let mut index = target.last_film_index;
    let mut last = FilmCreateFailed {
        name: target.base.clone(),
        next_index: index,
        error: String::new(),
    };
    let _ = std::fs::create_dir_all(&target.folder);
    for _ in 0..target.queue.max(1) {
        let (name, next) = rotate_film_name(&target.base, index, target.queue);
        index = next;
        let path = film_path(&target.folder, &name);
        match FilmFile::create(&path, &header, &prelude.map_strings, &players) {
            Ok(file) => {
                // 59FDA0: the separate text beside the film; its date is the
                // film's start.
                let _ = std::fs::write(
                    target
                        .folder
                        .join(format!("{name}.{}", film_writer::TEXT_EXTENSION)),
                    &prelude.map_strings,
                );
                return Ok((
                    file,
                    FilmOpened {
                        name,
                        path,
                        next_index: index,
                    },
                ));
            }
            Err(e) => {
                last = FilmCreateFailed {
                    name,
                    next_index: index,
                    error: e.to_string(),
                };
            }
        }
    }
    Err(last)
}

/// The ParameterSet of a recruit: the definition's own layout and the element of
/// each layout entry, which is what the real films' orders name.
pub fn own_parameters(world: &World, def: aa_play::DefId) -> aa_play::RecruitParameters {
    let d = world.data.def(def);
    let entries = d.slot_entries.iter().max().map_or(0, |m| *m as usize + 1);
    let mut elements = vec![aa_play::data::NO_DEF; entries];
    for (slot, entry) in d.slot_entries.iter().enumerate() {
        if let Some(own) = d.slots.get(slot) {
            elements[*entry as usize] = own.0;
        }
    }
    aa_play::RecruitParameters { layout: d.layout.clone(), elements }
}

/// `Tables` of the wire forms, from the registries of the running data.
pub fn wire_tables(registries: &Registries) -> Tables {
    use crate::registries as r;
    let count = |registry: Registry| registries.count(registry).map_or(0, |(n, _)| n);
    Tables {
        objects: registries.object_ids.clone(),
        formations: registries.formation_ids.clone(),
        properties: registries.property_ids.clone(),
        property_count: count(r::SHARED_PROPERTIES),
        abilities: registries.ability_ids.clone(),
        ability_count: count(r::ABILITIES),
        organization_layout_count: count(r::ORGANIZATION_LAYOUTS),
        layouts: registries.layout_ids.clone(),
    }
}

/// What a recording has done so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RecordingStats {
    /// Records written (world ticks).
    pub records: u32,
    pub decrees: u64,
    pub checkpoints: u32,
    /// Commands that have no decree yet, by reason. The world still got them,
    /// so a film with any of these will go out of sync.
    pub unmapped: BTreeMap<String, u64>,
    /// The orders of a person the Mapper turned into no command, logged once each.
    pub dropped: std::collections::BTreeSet<&'static str>,
}

/// One request waiting for the next admin tick: who asked, the decree, and the
/// commands the world runs on it.
struct Pending {
    player: u32,
    decree: WireDecree,
    commands: Vec<Command>,
}

/// The decree stream and film record of a running battle.
pub struct Recorder {
    registries: Arc<Registries>,
    tables: Tables,
    roster: Vec<RosterPlayer>,
    host: u32,
    /// Node+0x2C: the 298 of the node's first request in a tick (0x751B60).
    own: u32,
    /// Admin+0x210: the player of the last SetPlayer request (0x74B360).
    last_request: Option<u32>,
    outboxes: BTreeMap<u32, Outbox>,
    orders: OrdersWriter,
    /// `[0xB40904]`: admin ticks since the last checkpoint.
    counter: u32,
    due: bool,
    /// The caller runs the admin clock with [`Recorder::admin_tick`].
    pub own_admin_clock: bool,
    /// The HUD's MoveOnto waits for the kill it goes with.
    move_onto: bool,
    file: Option<FilmFile>,
    opened: Option<FilmOpened>,
    stats: RecordingStats,
    /// What a replay's driver keeps: the ordering player and the selections.
    mapper: Mapper,
    /// The film's players as a replay sees them.
    players: Vec<SessionPlayer>,
}

impl Recorder {
    /// The stream of `world`, freshly built and with its generals set
    /// (OnGWorldReadyForState 0x7757F0). `file` is where the records go; a test
    /// launch has none.
    pub fn new(world: &World, registries: Arc<Registries>, file: Option<(FilmFile, FilmOpened)>) -> Self {
        let roster = roster(world);
        let host = roster.first().map_or(1, |p| p.id);
        let tables = wire_tables(&registries);
        let players = session_players(world, &roster);
        let (file, opened) = match file {
            Some((f, o)) => (Some(f), Some(o)),
            None => (None, None),
        };
        Self {
            orders: OrdersWriter::new().expect("a new order stream"),
            registries,
            tables,
            roster,
            host,
            own: host,
            last_request: None,
            outboxes: BTreeMap::new(),
            counter: 0,
            due: false,
            own_admin_clock: false,
            move_onto: false,
            file,
            opened,
            stats: RecordingStats::default(),
            mapper: Mapper::new(),
            players,
        }
    }

    pub fn stats(&self) -> &RecordingStats {
        &self.stats
    }
    /// Session+0x150 and the file, while a file is being written.
    pub fn opened(&self) -> Option<&FilmOpened> {
        self.opened.as_ref()
    }
    /// True while records go to a file (Admin+0x268 set).
    pub fn writing(&self) -> bool {
        self.file.is_some()
    }

    fn player_of(&self, kingdom: KingdomId) -> Option<&RosterPlayer> {
        self.roster.iter().find(|p| p.kingdom == Some(kingdom))
    }

    /// One admin tick that the game ran (0x755290 with 0x76AEE0 true). The first
    /// one, before update 1, counts for nothing: the films' first checkpoint is
    /// in record 17.
    pub fn admin_tick(&mut self) {
        if self.stats.records == 0 {
            return;
        }
        self.counter += 1;
        if self.counter > 0x10 {
            self.counter = 0;
            self.due = true;
        }
    }

    /// `kingdom`'s command as requests: selection, then the order. The commands
    /// are returned on the decree that runs them.
    fn requests(&mut self, world: &World, command: Command, out: &mut Vec<Pending>) {
        // A person's right click on an enemy: the HUD's MoveOnto and kill are one order,
        // move at the thing (0x4DA570's hot command, 0x527E30). The mapper makes both of
        // it, here and in the film.
        if matches!(command, Command::Order { order: ActorOrder::MoveOnto, .. }) {
            self.move_onto = true;
            return;
        }
        let command = match (std::mem::take(&mut self.move_onto), command) {
            (true, Command::Attack { kingdom, units, target }) => Command::Follow { kingdom, units, target },
            (_, other) => other,
        };
        let Some(kingdom) = command_kingdom(&command) else {
            self.unmapped("a command without a kingdom");
            return;
        };
        let Some(&RosterPlayer { id: player, computer, .. }) = self.player_of(kingdom) else {
            self.unmapped("a command of a kingdom without a session player");
            out.push(Pending {
                player: self.host,
                decree: WireDecree {
                    descriptor: id::SET_PLAYER,
                    fields: vec![Field::Uint {
                        width: 32,
                        value: self.host,
                    }],
                },
                commands: vec![command],
            });
            return;
        };
        let outbox = self.outboxes.entry(player).or_default();
        outbox.form = if computer { wire::OrderForm::Computer } else { wire::OrderForm::Person };
        if computer {
            // 0x7AF220: 0x7AD6D0 selects before every order.
            outbox.selection.clear();
        }
        // An organization's recruit carries its ParameterSet (flag 5, 0x61EBA0): the
        // real films' do, and without one 0x582AE0 logs "Using default layout" and
        // the original's regiment is not this one.
        let command = match command {
            Command::Recruit { kingdom, building, def } if world.data.def(def).layout.is_some() => {
                Command::RecruitWith { kingdom, building, def, parameters: own_parameters(world, def) }
            }
            Command::Order { kingdom, units, order: ActorOrder::RecruitAt { def, at, parameters: None } } if world.data.def(def).layout.is_some() => {
                Command::Order { kingdom, units, order: ActorOrder::RecruitAt { def, at, parameters: Some(own_parameters(world, def)) } }
            }
            other => other,
        };
        // A person's selection as it changes: the decrees, nothing for the world.
        if matches!(command, Command::Select { .. }) {
            if let Ok(requests) = wire::requests(world, &self.tables, outbox, &command) {
                for (request, fields) in requests {
                    for decree in film_decrees(request, fields, computer, 0, self.outboxes.entry(player).or_default()) {
                        out.push(Pending { player, decree, commands: Vec::new() });
                    }
                }
            }
            return;
        }
        match wire::requests(world, &self.tables, outbox, &command) {
            Ok(requests) => {
                let last = requests.len();
                let units = command_units(&command);
                let group = (computer && units.len() > 1).then(|| Command::Order {
                    kingdom,
                    units: units.clone(),
                    order: ActorOrder::GroupBehavior {
                        enable: true,
                        boolean: Some(true),
                    },
                });
                let mut group = group;
                let mut command = Some(command);
                for (i, (request, fields)) in requests.into_iter().enumerate() {
                    let is_last = i + 1 == last;
                    for d in film_decrees(request, fields, computer, units.len(), self.outboxes.entry(player).or_default()) {
                        // 0x4ED360 sets the selection's group on the first order.
                        let commands = if d.descriptor == id::TELL_SELECTED_COMMAND && d.fields == group_behavior_enable() {
                            group.take().into_iter().collect()
                        } else {
                            Vec::new()
                        };
                        out.push(Pending {
                            player,
                            decree: d,
                            commands,
                        });
                    }
                    if is_last && let Some(p) = out.last_mut() {
                        p.commands.extend(command.take());
                    }
                }
                // No request at all: a selection already right and no order
                // cannot happen; keep the world's command regardless.
                if let Some(c) = command {
                    self.unmapped("a command with no request");
                    out.push(Pending {
                        player,
                        decree: set_player(player),
                        commands: vec![c],
                    });
                }
            }
            Err(why) => {
                // The game goes on: the world gets its command, the film cannot
                // carry it. Reported, and the film will go out of sync.
                self.unmapped(&why);
                out.push(Pending {
                    player,
                    decree: set_player(player),
                    commands: vec![command],
                });
            }
        }
    }

    fn unmapped(&mut self, why: &str) {
        *self.stats.unmapped.entry(why.to_owned()).or_default() += 1;
    }

    /// One world tick of a recorded battle: the admin tick's decrees, the record,
    /// and the update. Use it in place of `World::step`.
    pub fn step(&mut self, world: &mut World, commands: Vec<Command>) {
        if !self.own_admin_clock {
            self.admin_tick();
        }
        let record = self.stats.records;
        let mut requests: Vec<Pending> = Vec::new();
        // 0x74E670 captures S first of all: its value holds none of this tick's
        // decrees, though its decree runs after the requests already queued.
        let checkpoint = std::mem::take(&mut self.due).then(|| world.trace.checkpoint());
        if record == 1 {
            self.rename_computers(world, &mut requests);
        }
        for command in std::mem::take(&mut world.sai_orders) {
            self.requests(world, command, &mut requests);
        }
        for command in commands {
            self.requests(world, command, &mut requests);
        }
        if let Some((index, hash)) = checkpoint {
            requests.push(Pending {
                player: self.host,
                decree: WireDecree {
                    descriptor: id::SIM_SYNC_VALUE,
                    fields: vec![
                        Field::Uint {
                            width: 32,
                            value: index as u32,
                        },
                        Field::Uint {
                            width: 32,
                            value: hash as u32,
                        },
                    ],
                },
                commands: Vec::new(),
            });
            self.stats.checkpoints += 1;
        }
        // The head of 0x752AE0's decrees: the tick the S entries are logged in.
        world.events.clear();
        world.trace.tick = world.clock.tick.wrapping_add(1);
        if record == 0 {
            // The two admin ticks before update 1 each announce the host; their
            // only request is the version acknowledgement, which writes no decree.
            for _ in 0..2 {
                self.run_decree(world, self.host, &set_player(self.host), Vec::new());
            }
            self.last_request = Some(self.host);
        }
        if !requests.is_empty() {
            self.run_decree(world, self.own, &set_player(self.own), Vec::new());
        }
        for Pending { player, decree, commands } in requests {
            if self.last_request != Some(player) {
                self.run_decree(world, player, &set_player(player), Vec::new());
                self.last_request = Some(player);
            }
            self.own = player;
            self.run_decree(world, player, &decree, commands);
        }
        // 0x74D640 at the start of 0x752AE0, then the update.
        match self.orders.record() {
            Ok(bytes) => {
                if let Some(file) = &mut self.file
                    && let Err(e) = file.write_record(&bytes)
                {
                    self.unmapped(&format!("film write failed: {e}"));
                }
            }
            Err(e) => self.unmapped(&format!("film record failed: {e}")),
        }
        self.stats.records += 1;
        crate::replay::update_world(world);
    }

    /// One decree of the mirror stream, as a film's runs (the loop of
    /// `FilmDriver::run_record`): written, its S entries, then what the mapper
    /// makes of it. `fallback` are the commands it stands for: they run when the
    /// mapper cannot, so the game goes on, and the film will not match.
    fn run_decree(&mut self, world: &mut World, player: u32, decree: &WireDecree, fallback: Vec<Command>) {
        // A carrier of commands with no decree of their own writes nothing.
        let carrier = decree.descriptor == id::SET_PLAYER && !fallback.is_empty();
        if !carrier {
            match self.orders.decree(decree, &*self.registries) {
                Ok(()) => self.stats.decrees += 1,
                Err(e) => self.unmapped(&format!("decree {} not written: {e}", decree.descriptor)),
            }
        }
        if let Some(name) = decree_s_entries(decree.descriptor) {
            let units: Vec<u16> = name.encode_utf16().collect();
            world.trace.log_decree(decree.descriptor as u32, &units);
        }
        let mapped = if carrier {
            Mapped::Unresolved("a command with no decree".into())
        } else {
            let registries = &self.registries;
            let counts = |r| registries.count(r).map(|(n, _)| n);
            let film_decree = FilmDecree {
                descriptor: decree.descriptor as u16,
                name: aa_orders::native::DESCRIPTORS[decree.descriptor].name,
                player: Some(player),
                payload_bit: 0,
                decree: Decree::from_fields(decree.descriptor, &decree.fields, &counts),
                fields: decree.fields.clone(),
            };
            let ids = PlayIds {
                world: &*world,
                players: &self.players,
                object_ids: &registries.object_ids,
                property_ids: &registries.property_ids,
                formation_ids: &registries.formation_ids,
                ability_ids: &registries.ability_ids,
                layout_ids: &registries.layout_ids,
            };
            self.mapper.map(&film_decree, &ids)
        };
        match mapped {
            Mapped::Commands(commands) => {
                // The Mapper's guess found nothing for a person's order: it does nothing.
                if commands.is_empty() && !fallback.is_empty() {
                    let name = aa_orders::native::DESCRIPTORS[decree.descriptor].name;
                    if self.stats.dropped.insert(name) {
                        eprintln!("film: an order of a person came to nothing: {name}");
                    }
                }
                for command in commands {
                    world.apply(command);
                }
            }
            Mapped::State | Mapped::NoEffect(_) => {}
            Mapped::NoEquivalent(why) | Mapped::Unresolved(why) => {
                if !fallback.is_empty() {
                    self.unmapped(&why);
                    for command in fallback {
                        world.apply(command);
                    }
                }
            }
        }
    }

    /// 328 for every computer player, in slot order: `<name> AI` (SAI naming).
    fn rename_computers(&self, world: &World, out: &mut Vec<Pending>) {
        for p in self.roster.iter().filter(|p| p.computer) {
            let Some(kingdom) = p.kingdom else { continue };
            let Some(k) = world.kingdoms.iter().find(|k| k.id == kingdom) else {
                continue;
            };
            let Some(slot) = slot_of(world, kingdom) else {
                continue;
            };
            // The SAI's name is its profile's, which already says "AI" in the
            // shipped data.
            let name = if k.name.ends_with(" AI") {
                utf16(&k.name)
            } else {
                utf16(&format!("{} AI", k.name))
            };
            out.push(Pending {
                player: self.host,
                decree: WireDecree {
                    descriptor: id::UPDATE_KINGDOM_NAME,
                    fields: vec![
                        Field::Text(utf16(&slot.id)),
                        Field::Text(name.clone()),
                        Field::Text(name),
                        Field::Uint { width: 1, value: 0 },
                    ],
                },
                commands: Vec::new(),
            });
        }
    }

    /// 0x74B310 and the stream's close (leaving states 2 and 5: 0x770500).
    pub fn close(&mut self) -> std::io::Result<Option<PathBuf>> {
        match self.file.take() {
            Some(file) => file.close().map(Some),
            None => Ok(None),
        }
    }
}

/// Leaving the state closes the film (0x770500), whichever way it is left.
impl Drop for Recorder {
    fn drop(&mut self) {
        if !self.stats.unmapped.is_empty() {
            eprintln!("film: commands the film could not carry: {:?}", self.stats.unmapped);
        }
        if let Err(e) = self.close() {
            eprintln!("film close: {e}");
        }
    }
}

fn set_player(player: u32) -> WireDecree {
    WireDecree {
        descriptor: id::SET_PLAYER,
        fields: vec![Field::Uint {
            width: 32,
            value: player,
        }],
    }
}

/// The units an order selects, for the group behaviour of 0x7AF220.
fn command_units(command: &Command) -> Vec<Unit> {
    match command {
        Command::Move { units, .. }
        | Command::Attack { units, .. }
        | Command::Stop { units, .. }
        | Command::Disband { units, .. }
        | Command::Follow { units, .. }
        | Command::Bombard { units, .. }
        | Command::Repair { units, .. }
        | Command::Construct { units, .. }
        | Command::Order { units, .. }
        | Command::Hot { units, .. } => units.clone(),
        Command::SetFormation { orgs, .. } => orgs.iter().map(|o| Unit::Org(*o)).collect(),
        _ => Vec::new(),
    }
}

/// The kingdom a command is for.
fn command_kingdom(command: &Command) -> Option<KingdomId> {
    match command {
        Command::Move { kingdom, .. }
        | Command::Attack { kingdom, .. }
        | Command::Stop { kingdom, .. }
        | Command::Recruit { kingdom, .. }
        | Command::RecruitWith { kingdom, .. }
        | Command::CancelRecruit { kingdom, .. }
        | Command::Deploy { kingdom, .. }
        | Command::Pack { kingdom, .. }
        | Command::Research { kingdom, .. }
        | Command::Attach { kingdom, .. }
        | Command::SetFormation { kingdom, .. }
        | Command::SetRallyPoint { kingdom, .. }
        | Command::Sell { kingdom, .. }
        | Command::Detach { kingdom, .. }
        | Command::Disband { kingdom, .. }
        | Command::UseAbility { kingdom, .. }
        | Command::Follow { kingdom, .. }
        | Command::Bombard { kingdom, .. }
        | Command::Repair { kingdom, .. }
        | Command::Construct { kingdom, .. }
        | Command::AirAttack { kingdom, .. }
        | Command::AirRecon { kingdom, .. }
        | Command::Order { kingdom, .. }
        | Command::Select { kingdom, .. }
        | Command::Hot { kingdom, .. } => Some(*kingdom),
    }
}

/// `group_behavior_enable` (37), the first order 0x7AF220 gives a group: 0x4D9E20
/// with 0x25, then 0x4D9B10(1), the boolean: flags 0x240, every computer's in the
/// real films.
fn group_behavior_enable() -> Vec<Field> {
    vec![
        Field::Bounded { n: 0x800, value: 0x240 },
        Field::Def {
            registry: crate::registries::ACTOR_COMMANDS,
            index: 37,
        },
        Field::Uint { width: 1, value: 1 },
        Field::Uint { width: 1, value: 0 },
    ]
}

/// What one request of `player_orders::native` is in a film's mirror stream.
///
/// The request forms say 85 KKC_TellActorCommand for an order to a single
/// building, which the films never hold: there every order is a selection and 86.
fn film_decrees(request: usize, fields: Vec<Field>, computer: bool, units: usize, outbox: &mut Outbox) -> Vec<WireDecree> {
    let decree_of = |request: usize| {
        aa_orders::flow::shape(request).map_or(request, |s| s.decree as usize)
    };
    let select = |slot: u16| -> Vec<WireDecree> {
        let thing = vec![Field::Thing {
            check: ThingCheck::B0,
            slot,
        }];
        if computer {
            vec![WireDecree {
                descriptor: id::SELECTION_SET,
                fields: thing,
            }]
        } else {
            vec![
                WireDecree {
                    descriptor: id::SELECTION_CLEAR,
                    fields: Vec::new(),
                },
                WireDecree {
                    descriptor: id::SELECTION_ADD,
                    fields: thing,
                },
            ]
        }
    };
    let decree = decree_of(request);
    match decree {
        id::TELL_ACTOR_COMMAND => {
            let mut fields = fields;
            let slot = match fields.pop() {
                Some(Field::Thing { slot, .. }) => slot,
                _ => 0,
            };
            // The film's selection is now this one thing; a human's stays as it
            // is when it already is (the request forms track it in the outbox).
            let mut out = if !computer && outbox.selection == [slot] { Vec::new() } else { select(slot) };
            outbox.selection = vec![slot];
            out.push(WireDecree {
                descriptor: id::TELL_SELECTED_COMMAND,
                fields,
            });
            out
        }
        id::SELECTION_SET => {
            let slot = match fields.first() {
                Some(Field::Thing { slot, .. }) => *slot,
                _ => 0,
            };
            select(slot)
        }
        id::TELL_SELECTED_COMMAND if computer && units > 1 => vec![
            WireDecree {
                descriptor: id::TELL_SELECTED_COMMAND,
                fields: group_behavior_enable(),
            },
            WireDecree {
                descriptor: decree,
                fields,
            },
        ],
        _ => vec![WireDecree {
            descriptor: decree,
            fields,
        }],
    }
}

/// The aa-play unit a selected slot stands for, for reports.
pub fn unit_slot(world: &World, unit: Unit) -> Option<u16> {
    wire::slot_of(world, unit)
}
