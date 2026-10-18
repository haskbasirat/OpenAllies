use aa_mapworld::saved_components::SavedComponent;
use serde_json::Value;
#[test]
fn element_load_accepts_signed_negative_history_count_and_resets_transient_fields() {
    use aa_actors::{ActorId, ground_states::ElementState};
    use aa_body::F32;
    use aa_mapworld::restore_components::{GroundDefinitions, RestoredComponent, apply_saved};
    use aa_numeric::Precision;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-saved-components.json"
    ))
    .unwrap();
    let cases: Vec<_> = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["slot"] == 8)
        .collect();
    assert_eq!(cases.len(), 5);
    for case in cases {
        let payload: SavedComponent = serde_json::from_value(case["expected"].clone()).unwrap();
        let mut element = ElementState::<String>::new(ActorId::from_raw(1)).unwrap();
        element.flag_1c = true;
        element.time_24 = F32(2f32.to_bits());
        element.header.scheduled_head = Some(99);
        let mut component = RestoredComponent::Element(Box::new(element));
        apply_saved(
            &mut component,
            &payload,
            &GroundDefinitions::default(),
            &mut 0,
            Precision::Double53,
        )
        .unwrap();
        let RestoredComponent::Element(element) = component else {
            panic!("wrong component")
        };
        assert_eq!(
            element.flag_1c,
            (case["native"]["words"]["0x1c"].as_u64().unwrap() & 255) != 0
        );
        assert_eq!(
            element.time_24.0,
            case["native"]["words"]["0x24"].as_u64().unwrap() as u32
        );
        assert_eq!(
            element.header.scheduled_head,
            if payload.updatable().is_some() {
                None
            } else {
                Some(99)
            }
        );
    }
}
#[test]
fn failed_mover_speed_restoration_preserves_native_partial_write_boundary() {
    use aa_actors::{
        ActorId,
        ground_states::{MoverDefinition, MoverState},
    };
    use aa_body::F32;
    use aa_numeric::Precision;
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-saved-components.json"
    ))
    .unwrap();
    let case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["slot"] == 3 && c["expected"]["Mover"]["version"] == 8)
        .unwrap();
    let SavedComponent::Mover(input) = serde_json::from_value(case["expected"].clone()).unwrap()
    else {
        panic!("missing native Mover case")
    };
    for missing in [true, false] {
        let mut input = input.clone();
        if !missing {
            input.speed_multiplier_bits = Some(0x7fc01234);
        }
        let mut state = MoverState::new(
            ActorId::from_raw(1),
            MoverDefinition {
                speed: F32(2.5f32.to_bits()),
                acceleration: F32::ONE,
                turn_radius: F32::ONE,
                turn_type: 1,
            },
            Precision::Double53,
        )
        .unwrap();
        state.header.scheduled_head = Some(99);
        let prior = state.clone();
        let mut flags = 0x123;
        assert!(
            input
                .apply(
                    &mut state,
                    if missing {
                        None
                    } else {
                        Some(2.5f32.to_bits())
                    },
                    false,
                    &mut flags,
                    Precision::Double53
                )
                .is_err()
        );
        assert_eq!(state.header.scheduled_head, None);
        assert_eq!(state.speed, prior.speed);
        assert_eq!(state.bytes, prior.bytes);
        assert_eq!(flags, 0x123);
        for (i, value) in input.vector_10.into_iter().enumerate() {
            assert_eq!(state.words[&(0x10 + i as u32 * 4)], value);
        }
        assert_eq!(state.words[&0x20], input.word_20);
        assert_eq!(state.words[&0x24], input.word_24);
        let allowed = [0x10, 0x14, 0x18, 0x20, 0x24];
        let later: std::collections::BTreeMap<_, _> = state
            .words
            .iter()
            .filter(|(o, _)| !allowed.contains(o))
            .collect();
        let prior_later: std::collections::BTreeMap<_, _> = prior
            .words
            .iter()
            .filter(|(o, _)| !allowed.contains(o))
            .collect();
        assert_eq!(later, prior_later);
    }
}
#[test]
fn saved_component_wire_matches_original_readers() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-saved-components.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 170);
    for case in cases {
        let hex = case["payload"].as_str().unwrap();
        let mut bytes: Vec<u8> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|v| u8::from_str_radix(std::str::from_utf8(v).unwrap(), 16).unwrap())
            .collect();
        bytes.extend_from_slice(b"FOLLOWING");
        let (value, consumed) =
            SavedComponent::decode_prefix(case["slot"].as_u64().unwrap() as u8, &bytes).unwrap();
        assert_eq!(serde_json::to_value(&value).unwrap(), case["expected"]);
        assert_eq!(
            consumed,
            case["native"]["consumed"].as_u64().unwrap() as usize
        );
        let pc = serde_json::from_value(case["precision"].clone()).unwrap();
        if let SavedComponent::Body(input) = &value {
            let mut state = aa_actors::ground_states::BodyState::new(
                aa_actors::ActorId::from_raw(1),
                serde_json::from_value(serde_json::json!(200f32.to_bits())).unwrap(),
                serde_json::from_value(serde_json::json!(1f32.to_bits())).unwrap(),
            )
            .unwrap();
            input.apply(&mut state, pc).unwrap();
            for (offset, bits) in [
                ("0x10", state.body.health.0),
                ("0x1c", state.body.dying_started.0),
                ("0x20", state.body.last_heal_time.0),
                ("0x24", state.body.sink_duration.0),
            ] {
                assert_eq!(
                    bits,
                    case["native"]["words"][offset].as_u64().unwrap() as u32,
                    "Body{offset}"
                );
            }
        }
        if let SavedComponent::Mover(input) = &value {
            let def = aa_actors::ground_states::MoverDefinition {
                speed: serde_json::from_value(serde_json::json!(2.5f32.to_bits())).unwrap(),
                acceleration: serde_json::from_value(serde_json::json!(1f32.to_bits())).unwrap(),
                turn_radius: serde_json::from_value(serde_json::json!(2f32.to_bits())).unwrap(),
                turn_type: 1,
            };
            let mut state =
                aa_actors::ground_states::MoverState::new(aa_actors::ActorId::from_raw(1), def, pc)
                    .unwrap();
            let mut flags = 0;
            input
                .apply(&mut state, Some(2.5f32.to_bits()), true, &mut flags, pc)
                .unwrap();
            for (offset, actual) in &state.words {
                let written = matches!(
                    *offset,
                    0x10 | 0x14 | 0x18 | 0x1c | 0x20 | 0x24 | 0x44 | 0x48
                ) || (input.version >= 3
                    && matches!(*offset, 0x38 | 0x3c | 0x40 | 0x50))
                    || (input.version >= 5 && *offset == 0x58)
                    || (input.version >= 6
                        && matches!(
                            *offset,
                            0x60 | 0x64 | 0x68 | 0x70 | 0x74 | 0x78 | 0x7c | 0x80 | 0x84
                        ));
                if written {
                    assert_eq!(
                        *actual,
                        case["native"]["words"][format!("{offset:#x}")]
                            .as_u64()
                            .unwrap() as u32,
                        "Mover {offset:#x}"
                    );
                }
            }
            assert_eq!(
                state.speed.0,
                case["native"]["words"]["0x28"].as_u64().unwrap() as u32
            );
            assert_eq!(
                flags,
                case["native"]["owner_flags"].as_u64().unwrap() as u32
            );
            for (offset, value) in [
                (0x4c, input.byte_4c),
                (0x54, input.byte_54),
                (0x56, input.byte_56),
                (0x5c, input.byte_5c),
                (0x6c, input.byte_6c),
            ] {
                if value.is_some() {
                    assert_eq!(
                        state.bytes[&offset],
                        case["native"]["bytes"][format!("{offset:#x}")]
                            .as_u64()
                            .unwrap() as u8
                    );
                }
            }
        }
    }
}
