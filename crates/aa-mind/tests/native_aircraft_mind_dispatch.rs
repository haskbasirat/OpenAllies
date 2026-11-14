//! 51C6F0 / 51C8C0 / 517190 against original x86 executions
//! (research/aircraft-mind-dispatch).
use aa_mind::aai_mind_dispatch::{self as dispatch, DispatchWorld, MindState, StateExitWorld};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/aircraft-mind-dispatch/native-aircraft-mind-dispatch.json"
    ))
    .unwrap()
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Mind<'a> {
    input: &'a Value,
    calls: Vec<Value>,
    word_b0: u32,
    word_14: u32,
    word_c: u32,
}
impl Mind<'_> {
    fn separate(&self) -> bool {
        self.input["separate_owner"].as_bool().unwrap()
    }
    fn owner(&self) -> (u32, u32) {
        let status = if self.separate() { &self.input["owner_status"] } else { &self.input["status"] };
        (word(status), word(&self.input["phase"]))
    }
    fn leave(&mut self) {
        let who = if self.separate() { "owner" } else { "mind" };
        self.calls.push(json!(["leave_e8", who]));
    }
}
impl DispatchWorld for Mind<'_> {
    type Error = ();
    fn is_current(&mut self, state: MindState) -> Result<bool, ()> {
        let name = match state {
            MindState::At9C => "9c",
            MindState::AtE8 => "e8",
            MindState::At12C => "12c",
        };
        Ok(self.input["current"] == name)
    }
    fn attack_status(&mut self) -> Result<u32, ()> {
        Ok(word(&self.input["status"]))
    }
    fn clear_b0(&mut self) -> Result<(), ()> {
        self.word_b0 = 0;
        Ok(())
    }
    fn owner_attack_status_phase(&mut self) -> Result<(u32, u32), ()> {
        Ok(self.owner())
    }
    fn leave_owner_attack(&mut self) -> Result<(), ()> {
        self.leave();
        Ok(())
    }
    fn pop_state(&mut self) -> Result<(), ()> {
        self.calls.push(json!(["pop"]));
        Ok(())
    }
    fn schedule(&mut self, delay_bits: u32) -> Result<(), ()> {
        self.calls.push(json!(["schedule", delay_bits, 0]));
        Ok(())
    }
}
impl StateExitWorld for Mind<'_> {
    type Error = ();
    fn set_word_14(&mut self, value: u32) -> Result<(), ()> {
        self.word_14 = value;
        Ok(())
    }
    fn set_status(&mut self, value: u32) -> Result<(), ()> {
        self.word_c = value;
        Ok(())
    }
    fn owner_attack_status_phase(&mut self) -> Result<(u32, u32), ()> {
        Ok(self.owner())
    }
    fn leave_owner_attack(&mut self) -> Result<(), ()> {
        self.leave();
        Ok(())
    }
}

#[test]
fn dispatcher_and_state_exits_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1512);
    for row in rows {
        let input = &row["input"];
        let mut mind =
            Mind { input, calls: vec![], word_b0: 0x5555, word_14: 0x5555, word_c: 0x66 };
        match input["function"].as_str().unwrap() {
            "51c6f0" => {
                let al = dispatch::dispatch(&mut mind).unwrap();
                assert_eq!(u64::from(al), row["al"].as_u64().unwrap(), "{row}");
                assert_eq!(mind.word_b0, word(&row["word_b0"]), "{row}");
            }
            f => {
                if f == "51c8c0" {
                    dispatch::exit_51c8c0(&mut mind).unwrap();
                } else {
                    dispatch::exit_517190(&mut mind).unwrap();
                }
                assert_eq!(mind.word_14, word(&row["word_14"]), "{row}");
                assert_eq!(mind.word_c, word(&row["word_c"]), "{row}");
            }
        }
        assert_eq!(json!(mind.calls), row["calls"], "{row}");
    }
}
