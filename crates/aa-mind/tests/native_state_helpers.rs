//! Replays research/phase8-ai/native-state-helpers.json (original 51E1D0,
//! 52D6D0, 51D3F0, 51EB80, 52E1D0 and 51E030 under Unicorn, at
//! PC24/PC53/PC64) against aa_mind::state_helpers.
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    state_helpers::{
        MemberActivateWorld, TargetActivateWorld, activate_if_member, activate_with_target,
        any_resolves, engage_flag, first_or_zero, near_destination,
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const STATES: [(&str, u32); 11] = [
    ("unload", 0x34),
    ("deploy", 0x48),
    ("sleep", 0x68),
    ("move", 0x7c),
    ("kill", 0xf8),
    ("bombard", 0x10c),
    ("engage", 0x12c),
    ("repair", 0x148),
    ("join", 0x164),
    ("reattach", 0x178),
    ("unpack", 0x188),
];
const TARGET: u32 = 9;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-state-helpers.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn state_name(s: StateId) -> &'static str {
    STATES[s.0 as usize - 1].0
}
fn state_id(name: &str) -> StateId {
    StateId(STATES.iter().position(|(n, _)| *n == name).unwrap() as u32 + 1)
}

struct Host {
    c: Value,
    events: Vec<Value>,
    target: u32,
    byte15: u8,
}
impl World for Host {
    fn can_activate(&mut self, state: StateId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["can_activate"].as_bool().unwrap();
        self.events
            .push(json!({"event": "can_activate", "state": state_name(state), "result": r}));
        Ok(r)
    }
    fn enter(&mut self, state: StateId, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "enter", "state": state_name(state)}));
        Ok(())
    }
    fn leave(&mut self, state: StateId, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "leave", "state": state_name(state)}));
        Ok(())
    }
    fn schedule(&mut self, _m: MindId, delay_bits: u32, data: u32, _a: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "schedule", "delay": delay_bits, "data": data}));
        Ok(())
    }
    fn reset_default(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "reset_default"}));
        Ok(())
    }
    fn state_changed(&mut self, _m: MindId, _a: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "state_changed"}));
        Ok(())
    }
}
impl TargetActivateWorld for Host {
    fn target_gtid(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(u(&self.c["target"]))
    }
    fn set_state_target(&mut self, _state: StateId, gtid: u32) -> Result<()> {
        self.target = gtid;
        Ok(())
    }
    fn set_state_byte15(&mut self, _state: StateId, value: u8) -> Result<()> {
        self.byte15 = value;
        Ok(())
    }
}
impl MemberActivateWorld for Host {
    fn has_organization(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.c["membership"].as_bool().unwrap_or(false))
    }
}

fn arena_for(c: &Value) -> Arena {
    let mut arena = Arena::default();
    let mut mind = Mind::base(1);
    for key in c["stack"].as_array().unwrap() {
        mind.stack.push_back(key.as_str().map(state_id));
    }
    arena.minds.insert(MindId(1), mind);
    for (i, (n, _)) in STATES.iter().enumerate() {
        arena.states.insert(
            StateId(i as u32 + 1),
            State {
                parent: MindId(1),
                discard_on_suspend: matches!(*n, "deploy" | "sleep"),
                status: u(&c["status"][*n]),
            },
        );
    }
    arena
}
fn snapshot(arena: &Arena) -> (Value, Value) {
    let status: serde_json::Map<String, Value> = STATES
        .iter()
        .enumerate()
        .map(|(i, (n, _))| {
            (
                n.to_string(),
                json!(arena.states[&StateId(i as u32 + 1)].status),
            )
        })
        .collect();
    let stack: Vec<Value> = arena.minds[&MindId(1)]
        .stack
        .iter()
        .map(|s| s.map_or(json!(0), |s| json!(state_name(s))))
        .collect();
    (json!(status), json!(stack))
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}

#[test]
fn native_state_helpers_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 1020);
    for (name, total) in [("51E1D0", 32), ("52D6D0", 18), ("51D3F0", 6), ("52E1D0", 24), ("51E030", 17)] {
        assert_eq!(f["coverage"][name], json!([total, total]), "{name}");
    }
    // 51EB80: the one instruction left is alignment padding jumped over.
    assert_eq!(f["coverage"]["51EB80"], json!([29, 30]));
    let mut near = [0; 2];
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        match c["entry"].as_str().unwrap() {
            e @ ("51E1D0" | "52D6D0") => {
                let mut arena = arena_for(c);
                let mut host = Host {
                    c: c.clone(),
                    events: vec![],
                    target: 0x5a5a_5a5a,
                    byte15: 0x5a,
                };
                let r = if e == "51E1D0" {
                    let target = (!c["target"].is_null()).then_some(TARGET);
                    let flag = u(&c["flag"]) as u8;
                    activate_with_target(
                        state_id("engage"),
                        u(&c["mode"]),
                        target,
                        flag,
                        &mut arena,
                        &mut host,
                    )
                } else {
                    activate_if_member(state_id("join"), u(&c["mode"]), &mut arena, &mut host)
                }
                .unwrap();
                assert_eq!(json!(host.events), o["events"], "{name}");
                assert_eq!(json!(u8::from(r)), o["result"], "{name}");
                if e == "51E1D0" {
                    assert_eq!(json!(host.target), o["target"], "{name}");
                    assert_eq!(json!(host.byte15), o["byte15"], "{name}");
                }
                let (status, stack) = snapshot(&arena);
                assert_eq!(status, o["status"], "{name}");
                assert_eq!(stack, o["stack"], "{name}");
            }
            "51D3F0" => {
                let r = engage_flag(u(&c["engage_status"]), u(&c["byte"]) as u8);
                let native = u(&o["result"]) != 0;
                assert_eq!(r, native, "{name}");
            }
            "51EB80" => {
                let registered: Vec<u32> = c["registered"].as_array().unwrap().iter().map(u).collect();
                let gtids: Vec<u32> = c["gtids"].as_array().unwrap().iter().map(u).collect();
                let r = any_resolves(u(&c["byte20"]) != 0, &gtids, |g| {
                    Ok(registered.contains(&g).then_some(1))
                })
                .unwrap();
                assert_eq!(json!(u8::from(r)), o["result"], "{name}");
            }
            "52E1D0" => {
                let r = near_destination(pair(&c["pos"]), pair(&c["dest"]), p).unwrap();
                assert_eq!(json!(u8::from(r)), o["result"], "{name}");
                near[usize::from(r)] += 1;
            }
            _ => {
                let r = first_or_zero(u(&c["count"]), || Ok(u(&c["first"]))).unwrap();
                assert_eq!(json!(r), o["result"], "{name}");
            }
        }
    }
    assert!(near[0] > 40 && near[1] > 40, "{near:?}");
}
