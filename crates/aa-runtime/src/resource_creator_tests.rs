use super::*;
#[test]
#[ignore = "requires actual AA_GAME_ROOT source, maps and sealed startup receipt"]
fn actual_source_creator_constructor_selection_and_resource_slot_are_distinct() {
    use aa_rules::sources::{SourceLimits, V14Sources};
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: u64::MAX,
        },
    )
    .unwrap();
    let actors = aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(
        crate::settings::Settings::load_v14(&mut files, aa_rules::tgi::Limits::default()).unwrap(),
    );
    let text=crate::localization::Localization::load_v14(
        &mut files,aa_rules::tgi::Limits::default()).unwrap();
    catalog.definition_text=Some(text.clone());
    catalog.replace_current_text(text);
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
    let variables = crate::creator_variables::State::from_fresh_sources(
        &catalog,
        &mut files,
        crate::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
    let process=crate::film_process_metadata::State::from_fresh_sources(
        &catalog,&mut files,crate::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let header_counter=process.constructor_counter();
    assert_eq!((variables.width(), variables.height()), (192, 192));
    assert_eq!(variables.independent_kingdoms(), 0);
    assert!(
        crate::creator_variables::State::from_fresh_sources(
            &catalog,
            &mut files,
            crate::load_view_variables::NativeOverrides::Unbound
        )
        .is_err()
    );
    let native: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-mission-start/native-creator-constructor.json"
    ))
    .unwrap();
    let copies: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-mission-start/native-creator-copy.json"
    ))
    .unwrap();
    assert_eq!(native["cases"].as_array().unwrap().len(), 24);
    assert_eq!(copies["cases"].as_array().unwrap().len(), 36);
    for name in [
        "Campaign/tutorial02",
        "DeveloperMaps/(3v3)Firefight",
        "(2v2)Berlin",
    ] {
        let map = aa_mapworld::MapArchive::from_bytes(
            std::fs::read(root.join(format!(
                "reconstruction/assets/original/Data/Maps/{name}.RMP"
            )))
            .unwrap(),
        )
        .unwrap();
        let mut streams = crate::map_resource_streams::Owner::open(map);
        let inventory =
            aa_mapworld::WorldInventory::decode(streams.archive().unwrap(), u64::MAX).unwrap();
        //This is an explicit resource-key fixture input, not a claim that the
        //native depot/scenario lookup protocol has been run by this test.
        let resource = SourceResource::load(
            &catalog,
            &streams,
            &format!("Maps/{name}.RMP")
                .encode_utf16()
                .collect::<Vec<_>>(),
        )
        .unwrap();
        if name == "Campaign/tutorial02" {
            let source_pc = catalog
                .definition_bank
                .borrow()
                .combat_source()
                .unwrap()
                .source_precision();
            let order = catalog.actors.native_resource_registry(source_pc).unwrap();
            let fixture: serde_json::Value = serde_json::from_str(include_str!(
                "../../../research/skirmish-20261017-mission-start/native-creator-resources.json"
            ))
            .unwrap();
            let mut checked = 0;
            for case in fixture["cases"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["input"]["count"].as_u64() == Some(order.ids.len() as u64))
            {
                let mut parameters = resource.header().parameters.clone();
                parameters.version = 13;
                let first = parameters.kingdoms.first_mut().unwrap();
                first.resources.clear();
                let mut supported = true;
                for row in case["input"]["rows"].as_array().unwrap() {
                    let ids = match row["index"]
                        .as_u64()
                        .and_then(|i| order.ids.get(i as usize))
                    {
                        Some(id) => id.clone(),
                        None => {
                            supported = false;
                            "explicit-unknown-resource-fixture".into()
                        }
                    };
                    first.resources.push((
                        ids.encode_utf16().collect(),
                        row["word"].as_u64().unwrap() as u32,
                    ));
                }
                let actual = materialize_parameters(&catalog, parameters);
                if supported {
                    let actual = actual.unwrap();
                    assert_eq!(
                        serde_json::json!(
                            actual.kingdoms[0]
                                .resources
                                .iter()
                                .map(|r| r.1)
                                .collect::<Vec<_>>()
                        ),
                        case["output"]
                    );
                } else {
                    assert!(
                        actual.is_err(),
                        "unbound native warning path must remain required"
                    );
                }
                checked += 1;
            }
            assert_eq!(checked, 30);
        }
        let mut selected = Creator::from_load_constructor(&catalog, &variables).unwrap();
        assert_eq!(selected.kind(), Kind::Load);
        assert!(selected.source_resource().is_none());
        assert!(selected.source().is_none());
        let (mut unselected, _) = crate::map_resource_load::Owner::fresh_loading_archive(
            1,
            &catalog.actors.sources,
            Some(streams.witness()),
        )
        .unwrap();
        unselected.bind_header_constructor_counter(&header_counter).unwrap();
        let error = unselected
            .load_creator_before_world(&catalog, streams.archive().unwrap(), Some(&selected))
            .unwrap_err();
        assert!(error.contains("5A2D09"));
        assert_eq!(unselected.creator().unwrap().kind(), Kind::Load);
        assert!(unselected.creator().unwrap().source_resource().is_none());
        assert!(
            unselected
                .validate_creator(&catalog, &catalog.definition_bank.borrow())
                .is_err()
        );
        assert_eq!(selected.extents_bits(), [192f32.to_bits(); 2]);
        assert_eq!(selected.rng_seed(), 0);
        assert_eq!(selected.parameters().kingdoms.len(), 2);
        assert!(selected.parameters().teams.is_empty());
        let resources = catalog
            .actors
            .native_resource_registry(
                catalog
                    .definition_bank
                    .borrow()
                    .combat_source()
                    .unwrap()
                    .source_precision(),
            )
            .unwrap();
        for kingdom in &selected.parameters().kingdoms {
            assert_eq!(
                (
                    kingdom.byte_10,
                    kingdom.byte_28,
                    kingdom.byte_29,
                    kingdom.byte_31,
                    kingdom.byte_32
                ),
                (0, 1, 1, 0, 1)
            );
            assert_eq!(kingdom.string_0c, Some(Vec::new()));
            assert_eq!(kingdom.resources.len(), resources.ids.len());
            assert_eq!(
                kingdom
                    .resources
                    .iter()
                    .map(|r| String::from_utf16(&r.0).unwrap())
                    .collect::<Vec<_>>(),
                resources.ids
            );
            assert!(kingdom.resources.iter().all(|(_, word)| *word == 0));
        }
        let first = catalog
            .actors
            .native_data_registry("GameType")
            .unwrap()
            .entries
            .first()
            .unwrap();
        assert_eq!(
            selected.rules().to_serialized().game_type,
            first.ids.encode_utf16().collect::<Vec<_>>()
        );
        let original_dimensions = selected.extents_bits();
        let original_biome = selected.base_payload().biome;
        {
            let bank = catalog.definition_bank.borrow();
            selected.validate(&catalog, &bank).unwrap();
            selected
                .assign_source_header_game_settings(&catalog, &bank, &resource)
                .unwrap();
            assert_eq!(selected.parameters(), &resource.header().parameters);
            assert_eq!(
                selected.rules().to_serialized(),
                resource.header_rules().to_serialized()
            );
            assert_eq!(selected.extents_bits(), original_dimensions);
            assert_eq!(selected.base_payload().biome, original_biome);
            assert!(selected.source_resource().is_none());
            selected
                .assign_source_resource(&catalog, &bank, Arc::clone(&resource))
                .unwrap();
        }
        assert!(Arc::ptr_eq(selected.source_resource().unwrap(), &resource));
        assert!(
            selected
                .parameters()
                .kingdoms
                .iter()
                .enumerate()
                .all(|(i, _)| !selected.parameter_modified(i).unwrap())
        );
        if !selected.parameters().kingdoms.is_empty() {
            if name == "Campaign/tutorial02" {
                let flag_fixture:serde_json::Value=serde_json::from_str(include_str!(
                    "../../../research/skirmish-20261017-mission-start/native-creator-parameter-flags.json")).unwrap();
                for row in flag_fixture["cases"].as_array().unwrap() {
                    let value = row["value"].as_u64().unwrap() as u8;
                    selected.human_assignment_fields_mut().1[0] = value;
                    let copied = selected
                        .clone_for_resource(&catalog, &catalog.definition_bank.borrow())
                        .unwrap();
                    assert_eq!(
                        copied.parameter_modified68[0],
                        (row["target_word"].as_u64().unwrap() & 255) as u8
                    );
                    selected.human_assignment_fields_mut().1[0] = value.wrapping_add(1);
                    assert_eq!(copied.parameter_modified68[0], value);
                }
            }
            selected.human_assignment_fields_mut().1[0] = 0x80;
            assert!(selected.parameter_modified(0).unwrap());
            assert_eq!(resource.header_modified68()[0], 0);
        }
        //Deliberately changed current selected values are copied at the real
        //4FC814 boundary; the source standalone RULES is a different owner.
        selected.rules.set_mode_from_current(0x87654321);
        selected.rules.set_changes_from_current(0xfedcba98);
        assert_eq!(resource.parsed_creator().kind(), Kind::Base);
        let failed_sources = catalog.actors.sources.clone();
        let (mut missing, _) = crate::map_resource_load::Owner::fresh_loading_archive(
            1,
            &failed_sources,
            Some(streams.witness()),
        )
        .unwrap();
        missing.bind_header_constructor_counter(&header_counter).unwrap();
        assert!(
            missing
                .load_creator_before_world(&catalog, streams.archive().unwrap(), None)
                .is_err()
        );
        assert_eq!(missing.creator().unwrap().kind(), Kind::Base);
        assert!(
            missing
                .validate_creator(&catalog, &catalog.definition_bank.borrow())
                .is_err()
        );
        let mut returned = None;
        for mode in [1, 2, 4] {
            catalog.native_load_mode = Some(mode);
            let (mut target, word) = crate::map_resource_load::Owner::fresh_loading_archive(
                mode,
                &catalog.actors.sources,
                Some(streams.witness()),
            )
            .unwrap();
            target.bind_header_constructor_counter(&header_counter).unwrap();
            let before_header_reset=header_counter.value();
            target
                .prepare_creator_header(&catalog, streams.archive().unwrap(), Some(&selected))
                .unwrap();
            assert_eq!(header_counter.value(),before_header_reset.wrapping_add(1));
            assert!(target.bind_header_constructor_counter(&header_counter).is_err());
            let other_selected = selected
                .clone_for_resource(&catalog, &catalog.definition_bank.borrow())
                .unwrap();
            assert!(
                target
                    .select_creator_before_world_load(&catalog, &other_selected)
                    .is_err()
            );
            assert!(
                target
                    .validate_creator(&catalog, &catalog.definition_bank.borrow())
                    .is_err()
            );
            target
                .select_creator_before_world_load(&catalog, &selected)
                .unwrap();
            target
                .validate_creator(&catalog, &catalog.definition_bank.borrow())
                .unwrap();
            let current = target.creator().unwrap();
            if mode == 1 {
                assert_eq!(current.kind(), Kind::Load);
                assert!(Arc::ptr_eq(current.source_resource().unwrap(), &resource));
                assert_eq!(current.parameters(), selected.parameters());
                assert_eq!(
                    current.rules().to_serialized(),
                    selected.rules().to_serialized()
                );
                if !current.parameters().kingdoms.is_empty() {
                    assert_ne!(
                        current.parameters().kingdoms.as_ptr(),
                        selected.parameters().kingdoms.as_ptr()
                    );
                }
                assert!(!std::ptr::eq(current, selected.as_ref()));
                for i in 0..selected.parameters().kingdoms.len() {
                    assert_eq!(
                        current.parameter_modified(i).unwrap(),
                        selected.parameter_modified(i).unwrap()
                    );
                }
            } else {
                assert_eq!(current.kind(), Kind::Base);
                assert_eq!(current.parameters(), resource.parsed_creator().parameters());
            }
            let loaded_rules =
                crate::load_world_rules::Owner::load_for_resource(&catalog, &inventory, &target)
                    .unwrap();
            if mode & 2 == 0 {
                assert_eq!(
                    loaded_rules.to_serialized(),
                    current.rules().to_serialized()
                );
                assert_eq!(loaded_rules.changes(), current.rules().changes());
            } else {
                let saved_reset =
                    crate::load_world_rules::Owner::load(&catalog, &inventory).unwrap();
                assert_eq!(loaded_rules.to_serialized(), saved_reset.to_serialized());
                assert_eq!(loaded_rules.changes(), 0);
            }
            if mode == 1 {
                returned = Some((target, word));
            }
        }
        let (mut target, mut word) = returned.unwrap();
        let kind=crate::map_resource_mode::LoadGameTaskKind::from_native_word(0);
        let task=crate::load_game_task::Inputs::from_selected_creator(
            &catalog,&catalog.definition_bank.borrow(),&selected,kind).unwrap();
        assert!(target.require_task_inputs(&task).is_err(),"unreturned resource is not a completed task");
        let foreign_selected=selected.clone_for_resource(&catalog,&catalog.definition_bank.borrow()).unwrap();
        let foreign_task=crate::load_game_task::Inputs::from_selected_creator(
            &catalog,&catalog.definition_bank.borrow(),&foreign_selected,kind).unwrap();
        let wrong_kind=crate::load_game_task::Inputs::from_selected_creator(
            &catalog,&catalog.definition_bank.borrow(),&selected,
            crate::map_resource_mode::LoadGameTaskKind::from_native_word(1)).unwrap();
        let fresh_source=SourceResource::load(&catalog,&streams,
            &format!("Maps/{name}.RMP").encode_utf16().collect::<Vec<_>>()).unwrap();
        assert!(!Arc::ptr_eq(&fresh_source,&resource));
        let mut other_source_selected=selected.clone_for_resource(&catalog,&catalog.definition_bank.borrow()).unwrap();
        other_source_selected.assign_source_resource(&catalog,&catalog.definition_bank.borrow(),fresh_source).unwrap();
        let other_source_task=crate::load_game_task::Inputs::from_selected_creator(
            &catalog,&catalog.definition_bank.borrow(),&other_source_selected,kind).unwrap();
        target.close_owned_streams(&mut streams).unwrap();
        target
            .finish_normal_return(word.as_mut(), &catalog.actors.sources)
            .unwrap();
        assert!(streams.is_closed());
        assert_eq!(*word, Some(0));
        assert_eq!(target.creator().unwrap().kind(), Kind::Load);
        target.require_selected_origin(&selected).unwrap();
        target.require_task_inputs(&task).unwrap();
        assert!(target.require_selected_origin(&foreign_selected).is_err());
        assert!(target.require_task_inputs(&foreign_task).is_err());
        assert!(target.require_task_inputs(&wrong_kind).is_err());
        assert!(target.require_task_inputs(&other_source_task).is_err());
        target.creator_mut_for_world_ready(&catalog,&catalog.definition_bank.borrow()).unwrap()
            .assign_source_resource(&catalog,&catalog.definition_bank.borrow(),
                Arc::clone(other_source_selected.source_resource().unwrap())).unwrap();
        assert!(target.require_task_inputs(&task).is_err(),"same selected witness must not bless a replaced current source allocation");
        assert!(target.require_selected_origin(&selected).is_err());
        target.creator_mut_for_world_ready(&catalog,&catalog.definition_bank.borrow()).unwrap()
            .assign_source_resource(&catalog,&catalog.definition_bank.borrow(),Arc::clone(&resource)).unwrap();
        target.require_task_inputs(&task).unwrap();
        target
            .validate_creator(&catalog, &catalog.definition_bank.borrow())
            .unwrap();
    }
}
