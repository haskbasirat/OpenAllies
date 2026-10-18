// Bounded, source-pinned base constructor fixtures; not live map actor creation.
pub fn check_actors(input: &serde_json::Value) -> anyhow::Result<Vec<serde_json::Value>> {
    use aa_actors::{creation::*, ActorId, IdentityRegistry};
    use aa_numeric::Precision;
    use aa_simulation::NativeRng;
    use serde_json::{json, Value};
    let mut rows = Vec::new();
    for sample in input["samples"].as_array().ok_or_else(|| anyhow::anyhow!("missing actor samples"))? {
        let c = &sample["input"];
        let u = |value: &Value| -> anyhow::Result<u32> {
            Ok(u32::try_from(value.as_u64().ok_or_else(|| anyhow::anyhow!("invalid actor word"))?)?)
        };
        let pc: Precision = serde_json::from_value(c["precision"].clone())?;
        let mut record = CreationRecord {
            definition: ThingDefinition { key: (), flags: u(&c["definition_flags"])? },
            position_bits: serde_json::from_value(c["position_bits"].clone())?,
            direction_bits: serde_json::from_value(c["direction_bits"].clone())?,
            name_utf16: serde_json::from_value(c["name_utf16"].clone())?,
            saved_id: ActorId::from_raw(u(&c["saved_id"])?),
        };
        let mut rng = NativeRng::new(u(&c["rng_state"])?);
        let table = DirectionTable::original(serde_json::from_value(c["startup_precision"].clone())?);
        let thing = construct(&mut record, u(&c["world_begin_bits"])?, &mut rng, table, pc)?;
        let mut constructor = serde_json::to_value(&thing)?;
        constructor.as_object_mut().unwrap().remove("definition");
        constructor["id"] = json!(0);
        let mut registry = IdentityRegistry::new();
        let id = initialize_identity(&mut registry, record.saved_id, false, thing)?;
        let actual = json!({"constructor": constructor, "record_direction_after": record.direction_bits,
            "rng_after": rng.state, "registered_id": id.raw(), "active": registry.len()});
        let expected = json!({"constructor": sample["constructor"], "record_direction_after": sample["record_direction_after"],
            "rng_after": sample["rng_after"], "registered_id": sample["registered_id"], "active": sample["registry"]["active"]});
        rows.push(json!({"name": format!("base_actor_creation_{}", sample["source_case_index"]),
            "passed": actual == expected, "actual": actual, "expected": expected}));
    }
    Ok(rows)
}
