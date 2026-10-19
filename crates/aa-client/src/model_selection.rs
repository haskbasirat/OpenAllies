//!5A94F0 KKC_NiAVObjResource::ComputeSelectionCuboid over the SAME selected
//!graph (child of model_payload_backend; native proof in
//!research/development10/model-selection-fallback).
//!
//!5A9524 looks up the Bone "selection_box" CString, 5A955C calls the root's
//!vf40 GetObjectByName (first preorder match), 5A95A0 keeps the object only
//!if its RTTI chain reaches NiGeometry (B43CF0), then 5A95C4 runs the root's
//!vf48 Update(0,0). A kept NiGeometry gives the 5A96B2 vertex extents and is
//!unlinked (6FF850); otherwise the caller (model_resource_load) applies the
//!BoundingSphere fallback to the root's world bound.
use super::*;
use aa_runtime::resource_model_payload::Selection;

///5A95A0 RTTI B43CF0 membership for the graph kinds the port represents.
fn ni_geometry(kind: &str) -> Result<bool> {
    match kind {
        "NiTriShape" | "NiTriStrips" => Ok(true),
        //NiNode and its subclasses derive from NiAVObject, not NiGeometry.
        k if k.ends_with("Node") => Ok(false),
        k => Err(format!("5A95A0 RTTI B43CF0 (NiGeometry) membership of {k} is unproven")),
    }
}

