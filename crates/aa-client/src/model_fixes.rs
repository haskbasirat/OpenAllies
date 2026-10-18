//! Audited original-NIF material corrections for the Rust preview's static models.
//! This changes only prepared in-memory geometry; original/recovered files remain read-only.

use aa_assets::{AssetStore, AssetWarning, Bounds, ModelAsset, PreparedMesh, PreparedModel};
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const SIDECAR: &str = "assets-cache/model-materials.json";

#[derive(Deserialize)]
struct Document {
    schema: u32,
    complete: bool,
    model_count: usize,
    audited_models: usize,
    manifest_sha256: String,
    #[serde(deserialize_with = "unique_models")]
    models: BTreeMap<String, Record>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn selected_resource_digest_and_same_read_output_are_required(){
        let fixture=Fixture::new();let fixes=fixture.fixes();
        let digest=sha256(b"Synthetic source-identity fixture; not a parsed native NIF");
        let selected=fixes.load_selected_model(&fixture.store,"Test/fixture.nif",&digest).unwrap();
        assert!(fixes.load_selected_model(&fixture.store,"Test/fixture.nif",&"00".repeat(32)).is_err());
        assert!(fixes.load_selected_model(&fixture.store,"Test/unknown.nif",&digest).is_err());
        let converted=fixture.root.join("assets/converted/models/Test/fixture.json");
        let mut changed=fs::read(&converted).unwrap();changed.push(b' ');fs::write(&converted,changed).unwrap();
        //Already validated bytes remain the exact in-memory payload. Applying
        //the paired correction must not reread a changed converted file.
        let mut prepared=selected.prepare().unwrap();fixes.apply(&fixture.store,&mut prepared).unwrap();
        assert!(fixes.load_selected_model(&fixture.store,"Test/fixture.nif",&digest).unwrap_err().to_string().contains("JSON hash mismatch"));
    }

    struct Fixture {
        root: PathBuf,
        files: Vec<PathBuf>,
        directories: Vec<PathBuf>,
        store: AssetStore,
        path: PathBuf,
        document: Value,
        prepared: PreparedModel,
    }

