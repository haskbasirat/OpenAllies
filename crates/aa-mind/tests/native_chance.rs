//! Replays research/phase8-combat/native-chance-fixtures.json: original
//! 4A6540 under Unicorn with the real 85D840 and _CIpow (SSE2 path), all
//! precisions; result, RNG state and seam calls must match.
use aa_mind::{
    Result,
    sai::chance::{ChanceFields, ChanceWorld, chance},
};
use aa_numeric::{Binary, Precision, crt_pow::CrtEnvironment};
use serde_json::{Value, json};

struct Host<'a> {
    case: &'a Value,
    state: u32,
    events: Vec<Value>,
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

impl ChanceWorld for Host<'_> {
    fn lookup(&mut self, owner: u32, key: u32) -> Result<u32> {
        assert_eq!((owner, key), (2, 1));
        self.events.push(json!(["lookup"]));
        Ok(3)
    }
    fn key_word8(&mut self, _: u32) -> Result<u32> {
        Ok(word(&self.case["word8"]))
    }
    fn value_7b9570(&mut self, object: u32, w: u32) -> Result<Binary> {
        assert_eq!(object, 3);
        self.events.push(json!(["7B9570", w]));
        Ok(Binary::from_f64_bits(self.case["a"].as_u64().unwrap()).unwrap())
    }
    fn value_7b96e0(&mut self, object: u32) -> Result<Binary> {
        assert_eq!(object, 3);
        self.events.push(json!(["7B96E0"]));
        Ok(Binary::from_f64_bits(self.case["b"].as_u64().unwrap()).unwrap())
    }
    fn draw(&mut self, rng: u32) -> Result<u32> {
        assert_eq!(rng, 9);
        let wide = u64::from(self.state) * 0x41c6_4e6d + 0x3039;
        self.state = wide as u32;
        Ok((wide >> 16) as u32)
    }
}

#[test]
fn original_chance_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-chance-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 450);
    for (i, c) in cases.iter().enumerate() {
        let (p, control) = match c["precision"].as_str().unwrap() {
            "Single24" => (Precision::Single24, 0x7f),
            "Double53" => (Precision::Double53, 0x27f),
            _ => (Precision::Extended64, 0x37f),
        };
        let crt = CrtEnvironment { sse2_available: true, mxcsr: 0x1f80, x87_control: control };
        let fields = ChanceFields {
            field804: word(&c["field804"]),
            field808: word(&c["field808"]),
            field80c: word(&c["field80c"]),
            field810: word(&c["field810"]),
            field814: word(&c["field814"]),
        };
        let mut host = Host { case: c, state: word(&c["state"]), events: Vec::new() };
        let result = chance(&mut host, fields, 9, 1, 2, crt, p).unwrap();
        assert_eq!(json!(result as u8), c["result"], "case {i} {c}");
        assert_eq!(host.state, word(&c["state_after"]), "case {i} state");
        assert_eq!(Value::Array(host.events), c["events"], "case {i} events");
    }
}
