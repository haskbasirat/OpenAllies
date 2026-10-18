//! Replays research/phase8-ai/native-rout-enter.json (original 51F190
//! KKC_CAIStateRout::OnEnter with 51F040 and 52C0D0 under Unicorn).
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId,
    cai_engage::{EngageWorld, TargetNode, Targets},
    cai_rout::{RoutEnterWorld, enter},
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-rout-enter.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

/// Actor handles: 0 = organization, i+1 = key "t{i}".
struct Host {
    c: Value,
    keys: Vec<String>,
    events: Vec<Value>,
    refs: std::collections::BTreeMap<u32, u32>,
    entered: Option<u32>,
}
impl Host {
    fn name(&self, actor: u32) -> String {
        if actor == 0 {
            "org".into()
        } else {
            self.keys[actor as usize - 1].clone()
        }
    }
    fn handle(&self, key: &str) -> u32 {
        if key == "org" {
            0
        } else {
            self.keys.iter().position(|k| k == key).unwrap() as u32 + 1
        }
    }
    fn actor(&self, actor: u32) -> &Value {
        &self.c["actors"][self.name(actor)]
    }
    fn table(&self, table: &str, key: &str) -> Option<Value> {
        self.c[table].get(key).cloned()
    }
}
impl EngageWorld for Host {
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        Ok(self
            .keys
            .iter()
            .position(|k| {
                let a = &self.c["actors"][k];
                u(&a["gtid"]) == gtid && a["registered"].as_bool() != Some(false)
            })
            .map(|i| i as u32 + 1))
    }
    fn gtid(&mut self, actor: u32) -> Result<u32> {
        Ok(u(&self.actor(actor)["gtid"]))
    }
    fn removed(&mut self, actor: u32) -> Result<bool> {
        Ok(self.actor(actor)["flags"].as_u64().unwrap_or(0) & 1 != 0)
    }
    fn attached_organization(&mut self, actor: u32) -> Result<Option<u32>> {
        let m = &self.actor(actor)["membership"];
        if m.is_null() || m["org"].is_null() || m["attached"] == 0 {
            return Ok(None);
        }
        Ok(Some(self.handle(m["org"].as_str().unwrap())))
    }
    fn can_target(&mut self, target: u32) -> Result<bool> {
        let t = self.name(target);
        let r = self
            .table("can_target", &t)
            .is_none_or(|v| v.as_bool().unwrap());
        self.events
            .push(json!({"event": "can_target", "target": t, "result": r}));
        Ok(r)
    }
    fn relation(&mut self, target: u32) -> Result<i32> {
        let t = self.name(target);
        let r = self
            .table("relation", &t)
            .map_or(0, |v| v.as_i64().unwrap() as i32);
        self.events
            .push(json!({"event": "relation", "target": t, "result": r}));
        Ok(r)
    }
    fn register_attacker(&mut self, target: u32) -> Result<()> {
        let t = self.name(target);
        self.events
            .push(json!({"event": "register_attacker", "target": t}));
        Ok(())
    }
    fn unregister_attacker(&mut self, target: u32) -> Result<()> {
        let t = self.name(target);
        self.events
            .push(json!({"event": "unregister_attacker", "target": t}));
        Ok(())
    }
    fn has_component(&mut self) -> Result<bool> {
        Ok(self.c["component"].as_bool().unwrap())
    }
    fn component_target_added(&mut self, target: u32) -> Result<()> {
        let t = self.name(target);
        self.events
            .push(json!({"event": "component_target_added", "target": t}));
        Ok(())
    }
    fn component_update(&mut self) -> Result<()> {
        if !self.c["component"].as_bool().unwrap() {
            return Err(Error::World("579BA0 on a null component".into()));
        }
        self.events.push(json!({"event": "component_update"}));
        Ok(())
    }
    fn priority(&mut self, target: u32) -> Result<i32> {
        let t = self.name(target);
        let r = self
            .table("priority", &t)
            .map_or(0, |v| v.as_i64().unwrap() as i32);
        self.events
            .push(json!({"event": "priority", "target": t, "result": r}));
        Ok(r)
    }
    fn engage(&mut self, target: u32) -> Result<bool> {
        let t = self.name(target);
        let r = self
            .table("engage", &t)
            .is_none_or(|v| v.as_bool().unwrap());
        self.events
            .push(json!({"event": "engage", "target": t, "result": r}));
        Ok(r)
    }
    fn chasing_target(&mut self) -> Result<u32> {
        Ok(u(&self.c["chasing_target"]))
    }
    fn organization_entry4(&mut self) -> Result<Option<u32>> {
        let e = self.c["org_entries"].as_array().unwrap();
        Ok((e.len() > 4).then(|| u(&e[4])).filter(|v| *v != 0))
    }
    fn target_check130(&mut self, target: u32, entry: u32) -> Result<bool> {
        let t = self.name(target);
        let r = self
            .table("check130", &t)
            .is_some_and(|v| v.as_bool().unwrap());
        self.events
            .push(json!({"event": "check130", "target": t, "entry": entry, "result": r}));
        Ok(r)
    }
    fn target_word90(&mut self, target: u32) -> Result<bool> {
        Ok(self.actor(target)["word90"].as_u64().unwrap_or(0) != 0)
    }
    fn related(&mut self, a: u32, b: u32) -> Result<bool> {
        let (a, b) = (self.name(a), self.name(b));
        let r = self
            .table("related", &format!("{a}>{b}"))
            .is_some_and(|v| v.as_bool().unwrap());
        self.events
            .push(json!({"event": "related", "a": a, "b": b, "result": r}));
        Ok(r)
    }
    fn organization(&mut self) -> Result<u32> {
        Ok(0)
    }
    fn component_accepts(&mut self, target: u32) -> Result<bool> {
        let t = self.name(target);
        let r = self
            .table("accepts", &t)
            .is_none_or(|v| v.as_bool().unwrap());
        self.events
            .push(json!({"event": "component_accepts", "target": t, "result": r}));
        Ok(r)
    }
}

