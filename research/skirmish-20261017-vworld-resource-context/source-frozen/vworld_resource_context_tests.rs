use super::*;
use aa_numeric::Precision;

#[test]
fn original_v14_counted_assignment_and_required_invalidation_prefixes() {
    let native: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-vworld-resource-context/native-context.json"
    ))
    .unwrap();
    for row in native["cases"].as_array().unwrap() {
        let current: Vec<u16> = serde_json::from_value(row["input"]["current"].clone()).unwrap();
        let next: Vec<u16> = serde_json::from_value(row["input"]["next"].clone()).unwrap();
        let expected: Vec<u16> = serde_json::from_value(row["output"]["current"].clone()).unwrap();
        let mut state = State { current };
        let mut reached = false;
        let result = state.assign_and_invalidate(&next, || {
            reached = true;
            //Ni eligibility remains a native-only boundary here, never a
            //ported successful unload. Supply precisely that reached error.
            if let Some(stop) = row["output"]["stop"].as_str() {
                return Err(stop.into());
            }
            let examined = row["output"]["examined"].as_array().unwrap();
            Ok((
                examined.iter().filter(|v| **v == "Map").count(),
                examined.iter().filter(|v| **v == "Film").count(),
            ))
        });
        let changed = row["output"]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| *v == "assign_counted_string");
        assert_eq!(state.current(), expected);
        assert_eq!(reached, changed);
        if let Some(stop) = row["output"]["stop"].as_str() {
            assert_eq!(result.unwrap_err(), stop);
        } else {
            assert_eq!(result.unwrap().changed, changed);
        }
    }
    for row in native["constructor_rows"].as_array().unwrap() {
        let expected: Vec<u16> = serde_json::from_value(row["output"]["current"].clone()).unwrap();
        assert_eq!(State::constructed().current(), expected);
    }
}

#[test]
fn case_sensitive_counted_assignment_retains_error_prefix() {
    let mut state = State::constructed();
    assert!(
        !state
            .assign_and_invalidate(&[], || panic!("equal prefix must skip invalidation"))
            .unwrap()
            .changed
    );
    assert!(state
        .assign_and_invalidate(&[65, 0, 88], || Err("required unload".into()))
        .is_err());
    assert_eq!(state.current(), &[65, 0, 88]);
    assert!(
        !state
            .assign_and_invalidate(&[65, 0, 89], || panic!("equal prefix"))
            .unwrap()
            .changed
    );
    assert_eq!(state.current(), &[65, 0, 88]);
    assert_eq!(
        state.assign_and_invalidate(&[97], || Ok((2, 1))).unwrap(),
        Update {
            changed: true,
            maps_examined: 2,
            films_examined: 1
        }
    );
}

