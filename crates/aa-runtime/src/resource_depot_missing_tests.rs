//! Real-data pre-World source census for the5130A0 missing-source family.
//! The laboratory policies below are explicit seams: hardware model false,
//! UVar61 DataCompatibilityMode1 and DataMissingResourceWarning0 (the shipped
//! UVars.tgi/UVars_AA.tgi values). No World, GPU or decoder readiness.
use super::*;
use crate::{
    combat_source_receipt::VerifiedStartupReceipt,
    load_view_variables::NativeOverrides,
    resource_objects::{ModelPolicy, TexturePolicy},
};
use aa_rules::{actors::ActorCatalog, resources::ResourceCatalog, sources::SourceLimits, tgi::Limits};

struct LabModel;
impl ModelPolicy for LabModel {
    fn use_hardware_model_73a560(&mut self) -> Result<bool> { Ok(false) }
}
#[derive(Default)]
struct LabTexture { diagnostics: Vec<(String, u32)> }
impl TexturePolicy for LabTexture {
    fn data_compatibility_mode_current(&mut self) -> Result<u8> { Ok(1) }
    fn data_missing_resource_warning_current(&mut self) -> Result<u8> { Ok(0) }
    fn missing_texture_diagnostic_404fc0(&mut self, ids: &[u16], level: u32) -> Result<()> {
        self.diagnostics.push((String::from_utf16_lossy(ids), level));
        Ok(())
    }
    fn texture_cache_source_overrides(&mut self) -> Result<NativeOverrides> { Ok(NativeOverrides::ControlledNone) }
}

fn open_sources() -> V14Sources {
    let game = PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    V14Sources::open(game.join("Data.rwd"), game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: u64::MAX }).unwrap()
}
fn completed_catalog<'a>(actors: &'a ActorCatalog, files: &mut V14Sources) -> Catalog<'a> {
    let mut catalog = Catalog::new(actors);
    catalog.settings = Some(crate::settings::Settings::load_v14(files, Limits::default()).unwrap());
    catalog.bind_resource_catalog(Arc::new(ResourceCatalog::load_v14(files).unwrap())).unwrap();
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, files).unwrap();
    crate::combat_source_completion::install(&catalog, files, receipt.context(), receipt.csv(), receipt.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    crate::replacement_flags_source::complete_source_suffix(&catalog).unwrap();
    catalog
}

