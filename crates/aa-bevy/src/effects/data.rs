//! The original FX data that effects and sound play from, read from the
//! original rules catalog with the template resolution aa-play uses
//! (`aa_rules::actors::ActorCatalog::load_v14` over Data.rwd and the v14
//! overlay, index_aa.lst order). Owner: effects and sound.
//!
//! Per actor Thing: the FXC lists of every weapon (AbilityDefinition
//! WhenStart / WhenDo / WhenEnd / WhenDoPerTarget, in `Def::weapons` order:
//! PerformerComponent abilities, then DefenderComponent), its View lists
//! (WhenCreated, WhenDestroyed, WhenSelected, WhenCommanded,
//! WhenCommandedAggressive), BodyComponent WhenHealth, MoverComponent
//! WhenStartMoving / WhenStopMoving and the death aspect model.
//! Global tables: every VFX and Audio definition, Bone IDS -> NIF node name,
//! Terrain -> terrain_group, Nation -> game_music, and the ambient audio
//! doodads (ObjectAnimation PlaySFX with their RANDOM start interval).
//!
//! The catalog load takes seconds, so the extracted tables are cached in
//! `assets-cache/aa-bevy/fx.json`, keyed like aa-play's GameData cache.
//! No Bevy types here (the module is also built by a standalone probe).
use aa_numeric::Precision;
use aa_rules::actors::{ActorCatalog, ResolvedNode};
use aa_rules::native::TypedValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const PC: Precision = Precision::Double53;
/// Bump when the extracted layout changes.
const CACHE_VERSION: &str = "aa-bevy-fx-4";

/// One FX system command (template_fx.tgi `FXC` and its specialisations).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Fxc {
    /// CREATE_VFX, STOP_VFX, PLAY_SFX, STOP_SFX, EQUIPMENT, PLAY_OBJECT_ANIMATION...
    pub kind: String,
    /// VFX_IDS / audio_ids / object_animation_IDS, lower case.
    pub ids: String,
    /// SELF, TARGET_THING, SOURCE_GROUND, TARGET_GROUND,
    /// SOURCE_ABILITY_POSITION, TARGET_HIT_POSITION, MAP_CENTER.
    pub at: String,
    /// Bone IDS (bones.tgi), lower case; empty = the first bone ("position").
    pub location: String,
    pub attached: bool,
    pub looping: bool,
    /// `+infantry -building` style filter on the FX target.
    pub target_flags: String,
    pub terrain_group: String,
    /// Every how many triggers the command really runs (0 never, 1 always).
    pub frequency: i32,
    pub delay: f32,
    pub duration: f32,
    pub translation: [f32; 3],
    /// WhenHealth trigger fraction (-1 = none) and ABSOLUTE_BELOW etc.
    pub health: f32,
    pub health_trigger: String,
    /// WhenDoAtTime time.
    pub time: f32,
    pub chain: Vec<Fxc>,
    pub chain_immediate: Vec<Fxc>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WeaponFx {
    pub start: Vec<Fxc>,
    pub do_: Vec<Fxc>,
    pub end: Vec<Fxc>,
    pub per_target: Vec<Fxc>,
    /// AbilityDefinition projectile_IDS (lower case).
    pub projectile: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ActorFx {
    pub weapons: Vec<WeaponFx>,
    pub created: Vec<Fxc>,
    pub destroyed: Vec<Fxc>,
    pub selected: Vec<Fxc>,
    pub commanded: Vec<Fxc>,
    pub commanded_aggressive: Vec<Fxc>,
    /// View WhenConstructed (buildings: ground planes, flags, lights).
    pub constructed: Vec<Fxc>,
    /// BodyComponent WhenHealth.
    pub health: Vec<Fxc>,
    pub start_moving: Vec<Fxc>,
    pub stop_moving: Vec<Fxc>,
    /// Aspect `aspect_death` model, Data-relative ("%s" already expanded
    /// with the View resource_prefix).
    pub death_model: Option<String>,
    /// View resource_prefix, Data-relative.
    pub resource_prefix: Option<String>,
}

/// A VFX definition (template_vfx.tgi), the fields the view reads.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VfxDef {
    /// VFXMODEL, VFXTRACER, VFXPROJECTEDTEXTURE, VFXLIGHT, VFXEXTRUSION...
    pub category: String,
    /// Data-relative model path, lower case, forward slashes.
    pub model: String,
    pub permanent: bool,
    pub critical: bool,
    pub duration: f32,
    pub gyroscope: bool,
    pub attached_scale: f32,
    pub random_orientation: bool,
    pub nightonly: bool,
    /// Tracer.
    pub color: [f32; 3],
    pub speed: f32,
    pub length: f32,
    pub spread: [f32; 3],
    /// Projected texture (Data-relative), size in metres, fades (in, out).
    pub texture: String,
    pub texture_size: f32,
    pub decal_fade: [f32; 2],
    pub additive: bool,
    pub decal_type: String,
    pub rotate_dps: f32,
    /// VFXProjectedTextureLit: lit by the world like the ground.
    pub respect_lighting: bool,
}

