use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    economy_values::recruitability_word,
    sources::{Origin, ResolvedFile, SourceLimits, V14Sources, sha256},
    tgi::Limits,
};
use serde_json::{Value, json};
fn file(name: &str, value: String) -> ResolvedFile {
    ResolvedFile {
        logical_path: name.into(),
        origin: Origin::V14Loose,
        sha256: sha256(value.as_bytes()),
        bytes: value.into_bytes(),
    }
}
#[test]
fn native_economy_loader_dense_fields_and_recruitability_registration() {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-economy-values/native-source.json"
    ))
    .unwrap();
    for row in proof["enums"].as_array().unwrap() {
        assert_eq!(
            recruitability_word(row["name"].as_str().unwrap()).unwrap() as u64,
            row["word"]
        );
    }
    assert!(recruitability_word("MAYBE").is_err());
    for row in proof["cases"].as_array().unwrap() {
        let input = &row["input"];
        let pc = serde_json::from_value(input["precision"].clone()).unwrap();
        let ids = input["resource_ids"].as_array().unwrap();
        let mut resources = String::new();
        for (i, id) in ids.iter().enumerate() {
            let kind = ["STOCKPILED", "UPKEEP", "LIMITED_CONSUMED"]
                [input["kinds"][i].as_u64().unwrap() as usize];
            resources.push_str(&format!(
                "[Resource]{{\nstring IDS={}\nstring type={kind}\n}}\n",
                id.as_str().unwrap()
            ));
        }
        let mut actor = "[Thing]{\nIDS=test\nstring category=UNIT\nstring recruitable=YES\nstring transport_actor=NULL\n".to_owned();
        if input["present"].as_bool().unwrap() {
            actor.push_str(&format!(
                "[EconomyComponent]{{\nflag supply_required={}\n",
                input["supply_required"].as_bool().unwrap()
            ));
            for name in ["Cost", "Upkeep", "Production"] {
                actor.push_str(&format!("[{name}]{{\n"));
                for (i, id) in ids.iter().enumerate() {
                    let value = f32::from_bits(input[name][i].as_u64().unwrap() as u32);
                    actor.push_str(&format!("fixed {}={value:.40}\n", id.as_str().unwrap()));
                }
                actor.push_str("}\n");
            }
            actor.push_str("}\n");
        }
        actor.push_str("}\n");
        let catalog = ActorCatalog::from_files(
            vec![
                file("Game/resources.tgi", resources),
                file("actor.tgi", actor),
            ],
            Limits::default(),
        );
        let result = catalog.native_economy_inputs("test", pc).unwrap();
        assert_eq!(result.recruitable_word, 1);
        assert!(result.transport_actor.is_none());
        assert!(result.unpack_to_actor.is_none());
        if let Some(values) = result.resources {
            assert_eq!(
                json!({"Cost":values.cost_bits,"Upkeep":values.upkeep_bits,"Production":values.production_bits}),
                row["output"]["vectors"]
            );
            assert_eq!(
                values.supply_required.boolean().unwrap(),
                row["output"]["supply_required"]
            );
            assert_eq!(json!(values.resource_ids), input["resource_ids"]);
        } else {
            assert!(!input["present"].as_bool().unwrap());
            assert_eq!(row["output"]["flags174"], input["initial_flags"]);
        }
    }
}
#[test]
#[ignore = "requires original Data.rwd and accepted v14 sources via AA_GAME_ROOT"]
fn original_actor_economy_values_preserve_actual_resource_order_and_pairing_inputs() {
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
    let pc = Precision::Double53;
    let native: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-economy-values/native-source.json"
    ))
    .unwrap();
    let mut rows = Vec::new();
    for id in [
        "usa_infantry",
        "ger_infantry",
        "gbr_infantry",
        "jpn_infantry",
        "rus_infantry",
        "usa_tank_medium",
        "regiment_usa_infantry",
        "usa_corps",
        "usa_truck_corps",
    ] {
        let value = catalog.native_economy_inputs(id, pc).unwrap();
        let resources = value.resources.as_ref().unwrap();
        assert_eq!(
            resources.resource_ids,
            [
                "money",
                "ammo",
                "oil",
                "unit_limit_provided",
                "unit_limit_consumed",
                "building_limit_provided",
                "building_limit_consumed"
            ]
        );
        assert_eq!(resources.cost_bits.len(), 7);
        assert_eq!(resources.upkeep_bits.len(), 7);
        assert_eq!(resources.production_bits.len(), 7);
        assert_eq!(&resources.cost_bits[1..], &[0; 6]);
        let native = native["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["input"]["source_actor"] == id && row["input"]["precision"] == "Double53"
            })
            .unwrap();
        assert_eq!(
            json!({"Cost":resources.cost_bits,"Upkeep":resources.upkeep_bits,"Production":resources.production_bits}),
            native["output"]["vectors"]
        );
        if id == "usa_truck_corps" {
            assert_eq!(value.unpack_to_actor.as_ref().unwrap().id, "usa_corps");
        }
        rows.push(value);
    }
    if let Some(path) = std::env::var_os("AA_ECONOMY_SOURCE_OUTPUT") {
        std::fs::write(path, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
}

#[test]
fn implicit_identifier_strings_keep_origin_and_reject_other_value_types() {
    let text = "[Template Base]{\nstring category=UNIT\nrecruitable=NO\ntransport_actor=NULL\n}\n[Thing Template=Base]{\nIDS=test\nrecruitable=YES\n}\n";
    let catalog =
        ActorCatalog::from_files(vec![file("strings.tgi", text.into())], Limits::default());
    let node = catalog.resolved_actor("test").unwrap();
    let aa_rules::native::TypedValue::String {
        value,
        source,
        declaration_source,
    } = node.plain_string_field("recruitable").unwrap().unwrap()
    else {
        panic!()
    };
    assert_eq!(value, "YES");
    assert_eq!(&text[source.span.start..source.span.end], "recruitable=YES");
    assert_eq!(
        &text[declaration_source.span.start..declaration_source.span.end],
        "recruitable=NO"
    );
    assert!(node.field("recruitable", Precision::Double53).is_err());
    let aa_rules::native::TypedValue::String { value, .. } =
        node.plain_string_field("transport_actor").unwrap().unwrap()
    else {
        panic!()
    };
    assert!(value.is_empty());
    for assignment in [
        "x=\"YES\"",
        "x=$other",
        "fixed x=1",
        "x=1",
        "x=YES\nx=NO",
        "x=hello world",
        "string x",
    ] {
        let catalog = ActorCatalog::from_files(
            vec![file(
                "strings.tgi",
                format!("[Thing]{{\nIDS=test\nstring category=UNIT\n{assignment}\n}}\n"),
            )],
            Limits::default(),
        );
        assert!(
            catalog
                .resolved_actor("test")
                .unwrap()
                .plain_string_field("x")
                .is_err(),
            "{assignment}"
        );
    }
}
