use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    native::actor_morale::{actor_morale_contribution, actor_morale_source},
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/resume-20261016-morale/native-morale.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn file(name: &str, text: &str) -> ResolvedFile {
    let bytes = text.as_bytes().to_vec();
    ResolvedFile {
        logical_path: name.into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}
fn pc(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!("unknown precision {name}"),
    }
}

#[test]
fn parser_producer_getter_match_original_x86() {
    let doc = fixture();
    for row in doc["cases"].as_array().unwrap() {
        let label = format!("{} {}", row["name"], row["precision"]);
        let mut report = NativeLoadReport::default();
        let catalog = load_files(
            vec![
                file("template.tgi", row["template"].as_str().unwrap()),
                file("subject.tgi", row["source"].as_str().unwrap()),
            ],
            Limits::default(),
            &mut report,
        );
        let result =
            actor_morale_source(&catalog, "subject", pc(row["precision"].as_str().unwrap()));
        let native = &row["result"];
        if native["thrown"].is_null() {
            let rust = result.unwrap_or_else(|e| panic!("{label}: {e}; {report:?}"));
            assert_eq!(u64::from(rust.bits), native["stored_bits"], "{label}");
            assert_eq!(u64::from(rust.bits), native["getter_bits"], "{label}");
            assert_eq!(
                rust.morale_value.is_some(),
                native["stopped_at"] == "0x4668a8",
                "{label}"
            );
        } else {
            assert!(
                result.is_err(),
                "{label}: native threw {:?}, Rust accepted",
                native["messages"]
            );
        }
    }
}

#[test]
fn rejects_unknown_and_non_actor_definition() {
    let mut report = NativeLoadReport::default();
    let catalog = load_files(
        vec![file(
            "thing.tgi",
            "[Thing]\n{ string IDS = tree\n string category = OBJECT\n}\n",
        )],
        Limits::default(),
        &mut report,
    );
    assert!(actor_morale_contribution(&catalog, "tree", Precision::Double53).is_err());
    assert!(actor_morale_contribution(&catalog, "absent", Precision::Double53).is_err());
}

#[test]
fn rejects_a_shadowed_canonical_identity_without_translating_twice() {
    let template = "[template Thing]\n{ string IDS\n string category = ACTOR\n [optional ElementComponent]\n { fixed morale_value\n fixed resupply_rate = 0\n }\n}\n[template Translate]\n{ string node\n string ids_old\n string ids_new\n}\n";
    let things = "[Thing template=Thing]\n{ IDS = b\n [ElementComponent]\n { morale_value = 12\n }\n}\n[Thing template=Thing]\n{ IDS = c\n [ElementComponent]\n { morale_value = 99\n }\n}\n";
    let translations = "[Translate template=Translate]\n{ node = Thing\n ids_old = a\n ids_new = b\n}\n[Translate template=Translate]\n{ node = Thing\n ids_old = b\n ids_new = c\n}\n";
    let mut report = NativeLoadReport::default();
    let catalog = load_files(
        vec![
            file("template.tgi", template),
            file("actors.tgi", things),
            file("translations.tgi", translations),
        ],
        Limits::default(),
        &mut report,
    );
    assert_eq!(catalog.find("a").unwrap().id, "b");
    assert_eq!(catalog.find("b").unwrap().id, "c");
    let error = actor_morale_contribution(&catalog, "b", Precision::Double53).unwrap_err();
    assert!(error.to_string().contains("unshadowed canonical"));
    assert_eq!(
        actor_morale_contribution(&catalog, "c", Precision::Double53).unwrap(),
        99f32.to_bits()
    );
}

#[test]
#[ignore = "requires original Data.rwd and v14 files via AA_GAME_ROOT"]
fn every_shipped_standard_actor_morale_projection() {
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
    let mut runs = Vec::new();
    for precision in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        let mut values = BTreeMap::<u32, usize>::new();
        let mut actor_count = 0;
        let mut elements = 0;
        let mut nonactors = 0;
        let mut errors = Vec::new();
        let mut category_errors = Vec::new();
        for actor in &catalog.actors {
            match catalog.native_category(&actor.id) {
                Ok(c) if c.native_kind != 2 => {
                    nonactors += 1;
                    continue;
                }
                Err(e) => {
                    category_errors.push(format!("{}: {e}", actor.id));
                    continue;
                }
                _ => {}
            }
            actor_count += 1;
            match actor_morale_source(&catalog, &actor.id, precision) {
                Ok(value) => {
                    elements += usize::from(value.morale_value.is_some());
                    *values.entry(value.bits).or_default() += 1;
                }
                Err(e) => errors.push(format!("{}: {e}", actor.id)),
            }
        }
        println!(
            "{precision:?}: {actor_count} standard actors, {elements} ElementComponents, {nonactors} non-actors; values {values:?}; category errors {category_errors:?}; errors {errors:?}"
        );
        assert!(actor_count > 500);
        assert!(elements > 100);
        assert!(errors.is_empty(), "{errors:#?}");
        assert!(category_errors.is_empty(), "{category_errors:#?}");
        runs.push(json!({"precision": format!("{precision:?}"), "standard_actors": actor_count, "elements": elements, "other_kinds": nonactors, "values_bits": values, "errors": errors, "category_errors": category_errors}));
    }
    if let Some(out) = std::env::var_os("AA_MORALE_CENSUS") {
        let output = json!({"scope": "Rust projection across all loaded original/v14 sources; native fragment parity is separately proven by native-morale.json. Not a complete ActorData loader.", "sources": catalog.sources, "diagnostics": catalog.diagnostics, "actor_definitions": catalog.actors.len(), "runs": runs});
        std::fs::write(out, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    }
}
