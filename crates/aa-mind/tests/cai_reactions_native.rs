//! Replays research/phase8-ai/native-cai-reactions.json (original 51F9F0,
//! 51F920, 5206A0, 52E830, 527240 under Unicorn).
use aa_mind::{
    Arena, Mind, MindId, Result, State, StateId, World,
    cai_reactions::{
        KillEnterWorld, PriorityWorld, ReactionWorld, SaiRecord, engage_reaction_delay, exhausted,
        kill_on_enter, target_priority, wavering,
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const STATES: [&str; 5] = ["engage", "rout", "other", "move", "kill"];

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-reactions.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn sid(name: &str) -> StateId {
    StateId(STATES.iter().position(|n| *n == name).unwrap() as u32 + 1)
}
fn sname(id: StateId) -> &'static str {
    STATES[id.0 as usize - 1]
}

struct Host {
    c: Value,
    events: Vec<Value>,
}
impl Host {
    fn snap(arena: &Arena) -> Value {
        let status: serde_json::Map<String, Value> = STATES
            .iter()
            .map(|n| (n.to_string(), json!(arena.states[&sid(n)].status)))
            .collect();
        let stack: Vec<Value> = arena.minds[&MindId(1)]
            .stack
            .iter()
            .map(|s| s.map_or(Value::Null, |s| json!(sname(s))))
            .collect();
        json!({"status": status, "stack": stack})
    }
    fn event(&mut self, name: &str, extra: Value, arena: &Arena) {
        let mut e = json!({"event": name});
        for (k, v) in extra.as_object().unwrap() {
            e[k] = v.clone();
        }
        for (k, v) in Self::snap(arena).as_object().unwrap() {
            e[k] = v.clone();
        }
        self.events.push(e);
    }
}
impl World for Host {
    fn can_activate(&mut self, state: StateId, arena: &mut Arena) -> Result<bool> {
        self.event("can_activate", json!({"state": sname(state)}), arena);
        Ok(self.c["can_activate"].as_bool().unwrap_or(true))
    }
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("enter", json!({"state": sname(state)}), arena);
        Ok(())
    }
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        self.event("leave", json!({"state": sname(state)}), arena);
        Ok(())
    }
    fn schedule(&mut self, _m: MindId, delay: u32, data: u32, arena: &mut Arena) -> Result<()> {
        self.event("schedule", json!({"delay": delay, "data": data}), arena);
        Ok(())
    }
    fn reset_default(&mut self, _m: MindId, arena: &mut Arena) -> Result<()> {
        self.event("default", json!({}), arena);
        Ok(())
    }
    fn state_changed(&mut self, _m: MindId, arena: &mut Arena) -> Result<()> {
        self.event("changed", json!({}), arena);
        Ok(())
    }
}
impl Host {
    fn session_mode(&self) -> u32 {
        self.c["session_mode"].as_u64().unwrap_or(0) as u32
    }
}
/// Wraps the host with an arena snapshot for the two reaction events that
/// the trait does not pass the arena to.
struct NotifyHost {
    host: Host,
    snapshot: Value,
}
impl World for NotifyHost {
    fn can_activate(&mut self, s: StateId, a: &mut Arena) -> Result<bool> {
        self.host.can_activate(s, a)
    }
    fn enter(&mut self, s: StateId, a: &mut Arena) -> Result<()> {
        self.host.enter(s, a)
    }
    fn leave(&mut self, s: StateId, a: &mut Arena) -> Result<()> {
        self.host.leave(s, a)
    }
    fn schedule(&mut self, m: MindId, d: u32, x: u32, a: &mut Arena) -> Result<()> {
        self.host.schedule(m, d, x, a)
    }
    fn reset_default(&mut self, m: MindId, a: &mut Arena) -> Result<()> {
        self.host.reset_default(m, a)
    }
    fn state_changed(&mut self, m: MindId, a: &mut Arena) -> Result<()> {
        self.host.state_changed(m, a)
    }
}
impl ReactionWorld for NotifyHost {
    fn session_mode(&mut self) -> Result<u32> {
        Ok(self.host.session_mode())
    }
    fn sai_record(&mut self, _mind: MindId) -> Result<Option<SaiRecord>> {
        let r = &self.host.c["record"];
        Ok((!r.is_null()).then(|| SaiRecord {
            word10: r["word10"].as_u64().unwrap() as u32,
            word3c: r["word3c"].as_u64().unwrap() as u32,
            state30: r["state30"].as_u64().map(|v| v as u32),
        }))
    }
    fn notify_sai(&mut self, _mind: MindId) -> Result<()> {
        let mut e = json!({"event": "notify_sai"});
        for (k, v) in self.snapshot.as_object().unwrap() {
            e[k] = v.clone();
        }
        self.host.events.push(e);
        Ok(())
    }
    fn actor_blocked(&mut self, _mind: MindId) -> Result<bool> {
        let r = self.host.c["blocked"].as_bool().unwrap_or(false);
        let mut e = json!({"event": "actor_blocked", "result": r});
        for (k, v) in self.snapshot.as_object().unwrap() {
            e[k] = v.clone();
        }
        self.host.events.push(e);
        Ok(r)
    }
}
impl KillEnterWorld for Host {
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        let t = &self.c["targets"]["t0"];
        Ok(
            (t["gtid"].as_u64() == Some(gtid as u64) && t["registered"].as_bool() != Some(false))
                .then_some(0),
        )
    }
    fn activate_move_to_actor(&mut self, target: Option<u32>) -> Result<bool> {
        let r = self.c["move_result"].as_bool().unwrap();
        self.events.push(json!({"event": "activate_move_to_actor",
            "target": target.map(|_| "t0"), "zero": self.c["zero"], "result": r}));
        if !r {
            self.events.push(json!({"event": "diagnostic"}));
        }
        Ok(r)
    }
}
impl PriorityWorld for Host {
    fn target_4d03f0(&mut self, _t: u32) -> Result<bool> {
        let r = self.c["targets"]["t0"]["a"].as_bool().unwrap_or(false);
        self.events
            .push(json!({"event": "target_4d03f0", "target": "t0", "result": r}));
        Ok(r)
    }
    fn target_mind_58(&mut self, _t: u32) -> Result<Option<bool>> {
        let t = &self.c["targets"]["t0"];
        if !t["mind"].as_bool().unwrap_or(false) {
            return Ok(None);
        }
        let r = t["mind58"].as_bool().unwrap_or(false);
        self.events
            .push(json!({"event": "target_mind_58", "target": "t0", "result": r}));
        Ok(Some(r))
    }
    fn current_target(&mut self) -> Result<Option<u32>> {
        let cur = self.c["current_target"].clone();
        self.events
            .push(json!({"event": "current_target", "result": cur}));
        Ok(match cur.as_str() {
            Some("t0") => Some(0),
            Some(_) => Some(1),
            None => None,
        })
    }
    fn kill_target_is_highest(&mut self) -> Result<bool> {
        let r = self.c["kill_target_is_highest"].as_bool().unwrap();
        self.events
            .push(json!({"event": "kill_target_is_highest", "result": r}));
        Ok(r)
    }
    fn plain_target(&mut self, _t: u32) -> Result<bool> {
        let t = &self.c["targets"]["t0"];
        Ok(t["word60"].as_u64().unwrap_or(0) != 0
            && t["b290"].as_u64().unwrap_or(0) == 0
            && t["b291"].as_u64().unwrap_or(0) == 0)
    }
}