/// An Audio definition as 46C3A0 LoadData stores it (template_audio.tgi).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AudioDef {
    pub ids: String,
    /// 0 SOUND, 1 MUSIC, 2 SPEECH, -1 unknown.
    pub category: i32,
    pub priority: i32,
    /// control_flags: 1 INTERRUPT, 2 SEQUENTIAL, 4 ALL, 8 VOICE, 0x10 WORLD,
    /// 0x20 CAUSEFADE, 0x40 TESTSOUND.
    pub control: u8,
    /// world_flags: 1 GLOBAL, 2 FOGGED.
    pub world: u8,
    /// who_flags: 1 LOCAL, 2 PLAYER, 4 ALLIES, 8 NEUTRALS, 0x10 ENEMIES, 0x20 EVERYONE.
    pub who: u8,
    pub volume: f32,
    /// Miles min distance (falloff), max distance (muted beyond).
    pub range: [f32; 2],
    pub simultaneous_limit: i32,
    /// Audio-mount paths ("Audio/Sounds/RUS/Attack/RUSRifle01.wav").
    pub leadin: Vec<String>,
    pub files: Vec<String>,
    pub fallout: Vec<String>,
    pub fade_time_s: [f32; 2],
    pub delay_ms: [f32; 2],
    pub pitch_adjust: [f32; 2],
    pub volume_adjust: f32,
}

/// An audio doodad (Doodads/Audio): RANDOM object animation PlaySFX, and the
/// Disturbable component's action when units come near.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AmbientDef {
    pub sounds: Vec<String>,
    /// RANDOM start interval (seconds), (0, 0) when the animation is not RANDOM.
    pub start: [f32; 2],
    /// DisturbableDynamicDoodadComponent: radius, recharge min/max, update time.
    pub disturb_radius: f32,
    pub disturb_recharge: [f32; 2],
    pub disturb_update: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FxTables {
    pub source_key: String,
    pub actors: BTreeMap<String, ActorFx>,
    pub vfx: BTreeMap<String, VfxDef>,
    pub audio: BTreeMap<String, AudioDef>,
    /// Bone IDS -> NIF node name.
    pub bones: BTreeMap<String, String>,
    /// Terrain IDS -> terrain_group.
    pub terrain_groups: BTreeMap<String, String>,
    /// Nation id ("rus") -> game_music audio IDS.
    pub nation_music: BTreeMap<String, String>,
    pub ambient: BTreeMap<String, AmbientDef>,
    pub notes: Vec<String>,
}