///Counts every current-variable read; the catch path is the only reader.
struct Recording { mode: u8, warning: u8, reads: Vec<&'static str> }
impl TexturePolicy for Recording {
    fn data_compatibility_mode_current(&mut self) -> Result<u8> { self.reads.push("mode"); Ok(self.mode) }
    fn data_missing_resource_warning_current(&mut self) -> Result<u8> { self.reads.push("warning"); Ok(self.warning) }
}
fn units(text: &str) -> Vec<u16> { text.encode_utf16().collect() }

///Replays every row of research/phase8-resource-missing/fixtures.json (original
///5A80D0/59B4B0/59B280 under Unicorn) through the SAME Rust factories on the
///real v14 sources. Missing rows use the oracle's names, which are genuinely
///absent from Data.rwd and every depot; the oracle's "present" seam is
///replayed with a real present file of the same class. One Application is
///shared, so a cached hit replays the native5AD430 path after the first row.
#[test]
#[ignore = "requires actual V14 sources (AA_GAME_ROOT); replays the native catch-path oracle"]
fn native_missing_catch_fixture_replay() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase8-resource-missing/fixtures.json")).unwrap();
    let mut files = open_sources();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let catalog = completed_catalog(&actors, &mut files);
    let mut application = Application::from_sources(files);
    application.install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone).unwrap();
    let bank = catalog.definition_bank.borrow();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 24);
    let mut resources = application.resources();
    let mut expected_diagnostics = Vec::new();
    for row in rows {
        let case = &row["case"];
        let (mode, warning) = (case["mode"].as_u64().unwrap() as u8, case["warning"].as_u64().unwrap() as u8);
        let present = case["present"].as_bool().unwrap();
        let events = row["events"].as_array().unwrap();
        let threw = events.iter().any(|e| e.get("throw").is_some());
        let logged: Vec<_> = events.iter().filter(|e| e["call"] == "0x404fc0").collect();
        let expected_reads: Vec<&str> = if !threw { vec![] } else if logged.is_empty() { vec!["mode"] } else { vec!["mode", "warning"] };
        for e in &logged {
            expected_diagnostics.push(MissingResourceDiagnostic {
                channel: match e["channel"].as_str().unwrap() { "0xb38c28" => DiagnosticChannel::ResourceB38C28,
                    "0xb38b98" => DiagnosticChannel::AudioB38B98, other => panic!("{other}") },
                level: e["level"].as_u64().unwrap() as u32,
                file: if e["file"] == ".\\Source\\resource\\NiResource.cpp" { ".\\Source\\resource\\NiResource.cpp" } else { ".\\Source\\resource\\AudioResource.cpp" },
                line: e["line"].as_u64().unwrap() as u32,
                function: if e["function"] == "KKC_NiSeqResource::GetNiSeqResource" { "KKC_NiSeqResource::GetNiSeqResource" } else { "KKC_AudioResource::GetDefaultResource" },
                format: if e["format"] == "Missing animation resource '%s' - disabling animation" { "Missing animation resource '%s' - disabling animation" } else { "Missing audio resource '%s' - replacing with default" },
                ids: units(e["name"].as_str().unwrap()),
            });
            let last = expected_diagnostics.last().unwrap();
            assert_eq!((last.file, last.function, last.format), (e["file"].as_str().unwrap(), e["function"].as_str().unwrap(), e["format"].as_str().unwrap()));
        }
        let mut policy = Recording { mode, warning, reads: Vec::new() };
        let outcome = &row["outcome"];
        if case["factory"] == "5A80D0" {
            let name = if present { "Units/Ambient/Cow/CowWalk.kf" } else { events[0]["name"].as_str().unwrap() };
            let result = resources.sequence_5a80d0_with_policy(&catalog, &bank, &units(name), &mut policy);
            match (outcome.get("rethrown"), outcome["returned"].as_str()) {
                (Some(_), _) => assert!(result.unwrap_err().contains("5A80D0 rethrows")),
                (None, Some("0x0")) => assert!(result.unwrap().is_none()),
                (None, Some(_)) => assert_eq!(result.unwrap().unwrap().kind(), crate::resource_objects::Kind::Sequence),
                _ => panic!("fixture outcome"),
            }
        } else {
            let streaming = case["streaming"].as_bool().unwrap();
            let name = if present { if streaming { "Audio/Sounds/RUS/Attack/RUSRifle02.wav" } else { "Audio/Sounds/RUS/Attack/RUSRifle01.wav" } }
                else { events[0]["name"].as_str().unwrap() };
            let result = resources.sound_59b4b0_with_policy(&catalog, &bank, &units(name), streaming, &mut policy);
            if outcome.get("rethrown").is_some() {
                assert!(result.unwrap_err().contains("59B4B0 rethrows"));
            } else {
                let handle = result.unwrap();
                let expected_ids = match events.iter().find(|e| e["call"] == "0x410cb0") {
                    //59B280's inner59B1F0 receives exactly the current AVar text.
                    Some(e) => { assert_eq!(events.iter().filter(|v| v["call"] == "0x59b1f0").nth(1).unwrap()["name"], e["value"]); e["value"].as_str().unwrap() }
                    None => name,
                };
                assert_eq!(String::from_utf16(handle.logical_ids()).unwrap(), expected_ids);
                assert_eq!(resources.sound_bytes_40_42(&handle).unwrap()[1] as u64, outcome["byte41"].as_u64().unwrap());
            }
        }
        assert_eq!(policy.reads, expected_reads, "{case}");
        assert_eq!(resources.missing_resource_diagnostics().unwrap(), expected_diagnostics, "{case}");
        if !present {
            let missing = resource_map_cache::normalized_key(&units(events[0]["name"].as_str().unwrap())).unwrap();
            assert!(!resources.manager.lock().unwrap().resources.contains_key(&missing), "missing name was published");
        }
    }
    let messages: Vec<_> = resources.missing_resource_diagnostics().unwrap().iter().map(MissingResourceDiagnostic::message).collect();
    assert!(messages.contains(&"Missing animation resource 'Units/Ambient/Cow/Cowjog.kf' - disabling animation".to_string()));
    assert!(messages.contains(&"Missing audio resource 'Audio/Sounds/RUS/Attack/RUSRocketTruck01.wav' - replacing with default".to_string()));
    //The policy-less factories still refuse the catch path explicitly, naming the path.
    let mut resources = application.resources();
    assert!(resources.sequence_5a80d0(&catalog, &bank, &units("Units/Ambient/Cow/Cowjog.kf")).unwrap_err().contains("Units/Ambient/Cow/Cowjog.kf"));
    assert!(resources.sound_59b4b0(&catalog, &bank, &units("Audio/Sounds/RUS/Attack/RUSRocketTruck02.wav"), false).unwrap_err().contains("RUSRocketTruck02.wav"));
}

