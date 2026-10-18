//! Replays research/phase8-ai/native-cai-siege.json (original 525930
//! KKC_CAIStateSiege::Think with 51D130, 51E000, 529C20 and the GTID list
//! operations under Unicorn at PC24/PC53/PC64) against
//! aa_mind::cai_thinks::siege::think.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        siege::{SiegeThinkWorld, think},
        siege_target::SiegeTargetWorld,
    },
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::VecDeque;

const STATES: [(&str, u32); 5] = [
    ("siege", 0xf8),
    ("kill", 0xc0),
    ("move", 0x68),
    ("sleep", 0x34),
    ("other", 0x700),
];
const INFO: u32 = 9000;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-siege.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn id(name: &str) -> StateId {
    StateId(STATES.iter().position(|(n, _)| *n == name).unwrap() as u32 + 1)
}
fn state_name(s: StateId) -> &'static str {
    STATES[s.0 as usize - 1].0
}

/// Object handles are index + 1 in key order; list nodes are index + 1.
struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    accepts: VecDeque<bool>,
    target_word90: u32,
    list: Vec<u32>,
    bytes: [u32; 2],
    stamp: u32,
    info_byte14: u32,
}
impl Host {
    fn event(&mut self, event: &str, extra: Value, arena: &Arena) {
        let mut e = json!({"event": event});
        for (k, v) in extra.as_object().unwrap() {
            e[k] = v.clone();
        }
        let states: serde_json::Map<String, Value> = STATES
            .iter()
            .map(|(n, _)| (n.to_string(), json!(arena.states[&id(n)].status)))
            .collect();
        e["states"] = json!(states);
        e["stack"] = json!(
            arena.minds[&MindId(1)]
                .stack
                .iter()
                .map(|s| s.map_or(Value::Null, |s| json!(state_name(s))))
                .collect::<Vec<_>>()
        );
        self.events.push(e);
    }
    fn key(&self, h: u32) -> String {
        self.keys[h as usize - 1].clone()
    }
    fn handle(&self, key: &str) -> u32 {
        self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
    }
    fn object(&self, h: u32) -> &Value {
        &self.c["objects"][&self.keys[h as usize - 1]]
    }
}
impl World for Host {
    fn can_activate(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        unreachable!()
    }
    fn enter(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("leave", json!({"state": state_name(state)}), arena);
        Ok(())
    }
    fn schedule(&mut self, _m: MindId, _d: u32, _x: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn reset_default(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn state_changed(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
}
impl RegimentWorld for Host {
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        Ok(self
            .keys
            .iter()
            .position(|k| {
                let o = &self.c["objects"][k];
                u(&o["gtid"]) == gtid && o["registered"].as_bool().unwrap()
            })
            .map(|i| i as u32 + 1))
    }
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["gtid"]))
    }
    fn embedded_state(&mut self, _mind: MindId, offset: u32) -> Result<StateId> {
        Ok(StateId(
            STATES.iter().position(|(_, o)| *o == offset).unwrap() as u32 + 1,
        ))
    }
    fn embedded_think(&mut self, _mind: MindId, offset: u32, arena: &mut Arena) -> Result<()> {
        let name = STATES.iter().find(|(_, o)| *o == offset).unwrap().0;
        self.event("embedded_think", json!({"state": name}), arena);
        Ok(())
    }
    fn mind_virtual_140(&mut self, _mind: MindId, gtid: u32, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_140", json!({"args": [gtid]}), arena);
        Ok(())
    }
    fn mind_virtual_b8(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
}
impl SiegeTargetWorld for Host {
    fn object_word90(&mut self, object: u32) -> Result<u32> {
        if self.key(object) == "target" {
            return Ok(self.target_word90);
        }
        Ok(u(&self.object(object)["word90"]))
    }
    fn object_4d11d0(&mut self, object: u32, arena: &mut Arena) -> Result<Option<u32>> {
        let site = self.object(object)["site"].clone();
        let k = self.key(object);
        let result = if site.is_null() {
            json!(0)
        } else {
            site.clone()
        };
        self.event(
            "object_4d11d0",
            json!({"object": k, "result": result}),
            arena,
        );
        Ok(site.as_str().map(|s| self.handle(s)))
    }
    fn organization_virtual_16c(
        &mut self,
        _mind: MindId,
        site: u32,
        arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.c["org_16c"].as_bool().unwrap();
        let k = self.key(site);
        self.event(
            "organization_virtual_16c",
            json!({"site": k, "result": r}),
            arena,
        );
        Ok(r)
    }
    fn component_57a500(
        &mut self,
        _mind: MindId,
        object: u32,
        a: bool,
        b: bool,
        arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.accepts.pop_front().unwrap();
        let k = self.key(object);
        self.event(
            "component_57a500",
            json!({"object": k, "flags": [u32::from(a), u32::from(b)], "result": r}),
            arena,
        );
        Ok(r)
    }
    fn set_siege_words(&mut self, _state: StateId, _target: u32, _site: u32) -> Result<()> {
        unreachable!()
    }
}
impl SiegeThinkWorld for Host {
    fn siege_target(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["target_gtid"]))
    }
    fn siege_site(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["site_gtid"]))
    }
    fn site_anchor(&mut self, site: u32, arena: &mut Arena) -> Result<Option<u32>> {
        let a = self.c["anchor"].clone();
        let k = self.key(site);
        let result = if a.is_null() { json!(0) } else { a.clone() };
        self.event("site_4ceda0", json!({"site": k, "result": result}), arena);
        Ok(a.as_str().map(|s| self.handle(s)))
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let p = &self.object(object)["position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn organization_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        let p = &self.c["org_position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn siege_range_base(&mut self) -> Result<u32> {
        Ok(u(&self.c["range_base"]))
    }
    fn svar_c4(&mut self) -> Result<u32> {
        Ok(u(&self.c["svar_c4"]))
    }
    fn svar_c8(&mut self) -> Result<u32> {
        Ok(u(&self.c["svar_c8"]))
    }
    fn game_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["time"]))
    }
    fn siege_attacking(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.bytes[0] != 0)
    }
    fn set_siege_attacking(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.bytes[0] = u32::from(value);
        Ok(())
    }
    fn set_siege_approaching(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.bytes[1] = u32::from(value);
        Ok(())
    }
    fn siege_stamp(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.stamp)
    }
    fn set_siege_stamp(&mut self, _state: StateId, value: u32) -> Result<()> {
        self.stamp = value;
        Ok(())
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn move_order(&mut self, _mind: MindId, args: [u32; 6], arena: &mut Arena) -> Result<()> {
        self.event("move_order", json!({"args": args}), arena);
        Ok(())
    }
    fn kill_target(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["kill_target"]))
    }
    fn kill_activate(
        &mut self,
        _mind: MindId,
        mode: u32,
        target: u32,
        arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.c["kill_activate"].as_bool().unwrap();
        let k = self.key(target);
        self.event(
            "kill_activate",
            json!({"mode": mode, "target": k, "result": r}),
            arena,
        );
        if self.c["kill_activate_sets_word90"].as_bool().unwrap() {
            self.target_word90 = 1;
        }
        Ok(r)
    }
    fn mind_virtual_44(&mut self, _mind: MindId, arena: &mut Arena) -> Result<Option<u32>> {
        let null = self.c["info_null"].as_bool().unwrap();
        let result = if null { json!(0) } else { json!("info") };
        self.event("mind_virtual_44", json!({"result": result}), arena);
        Ok((!null).then_some(INFO))
    }
    fn info_word4(&mut self, info: u32) -> Result<u32> {
        assert_eq!(info, INFO);
        Ok(u(&self.c["info_word4"]))
    }
    fn info_byte14(&mut self, _info: u32) -> Result<bool> {
        Ok(self.info_byte14 != 0)
    }
    fn set_info_byte14(&mut self, _info: u32, value: bool) -> Result<()> {
        self.info_byte14 = u32::from(value);
        Ok(())
    }
    fn siege_list_clear(&mut self, _state: StateId) -> Result<()> {
        self.list.clear();
        Ok(())
    }
    fn siege_list_push(&mut self, _state: StateId, gtid: u32) -> Result<()> {
        self.list.push(gtid);
        Ok(())
    }
    fn siege_fill(
        &mut self,
        _state: StateId,
        info: u32,
        site: u32,
        arena: &mut Arena,
    ) -> Result<()> {
        assert_eq!(info, INFO);
        let fill: Vec<u32> = self.c["fill"].as_array().unwrap().iter().map(u).collect();
        let k = self.key(site);
        self.event("siege_fill", json!({"site": k, "gtids": fill}), arena);
        self.list.extend(fill);
        Ok(())
    }
    fn siege_list_head(&mut self, _state: StateId) -> Result<Option<u32>> {
        Ok((!self.list.is_empty()).then_some(1))
    }
    fn list_node_gtid(&mut self, node: u32) -> Result<u32> {
        Ok(self.list[node as usize - 1])
    }
    fn list_node_next(&mut self, node: u32) -> Result<Option<u32>> {
        Ok(((node as usize) < self.list.len()).then_some(node + 1))
    }
    fn organization_count_14c(&mut self, _mind: MindId) -> Result<i32> {
        Ok(u(&self.c["count"]) as i32)
    }
    fn organization_entry_148(&mut self, _mind: MindId, index: u32) -> Result<u32> {
        assert_eq!(index, 4);
        Ok(if self.c["entry8"].is_null() { 0 } else { 77 })
    }
    fn world_4f3d90(&mut self, entry: u32, arena: &mut Arena) -> Result<()> {
        assert_eq!(entry, 77);
        self.event("world_4f3d90", json!({}), arena);
        Ok(())
    }
    fn entry_word8(&mut self, entry: u32) -> Result<u32> {
        assert_eq!(entry, 77);
        Ok(u(&self.c["entry8"]))
    }
    fn organization_virtual_0(&mut self, _mind: MindId, arena: &mut Arena) -> Result<Binary> {
        let r = u(&self.c["org_reach"]);
        self.event("organization_virtual_0", json!({"result": r}), arena);
        Ok(Binary::from_f32_bits(r).unwrap())
    }
    fn object_virtual_0(&mut self, object: u32, arena: &mut Arena) -> Result<Binary> {
        let r = u(&self.object(object)["reach"]);
        let k = self.key(object);
        self.event("object_virtual_0", json!({"object": k, "result": r}), arena);
        Ok(Binary::from_f32_bits(r).unwrap())
    }
    fn target_in_range(&mut self, _mind: MindId, object: u32, arena: &mut Arena) -> Result<bool> {
        let r = self.object(object)["in_range"].as_bool().unwrap();
        let k = self.key(object);
        self.event("target_in_range", json!({"object": k, "result": r}), arena);
        Ok(r)
    }
    fn mind_virtual_f0(&mut self, _mind: MindId, object: u32, arena: &mut Arena) -> Result<bool> {
        let r = self.c["mind_f0"].as_bool().unwrap();
        let k = self.key(object);
        self.event("mind_virtual_f0", json!({"object": k, "result": r}), arena);
        Ok(r)
    }
}

