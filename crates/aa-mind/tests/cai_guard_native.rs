//! Replays research/phase8-ai/native-guard-think.json (original 5294F0
//! KKC_CAIStateGuard::Think with 529C20 and 52A810 under Unicorn) against
//! aa_mind::cai_guard::think.
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_guard::{GuardThinkWorld, think},
};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-guard-think.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
const STATES: [(&str, u32); 5] = [
    ("sleep", 0x34),
    ("move", 0x68),
    ("follow", 0xc0),
    ("attack", 0xd4),
    ("guard", 0x17c),
];
fn state_name(s: StateId) -> &'static str {
    STATES[s.0 as usize - 1].0
}

/// Object handles are index+1 in key order; component handles 100+index;
/// object Mind handles 200+index.
struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    word10: u32,
    byte14: bool,
}
impl Host {
    fn object(&self, h: u32) -> &Value {
        &self.c["objects"][&self.keys[(h % 100) as usize - 1]]
    }
    fn key(&self, h: u32) -> String {
        self.keys[(h % 100) as usize - 1].clone()
    }
    fn handle(&self, key: &str) -> u32 {
        self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
    }
    fn name(&self, h: u32) -> Value {
        if h == 0 { json!(0) } else { json!(self.key(h)) }
    }
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
impl GuardThinkWorld for Host {
    fn guarded(&mut self, _state: StateId) -> Result<u32> {
        Ok(u(&self.c["guarded"]))
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
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        Ok(u(&self.object(object)["gtid"]))
    }
    fn mind_virtual_140(&mut self, _mind: MindId, gtid: u32, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "mind_virtual_140", "gtid": gtid}));
        Ok(())
    }
    fn object_virtual_158(&mut self, object: u32) -> Result<bool> {
        let r = self.object(object)["v158"].as_bool().unwrap();
        let k = self.key(object);
        self.events
            .push(json!({"event": "object_virtual_158", "object": k, "result": r}));
        Ok(r)
    }
    fn object_virtual_15c(&mut self, object: u32) -> Result<Option<u32>> {
        let t = self.object(object)["v15c"].clone();
        let k = self.key(object);
        self.events
            .push(json!({"event": "object_virtual_15c", "object": k, "result": t}));
        Ok(t.as_str().map(|k| self.handle(k)))
    }
    fn denizen_component(&mut self, object: u32) -> Result<Option<u32>> {
        Ok((!self.object(object)["component"].is_null()).then_some(100 + object))
    }
    fn component_byte20(&mut self, component: u32) -> Result<bool> {
        Ok(self.object(component)["component"]["byte20"]
            .as_bool()
            .unwrap())
    }
    fn object_word98(&mut self, object: u32) -> Result<Option<u32>> {
        let v = u(&self.object(object)["word98"]);
        Ok((v != 0).then_some(v))
    }
    fn check_585160(&mut self, value: u32) -> Result<bool> {
        let r = self.c["check_585160"].as_bool().unwrap();
        self.events
            .push(json!({"event": "check_585160", "value": value, "result": r}));
        Ok(r)
    }
    fn component_threats(&mut self, component: u32) -> Result<Vec<u32>> {
        let k = self.key(component);
        self.events
            .push(json!({"event": "component_threats", "object": k}));
        Ok(self.object(component)["component"]["threats"]
            .as_array()
            .unwrap()
            .iter()
            .map(u)
            .collect())
    }
    fn object_threats(&mut self, object: u32) -> Result<Vec<u32>> {
        Ok(self.object(object)["threats"]
            .as_array()
            .unwrap()
            .iter()
            .map(u)
            .collect())
    }
    fn object_mind(&mut self, object: u32) -> Result<Option<u32>> {
        Ok((!self.object(object)["mind"].is_null()).then_some(200 + object))
    }
    fn object_mind_virtual_90(&mut self, object_mind: u32) -> Result<Option<u32>> {
        let t = self.object(object_mind)["mind"].clone();
        let k = self.key(object_mind);
        self.events
            .push(json!({"event": "object_mind_virtual_90", "object": k, "result": t}));
        Ok(t.as_str().filter(|s| !s.is_empty()).map(|k| self.handle(k)))
    }
    fn state_target(&mut self, _mind: MindId, offset: u32) -> Result<u32> {
        let name = STATES.iter().find(|(_, o)| *o == offset).unwrap().0;
        Ok(u(&self.c["word10"][name]))
    }
    fn embedded_state(&mut self, _mind: MindId, offset: u32) -> Result<StateId> {
        Ok(StateId(
            STATES.iter().position(|(_, o)| *o == offset).unwrap() as u32 + 1,
        ))
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let p = &self.object(object)["position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn organization_position(&mut self, _mind: MindId) -> Result<[u32; 2]> {
        let p = &self.c["org_position"];
        Ok([u(&p[0]), u(&p[1])])
    }
    fn guard_old_target_bias(&mut self) -> Result<u32> {
        Ok(u(&self.c["bias"]))
    }
    fn embedded_think(&mut self, _mind: MindId, offset: u32, _arena: &mut Arena) -> Result<()> {
        let name = STATES.iter().find(|(_, o)| *o == offset).unwrap().0;
        self.events
            .push(json!({"event": "embedded_think", "state": name}));
        Ok(())
    }
    fn relation(&mut self, _mind: MindId, object: u32) -> Result<i32> {
        let r = self.c["relation"].as_i64().unwrap() as i32;
        let k = self.name(object);
        self.events
            .push(json!({"event": "relation", "object": k, "result": r}));
        Ok(r)
    }
    fn guard_527ce0(
        &mut self,
        _state: StateId,
        mode: u32,
        object: u32,
        _arena: &mut Arena,
    ) -> Result<()> {
        let k = self.name(object);
        self.events
            .push(json!({"event": "guard_527ce0", "mode": mode, "object": k}));
        Ok(())
    }
    fn follow_activate(
        &mut self,
        _mind: MindId,
        mode: u32,
        object: u32,
        _arena: &mut Arena,
    ) -> Result<bool> {
        let r = self.c["follow_ok"].as_bool().unwrap();
        let k = self.name(object);
        self.events
            .push(json!({"event": "follow_activate", "mode": mode, "object": k, "result": r}));
        Ok(r)
    }
    fn mind_virtual_b8(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "mind_virtual_b8"}));
        Ok(())
    }
    fn set_guard_word10(&mut self, _state: StateId, value: u32) -> Result<()> {
        self.word10 = value;
        Ok(())
    }
    fn guard_byte14(&mut self, _state: StateId) -> Result<bool> {
        Ok(self.byte14)
    }
    fn set_guard_byte14(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.byte14 = value;
        Ok(())
    }
    fn home_settle(&mut self, component: Option<u32>, word30: u32, word24: u32) -> Result<()> {
        let component = component.ok_or_else(|| Error::World("null home component".into()))?;
        let k = self.key(component);
        self.events
            .push(json!({"event": "home_settle", "object": k, "word30": word30, "word24": word24}));
        Ok(())
    }
    fn guard_51ee20(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["check_51ee20"].as_bool().unwrap();
        self.events
            .push(json!({"event": "guard_51ee20", "result": r}));
        Ok(r)
    }
}

