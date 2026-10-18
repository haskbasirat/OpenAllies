use aa_mind::{Arena, Error, MindId, Result, StateId, World};
use serde_json::{Value, json};

struct Host {
    events: Vec<Value>,
    mutations: Vec<Value>,
    can: bool,
    fail: Option<&'static str>,
    actors: Value,
    change_actor: bool,
}
impl Host {
    fn event(
        &mut self,
        event: &str,
        identity: u32,
        extra: &[u32],
        arena: &mut Arena,
    ) -> Result<()> {
        let mut values = vec![json!(event), json!(identity)];
        values.extend(extra.iter().map(|n| json!(n)));
        values.push(json!(arena));
        self.events.push(Value::Array(values));
        if self.fail == Some(event) {
            return Err(Error::World(format!("fixture {event} failure")));
        }
        if self.mutations.first().is_some_and(|m| m["event"] == event) {
            let m = self.mutations.remove(0);
            let mut value = serde_json::to_value(&*arena).unwrap();
            if let Some(states) = m["states"].as_object() {
                for (id, changes) in states {
                    for (key, v) in changes.as_object().unwrap() {
                        value["states"][id][key] = v.clone();
                    }
                }
            }
            if let Some(stacks) = m["stacks"].as_object() {
                for (id, stack) in stacks {
                    value["minds"][id]["stack"] = stack.clone();
                }
            }
            *arena = serde_json::from_value(value).unwrap();
        }
        if event == "changed" && self.change_actor {
            arena.mind_mut(MindId(1))?.actor = 102;
        }
        Ok(())
    }
}
impl World for Host {
    fn can_activate(&mut self, s: StateId, a: &mut Arena) -> Result<bool> {
        self.event("can", s.0, &[], a)?;
        Ok(self.can)
    }
    fn enter(&mut self, s: StateId, a: &mut Arena) -> Result<()> {
        self.event("enter", s.0, &[], a)
    }
    fn leave(&mut self, s: StateId, a: &mut Arena) -> Result<()> {
        self.event("leave", s.0, &[], a)
    }
    fn schedule(&mut self, m: MindId, d: u32, v: u32, a: &mut Arena) -> Result<()> {
        self.event("schedule", m.0, &[d, v], a)
    }
    fn reset_default(&mut self, m: MindId, a: &mut Arena) -> Result<()> {
        self.event("default", m.0, &[], a)
    }
    fn state_changed(&mut self, m: MindId, a: &mut Arena) -> Result<()> {
        self.event("changed", m.0, &[], a)
    }
}

impl aa_mind::disable_command::World for Host {
    fn element_leader(&self, actor: u32) -> Result<Option<u32>> {
        let a = &self.actors[actor.to_string()];
        Ok(if a["element"] == true {
            a["leader"].as_u64().map(|v| v as u32)
        } else {
            None
        })
    }
    fn actor_facing(&self, actor: u32) -> Result<[u32; 2]> {
        Ok(serde_json::from_value(self.actors[actor.to_string()]["facing"].clone()).unwrap())
    }
    fn set_actor_facing(&mut self, actor: u32, facing: [u32; 2], arena: &mut Arena) -> Result<()> {
        self.event("face", actor, &facing, arena)
    }
}
fn installed() -> aa_mind::construct::Installed {
    aa_mind::construct::Installed {
        kind: aa_mind::construct::Kind::Element,
        mind_id: MindId(1),
        state_bindings: [(0x68, StateId(4))].into_iter().collect(),
        update_head: None,
        words: Default::default(),
        bytes: Default::default(),
        references: Default::default(),
    }
}
#[test]
fn original_disable_command_runs_sleep_transition_before_leader_facing() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-disable-command/native-disable-fixtures.json"
    ))
    .unwrap();
    for row in f["cases"].as_array().unwrap() {
        let c = &row["input"];
        let mut arena: Arena = serde_json::from_value(c["arena"].clone()).unwrap();
        let mut host = Host {
            events: vec![],
            mutations: serde_json::from_value(c["mutations"].clone()).unwrap(),
            can: c["can_activate"] == true,
            fail: None,
            actors: c["actors"].clone(),
            change_actor: c["change_actor"] == true,
        };
        let accepted =
            aa_mind::disable_command::apply(&installed(), &mut arena, &mut host).unwrap();
        assert_eq!(
            json!({"accepted":accepted,"arena":arena,"events":host.events}),
            row["output"],
            "{}",
            c["name"]
        );
        assert!(host.mutations.is_empty());
    }
}
#[test]
fn failing_facing_does_not_undo_committed_sleep_activation() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-disable-command/native-disable-fixtures.json"
    ))
    .unwrap();
    let c = &f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["input"]["can_activate"] == true
                && r["input"]["actors"]["101"]["element"] == true
                && r["input"]["actors"]["101"]["leader"] == 103
        })
        .unwrap()["input"];
    let mut arena: Arena = serde_json::from_value(c["arena"].clone()).unwrap();
    let mut host = Host {
        events: vec![],
        mutations: vec![],
        can: true,
        fail: Some("face"),
        actors: c["actors"].clone(),
        change_actor: false,
    };
    assert!(aa_mind::disable_command::apply(&installed(), &mut arena, &mut host).is_err());
    assert_eq!(
        arena.minds[&MindId(1)].stack.front(),
        Some(&Some(StateId(4)))
    );
    assert_eq!(arena.states[&StateId(4)].status, 1);
    assert_eq!(host.events.last().unwrap()[0], "face");
}
