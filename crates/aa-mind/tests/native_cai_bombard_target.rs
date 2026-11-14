//! Replays research/phase8-ai/native-cai-bombard-target.json (original
//! 527A60 under Unicorn) against aa_mind::cai_thinks::bombard_activate::
//! set_target. The release of the original's temporary name CString is a
//! string-lifetime detail: its `free` event is checked against the
//! temporary's reference count and not replayed.
use aa_mind::{
    Result, StateId,
    cai_thinks::bombard_activate::{BombardTargetWorld, set_target},
};
use serde_json::{Value, json};

const TARGET: u32 = 3;

fn fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-ai/native-cai-bombard-target.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

struct Host {
    c: Value,
    events: Vec<Value>,
    gtid: u32,
}
impl BombardTargetWorld for Host {
    fn set_bombard_target_gtid(&mut self, _state: StateId, gtid: u32) -> Result<()> {
        self.gtid = gtid;
        Ok(())
    }
    fn target_gtid(&mut self, object: u32) -> Result<u32> {
        assert_eq!(object, TARGET);
        Ok(self.c["target"]["gtid"].as_u64().unwrap() as u32)
    }
    fn object_name(&mut self, object: u32) -> Result<String> {
        assert_eq!(object, TARGET);
        self.events.push(json!({"event": "object_name"}));
        Ok(self.c["target"]["name"].as_str().unwrap().to_string())
    }
    fn set_bombard_label(&mut self, _state: StateId, label: Option<String>) -> Result<()> {
        self.events.push(match label {
            Some(text) => json!({"event": "assign_label", "text": text}),
            None => json!({"event": "empty_label"}),
        });
        Ok(())
    }
}

#[test]
fn native_cai_bombard_target_matches_original() {
    let f = fixture();
    assert_eq!(f["coverage"]["527A60"], json!([46, 46]));
    let cases = f["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 60);
    for row in cases {
        let c = &row["input"];
        let o = &row["output"];
        let name = &c["name"];
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            gtid: c["old_gtid"].as_u64().unwrap() as u32,
        };
        let target = (!c["target"].is_null()).then_some(TARGET);
        set_target(StateId(5), target, &mut host).unwrap();
        let native: Vec<Value> = o["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "free")
            .cloned()
            .collect();
        let freed = o["events"].as_array().unwrap().len() - native.len();
        let last_reference = target.is_some() && c["temp_refs"] == json!(1);
        assert_eq!(freed, usize::from(last_reference), "{name}");
        assert_eq!(json!(host.events), json!(native), "{name}");
        assert_eq!(json!(host.gtid), o["gtid"], "{name}");
    }
}
