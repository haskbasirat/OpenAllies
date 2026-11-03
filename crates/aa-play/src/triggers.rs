//! The map's scripted triggers: the World+0x13C TriggerManager of the
//! original, run by the exact aa-triggers port over aa-play's world. Owner:
//! scenarios.
//!
//! What the original does, and where this follows it:
//! - map load: MAPSTRINGS (47EA40) registers the map text, TriggerManager
//!   54D3D0 reads TRIGGERS and 49AD70 binds every localized parameter
//!   through it; OBJECTIVES (4FCB3B / 502FC0) fills the World+250 objective
//!   definitions; SUN (4EDBD0) the World+4C Sun; every named map thing is
//!   published in the World+F0 name table (4F9510). [`attach`] does all of
//!   this from the original RMP; [`tick`] attaches on its first call when the
//!   install is found (`AA_GAME_ROOT`, else the folders above the program).
//! - every world update 0x4F9970 runs the manager after the four periodic /
//!   deferred queues (+1AC, +1C0, +1D4, +1E8) and phase 4, before phase 5
//!   and the +208 queue (`mov ecx,[esi+13C]; call 54C280` at 0x4F9AC2):
//!   `sim.rs` calls [`tick`] right before `scheduler::think`.
//! - 54C280 evaluates the enabled triggers in list order (5411D0: events,
//!   conditions by mode, actions, repeat / disable), resets the subject list
//!   after each, then drains the event queue (4DB300). Every element is the
//!   546AF0 port (`aa_triggers::world_program::Program`); the engine services
//!   behind it are [`Host`]'s trait methods below, each naming the native
//!   function it stands for.
//! - the queued events come from the engine: actor death 4D57C0 -> 54BE10
//!   (victim, last attacker), capture 4D5230 -> 54BEA0, the Area update
//!   4DB430 -> 54BF30 (queue 0, every tick), construction 4D6F10 / 565B00 ->
//!   54BFC0 / 54C050, recruitment 582190 / 582E80 -> 54C0E0, and the
//!   TriggerScript order 61E420 -> 54C170 ([`script_heard`]).
//! - the Sun's deferred +18 update (4EDDC0) runs on World+1E8 one tick after
//!   each schedule; [`tick`] delivers it before the manager runs, as the
//!   deferred queue runs before the manager in the native update.
//!
//! The presentation side (banners, speech, camera commands, objective and
//! mission game events, letterbox, fade, counter, sun colours) is exposed
//! through [`notices`] and the getters for the client (ui/objectives.rs).
//!
//! Compare against the original (behaviour-faithful seams, not exact):
//! - the Area member iteration (558720/558650/558750 spatial query) walks the
//!   living actors and organizations in ascending GTID order, each tested with
//!   the exact 46EF30 overlap (aa_spatial::shape_overlap) of its blocking shape
//!   (4EA0F0; an organization's leader point) and the area's; the areas update
//!   right before the manager instead of at their queue-0 delivery;
//! - the actor states of `actor_has_state` (word +0x100 and on, names 0x477FC0): the
//!   ones aa-play can answer are read from its orders, combat and supply state
//!   ([`Host::state_bits`]), the others stay off;
//! - 4D7960 (object_destroyed) brings the body to zero health with its damage record
//!   standing (`combat::script_kill`); the death, and its Destroyed event with the
//!   record's attacker, follow in the next tick's combat pass;
//! - 554B30 placement is the ring search of 554850 with the occupancy test of
//!   `special_ops::spot_open` (553BA0's query, not the spatial index port);
//! - transported creation (543BA0): the transport (ActorData+184
//!   transport_actor) has no simulation actor: it is a flight of distance /
//!   speed_max from the spawn point (Notice::Transport for the view), and the
//!   cargo is created at the unload point on arrival;
//! - actor commands from triggers (4D9B60) map to aa-play's commands (move,
//!   stop, kill, bombard, air_attack, unpack, guard, retreat, explore, rout);
//! - camera pitch / yaw (4DBD80 / 4DBD40 table lookups) use f32 asin/atan2
//!   (within 0.15 deg of the original's log);
//! - a seam failure stops the native-exact manager; the update is then
//!   rebuilt from the manager's state with the failing trigger disabled, and
//!   the failure is kept in `notes` (the original never fails there).
//!
//! Refused (an error, or a note, never a success) because aa-play has no
//! counterpart. No shipped map reaches the errors (tests/triggers.rs, the
//! census): the hero table (26, 35, 48, 49), the fidget variable (41), the
//! teleports (33, 37), the fog tile pass (106). Abilities used by a trigger
//! (96: 4C9F10) run as `special_ops::trigger_use` when a general's data holds
//! the ability; a team command flare (77, 78: 572E50) is a Notice for the view
//! and its SAI priority for the computer team mates (`ai::team_flare`); the
//! flare's actor itself (4FA030) has no simulation counterpart.
//!
//! The readers: `sai_disabled` / `unit_sai_disabled` (crate::ai),
//! `default_victory` (crate::victory), `listening` / `script_heard` (the
//! client's UI commands, ui/objectives.rs), `set_difficulty` (the session's
//! launch), `objectives` (the objectives display, ui/panels.rs).
use crate::data::DefClass;
use crate::world::{ActorId, Command, Event, KingdomId, Order, OrgId, Unit, World};
use aa_numeric::Precision;
use aa_triggers::{
    actions::{AbilityRequest, CameraFields, Command as TriggerCommand, SpawnRequest},
    decode::{Localization, SavedManager, Trigger},
    elements::Bank,
    manager::{DeferredPayload, Error, Manager, Result},
    mapstrings::MapStrings,
    objectives::{Entry, ObjectiveDef},
    predicates::Event as Payload,
    program::CoreProgram,
    sun::Sun,
    world_program::{Presentation, Program, Transition},
};
use aa_navigation::control;
use aa_navigation::startup::Object;
use aa_triggers::selectors::Registry;
use glam::Vec2;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The simulation thread's x87 precision (PC53, CW 0x027F).
const PC: Precision = Precision::Double53;
/// Decoding budget of one RMP chunk (an input guard, not a game limit).
const CHUNK_BUDGET: u64 = 1 << 28;

// ---------------------------------------------------------------------------
// State

/// A presentation effect of a trigger for the client, in the order the
/// original produced them. `World::triggers.notices` accumulates; a client
/// remembers how many it has seen.
#[derive(Clone, Debug, PartialEq)]
pub enum Notice {
    /// 42D4B0 game event `code` (enum 0x4083F0: 0x2A..0x31 objectives, 0x34
    /// kingdom_defeated, 0x35 kingdom_victory, 0x37 team_defeated, 0x38
    /// team_victory) for `kingdom` with its text argument.
    GameEvent { code: u32, kingdom: KingdomId, text: String },
    /// 50CD00: the UI command `DisplayBanner "<text>" <seconds>`.
    Banner { text: String, seconds: f32 },
    /// 50D120: a UI script command (MoveCamera, PlayVideo, CameraFollow,
    /// ShakeCamera), the exact text the original builds.
    UiCommand(String),
    /// 7476E0: speech (audio IDS).
    Speech(String),
    /// 7474F0: a sound, 2-D or at a native point.
    Sound { ids: String, at: Option<[f32; 3]> },
    /// 747630: game music (None stops it).
    Music(Option<String>),
    /// 50C880: flash a UI button (widget name).
    FlashButton(String),
    /// 50C3E0: flash an actor.
    FlashActor(Unit),
    /// 50D280: a VFX at a native point with its orientation.
    Vfx { ids: String, at: [f32; 3], orientation: [f32; 3] },
    /// 50D230: a VFX on an actor.
    VfxOnActor { ids: String, actor: Unit },
    /// 40BC50 (all) / 40BF70 (one kingdom): a chat line.
    Chat { kingdom: Option<KingdomId>, text: String },
    /// 4CE730: an actor plays an animation of its set.
    Animation { actor: Unit, name: String },
    /// 50D370: the local kingdom's mission ended (UI event type 2).
    MissionEnded { kingdom: KingdomId },
    /// 6FE710 / 700A90: the camera is stored before / restored after a
    /// cutscene.
    CameraStore,
    CameraRestore,
    /// 4F1080: cutscene mode on / off.
    CutsceneMode(bool),
    /// 76F370: the game speed exponent (binary32 of 5420D0).
    GameSpeed(f32),
    /// World+290: the weather (WeatherData IDS, None = clear).
    Weather(Option<String>),
    /// 572E50: a team command flare.
    Flare { kingdom: KingdomId, ids: String, at: [f32; 3], actor: Option<Unit>, seconds: f32 },
    /// 4C9F10: an ability used by a kingdom from / at a point.
    Ability { kingdom: KingdomId, ids: String, from: [f32; 3], at: [f32; 3] },
    /// 79F5D0: a kingdom's SAI profile changed.
    SaiProfile { kingdom: KingdomId, ids: String },
    /// 543BA0: a transport (`ids`, ActorData+184) flies from a point to the
    /// unload point in `seconds`; the cargo appears on arrival.
    Transport { kingdom: KingdomId, ids: String, from: [f32; 3], to: [f32; 2], seconds: f32 },
}

/// A transported creation on its way (543BA0, see the module documentation).
#[derive(Clone, Debug)]
struct Drop {
    due: f32,
    kingdom: KingdomId,
    cargo: crate::data::DefId,
    name: Vec<u16>,
    at: Vec2,
}

/// One Area (map thing class 6): its shape and the Area+84 member list.
#[derive(Clone, Debug)]
struct Area {
    placed: usize,
    /// Saved Updatable id (its queue-0 entry).
    updatable: Option<i32>,
    position: Vec2,
    facing: Vec2,
    /// Shape68: POINT 0 / CIRCLE 1 / SQUARE 2 / RECT 3, then five words.
    shape: [u32; 6],
    /// +84 list head first: (member GTID, present this update).
    members: Vec<(u32, bool)>,
}

/// One Camera (map thing class 7).
#[derive(Clone, Debug)]
struct Camera {
    placed: usize,
    position: [u32; 3],
    facing: [f32; 3],
    /// +60 zoom.
    zoom: u32,
}

/// A Sun definition (Data/Sun, GameData+73C): diffuse, ambient, specular.
#[derive(Clone, Debug)]
struct SunData {
    ids: String,
    colors: [[u32; 3]; 3],
}

/// Everything the trigger services own beside the manager.
struct Env {
    strings: MapStrings,
    objectives: Vec<ObjectiveDef>,
    /// Kingdom+300 lists.
    lists: BTreeMap<KingdomId, Vec<Entry>>,
    /// Kingdom+19C mission result (0 none, 1 won, 2 lost).
    results: BTreeMap<KingdomId, u32>,
    /// Kingdom+238 default-victory byte (constructor 4E5370: 1).
    default_victory: BTreeMap<KingdomId, u8>,
    /// World+48 (2 once a mission result ended the game).
    world_48: u32,
    presentation: Presentation,
    sun: Sun,
    suns: Vec<SunData>,
    weather: Option<String>,
    /// World+27C counter {start, duration}.
    counter: Option<(u32, u32)>,
    cutscene: bool,
    /// World+294 listened UI script names, newest first.
    listens: Vec<Vec<u16>>,
    /// World+F0 name table: upper-case key -> GTIDs, newest first.
    names: BTreeMap<Vec<u16>, Vec<u32>>,
    /// Object+8 words the trigger services set (0x20 SAI disabled, 0x40, 4).
    flags: BTreeMap<u64, u32>,
    areas: Vec<Area>,
    cameras: Vec<Camera>,
    /// Interned bank names (audio, commands, formations...).
    interned: Vec<String>,
    /// The local player's selection per kingdom (Player+50), from the client.
    selections: BTreeMap<KingdomId, Vec<Unit>>,
    /// The last game speed a trigger set (76F370).
    game_speed: Option<u32>,
    notices: Vec<Notice>,
    notes: Vec<String>,
    next_payload: u64,
    sort_objectives: bool,
    team_victory: Option<bool>,
    /// The kingdom and transport of the transported creation 543BA0 in
    /// progress (its spawn request reaches create_transport, not transport_load).
    pending_kingdom: Option<KingdomId>,
    pending_transport: Option<(crate::data::DefId, [f32; 3])>,
    /// ActorData+184 by lower-case actor IDS (the rules catalog, read on the
    /// first transported creation).
    transports: Option<BTreeMap<String, String>>,
    /// The transported creations on their way.
    drops: Vec<Drop>,
    /// The ability of the element 96 in progress (4C8C20 .. 4C9F10).
    pending_ability: Option<String>,
    /// What was already noted as not modelled (once per kind).
    not_modelled: Vec<String>,
    /// AVars.tgi CameraZoomDefault (binary32).
    camera_zoom_default: u32,
    /// *(GameVars+104): the mission difficulty (index into `difficulties`)
    /// when the session set it.
    difficulty: Option<i32>,
    /// The DifficultyLevel data bank (GameData+140, Game/difficulty_level.tgi
    /// order: easy, medium, hard).
    difficulties: Vec<String>,
    /// The DifficultyLevel world rule's default (Game/world_rules_aa.tgi,
    /// MEDIUM; 511FF0 reads the enum through that bank).
    default_difficulty: Option<i32>,
}

/// A loaded scenario: the manager, its 546AF0 evaluator state and the rest.
struct Scenario {
    manager: Manager,
    core: CoreProgram,
    env: Env,
    map_id: String,
}

/// `World::triggers`.
#[derive(Default)]
pub struct TriggerState {
    scenario: Option<Box<Scenario>>,
    /// The map was looked for (found or not).
    probed: bool,
    /// [`set_difficulty`] before the map loaded.
    difficulty: Option<i32>,
    /// [`set_rule_sun`] before the map loaded.
    rule_sun: Option<String>,
    /// `World::events` the manager queue has taken this tick ([`tick`] and
    /// [`late_events`]).
    events_queued: usize,
    /// Load problems and seam failures (the original has none).
    pub notes: Vec<String>,
}

impl std::fmt::Debug for TriggerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TriggerState").field("loaded", &self.scenario.is_some()).field("notes", &self.notes).finish()
    }
}

// ---------------------------------------------------------------------------
// Object handles

mod tag {
    pub const ACTOR: u64 = 1;
    pub const ORG: u64 = 2;
    pub const KINGDOM: u64 = 3;
    pub const PLACED: u64 = 4;
    pub const TEAM: u64 = 5;
    pub const MAP_KINGDOM: u64 = 6;
    pub const TRAILER: u64 = 7;
    pub const DATA: u64 = 8;
    pub const PROPERTY: u64 = 9;
    pub const AUDIO: u64 = 10;
    pub const COMMAND: u64 = 11;
    pub const COMMAND_ORDINAL: u64 = 12;
    pub const WEATHER: u64 = 13;
    pub const SUN: u64 = 14;
    pub const FORMATION: u64 = 15;
    pub const SAI: u64 = 16;
    pub const ABILITY: u64 = 17;
    pub const PLAYER: u64 = 18;
    pub const SELECTION: u64 = 19;
    pub const UI_ROOT: u64 = 20;
    pub const WIDGET: u64 = 21;
    pub const ANIMATION_SET: u64 = 22;
    pub const ANIMATION: u64 = 23;
    pub const RNG: u64 = 24;
    pub const TRANSPORT: u64 = 25;
    pub const VEHICLE: u64 = 26;
    pub const GROUP: u64 = 27;
    pub const COMPONENT: u64 = 28;
}
fn obj(t: u64, id: u64) -> Object {
    Object(t << 40 | id)
}
fn split(o: Object) -> (u64, u64) {
    (o.0 >> 40, o.0 & 0xff_ffff_ffff)
}
/// Property handles in aa-body's 32-bit identity domain.
const BODY_PROPERTY: u32 = 0xc000_0000;

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn text(v: &[u16]) -> String {
    String::from_utf16_lossy(&v[..v.iter().position(|&c| c == 0).unwrap_or(v.len())])
}
fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn state(e: impl std::fmt::Display) -> Error {
    Error::State(e.to_string())
}
/// 4F7B70 name key: ASCII upper case; other characters are refused.
fn name_key(name: &[u16]) -> Option<Vec<u16>> {
    if name.iter().any(|&c| c > 127 || c == 0) {
        return None;
    }
    Some(name.iter().map(|&c| if (97..=122).contains(&c) { c - 32 } else { c }).collect())
}

// ---------------------------------------------------------------------------
// Loading

/// The install root: `AA_GAME_ROOT`, else the first folder above the program
/// or the working directory that holds the extracted original maps.
fn find_root() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("AA_GAME_ROOT") {
        return Some(PathBuf::from(p));
    }
    let maps = Path::new("reconstruction").join("assets").join("original").join("Data").join("Maps");
    let mut starts = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        starts.push(exe);
    }
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    starts.iter().flat_map(|s| s.ancestors()).find(|a| a.join(&maps).is_dir()).map(Path::to_path_buf)
}

/// The original RMP of `map_id` (`Campaign/...` ids included; a bare id is
/// also looked for in the Campaign, TrailerMaps and DeveloperMaps folders).
fn read_rmp(root: &Path, map_id: &str) -> Option<Vec<u8>> {
    let maps = root.join("reconstruction").join("assets").join("original").join("Data").join("Maps");
    let mut dirs = vec![maps.clone()];
    if !map_id.contains('/') && !map_id.contains('\\') {
        for sub in ["Campaign", "TrailerMaps", "DeveloperMaps"] {
            dirs.push(maps.join(sub));
        }
    }
    for d in &dirs {
        for ext in ["RMP", "rmp"] {
            if let Ok(b) = std::fs::read(d.join(format!("{map_id}.{ext}"))) {
                return Some(b);
            }
        }
    }
    let mut sources = crate::data::Sources::open(root).ok()?;
    for ext in ["RMP", "rmp"] {
        if let Ok(file) = sources.files.read(&format!("Maps/{map_id}.{ext}")) {
            return Some(file.bytes);
        }
    }
    None
}

