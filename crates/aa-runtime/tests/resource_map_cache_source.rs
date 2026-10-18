//!Actual accepted archive/overlay maps through one retained application depot.
//!No native globalResourceManager worker or loaded battle capability is minted.
#[test]
#[ignore = "requires AA_GAME_ROOT original archive and accepted v14 source receipt"]
fn actual_maps_reuse_same_cached_resource_and_open_independent_shared_views() {
    use aa_rules::{
        actors::ActorCatalog,
        sources::{SourceLimits, V14Sources},
        tgi::Limits,
    };
    use aa_runtime::{
        catalog::Catalog, combat_source_completion as complete,
        combat_source_receipt::VerifiedStartupReceipt, resource_depot::Application,
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
        Some(aa_runtime::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
    let text=aa_runtime::localization::Localization::load_v14(&mut files,Limits::default()).unwrap();
    catalog.definition_text=Some(text.clone());
    catalog.replace_current_text(text);
    let proof = VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
    complete::install(
        &catalog,
        &mut files,
        proof.context(),
        proof.csv(),
        proof.globals(),
    )
    .unwrap();
    complete::compute(&catalog).unwrap();
    let process=aa_runtime::film_process_metadata::State::from_fresh_sources(&catalog,&mut files,aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let mut depot = Application::from_sources(files);
    if let Err((_,error))=depot.attach_process_metadata(process){panic!("{error}")}
    assert_eq!(depot.process_metadata().unwrap().constructed_headers(),0);
    let mut headers=0;
    for name in [
        "Maps/Campaign/tutorial02.RMP",
        "Maps/(2v2)Berlin.RMP",
        "Maps/DeveloperMaps/GER_Art_Map.RMP",
    ] {
        let ids: Vec<u16> = name.encode_utf16().collect();
        let selected = depot.select_map(&catalog, &ids).unwrap();
        headers+=1;assert_eq!(depot.process_metadata().unwrap().constructed_headers(),headers);
        assert_eq!(depot.map_references_14(&ids).unwrap(), 0);
        let query = name
            .to_ascii_uppercase()
            .replace('/', "\\")
            .encode_utf16()
            .collect::<Vec<_>>();
        let found = depot.find_scenario(&catalog, &query).unwrap();
        assert!(std::sync::Arc::ptr_eq(&selected, &found));
        assert_eq!(depot.map_references_14(&ids).unwrap(), 1);
        let first = depot.open_map_streams(&selected).unwrap();
        let second = depot.open_map_streams(&found).unwrap();
        assert!(std::ptr::eq(
            first.archive().unwrap(),
            second.archive().unwrap()
        ));
        let different_resource =
            aa_runtime::resource_creator::SourceResource::load(&catalog, &first, selected.ids())
                .unwrap();
        assert!(depot.open_map_streams(&different_resource).is_err());
        drop(first);
        assert!(second.archive().is_ok());
        drop(second);
        let again = depot.select_map(&catalog, &ids).unwrap();
        assert!(std::sync::Arc::ptr_eq(&selected, &again));
        assert_eq!(depot.map_references_14(&ids).unwrap(), 2);
        assert_eq!(depot.process_metadata().unwrap().constructed_headers(),headers);
    }
    assert_eq!(depot.map_names().count(), 3);
}
