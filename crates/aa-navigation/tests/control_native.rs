use aa_navigation::{
    control::{self, Command, Error, Node, Result, Shared, World},
    startup::Object,
};
use serde_json::{Value, json};
struct Host {
    data: Value,
    events: Vec<Value>,
    fail: Option<&'static str>,
}
impl Host {
    fn actor(&self, id: Object) -> &Value {
        &self.data["actors"][id.0.to_string()]
    }
    fn event(
        &mut self,
        name: &str,
        args: Vec<u64>,
        mut command: Option<&mut Command>,
    ) -> Result<()> {
        self.events.push(json!({"name":name,"args":args}));
        let key = format!("{name}:{}", args.first().copied().unwrap_or(0));
        if let Some(actions) = self.data["mutations"].as_object_mut().unwrap().remove(&key) {
            for m in actions.as_array().unwrap() {
                let key = m["key"].as_u64().unwrap_or(0).to_string();
                let value = m["value"].clone();
                match m["kind"].as_str().unwrap() {
                    "next" => self.data["nodes"][key]["next"] = value,
                    "sai" => self.data["actors"][key]["sai"] = value,
                    "references" => self.data["shared"][key]["references"] = value,
                    "component" => self.data["actors"][key]["component"] = value,
                    "shared34" => command.as_mut().unwrap().shared34 = value.as_u64().map(Shared),
                    "definition" => {
                        command.as_mut().unwrap().definition = Some(Object(2));
                        self.data["second_slot"] = value;
                    }
                    name => panic!("unknown mutation{name}"),
                }
            }
        }
        if self.fail == Some(name) {
            return Err(Error(format!("injected {name}")));
        }
        Ok(())
    }
    fn snapshot(&self, returned: Option<bool>) -> Value {
        let actors = self.data["actors"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, a)| (key.clone(), json!({"flags":a["flags"],"sai":a["sai"]})))
            .collect::<serde_json::Map<_, _>>();
        let refs = self.data["shared"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, s)| (key.clone(), s["references"].clone()))
            .collect::<serde_json::Map<_, _>>();
        json!({"returned":returned,"kingdom_flags":self.data["kingdom_flags"],"actors":actors,"references":refs,"events":self.events})
    }
}
const KINGDOM: Object = Object(999);
impl World for Host {
    fn flags8(&self, id: Object) -> Result<u32> {
        Ok(if id == KINGDOM {
            self.data["kingdom_flags"].as_u64().unwrap()
        } else {
            self.actor(id)["flags"].as_u64().unwrap()
        } as u32)
    }
    fn set_flags8(&mut self, id: Object, value: u32) -> Result<()> {
        if id == KINGDOM {
            self.data["kingdom_flags"] = json!(value)
        } else {
            self.data["actors"][id.0.to_string()]["flags"] = json!(value)
        };
        Ok(())
    }
    fn actor_sai_handle(&self, id: Object) -> Result<Option<Shared>> {
        Ok(self.actor(id)["sai"].as_u64().map(Shared))
    }
    fn references(&self, id: Shared) -> Result<u32> {
        Ok(self.data["shared"][id.0.to_string()]["references"]
            .as_u64()
            .unwrap() as u32)
    }
    fn set_references(&mut self, id: Shared, value: u32) -> Result<()> {
        self.data["shared"][id.0.to_string()]["references"] = json!(value);
        Ok(())
    }
    fn destroy_shared(&mut self, id: Shared) -> Result<()> {
        self.event("destroy", vec![id.0, 1], None)
    }
    fn sai_target(&self, id: Shared) -> Result<Option<Object>> {
        Ok(self.data["shared"][id.0.to_string()]["target"]
            .as_u64()
            .map(Object))
    }
    fn notify_disabled(&mut self, target: Object, handle: Shared, args: [u32; 4]) -> Result<()> {
        let mut values = vec![target.0, handle.0];
        values.extend(args.map(u64::from));
        values.push(u64::from(self.references(handle)?));
        self.event("notify", values, None)
    }
    fn kingdom_head284(&self, _: Object) -> Result<Option<Node>> {
        Ok(self.data["head"].as_u64().map(Node))
    }
    fn node_actor(&self, id: Node) -> Result<Option<Object>> {
        Ok(self.data["nodes"][id.0.to_string()]["actor"]
            .as_u64()
            .map(Object))
    }
    fn node_next(&self, id: Node) -> Result<Option<Node>> {
        Ok(self.data["nodes"][id.0.to_string()]["next"]
            .as_u64()
            .map(Node))
    }
    fn command_definition(&self, _: u32) -> Result<Option<Object>> {
        Ok(self.data["definition"]
            .as_bool()
            .unwrap()
            .then_some(Object(1)))
    }
    fn definition_component18(&self, id: Object) -> Result<i32> {
        Ok(self.data[if id.0 == 1 { "slot" } else { "second_slot" }]
            .as_i64()
            .unwrap() as i32)
    }
    fn actor_component(&self, id: Object, _: usize) -> Result<Option<Object>> {
        Ok(self.actor(id)["component"].as_u64().map(Object))
    }
    fn can_receive(
        &mut self,
        id: Object,
        definition: Object,
        command: &mut Command,
    ) -> Result<bool> {
        self.event("admit", vec![id.0, definition.0], Some(command))?;
        Ok(self.actor(id)["admit"].as_bool().unwrap())
    }
    fn process_component(&mut self, id: Object, command: &mut Command) -> Result<bool> {
        self.event("process", vec![id.0], Some(command))?;
        Ok(self.data["process_result"].as_bool().unwrap())
    }
    fn actor_virtual158(&mut self, id: Object, command: &mut Command) -> Result<bool> {
        self.event("virtual158", vec![id.0], Some(command))?;
        Ok(self.actor(id)["virtual158"].as_bool().unwrap())
    }
    fn actor_virtual15c(&mut self, id: Object, command: &mut Command) -> Result<Option<Object>> {
        self.event("virtual15c", vec![id.0], Some(command))?;
        Ok(self.actor(id)["virtual15c"].as_u64().map(Object))
    }
    fn organization_parent(&self, id: Object) -> Result<Option<Object>> {
        Ok(self.actor(id)["organization_parent"].as_u64().map(Object))
    }
}
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase7-control/native-control.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn execute(host: &mut Host) -> Result<Option<bool>> {
    let target = if host.data["target"].is_string() {
        Some(KINGDOM)
    } else {
        host.data["target"].as_u64().map(Object)
    };
    match host.data["operation"].as_str().unwrap() {
        "enable" => control::enable(target, host).map(Some),
        "disable_actor" => control::disable_actor(target, host).map(Some),
        "disable_kingdom" => control::disable_kingdom(target, host).map(Some),
        "disable_actor_state" => control::disable_actor_state(target.unwrap(), host).map(|_| None),
        "process" => {
            let mut command = Command::new(host.command_definition(6)?);
            control::process(&mut command, target, host).map(Some)
        }
        _ => unreachable!(),
    }
}
#[test]
fn original_control_wrappers_and_mutable_actor_routing_match() {
    let doc = fixture();
    assert_eq!(doc["fixtures"].as_array().unwrap().len(), 31);
    for row in doc["fixtures"].as_array().unwrap() {
        let mut host = Host {
            data: row["input"].clone(),
            events: Vec::new(),
            fail: None,
        };
        let returned = execute(&mut host).unwrap();
        let mut expected = row["native"].clone();
        // Constructor payload is separately verified below. Diagnostics and pure
        // constructor observations are not live World services in the Rust API.
        expected["events"].as_array_mut().unwrap().retain(|e| {
            !matches!(
                e["name"].as_str(),
                Some("construct" | "command" | "null_diagnostic")
            )
        });
        assert_eq!(
            host.snapshot(returned),
            expected,
            "{}",
            row["input"]["name"]
        );
        for event in row["native"]["events"].as_array().unwrap() {
            if event["name"] == "command" {
                let definition = row["input"]["definition"]
                    .as_bool()
                    .unwrap()
                    .then_some(Object(1));
                assert_eq!(
                    serde_json::to_value(Command::new(definition)).unwrap(),
                    event["value"]
                );
            }
        }
    }
}
#[test]
fn control_failures_release_temporaries_and_stop_later_submission() {
    let doc = fixture();
    let rows = doc["fixtures"].as_array().unwrap();
    let row = rows
        .iter()
        .find(|r| r["input"]["name"] == "shared_1_1")
        .unwrap();
    let mut host = Host {
        data: row["input"].clone(),
        events: Vec::new(),
        fail: Some("notify"),
    };
    assert!(
        execute(&mut host)
            .unwrap_err()
            .to_string()
            .contains("injected notify")
    );
    assert_eq!(host.references(Shared(1)).unwrap(), 1);
    assert_eq!(host.flags8(Object(1)).unwrap() & 0x20, 0x20);
    assert_eq!(
        host.events
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["notify"]
    );
    let row = rows
        .iter()
        .find(|r| r["input"]["name"] == "shared_cleanup")
        .unwrap();
    let mut host = Host {
        data: row["input"].clone(),
        events: Vec::new(),
        fail: Some("process"),
    };
    host.data["operation"] = json!("disable_kingdom");
    host.data["target"] = json!("kingdom");
    assert!(
        execute(&mut host)
            .unwrap_err()
            .to_string()
            .contains("injected process")
    );
    assert_eq!(host.references(Shared(2)).unwrap(), 0);
    assert_eq!(
        host.events
            .iter()
            .map(|e| e["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["admit", "process", "destroy"]
    );
}
