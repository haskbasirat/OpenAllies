use super::*;

#[test]
fn actual_source_ids_spelling_survives_folded_bank_identity() {
    let mut bank = crate::definitions::Bank::default();
    let token = bank.intern("ActorDefinition", "MiXeD_Target").unwrap();
    let definition = DefinitionRef {
        token,
        canonical_ids: "MiXeD_Target".into(),
    };
    assert_eq!(current_ids(&bank, &definition).unwrap(), "MiXeD_Target");
    let foreign = DefinitionRef {
        token,
        canonical_ids: "another_target".into(),
    };
    assert!(current_ids(&bank, &foreign).is_err());
    let property = bank.intern("Property", "MiXeD_Target").unwrap();
    assert!(
        current_ids(
            &bank,
            &DefinitionRef {
                token: property,
                ..definition
            }
        )
        .is_err()
    );
}

#[test]
fn current_post_creation_arguments_match_original_and_v14() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../research/development10/first-mission-callbacks/native-created-log.json"
    ))
    .unwrap();
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        let definition = DefinitionRef {
            token: 7,
            canonical_ids: input["ids"].as_str().unwrap().into(),
        };
        let position = [
            input["x"].as_u64().unwrap() as u32,
            input["y"].as_u64().unwrap() as u32,
            0x7f800001,
        ];
        if input.get("stop").is_some() {
            assert!(row["output"]["args"].is_null());
            continue;
        }
        let actual = arguments(Some(&definition), &position, |value| {
            Ok(value.canonical_ids.clone())
        })
        .unwrap();
        assert_eq!(actual.ids, row["output"]["args"]["ids"].as_str().unwrap());
        assert_eq!(
            format!("{:016x}", actual.x_double),
            row["output"]["args"]["x_double"].as_str().unwrap()
        );
        assert_eq!(
            format!("{:016x}", actual.y_double),
            row["output"]["args"]["y_double"].as_str().unwrap()
        );
    }
}

#[test]
fn modern_transport_uses_current_definition_xy_and_preserves_failed_read() {
    let definition = DefinitionRef {
        token: 7,
        canonical_ids: "current_target".into(),
    };
    let position = [(-0.0f32).to_bits(), 108.0f32.to_bits(), 0x7f800001];
    let message = message(Some(&definition), &position, |value| {
        assert_eq!(value.token, 7);
        Ok(value.canonical_ids.clone())
    })
    .unwrap();
    assert_eq!(message.function, 0x404f90);
    assert_eq!(message.line, 0x135e);
    assert_eq!(
        message.text,
        "created replacement current_target at -0.000000,108.000000"
    );
    assert!(
        super::message(Some(&definition), &position, |_| Err(
            "actual source unavailable".into()
        ))
        .is_err()
    );
    assert!(
        super::message(None, &position, |_| panic!(
            "null Data must stop before IDS read"
        ))
        .is_err()
    );
    assert_eq!(position[2], 0x7f800001); //Z is never interpreted by this caller.
}