fn value(node: &ResolvedNode<'_>, name: &str) -> Option<TypedValue> {
    match node.field(name, PC) {
        Ok(v) => v,
        // Undeclared assignments ("critical = false" on an FXC): the plain
        // string the original's GetString would return.
        Err(_) => node.plain_string_field(name).ok().flatten(),
    }
}
fn text(node: &ResolvedNode<'_>, name: &str) -> Option<String> {
    match value(node, name)? {
        TypedValue::String { value, .. } => {
            let v = value.trim().to_owned();
            (!v.is_empty() && !v.eq_ignore_ascii_case("null")).then_some(v)
        }
        _ => None,
    }
}
/// The folder of the file that assigned `name` (for 4763C0 relative paths).
fn text_with_folder(node: &ResolvedNode<'_>, name: &str) -> Option<(String, String)> {
    match value(node, name)? {
        TypedValue::String { value, source, .. } => {
            let v = value.trim().to_owned();
            if v.is_empty() || v.eq_ignore_ascii_case("null") {
                return None;
            }
            let path = source.path.replace('\\', "/");
            let folder = path.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
            Some((v, folder))
        }
        _ => None,
    }
}
fn num(node: &ResolvedNode<'_>, name: &str) -> Option<f32> {
    match value(node, name)? {
        TypedValue::Scalar(s) => Some(f32::from_bits(s.bits)),
        TypedValue::Integer { value, .. } => Some(value as f32),
        TypedValue::Boolean { value, .. } => Some(if value { 1.0 } else { 0.0 }),
        TypedValue::String { value, .. } => {
            let v = value.trim();
            match v.strip_suffix('%') {
                Some(p) => p.trim().parse::<f32>().ok().map(|x| x / 100.0),
                None => v.parse().ok(),
            }
        }
        _ => None,
    }
}
fn flag(node: &ResolvedNode<'_>, name: &str) -> Option<bool> {
    match value(node, name)? {
        TypedValue::Boolean { value, .. } => Some(value),
        TypedValue::Integer { value, .. } => Some(value != 0),
        TypedValue::String { value, .. } => match value.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}
fn point(node: &ResolvedNode<'_>, name: &str) -> Option<Vec<f32>> {
    match value(node, name)? {
        TypedValue::Point { bits, .. } => Some(bits.iter().map(|b| f32::from_bits(*b)).collect()),
        TypedValue::Scalar(s) => Some(vec![f32::from_bits(s.bits)]),
        _ => None,
    }
}
fn p2(node: &ResolvedNode<'_>, name: &str, default: [f32; 2]) -> [f32; 2] {
    point(node, name).filter(|p| p.len() >= 2).map_or(default, |p| [p[0], p[1]])
}
fn p3(node: &ResolvedNode<'_>, name: &str, default: [f32; 3]) -> [f32; 3] {
    point(node, name).filter(|p| p.len() >= 3).map_or(default, |p| [p[0], p[1], p[2]])
}

/// 4763C0: '/' is from the Data root, otherwise relative to the folder.
pub fn relative(folder: &str, value: &str) -> String {
    let value = value.replace('\\', "/");
    let folder = folder.trim_start_matches("Data/").trim_start_matches("data/");
    if let Some(v) = value.strip_prefix('/') {
        v.to_owned()
    } else if folder.is_empty() || folder.eq_ignore_ascii_case("data") {
        value
    } else {
        format!("{folder}/{value}")
    }
}

fn fxc(n: &ResolvedNode<'_>) -> Fxc {
    Fxc {
        kind: text(n, "type").unwrap_or_default().to_ascii_uppercase(),
        ids: text(n, "VFX_IDS")
            .or_else(|| text(n, "audio_ids"))
            .or_else(|| text(n, "object_animation_IDS"))
            .or_else(|| text(n, "equipment_type_IDS"))
            .unwrap_or_default()
            .to_ascii_lowercase(),
        at: text(n, "at").unwrap_or_else(|| "SELF".into()).to_ascii_uppercase(),
        location: text(n, "location").unwrap_or_default().to_ascii_lowercase(),
        attached: flag(n, "attached").unwrap_or(true),
        looping: flag(n, "looping").unwrap_or(false),
        target_flags: text(n, "target_flags").unwrap_or_default(),
        terrain_group: text(n, "terrain_group_filter").unwrap_or_default().to_ascii_lowercase(),
        frequency: num(n, "frequency").map_or(1, |f| f as i32),
        delay: num(n, "delay").unwrap_or(0.0),
        duration: num(n, "duration").unwrap_or(0.0),
        translation: p3(n, "translation", [0.0; 3]),
        health: num(n, "health").unwrap_or(-1.0),
        health_trigger: text(n, "health_trigger_type").unwrap_or_else(|| "ABSOLUTE_BELOW".into()).to_ascii_uppercase(),
        time: num(n, "time").unwrap_or(0.0),
        chain: list(n, "Chain"),
        chain_immediate: list(n, "ChainImmediate"),
    }
}
fn list(node: &ResolvedNode<'_>, name: &str) -> Vec<Fxc> {
    node.children(name).unwrap_or_default().iter().map(fxc).collect()
}

fn weapon_fx(catalog: &ActorCatalog, ability: &ResolvedNode<'_>) -> WeaponFx {
    let definition = match ability.child("AbilityDefinition") {
        Ok(Some(local)) => Some(local),
        _ => text(ability, "ability_IDS").and_then(|id| catalog.resolved_definition("AbilityDefinition", &id, PC).ok()),
    };
    let Some(d) = definition else { return WeaponFx::default() };
    WeaponFx {
        start: list(&d, "WhenStart"),
        do_: list(&d, "WhenDo"),
        end: list(&d, "WhenEnd"),
        per_target: list(&d, "WhenDoPerTarget"),
        projectile: text(&d, "projectile_IDS").map(|p| p.to_ascii_lowercase()),
    }
}

fn actor_fx(catalog: &ActorCatalog, id: &str) -> Option<ActorFx> {
    let actor = catalog.resolved_actor(id).ok()?;
    let mut fx = ActorFx::default();
    for component in ["PerformerComponent", "DefenderComponent"] {
        if let Ok(Some(c)) = actor.child(component) {
            for ability in c.children("Ability").unwrap_or_default() {
                fx.weapons.push(weapon_fx(catalog, &ability));
            }
        }
    }
    if let Ok(Some(view)) = actor.child("View") {
        fx.created = list(&view, "WhenCreated");
        fx.destroyed = list(&view, "WhenDestroyed");
        fx.selected = list(&view, "WhenSelected");
        fx.commanded = list(&view, "WhenCommanded");
        fx.commanded_aggressive = list(&view, "WhenCommandedAggressive");
        fx.constructed = list(&view, "WhenConstructed");
        let prefix = text_with_folder(&view, "resource_prefix").map(|(v, folder)| relative(&folder, &v));
        for aspect in view.children("Aspect").unwrap_or_default() {
            let ids = text(&aspect, "IDS").unwrap_or_default();
            if ids.eq_ignore_ascii_case("aspect_death")
                && let (Some(model), Some(prefix)) = (text(&aspect, "model"), prefix.as_ref())
            {
                let expanded = model.replacen("%s", prefix, 1);
                fx.death_model = Some(expanded.replace('\\', "/").trim_start_matches('/').to_owned());
            }
        }
        fx.resource_prefix = prefix;
    }
    if let Ok(Some(body)) = actor.child("BodyComponent") {
        fx.health = list(&body, "WhenHealth");
    }
    if let Ok(Some(mover)) = actor.child("MoverComponent") {
        fx.start_moving = list(&mover, "WhenStartMoving");
        fx.stop_moving = list(&mover, "WhenStopMoving");
    }
    Some(fx)
}

fn vfx_def(node: &ResolvedNode<'_>) -> VfxDef {
    let flags = node.child("Flags").ok().flatten();
    let flag_any = |name: &str| flag(node, name).or_else(|| flags.as_ref().and_then(|f| flag(f, name)));
    VfxDef {
        category: text(node, "category").unwrap_or_default().to_ascii_uppercase(),
        model: text_with_folder(node, "model").map(|(v, f)| relative(&f, &v).to_ascii_lowercase()).unwrap_or_default(),
        permanent: flag_any("permanent").unwrap_or(false),
        critical: flag_any("critical").unwrap_or(true),
        duration: num(node, "duration").unwrap_or(0.0),
        gyroscope: flag_any("gyroscope").unwrap_or(true),
        attached_scale: num(node, "attached_scale").unwrap_or(0.0),
        random_orientation: flag_any("random_orientation").unwrap_or(false),
        nightonly: flag_any("nightonly").unwrap_or(false),
        color: p3(node, "color", [1.0; 3]),
        speed: num(node, "speed").unwrap_or(0.0),
        length: num(node, "length").unwrap_or(0.0),
        spread: p3(node, "spread", [0.0; 3]),
        texture: text_with_folder(node, "texture").map(|(v, f)| relative(&f, &v)).unwrap_or_default(),
        texture_size: num(node, "texture_size").unwrap_or(0.0),
        decal_fade: p2(node, "decal_fade_time", [0.0; 2]),
        additive: flag_any("additive").unwrap_or(false),
        decal_type: text(node, "decal_type").unwrap_or_default().to_ascii_lowercase(),
        rotate_dps: num(node, "rotate_degrees_per_sec").unwrap_or(0.0),
        respect_lighting: flag_any("respect_lighting").unwrap_or(false),
    }
}

const CONTROL: [&str; 7] = ["INTERRUPT", "SEQUENTIAL", "ALL", "VOICE", "WORLD", "CAUSEFADE", "TESTSOUND"];
const WORLD: [&str; 2] = ["GLOBAL", "FOGGED"];
const WHO: [&str; 6] = ["LOCAL", "PLAYER", "ALLIES", "NEUTRALS", "ENEMIES", "EVERYONE"];
const PRIORITY: [&str; 5] = ["LOWEST", "LOW", "MEDIUM", "HIGH", "CRITICAL"];
const CATEGORY: [&str; 3] = ["SOUND", "MUSIC", "SPEECH"];

fn ordinal(names: &[&str], v: &str) -> i32 {
    names.iter().position(|n| n.eq_ignore_ascii_case(v)).map_or(-1, |i| i as i32)
}
/// 46C1C0: tokens split on ',' / ' ', each known name ORs 1 << ordinal.
fn flags(names: &[&str], v: &str) -> u8 {
    let mut byte = 0u8;
    for token in v.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
        let i = ordinal(names, token);
        if i >= 0 {
            byte |= 1u8 << i;
        }
    }
    byte
}

