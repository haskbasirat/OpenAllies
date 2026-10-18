//! Replays research/phase8-sai/native-send.json (original
//! KKC_SAICommandData::Send 7AF220 under Unicorn) against
//! aa_mind::sai::command::send.
use aa_mind::{
    Result,
    sai::base_priority::Obj,
    sai::command::{SaiCommand, SaiCommandData, SaiRequest, SendWorld, send},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

struct Host {
    c: Value,
    events: Vec<Value>,
    pending: BTreeMap<String, u64>,
}

fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}
fn opt(v: &Value) -> Option<Obj> {
    let x = u(v);
    (x != 0).then_some(x)
}

impl SendWorld for Host {
    fn resolve(&mut self, gtid: u32) -> Result<Option<Obj>> {
        let entry = &self.c["registry"][(gtid & 0xffff).to_string()];
        if entry.is_null() {
            return Ok(None);
        }
        let (serial, obj) = (u(&entry[0]), u(&entry[1]));
        Ok((serial == gtid >> 16 && obj != 0).then_some(obj))
    }
    fn flags(&mut self, obj: Obj) -> Result<u32> {
        Ok(u(&self.c["objects"][obj.to_string()]["flags"]))
    }
    fn player_is_ai(&mut self, player: Obj) -> Result<bool> {
        Ok(u(&self.c["objects"][player.to_string()]["ai"]) != 0)
    }
    fn sai_record(&mut self, obj: Obj) -> Result<Option<Obj>> {
        Ok(opt(&self.c["objects"][obj.to_string()]["record"]))
    }
    fn set_record_pending(&mut self, record: Obj, pending: bool) -> Result<()> {
        self.pending.insert(record.to_string(), u64::from(pending));
        Ok(())
    }
    fn command_defined(&mut self, ordinal: u32) -> Result<bool> {
        Ok(self.c["defined"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| u(d) == ordinal))
    }
    fn log_non_ai_player(&mut self) -> Result<()> {
        self.events.push(json!({"op": "log", "line": 0xff5}));
        Ok(())
    }
}

#[test]
fn native_send_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-sai/native-send.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    assert!(cases.len() >= 600);
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let o = &row["output"];
        let mut host = Host {
            c: c.clone(),
            events: vec![],
            pending: c["objects"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(_, v)| v["kind"] == "record")
                .map(|(k, v)| (k.clone(), v["pending"].as_u64().unwrap()))
                .collect(),
        };
        let command = if c["command"].is_null() {
            None
        } else {
            let m = &c["command"];
            Some(SaiCommand {
                handle: opt(&m["handle"]),
                ordinal: u(&m["ordinal"]),
                flags: u(&m["flags"]),
                target: u(&m["target"]),
                byte30: u(&m["byte30"]) as u8,
            })
        };
        let mut data = SaiCommandData {
            actors: c["actors"].as_array().unwrap().iter().map(u).collect(),
            player: opt(&c["player"]),
            command,
            ability: opt(&c["ability"]),
        };
        let requests = send(&mut data, &mut host).unwrap();
        for r in &requests {
            let e = match r {
                SaiRequest::FactionUseAbility { player, ability } => {
                    json!({"descriptor": 51, "player": player, "ability": ability})
                }
                SaiRequest::SelectionSet { player, actor } | SaiRequest::SelectionAdd { player, actor } => {
                    json!({"descriptor": r.descriptor(), "player": player, "actor": actor})
                }
                SaiRequest::TellSelectedCommand { player, command } => {
                    let defined = host.command_defined(command.ordinal).unwrap();
                    json!({"descriptor": 84, "player": player, "command": {
                        "handle": command.handle, "definition": defined.then_some(command.ordinal),
                        "flags": command.flags, "target": command.target, "byte30": command.byte30}})
                }
            };
            let mut e = e.as_object().unwrap().clone();
            e.insert("op".into(), json!("request"));
            host.events.push(Value::Object(e));
        }
        assert_eq!(json!(host.events), o["events"], "case {i}");
        assert_eq!(json!(data.command.is_some()), o["command_kept"], "case {i}: +10");
        assert_eq!(json!(data.ability.is_some()), o["ability_kept"], "case {i}: +14");
        assert_eq!(json!(host.pending), o["pending"], "case {i}: record +20");
    }
}
