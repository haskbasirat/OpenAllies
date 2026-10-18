use aa_body::{
    F32, Result,
    credit::Object,
    properties::PropertiesState,
    property_effects::{
        self, Definitions, Effects, Instance, Instances, NumericModifier, Selection,
    },
};
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
enum Operation {
    Insert(Instance),
    Remove(Selection),
}
struct Host {
    events: Vec<Value>,
    mutations: Vec<Value>,
    fail: bool,
}
impl Host {
    fn event(&mut self, event: Value, state: &mut PropertiesState) {
        self.events.push(event.clone());
        let name = event
            .as_str()
            .unwrap_or_else(|| event.as_object().unwrap().keys().next().unwrap());
        if self.mutations.first().is_some_and(|m| m["on"] == name) {
            let m = self.mutations.remove(0);
            if let Some(fields) = m["scalars"].as_object() {
                for (slot, value) in fields {
                    state.modifier.scalars[slot.parse::<usize>().unwrap()] =
                        F32(value.as_u64().unwrap() as u32);
                }
            }
        }
    }
}
impl Effects for Host {
    fn resolve_source(&self, id: u32) -> Result<Option<Object>> {
        Ok((id >> 16 == 1 && [1, 2].contains(&(id & 65535))).then_some(Object(id & 65535)))
    }
    fn allegiance(
        &mut self,
        state: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<Option<Object>> {
        self.event(json!("Allegiance"), state);
        Ok(Some(Object(1)))
    }
    fn numeric_changed(
        &mut self,
        m: &NumericModifier,
        state: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"NumericChanged":{"kind":m.kind,"index":m.index,"amount":m.amount,"repair":m.repair,"value":state.modifier.scalars[m.index as usize]}}),state);
        Ok(())
    }
    fn property_added(
        &mut self,
        handle: Object,
        state: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"PropertyAdded":{"handle":handle}}), state);
        if self.fail {
            return Err(aa_body::Error::Callback(
                "fixture property-added failure".into(),
            ));
        }
        Ok(())
    }
    fn actor_added(
        &mut self,
        handle: Object,
        state: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"ActorAdded":{"handle":handle}}), state);
        Ok(())
    }
    fn actor_removed(
        &mut self,
        handle: Object,
        state: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"ActorRemoved":{"handle":handle}}), state);
        Ok(())
    }
    fn property_removed(
        &mut self,
        handle: Object,
        state: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"PropertyRemoved":{"handle":handle}}), state);
        Ok(())
    }
    fn released(&mut self, instance: Instance) -> Result<()> {
        self.events
            .push(json!({"Released":{"handle":instance.handle}}));
        Ok(())
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-properties/native-effects-fixtures.json"
    ))
    .unwrap()
}

#[test]
fn native_property_numeric_lifetime_sequences() {
    let doc = fixture();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 111);
    for row in cases {
        let c = &row["input"];
        let definitions: Definitions = serde_json::from_value(c["definitions"].clone()).unwrap();
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let minimum = F32(c["minimum"].as_u64().unwrap() as u32);
        let mut state = PropertiesState::new(Object(1), 19).unwrap();
        let mut instances = Instances::new();
        let mut host = Host {
            events: vec![],
            mutations: serde_json::from_value(c["mutations"].clone()).unwrap(),
            fail: false,
        };
        let mut outputs = vec![];
        for op in c["operations"].as_array().unwrap() {
            match serde_json::from_value::<Operation>(op.clone()).unwrap() {
                Operation::Insert(instance) => {
                    property_effects::insert(
                        instance,
                        &mut state,
                        &mut instances,
                        &definitions,
                        minimum,
                        p,
                        &mut host,
                    )
                    .unwrap();
                }
                Operation::Remove(selection) => {
                    property_effects::remove(
                        selection,
                        &mut state,
                        &mut instances,
                        &definitions,
                        minimum,
                        p,
                        &mut host,
                    )
                    .unwrap();
                }
            }
            outputs.push(json!({"effects":state.effects,"scalars":state.modifier.scalars,"modifier_flags":state.modifier_flags,"instances":instances,"events":host.events}));
        }
        assert_eq!(json!(outputs), row["output"], "{}", c["name"]);
        assert!(host.mutations.is_empty());
    }
}

#[test]
fn property_callback_failure_keeps_applied_state_and_missing_records_fail() {
    let doc = fixture();
    let c = &doc["cases"][0]["input"];
    let definitions: Definitions = serde_json::from_value(c["definitions"].clone()).unwrap();
    let mut state = PropertiesState::new(Object(1), 19).unwrap();
    let mut instances = Instances::new();
    let mut host = Host {
        events: vec![],
        mutations: vec![],
        fail: true,
    };
    let instance: Instance = serde_json::from_value(c["operations"][0]["Insert"].clone()).unwrap();
    assert!(
        property_effects::insert(
            instance,
            &mut state,
            &mut instances,
            &definitions,
            F32(0.001f32.to_bits()),
            Precision::Double53,
            &mut host
        )
        .unwrap_err()
        .to_string()
        .contains("fixture")
    );
    assert_eq!(state.effects, vec![Object(1)]);
    assert_eq!(state.modifier.scalars[3], F32::ONE);
    assert_eq!(state.modifier_flags, 1);
    assert!(instances.contains_key(&Object(1)));
    assert!(host.events.last().unwrap().get("PropertyAdded").is_some());
    instances.clear();
    let modifier = &definitions[&Object(1)].modifiers[0];
    assert!(
        property_effects::recompute_numeric(
            modifier,
            &mut state,
            &instances,
            &definitions,
            F32(0.001f32.to_bits()),
            Precision::Double53
        )
        .is_err()
    );
}
