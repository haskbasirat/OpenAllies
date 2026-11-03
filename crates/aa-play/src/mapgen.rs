//! Random map generator: the original "RMG" (KKC_RandomMapGenerator,
//! Source\Sim\RandomMap\*.cpp in AA.exe) driven by the original RMC data
//! (Data/RandomMap/rmc_*.tgi on the Templates/template_rmc*.tgi templates).
//! Owner: mapgen.
//!
//! A port of the original's stages on its world stream and x87 arithmetic; the
//! parity gate is `crates/aa-parity/tests/rmg_gate.rs`. Parity probes, as the
//! RmgProbe sampler logs the original's: `AA_RMG_CG` (every creation, stderr),
//! `AA_RMG_CM` / `AA_RMG_CM_DETAIL` (terrain commits), `AA_RMG_BR` (height
//! brushes), `AA_RMG_WORLD` (the final world); `AA_MAPGEN_TIMING` times the stages.
//!
//! The options are exactly what the original random-map dialog
//! (Data/UI/Shared/random_map_dialog.tgi, filled by 0x686B10) offers:
//! the RMC ("Types"), "Map Size", "Sides", one "Terrain" slider per shown
//! RMC terrain brush and one "Objects" slider per shown actor/doodad group,
//! stored like the native KKC_RMCSettings (0x505EA0: m_terrain_weights,
//! m_object_counts, m_num_heroes).
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The settings one random map is generated from (the native world creator's
/// m_rmc, m_width/m_height and m_rmc_settings, plus the side count and teams
/// chosen on the staging screen, plus the seed).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RandomMapParams {
    /// RMC IDS, e.g. "rmc_southerneurope" (the original default,
    /// AVars_AA StagingSinglePlayerDefaultRMCIds). One of `RmcCatalog::rmcs`.
    pub rmc: String,
    /// Map size in metres, one of the RMC's `[MapSize]` entries
    /// (320, 384, 448, 512, 576, 640 square; the v14 data adds 768 and 1024).
    /// Default 384 x 384 (AVars_AA StagingSinglePlayerDefaultRMCWidth/Height).
    pub width: f32,
    pub height: f32,
    /// "Sides": number of major kingdoms that get a start location
    /// (2..=`RmcInfo::max_sides`; 8 original, 16 with the v14 data).
    pub sides: u32,
    /// Team of each side, `teams.len() == sides` (0-based team index; sides on
    /// the same team are placed `team_desired_distance` apart, 96 m in the
    /// data). Empty means every side on its own team.
    pub teams: Vec<u32>,
    /// One value per RMC terrain brush, in RMC order (`RmcInfo::terrain_brushes`),
    /// within that brush's `range`. Only brushes with `show_ui` are sliders.
    /// Defaults are the middle of each range (KKC_RMCSettings 0x506370); a
    /// hidden brush of the same terrain as a shown `override_same` slider
    /// takes that slider's fraction of its own range. Empty or short means
    /// "use the defaults".
    pub terrain_weights: Vec<f32>,
    /// Object slider values by group IDS (`RmcInfo::object_sliders`). Missing
    /// groups use their default.
    pub object_counts: BTreeMap<String, f32>,
    /// Random heroes per kingdom (the RMC [Heroes] node; hidden in A&A).
    pub heroes: u32,
    /// Same params + same seed = same map.
    pub seed: u64,
}

/// What the random map dialog shows for every RMC (read from the original
/// data by [`RmcCatalog::load`]).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RmcCatalog {
    /// In data load order (Data/RandomMap file order), only `list_in_staging`.
    pub rmcs: Vec<RmcInfo>,
    /// AVars_AA StagingSinglePlayerDefaultRMCIds.
    pub default_rmc: String,
    /// AVars_AA StagingSinglePlayerDefaultRMCWidth/Height.
    pub default_size: [f32; 2],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RmcInfo {
    pub ids: String,
    /// Localization keys ("#rmc_france_name"), resolved by the UI.
    pub name: String,
    pub description: String,
    pub biome: String,
    pub map_sizes: Vec<RmcMapSize>,
    /// Number of playable major kingdoms the RMC defines (the "Sides" maximum).
    pub max_sides: u32,
    /// Every terrain brush in RMC order (`RandomMapParams::terrain_weights`
    /// has one value per entry); the UI shows those with `show_ui`.
    pub terrain_brushes: Vec<RmcSlider>,
    /// Actor and doodad groups with `show_ui` (the "Objects" sliders).
    pub object_sliders: Vec<RmcSlider>,
    pub heroes_range: [u32; 2],
    pub heroes_show_ui: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RmcMapSize {
    pub width: f32,
    pub height: f32,
    pub recommended_kingdoms_min: u32,
    pub recommended_kingdoms_max: u32,
}

/// One RMCSliderOption (template_rmc.tgi): `range`, `show_ui`, `locked`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RmcSlider {
    /// Terrain brush: the terrain IDS; object group: the group IDS.
    pub ids: String,
    /// Slider label: the terrain's / group's localized name key.
    pub name: String,
    pub range: [f32; 2],
    pub default: f32,
    pub show_ui: bool,
    /// Shown but not adjustable.
    pub locked: bool,
}

