//! 4E1D90 against original x86 executions (research/kingdom-bases).
use aa_economy::kingdom_bases::{BaseWorld, nearest_base};
use aa_economy::{EconomyError, F32Bits, X87Precision};
use serde_json::{Value, json};

const ACTOR: usize = usize::MAX;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/kingdom-bases/native-kingdom-bases.json"))
        .unwrap()
}

fn precision(row: &Value) -> X87Precision {
    match row["precision"].as_str().unwrap() {
        "Single24" => X87Precision::Single24,
        "Double53" => X87Precision::Double53,
        "Extended64" => X87Precision::Extended64,
        other => panic!("{other}"),
    }
}

struct Kingdom<'a> {
    input: &'a Value,
    calls: Vec<Value>,
}
impl Kingdom<'_> {
    fn record(&self, actor: usize) -> &Value {
        if actor == ACTOR { &self.input["actor"] } else { &self.input["bases"][actor] }
    }
    fn query(&mut self, name: &str, base: usize, actor: usize) -> bool {
        assert_eq!(actor, ACTOR);
        // The argument is the actor's +4 word the oracle planted.
        self.calls.push(json!([name, base, 0x1234abcd_u32]));
        self.input["bases"][base]["accepts"].as_bool().unwrap()
    }
}
impl BaseWorld for Kingdom<'_> {
    type Actor = usize;
    type Error = EconomyError;
    fn removed(&mut self, actor: usize) -> Result<bool, EconomyError> {
        Ok(self.record(actor)["removed"].as_bool().unwrap())
    }
    fn position(&mut self, actor: usize) -> Result<[F32Bits; 2], EconomyError> {
        let p = &self.record(actor)["pos"];
        Ok([F32Bits(p[0].as_u64().unwrap() as u32), F32Bits(p[1].as_u64().unwrap() as u32)])
    }
    fn has_denizen(&mut self, base: usize) -> Result<bool, EconomyError> {
        Ok(self.input["bases"][base]["denizen"].as_bool().unwrap())
    }
    fn word_c0(&mut self, base: usize) -> Result<bool, EconomyError> {
        Ok(self.input["bases"][base]["c0"].as_bool().unwrap())
    }
    fn denizen_568120(&mut self, base: usize, actor: usize) -> Result<bool, EconomyError> {
        Ok(self.query("568120", base, actor))
    }
    fn denizen_568ea0(&mut self, base: usize, actor: usize) -> Result<bool, EconomyError> {
        Ok(self.query("568ea0", base, actor))
    }
}

#[test]
fn nearest_base_and_denizen_queries_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 738);
    for row in rows {
        let input = &row["input"];
        let bases: Vec<Option<usize>> = input["bases"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, b)| (!b["null"].as_bool().unwrap()).then_some(i))
            .collect();
        let actor = (!input["actor"].is_null()).then_some(ACTOR);
        let mut kingdom = Kingdom { input, calls: vec![] };
        let result = nearest_base(&mut kingdom, &bases, actor, precision(row)).unwrap();
        assert_eq!(json!(result), row["result"], "{row}");
        assert_eq!(json!(kingdom.calls), row["calls"], "{row}");
    }
}
