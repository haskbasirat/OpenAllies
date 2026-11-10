//! 0x5690F0 against research/phase8-combat/native-denizen-slot-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result,
    denizen_slot::{DenizenSlotWorld, slot_health},
};
use aa_numeric::Fast;
use serde_json::Value;

const LIVE: u32 = 0x0001_0040;
const ACTOR: u32 = 0x70;

struct Adapter<'a>(&'a Value);
impl DenizenSlotWorld for Adapter<'_> {
    fn resolve(&self, handle: u32) -> Result<Option<u32>> {
        Ok((handle == LIVE).then_some(ACTOR))
    }
    fn body_health(&self, actor: u32) -> Result<Option<F32>> {
        assert_eq!(actor, ACTOR);
        Ok(self.0["body"]
            .as_bool()
            .unwrap()
            .then(|| F32(self.0["health"].as_u64().unwrap() as u32)))
    }
}

#[test]
fn slot_health_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-denizen-slot-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 80);
    for case in cases {
        let handle = match case["kind"].as_str().unwrap() {
            "live" => LIVE,
            "zero" => 0,
            _ => LIVE + 0x10000,
        };
        let stored = F32(case["stored"].as_u64().unwrap() as u32);
        let value = slot_health(handle, stored, &Adapter(case)).unwrap();
        let native = u128::from(case["result"][0].as_u64().unwrap())
            | (u128::from(case["result"][1].as_u64().unwrap()) << 64);
        assert_eq!(
            Fast::from_f32_bits(value.0).unwrap().to_x87_extended_bits(),
            native,
            "{case}"
        );
    }
}
