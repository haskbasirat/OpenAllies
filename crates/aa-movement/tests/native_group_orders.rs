//! 0x4DE7F0 / 0x4DEA20 / 0x4DEE40 against
//! research/phase8-orders-input/group/native-group-orders.json: the same
//! world calls with the same command words in the same order, the member
//! formation flags afterwards and the return bytes. Allocation and reference
//! release events of the native run are not part of the port and are
//! filtered out.
use aa_movement::group_orders::{Command, GroupOrderWorld, give, rest, take_places};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::HashMap;

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

fn words(c: &Command) -> Value {
    json!([
        c.descriptor,
        c.flags,
        c.target,
        c.word14,
        c.position[0],
        c.position[1],
        c.aux[0],
        c.aux[1],
        c.word28
    ])
}

const KINGDOM: u32 = 0x6600;

struct Adapter<'a> {
    case: &'a Value,
    answers: HashMap<String, Vec<Value>>,
    events: Vec<Value>,
    flags: Vec<u8>,
    idle: bool,
}

impl<'a> Adapter<'a> {
    fn new(case: &'a Value) -> Self {
        let answers = case["answers"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_array().unwrap().clone()))
            .collect();
        let flags = case["flags_before"]
            .as_array()
            .map(|a| a.iter().map(|v| v.as_u64().unwrap() as u8).collect())
            .unwrap_or_default();
        Self { case, answers, events: Vec::new(), flags, idle: false }
    }
    fn answer(&mut self, key: String) -> Value {
        self.answers.get_mut(&key).unwrap().remove(0)
    }
}

impl GroupOrderWorld for Adapter<'_> {
    type Member = i32;
    type Error = String;
    fn kingdom(&self) -> u32 {
        KINGDOM
    }
    fn member_count(&self) -> i32 {
        self.case["members"].as_array().unwrap().len() as i32
    }
    fn member(&mut self, index: i32) -> Result<Option<i32>, String> {
        let kind = self.case["members"][index as usize].as_str().unwrap();
        Ok((kind == "live").then_some(index))
    }
    fn accepts(&mut self, member: i32, kingdom: u32, command: &Command) -> Result<bool, String> {
        self.events.push(json!({"accepts": member, "kingdom": kingdom, "third": 1, "command": words(command)}));
        Ok(self.answer(format!("accepts:{member}")).as_u64().unwrap() & 0xff != 0)
    }
    fn formation_flag(&mut self, member: i32, command: &Command) -> Result<u8, String> {
        self.events.push(json!({"flag": member, "command": words(command)}));
        Ok(self.answer(format!("flag:{member}")).as_u64().unwrap() as u8)
    }
    fn set_formation_flag(&mut self, index: i32, flag: u8) -> Result<(), String> {
        self.flags[index as usize] = flag;
        Ok(())
    }
    fn deliver(&mut self, command: &Command, member: i32) -> Result<u8, String> {
        self.events.push(json!({"deliver": member, "command": words(command)}));
        Ok(self.answer(format!("deliver:{member}")).as_u64().unwrap() as u8)
    }
    fn command_mask(&self, descriptor: u32) -> Result<u32, String> {
        assert_eq!(descriptor, word(&self.case["descriptor"]));
        Ok(word(&self.case["mask"]))
    }
    fn command_descriptor(&self, kind: u32) -> Result<u32, String> {
        assert_eq!(kind, 5);
        Ok(word(&self.case["descriptor5"]))
    }
    fn target_handle(&mut self, handle: u32) -> Result<Option<u32>, String> {
        assert_eq!(handle, word(&self.case["command"][2]));
        Ok(match self.case["target_kind"].as_str().unwrap() {
            "live" => Some(word(&self.case["target_object_handle"])),
            _ => None,
        })
    }
    fn place(&mut self, index: i32, point: [u32; 2]) -> Result<[u32; 2], String> {
        self.events.push(json!({"place": index, "point": point}));
        let offset = self.answer(format!("place:{index}"));
        Ok([word(&offset[0]), word(&offset[1])])
    }
    fn set_idle(&mut self) -> Result<(), String> {
        self.idle = true;
        Ok(())
    }
    fn set_position(&mut self, at: [u32; 2]) -> Result<(), String> {
        self.events.push(json!({"set_position": at}));
        Ok(())
    }
    fn queued(&self) -> u32 {
        word(&self.case["queued"])
    }
    fn has_zone7(&self) -> bool {
        word(&self.case["zone_slots"]) > 7 && self.case["zone7"].as_bool().unwrap()
    }
    fn remove_zone7(&mut self) -> Result<(), String> {
        self.events.push(json!({"remove_zone": true}));
        Ok(())
    }
    fn schedule(&mut self, delay_bits: u32, word: u32) -> Result<(), String> {
        self.events.push(json!({"schedule": [delay_bits, word], "scheduler": true}));
        Ok(())
    }
    fn numeric(error: aa_numeric::Error) -> String {
        error.to_string()
    }
}

fn native_events(case: &Value) -> Vec<Value> {
    case["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e.get("new").is_none() && e.get("release").is_none())
        .cloned()
        .collect()
}

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-orders-input/group/native-group-orders.json"
    ))
    .unwrap()
}

#[test]
fn give_matches_native() {
    let doc = fixture();
    assert_eq!(doc["passed"], true);
    let cases = doc["give"].as_array().unwrap();
    assert_eq!(cases.len(), 400);
    for (index, case) in cases.iter().enumerate() {
        let c = &case["command"];
        let command = Command {
            descriptor: word(&c[0]),
            flags: word(&c[1]),
            target: word(&c[2]),
            word14: word(&c[3]),
            position: [word(&c[4]), word(&c[5])],
            aux: [word(&c[6]), word(&c[7])],
            word28: word(&c[8]),
        };
        let mut world = Adapter::new(case);
        let result = give(&mut world, case["index"].as_i64().unwrap() as i32, &command).unwrap();
        assert_eq!(world.events, native_events(case), "give case {index}");
        assert_eq!(json!(world.flags), case["entry_flags"], "give case {index} flags");
        assert_eq!(u32::from(result), word(&case["result"]), "give case {index} result");
    }
}

#[test]
fn take_places_matches_native() {
    let doc = fixture();
    let cases = doc["take_places"].as_array().unwrap();
    assert_eq!(cases.len(), 200);
    for (index, case) in cases.iter().enumerate() {
        let mut world = Adapter::new(case);
        let at = [word(&case["at"][0]), word(&case["at"][1])];
        take_places(&mut world, at, Precision::Double53).unwrap();
        assert_eq!(world.events, native_events(case), "take_places case {index}");
    }
}

#[test]
fn rest_matches_native() {
    let doc = fixture();
    let cases = doc["rest"].as_array().unwrap();
    assert_eq!(cases.len(), 200);
    for (index, case) in cases.iter().enumerate() {
        let mut world = Adapter::new(case);
        let at = [word(&case["at"][0]), word(&case["at"][1])];
        let reform = word(&case["reform"]) & 0xff != 0;
        let result = rest(&mut world, at, reform, Precision::Double53).unwrap();
        assert_eq!(world.events, native_events(case), "rest case {index}");
        assert_eq!(json!(world.flags), case["entry_flags"], "rest case {index} flags");
        assert!(world.idle);
        assert_eq!(word(&case["state"]), 0);
        assert_eq!(u32::from(result), word(&case["result"]), "rest case {index} result");
    }
}
