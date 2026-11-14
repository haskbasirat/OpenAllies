//! 51B4C0 against original x86 executions (research/aircraft-attack).
use aa_mind::aai_attack_phase::{AttackPhaseWorld, step};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/aircraft-attack/native-attack-phase.json"))
        .unwrap()
}

struct State<'a> {
    input: &'a Value,
    calls: Vec<Value>,
    phase: u32,
    retarget: u8,
    flight_10: u8,
}
impl AttackPhaseWorld for State<'_> {
    type Error = ();
    fn phase(&mut self) -> Result<u32, ()> {
        Ok(self.phase)
    }
    fn set_phase(&mut self, phase: u32) -> Result<(), ()> {
        self.phase = phase;
        Ok(())
    }
    fn retarget_byte(&mut self) -> Result<u8, ()> {
        Ok(self.retarget)
    }
    fn set_retarget_byte(&mut self, value: u8) -> Result<(), ()> {
        self.retarget = value;
        Ok(())
    }
    fn plan(&mut self) -> Result<(), ()> {
        self.calls.push(json!(["plan", self.phase]));
        if let Some(p) = self.input["plan_sets"].as_u64() {
            self.phase = p as u32;
        }
        Ok(())
    }
    fn clear_flight_byte_10(&mut self) -> Result<(), ()> {
        self.flight_10 = 0;
        Ok(())
    }
    fn clear_flight_path(&mut self) -> Result<(), ()> {
        self.calls.push(json!(["clear_path", self.flight_10]));
        Ok(())
    }
    fn move_think(&mut self) -> Result<(), ()> {
        self.calls.push(json!(["move_think"]));
        Ok(())
    }
    fn schedule(&mut self, delay_bits: u32) -> Result<(), ()> {
        self.calls.push(json!(["schedule", delay_bits, 0]));
        Ok(())
    }
}

#[test]
fn phase_advance_plan_and_schedule_match_both_images_all_precisions() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 504);
    for row in rows {
        let input = &row["input"];
        let mut s = State {
            input,
            calls: vec![],
            phase: input["phase"].as_u64().unwrap() as u32,
            retarget: input["retarget"].as_u64().unwrap() as u8,
            flight_10: 0x77,
        };
        step(&mut s).unwrap();
        assert_eq!(json!(s.calls), row["calls"], "{row}");
        assert_eq!(u64::from(s.phase), row["phase"].as_u64().unwrap(), "{row}");
        assert_eq!(u64::from(s.retarget), row["retarget"].as_u64().unwrap(), "{row}");
        assert_eq!(u64::from(s.flight_10), row["flight_10"].as_u64().unwrap(), "{row}");
    }
}