    impl Fixture {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "aa-model-fixes-{}-{nonce}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            let mut directories = vec![root.clone()];
            for relative in [
                "game",
                "game/data",
                "game/data/maps",
                "assets",
                "assets/original",
                "assets/original/Data",
                "assets/original/Data/Test",
                "assets/converted",
                "assets/converted/models",
                "assets/converted/models/Test",
            ] {
                let directory = root.join(relative);
                fs::create_dir(&directory).unwrap();
                directories.push(directory);
            }
            let source = "Test/fixture.nif";
            let original = b"Synthetic source-identity fixture; not a parsed native NIF";
            let material = aa_assets::Material::default();
            let mesh = |name: &str, positions: Vec<f32>| {
                json!({
                    "name":name,"hierarchy":["Root",name],"positions":positions,
                    "normals":[0,0,1,0,0,1,0,0,1],"uvs":[0,0,1,0,0,1],
                    "indices":[0,1,2],"material":material,"skinnedPose":false
                })
            };
            let model_json = json!({
                "format":"aa-nif-mesh-v1","source":source,"nifVersion":"4.0.0.2",
                "upAxis":"Y","pose":"authored","bounds":{"min":[-10,-5,-10],"max":[10,1,10],"size":[20,6,20]},
                "meshes":[mesh("body",vec![0.,0.,0.,1.,0.,0.,0.,1.,0.]),
                    mesh("helper",vec![-10.,-5.,-10.,10.,-5.,-10.,0.,-5.,10.])]
            });
            let converted = serde_json::to_vec(&model_json).unwrap();
            let model: ModelAsset = serde_json::from_slice(&converted).unwrap();
            let prepared = model.prepare().unwrap();
            let metadata = json!({"format":"aa-model-manifest-v1","modelCount":1,"models":[{
                "source":source,"url":"/assets/converted/models/Test/fixture.json","name":"fixture",
                "meshCount":2,"vertices":6,"triangles":2,"bounds":prepared.bounds
            }],"byPath":{"test/fixture.nif":"/assets/converted/models/Test/fixture.json"}});
            let manifest_bytes = serde_json::to_vec(&metadata).unwrap();
            let mut files = Vec::new();
            for (relative, bytes) in [
                ("game/data/maps/index.json", b"[]".to_vec()),
                (
                    "assets/converted/models/manifest.json",
                    manifest_bytes.clone(),
                ),
                (
                    "assets/converted/models/Test/fixture.json",
                    converted.clone(),
                ),
                ("assets/original/Data/Test/fixture.nif", original.to_vec()),
            ] {
                let path = root.join(relative);
                fs::write(&path, bytes).unwrap();
                files.push(path);
            }
            let evidence = |tag: &str, name: &str| json!({"path":["Root",name],"tag":tag,"extra_name":"","value":""});
            let document = json!({"schema":1,"complete":true,"model_count":1,"audited_models":1,"manifest_sha256":sha256(&manifest_bytes),"models":{source:{
                "source_sha256":sha256(original),"converted_sha256":sha256(&converted),"mesh_count":2,
                "corrections":[
                    {"mesh_index":0,"name":"body","hierarchy":["Root","body"],"source_node_ordinal":4,
                        "vertex_count":3,"triangle_count":1,"skip":false,"skip_reason":null,
                        "team_color_mask":true,"tag_evidence":[evidence("RepColor","body")]},
                    {"mesh_index":1,"name":"helper","hierarchy":["Root","helper"],"source_node_ordinal":8,
                        "vertex_count":3,"triangle_count":1,"skip":true,"skip_reason":"native_shadow_tag",
                        "team_color_mask":false,"tag_evidence":[evidence("Sg","helper")]}
                ]
            }}});
            let path = root.join("model-materials.json");
            fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
            files.push(path.clone());
            let store = AssetStore::open(&root).unwrap();
            Self {
                root,
                files,
                directories,
                store,
                path,
                document,
                prepared,
            }
        }
        fn write_document(&self) {
            fs::write(&self.path, serde_json::to_vec(&self.document).unwrap()).unwrap();
        }
        fn fixes(&self) -> ModelFixes {
            ModelFixes::load_path(&self.store, self.path.clone()).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // Remove only this fixture's known files/directories, never recurse
            // into caller data or follow an enumerated filesystem path.
            assert!(self.files.iter().all(|path| path.starts_with(&self.root)));
            assert!(
                self.directories
                    .iter()
                    .all(|path| path.starts_with(&self.root))
            );
            for path in &self.files {
                let _ = fs::remove_file(path);
            }
            for path in self.directories.iter().rev() {
                let _ = fs::remove_dir(path);
            }
        }
    }

    #[test]
    fn audited_corrections_keep_geometry_and_recompute_bounds_after_helper_removal() {
        let fixture = Fixture::new();
        let fixes = fixture.fixes();
        let mut prepared = fixture.prepared.clone();
        let body = prepared.meshes[0].clone();
        let stats = fixes.apply(&fixture.store, &mut prepared).unwrap();
        assert_eq!(
            stats,
            FixStats {
                meshes_before: 2,
                meshes_after: 1,
                shadow_helpers_removed: 1,
                team_color_flags_changed: 1,
                team_color_meshes: 1,
                empty_after_fixes: false
            }
        );
        assert_eq!(prepared.meshes[0].vertices, body.vertices);
        assert_eq!(prepared.meshes[0].indices, body.indices);
        assert!(prepared.meshes[0].material.team_color_mask);
        assert_eq!(prepared.bounds.min, [0.0, 0.0, 0.0]);
        assert_eq!(prepared.bounds.max, [1.0, 1.0, 0.0]);
        assert_eq!(prepared.bounds.size, [1.0, 1.0, 0.0]);
        assert!(
            fixes.entries["test/fixture.nif"]
                .verified
                .get()
                .unwrap()
                .is_ok()
        );
        let mut fresh = fixture.prepared.clone();
        assert_eq!(fixes.apply(&fixture.store, &mut fresh).unwrap(), stats);
        assert_eq!(prepared, fresh);
        assert!(
            fixes.apply(&fixture.store, &mut prepared).is_err(),
            "applying twice must not guess changed mesh indices"
        );
    }

    #[test]
    fn source_bound_render_properties_survive_application_and_reject_invalid_codes() {
        let mut fixture = Fixture::new();
        let state = aa_assets::NativeRenderState {
            alpha_flags: 4845,
            alpha_threshold: 150,
            zbuffer_flags: 3,
            zbuffer_function: 3,
        };
        fixture.document["models"]["Test/fixture.nif"]["corrections"][0]["render_state"] =
            serde_json::to_value(state).unwrap();
        fixture.write_document();
        let mut prepared = fixture.prepared.clone();
        fixture
            .fixes()
            .apply(&fixture.store, &mut prepared)
            .unwrap();
        assert_eq!(prepared.meshes[0].material.native_render_state, Some(state));
        assert_eq!(
            prepared.meshes[0].vertices,
            fixture.prepared.meshes[0].vertices
        );
        fixture.document["models"]["Test/fixture.nif"]["corrections"][0]["render_state"]["alpha_flags"] =
            json!(1 | (15 << 1));
        fixture.write_document();
        assert!(ModelFixes::load_path(&fixture.store, fixture.path.clone()).is_err());
    }

    #[test]
    fn supplied_alpha_metadata_preserves_audited_render_state() {
        let fixture = Fixture::new();
        let state = aa_assets::NativeRenderState {
            alpha_flags: 4845,
            alpha_threshold: 150,
            zbuffer_flags: 3,
            zbuffer_function: 3,
        };
        let mut document = fixture.document.clone();
        document["models"]["Test/fixture.nif"]["corrections"][0]["render_state"] =
            serde_json::to_value(state).unwrap();
        fs::write(&fixture.path, serde_json::to_vec(&document).unwrap()).unwrap();
        let mut prepared = fixture.prepared.clone();
        let authored = aa_assets::AlphaPropertyState {
            flags: state.alpha_flags,
            threshold: state.alpha_threshold,
        };
        prepared.meshes[0].material.alpha_property_state = Some(authored);
        fixture
            .fixes()
            .apply(&fixture.store, &mut prepared)
            .unwrap();
        assert_eq!(
            prepared.meshes[0].material.alpha_property_state,
            Some(authored)
        );
        assert_eq!(prepared.meshes[0].material.native_render_state, Some(state));
        assert!(!prepared.meshes[0].material.alpha_blend);
    }

    #[test]
    fn supplied_alpha_conflicts_and_missing_audit_are_rejected_atomically() {
        for variant in 0..4 {
            let mut fixture = Fixture::new();
            let state = aa_assets::NativeRenderState {
                alpha_flags: 4845,
                alpha_threshold: 150,
                zbuffer_flags: 3,
                zbuffer_function: 3,
            };
            for row in fixture.document["models"]["Test/fixture.nif"]["corrections"]
                .as_array_mut()
                .unwrap()
            {
                row["render_state"] = serde_json::to_value(state).unwrap();
            }
            let mut prepared = fixture.prepared.clone();
            let authored = aa_assets::AlphaPropertyState {
                flags: state.alpha_flags,
                threshold: state.alpha_threshold,
            };
            prepared.meshes[0].material.alpha_property_state = Some(authored);
            match variant {
                0 => {
                    prepared.meshes[0]
                        .material
                        .alpha_property_state
                        .as_mut()
                        .unwrap()
                        .flags ^= 1
                }
                1 => {
                    prepared.meshes[0]
                        .material
                        .alpha_property_state
                        .as_mut()
                        .unwrap()
                        .threshold ^= 1
                }
                2 => {
                    fixture.document["models"]["Test/fixture.nif"]["corrections"][0]["render_state"] =
                        Value::Null
                }
                _ => {
                    prepared.meshes[1].material.alpha_property_state =
                        Some(aa_assets::AlphaPropertyState {
                            flags: authored.flags ^ 1,
                            ..authored
                        })
                }
            }
            fixture.write_document();
            let before = prepared.clone();
            let error = fixture
                .fixes()
                .apply(&fixture.store, &mut prepared)
                .unwrap_err();
            assert!(format!("{error:#}").contains("alpha state"));
            assert_eq!(prepared, before, "variant {variant}");
        }
    }

    #[test]
    fn either_source_hash_mismatch_rejects_without_mutating_prepared_model() {
        for field in ["source_sha256", "converted_sha256"] {
            let mut fixture = Fixture::new();
            fixture.document["models"]["Test/fixture.nif"][field] = Value::String("00".repeat(32));
            fixture.write_document();
            let fixes = fixture.fixes();
            let mut prepared = fixture.prepared.clone();
            let error = fixes
                .apply(&fixture.store, &mut prepared)
                .unwrap_err()
                .to_string();
            assert!(error.contains("hash mismatch"), "{error}");
            assert_eq!(prepared, fixture.prepared);
        }
    }

    #[test]
    fn changed_count_hierarchy_name_and_geometry_are_rejected_atomically() {
        let fixture = Fixture::new();
        let fixes = fixture.fixes();
        for variant in 0..5 {
            let mut prepared = fixture.prepared.clone();
            match variant {
                0 => {
                    prepared.meshes.pop();
                }
                1 => prepared.meshes[1].hierarchy[0] = "OtherRoot".into(),
                2 => prepared.meshes[1].name = "OtherMesh".into(),
                3 => prepared.meshes[1].indices.swap(0, 1),
                _ => prepared.meshes[1].vertices[0].position[0] += 1.0,
            }
            let before = prepared.clone();
            assert!(fixes.apply(&fixture.store, &mut prepared).is_err());
            assert_eq!(prepared, before);
        }
    }

    #[test]
    fn incomplete_coverage_bad_indices_and_unaudited_exclusions_fail_closed() {
        let missing = Fixture::new();
        assert!(ModelFixes::load_path(&missing.store, missing.root.join("absent.json")).is_err());
        for variant in 0..7 {
            let mut fixture = Fixture::new();
            match variant {
                0 => fixture.document["complete"] = json!(false),
                1 => fixture.document["models"]["Test/fixture.nif"]["mesh_count"] = json!(3),
                2 => {
                    fixture.document["models"]["Test/fixture.nif"]["corrections"][1]["mesh_index"] =
                        json!(0)
                }
                3 => {
                    fixture.document["models"]["Test/fixture.nif"]["corrections"][1]["skip_reason"] =
                        json!("name_contains_shadow")
                }
                4 => {
                    fixture.document["models"]["Test/fixture.nif"]["corrections"][1]["tag_evidence"] =
                        json!([])
                }
                5 => fixture.document["audited_models"] = json!(0),
                _ => {
                    fixture.document["models"]["Test/fixture.nif"]["corrections"][1]["tag_evidence"]
                        [0]["tag"] = json!("RepColor")
                }
            }
            fixture.write_document();
            assert!(
                ModelFixes::load_path(&fixture.store, fixture.path.clone()).is_err(),
                "variant {variant}"
            );
        }
    }

    #[test]
    fn fully_audited_helper_only_model_has_explicit_empty_bounds() {
        let mut fixture = Fixture::new();
        let body = &mut fixture.document["models"]["Test/fixture.nif"]["corrections"][0];
        body["skip"] = json!(true);
        body["skip_reason"] = json!("native_shadow_tag");
        body["tag_evidence"]
            .as_array_mut()
            .unwrap()
            .push(json!({"path":["Root","body"],"tag":"Sg","extra_name":"Sg","value":""}));
        fixture.write_document();
        let mut prepared = fixture.prepared.clone();
        let stats = fixture
            .fixes()
            .apply(&fixture.store, &mut prepared)
            .unwrap();
        assert!(stats.empty_after_fixes);
        assert_eq!(stats.shadow_helpers_removed, 2);
        assert!(prepared.meshes.is_empty());
        assert_eq!(prepared.bounds.size, [0.0; 3]);
    }

    #[test]
    fn native_sg2_shadow_subtype_is_supported_by_tag_not_mesh_name() {
        let mut fixture = Fixture::new();
        fixture.document["models"]["Test/fixture.nif"]["corrections"][1]["tag_evidence"][0]["tag"] =
            json!("Sg2");
        fixture.write_document();
        let mut prepared = fixture.prepared.clone();
        assert_eq!(
            fixture
                .fixes()
                .apply(&fixture.store, &mut prepared)
                .unwrap()
                .shadow_helpers_removed,
            1
        );
        assert_eq!(prepared.meshes[0].name, "body");
    }

    #[test]
    fn traversal_and_duplicate_source_keys_are_rejected() {
        for source in [
            "../model.nif",
            "C:/model.nif",
            "/model.nif",
            "a//model.nif",
            "a/./model.nif",
            "file.json",
        ] {
            assert!(resource_key(source).is_err(), "{source}");
        }
        let fixture = Fixture::new();
        let record =
            serde_json::to_string(&fixture.document["models"]["Test/fixture.nif"]).unwrap();
        let duplicate = format!(
            r#"{{"schema":1,"complete":true,"model_count":1,"audited_models":1,"manifest_sha256":"{}","models":{{"Test/fixture.nif":{record},"Test/fixture.nif":{record}}}}}"#,
            fixture.document["manifest_sha256"].as_str().unwrap()
        );
        assert!(serde_json::from_str::<Document>(&duplicate).is_err());
    }

    #[test]
    #[ignore = "requires recovered original assets and the completed audited material sidecar"]
    fn actual_three_original_models_pass_hashes_and_exact_correction_mapping() {
        let game = std::env::var_os("AA_GAME_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .ancestors()
                    .nth(3)
                    .unwrap()
                    .to_owned()
            });
        let store = AssetStore::open(game.join("reconstruction")).unwrap();
        let sidecar = game.join("rust-rebuild").join(SIDECAR);
        let fixes = ModelFixes::load_path(&store, sidecar).unwrap();
        // Exact original assets behind the reported building texture/helper
        // issues. In particular, Sg01 is a name, not sufficient shadow evidence.
        for (source, skipped, changed) in [
            ("Buildings/GBR/GBRCorps/GBRCorps.nif", 0, 0),
            ("Buildings/GBR/GBRCorps/GBRCorpsPack.nif", 15, 10),
            ("Buildings/GBR/GBRDepotOil/GBRDepotOil.nif", 0, 0),
        ] {
            let entry = &fixes.entries[&resource_key(source).unwrap()];
            let original_path=store.resolve_url(&format!("/assets/original/Data/{}",entry.source)).unwrap();
            let selected_digest=hash_file(&original_path).unwrap();
            let mut model = fixes.load_selected_model(&store,&entry.source,&selected_digest).unwrap().prepare().unwrap();
            let original = model.clone();
            let stats = fixes.apply(&store, &mut model).unwrap();
            assert_eq!(stats.shadow_helpers_removed, skipped, "{source}");
            assert_eq!(stats.team_color_flags_changed, changed, "{source}");
            if source.ends_with("GBRDepotOil.nif") {
                assert!(
                    model
                        .meshes
                        .iter()
                        .any(|mesh| mesh.name == "Sg01" && mesh.material.team_color_mask)
                );
            }
            let expected: Vec<_> = entry
                .record
                .corrections
                .iter()
                .filter(|row| !row.skip)
                .collect();
            assert_eq!(model.meshes.len(), expected.len());
            assert_eq!(
                stats.shadow_helpers_removed,
                entry
                    .record
                    .corrections
                    .iter()
                    .filter(|row| row.skip)
                    .count()
            );
            for (mesh, row) in model.meshes.iter().zip(expected) {
                assert_eq!(mesh.hierarchy, row.hierarchy);
                assert_eq!(mesh.material.team_color_mask, row.team_color_mask);
                assert_eq!(mesh.vertices, original.meshes[row.mesh_index].vertices);
                assert_eq!(mesh.indices, original.meshes[row.mesh_index].indices);
            }
        }
    }
}

