//! Replays research/phase8-ai/native-cai-rampage.json (original 5267E0
//! KKC_CAIStateRampage::Think with 51D6E0, 51E910, 526740, 529C20, 52A810
//! mode 0 and the world RNG draw under Unicorn at PC24/PC53/PC64) against
//! aa_mind::cai_thinks::rampage::think.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_thinks::{
        RegimentWorld,
        rampage::{RampageThinkWorld, think},
    },
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

const STATES: [(&str, u32); 5] = [
    ("rampage", 0x54),
    ("sleep", 0x34),
    ("move", 0x68),
    ("kill", 0xc0),
    ("other", 0x700),
];
/// Target object handles are 500 + index in key order; member handles are
/// index + 1, their +80 objects 100 + index and their Minds 200 + index.
const OBJECT_BASE: u32 = 500;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-rampage.json"
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

struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    byte10: bool,
    rng: u32,
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
    fn member(&self, h: u32) -> &Value {
        &self.c["members"][(h % 100) as usize - 1]
    }
    fn key(&self, object: u32) -> Value {
        json!(self.keys[(object - OBJECT_BASE) as usize])
    }
}
impl World for Host {
    fn can_activate(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        unreachable!("mode 0 never asks")
    }
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("enter", json!({"state": state_name(state)}), arena);
        Ok(())
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
            .map(|i| i as u32 + OBJECT_BASE))
    }
    fn object_gtid(&mut self, _object: u32) -> Result<u32> {
        unreachable!()
    }
    fn embedded_state(&mut self, _mind: MindId, offset: u32) -> Result<StateId> {
        Ok(StateId(
            STATES.iter().position(|(_, o)| *o == offset).unwrap() as u32 + 1,
        ))
    }
    fn embedded_think(&mut self, _mind: MindId, offset: u32, arena: &mut Arena) -> Result<()> {
        let name = STATES.iter().find(|(_, o)| *o == offset).unwrap().0;
        self.event("embedded_think", json!({"state": name}), arena);
        if let Some(effects) = self.c["think_effects"][name].as_object() {
            for (k, v) in effects {
                arena.states.get_mut(&id(k)).unwrap().status = u(v);
            }
        }
        Ok(())
    }
    fn mind_virtual_140(&mut self, _mind: MindId, _gtid: u32, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn mind_virtual_b8(&mut self, _mind: MindId, arena: &mut Arena) -> Result<()> {
        self.event("mind_virtual_b8", json!({"args": []}), arena);
        Ok(())
    }
}
impl RampageThinkWorld for Host {
    fn rampage_wander_chance(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["wander_chance"]))
    }
    fn rampage_threshold(&mut self, _mind: MindId) -> Result<u32> {
        Ok(u(&self.c["threshold"]))
    }
    fn rampage_hunts(&mut self, _mind: MindId) -> Result<bool> {
        Ok(u(&self.c["hunts"]) != 0)
    }
    fn component_5c_60(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        Ok([u(&self.c["comp5c"]), u(&self.c["comp60"])])
    }
    fn rampaging(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.byte10)
    }
    fn set_rampaging(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.byte10 = value;
        Ok(())
    }
    fn component_578790(&mut self, _mind: MindId, arena: &mut Arena) -> Result<Binary> {
        let r = self.c["fraction"].as_u64().unwrap();
        self.event("component_578790", json!({"result": r}), arena);
        Ok(Binary::from_f64_bits(r).unwrap())
    }
    fn component_member_count(&mut self, _mind: MindId) -> Result<i32> {
        Ok(u(&self.c["member_count"]) as i32)
    }
    fn component_member(&mut self, _mind: MindId, index: u32) -> Result<u32> {
        Ok(if self.c["members"][index as usize].is_null() {
            0
        } else {
            index + 1
        })
    }
    fn member_word80(&mut self, member: u32) -> Result<Option<u32>> {
        Ok((!self.member(member)["c80"].is_null()).then_some(100 + member))
    }
    fn word80_byte1c(&mut self, object: u32) -> Result<bool> {
        Ok(u(&self.member(object)["c80"]) != 0)
    }
    fn member_mind(&mut self, member: u32) -> Result<Option<u32>> {
        Ok(self.member(member)["mind"]
            .as_bool()
            .unwrap()
            .then_some(200 + member))
    }
    fn member_mind_virtual_fc(&mut self, member_mind: u32, arena: &mut Arena) -> Result<()> {
        self.event(
            "member_mind_virtual_fc",
            json!({"member": member_mind - 201}),
            arena,
        );
        Ok(())
    }
    fn component_577f20(&mut self, _mind: MindId, arena: &mut Arena) -> Result<u32> {
        let r = u(&self.c["radius"]);
        self.event("component_577f20", json!({"result": r}), arena);
        Ok(r)
    }
    fn nearest_target(
        &mut self,
        _mind: MindId,
        radius: u32,
        arena: &mut Arena,
    ) -> Result<Option<u32>> {
        let r = self.c["nearest"].clone();
        let position = self.c["org_position"].clone();
        let result = if r.is_null() { json!(0) } else { r.clone() };
        self.event(
            "nearest_target",
            json!({"position": position, "radius": radius, "result": result}),
            arena,
        );
        Ok(r.as_str()
            .map(|k| self.keys.iter().position(|x| x == k).unwrap() as u32 + OBJECT_BASE))
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
        Ok(r)
    }
    fn random(&mut self) -> Result<u32> {
        let wide = u64::from(self.rng) * 0x41c6_4e6d + 0x3039;
        self.rng = wide as u32;
        Ok((wide >> 16) as u32)
    }
    fn wander_point(&mut self, _state: StateId, arena: &mut Arena) -> Result<[u32; 2]> {
        let p = [u(&self.c["wander_point"][0]), u(&self.c["wander_point"][1])];
        self.event("wander_point", json!({"result": p}), arena);
        Ok(p)
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn move_order(&mut self, _mind: MindId, args: [u32; 6], arena: &mut Arena) -> Result<()> {
        self.event("move_order", json!({"args": args}), arena);
        Ok(())
    }
}

#[test]
fn native_rampage_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
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
                    discard_on_suspend: n == "sleep" || n == "rampage",
                    status: u(&c["states"][n]),
                },
            );
        }
        let mut host = Host {
            c: c.clone(),
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            byte10: u(&c["byte10"]) != 0,
            rng: u(&c["rng"]),
        };
        let status_in = u(&c["states"]["rampage"]);
        let result = think(id("rampage"), &mut arena, &mut host, p);
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
        let stack: Vec<Value> = arena.minds[&MindId(1)]
            .stack
            .iter()
            .map(|s| s.map_or(Value::Null, |s| json!(state_name(s))))
            .collect();
        assert_eq!(json!(stack), o["stack"], "{name}");
        // A nonzero byte other than 1 survives unless the Think rewrites it.
        let byte10 = u(&o["byte10"]);
        if byte10 <= 1 {
            assert_eq!(json!(u32::from(host.byte10)), o["byte10"], "{name}");
        } else {
            assert!(host.byte10 && byte10 == u(&c["byte10"]), "{name}");
        }
        assert_eq!(json!(host.rng), o["rng"], "{name}");
    }
    assert_eq!(faults, 2);
}
