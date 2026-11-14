//! Replays research/phase8-combat/native-member-choice-fixtures.json
//! (original 52F010 with 455000, 534700 and 535910 under Unicorn, at
//! PC24/PC53/PC64) against aa_combat::member_choice.
use aa_combat::{
    Result,
    member_choice::{
        Decision, ENGAGE_MINIMUM_PRIORITY, EngageChoiceWorld, MemberChoiceWorld, Query, choose,
        engage_arguments, engage_choice, rank,
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const ORGANIZATION: u32 = 900;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-member-choice-fixtures.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn decision(v: &Value) -> Decision {
    let w: Vec<u32> = v.as_array().unwrap().iter().map(u).collect();
    Decision {
        ability: w[0],
        kind: w[1],
        status: w[2],
        score: w[3],
        target: w[4],
        distance2: w[5],
        priority: w[6] as i32,
    }
}

/// Objects are handles index+1 in key order.
struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    next: usize,
}
impl Host {
    fn name(&self, h: u32) -> Value {
        match h {
            0 => json!(0),
            ORGANIZATION => json!("organization"),
            h if (h as usize) <= self.keys.len() => json!(self.keys[h as usize - 1]),
            h => json!(format!("{h:#x}")),
        }
    }
    fn handle(&self, key: &str) -> u32 {
        self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
    }
    fn record(&self, d: &Decision) -> Value {
        json!([d.ability, d.kind, d.status, d.score, self.name(d.target), d.distance2, d.priority])
    }
    fn current(&self) -> Value {
        self.c["candidates"][self.next - 1].clone()
    }
    fn answer(&mut self, event: &str, field: &str, candidate: u32) -> Value {
        let k = self.current();
        assert_eq!(self.handle(k["key"].as_str().unwrap()), candidate);
        let r = k[field].clone();
        self.events
            .push(json!({"event": event, "object": k["key"], "result": r}));
        r
    }
    fn position(v: &Value) -> [u32; 2] {
        [u(&v[0]), u(&v[1])]
    }
}
impl MemberChoiceWorld for Host {
    fn has_performer(&mut self) -> Result<bool> {
        Ok(self.c["performer"].as_bool().unwrap_or(true))
    }
    fn organization(&mut self) -> Result<Option<u32>> {
        Ok(self.c["membership"]["organization"]
            .as_bool()
            .unwrap_or(false)
            .then_some(ORGANIZATION))
    }
    fn query_begin(&mut self, query: Query) -> Result<()> {
        self.events.push(match query {
            Query::Organization(o) => {
                json!({"event": "query_begin", "organization": self.name(o)})
            }
            Query::Circle { point, radius } => {
                json!({"event": "query_begin", "point": point, "radius": radius})
            }
        });
        Ok(())
    }
    fn query_next(&mut self) -> Result<Option<u32>> {
        let order = self.c["candidates"].as_array().unwrap();
        let r = if self.next < order.len() {
            let h = self.handle(order[self.next]["key"].as_str().unwrap());
            self.next += 1;
            Some(h)
        } else {
            None
        };
        let n = self.name(r.unwrap_or(0));
        self.events.push(json!({"event": "next", "result": n}));
        Ok(r)
    }
    fn query_end(&mut self) -> Result<()> {
        self.events.push(json!({"event": "query_end"}));
        Ok(())
    }
    fn candidate_matches(&mut self, candidate: u32, filter: u32) -> Result<bool> {
        assert_eq!(filter, self.handle(self.c["filter"].as_str().unwrap()));
        Ok(self.answer("matches", "matches", candidate).as_bool().unwrap())
    }
    fn mind_considers(&mut self, candidate: u32) -> Result<bool> {
        Ok(self.answer("considers", "considers", candidate).as_bool().unwrap())
    }
    fn remembered_unreachable(&mut self, candidate: u32) -> Result<bool> {
        Ok(self.answer("unreachable", "unreachable", candidate).as_bool().unwrap())
    }
    fn actor_accepts(&mut self, candidate: u32) -> Result<bool> {
        Ok(self.answer("accepts", "accepts", candidate).as_bool().unwrap())
    }
    fn priority(&mut self, candidate: u32) -> Result<i32> {
        Ok(self.answer("priority", "priority", candidate).as_i64().unwrap() as i32)
    }
    fn decide(&mut self, candidate: u32, record: &mut Decision) -> Result<()> {
        let k = self.current();
        assert_eq!(self.handle(k["key"].as_str().unwrap()), candidate);
        let before = self.record(record);
        record.target = candidate;
        for w in k["writes"].as_array().unwrap() {
            let v = u(&w[1]);
            match w[0].as_u64().unwrap() {
                0 => record.ability = v,
                1 => record.kind = v,
                2 => record.status = v,
                3 => record.score = v,
                5 => record.distance2 = v,
                6 => record.priority = v as i32,
                i => panic!("write {i}"),
            }
        }
        self.events
            .push(json!({"event": "decide", "object": k["key"], "before": before}));
        Ok(())
    }
}
impl EngageChoiceWorld for Host {
    fn organization_position(&mut self, organization: u32) -> Result<[u32; 2]> {
        assert_eq!(organization, ORGANIZATION);
        Ok(Self::position(&self.c["membership"]["pos"]))
    }
    fn organization_radius(&mut self, organization: u32) -> Result<u32> {
        assert_eq!(organization, ORGANIZATION);
        Ok(u(&self.c["membership"]["radius"]))
    }
    fn actor_position(&mut self) -> Result<[u32; 2]> {
        Ok(Self::position(&self.c["actor_pos"]))
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let key = self.keys[object as usize - 1].clone();
        Ok(Self::position(&self.c["objects"][&key]["pos"]))
    }
}

#[test]
fn native_member_choice_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1740);
    assert_eq!(f["coverage"]["52F010"], json!([77, 77]));
    assert_eq!(f["coverage"]["455000"], json!([15, 15]));
    // Not reached: a dead null-membership branch (6) and alignment padding (2).
    assert_eq!(f["coverage"]["534700"], json!([170, 178]));
    assert_eq!(f["coverage"]["535910"], json!([66, 66]));
    let mut counts = [0; 4];
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let keys: Vec<String> = c["objects"]
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        let mut host = Host {
            c: c.clone(),
            keys,
            events: vec![],
            next: 0,
        };
        match c["entry"].as_str().unwrap() {
            "rank" => {
                let bypass = u(&c["flag"]) & 0xff != 0;
                let r = rank(&decision(&c["a"]), &decision(&c["b"]), bypass, p).unwrap();
                assert_eq!(json!(r), o["result"], "{name}");
                counts[0] += 1;
            }
            "choose" => {
                let mut out = Decision {
                    ability: 0x5a5a_5a5a,
                    kind: 0x5a5a_5a5a,
                    status: 0x5a5a_5a5a,
                    score: 0x5a5a_5a5a,
                    target: 0x5a5a_5a5a,
                    distance2: 0x5a5a_5a5a,
                    priority: 0x5a5a_5a5a,
                };
                let want_out = c["out"].as_bool().unwrap();
                let filter = c["filter"].as_str().map(|k| host.handle(k));
                let preferred = c["preferred"].as_str().map(|k| host.handle(k));
                let point = Host::position(&c["point"]);
                let minimum = c["minimum"].as_i64().unwrap() as i32;
                let r = choose(
                    point,
                    u(&c["radius"]),
                    want_out.then_some(&mut out),
                    minimum,
                    filter,
                    preferred,
                    &mut host,
                    p,
                )
                .unwrap();
                assert_eq!(json!(host.events), o["events"], "{name}");
                assert_eq!(host.name(r.unwrap_or(0)), o["result"], "{name}");
                if want_out {
                    assert_eq!(host.record(&out), o["out"], "{name}");
                } else {
                    assert!(o["out"].is_null(), "{name}");
                }
                counts[1] += 1;
            }
            _ => {
                let target = c["target"].as_str().map(|k| host.handle(k));
                if c["seam_choose"].as_bool().unwrap() {
                    let args = engage_arguments(target, &mut host, p).unwrap();
                    let events = o["events"].as_array().unwrap();
                    match args {
                        None => {
                            assert!(events.is_empty(), "{name}");
                            assert_eq!(o["result"], json!(0), "{name}");
                        }
                        Some((point, radius)) => {
                            assert_eq!(
                                events[0],
                                json!({"event": "choose", "point": point, "radius": radius,
                                    "out": 0, "minimum": ENGAGE_MINIMUM_PRIORITY, "filter": 0,
                                    "preferred": host.name(target.unwrap_or(0))}),
                                "{name}"
                            );
                            assert_eq!(events.len(), 1, "{name}");
                        }
                    }
                    counts[2] += 1;
                } else {
                    let r = engage_choice(target, &mut host, p).unwrap();
                    assert_eq!(json!(host.events), o["events"], "{name}");
                    assert_eq!(host.name(r.unwrap_or(0)), o["result"], "{name}");
                    counts[3] += 1;
                }
            }
        }
    }
    assert_eq!(counts, [900, 600, 120, 120]);
}
