//! Replays research/phase8-combat/native-damage-number-fixtures.json (50D0B0).
use aa_body::{F32, damage::DamageRecord};
use aa_combat::{
    Result,
    presentation::{self, DamageNumber, World},
};
use serde_json::{Value, json};

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn record(w: &[u32]) -> DamageRecord {
    DamageRecord {
        source_id: w[0],
        allegiance_id: w[1],
        damage_type: w[2],
        amount: F32(w[3]),
        time: F32(w[4]),
        event_kind: w[5],
        reason: w[6],
    }
}

struct Host(Value);
impl World for Host {
    fn session_mode(&self) -> Result<u32> {
        Ok(u(&self.0["mode"]))
    }
    fn show_damage(&self) -> Result<bool> {
        Ok(self.0["show"] == true)
    }
    fn definition_flags54(&self, _: u32) -> Result<Option<u32>> {
        Ok(self.0["definition"].as_u64().map(|v| v as u32))
    }
    fn actor_id(&self, _: u32) -> Result<u32> {
        Ok(0x30003)
    }
}

#[test]
fn original_damage_number_queue_matches_every_fixture() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-damage-number-fixtures.json"
    ))
    .unwrap();
    for row in doc["cases"].as_array().unwrap() {
        let c = &row["input"];
        let mut queue: Vec<DamageNumber> = (0..u(&c["existing"]))
            .map(|i| {
                let w: Vec<u32> = (0..9).map(|j| 0x1000 * (i + 1) + j).collect();
                DamageNumber {
                    word0: w[0],
                    actor_id: w[1],
                    record: record(&w[2..]),
                }
            })
            .collect();
        let rec: Vec<u32> = c["record"].as_array().unwrap().iter().map(u).collect();
        presentation::queue_damage_number(&mut queue, 1, &record(&rec), &Host(c.clone())).unwrap();
        let entries: Vec<Value> = queue
            .iter()
            .map(|e| {
                let mut w = vec![e.word0, e.actor_id];
                w.extend(e.record.words());
                json!(w)
            })
            .collect();
        assert_eq!(json!(entries), row["output"]["entries"], "{}", c["name"]);
    }
}
