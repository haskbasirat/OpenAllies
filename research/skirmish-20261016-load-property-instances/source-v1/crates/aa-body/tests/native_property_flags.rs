use aa_body::property_flags::{self as property_effects, Definitions, FlagModifier};
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
        if id == 0xfee1dead {
            return Err(aa_body::Error::Callback(
                "unavailable source lookup fixture".into(),
            ));
        }
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
        instances: &mut Instances,
    ) -> Result<()> {
        let source = self
            .mutations
            .first()
            .and_then(|v| v["instance_source"].as_u64());
        if let Some(source) = source {
            instances.get_mut(&Object(1)).unwrap().source_id = source as u32;
        }
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
        "../../../research/phase7-actor-live/native-property-flags.json"
    ))
    .unwrap()
}

#[test]
fn native_property_mixed_lifetime_sequences() {
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
            outputs.push(json!({"effects":state.effects,"scalars":state.modifier.scalars,"modifier_flags":state.modifier_flags,"instances":instances,"events":host.events,"flags":state.modifier.flags,"mover_byte_4c":u32::from(host.mover_byte)}));
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
fn native_flag_loader_and_registered_domain() {
    let doc = fixture();
    for row in doc["loader_cases"].as_array().unwrap() {
        let input = &row["input"];
        let index = if input["found"].as_bool().unwrap() {
            input["attribute"].as_i64().unwrap() as i32
        } else {
            -1
        };
        let result = FlagModifier::from_resolved(index);
        if row["output"]["error"].is_string() {
            assert!(result.is_err());
        } else {
            assert_eq!(
                result.unwrap().index,
                row["output"]["index"].as_u64().unwrap() as u32
            );
        }
    }
    assert_eq!(doc["registration"][11]["name"], "NOT_DISPOSABLE");
    assert_eq!(doc["registration"][11]["index"], 11);
}
#[test]
fn flag_failure_retains_attached_instance_and_changed_bit_before_later_descriptors() {
    let doc = fixture();
    let c = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["input"]["name"] == "mixed_order_Double53")
        .unwrap()["input"]
        .clone();
    let definitions: Definitions = serde_json::from_value(c["definitions"].clone()).unwrap();
    let mut state = PropertiesState::new(Object(1), 19).unwrap();
    let mut instances = Instances::new();
    let mut host = Host {
        events: vec![],
        mutations: vec![],
        fail: true,
        mover: true,
        character: true,
        economy: true,
        mover_byte: false,
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
        .is_err()
    );
    assert_eq!(state.effects, vec![Object(1)]);
    assert_eq!(state.modifier.flags, 1 << 11);
    assert_eq!(state.modifier.scalars[3], F32::ZERO);
    assert_eq!(state.modifier_flags, 0);
    assert!(instances.contains_key(&Object(1)));
    assert!(host.events.last().unwrap().get("Actions").is_some());
    instances.clear();
    assert!(
        property_effects::recompute_flag(
            FlagModifier { index: 11 },
            &mut state,
            &instances,
            &definitions
        )
        .is_err()
    );
}

#[test]
fn lazy_clamp_and_table_identity_native_prefixes() {
    use property_effects::{Definition, Modifier};
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-load-property-instances/native-lazy-clamp.json"
    ))
    .unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(), 15);
    for row in doc["rows"].as_array().unwrap() {
        let c = &row["input"];
        let p: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let modifiers: Vec<Modifier> = serde_json::from_value(c["mods"].clone()).unwrap();
        let definitions = Definitions::from([(
            Object(1),
            Definition {
                flags: 0,
                modifiers,
            },
        )]);
        let mut state = PropertiesState::new(Object(1), 19).unwrap();
        state.effects.push(Object(1));
        let mut instances = Instances::from([(
            Object(1),
            Instance::new(
                Object(1),
                Object(1),
                0,
                F32::ZERO,
                Some(c["source"].as_u64().unwrap() as u32),
            ),
        )]);
        let mut host = Host {
            events: vec![],
            mutations: if c["name"] == "live_periodic_source_after_callback" {
                vec![json!({"on":"NumericChanged","instance_source":0x10002})]
            } else {
                vec![]
            },
            fail: false,
            mover: false,
            character: false,
            economy: false,
            mover_byte: false,
        };
        let result = property_effects::add_modifiers(
            Object(1),
            &mut state,
            &mut instances,
            &definitions,
            p,
            &mut host,
        );
        assert_eq!(
            result.is_err(),
            !row["output"]["boundary"].is_null(),
            "{}",
            c["name"]
        );
        if let Err(e) = result {
            assert!(e.to_string().contains("current modifier clamp SVar"));
        }
        let out = json!({"scalars":state.modifier.scalars,"damage_factors":state.modifier.damage_factors,
            "periodic_heals":state.modifier.periodic_heals,"periodic_damage":state.modifier.periodic_damage,
            "source":instances[&Object(1)].source_id,"events":host.events,"boundary":row["output"]["boundary"]});
        assert_eq!(out, row["output"], "{} {:?}", c["name"], p);
    }
}
