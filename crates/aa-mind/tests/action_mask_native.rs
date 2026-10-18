use aa_mind::{
    Arena, Error, MindId, Result, StateId,
    action_mask::{self, Ability, Actor, RegimentStates, World},
};
use serde_json::{Value, json};
struct Host {
    input: Value,
    events: Vec<Value>,
    mutations: Vec<Value>,
    predicate_count: usize,
    fail: Option<&'static str>,
}
impl Host {
    fn event(&mut self, name: &str, args: &[u32], arena: &mut Arena) -> Result<()> {
        let mut event = vec![json!(name)];
        event.extend(args.iter().map(|v| json!(v)));
        self.events.push(json!(event));
        if self.fail == Some(name) {
            return Err(Error::World(format!("fixture {name} failed")));
        }
        if self.mutations.first().is_some_and(|m| m["event"] == name) {
            let m = self.mutations.remove(0);
            if let Some(owner) = m["owner"].as_u64() {
                arena.mind_mut(MindId(1))?.actor = owner as u32;
            }
            if let Some(stack) = m.get("stack") {
                arena.mind_mut(MindId(1))?.stack = serde_json::from_value(stack.clone()).unwrap();
            }
            if let Some(actors) = m["actors"].as_object() {
                for (k, fields) in actors {
                    for (name, v) in fields.as_object().unwrap() {
                        self.input["actors"][k][name] = v.clone();
                    }
                }
            }
        }
        Ok(())
    }
}
impl World for Host {
    fn actor(&self, id: u32) -> Result<Actor> {
        Ok(serde_json::from_value(self.input["actors"][id.to_string()].clone()).unwrap())
    }
    fn action_bits(&self, id: u32) -> Result<u64> {
        Ok(self.input["actors"][id.to_string()]["action_bits"]
            .as_u64()
            .unwrap())
    }
    fn set_action_bits(&mut self, id: u32, bits: u64) -> Result<()> {
        self.input["actors"][id.to_string()]["action_bits"] = json!(bits);
        Ok(())
    }
    fn element_organization(&self, id: u32) -> Result<Option<u32>> {
        Ok(self.input["elements"][id.to_string()]
            .as_u64()
            .map(|v| v as u32))
    }
    fn abilities(&self, id: u32) -> Result<Vec<Ability>> {
        Ok(serde_json::from_value(self.input["abilities"][id.to_string()].clone()).unwrap())
    }
    fn resolve(&self, id: u32) -> Result<Option<u32>> {
        Ok(self.input["registry"][id.to_string()]
            .as_u64()
            .map(|v| v as u32))
    }
    fn organization_link_d0(&self, id: u32) -> Result<u32> {
        Ok(self.input["organizations"][id.to_string()]["link"]
            .as_u64()
            .unwrap() as u32)
    }
    fn organization_status_578140(&self, id: u32) -> Result<i32> {
        Ok(self.input["organizations"][id.to_string()]["status"]
            .as_i64()
            .unwrap() as i32)
    }
    fn organization_values(&self, id: u32) -> Result<[u32; 2]> {
        Ok(
            serde_json::from_value(self.input["organizations"][id.to_string()]["values"].clone())
                .unwrap(),
        )
    }
    fn setting_cache108(&self) -> Result<bool> {
        Ok(self.input["setting108"].as_bool().unwrap())
    }
    fn setting_cached8(&self) -> Result<u32> {
        Ok(self.input["settingd8"].as_u64().unwrap() as u32)
    }
    fn actor_player(&mut self, id: u32, arena: &mut Arena) -> Result<Option<u32>> {
        self.event("player", &[id], arena)?;
        Ok(self.input["actor_players"][id.to_string()]
            .as_u64()
            .map(|v| v as u32))
    }
    fn player_flags(&self, id: u32) -> Result<u32> {
        Ok(self.input["players"][id.to_string()].as_u64().unwrap() as u32)
    }
    fn actor_state(&mut self, id: u32, index: u32, arena: &mut Arena) -> Result<bool> {
        self.event("state", &[id, index], arena)?;
        Ok(self.input["state_results"][index.to_string()]
            .as_bool()
            .unwrap_or(false))
    }
    fn actor_predicate_4d12c0(&mut self, id: u32, arena: &mut Arena) -> Result<bool> {
        self.event("predicate12", &[id], arena)?;
        let values = self.input["predicate12"].as_array().unwrap();
        let v = values[self.predicate_count.min(values.len() - 1)]
            .as_bool()
            .unwrap();
        self.predicate_count += 1;
        Ok(v)
    }
    fn component_ac_predicate(&mut self, id: u32, entry: u32, arena: &mut Arena) -> Result<bool> {
        self.event("component", &[id, entry], arena)?;
        Ok(self.input["ac"][entry.to_string()].as_bool().unwrap())
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-mind/native-action-mask.json"
    ))
    .unwrap()
}
#[test]
fn original_mind_command_masks_and_real_performer_capability_scans() {
    let fixture = fixture();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 219);
    for case in fixture["cases"].as_array().unwrap() {
        let input = &case["input"];
        let mut host = Host {
            input: input.clone(),
            events: vec![],
            mutations: input["mutations"].as_array().unwrap().clone(),
            predicate_count: 0,
            fail: None,
        };
        let mut arena = serde_json::from_value(input["arena"].clone()).unwrap();
        let mut predicate = None;
        match input["kind"].as_str().unwrap() {
            "element" => action_mask::element(MindId(1), &mut arena, &mut host).unwrap(),
            "aircraft" => action_mask::aircraft(MindId(1), &arena, &mut host).unwrap(),
            "regiment" => action_mask::regiment(
                MindId(1),
                RegimentStates {
                    rout: StateId(4),
                    engage: StateId(5),
                },
                &mut arena,
                &mut host,
                serde_json::from_value(input["precision"].clone()).unwrap(),
            )
            .unwrap(),
            "exhausted" => {
                predicate =
                    Some(action_mask::exhausted_in_stack(MindId(1), StateId(6), &arena).unwrap())
            }
            "constant_false" => predicate = Some(false),
            _ => unreachable!(),
        }
        let actions: serde_json::Map<String, Value> = host.input["actors"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v["action_bits"].clone()))
            .collect();
        assert_eq!(
            json!({"actions":actions,"events":host.events,"predicate":predicate}),
            case["output"],
            "{}",
            input["name"]
        );
        assert!(host.mutations.is_empty());
    }
}
#[test]
fn contributor_errors_retain_earlier_action_bits() {
    let fixture = fixture();
    let input = &fixture["cases"][0]["input"];
    let mut host = Host {
        input: input.clone(),
        events: vec![],
        mutations: vec![],
        predicate_count: 0,
        fail: Some("player"),
    };
    let mut arena = serde_json::from_value(input["arena"].clone()).unwrap();
    host.set_action_bits(101, 0).unwrap();
    assert!(action_mask::element(MindId(1), &mut arena, &mut host).is_err());
    assert_ne!(host.action_bits(101).unwrap() & (1 << 5), 0);
    assert_eq!(host.action_bits(101).unwrap() & (1 << 11), 0);
    assert_eq!(host.events.last().unwrap()[0], "player");
}
