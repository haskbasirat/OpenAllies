use super::*;

#[test]
fn native_ordinary_constructor_fields_before_source_projection() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-data-pre-request/native-aspect-constructor.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 9);
    for case in fixture["rows"].as_array().unwrap() {
        let row = Row::constructed();
        let n = &case["output"];
        assert_eq!(row.count, n["18"]);
        assert_eq!(row.extra_count, n["4c"]);
        assert_eq!(row.extra_requested, n["54"] == 1);
        assert_eq!(row.extras.data.is_none(), n["48"] == 0);
        assert_eq!(row.extras.capacity, n["50"]);
        assert_eq!(row.model.is_none(), n["28"] == 0);
        assert_eq!(row.biome.is_none(), n["24"] == 0);
        assert_eq!(row.skins[0].is_none(), n["30"] == 0);
        assert_eq!(row.skins[1].is_none(), n["34"] == 0);
        assert_eq!(row.animations.is_none(), n["38"] == 0);
        assert_eq!(n["1c"], 1);
        assert_eq!(n["68"], 0);
        assert!(row.input.is_none() && !row.source_ready);
    }
}

#[test]
#[ignore = "requires AA_GAME_ROOT actual sources/completed Bank; source-only lab, not initialized VWorld"]
fn actual_source_aspect_retains_model_biome_and_second_view_factory_allocation() {
    use crate::{
        combat_source_receipt::VerifiedStartupReceipt, load_view_variables::NativeOverrides,
    };
    use aa_mapworld::restore::RestoreDefinitions;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
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
    let variables = crate::creator_variables::State::from_fresh_sources(
        &catalog,
        &mut files,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let creator =
        crate::resource_creator::Creator::from_load_constructor(&catalog, &variables).unwrap();
    let pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let d = catalog
        .resolve(&"weurope_farm6x8_01".encode_utf16().collect::<Vec<_>>(), pc)
        .unwrap();
    let portrait = catalog
        .resolve(
            &"usa_infantry_co_portrait"
                .encode_utf16()
                .collect::<Vec<_>>(),
            pc,
        )
        .unwrap();
    assert_eq!(portrait.native_class, 0);
    assert_eq!(portrait.proof.category.native_kind, 1);
    let mut app = crate::resource_depot::Application::from_sources(files);
    //59CB00 Model class creation reads the actual fresh cache-variable scope
    //(installed from the SAME completed Bank before it is moved out).
    app.install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone)
        .unwrap();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    //The Biome source comes from the real Creator constructor. This lab does
    //not claim the whole GMap Init or an initialized World/renderer owner.
    let biome = Biome::from_world_init_creator(&catalog, &bank, &creator).unwrap();
    struct ExplicitFalsePolicy;
    impl ModelPolicy for ExplicitFalsePolicy {
        fn use_hardware_model_73a560(&mut self) -> Result<bool> {
            Ok(false)
        }
    }
    app.with_data_pre_request_resources(|state,resources|{
        //Source-only ObjectData has no runtime Thing class. Bind its actual
        //descriptor without manufacturing a class2 LoadShell/instance.
        let portrait_data=state.bind_object(&catalog,&bank,&portrait)?;
        assert_eq!(state.count_word(&portrait_data)?,0);
        state.prepare_object_aspects(&catalog,&bank,&d,resources)?;
        let aspect=state.object_aspect(&catalog,&bank,&d,0)?;
        let radius=state.aspect_visual_radius_64(&aspect)?;
        let mut policy=ExplicitFalsePolicy;
        let first=state.resolve_aspect_model(&catalog,&bank,&aspect,Some(&biome),resources,&mut policy)?;
        let count=resources.references_14(&first)?;
        let same=state.resolve_aspect_model(&catalog,&bank,&aspect,Some(&biome),resources,&mut policy)?;
        assert!(first.same_identity(&same));
        assert_eq!(resources.references_14(&first)?,count);
        let view=state.model_for_view454f70(&catalog,&bank,&aspect,Some(&biome),resources,&mut policy)?;
        assert!(view.same_identity(&first));
        assert!(resources.references_14(&first)?>count);
        assert_eq!(resources.flags_08(&first)?&1,0);
        assert_eq!(state.count_word(&aspect.data)?,0);
        assert_eq!(state.aspect_row(&aspect)?.count,0);
        assert!(State::from_native_constructor().aspect_visual_radius_64(&aspect).is_err());
        assert!(state.resolve_aspect_model(&catalog,&Bank::default(),&aspect,Some(&biome),resources,&mut policy).is_err());
        println!("actual {} Aspect0 {:?},radius64={radius:08x}; explicit false renderer predicate, no decoded model/View readiness",d.proof.id,view.source_file().expect("actual physical source").logical_path());
        Ok(())
    }).unwrap();
}

