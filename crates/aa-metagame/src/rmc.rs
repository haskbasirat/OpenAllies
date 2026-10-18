//! Random-map configurations (`KKC_RMCData`, `.\Source\Data\RMCData.cpp`):
//! the original RMC loader, used by the metagame battle launcher (RMC of a
//! territory, its kingdom slots, sliders and map sizes) and later by the
//! skirmish random map.
//!
//! | native | role |
//! |---|---|
//! | 4A0B60 | RMCData constructor (vtable A15058) |
//! | 4A5420 | `Load` (vtable slot 0): fields, map sizes, then the sub-loaders below, then `[Heroes]` |
//! | 47D960 | base data load: localized `name`, loaded flag |
//! | 59E760 / 59E7A0 / 59E230 | game settings (+0x90) name / description / file word |
//! | 4A19D0 | `LoadRules`: `game_type` (enum B3B1A4), world rule defaults (511CE0, 511A60, 473140/473170/4733C0/473440) |
//! | 4A4410 | terrain sliders (+0x50): `TerrainBlob` 4A0720/4A2EC0, `TerrainStreak` 4A07D0/4A2FD0, base 4A2B70, slider 4A0470, MicroBlobs 4A0510, dir_bias 409230, biome check 4B6D20, override_same map 683BD0/4A0FF0 |
//! | 4A1DB0 / 4B6A00 | terrain noise (+0x5C) |
//! | 4A4EB0 | kingdoms: kingdom infos (settings+8 +0x10, 50F4C0/4A4DC0), major slots +0xF8 (4A3FE0), independent slots +0x104 (4A3FD0 = 4A3A00), relations 4A3C10/4A2AB0, KingdomsMax |
//! | 4A4600 | object groups: `DoodadGroup` +0x68 (4A1060/4A31A0), `ActorGroup` +0x74 (4A1320/4A3590), apply_to_all_major_kingdoms expansion |
//! | 4A31A0 / 4A30F0 / 4A3060 / 4B6BA0 / 4A2150 / 4A0880 | object group, required objects, objects, weighted list, group restrictions, group lookup |
//! | 4A1CD0 / 5103D0 | teams (settings+8 +0x04) |
//! | 424FA0 / 412480 | comma tokenizer (flags 0) |
//! | 4994D0 / 4995A0 / 499840 / 4A1F80 (4A18D0) / 4A1EA0 | enum tables game_type / brush_type / relation / attachto / kingdom type |
//!
//! The loader consumes resolved TGI nodes ([`RmcNodeData`]) and engine
//! registries ([`RmcRegistries`]). Both are exported from the original
//! sources through aa-rules ([`export_v14`]); the original loader is proven
//! on the same exported nodes by `research/phase8-metagame/native_rmc.py`
//! (native-rmc.json, `tests/native_rmc.rs`).
//!
//! Node contract (the getter seams of the oracle implement the same rules):
//! field and child names are ASCII case-insensitive; a required getter on an
//! absent field fails with "Node doesn't contain field. (name)"; a required
//! absent sub-node with "SubNode not found. (name)"; the fixed getter accepts
//! fixed fields and int fields (exact conversion); every other cross-type
//! read is [`Error::Unsupported`]. Localized fields hold the TGI text (the
//! string-table translation 4B9F40 of `#keys` is not ported).
//!
//! Not ported (explicit): the `[Stockpile]` loader of a major kingdom (473B10,
//! PurchaseResource), the WorldRule registry (its loader 48A720; a caller may
//! supply rule definitions, otherwise rule rows are recorded unresolved), the
//! settings `+0x14` word (the TGI file object's `+0x40`), the localization
//! registration of 49AD00/49AE00 (only key and text are kept), and the Biome
//! loader (479080) that builds the biome terrain list (+0x6C) read by 4B6D20.
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn native(message: String) -> Error {
    Error::NativeData(message)
}
fn unsupported<T>(what: impl Into<String>) -> Result<T> {
    Err(Error::Unsupported(what.into()))
}

// ---------------------------------------------------------------- nodes

/// A typed TGI field value as the native field object holds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
pub enum FieldValue {
    String(String),
    /// A `localized` field: the stored TGI text (key or literal).
    Localized(String),
    Int(i32),
    /// binary32 bits of a `fixed` field.
    Fixed(u32),
    /// binary32 bits of a `float` field (native GetFixed rejects it).
    Float(u32),
    Bool(bool),
    /// binary32 bits of a point field.
    Point(Vec<u32>),
    /// A field the resolver could not type (message); reading it is unsupported.
    Unsupported(String),
}

/// A resolved TGI node: label (node name), typed fields (lower-case names)
/// and children in native order (`node+0x34` list).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcNodeData {
    pub label: String,
    pub fields: BTreeMap<String, FieldValue>,
    pub children: Vec<RmcNodeData>,
}

