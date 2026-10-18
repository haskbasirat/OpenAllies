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

#[derive(Debug, Clone)]
pub struct AssetStore {
    root: PathBuf,
    maps: Vec<MapInfo>,
    models: ModelManifest,
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
        let terrain: TerrainMap = self.read_json(&format!("game/data/maps/{id}.json"))?;
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
    pub fn model_url(&self, resource_path: &str) -> Result<&str> {
        let key = relative_parts(resource_path)?.to_lowercase();
        self.models
            .by_path
            .get(&key)
            .map(String::as_str)
            .ok_or_else(|| AssetError::UnknownModel(resource_path.into()))
    }
    pub fn load_model(&self, resource_path: &str) -> Result<ModelAsset> {
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
