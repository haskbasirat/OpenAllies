//! 4ECB90 against original x86 executions (research/memory-reveal).
use aa_combat::Result;
use aa_combat::memory_reveal::{MemoryWorld, update_memory};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/memory-reveal/native-memory-reveal.json"))
        .unwrap()
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Kingdom<'a> {
    input: &'a Value,
    owner: u32,
    calls: Vec<Value>,
    word_14: u32,
}
impl MemoryWorld for Kingdom<'_> {
    type Actor = ();
    type Record = ();
    fn actor_word_190(&mut self, _: ()) -> Result<u32> {
        Ok(word(&self.input["word_190"]))
    }
    fn kingdom_8c(&mut self) -> Result<bool> {
        self.calls.push(json!(["8c"]));
        Ok(word(&self.input["kingdom_8c"]) != 0)
    }
    fn actor_index(&mut self, _: ()) -> Result<u32> {
        Ok(5)
    }
    fn record(&mut self, index: u32) -> Result<Option<()>> {
        assert_eq!(index, 5);
        Ok(self.input["record"].as_bool().unwrap().then_some(()))
    }
    fn kingdom_108(&mut self) -> Result<u32> {
        self.calls.push(json!(["108"]));
        Ok(self.owner)
    }
    fn record_word_14(&mut self, _: ()) -> Result<u32> {
        Ok(self.word_14)
    }
    fn set_record_word_14(&mut self, _: (), value: u32) -> Result<()> {
        self.word_14 = value;
        Ok(())
    }
    fn refresh_record(&mut self, _: ()) -> Result<()> {
        self.calls.push(json!(["refresh"]));
        Ok(())
    }
    fn notifications(&mut self) -> Result<bool> {
        Ok(self.input["notifications"] == "on")
    }
    fn notify_before(&mut self, slot: Option<()>, _: ()) -> Result<()> {
        self.calls.push(json!(["before", slot.is_some()]));
        Ok(())
    }
    fn notify_after(&mut self, slot: Option<()>, _: ()) -> Result<()> {
        self.calls.push(json!(["after", slot.is_some()]));
        Ok(())
    }
}

#[test]
fn record_update_calls_and_al_match_both_images_all_precisions() {
    let doc = fixture();
    let owner = word(&doc["owner"]);
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1152);
    for row in rows {
        let input = &row["input"];
        let word_14 = if input["word_14"] == "owner" { owner } else { 0x1234 };
        let mut k = Kingdom { input, owner, calls: vec![], word_14 };
        let actor = input["actor"].as_bool().unwrap().then_some(());
        let keep = word(&input["keep_owner"]) != 0;
        let al = update_memory(&mut k, actor, keep).unwrap();
        assert_eq!(u64::from(al), row["al"].as_u64().unwrap(), "{row}");
        assert_eq!(json!(k.calls), row["calls"], "{row}");
        assert_eq!(k.word_14, word(&row["word_14"]), "{row}");
    }
}
