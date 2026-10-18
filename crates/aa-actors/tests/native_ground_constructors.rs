use aa_actors::{ActorId, ground_states::*};
use aa_body::F32;
use aa_numeric::Precision;
use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn original_ground_constructor_initialized_fields_match() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actors/component-constructors-native.json"
    ))
    .unwrap();
    let rows = document["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 336);
    for row in rows {
        let input = &row["input"];
        let word = |name: &str| F32(input[name].as_u64().unwrap() as u32);
        let precision: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let owner = ActorId::from_raw(0x12340001);
        let mut fields = BTreeMap::<u32, u32>::new();
        fields.insert(0xc, 0);
        let mut bytes = BTreeMap::<u32, u8>::new();
        let excluded: Vec<u32>;
        match row["kind"].as_str().unwrap() {
            "Body" => {
                let state = BodyState::new(owner, word("health"), word("defense")).unwrap();
                assert_eq!(state.header.owner, owner);
                assert!(state.header.scheduled_head.is_none());
                let b = state.body;
                for (offset, value) in [
                    (0x10, b.health.0),
                    (0x14, b.maximum.0),
                    (0x18, b.defense.0),
                    (0x1c, b.dying_started.0),
                    (0x20, b.last_heal_time.0),
                    (0x24, b.sink_duration.0),
                ] {
                    fields.insert(offset, value);
                }
                for (i, value) in b.last_damage.iter().enumerate() {
                    fields.insert(0x28 + i as u32 * 4, *value);
                }
                excluded = vec![0, 4, 8];
            }
            "Mover" => {
                let state = MoverState::new(
                    owner,
                    MoverDefinition {
                        speed: word("speed"),
                        acceleration: word("acceleration"),
                        turn_radius: word("turn_radius"),
                        turn_type: input["turn_type"].as_i64().unwrap() as i32,
                    },
                    precision,
                )
                .unwrap();
                assert_eq!(state.header.owner, owner);
                fields.extend(state.words);
                bytes = state.bytes;
                fields.insert(0x28, state.speed.0);
                fields.insert(0x2c, state.acceleration.0);
                fields.insert(0x30, state.turn_radius.0);
                if let Some(value) = state.angular_speed {
                    fields.insert(0x34, value.0);
                }
                excluded = vec![0, 4, 8];
            }
            "Character" => {
                let animations: Vec<u32> =
                    serde_json::from_value(input["character_animations"].clone()).unwrap();
                let state = CharacterState::new(
                    owner,
                    &animations.into_iter().map(Some).collect::<Vec<_>>(),
                );
                for (offset, value) in [
                    (0x14, state.elapsed.0),
                    (0x18, state.resumed_at.0),
                    (0x1c, state.rate.0),
                    (0x20, state.animation.unwrap_or(0)),
                    (0x28, state.word_28),
                    (0x2c, state.variant as u32),
                ] {
                    fields.insert(offset, value);
                }
                bytes.insert(0x24, u8::from(state.active));
                excluded = vec![0, 4, 8, 0x10];
            }
            "Element" => {
                let state: ElementState<u32> = ElementState::new(owner).unwrap();
                assert!(
                    state.organization.is_none()
                        && state.element_definition.is_none()
                        && state.history.is_empty()
                );
                for (offset, value) in [
                    (0x10, 0),
                    (0x14, 0),
                    (0x18, state.slot_index as u32),
                    (0x20, state.word_20),
                    (0x24, state.time_24.0),
                    (0x2c, 0),
                    (0x30, 1),
                ] {
                    fields.insert(offset, value);
                }
                bytes.insert(0x1c, u8::from(state.flag_1c));
                excluded = vec![0, 4, 8, 0x28];
            }
            "Organization" => {
                let parameters: Vec<u32> =
                    serde_json::from_value(input["layout_parameters"].clone()).unwrap();
                let count = parameters.len();
                let slots = input["layout_slots"].as_u64().unwrap() as usize;
                let position: Vec<F32> = serde_json::from_value(input["position"].clone()).unwrap();
                let state = OrganizationState::new(
                    owner,
                    LayoutDefinition {
                        definition: 1,
                        parameters: parameters.iter().copied().map(Some).collect(),
                        slots,
                    },
                    word("world_time"),
                    word("organization_value"),
                    [position[0], position[1]],
                )
                .unwrap();
                assert!(
                    state.members.is_empty()
                        && state.words_34.is_empty()
                        && state.words_50.is_empty()
                        && state.offsets_78.is_empty()
                );
                assert_eq!(
                    parameters,
                    serde_json::from_value::<Vec<u32>>(
                        row["output"]["vectors"]["layout_parameters"].clone()
                    )
                    .unwrap()
                );
                fields.extend(state.words);
                bytes = state.bytes;
                for (offset, value) in [
                    (0x14, state.layout_word_14),
                    (0x20, count as u32),
                    (0x24, count as u32),
                    (0x2c, 0),
                    (0x30, slots as u32),
                    (0x38, 0),
                    (0x3c, slots as u32),
                    (0x54, 0),
                    (0x58, slots as u32),
                    (0x7c, 0),
                    (0x80, slots as u32),
                    (0xb4, state.created_at.0),
                    (0xb8, state.definition_value_308.0),
                    (0xbc, state.position_xy[0].0),
                    (0xc0, state.position_xy[1].0),
                ] {
                    fields.insert(offset, value);
                }
                excluded = vec![0, 4, 8, 0x10, 0x18, 0x1c, 0x28, 0x34, 0x50, 0x78];
            }
            "Economy" => {
                let count = input["resource_count"].as_u64().unwrap() as usize;
                let state = EconomyState::new(owner, count).unwrap();
                bytes.insert(0x10, u8::from(state.flag_10));
                bytes.insert(0x11, u8::from(state.flag_11));
                for (index, resource) in state.resources.iter().enumerate() {
                    let base = 0x14 + index as u32 * 0x10;
                    fields.insert(base + 8, resource.values.len() as u32);
                    fields.insert(base + 12, resource.initial_capacity as u32);
                    let original: Vec<u32> = serde_json::from_value(
                        row["output"]["vectors"][format!("resources_{base:x}")].clone(),
                    )
                    .unwrap();
                    assert_eq!(
                        resource.values.iter().map(|f| f.0).collect::<Vec<_>>(),
                        original
                    );
                }
                excluded = vec![0, 4, 8, 0x14, 0x18, 0x24, 0x28, 0x34, 0x38, 0x44, 0x48];
            }
            "Detection" => {
                let state = DetectionState::new(owner, word("detection_radius"));
                fields.insert(0x10, state.word_10);
                fields.insert(0x14, state.radius.0);
                excluded = vec![0, 4, 8];
            }
            _ => unreachable!(),
        }
        let native: BTreeMap<u32, u32> =
            serde_json::from_value(row["output"]["initialized_words"].clone()).unwrap();
        let native: BTreeMap<_, _> = native
            .into_iter()
            .filter(|(offset, _)| !excluded.contains(offset))
            .collect();
        assert_eq!(fields, native, "{} {precision:?}", row["kind"]);
        assert_eq!(
            bytes,
            serde_json::from_value(row["output"]["initialized_bytes"].clone()).unwrap()
        );
    }
}