impl RmcNodeData {
    fn get(&self, name: &str) -> Option<&FieldValue> {
        self.fields.get(&name.to_ascii_lowercase())
    }
    fn missing(&self, name: &str) -> Error {
        native(format!("Node doesn't contain field. ({name})"))
    }
    fn typed<T>(&self, name: &str, what: &str, f: impl Fn(&FieldValue) -> Option<T>) -> Result<T> {
        let v = self.get(name).ok_or_else(|| self.missing(name))?;
        if let FieldValue::Unsupported(m) = v {
            return unsupported(format!("field {name} of {}: {m}", self.label));
        }
        f(v).ok_or_else(|| Error::Unsupported(format!("{what} getter on field {name} of {}: {v:?}", self.label)))
    }
    /// `5B54D0` string getter.
    pub fn string(&self, name: &str) -> Result<String> {
        self.typed(name, "string", |v| match v {
            FieldValue::String(s) => Some(s.clone()),
            _ => None,
        })
    }
    /// `5B5620` localized getter (the stored text; see the module doc).
    pub fn localized(&self, name: &str) -> Result<String> {
        self.typed(name, "localized", |v| match v {
            FieldValue::Localized(s) => Some(s.clone()),
            _ => None,
        })
    }
    /// `5B4ED0` int getter.
    pub fn int(&self, name: &str) -> Result<i32> {
        self.typed(name, "int", |v| match v {
            FieldValue::Int(i) => Some(*i),
            _ => None,
        })
    }
    /// `5B5130` fixed getter (binary32 bits): fixed, or an int field (exact).
    pub fn fixed(&self, name: &str) -> Result<u32> {
        let v = self.typed(name, "fixed", |v| match v {
            FieldValue::Fixed(b) => Some(Ok(*b)),
            FieldValue::Int(i) => Some(Binary::from_i32(*i).store_f32()),
            _ => None,
        })?;
        Ok(v?)
    }
    /// `5B5380` bool getter.
    pub fn boolean(&self, name: &str) -> Result<bool> {
        self.typed(name, "bool", |v| match v {
            FieldValue::Bool(b) => Some(*b),
            _ => None,
        })
    }
    /// `5B4D80` point2 getter.
    pub fn point2(&self, name: &str) -> Result<[u32; 2]> {
        self.typed(name, "point2", |v| match v {
            FieldValue::Point(p) if p.len() == 2 => Some([p[0], p[1]]),
            _ => None,
        })
    }
    /// `5B4220` + `42BE10`: the children with this label, in order.
    pub fn list(&self, name: &str) -> Vec<&RmcNodeData> {
        self.children.iter().filter(|c| c.label.eq_ignore_ascii_case(name)).collect()
    }
    /// `5B4410(name, required)`.
    pub fn subnode(&self, name: &str, required: bool) -> Result<Option<&RmcNodeData>> {
        match self.children.iter().find(|c| c.label.eq_ignore_ascii_case(name)) {
            Some(c) => Ok(Some(c)),
            None if required => Err(native(format!("SubNode not found. ({name})"))),
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------- tokenizer

/// CRT `iswspace` for the token start (C locale: 0x09..0x0D, 0x20).
fn iswspace(c: u16) -> Result<bool> {
    if c >= 0x80 {
        return unsupported(format!("iswspace of non-ASCII U+{c:04X} (CRT locale table)"));
    }
    Ok(matches!(c, 0x09..=0x0d | 0x20))
}

/// `424FA0(text, ",", 0)` followed by `412480` until an empty token
/// (flags 0: leading whitespace and delimiters skipped, quoted tokens, no
/// case folding; a token keeps its trailing whitespace).
pub fn tokens(text: &str) -> Result<Vec<String>> {
    let s: Vec<u16> = text.encode_utf16().collect();
    let delim = |c: u16| c == u16::from(b',');
    let at = |p: usize| if p < s.len() { s[p] } else { 0 };
    let mut pos = 0usize;
    let mut out = Vec::new();
    loop {
        if pos >= s.len() {
            return Ok(out);
        }
        while iswspace(s[pos])? {
            pos += 1;
            if pos >= s.len() {
                return Ok(out);
            }
        }
        while delim(s[pos]) {
            pos += 1;
            if pos >= s.len() {
                return Ok(out);
            }
        }
        let c = s[pos];
        let mut tok: Vec<u16> = Vec::new();
        let mut finished = false;
        if c == u16::from(b'"') || c == u16::from(b'\'') {
            loop {
                pos += 1;
                let ch = at(pos);
                if ch == c {
                    break;
                }
                if pos >= s.len() {
                    finished = true;
                    break;
                }
                tok.push(ch);
            }
        } else {
            let mut ch = c;
            loop {
                tok.push(ch);
                pos += 1;
                let next = at(pos);
                if next != 0 && delim(next) {
                    break;
                }
                if pos >= s.len() {
                    finished = true;
                    break;
                }
                ch = next;
            }
        }
        if !finished {
            pos += 1; // 730520
        }
        if tok.is_empty() {
            return Ok(out);
        }
        out.push(String::from_utf16(&tok).map_err(|_| Error::Unsupported("token is not UTF-16".into()))?);
        if finished {
            return Ok(out);
        }
    }
}

// ---------------------------------------------------------------- enum tables

/// `4994D0` / 4981A0.
pub const GAME_TYPE: &[(&str, u32)] = &[
    ("playable", 0),
    ("cutscene", 1),
    ("savegame", 2),
    ("film", 3),
    ("savegame_metagame_rts", 4),
    ("savegame_metagame_map", 5),
    ("savegame_campaign", 6),
];
/// `4995A0`.
pub const BRUSH_TYPE: &[(&str, u32)] = &[("BLOB", 0), ("STREAK", 1)];
/// `499840`.
pub const RELATION: &[(&str, u32)] = &[("SELF", 0), ("ALLY", 1), ("NEUTRAL", 2), ("ENEMY", 3)];
/// `4A1F80` / 4A18D0.
pub const ATTACH_TO: &[(&str, u32)] = &[
    ("NONE", 0),
    ("SOUTHWEST", 1),
    ("WEST", 2),
    ("NORTHWEST", 3),
    ("NORTH", 4),
    ("NORTHEAST", 5),
    ("EAST", 6),
    ("SOUTHEAST", 7),
    ("SOUTH", 8),
    ("CENTER", 9),
    ("RANDOM", 10),
];
/// `4A1EA0`: RMC kingdom `type`.
pub const KINGDOM_TYPE: &[(&str, u32)] = &[("land", 0), ("water", 1)];

/// `497220`: case-insensitive (wcsicmp) enum lookup.
fn enum_lookup(table: &[(&str, u32)], name: &str) -> Option<u32> {
    table.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| *v)
}
fn enum_required(table: &[(&str, u32)], name: &str) -> Result<u32> {
    enum_lookup(table, name).ok_or_else(|| native(format!("Unknown IDS '{name}' specified in enum table")))
}

// ---------------------------------------------------------------- registries

/// A generic DataManager (48D500 Find): display name (+0x20) and the
/// registered IDS in native order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagerData {
    pub name: String,
    pub entries: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThingEntry {
    pub ids: String,
    /// Thing data vtable+0x38 (actor).
    pub is_actor: bool,
    /// Thing data flags `+0x1C` bit 23 (OWNABLE); `None` when not derived.
    pub ownable: Option<bool>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyEntry {
    pub ids: String,
    /// Property data byte `+0x5C` (technology); `None` when not derived.
    pub technology: Option<bool>,
}
/// A WorldRule definition as the RMC rule table reads it (rule `+0x20` type,
/// `+0x40` default word, `+0x34` enum value names).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRuleDef {
    pub ids: String,
    pub kind: u32,
    pub default_word: u32,
    pub enum_values: Vec<String>,
}

/// Engine registries consulted by the RMC loader.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcRegistries {
    /// Generic managers by tag: Nation (GameData+5C), Faction (+A8),
    /// KingdomColor (+3CC), DifficultyLevel (+140), Biome (+5C0),
    /// Terrain (+60C), Weather (+6F0), Sun (+73C).
    pub managers: BTreeMap<String, ManagerData>,
    /// Biome `+0x6C` terrain list (canonical terrain IDS) per canonical biome IDS.
    pub biome_terrains: BTreeMap<String, Vec<String>>,
    /// Thing DataManager (GameData+C, 427BD0) entries referenced by RMCs.
    pub things: Vec<ThingEntry>,
    pub thing_manager_name: String,
    /// Property manager (GameData+464, 463FA0) entries referenced by RMCs.
    pub properties: Vec<PropertyEntry>,
    pub property_manager_name: String,
    /// SVars `KingdomsMax` (4BE100/4BE120).
    pub kingdoms_max: i32,
    /// Resource registry count (B36530+38): the kingdom info stockpile size.
    pub resource_count: u32,
    /// WorldRule registry in native order; `None` = not provided.
    pub world_rules: Option<Vec<WorldRuleDef>>,
}

impl RmcRegistries {
    fn manager(&self, tag: &str) -> Result<&ManagerData> {
        self.managers
            .get(tag)
            .ok_or_else(|| Error::Unsupported(format!("registry {tag} not provided")))
    }
    /// `48D500` on a manager: canonical IDS or `None`.
    pub fn find(&self, tag: &str, ids: &str) -> Result<Option<String>> {
        if ids.is_empty() {
            return Ok(None);
        }
        Ok(self.manager(tag)?.entries.iter().find(|e| e.eq_ignore_ascii_case(ids)).cloned())
    }
    /// The common caller pattern: empty -> NULL, unknown -> "Unknown IDS".
    fn find_checked(&self, tag: &str, ids: &str) -> Result<Option<String>> {
        if ids.is_empty() {
            return Ok(None);
        }
        match self.find(tag, ids)? {
            Some(v) => Ok(Some(v)),
            None => Err(native(format!(
                "Unknown IDS '{ids}' for data manager '{}'",
                self.manager(tag)?.name
            ))),
        }
    }
    fn thing(&self, ids: &str) -> Option<&ThingEntry> {
        self.things.iter().find(|t| t.ids.eq_ignore_ascii_case(ids))
    }
    /// 427BD0 with the caller's "Unknown IDS" check.
    fn thing_checked(&self, ids: &str) -> Result<Option<&ThingEntry>> {
        if ids.is_empty() {
            return Ok(None);
        }
        match self.thing(ids) {
            Some(t) => Ok(Some(t)),
            None => Err(native(format!(
                "Unknown IDS '{ids}' for data manager '{}'",
                self.thing_manager_name
            ))),
        }
    }
    fn property_checked(&self, ids: &str) -> Result<Option<&PropertyEntry>> {
        if ids.is_empty() {
            return Ok(None);
        }
        match self.properties.iter().find(|p| p.ids.eq_ignore_ascii_case(ids)) {
            Some(p) => Ok(Some(p)),
            None => Err(native(format!(
                "Unknown IDS '{ids}' for data manager '{}'",
                self.property_manager_name
            ))),
        }
    }
}

// ---------------------------------------------------------------- records

/// A localized object (49AD00/49AE00): key (context) and text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Localized {
    pub key: String,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcMapSize {
    pub width_bits: u32,
    pub height_bits: u32,
    pub recommended_kingdoms_min: i32,
    pub recommended_kingdoms_max: i32,
}

/// `4A0470` slider option (RMCSliderOption).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliderOption {
    pub range: [u32; 2],
    pub area_factor_bits: u32,
    pub show_ui: bool,
    pub locked: bool,
}

/// `4A0510`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroBlobs {
    pub ratio_range: [u32; 2],
    pub blob_size: [u32; 2],
    pub consistency_bits: u32,
    pub proximity_bits: u32,
    pub fuzzy_factor_bits: u32,
    pub affect_height: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "brush", rename_all = "snake_case")]
pub enum TerrainShape {
    /// 4A2EC0.
    Blob {
        blob_size: [u32; 2],
        consistency_bits: u32,
        proximity_bits: u32,
        fuzzy_factor_bits: u32,
        dir_bias: [u32; 2],
        dir_bias_factor_bits: u32,
        micro_blobs: Option<MicroBlobs>,
    },
    /// 4A2FD0.
    Streak {
        streaks_min: i32,
        streaks_max: i32,
        length_range: [u32; 2],
        width_range: [u32; 2],
        angle_range: [u32; 2],
    },
}

/// A terrain slider (RMC +0x50 entry).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainSlider {
    pub slider: SliderOption,
    /// `+0x14`: canonical terrain IDS.
    pub terrain: String,
    pub brush_type: u32,
    pub paint_first_variation_only: bool,
    pub attachto: Vec<u32>,
    pub attach_range_bits: u32,
    pub override_same: bool,
    pub shape: TerrainShape,
}