#[test]
#[ignore = "requires actual AA_GAME_ROOT source completion and real map/film cache"]
fn actual_retained_biomes_and_nonempty_same_application_namespace() {
    use crate::{film_create::Depot, load_view_variables::NativeOverrides};
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
    let text = crate::localization::Localization::load_v14(&mut files, Limits::default()).unwrap();
    catalog.definition_text = Some(text.clone());
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
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    //Bounded genuine field constructors as in the accepted DataText backend
    //test. This is not a5A26E0 managed capture or a fresh whole application.
    let creator = Creator::from_load_constructor(&catalog, &variables).unwrap();
    let mut process = crate::film_process_metadata::State::from_fresh_sources(
        &catalog,
        &mut files,
        NativeOverrides::ControlledNone,
    )
    .unwrap();
    let header = crate::film_header_owner::State::from_film_constructor(
        &catalog,
        &catalog.definition_bank.borrow(),
        &mut process,
        || Ok([0x12345678, 0x9abcdef0]),
    )
    .unwrap();
    let ids: Vec<u16> = format!("VWorldContext-{}.RFM", std::process::id())
        .encode_utf16()
        .collect();
    let mut film = crate::film_resource::Resource::from_constructed_fields(ids, header, creator);
    let mut app = crate::resource_depot::Application::from_sources(files);
    let work = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261017-vworld-resource-context/work");
    std::fs::create_dir_all(&work).unwrap();
    app.configure_portable_work_depot(&work).unwrap();
    {
        let mut depot = app
            .film_depot(&catalog, NativeOverrides::ControlledNone)
            .unwrap();
        depot.ensure_work_depot(&mut film).unwrap();
        depot.publish(&mut film).unwrap();
    }
    let references = film.references_14().unwrap();
    let flags = film.base_flags_8().unwrap();
    assert!(app.vworld_texture_subfolder().unwrap().is_empty());
    let bank = catalog.definition_bank.borrow();
    let source_pc = bank.combat_source().unwrap().source_precision();
    assert_eq!(source_pc, Precision::Double53);
    let retained_default =
        Biome::from_world_init_creator(&catalog, &bank, film.metadata().unwrap().creator())
            .unwrap();
    let default = Input::bind(&catalog, &bank, &retained_default).unwrap();
    let outcome = app
        .set_vworld_resource_context(&catalog, &bank, &retained_default)
        .unwrap();
    assert_eq!(outcome.changed, !default.units.is_empty());
    if outcome.changed {
        assert_eq!((outcome.maps_examined, outcome.films_examined), (0, 1));
    }
    assert_eq!(app.vworld_texture_subfolder().unwrap(), default.units);
    assert_eq!(film.references_14().unwrap(), references);
    assert_eq!(film.base_flags_8().unwrap(), flags);
    drop(bank);
    let mut current_maps = 0;
    for name in [
        "Maps/Campaign/tutorial02.RMP",
        "Maps/DeveloperMaps/(3v3)Firefight.RMP",
        "Maps/(2v2)Berlin.RMP",
    ] {
        let map_ids: Vec<u16> = name.encode_utf16().collect();
        let map = app.select_map(&catalog, &map_ids).unwrap();
        current_maps += 1;
        let bank = catalog.definition_bank.borrow();
        let retained =
            Biome::from_world_init_creator(&catalog, &bank, map.parsed_creator()).unwrap();
        let input = Input::bind(&catalog, &bank, &retained).unwrap();
        let biome = retained.node(&catalog, &bank).unwrap();
        let TypedValue::String { value, source, .. } = biome
            .field("texture_subfolder_name", source_pc)
            .unwrap()
            .unwrap()
        else {
            panic!("typed string")
        };
        assert_eq!(input.units, value.encode_utf16().collect::<Vec<_>>());
        assert!(!source.path.is_empty());
        let before = app.vworld_texture_subfolder().unwrap();
        let refs = app.map_references_14(&map_ids).unwrap();
        let update = app
            .set_vworld_resource_context(&catalog, &bank, &retained)
            .unwrap();
        assert_eq!(update.changed, c_string(&before) != c_string(&input.units));
        if name.contains("Firefight") {
            assert_eq!(input.units, "Tropical".encode_utf16().collect::<Vec<_>>());
            assert!(update.changed);
            //The selected Load constructor's temperate default is a DIFFERENT
            //pointer. The actual WorldInit capture must retain this parsed
            //Tropical registration across later resource+88 replacement.
            assert_ne!(input.units, default.units);
        }
        if update.changed {
            assert_eq!(
                (update.maps_examined, update.films_examined),
                (current_maps, 1)
            );
        }
        assert_eq!(
            c_string(&app.vworld_texture_subfolder().unwrap()),
            c_string(&input.units)
        );
        assert_eq!(app.map_references_14(&map_ids).unwrap(), refs);
        let unchanged = app
            .set_vworld_resource_context(&catalog, &bank, &retained)
            .unwrap();
        assert!(!unchanged.changed);
        assert_eq!((unchanged.maps_examined, unchanged.films_examined), (0, 0));
        let value = app.vworld_texture_subfolder().unwrap();
        assert!(app
            .set_vworld_resource_context(&catalog, &Bank::default(), &retained)
            .is_err());
        assert_eq!(app.vworld_texture_subfolder().unwrap(), value);
    }
    //All actual native Biome registry entries use the typed string getter at
    //all PCs; this immutable source census is not a second Biome manager.
    let registry = actors.native_data_registry("Biome").unwrap();
    assert!(!registry.entries.is_empty());
    for entry in &registry.entries {
        let node = actors.native_data_node(entry).unwrap();
        let mut previous = None;
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let field = node.field("texture_subfolder_name", pc).unwrap().unwrap();
            let TypedValue::String { value, source, .. } = field else {
                panic!("typed string required")
            };
            assert!(actors.sources.iter().any(|s| s.sha256 == source.sha256));
            if let Some(expected) = &previous {
                assert_eq!(&value, expected);
            }
            previous = Some(value);
        }
    }
    assert_eq!(film.references_14().unwrap(), references);
    assert_eq!(film.base_flags_8().unwrap(), flags);
    println!("actual typed namespace: {current_maps} Maps +1 Film; {} canonical Biomes/all3PC; source {source_pc:?}", registry.entries.len());
}
