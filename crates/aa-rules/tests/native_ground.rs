use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    ground::{expand_slots, finalized_detection_range, finalized_remote_detection_range},
    sources::{SourceLimits, V14Sources},
};
use serde_json::Value;
#[test]
fn native_layout_expansion_and_detection_finalizers() {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-ground-definitions.json"
    ))
    .unwrap();
    for row in proof["layout_cases"].as_array().unwrap() {
        let primary = row["primary"].as_u64();
        let morale = row["morale_line"].as_bool().unwrap();
        let input: Vec<_> = row["counts"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    n.as_i64().unwrap() as i32,
                    Some(i as u64) == primary,
                    morale,
                )
            })
            .collect();
        let (groups, primary, morale) = expand_slots(&input).unwrap();
        assert_eq!(groups.len() as u64, row["stored_count"]);
        assert_eq!(
            serde_json::to_value(groups).unwrap(),
            row["expanded_groups"]
        );
        assert_eq!(primary as u32 as u64, row["stored_primary"]);
        assert_eq!(morale as u32 as u64, row["stored_morale_count"]);
    }
    for row in proof["detection_cases"].as_array().unwrap() {
        let input = serde_json::from_value(row["input_bits"].clone()).unwrap();
        assert_eq!(
            finalized_detection_range(row["active"].as_bool().unwrap(), input).unwrap() as u64,
            row["output_bits"]
        );
    }
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-remote-detection.json"
    ))
    .unwrap();
    for row in proof["cases"].as_array().unwrap() {
        assert_eq!(
            finalized_remote_detection_range(
                row["flags"].as_u64().unwrap() as u32,
                row["remote_bits"].as_u64().unwrap() as u32,
                row["control_bits"].as_u64().unwrap() as u32
            )
            .unwrap() as u64,
            row["output_bits"]
        );
    }
    assert!(expand_slots(&[(1, true, true), (1, true, true)]).is_err());
}
#[test]
#[ignore = "requires original Data.rwd and accepted runtime-expanded-v14 via AA_GAME_ROOT"]
fn original_ground_definitions_bind_real_component_inputs() {
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
    assert_eq!(
        catalog.native_ability_type_domain(pc).unwrap().ids,
        ["repair", "attack"]
    );
    assert_eq!(
        catalog.native_resource_domain(pc).unwrap().count().unwrap(),
        7
    );
    for id in [
        "usa_infantry",
        "ger_infantry",
        "gbr_infantry",
        "jpn_infantry",
        "rus_infantry",
        "usa_tank_medium",
    ] {
        let data = catalog.native_ground_definition(id, pc).unwrap();
        assert_eq!(
            data.detection.unwrap().finalized_range_bits,
            18f32.to_bits()
        );
        assert!(data.organization.is_none());
        assert!(!data.economy.unwrap().supply_required.boolean().unwrap());
        let abilities = catalog.native_performer_inputs(id, pc).unwrap().unwrap();
        assert_eq!(
            abilities.abilities.len(),
            if id == "usa_tank_medium" { 1 } else { 2 }
        );
        for ability in abilities.abilities {
            assert_eq!(ability.definition.effects.len(), 1);
            let aa_rules::native::TypedValue::String { value, .. } =
                &ability.definition.effects[0].fields["type"]
            else {
                panic!()
            };
            assert_eq!(value, "MAIN_DAMAGE");
        }
    }
    let data = catalog
        .native_ground_definition("regiment_usa_infantry", pc)
        .unwrap();
    assert_eq!(
        data.detection.unwrap().finalized_range_bits,
        26f32.to_bits()
    );
    let org = data.organization.unwrap();
    assert_eq!(org.layout.expanded_groups, [0, 0, 0, 1, 2, 3, 4, 4, 5]);
    assert_eq!(org.morale_breakpoint.bits, 20f32.to_bits());
    assert!(
        catalog
            .native_performer_inputs("regiment_usa_infantry", pc)
            .unwrap()
            .is_none()
    );
}
#[test]
fn original_organization_parameter_binding_validates_real_element_properties() {
    let d: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-definitions/native-layout-parameters.json"
    ))
    .unwrap();
    for row in d["cases"].as_array().unwrap() {
        let got = aa_rules::ground::validate_organization_element(
            row["is_element"].as_bool().unwrap(),
            row["has_property"].as_bool().unwrap(),
            row["mobile_deployment"].as_bool().unwrap(),
            row["multiplicity"].as_i64().unwrap() as i32,
        );
        assert_eq!(got.is_ok(), row["accepted"].as_bool().unwrap(), "{row}");
    }
}
