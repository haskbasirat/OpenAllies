//! Replays research/phase8-ai/native-travel-time.json (original 7A1C10 and
//! 7A1B30 under Unicorn) against aa_mind::sai::travel.
use aa_mind::{
    Result,
    sai::travel::{TravelWorld, travel_cost, travel_time},
    x87::binary_from_x87,
};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};

struct Host {
    c: Value,
    unavailable: Vec<bool>,
    events: Vec<Value>,
}

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

impl TravelWorld for Host {
    fn unavailable(&mut self, _actor: u32) -> Result<bool> {
        let r = self.unavailable.remove(0);
        self.events.push(json!({"op": "unavailable", "result": r}));
        Ok(r)
    }
    fn cached_cost(&mut self, _actor: u32) -> Result<u32> {
        Ok(u(&self.c["cached_cost"]))
    }
    fn cached_location(&mut self, _actor: u32) -> Result<u32> {
        let r = u(&self.c["cached_location"]);
        self.events
            .push(json!({"op": "cached_location", "result": r}));
        Ok(r)
    }
    fn start_region(&mut self, _actor: u32) -> Result<u32> {
        let r = u(&self.c["start_region"]);
        self.events.push(json!({"op": "start_region", "result": r}));
        Ok(r)
    }
    fn location_region(&mut self, location: u32) -> Result<u32> {
        Ok(u(&self.c["regions"][location.to_string()]))
    }
    fn speed(&mut self, _actor: u32) -> Result<Binary> {
        let v = self.c["speed"].clone();
        self.events.push(json!({"op": "speed", "result": v}));
        binary_from_x87(
            v["significand"].as_u64().unwrap(),
            v["sign_exponent"].as_u64().unwrap() as u16,
        )
    }
    fn route_cost(&mut self, start: u32, goal: u32, _p: Precision) -> Result<u32> {
        let r = u(&self.c["route_cost"]);
        self.events
            .push(json!({"op": "route", "start": start, "goal": goal, "result": r}));
        Ok(r)
    }
}

#[test]
fn native_travel_time_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-travel-time.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    let mut divided = 0;
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let o = &row["output"];
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        let mut host = Host {
            c: c.clone(),
            unavailable: c["unavailable"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_bool().unwrap())
                .collect(),
            events: vec![],
        };
        let location = u(&c["location"]);
        let st0 = if u(&c["entry"]) == 0x7a1b30 {
            Binary::from_f32_bits(travel_cost(&mut host, 1, location, p).unwrap()).unwrap()
        } else {
            travel_time(&mut host, 1, location, p).unwrap()
        };
        assert!(host.unavailable.is_empty(), "case {i}: unused answers");
        assert_eq!(json!(host.events), o["events"], "case {i}");
        let bits = st0.to_x87_extended_bits().unwrap();
        assert_eq!(
            (bits as u64, (bits >> 64) as u64),
            (
                o["st0"]["extended_significand"].as_u64().unwrap(),
                o["st0"]["extended_sign_exponent"].as_u64().unwrap()
            ),
            "case {i}: ST0"
        );
        if host.events.iter().any(|e| e["op"] == "speed") {
            divided += 1;
        }
    }
    assert!(divided > 300, "{divided}");
}
