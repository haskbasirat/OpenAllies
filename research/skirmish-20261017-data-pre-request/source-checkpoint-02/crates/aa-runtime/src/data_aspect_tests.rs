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
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    //The Biome source comes from the real Creator constructor. This lab does
    //not claim the whole GMap Init or an initialized World/renderer owner.
    let biome = Biome::from_world_init_creator(&catalog, &bank, &creator).unwrap();
    let mut app = crate::resource_depot::Application::from_sources(files);
    struct ExplicitFalsePolicy;
    impl ModelPolicy for ExplicitFalsePolicy {
        fn use_hardware_model_73a560(&mut self) -> Result<bool> {
            Ok(false)
        }
    }
    app.with_data_pre_request_resources(|state,resources|{
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
        println!("actual {} Aspect0 {:?},radius64={radius:08x}; explicit false renderer predicate, no decoded model/View readiness",d.proof.id,view.source_file().logical_path());
        Ok(())
    }).unwrap();
}
