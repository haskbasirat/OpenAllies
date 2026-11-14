//! Replays research/phase8-combat/native-organization-helpers.json (original
//! 578850, 578430, 578D50 and 577FA0 under Unicorn) against
//! aa_actors::organization_helpers: the 4CEF70 calls in order, the +D4 word,
//! the +34 recruit slots and the 577FA0 byte.
use aa_actors::organization_helpers::{
    Message, MemberNotifyWorld, Result, TargetObject, add_recruit_item, notify_members, ready,
    set_target_id,
};
use serde_json::{Value, json};

const OWNER: u32 = 0x0a0b_0c0d;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-organization-helpers.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

struct Host<'a> {
    c: &'a Value,
    events: Vec<Value>,
}
impl MemberNotifyWorld for Host<'_> {
    fn member_count(&mut self) -> Result<i32> {
        Ok(self.c["members"].as_array().unwrap().len() as i32)
    }
    fn member(&mut self, index: i32) -> Result<Option<u32>> {
        Ok((!self.c["members"][index as usize].is_null()).then_some(index as u32))
    }
    fn member_word_78(&mut self, member: u32) -> Result<u32> {
        Ok(u(&self.c["members"][member as usize]["w78"]))
    }
    fn member_word_a4(&mut self, member: u32) -> Result<u32> {
        Ok(u(&self.c["members"][member as usize]["wa4"]))
    }
    fn owner(&mut self) -> Result<u32> {
        Ok(OWNER)
    }
    fn notify(&mut self, member: u32, word: u32, owner: u32) -> Result<()> {
        self.events.push(json!(["notify", member, word, u32::from(owner == OWNER)]));
        Ok(())
    }
}

#[test]
fn organization_helpers_match_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 380);
    let mut notified = 0;
    for case in cases {
        let c = &case["input"];
        let want = &case["output"];
        let name = c["name"].as_str().unwrap();
        match c["entry"].as_str().unwrap() {
            "notify" => {
                let message = Message {
                    kind: u(&c["kind"]),
                    word: u(&c["word"]),
                    handled: c["handled"].as_bool().unwrap(),
                };
                let mut host = Host { c, events: Vec::new() };
                notify_members(&message, &mut host).unwrap();
                notified += host.events.len();
                assert_eq!(Value::Array(host.events), want["events"], "{name}");
            }
            "target" => {
                let mut target = u(&c["before"]);
                let object = (!c["object"].is_null()).then(|| TargetObject {
                    gtid: u(&c["object"]["gtid"]),
                    word_c0: u(&c["object"]["c0"]),
                });
                set_target_id(&mut target, object);
                assert_eq!(target, u(&want["target"]), "{name}");
            }
            "recruit" => {
                let mut recruits: Vec<u32> = c["recruits"].as_array().unwrap().iter().map(u).collect();
                add_recruit_item(&mut recruits, 1, u(&c["slot"]) as usize).unwrap();
                assert_eq!(json!(recruits), want["recruits"], "{name}");
            }
            _ => {
                let got = ready(u(&c["w40"]), u(&c["w44"]), u(&c["inner18"]) as i32);
                assert_eq!(u32::from(got), u(&want["ready"]), "{name}");
            }
        }
    }
    assert!(notified > 100, "{notified}");
}
