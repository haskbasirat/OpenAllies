use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use aa_runtime::{
    catalog::Catalog,
    combat_value_source::{self, DefinitionInputs},
};
use serde_json::{Value, json};

#[test]
fn syntax_only_catalog_cannot_claim_native_manager_order() {
    let actors = ActorCatalog::from_files(Vec::new(), aa_rules::tgi::Limits::default());
    assert!(
        combat_value_source::actor_manager_order(&Catalog::new(&actors))
            .unwrap_err()
            .contains("native-order")
    );
}

#[test]
fn source_projection_matches_native_getters_and_definition_argument_assembly() {
    let directory = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-combat-value-source");
    let source: Value = serde_json::from_slice(
        &std::fs::read(directory.join("actual-source-inputs.json")).unwrap(),
    )
    .unwrap();
    let native: Value = serde_json::from_slice(
        &std::fs::read(directory.join("native-source-inputs.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        native["source_sha256"],
        aa_rules::sources::sha256(
            &std::fs::read(directory.join("actual-source-inputs.json")).unwrap()
        )
    );
    let row = |id: &Value, pc: &Value| {
        source["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == *id && s["precision"] == *pc)
            .unwrap()
    };
    for probe in native["getters"].as_array().unwrap() {
        let input = row(&probe["id"], &probe["precision"]);
        let ability = &input["abilities"][probe["ability"].as_u64().unwrap() as usize];
        for field in ["range_24", "power_64", "recharge_18", "first_entry_10"] {
            assert_eq!(
                probe["output"][field].as_u64().unwrap(),
                ability[field].as_u64().unwrap_or(0)
            );
        }
    }
    for probe in native["argument_prefixes"].as_array().unwrap() {
        let input = row(&probe["id"], &probe["precision"]);
        let flags = input["flags_174"].as_u64().unwrap();
        let pc = match input["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!("PC"),
        };
        let mut sum = aa_numeric::Binary::from_i32(0);
        if flags & (1 << 6) != 0 {
            for property in input["properties_1d4"].as_array().unwrap() {
                sum = sum
                    .add(
                        &aa_numeric::Binary::from_f32_bits(property[1].as_u64().unwrap() as u32)
                            .unwrap(),
                        pc,
                    )
                    .unwrap();
            }
        }
        let expected=input["abilities"].as_array().unwrap().iter().enumerate().filter(|_|flags&(1<<5)!=0).map(|(i,_)|json!({
            "ability":i,"defense":if flags&1!=0{input["body"][0].as_u64().unwrap()}else{0},
            "health":if flags&1!=0{input["body"][1].as_u64().unwrap()}else{0},
            "speed":if flags&8!=0{input["speed_228"].as_u64().unwrap()}else{0},
            "property_sum":sum.store_f32().unwrap(),"bias":input["bias_210"].as_u64().unwrap(),"modifier":0
        })).collect::<Vec<_>>();
        assert_eq!(probe["calls"], json!(expected));
    }
    assert_eq!(
        native["argument_prefixes"].as_array().unwrap().len(),
        source["rows"].as_array().unwrap().len() * 2
    );
}

fn snapshot(input: &DefinitionInputs) -> Value {
    json!({
        "id":input.canonical_id,"source":input.source,"precision":format!("{:?}",input.source_precision),
        "flags_174":input.flags_174,"body":input.body.map(|(a,b)|[a.0,b.0]),
        "speed_228":input.speed_228.map(|v|v.0),"bias_210":input.performer_bias_210.map(|v|v.0),
        "properties_1d4":input.properties_1d4.iter().map(|(id,v)|json!([id,v.0])).collect::<Vec<_>>(),
        "children_2f0":input.children_2f0,"weighted_35c":input.weighted_35c,
        "abilities":input.abilities.iter().map(|local|json!({
            "flags_54":local.definition.flags_54,"range_24":local.definition.range_24.0,
            "area_38":local.definition.area_38.0,"projectile_3c":local.definition.projectile_3c,
            "power_64":local.definition.power_64.0,"value_04":local.value_04.0,
            "recharge_18":local.recharge_18.0,"first_entry_10":local.first_entry_10.map(|v|v.0)
        })).collect::<Vec<_>>()
    })
}

#[test]
#[ignore = "requires AA_GAME_ROOT original/v14 sources"]
fn actual_completed_source_inputs_preserve_precision_order_and_identity() {
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 * 1024 * 1024,
        },
    )
    .unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let catalog = Catalog::new(&actors);
    let order_report = combat_value_source::actor_manager_order(&catalog).unwrap();
    assert_eq!(
        order_report.source_diagnostics.len(),
        actors.diagnostics.len()
    );
    assert_eq!(
        order_report.require_no_source_diagnostics().is_ok(),
        actors.diagnostics.is_empty()
    );
    let order = &order_report.ids;
    assert!(!order.is_empty());
    let forward = actors
        .actors
        .iter()
        .filter(|d| order.contains(&d.id))
        .map(|d| d.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(order.iter().rev().cloned().collect::<Vec<_>>(), forward);
    let mut selected = [
        "usa_infantry",
        "gbr_infantry",
        "ger_infantry",
        "jpn_infantry",
        "rus_infantry",
        "usa_corps",
        "gbr_corps",
        "regiment_usa_infantry",
        "herd_horse",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    // Choose one real nonempty Denizen inventory, without inventing a fixture
    // definition ID or calling the later live Denizen slot classifier.
    let denizen = order
        .iter()
        .find(|id| {
            actors
                .resolved_actor(id)
                .unwrap()
                .child("DenizenComponent")
                .unwrap()
                .is_some_and(|node| !node.children("Inventory").unwrap().is_empty())
        })
        .expect("real Denizen inventory");
    if !selected.contains(denizen) {
        selected.push(denizen.clone());
    }
    let mut rows = Vec::new();
    for pc in [
        Precision::Single24,
        Precision::Double53,
        Precision::Extended64,
    ] {
        for id in &selected {
            let input = combat_value_source::definition_inputs(&catalog, id, pc)
                .unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(input.source_precision, pc);
            assert_eq!(input.canonical_id, *id);
            if id == "regiment_usa_infantry" {
                assert!(!input.children_2f0.is_empty());
            }
            if id == denizen {
                assert!(!input.weighted_35c.is_empty());
            }
            rows.push(snapshot(&input));
        }
    }
    let result = json!({"scope":"Immutable completed source inputs only; no computed ActorData CV or source-phase precision certificate","actor_manager_order":order,"source_diagnostics":order_report.source_diagnostics,"rows":rows});
    if let Some(path) = std::env::var_os("AA_COMBAT_SOURCE_CAPTURE") {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        serde_json::to_writer_pretty(file, &result).unwrap();
    } else {
        let expected: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261016-combat-value-source/actual-source-inputs.json"
        ))
        .unwrap();
        assert_eq!(result, expected);
    }
    assert!(
        combat_value_source::definition_inputs(&catalog, "missing_actor", Precision::Double53)
            .is_err()
    );
}