fn unique_models<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, Record>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = BTreeMap<String, Record>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a model map with unique source keys")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut input: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut models = BTreeMap::new();
            while let Some((key, value)) = input.next_entry::<String, Record>()? {
                if models.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate model source key"));
                }
            }
            Ok(models)
        }
    }
    deserializer.deserialize_map(Visitor)
}

#[derive(Deserialize)]
struct Record {
    source_sha256: String,
    converted_sha256: String,
    mesh_count: usize,
    corrections: Vec<Correction>,
}

#[derive(Deserialize)]
struct Correction {
    mesh_index: usize,
    name: String,
    hierarchy: Vec<String>,
    source_node_ordinal: usize,
    vertex_count: usize,
    triangle_count: usize,
    skip: bool,
    skip_reason: Option<String>,
    team_color_mask: bool,
    tag_evidence: Vec<TagEvidence>,
    #[serde(default)]
    render_state: Option<aa_assets::NativeRenderState>,
    #[serde(default)]
    sort_center: Option<[f32; 3]>,
}

#[derive(Deserialize)]
struct TagEvidence {
    path: Vec<String>,
    tag: String,
    #[serde(rename = "extra_name")]
    _extra_name: String,
    #[serde(rename = "value")]
    _value: String,
}

struct Entry {
    source: String,
    record: Record,
    // Verification is once per model in this loader instance, not per frame.
    // Repeated calls still verify the supplied prepared geometry against identity.
    verified: OnceLock<std::result::Result<VerifiedModel, String>>,
}

