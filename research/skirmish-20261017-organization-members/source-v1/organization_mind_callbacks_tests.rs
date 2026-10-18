use super::*;
use serde_json::{Value, json};
struct Probe {
    status: u32,
    word50: u32,
    wordb8: u32,
    trace: Vec<Value>,
    fail: Option<u32>,
}
impl Fields for Probe {
    fn embedded_status(&mut self, _: u32) -> Result<u32> {
        self.trace.push(json!(["status", self.status]));
        Ok(self.status)
    }
    fn move_speed_changed(&mut self, old: u32, new: u32) -> Result<()> {
        self.trace.push(json!(["move", old, new]));
        Err("required actual move entry".into())
    }
    fn put_word(&mut self, offset: u32, word: u32) -> Result<()> {
        if self.fail == Some(offset) {
            return Err("explicit storage failure probe".into());
        }
        self.trace.push(json!(["put", offset, word]));
        match offset {
            0x50 => self.word50 = word,
            0xb8 => self.wordb8 = word,
            _ => panic!(),
        };
        Ok(())
    }
}
#[test]
fn native_mind114_118_current_status_and_store_prefixes() {
    let fixture: Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261017-organization-members/native-mind-callbacks.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 144);
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let n = &row["native"];
        let kind = match c["kind"].as_str().unwrap() {
            "base" => Kind::Base,
            "element" => Kind::Element,
            "regiment" => Kind::Regiment,
            "aircraft" => Kind::Aircraft,
            _ => panic!(),
        };
        let mut p = Probe {
            status: c["status"].as_u64().unwrap() as u32,
            word50: c["word50"].as_u64().unwrap() as u32,
            wordb8: c["wordb8"].as_u64().unwrap() as u32,
            trace: vec![],
            fail: None,
        };
        let r = if c["operation"] == "speed" {
            speed_changed(
                kind,
                &mut p,
                c["old"].as_u64().unwrap() as u32,
                c["new"].as_u64().unwrap() as u32,
            )
        } else {
            leader_changed(kind, &mut p)
        };
        assert_eq!(json!(p.trace), n["trace"], "{row}");
        assert_eq!(json!(p.word50), n["word50"], "{row}");
        assert_eq!(json!(p.wordb8), n["wordb8"], "{row}");
        assert_eq!(r.is_err(), n["stop"] == "move", "{row}");
    }
}
#[test]
fn leader_second_storage_error_retains_first_store() {
    let mut p = Probe {
        status: 0,
        word50: 7,
        wordb8: 9,
        trace: vec![],
        fail: Some(0xb8),
    };
    assert!(leader_changed(Kind::Regiment, &mut p).is_err());
    assert_eq!(p.word50, 0);
    assert_eq!(p.wordb8, 9);
    assert_eq!(p.trace, vec![json!(["put", 0x50, 0])]);
}
#[test]
fn native_current_mind_position_predicates() {
    let fixture: Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261017-organization-members/native-mind-predicates.json"
    ))
    .unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(), 144);
    for row in fixture["rows"].as_array().unwrap() {
        let c = &row["input"];
        let n = &row["native"];
        let kind = match c["kind"].as_str().unwrap() {
            "base" => Kind::Base,
            "element" => Kind::Element,
            "regiment" => Kind::Regiment,
            "aircraft" => Kind::Aircraft,
            _ => panic!(),
        };
        let mut p = Probe {
            status: c["status"].as_u64().unwrap() as u32,
            word50: 1,
            wordb8: 2,
            trace: vec![],
            fail: None,
        };
        let result = position_active(kind, c["slot"].as_u64().unwrap() as u32, &mut p);
        let trace = n["trace"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v[0] != "purecall")
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(p.trace, trace, "{row}");
        if n["stop"] == "purecall" {
            assert!(result.unwrap_err().contains("purecall96CAC2"), "{row}")
        } else {
            assert_eq!(json!(result.unwrap()), n["value"], "{row}")
        }
    }
}
