//! Replays research/phase8-combat/native-performer-context.json (original
//! 57E360 under Unicorn) against aa_combat::performer_context::fill: every
//! context word, the untouched marker words included.
use aa_combat::{
    Result,
    performer_context::{PerformerContextWorld, fill},
};
use serde_json::{Value, json};

const TARGET: u32 = 1;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-combat/native-performer-context.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}

struct Host<'a>(&'a Value);
impl PerformerContextWorld for Host<'_> {
    fn actor(&mut self) -> Result<u32> {
        Ok(0x00aa_00aa)
    }
    fn target_gtid(&mut self) -> Result<u32> {
        Ok(u(&self.0["gtid"]))
    }
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        let c = self.0;
        let generation = if gtid & 0xffff == 0x51 { u(&c["generation"]) } else { 0 };
        let entry = gtid & 0xffff == 0x51 && c["entry"].as_bool().unwrap();
        Ok((generation == gtid >> 16 && entry).then_some(TARGET))
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        assert_eq!(object, TARGET);
        Ok(pair(&self.0["target_point"]))
    }
    fn task_point(&mut self) -> Result<[u32; 2]> {
        Ok(pair(&self.0["task_point"]))
    }
}

#[test]
fn performer_context_matches_native() {
    let d = fixture();
    let cases = d["cases"].as_array().unwrap();
    assert!(cases.len() >= 120);
    let mut targeted = 0;
    for case in cases {
        let c = &case["input"];
        let name = c["name"].as_str().unwrap();
        let w = fill(&mut Host(c)).unwrap();
        let mut words: Vec<u32> = (0..12).map(|k| 0xc000_0000 + k).collect();
        words[1] = w.word4;
        words[2] = w.actor;
        words[7] = w.target;
        words[8] = w.point[0];
        words[9] = w.point[1];
        words[10] = w.word28;
        assert_eq!(json!(words), case["output"]["context"], "{name}");
        targeted += usize::from(w.target != 0);
    }
    assert!(targeted > 20, "{targeted}");
}