struct VerifiedModel {
    pose: String,
    geometry: Vec<[u8; 32]>,
}

pub struct ModelFixes {
    root: PathBuf,
    path: PathBuf,
    metadata_sha256: String,
    entries: BTreeMap<String, Entry>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FixStats {
    pub meshes_before: usize,
    pub meshes_after: usize,
    pub shadow_helpers_removed: usize,
    pub team_color_flags_changed: usize,
    pub team_color_meshes: usize,
    pub empty_after_fixes: bool,
}

fn resource_key(source: &str) -> Result<String> {
    let path = source.replace('\\', "/");
    ensure!(
        !path.is_empty()
            && !path.starts_with('/')
            && !path.contains(':')
            && !path.chars().any(char::is_control)
            && !path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == ".."),
        "Unsafe model correction source path: {source:?}"
    );
    ensure!(
        path.to_ascii_lowercase().ends_with(".nif"),
        "Expected original NIF source: {source}"
    );
    Ok(path.to_ascii_lowercase())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit())
}

fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("Cannot read {}", path.display()))?;
    ensure!(
        file.metadata()?.is_file(),
        "Not a regular asset file: {}",
        path.display()
    );
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn raw_geometry(model: &ModelAsset) -> Vec<[u8; 32]> {
    model
        .meshes
        .iter()
        .map(|mesh| {
            let mut hash = Sha256::new();
            hash.update((mesh.positions.len() as u64).to_le_bytes());
            for value in &mesh.positions {
                hash.update(value.to_bits().to_le_bytes());
            }
            hash.update((mesh.indices.len() as u64).to_le_bytes());
            for value in &mesh.indices {
                hash.update(value.to_le_bytes());
            }
            hash.finalize().into()
        })
        .collect()
}