impl RandomMapParams {
    /// The original staging defaults for `rmc` (sliders at their defaults).
    pub fn defaults(info: &RmcInfo, sides: u32, seed: u64) -> Self {
        let size = info
            .map_sizes
            .iter()
            .find(|s| s.width == 384.0)
            .or(info.map_sizes.first())
            .copied()
            .unwrap_or(RmcMapSize { width: 384.0, height: 384.0, recommended_kingdoms_min: 2, recommended_kingdoms_max: 4 });
        Self {
            rmc: info.ids.clone(),
            width: size.width,
            height: size.height,
            sides,
            teams: (0..sides).collect(),
            terrain_weights: info.terrain_brushes.iter().map(|b| b.default).collect(),
            object_counts: info.object_sliders.iter().map(|s| (s.ids.clone(), s.default)).collect(),
            heroes: (info.heroes_range[1].saturating_sub(info.heroes_range[0])) / 2 + info.heroes_range[0],
            seed,
        }
    }
}

/// A placed scenery object with the original model the renderer draws.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratedPlacement {
    pub thing_id: String,
    /// Original model path, e.g. "Doodads/Temperate/TemperateTrees/TemperateTree02.nif".
    pub model: Option<String>,
    /// Native x, y (ground), z (height).
    pub position: [f32; 3],
    /// Same convention as aa-assets placement yaw.
    pub yaw: f32,
    pub scale: f32,
}

/// One generated battlefield.
#[derive(Clone, Debug)]
pub struct GeneratedMap {
    pub params: RandomMapParams,
    pub terrain: aa_assets::TerrainMap,
    /// Everything placed, in placement order, like the RMP DYNAMIC_THINGS of
    /// a real map (also `map.placed`): per side a `start_loc`, the
    /// `placeholder_corpsunpacked` / `placeholder_depot_supply` / bunker
    /// placeholders the core replaces with nation buildings, the villages
    /// (centre + city layout buildings, owned by the independent "Enemies"
    /// kingdom slot), then the auto-placed doodads (trees, rocks, bushes...).
    pub placed: Vec<crate::map::PlacedThing>,
    /// `placed` again, one entry per thing, with the original model to draw
    /// (the chosen aspect of aspects_as_variations things; None for invisible
    /// things such as the audio doodads).
    pub scenery: Vec<GeneratedPlacement>,
    pub map: crate::map::GameMap,
    /// Tile +0x2C of every tile: the water area id (0 none).
    pub water_areas: Vec<u8>,
    /// Every thing standing when the session starts, in GTID order.
    pub things: Vec<GeneratedThing>,
    /// The generator's log in the original's terms (its RMG log lines:
    /// normalized brush weights, marker balance scores, ...), for parity.
    pub log: Vec<LogLine>,
    /// What ConfigureGWorld 0x50E8B0 picked, by `WorldSpec` kingdom, in a world
    /// generated from a creator's lists (`generate_world`): the nation (the row's
    /// or 0x50ECA6's) and the general a row left open (0x474C14). Empty otherwise.
    pub world_picks: Vec<WorldPick>,
}

/// One kingdom's picks of [`GeneratedMap::world_picks`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldPick {
    pub nation: Option<String>,
    pub general: Option<String>,
}

/// A thing of a generated world as the original's world dump shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedThing {
    /// The slot of its GTID.
    pub id: u32,
    pub ids: String,
    /// Position and height.
    pub position: [f32; 3],
    pub facing: [f32; 2],
}

/// One generator log line: the original's message (without its numbers)
/// and the numbers it prints, as the doubles it passes to the logger.
#[derive(Clone, Debug, PartialEq)]
pub struct LogLine {
    pub what: String,
    pub values: Vec<f64>,
}


mod blob;
mod data;
mod grid;
pub(crate) mod height;
mod output;
mod place;
mod rng;
mod space;
mod terrain;

/// The original random-map data, read once (about a second) and reused for
/// the options screen and for every generation.
pub struct RandomMapData {
    data: data::RmgData,
}

