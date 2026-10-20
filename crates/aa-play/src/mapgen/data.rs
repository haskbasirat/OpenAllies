//! The original data the random map generator reads, resolved through the
//! aa-rules TGI catalog (Data.rwd + the v14 overlay):
//! - RMC definitions (Data/RandomMap/rmc_*.tgi on Templates/template_rmc*.tgi),
//!   loaded like KKC_RMCData (0x4A4410 terrain brushes, 0x4A2B70/0x4A2EC0/
//!   0x4A2FD0 brush fields, 0x4A0510 MicroBlobs, 0x4B6A00 NoiseData,
//!   0x4A31A0/0x4A3590/0x4A4600 object and actor groups, 0x4A4EB0 kingdoms);
//! - Terrain / Variation / Biome definitions (0x4B87A0, 0x4B8E80) including
//!   the biome Transition table (symmetric, 0x4B8E80 lines 279-280);
//! - Thing views (model paths), sizes and [Placement] biome/terrain doodad
//!   weights (KKC_ObjectData::LoadBiomePlacementData 0x49D1C0);
//! - SVars RMG values and the AVars_AA staging defaults.
use aa_numeric::Precision;
use aa_rules::{
    actors::{ActorCatalog, ResolvedNode, ThingRef},
    native::TypedValue,
};
use std::collections::BTreeMap;

pub(super) const PC: Precision = Precision::Double53;

pub(super) fn num(v: &TypedValue) -> Option<f32> {
    match v {
        TypedValue::Scalar(s) => Some(f32::from_bits(s.bits)),
        TypedValue::Integer { value, .. } => Some(*value as f32),
        TypedValue::Boolean { value, .. } => Some(if *value { 1.0 } else { 0.0 }),
        TypedValue::String { value, .. } => value.trim().parse().ok(),
        TypedValue::Point { bits, .. } => bits.first().map(|b| f32::from_bits(*b)),
    }
}
pub(super) fn fx(node: &ResolvedNode<'_>, name: &str) -> Option<f32> {
    match node.field(name, PC) {
        Ok(Some(v)) => num(&v),
        _ => match node.plain_string_field(name).ok().flatten() {
            Some(v) => num(&v),
            None => None,
        },
    }
}
pub(super) fn int(node: &ResolvedNode<'_>, name: &str) -> Option<i32> {
    fx(node, name).map(|v| v as i32)
}
pub(super) fn st(node: &ResolvedNode<'_>, name: &str) -> Option<String> {
    let value = match node.field(name, PC).ok().flatten() {
        Some(TypedValue::String { value, .. }) => Some(value),
        _ => match node.plain_string_field(name).ok().flatten() {
            Some(TypedValue::String { value, .. }) => Some(value),
            _ => None,
        },
    }?;
    let value = value.trim().trim_matches('"').to_owned();
    (!value.is_empty() && !value.eq_ignore_ascii_case("null")).then_some(value)
}
/// String field and the folder of the file that authored it (for the
/// native source-relative paths, 0x4763C0).
fn st_folder(node: &ResolvedNode<'_>, name: &str) -> Option<(String, String)> {
    let (value, source) = match node.field(name, PC).ok().flatten() {
        Some(TypedValue::String { value, source, .. }) => (value, source),
        _ => match node.plain_string_field(name).ok().flatten() {
            Some(TypedValue::String { value, source, .. }) => (value, source),
            _ => return None,
        },
    };
    let value = value.trim().trim_matches('"').to_owned();
    if value.is_empty() || value.eq_ignore_ascii_case("null") {
        return None;
    }
    let folder = source.path.replace('\\', "/").rsplit_once('/').map_or(String::new(), |(f, _)| f.to_owned());
    Some((value, folder))
}
pub(super) fn bl(node: &ResolvedNode<'_>, name: &str) -> Option<bool> {
    match node.field(name, PC).ok().flatten() {
        Some(TypedValue::Boolean { value, .. }) => Some(value),
        Some(TypedValue::Integer { value, .. }) => Some(value != 0),
        _ => match st(node, name)?.to_ascii_lowercase().as_str() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        },
    }
}
pub(super) fn pt(node: &ResolvedNode<'_>, name: &str) -> Option<[f32; 2]> {
    match node.field(name, PC).ok().flatten() {
        Some(TypedValue::Point { bits, .. }) if bits.len() >= 2 => Some([f32::from_bits(bits[0]), f32::from_bits(bits[1])]),
        _ => {
            let text = st(node, name)?;
            let mut it = text.split(',').map(|s| s.trim().trim_start_matches('+').parse::<f32>());
            Some([it.next()?.ok()?, it.next()?.ok()?])
        }
    }
}
fn pt3(node: &ResolvedNode<'_>, name: &str) -> Option<[f32; 3]> {
    match node.field(name, PC).ok().flatten() {
        Some(TypedValue::Point { bits, .. }) if bits.len() >= 3 => Some([f32::from_bits(bits[0]), f32::from_bits(bits[1]), f32::from_bits(bits[2])]),
        _ => None,
    }
}
fn loc(node: &ResolvedNode<'_>, name: &str) -> String {
    node.localized(name).ok().flatten().map(|l| l.text).unwrap_or_default()
}
fn children<'a>(node: &ResolvedNode<'a>, name: &str) -> Vec<ResolvedNode<'a>> {
    node.children(name).unwrap_or_default()
}
fn child<'a>(node: &ResolvedNode<'a>, name: &str) -> Option<ResolvedNode<'a>> {
    match node.child(name) {
        Ok(c) => c,
        Err(_) => node.children(name).ok().and_then(|v| v.into_iter().next()),
    }
}

