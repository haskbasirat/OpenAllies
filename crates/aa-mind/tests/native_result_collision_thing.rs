//! Replays research/phase8-movement/native-result-collision-thing.json
//! (original 536840 KKC_UAIStateMove::ResultCollisionThing under Unicorn)
//! against aa_mind::uai_thinks::result_collision_thing: the byte result and
//! every seam call (thing +B4, actor +16C, 4D4FF0, 5340C0 with the record it
//! was handed, 5358A0, 5351C0, Mind+8 virtual +4) in order.
use aa_mind::{
    Result,
    uai_kill::KillDecision,
    uai_thinks::result_collision_thing::{CollisionThingWorld, result_collision_thing},
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::HashMap;

const THING: u32 = 1;
const CURRENT: u32 = 2;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-movement/native-result-collision-thing.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}
fn words(r: &KillDecision) -> [u32; 7] {
    [
        r.ability,
        r.kind,
        r.status,
        r.score,
        r.target,
        r.distance2,
        r.priority as u32,
    ]
}

struct Host<'a> {
    c: &'a Value,
    /// GTID registry: index -> (generation, entry).
    registry: HashMap<u32, (u32, u32)>,
    events: Vec<Value>,
    calls: usize,
}
impl<'a> Host<'a> {
    fn new(c: &'a Value) -> Self {
        let mut registry = HashMap::new();
        let thing = &c["thing"];
        if !thing.is_null() && thing["registered"].as_bool().unwrap() {
            let g = u(&thing["gtid"]);
            registry.insert(g & 0xffff, (g >> 16, THING));
        }
        let current = &c["current"];
        if !current.is_null() {
            let g = u(&current["gtid"]);
            let entry = if current["null_entry"].as_bool().unwrap() {
                0
            } else {
                CURRENT
            };
            registry.insert(g & 0xffff, (g >> 16, entry));
        }
        Self {
            c,
            registry,
            events: Vec::new(),
            calls: 0,
        }
    }
}
impl CollisionThingWorld for Host<'_> {
    fn thing_is_actor(&mut self, thing: u32) -> Result<bool> {
        self.events.push(json!(["is_actor", thing]));
        Ok(self.c["is_actor"].as_bool().unwrap())
    }
    fn actor_accepts(&mut self, thing: u32) -> Result<bool> {
        self.events.push(json!(["accepts", thing]));
        Ok(self.c["accepts"].as_bool().unwrap())
    }
    fn relation(&mut self, thing: u32, a: u32, b: u32) -> Result<i32> {
        self.events.push(json!(["relation", thing, a, b]));
        Ok(u(&self.c["relation"]) as i32)
    }
    fn engage_status(&mut self) -> Result<u32> {
        Ok(u(&self.c["engage_status"]))
    }
    fn kill_status(&mut self) -> Result<u32> {
        Ok(u(&self.c["kill_status"]))
    }
    fn kill_target(&mut self) -> Result<u32> {
        Ok(u(&self.c["kill_target"]))
    }
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        let (generation, entry) = self
            .registry
            .get(&(gtid & 0xffff))
            .copied()
            .unwrap_or((0, 0));
        Ok((generation == gtid >> 16 && entry != 0).then_some(entry))
    }
    fn actor_position(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.c["actor_position"]))
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let key = if object == THING { "thing" } else { "current" };
        Ok(pair(&self.c[key]["position"]))
    }
    fn choose(&mut self, target: Option<u32>, record: &mut KillDecision) -> Result<u32> {
        let mut event = vec![json!("choose"), json!(target.unwrap_or(0)), json!(0), json!(0)];
        event.extend(words(record).iter().map(|w| json!(w)));
        self.events.push(Value::Array(event));
        let w = &self.c["records"][self.calls];
        self.calls += 1;
        *record = KillDecision {
            ability: u(&w[0]),
            kind: u(&w[1]),
            status: u(&w[2]),
            score: u(&w[3]),
            target: u(&w[4]),
            distance2: u(&w[5]),
            priority: u(&w[6]) as i32,
        };
        Ok(record.status)
    }
    fn engage(&mut self, thing: u32) -> Result<()> {
        self.events.push(json!(["engage", thing]));
        Ok(())
    }
    fn kill(&mut self, mode: u32, thing: u32) -> Result<()> {
        self.events.push(json!(["kill", mode, thing]));
        Ok(())
    }
    fn schedule(&mut self, delay_bits: u32, data: u32) -> Result<()> {
        self.events.push(json!(["schedule", delay_bits, data]));
        Ok(())
    }
}

#[test]
fn result_collision_thing_matches_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 3000);
    let mut taken = 0;
    for case in cases {
        let c = &case["input"];
        let want = &case["output"];
        let mut host = Host::new(c);
        let thing = (!c["thing"].is_null()).then_some(THING);
        let got = result_collision_thing(thing, &mut host, precision(c["precision"].as_str().unwrap()))
            .unwrap_or_else(|e| panic!("{}: {e}", c["name"]));
        let name = c["name"].as_str().unwrap();
        assert_eq!(u8::from(got), want["result"].as_u64().unwrap() as u8, "{name} result");
        assert_eq!(Value::Array(host.events), want["events"], "{name} events");
        taken += usize::from(got);
    }
    assert!(taken > 500, "{taken} cases took the thing");
}