impl RandomMapData {
    /// Read the RMC, terrain, biome and thing data from the original install
    /// (`data_root` holds Data.rwd, as for `GameMap::load`).
    pub fn load(data_root: &std::path::Path) -> anyhow::Result<Self> {
        let catalog = data::RmgData::open_catalog(data_root)?;
        Ok(Self { data: data::RmgData::load(&catalog)? })
    }
    /// The battlefield of a map file the extracted set does not hold (a map of a
    /// profile's data, a saved game), as the client draws a generated one: `bytes`
    /// is the file, `map` its loaded `GameMap`, and `scenery` the `map.placed`
    /// indices that stayed scenery (`Books::scenery`). The palette and the models
    /// are this data's. `params` names the file; nothing is generated.
    pub fn scene_of_file(&self, map: &crate::map::GameMap, bytes: Vec<u8>, scenery: &[usize]) -> anyhow::Result<GeneratedMap> {
        let archive = aa_mapworld::MapArchive::from_bytes(bytes)?;
        let chunk = aa_mapworld::terrain::TerrainChunk::decode(&archive.decode_chunk("TERRAIN", 1 << 30)?)?;
        let native = map.native_terrain.as_ref().ok_or_else(|| anyhow::anyhow!("{}: no height layers", map.id))?;
        let d = &self.data;
        let mut used = std::collections::BTreeSet::new();
        let tiles: Vec<u32> = chunk
            .tiles
            .iter()
            .map(|t| {
                let index = u32::from(t.variation_index());
                used.insert(index);
                index
            })
            .collect();
        let mut palette = BTreeMap::new();
        for index in used {
            let Some(id) = chunk.palette_utf16.get(index as usize).map(|u| String::from_utf16_lossy(u)) else { continue };
            let def = d.variation_by_id.get(&id.to_ascii_lowercase()).map(|&v| &d.variations[v as usize]);
            palette.insert(
                index,
                aa_assets::TerrainPaletteEntry {
                    texture: def.and_then(|v| v.texture.clone()),
                    color: def.map_or([0.4, 0.5, 0.3], |v| v.color),
                    width: def.map(|v| v.texture_size[0]),
                    height: def.map(|v| v.texture_size[1]),
                    id,
                },
            );
        }
        let terrain = aa_assets::TerrainMap {
            id: map.id.clone(),
            name: map.name.clone(),
            description: String::new(),
            source: map.id.clone(),
            width: map.width,
            height: map.height,
            cell_size: map.cell_size,
            biomes: vec![map.biome.clone()],
            tiles,
            heights: native.base.clone(),
            mountain_heights: native.overlay.clone(),
            palette,
            limitations: vec!["heights = base HEIGHT_MAP, mountainHeights = MOUNTAIN_HEIGHT_MAP overlay; draw with the native conditional height.".into()],
        };
        terrain.validate()?;
        let placements = scenery
            .iter()
            .filter_map(|&i| {
                let p = map.placed.get(i)?;
                let def = d.thing_by_id.get(&p.thing_id.to_ascii_lowercase()).map(|&t| &d.things[t as usize])?;
                if def.invisible {
                    return None;
                }
                let model = if p.variant >= 0 { def.models.get(p.variant as usize) } else { def.models.first() }.or_else(|| def.models.first())?;
                Some(GeneratedPlacement {
                    thing_id: p.thing_id.clone(),
                    model: Some(model.clone()),
                    position: p.position,
                    yaw: p.facing[0].atan2(-p.facing[1]),
                    scale: p.scale,
                })
            })
            .collect();
        Ok(GeneratedMap {
            params: RandomMapParams {
                rmc: map.id.clone(),
                width: map.world_extents[0],
                height: map.world_extents[1],
                sides: 0,
                teams: Vec::new(),
                terrain_weights: Vec::new(),
                object_counts: BTreeMap::new(),
                heroes: 0,
                seed: 0,
            },
            terrain,
            placed: map.placed.clone(),
            scenery: placements,
            map: map.clone(),
            world_picks: Vec::new(),
            water_areas: Vec::new(),
            things: Vec::new(),
            log: Vec::new(),
        })
    }
    /// [`RandomMapData::load`] on a loose folder, a profile's data over the game's.
    pub fn load_on(data_root: &std::path::Path, folder: &crate::data::DataFolder) -> anyhow::Result<Self> {
        Ok(Self { data: data::RmgData::load(folder.catalog(data_root)?)? })
    }
    /// What the random map dialog lists.
    pub fn catalog(&self) -> RmcCatalog {
        let d = &self.data;
        let rmcs = d
            .rmcs
            .iter()
            .filter(|r| r.list_in_staging)
            .map(|r| RmcInfo {
                ids: r.ids.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
                biome: r.biome.clone(),
                map_sizes: r.sizes.clone(),
                max_sides: r.kingdoms.iter().filter(|k| !k.independent && k.playable).count() as u32,
                terrain_brushes: r
                    .brushes
                    .iter()
                    .map(|b| RmcSlider {
                        ids: d.terrains[b.terrain].ids.clone(),
                        name: d.terrains[b.terrain].name.clone(),
                        range: b.slider.range,
                        default: midpoint(b.slider.range),
                        show_ui: b.slider.show_ui,
                        locked: b.slider.locked,
                    })
                    .collect(),
                object_sliders: r
                    .actor_groups
                    .iter()
                    .chain(&r.doodad_groups)
                    .filter(|g| g.slider.show_ui && !g.objects.is_empty())
                    .map(|g| RmcSlider { ids: g.ids.clone(), name: g.name.clone(), range: g.slider.range, default: midpoint(g.slider.range), show_ui: true, locked: g.slider.locked })
                    .collect(),
                heroes_range: [r.heroes[0].max(0) as u32, r.heroes[1].max(0) as u32],
                heroes_show_ui: r.heroes_show_ui,
            })
            .collect();
        RmcCatalog { rmcs, default_rmc: d.default_rmc.clone(), default_size: d.default_size }
    }
    /// What the world creator of a random map keeps for `params`
    /// (`KKC_WorldCreatorRandom`, `aa_mapworld::header::RandomCreatorSettings`):
    /// the RMC's kingdoms and teams and the settings as the generator reads them.
    pub fn creator_parts(&self, params: &RandomMapParams) -> anyhow::Result<CreatorParts> {
        let rmc = self.data.rmc(&params.rmc).ok_or_else(|| anyhow::anyhow!("unknown RMC {}", params.rmc))?;
        let values = terrain_values(rmc, params);
        let terrain_weights: Vec<(String, f32)> = rmc.brushes.iter().zip(&values).map(|(b, v)| (self.data.terrains[b.terrain].ids.clone(), *v)).collect();
        let total_terrain_weight = values.iter().sum();
        // 0x505EA0 stores every group of the RMC, doodads too, and one row per major
        // kingdom of a group for all of them (`group_ids`, what 0x505B00 compares a
        // film's list with); 0x506370's midpoint is its f64 sum.
        let default = |r: [f32; 2]| ((f64::from(r[1]) - f64::from(r[0])) * 0.5 + f64::from(r[0])) as f32;
        let mut object_counts = Vec::new();
        for g in rmc.actor_groups.iter().chain(&rmc.doodad_groups) {
            let value = |ids: &str| params.object_counts.get(ids).or_else(|| params.object_counts.get(&g.ids)).copied().unwrap_or_else(|| default(g.slider.range));
            if g.all_major_kingdoms {
                for k in rmc.kingdoms.iter().filter(|k| k.is_major()) {
                    let ids = format!("{}_{}", g.ids, k.ids);
                    let v = value(&ids);
                    object_counts.push((ids, v));
                }
            } else {
                object_counts.push((g.ids.clone(), value(&g.ids)));
            }
        }
        let teams = rmc.teams.clone();
        Ok(CreatorParts {
            kingdoms: rmc.kingdoms.iter().map(|k| RmcKingdom { ids: k.ids.clone(), name: k.name.clone(), independent: k.independent, color: k.color.clone(), team: k.team.clone() }).collect(),
            teams,
            terrain_weights,
            total_terrain_weight,
            object_counts,
        })
    }
    /// KKC_WorldCreatorRandom reading a film's settings (0x513A30): 0x505B00
    /// compares the stored brush terrains and object groups with the RMC's, by
    /// exact ids in order, and when they differ the creator takes the RMC's
    /// defaults (0x506370) instead: a film recorded on data whose RMC lists other
    /// groups (8 sides against 16, a mod's) is not generated from what it stored.
    pub fn film_settings(&self, rmc: &str, terrain: &[(String, f32)], objects: &[(String, f32)], heroes: u32) -> anyhow::Result<FilmSettings> {
        let rmc = self.data.rmc(rmc).ok_or_else(|| anyhow::anyhow!("unknown RMC {rmc}"))?;
        let brushes: Vec<&str> = rmc.brushes.iter().map(|b| self.data.terrains[b.terrain].ids.as_str()).collect();
        let groups = rmc.group_ids();
        let fits = brushes.len() == terrain.len()
            && groups.len() == objects.len()
            && brushes.iter().zip(terrain).all(|(a, b)| *a == b.0)
            && groups.iter().zip(objects).all(|(a, b)| *a == b.0);
        if fits {
            return Ok(FilmSettings { fits, terrain_weights: terrain.iter().map(|t| t.1).collect(), object_counts: objects.iter().cloned().collect(), heroes });
        }
        // 0x506370.
        let default = |r: [f32; 2]| ((f64::from(r[1]) - f64::from(r[0])) * 0.5 + f64::from(r[0])) as f32;
        let all = rmc.actor_groups.iter().chain(&rmc.doodad_groups);
        let mut object_counts = BTreeMap::new();
        for g in all {
            let value = default(g.slider.range);
            if g.all_major_kingdoms {
                for k in rmc.kingdoms.iter().filter(|k| k.is_major()) {
                    object_counts.insert(format!("{}_{}", g.ids, k.ids), value);
                }
            } else {
                object_counts.insert(g.ids.clone(), value);
            }
        }
        Ok(FilmSettings {
            fits,
            terrain_weights: rmc.brushes.iter().map(|b| default(b.slider.range)).collect(),
            object_counts,
            heroes: ((rmc.heroes[1] - rmc.heroes[0]) / 2 + rmc.heroes[0]).max(0) as u32,
        })
    }
    /// Problems met while reading the data (skipped definitions).
    pub fn notes(&self) -> &[String] {
        &self.data.notes
    }
    /// Generate a map; same params + seed = same map.
    pub fn generate(&self, params: &RandomMapParams) -> anyhow::Result<GeneratedMap> {
        generate_with(&self.data, params, None, None, false).map(|(m, _)| m)
    }
    /// Generate on a world stream another part already drew from (natively
    /// the creator's kingdoms and their random nation/general picks draw
    /// between the seeding and the generator); `params.seed` is not used.
    pub fn generate_from(&self, params: &RandomMapParams, start: &StreamStart) -> anyhow::Result<GeneratedMap> {
        generate_with(&self.data, params, None, Some(start), false).map(|(m, _)| m)
    }
    /// [`Self::generate_from`] (or [`Self::generate`] without `start`) that
    /// also returns every draw as (stream state before it, native call site:
    /// the return address after the original's 0x85D840 call or after its
    /// unit/modulo wrapper), for the parity harness.
    pub fn generate_traced(&self, params: &RandomMapParams, start: Option<&StreamStart>) -> anyhow::Result<(GeneratedMap, Vec<(u32, u32)>)> {
        generate_with(&self.data, params, None, start, true)
    }
    /// The whole start of a world as the original runs it for a world creator's
    /// lists (a film's): seeded from `params.seed`, the teams and kingdoms and
    /// their picks, the generator, the session start. `rng_state` of the map is
    /// the stream at the world's sync dump 0x4F8AE0.
    pub fn generate_world(&self, params: &RandomMapParams, spec: &WorldSpec, game: &crate::data::GameData) -> anyhow::Result<GeneratedMap> {
        generate_with(&self.data, params, Some((spec, game)), None, false).map(|(m, _)| m)
    }
    /// [`Self::generate_world`] with every draw, as [`Self::generate_traced`].
    pub fn generate_world_traced(&self, params: &RandomMapParams, spec: &WorldSpec, game: &crate::data::GameData) -> anyhow::Result<Traced> {
        generate_with(&self.data, params, Some((spec, game)), None, true)
    }
}

