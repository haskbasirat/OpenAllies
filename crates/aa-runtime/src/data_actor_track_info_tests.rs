use super::*;
use serde_json::{Value, json};

///Replays the oracle's typed getter seams in call order. Growth (6F7810/
///6F7430, a removed fixed-capacity array) has no Rust counterpart.
struct Lab<'a> {
    entry: &'a Value,
    calls: Vec<Value>,
    prefix: &'a str,
    folder: &'a str,
    kind: String,
}
impl Source for Lab<'_> {
    type Texture = u32;
    fn string(&mut self, key: &'static str) -> Result<String> {
        self.calls.push(json!(["5B54D0", key]));
        let value = self.entry[key].as_str().unwrap().to_owned();
        if key == "track_type" {
            self.kind = value.clone();
        }
        Ok(value)
    }
    fn path(&mut self, format: &str) -> Result<String> {
        self.calls.push(json!(["405D60", format, self.prefix]));
        let expanded = super::super::super::path::expand(format, self.prefix)?;
        self.calls.push(json!(["4763C0", expanded]));
        super::super::super::path::relative(self.folder, &expanded)
    }
    fn texture(&mut self, path: &str) -> Result<Option<u32>> {
        //read_entry stores +00 from the same lookup before this call.
        self.calls.push(json!(["store+0", track_type_497220(&self.kind)?]));
        self.calls.push(json!(["5A77E0", path, 0]));
        let t = self.entry["texture"].as_u64().unwrap() as u32;
        Ok((t != 0).then_some(t))
    }
    fn flag(&mut self, key: &'static str) -> Result<bool> {
        self.calls.push(json!(["5B5380", key]));
        Ok(self.entry[key].as_u64().unwrap() != 0)
    }
    fn float(&mut self, key: &'static str) -> Result<u32> {
        self.calls.push(json!(["5B4AE0", key]));
        Ok(self.entry[key].as_u64().unwrap() as u32)
    }
    fn point(&mut self, key: &'static str) -> Result<[u32; 2]> {
        self.calls.push(json!(["5B4D80", key]));
        let v = &self.entry[key];
        Ok([v[0].as_u64().unwrap() as u32, v[1].as_u64().unwrap() as u32])
    }
}
///The 0x24-byte image; None is a byte native code never writes.
fn image(e: &TrackInfo<u32>) -> Vec<Value> {
    let mut words = [e.track_type as u32, u32::from(e.mirrored), e.placement_pcg_bits, e.texture.unwrap_or(0),
        e.offset_bits[0], e.offset_bits[1], e.size_bits[0], e.size_bits[1], e.fade_sec_bits]
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .map(|b| json!(b))
        .collect::<Vec<_>>();
    for b in &mut words[5..8] {
        *b = Value::Null;
    }
    words
}

#[test]
fn actual_464bd0_trackinfo_loop_order_offsets_and_unknown_type_error() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/development10/view-actor-tracks/native-track-info.json"
    ))
    .unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 21);
    for row in rows {
        let input = &row["input"];
        let out = &row["output"];
        let id = input["id"].as_str().unwrap();
        let mut calls = vec![json!(["5B4220", "TrackInfo"])];
        let mut entries: Vec<TrackInfo<u32>> = vec![];
        let mut error = None;
        for e in input["entries"].as_array().unwrap() {
            entries.push(TrackInfo::constructed());
            let mut lab = Lab {
                entry: e,
                calls: vec![],
                prefix: input["prefix"].as_str().unwrap(),
                folder: input["folder"].as_str().unwrap(),
                kind: String::new(),
            };
            let result = read_entry(&mut lab, id, entries.last_mut().unwrap());
            calls.extend(lab.calls);
            if let Err(e) = result {
                error = Some(e);
                break;
            }
        }
        let native: Vec<Value> = out["calls"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| !matches!(c[0].as_str(), Some("6F7810" | "6F7430" | "410C40")))
            .cloned()
            .collect();
        assert_eq!(json!(calls), json!(native), "{}", input["name"]);
        let thrown = out["thrown"].as_bool().unwrap();
        assert_eq!(error.is_some(), thrown, "{}", input["name"]);
        if let Some(error) = &error {
            let raised = out["calls"].as_array().unwrap().last().unwrap();
            assert_eq!(raised[0], "410C40");
            assert!(error.contains(&format!("Unknown track type '{}' on object '{}'", raised[2].as_str().unwrap(), raised[3].as_str().unwrap())), "{error}");
        }
        assert_eq!(entries.len() as u64, out["count"].as_u64().unwrap());
        let images: Vec<Value> = entries.iter().map(|e| json!(image(e))).collect();
        assert_eq!(json!(images), out["entries"], "{}", input["name"]);
    }
}

#[test]
fn track_type_lookup_is_c_locale_ascii_case_insensitive_only() {
    assert_eq!(track_type_497220("discrete").unwrap(), DISCRETE);
    assert_eq!(track_type_497220("CoNtInUoUs").unwrap(), CONTINUOUS);
    for name in ["", "DISCRETE ", "TREAD", "D\u{130}SCRETE", "CONT\u{131}NUOUS"] {
        assert_eq!(track_type_497220(name).unwrap(), -1, "{name}");
    }
    assert!(track_type_497220("DISCRETE\0x").is_err());
}

