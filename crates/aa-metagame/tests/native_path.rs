//! Replays research/phase8-metagame/native-path.json: the original metagame
//! pathfinder under Unicorn (generic search 961550/961350/960FB0 with node
//! store 961230 and heap 53E7B0, moves used 93EAE0, army pathfinder 93FDC0 ->
//! 93FCC0 / 93FAD0) on the InitCreate worlds of every shipped campaign after
//! original owner changes (955620) and army relocations (940100), each query
//! at x87 PC24, PC53 and PC64, plus the heuristic 960E70 (ST0, 80 bits).
//! The last phases of every campaign replace the definition locations
//! (fixture op `locations`, patched into `MetaGameData` here as natively).
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::data::{MetaGameData, TerritoryKind};
use aa_metagame::path::{PathGraph, SEARCH_DEPTH_LIMIT, SEARCH_FAILED, SEARCH_FOUND, UNLIMITED};
use aa_metagame::state::{ArmyId, MgState};
use aa_numeric::Precision;
use serde_json::Value;
use std::collections::BTreeMap;

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../../research/phase8-metagame/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

/// vtable+0x140 from the fixture matrix; +0x138 team based (as mg_campaign).
struct Relations {
    enemy: Vec<Vec<bool>>,
    team: Vec<Option<usize>>,
}
impl KingdomRelations for Relations {
    fn enemy(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.enemy[s.0 as usize][o.0 as usize]
    }
    fn relation_138(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.team[s.0 as usize] == self.team[o.0 as usize]
    }
}

fn usize_list(v: &Value) -> Vec<usize> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect()
}

/// PC53 first: `aa_metagame::GAME_PRECISION` (metagame thread, lead policy).
const PRECISIONS: [(&str, Precision); 3] = [
    ("pc53", Precision::Double53),
    ("pc24", Precision::Single24),
    ("pc64", Precision::Extended64),
];