/// [`RandomMapData::film_settings`]: what the creator keeps of a film's settings.
#[derive(Clone, Debug, PartialEq)]
pub struct FilmSettings {
    /// Whether the stored settings fit the RMC (0x505B00); else the rest is the
    /// RMC's defaults.
    pub fits: bool,
    /// One value per RMC brush (`RandomMapParams::terrain_weights`).
    pub terrain_weights: Vec<f32>,
    /// By group, per major kingdom a group of all of them
    /// (`RandomMapParams::object_counts`).
    pub object_counts: BTreeMap<String, f32>,
    pub heroes: u32,
}

/// One kingdom of an RMC in its order (the creator's kingdom list, whose
/// position is Kingdom+0x188).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RmcKingdom {
    pub ids: String,
    /// Its display text.
    pub name: String,
    pub independent: bool,
    pub color: Option<String>,
    pub team: Option<String>,
}

/// See [`RandomMapData::creator_parts`].
#[derive(Clone, Debug, PartialEq)]
pub struct CreatorParts {
    pub kingdoms: Vec<RmcKingdom>,
    pub teams: Vec<String>,
    /// `m_terrain_weights`: per brush its terrain IDS and weight.
    pub terrain_weights: Vec<(String, f32)>,
    pub total_terrain_weight: f32,
    /// `m_object_counts`: per group row (`RmcDef::group_ids`) its IDS and count.
    pub object_counts: Vec<(String, f32)>,
}