fn prepared_geometry(mesh: &PreparedMesh) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update((mesh.vertices.len() as u64 * 3).to_le_bytes());
    for vertex in &mesh.vertices {
        for value in vertex.position {
            hash.update(value.to_bits().to_le_bytes());
        }
    }
    hash.update((mesh.indices.len() as u64).to_le_bytes());
    for value in &mesh.indices {
        hash.update(value.to_le_bytes());
    }
    hash.finalize().into()
}

impl ModelFixes {
    fn discover_path(store: &AssetStore) -> Result<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(directory) = std::env::current_exe()?.parent() {
            candidates.extend(directory.ancestors().map(|ancestor| ancestor.join(SIDECAR)));
        }
        if let Some(game) = store.root().parent() {
            candidates.push(game.join("rust-rebuild").join(SIDECAR));
        }
        for candidate in candidates {
            if candidate.try_exists()? {
                ensure!(
                    candidate.is_file(),
                    "Material correction sidecar is not a file: {}",
                    candidate.display()
                );
                return candidate
                    .canonicalize()
                    .with_context(|| format!("Resolve {}", candidate.display()));
            }
        }
        bail!(
            "Required audited model-materials.json is missing. Install this build's assets-cache beside the executable or in rust-rebuild/assets-cache; rendering without these corrections is disabled."
        )
    }

    /// Source size for the caller's existing memory preflight, before JSON parsing.
    pub fn metadata_bytes(store: &AssetStore) -> Result<u64> {
        Ok(fs::metadata(Self::discover_path(store)?)?.len())
    }

    pub fn discover(store: &AssetStore) -> Result<Self> {
        Self::load_path(store, Self::discover_path(store)?)
    }

    fn load_path(store: &AssetStore, path: PathBuf) -> Result<Self> {
        let bytes = fs::read(&path)
            .with_context(|| format!("Read model corrections {}", path.display()))?;
        let document: Document = serde_json::from_slice(&bytes)
            .with_context(|| format!("Parse model corrections {}", path.display()))?;
        ensure!(
            document.schema == 1 && document.complete,
            "Model material audit is incomplete or has an unsupported schema"
        );
        ensure!(
            valid_digest(&document.manifest_sha256)
                && hash_file(&store.resolve_url("/assets/converted/models/manifest.json")?)?
                    .eq_ignore_ascii_case(&document.manifest_sha256),
            "Converted model catalog hash mismatch"
        );
        ensure!(
            document.model_count == store.model_infos().len()
                && document.audited_models == document.model_count
                && document.models.len() == document.model_count,
            "Model correction/catalog coverage counts do not match"
        );
        let mut entries = BTreeMap::new();
        for (source, record) in document.models {
            let key = resource_key(&source)?;
            ensure!(
                valid_digest(&record.source_sha256) && valid_digest(&record.converted_sha256),
                "Invalid SHA-256 for {source}"
            );
            ensure!(
                record.mesh_count == record.corrections.len(),
                "Incomplete per-mesh corrections for {source}"
            );
            for (index, row) in record.corrections.iter().enumerate() {
                ensure!(
                    row.sort_center
                        .is_none_or(|center| center.iter().all(|value| value.is_finite())),
                    "Invalid source sphere center for {source} mesh {index}"
                );
                if let Some(state) = row.render_state {
                    ensure!(
                        (state.alpha_flags >> 1) & 15 <= 10
                            && (state.alpha_flags >> 5) & 15 <= 10
                            && state.zbuffer_function <= 7,
                        "Unsupported native render property for {source} mesh {index}"
                    );
                }
                ensure!(
                    row.mesh_index == index,
                    "Correction index/order mismatch for {source} mesh {index}"
                );
                ensure!(
                    !row.hierarchy.is_empty(),
                    "Missing audited hierarchy for {source} mesh {index}"
                );
                ensure!(
                    if row.skip {
                        row.skip_reason.as_deref() == Some("native_shadow_tag")
                    } else {
                        row.skip_reason.is_none()
                    },
                    "Unsupported helper exclusion reason for {source} mesh {index}"
                );
                ensure!(
                    !(row.skip || row.team_color_mask) || !row.tag_evidence.is_empty(),
                    "Missing native tag evidence for {source} mesh {index}"
                );
                ensure!(
                    // Native ProcessObject879FDB..87A00E marks both Sg and
                    // Sg2 as shadow geometry (different shadow subtypes).
                    !row.skip
                        || row
                            .tag_evidence
                            .iter()
                            .any(|tag| matches!(tag.tag.as_str(), "Sg" | "Sg2")),
                    "Shadow exclusion lacks audited Sg/Sg2 tag for {source} mesh {index}"
                );
                ensure!(
                    !row.team_color_mask
                        || row.tag_evidence.iter().any(|tag| tag.tag == "RepColor"),
                    "Team-color correction lacks audited RepColor tag for {source} mesh {index}"
                );
                for tag in &row.tag_evidence {
                    // Named native integer extras can legitimately have an
                    // empty string name/value; preserve those provenance fields.
                    ensure!(
                        !tag.path.is_empty() && !tag.tag.is_empty(),
                        "Incomplete native tag evidence for {source} mesh {index}"
                    );
                }
            }
            ensure!(
                entries
                    .insert(
                        key,
                        Entry {
                            source,
                            record,
                            verified: OnceLock::new()
                        }
                    )
                    .is_none(),
                "Case/separator-aliased duplicate model correction"
            );
        }
        for info in store.model_infos() {
            let key = resource_key(&info.source)?;
            let entry = entries
                .get(&key)
                .with_context(|| format!("Missing audited model {}", info.source))?;
            ensure!(
                entry.record.mesh_count == info.mesh_count,
                "Catalog mesh count mismatch for {}",
                info.source
            );
        }
        Ok(Self {
            root: store.root().to_owned(),
            path,
            metadata_sha256: sha256(&bytes),
            entries,
        })
    }

    pub fn metadata_path(&self) -> &Path {
        &self.path
    }
    pub fn metadata_sha256(&self) -> &str {
        &self.metadata_sha256
    }

    fn verify_entry(store: &AssetStore, entry: &Entry) -> Result<VerifiedModel> {
        Self::read_verified_entry(store,entry).map(|(_,verified)|verified)
    }
    ///Read and parse the converted allocation once. Both old correction
    ///validation and the selected native Resource path use this same parser.
    fn read_verified_entry(store:&AssetStore,entry:&Entry)->Result<(ModelAsset,VerifiedModel)>{
        let source = entry.source.replace('\\', "/");
        let original = store.resolve_url(&format!("/assets/original/Data/{source}"))?;
        let converted_source = format!("{}.json", &source[..source.len() - 4]);
        let expected_url = format!("/assets/converted/models/{converted_source}");
        ensure!(
            store.model_url(&source)?.replace('\\', "/") == expected_url,
            "Converted model mapping changed for {source}"
        );
        ensure!(
            hash_file(&original)?.eq_ignore_ascii_case(&entry.record.source_sha256),
            "Original NIF hash mismatch for {source}"
        );
        let converted = store.resolve_url(&expected_url)?;
        let bytes = fs::read(&converted)?;
        ensure!(
            sha256(&bytes).eq_ignore_ascii_case(&entry.record.converted_sha256),
            "Converted JSON hash mismatch for {source}"
        );
        let model: ModelAsset = serde_json::from_slice(&bytes)?;
        ensure!(
            model.format == "aa-nif-mesh-v1"
                && model.up_axis == "Y"
                && resource_key(&model.source)? == resource_key(&source)?,
            "Converted model identity/format mismatch for {source}"
        );
        ensure!(
            model.meshes.len() == entry.record.mesh_count,
            "Converted mesh count mismatch for {source}"
        );
        for (index, (mesh, row)) in model
            .meshes
            .iter()
            .zip(&entry.record.corrections)
            .enumerate()
        {
            ensure!(
                mesh.name == row.name
                    && mesh.hierarchy == row.hierarchy
                    && mesh.positions.len().is_multiple_of(3)
                    && mesh.positions.len() / 3 == row.vertex_count
                    && mesh.indices.len().is_multiple_of(3)
                    && mesh.indices.len() / 3 == row.triangle_count,
                "Audited mesh identity mismatch for {source} mesh {index} (NIF node {})",
                row.source_node_ordinal
            );
        }
        let verified=VerifiedModel {
            pose: model.pose.clone(),
            geometry: raw_geometry(&model),
        };Ok((model,verified))
    }

    ///The expected input hash comes from the SAME retained Resource's real
    ///selected bytes. Existing paired input/output audit records correlate
    ///those bytes with this exact converted payload and source-node mapping.
    ///No new hash is assigned to an old mesh by matching a filename.
    pub(crate) fn load_selected_model(&self,store:&AssetStore,source:&str,input_sha256:&str)->Result<ModelAsset>{
        ensure!(store.root()==self.root,"Selected Model artifact belongs to another asset store");
        ensure!(valid_digest(input_sha256),"Selected native Model input digest is invalid");
        let entry=self.entries.get(&resource_key(source)?).context("Selected native Model has no paired source/output artifact")?;
        ensure!(entry.record.source_sha256.eq_ignore_ascii_case(input_sha256),"Selected native Model bytes differ from paired artifact input");
        if let Some(Err(error))=entry.verified.get(){bail!("{error}")}
        let(model,verified)=Self::read_verified_entry(store,entry)?;
        //The source package/entry is immutable for this loader. A cached
        //success is the same paired payload; no error is replaced by success.
        let _=entry.verified.set(Ok(verified));Ok(model)
    }

    ///A raw-source graph sidecar must agree with the existing paired artifact,
    ///including every native node ordinal. This does not invent a new input
    ///digest for converted geometry by matching its filename.
    pub(crate) fn verify_native_graph_pair(&self,source:&str,input_sha:&str,converted_sha:&str,audit_sha:&str,mapping:&serde_json::Value)->Result<()>{
        let entry=self.entries.get(&resource_key(source)?).context("Native graph has no paired render artifact")?;
        ensure!(entry.record.source_sha256==input_sha&&entry.record.converted_sha256==converted_sha&&self.metadata_sha256==audit_sha,"Native graph/source/output/material-audit digest differs");
        let rows=mapping.as_array().context("Native graph source-node mapping is absent")?;
        ensure!(rows.len()==entry.record.corrections.len(),"Native graph mesh mapping is incomplete");
        for(row,expected)in rows.iter().zip(&entry.record.corrections){
            ensure!(row["mesh_index"]==expected.mesh_index&&row["source_node_ordinal"]==expected.source_node_ordinal&&row["name"]==expected.name&&row["hierarchy"]==serde_json::json!(expected.hierarchy)&&row["vertex_count"]==expected.vertex_count&&row["triangle_count"]==expected.triangle_count,"Native graph source-node mapping differs from the paired material audit");
        }Ok(())
    }

    /// Apply once to a freshly prepared model. Source hashing/JSON decoding is
    /// cached once per model per loader; never call this from the rendering loop.
    /// Assets are expected to stay immutable for that loader's lifetime. All
    /// validation completes before changing the supplied model.
    pub fn apply(&self, store: &AssetStore, model: &mut PreparedModel) -> Result<FixStats> {
        ensure!(
            store.root() == self.root,
            "Model corrections belong to a different asset store"
        );
        let key = resource_key(&model.source)?;
        let entry = self
            .entries
            .get(&key)
            .with_context(|| format!("Model has no audited corrections: {}", model.source))?;
        let verified = entry
            .verified
            .get_or_init(|| Self::verify_entry(store, entry).map_err(|error| format!("{error:#}")))
            .as_ref()
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        ensure!(
            model.pose == verified.pose && model.meshes.len() == entry.record.mesh_count,
            "Prepared model count/pose mismatch for {}",
            model.source
        );
        for (index, (mesh, row)) in model
            .meshes
            .iter()
            .zip(&entry.record.corrections)
            .enumerate()
        {
            ensure!(
                mesh.name == row.name
                    && mesh.hierarchy == row.hierarchy
                    && mesh.vertices.len() == row.vertex_count
                    && mesh.indices.len() == row.triangle_count * 3
                    && prepared_geometry(mesh) == verified.geometry[index],
                "Prepared mesh identity/geometry mismatch for {} mesh {index}",
                model.source
            );
            if let Some(authored) = mesh.material.alpha_property_state {
                let audited = row.render_state.with_context(|| {
                    format!(
                        "Missing audited alpha state for {} mesh {index}",
                        model.source
                    )
                })?;
                ensure!(
                    authored.flags == audited.alpha_flags
                        && authored.threshold == audited.alpha_threshold,
                    "Authored alpha state disagrees with audited state for {} mesh {index}",
                    model.source
                );
            }
        }
        let remaining_positions = model
            .meshes
            .iter()
            .zip(&entry.record.corrections)
            .filter(|(_, row)| !row.skip)
            .flat_map(|(mesh, _)| mesh.vertices.iter().map(|v| &v.position));
        let bounds = Bounds::from_positions(remaining_positions).unwrap_or(Bounds {
            min: [0.0; 3],
            max: [0.0; 3],
            size: [0.0; 3],
        });
        ensure!(
            bounds
                .min
                .iter()
                .chain(&bounds.max)
                .chain(&bounds.size)
                .all(|v| v.is_finite()),
            "Invalid corrected bounds for {}",
            model.source
        );
        let mut stats = FixStats {
            meshes_before: model.meshes.len(),
            ..Default::default()
        };
        for (mesh, row) in model.meshes.iter_mut().zip(&entry.record.corrections) {
            if row.skip {
                stats.shadow_helpers_removed += 1;
            } else {
                stats.team_color_flags_changed +=
                    usize::from(mesh.material.team_color_mask != row.team_color_mask);
                stats.team_color_meshes += usize::from(row.team_color_mask);
                mesh.material.team_color_mask = row.team_color_mask;
                mesh.material.native_render_state = row.render_state;
                mesh.material.native_sort_center = row.sort_center;
            }
        }
        let mut index = 0;
        model.meshes.retain(|_| {
            let keep = !entry.record.corrections[index].skip;
            index += 1;
            keep
        });
        model.bounds = bounds;
        stats.meshes_after = model.meshes.len();
        stats.empty_after_fixes = model.meshes.is_empty();
        if stats.shadow_helpers_removed != 0 || stats.team_color_flags_changed != 0 {
            model.warnings.push(AssetWarning { source:model.source.clone(), message:format!("Audited original NIF corrections: removed {} shadow helper meshes; corrected {} team-color flags.", stats.shadow_helpers_removed, stats.team_color_flags_changed) });
        }
        Ok(stats)
    }

    /// Return the original converted-mesh index for each retained mesh. This
    /// binds an animation export to the same audited geometry/material order.
    pub fn apply_with_mesh_mapping(
        &self,
        store: &AssetStore,
        model: &mut PreparedModel,
    ) -> Result<(FixStats, Vec<usize>)> {
        let key = resource_key(&model.source)?;
        let entry = self.entries.get(&key).context("No audited model mapping")?;
        let mut retained = Vec::new();
        retained.try_reserve(entry.record.mesh_count)?;
        retained.extend(
            entry
                .record
                .corrections
                .iter()
                .filter(|row| !row.skip)
                .map(|row| row.mesh_index),
        );
        // Allocation and the full apply validation happen before any mutation.
        let stats = self.apply(store, model)?;
        Ok((stats, retained))
    }
}
