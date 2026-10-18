#[path="../../aa-runtime/src/zone_primary_reference.rs"]
mod production;
use aa_actors::{ActorId,IdentityRegistry};
#[test]
fn native45bc70_resolves_against_actual_current_registry_generations() {
    let f:serde_json::Value=serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-city-after-init/native-primary-reference.json")).unwrap();
    let rows=f["rows"].as_array().unwrap();assert_eq!(rows.len(),102);
    for row in rows {
        let i=&row["input"];let reference=i["reference"].as_u64().unwrap()as u32;
        let mut registry=IdentityRegistry::new();
        if reference!=0 {
            let actual=ActorId::from_raw((i["slot_generation"].as_u64().unwrap()as u32)<<16|u32::from(ActorId::from_raw(reference).slot()));
            registry.seed_saved_generation(actual).unwrap();
            if i["present"].as_bool().unwrap(){registry.register_saved(actual,true,"same actual held allocation").unwrap();}
        }
        let value=production::resolve(reference,|raw|registry.get(ActorId::from_raw(raw)).is_some());
        assert_eq!(value,row["native"]["logical"].as_u64().unwrap()as u32);
    }
    let mut registry=IdentityRegistry::new();let id=registry.allocate(true,"owned").unwrap();
    assert_eq!(production::resolve(id.raw(),|r|registry.get(ActorId::from_raw(r)).is_some()),id.raw());
    registry.remove(id).unwrap();
    assert_eq!(production::resolve(id.raw(),|r|registry.get(ActorId::from_raw(r)).is_some()),0);
}
