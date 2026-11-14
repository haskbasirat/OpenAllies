//! 517C60 against original x86 executions (research/aircraft-mind-activate).
use aa_mind::aai_mind_activate::{self, World};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/aircraft-mind-activate/native-aircraft-mind-activate.json"
    ))
    .unwrap()
}

struct Mind<'a> {
    input: &'a Value,
    calls: Vec<Value>,
    byte_1d0: u8,
    byte_7c: u8,
}
impl World for Mind<'_> {
    type Error = ();
    fn stack_empty(&mut self) -> Result<bool, ()> {
        Ok(self.input["stack_empty"].as_bool().unwrap())
    }
    fn clear_188_owner_byte_7c(&mut self) -> Result<(), ()> {
        self.byte_7c = 0;
        Ok(())
    }
    fn join_formation(&mut self, mode: u32) -> Result<(), ()> {
        self.calls.push(json!(["join", true, mode]));
        Ok(())
    }
    fn activate_188(&mut self, mode: u32) -> Result<(), ()> {
        self.calls.push(json!(["activate_188", true, mode]));
        Ok(())
    }
    fn base_activate(&mut self) -> Result<u8, ()> {
        self.calls.push(json!(["base", true]));
        Ok(self.input["base_al"].as_u64().unwrap() as u8)
    }
    fn ability_count(&mut self) -> Result<i32, ()> {
        Ok(self.input["count"].as_i64().unwrap() as i32)
    }
    fn ability_3c_word_2b4(&mut self, index: i32) -> Result<Option<u32>, ()> {
        let ability = &self.input["abilities"][index as usize];
        Ok(ability.as_u64().map(|w| w as u32))
    }
    fn set_dogfighter(&mut self, value: u8) -> Result<(), ()> {
        self.byte_1d0 = value;
        Ok(())
    }
}

#[test]
fn dogfighter_byte_calls_and_al_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1152);
    for row in rows {
        let mut mind = Mind { input: &row["input"], calls: vec![], byte_1d0: 0xaa, byte_7c: 0xcc };
        let al = aai_mind_activate::activate(&mut mind).unwrap();
        assert_eq!(u64::from(al), row["al"].as_u64().unwrap(), "{row}");
        assert_eq!(json!(mind.calls), row["calls"], "{row}");
        assert_eq!(u64::from(mind.byte_1d0), row["byte_1d0"].as_u64().unwrap(), "{row}");
        assert_eq!(u64::from(mind.byte_7c), row["byte_7c"].as_u64().unwrap(), "{row}");
    }
}
