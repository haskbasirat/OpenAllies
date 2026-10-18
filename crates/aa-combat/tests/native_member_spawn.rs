//! 0x57AD30 against research/phase8-combat/native-member-spawn-fixtures.json.
use aa_body::{F32, damage::DamageRecord};
use aa_combat::{
    Result,
    morale::Organization,
    regeneration::{CreationDescriptor, SpawnWorld, spawn_member},
};
use serde_json::{Value, json};

struct Adapter {
    case: Value,
    calls: Vec<Value>,
}
impl SpawnWorld for Adapter {
    fn slot_occupied(&self, _: &Organization, _: i32) -> Result<bool> {
        Ok(self.case["occupied"].as_bool().unwrap())
    }
    fn slot_definition_handle(&self, _: &Organization, _: i32) -> Result<u32> {
        Ok(self.case["definition"].as_u64().unwrap() as u32)
    }
    fn allegiance(&mut self, _: &mut Organization) -> Result<u32> {
        self.calls.push(json!("allegiance"));
        Ok(self.case["allegiance"].as_u64().unwrap() as u32)
    }
    fn create_actor(&mut self, d: &CreationDescriptor, owner: u32, slot: i32) -> Result<bool> {
        let mut words = vec![0u32; 21];
        words[0] = 0xa25b28;
        words[1] = d.definition;
        words[6] = d.word_18;
        words[8] = d.word_20;
        words[10] = d.allegiance;
        words[13] = d.word_34;
        words[20] = u32::from(d.flag) << 8;
        self.calls.push(json!({"create": {
            "owner": owner == 1, "slot": slot, "world": true,
            "name_empty": d.name.is_empty(), "words": words}}));
        Ok(self.case["result"].as_u64().unwrap() != 0)
    }
}

#[test]
fn member_spawn_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-member-spawn-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 40);
    for case in cases {
        let mut o = Organization {
            owner: 1,
            morale: F32::ZERO,
            maximum: F32::ZERO,
            breakpoint: F32::ZERO,
            living: 0,
            engaged_latch: false,
            engage_latch: false,
            attack_efficiency: F32::ZERO,
            attack_efficiency_desired: F32::ZERO,
            byte8d: false,
            last_damage: DamageRecord {
                source_id: 0,
                allegiance_id: 0,
                damage_type: 0,
                amount: F32::ZERO,
                time: F32::ZERO,
                event_kind: 0,
                reason: 0,
            },
            leader: None,
            primary: None,
            regroup_started: F32::ZERO,
            entrench_base: F32::ZERO,
            flare_pending: false,
            flare_word: 0,
            marker_pending: false,
            watched_target: 0,
            current_combat_value: F32::ZERO,
            maximum_combat_value: F32::ZERO,
            field_d8: F32::ZERO,
        };
        let mut world = Adapter {
            case: case.clone(),
            calls: vec![],
        };
        // The native stores the flag argument's low byte (+51).
        let flag = case["flag"].as_u64().unwrap() & 0xff != 0;
        let slot = case["slot"].as_i64().unwrap() as i32;
        let got = spawn_member(&mut o, slot, flag, &mut world).unwrap();
        assert_eq!(u64::from(got), case["returned"].as_u64().unwrap(), "{case}");
        // Word 7 (+1C) is the native CString pointer; its emptiness is compared.
        let mut native = case["calls"].clone();
        for call in native.as_array_mut().unwrap() {
            if let Some(words) = call.pointer_mut("/create/words") {
                words[7] = json!(0);
            }
        }
        assert_eq!(Value::Array(world.calls), native, "{case}");
    }
}
