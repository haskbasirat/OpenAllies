//! Replays research/phase8-ai/native-sleep-think.json (original 524D70
//! KKC_CAIStateSleep::Think with 52A810 under Unicorn) against
//! aa_mind::cai_sleep::think.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_sleep::{SleepThinkWorld, think},
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-sleep-think.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
const NAMES: [&str; 4] = ["sleep", "s54", "sC0", "other"];
const ORG: u32 = 0xa0a0;
const OTHER_ORG: u32 = 0x515_0000;

struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    arrivals: Vec<u32>,
    point: [u32; 2],
    word1c: u32,
}
impl Host {
    fn object(&self, handle: u32) -> &Value {
        &self.c["objects"][&self.keys[handle as usize - 1]]
    }
    fn name(&self, handle: u32) -> Value {
        if handle == 0 {
            json!(0)
        } else {
            json!(self.keys[handle as usize - 1])
        }
    }
    fn component(&self) -> Result<u32> {
        self.c["component"]
            .as_u64()
            .map(|v| v as u32)
            .ok_or_else(|| Error::World("null organization component".into()))
    }
}
fn state_name(state: StateId) -> &'static str {
    NAMES[state.0 as usize - 1]
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
    fn schedule(
        &mut self,
        _mind: MindId,
        delay_bits: u32,
        data: u32,
        _arena: &mut Arena,
    ) -> Result<()> {
        self.events
            .push(json!({"event": "schedule", "delay": delay_bits, "data": data}));
        Ok(())
    }
    fn reset_default(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "reset_default"}));
        Ok(())
    }
    fn state_changed(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "state_changed"}));
        Ok(())
    }
}
impl SleepThinkWorld for Host {
    fn organization_status(&mut self, _mind: MindId, _arena: &Arena) -> Result<u32> {
        self.component()?;
        let r = u(&self.c["status"]);
        self.events
            .push(json!({"event": "organization_status", "result": r}));
        Ok(r)
    }
    fn exhausted_reaction(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["exhausted"].as_bool().unwrap();
        self.events
            .push(json!({"event": "exhausted_reaction", "result": r}));
        Ok(r)
    }
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
    fn object_removed(&mut self, object: u32) -> Result<bool> {
        Ok(u(&self.object(object)["flags"]) & 1 != 0)
    }
    fn organization_follows(
        &mut self,
        _mind: MindId,
        object: u32,
        _arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.c["follows"].as_bool().unwrap();
        let o = self.name(object);
        self.events
            .push(json!({"event": "organization_follows", "object": o, "result": r}));
        Ok(r)
    }
    fn mind_virtual_188(&mut self, _mind: MindId, object: u32, _arena: &mut Arena) -> Result<()> {
        let o = self.name(object);
        self.events
            .push(json!({"event": "mind_virtual_188", "object": o}));
        Ok(())
    }
    fn mind_virtual_190(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "mind_virtual_190"}));
        Ok(())
    }
    fn follow_activate(&mut self, _mind: MindId, target: u32, _arena: &mut Arena) -> Result<()> {
        let t = self.name(target);
        self.events
            .push(json!({"event": "follow_activate", "target": t}));
        Ok(())
    }
    fn embedded_think(&mut self, _mind: MindId, offset: u32, _arena: &mut Arena) -> Result<()> {
        assert_eq!(offset, 0xc0);
        self.events
            .push(json!({"event": "embedded_think", "state": "sC0"}));
        Ok(())
    }
    fn organization_4d7960(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "organization_4d7960"}));
        Ok(())
    }
    fn home_component(&mut self, object: u32) -> Result<Option<u32>> {
        let v = u(&self.object(object)["home_component"]);
        Ok((v != 0).then_some(v))
    }
    fn home_settle(&mut self, component: u32, word30: u32, word24: u32) -> Result<()> {
        self.events
            .push(json!({"event": "home_settle", "component": component,
            "word30": word30, "word24": word24}));
        Ok(())
    }
    fn mind_virtual_f4(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "mind_virtual_f4"}));
        Ok(())
    }
    fn take_arrival(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok((!self.arrivals.is_empty()).then(|| self.arrivals.remove(0)))
    }
    fn member_organization(&mut self, object: u32) -> Result<u32> {
        match self.object(object)["member"].as_str() {
            Some("org") => Ok(ORG),
            Some(_) => Ok(OTHER_ORG),
            None => Err(Error::World("null membership".into())),
        }
    }
    fn organization(&mut self, _mind: MindId, _arena: &Arena) -> Result<u32> {
        Ok(ORG)
    }
    fn member_arrived(&mut self, _mind: MindId, object: u32, _arena: &mut Arena) -> Result<()> {
        let o = self.name(object);
        self.events
            .push(json!({"event": "member_arrived", "object": o}));
        Ok(())
    }
    fn sleep_point(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        Ok(self.point)
    }
    fn set_sleep_point(&mut self, _mind: MindId, point: [u32; 2]) -> Result<()> {
        self.point = point;
        Ok(())
    }
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([u(&self.c["null_point"][0]), u(&self.c["null_point"][1])])
    }
    fn organization_move(
        &mut self,
        _mind: MindId,
        point: [u32; 2],
        _arena: &mut Arena,
    ) -> Result<()> {
        self.component()?;
        self.events
            .push(json!({"event": "organization_move", "point": point}));
        Ok(())
    }
    fn organization_busy(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["busy"].as_bool().unwrap();
        self.events
            .push(json!({"event": "organization_busy", "result": r}));
        Ok(r)
    }
    fn resume_pending(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["resumed"].as_bool().unwrap();
        self.events
            .push(json!({"event": "resume_pending", "result": r}));
        Ok(r)
    }
    fn organization_flag310(&mut self, _mind: MindId, _arena: &Arena) -> Result<bool> {
        Ok(self.c["flag310"].as_bool().unwrap())
    }
    fn embedded_state(&mut self, _mind: MindId, offset: u32) -> Result<StateId> {
        assert_eq!(offset, 0x54);
        Ok(StateId(2))
    }
    fn organization_word74(&mut self, _mind: MindId, _arena: &Arena) -> Result<u32> {
        self.component()
    }
    fn sleep_word1c(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.word1c)
    }
    fn set_sleep_word1c(&mut self, _state: StateId, value: u32) -> Result<()> {
        self.word1c = value;
        Ok(())
    }
    fn sleep_byte10(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.c["byte10"].as_bool().unwrap())
    }
    fn sleep_522f60(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "sleep_522f60"}));
        Ok(())
    }
    fn sleep_523000(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "sleep_523000"}));
        Ok(())
    }
}

