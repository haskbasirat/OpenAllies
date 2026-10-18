use super::*;
use serde_json::{Value, json};
struct Replay {
    input: Value,
    events: Vec<Value>,
    notifications: Vec<Value>,
    flags: [bool; 4],
    s: Synchronizer,
}
impl Replay {
    fn event(&mut self, kind: &str, fields: Value) -> Result<()> {
        if self.input["stop"].as_str() == Some(kind) {
            return Err(missing(kind));
        }
        let mut value = json!({"kind":kind});
        for (k, v) in fields.as_object().unwrap() {
            value[k] = v.clone();
        }
        self.events.push(value);
        Ok(())
    }
}
impl World for Replay {
    fn diagnostic(&mut self) -> Result<()> {
        self.event("diagnostic", json!({}))
    }
    fn notify(&mut self) -> Result<()> {
        self.event("notify", json!({}))?;
        //The native observer records7851C0's argument before application3's
        //guard. This compares construction; it does not assert queueacceptance.
        self.notifications.push(json!(Notification::native_kind(9)));
        Ok(())
    }
    fn state(&mut self) -> Result<u32> {
        Ok(self.input["state"].as_u64().unwrap() as u32)
    }
    fn unfreeze(&mut self) -> Result<()> {
        self.event("unfreeze", json!({}))
    }
    fn flag(&mut self, offset: u32) -> Result<bool> {
        Ok(self.flags[match offset {
            0x16a => 0,
            0x16b => 1,
            0x16d => 2,
            0x16e => 3,
            _ => panic!("flag"),
        }])
    }
    fn store_flag(&mut self, offset: u32, value: bool) -> Result<()> {
        self.flags[match offset {
            0x16a => 0,
            0x16b => 1,
            0x16d => 2,
            0x16e => 3,
            _ => panic!("flag"),
        }] = value;
        Ok(())
    }
    fn restart_recording(&mut self) -> Result<()> {
        self.event("restart", json!({}))
    }
    fn begin_diagnostic(&mut self) -> Result<()> {
        self.event("begin", json!({}))
    }
    fn human_opposition(&mut self) -> Result<bool> {
        self.event("opposition", json!({}))?;
        Ok(self.input["opposition"].as_bool().unwrap())
    }
    fn opposition_diagnostic(&mut self, value: bool) -> Result<()> {
        self.event("opposition_diagnostic", json!({"value":value}))
    }
    fn update_game_spy(&mut self) -> Result<()> {
        self.event("game_spy", json!({}))
    }
    fn flush_setup_ai(&mut self) -> Result<()> {
        self.event("setup", json!({}))
    }
    fn synchronize_svars(&mut self) -> Result<()> {
        self.event("svars", json!({}))
    }
    fn synchronize_world(&mut self) -> Result<()> {
        self.event("world", json!({"label":"g_gworld"}))
    }
    fn reset_s(&mut self) -> Result<()> {
        self.event("reset_s", json!({"index":0,"hash":0}))?;
        self.s.reset(0, 0);
        Ok(())
    }
}
#[test]
fn original_v14_all_tasks_success_order_and_partial_flags() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/world-ready-7757f0/native-all-tasks-complete.json"
    ))
    .unwrap();
    assert_eq!(fixture["passed"], true);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 540);
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        let mut replay = Replay {
            input: input.clone(),
            events: vec![],
            notifications: vec![],
            s: Synchronizer::new(2),
            flags: [
                input["flag_a"].as_bool().unwrap(),
                input["flag_b"].as_bool().unwrap(),
                true,
                true,
            ],
        };
        assert_eq!(run(&mut replay).is_err(), !row["stop"].is_null(), "{row}");
        assert_eq!(json!(replay.events), row["events"], "{row}");
        assert_eq!(json!(replay.notifications), row["notifications"], "{row}");
        assert_eq!(
            json!({"16a":replay.flags[0],"16b":replay.flags[1],"16d":replay.flags[2],"16e":replay.flags[3]}),
            row["flags"],
            "{row}"
        );
        assert_eq!(
            json!({"enabled":replay.s.enabled,"dump":replay.s.file_dump,"suspend":replay.s.suspend as u32,
            "hash":replay.s.hash as u32,"begin":replay.s.begin()as u32,"end":replay.s.end()as u32}),
            row["s"],
            "{row}"
        );
    }
}
struct Opposite {
    input: Value,
    getters: Vec<usize>,
}
impl OppositionFields for Opposite {
    type Kingdom = usize;
    type Team = u32;
    fn count(&mut self) -> usize {
        self.input["players"].as_array().unwrap().len()
    }
    fn ai(&mut self, index: usize) -> Result<bool> {
        Ok(self.input["players"][index]["ai"].as_bool().unwrap())
    }
    fn kingdom(&mut self, index: usize) -> Result<Option<usize>> {
        if self.input["stop_getter"].as_u64() == Some(self.getters.len() as u64 + 1) {
            return Err(missing("getter"));
        }
        self.getters.push(index);
        Ok(if self.input["world"] == true {
            self.input["players"][index]["kingdom"]
                .as_u64()
                .map(|v| v as usize)
        } else {
            None
        })
    }
    fn result(&mut self, id: usize) -> Result<u32> {
        Ok(self.input["kingdoms"][id]["result"].as_u64().unwrap() as u32)
    }
    fn team(&mut self, id: usize) -> Result<Option<u32>> {
        let value = self.input["kingdoms"][id]["team"].as_u64().unwrap() as u32;
        Ok((value != 0).then_some(value))
    }
}
#[test]
fn native_human_opposition_preserves_laziness_and_getter_order() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/world-ready-7757f0/native-human-opposition.json"
    ))
    .unwrap();
    assert_eq!(fixture["passed"], true);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 816);
    for row in fixture["cases"].as_array().unwrap() {
        let mut fields = Opposite {
            input: row["input"].clone(),
            getters: vec![],
        };
        let result = opposition(&mut fields);
        assert_eq!(result.is_err(), !row["stop"].is_null(), "{row}");
        assert_eq!(json!(result.ok()), row["result"], "{row}");
        assert_eq!(json!(fields.getters), row["getters"], "{row}");
    }
}

