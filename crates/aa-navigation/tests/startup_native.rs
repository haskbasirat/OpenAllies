use aa_navigation::startup::{self, Object, State, World};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf};
struct Host {
    data: Value,
    actions: serde_json::Map<String, Value>,
    events: Vec<Value>,
    removed: BTreeSet<usize>,
    fail: Option<&'static str>,
}
impl Host {
    fn new(data: Value) -> Self {
        let actions = data["mutations"].as_object().unwrap().clone();
        Self {
            data,
            actions,
            events: vec![],
            removed: BTreeSet::new(),
            fail: None,
        }
    }
    fn event(&mut self, name: &str, args: Vec<u64>, state: &mut State) -> startup::Result<()> {
        self.events.push(
            json!({"name":name,"args":args,"ready":state.ready,"revision":state.world_revision}),
        );
        let key = args
            .first()
            .map_or_else(|| name.to_owned(), |a| format!("{name}:{a}"));
        if let Some(changes) = self.actions.remove(&key) {
            for change in changes.as_array().unwrap() {
                let value = change["value"].clone();
                match change["kind"].as_str().unwrap() {
                    "kingdoms" => self.data["kingdoms"] = value,
                    "ai_slots" => self.data["ai_slots"] = value,
                    "ready" => state.ready = value.as_bool().unwrap(),
                    "revision" => self.data["global_revision"] = value,
                    "actor_flags" => {
                        self.data["actors"][change["index"].as_u64().unwrap() as usize]["flags"] =
                            value
                    }
                    "context_words" => {
                        self.data["seen"][change["index"].as_u64().unwrap() as usize]["words"] =
                            value
                    }
                    "remove_actor" => {
                        self.removed.insert(value.as_u64().unwrap() as usize);
                    }
                    _ => panic!(),
                }
            }
        }
        if self.fail == Some(name) {
            return Err(startup::Error("injected service failure".into()));
        }
        Ok(())
    }
}
impl World for Host {
    fn clear_list78(&mut self, s: &mut State) -> startup::Result<()> {
        self.event("clear78", vec![], s)
    }
    fn clear_list80(&mut self, s: &mut State) -> startup::Result<()> {
        self.event("clear80", vec![], s)
    }
    fn global_revision(&self) -> u32 {
        self.data["global_revision"].as_u64().unwrap() as u32
    }
    fn prepare_topology(&mut self, s: &mut State) -> startup::Result<()> {
        self.event("prepare", vec![], s)
    }
    fn kingdom_count(&self) -> usize {
        self.data["kingdoms"].as_array().unwrap().len()
    }
    fn kingdom_at(&self, i: usize) -> Option<Object> {
        self.data["kingdoms"][i].as_u64().map(|v| Object(1000 + v))
    }
    fn register_kingdom(&mut self, k: Object, s: &mut State) -> startup::Result<()> {
        self.event("kingdom", vec![k.0 - 1000], s)
    }
    fn ai_slot_count(&self) -> usize {
        self.data["ai_slots"].as_array().unwrap().len()
    }
    fn ai_slot_at(&self, i: usize) -> Option<Object> {
        self.data["ai_slots"][i].as_u64().map(|v| Object(2000 + v))
    }
    fn start_ai(&mut self, a: Object, s: &mut State) -> startup::Result<()> {
        self.event("ai", vec![a.0 - 2000], s)
    }
    fn registry_count(&self) -> i32 {
        (self.data["actors"].as_array().unwrap().len() - self.removed.len()) as i32
    }
    fn next_registered_after(&self, cursor: u32) -> startup::Result<Option<Object>> {
        let first = if cursor == 0 {
            0
        } else {
            (cursor & 0xffff) as usize
        };
        Ok((first..self.data["actors"].as_array().unwrap().len())
            .find(|i| !self.removed.contains(i))
            .map(|i| Object(100 + i as u64)))
    }
    fn actor_id(&self, a: Object) -> u32 {
        0x10000 + (a.0 - 100) as u32 + 1
    }
    fn actor_b4(&mut self, a: Object, s: &mut State) -> startup::Result<bool> {
        self.event("b4", vec![a.0 - 100], s)?;
        Ok(self.data["actors"][(a.0 - 100) as usize]["b4"]
            .as_bool()
            .unwrap())
    }
    fn actor_flags8(&self, a: Object) -> u32 {
        self.data["actors"][(a.0 - 100) as usize]["flags"]
            .as_u64()
            .unwrap() as u32
    }
    fn register_special_actor(&mut self, a: Object, s: &mut State) -> startup::Result<()> {
        self.event("special", vec![a.0 - 100], s)
    }
    fn actor_b0(&mut self, a: Object, s: &mut State) -> startup::Result<bool> {
        self.event("b0", vec![a.0 - 100], s)?;
        Ok(self.data["actors"][(a.0 - 100) as usize]["b0"]
            .as_bool()
            .unwrap())
    }
    fn seen_record(
        &mut self,
        a: Object,
        k: Object,
        s: &mut State,
    ) -> startup::Result<Option<Object>> {
        self.event("seen", vec![a.0 - 100, k.0 - 1000], s)?;
        Ok(self.data["seen"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| {
                r["actor"].as_u64() == Some(a.0 - 100) && r["kingdom"].as_u64() == Some(k.0 - 1000)
            })
            .map(|i| Object(3000 + i as u64)))
    }
    fn seen_context(&self, r: Object) -> Object {
        Object(r.0 + 1000)
    }
    fn context_eligible(&mut self, c: Object, s: &mut State) -> startup::Result<bool> {
        self.event("eligible", vec![c.0 - 4000], s)?;
        Ok(self.data["seen"][(c.0 - 4000) as usize]["eligible"]
            .as_bool()
            .unwrap())
    }
    fn context_words(&self, c: Object) -> [u32; 2] {
        serde_json::from_value(self.data["seen"][(c.0 - 4000) as usize]["words"].clone()).unwrap()
    }
    fn seen_position(&self, r: Object) -> [u32; 2] {
        serde_json::from_value(self.data["seen"][(r.0 - 3000) as usize]["position_bits"].clone())
            .unwrap()
    }
    fn region_at(&mut self, p: [u32; 2], s: &mut State) -> startup::Result<Option<u32>> {
        self.event("region", p.map(u64::from).to_vec(), s)?;
        Ok(self.data["seen"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["position_bits"] == json!(p))
            .unwrap()["region"]
            .as_u64()
            .map(|v| v as u32))
    }
    fn region_kingdom(
        &mut self,
        r: u32,
        k: Object,
        s: &mut State,
    ) -> startup::Result<Option<Object>> {
        self.event("binding", vec![u64::from(r), k.0 - 1000], s)?;
        Ok(self.data["seen"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| {
                v["region"].as_u64() == Some(u64::from(r))
                    && v["kingdom"].as_u64() == Some(k.0 - 1000)
            })
            .unwrap()["binding"]
            .as_u64()
            .map(|v| Object(5000 + v)))
    }
    fn register_seen(&mut self, b: Object, r: Object, s: &mut State) -> startup::Result<()> {
        self.event("register_seen", vec![b.0 - 5000, r.0 - 3000], s)
    }
}
fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../research/phase7-readiness/native-startup.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn native_startup_order_and_field_rereads_match() {
    let doc = fixture();
    let rows = doc["fixtures"].as_array().unwrap();
    assert_eq!(rows.len(), 17);
    for row in rows {
        let mut state = serde_json::from_value(row["input"]["state"].clone()).unwrap();
        let mut world = Host::new(row["input"].clone());
        startup::initialize(&mut state, &mut world).unwrap();
        assert_eq!(
            json!({"state":state,"events":world.events}),
            row["native"],
            "{}",
            row["input"]["name"]
        );
    }
}
#[test]
fn failed_startup_cannot_be_inferred_successful_from_native_ready_byte() {
    let doc = fixture();
    let row = doc["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "actors_and_seen")
        .unwrap();
    for (failure, ready) in [
        ("prepare", false),
        ("special", true),
        ("register_seen", true),
    ] {
        let mut state: State = serde_json::from_value(row["input"]["state"].clone()).unwrap();
        let mut world = Host::new(row["input"].clone());
        world.fail = Some(failure);
        assert!(startup::initialize(&mut state, &mut world).is_err());
        assert_eq!(state.ready, ready);
        assert_eq!(state.word6c, 123);
        assert_eq!(state.word70, 456);
        assert_eq!(world.events.last().unwrap()["name"], failure);
    }
}
