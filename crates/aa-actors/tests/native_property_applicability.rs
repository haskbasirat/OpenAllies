use aa_actors::{
    ActorId,
    state_effects::{self, Applicability, Result},
};
use aa_body::credit::Object;
use serde_json::{Value, json};
use std::cell::RefCell;
struct World {
    input: Value,
    events: RefCell<Vec<Value>>,
}
impl World {
    fn actor(&self, a: ActorId) -> &Value {
        &self.input["actors"][a.raw().to_string()]
    }
}
impl Applicability for World {
    fn has_properties(&self, a: ActorId) -> Result<bool> {
        Ok(self.actor(a)["properties"].as_bool().unwrap())
    }
    fn property_requirements(&self, _: Object) -> Result<Vec<Object>> {
        Ok(self.input["requires"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| Object(x.as_u64().unwrap() as u32))
            .collect())
    }
    fn actor_has_property(&self, a: ActorId, p: Object) -> Result<bool> {
        self.events
            .borrow_mut()
            .push(json!(["required", a.raw(), p.0]));
        Ok(self.actor(a)["available"]
            .as_array()
            .unwrap()
            .contains(&json!(p.0)))
    }
    fn organization_count(&self, a: ActorId) -> Result<Option<usize>> {
        self.events.borrow_mut().push(json!(["actor", a.raw()]));
        Ok(self.actor(a)["members"].as_array().map(Vec::len))
    }
    fn organization_member(&self, a: ActorId, i: usize) -> Result<Option<ActorId>> {
        Ok(self.actor(a)["members"][i]
            .as_u64()
            .map(|v| ActorId::from_raw(v as u32)))
    }
    fn vacant_member_definition(&self, a: ActorId, i: usize) -> Result<Option<Object>> {
        Ok(self.actor(a)["vacant"][i]
            .as_u64()
            .map(|v| Object(v as u32)))
    }
    fn definition_component_mask(&self, d: Object) -> Result<u32> {
        self.events.borrow_mut().push(json!(["definition", d.0]));
        Ok(self.input["definitions"][d.0.to_string()]["mask"]
            .as_u64()
            .unwrap() as u32)
    }
    fn definition_properties(&self, d: Object) -> Result<Vec<Object>> {
        Ok(self.input["definitions"][d.0.to_string()]["available"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| Object(x.as_u64().unwrap() as u32))
            .collect())
    }
}
#[test]
fn original_recursive_and_vacant_definition_applicability_matches() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actor-live/native-property-applicability.json"
    ))
    .unwrap();
    assert_eq!(doc["cases"].as_array().unwrap().len(), 128);
    for row in doc["cases"].as_array().unwrap() {
        let world = World {
            input: row["input"].clone(),
            events: RefCell::new(vec![]),
        };
        let result = state_effects::applicable(&world, ActorId::from_raw(1), Object(9)).unwrap();
        assert_eq!(
            json!({"result":result,"events":world.events.borrow().clone()}),
            row["output"]
        );
    }
}
#[test]
fn cyclic_runtime_membership_fails_explicitly() {
    let world = World {
        input: json!({"actors":{"1":{"members":[1],"vacant":[null]}}}),
        events: RefCell::new(vec![]),
    };
    assert!(matches!(
        state_effects::applicable(&world, ActorId::from_raw(1), Object(9)),
        Err(state_effects::Error::Cycle)
    ));
}