/// An application variable of Data/<file> (`<type> <name> = <value>`, the
/// comment after `;` dropped).
fn avar(root: &Path, file: &str, name: &str) -> Option<String> {
    let path = root.join("reconstruction").join("assets").join("original").join("Data").join(file);
    let s = std::fs::read_to_string(path).ok()?;
    s.lines().find_map(|l| {
        let l = l.split(';').next()?.trim();
        let mut words = l.splitn(2, char::is_whitespace);
        words.next()?;
        let rest = words.next()?.trim_start();
        let v = rest.strip_prefix(name)?.trim_start().strip_prefix('=')?;
        Some(v.trim().trim_matches('"').to_string())
    })
}

/// A data file, the v14 overlay first.
fn data_file(root: &Path, relative: &str) -> Option<Vec<u8>> {
    let overlay = root.join("reconstruction").join("native-game").join("runtime-expanded-v14").join("Data");
    let original = root.join("reconstruction").join("assets").join("original").join("Data");
    std::fs::read(overlay.join(relative)).or_else(|_| std::fs::read(original.join(relative))).ok()
}

/// The `ids` of every `[DifficultyLevel]` of Game/difficulty_level.tgi in file
/// order (the bank's index is entry +C, 48D500) and the index of the
/// DifficultyLevel world rule's `default`.
fn load_difficulties(root: &Path, notes: &mut Vec<String>) -> (Vec<String>, Option<i32>) {
    fn nodes(bytes: Vec<u8>, header: &str) -> Vec<BTreeMap<String, String>> {
        let Ok(doc) = aa_rules::tgi::Document::parse_native(bytes, aa_rules::tgi::Limits::default()) else { return Vec::new() };
        let mut out = Vec::new();
        for item in &doc.items {
            let aa_rules::tgi::Item::Node(node) = item else { continue };
            if !doc.header_name(&node.header).is_some_and(|s| s.eq_ignore_ascii_case(header)) {
                continue;
            }
            let mut fields = BTreeMap::new();
            for item in &node.items {
                let aa_rules::tgi::Item::Property(p) = item else { continue };
                if let (Some(name), Some(raw)) = (doc.property_name(p), doc.property_value(p)) {
                    fields.insert(name.to_ascii_lowercase(), raw.trim().trim_matches('"').to_string());
                }
            }
            out.push(fields);
        }
        out
    }
    let levels: Vec<String> = data_file(root, "Game/difficulty_level.tgi").map(|b| nodes(b, "DifficultyLevel")).unwrap_or_default().iter().filter_map(|n| n.get("ids").cloned()).collect();
    if levels.is_empty() {
        notes.push("Game/difficulty_level.tgi gives no difficulty levels".into());
    }
    let rule = data_file(root, "Game/world_rules_aa.tgi").map(|b| nodes(b, "WorldRule")).unwrap_or_default();
    let default = rule
        .iter()
        .find(|n| n.get("ids").is_some_and(|i| i.eq_ignore_ascii_case("DifficultyLevel")))
        .and_then(|n| n.get("default"))
        .and_then(|d| levels.iter().position(|l| l.eq_ignore_ascii_case(d)))
        .map(|i| i as i32);
    if default.is_none() {
        notes.push("the DifficultyLevel world rule has no default level".into());
    }
    (levels, default)
}

/// AVars_Locale.tgi `LanguageIDS` (English when absent).
fn language(root: &Path) -> String {
    avar(root, "AVars_Locale.tgi", "LanguageIDS").unwrap_or_else(|| "English".to_string())
}

/// AVars.tgi `CameraZoomDefault` through the native scalar reader.
fn camera_zoom_default(root: &Path) -> u32 {
    avar(root, "AVars.tgi", "CameraZoomDefault").and_then(|v| aa_rules::native::scalar_literal(&v, PC).ok()).unwrap_or(0x3f80_0000)
}

/// The Sun DataManager (GameData+73C): Data/Sun/*.tgi, the v14 overlay
/// first, each [Sun] node's IDS and diffuse / ambient / specular point3
/// through the native scalar literal reader.
fn load_suns(root: &Path, notes: &mut Vec<String>) -> Vec<SunData> {
    let mut out: Vec<SunData> = Vec::new();
    let dirs = [
        root.join("reconstruction").join("native-game").join("runtime-expanded-v14").join("Data").join("Sun"),
        root.join("reconstruction").join("assets").join("original").join("Data").join("Sun"),
    ];
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        let mut files: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("tgi"))).collect();
        files.sort();
        for path in files {
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let Ok(doc) = aa_rules::tgi::Document::parse_native(bytes, aa_rules::tgi::Limits::default()) else {
                notes.push(format!("{} does not parse", path.display()));
                continue;
            };
            for item in &doc.items {
                let aa_rules::tgi::Item::Node(node) = item else { continue };
                if !doc.header_name(&node.header).is_some_and(|s| s.eq_ignore_ascii_case("Sun")) {
                    continue;
                }
                let mut ids = None;
                let mut colors = [[0u32; 3]; 3];
                for item in &node.items {
                    let aa_rules::tgi::Item::Property(p) = item else { continue };
                    let (Some(name), Some(raw)) = (doc.property_name(p), doc.property_value(p)) else { continue };
                    let slot = match name.to_ascii_lowercase().as_str() {
                        "ids" => {
                            ids = Some(raw.trim().trim_matches('"').to_string());
                            continue;
                        }
                        "diffuse" => 0,
                        "ambient" => 1,
                        "specular" => 2,
                        _ => continue,
                    };
                    for (i, part) in raw.split(',').take(3).enumerate() {
                        match aa_rules::native::scalar_literal(part.trim(), PC) {
                            Ok(bits) => colors[slot][i] = bits,
                            Err(e) => notes.push(format!("{}: {name} {e}", path.display())),
                        }
                    }
                }
                if let Some(ids) = ids
                    && !out.iter().any(|s| s.ids.eq_ignore_ascii_case(&ids))
                {
                    out.push(SunData { ids, colors });
                }
            }
        }
    }
    out
}

/// ActorData+184 for every thing: the rules catalog's `transport_actor` by
/// lower-case IDS (read only for a map with a transported creation element).
fn transport_table(root: &Path, data: &crate::data::GameData) -> Option<BTreeMap<String, String>> {
    let mut sources = crate::data::Sources::open(root).ok()?;
    let catalog = aa_rules::actors::ActorCatalog::load_v14(&mut sources.files).ok()?;
    let mut out = BTreeMap::new();
    for d in &data.defs {
        let Ok(a) = catalog.resolved_actor(d.id.as_str()) else { continue };
        let raw = crate::data::loader::st(&a, "transport_actor");
        if let Some(raw) = raw.filter(|r| !r.is_empty() && !r.eq_ignore_ascii_case("null")) {
            out.insert(d.id.to_ascii_lowercase(), raw);
        }
    }
    Some(out)
}

fn word(b: &[u8], at: &mut usize) -> Option<u32> {
    let v = u32::from_le_bytes(b.get(*at..*at + 4)?.try_into().ok()?);
    *at += 4;
    Some(v)
}
fn skip_string(b: &[u8], at: &mut usize) -> Option<()> {
    let n = u16::from_le_bytes(b.get(*at..*at + 2)?.try_into().ok()?) as usize;
    *at += 2 + 2 * n;
    (*at <= b.len()).then_some(())
}

/// The Area special data (aa-mapworld `map_objects::Area::load` layout):
/// [world >= 22: Updatable version, saved id], Area version, label, Shape68
/// six words, then (version >= 1) the +84 entries (saved GTID, byte),
/// prepended as read.
/// (saved Updatable id, Shape68 words, +84 entries) of an Area record.
type AreaRecord = (Option<i32>, [u32; 6], Vec<(u32, bool)>);

fn decode_area(tail: &[u8], world_version: u32) -> Option<AreaRecord> {
    let mut at = 0;
    let mut updatable = None;
    if world_version >= 22 && word(tail, &mut at)? != 0 {
        let id = word(tail, &mut at)? as i32;
        updatable = (id >= 0).then_some(id);
    }
    let version = word(tail, &mut at)?;
    skip_string(tail, &mut at)?;
    let mut shape = [0u32; 6];
    for w in &mut shape {
        *w = word(tail, &mut at)?;
    }
    let mut members = Vec::new();
    if version >= 1 {
        let count = word(tail, &mut at)? as i32;
        for _ in 0..count.max(0) {
            let id = word(tail, &mut at)?;
            let byte = *tail.get(at)?;
            at += 1;
            members.insert(0, (id, byte != 0));
        }
    }
    Some((updatable, shape, members))
}

/// The Camera special data: version, label (v1: localization key), +60.
fn decode_camera(tail: &[u8]) -> Option<u32> {
    let mut at = 0;
    word(tail, &mut at)?;
    skip_string(tail, &mut at)?;
    word(tail, &mut at)
}

/// Load the map's triggers for `world` from the install at `root`. Returns
/// whether the map has a trigger manager. A map without TRIGGERS (a
/// generated map) loads nothing.
pub fn attach(world: &mut World, root: &Path) -> bool {
    world.triggers.probed = true;
    let Some(bytes) = read_rmp(root, &world.map.id) else {
        return false;
    };
    attach_bytes(world, bytes, root)
}

/// [`attach`] from RMP bytes already read (tests, tools).
pub fn attach_bytes(world: &mut World, rmp: Vec<u8>, root: &Path) -> bool {
    world.triggers.probed = true;
    let language = language(root);
    let mut notes = Vec::new();
    let suns = load_suns(root, &mut notes);
    let difficulties = load_difficulties(root, &mut notes);
    load(world, rmp, &language, suns, difficulties, camera_zoom_default(root), notes, Some(root))
}

#[allow(clippy::too_many_arguments)]
fn load(world: &mut World, rmp: Vec<u8>, language: &str, suns: Vec<SunData>, difficulties: (Vec<String>, Option<i32>), camera_zoom_default: u32, mut notes: Vec<String>, root: Option<&Path>) -> bool {
    let archive = match aa_mapworld::MapArchive::from_bytes(rmp) {
        Ok(a) => a,
        Err(e) => {
            world.triggers.notes.push(format!("{}: RMP refused: {e}", world.map.id));
            return false;
        }
    };
    let chunk = |name: &str| archive.decode_chunk(name, CHUNK_BUDGET).ok().map(|c| c.bytes);
    let Some(triggers) = chunk("TRIGGERS") else {
        return false;
    };
    let mapstrings = chunk("MAPSTRINGS").unwrap_or_default();
    // 47EA40 then 54D3D0 then 49AD70.
    let strings = match MapStrings::parse(&mapstrings, language) {
        Ok(s) => s,
        Err(e) => {
            world.triggers.notes.push(format!("{}: MAPSTRINGS: {e}", world.map.id));
            MapStrings::default()
        }
    };
    let mut saved = match SavedManager::decode(&triggers, Localization::External, triggers.len().max(1 << 20)) {
        Ok(s) => s,
        Err(e) => {
            world.triggers.notes.push(format!("{}: TRIGGERS: {e}", world.map.id));
            return false;
        }
    };
    if let Err(e) = strings.bind(&mut saved) {
        notes.push(format!("MAPSTRINGS binding: {e}"));
    }
    // 4FCB3B: OBJECTIVES (world version >= 6), display text through the map
    // text first.
    let objectives = match chunk("OBJECTIVES") {
        // 4B9F40 falls back to the global text tables for a key the map text
        // lacks: the key is kept as "#key" for the client's string table.
        Some(b) => match aa_triggers::objectives::load_chunk(&b, |key| {
            Ok(strings.resolve(key).unwrap_or_else(|_| {
                let mut k = utf16("#");
                k.extend_from_slice(key);
                k
            }))
        }) {
            Ok((defs, _)) => defs,
            Err(e) => {
                notes.push(format!("OBJECTIVES: {e}"));
                Vec::new()
            }
        },
        None => Vec::new(),
    };
    // 4EE0A0 then 4EDBD0.
    let mut sun = Sun::new();
    if let Some(b) = chunk("SUN")
        && let Err(e) = sun.load(&b)
    {
        notes.push(format!("SUN: {e}"));
    }
    // DYNAMIC_THINGS: thing names (4F9510), Areas and Cameras.
    let mut env_names: BTreeMap<Vec<u16>, Vec<u32>> = BTreeMap::new();
    let mut areas = Vec::new();
    let mut cameras = Vec::new();
    match aa_mapworld::WorldInventory::decode(&archive, CHUNK_BUDGET) {
        Ok(inv) => {
            let version = inv.world.version;
            for (index, p) in inv.dynamic_things.placements.iter().enumerate() {
                if let Some(key) = name_key(&p.thing.name_utf16).filter(|k| !k.is_empty()) {
                    let rows = env_names.entry(key).or_default();
                    // publish: the first same-name entry is replaced, the new one prepended
                    if !rows.is_empty() {
                        rows.remove(0);
                    }
                    rows.insert(0, p.serialized_id);
                }
                let position = p.thing.position_bits;
                let facing = p.thing.facing_bits.map(f32::from_bits);
                match p.class_code {
                    6 => match decode_area(&p.opaque_tail, version) {
                        Some((updatable, shape, members)) => areas.push(Area {
                            placed: index,
                            updatable,
                            position: Vec2::new(f(position[0]), f(position[1])),
                            facing: Vec2::new(facing[0], facing[1]),
                            shape,
                            members,
                        }),
                        None => notes.push(format!("area record {index} does not decode")),
                    },
                    7 => match decode_camera(&p.opaque_tail) {
                        Some(zoom) => cameras.push(Camera { placed: index, position, facing, zoom }),
                        None => notes.push(format!("camera record {index} does not decode")),
                    },
                    _ => {}
                }
            }
        }
        Err(e) => notes.push(format!("DYNAMIC_THINGS: {e}")),
    }
    // The kingdoms' names (KINGDOMS records, the map slot ids).
    for (s, slot) in world.map.slots.iter().enumerate() {
        let raw = (0..world.kingdoms.len() as KingdomId).find(|&k| world.ids.kingdom_slot(k) == Some(s)).and_then(|k| world.ids.kingdom(k)).map(|g| g.raw()).or(slot.serialized_id);
        if let (Some(key), Some(raw)) = (name_key(&utf16(&slot.id)).filter(|k| !k.is_empty()), raw) {
            env_names.insert(key, vec![raw]);
        }
    }
    let sort_objectives = world.data.rule_ci("SortObjectives").is_none_or(|v| v != 0.0);
    let team_victory = Some(world.data.rule_ci("TeamVictory").is_none_or(|v| v != 0.0));
    let now = world.clock.begin.0;
    let env = Env {
        strings,
        objectives,
        lists: BTreeMap::new(),
        results: BTreeMap::new(),
        default_victory: BTreeMap::new(),
        world_48: 0,
        // World+BC..+D0 after the World constructor: nothing shown.
        presentation: Presentation {
            letterbox: Transition { enabled: 0, started_bits: 0, duration_bits: 0 },
            screenfade: Transition { enabled: 0, started_bits: 0, duration_bits: 0 },
        },
        sun,
        suns,
        weather: (!world.map.weather.is_empty()).then(|| world.map.weather.clone()),
        counter: None,
        cutscene: false,
        listens: Vec::new(),
        names: env_names,
        flags: BTreeMap::new(),
        areas,
        cameras,
        interned: Vec::new(),
        selections: BTreeMap::new(),
        game_speed: None,
        notices: Vec::new(),
        notes: Vec::new(),
        next_payload: 1,
        sort_objectives,
        team_victory,
        pending_kingdom: None,
        pending_transport: None,
        transports: None,
        drops: Vec::new(),
        pending_ability: None,
        not_modelled: Vec::new(),
        camera_zoom_default,
        difficulty: world.triggers.difficulty,
        difficulties: difficulties.0,
        default_difficulty: difficulties.1,
    };
    let manager = Manager::from_saved(saved);
    let id = world.map.id.clone();
    let mut env = env;
    if manager.definitions().iter().any(|t| t.actions.iter().any(|e| e.opcode == 30)) {
        env.transports = root.and_then(|r| transport_table(r, &world.data));
    }
    world.triggers.notes.extend(notes.into_iter().map(|n| format!("{id}: {n}")));
    world.triggers.scenario = Some(Box::new(Scenario { manager, core: CoreProgram::new(now, 0, PC), env, map_id: id }));
    true
}

// ---------------------------------------------------------------------------
// The tick

/// Step 4/5 of 0x4F9970: the trigger manager update of this tick (see the
/// module documentation for the order).
pub fn tick(world: &mut World) {
    if !world.triggers.probed {
        world.triggers.probed = true;
        // A generated world has no map file and so no TRIGGERS chunk (54D3D0 runs
        // in the map load 0x4FC600): looking for one would only open the whole
        // archive for nothing, 1.5 s on the battle's first tick.
        if world.map.generated.is_none() {
            match find_root() {
                Some(root) => {
                    attach(world, &root);
                }
                None => world.triggers.notes.push("the install was not found: the map's triggers do not run".into()),
            }
        }
        if let Some(ids) = world.triggers.rule_sun.take() {
            apply_rule_sun(world, &ids);
        }
    }
    let Some(mut sc) = world.triggers.scenario.take() else { return };
    deliver_drops(world, &mut sc);
    queue_engine_events(world, &mut sc, 0);
    world.triggers.events_queued = world.events.len();
    areas_update(world, &mut sc);
    // World+1E8: the Sun's deferred update scheduled on an earlier tick.
    if sc.env.sun.scheduled
        && let Err(e) = sc.env.sun.deliver(world.clock.begin.0, PC)
    {
        sc.env.notes.push(format!("sun update: {e}"));
    }
    run_manager(world, &mut sc);
    let notes = std::mem::take(&mut sc.env.notes);
    world.triggers.notes.extend(notes.into_iter().map(|n| format!("{}: {n}", sc.map_id)));
    world.triggers.scenario = Some(sc);
}