#[test]
fn native_siege_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 900);
    for (_, cov) in f["coverage"].as_object().unwrap() {
        assert_eq!(cov["visited"], cov["instructions"]);
    }
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        mind.stack = c["stack"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().map(id))
            .collect();
        arena.minds.insert(MindId(1), mind);
        for (n, _) in STATES {
            arena.states.insert(
                id(n),
                State {
                    parent: MindId(1),
                    discard_on_suspend: n == "sleep",
                    status: u(&c["states"][n]),
                },
            );
        }
        let keys: Vec<String> = c["objects"].as_object().unwrap().keys().cloned().collect();
        let mut host = Host {
            c: c.clone(),
            keys,
            events: vec![],
            accepts: c["accepts"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b.as_bool().unwrap())
                .collect(),
            target_word90: u(&c["objects"]["target"]["word90"]),
            list: c["list"].as_array().unwrap().iter().map(u).collect(),
            bytes: [u(&c["byte20"]), u(&c["byte21"])],
            stamp: u(&c["stamp"]),
            info_byte14: u(&c["info_byte14"]),
        };
        let status_in = u(&c["states"]["siege"]);
        let result = think(id("siege"), &mut arena, &mut host, p);
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "diagnostic_log")
            .cloned()
            .collect();
        let logged = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["event"] == "diagnostic_log");
        assert_eq!(host.events, native, "{name}");
        if !o["fault"].is_null() {
            assert!(matches!(result, Err(Error::World(_))), "{name}");
            faults += 1;
            continue;
        }
        let wrong = result.unwrap();
        assert_eq!(wrong, logged, "{name}");
        assert_eq!(wrong, status_in != 2, "{name}");
        let states: serde_json::Map<String, Value> = STATES
            .iter()
            .map(|(n, _)| (n.to_string(), json!(arena.states[&id(n)].status)))
            .collect();
        assert_eq!(json!(states), o["states"], "{name}");
        assert_eq!(json!(host.list), o["list"], "{name}");
        assert_eq!(json!(host.bytes), o["bytes"], "{name}");
        assert_eq!(json!(host.stamp), o["stamp"], "{name}");
        let info = if c["info_null"].as_bool().unwrap() {
            Value::Null
        } else {
            json!(host.info_byte14)
        };
        assert_eq!(info, o["info_byte14"], "{name}");
    }
    assert_eq!(faults, 1);
}
