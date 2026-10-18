//! Native 5788B0/57DC40 full read/store traces, image/PC parity and failure prefixes.
use aa_body::F32;
use aa_combat::{
    Error, Result,
    organization_combat_value::{self, World},
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Host {
    input: Value,
    values: [u32; 2],
    events: Vec<Value>,
    stores: Vec<Value>,
    counts: BTreeMap<String, u32>,
}
impl Host {
    fn new(input: Value) -> Self {
        Self {
            values: [u(&input["initial"][0]), u(&input["initial"][1])],
            input,
            events: Vec::new(),
            stores: Vec::new(),
            counts: BTreeMap::new(),
        }
    }
    fn event(&mut self, name: &str, args: &[u32]) -> Result<()> {
        let mut event = vec![json!(name)];
        event.extend(args.iter().map(|x| json!(x)));
        self.events.push(json!(event));
        let count = self.counts.entry(name.to_owned()).or_default();
        *count += 1;
        let count = *count;
        let mutations = self.input["mutations"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for a in mutations {
            if a["event"] != name || u(&a["occurrence"]) != count {
                continue;
            }
            let value = a["value"].clone();
            match a["kind"].as_str().unwrap() {
                "count" => self.input["count"] = value,
                "array" => self.input["array"] = value,
                "member" => {
                    self.input["arrays"][u(&a["array"]) as usize][u(&a["index"]) as usize] = value
                }
                "performer" => self.input["actors"][(u(&a["member"]) - 1) as usize] = value,
                "value" => {
                    self.input["performers"][(u(&a["performer"]) - 1) as usize]["value"] = value
                }
                "current" => self.values[0] = u(&value),
                "maximum" => self.values[1] = u(&value),
                kind => panic!("unknown mutation {kind}"),
            }
        }
        let stop = &self.input["stop"];
        if stop["event"] == name && u(&stop["occurrence"]) == count {
            return Err(Error::World(name.into()));
        }
        Ok(())
    }
}
impl World for Host {
    type Members = u32;
    type Member = u32;
    type Performer = u32;
    type Slot = u32;
    fn member_count(&mut self) -> Result<i32> {
        self.event("count", &[])?;
        Ok(self.input["count"].as_i64().unwrap() as i32)
    }
    fn member_array(&mut self) -> Result<u32> {
        self.event("array", &[])?;
        Ok(u(&self.input["array"]))
    }
    fn member(&mut self, array: u32, index: i32) -> Result<Option<u32>> {
        self.event("member", &[array, index as u32])?;
        let id = u(&self.input["arrays"][array as usize][index as usize]);
        Ok((id != 0).then_some(id))
    }
    fn performer(&mut self, member: u32) -> Result<Option<u32>> {
        self.event("performer", &[member])?;
        let id = u(&self.input["actors"][(member - 1) as usize]);
        Ok((id != 0).then_some(id))
    }
    fn performer_health(&mut self, performer: u32) -> Result<Option<[F32; 2]>> {
        self.event("health", &[performer])?;
        let health = &self.input["performers"][(performer - 1) as usize]["health"];
        Ok(health.as_array().map(|h| [F32(u(&h[0])), F32(u(&h[1]))]))
    }
    fn performer_value(&mut self, performer: u32) -> Result<F32> {
        self.event("value", &[performer])?;
        Ok(F32(u(
            &self.input["performers"][(performer - 1) as usize]["value"]
        )))
    }
    fn slot(&mut self, index: i32) -> Result<Option<u32>> {
        self.event("slot", &[index as u32])?;
        Ok((!self.input["slots"][index as usize].is_null()).then_some(index as u32))
    }
    fn slot_flags(&mut self, slot: u32) -> Result<u32> {
        self.event("flags", &[slot])?;
        Ok(u(&self.input["slots"][slot as usize]["flags"]))
    }
    fn slot_value(&mut self, slot: u32) -> Result<F32> {
        self.event("slot_value", &[slot])?;
        Ok(F32(u(&self.input["slots"][slot as usize]["value"])))
    }
    fn current(&mut self) -> Result<F32> {
        self.event("current", &[])?;
        Ok(F32(self.values[0]))
    }
    fn maximum(&mut self) -> Result<F32> {
        self.event("maximum", &[])?;
        Ok(F32(self.values[1]))
    }
    fn store_current(&mut self, value: F32) -> Result<()> {
        self.event("store_current", &[])?;
        self.values[0] = value.0;
        self.stores.push(json!([0x68, value.0]));
        Ok(())
    }
    fn store_maximum(&mut self, value: F32) -> Result<()> {
        self.event("store_maximum", &[])?;
        self.values[1] = value.0;
        self.stores.push(json!([0x6c, value.0]));
        Ok(())
    }
}

#[test]
fn native_original_and_v14_borrowed_combat_value() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-borrowed-combat-value/native-borrowed-combat-value.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 306);
    for row in cases {
        let mut host = Host::new(row["input"].clone());
        let pc: Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let result = organization_combat_value::recompute(&mut host, pc);
        let outcome = match result {
            Ok(()) => "ok".to_owned(),
            Err(Error::World(s)) => s,
            Err(Error::Invalid(_)) => "invalid".to_owned(),
            Err(e) => panic!("{}: {e}", row["input"]["name"]),
        };
        let out = json!({"outcome":outcome,"values":host.values,"events":host.events,"stores":host.stores});
        assert_eq!(
            out, row["output"],
            "{} / {} / {}",
            row["image"], row["precision"], row["input"]["name"]
        );
    }
}