/// The world stream (GWorld+0xD4) where a caller left it: the state and
/// every getter entry so far (see `GameMap::rng_log`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamStart {
    pub state: u32,
    pub log: Vec<u32>,
}

impl RmcCatalog {
    /// Read every RMC the original staging screen lists.
    pub fn load(data_root: &std::path::Path) -> anyhow::Result<Self> {
        Ok(RandomMapData::load(data_root)?.catalog())
    }
}

/// KKC_RMCSettings defaults (0x506370): every slider at the middle of its range.
fn midpoint(r: [f32; 2]) -> f32 {
    (r[1] - r[0]) * 0.5 + r[0]
}

/// Generate a random map from the original RMC data under `data_root` (the
/// original install root holding Data.rwd, see `GameMap::load`).
pub fn generate(params: &RandomMapParams, data_root: &std::path::Path) -> anyhow::Result<GeneratedMap> {
    RandomMapData::load(data_root)?.generate(params)
}

type Traced = (GeneratedMap, Vec<(u32, u32)>);

/// Slider values per brush: the params, defaults for missing ones, and
/// hidden brushes following an `override_same` slider of the same terrain
/// (same fraction of their own range).
fn terrain_values(rmc: &data::RmcDef, params: &RandomMapParams) -> Vec<f32> {
    let mut values: Vec<f32> = rmc
        .brushes
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let r = b.slider.range;
            params.terrain_weights.get(i).copied().unwrap_or_else(|| midpoint(r)).clamp(r[0].min(r[1]), r[0].max(r[1]))
        })
        .collect();
    for (i, b) in rmc.brushes.iter().enumerate() {
        if !(b.override_same && b.slider.show_ui) {
            continue;
        }
        let span = b.slider.range[1] - b.slider.range[0];
        let f = if span != 0.0 { (values[i] - b.slider.range[0]) / span } else { 0.5 };
        for (j, o) in rmc.brushes.iter().enumerate() {
            if j != i && o.terrain == b.terrain && !o.slider.show_ui {
                values[j] = o.slider.range[0] + f * (o.slider.range[1] - o.slider.range[0]);
            }
        }
    }
    values
}

