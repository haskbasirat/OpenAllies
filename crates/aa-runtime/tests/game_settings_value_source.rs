//! Accepted source/Header90 -> distinct current Session68 -> selected Creator.
//! This fixture supplies the real map key, not the unported UI Find protocol.
#[test]
#[ignore = "requires AA_GAME_ROOT accepted native source, original maps and startup receipt"]
fn game_settings_current_owner_copies_source_header_and_survives_creator_writes() {
    use aa_rules::sources::{SourceLimits, V14Sources};
    use aa_runtime::{
        catalog::Catalog,
        creator_variables,
        game_settings_value::Owner,
        resource_creator::{Creator, SourceResource},
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
    let actors = aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
    let mut catalog = Catalog::new(&actors);
    catalog.settings = Some(
        aa_runtime::settings::Settings::load_v14(&mut files, aa_rules::tgi::Limits::default())
            .unwrap(),
    );
    let text=aa_runtime::localization::Localization::load_v14(
        &mut files,aa_rules::tgi::Limits::default()).unwrap();
    catalog.definition_text=Some(text.clone());
    catalog.replace_current_text(text);
    let receipt =
        aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(
            &catalog, &mut files,
        )
        .unwrap();
    aa_runtime::combat_source_completion::install(
        &catalog,
        &mut files,
        receipt.context(),
        receipt.csv(),
        receipt.globals(),
    )
    .unwrap();
    aa_runtime::combat_source_completion::compute(&catalog).unwrap();
    let variables = creator_variables::State::from_fresh_sources(
        &catalog,
        &mut files,
        aa_runtime::load_view_variables::NativeOverrides::ControlledNone,
    )
    .unwrap();
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
        let streams = aa_runtime::map_resource_streams::Owner::open(map);
        let resource = SourceResource::load(
            &catalog,
            &streams,
            &format!("Maps/{name}.RMP")
                .encode_utf16()
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let bank = catalog.definition_bank.borrow();
        let current = Owner::copy_source_header_5a46d0(&catalog, &bank, &resource).unwrap();
        current.validate(&catalog, &bank).unwrap();
        assert_eq!(current.parameters(), &resource.header().parameters);
        assert_eq!(
            current.rules().to_serialized(),
            resource.header_rules().to_serialized()
        );
        assert_eq!(current.words_0c_10(), resource.header().words_0c_10);
        assert_eq!(current.words_18_1c(), resource.header().words_18_1c);
        assert_eq!(current.string_38(), resource.header().string_38);
        assert_eq!(current.string_3c(), resource.header().string_3c);
        assert_eq!(
            current.display_extents_bits().unwrap(),
            resource.parsed_creator().extents_bits()
        );
        assert!(current.word_14().is_err());
        assert!(current.name_pair().is_err());
        assert!(current.description_pair().is_err());
        if !current.parameters().kingdoms.is_empty() {
            assert_ne!(
                current.parameters().kingdoms.as_ptr(),
                resource.header().parameters.kingdoms.as_ptr()
            );
        }
        let mut selected = Creator::from_load_constructor(&catalog, &variables).unwrap();
        selected
            .assign_current_game_settings(&catalog, &bank, &current)
            .unwrap();
        assert_eq!(selected.parameters(), current.parameters());
        assert_eq!(
            selected.rules().to_serialized(),
            current.rules().to_serialized()
        );
        assert!(selected.source_resource().is_none());
        assert_eq!(selected.extents_bits(), [192f32.to_bits(); 2]);
        if !current.parameters().kingdoms.is_empty() {
            assert_ne!(
                selected.parameters().kingdoms.as_ptr(),
                current.parameters().kingdoms.as_ptr()
            );
        }
        let before = current.parameters().clone();
        // Actual later Creator name/glyph stores must not mutate Session68.
        if let Some(kingdom) = selected.parameters().kingdoms.first() {
            let id = kingdom.id.clone();
            let mut text = aa_runtime::localization::Localization::load_v14(
                &mut files,
                aa_rules::tgi::Limits::default(),
            )
            .unwrap();
            let old_companion = current
                .labels()
                .kingdom_pair(current.parameters(), 0)
                .map(|v| v.text().to_vec());
            let outcome =
                aa_runtime::creator_labels::set_name(&mut selected, &id, &[88], &mut text);
            //49AE00 commits the pair companion before any required missing
            //TextManager fallback. Verify that real committed store either way.
            assert_eq!(
                selected
                    .labels()
                    .kingdom_pair(selected.parameters(), 0)
                    .unwrap()
                    .text(),
                [88]
            );
            if let Ok(handle) = outcome {
                aa_runtime::creator_labels::set_glyph(&mut selected, &handle, &[89]).unwrap();
            }
            assert_eq!(
                current
                    .labels()
                    .kingdom_pair(current.parameters(), 0)
                    .map(|v| v.text().to_vec()),
                old_companion
            );
        }
        assert_eq!(current.parameters(), &before);
        assert_eq!(current.parameters(), &resource.header().parameters);
    }
}