impl RoutEnterWorld for Host {
    fn world_time(&mut self) -> Result<u32> {
        Ok(u(&self.c["time"]))
    }
    fn set_entered_time(&mut self, _state: StateId, time: u32) -> Result<()> {
        self.entered = Some(time);
        Ok(())
    }
    fn organization_halt(&mut self, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "organization_halt"}));
        Ok(())
    }
    fn release_command(&mut self, command: u32) -> Result<()> {
        let r = self.refs.get_mut(&command).unwrap();
        *r -= 1;
        if *r == 0 {
            self.events
                .push(json!({"event": "command_destroyed", "command": command}));
        }
        Ok(())
    }
}

#[test]
fn native_rout_enter_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 80);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let keys: Vec<String> = c["actors"].as_object().unwrap().keys().cloned().collect();
        let commands = c["commands"].as_array().unwrap();
        let mut host = Host {
            c: c.clone(),
            keys,
            events: vec![],
            refs: commands
                .iter()
                .filter(|x| !x.is_null())
                .map(|x| (u(&x["id"]), u(&x["refs"])))
                .collect(),
            entered: None,
        };
        let rout = StateId(7);
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        mind.pending_commands = commands
            .iter()
            .map(|x| if x.is_null() { 0 } else { u(&x["id"]) })
            .collect();
        arena.minds.insert(MindId(1), mind);
        arena.states.insert(
            rout,
            State {
                parent: MindId(1),
                discard_on_suspend: false,
                status: u(&c["status"]),
            },
        );
        let mut targets = Targets {
            nodes: c["list"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| TargetNode {
                    gtid: u(&n["gtid"]),
                    priority: n["priority"].as_i64().unwrap() as i32,
                })
                .collect(),
        };
        let result = enter(rout, &mut targets, &mut arena, &mut host);
        assert_eq!(json!(host.events), o["events"], "{name}");
        result.unwrap();
        assert_eq!(json!(targets.nodes), o["list"], "{name}");
        assert_eq!(json!(arena.states[&rout].status), o["status"], "{name}");
        assert_eq!(json!(host.entered), o["entered"], "{name}");
        assert!(arena.minds[&MindId(1)].pending_commands.is_empty());
        assert_eq!(o["pending_head"], 0);
        let refs: serde_json::Map<String, Value> = host
            .refs
            .iter()
            .map(|(k, v)| (k.to_string(), json!(v)))
            .collect();
        assert_eq!(json!(refs), o["command_refs"], "{name}");
    }
}
