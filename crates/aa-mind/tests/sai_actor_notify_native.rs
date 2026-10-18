//! Replays research/phase8-ai/native-actor-notify.json (original 7A0C20
//! under Unicorn) against aa_mind::sai::actor_notify.
use aa_mind::{
    Result,
    sai::actor_notify::{ActorNotifyWorld, notify},
};
use serde_json::{Value, json};

struct Host {
    c: Value,
    ready: bool,
    events: Vec<Value>,
}

impl Host {
    fn flag(&self, key: &str) -> bool {
        self.c[key].as_bool().unwrap()
    }
}

impl ActorNotifyWorld for Host {
    fn sai_ready(&mut self) -> Result<bool> {
        Ok(self.ready)
    }
    fn actor_c8(&mut self, _actor: u32) -> Result<bool> {
        Ok(self.flag("c8"))
    }
    fn actor_category(&mut self, _actor: u32) -> Result<u32> {
        Ok(self.c["category"].as_u64().unwrap() as u32)
    }
    fn actor_removed(&mut self, _actor: u32) -> Result<bool> {
        Ok(self.c["flags"].as_u64().unwrap() & 1 != 0)
    }
    fn predicate_79e300(&mut self, _actor: u32) -> Result<bool> {
        let r = self.flag("p79e300");
        self.events.push(json!({"op": "79e300", "result": r}));
        Ok(r)
    }
    fn actor_4ceff0(&mut self, _actor: u32) -> Result<bool> {
        let r = self.flag("p4ceff0");
        self.events.push(json!({"op": "4ceff0", "result": r}));
        if self.flag("clear_ready") {
            self.ready = false;
        }
        Ok(r)
    }
    fn actor_virtual_20(&mut self, _actor: u32) -> Result<bool> {
        let r = self.flag("v20");
        self.events.push(json!({"op": "virtual_20", "result": r}));
        Ok(r)
    }
    fn predicate_79e080(&mut self, _actor: u32) -> Result<bool> {
        let r = self.flag("p79e080");
        self.events.push(json!({"op": "79e080", "result": r}));
        Ok(r)
    }
    fn unregister(&mut self, _actor: u32) -> Result<()> {
        self.events.push(json!({"op": "unregister"}));
        Ok(())
    }
    fn register(&mut self, _actor: u32, category: u32) -> Result<()> {
        self.events
            .push(json!({"op": "register", "category": category}));
        Ok(())
    }
    fn register_unit(&mut self, _actor: u32, flag: bool) -> Result<()> {
        self.events
            .push(json!({"op": "register_unit", "flag": u8::from(flag)}));
        Ok(())
    }
}

#[test]
fn native_actor_notify_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-actor-notify.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 680);
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let mut host = Host {
            c: c.clone(),
            ready: c["ready"].as_bool().unwrap(),
            events: vec![],
        };
        notify(&mut host, 1).unwrap();
        assert_eq!(json!(host.events), row["output"]["events"], "case {i}");
    }
}