/// The events of the rest of the update (phase 5 and the +208 queue, 0x4F9970):
/// natively each goes into the manager's queue when it happens, for the
/// manager's next update. `sim.rs` calls this at the end of the update.
pub fn late_events(world: &mut World) {
    let Some(mut sc) = world.triggers.scenario.take() else { return };
    let from = world.triggers.events_queued;
    queue_engine_events(world, &mut sc, from);
    world.triggers.events_queued = 0;
    world.triggers.scenario = Some(sc);
}

/// A transported creation's arrival (58BA90's unload): the cargo at the first
/// free spot around the point (554B30 within 8 m, facing (0, -1)), its name
/// published.
fn deliver_drops(world: &mut World, sc: &mut Scenario) {
    if sc.env.drops.is_empty() {
        return;
    }
    let now = f(world.clock.begin.0);
    let due: Vec<Drop> = {
        let mut out = Vec::new();
        sc.env.drops.retain(|d| {
            let ready = d.due <= now;
            if ready {
                out.push(d.clone());
            }
            !ready
        });
        out
    };
    let mut host = Host { world, env: &mut sc.env };
    for d in due {
        let at = host.open_spot(d.at, Vec2::new(0.0, -1.0), Some(d.cargo), 8.0).unwrap_or(d.at);
        host.create(d.cargo, d.kingdom, at, Vec2::new(0.0, -1.0), &d.name);
    }
}

fn run_manager(world: &mut World, sc: &mut Scenario) {
    let now = world.clock.begin.0;
    let core = std::mem::replace(&mut sc.core, CoreProgram::new(now, 0, PC));
    let mut host = Host { world, env: &mut sc.env };
    let mut program = Program { core, world: &mut host };
    let result = sc.manager.update(&mut program).map(|_| ());
    sc.core = program.core;
    // Diagnostics are the original's log lines; keep them bounded.
    if sc.core.diagnostics.len() > 256 {
        sc.core.diagnostics.drain(..128);
    }
    if let Err(e) = result {
        let failed = sc.manager.trace().evaluated.last().copied();
        let name = failed
            .and_then(|id| sc.manager.definitions().iter().find(|t| t.id == id))
            .map(|t| text(&t.name))
            .unwrap_or_default();
        sc.env.notes.push(format!("trigger {failed:?} '{name}' failed at {:.3} s: {e}; it is disabled", f(now)));
        rebuild(sc, failed);
    }
}

/// After a failed update: a manager with the live trigger states and
/// variables, the failing trigger disabled (see the module documentation).
fn rebuild(sc: &mut Scenario, failed: Option<u32>) {
    let snapshot = sc.manager.snapshot();
    let triggers: Vec<Trigger> = sc
        .manager
        .definitions()
        .iter()
        .zip(&snapshot.triggers)
        .map(|(d, s)| {
            let mut t = d.clone();
            t.mode = s.mode;
            t.enabled = if Some(s.id) == failed { 0 } else { s.enabled };
            t.repeat = s.repeat;
            t.activation_bits = s.activation_bits;
            t
        })
        .collect();
    sc.manager = Manager::from_saved(SavedManager { next_id: snapshot.next_id, triggers, variables: snapshot.variables, strings: snapshot.strings });
}

fn push(sc: &mut Scenario, data: Payload) {
    let identity = sc.env.next_payload;
    sc.env.next_payload += 1;
    sc.manager.push_event(Some(DeferredPayload { identity, data }));
}

/// The engine's trigger events of this tick, in the order they happened
/// (each prepended to manager+50, newest first).
fn queue_engine_events(world: &World, sc: &mut Scenario, from: usize) {
    let gtid_actor = |a: ActorId| world.ids.actor(a).map_or(0, |g| g.raw());
    let gtid_org = |o: OrgId| world.ids.org(o).map_or(0, |g| g.raw());
    let gtid_kingdom = |k: KingdomId| world.ids.kingdom(k).map_or(0, |g| g.raw());
    let mut out = Vec::new();
    for e in world.events.get(from..).unwrap_or(&[]) {
        match e {
            // 4D57C0 -> 54BE10 / 54B7B0: victim, then the body's (or the
            // organization's +90) last damage record (attacker GTID).
            Event::Died { actor } => {
                let killer = crate::combat::credited(world, world.combat.actor(*actor).and_then(|c| c.last_hit)).map_or(0, gtid_actor);
                out.push(Payload::Destroyed { victim_id: gtid_actor(*actor), killer_id: killer, word_c: 0 });
            }
            Event::OrgDestroyed { org } => {
                let killer = crate::combat::credited(world, world.combat.org(*org).and_then(|c| c.last_hit)).map_or(0, gtid_actor);
                out.push(Payload::Destroyed { victim_id: gtid_org(*org), killer_id: killer, word_c: 0 });
            }
            // 4D5230 -> 54BEA0 / 54B820: the captured actor, the capturing
            // source (aa-play reports the kingdom) and its kingdom.
            Event::Captured { actor, kingdom } => {
                let k = gtid_kingdom(*kingdom);
                out.push(Payload::Captured { actor_id: gtid_actor(*actor), capturer_id: k, kingdom_id: k });
            }
            // 4D6F10 (created under construction) -> 54BFC0 flag 1;
            // 565B00 OnConstructionFinished -> 54C050 flag 0.
            Event::ConstructionStarted { actor, kingdom } | Event::ConstructionFinished { actor, kingdom } => {
                let def = world.actors.get(*actor as usize).map(|a| a.def);
                out.push(Payload::Construction {
                    kingdom_id: gtid_kingdom(*kingdom),
                    definition: def.map_or(0, |d| obj(tag::DATA, u64::from(d)).0),
                    flag_10: u8::from(matches!(e, Event::ConstructionStarted { .. })),
                });
            }
            // The end of the unpacking (0x57634D, 54C050 with the kingdom and the definition,
            // not by 565B00): the "finishes construction" of the tutorials' trucks.
            Event::Deployed { building } => {
                let a = world.actors.get(*building as usize);
                if let Some(kingdom) = a.and_then(|a| a.kingdom) {
                    out.push(Payload::Construction {
                        kingdom_id: gtid_kingdom(kingdom),
                        definition: a.map_or(0, |a| obj(tag::DATA, u64::from(a.def)).0),
                        flag_10: 0,
                    });
                }
            }
            // 582190 / 582E80 -> 54C0E0 (flag 0): the recruited definition.
            Event::Recruited { kingdom, org, actor } => {
                let def = org.and_then(|o| world.orgs.get(o as usize)).map(|o| o.def).or_else(|| actor.and_then(|a| world.actors.get(a as usize)).map(|a| a.def));
                out.push(Payload::Recruitment { kingdom_id: gtid_kingdom(*kingdom), definition: def.map_or(0, |d| obj(tag::DATA, u64::from(d)).0), flag_10: 0 });
            }
            _ => {}
        }
    }
    for p in out {
        push(sc, p);
    }
}

/// A thing an area can hold: its position and blocking shape (4EA0F0): a
/// living actor (body) or an organization (its leader's point).
#[derive(Clone, Copy, Debug)]
struct Member {
    position: Vec2,
    facing: Vec2,
    /// Shape68: kind, radius, min x, min y, max x, max y (binary32 bits).
    shape: [u32; 6],
}

fn member_position(world: &World, gtid: u32) -> Option<Member> {
    match world.ids.thing(gtid)? {
        crate::ids::Thing::Actor(a) => {
            let x = world.actors.get(a as usize).filter(|x| x.alive && !x.removed)?;
            let d = world.data.def(x.def);
            let (sin, cos) = x.facing.sin_cos();
            let shape = match (d.blocking_shape.as_deref(), d.blocking_size) {
                (Some(s), Some(v)) if s.eq_ignore_ascii_case("CIRCLE") => [1, v[0].to_bits(), 0, 0, 0, 0],
                (Some(s), Some(v)) if s.eq_ignore_ascii_case("SQUARE") || s.eq_ignore_ascii_case("RECTANGLE") || s.eq_ignore_ascii_case("RECT") => {
                    let (hx, hy) = (v[0] * 0.5, v[1] * 0.5);
                    [3, hx.hypot(hy).to_bits(), (-hx).to_bits(), (-hy).to_bits(), hx.to_bits(), hy.to_bits()]
                }
                _ => [0, 0, 0, 0, 0, 0],
            };
            Some(Member { position: x.pos.truncate(), facing: Vec2::new(cos, sin), shape })
        }
        crate::ids::Thing::Org(o) => {
            world.orgs.get(o as usize).filter(|x| x.alive)?;
            Some(Member { position: crate::supply::org_position(world, o), facing: Vec2::new(0.0, -1.0), shape: [0, 0, 0, 0, 0, 0] })
        }
        _ => None,
    }
}

/// 46EF30 of the member's shape and the area's (aa_spatial::shape_overlap).
fn inside(area: &Area, m: &Member) -> bool {
    use aa_spatial::{collision::Shape, geometry::Placement};
    let placement = |shape: &[u32; 6], facing: Vec2, position: Vec2| Placement {
        shape: Shape { kind: shape[0], radius_bits: shape[1], bounds: [shape[2], shape[3], shape[4], shape[5]] },
        facing: [facing.x.to_bits(), facing.y.to_bits()],
        position: [position.x.to_bits(), position.y.to_bits()],
    };
    let a = placement(&area.shape, area.facing, area.position);
    let b = placement(&m.shape, m.facing, m.position);
    aa_spatial::shape_overlap::overlaps(&a, &b, PC).unwrap_or(false)
}

/// 4DB430 for every Area (queue 0, its saved delivery order; areas not on
/// the saved queue after them): a member found inside that is not on the
/// +84 list joins it and queues an AreaEntry (54BF30); listed members not
/// found leave the list.
fn areas_update(world: &World, sc: &mut Scenario) {
    if sc.env.areas.is_empty() {
        return;
    }
    let mut order: Vec<usize> = Vec::new();
    for bucket in world.sched.entries(crate::scheduler::EVERY_TICK) {
        for owner in bucket {
            if let crate::scheduler::Owner::Saved(id) = owner
                && let Some(i) = sc.env.areas.iter().position(|a| a.updatable == Some(id as i32))
                && !order.contains(&i)
            {
                order.push(i);
            }
        }
    }
    for i in 0..sc.env.areas.len() {
        if !order.contains(&i) {
            order.push(i);
        }
    }
    // Candidates in ascending GTID slot order (4EFBE0 enumeration).
    let mut things: Vec<(u32, Member)> = world.ids.iter().filter_map(|(g, _)| member_position(world, g.raw()).map(|m| (g.raw(), m))).collect();
    things.sort_by_key(|t| t.0 & 0xffff);
    for i in order {
        let area_obj = obj(tag::PLACED, sc.env.areas[i].placed as u64).0;
        let mut entered = Vec::new();
        {
            let area = &mut sc.env.areas[i];
            for (gtid, m) in &things {
                if !inside(area, m) {
                    continue;
                }
                if let Some(m) = area.members.iter_mut().find(|m| m.0 & 0xffff == gtid & 0xffff) {
                    m.1 = true;
                    continue;
                }
                area.members.insert(0, (*gtid, true));
                entered.push(*gtid);
            }
            area.members.retain(|m| m.1);
            for m in &mut area.members {
                m.1 = false;
            }
        }
        for gtid in entered {
            push(sc, Payload::AreaEntry { actor_id: gtid, area: area_obj });
        }
    }
}

// ---------------------------------------------------------------------------
// Public API for the client

/// The trigger notices so far (accumulating; remember how many were seen).
pub fn notices(world: &World) -> &[Notice] {
    world.triggers.scenario.as_ref().map_or(&[], |s| &s.env.notices)
}

/// Whether the map has a trigger manager running.
pub fn loaded(world: &World) -> bool {
    world.triggers.scenario.is_some()
}

/// One scripted objective of a kingdom (Kingdom+300 entry): its World+250
/// key and text, the +10 flag (flagged = primary / required) and its state
/// (0 incomplete, 1 complete, 2 failed).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectiveView {
    pub key: String,
    pub text: String,
    pub primary: bool,
    pub state: u32,
}

