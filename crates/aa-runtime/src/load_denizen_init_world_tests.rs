use super::*;
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[test]
fn original_source_behavior_tokenizer_matches_all63_rows() {
    let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-denizen-init/native-denizen-behaviors.json")).unwrap();
    let rows=doc["cases"].as_array().unwrap();assert_eq!(rows.len(),63);
    for row in rows {
        let result=aa_rules::native::denizen::native_behavior_flags(row["input"].as_str().unwrap());
        if row["output"]["stopped"].is_null(){assert_eq!(u64::from(result.unwrap()),row["output"]["flags"],"{row}")}
        else{assert!(result.is_err(),"{row}")}
    }
}
struct State {
    input: Value,
    stop: Option<String>,
    stopped: Option<String>,
    counts: BTreeMap<String, u32>,
    events: Vec<Value>,
    mask: u32,
    last_list: u32,
}
#[derive(Clone)]
struct Probe(Rc<RefCell<State>>);
impl Probe {
    fn event(&self, label: &str, payload: Value) -> Result<()> {
        let mut s = self.0.borrow_mut();
        let count = s.counts.entry(label.into()).or_default();
        let key = format!("{label}:{count}");
        *count += 1;
        if s.stop.as_ref() == Some(&key) {
            s.stopped = Some(key.clone());
            return Err(Error::World(key));
        }
        s.events.push(json!([key, payload]));
        Ok(())
    }
    fn number(&self, key: &str) -> u32 {
        self.0.borrow().input[key].as_u64().unwrap() as u32
    }
    fn values(&self, list: u32) -> Vec<u32> {
        self.0.borrow().input[if list == 0x34c {
            "organization"
        } else {
            "groups"
        }]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect()
    }
    fn condition(&self) -> Result<bool> {
        self.event("guard", Value::Null)?;
        if f32::from_bits(self.number("guard")) > 0.0 {
            return Ok(true);
        }
        self.event("inventory", Value::Null)?;
        Ok(self.0.borrow().input["inventory"].as_bool().unwrap())
    }
}
impl MaskWorld for Probe {
    type Node = (u32, usize);
    fn head(&mut self, list: u32) -> Result<Option<Self::Node>> {
        self.event(&format!("head{list:x}"), Value::Null)?;
        Ok((!self.values(list).is_empty()).then_some((list, 0)))
    }
    fn value(&mut self, (list, index): Self::Node) -> Result<u32> {
        self.event(&format!("value{list:x}"), Value::Null)?;
        self.0.borrow_mut().last_list = list;
        Ok(self.values(list)[index])
    }
    fn next(&mut self, (list, index): Self::Node) -> Result<Option<Self::Node>> {
        self.event(&format!("next{list:x}"), Value::Null)?;
        Ok((index + 1 < self.values(list).len()).then_some((list, index + 1)))
    }
    fn or_mask(&mut self, v: u32) -> Result<()> {
        let list = self.0.borrow().last_list;
        self.event(&format!("or{list:x}"), Value::Null)?;
        self.0.borrow_mut().mask |= v;
        Ok(())
    }
}
impl crate::load_detection_init_world::World for Probe {
    fn version(&mut self) -> Result<u32> {
        self.event("version", Value::Null)?;
        Ok(self.number("version"))
    }
    fn pending(&mut self) -> Result<bool> {
        self.event("pending", Value::Null)?;
        Ok(self.0.borrow().input["pending"].as_bool().unwrap())
    }
    fn add_periodic(&mut self) -> Result<()> {
        self.event("queue", json!([1, 0]))
    }
}
impl AfterWorld for Probe {
    fn guard(&mut self) -> Result<u32> {
        self.event("guard", Value::Null)?;
        Ok(self.number("guard"))
    }
    fn flags(&mut self) -> Result<u32> {
        self.event("flags", Value::Null)?;
        Ok(self.number("flags"))
    }
    fn create(&mut self) -> Result<bool> {
        self.event("create", json!(5))?;
        Ok(self.0.borrow().input["create_result"].as_bool().unwrap())
    }
    fn mask(&mut self) -> Result<u32> {
        self.event("mask", Value::Null)?;
        Ok(self.0.borrow().mask)
    }
    fn zone_present(&mut self) -> Result<bool> {
        self.event("zone_for_flag", Value::Null)?;
        Ok(self.0.borrow().input["zone"].as_bool().unwrap())
    }
    fn zone_flag(&mut self) -> Result<()> {
        self.event("zone_flag", json!([1, 1]))
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        self.event("position", Value::Null)?;
        let s = self.0.borrow();
        Ok([
            s.input["xy"][0].as_u64().unwrap() as u32,
            s.input["xy"][1].as_u64().unwrap() as u32,
        ])
    }
    fn move_zone(&mut self, xy: [u32; 2], radius: u32) -> Result<()> {
        self.event("zone_for_move", Value::Null)?;
        self.event("move", json!([xy[0], xy[1], radius]))
    }
}
#[test]
fn original_v14_all_pc_node_store_and_lazy_queue_zone_prefixes() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-denizen-init/native-denizen-init.json"
    ))
    .unwrap();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 1194);
    for row in rows {
        let mut probe = Probe(Rc::new(RefCell::new(State {
            input: row["input"].clone(),
            stop: row["stop"].as_str().map(str::to_owned),
            stopped: None,
            counts: BTreeMap::new(),
            events: vec![],
            mask: row["input"]["mask"].as_u64().unwrap() as u32,
            last_list: 0,
        })));
        let result = if row["input"]["phase"] == "init" {
            merge(&mut probe).and_then(|()| {
                let condition = probe.clone();
                crate::load_detection_init_world::initialize_when(&mut probe, || {
                    condition.condition()
                })
                .map(|_| ())
            })
        } else {
            after(&mut probe)
        };
        if matches!(result, Err(Error::NullDereference(_))) {
            probe.0.borrow_mut().stopped = Some("null_zone_flag".into())
        }
        let s = probe.0.borrow();
        let actual = json!({"events":s.events,"mask":s.mask,"stopped":s.stopped});
        assert_eq!(actual, row["output"], "{row}");
        assert_eq!(result.is_ok(), row["output"]["stopped"].is_null(), "{row}");
    }
}
