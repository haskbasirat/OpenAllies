//! 518850 against original x86 executions (research/aircraft-mind-dispatch).
use aa_mind::aai_guard_command::{GuardCommandWorld, command};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/aircraft-mind-dispatch/native-aircraft-guard-command.json"
    ))
    .unwrap()
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Object {
    Home,
    Target,
}

struct Mind<'a> {
    doc: &'a Value,
    input: &'a Value,
    calls: Vec<Value>,
}
impl Mind<'_> {
    fn yes(&self, name: &str) -> bool {
        self.input[name].as_bool().unwrap()
    }
    fn leader(&self, name: &str) -> Option<&'static str> {
        match self.input[name].as_str() {
            Some("a") => Some("a"),
            Some("b") => Some("b"),
            _ => None,
        }
    }
}
impl GuardCommandWorld for Mind<'_> {
    type Error = &'static str;
    type Object = Object;
    type Leader = &'static str;
    fn has_performer(&mut self) -> Result<bool, &'static str> {
        Ok(self.yes("performer"))
    }
    fn command_flag(&mut self) -> Result<bool, &'static str> {
        Ok(self.yes("flag"))
    }
    fn command_gtid(&mut self) -> Result<u32, &'static str> {
        let g = word(&self.doc["command_gtid"]);
        Ok(if self.yes("command_live") { g } else { g ^ 0x1_0000 })
    }
    fn home_gtid(&mut self) -> Result<u32, &'static str> {
        let g = word(&self.doc["home_gtid"]);
        Ok(if self.yes("home_live") { g } else { g ^ 0x1_0000 })
    }
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, &'static str> {
        Ok(gtid == word(&self.doc["command_gtid"]) || gtid == word(&self.doc["home_gtid"]))
    }
    fn home_object(&mut self) -> Result<Option<Object>, &'static str> {
        self.calls.push(json!(["home"]));
        Ok(self.yes("home_object").then_some(Object::Home))
    }
    fn command_object(&mut self) -> Result<Option<Object>, &'static str> {
        self.calls.push(json!(["command_object"]));
        Ok(self.yes("command_object").then_some(Object::Target))
    }
    fn object_gtid(&mut self, object: Object) -> Result<u32, &'static str> {
        let matches = object == Object::Target || self.yes("home_matches");
        Ok(word(&self.doc[if matches { "object_gtid" } else { "other_gtid" }]))
    }
    fn escort_current(&mut self) -> Result<bool, &'static str> {
        Ok(self.input["current"] == "9c")
    }
    fn guard_current(&mut self) -> Result<bool, &'static str> {
        Ok(self.input["current"] == "d0")
    }
    fn escorted_gtid(&mut self) -> Result<u32, &'static str> {
        let matches = self.yes("escorted_matches");
        Ok(word(&self.doc[if matches { "object_gtid" } else { "other_gtid" }]))
    }
    fn guarded_leader(&mut self) -> Result<Option<&'static str>, &'static str> {
        Ok(self.leader("guarded"))
    }
    fn activate_escort(&mut self, mode: u32, object: Option<Object>, formation: u8) -> Result<u8, &'static str> {
        self.calls.push(json!(["escort", mode, object == Some(Object::Target), formation]));
        Ok(word(&self.input["al"]) as u8)
    }
    fn has_denizen(&mut self, object: Object) -> Result<bool, &'static str> {
        assert_eq!(object, Object::Home);
        Ok(self.yes("denizen"))
    }
    fn denizen_leader(&mut self, object: Object, gtid: u32) -> Result<Option<&'static str>, &'static str> {
        assert_eq!(object, Object::Home);
        self.calls.push(json!(["leader", gtid]));
        Ok(self.leader("leader"))
    }
    fn null_object(&mut self) -> &'static str {
        "null object"
    }
    fn activate_guard(&mut self, mode: u32, leader: Option<&'static str>) -> Result<u8, &'static str> {
        self.calls.push(json!(["guard", mode, leader]));
        Ok(word(&self.input["al"]) as u8)
    }
}

#[test]
fn guard_and_escort_command_paths_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 4200);
    for row in rows {
        let mut mind = Mind { doc: &doc, input: &row["input"], calls: vec![] };
        let al = command(&mut mind).unwrap();
        assert_eq!(u64::from(al), row["al"].as_u64().unwrap(), "{row}");
        assert_eq!(json!(mind.calls), row["calls"], "{row}");
    }
}
