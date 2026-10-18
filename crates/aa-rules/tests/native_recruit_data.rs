use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    native::recruit_data,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../research/skirmish-20261016-recruit-actions/native-recruit-source.json",
            ),
        )
        .unwrap(),
    )
    .unwrap()
}
fn file(path: &str, text: &str) -> ResolvedFile {
    let bytes: Vec<u8> = text
        .chars()
        .map(|c| u8::try_from(c as u32).unwrap())
        .collect();
    ResolvedFile {
        logical_path: path.into(),
        origin: Origin::V14Loose,
        sha256: sha256(&bytes),
        bytes,
    }
}
fn catalog(files: Vec<ResolvedFile>) -> ActorCatalog {
    load_files(files, Limits::default(), &mut NativeLoadReport::default())
}
fn pc(s: &str) -> Precision {
    match s {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        _ => Precision::Extended64,
    }
}
const TARGETS: &str = concat!(
    "[Thing]\n{ string IDS = a\n string category = ACTOR\n}\n",
    "[Thing]\n{ string IDS = b\n string category = ACTOR\n}\n",
    "[Thing]\n{ string IDS = wrong_class\n string category = OBJECT\n}\n",
    "[Thing]\n{ string IDS = plain\n string category = ACTOR\n}\n",
    "[Thing]\n{ string IDS = transport\n string category = ACTOR\n [TransportComponent]\n {}\n}\n",
);
#[test]
fn native_deferred_lists_transport_and_state_type_match() {
    let doc = fixture();
    let mut checked = 0;
    for row in doc["cases"].as_array().unwrap() {
        let precision = pc(row["precision"].as_str().unwrap());
        match row["kind"].as_str().unwrap() {
            "targets" => {
                let actors = catalog(vec![
                    file(
                        "Templates/template_recruit.tgi",
                        doc["template"]["text"].as_str().unwrap(),
                    ),
                    file("targets.tgi", TARGETS),
                    file("owner.tgi", row["text"].as_str().unwrap()),
                ]);
                let result = recruit_data::actor_recruit_targets(&actors, "owner");
                if row["result"]["error"].is_null() {
                    assert_eq!(json!(result.unwrap().ids), row["result"]["list"]);
                } else {
                    assert!(result.is_err(), "{row}");
                }
            }
            "transport" => {
                let value = row["value"].as_str().unwrap();
                let text = format!(
                    "[Thing]\n{{ string IDS = owner\n string category = ACTOR\n string transport_actor = {}\n}}\n",
                    if value.is_empty() { "NULL" } else { value }
                );
                let actors = catalog(vec![file("targets.tgi", TARGETS), file("owner.tgi", &text)]);
                let result = recruit_data::actor_transport_class(&actors, "owner");
                if row["result"]["error"] == true {
                    assert!(result.is_err(), "{row}");
                } else {
                    assert_eq!(
                        json!(result.unwrap_or_else(|e| panic!("{row}: {e}; {text}"))),
                        row["result"]["nonnull"]
                    );
                }
            }
            "state" => {
                let actors = catalog(vec![file("state.tgi", row["text"].as_str().unwrap())]);
                let result = recruit_data::actor_state_type(&actors, "fixture_state", precision);
                if row["result"]["error"] == true {
                    assert!(result.is_err(), "{row}");
                } else {
                    assert_eq!(json!(result.unwrap()), row["result"]["value"]);
                }
            }
            other => panic!("{other}"),
        }
        checked += 1;
    }
    assert_eq!(checked, 45);
}

#[test]
#[ignore = "requires original/v14 source assets via AA_GAME_ROOT"]
fn every_real_actor_recruit_list_and_transport_and_state_type_resolves() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let mut rows = Vec::new();
    let mut transport = 0;
    for actor in &actors.actors {
        if actors.native_category(&actor.id).unwrap().native_kind != 2 {
            continue;
        }
        let targets = recruit_data::actor_recruit_targets(&actors, &actor.id)
            .unwrap_or_else(|e| panic!("{}: {e}", actor.id));
        let class = recruit_data::actor_transport_class(&actors, &actor.id)
            .unwrap_or_else(|e| panic!("{}: {e}", actor.id));
        transport += usize::from(class);
        if !targets.ids.is_empty() {
            rows.push(json!({"actor":actor.id,"targets":targets.ids}));
        }
    }
    let mut states = Vec::new();
    for entry in &actors.native_data_registry("ActorState").unwrap().entries {
        for precision in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let value = recruit_data::actor_state_type(&actors, &entry.ids, precision)
                .unwrap_or_else(|e| panic!("{}: {e}", entry.ids));
            if precision == Precision::Double53 {
                states.push(json!({"id":entry.ids,"type":value}));
            }
        }
    }
    assert!(!rows.is_empty());
    assert!(!states.is_empty());
    println!(
        "{} nonempty Recruit lists, {transport} transport-linked actors, {} state types",
        rows.len(),
        states.len()
    );
    if let Some(path) = std::env::var_os("AA_RECRUIT_SOURCE_CENSUS") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"lists":rows,"transport_linked_actors":transport,"states":states,"sources":actors.sources})).unwrap()).unwrap();
    }
}
