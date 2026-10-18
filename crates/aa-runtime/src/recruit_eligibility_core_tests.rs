use super::*;
use serde_json::Value;

struct Laboratory {
    input: Value,
    available: Vec<bool>,
    events: Vec<Value>,
    stop: Option<usize>,
}
impl Laboratory {
    fn event(&mut self, kind: &str, value: Value) -> Result<()> {
        if self.stop == Some(self.events.len()) {
            return Err(Error::Missing("controlled native child cutoff"));
        }
        self.events.push(serde_json::json!([kind, value]));
        Ok(())
    }
    fn query(&mut self, definition: usize, wanted: i32) -> Result<bool> {
        let Some(index) = self.input["items"]
            .as_array()
            .unwrap()
            .iter()
            .rposition(|item| item["definition"].as_u64().unwrap() as usize == definition)
        else {
            return Ok(false);
        };
        self.event("single_consume", serde_json::json!([index, wanted as u32]))?;
        if wanted > 1 || !self.available[index] {
            return Ok(false);
        }
        //Native stopped-child probes can observe consume before restore. The
        //production Single helper has no fallible child between these writes.
        self.available[index] = false;
        self.event("single_restore", serde_json::json!([index, wanted as u32]))?;
        self.available[index] = true;
        Ok(true)
    }
}
impl World for Laboratory {
    type Definition = usize;
    type Layout = usize;
    fn organization(&mut self, d: &usize) -> Result<bool> {
        Ok(self.input["definitions"][d]["mask"].as_u64().unwrap() & 128 != 0)
    }
    fn default_layout(&mut self, d: &usize) -> Result<Option<usize>> {
        Ok(self.input["definitions"][d]["layout"]
            .as_u64()
            .map(|i| i as usize))
    }
    fn owner(&mut self) -> Result<Option<ActorId>> {
        Ok(self.input["owner"]
            .as_bool()
            .unwrap()
            .then_some(ActorId::from_raw(7)))
    }
    fn owner_allows(&mut self, _: ActorId, d: &usize, layout: Option<&usize>) -> Result<bool> {
        let data = &self.input["definitions"][d];
        let class = data["class"].as_u64().unwrap() as u32;
        let flags = data["mask"].as_u64().unwrap() as u32;
        let default = data["layout"].as_u64().map(|i| i as usize);
        let recruitable = aa_actors::economy_values::recruitable_fields(class, flags, || {
            Ok(self.input["layouts"][default.unwrap()]["primary"]
                .as_i64()
                .unwrap() as i32)
        })
        .unwrap();
        if !recruitable {
            return Ok(false);
        }
        let layout = layout
            .copied()
            .or(if flags & 128 != 0 { default } else { None });
        if let Some(layout) = layout {
            let pairs = self.input["layouts"][layout]["members"]
                .as_array()
                .unwrap()
                .iter()
                .zip(self.input["layouts"][layout]["numbers"].as_array().unwrap())
                .map(|(m, n)| (m.as_u64().map(|m| m as usize), n.as_i64().unwrap() as i32))
                .collect::<Vec<_>>();
            for (member, wanted) in pairs {
                if let Some(member) = member {
                    if !self.query(member, wanted)? {
                        return Ok(false);
                    }
                }
            }
            Ok(true)
        } else {
            self.query(*d, 1)
        }
    }
    fn own_recruit_list_present(&mut self) -> Result<bool> {
        let present = !self.input["own_list"].as_array().unwrap().is_empty();
        self.event("own_list", Value::Bool(present))?;
        Ok(present)
    }
    fn own_recruit_list_contains(&mut self, d: &usize) -> Result<bool> {
        self.event("membership", serde_json::json!(d))?;
        Ok(self.input["own_list"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(*d as u64)))
    }
    fn layout_count(&mut self, l: &usize) -> Result<i32> {
        Ok(self.input["layouts"][l]["members"]
            .as_array()
            .unwrap()
            .len() as i32)
    }
    fn layout_member(&mut self, l: &usize, i: i32) -> Result<Option<usize>> {
        Ok(self.input["layouts"][l]["members"][i as usize]
            .as_u64()
            .map(|v| v as usize))
    }
    fn requirements(&mut self, d: &usize) -> Result<RequirementResult> {
        self.event("requirements", serde_json::json!(d))?;
        let data = &self.input["definitions"][d];
        Ok(RequirementResult {
            met: data["met"].as_bool().unwrap(),
            properties_met: data["properties_met"].as_bool().unwrap(),
        })
    }
    fn strict_requirements(&mut self, d: &usize) -> Result<bool> {
        Ok(self.input["definitions"][d]["strict"].as_bool().unwrap())
    }
}

fn replay(fixture: &str) {
    let fixture: Value = serde_json::from_str(fixture).unwrap();
    for row in fixture["rows"].as_array().unwrap() {
        let input = row["input"].clone();
        let available = input["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["available"].as_bool().unwrap())
            .collect();
        let target = input["target"].as_u64().map(|i| i as usize);
        let layout = input["explicit_layout"].as_u64().map(|i| i as usize);
        let check = input["check"].as_u64().unwrap() as u8 != 0;
        let mut world = Laboratory {
            input,
            available,
            events: Vec::new(),
            stop: row["stop"].as_u64().map(|i| i as usize),
        };
        let actual = eligibility(&mut world, target.as_ref(), layout.as_ref(), check);
        let output = &row["output"];
        if output["stopped"].is_null() {
            assert_eq!(
                actual.unwrap(),
                output["result"].as_u64().unwrap() as u32,
                "{row}"
            );
        } else {
            assert!(actual.is_err(), "{row}");
        }
        assert_eq!(world.events, *output["events"].as_array().unwrap(), "{row}");
        assert_eq!(
            world
                .available
                .iter()
                .map(|v| u8::from(*v))
                .collect::<Vec<_>>(),
            output["available"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u8)
                .collect::<Vec<_>>(),
            "{row}"
        );
    }
}

#[test]
fn native_parent_item_queries_and_cutoffs_all_pc() {
    replay(include_str!(
        "../../../research/skirmish-20261017-recruit-eligibility/native-eligibility.json"
    ));
}

#[test]
fn actual_regiment_and_truck_source_metadata_in_controlled_contexts_all_pc() {
    replay(include_str!(
        "../../../research/skirmish-20261017-recruit-eligibility/native-actual-source.json"
    ));
}