#[test]
fn unsupported_mover_angular_state_and_checked_zero_radius() {
    let definition = MoverDefinition {
        speed: F32::ONE,
        acceleration: F32::ZERO,
        turn_radius: F32::ZERO,
        turn_type: 3,
    };
    let state = MoverState::new(ActorId::from_raw(1), definition, Precision::Double53).unwrap();
    assert!(state.require_angular_speed().is_err());
    assert!(
        MoverState::new(
            ActorId::from_raw(1),
            MoverDefinition {
                turn_type: 1,
                ..definition
            },
            Precision::Double53
        )
        .is_err()
    );
    assert_eq!(
        MoverState::new(
            ActorId::from_raw(1),
            MoverDefinition {
                turn_type: 0,
                ..definition
            },
            Precision::Double53
        )
        .unwrap()
        .require_angular_speed()
        .unwrap(),
        F32::ZERO
    );
}

#[test]
fn movement_adapter_preserves_uninitialized_range_and_unrelated_native_fields() {
    let mut original = MoverState::new(
        ActorId::from_raw(7),
        MoverDefinition {
            speed: F32(4f32.to_bits()),
            acceleration: F32::ONE,
            turn_radius: F32::ONE,
            turn_type: 1,
        },
        Precision::Double53,
    )
    .unwrap();
    let before = original.clone();
    let mut view = original.movement_state().unwrap();
    assert!(view.target_range.is_none());
    view.destination = [4.0, 2.0, 0.0];
    view.requested_speed = 3.0;
    original.apply_movement_state(&view);
    assert!(!original.words.contains_key(&0x48));
    assert_eq!(original.speed, before.speed);
    assert_eq!(original.words.get(&0x60), before.words.get(&0x60));
    assert_eq!(original.header, before.header);
    assert_eq!(original.words[&0x10], 4f32.to_bits());
    assert_eq!(original.words[&0x24], 3f32.to_bits());
    view.target_range = Some(2.5);
    original.apply_movement_state(&view);
    assert_eq!(original.words[&0x48], 2.5f32.to_bits());
}