/// One kingdom of the world creator's list (WP::CreateKingdomsAndTeams), as a
/// film keeps it: an empty nation or general is the staging "Random" that
/// ConfigureGWorld 0x50E8B0 draws.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldKingdom {
    pub ids: String,
    pub independent: bool,
    pub nation: String,
    pub general: String,
}

/// The world creator's lists a world starts from (`KKC_WorldCreatorRandom`):
/// how many teams it creates first, then its kingdoms in order. Creating them
/// takes the first GTIDs and draws from the world stream before the generator.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldSpec {
    pub teams: u32,
    pub kingdoms: Vec<WorldKingdom>,
    /// The sides that stay: ConfigureGWorld 0x50E8B0 destroys a kingdom whose KingdomInfo
    /// +0x31 is set once the random settings are resolved, so the generator sees the
    /// rest (the creator's rows with `byte_31` 0, which the map header's kingdoms are:
    /// `side05` and no `side03`). Empty: the first `sides` of the RMC.
    pub present: Vec<String>,
}

/// CreateKingdomsAndTeams 0x512200 (a kingdom draws its scale 0x49B458 and two
/// queue offsets 0x50B2EA; a team draws nothing), then ConfigureGWorld 0x50E8B0:
/// every kingdom without a nation draws one until a playable one comes
/// (0x50ECA6, GameData +0x80 in registry order), and without a general the
/// getter and, among more than one public faction of the nation, a pick
/// (0x474B40, site 0x474C14). Returns the nations by kingdom.
fn world_start(d: &data::RmgData, spec: &WorldSpec, rng: &mut rng::Rng) -> (Vec<Option<String>>, Vec<KingdomDraw>, Vec<Option<String>>) {
    let mut draws = Vec::new();
    for i in 0..spec.kingdoms.len() {
        rng.at(0x49B458).next_u32();
        // The Properties component on queue 1 (0x563FD0), the economy on queue 0 (0x4E8535).
        draws.push(KingdomDraw { kingdom: i, state: rng.state(), economy: false });
        rng.at(0x50B2EA).next_u32();
        draws.push(KingdomDraw { kingdom: i, state: rng.state(), economy: true });
        rng.at(0x50B2EA).next_u32();
    }
    let mut generals = vec![None; spec.kingdoms.len()];
    let nations = spec.kingdoms
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let mut nation = (!k.nation.is_empty()).then(|| k.nation.clone());
            // GWorld+0x154 holds the kingdoms that are no independents.
            if k.independent {
                return nation;
            }
            if nation.is_none() && !d.nations.is_empty() {
                let n = d.nations.len() as u32;
                loop {
                    let pick = &d.nations[(rng.at(0x50ECA6).next_u32() % n) as usize];
                    if pick.playable {
                        nation = Some(pick.ids.clone());
                        break;
                    }
                }
            }
            if k.general.is_empty() {
                let def = nation.as_ref().and_then(|n| d.nations.iter().find(|x| x.ids.eq_ignore_ascii_case(n)));
                if let Some(def) = def.filter(|def| !def.factions.is_empty()) {
                    rng.getter();
                    let pick = if def.factions.len() > 1 {
                        rng.at(0x474C14).draw_after_getter() as usize % def.factions.len()
                    } else {
                        0
                    };
                    generals[i] = Some(def.factions[pick].clone());
                }
            }
            nation
        })
        .collect();
    (nations, draws, generals)
}

