use super::*;
use serde_json::{Value, json};

struct Fixture {
    input: Value,
    events: Vec<Value>,
    dispatch: u32,
    fail: Option<&'static str>,
    cycle: bool,
}
impl Fixture {
    fn new(input: &Value) -> Self {
        Self {
            input: input.clone(),
            events: vec![],
            dispatch: 1,
            fail: None,
            cycle: false,
        }
    }
    fn event(&mut self, value: Value) -> Result<()> {
        let name = value["op"].as_str().unwrap().to_owned();
        self.events.push(value);
        let mutation = self.input["mutations"][&name].clone();
        if mutation["dispatch2"].as_bool() == Some(true) {
            self.dispatch = 2;
        }
        if let Some(v) = mutation["target_flags"].as_u64() {
            self.input["actors"]["2"]["flags"] = json!(v);
        }
        if let Some(v) = mutation["linked168"].as_u64() {
            self.input["actors"]["1"]["linked_168"] = json!(v);
        }
        if mutation["remove_other_mind"].as_bool() == Some(true) {
            self.input["minds"]["4"] = Value::Null;
        }
        if self.fail == Some(name.as_str()) {
            return Err(Error::World(format!("failed {name}")));
        }
        Ok(())
    }
    fn boolean(&self, key: &str) -> bool {
        self.input["callbacks"][key].as_bool().unwrap()
    }
}
impl World for Fixture {
    type Actor = u32;
    type Mind = u32;
    type AcceptanceDispatch = u32;
    type Owner = u32;
    fn actor(&self, actor: u32) -> Result<Actor> {
        Ok(serde_json::from_value(self.input["actors"][actor.to_string()].clone()).unwrap())
    }
    fn definition(&self, id: u32) -> Result<Definition> {
        Ok(serde_json::from_value(self.input["definitions"][id.to_string()].clone()).unwrap())
    }
    fn resolve(&self, reference: u32) -> Result<Option<u32>> {
        Ok(self.input["actors"]
            .as_object()
            .unwrap()
            .iter()
            .find_map(|(key, actor)| {
                let key: u32 = key.parse().unwrap();
                (actor["reference"].as_u64() == Some(u64::from(reference))
                    && !self.input["retired"]
                        .as_array()
                        .unwrap()
                        .contains(&json!(key)))
                .then_some(key)
            }))
    }
    fn exclusion_head(&self, _: u32) -> Result<Option<u32>> {
        Ok((!self.input["ignored"].as_array().unwrap().is_empty()).then_some(0))
    }
    fn exclusion_node(&self, id: u32) -> Result<ExclusionNode> {
        let values = self.input["ignored"].as_array().unwrap();
        Ok(ExclusionNode {
            reference: values[id as usize].as_u64().unwrap() as u32,
            next: if self.cycle {
                Some(id)
            } else {
                ((id as usize + 1) < values.len()).then_some(id + 1)
            },
        })
    }
    fn actor_category(&mut self, actor: u32) -> Result<bool> {
        self.event(json!({"op":"category","actor":actor}))?;
        Ok(self.boolean("category"))
    }
    fn acceptance_dispatch(&self, _: u32) -> Result<u32> {
        Ok(1)
    }
    fn owner(&mut self, actor: u32) -> Result<u32> {
        self.event(json!({"op":"owner","actor":actor}))?;
        Ok(self.input["callbacks"]["owner"].as_u64().unwrap() as u32)
    }
    fn accepts_owner(&mut self, dispatch: u32, actor: u32, owner: u32) -> Result<bool> {
        assert_eq!(dispatch, 1, "the retained native vtable identity");
        let dispatch = self.dispatch;
        self.event(json!({"op":"accepts","actor":actor,"dispatch":dispatch,"owner":owner}))?;
        Ok(self.boolean(if dispatch == 1 { "accepts" } else { "accepts2" }))
    }
    fn organization(&self, actor: u32) -> Result<Option<u32>> {
        Ok(self.input["organizations"][actor.to_string()]
            .as_u64()
            .map(|v| v as u32))
    }
    fn mind(&self, actor: u32) -> Result<Option<u32>> {
        Ok(self.input["minds"][actor.to_string()]
            .as_u64()
            .map(|v| v as u32))
    }
    fn mind_ready_64(&mut self, mind: u32) -> Result<bool> {
        self.event(json!({"op":"mind_ready","mind":mind}))?;
        Ok(self.input["callbacks"]["ready"][mind.to_string()]
            .as_bool()
            .unwrap())
    }
    fn wall_allows_source(&mut self, actor: u32, source: u32) -> Result<bool> {
        self.event(json!({"op":"wall_allows","actor":actor,"source":source}))?;
        Ok(self.boolean("wall_allows"))
    }
    fn parent(&mut self, actor: u32) -> Result<Option<u32>> {
        self.event(json!({"op":"parent","actor":actor}))?;
        Ok(self.input["callbacks"]["parent"].as_u64().map(|v| v as u32))
    }
    fn parent_collision_required(&mut self, source: u32, parent: Option<u32>) -> Result<bool> {
        self.event(json!({"op":"parent_collision","source":source,"parent":parent}))?;
        Ok(self.boolean("parent_collision"))
    }
}
fn corpus() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase7-source-exclusion/native-exclusion.json"
    ))
    .unwrap()
}
#[test]
fn original_exclusion_results_and_ordered_mutating_callbacks() {
    let data = corpus();
    assert_eq!(data["cases"].as_array().unwrap().len(), 336);
    for case in data["cases"].as_array().unwrap() {
        let mut world = Fixture::new(&case["input"]);
        let result = excludes(
            1,
            case["input"]["candidate"].as_u64().unwrap() as u32,
            &mut world,
        )
        .unwrap();
        assert_eq!(
            json!({"result":result,"events":world.events}),
            case["output"],
            "{}",
            case["input"]["name"]
        );
    }
}
#[test]
fn invalid_lists_and_failed_callbacks_are_explicit_and_preserve_effects() {
    let data = corpus();
    let mut world = Fixture::new(&data["cases"][0]["input"]);
    world.input["ignored"] = json!([0x10003]);
    world.cycle = true;
    assert!(
        excludes(1, 2, &mut world)
            .unwrap_err()
            .to_string()
            .contains("cyclic")
    );
    assert!(world.events.is_empty());
    world.input["ignored"] = json!([]);
    world.input["mutations"]["category"] = json!({"linked168":0x10002});
    world.fail = Some("category");
    assert!(
        excludes(1, 2, &mut world)
            .unwrap_err()
            .to_string()
            .contains("failed category")
    );
    assert_eq!(world.actor(1).unwrap().linked_168, 0x10002);
    assert_eq!(world.events, vec![json!({"op":"category","actor":2})]);
}

#[test]
fn original_ordinary_owner_acceptance_short_circuit() {
    let data = corpus();
    let cases = data["ordinary_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 75);
    for case in cases {
        let i = &case["input"];
        let mut events = vec![];
        let result =
            ordinary_accepts_owner(i["candidate_owner"].as_u64(), i["owner"].as_u64(), || {
                events.push(json!({"op":"relation","owner":i["owner"]}));
                Ok(i["relation"].as_u64().unwrap() as u32)
            })
            .unwrap();
        assert_eq!(
            json!({"result":result,"events":events,"owner":i["candidate_owner"],"category":true}),
            case["output"]
        );
    }
    assert!(
        ordinary_accepts_owner(Some(1), Some(2), || Err(Error::World(
            "relation missing".into()
        )))
        .is_err()
    );
    assert!(
        ordinary_accepts_owner(Some(1), Some(1), || panic!(
            "native does not dispatch on equality"
        ))
        .unwrap()
    );
}
