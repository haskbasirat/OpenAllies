//! Replays research/phase8-combat/native-overlap-escape-fixtures.json:
//! original 4D1820 (overlap list walk), 4CF210 and 4D04D0 (with 409230)
//! under Unicorn, all precisions. The x87 FCOS/FSIN leaf comes from the
//! fixture's per-precision table, looked up by the extended angle the port
//! computes; result, point, remaining list, frees and query points must match.
use aa_navigation::{Result, overlap_escape::*};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::VecDeque;

const ACTOR: u32 = 100;

fn ext(v: &Value) -> u128 {
    (u128::from(v["sign_exponent"].as_u64().unwrap() as u16) << 64)
        | u128::from(u64::from_str_radix(v["mantissa"].as_str().unwrap(), 16).unwrap())
}

fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}

struct Host<'a> {
    case: &'a Value,
    table: Vec<&'a Value>,
    overlaps: VecDeque<u32>,
    events: Vec<Value>,
    queries: usize,
}

impl Host<'_> {
    fn thing(&self, handle: u32) -> &Value {
        &self.case["things"][(handle - 1) as usize]
    }
}

impl World for Host<'_> {
    fn overlap_head(&mut self, _: u32) -> Result<Option<u32>> {
        Ok(self.overlaps.front().copied())
    }
    fn pop_overlap_head(&mut self, _: u32) -> Result<()> {
        self.overlaps.pop_front();
        self.events.push(json!(["free"]));
        Ok(())
    }
    fn registry_generation(&mut self, index: u32) -> Result<u16> {
        Ok(self.case["things"].get((index as usize).wrapping_sub(1)).map_or(0, |t| word(&t["generation"]) as u16))
    }
    fn registry_object(&mut self, index: u32) -> Result<Option<u32>> {
        Ok(self.case["things"]
            .get((index as usize).wrapping_sub(1))
            .and_then(|t| t["live"].as_bool().unwrap().then_some(index)))
    }
    fn position(&mut self, thing: u32) -> Result<[u32; 2]> {
        let p = if thing == ACTOR { &self.case["position"] } else { &self.thing(thing)["position"] };
        Ok([word(&p[0]), word(&p[1])])
    }
    fn snapshot_radius(&mut self, thing: u32) -> Result<u32> {
        Ok(word(&self.thing(thing)["radius"]))
    }
    fn own_radius(&mut self, _: u32) -> Result<u32> {
        Ok(word(&self.case["own_radius"]))
    }
    fn cos_sin(&mut self, angle: &Binary) -> Result<(Binary, Binary)> {
        let bits = angle.to_x87_extended_bits().expect("extended angle");
        let row = self
            .table
            .iter()
            .find(|r| ext(&r["angle"]) == bits)
            .unwrap_or_else(|| panic!("no tabulated angle {bits:#x}"));
        Ok((Binary::from_x87_extended_bits(ext(&row["cos"]))?, Binary::from_x87_extended_bits(ext(&row["sin"]))?))
    }
    fn free_at(&mut self, _: u32, point: [u32; 2]) -> Result<bool> {
        self.events.push(json!(["query", point[0], point[1]]));
        self.queries += 1;
        Ok(!self.case["occupied"][self.queries - 1].as_bool().unwrap())
    }
}

#[test]
fn original_overlap_escape_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-overlap-escape-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 480);
    for (i, case) in cases.iter().enumerate() {
        let name = case["precision"].as_str().unwrap();
        let precision = match name {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let table = doc["transcendentals"].as_array().unwrap().iter().filter(|r| r["precision"] == name).collect();
        let overlaps = case["overlaps"].as_array().unwrap().iter().map(word).collect();
        let mut host = Host { case, table, overlaps, events: Vec::new(), queries: 0 };
        let point = escape_point(&mut host, ACTOR, precision).unwrap();
        assert_eq!(json!(point.is_some() as u8), case["result"], "case {i}");
        let written = point.unwrap_or([0xDEAD_BEEF, 0xDEAD_BEEF]);
        assert_eq!(json!(written), case["point"], "case {i} point");
        assert_eq!(json!(host.overlaps), case["remaining"], "case {i} remaining");
        assert_eq!(case["tail_ok"], true);
        assert_eq!(Value::Array(host.events), case["events"], "case {i} events");
    }
}
