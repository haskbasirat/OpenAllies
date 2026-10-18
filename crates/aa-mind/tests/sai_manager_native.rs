//! Replays research/phase8-sai/native-manager.json (original 7A0AA0,
//! 85D8A0/85D840, 7A0080, 79FF10 and 7A9A60/7B8E70 under Unicorn) against
//! aa_mind::sai::manager.
use aa_mind::{
    Result,
    sai::command::SaiCommandData,
    sai::manager::{Fiber, Sai, SaiRng, ThinkGlobals, ThinkHost, think, white_fog_tile_hidden},
};
use aa_navigation::sai_state::{Kingdom, RegionKingdom, RegionState, State};
use aa_navigation::startup::Object;
use aa_numeric::Precision;
use serde_json::{Map, Value, json};
use std::collections::VecDeque;

fn u(v: &Value) -> u32 {
    v.as_i64().unwrap() as u32
}

struct Host {
    expected: VecDeque<Value>,
    case: usize,
}

impl Host {
    fn next(&mut self, op: &str, args: Value) -> Value {
        let e = self
            .expected
            .pop_front()
            .unwrap_or_else(|| panic!("case {}: unexpected {op} {args}", self.case));
        let mut exp: Map<String, Value> = e.as_object().unwrap().clone();
        let result = exp.remove("result").unwrap_or(Value::Null);
        let mut act: Map<String, Value> = args.as_object().unwrap().clone();
        act.insert("op".into(), json!(op));
        assert_eq!(Value::Object(exp), Value::Object(act), "case {}", self.case);
        result
    }
}

fn fiber(f: Fiber) -> &'static str {
    match f {
        Fiber::GoalEngine => "goal_engine",
        Fiber::Rai => "rai",
        Fiber::Metagame => "metagame",
    }
}

impl ThinkHost for Host {
    fn session_mode(&mut self) -> Result<u32> {
        Ok(u(&self.next("session_mode", json!({}))))
    }
    fn lock(&mut self) -> Result<()> {
        self.next("lock", json!({}));
        Ok(())
    }
    fn unlock(&mut self) -> Result<()> {
        self.next("unlock", json!({}));
        Ok(())
    }
    fn now(&mut self) -> Result<u32> {
        Ok(u(&self.next("now", json!({}))))
    }
    fn delete_fiber(&mut self, f: Fiber) -> Result<()> {
        self.next("delete_fiber", json!({"fiber": fiber(f)}));
        Ok(())
    }
    fn create_fiber(&mut self, f: Fiber) -> Result<()> {
        self.next("create_fiber", json!({"fiber": fiber(f)}));
        Ok(())
    }
    fn convert_thread(&mut self) -> Result<bool> {
        Ok(self.next("convert_thread", json!({})).as_bool().unwrap())
    }
    fn log_convert_failure(&mut self) -> Result<()> {
        self.next("log", json!({}));
        Ok(())
    }
    fn run(&mut self, f: Fiber, deadline: u32) -> Result<()> {
        self.next("run", json!({"fiber": fiber(f), "deadline": deadline}));
        Ok(())
    }
    fn send(&mut self, _data: SaiCommandData) -> Result<()> {
        self.next("send", json!({}));
        Ok(())
    }
    fn metagame_post(&mut self) -> Result<()> {
        self.next("metagame_post", json!({}));
        Ok(())
    }
}

fn data() -> SaiCommandData {
    SaiCommandData {
        actors: vec![],
        player: None,
        command: None,
        ability: None,
    }
}

