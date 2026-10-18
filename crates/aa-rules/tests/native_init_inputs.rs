use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
#[test]
fn initial_property_references_keep_native_prepend_order() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-init-lists.json"
    ))
    .unwrap();
    for row in fixture["property_lists"].as_array().unwrap() {
        let tags = row["authored"].as_array().unwrap();
        let mut source = String::from(
            "[Template A]{\n[optional PropertiesComponent]{[list Property]{string property_IDS}}\n}\n[Thing template=A]{\nIDS=unit\nstring category=ACTOR\nstring name_list_ids=NULL\nbool numbered=false\n[PropertiesComponent]{\n",
        );
        for tag in tags {
            source.push_str(&format!("[Property]{{property_IDS=prop_{tag}}}\n"));
        }
        source.push_str("}\n}\n");
        let ids: std::collections::BTreeSet<_> = tags.iter().map(|v| v.as_u64().unwrap()).collect();
        for id in ids {
            source.push_str(&format!("[Property]{{string IDS=prop_{id}}}\n"));
        }
        let catalog = ActorCatalog::from_files(
            vec![ResolvedFile {
                logical_path: "source.tgi".into(),
                origin: Origin::V14Loose,
                sha256: sha256(source.as_bytes()),
                bytes: source.as_bytes().to_vec(),
            }],
            Limits::default(),
        );
        let out = catalog
            .native_actor_init_inputs("UNIT", Precision::Double53)
            .unwrap();
        let expected: Vec<_> = row["native"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| format!("prop_{v}"))
            .collect();
        assert_eq!(
            out.initial_properties
                .iter()
                .map(|x| x.id.clone())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(out.actor_id, "unit");
        assert!(out.name_list.is_none());
        assert!(!out.numbered.boolean().unwrap());
        assert!(out.trailer.is_none());
    }
}
#[test]
#[ignore = "requires original/v14 files via AA_GAME_ROOT"]
fn original_ground_creation_inputs_bind_native_turrets() {
    let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut source = V14Sources::open(
        root.join("Data.rwd"),
        root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut source).unwrap();
    let mut rows = Vec::new();
    for id in [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "rus_infantry",
        "jpn_infantry",
    ] {
        let init = catalog
            .native_actor_init_inputs(id, Precision::Double53)
            .unwrap();
        let performer = catalog
            .native_ground_performer_constructor(id, Precision::Double53)
            .unwrap()
            .unwrap();
        assert_eq!(init.actor_id, id);
        assert!(performer.turret_yaw_bits.is_empty());
        rows.push(serde_json::json!({"init":init,"performer":performer}));
    }
    // 464BD0/4C7820 Turret list: one turret, yaw_start through 9FC064.
    let tank = catalog
        .native_ground_performer_constructor("usa_tank_medium", Precision::Double53)
        .unwrap()
        .unwrap();
    let turrets = catalog
        .native_turrets("usa_tank_medium", Precision::Double53)
        .unwrap();
    assert!(!turrets.is_empty());
    assert_eq!(
        tank.turret_yaw_bits,
        turrets.iter().map(|t| t.yaw_start_bits).collect::<Vec<_>>()
    );
    println!("{}", serde_json::to_string(&rows).unwrap());
}
