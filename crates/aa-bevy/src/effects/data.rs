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
const CACHE_VERSION: &str = "aa-bevy-fx-8";

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
    /// FXC_Equipment action (HIDE / UNHIDE), upper case.
    #[serde(default)]
    pub action: String,
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
    /// AircraftComponent operational_height (air transports fly at it).
    #[serde(default)]
    pub operational_height: Option<f32>,
    /// What battle_view animates on the model (tires, aspects, trailer,
    /// character animations).
    #[serde(default)]
    pub view: ViewFx,
    /// PerformerComponent Turret list: each turret's turret_ids (lower case),
    /// in the order Weapon::turret indexes.
    #[serde(default)]
    pub turrets: Vec<String>,
    /// ProjectileComponent (projectiles): projectile_spread, the radius
    /// around the sim hit point where the view's projectile lands.
    #[serde(default)]
    pub projectile_spread: f32,
}

/// The original's view data of one Thing that battle_view plays.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ViewFx {
    /// MoverComponent Tire list: the bone IDS (lower case) and FRONT.
    pub tires: Vec<(String, bool)>,
    /// MoverComponent tire_circumference (metres per turn).
    pub tire_circumference: f32,
    /// MoverComponent trailer_actor (lower case) and TrailerHitch
    /// (node IDS lower case, node_offset).
    pub trailer: Option<String>,
    pub hitch: Option<(String, [f32; 3])>,
    /// View AspectNormal and Aspect list: IDS (lower case; "aspect_normal"
    /// for AspectNormal) -> model and skin, Data-relative, "%s" expanded.
    pub aspects: Vec<AspectFx>,
    /// CharacterComponent Animation list, in authored order.
    pub animations: Vec<AnimFx>,
    /// The rear corners of the soft shape in data space (x right, y
    /// forward): RECT/SQUARE (minX, minY), (maxX, minY), else the blocking
    /// radius (-r, -r), (r, -r) (trailers, `45CC40`).
    #[serde(default)]
    pub rear: [[f32; 2]; 2],
    /// CharacterComponent AspectAnimationSet lists: aspect IDS (lower case)
    /// -> the set's own animations, then every base animation whose IDS it
    /// does not name (464BD0 / 463F10 merge).
    #[serde(default)]
    pub anim_sets: Vec<(String, Vec<AnimFx>)>,
    /// The Stance list in authored order (45E620 takes the last that applies).
    #[serde(default)]
    pub stances: Vec<StanceFx>,
    /// View idle_loops_min / idle_loops_max (49B4D0).
    #[serde(default)]
    pub idle_loops: [i32; 2],
    /// Per ability in the weapon order (Performer, then Defender): its
    /// animation1..5 IDS and idle_animation IDS (lower case).
    #[serde(default)]
    pub abilities: Vec<AbilityAnims>,
}

/// One Stance (template_stance.tgi).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StanceFx {
    pub ids: String,
    /// aspect_ids (lower case), None for NULL.
    pub aspect: Option<String>,
    pub health: f32,
    /// actor_state list (lower case), empty for NULL.
    pub states: Vec<String>,
}

/// The animations an Ability plays (template_actorability.tgi).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AbilityAnims {
    pub animations: Vec<String>,
    pub idle: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AspectFx {
    pub ids: String,
    pub model: Option<String>,
    pub skin: Option<String>,
}

/// One CharacterComponent Animation (template_animation.tgi).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnimFx {
    /// MOVE, ABILITY, IDLE, FIDGET, DIE, HIT, SUMMON (upper case).
    pub kind: String,
    pub ids: String,
    /// The KF file, Data-relative.
    pub kf: String,
    /// Desired gameplay length (s), walk_distance (m per cycle), key_ratio.
    pub length: f32,
    pub walk_distance: f32,
    pub key_ratio: f32,
    pub weight: f32,
    pub damage_threshold: f32,
    pub damage_filter: i32,
    pub start: Vec<Fxc>,
    pub do_: Vec<Fxc>,
    pub end: Vec<Fxc>,
    /// WhenDoAtTime (each with its `time`).
    pub at_time: Vec<Fxc>,
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
    /// The generals' faction abilities (Faction Ability -> AbilityDefinition):
    /// definition IDS (lower case) -> its FX lists (WhenDo...).
    #[serde(default)]
    pub special_ops: BTreeMap<String, WeaponFx>,
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
        action: text(n, "action").unwrap_or_default().to_ascii_uppercase(),
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
    if let Ok(Some(air)) = actor.child("AircraftComponent") {
        fx.operational_height = num(&air, "operational_height");
    }
    if let Ok(Some(performer)) = actor.child("PerformerComponent") {
        fx.turrets = performer.children("Turret").unwrap_or_default().iter().map(|t| text(t, "turret_ids").unwrap_or_default().to_ascii_lowercase()).collect();
    }
    if let Ok(Some(p)) = actor.child("ProjectileComponent") {
        fx.projectile_spread = num(&p, "projectile_spread").unwrap_or(0.0);
    }
    view_fx(&actor, &mut fx);
    Some(fx)
}