fn arena(c: &Value) -> Arena {
    let mut arena = Arena::default();
    let mut mind = Mind::base(1);
    for s in c["stack"].as_array().into_iter().flatten() {
        mind.stack.push_back(Some(sid(s.as_str().unwrap())));
    }
    arena.minds.insert(MindId(1), mind);
    for n in STATES {
        arena.states.insert(
            sid(n),
            State {
                parent: MindId(1),
                discard_on_suspend: c["discard"][n].as_u64().unwrap_or(0) != 0,
                status: c["status"][n].as_u64().unwrap_or(0) as u32,
            },
        );
    }
    arena
}

#[test]
fn native_mind_reactions_match_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 248);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = arena(c);
        let host = Host {
            c: c.clone(),
            events: vec![],
        };
        let (events, result) = match c["entry"].as_str().unwrap() {
            e @ ("exhausted" | "wavering") => {
                let snapshot = Host::snap(&arena);
                let mut nh = NotifyHost { host, snapshot };
                let r = if e == "exhausted" {
                    exhausted(MindId(1), sid("engage"), sid("rout"), &mut arena, &mut nh)
                } else {
                    wavering(MindId(1), sid("rout"), &mut arena, &mut nh)
                }
                .unwrap();
                (nh.host.events, json!(u8::from(r)))
            }
            "kill_enter" => {
                let mut h = host;
                let gtid = c["kill_target_gtid"].as_u64().unwrap() as u32;
                kill_on_enter(sid("kill"), gtid, &mut arena, &mut h).unwrap();
                (h.events, Value::Null)
            }
            "reaction" => {
                let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
                let mut rng = c["rng"].as_u64().unwrap() as u32;
                let wide = u64::from(rng) * 0x41c6_4e6d + 0x3039;
                rng = wide as u32;
                let draw = (wide >> 16) as u32;
                let range = c["reaction_range"].as_u64().unwrap() as u32;
                let delay = engage_reaction_delay(draw, range, p).unwrap();
                let mut h = host;
                h.schedule(MindId(1), delay, 0, &mut arena).unwrap();
                assert_eq!(json!(rng), o["rng"], "{name}");
                (h.events, Value::Null)
            }
            "priority" => {
                let mut h = host;
                let r = target_priority(0, &mut h).unwrap();
                (h.events, json!(r))
            }
            other => panic!("{other}"),
        };
        assert_eq!(json!(events), o["events"], "{name}");
        assert_eq!(result, o["result"], "{name}");
        assert_eq!(Host::snap(&arena)["status"], o["status"], "{name}");
        assert_eq!(Host::snap(&arena)["stack"], o["stack"], "{name}");
    }
}