/// `4B6A00` NoiseData.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoiseData {
    pub num_blobs_min: i32,
    pub num_blobs_max: i32,
    pub blob_size_min_bits: u32,
    pub blob_size_max_bits: u32,
    pub offset_magnitude_min_bits: u32,
    pub offset_magnitude_max_bits: u32,
    pub up_frequency_bits: u32,
    pub reference_size_bits: u32,
    pub area_factor_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredObject {
    pub object: String,
    pub position: [u32; 2],
    pub distance_max_bits: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupObject {
    pub object: String,
    pub weight_bits: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupRestriction {
    /// Target group IDS.
    pub group: String,
    pub proximity_min_bits: u32,
    pub proximity_desired_bits: u32,
}

/// ActorGroup extension (4A1320 / 4A3590).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorGroupData {
    pub kingdom_ids: String,
    pub apply_to_all_major_kingdoms: bool,
    pub difficulty_level: Option<String>,
    pub properties: Vec<Option<String>>,
}

/// An object group (DoodadGroup 4A1060/4A31A0; ActorGroup adds `actor`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectGroup {
    pub ids: String,
    pub name: String,
    pub slider: SliderOption,
    pub objects_min: i32,
    pub required: Vec<RequiredObject>,
    pub objects: Vec<GroupObject>,
    /// `+0x44` weighted list from its head (last inserted first), weight > 0 only.
    pub weighted: Vec<GroupObject>,
    pub weighted_total_bits: u32,
    /// `+0x4C`: a group IDS (a group's own IDS for `this`).
    pub search_relative_group: Option<String>,
    /// `+0x50`: an RMC kingdom IDS.
    pub search_relative_kingdom: Option<String>,
    pub search_radius_bits: u32,
    pub balanced_placement: bool,
    pub intraproximity_min_bits: u32,
    pub intraproximity_desired_bits: u32,
    pub starting_location_weight_bits: u32,
    pub restrictions: Vec<GroupRestriction>,
    pub actor: Option<ActorGroupData>,
}

/// Kingdom info (0x6C bytes, settings+8 +0x10 list; 50F4C0 defaults).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomInfo {
    pub ids: String,
    pub name: Localized,
    pub independent: bool,
    pub team: String,
    pub nation: Option<String>,
    pub faction: Option<String>,
    pub color: Option<String>,
    pub playable: bool,
    pub required: bool,
    pub sai_default_ids: String,
    pub technologies: Vec<Option<String>>,
    /// `+0x5C`: per-resource floats, zero (size = resource registry count).
    pub stockpile: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    pub kingdom: String,
    /// Relation enum value (499840), -1 for an unknown name.
    pub relation: i32,
}

/// A kingdom slot (+0xF8 major 0x34 bytes, +0x104 independent 0x20 bytes).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomSlot {
    /// Index into [`RmcSettings::kingdoms`] (slot `+0x0C`).
    pub info: usize,
    /// `+0x1C` type (land 0, water 1).
    pub kind: u32,
    /// `+0x10` relations.
    pub relations: Vec<Relation>,
    /// Major only: `+0x04/+0x08` starting_pos.
    pub starting_pos: Option<[u32; 2]>,
    /// Major only: `+0x24` heroes (Thing IDS).
    pub heroes: Vec<String>,
    /// Major only: `+0x30` random heroes (empty Hero ids).
    pub random_heroes: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleValue {
    pub ids: String,
    pub locked: bool,
    pub value_word: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Team {
    pub ids: String,
    pub name: Localized,
}

/// The game settings object (+0x90) fields the RMC loader writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcSettingsData {
    pub name: Localized,
    pub description: Localized,
    pub ids: String,
    /// `+4 -> +4` world rules game type.
    pub game_type: u32,
    /// `+4` rule table after 511CE0 and the RMC `[Rule]` defaults; `None`
    /// when no WorldRule registry is provided.
    pub rules: Option<Vec<RuleValue>>,
    /// `+8 -> +4` teams.
    pub teams: Vec<Team>,
    /// `+8 -> +0x10` kingdom infos, node order.
    pub kingdoms: Vec<KingdomInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcHeroes {
    pub random_heroes_min: i32,
    pub random_heroes_max: i32,
    pub show_ui: bool,
    pub name: String,
}

/// A loaded RMC (`KKC_RMCData`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcData {
    pub ids: String,
    pub name: String,
    pub list_in_staging: bool,
    pub reference_size_bits: u32,
    pub biome: String,
    pub sun: Vec<Option<String>>,
    pub weather: Vec<Option<String>>,
    pub max_percent_water_bits: u32,
    pub max_percent_mountain_bits: u32,
    pub balanced_kingdoms: bool,
    pub team_desired_distance_bits: u32,
    pub map_sizes: Vec<RmcMapSize>,
    pub terrain_sliders: Vec<TerrainSlider>,
    pub terrain_noise: Vec<NoiseData>,
    pub major_kingdoms: Vec<KingdomSlot>,
    pub independent_kingdoms: Vec<KingdomSlot>,
    pub doodad_groups: Vec<ObjectGroup>,
    pub actor_groups: Vec<ObjectGroup>,
    pub heroes: RmcHeroes,
    pub settings: RmcSettingsData,
    /// Non-fatal native log lines (404ED0), in order.
    pub log: Vec<String>,
}

impl RmcData {
    /// The kingdom slot named `id` as 4A0A90 finds it (independent slots
    /// first, then major slots; wcscmp on the kingdom info IDS).
    pub fn kingdom_slot(&self, id: &str) -> Option<&KingdomSlot> {
        self.independent_kingdoms
            .iter()
            .chain(&self.major_kingdoms)
            .find(|s| self.settings.kingdoms[s.info].ids == id)
    }
}

// ---------------------------------------------------------------- loader

const DIR_EPSILON_BITS: u32 = 0x38d1_b717; // A14EC8 / 9F3228 (0.0001f)
const ONE_BITS: u32 = 0x3f80_0000;

fn bin(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
/// x87 compare of a binary32 value with `c`: Greater only (unordered never).
fn greater(bits: u32, c: u32) -> Result<bool> {
    if is_nan(bits) {
        return Ok(false);
    }
    Ok(bin(bits)?.compare(&bin(c)?) == std::cmp::Ordering::Greater)
}
fn less(bits: u32, c: u32) -> Result<bool> {
    if is_nan(bits) {
        return Ok(false);
    }
    Ok(bin(bits)?.compare(&bin(c)?) == std::cmp::Ordering::Less)
}
fn is_nan(bits: u32) -> bool {
    bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x7f_ffff != 0
}
fn finite(bits: u32, what: &str) -> Result<()> {
    if bits & 0x7f80_0000 == 0x7f80_0000 {
        return unsupported(format!("non-finite {what} (x87 special values not modelled)"));
    }
    Ok(())
}

/// C `%f` of a binary32 value promoted to double (410C40 messages).
fn percent_f(bits: u32) -> String {
    format!("{:.6}", f32::from_bits(bits) as f64)
}

struct Loader<'a> {
    reg: &'a RmcRegistries,
    precision: Precision,
    rmc: RmcData,
}

