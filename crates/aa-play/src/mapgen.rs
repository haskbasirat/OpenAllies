//! Random map generator: the original "RMG" (KKC_RandomMapGenerator,
//! Source\Sim\RandomMap\*.cpp in AA.exe) driven by the original RMC data
//! (Data/RandomMap/rmc_*.tgi on the Templates/template_rmc*.tgi templates).
//! Owner: mapgen.
//!
//! Behaviour-faithful, not byte-exact: the stages, their order and every
//! number follow the original code and data, but the random number stream and
//! the floating point arithmetic are our own (seeded, plain `f32`).
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
}


mod data;
mod grid;
mod height;
mod output;
mod place;
mod rng;
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
    /// Problems met while reading the data (skipped definitions).
    pub fn notes(&self) -> &[String] {
        &self.data.notes
    }
    /// Generate a map; same params + seed = same map.
    pub fn generate(&self, params: &RandomMapParams) -> anyhow::Result<GeneratedMap> {
        generate_with(&self.data, params)
    }
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

fn generate_with(d: &data::RmgData, params: &RandomMapParams) -> anyhow::Result<GeneratedMap> {
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
    let values = terrain_values(rmc, params);
    let mut rng = rng::Rng::new(params.seed);
    let mut notes = Vec::new();
    // Sun and weather: one of the comma-delimited ids at random (0x5925E0).
    let weather = (!rmc.weather.is_empty()).then(|| rmc.weather[rng.below(rmc.weather.len() as u32) as usize].clone());
    let teams: Vec<u32> = if params.teams.len() == params.sides as usize { params.teams.clone() } else { (0..params.sides).collect() };
    let mut last_error = None;
    for attempt in 0..d.svars.retries.max(1) {
        let mut stage_rng = rng.fork(attempt as u64);
        let timing = std::env::var_os("AA_MAPGEN_TIMING").is_some();
        let clock = std::time::Instant::now();
        let mut world = grid::World::new(params.width, params.height, d, initial);
        // StageTerrain, then StageAddNoise.
        terrain::run(&mut world, d, rmc, biome, &values, &mut stage_rng.fork(1));
        if timing {
            eprintln!("mapgen terrain {:?}", clock.elapsed());
        }
        let mut noise_rng = stage_rng.fork(2);
        for n in &rmc.noise {
            height::add_noise(&mut world, d, n, &mut noise_rng);
        }
        if timing {
            eprintln!("mapgen noise {:?}", clock.elapsed());
        }
        // Placement tries (RMGNumPlacementRetries).
        let base = (world.var.clone(), world.base.clone());
        for try_index in 0..d.svars.placement_retries.max(1) {
            world.var.clone_from(&base.0);
            world.base.clone_from(&base.1);
            let mut place_rng = stage_rng.fork(10 + try_index as u64);
            let mut placement = place::Placement::new(d, rmc, &world);
            let counts = &params.object_counts;
            let value = |g: &data::GroupDef| counts.get(&g.ids).copied().unwrap_or_else(|| midpoint(g.slider.range));
            let mut run = || -> anyhow::Result<()> {
                placement.setup_kingdoms(params.sides, &teams)?;
                placement.place_kingdoms(&world, &mut place_rng)?;
                placement.setup_groups();
                placement.place_required(&mut world, &mut place_rng)?;
                placement.place_groups(&mut world, &rmc.biome, &value, &mut place_rng)?;
                Ok(())
            };
            match run() {
                Ok(()) => {
                    if timing {
                        eprintln!("mapgen placement {:?}", clock.elapsed());
                    }
                    if d.svars.doodad_auto {
                        placement.doodify(&world, &rmc.biome, &mut stage_rng.fork(20));
                    }
                    if timing {
                        eprintln!("mapgen doodads {:?}", clock.elapsed());
                    }
                    notes.extend(placement.notes.iter().cloned());
                    let out = output::build(d, params, &world, &placement, weather.as_deref(), notes)?;
                    return Ok(GeneratedMap { params: params.clone(), terrain: out.terrain, placed: out.placed, scenery: out.scenery, map: out.map });
                }
                Err(e) => {
                    notes.push(format!("placement try {try_index} of attempt {attempt}: {e}"));
                    last_error = Some(e);
                }
            }
        }
    }
    Err(last_error
        .unwrap_or_else(|| anyhow::anyhow!("random map generation failed"))
        .context(format!("{} after {} tries", rmc.ids, d.svars.retries)))
}
