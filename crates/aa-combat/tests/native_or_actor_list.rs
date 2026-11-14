//! Replays research/phase8-combat/native-or-actor-list.json (original 5781B0,
//! 57A780, 558150 and the cached branch of 558750 with 578470, under Unicorn)
//! against aa_combat::or_actor_list: after every step of a case the list
//! (head to tail), its refresh time, the seam calls in order, and for a
//! begin step the mode and the actors the cached walk returns.
use aa_body::F32;
use aa_combat::{
    Result, engagement,
    or_actor_list::{
        CachedActorList, CachedNextWorld, OrActorListWorld, begin, next_cached, refresh,
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::HashMap;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-or-actor-list.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}

struct Host<'a> {
    c: &'a Value,
    step: &'a Value,
    /// Registry index -> (generation, actor number; 0 is a null entry).
    registry: HashMap<u32, (u32, u32)>,
    events: Vec<Value>,
    query: usize,
    p: Precision,
}
impl Host<'_> {
    fn script(&mut self) -> Vec<u32> {
        let queries = self.c["queries"].as_array().unwrap();
        let q = &queries[self.query % queries.len()];
        self.query += 1;
        q.as_array()
            .unwrap()
            .iter()
            .map(|i| u(&self.c["actors"][u(i) as usize]["gtid"]))
            .collect()
    }
}
impl OrActorListWorld for Host<'_> {
    fn timeout(&mut self) -> Result<u32> {
        Ok(u(&self.step["timeout"]))
    }
    fn moving(&mut self) -> Result<bool> {
        self.events.push(json!(["moving"]));
        Ok(self.step["moving"].as_bool().unwrap())
    }
    fn time(&mut self) -> Result<u32> {
        Ok(u(&self.step["time"]))
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["position"][0]), u(&self.c["position"][1])])
    }
    fn operational_range(&mut self) -> Result<u32> {
        Ok(u(&self.c["range"]))
    }
    fn query(&mut self, position: [u32; 2], radius: u32) -> Result<Vec<u32>> {
        self.events.push(json!(["construct"]));
        self.events.push(json!(["begin", position[0], position[1], radius]));
        let found = self.script();
        self.events.push(json!(["finish"]));
        Ok(found)
    }
    fn begin_live(&mut self, position: [u32; 2], radius: u32) -> Result<()> {
        self.events.push(json!(["begin", position[0], position[1], radius]));
        self.script();
        Ok(())
    }
}

/// The 578470 inputs: actor 0 is the component actor, others are 1-based.
struct Range<'a>(&'a Value);
impl engagement::World for Range<'_> {
    fn resolve(&self, _: u32) -> Result<Option<u32>> {
        unreachable!()
    }
    fn has_mind(&self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn can_engage(&mut self, _: u32, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn mind_busy64(&mut self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn mind_busy7c(&mut self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn position(&self, actor: u32) -> Result<[F32; 2]> {
        let p = if actor == 0 {
            &self.0["position"]
        } else {
            &self.0["actors"][actor as usize - 1]["position"]
        };
        Ok([F32(u(&p[0])), F32(u(&p[1]))])
    }
    fn operating_radius_squared(&self, owner: u32) -> Result<F32> {
        assert_eq!(owner, 0);
        Ok(F32(u(&self.0["range2"])))
    }
    fn element_attached(&self, _: u32) -> Result<bool> {
        unreachable!()
    }
    fn flags100(&self, _: u32) -> Result<u32> {
        unreachable!()
    }
    fn add_engage_target(&mut self, _: u32, _: u32) -> Result<()> {
        unreachable!()
    }
    fn outside_operating_radius(&mut self, _: u32, _: u32) -> Result<()> {
        unreachable!()
    }
}
impl CachedNextWorld for Host<'_> {
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        let (generation, entry) = self
            .registry
            .get(&(gtid & 0xffff))
            .copied()
            .unwrap_or((0, 0));
        Ok((generation == gtid >> 16 && entry != 0).then_some(entry))
    }
    fn within_range(&mut self, actor: u32) -> Result<bool> {
        engagement::within_operating_radius(0, actor, &Range(self.c), self.p)
    }
}

#[test]
fn or_actor_list_matches_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 400);
    let (mut walks, mut rebuilds) = (0, 0);
    for case in cases {
        let c = &case["input"];
        let name = c["name"].as_str().unwrap();
        let p = precision(c["precision"].as_str().unwrap());
        let mut registry = HashMap::new();
        for (i, a) in c["actors"].as_array().unwrap().iter().enumerate() {
            let g = u(&a["gtid"]);
            let entry = if a["null_entry"].as_bool().unwrap() { 0 } else { i as u32 + 1 };
            registry.insert(g & 0xffff, (g >> 16, entry));
        }
        let mut list = CachedActorList::default();
        let mut query = 0;
        let steps = c["steps"].as_array().unwrap();
        let outputs = case["output"].as_array().unwrap();
        for (k, (step, want)) in steps.iter().zip(outputs).enumerate() {
            for i in step["stale"].as_array().unwrap() {
                let i = u(i);
                let g = u(&c["actors"][i as usize]["gtid"]).wrapping_add(0x10000);
                registry.insert(g & 0xffff, (g >> 16, i + 1));
            }
            let mut host = Host { c, step, registry, events: Vec::new(), query, p };
            if step["kind"] == "refresh" {
                rebuilds += usize::from(refresh(&mut list, &mut host, p).unwrap());
            } else {
                let cursor = begin(&mut list, &mut host, p).unwrap();
                assert_eq!(cursor.is_some(), want["cached"].as_bool().unwrap(), "{name} step {k} mode");
                if let Some(mut cursor) = cursor {
                    let mut walked = Vec::new();
                    while let Some(actor) = next_cached(&mut cursor, &list, &mut host).unwrap() {
                        walked.push(actor);
                    }
                    assert_eq!(json!(walked), want["walked"], "{name} step {k} walk");
                    walks += 1;
                }
            }
            assert_eq!(Value::Array(host.events.clone()), want["events"], "{name} step {k} events");
            assert_eq!(json!(list.gtids), want["list"], "{name} step {k} list");
            assert_eq!(list.refreshed_bits, u(&want["refreshed"]), "{name} step {k} +CC");
            registry = host.registry;
            query = host.query;
        }
    }
    assert!(walks > 500 && rebuilds > 300, "{walks} walks, {rebuilds} rebuilds");
}