#[test]
fn positive_46a180_empty_name_is_refused_before_5a79f0() {
    //Actual46A8A0/46A180: a rename that empties a positive skin still calls
    //5A79F0 with an empty name (fixture "rename empties skin").
    let doc: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/mapload-20261018/aspect-texture-rename/native-aspect-texture-rename.json"
    ))
    .unwrap();
    let case = &doc["loader"][4];
    assert_eq!(case["input"]["definition"], "rename empties skin");
    assert_eq!(case["native"]["textures"][0]["name"], "");
    assert_ne!(case["native"]["skin_30"], 0);
    assert!(texture_request_name(&Some(String::new())).is_err());
    assert_eq!(texture_request_name(&None).unwrap(), None);
    assert_eq!(
        texture_request_name(&Some("a/b.dds".into())).unwrap(),
        Some("a/b.dds".encode_utf16().collect())
    );
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources and completed Bank; source-only lab, no View/World readiness"]
fn actual_gbr_brigade_engineer_texture_rename_source_aspects() {
    use crate::load_view_variables::NativeOverrides;
    use aa_mapworld::restore::RestoreDefinitions;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog, &mut files,
    )
    .unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let definition = catalog
        .resolve(&"gbr_brigade_engineer".encode_utf16().collect::<Vec<_>>(), pc)
        .unwrap();
    let mut application = crate::resource_depot::Application::from_sources(files);
    application
        .install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone)
        .unwrap();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    application
        .with_data_pre_request_resources(|state, resources| {
            let result = state.prepare_object_aspects(&catalog, &bank, &definition, resources);
            let data = state.bind_object(&catalog, &bank, &definition)?;
            let object = state.entry(&data)?.aspects.as_ref().unwrap();
            //AspectNormal is source row 0 whether or not a later child stops.
            let normal = object
                .rows
                .first()
                .or(object.pending.as_ref())
                .and_then(|r| r.input.as_ref())
                .ok_or("gbr_brigade_engineer AspectNormal source fields absent")?;
            assert_eq!(normal.folder, "Buildings/GBR");
            assert_eq!(
                normal.renames,
                TextureRenames::from_source_order(vec![TextureRename {
                    from: "DeathArcticBase".encode_utf16().collect(),
                    to: "Base".encode_utf16().collect(),
                }])
            );
            assert_eq!(
                normal.skin_paths,
                [
                    Some("Buildings/GBR/GBRBrigadeEngineer/GBRBrigadeEngineerBase.dds".into()),
                    None
                ]
            );
            //46AE20 flag0 on the actual NIF texture names leaves them unchanged.
            assert_eq!(
                normal
                    .texture_name_46a180("GBRBrigadeEngineerGround.dds", false, &normal.folder)?
                    .unwrap(),
                "Buildings/GBR/GBRBrigadeEngineerGround.dds"
            );
            println!(
                "actual gbr_brigade_engineer: {} prepared rows, pending={}, result={result:?}",
                object.rows.len(),
                object.pending.is_some()
            );
            result?;
            assert!(object.complete);
            Ok(())
        })
        .unwrap();
}

