use aa_actors::IdentityRegistry;
use serde_json::Value;

#[test]
fn original_id_manager_load_preserves_empty_registry_high_water_and_generation_prefix() {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-mapworld/native-load-fixtures.json"
    ))
    .unwrap();
    let mut count = 0;
    for row in proof["id_manager_cases"].as_array().unwrap() {
        if row.get("next_slot").is_none() {
            continue;
        }
        let hex = row["input_hex"].as_str().unwrap();
        let raw: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let generations: Vec<u16> = raw[16..]
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let next = row["next_slot"].as_u64().unwrap() as u32;
        let highest = row["highest_slot"].as_u64().unwrap() as u32;
        let mut registry =
            IdentityRegistry::<()>::from_load_metadata(next, highest, &generations).unwrap();
        assert_eq!(u32::from(registry.next_slot()), next);
        assert_eq!(registry.highest_slot(), highest as i32);
        assert!(registry.is_empty());
        for i in 0..65536 {
            assert_eq!(
                registry.generation_at(i as u16),
                generations.get(i).copied().unwrap_or(0)
            );
        }
        // Restored registration must validate the loaded generation, never
        // overwrite it by the separate new-creation saved-ID seeding helper.
        let slot = if highest > 1 { 1 } else { 65535 };
        let id = aa_actors::ActorId::from_raw(
            (u32::from(registry.generation_at(slot)) << 16) | u32::from(slot),
        );
        registry.register_saved(id, true, ()).unwrap();
        assert!(registry.get_actor(id).is_some());
        count += 1;
    }
    assert_eq!(count, 6);
    assert!(IdentityRegistry::<()>::from_load_metadata(0, 0, &[]).is_err());
    assert!(IdentityRegistry::<()>::from_load_metadata(1, 65536, &[]).is_err());
    assert!(IdentityRegistry::<()>::from_load_metadata(1, 1, &[]).is_err());
}