#[test]
fn native_manager_matches_original() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-sai/native-manager.json"
    );
    let f: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let cases = f["cases"].as_array().unwrap();
    let mut seen = [0_usize; 5];
    for (i, row) in cases.iter().enumerate() {
        let c = &row["input"];
        let o = &row["output"];
        assert!(!o["fault"].as_bool().unwrap(), "case {i}");
        let p: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
        match c["entry"].as_str().unwrap() {
            "ctor" => {
                seen[0] += 1;
                let sai = Sai::new(u(&c["ticks"]));
                assert_eq!(sai.rng.state, u(&o["rng"]), "case {i}: +2C");
                let w = &o["words"];
                // Uninitialized by the constructor (heap fill kept).
                assert_eq!(u(&w["24"]), 0xcdcd_cdcd);
                assert_eq!(u(&w["48"]), 0xcdcd_cdcd);
                assert!(sai.revision.is_none());
                for off in ["28", "30", "34", "38", "3c", "40", "44", "4c", "50", "54", "58", "64", "78", "7c", "80", "84", "88", "8c", "94"] {
                    assert_eq!(u(&w[off]), 0, "case {i}: +{off}");
                }
                assert_eq!(u(&w["6c"]), sai.word_6c);
                assert_eq!(u(&w["70"]), sai.word_70);
                assert_eq!((u(&w["68"]), u(&w["98"])), (0x10, 10), "reserved capacities");
                for (_, b) in o["bytes"].as_object().unwrap() {
                    assert_eq!(b.as_u64(), Some(0));
                }
                assert!(!sai.ready && sai.goal_deadline == 0 && sai.rai_deadline == 0);
                assert!(sai.players.is_empty() && sai.commands.is_empty());
                assert!(o["singleton"].as_bool().unwrap());
            }
            "rng" => {
                seen[1] += 1;
                let mut rng = SaiRng::from_seed(u(&c["ticks"]));
                let draws: Vec<u32> = (0..o["draws"].as_array().unwrap().len())
                    .map(|_| rng.next_u32())
                    .collect();
                assert_eq!(json!(draws), o["draws"], "case {i}");
                assert_eq!(rng.state, u(&o["state"]), "case {i}");
            }
            "think" => {
                seen[2] += 1;
                let s = &c["sai"];
                let b = |k: &str| s[k].as_bool().unwrap();
                let mut sai = Sai::new(0);
                sai.ready = u(&s["ready"]) != 0;
                sai.revision = Some(u(&s["revision"]));
                sai.metagame = b("metagame");
                sai.goal_fiber = b("goal_fiber");
                sai.rai_fiber = b("rai_fiber");
                sai.metagame_fiber = b("metagame_fiber");
                sai.main_fiber = b("main_fiber");
                sai.goal_deadline = u(&s["deadlines"][0]);
                sai.rai_deadline = u(&s["deadlines"][1]);
                sai.metagame_deadline = u(&s["deadlines"][2]);
                sai.players = vec![Some(1); u(&s["players"]) as usize];
                sai.commands = s["queue"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|d| (u(d), data()))
                    .collect();
                let g = &c["globals"];
                let globals = ThinkGlobals {
                    mode7_enabled: g["mode7_enabled"].as_bool().unwrap(),
                    world: (!g["world"].is_null())
                        .then(|| (g["world"][0].as_i64().unwrap() as i32, u(&g["world"][1]) != 0)),
                    map: (!g["map"].is_null()).then(|| (u(&g["map"][0]) != 0, u(&g["map"][1]) != 0)),
                    session_e4: u(&g["session_e4"]),
                    revision: u(&g["revision"]),
                    paused: g["paused"].as_bool().unwrap(),
                };
                let mut host = Host {
                    expected: o["events"].as_array().unwrap().iter().cloned().collect(),
                    case: i,
                };
                think(&mut sai, u(&c["budget"]), &globals, &mut host, p).unwrap();
                assert!(host.expected.is_empty(), "case {i}: unconsumed {:?}", host.expected);
                let a = &o["sai"];
                assert_eq!(sai.revision, Some(u(&a["revision"])), "case {i}");
                assert_eq!(sai.metagame, a["metagame"].as_bool().unwrap(), "case {i}");
                assert_eq!(
                    json!([sai.goal_fiber, sai.rai_fiber, sai.metagame_fiber, sai.main_fiber]),
                    a["fibers"],
                    "case {i}: fibers"
                );
                assert_eq!(
                    json!([sai.goal_deadline, sai.rai_deadline, sai.metagame_deadline]),
                    a["deadlines"],
                    "case {i}: deadlines"
                );
                let left: Vec<u32> = sai.commands.iter().map(|(d, _)| *d).collect();
                assert_eq!(json!(left), a["queue"], "case {i}: queue");
            }
            "reset" => {
                seen[3] += 1;
                let mut sai = Sai::new(0);
                sai.ready = true;
                sai.players = c["players"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| Some(u(p)).filter(|x| *x != 0))
                    .collect();
                sai.goal_fiber = c["goal_fiber"].as_bool().unwrap();
                sai.rai_fiber = c["rai_fiber"].as_bool().unwrap();
                sai.metagame_fiber = true;
                sai.general_powers = c["general_powers"].as_array().unwrap().iter().map(u).collect();
                let mut order = vec![];
                sai.reset(|p| {
                    order.push(p);
                    Ok(())
                })
                .unwrap();
                let expected: Vec<u32> = o["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|e| e["op"] == "reset_player")
                    .map(|e| u(&e["player"]))
                    .collect();
                assert_eq!(order, expected, "case {i}: 7B1760 order");
                let after = &o["after"];
                assert_eq!(u8::from(sai.ready), after["ready"].as_u64().unwrap() as u8);
                assert_eq!(sai.goal_fiber, u(&after["goal_fiber"]) != 0);
                assert_eq!(sai.rai_fiber, u(&after["rai_fiber"]) != 0);
                assert_eq!(sai.metagame_fiber, u(&after["metagame_fiber"]) != 0, "MGAI fiber kept");
                assert_eq!(sai.general_powers.is_empty(), u(&after["gp_head"]) == 0);
                let released: Vec<u32> = o["events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|e| e["op"] == "release")
                    .map(|e| u(&e["gp"]))
                    .collect();
                assert_eq!(json!(released), c["general_powers"], "case {i}: released in list order");
            }
            "fog" => {
                seen[4] += 1;
                let mut state = State { regions: vec![] };
                let mut areas = vec![];
                for (ri, rg) in c["regions"].as_array().unwrap().iter().enumerate() {
                    areas.push(u(&rg["area"]) as i32);
                    state.regions.push(RegionState {
                        kingdoms: rg["records"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|r| {
                                (!r.is_null()).then(|| RegionKingdom {
                                    region: ri as u32,
                                    kingdom: Object(0),
                                    explored: 0,
                                    occupied: 0,
                                    visible: u(&r["visible"]),
                                    word14: u(&r["timer"]),
                                    seen_records: vec![],
                                })
                            })
                            .collect(),
                    });
                }
                let tile_region = (!c["tile_region"].is_null())
                    .then(|| c["tile_region"].as_i64().unwrap() as i32);
                let kingdom = Kingdom {
                    object: Object(1),
                    native_id: 0,
                    major_index: c["kingdom"].as_i64().unwrap() as i32,
                    category: 0,
                };
                white_fog_tile_hidden(
                    &mut state,
                    tile_region,
                    kingdom,
                    &areas,
                    u(&c["percent"]),
                    u(&c["time"]),
                    p,
                )
                .unwrap();
                for (key, v) in o["records"].as_object().unwrap() {
                    let (r, k) = key.split_once(':').unwrap();
                    let rec = state.regions[r.parse::<usize>().unwrap()].kingdoms
                        [k.parse::<usize>().unwrap()]
                    .as_ref()
                    .unwrap();
                    assert_eq!(json!([rec.visible, rec.word14]), *v, "case {i}: record {key}");
                }
            }
            other => panic!("{other}"),
        }
    }
    assert!(seen.iter().all(|n| *n >= 100), "{seen:?}");
}
