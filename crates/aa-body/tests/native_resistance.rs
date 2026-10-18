use aa_body::property_flags::{self as property_effects, Definitions};
use aa_body::{
    F32, Result,
    credit::Object,
    properties::PropertiesState,
    property_effects::{Effects, Instance, Instances, NumericModifier, Selection},
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
    mover: bool,
    character: bool,
    economy: bool,
    mover_byte: bool,
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
        "../../../research/phase7-actor-live/native-resistance.json"
    ))
    .unwrap()
}

#[test]
fn native_resistance_mixed_lifetime_sequences() {
    let doc = fixture();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 66);
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
            mover: c["mover"].as_bool().unwrap(),
            character: c["character"].as_bool().unwrap(),
            economy: c["economy"].as_bool().unwrap(),
            mover_byte: false,
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
            outputs.push(json!({"effects":state.effects,"scalars":state.modifier.scalars,"modifier_flags":state.modifier_flags,"instances":instances,"events":host.events,"flags":state.modifier.flags,"mover_byte_4c":u32::from(host.mover_byte),"damage_factors":state.modifier.damage_factors}));
        }
        assert_eq!(json!(outputs), row["output"], "{}", c["name"]);
        assert!(host.mutations.is_empty());
    }
}

impl property_effects::Effects for Host {
    fn has_mover(&self, _: Object) -> Result<bool> {
        Ok(self.mover)
    }
    fn set_mover_byte_4c(
        &mut self,
        _: Object,
        v: bool,
        _: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.mover_byte = v;
        Ok(())
    }
    fn actor_stop_4d4c60(
        &mut self,
        _: Object,
        v: bool,
        s: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"ActorStop":{"enabled":v}}), s);
        Ok(())
    }
    fn has_character(&self, _: Object) -> Result<bool> {
        Ok(self.character)
    }
    fn character_pause_4daf20(
        &mut self,
        _: Object,
        s: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"Pause":{"flags":s.modifier.flags}}), s);
        Ok(())
    }
    fn character_resume_4daf40(
        &mut self,
        _: Object,
        s: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"Resume":{"flags":s.modifier.flags}}), s);
        Ok(())
    }
    fn mover_576800(
        &mut self,
        _: Object,
        v: bool,
        s: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"Mover576800":{"enabled":v}}), s);
        Ok(())
    }
    fn has_economy(&self, _: Object) -> Result<bool> {
        Ok(self.economy)
    }
    fn economy_570020(
        &mut self,
        _: Object,
        s: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"Economy":{"flags":s.modifier.flags}}), s);
        Ok(())
    }
    fn rebuild_actions_4d1350(
        &mut self,
        _: Object,
        s: &mut PropertiesState,
        _: &mut Instances,
    ) -> Result<()> {
        self.event(json!({"Actions":{"flags":s.modifier.flags}}), s);
        if self.fail {
            return Err(aa_body::Error::Callback(
                "fixture failure in action rebuild".into(),
            ));
        }
        Ok(())
    }
}

#[test]
fn missing_live_instance_preserves_prior_resistance_entry_allocation() {
    use aa_body::{
        property_flags::Modifier,
        property_resistance::{self, ResistDamage},
    };
    let mut state = PropertiesState::new(Object(1), 0).unwrap();
    state.effects.push(Object(8));
    let defs = Definitions::new();
    let instances = Instances::new();
    let modifier = ResistDamage {
        damage_type: Object(3),
        amount: F32(0.2f32.to_bits()),
    };
    assert!(
        property_resistance::recompute(
            &modifier,
            &mut state,
            &instances,
            &defs,
            F32(0.001f32.to_bits()),
            Precision::Double53,
            |m| match m {
                Modifier::Resistance(v) => Some(v),
                _ => None,
            }
        )
        .is_err()
    );
    assert_eq!(state.modifier.damage_factors.len(), 1);
    let entry = &state.modifier.damage_factors[0];
    assert_eq!(entry.damage_type, Object(3));
    assert_eq!(entry.factor, F32::ZERO);
    assert_eq!(entry.resistance, Some(F32::ZERO));
}
