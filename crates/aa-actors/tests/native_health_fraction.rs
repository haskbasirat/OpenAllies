//! Replays research/phase8-combat/native-health-fraction-fixtures.json:
//! original 4D0230 (with 4CEDA0) fully under Unicorn, all precisions; ST0 as
//! the 80-bit register image, the x87 indefinite for a mean over no parts.
use aa_actors::health_fraction::*;
use aa_body::F32;
use aa_numeric::Precision;
use serde_json::Value;

struct Host<'a> {
    world: &'a Value,
}

impl Host<'_> {
    /// Handles: leaves 1..=n, the top actor n + 1.
    fn thing(&self, handle: u32) -> &Value {
        let leaves = self.world["leaves"].as_array().unwrap();
        if handle as usize == leaves.len() + 1 { &self.world["top"] } else { &leaves[(handle - 1) as usize] }
    }
}

fn handle(v: &Value) -> Option<u32> {
    v.as_u64().map(|i| i as u32 + 1)
}

impl World for Host<'_> {
    fn body(&mut self, actor: u32) -> Result<Option<[F32; 2]>> {
        let b = &self.thing(actor)["body"];
        Ok((!b.is_null()).then(|| [F32(b[0].as_u64().unwrap() as u32), F32(b[1].as_u64().unwrap() as u32)]))
    }
    fn component98(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok((!self.thing(actor)["c98"].is_null()).then_some(actor))
    }
    fn component98_leader(&mut self, c: u32) -> Result<Option<u32>> {
        Ok(handle(&self.thing(c)["c98"]["leader"]))
    }
    fn component98_list_count(&mut self, c: u32) -> Result<i32> {
        Ok(self.thing(c)["c98"]["list_count"].as_i64().unwrap() as i32)
    }
    fn component98_list(&mut self, c: u32, i: i32) -> Result<Option<u32>> {
        Ok(handle(&self.thing(c)["c98"]["list"][i as usize]))
    }
    fn component98_array_count(&mut self, c: u32) -> Result<i32> {
        Ok(self.thing(c)["c98"]["array_count"].as_i64().unwrap() as i32)
    }
    fn component98_array(&mut self, c: u32, i: i32) -> Result<Option<u32>> {
        Ok(handle(&self.thing(c)["c98"]["array"][i as usize]))
    }
    fn organization(&mut self, actor: u32) -> Result<Option<u32>> {
        Ok((!self.thing(actor)["org"].is_null()).then_some(actor))
    }
    fn member_count(&mut self, o: u32) -> Result<i32> {
        Ok(self.thing(o)["org"]["members"].as_array().unwrap().len() as i32)
    }
    fn slot_enabled(&mut self, o: u32, i: i32) -> Result<bool> {
        let org = &self.thing(o)["org"];
        let slot = org["slot_index"][i as usize].as_u64().unwrap() as usize;
        Ok(org["enabled"][slot].as_u64().unwrap() != 0)
    }
    fn member(&mut self, o: u32, i: i32) -> Result<Option<u32>> {
        Ok(handle(&self.thing(o)["org"]["members"][i as usize]))
    }
}

#[test]
fn original_health_fraction_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-health-fraction-fixtures.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 600);
    for (i, c) in cases.iter().enumerate() {
        let precision = match c["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            _ => Precision::Extended64,
        };
        let world = &c["world"];
        let mut host = Host { world };
        let top = world["leaves"].as_array().unwrap().len() as u32 + 1;
        let flag = c["flag"].as_u64().unwrap() & 0xFF != 0;
        let value = health_fraction(&mut host, top, flag, precision).unwrap();
        let expected = (u128::from(c["sign_exponent"].as_u64().unwrap() as u16) << 64)
            | u128::from(u64::from_str_radix(c["mantissa"].as_str().unwrap(), 16).unwrap());
        let got = match value {
            Some(v) => v.to_x87_extended_bits(),
            None => Some(0xffff_c000_0000_0000_0000),
        };
        assert_eq!(got, Some(expected), "case {i} {c}");
    }
}
