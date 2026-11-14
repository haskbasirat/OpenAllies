//! 516FD0 / 518170 against original x86 executions
//! (research/aircraft-mind-dispatch).
use aa_mind::aai_guard_activate::{
    EscortActivateWorld, GuardActivateWorld, activate_escort, activate_guard,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/aircraft-mind-dispatch/native-aircraft-guard-activate.json"
    ))
    .unwrap()
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Escort<'a> {
    input: &'a Value,
    gtid: u32,
    calls: Vec<Value>,
    word_10: u32,
}
impl EscortActivateWorld for Escort<'_> {
    type Error = ();
    type Object = ();
    fn object_gtid(&mut self, _: ()) -> Result<u32, ()> {
        Ok(match self.input["target"].as_str().unwrap() {
            "stale" => self.gtid ^ 0x1_0000,
            "zero" => 0,
            _ => self.gtid,
        })
    }
    fn set_word_10(&mut self, gtid: u32) -> Result<(), ()> {
        self.word_10 = gtid;
        Ok(())
    }
    fn gtid_live(&mut self, gtid: u32) -> Result<bool, ()> {
        // The oracle's table holds one entry, at the fixture GTID's index
        // with its generation, nonnull unless the case is "null".
        Ok(gtid == self.gtid && self.input["target"] != "null")
    }
    fn move_state_set_formation(&mut self, formation: u8) -> Result<(), ()> {
        self.calls.push(json!(["set_formation", formation]));
        Ok(())
    }
    fn activate(&mut self, mode: u32) -> Result<u8, ()> {
        self.calls.push(json!(["activate", mode]));
        Ok(self.input["al"].as_u64().unwrap() as u8)
    }
}

struct Guard {
    calls: Vec<Value>,
    word_10: Option<&'static str>,
    counts: BTreeMap<&'static str, i32>,
}
impl GuardActivateWorld for Guard {
    type Error = ();
    type Leader = &'static str;
    fn word_10(&mut self) -> Result<Option<&'static str>, ()> {
        Ok(self.word_10)
    }
    fn add_ref(&mut self, leader: &'static str) -> Result<(), ()> {
        *self.counts.get_mut(leader).unwrap() += 1;
        Ok(())
    }
    fn release(&mut self, leader: &'static str) -> Result<i32, ()> {
        let count = self.counts.get_mut(leader).unwrap();
        *count -= 1;
        Ok(*count)
    }
    fn destroy(&mut self, leader: &'static str) -> Result<(), ()> {
        self.calls.push(json!(["destroy", leader, 1]));
        Ok(())
    }
    fn set_word_10(&mut self, leader: Option<&'static str>) -> Result<(), ()> {
        self.word_10 = leader;
        Ok(())
    }
    fn move_state_follow(&mut self, leader: Option<&'static str>) -> Result<(), ()> {
        self.calls.push(json!(["follow", leader]));
        Ok(())
    }
    fn activate(&mut self, mode: u32) -> Result<(), ()> {
        self.calls.push(json!(["activate", mode]));
        Ok(())
    }
}

fn leader(v: &Value) -> Option<&'static str> {
    match v.as_str().unwrap() {
        "a" => Some("a"),
        "b" => Some("b"),
        _ => None,
    }
}

#[test]
fn escort_and_guard_activations_match_both_images_all_precisions() {
    let doc = fixture();
    let gtid = word(&doc["gtid"]);
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 342);
    for row in rows {
        let input = &row["input"];
        let mode = word(&input["mode"]);
        if input["function"] == "516fd0" {
            let mut w = Escort { input, gtid, calls: vec![], word_10: 0x9999 };
            let target = (input["target"] != "none").then_some(());
            let formation = input["formation"].as_u64().unwrap() as u8;
            let al = activate_escort(&mut w, mode, target, formation).unwrap();
            assert_eq!(u64::from(al), row["al"].as_u64().unwrap(), "{row}");
            assert_eq!(w.word_10, word(&row["word_10"]), "{row}");
            assert_eq!(json!(w.calls), row["calls"], "{row}");
        } else {
            let count = input["count"].as_i64().unwrap() as i32;
            let mut w = Guard {
                calls: vec![],
                word_10: leader(&input["old"]),
                counts: [("a", count), ("b", count)].into(),
            };
            activate_guard(&mut w, mode, leader(&input["new"])).unwrap();
            assert_eq!(json!(w.word_10), row["word_10"], "{row}");
            assert_eq!(json!(w.counts), row["counts"], "{row}");
            assert_eq!(json!(w.calls), row["calls"], "{row}");
        }
    }
}
