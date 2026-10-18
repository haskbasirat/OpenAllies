use aa_mind::{
    Arena, Error, MindId, Result,
    ability_decision::{self, Decision, Start, World},
};
use aa_numeric::Binary;
use serde_json::{Value, json};

struct Host {
    input: Value,
    events: Vec<Value>,
    mutations: Vec<Value>,
    fail: Option<&'static str>,
}
impl Host {
    fn event(
        &mut self,
        name: &str,
        args: &[Value],
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()> {
        let mut values = vec![json!(name)];
        values.extend_from_slice(args);
        values.push(json!(decision));
        self.events.push(json!(values));
        if self.fail == Some(name) {
            return Err(Error::World(format!("fixture {name} error")));
        }
        if self.mutations.first().is_some_and(|m| m["event"] == name) {
            let change = self.mutations.remove(0);
            if let Some(fields) = change["decision"].as_object() {
                let mut value = json!(decision);
                for (key, v) in fields {
                    value[key] = v.clone();
                }
                *decision = serde_json::from_value(value).unwrap();
            }
            if let Some(actor) = change["mind_actor"].as_u64() {
                arena.mind_mut(MindId(1))?.actor = actor as u32;
            }
            if let Some(actors) = change["actors"].as_object() {
                for (key, fields) in actors {
                    for (name, value) in fields.as_object().unwrap() {
                        self.input["actors"][key][name] = value.clone();
                    }
                }
            }
        }
        Ok(())
    }
}
impl World for Host {
    fn bombard_mode(&self, _mind: MindId) -> Result<u32> {
        Ok(self.input["bombard_mode"].as_u64().unwrap() as u32)
    }
    fn performer(&self, actor: u32) -> Result<u32> {
        Ok(self.input["actors"][actor.to_string()]["performer"]
            .as_u64()
            .unwrap() as u32)
    }
    fn actor_id(&self, actor: u32) -> Result<u32> {
        Ok(self.input["actors"][actor.to_string()]["id"]
            .as_u64()
            .unwrap() as u32)
    }
    fn start(
        &mut self,
        performer: u32,
        request: Start,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<bool> {
        self.event(
            "start",
            &[json!(performer), json!(request)],
            decision,
            arena,
        )?;
        Ok(self.input["start_result"].as_bool().unwrap())
    }
    fn decision_weight(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<Binary> {
        self.event("weight", &[json!(mind.0)], decision, arena)?;
        Ok(Binary::from_f64_bits(self.input["weight_bits"].as_u64().unwrap()).unwrap())
    }
    fn maximum_other_ability_weight(
        &mut self,
        performer: u32,
        ability: u32,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<Binary> {
        self.event(
            "other",
            &[json!(performer), json!(ability)],
            decision,
            arena,
        )?;
        Ok(Binary::from_f64_bits(self.input["other_bits"].as_u64().unwrap()).unwrap())
    }
    fn reconsider(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()> {
        self.event("reconsider", &[json!(mind.0)], decision, arena)
    }
    fn cancel_update(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()> {
        self.event("cancel", &[json!(mind.0)], decision, arena)
    }
    fn failed_target(
        &mut self,
        mind: MindId,
        target: u32,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()> {
        self.event(
            "failed_target",
            &[json!(mind.0), json!(target)],
            decision,
            arena,
        )
    }
    fn failed_without_target(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()> {
        self.event("failed_none", &[json!(mind.0)], decision, arena)
    }
    fn wait(&mut self, mind: MindId, decision: &mut Decision, arena: &mut Arena) -> Result<()> {
        self.event("wait", &[json!(mind.0)], decision, arena)
    }
    fn pursue(&mut self, mind: MindId, decision: &mut Decision, arena: &mut Arena) -> Result<()> {
        self.event("pursue", &[json!(mind.0)], decision, arena)
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-decision.json"
    ))
    .unwrap()
}
#[test]
fn original_decision_dispatch_preserves_float_store_and_callback_rereads() {
    let fixture = fixture();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 201);
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut host = Host {
            input: input.clone(),
            events: vec![],
            mutations: input["mutations"].as_array().unwrap().clone(),
            fail: None,
        };
        let mut arena = serde_json::from_value(input["arena"].clone()).unwrap();
        let mut decision = serde_json::from_value(input["decision"].clone()).unwrap();
        let point = serde_json::from_value(input["point"].clone()).unwrap();
        ability_decision::apply(MindId(1), &mut decision, point, &mut arena, &mut host).unwrap();
        assert_eq!(
            json!({"decision":decision,"events":host.events}),
            case["output"],
            "{}",
            input["name"]
        );
        assert!(host.mutations.is_empty());
    }
}
#[test]
fn failed_scoring_preserves_performer_activation_and_does_not_cancel() {
    let fixture = fixture();
    let input = &fixture["cases"][1]["input"];
    let mut host = Host {
        input: input.clone(),
        events: vec![],
        mutations: vec![],
        fail: Some("weight"),
    };
    let mut arena = serde_json::from_value(input["arena"].clone()).unwrap();
    let mut decision = serde_json::from_value(input["decision"].clone()).unwrap();
    assert!(
        ability_decision::apply(MindId(1), &mut decision, None, &mut arena, &mut host).is_err()
    );
    assert_eq!(decision.status, 1);
    assert_eq!(host.events.len(), 2);
    assert_eq!(host.events[1][0], "weight");
}
