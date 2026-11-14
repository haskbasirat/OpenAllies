//! Replays research/phase8-ai/native-speed-scale.json (original 52D620
//! under Unicorn, at PC24/PC53/PC64) against
//! aa_mind::element_speed::scale_speed.
use aa_mind::{
    Result,
    element_speed::{SpeedScaleWorld, scale_speed},
};
use aa_numeric::Precision;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-speed-scale.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

struct Host {
    speed: u32,
    word50: u32,
    events: Vec<Value>,
}
impl SpeedScaleWorld for Host {
    fn requested_speed(&mut self) -> Result<u32> {
        Ok(self.speed)
    }
    fn set_requested_speed(&mut self, bits: u32) -> Result<()> {
        self.speed = bits;
        Ok(())
    }
    fn locomotion_word50(&mut self) -> Result<u32> {
        Ok(self.word50)
    }
    fn locomotion_set_speed(&mut self, speed_bits: u32, factor_bits: u32) -> Result<()> {
        self.events.push(
            json!({"event": "locomotion_set_speed", "speed": speed_bits, "factor": factor_bits}),
        );
        Ok(())
    }
}

#[test]
fn native_speed_scale_matches_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["52D620"], json!([21, 21]));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 900);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Host {
            speed: u(&c["speed"]),
            word50: u(&c["word50"]),
            events: vec![],
        };
        let r = scale_speed(u(&c["divisor"]), u(&c["factor"]), &mut host, p).unwrap();
        assert_eq!(json!(u8::from(r)), o["result"], "{name}");
        assert_eq!(json!(host.speed), o["speed"], "{name}");
        assert_eq!(json!(host.events), o["events"], "{name}");
    }
}