/// RMCSliderOption (template_rmc.tgi, 0x4A0470).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Slider {
    pub range: [f32; 2],
    pub area_factor: f32,
    pub show_ui: bool,
    pub locked: bool,
}
fn slider(node: &ResolvedNode<'_>) -> Slider {
    Slider {
        range: pt(node, "range").unwrap_or([0.0, 0.0]),
        area_factor: fx(node, "area_factor").unwrap_or(1.0),
        show_ui: bl(node, "show_ui").unwrap_or(true),
        locked: bl(node, "locked").unwrap_or(false),
    }
}

#[derive(Clone, Debug)]
pub(super) struct MicroDef {
    /// 0x4A0510 clamps both ends to 0..1.
    pub ratio: [f32; 2],
    pub size: [f32; 2],
    pub affect_height: bool,
}
#[derive(Clone, Debug)]
pub(super) struct BlobDef {
    pub size: [f32; 2],
    pub consistency: f32,
    pub proximity: f32,
    pub fuzzy: f32,
    pub dir_bias: [f32; 2],
    pub dir_bias_factor: f32,
    pub micro: Option<MicroDef>,
}
#[derive(Clone, Debug)]
pub(super) struct StreakDef {
    pub streaks: [i32; 2],
    pub width: [f32; 2],
    pub length: [f32; 2],
    pub angle: [f32; 2],
}
#[derive(Clone, Debug)]
pub(super) enum BrushKind {
    Blob(BlobDef),
    Streak(StreakDef),
}
#[derive(Clone, Debug)]
pub(super) struct BrushDef {
    pub slider: Slider,
    pub terrain: usize,
    pub paint_first_variation_only: bool,
    /// Attach directions (0 NONE .. 9 CENTER, 10 RANDOM; 0x4A18D0 order).
    pub attach: Vec<u8>,
    pub attach_range: f32,
    pub override_same: bool,
    pub kind: BrushKind,
}
/// NoiseData (template_terrain.tgi, 0x4B6A00).
#[derive(Clone, Debug)]
pub(super) struct NoiseDef {
    pub num: [i32; 2],
    pub size: [f32; 2],
    pub magnitude: [f32; 2],
    pub up_frequency: f32,
    pub reference_size: f32,
    pub area_factor: f32,
}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(super) struct RequiredDef {
    pub object: String,
    pub position: [f32; 2],
    pub distance_max: f32,
}
#[derive(Clone, Debug)]
pub(super) struct RestrictionDef {
    /// Group IDS, a trailing '*' matches every group with that prefix.
    pub group_ids: String,
    pub min: f32,
    pub desired: f32,
}
/// RMCObjectGroup / RMCActorGroup.
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(super) struct GroupDef {
    pub ids: String,
    pub name: String,
    pub slider: Slider,
    pub objects_min: i32,
    pub required: Vec<RequiredDef>,
    pub objects: Vec<(String, f32)>,
    pub balanced: bool,
    pub search_radius: f32,
    /// "this" is kept as written; resolved per generated group.
    pub search_relative_group: Option<String>,
    pub search_relative_kingdom: Option<String>,
    pub intra_min: f32,
    pub intra_desired: f32,
    pub start_weight: f32,
    pub restrictions: Vec<RestrictionDef>,
    pub kingdom_ids: Option<String>,
    pub all_major_kingdoms: bool,
    pub properties: Option<String>,
}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(super) struct KingdomDef {
    pub ids: String,
    pub name: String,
    pub independent: bool,
    pub color: Option<String>,
    pub team: Option<String>,
    pub playable: bool,
    pub nation: Option<String>,
}
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(super) struct RmcDef {
    pub ids: String,
    pub name: String,
    pub description: String,
    pub biome: String,
    pub list_in_staging: bool,
    pub reference_size: f32,
    pub max_percent_water: f32,
    pub max_percent_mountain: f32,
    pub balanced_kingdoms: bool,
    pub team_desired_distance: f32,
    pub sizes: Vec<super::RmcMapSize>,
    pub brushes: Vec<BrushDef>,
    pub noise: Vec<NoiseDef>,
    pub kingdoms: Vec<KingdomDef>,
    pub actor_groups: Vec<GroupDef>,
    pub doodad_groups: Vec<GroupDef>,
    pub heroes: [i32; 2],
    pub heroes_show_ui: bool,
    pub sun: Vec<String>,
    pub weather: Vec<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct TerrainDef {
    pub ids: String,
    pub name: String,
    /// 0 LAND, 1 MOUNTAIN, 2 WATER (Terrain+1C).
    pub kind: u8,
    pub land: bool,
    pub water: bool,
    pub base_priority: i32,
    pub doodad_frequency: f32,
    pub primary_doodad_frequency: f32,
    /// Terrain+5C; zero for land terrains (never loaded for them).
    pub max_slope: f32,
    pub paint_slope: f32,
    pub paint_max_step: f32,
    pub noise: [f32; 2],
    /// Terrain+70 (water).
    pub max_depth: f32,
    pub color: [f32; 3],
    pub variations: Vec<usize>,
    /// Terrain+3C, the sum of the variation weights.
    pub total_weight: f32,
    pub water_doodad: Option<String>,
}
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct VariationDef {
    pub ids: String,
    pub terrain: usize,
    pub weight: f32,
    pub texture: Option<String>,
    pub texture_size: [f32; 2],
    pub color: [f32; 3],
    /// base_priority + ordinal within the terrain (0x4B87A0).
    pub priority: i32,
    /// m/s, None when the terrain does not cap speed.
    pub speed_limit: Option<f32>,
}
#[derive(Clone, Debug, Default)]
pub(super) struct DoodadLists {
    pub normal: Vec<(usize, f32)>,
    pub primary: Vec<(usize, f32)>,
}
#[derive(Clone, Debug, Default)]
pub(super) struct BiomeDef {
    pub ids: String,
    pub terrains: Vec<usize>,
    /// (terrain a, terrain b) -> variation, stored both ways.
    pub transitions: BTreeMap<(usize, usize), usize>,
    /// Per terrain: doodads with weight > 0 on it.
    pub doodads: BTreeMap<usize, DoodadLists>,
}
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub(super) struct ThingDef {
    pub ids: String,
    pub category: String,
    /// Model per aspect (aspects_as_variations) or the normal aspect.
    pub models: Vec<String>,
    pub scale: [f32; 2],
    pub random_orientation: bool,
    /// Blocking footprint half-extents in metres (0 = not blocking).
    pub half_extent: [f32; 2],
    pub blocking: bool,
    /// City layouts of a city/village center (CityComponent), rows of
    /// (object ids, offset, facing).
    pub layouts: Vec<Vec<(String, [f32; 2], [f32; 2])>>,
    /// [BaseTerrain]: biome -> variation painted under the footprint, the
    /// default variation, and the padding in metres.
    pub base_terrain: BTreeMap<String, String>,
    pub base_default: Option<String>,
    pub base_padding: f32,
    /// [Flags] flatten_terrain.
    pub flatten: bool,
    /// [Flags] invisible (audio doodads and other never-drawn things).
    pub invisible: bool,
    /// starting_location_priority (template_actor.tgi): the owned actor with
    /// the highest priority becomes its kingdom's starting location.
    pub start_priority: i32,
}
#[derive(Clone, Debug)]
pub(super) struct Svars {
    pub retries: i32,
    pub placement_retries: i32,
    pub max_percent_mountain: f32,
    pub max_percent_water: f32,
    pub balance_iterations: i32,
    pub balance_worse: i32,
    pub nudge_step: f32,
    pub nudge_delta: f32,
    pub elevation: [f32; 3],
    pub doodad_auto: bool,
    pub tile_bits: u32,
    pub height_bits: u32,
}

pub(super) struct RmgData {
    pub rmcs: Vec<RmcDef>,
    pub terrains: Vec<TerrainDef>,
    pub terrain_by_id: BTreeMap<String, usize>,
    pub variations: Vec<VariationDef>,
    pub variation_by_id: BTreeMap<String, usize>,
    pub biomes: BTreeMap<String, BiomeDef>,
    pub things: Vec<ThingDef>,
    pub thing_by_id: BTreeMap<String, usize>,
    /// RandomActor ids -> (thing, weight).
    pub random_actors: BTreeMap<String, Vec<(String, f32)>>,
    /// KingdomColor ids in registry order.
    pub colors: Vec<String>,
    pub svars: Svars,
    pub default_rmc: String,
    pub default_size: [f32; 2],
    pub notes: Vec<String>,
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

impl RmgData {
    pub(super) fn open_catalog(data_root: &std::path::Path) -> anyhow::Result<ActorCatalog> {
        let mut files = aa_rules::sources::V14Sources::open(
            data_root.join("Data.rwd"),
            data_root.join("reconstruction").join("native-game").join("runtime-expanded-v14"),
            aa_rules::sources::SourceLimits { max_member_bytes: 64 << 20 },
        )
        .map_err(|e| anyhow::anyhow!("opening the original data in {}: {e}", data_root.display()))?;
        ActorCatalog::load_v14(&mut files).map_err(|e| anyhow::anyhow!("reading the original rules: {e}"))
    }

    pub(super) fn load(catalog: &ActorCatalog) -> anyhow::Result<Self> {
        let mut data = RmgData {
            rmcs: Vec::new(),
            terrains: Vec::new(),
            terrain_by_id: BTreeMap::new(),
            variations: Vec::new(),
            variation_by_id: BTreeMap::new(),
            biomes: BTreeMap::new(),
            things: Vec::new(),
            thing_by_id: BTreeMap::new(),
            random_actors: BTreeMap::new(),
            colors: Vec::new(),
            svars: Svars {
                retries: 3,
                placement_retries: 3,
                max_percent_mountain: 0.15,
                max_percent_water: 0.15,
                balance_iterations: 120,
                balance_worse: 8,
                nudge_step: 2.0,
                nudge_delta: 2.0,
                elevation: [-10.0, 14.0, 2.0],
                doodad_auto: true,
                tile_bits: 1,
                height_bits: 1,
            },
            default_rmc: "rmc_southerneurope".into(),
            default_size: [384.0, 384.0],
            notes: Vec::new(),
        };
        data.load_vars(catalog);
        data.load_terrains(catalog);
        data.load_things(catalog);
        data.load_biomes(catalog);
        for color in catalog.resolved_definitions("KingdomColor").unwrap_or_default() {
            if let Some(id) = st(&color, "IDS") {
                data.colors.push(id);
            }
        }
        for rmc in catalog.resolved_definitions("RMC").map_err(|e| anyhow::anyhow!("RMC definitions: {e}"))? {
            match data.load_rmc(&rmc) {
                Ok(Some(def)) => data.rmcs.push(def),
                Ok(None) => {}
                Err(e) => data.notes.push(format!("RMC skipped: {e}")),
            }
        }
        anyhow::ensure!(!data.rmcs.is_empty(), "no RMC definitions in the original data");
        Ok(data)
    }

    fn load_vars(&mut self, catalog: &ActorCatalog) {
        for kind in ["SVars", "Vars"] {
            for node in catalog.resolved_definitions(kind).unwrap_or_default() {
                let s = &mut self.svars;
                if let Some(v) = int(&node, "RMGNumRetries") { s.retries = v }
                if let Some(v) = int(&node, "RMGNumPlacementRetries") { s.placement_retries = v }
                if let Some(v) = fx(&node, "RMCMaxPercentMountain") { s.max_percent_mountain = v }
                if let Some(v) = fx(&node, "RMCMaxPercentWater") { s.max_percent_water = v }
                if let Some(v) = int(&node, "RMGBalanceMaxNumIterations") { s.balance_iterations = v }
                if let Some(v) = int(&node, "RMGBalanceMaxNumWorseScores") { s.balance_worse = v }
                if let Some(v) = fx(&node, "RMGBalanceMarkerNudgeStep") { s.nudge_step = v }
                if let Some(v) = fx(&node, "RMGBalanceMarkerNudgeStepDelta") { s.nudge_delta = v }
                if let Some(v) = fx(&node, "ElevationMin") { s.elevation[0] = v }
                if let Some(v) = fx(&node, "ElevationMax") { s.elevation[1] = v }
                if let Some(v) = fx(&node, "ElevationDefault") { s.elevation[2] = v }
                if let Some(v) = bl(&node, "MapDoodadAutoPlacement") { s.doodad_auto = v }
                if let Some(v) = int(&node, "MapGTileResolutionBits") { s.tile_bits = v.max(0) as u32 }
                if let Some(v) = int(&node, "HeightmapResolutionBits") { s.height_bits = v.max(0) as u32 }
            }
        }
        for kind in ["AVars", "Vars"] {
            for node in catalog.resolved_definitions(kind).unwrap_or_default() {
                if let Some(v) = st(&node, "StagingSinglePlayerDefaultRMCIds") { self.default_rmc = v }
                if let Some(v) = fx(&node, "StagingSinglePlayerDefaultRMCWidth") { self.default_size[0] = v }
                if let Some(v) = fx(&node, "StagingSinglePlayerDefaultRMCHeight") { self.default_size[1] = v }
            }
        }
    }

    fn load_terrains(&mut self, catalog: &ActorCatalog) {
        let speeds = catalog.native_terrain_variation_speeds(PC).unwrap_or_else(|e| {
            self.notes.push(format!("terrain speeds: {e}"));
            Vec::new()
        });
        for t in catalog.resolved_definitions("Terrain").unwrap_or_default() {
            let Some(ids) = st(&t, "IDS") else { continue };
            if self.terrain_by_id.contains_key(&lower(&ids)) {
                continue;
            }
            let kind = match st(&t, "type").unwrap_or_default().to_ascii_uppercase().as_str() {
                "MOUNTAIN" => 1,
                "WATER" => 2,
                _ => 0,
            };
            let index = self.terrains.len();
            let base_priority = int(&t, "base_priority").unwrap_or(0);
            let mut def = TerrainDef {
                ids: ids.clone(),
                name: loc(&t, "name"),
                kind,
                land: bl(&t, "land").unwrap_or(kind == 0),
                water: bl(&t, "water").unwrap_or(kind == 2),
                base_priority,
                doodad_frequency: fx(&t, "doodad_frequency").unwrap_or(0.0),
                primary_doodad_frequency: fx(&t, "primary_doodad_frequency").unwrap_or(0.0),
                color: pt3(&t, "terrain_color").unwrap_or([0.5, 0.5, 0.5]),
                water_doodad: st(&t, "water_doodad_IDS"),
                ..Default::default()
            };
            if kind != 0 {
                def.max_slope = fx(&t, "max_slope").unwrap_or(0.0);
                def.paint_slope = fx(&t, "paint_slope").unwrap_or(0.0);
                def.paint_max_step = fx(&t, "paint_max_step").unwrap_or(0.0);
            }
            if kind == 1 {
                def.noise = [fx(&t, "noise_min").unwrap_or(0.0), fx(&t, "noise_max").unwrap_or(0.0)];
            }
            if kind == 2 {
                def.max_depth = fx(&t, "max_depth").unwrap_or(0.0);
            }
            for (ordinal, v) in children(&t, "Variation").iter().enumerate() {
                let Some(vid) = st(v, "IDS") else { continue };
                if self.variation_by_id.contains_key(&lower(&vid)) {
                    continue;
                }
                let speed = speeds
                    .iter()
                    .find(|s| s.variation_id.eq_ignore_ascii_case(&vid))
                    .map(|s| f32::from_bits(s.speed_limit_bits))
                    .filter(|v| v.is_finite() && *v > 0.0);
                let texture = st_folder(v, "texture").map(|(tex, folder)| super::output::texture_path(&folder, &tex));
                let weight = fx(v, "weight").unwrap_or(1.0);
                let vi = self.variations.len();
                self.variations.push(VariationDef {
                    ids: vid.clone(),
                    terrain: index,
                    weight,
                    texture,
                    texture_size: [fx(v, "texture_width").unwrap_or(8.0), fx(v, "texture_height").unwrap_or(8.0)],
                    color: pt3(v, "terrain_color").filter(|c| c[0] >= 0.0).unwrap_or(def.color),
                    priority: base_priority + ordinal as i32,
                    speed_limit: speed,
                });
                self.variation_by_id.insert(lower(&vid), vi);
                def.variations.push(vi);
                def.total_weight += weight;
            }
            self.terrain_by_id.insert(lower(&ids), index);
            self.terrains.push(def);
        }
    }

    fn load_things(&mut self, catalog: &ActorCatalog) {
        let ids: Vec<String> = catalog.actors.iter().map(|a| a.id.clone()).collect();
        for id in ids {
            if self.thing_by_id.contains_key(&lower(&id)) {
                continue;
            }
            let Ok(node) = catalog.resolved_actor(ThingRef::registered(&id)) else { continue };
            let source = catalog.registered(&id).map(|a| a.source.path.clone()).unwrap_or_default();
            let folder = source.replace('\\', "/").rsplit_once('/').map_or(String::new(), |(f, _)| f.to_owned());
            let mut def = ThingDef {
                ids: id.clone(),
                category: st(&node, "category").unwrap_or_default(),
                start_priority: int(&node, "starting_location_priority").unwrap_or(0),
                scale: [1.0, 1.0],
                blocking: true,
                ..Default::default()
            };
            if let Some(view) = child(&node, "View") {
                let prefix = st(&view, "resource_prefix").unwrap_or_default();
                let variations = bl(&view, "aspects_as_variations").unwrap_or(false);
                let mut aspects = Vec::new();
                if variations {
                    aspects = children(&view, "Aspect");
                }
                if aspects.is_empty() {
                    aspects.extend(child(&view, "AspectNormal"));
                }
                for aspect in &aspects {
                    let aspect_prefix = st(aspect, "aspect_prefix").unwrap_or_default();
                    let format = st(aspect, "model").unwrap_or_else(|| "%s.nif".into());
                    if !prefix.is_empty() || !format.contains("%s") {
                        if let Some(m) = super::output::model_path(&folder, &prefix, &aspect_prefix, &format) {
                            def.models.push(m);
                        }
                    }
                }
            }
            if let Some(size) = child(&node, "Size") {
                def.scale = [fx(&size, "scale_min").unwrap_or(1.0), fx(&size, "scale_max").unwrap_or(1.0)];
                match st(&size, "blocking_shape").unwrap_or_default().to_ascii_uppercase().as_str() {
                    "SQUARE" => {
                        let s = fx(&size, "square_size").unwrap_or(0.0) * 0.5;
                        def.half_extent = [s, s];
                    }
                    "RECT" | "RECTANGLE" => {
                        def.half_extent = [fx(&size, "rect_width").unwrap_or(0.0) * 0.5, fx(&size, "rect_height").unwrap_or(0.0) * 0.5];
                    }
                    "CIRCLE" => {
                        let r = fx(&size, "blocking_radius").unwrap_or(0.0);
                        def.half_extent = [r, r];
                    }
                    _ => {}
                }
            }
            if let Some(flags) = child(&node, "Flags") {
                def.random_orientation = bl(&flags, "random_orientation").unwrap_or(false);
                def.blocking = bl(&flags, "blocking").unwrap_or(true);
                def.flatten = bl(&flags, "flatten_terrain").unwrap_or(false);
                def.invisible = bl(&flags, "invisible").unwrap_or(false);
            }
            if let Some(base) = child(&node, "BaseTerrain") {
                def.base_default = st(&base, "default_variation_IDS");
                def.base_padding = fx(&base, "padding").unwrap_or(1.0);
                for b in children(&base, "BiomeBaseTerrain") {
                    if let (Some(biome), Some(var)) = (st(&b, "biome_IDS"), st(&b, "variation_IDS")) {
                        def.base_terrain.insert(lower(&biome), var);
                    }
                }
            }
            for random in children(&node, "RandomActor").into_iter().chain(children(&node, "RandomObject")) {
                let target = st(&random, "random_actor_ids").or_else(|| st(&random, "random_object_ids"));
                if let Some(target) = target {
                    self.random_actors.entry(lower(&target)).or_default().push((id.clone(), fx(&random, "weight").unwrap_or(1.0)));
                }
            }
            if let Some(city) = child(&node, "CityComponent") {
                for layout in children(&city, "Layout") {
                    let mut rows = Vec::new();
                    for o in children(&layout, "Object") {
                        if let Some(obj) = st(&o, "object_ids") {
                            rows.push((obj, pt(&o, "offset").unwrap_or([0.0, 0.0]), pt(&o, "orientation").unwrap_or([0.0, -1.0])));
                        }
                    }
                    // 0x473AD0/0x475230: rows end up in reverse authored order.
                    rows.reverse();
                    if !rows.is_empty() {
                        def.layouts.push(rows);
                    }
                }
            }
            self.thing_by_id.insert(lower(&id), self.things.len());
            self.things.push(def);
        }
    }

    fn load_biomes(&mut self, catalog: &ActorCatalog) {
        for b in catalog.resolved_definitions("Biome").unwrap_or_default() {
            let Some(ids) = st(&b, "IDS") else { continue };
            let mut def = BiomeDef { ids: ids.clone(), ..Default::default() };
            for t in children(&b, "Terrain") {
                if let Some(t) = st(&t, "terrain_IDS").and_then(|t| self.terrain_by_id.get(&lower(&t)).copied()) {
                    def.terrains.push(t);
                }
            }
            for tr in children(&b, "Transition") {
                let a = st(&tr, "terrain1_IDS").and_then(|t| self.terrain_by_id.get(&lower(&t)).copied());
                let c = st(&tr, "terrain2_IDS").and_then(|t| self.terrain_by_id.get(&lower(&t)).copied());
                let v = st(&tr, "variation_IDS").and_then(|t| self.variation_by_id.get(&lower(&t)).copied());
                if let (Some(a), Some(c), Some(v)) = (a, c, v) {
                    def.transitions.insert((a, c), v);
                    def.transitions.insert((c, a), v);
                }
            }
            self.biomes.insert(lower(&ids), def);
        }
        // KKC_ObjectData::LoadBiomePlacementData (0x49D1C0): Placement weight,
        // overridden per Biome, overridden per Terrain (with primary_doodad).
        let biome_ids: Vec<String> = self.biomes.keys().cloned().collect();
        let mut lists: BTreeMap<String, BTreeMap<usize, DoodadLists>> = BTreeMap::new();
        for (thing_index, thing) in self.things.iter().enumerate() {
            let Ok(node) = catalog.resolved_actor(ThingRef::registered(&thing.ids)) else { continue };
            let Some(placement) = child(&node, "Placement") else { continue };
            let biomes = children(&placement, "Biome");
            let default = fx(&placement, "weight").unwrap_or(-1.0);
            if default < 0.0 && biomes.is_empty() {
                continue;
            }
            for biome_id in &biome_ids {
                let biome = &self.biomes[biome_id];
                let mut weights: Vec<(f32, bool)> = vec![(default, false); biome.terrains.len()];
                for b in &biomes {
                    let Some(bid) = st(b, "IDS").or_else(|| st(b, "ids")) else { continue };
                    if !bid.eq_ignore_ascii_case(biome_id) {
                        continue;
                    }
                    let bw = fx(b, "weight").unwrap_or(default);
                    weights.iter_mut().for_each(|w| *w = (bw, false));
                    for t in children(b, "Terrain") {
                        let Some(tid) = st(&t, "IDS").and_then(|t| self.terrain_by_id.get(&lower(&t)).copied()) else { continue };
                        if let Some(slot) = biome.terrains.iter().position(|&x| x == tid) {
                            weights[slot] = (fx(&t, "weight").unwrap_or(bw), bl(&t, "primary_doodad").unwrap_or(false));
                        }
                    }
                }
                for (slot, &(w, primary)) in weights.iter().enumerate() {
                    if w > 0.0 {
                        let entry = lists.entry(biome_id.clone()).or_default().entry(biome.terrains[slot]).or_default();
                        if primary {
                            entry.primary.push((thing_index, w));
                        } else {
                            entry.normal.push((thing_index, w));
                        }
                    }
                }
            }
        }
        for (biome, doodads) in lists {
            if let Some(b) = self.biomes.get_mut(&biome) {
                b.doodads = doodads;
            }
        }
    }

    fn group(&self, g: &ResolvedNode<'_>) -> GroupDef {
        GroupDef {
            ids: st(g, "ids").or_else(|| st(g, "IDS")).unwrap_or_default(),
            name: loc(g, "name"),
            slider: slider(g),
            objects_min: int(g, "objects_min").unwrap_or(0),
            required: children(g, "RequiredObject")
                .iter()
                .filter_map(|r| {
                    Some(RequiredDef {
                        object: st(r, "object_ids")?,
                        position: pt(r, "position").unwrap_or([0.0, 0.0]),
                        distance_max: fx(r, "distance_max").unwrap_or(0.0),
                    })
                })
                .collect(),
            objects: children(g, "Object").iter().filter_map(|o| Some((st(o, "object_ids")?, fx(o, "weight").unwrap_or(0.0)))).collect(),
            balanced: bl(g, "balanced_placement").unwrap_or(false),
            search_radius: fx(g, "search_radius").unwrap_or(0.0),
            search_relative_group: st(g, "search_relative_group_ids"),
            search_relative_kingdom: st(g, "search_relative_kingdom_ids"),
            intra_min: fx(g, "intraproximity_min").unwrap_or(0.0),
            intra_desired: fx(g, "intraproximity_desired").unwrap_or(0.0),
            start_weight: fx(g, "starting_location_weight").unwrap_or(100.0),
            restrictions: children(g, "GroupRestriction")
                .iter()
                .filter_map(|r| {
                    Some(RestrictionDef {
                        group_ids: st(r, "group_ids")?,
                        min: fx(r, "proximity_min").unwrap_or(0.0),
                        desired: fx(r, "proximity_desired").unwrap_or(0.0),
                    })
                })
                .collect(),
            kingdom_ids: st(g, "kingdom_ids"),
            all_major_kingdoms: bl(g, "apply_to_all_major_kingdoms").unwrap_or(false),
            properties: st(g, "properties"),
        }
    }

    fn load_rmc(&mut self, rmc: &ResolvedNode<'_>) -> anyhow::Result<Option<RmcDef>> {
        let ids = st(rmc, "IDS").ok_or_else(|| anyhow::anyhow!("RMC without IDS"))?;
        let biome = st(rmc, "biome").unwrap_or_default();
        let mut def = RmcDef {
            ids: ids.clone(),
            name: loc(rmc, "name"),
            description: loc(rmc, "description"),
            biome: biome.clone(),
            list_in_staging: bl(rmc, "list_in_staging").unwrap_or(true),
            reference_size: fx(rmc, "reference_size").unwrap_or(256.0),
            max_percent_water: fx(rmc, "max_percent_water").unwrap_or(-1.0),
            max_percent_mountain: fx(rmc, "max_percent_mountain").unwrap_or(-1.0),
            balanced_kingdoms: bl(rmc, "balanced_kingdoms").unwrap_or(true),
            team_desired_distance: fx(rmc, "team_desired_distance").unwrap_or(96.0),
            sizes: Vec::new(),
            brushes: Vec::new(),
            noise: Vec::new(),
            kingdoms: Vec::new(),
            actor_groups: Vec::new(),
            doodad_groups: Vec::new(),
            heroes: [0, 0],
            heroes_show_ui: false,
            sun: st(rmc, "sun").map(|s| s.split(',').map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()).collect()).unwrap_or_default(),
            weather: st(rmc, "weather").map(|s| s.split(',').map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()).collect()).unwrap_or_default(),
        };
        for s in children(rmc, "MapSize") {
            def.sizes.push(super::RmcMapSize {
                width: fx(&s, "width").unwrap_or(0.0),
                height: fx(&s, "height").unwrap_or(0.0),
                recommended_kingdoms_min: int(&s, "recommended_kingdoms_min").unwrap_or(2).max(0) as u32,
                recommended_kingdoms_max: int(&s, "recommended_kingdoms_max").unwrap_or(2).max(0) as u32,
            });
        }
        // 0x4A4410: terrain brushes in authored order, blobs and streaks interleaved.
        let mut nodes: Vec<(usize, bool, ResolvedNode<'_>)> = Vec::new();
        for b in children(rmc, "TerrainBlob") {
            nodes.push((b.first_source().map_or(0, |s| s.span.start), true, b));
        }
        for b in children(rmc, "TerrainStreak") {
            nodes.push((b.first_source().map_or(0, |s| s.span.start), false, b));
        }
        nodes.sort_by_key(|n| n.0);
        for (_, blob, b) in nodes {
            let Some(terrain) = st(&b, "terrain_ids").and_then(|t| self.terrain_by_id.get(&lower(&t)).copied()) else {
                self.notes.push(format!("{ids}: brush with unknown terrain"));
                continue;
            };
            let attach = st(&b, "attachto")
                .unwrap_or_else(|| "NONE".into())
                .split(',')
                .filter_map(|d| {
                    ["NONE", "SOUTHWEST", "WEST", "NORTHWEST", "NORTH", "NORTHEAST", "EAST", "SOUTHEAST", "SOUTH", "CENTER", "RANDOM"]
                        .iter()
                        .position(|n| n.eq_ignore_ascii_case(d.trim()))
                        .map(|p| p as u8)
                })
                .collect();
            let kind = if blob {
                let micro = child(&b, "MicroBlobs").map(|m| {
                    let r = pt(&m, "ratio_range").unwrap_or([0.0, 0.0]);
                    MicroDef {
                        ratio: [r[0].clamp(0.0, 1.0), r[1].clamp(0.0, 1.0)],
                        size: pt(&m, "blob_size").unwrap_or([1.0, 3.0]).map(|v| v.max(0.0)),
                        affect_height: bl(&m, "affect_height").unwrap_or(true),
                    }
                });
                BrushKind::Blob(BlobDef {
                    size: pt(&b, "blob_size").unwrap_or([1.0, 3.0]),
                    consistency: fx(&b, "consistency").unwrap_or(0.0),
                    proximity: fx(&b, "proximity").unwrap_or(0.0),
                    fuzzy: fx(&b, "fuzzy_factor").unwrap_or(1.0),
                    dir_bias: pt(&b, "dir_bias").unwrap_or([0.0, 0.0]),
                    dir_bias_factor: fx(&b, "dir_bias_factor").unwrap_or(0.0),
                    micro,
                })
            } else {
                BrushKind::Streak(StreakDef {
                    streaks: [int(&b, "streaks_min").unwrap_or(1), int(&b, "streaks_max").unwrap_or(1)],
                    width: pt(&b, "width_range").unwrap_or([4.0, 8.0]),
                    length: pt(&b, "length_range").unwrap_or([4.0, 8.0]),
                    angle: pt(&b, "angle_range").unwrap_or([-30.0, 30.0]),
                })
            };
            def.brushes.push(BrushDef {
                slider: slider(&b),
                terrain,
                paint_first_variation_only: bl(&b, "paint_first_variation_only").unwrap_or(false),
                attach,
                attach_range: fx(&b, "attach_range").unwrap_or(-1.0),
                override_same: bl(&b, "override_same").unwrap_or(false),
                kind,
            });
        }
        for n in children(rmc, "TerrainNoise") {
            def.noise.push(NoiseDef {
                num: [int(&n, "terrain_noise_num_blobs_min").unwrap_or(90), int(&n, "terrain_noise_num_blobs_max").unwrap_or(120)],
                size: [fx(&n, "terrain_noise_blob_size_min").unwrap_or(40.0), fx(&n, "terrain_noise_blob_size_max").unwrap_or(60.0)],
                magnitude: [fx(&n, "terrain_noise_offset_magnitude_min").unwrap_or(5.0), fx(&n, "terrain_noise_offset_magnitude_max").unwrap_or(7.0)],
                up_frequency: fx(&n, "terrain_noise_up_frequency").unwrap_or(0.3),
                reference_size: fx(&n, "terrain_noise_reference_size").unwrap_or(128.0),
                area_factor: fx(&n, "terrain_noise_area_factor").unwrap_or(0.5),
            });
        }
        for k in children(rmc, "Kingdom") {
            let Some(kids) = st(&k, "IDS") else { continue };
            def.kingdoms.push(KingdomDef {
                ids: kids,
                name: loc(&k, "name"),
                independent: bl(&k, "independent").unwrap_or(false),
                color: st(&k, "color"),
                team: st(&k, "team"),
                playable: bl(&k, "playable").unwrap_or(false),
                nation: st(&k, "nation"),
            });
        }
        for g in children(rmc, "ActorGroup") {
            let g = self.group(&g);
            def.actor_groups.push(g);
        }
        for g in children(rmc, "DoodadGroup") {
            let g = self.group(&g);
            def.doodad_groups.push(g);
        }
        if let Some(h) = child(rmc, "Heroes") {
            def.heroes = [int(&h, "random_heroes_min").unwrap_or(0), int(&h, "random_heroes_max").unwrap_or(0)];
            def.heroes_show_ui = bl(&h, "show_ui").unwrap_or(false);
        }
        Ok(Some(def))
    }

    pub(super) fn rmc(&self, ids: &str) -> Option<&RmcDef> {
        self.rmcs.iter().find(|r| r.ids.eq_ignore_ascii_case(ids))
    }
    pub(super) fn thing(&self, ids: &str) -> Option<&ThingDef> {
        self.thing_by_id.get(&lower(ids)).map(|&i| &self.things[i])
    }
}