impl Loader<'_> {
    fn log(&mut self, line: String) {
        self.rmc.log.push(line);
    }

    /// 4A0470.
    fn slider(n: &RmcNodeData) -> Result<SliderOption> {
        Ok(SliderOption {
            range: n.point2("range")?,
            area_factor_bits: n.fixed("area_factor")?,
            show_ui: n.boolean("show_ui")?,
            locked: n.boolean("locked")?,
        })
    }

    /// 409230: normalize (x, y) at `precision`; 0 when the length is not
    /// greater than 0.0001.
    fn normalize(&self, v: [u32; 2]) -> Result<[u32; 2]> {
        let p = self.precision;
        let (x, y) = (bin(v[0])?, bin(v[1])?);
        let len = x.mul(&x, p)?.add(&y.mul(&y, p)?, p)?.sqrt(p)?;
        if len.compare(&bin(DIR_EPSILON_BITS)?) != std::cmp::Ordering::Greater {
            return Ok([0, 0]);
        }
        let inv = bin(ONE_BITS)?.div(&len, p)?;
        Ok([inv.mul(&x, p)?.store_f32()?, inv.mul(&y, p)?.store_f32()?])
    }

    /// 4A0510.
    fn micro_blobs(n: &RmcNodeData) -> Result<MicroBlobs> {
        let clamp01 = |b: u32| -> Result<u32> {
            finite(b, "ratio_range")?;
            Ok(if less(b, 0)? {
                0
            } else if greater(b, ONE_BITS)? {
                ONE_BITS
            } else {
                b
            })
        };
        let r = n.point2("ratio_range")?;
        let ratio_range = [clamp01(r[0])?, clamp01(r[1])?];
        let s = n.point2("blob_size")?;
        let nonneg = |b: u32| -> Result<u32> {
            finite(b, "blob_size")?;
            Ok(if less(b, 0)? { 0 } else { b })
        };
        let blob_size = [nonneg(s[0])?, nonneg(s[1])?];
        Ok(MicroBlobs {
            ratio_range,
            blob_size,
            consistency_bits: n.fixed("consistency")?,
            proximity_bits: n.fixed("proximity")?,
            fuzzy_factor_bits: n.fixed("fuzzy_factor")?,
            affect_height: n.boolean("affect_height")?,
        })
    }

    /// 4A2B70 + 4A2EC0 / 4A2FD0.
    fn terrain_slider(&self, n: &RmcNodeData, blob: bool) -> Result<TerrainSlider> {
        let slider = Self::slider(n)?;
        let terrain_ids = n.string("terrain_ids")?;
        let terrain = self
            .reg
            .find_checked("Terrain", &terrain_ids)?
            .ok_or_else(|| native(format!("Unknown terrain '{terrain_ids}' specified by RMC")))?;
        let brush_type = enum_required(BRUSH_TYPE, &n.string("brush_type")?)?;
        let paint_first_variation_only = n.boolean("paint_first_variation_only")?;
        let mut attachto: Vec<u32> = Vec::new();
        let mut alone = false;
        for tok in tokens(&n.string("attachto")?)? {
            let value = enum_lookup(ATTACH_TO, &tok).unwrap_or(0);
            if attachto.contains(&value) {
                return Err(native(format!(
                    "Terrain '{terrain_ids}' micro-blobbing already specified '{tok}'"
                )));
            }
            let bad = if value == 0 || value == 10 {
                alone = true;
                !attachto.is_empty()
            } else {
                alone
            };
            if bad {
                return Err(native(format!(
                    "Terrain '{terrain_ids}' micro-blobbing must specify ATTACHTO_RANDOM or ATTACHTO_NONE alone."
                )));
            }
            attachto.push(value);
        }
        if attachto.is_empty() {
            return Err(native(format!(
                "Terrain '{terrain_ids}' micro-blobbing must specify at least one attachto value"
            )));
        }
        let attach_range_bits = n.fixed("attach_range")?;
        let override_same = n.boolean("override_same")?;
        let shape = if blob {
            let blob_size = n.point2("blob_size")?;
            let consistency_bits = n.fixed("consistency")?;
            let proximity_bits = n.fixed("proximity")?;
            let fuzzy_factor_bits = n.fixed("fuzzy_factor")?;
            let mut dir_bias = n.point2("dir_bias")?;
            let dir_bias_factor_bits = n.fixed("dir_bias_factor")?;
            finite(dir_bias[0], "dir_bias")?;
            finite(dir_bias[1], "dir_bias")?;
            if greater(dir_bias[0], DIR_EPSILON_BITS)? || greater(dir_bias[1], DIR_EPSILON_BITS)? {
                dir_bias = self.normalize(dir_bias)?;
            }
            let micro_blobs = match n.subnode("MicroBlobs", false)? {
                Some(m) => Some(Self::micro_blobs(m)?),
                None => None,
            };
            TerrainShape::Blob {
                blob_size,
                consistency_bits,
                proximity_bits,
                fuzzy_factor_bits,
                dir_bias,
                dir_bias_factor_bits,
                micro_blobs,
            }
        } else {
            TerrainShape::Streak {
                streaks_min: n.int("streaks_min")?,
                streaks_max: n.int("streaks_max")?,
                length_range: n.point2("length_range")?,
                width_range: n.point2("width_range")?,
                angle_range: n.point2("angle_range")?,
            }
        };
        Ok(TerrainSlider {
            slider,
            terrain,
            brush_type,
            paint_first_variation_only,
            attachto,
            attach_range_bits,
            override_same,
            shape,
        })
    }

    /// 4A4410.
    fn terrain_sliders(&mut self, node: &RmcNodeData) -> Result<()> {
        let mut override_terrains: Vec<String> = Vec::new();
        for c in &node.children {
            let blob = if c.label.eq_ignore_ascii_case("TerrainBlob") {
                true
            } else if c.label.eq_ignore_ascii_case("TerrainStreak") {
                false
            } else {
                continue;
            };
            let s = self.terrain_slider(c, blob)?;
            let members = self
                .reg
                .biome_terrains
                .get(&self.rmc.biome)
                .ok_or_else(|| Error::Unsupported(format!("biome {} terrain list not provided", self.rmc.biome)))?;
            if !members.contains(&s.terrain) {
                return Err(native(format!(
                    "Terrain '{}' does not belong to biome '{}'",
                    s.terrain, self.rmc.biome
                )));
            }
            let terrain = s.terrain.clone();
            let (override_same, show_ui) = (s.override_same, s.slider.show_ui);
            self.rmc.terrain_sliders.push(s);
            if override_same {
                if !show_ui {
                    return Err(native(format!(
                        "A slider for terrain '{terrain}' cannot be hidden if it has the 'override_same' option."
                    )));
                }
                if override_terrains.iter().any(|t| t.eq_ignore_ascii_case(&terrain)) {
                    return Err(native(format!(
                        "A slider for terrain '{terrain}' already has the 'override_same' option specified."
                    )));
                }
                override_terrains.push(terrain);
            }
        }
        Ok(())
    }

    /// 4A1DB0 / 4B6A00.
    fn terrain_noise(&mut self, node: &RmcNodeData) -> Result<()> {
        for n in node.list("TerrainNoise") {
            self.rmc.terrain_noise.push(NoiseData {
                num_blobs_min: n.int("terrain_noise_num_blobs_min")?,
                num_blobs_max: n.int("terrain_noise_num_blobs_max")?,
                blob_size_min_bits: n.fixed("terrain_noise_blob_size_min")?,
                blob_size_max_bits: n.fixed("terrain_noise_blob_size_max")?,
                offset_magnitude_min_bits: n.fixed("terrain_noise_offset_magnitude_min")?,
                offset_magnitude_max_bits: n.fixed("terrain_noise_offset_magnitude_max")?,
                up_frequency_bits: n.fixed("terrain_noise_up_frequency")?,
                reference_size_bits: n.fixed("terrain_noise_reference_size")?,
                area_factor_bits: n.fixed("terrain_noise_area_factor")?,
            });
        }
        Ok(())
    }

    /// `_wtol` for the world-rule int parse (decimal, optional sign).
    fn wtol(text: &str) -> Result<i32> {
        let t: Vec<u16> = text.encode_utf16().collect();
        let mut i = 0;
        while i < t.len() && iswspace(t[i])? {
            i += 1;
        }
        let mut neg = false;
        if i < t.len() && (t[i] == u16::from(b'-') || t[i] == u16::from(b'+')) {
            neg = t[i] == u16::from(b'-');
            i += 1;
        }
        let mut v: i64 = 0;
        while i < t.len() && (u16::from(b'0')..=u16::from(b'9')).contains(&t[i]) {
            v = v * 10 + i64::from(t[i] - u16::from(b'0'));
            if v > i64::from(i32::MAX) + 1 {
                return unsupported("_wtol overflow (CRT behaviour not modelled)");
            }
            i += 1;
        }
        let v = if neg { -v } else { v };
        i32::try_from(v).map_err(|_| Error::Unsupported("_wtol overflow (CRT behaviour not modelled)".into()))
    }

    /// 4A19D0.
    fn rules(&mut self, node: &RmcNodeData) -> Result<()> {
        let rules = node.subnode("Rules", true)?.expect("required");
        self.rmc.settings.game_type = enum_required(GAME_TYPE, &rules.string("game_type")?)?;
        // 511D80 -> 511CE0: the rule table from the WorldRule registry defaults.
        let defs = self.reg.world_rules.clone();
        let mut table: Option<Vec<RuleValue>> = defs.as_ref().map(|defs| {
            defs.iter()
                .map(|d| RuleValue {
                    ids: d.ids.clone(),
                    locked: false,
                    value_word: if d.kind == 0 { d.default_word & 0xff } else { d.default_word },
                })
                .collect()
        });
        for r in rules.list("Rule") {
            let ids = r.string("IDS")?;
            let Some(table) = table.as_mut() else { continue };
            let defs = defs.as_ref().expect("table implies defs");
            // 511A60: wcsicmp on the rule definition IDS
            let Some(index) = table.iter().position(|v| v.ids.eq_ignore_ascii_case(&ids)) else {
                self.log(format!("Skipping unknown RMC world rule: {ids}"));
                continue;
            };
            let default = r.string("default")?;
            let def = &defs[index];
            let parsed: Option<u32> = match def.kind {
                0 => {
                    if default.eq_ignore_ascii_case("true") || default == "1" {
                        Some(1)
                    } else if default.eq_ignore_ascii_case("false") || default == "0" {
                        Some(0)
                    } else {
                        None
                    }
                }
                1 => {
                    let v = Self::wtol(&default)?;
                    (v != 0 || default.starts_with('0')).then_some(v as u32)
                }
                2 => {
                    return unsupported("world rule float default (_wtof) not ported");
                }
                3 => def
                    .enum_values
                    .iter()
                    .position(|e| e.eq_ignore_ascii_case(&default))
                    .map(|i| i as u32),
                _ => None,
            };
            match parsed {
                None => self.log(format!("Unable to parse default for World Rule: {ids}")),
                Some(v) => {
                    let show_ui = r.boolean("show_ui")?;
                    let entry = &mut table[index];
                    if def.kind == 0 {
                        entry.value_word = (entry.value_word & !0xff) | v;
                    } else {
                        entry.value_word = v;
                    }
                    entry.locked = !show_ui;
                }
            }
        }
        self.rmc.settings.rules = table;
        Ok(())
    }

    /// 4A0900 then 4A0960: independent slots first, then major slots.
    fn kingdom_slot_index(&self, id: &str) -> Option<(bool, usize)> {
        let k = &self.rmc.settings.kingdoms;
        if let Some(i) = self.rmc.independent_kingdoms.iter().position(|s| k[s.info].ids == id) {
            return Some((false, i));
        }
        self.rmc.major_kingdoms.iter().position(|s| k[s.info].ids == id).map(|i| (true, i))
    }
    fn slot_ids(&self, major: bool, i: usize) -> String {
        let s = if major { &self.rmc.major_kingdoms[i] } else { &self.rmc.independent_kingdoms[i] };
        self.rmc.settings.kingdoms[s.info].ids.clone()
    }

    /// 4A3A00 on a slot whose info is `info`; returns the slot type.
    fn slot_base(&mut self, n: &RmcNodeData, info: usize) -> Result<u32> {
        let ids = n.string("IDS")?;
        let name_key = n.string("IDS")?;
        let name_text = n.localized("name")?;
        let team = n.string("team")?;
        let nation = self.reg.find_checked("Nation", &n.string("nation")?)?;
        let faction = self.reg.find_checked("Faction", &n.string("faction")?)?;
        let color = self.reg.find_checked("KingdomColor", &n.string("color")?)?;
        let kind = enum_required(KINGDOM_TYPE, &n.string("type")?)?;
        let k = &mut self.rmc.settings.kingdoms[info];
        k.ids = ids;
        k.name = Localized { key: name_key, text: name_text };
        k.team = team;
        k.nation = nation;
        k.faction = faction;
        k.color = color;
        Ok(kind)
    }

    /// 4A3FE0.
    fn major_slot(&mut self, n: &RmcNodeData, info: usize) -> Result<KingdomSlot> {
        let kind = self.slot_base(n, info)?;
        let playable = n.boolean("playable")?;
        let required = n.boolean("required")?;
        let sai = n.string("sai_default_ids")?;
        {
            let k = &mut self.rmc.settings.kingdoms[info];
            k.playable = playable;
            k.required = required;
            k.sai_default_ids = sai;
            if k.required && k.color.is_none() {
                return Err(native(format!(
                    "Required Kingdom,'{}', does not have a color assigned to it",
                    k.ids
                )));
            }
        }
        let starting_pos = n.point2("starting_pos")?;
        if n.subnode("stockpile", false)?.is_some() {
            return unsupported("RMC kingdom [Stockpile] (473B10 PurchaseResource loader) not ported");
        }
        let kingdom_ids = self.rmc.settings.kingdoms[info].ids.clone();
        let mut heroes: Vec<String> = Vec::new();
        let mut random_heroes = 0i32;
        for h in n.list("Hero") {
            let ids = h.string("ids")?;
            if ids.is_empty() {
                random_heroes = random_heroes.wrapping_add(1);
                continue;
            }
            let t = self.reg.thing_checked(&ids)?.expect("non-empty");
            if !t.is_actor {
                return Err(native(format!(
                    "Heros must be actors only. Non-hero thing '{}' specified in '{kingdom_ids}'",
                    t.ids
                )));
            }
            heroes.push(t.ids.clone());
        }
        if (1..heroes.len()).any(|i| heroes[..i].contains(&heroes[i])) {
            return Err(native(format!("Duplicate heroes specified in major kingdom '{kingdom_ids}'")));
        }
        let mut techs: Vec<Option<String>> = Vec::new();
        for t in n.list("Technology") {
            let ids = t.string("ids")?;
            let Some(p) = self.reg.property_checked(&ids)? else {
                return unsupported("empty Technology ids: native reads NULL+0x5C");
            };
            match p.technology {
                Some(true) => {}
                Some(false) => {
                    return Err(native(format!(
                        "Properties must be technologies only.  Non-tech property '{}' specified in '{kingdom_ids}'",
                        p.ids
                    )));
                }
                None => return unsupported(format!("property {} technology flag (+0x5C) not derived", p.ids)),
            }
            techs.push(Some(p.ids.clone()));
        }
        if (1..techs.len()).any(|i| techs[..i].contains(&techs[i])) {
            return Err(native(format!("Duplicate techs specified in major kingdom '{kingdom_ids}'")));
        }
        self.rmc.settings.kingdoms[info].technologies = techs;
        Ok(KingdomSlot {
            info,
            kind,
            relations: Vec::new(),
            starting_pos: Some(starting_pos),
            heroes,
            random_heroes,
        })
    }

    /// 4A3C10 on the slot (`major`, `index`).
    fn relations(&mut self, n: &RmcNodeData, major: bool, index: usize) -> Result<()> {
        let own = self.slot_ids(major, index);
        for r in n.list("Relation") {
            let kingdom_ids = r.string("kingdom_ids")?;
            let relation = enum_lookup(RELATION, &r.string("relation")?).map_or(-1, |v| v as i32);
            let mut added: Vec<Relation> = Vec::new();
            let invalid = || native(format!("Invalid kingdom '{kingdom_ids}' specified by relation."));
            match kingdom_ids.find('*') {
                None => {
                    if kingdom_ids == own || self.kingdom_slot_index(&kingdom_ids).is_none() {
                        return Err(invalid());
                    }
                    added.push(Relation { kingdom: kingdom_ids.clone(), relation });
                }
                Some(byte) => {
                    let prefix: Vec<u16> = kingdom_ids[..byte].encode_utf16().collect();
                    let idx = prefix.len();
                    let indep = self.rmc.independent_kingdoms.len();
                    for j in 0..indep + self.rmc.major_kingdoms.len() {
                        let (m, i) = if j < indep { (false, j) } else { (true, j - indep) };
                        if m == major && i == index {
                            continue;
                        }
                        let other = self.slot_ids(m, i);
                        let mut o: Vec<u16> = other.encode_utf16().collect();
                        if idx <= o.len() {
                            o.truncate(idx);
                        }
                        if o == prefix {
                            added.push(Relation { kingdom: other, relation });
                        }
                    }
                    if added.is_empty() {
                        return Err(invalid());
                    }
                }
            }
            let slot = if major { &mut self.rmc.major_kingdoms[index] } else { &mut self.rmc.independent_kingdoms[index] };
            slot.relations.extend(added);
        }
        Ok(())
    }

    /// 4A4EB0.
    fn kingdoms(&mut self, node: &RmcNodeData) -> Result<()> {
        let list = node.list("Kingdom");
        self.rmc.settings.kingdoms.clear(); // 510DC0(0, 0)
        for k in &list {
            let independent = k.boolean("independent")?;
            self.rmc.settings.kingdoms.push(KingdomInfo {
                ids: String::new(),
                name: Localized::default(),
                independent,
                team: String::new(),
                nation: None,
                faction: None,
                color: None,
                playable: true,
                required: true,
                sai_default_ids: String::new(),
                technologies: Vec::new(),
                stockpile: vec![0; self.reg.resource_count as usize],
            });
        }
        let mut order: Vec<(bool, usize)> = Vec::new();
        for (i, k) in list.iter().enumerate() {
            k.boolean("independent")?;
            if self.rmc.settings.kingdoms[i].independent {
                let kind = self.slot_base(k, i)?;
                self.rmc.independent_kingdoms.push(KingdomSlot {
                    info: i,
                    kind,
                    relations: Vec::new(),
                    starting_pos: None,
                    heroes: Vec::new(),
                    random_heroes: 0,
                });
                order.push((false, self.rmc.independent_kingdoms.len() - 1));
            } else {
                let slot = self.major_slot(k, i)?;
                self.rmc.major_kingdoms.push(slot);
                order.push((true, self.rmc.major_kingdoms.len() - 1));
                if self.rmc.major_kingdoms.len() as i64 > i64::from(self.reg.kingdoms_max) {
                    return Err(native(format!("RMC {} has more kingdoms than allowed", self.rmc.ids)));
                }
            }
        }
        for (k, (major, index)) in list.iter().zip(order) {
            self.relations(k, major, index)?;
        }
        Ok(())
    }

    /// 4A0880: doodad groups then actor groups (wcscmp on the group IDS).
    fn group_exists(&self, ids: &str) -> bool {
        self.rmc.doodad_groups.iter().chain(&self.rmc.actor_groups).any(|g| g.ids == ids)
    }

    /// 4A2150.
    fn restrictions(&self, n: &RmcNodeData, group_ids: &str) -> Result<Vec<GroupRestriction>> {
        let mut out = Vec::new();
        for r in n.list("GroupRestriction") {
            let target = r.string("group_ids")?;
            match target.find('*') {
                None => {
                    if !self.group_exists(&target) {
                        return Err(native(format!("Unknown group '{target}'.")));
                    }
                    out.push(GroupRestriction {
                        group: target.clone(),
                        proximity_min_bits: r.fixed("proximity_min")?,
                        proximity_desired_bits: r.fixed("proximity_desired")?,
                    });
                }
                Some(byte) => {
                    let prefix: Vec<u16> = target[..byte].encode_utf16().collect();
                    let idx = prefix.len();
                    let mut count = 0;
                    for g in self.rmc.actor_groups.iter().chain(&self.rmc.doodad_groups) {
                        let mut o: Vec<u16> = g.ids.encode_utf16().collect();
                        if idx <= o.len() {
                            o.truncate(idx);
                        }
                        if o == prefix {
                            out.push(GroupRestriction {
                                group: g.ids.clone(),
                                proximity_min_bits: r.fixed("proximity_min")?,
                                proximity_desired_bits: r.fixed("proximity_desired")?,
                            });
                            count += 1;
                        }
                    }
                    // No loaded group matched (`iStack_34 == 0`): the group
                    // being loaded (its own IDS at `this+8`, not yet in the
                    // lists) is compared the same way; only a mismatch there
                    // is fatal, and a self match adds no restriction.
                    if count == 0 {
                        let mut own: Vec<u16> = group_ids.encode_utf16().collect();
                        if idx <= own.len() {
                            own.truncate(idx);
                        }
                        if own != prefix {
                            return Err(native(format!(
                                "Unknown group prefix '{}'.",
                                String::from_utf16_lossy(&prefix)
                            )));
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    /// 4A3060 / 4A30F0 object lookup.
    fn object(&self, n: &RmcNodeData) -> Result<String> {
        let ids = n.string("object_ids")?;
        match self.reg.thing_checked(&ids)? {
            Some(t) => Ok(t.ids.clone()),
            None => Err(native(format!("Unknown object '{ids}' specified by RMC"))),
        }
    }

    /// 4A31A0 (`ids` given to the constructor).
    fn object_group(&self, n: &RmcNodeData, ids: &str) -> Result<ObjectGroup> {
        let slider = Self::slider(n)?;
        let name = n.localized("name")?;
        let objects_min = n.int("objects_min")?;
        let balanced_placement = n.boolean("balanced_placement")?;
        let rel_group = n.string("search_relative_group_ids")?;
        let mut search_relative_group = None;
        if !rel_group.is_empty() {
            if rel_group == "this" {
                search_relative_group = Some(ids.to_string());
            } else if self.group_exists(&rel_group) {
                search_relative_group = Some(rel_group.clone());
            } else {
                return Err(native(format!(
                    "Group '{ids}' references an invalid search_relative_group '{rel_group}'"
                )));
            }
        }
        let rel_kingdom = n.string("search_relative_kingdom_ids")?;
        let mut search_relative_kingdom = None;
        if !rel_kingdom.is_empty() {
            if search_relative_group.is_some() {
                return Err(native(format!(
                    "Group '{ids}' can't specify relative kingdom if already specified relative group."
                )));
            }
            if rel_kingdom != "this" {
                match self.kingdom_slot_index(&rel_kingdom) {
                    Some((m, i)) => search_relative_kingdom = Some(self.slot_ids(m, i)),
                    None => {
                        return Err(native(format!(
                            "Group '{ids}' references an invalid search_relative_kingdom '{rel_kingdom}'"
                        )));
                    }
                }
            }
        }
        let search_radius_bits = n.fixed("search_radius")?;
        let mut required = Vec::new();
        for r in n.list("RequiredObject") {
            if balanced_placement {
                return Err(native(format!(
                    "'{ids}' is a balanced group - it must not have required objects."
                )));
            }
            required.push(RequiredObject {
                object: self.object(r)?,
                position: r.point2("position")?,
                distance_max_bits: r.fixed("distance_max")?,
            });
        }
        let mut objects = Vec::new();
        let mut weighted: Vec<GroupObject> = Vec::new();
        let mut total = 0u32;
        for o in n.list("Object") {
            let object = self.object(o)?;
            let weight_bits = o.fixed("weight")?;
            objects.push(GroupObject { object: object.clone(), weight_bits });
            // 4B6BA0: inserted at the head when weight > 0 (or unordered)
            finite(weight_bits, "object weight")?;
            if greater(weight_bits, 0)? {
                weighted.insert(0, GroupObject { object, weight_bits });
                total = bin(weight_bits)?.add(&bin(total)?, self.precision)?.store_f32()?;
            }
        }
        let intraproximity_min_bits = n.fixed("intraproximity_min")?;
        let intraproximity_desired_bits = n.fixed("intraproximity_desired")?;
        let starting_location_weight_bits = n.fixed("starting_location_weight")?;
        let restrictions = self.restrictions(n, ids)?;
        Ok(ObjectGroup {
            ids: ids.to_string(),
            name,
            slider,
            objects_min,
            required,
            objects,
            weighted,
            weighted_total_bits: total,
            search_relative_group,
            search_relative_kingdom,
            search_radius_bits,
            balanced_placement,
            intraproximity_min_bits,
            intraproximity_desired_bits,
            starting_location_weight_bits,
            restrictions,
            actor: None,
        })
    }

    /// 4A3590.
    fn actor_group(&self, n: &RmcNodeData, ids: &str) -> Result<ObjectGroup> {
        let mut g = self.object_group(n, ids)?;
        let kingdom_ids = n.string("kingdom_ids")?;
        let apply = n.boolean("apply_to_all_major_kingdoms")?;
        if self.reg.manager("DifficultyLevel")?.entries.is_empty() {
            return Err(native(format!("INDEX.LST must load difficulty levels before RMCs: {ids}")));
        }
        let difficulty = n.string("difficulty_level")?;
        let mut difficulty_level = None;
        if !difficulty.is_empty() {
            difficulty_level = Some(self.reg.find("DifficultyLevel", &difficulty)?.ok_or_else(|| {
                native(format!("Unknown difficulty level '{difficulty}' for RMC: {ids}"))
            })?);
        }
        if kingdom_ids.is_empty() && !apply {
            for o in g.required.iter().map(|r| &r.object).chain(g.objects.iter().map(|o| &o.object)) {
                let t = self.reg.thing(o).expect("resolved object");
                match t.ownable {
                    Some(true) => {
                        return Err(native(format!(
                            "Kingdomless actor group '{ids}' must not have OWNABLE object '{}'.",
                            t.ids
                        )));
                    }
                    Some(false) => {}
                    None => return unsupported(format!("Thing {} flags (+0x1C) not derived", t.ids)),
                }
            }
        }
        if g.search_relative_kingdom.is_none() && !kingdom_ids.is_empty() && !apply {
            let rel = n.string("search_relative_kingdom_ids")?;
            if rel == "this" {
                g.search_relative_kingdom = self.kingdom_slot_index(&kingdom_ids).map(|(m, i)| self.slot_ids(m, i));
            }
        }
        let mut properties = Vec::new();
        let text = n.string("properties")?;
        if !text.is_empty() {
            for tok in tokens(&text)? {
                properties.push(self.reg.property_checked(&tok)?.map(|p| p.ids.clone()));
            }
        }
        g.actor = Some(ActorGroupData {
            kingdom_ids,
            apply_to_all_major_kingdoms: apply,
            difficulty_level,
            properties,
        });
        Ok(g)
    }

    /// 4A4600.
    fn groups(&mut self, node: &RmcNodeData) -> Result<()> {
        for c in &node.children {
            if c.label.eq_ignore_ascii_case("ActorGroup") {
                let ids = c.string("ids")?;
                let g = self.actor_group(c, &ids)?;
                if !g.actor.as_ref().expect("actor").apply_to_all_major_kingdoms {
                    self.rmc.actor_groups.push(g);
                    continue;
                }
                for s in 0..self.rmc.major_kingdoms.len() {
                    let kingdom = self.slot_ids(true, s);
                    let name = format!("{ids}_{kingdom}");
                    let mut g2 = self.actor_group(c, &name)?;
                    let a = g2.actor.as_mut().expect("actor");
                    a.apply_to_all_major_kingdoms = false;
                    a.kingdom_ids = kingdom.clone();
                    if g2.search_relative_kingdom.is_none() && c.string("search_relative_kingdom_ids")? == "this" {
                        g2.search_relative_kingdom = Some(kingdom);
                    }
                    self.rmc.actor_groups.push(g2);
                }
            } else if c.label.eq_ignore_ascii_case("DoodadGroup") {
                let ids = c.string("ids")?;
                let g = self.object_group(c, &ids)?;
                self.rmc.doodad_groups.push(g);
            }
        }
        Ok(())
    }

    /// 4A1CD0.
    fn teams(&mut self, node: &RmcNodeData) -> Result<()> {
        let list = node.list("Team");
        self.rmc.settings.teams = vec![Team { ids: String::new(), name: Localized::default() }; list.len()];
        for (j, t) in list.iter().enumerate() {
            let ids = t.string("IDS")?;
            let key = t.string("IDS")?;
            let text = t.localized("name")?;
            self.rmc.settings.teams[j] = Team { ids, name: Localized { key, text } };
        }
        Ok(())
    }

    /// 4A5420.
    fn load(&mut self, node: &RmcNodeData) -> Result<()> {
        // 47D960
        self.rmc.name = node.localized("name")?;
        self.rmc.list_in_staging = node.boolean("list_in_staging")?;
        let name = self.rmc.name.clone();
        self.rmc.settings.name = Localized { key: name.clone(), text: name };
        let description = node.localized("description")?;
        self.rmc.settings.description = Localized { key: description.clone(), text: description };
        self.rmc.settings.ids = self.rmc.ids.clone();
        self.rmc.reference_size_bits = node.fixed("reference_size")?;
        let biome_ids = node.string("biome")?;
        self.rmc.biome = self
            .reg
            .find_checked("Biome", &biome_ids)?
            .ok_or_else(|| native(format!("Unknown biome '{biome_ids}' specified by RMC")))?;
        for tok in tokens(&node.string("sun")?)? {
            let v = self.reg.find_checked("Sun", &tok)?;
            self.rmc.sun.push(v);
        }
        for tok in tokens(&node.string("weather")?)? {
            let v = if tok.eq_ignore_ascii_case("NONE") { None } else { self.reg.find("Weather", &tok)? };
            self.rmc.weather.push(v);
        }
        let water = node.fixed("max_percent_water")?;
        finite(water, "max_percent_water")?;
        self.rmc.max_percent_water_bits = water;
        if greater(water, ONE_BITS)? {
            return Err(native(format!("RMC specifies a bad max_percent_water value '{}'", percent_f(water))));
        }
        let mountain = node.fixed("max_percent_mountain")?;
        finite(mountain, "max_percent_mountain")?;
        self.rmc.max_percent_mountain_bits = mountain;
        if greater(mountain, ONE_BITS)? {
            return Err(native(format!(
                "RMC specifies a bad max_percent_mountain value '{}'",
                percent_f(mountain)
            )));
        }
        self.rmc.balanced_kingdoms = node.boolean("balanced_kingdoms")?;
        self.rmc.team_desired_distance_bits = node.fixed("team_desired_distance")?;
        for m in node.list("MapSize") {
            self.rmc.map_sizes.push(RmcMapSize {
                width_bits: m.fixed("width")?,
                height_bits: m.fixed("height")?,
                recommended_kingdoms_min: m.int("recommended_kingdoms_min")?,
                recommended_kingdoms_max: m.int("recommended_kingdoms_max")?,
            });
        }
        if self.rmc.map_sizes.is_empty() {
            return Err(native("RMC must specify at least one map-size.".into()));
        }
        self.rules(node)?;
        self.terrain_sliders(node)?;
        self.terrain_noise(node)?;
        self.kingdoms(node)?;
        self.groups(node)?;
        self.teams(node)?;
        let heroes = node.subnode("Heroes", true)?.expect("required");
        self.rmc.heroes = RmcHeroes {
            random_heroes_min: heroes.int("random_heroes_min")?,
            random_heroes_max: heroes.int("random_heroes_max")?,
            show_ui: heroes.boolean("show_ui")?,
            name: heroes.localized("name")?,
        };
        Ok(())
    }
}

/// `4A0B60` + `4A5420`: load the RMC `ids` from its resolved node.
pub fn load_rmc(ids: &str, node: &RmcNodeData, reg: &RmcRegistries, precision: Precision) -> Result<RmcData> {
    let mut l = Loader {
        reg,
        precision,
        rmc: RmcData {
            ids: ids.to_string(),
            name: String::new(),
            list_in_staging: false,
            reference_size_bits: 0,
            biome: String::new(),
            sun: Vec::new(),
            weather: Vec::new(),
            max_percent_water_bits: 0xbf80_0000,
            max_percent_mountain_bits: 0xbf80_0000,
            balanced_kingdoms: false,
            team_desired_distance_bits: 0,
            map_sizes: Vec::new(),
            terrain_sliders: Vec::new(),
            terrain_noise: Vec::new(),
            major_kingdoms: Vec::new(),
            independent_kingdoms: Vec::new(),
            doodad_groups: Vec::new(),
            actor_groups: Vec::new(),
            heroes: RmcHeroes { random_heroes_min: 0, random_heroes_max: 0, show_ui: false, name: String::new() },
            settings: RmcSettingsData {
                name: Localized::default(),
                description: Localized::default(),
                ids: String::new(),
                game_type: 0,
                rules: None,
                teams: Vec::new(),
                kingdoms: Vec::new(),
            },
            log: Vec::new(),
        },
    };
    l.load(node)?;
    Ok(l.rmc)
}

/// Every RMC of a data set, in native registration order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcSet {
    pub rmcs: Vec<RmcData>,
}
impl RmcSet {
    /// The RMC a territory's `rmc_ids` names (resolved at load, case-insensitive).
    pub fn find(&self, ids: &str) -> Option<&RmcData> {
        self.rmcs.iter().find(|r| r.ids.eq_ignore_ascii_case(ids))
    }
}

// ---------------------------------------------------------------- export from the original sources

/// Field names read by the native loader per node label (lower-case).
fn field_names(label: &str) -> &'static [&'static str] {
    const SLIDER: &[&str] = &["range", "area_factor", "show_ui", "locked"];
    let _ = SLIDER;
    match label.to_ascii_lowercase().as_str() {
        "rmc" => &[
            "ids", "name", "description", "list_in_staging", "reference_size", "biome", "sun", "weather",
            "max_percent_water", "max_percent_mountain", "balanced_kingdoms", "team_desired_distance",
        ],
        "mapsize" => &["width", "height", "recommended_kingdoms_min", "recommended_kingdoms_max"],
        "rules" => &["game_type"],
        "rule" => &["ids", "default", "show_ui"],
        "terrainblob" | "terrainstreak" => &[
            "range", "area_factor", "show_ui", "locked", "terrain_ids", "brush_type",
            "paint_first_variation_only", "attachto", "attach_range", "override_same", "blob_size",
            "consistency", "proximity", "fuzzy_factor", "dir_bias", "dir_bias_factor", "streaks_min",
            "streaks_max", "length_range", "width_range", "angle_range",
        ],
        "microblobs" => &["ratio_range", "blob_size", "consistency", "proximity", "fuzzy_factor", "affect_height"],
        "terrainnoise" => &[
            "terrain_noise_num_blobs_min", "terrain_noise_num_blobs_max", "terrain_noise_blob_size_min",
            "terrain_noise_blob_size_max", "terrain_noise_offset_magnitude_min",
            "terrain_noise_offset_magnitude_max", "terrain_noise_up_frequency", "terrain_noise_reference_size",
            "terrain_noise_area_factor",
        ],
        "kingdom" => &[
            "independent", "ids", "name", "team", "nation", "faction", "color", "type", "playable", "required",
            "sai_default_ids", "starting_pos",
        ],
        "hero" | "technology" => &["ids"],
        "relation" => &["kingdom_ids", "relation"],
        "actorgroup" | "doodadgroup" => &[
            "ids", "range", "area_factor", "show_ui", "locked", "name", "objects_min", "balanced_placement",
            "search_relative_group_ids", "search_relative_kingdom_ids", "search_radius", "intraproximity_min",
            "intraproximity_desired", "starting_location_weight", "kingdom_ids", "apply_to_all_major_kingdoms",
            "difficulty_level", "properties",
        ],
        "requiredobject" => &["object_ids", "position", "distance_max"],
        "object" => &["object_ids", "weight"],
        "grouprestriction" => &["group_ids", "proximity_min", "proximity_desired"],
        "team" => &["ids", "name"],
        "heroes" => &["random_heroes_min", "random_heroes_max", "show_ui", "name"],
        _ => &[],
    }
}
/// Child labels the native loader visits per node label.
fn child_labels(label: &str) -> &'static [&'static str] {
    match label.to_ascii_lowercase().as_str() {
        "rmc" => &[
            "MapSize", "Rules", "TerrainBlob", "TerrainStreak", "TerrainNoise", "Kingdom", "ActorGroup",
            "DoodadGroup", "Team", "Heroes",
        ],
        "rules" => &["Rule"],
        "terrainblob" => &["MicroBlobs"],
        "kingdom" => &["Stockpile", "Hero", "Technology", "Relation"],
        "actorgroup" | "doodadgroup" => &["RequiredObject", "Object", "GroupRestriction"],
        _ => &[],
    }
}
/// Labels whose relative order the loader observes (`node+0x34` walks).
const ORDERED: &[&str] = &["TerrainBlob", "TerrainStreak", "ActorGroup", "DoodadGroup"];

fn localized_field(name: &str) -> bool {
    matches!(name, "name" | "description")
}

/// Export one resolved node (aa-rules) as [`RmcNodeData`].
pub fn export_node(node: &aa_rules::actors::ResolvedNode<'_>, label: &str, precision: Precision) -> Result<RmcNodeData> {
    use aa_rules::native::{ScalarType, TypedValue};
    let mut fields = BTreeMap::new();
    for &name in field_names(label) {
        let value = if localized_field(name) {
            match node.localized(name) {
                Ok(Some(t)) => Some(FieldValue::Localized(t.text)),
                Ok(None) => None,
                Err(e) => Some(FieldValue::Unsupported(e.to_string())),
            }
        } else {
            match node.field(name, precision) {
                Ok(Some(v)) => Some(match v {
                    TypedValue::Scalar(s) if s.field_type == ScalarType::Fixed => FieldValue::Fixed(s.bits),
                    TypedValue::Scalar(s) => FieldValue::Float(s.bits),
                    TypedValue::Integer { value, .. } => FieldValue::Int(value),
                    TypedValue::Point { bits, .. } => FieldValue::Point(bits),
                    TypedValue::Boolean { value, .. } => FieldValue::Bool(value),
                    TypedValue::String { value, .. } => FieldValue::String(value),
                }),
                Ok(None) => None,
                Err(e) => match node.plain_string_field(name) {
                    Ok(Some(TypedValue::String { value, .. })) => Some(FieldValue::String(value)),
                    _ => Some(FieldValue::Unsupported(e.to_string())),
                },
            }
        };
        if let Some(v) = value {
            fields.insert(name.to_string(), v);
        }
    }
    // children: ordered labels interleaved by (template depth, source position)
    let layers = node.sources();
    let rank = |child: &aa_rules::actors::ResolvedNode<'_>| -> (usize, usize) {
        let src = child.sources();
        let Some(own) = src.first() else { return (usize::MAX, 0) };
        let depth = layers
            .iter()
            .position(|l| l.path == own.path && l.span.start <= own.span.start && own.span.end <= l.span.end)
            .map_or(usize::MAX, |i| layers.len() - 1 - i);
        (depth, own.span.start)
    };
    let mut ordered: Vec<((usize, usize), RmcNodeData)> = Vec::new();
    let mut rest: Vec<RmcNodeData> = Vec::new();
    for &child in child_labels(label) {
        for c in node.children(child)? {
            let data = export_node(&c, child, precision)?;
            if ORDERED.iter().any(|o| o.eq_ignore_ascii_case(child)) {
                let r = rank(&c);
                if r.0 == usize::MAX {
                    return unsupported(format!("cannot place {child} child in its parent's layers"));
                }
                ordered.push((r, data));
            } else {
                rest.push(data);
            }
        }
    }
    ordered.sort_by_key(|(r, _)| *r);
    let mut children: Vec<RmcNodeData> = rest;
    children.extend(ordered.into_iter().map(|(_, d)| d));
    Ok(RmcNodeData { label: label.to_string(), fields, children })
}

/// RMC nodes (native registration order) and the registries their loader consults.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcExport {
    pub rmcs: Vec<(String, RmcNodeData)>,
    pub registries: RmcRegistries,
}