/// The kingdom's scripted objectives in list order (`0x633A50` reads them).
pub fn objectives(world: &World, kingdom: KingdomId) -> Vec<ObjectiveView> {
    let Some(sc) = &world.triggers.scenario else { return Vec::new() };
    sc.env
        .lists
        .get(&kingdom)
        .map(|l| {
            l.iter()
                .filter_map(|e| {
                    let d = sc.env.objectives.get(e.definition)?;
                    Some(ObjectiveView { key: text(&d.name), text: text(&d.text), primary: d.flag_10 != 0, state: e.state })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The World+4C Sun now: (diffuse, ambient, specular) colours and the dimmer.
pub fn sun(world: &World) -> Option<([[f32; 3]; 3], f32)> {
    world.triggers.scenario.as_ref().map(|s| s.env.sun.current())
}

/// `KKC_GWorld::StartMission` 0x4FDCE0: the Sun world rule away from its default
/// sets the World+4C Sun's colours: 0x48D500 finds the Sun by IDS, 0x4EDB10 with
/// no transition. A map without a trigger manager has none of this Sun here (the
/// client lights it from its own copy, `look::SceneSun`).
pub fn set_rule_sun(world: &mut World, ids: &str) {
    if world.triggers.probed {
        apply_rule_sun(world, ids);
    } else {
        world.triggers.rule_sun = Some(ids.to_owned());
    }
}

fn apply_rule_sun(world: &mut World, ids: &str) {
    let now = world.clock.begin.0;
    let Some(sc) = world.triggers.scenario.as_mut() else { return };
    if let Some(colors) = sc.env.suns.iter().find(|s| s.ids.eq_ignore_ascii_case(ids)).map(|s| s.colors) {
        sc.env.sun.set_colors(colors, 0, now);
    }
}

/// One presentation transition: (enabled, start, duration) in seconds of
/// world time.
pub type TransitionView = (bool, f32, f32);

/// World+BC..+D0: the letterbox and the screen fade (4ED890 / 4ED8B0 of
/// global_toggle_letterbox 43 / global_toggle_screenfade 44); the AVar
/// LetterboxHeightRatio (0.22222) sizes the bars.
pub fn presentation(world: &World) -> Option<(TransitionView, TransitionView)> {
    world.triggers.scenario.as_ref().map(|s| {
        let p = &s.env.presentation;
        let t = |x: &Transition| (x.enabled != 0, f(x.started_bits), f(x.duration_bits));
        (t(&p.letterbox), t(&p.screenfade))
    })
}

/// World+27C: the counter's start and duration (seconds) while shown.
pub fn counter(world: &World) -> Option<(f32, f32)> {
    world.triggers.scenario.as_ref()?.env.counter.map(|(s, d)| (f(s), f(d)))
}

/// The world is in cutscene mode (4F1080).
pub fn cutscene(world: &World) -> bool {
    world.triggers.scenario.as_ref().is_some_and(|s| s.env.cutscene)
}

/// World+290: the current weather (WeatherData IDS).
pub fn weather(world: &World) -> Option<String> {
    world.triggers.scenario.as_ref()?.env.weather.clone()
}

/// Kingdom+19C: the kingdom's mission result (0 none, 1 won, 2 lost): set by
/// the trigger path here, or by the default victory checks (`crate::victory`).
pub fn mission_result(world: &World, kingdom: KingdomId) -> u32 {
    let scripted = world.triggers.scenario.as_ref().and_then(|s| s.env.results.get(&kingdom).copied());
    scripted.unwrap_or_else(|| default_result(world, kingdom))
}

/// Kingdom+19C as `crate::victory` leaves it.
fn default_result(world: &World, kingdom: KingdomId) -> u32 {
    match world.kingdoms.get(kingdom as usize) {
        Some(k) if k.defeated => 2,
        Some(k) if !k.independent && world.winner == Some(k.team) => 1,
        _ => 0,
    }
}

/// Kingdom+238: the default victory conditions apply to the kingdom (the
/// constructor 4E5370 sets 1; kingdom_enable_default_victory_condition 114
/// changes it). 4E9960 tests it before the team elimination check.
pub fn default_victory(world: &World, kingdom: KingdomId) -> bool {
    world.triggers.scenario.as_ref().and_then(|s| s.env.default_victory.get(&kingdom).copied()).is_none_or(|v| v != 0)
}

/// 4E9720(result, notify 1) for a map with a trigger manager: Kingdom+19C,
/// the kingdom_victory / kingdom_defeated game events, the local mission end
/// and 4F4DA0's settlement of the other kingdoms. False when the map has no
/// trigger manager (the caller's own victory path applies).
pub fn set_mission_result(world: &mut World, kingdom: KingdomId, result: u32) -> bool {
    let Some(mut sc) = world.triggers.scenario.take() else { return false };
    let r = {
        let mut host = Host { world, env: &mut sc.env };
        aa_triggers::objectives::set_mission_result(&mut host, obj(tag::KINGDOM, u64::from(kingdom)), result, true)
    };
    if let Err(e) = r {
        sc.env.notes.push(format!("mission result: {e}"));
    }
    world.triggers.scenario = Some(sc);
    true
}

/// The kingdom's computer player is disabled by a trigger (Object+8 bit
/// 0x20, sai_disabled_for_kingdom 62 / 79E2F0); 61 enables it again.
pub fn sai_disabled(world: &World, kingdom: KingdomId) -> bool {
    world.triggers.scenario.as_ref().is_some_and(|s| s.env.flags.get(&obj(tag::KINGDOM, u64::from(kingdom)).0).is_some_and(|f| f & 0x20 != 0))
}

/// An organization or actor whose AI a trigger disabled (63 / 64).
pub fn unit_sai_disabled(world: &World, unit: Unit) -> bool {
    let o = match unit {
        Unit::Org(o) => obj(tag::ORG, u64::from(o)),
        Unit::Actor(a) => obj(tag::ACTOR, u64::from(a)),
    };
    world.triggers.scenario.as_ref().is_some_and(|s| s.env.flags.get(&o.0).is_some_and(|f| f & 0x20 != 0))
}

/// 4F3FC0 World::IsListening: the UI command `name` is on the World+294
/// list (`_wcsicmp`), so the UI sends the TriggerScript order for it.
pub fn listening(world: &World, name: &str) -> bool {
    let n = utf16(name);
    world.triggers.scenario.as_ref().is_some_and(|s| s.env.listens.iter().any(|l| aa_triggers::predicates::wcsicmp_eq(l, &n)))
}

/// The TriggerScript order 61E420 -> 54C170: a nonempty script name heard
/// by the trigger manager (consumed by event_script_heard 9 this tick).
pub fn script_heard(world: &mut World, name: &str) {
    if name.is_empty() {
        return;
    }
    if let Some(sc) = world.triggers.scenario.as_deref_mut() {
        push(sc, Payload::Script { text: utf16(name) });
    }
}

/// *(GameVars+104): the mission difficulty the session plays (0 easy, 1
/// medium, 2 hard; the campaign's choice), read by difficulty_compare 17.
pub fn set_difficulty(world: &mut World, difficulty: i32) {
    world.triggers.difficulty = Some(difficulty);
    if let Some(sc) = world.triggers.scenario.as_deref_mut() {
        sc.env.difficulty = Some(difficulty);
    }
}

/// [`set_difficulty`] by a DifficultyLevel IDS (the world rule's EnumValue,
/// any case; 48D500 on the bank of Game/difficulty_level.tgi). False when the
/// bank does not have it (nothing changes).
pub fn set_difficulty_ids(world: &mut World, ids: &str, root: &Path) -> bool {
    let mut notes = Vec::new();
    let (levels, _) = load_difficulties(root, &mut notes);
    match levels.iter().position(|l| l.eq_ignore_ascii_case(ids)) {
        Some(i) => {
            set_difficulty(world, i as i32);
            true
        }
        None => false,
    }
}

/// The local player's current selection (Player+50), for the selection
/// conditions 18 / 19.
pub fn set_selection(world: &mut World, kingdom: KingdomId, units: Vec<Unit>) {
    if let Some(sc) = world.triggers.scenario.as_deref_mut() {
        sc.env.selections.insert(kingdom, units);
    }
}

/// 4BEDF0: a kingdom's name in the script commands (its map slot id).
pub fn kingdom_script_name(world: &World, kingdom: KingdomId) -> Option<String> {
    world.ids.kingdom_slot(kingdom).and_then(|s| world.map.slots.get(s)).map(|s| s.id.clone())
}

/// A `MoveCamera` UI command of the camera elements 71 / 72 / 73
/// (544540 / 544780 build it), as KKC_VWorld::CommandMoveCamera 0x43DD10
/// reads it: the end time (world seconds, or the real-time clock in seconds
/// for 72), the real-time flag, the look-at point (native X/Y), the zoom, the
/// pitch and the yaw in degrees (4DBD40 asin of the camera facing's height /
/// 4DBD80 its heading; 73: the AVars CameraPitch 40 / CameraYaw -45), the
/// ease flag and the kingdom whose camera moves (0x43DD10 compares it with
/// the local kingdom's name and ignores other kingdoms' moves).
#[derive(Clone, Debug, PartialEq)]
pub struct CameraMove {
    pub until: f32,
    pub realtime: bool,
    pub at: Vec2,
    pub zoom: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub ease: bool,
    pub kingdom: String,
}

/// Parse a `MoveCamera %f %d %f %f %f %f %f %d %s` command.
pub fn parse_move_camera(command: &str) -> Option<CameraMove> {
    let mut it = command.strip_prefix("MoveCamera")?.split_whitespace();
    let mut num = || it.next()?.parse::<f32>().ok();
    let (until, realtime, x, y, zoom, pitch, yaw, ease) = (num()?, num()?, num()?, num()?, num()?, num()?, num()?, num()?);
    let kingdom = command.split_whitespace().nth(9).unwrap_or_default().to_string();
    Some(CameraMove { until, realtime: realtime != 0.0, at: Vec2::new(x, y), zoom, pitch, yaw, ease: ease != 0.0, kingdom })
}

/// 4B9F40 on the map's text table (MAPSTRINGS): the text of `key`.
pub fn map_text(world: &World, key: &str) -> Option<String> {
    let sc = world.triggers.scenario.as_ref()?;
    sc.env.strings.resolve(&utf16(key)).ok().map(|t| text(&t))
}

/// The trigger records of the map (for tools and tests): id, name, enabled.
pub fn trigger_states(world: &World) -> Vec<(u32, String, bool)> {
    let Some(sc) = &world.triggers.scenario else { return Vec::new() };
    let snap = sc.manager.snapshot();
    sc.manager.definitions().iter().zip(&snap.triggers).map(|(d, s)| (d.id, text(&d.name), s.enabled != 0)).collect()
}

/// A verification aid (the mission census): the actor changes hands, with the capture event
/// of the manager's queue (54BEA0).
pub fn aid_capture(world: &mut World, actor: ActorId, kingdom: KingdomId) {
    if let Some(a) = world.actors.get_mut(actor as usize) {
        a.kingdom = Some(kingdom);
    }
    let raw_actor = world.ids.actor(actor).map_or(0, |g| g.raw());
    let raw_kingdom = world.ids.kingdom(kingdom).map_or(0, |g| g.raw());
    if let Some(sc) = world.triggers.scenario.as_deref_mut() {
        push(sc, Payload::Captured { actor_id: raw_actor, capturer_id: raw_kingdom, kingdom_id: raw_kingdom });
    }
}

/// One element a trigger waits on, as the map's data holds it.
#[derive(Clone, Debug)]
pub struct ArmedElement {
    pub name: String,
    pub parameters: Vec<String>,
}

/// An enabled trigger and what it waits on (its events and conditions), for following a
/// mission from a test or a tool: the parameters read as in the map's data (a word in
/// hexadecimal, a name as text).
#[derive(Clone, Debug)]
pub struct ArmedTrigger {
    pub name: String,
    pub waits: Vec<ArmedElement>,
}

pub fn armed(world: &World) -> Vec<ArmedTrigger> {
    use aa_triggers::decode::Value;
    let Some(sc) = &world.triggers.scenario else { return Vec::new() };
    let snap = sc.manager.snapshot();
    let parameter = |v: &Value| match v {
        Value::Text(t) => String::from_utf16_lossy(t),
        Value::Localized { key, .. } => format!("#{}", String::from_utf16_lossy(key)),
        Value::Word(w) => format!("{w:#x}"),
        Value::Position(p) => format!("{:?}", p.map(f32::from_bits)),
        Value::Empty => String::new(),
    };
    sc.manager
        .definitions()
        .iter()
        .zip(&snap.triggers)
        .filter(|(_, s)| s.enabled != 0)
        .map(|(d, _)| ArmedTrigger {
            name: text(&d.name),
            waits: d.events.iter().chain(&d.conditions).map(|e| ArmedElement { name: e.name.clone(), parameters: e.parameters.iter().map(|p| parameter(&p.value)).collect() }).collect(),
        })
        .collect()
}

/// What a script's parameter names (0xB364C0 for a word, the World+F0 name table for a text).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScriptThing {
    Thing(crate::ids::Thing),
    /// An Area of the map (map thing class 6), at its position.
    Area(Vec2),
}

pub fn script_thing(world: &World, parameter: &str) -> Option<ScriptThing> {
    let sc = world.triggers.scenario.as_deref()?;
    let raw = match parameter.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => *sc.env.names.get(&name_key(&utf16(parameter))?)?.first()?,
    };
    let thing = world.ids.thing(raw)?;
    if let crate::ids::Thing::Placed(i) = thing
        && let Some(a) = sc.env.areas.iter().find(|a| a.placed == i)
    {
        return Some(ScriptThing::Area(a.position));
    }
    Some(ScriptThing::Thing(thing))
}

// ---------------------------------------------------------------------------
// The engine services

/// The 546AF0 services over aa-play's world.
struct Host<'a> {
    world: &'a mut World,
    env: &'a mut Env,
}

impl Host<'_> {
    fn intern(&mut self, s: &str) -> u64 {
        if let Some(i) = self.env.interned.iter().position(|x| x == s) {
            return i as u64;
        }
        self.env.interned.push(s.to_string());
        (self.env.interned.len() - 1) as u64
    }
    fn interned(&self, id: u64) -> String {
        self.env.interned.get(id as usize).cloned().unwrap_or_default()
    }
    fn thing_object(t: crate::ids::Thing) -> Object {
        use crate::ids::Thing;
        match t {
            Thing::Actor(a) => obj(tag::ACTOR, u64::from(a)),
            Thing::Org(o) => obj(tag::ORG, u64::from(o)),
            Thing::Kingdom(k) => obj(tag::KINGDOM, u64::from(k)),
            Thing::MapKingdom(s) => obj(tag::MAP_KINGDOM, s as u64),
            Thing::Team(t) => obj(tag::TEAM, t as u64),
            Thing::Placed(i) => obj(tag::PLACED, i as u64),
            Thing::Trailer(a) => obj(tag::TRAILER, u64::from(a)),
            // A GGroup (orders): no trigger names one, this index resolves to nothing.
            Thing::Group(g) => obj(tag::PLACED, u64::from(g) | 1 << 40),
            // A projectile: no trigger names one, likewise.
            Thing::Transient(d) => obj(tag::PLACED, u64::from(d) | 1 << 41),
        }
    }
    fn kingdom_of(&self, o: Object) -> Option<KingdomId> {
        match split(o) {
            (tag::KINGDOM, k) => Some(k as KingdomId),
            _ => None,
        }
    }
    fn unit(&self, o: Object) -> Option<Unit> {
        match split(o) {
            (tag::ACTOR, a) | (tag::TRAILER, a) => Some(Unit::Actor(a as ActorId)),
            (tag::ORG, g) => Some(Unit::Org(g as OrgId)),
            _ => None,
        }
    }
    /// The unit a command for `o` goes to (4D9B60 routes a member's command
    /// to its organization).
    fn command_unit(&self, o: Object) -> Option<Unit> {
        match self.unit(o)? {
            Unit::Actor(a) => Some(match self.world.actors.get(a as usize).and_then(|x| x.org) {
                Some(g) => Unit::Org(g),
                None => Unit::Actor(a),
            }),
            u => Some(u),
        }
    }
    fn owner_of(&self, o: Object) -> Option<KingdomId> {
        match self.unit(o)? {
            Unit::Actor(a) => self.world.actors.get(a as usize)?.kingdom,
            Unit::Org(g) => Some(self.world.orgs.get(g as usize)?.kingdom),
        }
    }
    fn alive(&self, o: Object) -> bool {
        match split(o) {
            (tag::ACTOR, a) | (tag::TRAILER, a) => self.world.actors.get(a as usize).is_some_and(|x| x.alive && !x.removed),
            (tag::ORG, g) => self.world.orgs.get(g as usize).is_some_and(|x| x.alive),
            _ => true,
        }
    }
    fn position(&self, o: Object) -> Option<glam::Vec3> {
        match split(o) {
            (tag::ACTOR, a) | (tag::TRAILER, a) => self.world.actors.get(a as usize).map(|x| x.pos),
            (tag::ORG, g) => {
                let p = crate::supply::org_position(self.world, g as OrgId);
                Some(self.world.ground(p))
            }
            (tag::PLACED, i) => self.world.map.placed.get(i as usize).map(|p| glam::Vec3::from(p.position)),
            _ => None,
        }
    }
    fn def_of(&self, o: Object) -> Option<crate::data::DefId> {
        match split(o) {
            (tag::ACTOR, a) | (tag::TRAILER, a) => self.world.actors.get(a as usize).map(|x| x.def),
            (tag::ORG, g) => self.world.orgs.get(g as usize).map(|x| x.def),
            (tag::PLACED, i) => self.world.map.placed.get(i as usize).and_then(|p| self.world.data.find(&p.thing_id)),
            _ => None,
        }
    }
    fn notice(&mut self, n: Notice) {
        self.env.notices.push(n);
    }
    /// An element reached a service aa-play has no counterpart for: the
    /// presentation notice goes out, the effect is missing, and `notes` says
    /// so once per kind.
    fn not_modelled(&mut self, what: String) {
        if !self.env.not_modelled.contains(&what) {
            self.env.notes.push(format!("{what} is not modelled by aa-play"));
            self.env.not_modelled.push(what);
        }
    }
    fn local(&self) -> Option<KingdomId> {
        self.world.kingdoms.iter().find(|k| k.controller == crate::world::Controller::Human).map(|k| k.id)
    }
    fn team_victory(&self) -> bool {
        self.env.team_victory.unwrap_or(true)
    }
    /// Kingdom+19C.
    fn result_of(&self, k: KingdomId) -> u32 {
        self.env.results.get(&k).copied().unwrap_or_else(|| default_result(self.world, k))
    }
    /// Kingdom+1EC team (None for independents) and its +44 member list.
    fn team_members(&self, k: KingdomId) -> Option<Vec<KingdomId>> {
        let kd = self.world.kingdoms.get(k as usize)?;
        if kd.independent {
            return None;
        }
        Some(self.world.kingdoms.iter().filter(|x| !x.independent && x.team == kd.team).map(|x| x.id).collect())
    }
    /// Apply an aa-play command now (4D9B60 processes immediately).
    fn issue(&mut self, c: Command) -> bool {
        let before = self.world.events.len();
        self.world.apply(c);
        !self.world.events[before..].iter().any(|e| matches!(e, Event::Refused { .. }))
    }
    fn members_of(&self, k: KingdomId) -> Vec<Object> {
        // Kingdom+284: the kingdom's organizations and its actors outside one.
        let mut out: Vec<Object> = self.world.orgs.iter().filter(|o| o.alive && o.kingdom == k).map(|o| obj(tag::ORG, u64::from(o.id))).collect();
        out.extend(self.world.actors.iter().filter(|a| a.alive && !a.removed && a.org.is_none() && a.kingdom == Some(k)).map(|a| obj(tag::ACTOR, u64::from(a.id))));
        out
    }
    /// Set Kingdom+19C and the aa-play battle state it stands for.
    fn record_result(&mut self, k: KingdomId, result: u32) {
        self.env.results.insert(k, result);
        if result == 2
            && let Some(kd) = self.world.kingdoms.get_mut(k as usize)
        {
            kd.defeated = true;
        }
    }
    /// 4E9720(result, notify) of the objectives port for another kingdom
    /// (4F4DA0 settles the others with notify 0).
    fn settle(&mut self, k: KingdomId, result: u32) -> Result<()> {
        aa_triggers::objectives::set_mission_result(self, obj(tag::KINGDOM, u64::from(k)), result, false)
    }
    /// The placement search 554B30 (554850: the point itself, else rings k =
    /// 1..=rings at k * step along the direction, turned eight times by the
    /// native -pi/4 table; the ring count trunc(distance / radius) clamped to
    /// 1..=20 with the radius the blocking radius, at least a quarter cell).
    fn open_spot(&self, p: Vec2, facing: Vec2, def: Option<crate::data::DefId>, radius: f32) -> Option<Vec2> {
        let Some(def) = def else { return Some(p) };
        crate::special_ops::open_position_along(self.world, def, p, facing, radius)
    }
    /// 4FA030 / 4FA150 on aa-play: the actor or organization at `p`, the
    /// name published (4F9510), its GTID.
    fn create(&mut self, def: crate::data::DefId, kingdom: KingdomId, p: Vec2, facing: Vec2, name: &[u16]) -> Option<Object> {
        let heading = if facing.length_squared() > 0.0 { facing.y.atan2(facing.x) } else { -std::f32::consts::FRAC_PI_2 };
        let d = self.world.data.def(def);
        let made = if d.class == Some(DefClass::Organization) || !d.slots.is_empty() {
            let o = crate::production::spawn_organization(self.world, def, kingdom, p, heading, None);
            obj(tag::ORG, u64::from(o))
        } else {
            let a = crate::production::spawn_single(self.world, def, Some(kingdom), p, heading);
            obj(tag::ACTOR, u64::from(a))
        };
        if let Some(key) = name_key(name).filter(|k| !k.is_empty())
            && let Ok(raw) = aa_triggers::selectors::Registry::raw_id(self, made)
        {
            let rows = self.env.names.entry(key).or_default();
            if !rows.is_empty() {
                rows.remove(0);
            }
            rows.insert(0, raw);
        }
        Some(made)
    }
}

fn not_bound(what: &str) -> Error {
    Error::State(format!("{what} is not modelled by aa-play"))
}

impl aa_triggers::selectors::Registry for Host<'_> {
    /// 0xB364C0 generation lookup.
    fn resolve_id(&self, raw: u32) -> Result<Option<Object>> {
        if raw == 0 {
            return Ok(None);
        }
        Ok(self.world.ids.thing(raw).map(Self::thing_object))
    }
    /// Object+14.
    fn raw_id(&self, object: Object) -> Result<u32> {
        let ids = &self.world.ids;
        let raw = match split(object) {
            (tag::ACTOR, a) => ids.actor(a as ActorId).map(|g| g.raw()),
            (tag::ORG, g) => ids.org(g as OrgId).map(|g| g.raw()),
            (tag::KINGDOM, k) => ids.kingdom(k as KingdomId).map(|g| g.raw()),
            (tag::PLACED, i) => ids.placed(i as usize).map(|g| g.raw()),
            (tag::TRAILER, a) => ids.trailer(a as ActorId).map(|g| g.raw()),
            (tag::TEAM, t) => self.world.map.team_ids.get(t as usize).copied(),
            (tag::MAP_KINGDOM, s) => self.world.map.slots.get(s as usize).and_then(|s| s.serialized_id),
            _ => None,
        };
        Ok(raw.unwrap_or(0))
    }
    /// 4F8E90: the first entry of the name, its live id, the actor class.
    fn named_actor(&mut self, name: &[u16]) -> Result<Option<Object>> {
        let Some(key) = name_key(name) else { return Err(state("name needs a native locale rule")) };
        let Some(&raw) = self.env.names.get(&key).and_then(|r| r.first()) else { return Ok(None) };
        let o = self.resolve_id(raw)?;
        Ok(o.filter(|o| match split(*o) {
            (tag::ACTOR | tag::ORG | tag::TRAILER, _) => true,
            (tag::PLACED, i) => self.world.map.placed.get(i as usize).and_then(|p| self.world.data.find(&p.thing_id)).is_some_and(|d| self.world.data.def(d).actor_data),
            _ => false,
        }))
    }
    /// 4F8F00: the same with the Kingdom class check.
    fn named_kingdom(&mut self, name: &[u16]) -> Result<Option<Object>> {
        let Some(key) = name_key(name) else { return Err(state("name needs a native locale rule")) };
        let Some(&raw) = self.env.names.get(&key).and_then(|r| r.first()) else { return Ok(None) };
        Ok(self.resolve_id(raw)?.filter(|o| matches!(split(*o).0, tag::KINGDOM | tag::MAP_KINGDOM)))
    }
}