pub(super) fn compute_5a94f0(m: &mut Selected) -> Result<Selection> {
    let name = m
        .source()?
        .selection_box_name()?
        .ok_or("5A94F0 missing Bone selection_box fallback CString owner is required")?
        .to_vec();
    let found = match m.find_name(&name) {
        Some(i) if ni_geometry(&m.graph.nodes[i].kind)? => Some(i),
        _ => None,
    };
    m.update()?;
    let Some(i) = found else {
        return m.graph.nodes[m.root].world_sphere.map(Selection::RootSphere).ok_or_else(|| {
            "5A95FE root world bound after Update(0,0) has no merged child sphere; NiNode zero-bound words are unproven".into()
        });
    };
    let n = &m.graph.nodes[i];
    let out = math::selection_geometry(&n.world.ok_or("selection world transform absent")?, &m.geometry_data(i)?.vertices, m.pc)?;
    let parent = n.parent.ok_or("6FF850 actual selection parent is NULL")?;
    let slot = m.graph.nodes[parent]
        .children
        .iter()
        .position(|c| *c == Some(i))
        .ok_or("selection child is not in same parent")?;
    m.graph.nodes[parent].children[slot] = None;
    m.graph.nodes[i].parent = None;
    m.graph.nodes[i].detached = true;
    Ok(Selection::Geometry(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_runtime::resource_objects::Handle;
    use std::path::PathBuf;
    ///Explicit laboratory inputs: the original (non-hardware) model variant,
    ///the actual current UI/AVar sources, and recorded 404ED0 diagnostics.
    struct Lab<'a> {
        variables: &'a aa_ui::uvars::UVars,
        avars: &'a aa_ui::resources::AVars,
        store: &'a AssetStore,
        pc: Precision,
        timer: Option<crate::windows_reset_os::PerformanceTimer>,
        diagnostics: Vec<u32>,
    }
    impl aa_runtime::resource_objects::ModelPolicy for Lab<'_> {
        fn use_hardware_model_73a560(&mut self) -> Result<bool> {
            Ok(false)
        }
    }
    impl MetadataPolicy for Lab<'_> {
        fn renderer_remove_normals_current(&mut self) -> Result<u8> {
            self.variables.renderer_remove_normals_current()
        }
        fn preprocessed_data_missing_warning_current(&mut self) -> Result<u8> {
            self.avars.preprocessed_data_missing_warning_current()
        }
        fn preprocessed_data_wrong_version_warning_current(&mut self) -> Result<u8> {
            self.avars.preprocessed_data_wrong_version_warning_current()
        }
    }
    impl aa_runtime::model_resource_load::Backend for Lab<'_> {
        fn resource_no_ni_objects_current(&mut self) -> Result<u8> {
            self.variables.resource_no_ni_objects_current()
        }
        fn begin_model_load(&mut self, _: &Handle) -> Result<()> {
            crate::windows_reset_os::PerformanceTimer::initialize_frequency()?;
            self.timer = Some(crate::windows_reset_os::PerformanceTimer::from_native_constructor().map_err(|e| e.to_string())?);
            Ok(())
        }
        fn decode_selected_model(&mut self, input: &Input) -> Result<Payload> {
            prepare(input, self.store, self.pc)
        }
        fn process_model_metadata(&mut self, payload: &Payload, skin: bool) -> Result<Metadata> {
            payload.process_metadata_with_policy(skin, self)
        }
        fn skip_loaded_model_bounds(&mut self) -> Result<u8> {
            self.variables.resource_caching_current()
        }
        fn selection_diagnostic_404ed0(&mut self, _: &Handle, d: aa_runtime::model_resource_load::SelectionDiagnostic) -> Result<()> {
            self.diagnostics.push(d.line());
            Ok(())
        }
    }
    #[test]
    #[ignore = "requires AA_GAME_ROOT original/v14 sources; explicit non-hardware variant lab, not a ResourceTask/GPU claim"]
    fn berlin_no_selection_models_root_bound_and_fallback_match_native_all_precisions() {
        use aa_runtime::{catalog::Catalog, combat_source_receipt::VerifiedStartupReceipt, world_ready_services::Application};
        use aa_rules::{actors::ActorCatalog, sources::{SourceLimits, V14Sources}, tgi::Limits};
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/model-selection-fallback/native-selection-fallback.json"
        ))
        .unwrap();
        let game = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut files = V14Sources::open(game.join("Data.rwd"), game.join("reconstruction/native-game/runtime-expanded-v14"), SourceLimits { max_member_bytes: u64::MAX }).unwrap();
        let actors = ActorCatalog::load_v14(&mut files).unwrap();
        let mut catalog = Catalog::new(&actors);
        catalog.settings = Some(aa_runtime::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
        let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
        aa_runtime::combat_source_completion::install(&catalog, &mut files, receipt.context(), receipt.csv(), receipt.globals()).unwrap();
        aa_runtime::combat_source_completion::compute(&catalog).unwrap();
        let startup = aa_runtime::actor_creation_startup::StartupInputs::from_verified_startup_reference(&receipt).unwrap();
        let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/development10/model-selection-fallback/lab-work");
        std::fs::create_dir_all(&output).unwrap();
        let mut app = Application::from_fresh_sources(&catalog, files, aa_runtime::application_mode::GameAppState::from_native_constructor(), startup, &output,
            aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
        let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
        let store = AssetStore::open(game.join("reconstruction")).unwrap();
        let mut resources = app.depot_mut().resources();
        let root = aa_ui::fs::DataRoot::with_overlay(game.join("reconstruction/assets/original/Data"), game.join("reconstruction/native-game/runtime-expanded-v14/Data"));
        let mut tgi = aa_ui::tgi::TgiStore::new(root);
        let variables = aa_ui::uvars::UVars::load_with_view_scope(&mut tgi, aa_ui::view_variables::SourceOverrides::ControlledNone);
        let avars = aa_ui::resources::AVars::load(&mut tgi);
        let mut checked = 0;
        let mut upstream = Vec::new();
        for model in fixture["models"].as_array().unwrap().iter().filter(|m| m.get("excluded").is_none()) {
            let source = model["source"].as_str().unwrap();
            let rows: Vec<&serde_json::Value> = fixture["rows"].as_array().unwrap().iter().filter(|r| r["kind"] == "model" && r["name"] == source).collect();
            let mut lab = Lab { variables: &variables, avars: &avars, store: &store, pc: Precision::Double53, timer: None, diagnostics: vec![] };
            let ids: Vec<u16> = source.encode_utf16().collect();
            let handle = resources.model_5a8520(&catalog, &bank, &ids, &mut lab).unwrap();
            let input = resources.model_input_4c(&handle).unwrap();
            assert_eq!(input.sha256(), model["source_sha256"].as_str().unwrap(), "{source}");
            //The whole 5A99C0 kernel on the SAME Entry at the measured PC53; it
            //also publishes the B43E88 loading context the payload calls read.
            lab.pc = Precision::Double53;
            //A refusal BEFORE 5A94F0 (the model graph/texture backend) is reported,
            //never counted as a selection result.
            if let Err(error) = resources.load_model_4c(&handle, Precision::Double53, &mut lab) {
                assert!(!error.contains("5A94F0") && !error.contains("5A95"), "{source}: {error}");
                upstream.push(format!("{source}: {error}"));
                continue;
            }
            let row = rows.iter().find(|r| r["precision"] == "Double53" && r["flags"] == 0x410).unwrap();
            assert_eq!(serde_json::json!(resources.model_selection_cuboid_4c(&handle).unwrap()), row["output"]["cuboid"], "{source}");
            let lines: Vec<u64> = row["output"]["events"].as_array().unwrap().iter().filter(|e| e[0] == "404ED0").map(|e| e[3].as_u64().unwrap()).collect();
            assert_eq!(lab.diagnostics.iter().map(|&v| v as u64).collect::<Vec<_>>(), lines, "{source}");
            for pc in [Precision::Single24, Precision::Double53, Precision::Extended64] {
                let name = match pc {
                    Precision::Single24 => "Single24",
                    Precision::Double53 => "Double53",
                    Precision::Extended64 => "Extended64",
                };
                let row = rows.iter().find(|r| r["precision"] == name && r["flags"] == 0x410).unwrap();
                lab.pc = pc;
                let payload = prepare(&input, &store, pc).unwrap();
                payload.process_metadata_with_policy(false, &mut lab).unwrap();
                payload.prepare_effect(0x871bc0).unwrap(); //constructor flags+8 bit10
                payload.controllers().unwrap();
                let selection = payload.selection_5a94f0().unwrap();
                let sphere: [u32; 4] = serde_json::from_value(row["output"]["root_sphere"].clone()).unwrap();
                assert_eq!(selection, Selection::RootSphere(sphere), "{source} {pc:?} port 86E8D0 root bound vs native composition");
                let fallback = aa_runtime::resource_model_bounds::selection_from_sphere(sphere, pc).unwrap();
                assert_eq!(serde_json::json!(fallback.cuboid), row["output"]["cuboid"], "{source} {pc:?}");
            }
            checked += 1;
        }
        println!("{checked} Berlin no-SelectionBox models: port root bound + 5A94F0 fallback equal native at all PCs");
        for line in &upstream {
            println!("refused before 5A94F0: {line}");
        }
        assert_eq!(checked + upstream.len(), 15);
        assert!(checked >= 1);
    }
}
