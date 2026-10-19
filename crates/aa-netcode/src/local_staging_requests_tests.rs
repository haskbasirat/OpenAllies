//! Replays research/development10/custom-game/native-explicit-ai.json: the
//! original 0x754F50 / 0x777F00 / 0x777D90 / 0x777700 / 0x771880 over a
//! laboratory local-host Session, step by step, against [`super::process`].
use super::*;
use serde_json::Value;

const NATIONS: [&str; 4] = ["usa", "germany", "britain", "russia"];
const FACTIONS: [&str; 4] = ["infantry", "armor", "airborne", "artillery"];
const HANDICAPS: [&str; 4] = ["easy", "normal", "hard", "expert"];

fn units(v: &Value) -> Vec<u16> {
    v.as_array().unwrap().iter().map(|u| u.as_u64().unwrap() as u16).collect()
}
fn text(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn def(v: &Value, names: &[&str]) -> Vec<u16> {
    v.as_u64().map(|i| text(names[i as usize])).unwrap_or_default()
}
fn link(v: &Value) -> PeerLink {
    match v.as_str() {
        Some("Node") => PeerLink::Node,
        Some("Null") => PeerLink::Null,
        _ => panic!("laboratory link {v}"),
    }
}
fn row(k: &Value) -> (Vec<u16>, Row) {
    (
        units(&k["name"]),
        Row {
            playable_28: k["flag_28"].as_bool().unwrap(),
            nation_18: def(&k["nation"], &NATIONS),
            faction_1c: def(&k["faction"], &FACTIONS),
            handicap_24: def(&k["ai_handicap"], &HANDICAPS),
        },
    )
}
fn player(p: &Value) -> LabPlayer {
    LabPlayer {
        link: Some(link(&p["link"])),
        id: p["id"].as_u64().unwrap() as u32,
        name: units(&p["name"]),
        ai: p["ai"].as_bool().unwrap(),
        implicit: p["implicit"].as_bool().unwrap(),
        text_14: units(&p["text_14"]),
        kingdom: units(&p["kingdom"]),
        flag_2c: p["flag_2c"].as_bool().unwrap(),
        flag_38: p["flag_38"].as_bool().unwrap(),
        field_3c: p["field_3c"].as_u64().unwrap() as u32,
        field_40: p["field_40"].as_u64().unwrap() as u32,
        text_30: units(&p["text_30"]),
        text_34: units(&p["text_34"]),
    }
}

#[derive(Default)]
struct Log {
    messages: Vec<(String, bool)>,
    null_ordering: usize,
    created: Vec<(u32, bool, bool)>,
}
impl Effects for Log {
    fn session_message_76da60(&mut self, key: &str, shown: bool) -> Result<()> {
        self.messages.push((key.to_owned(), shown));
        Ok(())
    }
    fn ordering_player_null(&mut self) -> Result<()> {
        self.null_ordering += 1;
        Ok(())
    }
    fn creating_player(&mut self, id: u32, ai: bool, implicit: bool) -> Result<()> {
        self.created.push((id, ai, implicit));
        Ok(())
    }
    fn check_nation(&mut self, _: &[u16], _: &[u16]) -> Result<()> {
        Ok(())
    }
    fn check_faction(&mut self, _: &[u16], _: &[u16]) -> Result<()> {
        Ok(())
    }
}

fn request(op: &Value) -> StagingRequest {
    let a = op.as_array().unwrap();
    let k = text(a[1].as_str().unwrap());
    let d = |names: &[&str]| a[2].as_u64().map(|i| text(names[i as usize]));
    match a[0].as_str().unwrap() {
        "ai" => StagingRequest::CreateExplicitAi { kingdom: k, profile: text(a[2].as_str().unwrap()), name: text(a[3].as_str().unwrap()) },
        "nation" => StagingRequest::SetKingdomNation { kingdom: k, nation: d(&NATIONS) },
        "faction" => StagingRequest::SetKingdomFaction { kingdom: k, faction: d(&FACTIONS) },
        "handicap" => StagingRequest::SetAiHandicap { kingdom: k, handicap: d(&HANDICAPS) },
        other => panic!("{other}"),
    }
}

#[test]
fn staging_orders_replay_native_rows() {
    let doc: Value =
        serde_json::from_str(include_str!("../../../research/development10/custom-game/native-explicit-ai.json")).unwrap();
    let mut steps = 0;
    for case in doc["cases"].as_array().unwrap() {
        let s = &case["session"];
        let current = s["launch"]["map_scenario"]["header"]["kingdoms"].as_array().unwrap();
        let base = s["launch"]["game"]["header"]["kingdoms"].as_array().unwrap();
        let mut lab = Laboratory {
            countdown_118: case["countdown"].as_i64().unwrap() as i32,
            ordering_284: case["ordering"].as_u64().map(|v| v as u32),
            local_8: case["local"].as_u64().map(|v| v as u32),
            last_player_id_e8: s["last_player_id"].as_u64().unwrap() as u32,
            created_c4: s["created"].as_u64().map(|v| v as u32),
            changed_c8: s["changed"].as_bool().unwrap(),
            players: s["players"].as_array().unwrap().iter().map(player).collect(),
            rows: current.iter().map(|k| {
                let (id, r) = row(k);
                (id, r, false)
            }).collect(),
            base: base.iter().map(row).collect(),
            nations: NATIONS.iter().map(|n| text(n)).collect(),
            factions: FACTIONS.iter().map(|n| text(n)).collect(),
            nation_factions: vec![
                (text("usa"), vec![text("infantry"), text("armor")]),
                (text("germany"), vec![text("armor"), text("airborne"), text("artillery")]),
                (text("britain"), vec![]),
                (text("russia"), vec![text("artillery")]),
            ],
            duplicate_format: text("%s (%d)"),
        };
        for step in case["steps"].as_array().unwrap() {
            let name = format!("{} {}", case["name"], step["op"]);
            assert_eq!(step["status"], "ok", "{name}");
            let mut log = Log::default();
            process(&mut lab, &request(&step["op"]), &mut log).unwrap_or_else(|e| panic!("{name}: {e}"));
            let want: Vec<LabPlayer> = step["players"].as_array().unwrap().iter().map(player).collect();
            assert_eq!(lab.players, want, "{name} players");
            let rows: Vec<(Vec<u16>, Row)> = step["kingdoms"].as_array().unwrap().iter().map(row).collect();
            let got: Vec<(Vec<u16>, Row)> = lab.rows.iter().map(|(i, r, _)| (i.clone(), r.clone())).collect();
            assert_eq!(got, rows, "{name} rows");
            let modified: Vec<bool> = step["modified_68"].as_array().unwrap().iter().map(|v| v.as_bool().unwrap()).collect();
            assert_eq!(lab.rows.iter().map(|r| r.2).collect::<Vec<_>>(), modified, "{name} +68");
            assert_eq!(u64::from(lab.last_player_id_e8), step["last_player_id"].as_u64().unwrap(), "{name} E8");
            assert_eq!(lab.created_c4.map(u64::from), step["created"].as_u64(), "{name} C4");
            assert_eq!(lab.changed_c8, step["changed"].as_bool().unwrap(), "{name} C8");
            // 0x76DA60: shown messages localize their key and post kind 0x11
            let shown: Vec<String> = step["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["event"] == "ui_message")
                .map(|e| {
                    assert_eq!(e["kind"], 0x11, "{name}");
                    String::from_utf16(&units(&e["text"])).unwrap()
                })
                .collect();
            let formats = [
                (COUNTING_DOWN_KEY, "counting down"),
                (ORDER_FAILED_KEY, "order failed"),
                (PLAYER_KINGDOM_FAILED_KEY, PLAYER_KINGDOM_FAILED_KEY),
                (NATION_FAILED_KEY, NATION_FAILED_KEY),
                (FACTION_FAILED_KEY, FACTION_FAILED_KEY),
            ];
            let mine: Vec<String> = log
                .messages
                .iter()
                .filter(|m| m.1)
                .map(|(k, _)| formats.iter().find(|f| f.0 == k).map(|f| f.1.to_owned()).unwrap_or_else(|| k.clone()))
                .collect();
            assert_eq!(mine, shown, "{name} messages");
            steps += 1;
        }
    }
    assert!(steps >= 60, "{steps}");
}
