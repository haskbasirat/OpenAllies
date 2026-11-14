//! 516D70 / 516DD0 / 518010 against original x86 executions
//! (research/aircraft-move-state).
use aa_movement::aircraft_move_state::{self as move_state, MoveStateWorld};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/aircraft-move-state/native-aircraft-move-state.json"
    ))
    .unwrap()
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct State<'a> {
    doc: &'a Value,
    input: &'a Value,
    calls: Vec<Value>,
    byte_48: u8,
    word_c: u32,
    word_34: u32,
    words_38: [u32; 2],
}
impl MoveStateWorld for State<'_> {
    type Error = ();
    type Group = u32;
    type Leader = u32;
    fn set_formation_byte(&mut self, value: u8) -> Result<(), ()> {
        self.byte_48 = value;
        Ok(())
    }
    fn formation_byte(&mut self) -> Result<u8, ()> {
        Ok(self.byte_48)
    }
    fn mind_group(&mut self) -> Result<Option<u32>, ()> {
        Ok(self.input["group"].as_bool().unwrap().then(|| word(&self.doc["group"])))
    }
    fn join_formation(&mut self, group: Option<u32>) -> Result<(), ()> {
        self.calls.push(json!(["join", group.unwrap_or(0)]));
        Ok(())
    }
    fn follow_leader(&mut self, group: u32, leader: u32) -> Result<(), ()> {
        self.calls.push(json!(["follow", group, leader]));
        Ok(())
    }
    fn set_status(&mut self, status: u32) -> Result<(), ()> {
        self.word_c = status;
        Ok(())
    }
    fn mover_word_28(&mut self) -> Result<u32, ()> {
        Ok(word(&self.doc["mover_28"]))
    }
    fn set_cruise(&mut self, bits: u32) -> Result<(), ()> {
        self.word_34 = bits;
        Ok(())
    }
    fn followed_position(&mut self) -> Result<Option<[u32; 2]>, ()> {
        let p = &self.doc["target_position"];
        Ok((self.input["handle"] == "live").then(|| [word(&p[0]), word(&p[1])]))
    }
    fn set_followed_position(&mut self, xy: [u32; 2]) -> Result<(), ()> {
        self.words_38 = xy;
        Ok(())
    }
    fn clear_flight_path(&mut self) -> Result<(), ()> {
        self.calls.push(json!(["clear_path"]));
        Ok(())
    }
}

#[test]
fn formation_follow_and_activation_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 324);
    for row in rows {
        let input = &row["input"];
        let mut s = State {
            doc: &doc,
            input,
            calls: vec![],
            byte_48: input["initial"].as_u64().unwrap() as u8,
            word_c: 0x77,
            word_34: 0x1111_1111,
            words_38: [0x2222_2222, 0x3333_3333],
        };
        match input["function"].as_str().unwrap() {
            "516d70" => move_state::set_formation(&mut s, input["flag"].as_u64().unwrap() as u8),
            "516dd0" => {
                let leader = input["leader"].as_bool().unwrap().then(|| word(&doc["leader"]));
                move_state::follow(&mut s, leader)
            }
            "518010" => move_state::activate(&mut s),
            other => panic!("{other}"),
        }
        .unwrap();
        assert_eq!(json!(s.calls), row["calls"], "{row}");
        assert_eq!(u64::from(s.byte_48), row["byte_48"].as_u64().unwrap(), "{row}");
        assert_eq!(s.word_c, word(&row["word_c"]), "{row}");
        assert_eq!(s.word_34, word(&row["word_34"]), "{row}");
        assert_eq!(s.words_38, [word(&row["words_38"][0]), word(&row["words_38"][1])], "{row}");
    }
}
