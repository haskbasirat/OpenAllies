//! Replays research/phase8-combat/native-city-capture-fixtures.json: the
//! original City tick 5647A0, the combat value 4D0180 with 57DC40 and the
//! City block hand-over 5643A0 executed under Unicorn.
use aa_body::{F32, damage::DamageRecord};
use aa_combat::{
    Result, candidate_value,
    city_capture::{self, BlocksWorld, World},
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::cmp::Ordering;

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn tag(v: Option<u32>) -> u32 {
    v.unwrap_or(0)
}

struct Host {
    c: Value,
    p: Precision,
    owners: Vec<u32>,
    nodes: Vec<Value>,
    siege: u8,
    events: Vec<Value>,
}

impl Host {
    fn actor(&self, t: u32) -> &Value {
        &self.c["actors"][t.to_string()]
    }
    fn relation(&self, a: u32, b: u32) -> String {
        self.c["relations"][format!("{a}:{b}")]
            .as_str()
            .unwrap_or("")
            .to_string()
    }
    fn slots(&self) -> &Vec<Value> {
        self.c["slots"].as_array().unwrap()
    }
    fn answer(&mut self, slot: &str, object: u32, other: Option<u32>) -> Result<bool> {
        let other = tag(other);
        self.events.push(json!({ format!("V{slot}"): [object, other] }));
        if object >= 100 {
            let r = self.relation(object, other);
            return Ok(match slot {
                "138" => r == "friend",
                "140" => r == "enemy",
                _ => panic!("kingdom virtual {slot}"),
            });
        }
        let view = self.actor(object);
        if let Some(forced) = view["answers"][slot].as_bool() {
            return Ok(forced);
        }
        let owner = u(&view["owner"]);
        Ok(match slot {
            "134" => other == owner,
            "138" => other == owner || self.relation(owner, other) == "friend",
            _ => self.relation(owner, other) == "enemy",
        })
    }
    fn field(&self, actor: u32, key: &str) -> Option<F32> {
        self.actor(actor)[key].as_u64().map(|b| F32(b as u32))
    }
    fn node_object(&self, node: u32) -> Option<u32> {
        let e = &self.nodes[node as usize - 1];
        (e["state"] == "live").then(|| u(&e["actor"]))
    }
    fn next(&self, node: u32) -> Option<u32> {
        ((node as usize) < self.nodes.len()).then_some(node + 1)
    }
}

impl candidate_value::World for Host {
    fn organization_current(&mut self, actor: u32) -> Result<Option<F32>> {
        Ok(self.field(actor, "organization"))
    }
    fn component98_current(&mut self, actor: u32) -> Result<Option<F32>> {
        Ok(self.field(actor, "component98"))
    }
    fn performer(&mut self, actor: u32) -> Result<Option<(F32, Option<[F32; 2]>)>> {
        let p = &self.actor(actor)["performer"];
        if p.is_null() {
            return Ok(None);
        }
        let body = p["body"]
            .as_array()
            .map(|b| [F32(u(&b[0])), F32(u(&b[1]))]);
        Ok(Some((F32(u(&p["value"])), body)))
    }
    fn denizen_current(&mut self, actor: u32) -> Result<Option<F32>> {
        Ok(self.field(actor, "denizen"))
    }
}

impl World for Host {
    fn city_owner(&mut self) -> Result<Option<u32>> {
        let owner = if self.owners.len() > 1 {
            self.owners.remove(0)
        } else {
            self.owners[0]
        };
        self.events.push(json!({ "CityOwner": owner }));
        Ok((owner != 0).then_some(owner))
    }
    fn set_under_siege(&mut self, value: bool) -> Result<()> {
        self.siege = value as u8;
        Ok(())
    }
    fn first_candidate(&mut self) -> Result<Option<u32>> {
        Ok((!self.nodes.is_empty()).then_some(1))
    }
    fn next_node(&mut self, node: u32) -> Result<Option<u32>> {
        Ok(self.next(node))
    }
    fn resolve_node(&mut self, node: u32) -> Result<Option<u32>> {
        Ok(self.node_object(node))
    }
    fn owner(&mut self, object: u32) -> Result<Option<u32>> {
        self.events.push(json!({ "Owner": object }));
        let owner = u(&self.actor(object)["owner"]);
        Ok((owner != 0).then_some(owner))
    }
    fn virtual_134(&mut self, object: u32, other: Option<u32>) -> Result<bool> {
        self.answer("134", object, other)
    }
    fn virtual_138(&mut self, object: u32, other: Option<u32>) -> Result<bool> {
        self.answer("138", object, other)
    }
    fn virtual_140(&mut self, object: u32, other: Option<u32>) -> Result<bool> {
        self.answer("140", object, other)
    }
    fn combat_value(&mut self, object: u32) -> Result<Binary> {
        let p = self.p;
        candidate_value::value(object, self, p)
    }
    fn kingdom_count(&mut self) -> Result<i32> {
        Ok(self.slots().len() as i32)
    }
    fn kingdom_slot(&mut self, index: i32) -> Result<Option<u32>> {
        let s = &self.slots()[index as usize];
        Ok((!s.is_null()).then(|| u(&s["tag"])))
    }
    fn kingdom_index(&mut self, kingdom: u32) -> Result<i32> {
        let s = self
            .slots()
            .iter()
            .find(|s| !s.is_null() && u(&s["tag"]) == kingdom)
            .unwrap();
        Ok(s["index"].as_i64().unwrap() as i32)
    }
    fn capture(&mut self, winner: Option<u32>, record: &DamageRecord) -> Result<()> {
        self.events
            .push(json!({ "Capture": ["city", tag(winner), record.words()] }));
        Ok(())
    }
}

impl BlocksWorld for Host {
    fn first_block(&mut self) -> Result<Option<u32>> {
        Ok((!self.nodes.is_empty()).then_some(1))
    }
    fn next_node(&mut self, node: u32) -> Result<Option<u32>> {
        Ok(self.next(node))
    }
    fn resolve_node(&mut self, node: u32) -> Result<Option<u32>> {
        Ok(self.node_object(node))
    }
    fn change_kingdom(&mut self, block: u32, kingdom: Option<u32>) -> Result<()> {
        self.events
            .push(json!({ "ChangeKingdom": [block, tag(kingdom)] }));
        Ok(())
    }
}

fn extended(v: &Value) -> Binary {
    let bits = (u128::from(v["sign_exponent"].as_u64().unwrap() as u16) << 64)
        | u128::from(v["mantissa"].as_u64().unwrap());
    Binary::from_x87_extended_bits(bits).unwrap()
}

#[test]
fn original_city_capture_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-city-capture-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 108);
    let mut seen = [0usize; 3];
    for row in cases {
        let i = &row["input"];
        let out = &row["output"];
        let name = format!("{} {}", i["name"], i["precision"]);
        let p: Precision = serde_json::from_value(i["precision"].clone()).unwrap();
        let function = i["function"].as_str().unwrap();
        let list = if function == "blocks" { "blocks" } else { "list" };
        let mut host = Host {
            c: i.clone(),
            p,
            owners: i["city_owner"]
                .as_array()
                .map_or(vec![0], |a| a.iter().map(u).collect()),
            nodes: i[list].as_array().cloned().unwrap_or_default(),
            siege: 0x55,
            events: vec![],
        };
        match function {
            "tick" => {
                seen[0] += 1;
                city_capture::tick(&mut host, p).unwrap();
                assert_eq!(json!(host.events), out["events"], "{name}");
                assert_eq!(json!(host.siege), out["siege"], "{name}");
            }
            "value" => {
                seen[1] += 1;
                let v = candidate_value::value(1, &mut host, p).unwrap();
                let want = extended(&out["st0"]);
                assert_eq!(v.compare(&want), Ordering::Equal, "{name}");
                assert_eq!(v.is_negative_nonzero(), want.is_negative_nonzero(), "{name}");
                assert!(host.events.is_empty(), "{name}");
            }
            "blocks" => {
                seen[2] += 1;
                let k = u(&i["kingdom"]);
                city_capture::blocks_change_kingdom(&mut host, (k != 0).then_some(k)).unwrap();
                assert_eq!(json!(host.events), out["events"], "{name}");
            }
            other => panic!("{other}"),
        }
    }
    assert_eq!(seen, [78, 27, 3]);
}
