//! Replays research/phase8-metagame/native-positions.json (the original
//! KKC_TerritoryPositionSolver 959420, its layouts 959390/959560/959240, the
//! territory position 9552A0/957DD0 and the callers 9626D0, 962880 and
//! 955B70 under Unicorn at x87 PC53/PC24/PC64 on the InitCreate worlds) and
//! the army creation positions recorded by native-init.json (955B70 during
//! InitCreate 939B90).
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::data::{ActorSpacing, MetaGameData};
use aa_metagame::positions::{self, FIRST_FREE};
use aa_metagame::state::{ArmyId, MgArmy, MgState};
use aa_numeric::Precision;
use serde_json::Value;
use std::collections::BTreeMap;

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../../research/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn fixture() -> (Vec<u8>, MetaGameData, Value) {
    let data_bytes = read("phase8-metagame/metagame-data.json");
    let data: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("phase8-metagame/native-positions.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    (data_bytes, data, fx)
}

/// PC53 first: `aa_metagame::GAME_PRECISION`.
const PRECISIONS: [(&str, Precision); 3] = [
    ("pc53", Precision::Double53),
    ("pc24", Precision::Single24),
    ("pc64", Precision::Extended64),
];

fn bits(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn bits3(v: &Value) -> [u32; 3] {
    let a = v.as_array().unwrap();
    assert_eq!(a.len(), 3);
    [bits(&a[0]), bits(&a[1]), bits(&a[2])]
}
fn spacing(v: &Value) -> ActorSpacing {
    match v.as_u64().unwrap() {
        0 => ActorSpacing::Simple,
        1 => ActorSpacing::Circle,
        2 => ActorSpacing::Square,
        x => panic!("spacing {x}"),
    }
}

struct Teams(Vec<Option<usize>>);
impl KingdomRelations for Teams {
    fn enemy(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.0[s.0 as usize] != self.0[o.0 as usize]
    }
    fn relation_138(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.0[s.0 as usize] == self.0[o.0 as usize]
    }
}
fn teams(data: &MetaGameData, ci: usize) -> Teams {
    let c = &data.campaigns[ci];
    let mut team = vec![None; c.kingdoms.len()];
    for (ti, t) in c.teams.iter().enumerate() {
        for &k in &t.kingdoms {
            team[k] = Some(ti);
        }
    }
    Teams(team)
}

#[test]
fn trig_tables_are_the_aa_trig_tables() {
    let (_, _, fx) = fixture();
    let trig = read("phase4-trig/lookup-tables.bin");
    let rows = fx["trig_tables"].as_array().unwrap();
    assert!(rows.len() >= 5 + 3);
    for row in rows {
        assert!(row["equal"].as_bool().unwrap(), "{row}");
        assert_eq!(row["emulated_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&trig));
    }
}

/// research/phase8-metagame/trig-tables-exact.json (trig_tables_exact.py):
/// every entry of the cos/sin tables built by 9C8620/9C8640 (argument
/// round_f32(i * f32(2pi) / 4096), FCOS/FSIN, binary32 store) is the
/// binary32 rounding of the exact value (mpmath, 256 bits), with a margin to
/// the rounding boundary above 2 ulp53 + 2^-62 (x87 FSIN/FCOS accuracy, the
/// 66-bit pi reduction, Unicorn's binary64 path). aa-trig embeds exactly
/// these tables (research/phase4-trig/lookup-tables.bin).
#[test]
fn trig_tables_are_the_exact_roundings() {
    let fx: Value = serde_json::from_slice(&read("phase8-metagame/trig-tables-exact.json")).unwrap();
    let trig = read("phase4-trig/lookup-tables.bin");
    assert_eq!(fx["exact_rounding_tables_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&trig));
    for name in ["cos", "sin"] {
        let t = &fx["tables"][name];
        assert_eq!(t["entries"].as_u64(), Some(4096));
        assert!(t["table_equals_exact_rounding"].as_bool().unwrap(), "{name}");
        assert!(t["mismatched_indices"].as_array().unwrap().is_empty(), "{name}");
        assert!(t["margin_failures"].as_array().unwrap().is_empty(), "{name}");
        assert!(t["min_distance_over_bound"].as_f64().unwrap() > 1.0, "{name}");
    }
    for c in fx["captures"].as_array().unwrap() {
        assert!(c["arguments_equal_reconstruction"].as_bool().unwrap());
        assert!(c["emulated_tables_equal_aa_trig"].as_bool().unwrap());
    }
}

#[test]
fn layouts_match_native() {
    let (_, _, fx) = fixture();
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for row in fx["layouts"].as_array().unwrap() {
        let s = spacing(&row["spacing"]);
        let center = bits3(&row["center"]);
        let n = row["n"].as_i64().unwrap() as i32;
        let d = bits(&row["distance"]);
        let mut distinct = Vec::new();
        for (name, p) in PRECISIONS {
            let got = positions::layout_position(s, center, n, d, p).unwrap();
            assert_eq!(got, bits3(&row["results"][name]), "{s:?} n {n} distance {d:#x} center {center:x?} {name}");
            if !distinct.contains(&got) {
                distinct.push(got);
            }
        }
        *seen.entry(format!("{s:?}")).or_default() += 1;
        if distinct.len() > 1 {
            *seen.entry(format!("{s:?} precision-dependent")).or_default() += 1;
        }
    }
    eprintln!("{seen:#?}");
    for s in ["Simple", "Circle", "Square"] {
        assert!(seen.get(s).copied().unwrap_or(0) > 100, "{s}");
    }
}

#[test]
fn solver_and_callers_match_native() {
    let (_, shipped, fx) = fixture();
    let mut seen: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut count = |k: &'static str| *seen.entry(k).or_default() += 1;
    for seq in fx["sequences"].as_array().unwrap() {
        let ci = seq["campaign"].as_u64().unwrap() as usize;
        let mi = shipped.campaigns[ci].map.unwrap();
        let mut data = shipped.clone();
        let rel = teams(&data, ci);
        let mut st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        let mut vz: Vec<Option<u32>> = Vec::new();
        for step in seq["steps"].as_array().unwrap() {
            let visual = |i: usize| vz[i];
            match step["op"].as_str().unwrap() {
                "defs" => {
                    let defs = step["defs"].as_array().unwrap();
                    assert_eq!(defs.len(), data.maps[mi].territories.len());
                    for (t, d) in data.maps[mi].territories.iter_mut().zip(defs) {
                        t.actor_spacing = spacing(&d["spacing"]);
                        t.actor_spacing_distance_bits = bits(&d["distance"]);
                        t.actor_slots = d["slots"].as_array().unwrap().iter().map(bits3).collect();
                    }
                    count("definition variant");
                }
                "links" => {
                    for (t, per) in data.maps[mi].territories.iter_mut().zip(step["links"].as_array().unwrap()) {
                        let per = per.as_array().unwrap();
                        assert_eq!(per.len(), t.links.len());
                        for (l, pts) in t.links.iter_mut().zip(per) {
                            l.intermediate_points = pts.as_array().unwrap().iter().map(bits3).collect();
                        }
                    }
                }
                "visual_z" => {
                    vz = step["z"].as_array().unwrap().iter().map(|v| v.as_u64().map(|b| b as u32)).collect();
                    assert_eq!(vz.len(), st.territories.len());
                }
                "move" => {
                    let a = ArmyId(step["army"].as_u64().unwrap() as u32);
                    st.enter_territory(a, step["territory"].as_u64().unwrap() as usize, &rel).unwrap();
                }
                "solve" => {
                    let t = step["territory"].as_u64().unwrap() as usize;
                    let index = step["index"].as_i64().unwrap() as i32;
                    for (name, p) in PRECISIONS {
                        let got = positions::solve(&st, &data, t, index, &visual, p).unwrap();
                        assert_eq!(got, bits3(&step["results"][name]), "campaign {ci} solve {t} index {index} {name}");
                    }
                    let def = &data.maps[mi].territories[st.territories[t].definition];
                    let index = if index == FIRST_FREE { positions::first_free_slot(&st, t) } else { index };
                    count("solve");
                    if def.kind == aa_metagame::data::TerritoryKind::Alias {
                        count("solve alias (9552A0)");
                    } else if index >= def.actor_slots.len() as i32 {
                        count(match def.actor_spacing {
                            ActorSpacing::Simple => "solve layout simple",
                            ActorSpacing::Circle => "solve layout circle",
                            ActorSpacing::Square => "solve layout square",
                        });
                    } else {
                        count("solve slot only");
                    }
                    if step["index"].as_i64() == Some(-1) && index > 0 {
                        count("solve first free > 0");
                    }
                }
                "territory" => {
                    let t = step["territory"].as_u64().unwrap() as usize;
                    for (name, p) in PRECISIONS {
                        let got = positions::territory_position(&st, &data, t, vz[t], p).unwrap();
                        assert_eq!(got, bits3(&step["results"][name]), "campaign {ci} territory {t} {name}");
                    }
                    count(if vz[t].is_some() { "territory with visual z" } else { "territory without visual" });
                }
                "realign" => {
                    let a = ArmyId(step["army"].as_u64().unwrap() as u32);
                    let t = step["territory"].as_u64().unwrap() as usize;
                    let actor = bits3(&step["actor_position"]);
                    for (name, p) in PRECISIONS {
                        let want = &step["results"][name];
                        let got = positions::realign_position(&st, &data, a, t, actor, &visual, p).unwrap();
                        let expected = match want["ok"].as_u64().unwrap() {
                            0 => None,
                            1 => Some(bits3(&want["position"])),
                            v => panic!("9626D0 returned {v}"),
                        };
                        assert_eq!(got, expected, "campaign {ci} realign army {a:?} territory {t} {name}");
                    }
                    let r = positions::realign_position(&st, &data, a, t, actor, &visual, aa_metagame::GAME_PRECISION).unwrap();
                    count(if r.is_some() { "realign moved" } else { "realign kept" });
                    if r.is_some() && positions::army_slot(&st, t, a) == -1 {
                        count("realign army not listed (-1)");
                    }
                }
                "waypoints" => {
                    let a = step["army"].as_u64().unwrap() as u32;
                    let from = step["from"].as_u64().unwrap() as usize;
                    let to = step["to"].as_u64().unwrap() as usize;
                    let flag = step["flag"].as_u64().unwrap();
                    let _ = a;
                    for (name, p) in PRECISIONS {
                        let want = &step["results"][name];
                        let got = positions::move_waypoints(&st, &data, from, to, &visual, p).unwrap();
                        let points: Vec<[u32; 3]> = want["points"].as_array().unwrap().iter().map(bits3).collect();
                        assert_eq!(got.points, points, "campaign {ci} waypoints {from}->{to} {name}");
                        let flags: Vec<u64> = want["flags"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
                        let mut expected = vec![1u64; points.len()];
                        expected[0] = flag;
                        assert_eq!(flags, expected);
                    }
                    let w = positions::move_waypoints(&st, &data, from, to, &visual, aa_metagame::GAME_PRECISION).unwrap();
                    count(if w.territory != to { "waypoints to the alias partner" } else { "waypoints to the target" });
                    if w.points.len() > 1 {
                        count("waypoints with link points");
                    }
                }
                "place" => {
                    let t = step["territory"].as_u64().unwrap() as usize;
                    let ids = step["army_type"].as_str().unwrap();
                    let owner = step["owner"].as_u64().unwrap() as u32;
                    let got = positions::placement_position(&st, &data, t, FIRST_FREE, &visual, aa_metagame::GAME_PRECISION).unwrap();
                    assert_eq!(got, bits3(&step["position_pc53"]), "campaign {ci} place {ids} in {t}");
                    // keep the replayed state in step: the new army enters t (940100)
                    let definition = data.armies.iter().position(|a| a.ids == ids).unwrap();
                    let template = st.armies.iter().flatten().next().unwrap().clone();
                    let id = ArmyId(st.armies.len() as u32);
                    st.armies.push(Some(MgArmy {
                        definition,
                        owner: KingdomActor(owner),
                        fought: 0,
                        territory: None,
                        path: Vec::new(),
                        ..template
                    }));
                    st.enter_territory(id, t, &rel).unwrap();
                    count("place (955B70)");
                }
                op => panic!("unknown op {op}"),
            }
        }
    }
    eprintln!("{seen:#?}");
    for key in [
        "definition variant",
        "solve alias (9552A0)",
        "solve layout simple",
        "solve layout circle",
        "solve layout square",
        "solve slot only",
        "solve first free > 0",
        "territory with visual z",
        "territory without visual",
        "realign moved",
        "realign kept",
        "waypoints to the alias partner",
        "waypoints to the target",
        "waypoints with link points",
        "place (955B70)",
    ] {
        assert!(seen.get(key).copied().unwrap_or(0) > 0, "no fixture row covers: {key}");
    }
}

/// InitCreate (955D50 -> 955B70 per authored army): the creation position of
/// every army is its first free slot when created, which is the slot it
/// holds afterwards (956140 uses the same rule), with z = 2.0.
#[test]
fn init_create_positions_match_native_init() {
    let data_bytes = read("phase8-metagame/metagame-data.json");
    let data: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("phase8-metagame/native-init.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    let mut armies = 0;
    for (ci, row) in fx["campaigns"].as_array().unwrap().iter().enumerate() {
        let rel = teams(&data, ci);
        let st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        let want = row["armies"].as_array().unwrap();
        assert_eq!(want.len(), st.armies.len());
        for (t, territory) in st.territories.iter().enumerate() {
            for r in &territory.armies {
                // 9552A0 (alias) z is replaced by 2.0: the visual is irrelevant here.
                let got = positions::placement_position(&st, &data, t, r.slot, &|_| None, aa_metagame::GAME_PRECISION).unwrap();
                assert_eq!(got, bits3(&want[r.army.0 as usize]["position_bits"]), "campaign {ci} army {:?}", r.army);
                armies += 1;
            }
        }
    }
    assert!(armies > 100, "{armies}");
}