#[test]
fn native_banner_constructor_and_ground_size_copy_fields_match() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-data-pre-request/native-banner-aspect.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 54);
    for row in fixture["rows"].as_array().unwrap() {
        let n = &row["output"];
        let mut actual = Row::constructed_banner();
        let initial = &n["initial"];
        assert_eq!(actual.count, initial["18"]);
        assert_eq!(actual.extra_count, initial["4c"]);
        assert_eq!(u8::from(actual.extra_requested), initial["54"]);
        assert_eq!(actual.extras.data.is_none(), initial["48"] == 0);
        assert_eq!(actual.extras.capacity, initial["50"]);
        assert_eq!(u8::from(actual.context_sensitive), initial["1c"]);
        assert_eq!(u8::from(actual.unique), initial["68"]);
        assert_eq!(
            actual.ground_size,
            [
                initial["58"].as_u64().unwrap() as u32,
                initial["5c"].as_u64().unwrap() as u32
            ]
        );
        assert_eq!(actual.ground_radius, initial["60"]);
        assert_eq!(initial["64"], 0);
        assert_eq!(initial["34"], 0);
        assert_eq!(initial["38"], 0);
        let size: [u32; 2] = serde_json::from_value(row["size"].clone()).unwrap();
        let gate = f32::from_bits(size[0]) > 0.0 || f32::from_bits(size[1]) > 0.0;
        assert_eq!(u8::from(gate), n["gate"]);
        if gate {
            let pc = match row["precision"].as_str().unwrap() {
                "Single24" => Precision::Single24,
                "Double53" => Precision::Double53,
                "Extended64" => Precision::Extended64,
                _ => panic!(),
            };
            store_size(&mut actual, size, pc).unwrap();
        }
        assert_eq!(serde_json::json!(actual.ground_size), n["size"]);
        assert_eq!(actual.ground_radius, n["radius"]);
    }
}

#[test]
#[ignore = "requires original/v14 source and completed Bank; source-only Banner lab, no View readiness"]
fn actual_regiment_ger_banner_replacement_retains_one_source_and_texture_owner() {
    use crate::load_view_variables::NativeOverrides;
    use aa_mapworld::restore::RestoreDefinitions;
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings =
        Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let receipt = crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
        &catalog, &mut files,
    )
    .unwrap();
    crate::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    let pc = catalog
        .definition_bank
        .borrow()
        .combat_source()
        .unwrap()
        .source_precision();
    let definition = catalog
        .resolve(&"regiment_ger_infantry".encode_utf16().collect::<Vec<_>>(), pc)
        .unwrap();
    let mut application = crate::resource_depot::Application::from_sources(files);
    application
        .install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone)
        .unwrap();
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    application.with_data_pre_request_resources(|state,resources|{
        state.prepare_object_aspects(&catalog,&bank,&definition,resources)?;
        let data=state.bind_object(&catalog,&bank,&definition)?;
        let object=state.entry(&data)?.aspects.as_ref().unwrap();assert!(object.complete&&object.pending.is_none());assert_eq!(object.rows.len(),1);
        let row=&object.rows[0];assert!(!row.context_sensitive&&row.unique);assert_eq!(row.input.as_ref().unwrap().visual_radius,0);
        assert!(row.model.is_none()&&row.animations.is_none());assert_eq!(row.count,0);
        assert!(state.entry(&data)?.banner.is_some());
        let aspect=state.object_aspect(&catalog,&bank,&definition,0)?;assert!(!state.aspect_ids(&aspect)?.is_empty());
        let count=state.aspect_request_count(&data)?;
        state.prepare_object_aspects(&catalog,&bank,&definition,resources)?;assert_eq!(state.aspect_request_count(&data)?,count);
        assert_eq!(state.count_word(&data)?,0);
        assert!(state.object_aspect(&catalog,&Bank::default(),&definition,0).is_err());
        println!("actual regiment_ger_infantry Banner Aspect uses canonical source model/skin/Plane/Decal allocations; no model load or VWorld claim");Ok(())
    }).unwrap();
}
