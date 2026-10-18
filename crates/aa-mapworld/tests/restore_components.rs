use aa_actors::{
    ActorId,
    components::{ComponentSlot, Components, StateBits},
};
use aa_mapworld::{
    actor_payload::ActorTail,
    restore_components::{ComponentLoadAction, component_load_action},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct ActionFixture {
    cases: Vec<ActionCase>,
}
#[derive(Deserialize)]
struct ActionCase {
    slot: u8,
    saved: bool,
    default: bool,
    defined: bool,
    native_events: Vec<String>,
    native_present: bool,
    native_definition_diagnostic: bool,
}
#[test]
fn saved_component_actions_match_original_dispatch() {
    let f: ActionFixture = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-component-actions.json"
    ))
    .unwrap();
    assert_eq!(f.cases.len(), 248);
    for c in f.cases {
        let bit = 1 << c.slot;
        let got = component_load_action(
            ComponentSlot::from_index(c.slot as usize).unwrap(),
            c.saved,
            if c.default { bit } else { 0 },
            if c.defined { bit } else { 0 },
        );
        let (events, present, diagnostic) = match got {
            ComponentLoadAction::Skip => (vec![], false, false),
            ComponentLoadAction::Default => (vec!["construct"], true, false),
            ComponentLoadAction::Saved {
                remove_after_load: false,
            } => (vec!["construct", "load"], true, false),
            ComponentLoadAction::Saved {
                remove_after_load: true,
            } => (vec!["construct", "load", "delete"], false, false),
            ComponentLoadAction::DefinitionMismatch => (vec![], false, true),
        };
        assert_eq!(events, c.native_events, "slot {}", c.slot);
        assert_eq!(present, c.native_present);
        assert_eq!(diagnostic, c.native_definition_diagnostic);
    }
}
#[derive(Deserialize)]
struct Order {
    calls: Vec<u8>,
    bytes_read: usize,
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
#[test]
fn staged_callbacks_preserve_native_order_and_partial_progress() {
    let order: Order = serde_json::from_str(include_str!(
        "../../../research/phase7-restoration/native-component-order.json"
    ))
    .unwrap();
    assert_eq!(order.bytes_read, 8);
    let input = words(&[0x7fffffff, 0x7fffffff, u32::MAX]);
    let mut calls = vec![];
    let (_, n) =
        ActorTail::decode_prefix_with_stages(&input, 1, 0, |slot, saved, defaults, at, _| {
            assert!(saved);
            assert_eq!(defaults, 0x7fffffff);
            assert_eq!(at, 8);
            calls.push(slot);
            Ok((0, None))
        })
        .unwrap();
    assert_eq!(calls, order.calls);
    assert_eq!(n, input.len());
    calls.clear();
    let e = ActorTail::decode_prefix_with_stages(&input, 1, 0, |slot, _, _, _, _| {
        if slot == 3 {
            return Err(aa_mapworld::Error::Format(
                "fixture constructor failure".into(),
            ));
        }
        calls.push(slot);
        Ok((0, None))
    });
    assert!(e.is_err());
    assert_eq!(calls, [15, 0, 1, 2]);
    let input = words(&[0x7fffffff, 0, u32::MAX]);
    calls.clear();
    ActorTail::decode_prefix_with_stages(&input, 1, 0, |slot, saved, _, _, _| {
        assert!(!saved);
        calls.push(slot);
        Ok((0, None))
    })
    .unwrap();
    assert_eq!(calls, order.calls);
    assert!(
        ActorTail::decode_prefix_with_stages(&input, 1, 0, |_, _, _, _, _| Ok((1, None))).is_err()
    );
}
#[test]
fn constructor_state_is_installed_before_saved_load_failure() {
    use aa_mapworld::{
        restore_components::{
            BodyDefinition, GroundDefinitions, RestoredComponent, apply_component_stage,
        },
        saved_components::{BodyPayload, SavedComponent},
    };
    let definition = GroundDefinitions {
        body: Some(BodyDefinition {
            maximum_bits: 100f32.to_bits(),
            defense_bits: 0,
        }),
        ..Default::default()
    };
    let mut components = Components::new(StateBits::default());
    let mut flags = 0;
    let mut registrations = vec![];
    // NaN multiplication is rejected by the current exact numeric contract.
    // The already-created component and earlier dying-time write must remain.
    let input = SavedComponent::Body(BodyPayload {
        version: 1,
        updatable: None,
        dying_started_bits: Some(9f32.to_bits()),
        health_multiplier_bits: 0x7fc00000,
        sink_duration_bits: None,
        last_heal_time_bits: None,
    });
    let outcome = apply_component_stage(
        &mut components,
        aa_mapworld::restore_components::ComponentOwner {
            owner: None,
            owner_effects: &mut Vec::new(),
            register_updatable: &|_, _| Ok(()),
            updatable_registrations: &mut registrations,
            flags: &mut flags,
            id: ActorId::from_raw(1),
            world_time_bits: 0,
            position_bits: [0; 3],
        },
        ComponentSlot::Body,
        ComponentLoadAction::Saved {
            remove_after_load: false,
        },
        Some(&input),
        &definition,
        aa_numeric::Precision::Extended64,
    );
    assert!(outcome.is_err());
    let RestoredComponent::Body(body) = components.get(ComponentSlot::Body).unwrap() else {
        panic!("incorrect slot state")
    };
    assert_eq!(body.body.maximum.0, 100f32.to_bits());
    assert_eq!(body.body.dying_started.0, 9f32.to_bits());
}

#[test]
fn rejected_updatable_id_stops_remaining_saved_component_writes() {
    use aa_mapworld::{
        actor_payload::UpdatablePayload,
        restore_components::{
            BodyDefinition, ComponentOwner, GroundDefinitions, RestoredComponent,
            apply_component_stage,
        },
        saved_components::{BodyPayload, SavedComponent},
    };
    let definition = GroundDefinitions {
        body: Some(BodyDefinition {
            maximum_bits: 100f32.to_bits(),
            defense_bits: 0,
        }),
        ..Default::default()
    };
    let mut components = Components::new(StateBits::default());
    let mut flags = 0;
    let mut registrations = vec![];
    let input = SavedComponent::Body(BodyPayload {
        version: 2,
        updatable: Some(UpdatablePayload {
            version: 1,
            serialized_id: Some(7),
        }),
        dying_started_bits: Some(9f32.to_bits()),
        health_multiplier_bits: 0.5f32.to_bits(),
        sink_duration_bits: None,
        last_heal_time_bits: None,
    });
    let result = apply_component_stage(
        &mut components,
        ComponentOwner {
            owner: None,
            owner_effects: &mut Vec::new(),
            id: ActorId::from_raw(3),
            flags: &mut flags,
            world_time_bits: 0,
            position_bits: [0; 3],
            updatable_registrations: &mut registrations,
            register_updatable: &|id, target| {
                assert_eq!(id, 7);
                assert_eq!(
                    *target,
                    aa_mapworld::restore_components::UpdatableTarget::Component {
                        owner: 3,
                        slot: 0
                    }
                );
                Err("duplicate saved Updatable ID".into())
            },
        },
        ComponentSlot::Body,
        ComponentLoadAction::Saved {
            remove_after_load: false,
        },
        Some(&input),
        &definition,
        aa_numeric::Precision::Extended64,
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("duplicate saved Updatable ID")
    );
    assert!(registrations.is_empty());
    let RestoredComponent::Body(body) = components.get(ComponentSlot::Body).unwrap() else {
        panic!("incorrect component")
    };
    assert_eq!(body.body.health.0, 100f32.to_bits());
    assert_eq!(body.body.dying_started.0, (-1f32).to_bits());
}
