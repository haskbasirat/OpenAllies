use aa_actors::{
    ActorId, IdentityRegistry,
    components::{ComponentSlot, Components, StateBits},
    ground_states::BodyState,
    owned::{self, Component, Definitions},
};
use aa_body::F32;
use aa_numeric::Precision;
use std::convert::Infallible;
#[test]
fn move_loaded_components_preserves_boxes_ids_generations_and_mutated_state() {
    let owner = ActorId::from_raw(0x000c0001);
    let mut registry = IdentityRegistry::from_load_metadata(10, 3, &[11, 12, 13]).unwrap();
    let mut body = BodyState::new(owner, F32(100f32.to_bits()), F32(3f32.to_bits())).unwrap();
    body.body.health = F32(17f32.to_bits());
    let value: Component = Component::Body(Box::new(body));
    let pointer = match &value {
        Component::Body(v) => (&**v) as *const BodyState as usize,
        _ => unreachable!(),
    };
    let mut slots = Components::new(StateBits {
        low: 0xf1234567,
        high: 0xabcdef,
    });
    slots.insert(ComponentSlot::Body, value);
    registry.register_saved(owner, true, slots).unwrap();
    let removed = ActorId::from_raw(0x000d0002);
    registry
        .register_saved(
            removed,
            false,
            Components::new(StateBits { low: 0, high: 0 }),
        )
        .unwrap();
    registry.remove(removed).unwrap();
    let prior_generations = (0..=u16::MAX)
        .map(|slot| registry.generation_at(slot))
        .collect::<Vec<_>>();
    let prior_words = registry.occupied_words().to_vec();
    let mut visited = vec![];
    let mut runtime = registry.map_values(|id, kind, components| {
        visited.push((id, kind));
        components.map_values(|slot, value| {
            assert_eq!(slot, value.slot());
            value.map_specialized(
                |x: Infallible| -> u32 { match x {} },
                |x: Infallible| -> String { match x {} },
            )
        })
    });
    assert_eq!(visited, [(owner, true)]);
    assert_eq!(runtime.next_slot(), 10);
    assert_eq!(runtime.highest_slot(), 3);
    assert_eq!(runtime.len(), 1);
    assert_eq!(runtime.occupied_words(), prior_words);
    assert_eq!(
        (0..=u16::MAX)
            .map(|s| runtime.generation_at(s))
            .collect::<Vec<_>>(),
        prior_generations
    );
    assert!(runtime.get(removed).is_none());
    let actual = &runtime.get(owner).unwrap().value;
    assert_eq!(actual.state.low, 0xf1234567);
    assert_eq!(actual.state.high, 0xabcdef);
    let Component::Body(body) = actual.get(ComponentSlot::Body).unwrap() else {
        panic!("body")
    };
    assert_eq!((&**body) as *const BodyState as usize, pointer);
    assert_eq!(body.body.health.0, 17f32.to_bits());
    assert_eq!(body.header.owner, owner);
    let id = runtime
        .allocate(true, Components::new(StateBits { low: 0, high: 0 }))
        .unwrap();
    assert_eq!(id.raw(), 10);
}
#[test]
fn typed_allocation_requires_real_definitions_and_keeps_missing_range() {
    let context = owned::Context {
        owner: ActorId::from_raw(0x10005),
        world_time: F32(4f32.to_bits()),
        position: [F32::ZERO; 3],
        precision: Precision::Double53,
    };
    let mut defs = Definitions::<String>::default();
    assert!(
        owned::construct::<_, Infallible, Infallible>(ComponentSlot::Body, &defs, context).is_err()
    );
    defs.mover = Some(aa_actors::ground_states::MoverDefinition {
        speed: F32(3f32.to_bits()),
        acceleration: F32::ONE,
        turn_radius: F32::ONE,
        turn_type: 0,
    });
    let Component::Mover(mover) =
        owned::construct::<_, Infallible, Infallible>(ComponentSlot::Mover, &defs, context)
            .unwrap()
    else {
        panic!("mover")
    };
    assert!(mover.movement_state().unwrap().target_range.is_none());
    assert!(matches!(
        owned::construct::<_, Infallible, Infallible>(ComponentSlot::Performer, &defs, context),
        Err(owned::Error::Specialized(ComponentSlot::Performer))
    ));
}