fn collect_refs(n: &RmcNodeData, things: &mut Vec<String>, props: &mut Vec<String>) {
    let s = |name: &str| match n.get(name) {
        Some(FieldValue::String(v)) => Some(v.clone()),
        _ => None,
    };
    if n.label.eq_ignore_ascii_case("Object") || n.label.eq_ignore_ascii_case("RequiredObject") {
        things.extend(s("object_ids"));
    }
    if n.label.eq_ignore_ascii_case("Hero") {
        things.extend(s("ids"));
    }
    if n.label.eq_ignore_ascii_case("Technology") {
        props.extend(s("ids"));
    }
    if n.label.eq_ignore_ascii_case("ActorGroup")
        && let Some(p) = s("properties")
        && let Ok(t) = tokens(&p)
    {
        props.extend(t);
    }
    for c in &n.children {
        collect_refs(c, things, props);
    }
}

/// Export every RMC (native RMC DataManager order) and the registries from
/// the original data (aa-rules native load). Manager display names are the
/// root tag names (an assumption: only error texts use them).
pub fn export_v14(catalog: &aa_rules::actors::ActorCatalog, precision: Precision) -> Result<RmcExport> {
    use aa_rules::native::TypedValue;
    let registry = catalog
        .native_data_registry("RMC")
        .ok_or_else(|| Error::Unsupported("RMC registry absent".into()))?;
    let mut rmcs = Vec::new();
    for e in &registry.entries {
        let node = catalog.native_data_node(e)?;
        rmcs.push((e.ids.clone(), export_node(&node, "RMC", precision)?));
    }
    let mut reg = RmcRegistries::default();
    for tag in ["Nation", "Faction", "KingdomColor", "DifficultyLevel", "Biome", "Terrain", "Weather", "Sun"] {
        let entries = catalog
            .native_data_registry(tag)
            .map(|r| r.entries.iter().map(|e| e.ids.clone()).collect())
            .unwrap_or_default();
        reg.managers.insert(tag.to_string(), ManagerData { name: tag.to_string(), entries });
    }
    if let Some(biomes) = catalog.native_data_registry("Biome") {
        let terrains = catalog.native_data_registry("Terrain");
        for e in &biomes.entries {
            let node = catalog.native_data_node(e)?;
            let mut list = Vec::new();
            for t in node.children("Terrain")? {
                let Some(TypedValue::String { value, .. }) = t.field("terrain_IDS", precision)? else {
                    return unsupported(format!("biome {} terrain without terrain_IDS", e.ids));
                };
                let found = terrains.and_then(|r| r.find(&value)).ok_or_else(|| {
                    Error::Unsupported(format!("biome {} terrain {value} not registered", e.ids))
                })?;
                list.push(found.ids.clone());
            }
            reg.biome_terrains.insert(e.ids.clone(), list);
        }
    }
    let (mut things, mut props) = (Vec::new(), Vec::new());
    for (_, n) in &rmcs {
        collect_refs(n, &mut things, &mut props);
    }
    things.sort();
    things.dedup();
    for t in things {
        if let Some(d) = catalog.find(&t) {
            let category = catalog.native_category(&d.id)?;
            let ownable = catalog.native_final_flags(&d.id, precision).ok().map(|f| f & 0x0080_0000 != 0);
            if !reg.things.iter().any(|x: &ThingEntry| x.ids.eq_ignore_ascii_case(&d.id)) {
                reg.things.push(ThingEntry { ids: d.id.clone(), is_actor: category.native_kind == 2, ownable });
            }
        }
    }
    reg.thing_manager_name = "Thing".into();
    for p in catalog.resolved_definitions("Property")? {
        if let Some(TypedValue::String { value, .. }) = p.field("IDS", precision)?
            && props.iter().any(|x| x.eq_ignore_ascii_case(&value))
        {
            reg.properties.push(PropertyEntry { ids: value, technology: None });
        }
    }
    reg.property_manager_name = "Property".into();
    // `Game/SVars*.tgi` (index_aa.lst) loads SVars.tgi, then SVars_AA.tgi,
    // into the one SVars object: the later assignment is the configured value
    // (the 16-kingdom audit, reconstruction/research/native-aa/
    // sixteen-player-native-audit.json, raises it through SVars_AA.tgi). Any
    // other overriding source is refused.
    let mut kingdoms_max = None;
    for s in catalog.resolved_definitions("SVars")? {
        if let Some(TypedValue::Integer { value, .. }) = s.field("KingdomsMax", precision)? {
            let from_aa = s.sources().iter().any(|p| p.path.eq_ignore_ascii_case("Game/SVars_AA.tgi"));
            match kingdoms_max {
                Some((_, true)) => return unsupported("SVars KingdomsMax assigned after SVars_AA.tgi"),
                Some((_, false)) if !from_aa => return unsupported("SVars KingdomsMax assigned twice outside SVars_AA.tgi"),
                _ => kingdoms_max = Some((value, from_aa)),
            }
        }
    }
    let kingdoms_max = kingdoms_max.map(|(v, _)| v);
    reg.kingdoms_max = kingdoms_max.ok_or_else(|| Error::Unsupported("SVars KingdomsMax absent".into()))?;
    reg.resource_count = catalog.native_data_registry("Resource").map_or(0, |r| r.entries.len() as u32);
    Ok(RmcExport { rmcs, registries: reg })
}