#[test]
fn native_sleep_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 400);
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        for key in c["stack"].as_array().unwrap() {
            let i = NAMES
                .iter()
                .position(|n| *n == key.as_str().unwrap())
                .unwrap();
            mind.stack.push_back(Some(StateId(i as u32 + 1)));
        }
        let w: Vec<u32> = c["words"].as_array().unwrap().iter().map(u).collect();
        mind.words_24_30 = [w[0], w[1], w[2], w[3]];
        arena.minds.insert(MindId(1), mind);
        for (i, n) in NAMES.iter().enumerate() {
            arena.states.insert(
                StateId(i as u32 + 1),
                State {
                    parent: MindId(1),
                    discard_on_suspend: false,
                    status: u(&c["states"][*n]),
                },
            );
        }
        let mut host = Host {
            c: c.clone(),
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            arrivals: c["arrivals"].as_array().unwrap().iter().map(u).collect(),
            point: [u(&c["point"][0]), u(&c["point"][1])],
            word1c: u(&c["word1c"]),
        };
        let result = think(StateId(1), &mut arena, &mut host);
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        result.unwrap();
        let states: serde_json::Map<String, Value> = NAMES
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    n.to_string(),
                    json!(arena.states[&StateId(i as u32 + 1)].status),
                )
            })
            .collect();
        assert_eq!(json!(states), o["states"], "{name}");
        let stack: Vec<Value> = arena.minds[&MindId(1)]
            .stack
            .iter()
            .map(|s| json!(s.map(state_name)))
            .collect();
        assert_eq!(json!(stack), o["stack"], "{name}");
        assert_eq!(json!(host.arrivals), o["arrivals"], "{name}");
        assert_eq!(json!(host.point), o["point"], "{name}");
        assert_eq!(json!(host.word1c), o["word1c"], "{name}");
    }
    assert_eq!(faults, 35);
}