impl aa_triggers::selectors::Checks for Host<'_> {
    fn is_area(&mut self, object: Object) -> Result<bool> {
        Ok(matches!(split(object), (tag::PLACED, i) if self.env.areas.iter().any(|a| a.placed as u64 == i)))
    }
    fn is_camera(&mut self, object: Object) -> Result<bool> {
        Ok(matches!(split(object), (tag::PLACED, i) if self.env.cameras.iter().any(|c| c.placed as u64 == i)))
    }
}

impl aa_triggers::objectives::Services for Host<'_> {
    fn objective_definitions(&self) -> Result<&[ObjectiveDef]> {
        Ok(&self.env.objectives)
    }
    fn kingdom_objectives_mut(&mut self, kingdom: Object) -> Result<&mut Vec<Entry>> {
        let k = self.kingdom_of(kingdom).ok_or_else(|| state("objective owner is not a kingdom"))?;
        Ok(self.env.lists.entry(k).or_default())
    }
    /// 4BE150 on the SortObjectives game variable.
    fn sort_objectives(&mut self) -> Result<bool> {
        Ok(self.env.sort_objectives)
    }
    /// 42D4B0 on the game event bus.
    fn post_message(&mut self, code: u32, kingdom: Object, t: &[u16]) -> Result<()> {
        if let Some(k) = self.kingdom_of(kingdom) {
            self.notice(Notice::GameEvent { code, kingdom: k, text: text(t) });
        }
        Ok(())
    }
}

impl aa_triggers::objectives::MissionServices for Host<'_> {
    fn mission_result(&self, kingdom: Object) -> Result<u32> {
        Ok(self.kingdom_of(kingdom).map_or(0, |k| self.result_of(k)))
    }
    fn set_mission_result(&mut self, kingdom: Object, result: u32) -> Result<()> {
        if let Some(k) = self.kingdom_of(kingdom) {
            self.record_result(k, result);
        }
        Ok(())
    }
    /// 405370 (TeamVictory).
    fn game_var_byte(&mut self, name: &[u16]) -> Result<Option<u8>> {
        let n = text(name);
        if n.eq_ignore_ascii_case("TeamVictory") {
            return Ok(Some(u8::from(self.team_victory())));
        }
        Ok(self.world.data.rule_ci(&n).map(|v| u8::from(v != 0.0)))
    }
    fn team_member_count(&self, kingdom: Object) -> Result<Option<usize>> {
        Ok(self.kingdom_of(kingdom).and_then(|k| self.team_members(k)).map(|m| m.len()))
    }
    fn world_48(&self) -> Result<u32> {
        Ok(self.env.world_48)
    }
    /// Kingdom+239: 0 after the constructor (4E5370).
    fn kingdom_flag_239(&self, _kingdom: Object) -> Result<u8> {
        Ok(0)
    }
    /// 4E9560(kingdom, 0): the kingdom's display name.
    fn kingdom_result_text(&mut self, kingdom: Object, _flag: u32) -> Result<Vec<u16>> {
        Ok(self.kingdom_of(kingdom).and_then(|k| self.world.kingdoms.get(k as usize)).map(|k| utf16(&k.name)).unwrap_or_default())
    }
    fn post_mission_message(&mut self, code: u32, kingdom: Object, t: &[u16]) -> Result<()> {
        aa_triggers::objectives::Services::post_message(self, code, kingdom, t)
    }
    /// 4F5290.
    fn local_kingdom(&mut self) -> Result<Option<Object>> {
        Ok(self.local().map(|k| obj(tag::KINGDOM, u64::from(k))))
    }
    /// 50D370: the UI's mission-ended event.
    fn ui_mission_ended(&mut self) -> Result<()> {
        if let Some(k) = self.local() {
            self.notice(Notice::MissionEnded { kingdom: k });
        }
        Ok(())
    }
    /// 4F4DA0 World mission-result processing.
    fn world_mission_result(&mut self, kingdom: Object) -> Result<()> {
        let Some(k) = self.kingdom_of(kingdom) else { return Ok(()) };
        if self.world.kingdoms.get(k as usize).is_none_or(|x| x.independent) {
            return Ok(());
        }
        let tv = self.team_victory();
        let result = self.result_of(k);
        // World+154: the kingdoms (independents never win or lose).
        let all: Vec<KingdomId> = self.world.kingdoms.iter().filter(|x| !x.independent).map(|x| x.id).collect();
        let team = |h: &Self, x: KingdomId| h.world.kingdoms[x as usize].team;
        // Team virtual +24: the team's name (the map's TEAMS id).
        let team_name = |h: &Self, x: KingdomId| {
            let t = h.world.kingdoms[x as usize].team;
            h.world.map.teams.get(t as usize).cloned().unwrap_or_else(|| format!("team_{t}"))
        };
        if result == 1 {
            self.env.world_48 = 2;
            if tv && self.team_members(k).is_some_and(|m| m.len() > 1) {
                let name = team_name(self, k);
                self.notice(Notice::GameEvent { code: 0x38, kingdom: k, text: name });
            }
            for x in all {
                if x != k && self.result_of(x) == 0 {
                    let r = if tv && team(self, x) == team(self, k) { 1 } else { 2 };
                    self.settle(x, r)?;
                }
            }
            let t = team(self, k);
            self.world.winner = Some(t);
            self.world.events.push(Event::Victory { team: t });
        } else if result == 2 {
            let mut survivor: Option<KingdomId> = None;
            for &x in &all {
                if self.result_of(x) == 2 {
                    continue;
                }
                match survivor {
                    Some(s) if team(self, s) != team(self, x) => {
                        // Two sides remain; 4E27A0: the lost kingdom's whole
                        // team (two or more) lost -> team_defeated (0x37).
                        if tv && self.team_members(k).is_some_and(|m| m.len() >= 2 && m.iter().all(|y| self.result_of(*y) == 2)) {
                            let name = team_name(self, k);
                            self.notice(Notice::GameEvent { code: 0x37, kingdom: k, text: name });
                        }
                        return Ok(());
                    }
                    Some(_) => {}
                    None => survivor = Some(x),
                }
            }
            if let Some(s) = survivor {
                self.settle(s, 1)?;
                if let Some(m) = self.team_members(s) {
                    if tv && m.len() > 1 {
                        let name = team_name(self, s);
                        self.notice(Notice::GameEvent { code: 0x38, kingdom: s, text: name });
                    }
                    // Team +44: every member wins.
                    for y in m {
                        self.settle(y, 1)?;
                    }
                }
                let t = team(self, s);
                self.world.winner = Some(t);
                self.world.events.push(Event::Victory { team: t });
            } else {
                self.world.winner = Some(u32::MAX);
            }
            self.env.world_48 = 2;
        }
        Ok(())
    }
}

