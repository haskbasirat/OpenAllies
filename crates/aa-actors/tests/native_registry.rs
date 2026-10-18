use aa_actors::registry::{
    ActorId, Entry, IdentityRegistry, NATIVE_ADD_LIMIT, RegistryCheckpoint, SLOT_COUNT,
};
use serde_json::{Value, json};

fn u32_field(v: &Value, key: &str) -> u32 {
    v[key].as_u64().unwrap() as u32
}
fn snapshot(r: &IdentityRegistry<u32>) -> Value {
    json!({"active":r.len(),"next_slot":r.next_slot(),"highest_slot":r.highest_slot(),
        "generations":(0..SLOT_COUNT).filter_map(|i|{let generation=r.generation_at(i as u16);(generation!=0).then_some([i as u32,u32::from(generation)])}).collect::<Vec<_>>(),
        "entries":r.iter().map(|e|json!({"slot":e.id.slot(),"id":e.id.raw(),"key":e.value,"is_actor":e.is_actor})).collect::<Vec<_>>(),
        "occupied":r.occupied_words().iter().enumerate().filter_map(|(i,&b)|(b!=0).then_some([i as u32,b])).collect::<Vec<_>>()})
}

#[test]
fn original_registry_operation_sequences_match_every_state_word() {
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/phase4-actors/trace_registry.json"
    ))
    .unwrap();
    assert_eq!(proof["passed"], true);
    let mut count = 0;
    for case in proof["cases"].as_array().unwrap() {
        let initial = &case["initial"];
        let mut generations = vec![0; SLOT_COUNT];
        if let Some(rows) = initial["generations"].as_array() {
            for row in rows {
                generations[row[0].as_u64().unwrap() as usize] = row[1].as_u64().unwrap() as u16;
            }
        }
        let entries = initial["entries"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .map(|e| Entry {
                        id: ActorId::from_raw(u32_field(e, "id")),
                        is_actor: e["is_actor"].as_bool().unwrap(),
                        value: u32_field(e, "key"),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut registry = IdentityRegistry::from_checkpoint(RegistryCheckpoint {
            generations,
            entries,
            next_slot: initial["next_slot"].as_u64().unwrap_or(1) as u16,
            highest_slot: initial["highest_slot"].as_i64().unwrap_or(0) as i32,
        })
        .unwrap();
        for step in case["steps"].as_array().unwrap() {
            count += 1;
            let op = &step["operation"];
            let mut error = None;
            let mut result = Value::Null;
            match op["op"].as_str().unwrap() {
                "allocate" => match registry
                    .allocate(op["is_actor"].as_bool().unwrap(), u32_field(op, "key"))
                {
                    Ok(id) => result = json!(id.raw()),
                    Err(e) => {
                        error = Some(format!("{e:?}"));
                        result = json!(0);
                    }
                },
                "seed" => {
                    if let Err(e) =
                        registry.seed_saved_generation(ActorId::from_raw(u32_field(op, "id")))
                    {
                        error = Some(format!("{e:?}"));
                    }
                }
                "register" => {
                    if let Err(e) = registry.register_saved(
                        ActorId::from_raw(u32_field(op, "id")),
                        op["is_actor"].as_bool().unwrap(),
                        u32_field(op, "key"),
                    ) {
                        error = Some(format!("{e:?}"));
                    }
                }
                "remove" => {
                    let id = registry
                        .iter()
                        .find(|e| e.value == u32_field(op, "key"))
                        .unwrap()
                        .id;
                    registry.remove(id).unwrap();
                }
                "lookup" => {
                    result = json!(
                        registry
                            .get_actor(ActorId::from_raw(u32_field(op, "id")))
                            .map(|e| e.value)
                    )
                }
                "swap" => {
                    let a = registry
                        .iter()
                        .find(|e| e.value == u32_field(op, "a"))
                        .unwrap()
                        .id;
                    let b = registry
                        .iter()
                        .find(|e| e.value == u32_field(op, "b"))
                        .unwrap()
                        .id;
                    registry.swap_identities(a, b).unwrap();
                }
                "enumerate" => {
                    result = json!(
                        registry
                            .iter()
                            .map(|e| json!({"id":e.id.raw(),"key":e.value}))
                            .collect::<Vec<_>>()
                    )
                }
                other => panic!("{other}"),
            }
            assert_eq!(result, step["result"], "{} {op}", case["name"]);
            assert_eq!(json!(error), step["error"], "{} {op}", case["name"]);
            assert_eq!(snapshot(&registry), step["state"], "{} {op}", case["name"]);
        }
    }
    assert_eq!(count, proof["operation_count"].as_u64().unwrap());
}

#[test]
fn capacity_gate_checkpoint_validation_and_stale_mutation_are_explicit() {
    let mut r = IdentityRegistry::new();
    for i in 0..NATIVE_ADD_LIMIT {
        r.allocate(true, i).unwrap();
    }
    assert!(r.allocate(true, usize::MAX).is_err());
    let before = r.checkpoint();
    let restored = IdentityRegistry::from_checkpoint(before.clone()).unwrap();
    assert_eq!(before, restored.checkpoint());
    let mut bad = before;
    bad.next_slot = 0;
    assert!(IdentityRegistry::from_checkpoint(bad).is_err());
    let id = ActorId::from_raw(1);
    r.remove(id).unwrap();
    assert!(r.value_mut(id).is_none());
    assert!(r.remove(id).is_err());
}
