//! Replays research/phase8-ai/native-engage-think.json (original 526A70
//! KKC_CAIStateEngage::Think with 524C10 and 4364C0 under Unicorn).
use aa_mind::{
    Arena, Error, Mind, MindId, Result, State, StateId, World,
    cai_engage::{EngageWorld, TargetNode, Targets},
    cai_engage_think::{EngageThinkWorld, think},
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-engage-think.json"
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
    last_attacker: u32,
    byte1c: bool,
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

/// Member handles: 1000 + slot index; member Mind handles: 2000 + index.
impl Host {
    fn member(&self, handle: u32) -> &Value {
        let i = (handle % 1000) as usize;
        &self.c["members"][i]
    }
    fn member_key(handle: u32) -> String {
        format!("m{}", handle % 1000)
    }
    fn handle_of(&self, key: &Value) -> Option<u32> {
        key.as_str().map(|k| self.handle(k))
    }
}
impl World for Host {
    fn can_activate(&mut self, _state: StateId, _arena: &mut Arena) -> Result<bool> {
        unreachable!()
    }
    fn enter(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
    fn leave(&mut self, _state: StateId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
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
        unreachable!()
    }
    fn state_changed(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        unreachable!()
    }
}
impl EngageThinkWorld for Host {
    fn organization_status(&mut self, _mind: MindId, _arena: &Arena) -> Result<u32> {
        let r = u(&self.c["status"]);
        self.events
            .push(json!({"event": "organization_status", "result": r}));
        Ok(r)
    }
    fn organization_word118(&mut self, _mind: MindId, _arena: &Arena) -> Result<bool> {
        Ok(u(&self.c["word118"]) != 0)
    }
    fn wavering_reaction(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.events.push(json!({"event": "wavering_reaction"}));
        Ok(())
    }
    fn mind_virtual_140(&mut self, _mind: MindId, gtid: u32, _arena: &mut Arena) -> Result<()> {
        self.events
            .push(json!({"event": "mind_virtual_140", "gtid": gtid}));
        Ok(())
    }
    fn mind_virtual_4c(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<Option<u32>> {
        let a = self.c["attacker"].clone();
        self.events
            .push(json!({"event": "mind_virtual_4c", "result": a}));
        Ok(self.handle_of(&a))
    }
    fn last_attacker(&mut self, _state: StateId) -> Result<u32> {
        Ok(self.last_attacker)
    }
    fn set_last_attacker(&mut self, _state: StateId, gtid: u32) -> Result<()> {
        self.last_attacker = gtid;
        Ok(())
    }
    fn kill_target_is_highest_priority(&mut self) -> Result<bool> {
        Ok(self.c["svar"].as_bool().unwrap())
    }
    fn within_engage_range(&mut self, _mind: MindId, actor: u32, _arena: &Arena) -> Result<bool> {
        let r = self.c["in_range"].as_bool().unwrap();
        let t = self.name(actor);
        self.events
            .push(json!({"event": "within_engage_range", "target": t, "result": r}));
        Ok(r)
    }
    fn member_count(&mut self, _mind: MindId, _arena: &Arena) -> Result<i32> {
        Ok(self.c["members"].as_array().unwrap().len() as i32)
    }
    fn member_slot(&mut self, _mind: MindId, index: i32, _arena: &Arena) -> Result<Option<u32>> {
        Ok((!self.c["members"][index as usize].is_null()).then_some(1000 + index as u32))
    }
    fn member_mind(&mut self, member: u32) -> Result<Option<u32>> {
        Ok(self.member(member)["mind"]
            .as_bool()
            .unwrap()
            .then_some(member + 1000))
    }
    fn member_mind_virtual_17c(&mut self, member_mind: u32) -> Result<()> {
        self.events.push(json!({"event": "member_mind_virtual_17c",
            "member": Self::member_key(member_mind)}));
        Ok(())
    }
    fn chase_continue(&mut self, _mind: MindId, gtid: u32, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["chase"].as_bool().unwrap();
        self.events
            .push(json!({"event": "chase_continue", "gtid": gtid, "result": r}));
        Ok(r)
    }
    fn mind_virtual_f8(&mut self, _mind: MindId, actor: u32, _arena: &mut Arena) -> Result<bool> {
        let r = self.c["f8"].as_bool().unwrap();
        let a = if actor == 0 {
            json!(0)
        } else {
            json!(self.name(actor))
        };
        self.events
            .push(json!({"event": "mind_virtual_f8", "actor": a, "result": r}));
        Ok(r)
    }
    fn member_mind_virtual_6c(&mut self, member_mind: u32) -> Result<bool> {
        let r = self.member(member_mind)["busy"].as_bool().unwrap();
        self.events.push(json!({"event": "member_mind_virtual_6c",
            "member": Self::member_key(member_mind), "result": r}));
        Ok(r)
    }
    fn member_flags100(&mut self, member: u32) -> Result<u32> {
        Ok(u(&self.member(member)["flags100"]))
    }
    fn member_mind_virtual_f0(&mut self, member_mind: u32, target: u32) -> Result<()> {
        let t = self.name(target);
        self.events.push(json!({"event": "member_mind_virtual_f0",
            "member": Self::member_key(member_mind), "target": t}));
        Ok(())
    }
    fn member_e4_status(&mut self, member: u32) -> Result<Option<u32>> {
        let e = self.member(member)["e4"].as_u64().map(|v| v as u32);
        if let Some(r) = e {
            self.events.push(json!({"event": "member_e4_status",
                "member": Self::member_key(member), "result": r}));
        }
        Ok(e)
    }
    fn member_mind_virtual_4c(&mut self, member_mind: u32) -> Result<Option<u32>> {
        let t = self.member(member_mind)["target"].clone();
        self.events.push(json!({"event": "member_mind_virtual_4c",
            "member": Self::member_key(member_mind), "result": t}));
        Ok(self.handle_of(&t))
    }
    fn organization_follows(
        &mut self,
        _mind: MindId,
        actor: u32,
        _arena: &mut Arena,
    ) -> Result<bool> {
        let t = self.name(actor);
        let r = self.c["follows"]
            .get(&t)
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.events
            .push(json!({"event": "organization_follows", "target": t, "result": r}));
        Ok(r)
    }
    fn set_all_engaged(&mut self, _state: StateId, value: bool) -> Result<()> {
        self.byte1c = value;
        Ok(())
    }
}

#[test]
fn native_engage_think_matches_original() {
    let f = fixture();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 150);
    let mut faults = 0;
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let keys: Vec<String> = c["actors"].as_object().unwrap().keys().cloned().collect();
        let mut host = Host {
            c: c.clone(),
            keys,
            events: vec![],
            last_attacker: u(&c["last_attacker"]),
            byte1c: u(&c["byte1c"]) != 0,
        };
        let (engage, chasing, other) = (StateId(1), StateId(2), StateId(3));
        let mut arena = Arena::default();
        let mut mind = Mind::base(1);
        match c["current"].as_str() {
            Some("engage") => mind.stack.push_front(Some(engage)),
            Some("chasing") => mind.stack.push_front(Some(chasing)),
            Some("other") => mind.stack.push_front(Some(other)),
            _ => {}
        }
        arena.minds.insert(MindId(1), mind);
        for s in [engage, chasing, other] {
            arena.states.insert(
                s,
                State {
                    parent: MindId(1),
                    discard_on_suspend: false,
                    status: if s == engage {
                        u(&c["engage_status"])
                    } else {
                        0
                    },
                },
            );
        }
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
        let result = think(engage, chasing, &mut targets, &mut arena, &mut host);
        assert_eq!(json!(host.events), o["events"], "{name}");
        if !o["fault"].is_null() {
            assert!(result.is_err(), "{name}");
            faults += 1;
            continue;
        }
        result.unwrap();
        assert_eq!(json!(targets.nodes), o["list"], "{name}");
        assert_eq!(json!(arena.states[&engage].status), o["status"], "{name}");
        assert_eq!(json!(host.last_attacker), o["last_attacker"], "{name}");
        assert_eq!(json!(u8::from(host.byte1c)), o["byte1c"], "{name}");
    }
    assert_eq!(faults, 3);
}