fn audio_def(node: &ResolvedNode<'_>, ids: &str) -> AudioDef {
    let files = |name: &str| -> Vec<String> {
        let Some((v, folder)) = text_with_folder(node, name) else { return Vec::new() };
        v.split([',', ' ', '\t']).filter(|t| !t.is_empty()).map(|t| relative(&folder, t)).collect()
    };
    let fl = |name: &str, names: &[&str]| text(node, name).map_or(0, |v| flags(names, &v));
    AudioDef {
        ids: ids.to_ascii_lowercase(),
        category: text(node, "category").map_or(-1, |c| ordinal(&CATEGORY, &c)),
        priority: text(node, "priority").map_or(-1, |c| ordinal(&PRIORITY, &c)),
        control: fl("control_flags", &CONTROL),
        world: fl("world_flags", &WORLD),
        who: fl("who_flags", &WHO),
        volume: num(node, "volume").unwrap_or(0.8),
        range: p2(node, "range", [4.0, 64.0]),
        simultaneous_limit: num(node, "simultaneous_limit").map_or(2, |v| v as i32),
        leadin: files("leadin"),
        files: files("files"),
        fallout: files("fallout"),
        fade_time_s: p2(node, "fade_time_s", [0.0; 2]),
        delay_ms: p2(node, "delay_ms", [0.0; 2]),
        pitch_adjust: p2(node, "pitch_adjust", [0.0; 2]),
        volume_adjust: num(node, "volume_adjust").unwrap_or(0.0),
    }
}

