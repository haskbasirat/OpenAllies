use crate::types::ModelManifest;
use crate::*;
use rayon::prelude::*;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// One `[Variation]` of a terrain file, as a palette entry takes it.
struct VariationArt {
    texture: String,
    width: Option<f32>,
    height: Option<f32>,
    color: Option<[f32; 3]>,
}

/// A TGI file's lines as (lower-case key, value): comments out, a type word before
/// the key dropped (`fixed noise_min = 0.25`), a node header as (header, "").
fn tgi_lines(text: &str) -> Vec<(String, String)> {
    let mut clean = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("/*") {
        clean.push_str(&rest[..at]);
        match rest[at..].find("*/") {
            Some(end) => rest = &rest[at + end + 2..],
            None => rest = "",
        }
    }
    clean.push_str(rest);
    clean
        .lines()
        .filter_map(|line| {
            let line = line.split(';').next().unwrap_or("").trim();
            if line.starts_with('[') {
                return Some((line.to_ascii_lowercase(), String::new()));
            }
            let (key, value) = line.split_once('=')?;
            let key = key.split_whitespace().last()?.to_ascii_lowercase();
            Some((key, value.trim().trim_matches('"').to_owned()))
        })
        .collect()
}

fn tgi_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            tgi_files(&path, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("tgi")) {
            out.push(path);
        }
    }
}

/// What `Data/Terrain/**/*.tgi` holds for a map's palette.
#[derive(Default)]
struct TerrainArt {
    /// Every variation by lower-case IDS. A value the variation leaves out is its
    /// terrain's.
    variations: BTreeMap<String, VariationArt>,
    /// Terrain IDS -> its first variation.
    first_variation: BTreeMap<String, String>,
    /// Biome IDS -> its first terrain.
    first_terrain: BTreeMap<String, String>,
}

fn terrain_art(data: &Path) -> TerrainArt {
    let terrain_root = data.join("Terrain");
    let mut files = Vec::new();
    tgi_files(&terrain_root, &mut files);
    let float = |v: &str| v.trim().parse::<f32>().ok();
    let color = |v: &str| {
        let c: Vec<f32> = v.split(',').filter_map(|p| p.trim().parse().ok()).collect();
        // -1, -1, -1 in the template: the terrain's
        (c.len() == 3 && c[0] >= 0.0).then(|| [c[0], c[1], c[2]])
    };
    let mut out = TerrainArt::default();
    for path in files {
        let Ok(bytes) = fs::read(&path) else { continue };
        let text: String = bytes.iter().map(|&b| b as char).collect();
        let folder = path.parent().and_then(|p| p.strip_prefix(&terrain_root).ok()).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        let (mut base_color, mut base_w, mut base_h) = (None, None, None);
        let mut node = String::new();
        let (mut terrain, mut biome) = (String::new(), String::new());
        let mut current: Option<(String, VariationArt)> = None;
        let finish = |current: &mut Option<(String, VariationArt)>, terrain: &str, first: &mut BTreeMap<String, String>, all: &mut BTreeMap<String, VariationArt>| {
            if let Some((id, v)) = current.take().filter(|(_, v)| !v.texture.is_empty()) {
                first.entry(terrain.to_owned()).or_insert_with(|| id.clone());
                all.insert(id, v);
            }
        };
        for (key, value) in tgi_lines(&text) {
            if key.starts_with('[') {
                finish(&mut current, &terrain, &mut out.first_variation, &mut out.variations);
                if key.starts_with("[variation") {
                    current = Some((String::new(), VariationArt { texture: String::new(), width: base_w, height: base_h, color: base_color }));
                }
                node = key;
                continue;
            }
            match (&mut current, key.as_str()) {
                (Some((id, _)), "ids") => *id = value.to_ascii_lowercase(),
                (Some((_, v)), "texture") => {
                    let name = value.replace('\\', "/");
                    let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
                    v.texture = format!("/assets/converted/textures/Terrain/{folder}/{stem}.png");
                }
                (Some((_, v)), "texture_width") => v.width = float(&value),
                (Some((_, v)), "texture_height") => v.height = float(&value),
                (Some((_, v)), "terrain_color") => v.color = color(&value).or(v.color),
                (None, "ids") if node.starts_with("[biome") => biome = value.to_ascii_lowercase(),
                (None, "ids") if node.starts_with("[terrain") => {
                    terrain = value.to_ascii_lowercase();
                    (base_color, base_w, base_h) = (None, None, None);
                }
                (None, "terrain_ids") => {
                    out.first_terrain.entry(biome.clone()).or_insert_with(|| value.to_ascii_lowercase());
                }
                (None, "texture_width") => base_w = float(&value),
                (None, "texture_height") => base_h = float(&value),
                (None, "terrain_color") => base_color = color(&value),
                _ => {}
            }
        }
        finish(&mut current, &terrain, &mut out.first_variation, &mut out.variations);
    }
    out
}

