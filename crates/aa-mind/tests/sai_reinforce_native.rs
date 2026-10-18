//! Replays research/phase8-sai/native-reinforce.json (original 7A5E10 and
//! 7C9CE0 under Unicorn) against aa_mind::sai::reinforce.
use aa_mind::{
    Result,
    sai::base_priority::Obj,
    sai::map_analysis::RegionEntry,
    sai::reinforce::{Influence, ReinforceWorld, influence, reinforce, release_for},
    x87::binary_from_x87,
};
use aa_numeric::{Binary, Precision};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, VecDeque};

struct Host {
    c: Value,
    expected: VecDeque<Value>,
    case: usize,
    cleared: BTreeMap<String, bool>,
}

fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn opt(v: &Value) -> Option<Obj> {
    let x = u(v);
    (x != 0).then_some(x)
}
fn id(o: Option<Obj>) -> u32 {
    o.unwrap_or(0)
}
fn x87(v: &Value) -> Binary {
    binary_from_x87(
        v["significand"].as_u64().unwrap(),
        v["sign_exponent"].as_u64().unwrap() as u16,
    )
    .unwrap()
}

impl Host {
    fn obj(&self, o: Obj) -> &Value {
        let v = &self.c["objects"][o.to_string()];
        assert!(!v.is_null(), "case {}: unknown object {o}", self.case);
        v
    }
    fn next(&mut self, op: &str, args: Value) -> Value {
        let e = self
            .expected
            .pop_front()
            .unwrap_or_else(|| panic!("case {}: unexpected {op} {args}", self.case));
        let mut exp: Map<String, Value> = e.as_object().unwrap().clone();
        let result = exp.remove("result").unwrap_or(Value::Null);
        let mut act: Map<String, Value> = args.as_object().unwrap().clone();
        act.insert("op".into(), json!(op));
        assert_eq!(Value::Object(exp), Value::Object(act), "case {}", self.case);
        result
    }
}

impl ReinforceWorld for Host {
    fn resolve(&mut self, gtid: u32) -> Result<Option<Obj>> {
        let entry = &self.c["registry"][(gtid & 0xffff).to_string()];
        if entry.is_null() {
            return Ok(None);
        }
        let (serial, obj) = (u(&entry[0]), u(&entry[1]));
        Ok((serial == gtid >> 16 && obj != 0).then_some(obj))
    }
    fn flags(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["flags"]))
    }
    fn object_gtid(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.obj(obj)["gtid"]))
    }
    fn sai_record(&mut self, obj: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.obj(obj)["record"]))
    }
    fn record_goal(&mut self, record: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.obj(record)["goal"]))
    }
    fn record_owner_goal(&mut self, record: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.obj(record)["owner_goal"]))
    }
    fn record_object(&mut self, record: Obj) -> Result<u32> {
        Ok(u(&self.obj(record)["object"]))
    }
    fn record_route_state(&mut self, record: Obj) -> Result<Option<u32>> {
        if self.cleared.contains_key(&record.to_string()) {
            return Ok(None);
        }
        let v = &self.obj(record)["route"];
        Ok(if v.is_null() { None } else { Some(u(v)) })
    }
    fn clear_route(&mut self, record: Obj) -> Result<()> {
        self.cleared.insert(record.to_string(), true);
        Ok(())
    }
    fn record_dead(&mut self, record: Obj) -> Result<bool> {
        Ok(self
            .next("record_dead", json!({"record": record}))
            .as_bool()
            .unwrap())
    }
    fn goal_actor(&mut self, goal: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.obj(goal)["record"]))
    }
    fn goal_priority(&mut self, goal: Obj) -> Result<u32> {
        Ok(u(&self.obj(goal)["priority"]))
    }
    fn goal_player(&mut self, goal: Obj) -> Result<Obj> {
        Ok(u(&self.obj(goal)["player"]))
    }
    fn player_object(&mut self, player: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.obj(player)["object"]))
    }
    fn player_ego_304(&mut self, player: Obj) -> Result<u32> {
        Ok(u(&self.obj(player)["ego304"]))
    }
    fn goal_type(&mut self, goal: Obj) -> Result<u32> {
        Ok(u(&self.next("goal_type", json!({"goal": goal}))))
    }
    fn goal_location(&mut self, goal: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.next("goal_location", json!({"goal": goal}))))
    }
    fn goal_held_value(&mut self, goal: Obj, record: Obj) -> Result<Binary> {
        Ok(x87(&self.next(
            "held_value",
            json!({"goal": goal, "record": record}),
        )))
    }
    fn goal_virtual_40(&mut self, goal: Obj, record: Obj) -> Result<bool> {
        Ok(self
            .next("goal_virtual_40", json!({"goal": goal, "record": record}))
            .as_bool()
            .unwrap())
    }
    fn owner(&mut self, obj: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.next("owner", json!({"obj": obj}))))
    }
    fn finalize(&mut self, goal: Obj, owner: Option<Obj>) -> Result<()> {
        self.next(
            "finalize",
            json!({"goal": goal, "owner": id(owner), "draw": 0}),
        );
        Ok(())
    }
    fn goal_ready(&mut self, goal: Obj) -> Result<bool> {
        Ok(self
            .next("goal_ready", json!({"goal": goal}))
            .as_bool()
            .unwrap())
    }
    fn combat_value(&mut self, obj: Obj) -> Result<Binary> {
        Ok(x87(&self.next("combat_value", json!({"obj": obj}))))
    }
    fn region_entry(&mut self, region: Obj, player: Obj) -> Result<RegionEntry> {
        let entries = self.obj(region)["entries"].as_array().unwrap().clone();
        let slot = self.obj(player)["slot"].clone();
        let e = entries
            .iter()
            .find(|e| e["slot"] == slot)
            .unwrap_or_else(|| panic!("case {}: no entry", self.case));
        let v = |k: &str| e[k].as_array().unwrap().iter().map(u).collect::<Vec<_>>();
        Ok(RegionEntry {
            player: 0,
            mobile: v("mobile"),
            settlement: v("settlement"),
            border: v("border"),
        })
    }
    fn kingdom_at(&mut self, index: usize) -> Result<Option<Obj>> {
        Ok(self.c["kingdoms"]
            .as_array()
            .unwrap()
            .get(index)
            .and_then(opt))
    }
    fn virtual_138(&mut self, obj: Obj, other: Option<Obj>) -> Result<bool> {
        Ok(self
            .next("virtual_138", json!({"obj": obj, "other": id(other)}))
            .as_bool()
            .unwrap())
    }
    fn virtual_140(&mut self, obj: Obj, other: Option<Obj>) -> Result<bool> {
        Ok(self
            .next("virtual_140", json!({"obj": obj, "other": id(other)}))
            .as_bool()
            .unwrap())
    }
    fn issue_move(
        &mut self,
        goal: Obj,
        actor: Option<Obj>,
        location: Option<Obj>,
        player: Obj,
    ) -> Result<()> {
        self.next(
            "build_command",
            json!({"goal": goal, "actor": id(actor), "location": id(location)}),
        );
        self.next("queue_command", json!({"player": player}));
        Ok(())
    }
}