struct SetupReplay {
    stop: Option<usize>,
    emits: Vec<Value>,
}
impl SetupTransport for SetupReplay {
    fn emit(&mut self, fields: &[Field]) -> Result<()> {
        if self.stop == Some(self.emits.len() + 1) {
            return Err(missing("emit"));
        }
        let [
            Field::Text(a),
            Field::Text(b),
            Field::Text(c),
            Field::Uint {
                width: 1,
                value: flag,
            },
        ] = fields
        else {
            panic!("native773480 fields")
        };
        self.emits
            .push(json!({"texts":[a,b,c],"flag":flag,"player_argument":0}));
        Ok(())
    }
}
#[test]
fn actual_constructor_queue_7747f0_clears_only_after_all_requests() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/development10/world-ready-7757f0/native-setup-ai-flush.json"
    ))
    .unwrap();
    assert_eq!(fixture["passed"], true);
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 42);
    for row in fixture["cases"].as_array().unwrap() {
        let input = &row["input"];
        //A real retained Session constructor and its actual queue producer;
        //only the76CA70 transport response is a declared laboratory seam.
        let mut owner = LocalBootstrap::from_native_constructors();
        for values in input["entries"].as_array().unwrap() {
            let texts: Vec<Vec<u16>> = values
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().encode_utf16().collect())
                .collect();
            owner
                .queue_setup_ai_778f20(&texts[0], &texts[1], &texts[2])
                .unwrap();
        }
        let mut target = SetupReplay {
            stop: input["stop_emit"].as_u64().map(|v| v as usize),
            emits: vec![],
        };
        assert_eq!(
            flush_setup(&mut owner.session.setup_ai, &mut target).is_err(),
            !row["stop"].is_null(),
            "{row}"
        );
        assert_eq!(json!(target.emits), row["emits"], "{row}");
        assert_eq!(
            owner.setup_ai_entries().len(),
            row["count"].as_u64().unwrap() as usize,
            "{row}"
        );
    }
}