///Every missing name reported by the Berlin census (look01 stderr), through
///the policy factories with the shipped UVars.tgi/UVars_AA.tgi values
///(DataCompatibilityMode true, DataMissingResourceWarning false).
#[test]
#[ignore = "requires actual V14 sources (AA_GAME_ROOT); census paths through the native catch paths"]
fn census_missing_names_take_native_catch_paths() {
    let mut files = open_sources();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let catalog = completed_catalog(&actors, &mut files);
    let mut application = Application::from_sources(files);
    application.install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone).unwrap();
    let bank = catalog.definition_bank.borrow();
    let mut resources = application.resources();
    let sequences = [
        "Units/JPN/GJPNBattleship/JPNBattleshipRecoil5.kf", "Units/Ambient/Sheep/SheepFidget2.kf",
        "Units/USA/USAInfantryFlame/USAInfantryFlameJogKneel.kf", "Units/USA/USAInfantryFlame/USAInfantryFlameIdleAttackKneel.kf",
        "Units/USA/USAInfantryFlame/USAInfantryFlameAttack1Kneel.kf", "Units/Ambient/Horse/Horsejog.kf",
        "Units/Ambient/Fish/Fishdie1.kf", "Units/Ambient/Cow/Cowjog.kf",
        "Buildings/GER/GERBunkerArtillery/GERBunkerArtilleryDisabledRecoil1.kf", "Buildings/GER/GERBunkerAA/GERBunkerAADisabledRecoil1.kf",
        "Doodads/Tropical/TropicalBamboo/TropicalBamboo01Rustle.kf", "Doodads/Tropical/TropicalGrassTall/TropicalTallGrass.kf",
    ];
    for name in sequences {
        //Absent in every accepted depot, in any ASCII case.
        let key = resource_map_cache::normalized_key(&units(name)).unwrap();
        assert!(!resources.sources.catalog().iter().any(|f| resource_map_cache::normalized_key(&units(&f.logical_path)).as_ref() == Ok(&key)), "{name}");
        let mut policy = Recording { mode: 1, warning: 0, reads: Vec::new() };
        assert!(resources.sequence_5a80d0_with_policy(&catalog, &bank, &units(name), &mut policy).unwrap().is_none(), "{name}");
        assert_eq!(policy.reads, ["mode", "warning"]);
    }
    let mut default = None;
    for name in ["Audio/Sounds/RUS/Attack/RUSRocketTruck01.wav", "Audio/Sounds/RUS/Attack/RUSRocketTruck02.wav"] {
        let mut policy = Recording { mode: 1, warning: 0, reads: Vec::new() };
        let handle = resources.sound_59b4b0_with_policy(&catalog, &bank, &units(name), false, &mut policy).unwrap();
        assert_eq!(String::from_utf16(handle.logical_ids()).unwrap(), "audio/Default/SoundDefault.wav");
        //The second miss reaches the SAME cached default allocation.
        if let Some(first) = &default { assert!(handle.same_identity(first)) } else { default = Some(handle) }
    }
    let diagnostics = resources.missing_resource_diagnostics().unwrap();
    assert_eq!(diagnostics.len(), sequences.len() + 2);
    assert!(diagnostics.iter().all(|d| d.level == 10));
    for d in &diagnostics { println!("native404FC0 {:?} level{}: {}", d.channel, d.level, d.message()); }
}

#[test]
#[ignore = "requires actual V14 sources (AA_GAME_ROOT); pre-World source census, no World/GPU readiness"]
fn actual_pre_world_missing_source_census() {
    let mut files = open_sources();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let catalog = completed_catalog(&actors, &mut files);
    let pc = catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let mut application = Application::from_sources(files);
    application.install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone).unwrap();
    {
        let bank = catalog.definition_bank.borrow();
        application.prepare_model_source_context(&catalog, &bank).unwrap();
    }
    let work = std::env::temp_dir().join(format!("aa-missing-census-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    application.configure_portable_work_depot(&work).unwrap();
    let only = std::env::var("AA_MISSING_CENSUS_IDS").ok();
    let (mut model, mut texture) = (LabModel, LabTexture::default());
    let (mut prepared, mut failures) = (0usize, Vec::new());
    for entry in &catalog.actors.actors {
        if let Some(only) = &only { if !only.split(',').any(|v| v.eq_ignore_ascii_case(&entry.id)) { continue } }
        let thing = aa_rules::actors::ThingRef::registered(&entry.id);
        let Ok(category) = catalog.actors.native_category(thing) else { continue };
        if !matches!(category.native_kind, 1 | 2 | 20 | 21) { continue }
        let result = (|| -> Result<()> {
            let definition = catalog.resolve_registered(&entry.id, pc)?;
            let bank = catalog.definition_bank.try_borrow().map_err(|_| "Bank borrowed")?;
            application.with_data_pre_request_resources(|state, resources| {
                state.prepare_object_requests(&catalog, &bank, &definition, resources, &mut model, &mut texture).map(|_| ())
            })
        })();
        match result {
            Ok(()) => prepared += 1,
            Err(error) => failures.push(format!("{} ({}): {error}", entry.id, entry.source.path)),
        }
    }
    for failure in &failures { println!("census failure: {failure}"); }
    for (ids, level) in &texture.diagnostics { println!("census texture diagnostic level{level}: {ids}"); }
    println!("census prepared {prepared}, failed {}", failures.len());
}