/// A kingdom's registration draw of [`world_start`].
#[derive(Clone, Copy, Debug)]
struct KingdomDraw {
    kingdom: usize,
    state: u32,
    economy: bool,
}

fn generate_with(d: &data::RmgData, params: &RandomMapParams, world_of: Option<(&WorldSpec, &crate::data::GameData)>, start: Option<&StreamStart>, trace: bool) -> anyhow::Result<Traced> {
    let spec = world_of.map(|w| w.0);
    let game = world_of.map(|w| w.1);
    let rmc = d.rmc(&params.rmc).ok_or_else(|| anyhow::anyhow!("unknown RMC {}", params.rmc))?;
    let biome = d.biomes.get(&rmc.biome.to_ascii_lowercase()).ok_or_else(|| anyhow::anyhow!("{}: unknown biome {}", rmc.ids, rmc.biome))?;
    anyhow::ensure!(
        params.width >= 32.0 && params.height >= 32.0 && params.width.is_finite() && params.height.is_finite(),
        "map size {}x{} too small",
        params.width,
        params.height
    );
    let max_sides = rmc.kingdoms.iter().filter(|k| !k.independent && k.playable).count() as u32;
    anyhow::ensure!(params.sides >= 1 && params.sides <= max_sides, "{} sides requested, {} allows 1..={max_sides}", params.sides, rmc.ids);
    // The biome's initial variation (first Variation of its first Terrain).
    let initial = biome
        .terrains
        .first()
        .and_then(|&t| d.terrains[t].variations.first().copied())
        .ok_or_else(|| anyhow::anyhow!("biome {} has no terrain", biome.ids))? as u32;
    // A world creator's settings are used as they are (0x513A30, 0x598260); the
    // sliders' rules apply to what the options screen hands in.
    let values: Vec<f32> = match world_of {
        Some(_) => rmc.brushes.iter().enumerate().map(|(i, b)| params.terrain_weights.get(i).copied().unwrap_or_else(|| midpoint(b.slider.range))).collect(),
        None => terrain_values(rmc, params),
    };
    let teams: Vec<u32> = if params.teams.len() == params.sides as usize { params.teams.clone() } else { (0..params.sides).collect() };
    let mut notes = Vec::new();
    let mut last_error = None;
    let mut seed = params.seed;
    // KKC_WorldCreatorRandom::CreateGWorld 0x513410 ("RMG: %s Try #%d"): a failed
    // world is thrown away and the next starts from the stream's state.
    for attempt in 0..d.svars.retries.max(1) {
        let mut rng = match start {
            Some(s) if attempt == 0 => rng::Rng::resume(s.state, s.log.clone()),
            _ => rng::Rng::new(seed),
        };
        if trace {
            rng.trace();
        }
        let started = spec.map(|spec| world_start(d, spec, &mut rng));
        let (nations, kingdom_draws, generals) = match started {
            Some((n, k, g)) => (Some(n), k, g),
            None => (None, Vec::new(), Vec::new()),
        };
        match generate_world_once(d, rmc, biome, params, spec, game, nations.as_deref(), &kingdom_draws, initial, &values, &teams, &mut notes, &mut rng) {
            Ok(mut made) => {
                made.0.map.rng_state = Some(rng.state());
                made.0.map.rng_log = rng.log();
                made.1 = rng.take_trace();
                made.0.world_picks = nations
                    .iter()
                    .flatten()
                    .zip(&generals)
                    .map(|(nation, general)| WorldPick { nation: nation.clone(), general: general.clone() })
                    .collect();
                return Ok(made);
            }
            Err(e) => {
                notes.push(format!("{} try {attempt}: {e:#}", rmc.ids));
                last_error = Some(e);
                seed = u64::from(rng.state());
            }
        }
    }
    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("random map generation failed"))
        .context(format!("{} after {} tries", rmc.ids, d.svars.retries)))
}