impl aa_triggers::elements::Services for Host<'_> {
    /// KKC_DataManager GetDataByIDSWarn.
    fn data_by_ids(&mut self, bank: Bank, name: &[u16]) -> Result<Option<Object>> {
        let n = text(name);
        if n.is_empty() {
            return Ok(None);
        }
        Ok(match bank {
            Bank::ActorData => self.world.data.find(&n).map(|d| obj(tag::DATA, u64::from(d))),
            Bank::Property => self.world.data.property(&n).map(|p| p.id.to_ascii_lowercase()).map(|id| obj(tag::PROPERTY, self.intern(&id))),
            Bank::Sun => self.env.suns.iter().position(|s| s.ids.eq_ignore_ascii_case(&n)).map(|i| obj(tag::SUN, i as u64)),
            Bank::Audio => Some(obj(tag::AUDIO, self.intern(&n))),
            Bank::Command => Some(obj(tag::COMMAND, self.intern(&n.to_ascii_lowercase()))),
            Bank::Weather => Some(obj(tag::WEATHER, self.intern(&n))),
            Bank::Formation => Some(obj(tag::FORMATION, self.intern(&n))),
            Bank::Sai => Some(obj(tag::SAI, self.intern(&n))),
            Bank::Ability => Some(obj(tag::ABILITY, self.intern(&n))),
        })
    }
    /// Object+8: bit 0 dead / destroyed, and the trigger-set bits.
    fn flags8(&self, object: Object) -> Result<u32> {
        let stored = self.env.flags.get(&object.0).copied().unwrap_or(0);
        Ok(stored | u32::from(!self.alive(object)))
    }
    /// Actor virtual +108.
    fn owner(&mut self, actor: Object) -> Result<Option<Object>> {
        Ok(self.owner_of(actor).map(|k| obj(tag::KINGDOM, u64::from(k))))
    }
    /// Actor virtual +160: the actor (an organization with its members)
    /// changes hands.
    fn transfer_control(&mut self, actor: Object, kingdom: Object) -> Result<()> {
        let Some(k) = self.kingdom_of(kingdom) else { return Ok(()) };
        match self.unit(actor) {
            Some(Unit::Org(g)) => {
                let members = self.world.orgs.get(g as usize).map(|o| o.members.clone()).unwrap_or_default();
                if let Some(o) = self.world.orgs.get_mut(g as usize) {
                    o.kingdom = k;
                    o.hq = None;
                }
                for m in members {
                    if let Some(a) = self.world.actors.get_mut(m as usize) {
                        a.kingdom = Some(k);
                    }
                }
            }
            Some(Unit::Actor(a)) => {
                if let Some(x) = self.world.actors.get_mut(a as usize) {
                    x.kingdom = Some(k);
                }
            }
            None => {}
        }
        Ok(())
    }
    /// 4E64C0.
    fn kingdom_default_lose(&mut self, kingdom: Object) -> Result<bool> {
        let Some(k) = self.kingdom_of(kingdom) else { return Ok(false) };
        let counted = crate::victory::counted(self.world);
        if counted.get(k as usize).copied().unwrap_or(0) > 0 {
            return Ok(false);
        }
        let tv = self.team_victory();
        let team = self.world.kingdoms[k as usize].team;
        let total: u32 = self.world.kingdoms.iter().filter(|x| x.id == k || (tv && !x.independent && x.team == team)).map(|x| counted.get(x.id as usize).copied().unwrap_or(0)).sum();
        Ok(total == 0)
    }
    /// 4F5490: the object leaves the world without dying.
    fn world_remove_object(&mut self, actor: Object) -> Result<()> {
        match self.unit(actor) {
            Some(Unit::Org(g)) => {
                let members = self.world.orgs.get(g as usize).map(|o| o.members.clone()).unwrap_or_default();
                for m in members {
                    if let Some(a) = self.world.actors.get_mut(m as usize) {
                        a.alive = false;
                        a.removed = true;
                    }
                }
                if let Some(o) = self.world.orgs.get_mut(g as usize) {
                    o.alive = false;
                    o.members.clear();
                }
            }
            Some(Unit::Actor(a)) => {
                if let Some(x) = self.world.actors.get_mut(a as usize) {
                    x.alive = false;
                    x.removed = true;
                }
            }
            None => {}
        }
        Ok(())
    }
    /// 4D7960: the actor dies (see the module documentation).
    fn actor_destroy(&mut self, actor: Object) -> Result<()> {
        let targets: Vec<ActorId> = match self.unit(actor) {
            Some(Unit::Org(g)) => self.world.orgs.get(g as usize).map(|o| o.members.clone()).unwrap_or_default(),
            Some(Unit::Actor(a)) => vec![a],
            None => Vec::new(),
        };
        for a in targets {
            if self.world.actors.get(a as usize).is_some_and(|x| x.alive) {
                crate::combat::script_kill(self.world, a);
            }
        }
        Ok(())
    }
    /// 7476E0 on the audio system.
    fn play_speech(&mut self, data: Option<Object>) -> Result<()> {
        if let Some((tag::AUDIO, i)) = data.map(split) {
            let ids = self.interned(i);
            self.notice(Notice::Speech(ids));
        }
        Ok(())
    }
    /// 50C880 on World+130.
    fn flash_ui_button(&mut self, name: &[u16]) -> Result<()> {
        if !name.is_empty() {
            self.notice(Notice::FlashButton(text(name)));
        }
        Ok(())
    }
    /// 48D500 on GameData+140: the DifficultyLevel by IDS, its index (+C).
    fn difficulty_by_name(&mut self, name: &[u16]) -> Result<Option<i32>> {
        let n = text(name);
        Ok(self.env.difficulties.iter().position(|ids| ids.eq_ignore_ascii_case(&n)).map(|i| i as i32))
    }
    /// *(GameVars+104): the DifficultyLevel world rule: the session's choice
    /// ([`set_difficulty`]), else its default.
    fn map_difficulty(&self) -> Result<i32> {
        self.env.difficulty.or(self.env.default_difficulty).ok_or_else(|| state("the DifficultyLevel world rule is not loaded"))
    }
    fn is_actor_data(&mut self, data: Object) -> Result<bool> {
        Ok(matches!(split(data), (tag::DATA, d) if self.world.data.defs.get(d as usize).is_some_and(|x| x.actor_data || !x.category.is_empty())))
    }
    /// Actor virtual +158 (held in a transport) and +15C (the holder).
    fn actor_virtual_158(&mut self, _actor: Object) -> Result<bool> {
        Ok(false)
    }
    fn actor_virtual_15c(&mut self, _actor: Object) -> Result<Option<Object>> {
        Ok(None)
    }
    /// Actor+7C.
    fn organization(&self, actor: Object) -> Result<Option<Object>> {
        Ok(match split(actor) {
            (tag::ACTOR, a) => self.world.actors.get(a as usize).and_then(|x| x.org).map(|g| obj(tag::ORG, u64::from(g))),
            (tag::ORG, _) => Some(actor),
            _ => None,
        })
    }
    /// 578D10.
    fn organization_has_member(&mut self, organization: Object, id: u32) -> Result<bool> {
        let (tag::ORG, g) = split(organization) else { return Ok(false) };
        Ok(self.world.orgs.get(g as usize).is_some_and(|o| o.members.iter().any(|&m| self.world.ids.actor(m).is_some_and(|x| x.raw() == id))))
    }
    /// Actor+4.
    fn definition(&self, actor: Object) -> Result<Option<Object>> {
        Ok(self.def_of(actor).map(|d| obj(tag::DATA, u64::from(d))))
    }
    /// Actor+100/+104: the states aa-play tracks (5 mind_exhausted, 19
    /// out_of_supply, 21 under construction, 22 entrenched).
    fn state_bits(&self, actor: Object) -> Result<u64> {
        // the state indices of 0x477FC0 that the shipped scripts read or aa-play knows;
        // an index with no reader here stays off
        const MIND_ATTACKING: u32 = 0x00;
        const MIND_ENGAGED: u32 = 0x04;
        const MIND_EXHAUSTED: u32 = 0x05;
        const MIND_MOVING: u32 = 0x08;
        const MIND_RETREATING: u32 = 0x0D;
        const MIND_ROUTING: u32 = 0x0E;
        const OUT_OF_SUPPLY: u32 = 0x13;
        const CONSTRUCTION: u32 = 0x15;
        const ENTRENCHED: u32 = 0x16;
        const DEAD: u32 = 0x17;
        const STRUCTURE_PACKING: u32 = 0x19;
        const STRUCTURE_UNPACKING: u32 = 0x1A;
        const MIND_PACKING: u32 = 0x23;
        const MIND_UNPACKING: u32 = 0x24;
        const HQ_ATTACHED: u32 = 0x29;
        let mut bits = 0u64;
        let mut set = |state: u32, on: bool| {
            if on {
                bits |= 1 << state;
            }
        };
        match split(actor) {
            (tag::ACTOR, a) => {
                if let Some(b) = self.world.books.actor(a as ActorId) {
                    set(OUT_OF_SUPPLY, b.out_of_supply);
                    set(CONSTRUCTION, b.constructing);
                }
                if let Some(x) = self.world.actors.get(a as usize) {
                    use crate::world::Activity;
                    set(DEAD, !x.alive);
                    set(STRUCTURE_PACKING, x.activity == Activity::Packing);
                    set(STRUCTURE_UNPACKING, x.activity == Activity::Unpacking);
                    set(MIND_PACKING, x.activity == Activity::Packing);
                    set(MIND_UNPACKING, x.activity == Activity::Unpacking);
                    // a lone actor's Mind; a member's state is its organization's
                    match x.org.and_then(|g| self.world.orgs.get(g as usize)) {
                        Some(o) => {
                            set(MIND_MOVING, matches!(o.order, Order::Move { .. } | Order::Deploy { .. }));
                            set(MIND_ATTACKING, matches!(o.order, Order::Attack { .. }));
                            set(HQ_ATTACHED, o.hq.is_some());
                        }
                        None => {
                            set(MIND_MOVING, matches!(x.order, Order::Move { .. } | Order::Deploy { .. }));
                            set(MIND_ATTACKING, matches!(x.order, Order::Attack { .. }));
                        }
                    }
                    if let Some(c) = x.org.and_then(|g| self.world.combat.org(g)) {
                        set(MIND_ENGAGED, c.routing.is_none() && !c.exhausted && c.engaging());
                        set(MIND_ROUTING, c.routing.is_some());
                    } else {
                        set(MIND_ENGAGED, x.combat.target.is_some());
                    }
                }
            }
            (tag::ORG, g) => {
                set(OUT_OF_SUPPLY, self.world.books.org(g as OrgId).is_some_and(|b| b.out_of_supply));
                if let Some(c) = self.world.combat.org(g as OrgId) {
                    set(MIND_EXHAUSTED, c.exhausted);
                    set(ENTRENCHED, c.entrenched);
                    set(MIND_ENGAGED, c.routing.is_none() && !c.exhausted && c.engaging());
                    set(MIND_ROUTING, c.routing.is_some());
                }
                if let Some(o) = self.world.orgs.get(g as usize) {
                    set(DEAD, !o.alive);
                    set(MIND_MOVING, matches!(o.order, Order::Move { .. } | Order::Deploy { .. }));
                    set(MIND_ATTACKING, matches!(o.order, Order::Attack { .. }));
                    set(HQ_ATTACHED, o.hq.is_some());
                }
                set(MIND_RETREATING, crate::orders::retreating(self.world, g as OrgId));
            }
            _ => {}
        }
        Ok(bits)
    }
    /// Actor+60 body (health, maximum).
    fn body(&self, actor: Object) -> Result<Option<(u32, u32)>> {
        Ok(match split(actor) {
            (tag::ACTOR, a) => self.world.actors.get(a as usize).map(|x| {
                let max = self.world.combat.actor(a as ActorId).map(|c| c.max_health).filter(|m| *m > 0.0).unwrap_or(self.world.data.def(x.def).health);
                (x.health.to_bits(), max.to_bits())
            }),
            _ => None,
        })
    }
    /// 4F0360: the first thing of the area's member list.
    fn area_first_actor(&mut self, area: Option<Object>) -> Result<Option<Object>> {
        let Some((tag::PLACED, i)) = area.map(split) else { return Ok(None) };
        let Some(a) = self.env.areas.iter().find(|a| a.placed as u64 == i) else { return Ok(None) };
        let ids: Vec<u32> = a.members.iter().map(|m| m.0).collect();
        for id in ids {
            if let Some(o) = self.resolve_id(id)? {
                return Ok(Some(o));
            }
        }
        Ok(None)
    }
    /// 4F0950: 4EA0F0 actor shape overlaps the area (46EF30).
    fn actor_inside_area(&mut self, actor: Option<Object>, area: Option<Object>) -> Result<bool> {
        let (Some(actor), Some((tag::PLACED, i))) = (actor, area.map(split)) else { return Ok(false) };
        let raw = aa_triggers::selectors::Registry::raw_id(self, actor)?;
        let Some(a) = self.env.areas.iter().find(|a| a.placed as u64 == i) else { return Ok(false) };
        Ok(member_position(self.world, raw).is_some_and(|m| inside(a, &m)))
    }
    /// 76B960: a human kingdom's player.
    fn kingdom_player(&mut self, kingdom: Object) -> Result<Option<Object>> {
        let Some(k) = self.kingdom_of(kingdom) else { return Ok(None) };
        Ok(self.world.kingdoms.get(k as usize).filter(|x| x.controller == crate::world::Controller::Human).map(|_| obj(tag::PLAYER, u64::from(k))))
    }
    fn player_selection(&self, player: Object) -> Result<Option<Object>> {
        Ok(match split(player) {
            (tag::PLAYER, k) => Some(obj(tag::SELECTION, k)),
            _ => None,
        })
    }
    fn selection_actors(&self, selection: Object) -> Result<Vec<Option<Object>>> {
        let (tag::SELECTION, k) = split(selection) else { return Ok(Vec::new()) };
        Ok(self
            .env
            .selections
            .get(&(k as KingdomId))
            .map(|v| {
                v.iter()
                    .map(|u| {
                        Some(match u {
                            Unit::Org(g) => obj(tag::ORG, u64::from(*g)),
                            Unit::Actor(a) => obj(tag::ACTOR, u64::from(*a)),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default())
    }
    /// 4E0FB0: the Kingdom+318 property list (technologies, handicap).
    fn kingdom_has_property(&mut self, kingdom: Object, property: Object) -> Result<u32> {
        let (Some(k), (tag::PROPERTY, p)) = (self.kingdom_of(kingdom), split(property)) else { return Ok(0) };
        let id = self.interned(p);
        let b = self.world.books.kingdom(k);
        Ok(u32::from(b.is_some_and(|b| b.technologies.iter().any(|t| t.eq_ignore_ascii_case(&id)) || b.handicap.as_deref().is_some_and(|h| h.eq_ignore_ascii_case(&id)))))
    }
    /// Actor virtual +70 / +74 (4EA0D0) for the kingdom.
    fn actor_seen_by(&mut self, actor: Object, kingdom: Object, explored: bool) -> Result<u32> {
        let k = self.kingdom_of(kingdom);
        Ok(u32::from(match self.unit(actor) {
            Some(Unit::Actor(a)) => {
                if explored {
                    crate::fog::actor_seen(self.world, a, k)
                } else {
                    crate::fog::actor_visible(self.world, a, k)
                }
            }
            Some(Unit::Org(g)) => crate::fog::org_visible(self.world, g, k),
            None => false,
        }))
    }
}

impl aa_triggers::actions::Effects for Host<'_> {
    /// [[B39940+104]] (off in the shipped settings).
    fn warn_invalid_ids(&self) -> Result<bool> {
        Ok(false)
    }
    fn camera_debug_log(&self) -> Result<bool> {
        Ok(false)
    }
    fn world_time_bits(&self) -> Result<u32> {
        Ok(self.world.clock.begin.0)
    }
    /// 4BEDF0: object+18 name (a kingdom's name: its map slot id).
    fn object_name(&self, object: Object) -> Result<Vec<u16>> {
        Ok(match split(object) {
            (tag::KINGDOM, k) => self.world.ids.kingdom_slot(k as KingdomId).and_then(|s| self.world.map.slots.get(s)).map(|s| utf16(&s.id)).unwrap_or_default(),
            (tag::MAP_KINGDOM, s) => self.world.map.slots.get(s as usize).map(|s| utf16(&s.id)).unwrap_or_default(),
            _ => Vec::new(),
        })
    }
    /// Camera +20/+24/+28 look-at and +60 zoom.
    fn camera_fields(&self, camera: Object) -> Result<CameraFields> {
        let (tag::PLACED, i) = split(camera) else { return Err(state("not a camera")) };
        let c = self.env.cameras.iter().find(|c| c.placed as u64 == i).ok_or_else(|| state("not a camera"))?;
        Ok(CameraFields { look_at: c.position, zoom: c.zoom })
    }
    /// 4DBD80: -(85D8D0 atan2 of facing (+2C, +30), in [0, 2pi), + pi/2 [A028D4])
    /// times 57.29578 [9FDA94]: the camera's heading in degrees (the original's
    /// log of tutorial01: -404.852754 where this gives -405.0: its table is
    /// exact math here, within 0.15 deg).
    fn camera_pitch(&mut self, camera: Object) -> Result<u32> {
        let (tag::PLACED, i) = split(camera) else { return Ok(0) };
        let c = self.env.cameras.iter().find(|c| c.placed as u64 == i).ok_or_else(|| state("not a camera"))?;
        let mut a = c.facing[1].atan2(c.facing[0]);
        if a < 0.0 {
            a += std::f32::consts::TAU;
        }
        Ok((-(a + std::f32::consts::FRAC_PI_2) * 57.295_78).to_bits())
    }
    /// 4DBD40: 85DAE0 (asin table) of facing +34 * 57.29578: the camera's
    /// elevation in degrees. The saved facing holds the 2-D unit direction and the
    /// tangent of the elevation; a Thing's facing at +2C is the unit vector
    /// (the original's log of tutorial01: 39.901750 where this gives 40.0: its
    /// table is exact math here, within 0.1 deg).
    fn camera_yaw(&mut self, camera: Object) -> Result<u32> {
        let (tag::PLACED, i) = split(camera) else { return Ok(0) };
        let c = self.env.cameras.iter().find(|c| c.placed as u64 == i).ok_or_else(|| state("not a camera"))?;
        let v = glam::Vec3::from(c.facing);
        let len = v.length();
        let z = if len > 0.0 { v.z / len } else { 0.0 };
        Ok((z.clamp(-1.0, 1.0).asin() * 57.295_78).to_bits())
    }
    /// 59ABA0: the simulation keeps no real clock; world time in ms.
    fn real_time_ms(&mut self) -> Result<u32> {
        Ok((f(self.world.clock.begin.0) * 1000.0) as u32)
    }
    /// The AVars by index (*(B39990 + 4 * index), registered at 0x4021AC
    /// ff.): 0xB CameraZoomDefault (AVars.tgi), 0x12 CameraYaw (registered
    /// default -45), 0x14 CameraPitch (40, bounds 10..80).
    fn camera_var(&self, offset: u32) -> Result<u32> {
        Ok(match offset {
            0x2c => self.env.camera_zoom_default,
            0x48 => 0xc234_0000,
            0x50 => 0x4220_0000,
            _ => 0,
        })
    }
    /// 50D120 on World+130.
    fn ui_script_command(&mut self, command: &[u16]) -> Result<()> {
        self.notice(Notice::UiCommand(text(command)));
        Ok(())
    }
    /// 4F1080.
    fn world_set_cutscene_mode(&mut self, enabled: bool) -> Result<()> {
        self.env.cutscene = enabled;
        self.notice(Notice::CutsceneMode(enabled));
        Ok(())
    }
    fn camera_6fe710(&mut self) -> Result<()> {
        self.notice(Notice::CameraStore);
        Ok(())
    }
    fn camera_700b10(&mut self) -> Result<u32> {
        Ok(0)
    }
    fn camera_700a90(&mut self, _value: u32) -> Result<()> {
        self.notice(Notice::CameraRestore);
        Ok(())
    }
    /// 4E3AD0: a technology of the kingdom (Kingdom+318).
    fn kingdom_add_property(&mut self, kingdom: Object, property: Object, _kind: u32, _a: u32, _notify: u32) -> Result<()> {
        let (Some(k), (tag::PROPERTY, p)) = (self.kingdom_of(kingdom), split(property)) else { return Ok(()) };
        let id = self.interned(p);
        let n = self.world.data.resources.len();
        let b = self.world.books.kingdom_mut(k, n);
        if !b.technologies.iter().any(|t| t.eq_ignore_ascii_case(&id)) {
            b.technologies.push(id);
        }
        Ok(())
    }
    /// 4E23D0.
    fn kingdom_remove_property(&mut self, kingdom: Object, property: Object, _a: u32) -> Result<()> {
        let (Some(k), (tag::PROPERTY, p)) = (self.kingdom_of(kingdom), split(property)) else { return Ok(()) };
        let id = self.interned(p);
        let n = self.world.data.resources.len();
        self.world.books.kingdom_mut(k, n).technologies.retain(|t| !t.eq_ignore_ascii_case(&id));
        Ok(())
    }
    /// 4CEF70.
    fn actor_remove_property(&mut self, actor: Object, property: Object, _a: u32) -> Result<()> {
        let (tag::PROPERTY, p) = split(property) else { return Ok(()) };
        let id = self.interned(p);
        let targets: Vec<ActorId> = match self.unit(actor) {
            Some(Unit::Org(g)) => self.world.orgs.get(g as usize).map(|o| o.members.clone()).unwrap_or_default(),
            Some(Unit::Actor(a)) => vec![a],
            None => Vec::new(),
        };
        for a in targets {
            self.world.books.actor_mut(a).properties.retain(|t| !t.eq_ignore_ascii_case(&id));
        }
        Ok(())
    }
    /// World+154.
    fn world_kingdom_slots(&self) -> Result<Vec<Option<Object>>> {
        Ok(self.world.kingdoms.iter().map(|k| Some(obj(tag::KINGDOM, u64::from(k.id)))).collect())
    }
    /// 50CD00 DisplayBanner.
    fn ui_display_banner(&mut self, t: &[u16], duration: u32) -> Result<()> {
        self.notice(Notice::Banner { text: text(t), seconds: f(duration) });
        Ok(())
    }
    /// SunData +18/+24/+30.
    fn sun_colors(&self, sun: Object) -> Result<[[u32; 3]; 3]> {
        let (tag::SUN, i) = split(sun) else { return Err(state("not a sun")) };
        self.env.suns.get(i as usize).map(|s| s.colors).ok_or_else(|| state("not a sun"))
    }
    /// 4EDB10 on World+4C.
    fn view_set_sun_colors(&mut self, colors: &[[u32; 3]; 3], duration: u32) -> Result<()> {
        let now = self.world.clock.begin.0;
        self.env.sun.set_colors(*colors, duration, now);
        Ok(())
    }
    /// GameData+34 entry +18: the resource kind (0 purchase, 1 upkeep, 2
    /// limited).
    fn resource_flags18(&self) -> Result<Vec<u32>> {
        Ok(self
            .world
            .data
            .resources
            .iter()
            .map(|r| match r.kind {
                crate::data::ResourceKind::Stockpiled => 0,
                crate::data::ResourceKind::Upkeep => 1,
                crate::data::ResourceKind::LimitedConsumed => 2,
            })
            .collect())
    }
    /// 4E2080 -> 4E1630: stock + amount (at most EconomyStockpileResourceMax).
    fn kingdom_add_resource(&mut self, kingdom: Object, index: u32, amount: u32) -> Result<()> {
        let Some(k) = self.kingdom_of(kingdom) else { return Ok(()) };
        let max = self.world.data.rule_ci("EconomyStockpileResourceMax").unwrap_or(f32::MAX);
        let n = self.world.data.resources.len();
        let b = self.world.books.kingdom_mut(k, n);
        if let Some(s) = b.stockpiled.get_mut(index as usize) {
            *s = (*s + f(amount)).min(max);
        }
        Ok(())
    }
    /// 4F1340.
    fn world_show_counter(&mut self, start: u32, duration: u32) -> Result<()> {
        self.env.counter = Some((start, duration));
        Ok(())
    }
    /// 4F1390.
    fn world_hide_counter(&mut self) -> Result<()> {
        self.env.counter = None;
        Ok(())
    }
    /// World+290.
    fn set_weather(&mut self, weather: Option<Object>) -> Result<()> {
        let w = weather.and_then(|o| match split(o) {
            (tag::WEATHER, i) => Some(self.interned(i)),
            _ => None,
        });
        self.env.weather = w.clone();
        self.notice(Notice::Weather(w));
        Ok(())
    }
    /// 7474F0.
    fn play_sound(&mut self, data: Option<Object>, location: Option<[u32; 3]>) -> Result<()> {
        if let Some((tag::AUDIO, i)) = data.map(split) {
            let ids = self.interned(i);
            self.notice(Notice::Sound { ids, at: location.map(|l| l.map(f)) });
        }
        Ok(())
    }
    /// 50C3E0.
    fn ui_flash_actor(&mut self, actor: Object) -> Result<()> {
        if let Some(u) = self.unit(actor) {
            self.notice(Notice::FlashActor(u));
        }
        Ok(())
    }
    fn ui_root(&self) -> Result<Option<Object>> {
        Ok(Some(obj(tag::UI_ROOT, 0)))
    }
    /// UI root virtual +28: every game interface widget exists by name.
    fn ui_find_widget(&mut self, _root: Object, name: &[u16]) -> Result<Option<Object>> {
        let n = text(name);
        Ok(Some(obj(tag::WIDGET, self.intern(&n))))
    }
    fn widget_name(&self, widget: Object) -> Result<Vec<u16>> {
        let (tag::WIDGET, i) = split(widget) else { return Ok(Vec::new()) };
        Ok(utf16(&self.interned(i)))
    }
    /// 4F8C30: prepend to World+294.
    fn world_script_listen(&mut self, name: &[u16]) -> Result<()> {
        if !name.is_empty() {
            self.env.listens.insert(0, name.to_vec());
        }
        Ok(())
    }
    /// 4F5E70 -> 45A300: remove the first match.
    fn world_script_unlisten(&mut self, name: &[u16]) -> Result<bool> {
        if name.is_empty() {
            return Ok(false);
        }
        match self.env.listens.iter().position(|l| aa_triggers::predicates::wcsicmp_eq(l, name)) {
            Some(i) => {
                self.env.listens.remove(i);
                Ok(true)
            }
            None => Ok(false),
        }
    }
    /// 4CE770: the organization's experience (aa-play's award path).
    fn actor_gain_experience(&mut self, actor: Object, amount: u32) -> Result<()> {
        let org = match self.unit(actor) {
            Some(Unit::Org(g)) => Some(g),
            Some(Unit::Actor(a)) => self.world.actors.get(a as usize).and_then(|x| x.org),
            None => None,
        };
        if let Some(g) = org
            && self.world.combat.org(g).is_some()
        {
            let mut st = std::mem::take(&mut self.world.combat);
            crate::morale::award(self.world, &mut st, g, f(amount));
            self.world.combat = st;
        }
        Ok(())
    }
    /// ActorData virtual +48: a VFX thing.
    fn is_vfx(&mut self, data: Object) -> Result<bool> {
        Ok(matches!(split(data), (tag::DATA, d) if self.world.data.defs.get(d as usize).is_some_and(|x| x.category.eq_ignore_ascii_case("VFX"))))
    }
    /// [B39990+64] +4/+8: the default orientation (0, -1).
    fn default_vfx_orientation(&self) -> Result<[u32; 2]> {
        Ok([0, 0xbf80_0000])
    }
    /// 4454A0.
    fn rotate_2d(&mut self, vector: [u32; 2], angle: u32) -> Result<[u32; 2]> {
        let (s, c) = f(angle).sin_cos();
        let (x, y) = (f(vector[0]), f(vector[1]));
        Ok([(x * c - y * s).to_bits(), (x * s + y * c).to_bits()])
    }
    /// 50D280.
    fn spawn_vfx(&mut self, vfx: Object, position: [u32; 3], orientation: [u32; 3]) -> Result<()> {
        let ids = self.def_of(vfx).or(match split(vfx) {
            (tag::DATA, d) => Some(d as u32),
            _ => None,
        });
        if let Some(d) = ids {
            let id = self.world.data.def(d).id.clone();
            self.notice(Notice::Vfx { ids: id, at: position.map(f), orientation: orientation.map(f) });
        }
        Ok(())
    }
    /// 40BC50: the low byte of EAX is the element's result (nonzero).
    fn chat_message_all(&mut self, t: &[u16]) -> Result<u32> {
        self.notice(Notice::Chat { kingdom: None, text: text(t) });
        Ok(1)
    }
    fn chat_message_kingdom(&mut self, kingdom: Object, t: &[u16]) -> Result<()> {
        let k = self.kingdom_of(kingdom);
        self.notice(Notice::Chat { kingdom: k, text: text(t) });
        Ok(())
    }
    /// Actor+4 +1BC: the actor's animation set (resolved by name on use).
    fn animation_set(&self, actor: Object) -> Result<Option<Object>> {
        Ok(self.unit(actor).map(|_| obj(tag::ANIMATION_SET, 0)))
    }
    fn animation_by_name(&mut self, _set: Object, name: &[u16]) -> Result<Option<Object>> {
        let n = text(name);
        Ok(Some(obj(tag::ANIMATION, self.intern(&n))))
    }
    /// 4CE730.
    fn actor_play_animation(&mut self, actor: Object, animation: Object) -> Result<bool> {
        let (Some(u), (tag::ANIMATION, i)) = (self.unit(actor), split(animation)) else { return Ok(false) };
        let name = self.interned(i);
        self.notice(Notice::Animation { actor: u, name });
        Ok(true)
    }
    /// 4D4C60(flag): the actor's hold (lock) state, Object+8 bit 6.
    fn actor_4d4c60(&mut self, actor: Object, flag: u8) -> Result<()> {
        let w = self.env.flags.entry(actor.0).or_insert(0);
        if flag != 0 {
            *w |= 0x40;
        } else {
            *w &= !0x40;
        }
        Ok(())
    }
    /// 4D9B60 on aa-play's commands.
    fn process_command(&mut self, actor: Object, command: &mut TriggerCommand) -> Result<bool> {
        let name = match command.definition.map(split) {
            Some((tag::COMMAND, i)) => self.interned(i),
            Some((tag::COMMAND_ORDINAL, 5)) => "move".into(),
            Some((tag::COMMAND_ORDINAL, 6)) => "stop".into(),
            _ => return Ok(false),
        };
        if split(actor).0 == tag::VEHICLE {
            // the transport of 543BA0 (see transport_load)
            return Ok(true);
        }
        let Some(unit) = self.command_unit(actor) else { return Ok(false) };
        let Some(kingdom) = self.owner_of(actor) else { return Ok(false) };
        let at = (command.flags & 4 != 0).then(|| Vec2::new(f(command.location[0]), f(command.location[1])));
        let target = if command.flags & 1 != 0 {
            aa_triggers::selectors::Registry::resolve_id(self, command.target_id)?.and_then(|o| self.unit(o)).map(|u| match u {
                Unit::Actor(a) => a,
                Unit::Org(g) => crate::ids::target_actor(self.world, Unit::Org(g)).unwrap_or(u32::MAX),
            })
        } else {
            None
        };
        let units = vec![unit];
        let c = match (name.as_str(), at, target) {
            ("move", Some(to), _) => Command::Move { kingdom, units, to, attack_move: false },
            ("stop", _, _) => Command::Stop { kingdom, units },
            ("kill" | "attack", _, Some(t)) if t != u32::MAX => Command::Attack { kingdom, units, target: t },
            ("kill" | "attack", Some(to), _) => Command::Move { kingdom, units, to, attack_move: true },
            // (orders) the Mind states of `crate::orders`
            ("guard", at, Some(t)) if t != u32::MAX => Command::Order { kingdom, units, order: crate::orders::ActorOrder::Guard { target: Some(Unit::Actor(t)), at: at.unwrap_or_default(), aux: None } },
            ("guard", Some(to), _) => Command::Order { kingdom, units, order: crate::orders::ActorOrder::Guard { target: None, at: to, aux: None } },
            ("retreat", Some(to), _) => Command::Order { kingdom, units, order: crate::orders::ActorOrder::Retreat { target: None, at: to } },
            ("explore", Some(to), _) => Command::Order { kingdom, units, order: crate::orders::ActorOrder::Explore { at: to, scalar: None } },
            ("rout", _, _) => Command::Order { kingdom, units, order: crate::orders::ActorOrder::Rout },
            ("bombard", Some(p), _) => Command::Bombard { kingdom, units, target: crate::world::AbilityTarget::Point(p) },
            ("bombard", _, Some(t)) if t != u32::MAX => Command::Bombard { kingdom, units, target: crate::world::AbilityTarget::Actor(t) },
            ("air_attack" | "air_recon", tgt_at, tgt) => {
                let Unit::Actor(building) = unit else { return Ok(false) };
                let target = match (tgt_at, tgt) {
                    (Some(p), _) => crate::world::AbilityTarget::Point(p),
                    (_, Some(t)) if t != u32::MAX => crate::world::AbilityTarget::Actor(t),
                    _ => return Ok(false),
                };
                if name == "air_attack" { Command::AirAttack { kingdom, building, target } } else { Command::AirRecon { kingdom, building, target } }
            }
            ("unpack", Some(at), _) => {
                let Unit::Actor(truck) = unit else { return Ok(false) };
                let facing = self.world.actors.get(truck as usize).map_or(0.0, |a| a.facing);
                Command::Deploy { kingdom, truck, at, facing }
            }
            ("pack", _, _) => {
                let Unit::Actor(building) = unit else { return Ok(false) };
                Command::Pack { kingdom, building }
            }
            _ => {
                self.env.notes.push(format!("actor command '{name}' (flags {:#x}) has no aa-play counterpart", command.flags));
                return Ok(false);
            }
        };
        Ok(self.issue(c))
    }
    fn release_command_shared(&mut self, _shared: u64) -> Result<()> {
        Ok(())
    }
    /// World+148.
    fn world_slots_148(&self) -> Result<Vec<Option<Object>>> {
        Ok(self.world.kingdoms.iter().map(|k| Some(obj(tag::KINGDOM, u64::from(k.id)))).collect())
    }
    /// Kingdom+284 member list: node = kingdom << 32 | position, the list
    /// reread at every node (next after the callbacks, as the original).
    fn member_head(&self, kingdom: Object) -> Result<Option<u64>> {
        let Some(k) = self.kingdom_of(kingdom) else { return Ok(None) };
        Ok((!self.members_of(k).is_empty()).then_some(u64::from(k) << 32))
    }
    fn member_value(&self, node: u64) -> Result<Option<Object>> {
        let (k, i) = ((node >> 32) as KingdomId, (node & 0xffff_ffff) as usize);
        Ok(self.members_of(k).get(i).copied())
    }
    fn member_next(&self, node: u64) -> Result<Option<u64>> {
        let (k, i) = ((node >> 32) as KingdomId, (node & 0xffff_ffff) as usize);
        Ok((i + 1 < self.members_of(k).len()).then_some(node + 1))
    }
    /// Actor+80 (the Mind) and its +10.
    fn actor_80_ready(&self, actor: Object) -> Result<bool> {
        Ok(matches!(split(actor).0, tag::ORG) || matches!(split(actor), (tag::ACTOR, a) if self.world.actors.get(a as usize).is_some_and(|x| x.org.is_some())))
    }
    /// 5ADC90: the cutscene's video (Video.rwd, by name).
    fn video_path(&mut self, name: &[u16]) -> Result<Vec<u16>> {
        Ok(name.to_vec())
    }
    /// 410CB0 (AVars.tgi FullscreenVideoWidgetIDS).
    fn game_var_string(&mut self, name: &[u16]) -> Result<Vec<u16>> {
        Ok(if text(name).eq_ignore_ascii_case("FullscreenVideoWidgetIDS") { utf16("FullscreenVideoWidget") } else { Vec::new() })
    }
    /// GameData+298.
    fn command_definition(&self, ordinal: u32) -> Result<Option<Object>> {
        Ok(Some(obj(tag::COMMAND_ORDINAL, u64::from(ordinal))))
    }
    /// Actor+6C +28 (Mover maximum speed), else the organization's +D8.
    fn move_factor(&self, actor: Object) -> Result<Option<u32>> {
        Ok(match self.unit(actor) {
            Some(Unit::Actor(a)) => self.world.actors.get(a as usize).map(|x| self.world.data.def(x.def).speed_max).filter(|s| *s > 0.0).map(f32::to_bits),
            Some(Unit::Org(g)) => self
                .world
                .orgs
                .get(g as usize)
                .and_then(|o| o.members.iter().filter_map(|&m| self.world.actors.get(m as usize)).map(|x| self.world.data.def(x.def).speed_max).filter(|s| *s > 0.0).reduce(f32::min))
                .map(f32::to_bits),
            None => None,
        })
    }
    /// 554B30 (see the module documentation).
    fn find_placement(&mut self, position: &mut [u32; 2], facing: &mut [u32; 2], data: Object, radius: u32, _mode: u32) -> Result<bool> {
        let def = match split(data) {
            (tag::DATA, d) => Some(d as u32),
            _ => None,
        };
        let p = Vec2::new(f(position[0]), f(position[1]));
        let dir = Vec2::new(f(facing[0]), f(facing[1]));
        match self.open_spot(p, dir, def, f(radius)) {
            Some(q) => {
                *position = [q.x.to_bits(), q.y.to_bits()];
                Ok(true)
            }
            None => Ok(false),
        }
    }
    /// ActorData +1C flags (bit 12 flying), +174 bit 7 (organization), +414
    /// altitude, +2F0; actor +20/+24/+28 position.
    fn data_word(&self, object: Object, offset: u32) -> Result<u32> {
        Ok(match (split(object), offset) {
            ((tag::DATA, d), 0x174) => self.world.data.defs.get(d as usize).map_or(0, |x| if x.class == Some(DefClass::Organization) || !x.slots.is_empty() { 0x80 } else { 0 }),
            ((tag::DATA, d), 0x1c) => self.world.data.defs.get(d as usize).map_or(0, |x| if x.flying { 0x1000 } else { 0 }),
            // ActorData +1C bit 12 (flying), +414 the aircraft's operational
            // height is the view's: the flight is not simulated (module doc)
            ((tag::TRANSPORT, d), 0x1c) => self.world.data.defs.get(d as usize).map_or(0, |x| if x.flying { 0x1000 } else { 0 }),
            ((tag::TRANSPORT, _), 0x414) => 0,
            ((tag::DATA, _), _) | ((tag::TRANSPORT, _), _) => 0,
            (_, 0x20 | 0x24 | 0x28) => {
                let p = self.position(object).unwrap_or_default();
                match offset {
                    0x20 => p.x,
                    0x24 => p.y,
                    _ => p.z,
                }
                .to_bits()
            }
            _ => 0,
        })
    }
    /// ActorData +184 (the transport), actor +C0 (its transport component),
    /// flare +57C (the flare configuration).
    fn data_ref(&self, object: Object, offset: u32) -> Result<Option<Object>> {
        Ok(match (split(object), offset) {
            ((tag::DATA, d), 0x184) => {
                let id = self.world.data.defs.get(d as usize).map(|x| x.id.to_ascii_lowercase()).unwrap_or_default();
                self.env.transports.as_ref().and_then(|t| t.get(&id)).and_then(|t| self.world.data.find(t)).map(|t| obj(tag::TRANSPORT, u64::from(t)))
            }
            ((tag::VEHICLE, d), 0xc0) => Some(obj(tag::COMPONENT, d)),
            ((tag::DATA, d), 0x57c) => Some(obj(tag::COMPONENT, d)),
            _ => None,
        })
    }
    /// Only a transport with ActorData flag bit 21, which `data_word` never
    /// reports (the transport is not modelled).
    fn transport_path(&mut self, _from: [u32; 2], _to: [u32; 2]) -> Result<[u32; 2]> {
        Err(not_bound("the transport's path"))
    }
    fn transport_path_valid(&mut self, _point: [u32; 2], _to: [u32; 2]) -> Result<bool> {
        Err(not_bound("the transport's path"))
    }
    /// 4F9C10 of the transport: not modelled (see transport_load).
    fn create_transport(&mut self, request: &SpawnRequest) -> Result<Option<Object>> {
        let (_, d) = split(request.definition);
        self.env.pending_kingdom = self.kingdom_of(request.kingdom);
        self.env.pending_transport = Some((d as crate::data::DefId, request.position.map(f)));
        Ok(Some(obj(tag::VEHICLE, d)))
    }
    /// Object+8 |= bits.
    fn or_flags8(&mut self, object: Object, bits: u32) -> Result<()> {
        *self.env.flags.entry(object.0).or_insert(0) |= bits;
        Ok(())
    }
    fn create_group(&mut self, _word_2f0: u32) -> Result<Option<Object>> {
        Ok(Some(obj(tag::GROUP, 0)))
    }
    /// 496EF0 / 496F10 on the flare configuration.
    fn flare_ready(&mut self, _config: Object, _on_actor: bool) -> Result<bool> {
        Ok(true)
    }
    /// SAITriggerTeamCommandPriority.
    fn flare_var(&mut self, _on_actor: bool) -> Result<u32> {
        Ok(self.world.data.rule_ci("SAITriggerTeamCommandPriority").unwrap_or(0.0).to_bits())
    }
    /// 572E50.
    fn create_flare(&mut self, request: &SpawnRequest, kingdom: Object, actor: Option<Object>, duration: u32, variable: u32) -> Result<()> {
        let (Some(k), (tag::DATA, d)) = (self.kingdom_of(kingdom), split(request.definition)) else { return Ok(()) };
        let ids = self.world.data.def(d as u32).id.clone();
        let actor = actor.and_then(|a| self.unit(a));
        // 572E50's SAI priority (variable, SAITriggerTeamCommandPriority) for the
        // kingdom's computer team mates; the flare's actor is the view's
        let at = match actor.and_then(|u| self.position(match u {
            Unit::Org(g) => obj(tag::ORG, u64::from(g)),
            Unit::Actor(a) => obj(tag::ACTOR, u64::from(a)),
        })) {
            Some(p) => p,
            None => glam::Vec3::from(request.position.map(f)),
        };
        let attack = ids.to_ascii_lowercase().contains("attack");
        crate::ai::team_flare(self.world, k, at.truncate(), attack, f(variable), f(duration));
        self.notice(Notice::Flare { kingdom: k, ids, at: at.to_array(), actor, seconds: f(duration) });
        Ok(())
    }
    /// 4C8C20: the ability user of the element.
    fn ability_user_init(&mut self, ability: Object) -> Result<()> {
        if let (tag::ABILITY, i) = split(ability) {
            self.env.pending_ability = Some(self.interned(i));
        }
        Ok(())
    }
    /// 4C9F10: the ability at the point, when a general's data holds it
    /// (crate::special_ops::trigger_use); the notice tells the client where.
    fn ability_use(&mut self, request: &AbilityRequest) -> Result<()> {
        let ids = self.env.pending_ability.clone().unwrap_or_default();
        if let Some(k) = self.kingdom_of(request.kingdom) {
            let at = request.at.map(f);
            if !crate::special_ops::trigger_use(self.world, k, &ids, Vec2::new(at[0], at[1])) {
                self.not_modelled(format!("the trigger ability {ids} (4C9F10)"));
            }
            self.notice(Notice::Ability { kingdom: k, ids, from: request.from.map(f), at });
        }
        Ok(())
    }
    fn ability_user_release(&mut self) -> Result<()> {
        self.env.pending_ability = None;
        Ok(())
    }
    /// 58BA90 on the transport: the flight begins, the cargo is created at the
    /// unload point when it arrives (see the module documentation).
    fn transport_load(&mut self, _component: Object, cargo: Object, name: &[u16], _group: Option<Object>, position: [u32; 2], destination: [u32; 2]) -> Result<()> {
        let (tag::DATA, d) = split(cargo) else { return Ok(()) };
        let Some(k) = self.env.pending_kingdom.take() else { return Ok(()) };
        let to = Vec2::new(f(destination[0]), f(destination[1]));
        let (transport, from) = self.env.pending_transport.take().unwrap_or((d as crate::data::DefId, [f(position[0]), f(position[1]), 0.0]));
        let t = self.world.data.def(transport);
        let (speed, ids) = (t.speed_max, t.id.clone());
        let seconds = if speed > 0.0 { Vec2::new(from[0], from[1]).distance(to) / speed } else { 0.0 };
        let now = f(self.world.clock.begin.0);
        self.env.drops.push(Drop { due: now + seconds, kingdom: k, cargo: d as crate::data::DefId, name: name.to_vec(), at: to });
        self.notice(Notice::Transport { kingdom: k, ids, from, to: [to.x, to.y], seconds });
        Ok(())
    }
    /// ActorData +174.
    fn data_flags174(&self, data: Object) -> Result<u32> {
        aa_triggers::actions::Effects::data_word(self, data, 0x174)
    }
    /// 4FA030 / 4FA150.
    fn create_actor(&mut self, request: &SpawnRequest, _alternate: bool) -> Result<Option<Object>> {
        let ((tag::DATA, d), Some(k)) = (split(request.definition), self.kingdom_of(request.kingdom)) else { return Ok(None) };
        let p = Vec2::new(f(request.position[0]), f(request.position[1]));
        let facing = Vec2::new(f(request.facing[0]), f(request.facing[1]));
        Ok(self.create(d as u32, k, p, facing, &request.name))
    }
}

impl aa_triggers::extended::Effects for Host<'_> {
    /// 4ED820 on World+4C.
    fn view_adjust_sun_dimmer(&mut self, a: u32, b: u32) -> Result<()> {
        let now = self.world.clock.begin.0;
        self.env.sun.adjust_dimmer(a, b, now);
        Ok(())
    }
    /// 79F5D0.
    fn sai_change_profile(&mut self, kingdom: Object, sai: Object) -> Result<()> {
        let (Some(k), (tag::SAI, i)) = (self.kingdom_of(kingdom), split(sai)) else { return Ok(()) };
        let ids = self.interned(i);
        crate::ai::set_profile(self.world, k, &ids);
        self.notice(Notice::SaiProfile { kingdom: k, ids });
        Ok(())
    }
    /// 747630.
    fn play_music(&mut self, data: Option<Object>) -> Result<()> {
        let ids = data.and_then(|o| match split(o) {
            (tag::AUDIO, i) => Some(self.interned(i)),
            _ => None,
        });
        self.notice(Notice::Music(ids));
        Ok(())
    }
    /// 50D230.
    fn ui_vfx_on_actor(&mut self, vfx: Object, actor: Object) -> Result<()> {
        let (Some(u), Some(d)) = (self.unit(actor), self.def_of(vfx).or(match split(vfx) {
            (tag::DATA, d) => Some(d as u32),
            _ => None,
        })) else {
            return Ok(());
        };
        let ids = self.world.data.def(d).id.clone();
        self.notice(Notice::VfxOnActor { ids, actor: u });
        Ok(())
    }
    fn fidget_var_flags(&self) -> Result<u32> {
        Err(not_bound("the fidget game variable"))
    }
    fn var_permission(&mut self, _kind: u32) -> Result<bool> {
        Err(not_bound("the game variable permissions"))
    }
    fn set_fidget_var(&mut self, _value: u8) -> Result<()> {
        Err(not_bound("the fidget game variable"))
    }
    /// 76F370.
    fn set_game_speed(&mut self, bits: u32) -> Result<()> {
        self.env.game_speed = Some(bits);
        self.notice(Notice::GameSpeed(f(bits)));
        Ok(())
    }
    fn kingdom_hero_entry(&mut self, _kingdom: Object, _data: Object) -> Result<Option<Object>> {
        Err(not_bound("the Kingdom+1FC hero table"))
    }
    fn set_hero_entry_18(&mut self, _entry: Object, _bits: u32) -> Result<()> {
        Err(not_bound("the Kingdom+1FC hero table"))
    }
    /// 579750: above zero the gain (clamped, state 5 cleared, as
    /// `morale::gain`), else the loss 579660 of minus the value (`morale::lose`;
    /// the element's mode 4 passes 579660's protection test, which `lose` does
    /// not have).
    fn organization_modify_morale(&mut self, organization: Object, value: u32, _mode: u32) -> Result<()> {
        if let Some(Unit::Org(g)) = self.unit(organization)
            && self.world.combat.org(g).is_some()
        {
            let amount = f(value);
            let mut st = std::mem::take(&mut self.world.combat);
            if amount <= 0.0 {
                crate::morale::lose(self.world, &mut st, g, -amount);
            } else {
                crate::morale::gain(self.world, &mut st, g, amount);
            }
            self.world.combat = st;
        }
        Ok(())
    }
    /// Actor virtual +24.
    fn actor_display_name(&mut self, actor: Object) -> Result<Vec<u16>> {
        Ok(self.def_of(actor).map(|d| utf16(&self.world.data.def(d).name)).unwrap_or_default())
    }
    /// Area+84.
    fn area_actor_ids(&self, area: Object) -> Result<Vec<u32>> {
        let (tag::PLACED, i) = split(area) else { return Ok(Vec::new()) };
        Ok(self.env.areas.iter().find(|a| a.placed as u64 == i).map(|a| a.members.iter().map(|m| m.0).collect()).unwrap_or_default())
    }
    fn actor_4d12c0(&mut self, _actor: Object) -> Result<bool> {
        Err(not_bound("the teleport test 4D12C0"))
    }
    fn world_4f6760(&mut self, _actor: Object, _flag: u32) -> Result<()> {
        Err(not_bound("the teleport failure 4F6760"))
    }
    /// 4EE5C0.
    fn actor_teleport(&mut self, _actor: Object, _position: [u32; 3]) -> Result<()> {
        Err(not_bound("the teleport 4EE5C0"))
    }
    /// 4EEB40.
    fn member_teleport(&mut self, _member: Object, _position: [u32; 2]) -> Result<()> {
        Err(not_bound("the teleport 4EEB40"))
    }
    fn store_flags8(&mut self, object: Object, value: u32) -> Result<()> {
        self.env.flags.insert(object.0, value & !1);
        Ok(())
    }
    /// 4F03C0: World+275 cutscene mode (its second test compares the cutscene
    /// speed 76B3A0 with the current speed, which 4F1080 set to it).
    fn world_4f03c0(&mut self) -> Result<bool> {
        Ok(self.env.cutscene)
    }
    fn players_76ac60(&mut self) -> Result<Option<Object>> {
        Ok(self.local().map(|k| obj(tag::PLAYER, u64::from(k))))
    }
    fn player_765e00(&mut self, player: Option<Object>) -> Result<Option<Object>> {
        Ok(player.and_then(|p| match split(p) {
            (tag::PLAYER, k) => Some(obj(tag::KINGDOM, k)),
            _ => None,
        }))
    }
    fn audio_lookup(&mut self, name: &[u16]) -> Result<Option<Object>> {
        let n = text(name);
        Ok((!n.is_empty()).then(|| obj(tag::AUDIO, self.intern(&n))))
    }
    fn new_hero_entry(&mut self, _data: Object) -> Result<Option<Object>> {
        Err(not_bound("the Kingdom+1FC hero table"))
    }
    fn kingdom_add_hero(&mut self, _kingdom: Object, _entry: Option<Object>) -> Result<()> {
        Err(not_bound("the Kingdom+1FC hero table"))
    }
    fn kingdom_remove_hero(&mut self, _kingdom: Object, _entry: Option<Object>) -> Result<()> {
        Err(not_bound("the Kingdom+1FC hero table"))
    }
    fn fog_restore_tiles(&mut self, _area: Object, _query: [u32; 4]) -> Result<()> {
        Err(not_bound("the fog tile pass 542950"))
    }
    fn actor_data_bank(&self) -> Result<Vec<Option<Object>>> {
        Ok((0..self.world.data.defs.len() as u64).map(|d| Some(obj(tag::DATA, d))).collect())
    }
    fn world_rng(&mut self) -> Result<Object> {
        Ok(obj(tag::RNG, 0))
    }
    /// 85D840 on World+D4 (logged by the GetRNG getter).
    fn random(&mut self, _rng: Object) -> Result<u32> {
        Ok(self.world.native_rng.next_u32())
    }
    fn kingdom_hero_count(&self, _kingdom: Object) -> Result<u32> {
        Err(not_bound("the Kingdom+1FC hero table"))
    }
    fn kingdom_hero_at(&mut self, _kingdom: Object, _index: u32) -> Result<Option<Object>> {
        Err(not_bound("the Kingdom+1FC hero table"))
    }
}

impl aa_triggers::world_program::World for Host<'_> {
    /// World+E0.
    fn current_time_bits(&self) -> u32 {
        self.world.clock.begin.0
    }
    fn presentation_mut(&mut self) -> Result<&mut Presentation> {
        Ok(&mut self.env.presentation)
    }
    fn body_actor_identity(&self, actor: Object) -> Result<aa_body::credit::Object> {
        Ok(aa_body::credit::Object(aa_triggers::selectors::Registry::raw_id(self, actor)?))
    }
    /// 54E080 / 463FA0 SharedProperty by IDS.
    fn named_property(&mut self, name: &[u16]) -> Result<Option<aa_body::credit::Object>> {
        let n = text(name);
        Ok(self.world.data.property(&n).map(|p| p.id.to_ascii_lowercase()).map(|id| aa_body::credit::Object(BODY_PROPERTY | self.intern(&id) as u32)))
    }
    /// 5421A0: Kingdom+238.
    fn set_default_victory(&mut self, kingdom: Object, value: u8) -> Result<()> {
        if let Some(k) = self.kingdom_of(kingdom) {
            self.env.default_victory.insert(k, value);
        }
        Ok(())
    }
}

fn nav(e: Error) -> control::Error {
    control::Error(e.to_string())
}

impl control::World for Host<'_> {
    fn flags8(&self, object: Object) -> control::Result<u32> {
        aa_triggers::elements::Services::flags8(self, object).map_err(nav)
    }
    fn set_flags8(&mut self, object: Object, value: u32) -> control::Result<()> {
        self.env.flags.insert(object.0, value & !1);
        Ok(())
    }
    /// Actor+154: no SAI handle object is modelled.
    fn actor_sai_handle(&self, _actor: Object) -> control::Result<Option<control::Shared>> {
        Ok(None)
    }
    fn references(&self, _shared: control::Shared) -> control::Result<u32> {
        Ok(1)
    }
    fn set_references(&mut self, _shared: control::Shared, _value: u32) -> control::Result<()> {
        Ok(())
    }
    fn destroy_shared(&mut self, _shared: control::Shared) -> control::Result<()> {
        Ok(())
    }
    fn sai_target(&self, _shared: control::Shared) -> control::Result<Option<Object>> {
        Ok(None)
    }
    fn notify_disabled(&mut self, _target: Object, _handle: control::Shared, _zero_arguments: [u32; 4]) -> control::Result<()> {
        Ok(())
    }
    fn kingdom_head284(&self, kingdom: Object) -> control::Result<Option<control::Node>> {
        aa_triggers::actions::Effects::member_head(self, kingdom).map(|h| h.map(control::Node)).map_err(nav)
    }
    fn node_actor(&self, node: control::Node) -> control::Result<Option<Object>> {
        aa_triggers::actions::Effects::member_value(self, node.0).map_err(nav)
    }
    fn node_next(&self, node: control::Node) -> control::Result<Option<control::Node>> {
        aa_triggers::actions::Effects::member_next(self, node.0).map(|n| n.map(control::Node)).map_err(nav)
    }
    fn command_definition(&self, ordinal: u32) -> control::Result<Option<Object>> {
        Ok(Some(obj(tag::COMMAND_ORDINAL, u64::from(ordinal))))
    }
    fn definition_component18(&self, _definition: Object) -> control::Result<i32> {
        Ok(0)
    }
    fn actor_component(&self, actor: Object, _index: usize) -> control::Result<Option<Object>> {
        Ok(self.unit(actor).map(|_| actor))
    }
    fn can_receive(&mut self, actor: Object, _definition: Object, _command: &mut control::Command) -> control::Result<bool> {
        Ok(self.alive(actor))
    }
    /// The stop (ordinal 6) of the AI enable / disable wrappers.
    fn process_component(&mut self, component: Object, command: &mut control::Command) -> control::Result<bool> {
        let mut c = TriggerCommand::new(command.definition);
        c.flags = command.flags;
        aa_triggers::actions::Effects::process_command(self, component, &mut c).map_err(nav)
    }
    fn actor_virtual158(&mut self, _actor: Object, _command: &mut control::Command) -> control::Result<bool> {
        Ok(false)
    }
    fn actor_virtual15c(&mut self, _actor: Object, _command: &mut control::Command) -> control::Result<Option<Object>> {
        Ok(None)
    }
    fn organization_parent(&self, _actor: Object) -> control::Result<Option<Object>> {
        Ok(None)
    }
}

fn body_err(e: Error) -> aa_body::Error {
    aa_body::Error::Callback(e.to_string())
}

impl aa_body::property_admission::World for Host<'_> {
    /// Actor+78 (the Properties component): every actor has one.
    fn property_component(&self, actor: aa_body::credit::Object) -> aa_body::Result<Option<aa_body::credit::Object>> {
        Ok(self.world.ids.thing(actor.0).filter(|t| matches!(t, crate::ids::Thing::Actor(_) | crate::ids::Thing::Org(_))).map(|_| actor))
    }
    fn component_98_present(&self, _actor: aa_body::credit::Object) -> aa_body::Result<bool> {
        Ok(false)
    }
    /// aa-actors state_effects::applicable is not bound: every property applies.
    fn applicable(&mut self, _actor: aa_body::credit::Object, _property: aa_body::credit::Object) -> aa_body::Result<bool> {
        Ok(true)
    }
    /// 5817A0 -> 581630: the actor's property record.
    fn create_and_insert(&mut self, component: aa_body::credit::Object, property: aa_body::credit::Object, _kind: u32, _source: Option<aa_body::credit::Object>) -> aa_body::Result<()> {
        if property.0 & BODY_PROPERTY != BODY_PROPERTY {
            return Err(body_err(state("not a property handle")));
        }
        let id = self.interned(u64::from(property.0 & !BODY_PROPERTY));
        let targets: Vec<ActorId> = match self.world.ids.thing(component.0) {
            Some(crate::ids::Thing::Actor(a)) => vec![a],
            Some(crate::ids::Thing::Org(g)) => self.world.orgs.get(g as usize).map(|o| o.members.clone()).unwrap_or_default(),
            _ => Vec::new(),
        };
        for a in targets {
            let props = &mut self.world.books.actor_mut(a).properties;
            if !props.iter().any(|p| p.eq_ignore_ascii_case(&id)) {
                props.push(id.clone());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(position: [f32; 2], facing: [f32; 2], shape: [f32; 6]) -> Area {
        let mut w = shape.map(f32::to_bits);
        w[0] = shape[0] as u32;
        Area { placed: 0, updatable: None, position: Vec2::from(position), facing: Vec2::from(facing), shape: w, members: Vec::new() }
    }

    /// A circle member at the point (radius 0: a point).
    fn at(x: f32, y: f32, r: f32) -> Member {
        let shape = if r > 0.0 { [1, r.to_bits(), 0, 0, 0, 0] } else { [0, 0, 0, 0, 0, 0] };
        Member { position: Vec2::new(x, y), facing: Vec2::new(0.0, -1.0), shape }
    }

    #[test]
    fn moscow_cow_pen_areas() {
        // (2v2)Moscow area_0 "Cow1": RECT radius 23.491, x -23.38..23.38,
        // y -2.28..2.28, facing (0, -1): a strip along native X.
        let a = area([255.97, 121.99], [0.0, -1.0], [3.0, 23.491, -23.38, -2.28, 23.38, 2.28]);
        assert!(inside(&a, &at(270.0, 122.5, 0.0)));
        assert!(!inside(&a, &at(255.97, 126.0, 0.0)));
        assert!(inside(&a, &at(255.97, 126.0, 2.0)));
        // area_3 "Cow4" is turned by its facing (0.085, -0.996).
        let b = area([231.78, 137.75], [0.085, -0.996], [3.0, 17.02, -1.84, -16.92, 1.84, 16.92]);
        assert!(inside(&b, &at(231.78, 150.0, 0.0)));
        assert!(!inside(&b, &at(240.0, 137.75, 0.0)));
        let c = area([10.0, 10.0], [0.0, -1.0], [1.0, 5.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(inside(&c, &at(14.0, 10.0, 0.0)) && !inside(&c, &at(16.0, 10.0, 0.5)));
    }

    #[test]
    fn area_and_camera_records_decode() {
        // world version 22: Updatable version 1, id 186, Area version 2,
        // label "Cow1", Shape68, 1 entry.
        let mut b = Vec::new();
        for w in [1u32, 186, 2] {
            b.extend(w.to_le_bytes());
        }
        b.extend(4u16.to_le_bytes());
        for c in "Cow1".encode_utf16() {
            b.extend(c.to_le_bytes());
        }
        for w in [3u32, 1.0f32.to_bits(), 0, 0, 0, 0, 1, 0x0002_0010] {
            b.extend(w.to_le_bytes());
        }
        b.push(1);
        let (updatable, shape, members) = decode_area(&b, 22).unwrap();
        assert_eq!(updatable, Some(186));
        assert_eq!(shape[0], 3);
        assert_eq!(members, vec![(0x0002_0010, true)]);
        let mut c = Vec::new();
        c.extend(2u32.to_le_bytes());
        c.extend(0u16.to_le_bytes());
        c.extend(1.43f32.to_bits().to_le_bytes());
        assert_eq!(decode_camera(&c), Some(1.43f32.to_bits()));
    }

    #[test]
    fn move_camera_commands_parse() {
        let m = parse_move_camera("MoveCamera 12.500000 0 168.789993 145.339996 1.000000 23.800000 -150.800000 1 kingdom_0").unwrap();
        assert_eq!((m.until, m.realtime, m.at, m.zoom, m.ease), (12.5, false, Vec2::new(168.79, 145.34), 1.0, true));
        assert_eq!((m.pitch, m.yaw, m.kingdom.as_str()), (23.8, -150.8, "kingdom_0"));
        assert!(parse_move_camera("PlayVideo x").is_none());
    }

    #[test]
    fn difficulty_levels_are_the_data_order() {
        let Some(root) = find_root() else { return };
        let mut notes = Vec::new();
        let (levels, default) = load_difficulties(&root, &mut notes);
        assert_eq!(levels, ["easy", "medium", "hard"]);
        // the DifficultyLevel world rule's default is MEDIUM
        assert_eq!(default, Some(1));
        assert!(notes.is_empty(), "{notes:?}");
    }

    #[test]
    fn name_table_keys_fold_ascii_case() {
        assert_eq!(name_key(&utf16("Primary1")), name_key(&utf16("PRIMARY1")));
        assert_eq!(name_key(&[0xe9]), None);
    }
}
