//! Replays research/phase8-combat/native-actor-flags-fixtures.json: original
//! 4D07D0 (with _allshl), 4D0960, 4CFFD0, 4CE880, 4CEE40, 4CEFA0, 4D1240,
//! 4D1290, 45E660 and 471A40 under Unicorn. Returned words, watched memory
//! and virtual calls must match.
use aa_actors::actor_flags::*;
use serde_json::{Value, json};
use std::collections::HashMap;

struct Host<'a> {
    case: &'a Value,
    memory: HashMap<u32, u32>,
    events: Vec<Value>,
}

impl World for Host<'_> {
    fn word(&mut self, thing: u32, offset: u32) -> Result<u32> {
        Ok(*self.memory.get(&thing.wrapping_add(offset)).unwrap_or(&0))
    }
    fn set_word(&mut self, thing: u32, offset: u32, value: u32) -> Result<()> {
        self.memory.insert(thing.wrapping_add(offset), value);
        Ok(())
    }
    fn virtual100(&mut self, actor: u32) -> Result<u32> {
        self.events.push(json!(["virtual100", actor]));
        Ok(self.case["virtual100"].as_u64().unwrap() as u32)
    }
    fn release(&mut self, object: u32) -> Result<()> {
        self.events.push(json!(["release", object]));
        Ok(())
    }
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

#[test]
fn original_actor_flags_match_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-actor-flags-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 420);
    for (i, c) in cases.iter().enumerate() {
        let memory = c["memory"].as_array().unwrap().iter().map(|p| (word(&p[0]), word(&p[1]))).collect();
        let mut host = Host { case: c, memory, events: Vec::new() };
        let this = word(&c["this"]);
        let index = c["index"].as_i64().unwrap_or(0) as i32;
        let result: Value = match c["fn"].as_str().unwrap() {
            "4D07D0" => {
                let w = u64::from(host.word(this, 0).unwrap()) | u64::from(host.word(this, 4).unwrap()) << 32;
                let w = with_state_bit(w, word(&c["bit"]), word(&c["flag"]) & 0xFF != 0);
                host.set_word(this, 0, w as u32).unwrap();
                host.set_word(this, 4, (w >> 32) as u32).unwrap();
                Value::Null
            }
            "4D0960" => json!(flag_bit30(host.word(this, 0x100).unwrap()) as u8),
            "4CFFD0" => json!(d8_holder(&mut host, this).unwrap().unwrap_or(0)),
            "4CE880" => {
                clear_164(&mut host, this).unwrap();
                Value::Null
            }
            "4CEE40" => json!(has_98_or_detached_9c(&mut host, this).unwrap() as u32),
            "4CEFA0" => json!(organization_or_virtual100(&mut host, this).unwrap()),
            "4D1240" => {
                set_154(&mut host, this, word(&c["object"])).unwrap();
                Value::Null
            }
            "4D1290" => {
                clear_154(&mut host, this).unwrap();
                Value::Null
            }
            "45E660" => json!(stance_or_last(&mut host, this, index).unwrap() as u32),
            "471A40" => json!(indexed_word(&mut host, this, index).unwrap()),
            other => panic!("{other}"),
        };
        assert_eq!(result, c["result"], "case {i} {c}");
        for pair in c["after"].as_array().unwrap() {
            let address = word(&pair[0]);
            assert_eq!(host.word(address, 0).unwrap(), word(&pair[1]), "case {i} memory {address:#x}");
        }
        assert_eq!(Value::Array(host.events), c["events"], "case {i} events");
    }
}
