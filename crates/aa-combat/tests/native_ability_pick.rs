//! 51A260 against original x86 executions (research/ability-pick).
use aa_combat::Result;
use aa_combat::ability_pick::{AbilityPickWorld, pick};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::VecDeque;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/ability-pick/native-ability-pick.json"))
        .unwrap()
}

fn precision(row: &Value) -> Precision {
    match row["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Mind<'a> {
    input: &'a Value,
    calls: Vec<Value>,
    words: [u32; 4],
    draws: VecDeque<u32>,
}
impl Mind<'_> {
    fn ability(&self, index: i32) -> &Value {
        &self.input["abilities"][index as usize]
    }
}
impl AbilityPickWorld for Mind<'_> {
    type Modifier = ();
    fn begin_descriptor(&mut self, words: [u32; 4]) -> Result<()> {
        self.words = words;
        Ok(())
    }
    fn ability_count(&mut self) -> Result<i32> {
        Ok(self.input["abilities"].as_array().unwrap().len() as i32)
    }
    fn ability_enabled(&mut self, index: i32) -> Result<bool> {
        Ok(self.ability(index)["enabled"].as_bool().unwrap())
    }
    fn ability_flag_54_bit3(&mut self, index: i32) -> Result<bool> {
        Ok(self.ability(index)["bit3"].as_bool().unwrap())
    }
    fn admission(&mut self, index: i32) -> Result<u32> {
        self.calls.push(json!(["admission", index]));
        self.calls.push(json!(["descriptor", 1, true, self.words]));
        Ok(word(&self.ability(index)["status"]))
    }
    fn owner_has_mover(&mut self) -> Result<bool> {
        Ok(self.input["mover"].as_bool().unwrap())
    }
    fn owner_mana_regeneration(&mut self) -> Result<u32> {
        Ok(word(&self.input["regen"]))
    }
    fn owner_modifier(&mut self) -> Result<()> {
        self.calls.push(json!(["modifier"]));
        Ok(())
    }
    fn recharge(&mut self, index: i32) -> Result<Binary> {
        self.calls.push(json!(["recharge", index]));
        Ok(Binary::from_f64_bits(self.ability(index)["recharge"].as_u64().unwrap())?)
    }
    fn range(&mut self, index: i32, _: ()) -> Result<Binary> {
        self.calls.push(json!(["range", index]));
        Ok(Binary::from_f64_bits(self.ability(index)["range"].as_u64().unwrap())?)
    }
    fn random_draw(&mut self) -> Result<u32> {
        let value = self.draws.pop_front().unwrap();
        self.calls.push(json!(["rng"]));
        self.calls.push(json!(["draw", value]));
        Ok(value)
    }
}

#[test]
fn picked_ability_class_calls_and_draws_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3600);
    for row in rows {
        let input = &row["input"];
        let words = std::array::from_fn(|i| word(&input["words"][i]));
        let draws = input["draws"].as_array().unwrap().iter().map(word).collect();
        let mut mind = Mind { input, calls: vec![], words: [0; 4], draws };
        let result = pick(&mut mind, words, word(&input["last_word"]), precision(row)).unwrap();
        assert_eq!(result.class, word(&row["eax"]), "{row}");
        let out = if input["out_pointer"].as_bool().unwrap() {
            json!(result.ability)
        } else {
            json!("untouched")
        };
        assert_eq!(out, row["out"], "{row}");
        assert_eq!(json!(mind.calls), row["calls"], "{row}");
    }
}