/// KKC_RandomMapGenerator::_GenerateMap 0x5925E0, then the session start.
#[allow(clippy::too_many_arguments)]
fn generate_world_once(
    d: &data::RmgData,
    rmc: &data::RmcDef,
    biome: &data::BiomeDef,
    params: &RandomMapParams,
    spec: Option<&WorldSpec>,
    game: Option<&crate::data::GameData>,
    nations: Option<&[Option<String>]>,
    kingdom_draws: &[KingdomDraw],
    initial: u32,
    values: &[f32],
    teams: &[u32],
    notes: &mut Vec<String>,
    rng: &mut rng::Rng,
) -> anyhow::Result<Traced> {
    let timing = std::env::var_os("AA_MAPGEN_TIMING").is_some();
    let clock = std::time::Instant::now();
    // The sun, then the weather, each a getter and, when the RMC lists any, one
    // draw % count.
    rng.getter();
    if !rmc.sun.is_empty() {
        let _sun = &rmc.sun[(rng.at(0x5926B2).draw_after_getter() % rmc.sun.len() as u32) as usize];
    }
    rng.getter();
    let weather = (!rmc.weather.is_empty()).then(|| rmc.weather[(rng.at(0x59270F).draw_after_getter() % rmc.weather.len() as u32) as usize].clone());
    let mut world = grid::World::new(params.width, params.height, d, initial);
    // StageTerrain, then StageAddNoise.
    let terrain_log: Vec<LogLine> = rmc
        .brushes
        .iter()
        .zip(terrain::normalized_weights(d, rmc, values))
        .map(|(b, w)| LogLine { what: format!("terrain {} normalized weight", d.terrains[b.terrain].ids), values: vec![f64::from(w)] })
        .collect();
    terrain::run(&mut world, d, rmc, biome, values, rng);
    world.take_changed();
    if timing {
        eprintln!("mapgen terrain {:?}", clock.elapsed());
    }
    for n in &rmc.noise {
        height::add_noise(&mut world, d, n, rng);
    }
    if timing {
        eprintln!("mapgen noise {:?}", clock.elapsed());
    }
    // The GTIDs the teams and kingdoms took (a kingdom left out keeps its slot
    // used up to the cursor: the allocator is next-fit).
    let first_id = match spec {
        Some(spec) => spec.teams + spec.kingdoms.len() as u32,
        None => rmc.teams.len() as u32 + rmc.kingdoms.len() as u32,
    };
    let counts = &params.object_counts;
    let value = |ids: &str, g: &data::GroupDef| counts.get(ids).or_else(|| counts.get(&g.ids)).copied().unwrap_or_else(|| midpoint(g.slider.range));
    let mut placement = place::Placement::new(d, rmc, biome, &world, first_id, game);
    // The placement tries (RMGNumPlacementRetries): 0x4F5770(0) deletes the
    // things but the ground stays as they left it.
    let mut failed = None;
    for try_index in 0..d.svars.placement_retries.max(1) {
        if try_index != 0 {
            placement.start_over(&world);
        }
        let mut run = || -> anyhow::Result<()> {
            placement.setup_kingdoms(params.sides, teams, spec)?;
            if let (Some(spec), Some(nations)) = (spec, nations) {
                placement.set_nations(spec, nations);
            }
            placement.balance_log = placement.place_kingdoms_exact(&world, rng)?;
            placement.create_start_locations(&mut world, rng);
            placement.setup_groups();
            placement.place_required_exact(&mut world, rng)?;
            placement.place_groups_exact(&mut world, &value, rng)?;
            Ok(())
        };
        match run() {
            Ok(()) => {
                failed = None;
                break;
            }
            Err(e) => {
                notes.push(format!("placement try {try_index}: {e:#}"));
                failed = Some(e);
            }
        }
    }
    if let Some(e) = failed {
        return Err(e);
    }
    if timing {
        eprintln!("mapgen placement {:?}", clock.elapsed());
    }
    // 0x7ACB00, the water doodads 0x555F20, the auto doodads 0x552BF0 (when
    // MapDoodadAutoPlacement), the cleanup 0x592170, the heroes 0x590F90.
    placement.refresh_islands();
    placement.place_water_doodads(&mut world, rng);
    if d.svars.doodad_auto {
        placement.place_auto_doodads(&mut world, rng);
    }
    placement.cleanup_blocking_doodads();
    placement.place_heroes(rng, params.heroes);
    if timing {
        eprintln!("mapgen doodads {:?}", clock.elapsed());
    }
    // The session start: ResolvePlaceholdersAndRandomThings 0x4FDA70.
    placement.resolve_session(&mut world, rng);
    if timing {
        eprintln!("mapgen session {:?}", clock.elapsed());
    }
    placement.dump_probe(&world);
    notes.extend(placement.notes.iter().cloned());
    let mut out = output::build(d, params, &world, &placement, weather.as_deref(), notes.clone())?;
    if let Some(spec) = spec {
        placement.world_state(&mut out.map, spec, first_id, kingdom_draws);
    }
    let mut log = terrain_log;
    log.extend(placement.balance_log.iter().chain(&placement.group_balance_log).map(|&(i, s)| LogLine { what: "balance iteration".into(), values: vec![f64::from(i), s] }));
    log.extend(placement.marker_log.iter().map(|&(what, i, p)| LogLine { what: what.into(), values: vec![f64::from(i), f64::from(p[0]), f64::from(p[1])] }));
    let things = placement.things(d);
    Ok((GeneratedMap { params: params.clone(), terrain: out.terrain, placed: out.placed, scenery: out.scenery, map: out.map, water_areas: world.area.clone(), things, log, world_picks: Vec::new() }, Vec::new()))
}
