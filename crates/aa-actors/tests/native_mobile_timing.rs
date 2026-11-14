//! Replays research/phase8-combat/native-mobile-timing.json (original
//! 575B20 / 575A30 and 571700 under Unicorn) against
//! aa_actors::mobile_timing: the seam calls in order and the stored
//! deadline bits, and 571700's unstored 80-bit result.
use aa_actors::mobile_timing::{Result, TimedStateWorld, ratio, start_timed_state, update_timed_state};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-mobile-timing.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}

struct Host<'a> {
    c: &'a Value,
    events: Vec<Value>,
    deadline: u32,
}
impl TimedStateWorld for Host<'_> {
    fn actor_refresh(&mut self) -> Result<()> {
        self.events.push(json!(["refresh"]));
        Ok(())
    }
    fn actor_flags(&mut self) -> Result<u32> {
        Ok(u(&self.c["flags"]))
    }
    fn definition_word(&mut self, offset: u32) -> Result<u32> {
        Ok(u(&self.c[match offset {
            0x54c => "threshold",
            0x550 => "delay",
            0x554 => "cap",
            other => panic!("definition +{other:X}"),
        }]))
    }
    fn health_fraction(&mut self) -> Result<Binary> {
        self.events.push(json!(["health"]));
        Ok(Binary::from_f64_bits(self.c["health"].as_f64().unwrap().to_bits())?)
    }
    fn clear_timers(&mut self) -> Result<()> {
        self.events.push(json!(["clear"]));
        Ok(())
    }
    fn set_actor_state(&mut self, state: u32, on: bool) -> Result<()> {
        self.events.push(json!(["state", state, u32::from(on)]));
        Ok(())
    }
    fn time(&mut self) -> Result<u32> {
        Ok(u(&self.c["time"]))
    }
    fn deadline(&mut self) -> Result<u32> {
        Ok(self.deadline)
    }
    fn set_deadline(&mut self, bits: u32) -> Result<()> {
        self.deadline = bits;
        Ok(())
    }
    fn add_timer(&mut self, delay_bits: u32, data: u32) -> Result<()> {
        self.events.push(json!(["timer", delay_bits, data]));
        Ok(())
    }
}

#[test]
fn mobile_timing_matches_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 2400);
    let (mut started, mut ratios) = (0, 0);
    for case in cases {
        let c = &case["input"];
        let want = &case["output"];
        let name = c["name"].as_str().unwrap();
        let p = precision(c["precision"].as_str().unwrap());
        if c["entry"] == "ratio" {
            let got = ratio(u(&c["a"]), u(&c["b"]), p).unwrap();
            let hex = want["extended"].as_str().unwrap();
            let mut bytes = [0u8; 16];
            for (i, b) in bytes.iter_mut().take(10).enumerate() {
                *b = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
            }
            let native = Binary::from_x87_extended_bits(u128::from_le_bytes(bytes)).unwrap();
            assert_eq!(got.compare(&native), std::cmp::Ordering::Equal, "{name} ratio");
            assert_eq!(got.is_negative_nonzero(), native.is_negative_nonzero(), "{name} sign");
            ratios += 1;
            continue;
        }
        let mut host = Host { c, events: Vec::new(), deadline: u(&c["deadline"]) };
        if c["entry"] == "start" {
            start_timed_state(&mut host, p).unwrap();
            started += 1;
        } else {
            started += usize::from(update_timed_state(&mut host, p).unwrap());
        }
        assert_eq!(Value::Array(host.events), want["events"], "{name} events");
        assert_eq!(host.deadline, u(&want["deadline"]), "{name} deadline");
    }
    assert!(started > 800 && ratios == 900, "{started} {ratios}");
}