#[test]
fn native_guard_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 500);
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        for key in c["stack"].as_array().unwrap() {
            let i = STATES
                .iter()
                .position(|(n, _)| *n == key.as_str().unwrap())
                .unwrap();
            mind.stack.push_back(Some(StateId(i as u32 + 1)));
        }
        let w: Vec<u32> = c["words"].as_array().unwrap().iter().map(u).collect();
        mind.words_24_30 = [w[0], w[1], w[2], w[3]];
        arena.minds.insert(MindId(1), mind);
        for (i, (n, _)) in STATES.iter().enumerate() {
            arena.states.insert(
                StateId(i as u32 + 1),
                State {
                    parent: MindId(1),
                    discard_on_suspend: false,
                    status: u(&c["status"][*n]),
                },
            );
        }
        let mut host = Host {
            c: c.clone(),
            keys: c["objects"].as_object().unwrap().keys().cloned().collect(),
            events: vec![],
            word10: u(&c["word10"]["guard"]),
            byte14: c["byte14"].as_bool().unwrap(),
        };
        let result = think(StateId(5), &mut arena, &mut host, p);
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        result.unwrap();
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
        assert_eq!(json!(status), o["status"], "{name}");
        assert_eq!(json!(host.word10), o["guard_word10"], "{name}");
        assert_eq!(json!(u8::from(host.byte14)), o["guard_byte14"], "{name}");
    }
    assert_eq!(faults, 10);
}
