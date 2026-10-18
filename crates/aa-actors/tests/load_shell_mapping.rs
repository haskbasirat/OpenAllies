use aa_actors::{
    components::ComponentSlot,
    loading::{LoadKind, construct_shell_with_components},
};
#[test]
fn loaded_shell_mapping_moves_existing_component_and_preserves_saved_state() {
    let mut shell = construct_shell_with_components(LoadKind::Actor, "source definition");
    shell.thing.flags = 0x12345678;
    shell.thing.position_bits = [1, 2, 3];
    shell.object.as_mut().unwrap().variant = 7;
    let actor = shell.actor.as_mut().unwrap();
    actor.components.state.low = 0x87654321;
    actor.string_dc_utf16 = vec![65, 0xd800];
    actor.words.insert(0xe8, 0x10007);
    let node = actor.actor_list_108.allocate().unwrap();
    actor
        .actor_list_108
        .set_value(node, "canonical build target".to_owned())
        .unwrap();
    actor.actor_list_108.set_next(node, None).unwrap();
    actor.actor_list_108.set_head(Some(node));
    actor.actor_list_108.set_tail(Some(node));
    let list_value_before = actor.actor_list_108.value(node).unwrap().as_ptr();
    let overlap = actor
        .overlap_list_110
        .prepend(aa_actors::overlap_list::Entry {
            reference: 0x10008,
            expiry_bits: 17.0f32.to_bits(),
        })
        .unwrap();
    let overlap_before = actor.overlap_list_110.node(overlap).unwrap() as *const _;
    actor.zones_148.zones_mut().slots = vec![None; 9];
    actor.zones_148.zones_mut().slots[6] = Some(aa_zones::zone::Zone::new(71, 0x10008, 0x40));
    let zones_before = actor.zones_148.zones().slots.as_ptr();
    let value = Box::new(17u32);
    let before = value.as_ref() as *const u32;
    actor.components.insert(ComponentSlot::Properties, value);
    let mapped = shell.map_components(|slot, value| (slot, value));
    assert_eq!(mapped.kind, LoadKind::Actor);
    assert_eq!(mapped.thing.definition, "source definition");
    assert_eq!(mapped.thing.flags, 0x12345678);
    assert_eq!(mapped.thing.position_bits, [1, 2, 3]);
    assert_eq!(mapped.object.unwrap().variant, 7);
    let actor = mapped.actor.unwrap();
    assert_eq!(actor.components.state.low, 0x87654321);
    assert_eq!(actor.string_dc_utf16, [65, 0xd800]);
    assert_eq!(actor.words[&0xe8], 0x10007);
    assert_eq!(actor.actor_list_108.head(), Some(node));
    assert_eq!(actor.actor_list_108.tail(), Some(node));
    assert_eq!(actor.actor_list_108.allocated_nodes(), 1);
    assert_eq!(
        actor.actor_list_108.value(node).unwrap().as_ptr(),
        list_value_before
    );
    assert_eq!(actor.overlap_list_110.head(), Some(overlap));
    assert_eq!(actor.overlap_list_110.tail(), Some(overlap));
    assert_eq!(
        actor.overlap_list_110.node(overlap).unwrap() as *const _,
        overlap_before
    );
    let (slot, value) = actor.components.get(ComponentSlot::Properties).unwrap();
    assert_eq!(actor.zones_148.native_capacity(), 9);
    assert_eq!(actor.zones_148.zones().slots.as_ptr(), zones_before);
    assert_eq!(actor.zones_148.zones().slots[6].unwrap().key, 71);
    for offset in [0x148, 0x14c, 0x150] {
        assert!(!actor.words.contains_key(&offset));
    }
    assert_eq!(*slot, ComponentSlot::Properties);
    assert_eq!(value.as_ref() as *const u32, before);
    assert_eq!(**value, 17);
}
