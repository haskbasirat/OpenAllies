//! Replays research/phase8-combat/native-city-admission-fixtures.json: the
//! original City candidate test 5640A0, the City registration 564300 and the
//! maximum value 4D01E0(0) with 578C00 executed under Unicorn.
use aa_body::F32;
use aa_combat::{
    Result,
    city_admission::{self, AdmissionWorld, MaximumWorld},
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const OBJECT: u32 = 1;
const CITY_ACTOR: u32 = 2;
const KINGDOM: u32 = 3;

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Host {
    o: Value,
    owners: Vec<u32>,
    kingdom_190: u32,
    updatable: u32,
    blocks: Vec<u32>,
    candidates: Vec<u32>,
    tail: Option<u32>,
    events: Vec<Value>,
}
impl Host {
    fn field(&self, key: &str) -> Option<F32> {
        self.o[key].as_u64().map(|b| F32(b as u32))
    }
}
impl MaximumWorld for Host {
    fn organization_maximum(&mut self, actor: u32) -> Result<Option<F32>> {
        assert_eq!(actor, OBJECT);
        Ok(self.field("organization"))
    }
    fn component98_maximum(&mut self, actor: u32) -> Result<Option<F32>> {
        assert_eq!(actor, OBJECT);
        Ok(self.field("component98"))
    }
    fn performer_value(&mut self, actor: u32) -> Result<Option<F32>> {
        assert_eq!(actor, OBJECT);
        Ok(self.field("performer"))
    }
    fn denizen_maximum(&mut self, actor: u32) -> Result<Option<F32>> {
        assert_eq!(actor, OBJECT);
        Ok(self.field("denizen"))
    }
}
impl AdmissionWorld for Host {
    fn city_actor(&mut self) -> Result<u32> {
        Ok(CITY_ACTOR)
    }
    fn flags8(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, OBJECT);
        Ok(u(&self.o["flags8"]))
    }
    fn owner(&mut self, object: u32) -> Result<Option<u32>> {
        assert_eq!(object, OBJECT);
        let owner = if self.owners.len() > 1 {
            self.owners.remove(0)
        } else {
            self.owners[0]
        };
        self.events.push(json!({ "Owner": owner }));
        Ok((owner != 0).then_some(KINGDOM))
    }
    fn kingdom_word190(&mut self, kingdom: u32) -> Result<u32> {
        assert_eq!(kingdom, KINGDOM);
        Ok(self.kingdom_190)
    }
    fn membership_organization(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, OBJECT);
        Ok(self.o["membership"].as_u64().unwrap_or(0) as u32)
    }
    fn body_word(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, OBJECT);
        Ok(u(&self.o["body"]))
    }
    fn definition_bytes(&mut self, object: u32) -> Result<[u8; 2]> {
        assert_eq!(object, OBJECT);
        Ok([u(&self.o["bytes"][0]) as u8, u(&self.o["bytes"][1]) as u8])
    }
    fn gtid(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, OBJECT);
        Ok(u(&self.o["gtid"]))
    }
    fn blocks(&mut self) -> Result<Vec<u32>> {
        Ok(self.blocks.clone())
    }
    fn candidates(&mut self) -> Result<Vec<u32>> {
        Ok(self.candidates.clone())
    }
    fn push_candidate(&mut self, gtid: u32) -> Result<()> {
        self.candidates.insert(0, gtid);
        if self.tail.is_none() {
            self.tail = Some(gtid);
        }
        Ok(())
    }
    fn updatable_word(&mut self) -> Result<u32> {
        Ok(self.updatable)
    }
    fn register_update(&mut self) -> Result<()> {
        self.events.push(json!({ "Register": 0 }));
        Ok(())
    }
}

#[test]
fn original_city_admission_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-city-admission-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1806);
    let (mut admitted, mut pushed) = (0, 0);
    for row in cases {
        let i = &row["input"];
        let out = &row["output"];
        let name = format!("{} {}", i["name"], i["precision"]);
        let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
        let words = |v: &Value| v.as_array().unwrap().iter().map(u).collect::<Vec<_>>();
        let candidates = words(&i["candidates"]);
        let mut host = Host {
            o: i["object"].clone(),
            owners: i["object"]["owner"]
                .as_array()
                .map_or(vec![0], |a| a.iter().map(u).collect()),
            kingdom_190: u(&i["kingdom_190"]),
            updatable: u(&i["updatable_word"]),
            blocks: words(&i["blocks"]),
            tail: candidates.last().copied(),
            candidates,
            events: vec![],
        };
        let object = match &i["object"] {
            Value::Null => None,
            Value::String(s) if s == "city" => Some(CITY_ACTOR),
            _ => Some(OBJECT),
        };
        let before = host.candidates.len();
        if i["function"] == "admits" {
            let r = city_admission::admits(&mut host, object, p).unwrap();
            admitted += r as usize;
            assert_eq!(json!(r as u8), out["result"], "{name}");
        } else {
            city_admission::register(&mut host, object, p).unwrap();
            pushed += host.candidates.len() - before;
        }
        assert_eq!(json!(host.events), out["events"], "{name}");
        assert_eq!(json!(host.candidates), out["candidates"], "{name}");
        assert_eq!(json!(host.tail), out["tail"], "{name}");
    }
    assert_eq!((admitted, pushed), (93, 49));
}