fn ambient_def(actor: &ResolvedNode<'_>) -> Option<AmbientDef> {
    let view = actor.child("View").ok().flatten()?;
    let mut out = AmbientDef::default();
    let aspects = view.children("Aspect").unwrap_or_default();
    let normal = view.child("AspectNormal").ok().flatten();
    for aspect in aspects.iter().chain(normal.iter()) {
        for anim in aspect.children("Animation").unwrap_or_default() {
            for sfx in anim.children("PlaySFX").unwrap_or_default() {
                if let Some(id) = text(&sfx, "sfx_ids") {
                    out.sounds.push(id.to_ascii_lowercase());
                }
            }
            if text(&anim, "type").is_some_and(|t| t.eq_ignore_ascii_case("RANDOM")) {
                out.start = [num(&anim, "start_min").unwrap_or(0.0), num(&anim, "start_max").unwrap_or(0.0)];
            }
        }
    }
    if out.sounds.is_empty() {
        return None;
    }
    for component in actor.children("Component").unwrap_or_default() {
        if text(&component, "type").is_some_and(|t| t.eq_ignore_ascii_case("DISTURBABLE")) {
            out.disturb_radius = num(&component, "affect_radius").unwrap_or(0.0);
            out.disturb_recharge = [num(&component, "recharge_time_min").unwrap_or(0.0), num(&component, "recharge_time_max").unwrap_or(0.0)];
            out.disturb_update = num(&component, "update_time").unwrap_or(0.0);
        }
    }
    Some(out)
}

fn ids_of(node: &ResolvedNode<'_>) -> Option<String> {
    text(node, "IDS").map(|s| s.to_ascii_lowercase())
}

