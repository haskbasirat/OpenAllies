use super::*;
fn fixture(name: &str) -> serde_json::Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-view-asset-backend")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn units(v: &serde_json::Value) -> Vec<u16> {
    v.as_str().unwrap().encode_utf16().collect()
}
#[test]
fn exact_native_path_and_current_suffix_boundaries_all_pc() {
    for row in fixture("native-resource-paths.json")["rows"]
        .as_array()
        .unwrap()
    {
        for case in row["native"]["paths"].as_array().unwrap() {
            assert_eq!(
                subfolder_candidate(&units(&case["name"]), &units(&case["subfolder"])).unwrap(),
                units(&case["candidate"]),
                "{case}"
            );
        }
        for case in row["native"]["models"].as_array().unwrap() {
            let suffix = case["suffix"]
                .as_str()
                .map(|v| v.encode_utf16().collect::<Vec<_>>());
            assert_eq!(
                model_variant(
                    &units(&case["name"]),
                    case["hardware"].as_bool().unwrap(),
                    suffix.as_deref()
                )
                .unwrap(),
                units(&case["result"]),
                "{case}"
            );
        }
    }
    struct Current {
        enabled: bool,
        reads: usize,
    }
    impl ModelPolicy for Current {
        fn use_hardware_model_73a560(&mut self) -> Result<bool> {
            Ok(self.enabled)
        }
        fn hardware_model_suffix_40f7b0(&mut self) -> Result<Vec<u16>> {
            self.reads += 1;
            Err("missing actual current suffix".into())
        }
    }
    let name = "file.nif".encode_utf16().collect::<Vec<_>>();
    let mut current = Current {
        enabled: false,
        reads: 0,
    };
    assert_eq!(current_model_variant(&name, &mut current).unwrap(), name);
    assert_eq!(current.reads, 0);
    current.enabled = true;
    assert!(
        current_model_variant(&name, &mut current)
            .unwrap_err()
            .contains("missing actual")
    );
    assert_eq!(current.reads, 1);
}
#[test]
fn actual_native_missing_texture_catch_policy_all_pc() {
    struct Current {
        value: u8,
        reads: usize,
    }
    impl TexturePolicy for Current {
        fn data_compatibility_mode_current(&mut self) -> Result<u8> {
            self.reads += 1;
            Ok(self.value)
        }
    }
    for row in fixture("native-texture-missing.json")["rows"]
        .as_array()
        .unwrap()
    {
        for case in row["native"].as_array().unwrap() {
            if case["kind"] != "missing" {
                continue;
            }
            let mut policy = Current {
                value: case["compatibility_raw"].as_u64().unwrap() as u8,
                reads: 0,
            };
            let action =
                missing_texture_5a7976(case["caller_word"].as_u64().unwrap() as u32, &mut policy)
                    .unwrap();
            assert_eq!(policy.reads, 1);
            let expected = match case["result"].as_str().unwrap() {
                "null" => MissingTexture::Null,
                "rethrow" => MissingTexture::Rethrow,
                "default_texture_5a6e90_required" => MissingTexture::DefaultRequired,
                other => panic!("unexpected native action {other}"),
            };
            assert_eq!(action, expected, "{case}");
            assert_eq!(case["current_reads"], serde_json::json!(["0x5a7981"]));
        }
    }
    assert!(
        missing_texture_5a7976(1, &mut RequiredTexturePolicy)
            .err()
            .is_some_and(|error| error.contains("UVar61"))
    );
}
#[test]
fn native_default_texture_attaches_actual_work_identity_and_retains_unwritten_dimensions() {
    let origin = Arc::new(());
    let priority = crate::resource_task::DefaultPriority::from_fresh_application(&origin);
    for row in fixture("native-default-texture.json")["rows"]
        .as_array()
        .unwrap()
    {
        for case in row["native"].as_array().unwrap() {
            //Explicit identity-only fixture for the actual native Directory
            //transport. The public factory obtains it only from real work.
            let depot = crate::film_create::DepotId::from_work_owner();
            let mut entry = Entry::default_texture(
                &origin,
                "EnvironmentMap.tga".encode_utf16().collect(),
                &priority,
                depot.clone(),
            )
            .unwrap();
            assert_eq!(entry.references14, 0); //class creation has not run.
            entry.increment_reference14();
            assert_eq!(entry.flags8, case["flags8"]);
            assert_eq!(entry.references14, case["references14"]);
            assert_eq!(entry.requests4, case["requests4"]);
            assert_eq!(entry.locations.len(), 1);
            assert!(matches!(&entry.locations[0],Location::Work(actual)if actual.same(&depot)));
            assert!(entry.handle.source_file().is_none());
            let t = entry.texture.as_ref().unwrap();
            assert!(t.source3c.is_none() && t.writable_child40.is_none());
            assert!(t.word54.is_none() && t.word58.is_none());
            assert_eq!(
                missing_texture_diagnostic_level(case["warning_raw"].as_u64().unwrap() as u8),
                case["diagnostic_levels"][0]
            );
            assert_eq!(entry.task.references4().unwrap(), case["task_refs"]);
        }
    }
}
#[test]
fn exact_native_texture_and_model_constructor_fields_all_pc() {
    let native = fixture("native-resource-constructors.json");
    let origin = Arc::new(());
    let priority = crate::resource_task::DefaultPriority::from_fresh_application(&origin);
    for row in native["rows"].as_array().unwrap() {
        assert_eq!(row["native"]["globals"]["0xb282f0"], priority.current());
        for case in row["native"]["rows"].as_array().unwrap() {
            let source = SourceFile {
                logical_path: case["name"].as_str().unwrap().into(),
                size: 0,
                origin: aa_rules::sources::Origin::OriginalArchive { member_index: 0 },
            };
            //Explicit constructor-input transport, not an observed whole resource load.
            let entry = if case["kind"] == "Texture" {
                Entry::texture(&origin, units(&case["name"]), source, &priority)
            } else {
                Entry::model(&origin, units(&case["name"]), source, &priority)
            }
            .unwrap();
            assert_eq!(
                entry.requests4,
                case["base"]["0x4"].as_u64().unwrap() as u32
            );
            assert_eq!(entry.flags8, case["base"]["0x8"].as_u64().unwrap() as u32);
            assert_eq!(
                entry.references14,
                case["base"]["0x14"].as_u64().unwrap() as u32
            );
            assert!(entry.locations.is_empty());
            assert!(entry.task.resource().same_identity(&entry.handle));
            assert_eq!(entry.task.references4().unwrap(), case["task"]["0x4"]);
            assert_eq!(entry.task.priority8().unwrap(), case["task"]["0x8"]);
            assert_eq!(entry.task.sequence_c().unwrap(), case["task"]["0xc"]);
            assert!(!entry.task.queued10().unwrap());
            assert_eq!(entry.task.state14().unwrap(), case["task"]["0x14"]);
            assert!(entry.task.thread18().unwrap().is_none());
            if let Some(t) = &entry.texture {
                assert!(t.source3c.is_none() && t.writable_child40.is_none());
                assert_eq!(
                    serde_json::json!(t.rect44),
                    serde_json::json!([
                        case["texture"]["0x44"],
                        case["texture"]["0x48"],
                        case["texture"]["0x4c"],
                        case["texture"]["0x50"]
                    ])
                );
                assert!(t.word54.is_none() && t.word58.is_none());
                assert_eq!(case["texture"]["0x54"], case["poison"]);
                assert_eq!(case["texture"]["0x58"], case["poison"]);
                assert_eq!(
                    t.render_count60,
                    case["texture"]["0x60"].as_u64().unwrap() as usize
                );
                assert_eq!(t.render_slots.len(), 1);
                assert_eq!(serde_json::json!(t.render_slots[0]), case["render"]);
            }
            if let Some(m) = &entry.model {
                assert!(m.source3c.is_none() && m.source40.is_none());
                assert_eq!(m.word44, case["model"]["0x44"]);
                assert_eq!(m.value48, case["model"]["0x48"]);
                assert_eq!(m.radius64, case["model"]["0x64"]);
                for (i, v) in m.words4c_60.iter().enumerate() {
                    assert_eq!(
                        serde_json::json!(v),
                        case["model"][format!("0x{:x}", 0x4c + i * 4)]
                    );
                }
                for (i, v) in m.factors68_74.iter().enumerate() {
                    assert_eq!(
                        serde_json::json!(v),
                        case["model"][format!("0x{:x}", 0x68 + i * 4)]
                    );
                }
                assert_eq!(m.words78_7c, [0, 0]);
                assert_eq!(m.slots80.len(), 1);
                assert!(m.slots80[0].0.is_empty() && m.slots80[0].1.is_empty());
                assert!(m.name8c.is_empty());
                assert_eq!(case["slot_strings"], serde_json::json!(["", ""]));
                assert_eq!(case["name8c"], "");
                assert_eq!(m.count84, 0);
            }
        }
        for case in row["native"]["classes"].as_array().unwrap() {
            let actual = class_for_name(&units(&case["name"])).unwrap();
            let expected = match case["descriptor"].as_str().unwrap() {
                "0xb3d97c" => Some(Kind::Texture),
                "0xb3d990" => Some(Kind::Model),
                "0xb3d9a4" => Some(Kind::Sequence),
                _ => None,
            };
            assert_eq!(actual, expected);
        }
    }
}
#[test]
#[ignore = "requires actual completed source Bank and same Application resources; no native worker/GPU execution"]
fn actual_selected_resource_metadata_same_manager_and_moved_bank() {
    use crate::{
        catalog::Catalog, combat_source_receipt::VerifiedStartupReceipt,
        load_view_variables::NativeOverrides,
    };
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let aliases = crate::view_asset_aliases::State::from_fresh_sources(
        &catalog,
        &mut files,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let model = files
        .catalog()
        .into_iter()
        .find(|v| v.logical_path.to_ascii_lowercase().ends_with(".nif"))
        .unwrap()
        .logical_path
        .encode_utf16()
        .collect::<Vec<_>>();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let mut app = crate::resource_depot::Application::from_sources(files);
    let missing_environment = aliases
        .alias_410cb0(&catalog, &bank, "TextureEnvironmentMap")
        .unwrap();
    let mut resources = app.resources();
    //Actual EnvironmentMap.tga is absent. The native catch requires its
    //current UI61 allocation before the real default Texture factory; this
    //metadata test must not claim that source-created resource exists.
    assert!(
        resources
            .texture_5a77e0(&catalog, &bank, missing_environment.units(), 0)
            .unwrap_err()
            .contains("UVar61")
    );
    let alias = aliases
        .alias_410cb0(&catalog, &bank, "TextureCircle")
        .unwrap();
    let texture = resources
        .texture_5a77e0(&catalog, &bank, alias.units(), 0)
        .unwrap()
        .unwrap();
    assert_eq!(resources.flags_08(&texture).unwrap(), 0x200000);
    assert_eq!(resources.references_14(&texture).unwrap(), 0);
    assert_eq!(resources.requests_04(&texture).unwrap(), 0);
    let same = resources
        .texture_5a77e0(&catalog, &bank, alias.units(), 0)
        .unwrap()
        .unwrap();
    assert!(same.same_identity(&texture));
    assert_eq!(resources.references_14(&texture).unwrap(), 1);
    struct SourcePolicy;
    impl ModelPolicy for SourcePolicy {
        fn use_hardware_model_73a560(&mut self) -> Result<bool> {
            Ok(false)
        }
    }
    //Explicit renderer input for this metadata laboratory; no real current GPU
    //capability is inferred from this test. Native path matrix proves both inputs.
    let object = resources
        .model_5a8520(&catalog, &bank, &model, &mut SourcePolicy)
        .unwrap();
    assert_eq!(resources.flags_08(&object).unwrap(), 0x400);
    assert_eq!(resources.model_radius_64(&object).unwrap(), 0x3f800000);
    resources.aspect_model_flags(&object, true, false).unwrap();
    assert_eq!(resources.flags_08(&object).unwrap(), 0x80400);
    resources.vfx_model_flags(&object, false).unwrap();
    assert_eq!(resources.flags_08(&object).unwrap(), 0x80000);
    resources
        .set_load_for_reading_5a55c0(&object, true)
        .unwrap();
    assert_eq!(resources.flags_08(&object).unwrap(), 0x180000);
    struct Missing;
    impl crate::resource_task::Services for Missing {}
    impl crate::resource_depot::Backend for Missing {}
    assert!(
        resources
            .request_10(&texture, 1, 0, &mut Missing)
            .unwrap_err()
            .contains("required native service")
    );
    assert_eq!(resources.requests_04(&texture).unwrap(), 1);
    assert_eq!(resources.flags_08(&texture).unwrap() & 7, 0);
    let input = resources.read_source(&texture).unwrap();
    assert_eq!(
        input.logical_path,
        texture
            .source_file()
            .expect("actual physical source")
            .logical_path()
    );
    assert_eq!(aa_rules::sources::sha256(&input.bytes), input.sha256);
    assert!(
        resources
            .texture_5a77e0(
                &catalog,
                &catalog.definition_bank.borrow(),
                alias.units(),
                0
            )
            .is_err()
    );
    drop(resources);
    drop(app);
    assert!(texture.application.upgrade().is_none());
}
