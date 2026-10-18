use aa_numeric::Precision;
use aa_rules::{
    actors::{
        ActorCatalog,
        native_load::{NativeLoadReport, load_files},
    },
    native::recruit_backrefs::{self, Inputs},
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};

fn evidence(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-recruit-backrefs")
        .join(name)
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
#[test]
fn completed_lists_match_native_original_and_v14() {
    let doc: Value =
        serde_json::from_slice(&std::fs::read(evidence("native-recruit-backrefs.json")).unwrap())
            .unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(), 270);
    for row in doc["cases"].as_array().unwrap() {
        let input: Inputs = serde_json::from_value(row["input"].clone()).unwrap();
        let actual = input.complete();
        if row["output"]["outcome"] == "ok" {
            let out = actual.unwrap_or_else(|e| panic!("{row}: {e}"));
            let lists: std::collections::BTreeMap<_, _> = out
                .definitions
                .iter()
                .map(|(k, d)| (k.clone(), d.ids.clone()))
                .collect();
            assert_eq!(json!(lists), row["output"]["lists"], "{row}");
            assert_eq!(json!(out.resolution_order), row["output"]["order"], "{row}");
        } else {
            assert!(actual.is_err(), "{row}");
        }
    }
}

#[test]
fn final_translate_selects_reference_identity_without_redirecting_canonical_owner() {
    let actors = load_files(
        vec![file(
            "actors.tgi",
            concat!(
                "[Thing]\n{ string IDS = old\n string category = ACTOR\n [StructureComponent]\n{}\n [RecruitComponent]\n { [RecruitActor]\n { string actor_ids = alias\n }\n }\n}\n",
                "[Thing]\n{ string IDS = chosen\n string category = ACTOR\n}\n",
                "[Thing]\n{ string IDS = other\n string category = ACTOR\n}\n",
                "[Translate]\n{ string node = Thing\n string ids_old = alias\n string ids_new = chosen\n}\n",
                "[Translate]\n{ string node = Thing\n string ids_old = old\n string ids_new = other\n}\n"
            ),
        )],
        Limits::default(),
        &mut NativeLoadReport::default(),
    );
    let arena = actors.native_recruit_backrefs(Precision::Double53).unwrap();
    assert_eq!(arena.definition("chosen").unwrap().ids, vec!["old"]);
    assert!(arena.present("chosen").unwrap());
    assert!(arena.definition("old").unwrap().ids.is_empty());
    assert!(arena.definition("other").unwrap().ids.is_empty());
    assert!(
        arena.definition("alias").is_err(),
        "canonical getter must not resolve an authored alias"
    );
    assert!(
        recruit_backrefs::actor_recruit_backref_present(&actors, "missing", Precision::Double53)
            .is_err()
    );
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 data"]
fn real_canonical_actor_backreferences_are_source_bound_and_match_native() {
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
    let inputs = actors
        .native_recruit_backref_inputs(Precision::Double53)
        .unwrap();
    let arena = inputs.complete().unwrap();
    assert_eq!(inputs.actors.len(), 1028);
    let lists: std::collections::BTreeMap<_, _> = arena
        .definitions
        .iter()
        .map(|(k, v)| (k.clone(), v.ids.clone()))
        .collect();
    if let Some(path) = std::env::var_os("AA_RECRUIT_BACKREF_CAPTURE") {
        std::fs::write(path,serde_json::to_vec_pretty(&json!({"input":inputs,"lists":lists,"order":arena.resolution_order,"sources":actors.sources})).unwrap()).unwrap();
        return;
    }
    let expected: Value = serde_json::from_slice(
        &std::fs::read(evidence("native-real-recruit-backrefs.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(expected["cases"].as_array().unwrap().len(), 6);
    for row in expected["cases"].as_array().unwrap() {
        assert_eq!(row["output"]["outcome"], "ok");
        assert_eq!(json!(inputs), row["input"]);
        assert_eq!(json!(lists), row["output"]["lists"]);
        assert_eq!(json!(arena.resolution_order), row["output"]["order"]);
    }
    for pc in [Precision::Single24, Precision::Extended64] {
        let other_inputs = actors.native_recruit_backref_inputs(pc).unwrap();
        assert_eq!(json!(other_inputs), json!(inputs), "source facts at {pc:?}");
    }
    let nonempty = lists.values().filter(|l| !l.is_empty()).count();
    assert!(nonempty > 0);
    println!(
        "{} canonical actors, {nonempty} with completed+17C, {} callbacks",
        inputs.actors.len(),
        arena.resolution_order.len()
    );
}
