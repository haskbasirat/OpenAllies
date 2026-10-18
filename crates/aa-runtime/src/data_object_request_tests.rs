use super::*;

#[test]
#[ignore = "requires actual V14 sources; bounded pre-World source/Bank ownership, no GPU or World readiness"]
fn actual_pre_world_source_keeps_catalog_bank_and_recursive_identity() {
    use crate::{
        combat_source_receipt::VerifiedStartupReceipt,
        load_view_variables::NativeOverrides,
    };
    use aa_rules::{
        actors::ActorCatalog,
        resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits { max_member_bytes: u64::MAX },
    ).unwrap();
    let actors = ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    catalog.bind_resource_catalog(Arc::new(ResourceCatalog::load_v14(&mut files).unwrap())).unwrap();
    let receipt = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    crate::combat_source_completion::install(
        &catalog, &mut files, receipt.context(), receipt.csv(), receipt.globals(),
    ).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();
    crate::replacement_flags_source::complete_source_suffix(&catalog).unwrap();
    let pc = catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let farm = catalog.resolve(&"weurope_farm6x8_01".encode_utf16().collect::<Vec<_>>(), pc).unwrap();
    let mut application = crate::resource_depot::Application::from_sources(files);
    application.install_fresh_cache_variable_scope(&catalog, NativeOverrides::ControlledNone).unwrap();

    //The product source pass holds this immutable loan while a portrait Find
    //and an uncached Actor binding recurse. Keep the actual Bank IN Catalog;
    //a detached Bank or a second empty Bank would miss that ownership case.
    let bank = catalog.definition_bank.borrow();
    let bank_address = &*bank as *const Bank;
    assert!(catalog.definition_bank.try_borrow_mut().is_err());
    let actor = catalog.resolve(&"regiment_ger_infantry".encode_utf16().collect::<Vec<_>>(), pc).unwrap();
    assert_eq!(actor.proof.category.native_kind, 2);
    assert!(actor.component_mask.is_some());
    let portrait = catalog.resolve(&"usa_infantry_co_portrait".encode_utf16().collect::<Vec<_>>(), pc).unwrap();
    assert_eq!(portrait.proof.category.native_kind, 1);
    assert_eq!(portrait.native_class, 0);

    //This explicit laboratory renderer predicate is the SAME qualified seam
    //as the historical source-only test below, not a current UI/GPU policy.
    //Texture's positive missing/current-policy paths remain required.
    struct LabRenderer;
    impl ModelPolicy for LabRenderer {
        fn use_hardware_model_73a560(&mut self) -> Result<bool> { Ok(false) }
    }
    struct RequiredTexture;
    impl TexturePolicy for RequiredTexture {}
    application.with_data_pre_request_resources(|state, resources| {
        let data = state.prepare_object_requests(
            &catalog, &bank, &farm, resources, &mut LabRenderer, &mut RequiredTexture,
        )?;
        let portrait_data = state.prepare_object_requests(
            &catalog, &bank, &portrait, resources, &mut LabRenderer, &mut RequiredTexture,
        )?;
        assert_eq!(state.count_word(&data)?, 0);
        assert_eq!(state.count_word(&portrait_data)?, 0);
        assert_eq!(state.aspect_request_count(&data)?, 1);
        let aspect = state.object_aspect(&catalog, &bank, &farm, 0)?;
        assert!(f32::from_bits(state.aspect_visual_radius_64(&aspect)?).is_finite());
        let same = state.prepare_object_requests(
            &catalog, &bank, &farm, resources, &mut LabRenderer, &mut RequiredTexture,
        )?;
        assert!(same.same_allocation(&data));
        let held = catalog.definition_bank.try_borrow().map_err(|e| e.to_string())?;
        assert_eq!(&*held as *const Bank, bank_address);
        assert!(catalog.definition_bank.try_borrow_mut().is_err());
        println!("actual source same unmoved Bank: {} + {} + uncached {}; Data counts remain0; no World/GPU readiness", farm.proof.id, portrait.proof.id, actor.proof.id);
        Ok(())
    }).unwrap();
}