struct LabModel;
impl crate::resource_objects::ModelPolicy for LabModel {
    fn use_hardware_model_73a560(&mut self) -> Result<bool> {
        Ok(false)
    }
}
///Shipped UVars (DataCompatibilityMode1, DataMissingResourceWarning0).
#[derive(Default)]
struct LabTexture {
    diagnostics: usize,
}
impl TexturePolicy for LabTexture {
    fn data_compatibility_mode_current(&mut self) -> Result<u8> {
        Ok(1)
    }
    fn data_missing_resource_warning_current(&mut self) -> Result<u8> {
        Ok(0)
    }
    fn missing_texture_diagnostic_404fc0(&mut self, _: &[u16], _: u32) -> Result<()> {
        self.diagnostics += 1;
        Ok(())
    }
    fn texture_cache_source_overrides(&mut self) -> Result<crate::load_view_variables::NativeOverrides> {
        Ok(crate::load_view_variables::NativeOverrides::ControlledNone)
    }
}

///The production pre-World source preparation (the client's call for every
///registered definition) over every actual ActorData with a TrackInfo child,
///then the 45E710 read a VActor Init makes. AA_TRACK_INFO_CAPTURE=<new
///absolute path> writes the projected entries.
#[test]
#[ignore = "requires actual V14 sources (AA_GAME_ROOT); pre-World source preparation only"]
fn actual_v14_trackinfo_entries_prepare_with_their_textures() {
    use aa_rules::{actors::ActorCatalog, resources::ResourceCatalog, sources::{SourceLimits, V14Sources}, tgi::Limits};
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files = V14Sources::open(game.join("Data.rwd"), game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: u64::MAX }).unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    catalog.bind_resource_catalog(std::sync::Arc::new(ResourceCatalog::load_v14(&mut files).unwrap())).unwrap();
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    crate::combat_source_completion::install(&catalog, &mut files, receipt.context(), receipt.csv(), receipt.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    crate::replacement_flags_source::complete_source_suffix(&catalog).unwrap();
    let pc = catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let mut application = crate::resource_depot::Application::from_sources(files);
    application.install_fresh_cache_variable_scope(&catalog, crate::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    {
        let bank = catalog.definition_bank.borrow();
        application.prepare_model_source_context(&catalog, &bank).unwrap();
    }
    let work = std::env::temp_dir().join(format!("aa-trackinfo-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    application.configure_portable_work_depot(&work).unwrap();
    let (mut model, mut texture) = (LabModel, LabTexture::default());
    let bank = catalog.definition_bank.borrow();
    let (mut definitions, mut entries, mut rows) = (0, 0, vec![]);
    for actor in &actors.actors {
        if actors.native_category(&actor.id).unwrap().native_kind != 2 {
            continue;
        }
        let node = actors.resolved_actor(&actor.id).unwrap();
        let count = match node.child("MoverComponent").unwrap() {
            Some(mover) => mover.children("TrackInfo").unwrap().len(),
            None => 0,
        };
        if count == 0 {
            continue;
        }
        let definition = catalog.resolve_registered(&actor.id, pc).unwrap();
        let infos = application
            .with_data_pre_request_resources(|state, resources| {
                state.prepare_object_requests(&catalog, &bank, &definition, resources, &mut model, &mut texture)?;
                let view = state.actor_view_source(&catalog, &bank, &definition)?;
                assert_eq!(state.actor_view_track_count_260(&view)? as usize, count);
                assert!(state.actor_view_track_info_45e710(&view, count).is_err());
                (0..count).map(|i| state.actor_view_track_info_45e710(&view, i)).collect::<Result<Vec<_>>>()
            })
            .unwrap_or_else(|e| panic!("{}: {e}", actor.id));
        for info in &infos {
            assert!(matches!(info.track_type, DISCRETE | CONTINUOUS), "{}", actor.id);
            let handle = info.texture.as_ref().unwrap_or_else(|| panic!("{} NULL Track texture", actor.id));
            rows.push(json!({"id": actor.id, "type": info.track_type, "mirrored": info.mirrored,
                "pcg": info.placement_pcg_bits, "offset": info.offset_bits, "size": info.size_bits,
                "fade": info.fade_sec_bits, "texture": String::from_utf16_lossy(handle.logical_ids())}));
        }
        definitions += 1;
        entries += infos.len();
    }
    println!("TrackInfo definitions {definitions}, entries {entries}, missing-texture diagnostics {}", texture.diagnostics);
    assert!(definitions >= 200 && texture.diagnostics == 0);
    if let Some(path) = std::env::var_os("AA_TRACK_INFO_CAPTURE") {
        let path = std::path::PathBuf::from(path);
        assert!(path.is_absolute() && !path.exists());
        std::fs::write(path, serde_json::to_vec_pretty(&json!({"definitions": definitions, "entries": rows})).unwrap()).unwrap();
    }
}
