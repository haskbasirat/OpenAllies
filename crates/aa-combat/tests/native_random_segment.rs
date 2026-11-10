//! 0x5853E0 against research/phase8-combat/native-random-segment-fixtures.json.
use aa_combat::{
    Result,
    settlement::{SegmentWorld, random_segment},
};
use serde_json::{Value, json};

struct Adapter<'a> {
    case: &'a Value,
    seed: u32,
    events: Vec<Value>,
}
impl SegmentWorld for Adapter<'_> {
    fn random(&mut self) -> Result<u32> {
        // 0x4F02B0 logs the seed twice, then 0x85D840 draws.
        for log in [1, 2] {
            self.events.push(json!({"log": log, "text": "(int)m_rng.GetSeed()",
                "value": self.seed, "flag": 0}));
        }
        let wide = u64::from(self.seed) * 0x41c6_4e6d + 0x3039;
        self.seed = wide as u32;
        Ok((wide >> 16) as u32)
    }
    fn segment_kind(&self, slot: usize) -> Result<u32> {
        Ok(self.case["kinds"][slot].as_u64().unwrap() as u32)
    }
}

#[test]
fn random_segment_matches_native() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-random-segment-fixtures.json"
    ))
    .unwrap();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 160);
    for case in cases {
        let segments: Vec<u32> = case["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_u64().unwrap() as u32)
            .collect();
        let mut world = Adapter {
            case,
            seed: case["seed"].as_u64().unwrap() as u32,
            events: vec![],
        };
        let picked = random_segment(&segments, &mut world).unwrap();
        assert_eq!(u64::from(picked.unwrap_or(0)), case["result"].as_u64().unwrap(), "{case}");
        assert_eq!(u64::from(world.seed), case["seed_after"].as_u64().unwrap(), "{case}");
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}
