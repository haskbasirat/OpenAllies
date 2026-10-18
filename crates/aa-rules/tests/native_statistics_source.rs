use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};
use std::path::PathBuf;

fn file(name: &str, text: &str) -> ResolvedFile {
    ResolvedFile {
        logical_path: name.into(),
        origin: Origin::V14Loose,
        sha256: sha256(text.as_bytes()),
        bytes: text.as_bytes().to_vec(),
    }
}
fn fixture(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/skirmish-20261016-world-start")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!("precision"),
    }
}

#[test]
fn statistic_source_matches_original_parser_and_producer() {
    let data = fixture("native-statistics-source.json");
    let template = data["template"].as_str().unwrap();
    let resources = data["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| format!("[Resource]\n{{ string IDS = {}\n}}\n", id.as_str().unwrap()))
        .collect::<String>();
    for case in data["cases"].as_array().unwrap() {
        let mut report = NativeLoadReport::default();
        let catalog = load_files(
            vec![
                file("template.tgi", template),
                file("resources.tgi", &resources),
                file("scoring.tgi", case["source"].as_str().unwrap()),
            ],
            Limits::default(),
            &mut report,
        );
        let result =
            catalog.native_statistics_catalog(precision(case["precision"].as_str().unwrap()));
        if !case["result"]["thrown"].is_null() {
            assert!(
                result.is_err(),
                "{} accepted native rejection",
                case["name"]
            );
            continue;
        }
        let got = result
            .unwrap_or_else(|e| panic!("{} {}: {e}; {report:?}", case["name"], case["precision"]));
        assert_eq!(
            got.production_start as u64,
            case["result"]["production_start"]
        );
        assert_eq!(got.rate_start as u64, case["result"]["rate_start"]);
        assert_eq!(got.derived_start as u64, case["result"]["derived_start"]);
        let rows:Vec<_>=got.definitions.iter().map(|d|json!({"ids":d.ids,"index":d.index,"display_type":d.display_type,"terms":got.derived.iter().find(|r|r.index==d.index).map(|r|&r.terms).cloned().unwrap_or_default()})).collect();
        assert_eq!(
            json!(rows),
            case["result"]["definitions"],
            "{} {}",
            case["name"],
            case["precision"]
        );
    }
}

#[test]
fn technology_property_source_matches_native_head_list() {
    let data = fixture("native-technology-source.json");
    for case in data["cases"].as_array().unwrap() {
        let mut report = NativeLoadReport::default();
        let properties = "[Property]\n{ string IDS = first\n}\n[Property]\n{ string IDS = second\n}\n[Property]\n{ string IDS = third\n}\n";
        let catalog = load_files(
            vec![
                file("template.tgi", case["template"].as_str().unwrap()),
                file("properties.tgi", properties),
                file("subject.tgi", case["source"].as_str().unwrap()),
            ],
            Limits::default(),
            &mut report,
        );
        let result =
            catalog.native_statistics_technology_properties("subject", Precision::Double53);
        if case["thrown"].is_null() {
            assert_eq!(json!(result.unwrap()), case["native_ids"]);
        } else {
            assert!(result.is_err());
        }
    }
}

#[test]
#[ignore = "requires original/v14 source files through AA_GAME_ROOT"]
fn shipped_statistics_catalog_and_tracking_definitions() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut sources).unwrap();
    let stats = catalog
        .native_statistics_catalog(Precision::Double53)
        .unwrap();
    assert_eq!(stats.definitions.len(), 80);
    assert_eq!(stats.derived.len(), 4);
    assert_eq!(stats.resource_ids.len(), 7);
    let mut counts = [0usize; 3];
    let mut errors = Vec::new();
    for actor in &catalog.actors {
        if catalog.native_category(&actor.id).unwrap().native_kind != 2 {
            continue;
        }
        match catalog.native_statistics_tracking_definition(&actor.id, Precision::Double53) {
            Ok(row) => {
                counts[0] += 1;
                counts[1] += usize::from(!row.technology_property_ids.is_empty());
                counts[2] += usize::from(row.required_object_ids.is_some());
                assert!(!row.predecessor_nonempty);
            }
            Err(error) => errors.push(format!("{}: {error}", actor.id)),
        }
    }
    println!(
        "statistics registry rows={}, derived={}, tracking(all, technology, required)={counts:?}; errors={errors:?}",
        stats.definitions.len(),
        stats.derived.len()
    );
    assert!(errors.is_empty());
    assert_eq!(counts[0], 1028);
}