#[test]
#[ignore = "requires actual V14 sources/completed Bank; bounded source-only lab, not initialized World"]
fn actual_shared_dispatch_source_and_required_backend_prefix() {
    use crate::{
        combat_source_receipt::VerifiedStartupReceipt, load_view_variables::NativeOverrides,
    };
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
    let farm = catalog
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
    let mut app = crate::resource_depot::Application::from_sources(files);
    app.install_fresh_cache_variable_scope(&catalog,NativeOverrides::ControlledNone).unwrap();
    //This source-bound constructor reads the actual Bank before its move;
    //all subsequent prepared/request fields borrow that SAME moved owner.
    let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    struct ExplicitFalse;
    impl ModelPolicy for ExplicitFalse {
        fn use_hardware_model_73a560(&mut self) -> Result<bool> {
            Ok(false)
        }
    }
    struct RequiredTexture;
    impl TexturePolicy for RequiredTexture {}
    struct RequiredBackend;
    impl crate::resource_task::Services for RequiredBackend {}
    impl Backend for RequiredBackend {}
    app.with_data_pre_request_resources(|state,resources|{
  let mut policy=ExplicitFalse;
  //These resource source fields are produced before any GMap/World biome
  //replacement. The false renderer predicate is an explicit laboratory seam.
  let data=state.prepare_object_requests(&catalog,&bank,&farm,resources,&mut policy,&mut RequiredTexture)?;
  let portrait_data=state.prepare_object_requests(&catalog,&bank,&portrait,resources,&mut policy,&mut RequiredTexture)?;
  assert_eq!(portrait.native_class,0);assert_eq!(portrait.proof.category.registry_index,26);
  assert_eq!(state.count_word(&data)?,0);assert_eq!(state.count_word(&portrait_data)?,0);
  let graph=state.ready_object(&data)?.graph.as_ref().unwrap().clone();
  assert_eq!(graph.kind(),fx_graph::GraphKind::View(ListKind::Constructed));
  let lists=ListKind::ALL.map(|kind|graph.view_list_roots(kind).unwrap().len());
  assert!(lists[ListKind::Constructed as usize]>=5);
  let sound_cap=(0..graph.nodes().len()).find_map(|i|{
   let id=graph.node_id(i).unwrap();
   if graph.node_kind(id)!=Some(fx_graph::FxcKind::PlaySfx){return None}
   graph.registered_reference(id,0x44).unwrap().filter(|cap|cap.reference().id.eq_ignore_ascii_case("city_building_select"))
  }).expect("actual Farm selected source Audio city_building_select");
  let audio_lists=state.audio_data.prepared_sound_lists(&catalog,&bank,&sound_cap)?;
  assert_eq!(audio_lists.map(<[Handle]>::len),[0,1,0]);
  let sound=audio_lists[1][0].clone();
  assert_eq!(String::from_utf16(sound.logical_ids()).unwrap().replace('\\',"/"),"Audio/Sounds/General/CallOuts/CityBuildingSelect.wav");
  assert_eq!(resources.sound_bytes_40_42(&sound)?,[1,0,0]);
  let sound_refs=resources.references_14(&sound)?;
  state.audio_data.prepare(&catalog,&bank,&sound_cap,resources)?;
  let again=state.audio_data.prepared_sound_lists(&catalog,&bank,&sound_cap)?;
  assert!(again[1][0].same_identity(&sound));assert_eq!(resources.references_14(&sound)?,sound_refs);
  let same=state.prepare_object_requests(&catalog,&bank,&farm,resources,&mut policy,&mut RequiredTexture)?;
  assert!(same.same_allocation(&data));
  let biome=Biome::from_world_init_creator(&catalog,&bank,&creator)?;
  let aspect=state.object_aspect(&catalog,&bank,&farm,0)?;
  //Backend remains honestly required: no Task/decoder/loaded bit is supplied.
  let error=state.pre_request_object(&catalog,&bank,&farm,Some(&biome),resources,&mut policy,&mut RequiredBackend,1,0).unwrap_err();
  assert_eq!(state.count_word(&data)?,1);
  assert_eq!(state.aspect_request_count(&data)?,1);
  assert!(state.aspect_current_model(&data,0).is_ok());
  //46AEC0 stores Aspect18 only after every requested child returns.
  assert_eq!(state.aspect_count_word(&aspect)?,0);
  println!("actual selectedObject {} source{} lists={lists:?} Data20=1 Aspect18=0 backendboundary={error}",farm.proof.id,pc as u32);
  assert!(state.pre_request_object(&catalog,&Bank::default(),&farm,Some(&biome),resources,&mut policy,&mut RequiredBackend,1,0).is_err());
  assert_eq!(state.count_word(&data)?,1);
  Ok(())
 }).unwrap();
}