#[test]
fn pathfinder_matches_native() {
    let data_bytes = read("metagame-data.json");
    let shipped: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("native-path.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    let mut seen: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut count = |k: &'static str| *seen.entry(k).or_default() += 1;
    let sequences = fx["sequences"].as_array().unwrap();
    assert_eq!(sequences.len(), shipped.campaigns.len());
    for seq in sequences {
        let ci = seq["campaign"].as_u64().unwrap() as usize;
        let mut data = shipped.clone();
        let c = &shipped.campaigns[ci];
        let mi = c.map.unwrap();
        let n = c.kingdoms.len();
        let mut team = vec![None; n];
        for (ti, t) in c.teams.iter().enumerate() {
            for &k in &t.kingdoms {
                team[k] = Some(ti);
            }
        }
        let enemy = (0..n).map(|a| (0..n).map(|b| team[a] != team[b]).collect()).collect();
        let mut rel = Relations { enemy, team };
        let mut st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        for step in seq["steps"].as_array().unwrap() {
            let map = &data.maps[mi];
            match step["op"].as_str().unwrap() {
                "locations" => {
                    let bits = step["bits"].as_array().unwrap();
                    assert_eq!(bits.len(), data.maps[mi].territories.len());
                    for (t, b) in data.maps[mi].territories.iter_mut().zip(bits) {
                        let b: Vec<u32> = b.as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
                        assert_eq!(b[2], t.location[2]);
                        t.location = [b[0], b[1], b[2]];
                    }
                    count("locations replaced");
                }
                "heuristic" => {
                    let node = step["node"].as_u64().unwrap() as usize;
                    let goal = step["goal"].as_u64().unwrap() as usize;
                    let graph = PathGraph::new(&st, &data).unwrap();
                    let mut distinct = Vec::new();
                    for (name, p) in PRECISIONS {
                        let want = &step["st0"][name];
                        let bits = (u128::from(want["sign"].as_u64().unwrap() as u16) << 79)
                            | (u128::from(want["exponent"].as_u64().unwrap() as u16) << 64)
                            | u128::from(u64::from_str_radix(want["significand"].as_str().unwrap(), 16).unwrap());
                        let got = graph.heuristic(node, goal, p).unwrap().to_x87_extended_bits().unwrap();
                        assert_eq!(got, bits, "campaign {ci} heuristic {node}->{goal} {name}");
                        if !distinct.contains(&got) {
                            distinct.push(got);
                        }
                    }
                    count("heuristic");
                    if distinct.len() > 1 {
                        count("heuristic precision-dependent");
                    }
                }
                "relations" => {
                    rel.enemy = step["enemy_140"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| r.as_array().unwrap().iter().map(|b| b.as_bool().unwrap()).collect())
                        .collect();
                }
                "owner" => {
                    let t = step["territory"].as_u64().unwrap() as usize;
                    let k = step["kingdom"].as_u64().unwrap() as usize;
                    st.set_owner(t, k, &rel);
                }
                "move" => {
                    let a = ArmyId(step["army"].as_u64().unwrap() as u32);
                    st.enter_territory(a, step["territory"].as_u64().unwrap() as usize, &rel).unwrap();
                }
                "fought" => {
                    let a = step["army"].as_u64().unwrap() as usize;
                    st.armies[a].as_mut().unwrap().fought = step["value"].as_i64().unwrap() as i32;
                }
                "check" => {
                    let owners: Vec<Option<usize>> = step["owners"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().map(|x| x as usize))
                        .collect();
                    let got: Vec<Option<usize>> = st.territories.iter().map(|t| t.owner).collect();
                    assert_eq!(owners, got, "campaign {ci} owners");
                }
                "search" => {
                    let s = step["start"].as_u64().unwrap() as usize;
                    let g = step["goal"].as_u64().unwrap() as usize;
                    let actor = step["actor"].as_u64().map(|k| KingdomActor(k as u32));
                    let budget = step["budget"].as_i64().unwrap() as i32;
                    let graph = PathGraph::new(&st, &data).unwrap();
                    let mut distinct = Vec::new();
                    for (name, p) in PRECISIONS {
                        let want = &step["results"][name];
                        let r = graph.search(&rel, s, g, actor, budget, p).unwrap();
                        let ctx = format!("campaign {ci} search {s}->{g} actor {actor:?} budget {budget} {name}");
                        assert_eq!(r.code, want["code"].as_i64().unwrap() as i32, "{ctx} code");
                        assert_eq!(r.path, usize_list(&want["path"]), "{ctx} path");
                        if !distinct.contains(&r) {
                            distinct.push(r);
                        }
                    }
                    let r = &distinct[0];
                    count("search");
                    if distinct.len() > 1 {
                        count("search precision-dependent");
                    }
                    match r.code {
                        SEARCH_FOUND if s == g => count("search found start == goal"),
                        SEARCH_FOUND if r.path.len() == 1 => count("search found adjacent"),
                        SEARCH_FOUND => count("search found multi-step"),
                        SEARCH_DEPTH_LIMIT => count("search depth limit (2)"),
                        SEARCH_FAILED if !r.path.is_empty() => count("search path over budget (0, path kept)"),
                        SEARCH_FAILED => count("search failed"),
                        _ => panic!("unexpected code {}", r.code),
                    }
                    if budget == UNLIMITED {
                        count("search budget -1");
                    }
                    if budget < UNLIMITED {
                        count("search budget < -1");
                    }
                    if actor.is_none() {
                        count("search without kingdom actor");
                    }
                    if map.territories[st.territories[s].definition].kind == TerritoryKind::Alias {
                        count("search from alias");
                    }
                }
                "army" => {
                    let a = ArmyId(step["army"].as_u64().unwrap() as u32);
                    let target = step["target"].as_u64().unwrap() as usize;
                    let army = st.army(a);
                    assert_eq!(step["state"]["territory"].as_u64().map(|v| v as usize), army.territory);
                    assert_eq!(usize_list(&step["state"]["path"]), army.path);
                    assert_eq!(step["state"]["fought"].as_i64().unwrap() as i32, army.fought);
                    let graph = PathGraph::new(&st, &data).unwrap();
                    let used = graph.moves_used(a).unwrap();
                    assert_eq!(used, step["moves_used"].as_i64().unwrap() as i32, "campaign {ci} army {a:?} moves used");
                    let budget = graph.army_budget(&data, &rel, a).unwrap();
                    let mut distinct = Vec::new();
                    for (name, p) in PRECISIONS {
                        let want = &step["results"][name];
                        let got = graph.army_path(&data, &rel, a, target, p).unwrap();
                        let ctx = format!("campaign {ci} army {a:?} -> {target} budget {budget} {name}");
                        match want["ok"].as_u64().unwrap() {
                            0 => {
                                assert!(want["path"].as_array().unwrap().is_empty());
                                assert_eq!(got, None, "{ctx}");
                            }
                            1 => assert_eq!(got, Some(usize_list(&want["path"])), "{ctx}"),
                            v => panic!("93FDC0 returned {v}"),
                        }
                        if !distinct.contains(&got) {
                            distinct.push(got);
                        }
                    }
                    count("army");
                    if distinct.len() > 1 {
                        count("army precision-dependent");
                    }
                    let alias = map.territories[st.territories[target].definition].kind == TerritoryKind::Alias;
                    if alias {
                        count("army alias target");
                    }
                    let direct = graph.army_search(&data, &rel, a, target, aa_metagame::GAME_PRECISION);
                    let full = graph.army_path(&data, &rel, a, target, aa_metagame::GAME_PRECISION).unwrap();
                    match (alias, direct.unwrap(), full) {
                        (false, None, Some(p)) if p.is_empty() => panic!("fallback returned an empty path"),
                        (false, None, Some(_)) => {
                            count("army fallback success (93FAD0)");
                            // which 93FAD0 output branch: walk-back met the new path or not
                            let movement = data.armies[army.definition].movement;
                            let p = graph
                                .search(&rel, army.path[0], target, Some(army.owner), movement, aa_metagame::GAME_PRECISION)
                                .unwrap()
                                .path;
                            let k = army.path.len().saturating_sub(1);
                            if army.path.iter().take(k).skip(1).any(|t| p.contains(t)) {
                                count("army fallback walk-back meets the new path");
                            } else {
                                count("army fallback walk-back to path[0]");
                            }
                        }
                        (false, Some(_), _) => count("army direct success (93FCC0)"),
                        (false, None, None) => count("army failed"),
                        (true, ..) => {}
                    }
                    if budget == UNLIMITED {
                        count("army budget -1 (unlimited)");
                    }
                    if budget == 0 {
                        count("army budget 0");
                    }
                    if used > 0 {
                        count("army moves used > 0");
                    }
                    if army.path.windows(2).any(|w| {
                        map.territories[st.territories[w[0]].definition].kind == TerritoryKind::Alias
                    }) {
                        count("army path leaves an alias");
                    }
                }
                op => panic!("unknown op {op}"),
            }
        }
    }
    eprintln!("{seen:#?}");
    for key in [
        "search found start == goal",
        "search found adjacent",
        "search found multi-step",
        "search depth limit (2)",
        "search path over budget (0, path kept)",
        "search failed",
        "search budget -1",
        "search budget < -1",
        "search without kingdom actor",
        "search from alias",
        "army alias target",
        "army fallback success (93FAD0)",
        "army direct success (93FCC0)",
        "army failed",
        "army budget 0",
        "army moves used > 0",
        "army fallback walk-back meets the new path",
        "army fallback walk-back to path[0]",
        "heuristic",
        "heuristic precision-dependent",
        "search precision-dependent",
        "locations replaced",
    ] {
        assert!(seen.get(key).copied().unwrap_or(0) > 0, "no fixture row covers: {key}");
    }
}
