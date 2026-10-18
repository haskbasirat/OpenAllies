use aa_body::{
    Error, F32, Result,
    credit::Object,
    experience::{self, Actor, Request, ValueFields, World},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::{cmp::Ordering, collections::BTreeMap};

fn extended(row: &Value) -> Binary {
    let sig = row["extended_significand"].as_u64().unwrap();
    let se = row["extended_sign_exponent"].as_u64().unwrap() as u16;
    let p = Precision::Extended64;
    let mut value = Binary::from_u32((sig >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap(), p)
        .unwrap()
        .add(&Binary::from_u32(sig as u32), p)
        .unwrap();
    if sig != 0 {
        let exponent = i32::from(se & 0x7fff) - 16383 - 63;
        value = value
            .mul(
                &Binary::from_f64_bits(((exponent + 1023) as u64) << 52).unwrap(),
                p,
            )
            .unwrap();
    }
    if se & 0x8000 != 0 {
        value = value.negated();
    }
    value
}

struct Host {
    actors: BTreeMap<Object, Actor>,
    regiments: BTreeMap<Object, Vec<Option<Object>>>,
    mutations: Vec<Value>,
    events: Vec<Value>,
    fail: bool,
}
impl Host {
    fn new(c: &Value) -> Self {
        Self {
            actors: serde_json::from_value(c["actors"].clone()).unwrap(),
            regiments: serde_json::from_value(c["regiments"].clone()).unwrap(),
            mutations: serde_json::from_value(c["mutations"].clone()).unwrap(),
            events: vec![],
            fail: false,
        }
    }
}
impl World for Host {
    fn actor(&self, actor: Object) -> Result<Actor> {
        self.actors
            .get(&actor)
            .copied()
            .ok_or_else(|| Error::Callback("missing actor".into()))
    }
    fn regiment_count(&self, regiment: Object) -> Result<i32> {
        Ok(self.regiments[&regiment].len() as i32)
    }
    fn regiment_member(&self, regiment: Object, index: i32) -> Result<Option<Object>> {
        Ok(self.regiments[&regiment][index as usize])
    }
    fn apply(&mut self, component: Object, amount: F32, argument: bool) -> Result<()> {
        self.events
            .push(json!({"component":component,"amount":amount,"argument":argument}));
        if self.fail {
            return Err(Error::Callback("fixture apply failure".into()));
        }
        if self
            .mutations
            .first()
            .is_some_and(|m| m["on_component"] == component.0)
        {
            let m = self.mutations.remove(0);
            if let Some(changes) = m["actors"].as_object() {
                for (key, changes) in changes {
                    let key = Object(key.parse().unwrap());
                    let mut actor = serde_json::to_value(self.actors[&key]).unwrap();
                    for (name, value) in changes.as_object().unwrap() {
                        actor[name] = value.clone();
                    }
                    self.actors
                        .insert(key, serde_json::from_value(actor).unwrap());
                }
            }
            if let Some(changes) = m["regiments"].as_object() {
                for (key, values) in changes {
                    self.regiments.insert(
                        Object(key.parse().unwrap()),
                        serde_json::from_value(values.clone()).unwrap(),
                    );
                }
            }
        }
        Ok(())
    }
}

#[test]
fn original_component_value_and_experience_routing() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-experience/native-experience-fixtures.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 249);
    for row in rows {
        let c = &row["input"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        if c["method"] == "value" {
            let fields: ValueFields = serde_json::from_value(c["fields"].clone()).unwrap();
            let actual = experience::actor_value(&fields, p).unwrap();
            assert_eq!(
                actual.compare(&extended(&row["output"])),
                Ordering::Equal,
                "{} extended",
                c["name"]
            );
            assert_eq!(
                actual.store_f32().unwrap(),
                row["output"]["stored"].as_u64().unwrap() as u32,
                "{} store",
                c["name"]
            );
        } else {
            let mut host = Host::new(c);
            let request: Request = serde_json::from_value(c["request"].clone()).unwrap();
            experience::award(Object(1), request, &mut host).unwrap();
            assert_eq!(
                json!({"events":host.events,"actors":host.actors,"regiments":host.regiments,"abi_preserved":true}),
                row["output"],
                "{}",
                c["name"]
            );
            assert!(host.mutations.is_empty());
        }
    }
}

#[test]
fn invalid_graph_and_failed_application_are_explicit() {
    let actor = Actor {
        experience_component: None,
        regiment: Some(Object(10)),
        organization_leader: None,
    };
    let mut host = Host {
        actors: BTreeMap::from([(Object(1), actor)]),
        regiments: BTreeMap::from([(Object(10), vec![Some(Object(1))])]),
        mutations: vec![],
        events: vec![],
        fail: false,
    };
    let request = Request {
        amount: F32::ONE,
        argument1: true,
        argument2: false,
    };
    assert!(
        experience::award(Object(1), request, &mut host)
            .unwrap_err()
            .to_string()
            .contains("cyclic")
    );
    assert!(host.events.is_empty());
    host.actors
        .get_mut(&Object(1))
        .unwrap()
        .experience_component = Some(Object(20));
    host.fail = true;
    assert!(
        experience::award(Object(1), request, &mut host)
            .unwrap_err()
            .to_string()
            .contains("fixture apply failure")
    );
    assert_eq!(host.events.len(), 1);
    assert!(
        experience::actor_value(
            &ValueFields {
                regiment: None,
                wall: Some(F32(0x7f80_0000)),
                combat: None,
                component88: None
            },
            Precision::Double53
        )
        .is_err()
    );
}