/// Expand an aspect / animation resource: "%s" takes the resource prefix
/// (Data-relative already), "/" starts at the Data root, otherwise the
/// value is relative to the file that assigned it.
fn resource(value: &str, folder: &str, prefix: Option<&str>, second: &str) -> Option<String> {
    let v = value.replace('\\', "/");
    if v.contains("%s") {
        let prefix = prefix?;
        let once = v.replacen("%s", prefix, 1);
        return Some(once.replacen("%s", second, 1).trim_start_matches('/').to_owned());
    }
    Some(relative(folder, &v))
}

fn view_fx(actor: &ResolvedNode<'_>, fx: &mut ActorFx) {
    let prefix = fx.resource_prefix.clone();
    if let Ok(Some(mover)) = actor.child("MoverComponent") {
        fx.view.tire_circumference = num(&mover, "tire_circumference").unwrap_or(1.0);
        for t in mover.children("Tire").unwrap_or_default() {
            if let Some(bone) = text(&t, "tire_ids") {
                let front = text(&t, "type").is_some_and(|k| k.eq_ignore_ascii_case("front"));
                fx.view.tires.push((bone.to_ascii_lowercase(), front));
            }
        }
        fx.view.trailer = text(&mover, "trailer_actor").map(|t| t.to_ascii_lowercase());
        if let Ok(Some(h)) = mover.child("TrailerHitch")
            && let Some(bone) = text(&h, "node_ids")
        {
            fx.view.hitch = Some((bone.to_ascii_lowercase(), p3(&h, "node_offset", [0.0; 3])));
        }
    }
    if let Ok(Some(size)) = actor.child("Size") {
        let shape = text(&size, "soft_shape").unwrap_or_default().to_ascii_uppercase();
        let radius = num(&size, "blocking_radius").unwrap_or(0.5);
        fx.view.rear = if shape == "RECT" || shape == "SQUARE" {
            let w = num(&size, "rect_width").unwrap_or(radius * 2.0) * 0.5;
            let h = num(&size, "rect_height").unwrap_or(radius * 2.0) * 0.5;
            [[-w, -h], [w, -h]]
        } else {
            [[-radius, -radius], [radius, -radius]]
        };
    }
    if let Ok(Some(view)) = actor.child("View") {
        let mut aspects: Vec<(String, ResolvedNode<'_>)> = Vec::new();
        if let Ok(Some(normal)) = view.child("AspectNormal") {
            aspects.push(("aspect_normal".into(), normal));
        }
        for a in view.children("Aspect").unwrap_or_default() {
            let ids = text(&a, "IDS").unwrap_or_default().to_ascii_lowercase();
            aspects.push((ids, a));
        }
        for (ids, a) in aspects {
            let aspect_prefix = text(&a, "aspect_prefix").unwrap_or_default();
            let full = prefix.as_ref().map(|p| format!("{p}{aspect_prefix}"));
            let field = |name: &str| text_with_folder(&a, name).and_then(|(v, folder)| resource(&v, &folder, full.as_deref(), ""));
            fx.view.aspects.push(AspectFx { ids, model: field("model"), skin: field("skin") });
        }
    }
    if let Ok(Some(view)) = actor.child("View") {
        fx.view.idle_loops = [num(&view, "idle_loops_min").map_or(1, |v| v as i32), num(&view, "idle_loops_max").map_or(3, |v| v as i32)];
    }
    if let Ok(Some(character)) = actor.child("CharacterComponent") {
        for a in character.children("Animation").unwrap_or_default() {
            fx.view.animations.push(anim_fx(&a, prefix.as_deref()));
        }
        // 464BD0: one more set per AspectAnimationSet, loaded with the
        // aspect's own prefix; 463F10 then adds the base animations whose IDS
        // the set does not have.
        for set in character.children("AspectAnimationSet").unwrap_or_default() {
            let Some(aspect) = text(&set, "aspect_ids").map(|s| s.to_ascii_lowercase()) else { continue };
            let aspect_prefix = actor.child("View").ok().flatten().and_then(|view| {
                let node = view.children("Aspect").unwrap_or_default().into_iter().find(|n| text(n, "IDS").is_some_and(|i| i.eq_ignore_ascii_case(&aspect)))?;
                text(&node, "aspect_prefix")
            });
            let set_prefix = prefix.as_ref().map(|p| format!("{p}{}", aspect_prefix.unwrap_or_default()));
            let mut rows: Vec<AnimFx> = set.children("Animation").unwrap_or_default().iter().map(|a| anim_fx(a, set_prefix.as_deref())).collect();
            for base in &fx.view.animations {
                if !rows.iter().any(|r| r.ids == base.ids) {
                    rows.push(base.clone());
                }
            }
            if !fx.view.anim_sets.iter().any(|(a, _)| *a == aspect) {
                fx.view.anim_sets.push((aspect, rows));
            }
        }
    }
    for s in actor.children("Stance").unwrap_or_default() {
        fx.view.stances.push(StanceFx {
            ids: text(&s, "ids").or_else(|| text(&s, "IDS")).unwrap_or_default().to_ascii_lowercase(),
            aspect: text(&s, "aspect_ids").map(|a| a.to_ascii_lowercase()),
            health: num(&s, "health").unwrap_or(1.0),
            states: text(&s, "actor_state").map_or_else(Vec::new, |v| v.split([',', ' ', '\t']).filter(|t| !t.is_empty() && !t.eq_ignore_ascii_case("null")).map(|t| t.to_ascii_lowercase()).collect()),
        });
    }
    for component in ["PerformerComponent", "DefenderComponent"] {
        if let Ok(Some(c)) = actor.child(component) {
            for ability in c.children("Ability").unwrap_or_default() {
                let animations = (1..=5).filter_map(|i| text(&ability, &format!("animation{i}"))).map(|s| s.to_ascii_lowercase()).collect();
                fx.view.abilities.push(AbilityAnims { animations, idle: text(&ability, "idle_animation").map(|s| s.to_ascii_lowercase()) });
            }
        }
    }
}

/// One Animation node (template_animation.tgi; the kf expanded with the
/// prefix of the set that loads it).
fn anim_fx(a: &ResolvedNode<'_>, prefix: Option<&str>) -> AnimFx {
    let ids = text(a, "IDS").unwrap_or_default();
    let kf = text_with_folder(a, "kf").and_then(|(v, folder)| resource(&v, &folder, prefix, &ids)).unwrap_or_default();
    AnimFx {
        kind: text(a, "type").unwrap_or_default().to_ascii_uppercase(),
        ids: ids.to_ascii_lowercase(),
        kf,
        length: num(a, "length").unwrap_or(1.0),
        walk_distance: num(a, "walk_distance").unwrap_or(1.0),
        key_ratio: num(a, "key_ratio").unwrap_or(0.5),
        weight: num(a, "weight").unwrap_or(1.0),
        damage_threshold: num(a, "damage_threshold").unwrap_or(0.0),
        damage_filter: num(a, "damage_filter").map_or(-1, |f| f as i32),
        start: list(a, "WhenStart"),
        do_: list(a, "WhenDo"),
        end: list(a, "WhenEnd"),
        at_time: list(a, "WhenDoAtTime"),
    }
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
    // Projectiles named by any weapon, and the trucks' trailers.
    let projectiles: Vec<String> = t.actors.values().flat_map(|a| a.weapons.iter().filter_map(|w| w.projectile.clone()).chain(a.view.trailer.clone())).collect();
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
    // The generals' abilities (Data/Generals Faction roots, their Ability
    // children naming a shared AbilityDefinition or holding their own).
    for faction in catalog.resolved_definitions("Faction").unwrap_or_default() {
        for ability in faction.children("Ability").unwrap_or_default() {
            let definition = match ability.child("AbilityDefinition") {
                Ok(Some(local)) => Some(local),
                _ => text(&ability, "ability_IDS").and_then(|id| catalog.resolved_definition("AbilityDefinition", &id, PC).ok()),
            };
            let Some(ids) = definition.as_ref().and_then(|d| text(d, "IDS")) else { continue };
            let fx = weapon_fx(catalog, &ability);
            t.special_ops.entry(ids.to_ascii_lowercase()).or_insert(fx);
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

/// `assets-cache/aa-bevy` of this checkout (found from the executable or
/// the working directory), else below the install's rust-rebuild folder.
pub fn cache_dir(game_root: &Path) -> PathBuf {
    let mut starts = vec![];
    if let Ok(exe) = std::env::current_exe() {
        starts.push(exe);
    }
    if let Ok(cwd) = std::env::current_dir() {
        starts.push(cwd);
    }
    starts
        .iter()
        .flat_map(|s| s.ancestors())
        .map(|a| a.join("assets-cache"))
        .find(|p| p.is_dir())
        .unwrap_or_else(|| game_root.join("rust-rebuild").join("assets-cache"))
        .join("aa-bevy")
}

fn cache_path(game_root: &Path) -> PathBuf {
    cache_dir(game_root).join("fx.json")
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