/// `Data/translate.tgi`, node Variation: old IDS to new IDS, lower case.
fn variation_translations(data: &Path) -> BTreeMap<String, String> {
    let Ok(bytes) = fs::read(data.join("translate.tgi")) else { return BTreeMap::new() };
    let text: String = bytes.iter().map(|&b| b as char).collect();
    let mut out = BTreeMap::new();
    let (mut node, mut old): (String, String) = (String::new(), String::new());
    for (key, value) in tgi_lines(&text) {
        match key.as_str() {
            "node" => node = value.to_ascii_lowercase(),
            "ids_old" => old = value.to_ascii_lowercase(),
            "ids_new" if node == "variation" => {
                out.insert(std::mem::take(&mut old), value.to_ascii_lowercase());
            }
            _ => {}
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct AssetStore {
    root: PathBuf,
    maps: Vec<MapInfo>,
    models: ModelManifest,
    /// A profile's own art, converted the way the stock art is: consulted first.
    /// Shared by every clone of the store, so one battle sets it for all its readers.
    art: std::sync::Arc<std::sync::RwLock<Option<std::sync::Arc<crate::convert::FolderArt>>>>,
}


#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreparedInstance {
    pub source_index: usize,
    pub instance_id: u32,
    pub thing_id: String,
    pub position: [f32; 3],
    pub yaw_radians: f32,
    pub scale: f32,
    pub model_key: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PreparedScene {
    pub map: MapAsset,
    pub terrain: PreparedTerrain,
    pub instances: Vec<PreparedInstance>,
    pub models: BTreeMap<String, PreparedModel>,
    pub warnings: Vec<AssetWarning>,
}

fn io(path: &Path, source: std::io::Error) -> AssetError {
    AssetError::Io {
        path: path.into(),
        source,
    }
}
fn relative_parts(value: &str) -> Result<String> {
    let value = value.replace('\\', "/");
    if value.is_empty()
        || value.starts_with('/')
        || value.contains(':')
        || value.contains('\0')
        || value
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(AssetError::Invalid(format!(
            "unsafe relative asset path: {value:?}"
        )));
    }
    Ok(value)
}

/// Original render models face native -Y at identity (AVars.tgi and 0x8715f0).
/// The recovered `yaw` field is an XY direction angle, not the model rotation.
/// Native (x, y, z) -> render (x, z, -y) preserves this signed rotation about Y.
fn placement_model_yaw(orientation: [Option<f32>; 3]) -> f32 {
    match orientation {
        [Some(x), Some(y), _] if x.is_finite() && y.is_finite() && (x != 0.0 || y != 0.0) => {
            x.atan2(-y)
        }
        _ => 0.0,
    }
}

impl AssetStore {
    /// `root` is the recovered `reconstruction` directory, not the game install.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let path = root.as_ref();
        let root = fs::canonicalize(path).map_err(|e| io(path, e))?;
        let mut store = Self {
            root,
            maps: Vec::new(),
            models: ModelManifest {
                format: String::new(),
                model_count: 0,
                models: Vec::new(),
                by_path: BTreeMap::new(),
            },
            art: Default::default(),
        };
        store.maps = store.read_json("game/data/maps/index.json")?;
        store.models = store.read_json("assets/converted/models/manifest.json")?;
        if store.models.format != "aa-model-manifest-v1"
            || store.models.model_count != store.models.models.len()
        {
            return Err(AssetError::Invalid(
                "model manifest count/format mismatch".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for map in &store.maps {
            relative_parts(&map.id)?;
            if !ids.insert(&map.id) {
                return Err(AssetError::Invalid(format!(
                    "duplicate full map id: {}",
                    map.id
                )));
            }
        }
        store.maps.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(store)
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    /// The art of a battle on a profile's data: `(data folder, where its conversion is
    /// kept)`, `None` leaves the stock art alone.
    pub fn set_art_overlay(&self, folder: Option<(&Path, &Path)>) {
        let overlay = folder.map(|(data, output)| std::sync::Arc::new(crate::convert::FolderArt::new(data, output, Some(&self.root.join("assets/original/Data")))));
        if let Ok(mut art) = self.art.write() {
            *art = overlay;
        }
    }
    fn overlay(&self) -> Option<std::sync::Arc<crate::convert::FolderArt>> {
        self.art.read().ok().and_then(|art| art.clone())
    }
    pub fn maps(&self) -> &[MapInfo] {
        &self.maps
    }
    pub fn list_maps(&self) -> Vec<MapInfo> {
        self.maps.clone()
    }
    pub fn model_infos(&self) -> &[ModelInfo] {
        &self.models.models
    }
    /// Resolves only allowlisted local asset URL namespaces and checks symlinks.
    pub fn resolve_url(&self, url: &str) -> Result<PathBuf> {
        if let Some(path) = url.strip_prefix("/assets/")
            && let Some(overlay) = self.overlay()
        {
            let path = relative_parts(path)?;
            let own = if let Some(rel) = path.strip_prefix("converted/textures/") {
                overlay.texture(rel)
            } else if let Some(rel) = path.strip_prefix("original/Data/") {
                // The folder's own original files: a NIF of a model fix, a PNG a model names.
                overlay.original(rel)
            } else {
                None
            };
            if let Some(file) = own {
                return Ok(file);
            }
        }
        if let Some(path) = url.strip_prefix("/assets/") {
            self.resolve_relative(&format!("assets/{}", relative_parts(path)?))
        } else if let Some(path) = url.strip_prefix("/data/") {
            self.resolve_relative(&format!("game/data/{}", relative_parts(path)?))
        } else {
            Err(AssetError::Invalid(format!(
                "unsupported local asset URL: {url}"
            )))
        }
    }
    fn resolve_relative(&self, relative: &str) -> Result<PathBuf> {
        let path = self.root.join(relative_parts(relative)?);
        let actual = fs::canonicalize(&path).map_err(|e| io(&path, e))?;
        if !actual.starts_with(&self.root) {
            return Err(AssetError::Invalid(format!(
                "asset path escapes root: {}",
                path.display()
            )));
        }
        Ok(actual)
    }
    fn read_path<T: DeserializeOwned>(&self, path: &Path) -> Result<T> {
        let mut file = fs::File::open(path).map_err(|e| io(path, e))?;
        let metadata = file.metadata().map_err(|e| io(path, e))?;
        if !metadata.is_file() {
            return Err(AssetError::Invalid(format!(
                "asset JSON is not a regular file: {}",
                path.display()
            )));
        }
        let length = usize::try_from(metadata.len())
            .map_err(|_| AssetError::Invalid("JSON exceeds process address space".into()))?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|e| AssetError::Invalid(format!("JSON allocation: {e}")))?;
        bytes.resize(length, 0);
        file.read_exact(&mut bytes).map_err(|e| io(path, e))?;
        let mut tail = [0];
        if file.read(&mut tail).map_err(|e| io(path, e))? != 0 {
            return Err(AssetError::Invalid(format!(
                "asset changed length during read: {}",
                path.display()
            )));
        }
        serde_json::from_slice(&bytes).map_err(|source| AssetError::Json {
            path: path.into(),
            source,
        })
    }
    fn read_json<T: DeserializeOwned>(&self, relative: &str) -> Result<T> {
        self.read_path(&self.resolve_relative(relative)?)
    }
    pub fn load_map(&self, id: &str) -> Result<MapAsset> {
        relative_parts(id)?;
        let info = self
            .maps
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| AssetError::UnknownMap(id.into()))?;
        let mut terrain: TerrainMap = self.read_json(&format!("game/data/maps/{id}.json"))?;
        self.fill_palette(&mut terrain);
        let placements: PlacementMap =
            self.read_json(&format!("game/data/map-actors/{id}.json"))?;
        if terrain.id != id
            || placements.id != id
            || terrain.width != info.width
            || terrain.height != info.height
        {
            return Err(AssetError::Invalid(format!(
                "{id}: map identity/dimensions mismatch"
            )));
        }
        terrain.validate()?;
        if placements
            .actors
            .iter()
            .any(|p| p.native_position().iter().any(|v| !v.is_finite()))
        {
            return Err(AssetError::Invalid(format!("{id}: non-finite placement")));
        }
        Ok(MapAsset {
            terrain,
            placements,
        })
    }
    /// A map's palette as the game reads it: the file stores old ids and 0x48D500
    /// finds them through `Data/translate.tgi` (node Variation: the translation
    /// first, its target only, else the id itself). Recovered palettes and a map
    /// file's own list name such ids, which is a third of many a developer map and
    /// most of some. Each entry with no texture becomes the variation it comes to
    /// (`Data/Terrain/**/*.tgi`), id included, so the height rule and the layer
    /// order see the real terrain. A tile whose id comes to nothing keeps the
    /// previous tile's terrain, the walk starting on the biome's first variation
    /// (0x557D4B), as the world does (`aa_play::map`).
    pub fn fill_palette(&self, terrain: &mut TerrainMap) {
        if terrain.palette.values().all(|p| p.texture.is_some()) {
            return;
        }
        let data = self.root.join("assets/original/Data");
        let art = terrain_art(&data);
        let translate = variation_translations(&data);
        let find = |id: &str| -> Option<(String, &VariationArt)> {
            let key = id.to_ascii_lowercase();
            let found = match translate.get(&key) {
                Some(new) => art.variations.contains_key(new).then(|| new.clone()).or_else(|| translate.get(new).filter(|n| art.variations.contains_key(*n)).cloned()),
                None => art.variations.contains_key(&key).then_some(key),
            }?;
            let v = art.variations.get(&found)?;
            self.resolve_url(&v.texture).ok()?;
            Some((found, v))
        };
        let mut unresolved = std::collections::BTreeSet::new();
        for (&index, entry) in terrain.palette.iter_mut().filter(|(_, p)| p.texture.is_none()) {
            let Some((id, v)) = find(&entry.id) else {
                unresolved.insert(index);
                continue;
            };
            entry.id = id;
            entry.texture = Some(v.texture.clone());
            entry.width = v.width;
            entry.height = v.height;
            if let Some(color) = v.color {
                entry.color = color;
            }
        }
        if unresolved.is_empty() || !terrain.tiles.iter().any(|t| unresolved.contains(t)) {
            return;
        }
        // The walk's start, only when the first tiles need it.
        let mut previous = None;
        if terrain.tiles.first().is_some_and(|t| unresolved.contains(t)) {
            let initial = terrain.biomes.first().and_then(|b| art.first_terrain.get(&b.to_ascii_lowercase())).and_then(|t| art.first_variation.get(t));
            if let Some((id, v)) = initial.and_then(|id| find(id)) {
                let index = terrain.palette.keys().next_back().map_or(0, |k| k + 1);
                terrain.palette.insert(index, TerrainPaletteEntry { id, texture: Some(v.texture.clone()), color: v.color.unwrap_or([0.4, 0.5, 0.3]), width: v.width, height: v.height });
                previous = Some(index);
            }
        }
        for tile in &mut terrain.tiles {
            if unresolved.contains(tile) {
                if let Some(p) = previous {
                    *tile = p;
                }
            } else {
                previous = Some(*tile);
            }
        }
    }
    pub fn model_url(&self, resource_path: &str) -> Result<&str> {
        let key = relative_parts(resource_path)?.to_lowercase();
        self.models
            .by_path
            .get(&key)
            .map(String::as_str)
            .ok_or_else(|| AssetError::UnknownModel(resource_path.into()))
    }
    /// The model is one the battle's data folder holds and the stock data does not hold as
    /// it is: its conversion ([`crate::convert`]) already made the audited corrections.
    pub fn model_is_overlaid(&self, resource_path: &str) -> bool {
        self.overlay().is_some_and(|o| o.holds_model(resource_path))
    }
    pub fn load_model(&self, resource_path: &str) -> Result<ModelAsset> {
        // A model the profile's data holds replaces the archive's, as the file does.
        let path = relative_parts(resource_path)?;
        match self.overlay().and_then(|o| o.model(&path)) {
            Some(Ok(file)) => return self.read_path(&file),
            // One that does not convert falls back on the archive's of that name, if any.
            Some(Err(reason)) if self.model_url(resource_path).is_err() => {
                return Err(AssetError::Invalid(format!("{resource_path}: {reason}")));
            }
            _ => {}
        }
        let url = self.model_url(resource_path)?;
        self.read_path(&self.resolve_url(url)?)
    }
    pub fn load_scene(&self, id: &str, mode: HeightMode) -> Result<PreparedScene> {
        let map = self.load_map(id)?;
        let terrain = map.terrain.prepare(mode)?;
        let mut warnings = terrain.warnings.clone();
        warnings.extend(
            map.placements
                .limitations
                .iter()
                .map(|message| AssetWarning {
                    source: map.placements.source.clone(),
                    message: message.clone(),
                }),
        );
        for e in &map.placements.errors {
            warnings.push(AssetWarning {
                source: map.placements.source.clone(),
                message: format!("Undecoded placement at {}: {}", e.record_offset, e.error),
            });
        }
        let mut instances = Vec::with_capacity(map.placements.actors.len());
        let mut keys = BTreeSet::new();
        for (source_index, p) in map.placements.actors.iter().enumerate() {
            let source = format!("{}:instance {}", map.placements.source, p.instance_id);
            warnings.extend(p.warnings.iter().map(|message| AssetWarning {
                source: source.clone(),
                message: message.clone(),
            }));
            let definition = map.placements.types.get(&p.thing_id);
            let references = definition.map(|d| d.models.as_slice()).unwrap_or(&[]);
            let model_key = if references.is_empty() {
                None
            } else {
                let requested = p.variant_index.and_then(|i| usize::try_from(i).ok());
                let selected = requested.filter(|&i| i < references.len()).unwrap_or(0);
                if requested != Some(selected) && references.len() > 1 {
                    warnings.push(AssetWarning{source:source.clone(),message:"Unresolved model variant uses the first authored model deterministically.".into()});
                }
                let key = relative_parts(&references[selected].path)?.to_lowercase();
                keys.insert(key.clone());
                Some(key)
            };
            let scale = p.scale.filter(|v| v.is_finite() && *v > 0.0).unwrap_or(1.0);
            if p.scale.is_some_and(|v| !v.is_finite() || v <= 0.0) {
                warnings.push(AssetWarning {
                    source: source.clone(),
                    message: "Invalid scale replaced with 1 for inspection; raw value retained."
                        .into(),
                });
            }
            instances.push(PreparedInstance {
                source_index,
                instance_id: p.instance_id,
                thing_id: p.thing_id.clone(),
                position: p.render_position(),
                // Retain the extractor's unresolved/non-normalized facing guard;
                // its numeric yaw itself used the wrong model forward axis.
                yaw_radians: p
                    .yaw
                    .filter(|v| v.is_finite())
                    .map(|_| placement_model_yaw(p.orientation))
                    .unwrap_or(0.0),
                scale,
                model_key,
            });
        }
        // BTreeSet -> indexed Vec -> collect retains deterministic result/error ordering.
        let keys: Vec<_> = keys.into_iter().collect();
        let loaded: Vec<_> = keys
            .par_iter()
            .map(|key| (key.clone(), self.load_model(key).and_then(|m| m.prepare())))
            .collect();
        let mut models = BTreeMap::new();
        for (key, result) in loaded {
            match result {
                Ok(model) => {
                    models.insert(key, model);
                }
                Err(e) => warnings.push(AssetWarning {
                    source: key,
                    message: e.to_string(),
                }),
            }
        }
        for instance in &mut instances {
            if instance
                .model_key
                .as_ref()
                .is_some_and(|key| !models.contains_key(key))
            {
                instance.model_key = None;
            }
        }
        warnings.push(AssetWarning{source:map.placements.source.clone(),message:"Planar placement facing uses the verified native -Y model forward axis. Pitch/roll and live orientation interpolation remain unimplemented. Render positions use native (x,z,-y); model vertices are already in that basis.".into()});
        Ok(PreparedScene {
            map,
            terrain,
            instances,
            models,
            warnings,
        })
    }
}

#[cfg(test)]
mod placement_tests {
    use super::placement_model_yaw;

    #[test]
    fn planar_facing_matches_original_renderer_matrices() {
        let report: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/phase5-visual-audit/native-placement-basis.json"
        ))
        .unwrap();
        let cases = report["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 144);
        for case in cases {
            let direction = case["heading"].as_array().unwrap();
            let yaw = placement_model_yaw(std::array::from_fn(|i| {
                Some(direction[i].as_f64().unwrap() as f32)
            }));
            let (sin, cos) = yaw.sin_cos();
            let computed = [cos, -sin, 0.0, sin, cos, 0.0, 0.0, 0.0, 1.0];
            for (actual, bits) in computed
                .into_iter()
                .zip(case["native_matrix_bits"].as_array().unwrap())
            {
                let native = f32::from_bits(bits.as_u64().unwrap() as u32);
                assert!((actual - native).abs() < 0.000001, "{case}");
            }
        }
    }

    #[test]
    fn unresolved_facing_is_stable_and_native_default_is_identity() {
        assert_eq!(placement_model_yaw([Some(0.0), Some(-1.0), Some(0.0)]), 0.0);
        for heading in [
            [None, Some(-1.0), Some(0.0)],
            [Some(f32::NAN), Some(-1.0), Some(0.0)],
            [Some(0.0), Some(0.0), Some(0.0)],
        ] {
            assert_eq!(placement_model_yaw(heading), 0.0);
        }
    }
}