#[test]
fn native_reinforce_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-sai/native-reinforce.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() >= 900, "{}", cases.len());
    let mut orders = 0;
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let o = &row["output"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Host {
            c: c.clone(),
            expected: o["events"].as_array().unwrap().iter().cloned().collect(),
            case: i,
            cleared: BTreeMap::new(),
        };
        let goal = u(&c["goal"]);
        let mut list: Vec<u32> = c["list"].as_array().unwrap().iter().map(u).collect();
        let result = if c["entry"] == "reinforce" {
            reinforce(&mut host, goal, &mut list, p)
        } else if c["entry"] == "influence" {
            let player = u(&c["player"]);
            let entry = host.region_entry(u(&c["region"]), player).unwrap();
            let object = host.player_object(player).unwrap();
            let mobile_only = u(&c["flag"]) & 0xff != 0;
            let which = match c["function"].as_str().unwrap() {
                "7B96E0" => Influence::Hostile { mobile_only },
                "7B9600" => Influence::Allied { mobile_only },
                "7B97C0" => Influence::HostileBorder,
                _ => Influence::AlliedBorder,
            };
            influence(&mut host, &entry, object, which, p).map(|bits| {
                assert_eq!(bits, u(&o["st0"]["f32_bits"]), "case {i}: influence");
            })
        } else {
            release_for(&mut host, goal, u(&c["value"]), u(&c["flag"]) & 0xff != 0).map(|r| {
                assert_eq!(json!(r), o["result"], "case {i}: 7C9CE0 result");
            })
        };
        assert!(host.expected.is_empty(), "case {i}: unconsumed {:?}", host.expected);
        assert_eq!(result.is_err(), o["fault"].as_bool().unwrap(), "case {i}: {result:?}");
        let expected: Vec<u32> = o["list"].as_array().unwrap().iter().map(u).collect();
        assert_eq!(list, expected, "case {i}: list");
        for (record, kept) in o["routes"].as_object().unwrap() {
            let original = !c["objects"][record]["route"].is_null();
            let now = original && !host.cleared.contains_key(record);
            assert_eq!(now, kept.as_bool().unwrap(), "case {i}: record {record} +30");
        }
        orders += o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["op"] == "build_command")
            .count();
    }
    assert!(orders > 200, "{orders}");
}