/// Read every table from an opened catalog. `actor_ids` are the Things to
/// extract (GameData defs plus projectiles and audio doodads).
pub fn extract(catalog: &ActorCatalog, actor_ids: &[String]) -> FxTables {
    let mut t = FxTables::default();
    for id in actor_ids {
        match actor_fx(catalog, id) {
            Some(fx) => {
                t.actors.insert(id.to_ascii_lowercase(), fx);
            }
            None => t.notes.push(format!("actor {id}: not resolved")),
        }
    }
    // Projectiles named by any weapon.
    let projectiles: Vec<String> = t.actors.values().flat_map(|a| a.weapons.iter().filter_map(|w| w.projectile.clone())).collect();
    for p in projectiles {
        if !t.actors.contains_key(&p) {
            if let Some(fx) = actor_fx(catalog, &p) {
                t.actors.insert(p, fx);
            } else {
                t.notes.push(format!("projectile {p}: not resolved"));
            }
        }
    }
    for tag in ["VFX", "Thing", "Projectile", "Unit"] {
        for node in catalog.resolved_definitions(tag).unwrap_or_default() {
            let Some(ids) = ids_of(&node) else { continue };
            let category = text(&node, "category").unwrap_or_default().to_ascii_uppercase();
            if tag == "VFX" || category.starts_with("VFX") {
                t.vfx.entry(ids).or_insert_with(|| vfx_def(&node));
            } else if tag != "VFX"
                && let Some(a) = ambient_def(&node)
            {
                t.ambient.entry(ids).or_insert(a);
            }
        }
    }
    // Audio: the native registry (first registration kept), else every root.
    let mut audio_done = false;
    if let Some(registry) = catalog.native_data_registry("Audio") {
        for entry in &registry.entries {
            if let Ok(node) = catalog.native_data_node(entry) {
                t.audio.insert(entry.ids.to_ascii_lowercase(), audio_def(&node, &entry.ids));
            }
        }
        audio_done = !t.audio.is_empty();
    }
    if !audio_done {
        for node in catalog.resolved_definitions("Audio").unwrap_or_default() {
            if let Some(ids) = ids_of(&node) {
                let def = audio_def(&node, &ids);
                t.audio.entry(ids).or_insert(def);
            }
        }
    }
    for node in catalog.resolved_definitions("Bone").unwrap_or_default() {
        if let (Some(ids), Some(name)) = (ids_of(&node), text(&node, "bone_name")) {
            t.bones.entry(ids).or_insert(name);
        }
    }
    for node in catalog.resolved_definitions("Terrain").unwrap_or_default() {
        if let (Some(ids), Some(group)) = (ids_of(&node), text(&node, "terrain_group")) {
            t.terrain_groups.insert(ids, group.to_ascii_lowercase());
        }
    }
    for node in catalog.resolved_definitions("Nation").unwrap_or_default() {
        if let (Some(ids), Some(music)) = (ids_of(&node), text(&node, "game_music")) {
            let nation = ids.strip_prefix("nation_").unwrap_or(&ids).to_owned();
            t.nation_music.insert(nation, music.to_ascii_lowercase());
        }
    }
    t
}

fn cache_path(game_root: &Path) -> PathBuf {
    game_root.join("rust-rebuild").join("assets-cache").join("aa-bevy").join("fx.json")
}

/// The tables for `actor_ids`, from the cache when it matches, else from the
/// original catalog (seconds), then cached.
pub fn load(game_root: &Path, actor_ids: &[String]) -> anyhow::Result<FxTables> {
    let mut ids: Vec<String> = actor_ids.iter().map(|s| s.to_ascii_lowercase()).collect();
    ids.sort();
    ids.dedup();
    let key = format!("{CACHE_VERSION}:{}:{}", aa_rules::sources::BASELINE_LEDGER_SHA256, ids.len());
    let path = cache_path(game_root);
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(t) = serde_json::from_slice::<FxTables>(&bytes)
        && t.source_key == key
    {
        return Ok(t);
    }
    let mut sources = aa_play::data::Sources::open(game_root)?;
    let catalog = ActorCatalog::load_v14(&mut sources.files).map_err(|e| anyhow::anyhow!("loading the original rules catalog: {e}"))?;
    let mut t = extract(&catalog, &ids);
    t.source_key = key;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(bytes) = serde_json::to_vec(&t) {
        let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
        if std::fs::write(&tmp, bytes).is_ok() && std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
    Ok(t)
}
