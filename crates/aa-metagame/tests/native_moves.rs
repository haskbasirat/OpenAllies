//! Replays research/phase8-metagame/native-moves.json: original army
//! relocation 940100 (956280 removal, 956140 slot insertion, 93F9F0 path) on
//! the InitCreate worlds of every shipped campaign.
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::data::MetaGameData;
use aa_metagame::state::{ArmyId, MgState};
use serde_json::Value;

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../../research/phase8-metagame/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
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

#[test]
fn army_relocation_matches_native_940100() {
    let data_bytes = read("metagame-data.json");
    let data: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("native-moves.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    let mut truncations = 0;
    for seq in fx["sequences"].as_array().unwrap() {
        let ci = seq["campaign"].as_u64().unwrap() as usize;
        let c = &data.campaigns[ci];
        let mut team = vec![None; c.kingdoms.len()];
        for (ti, t) in c.teams.iter().enumerate() {
            for &k in &t.kingdoms {
                team[k] = Some(ti);
            }
        }
        let rel = Teams(team);
        let mut st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        for mv in seq["moves"].as_array().unwrap() {
            let a = ArmyId(mv["army"].as_u64().unwrap() as u32);
            let t = mv["territory"].as_u64().unwrap() as usize;
            let before = st.army(a).path.len();
            st.enter_territory(a, t, &rel).unwrap();
            truncations += usize::from(st.army(a).path.len() <= before);
            assert_eq!(mv["army_territory"].as_u64().map(|v| v as usize), st.army(a).territory);
            let path: Vec<usize> = mv["army_path"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
            assert_eq!(path, st.army(a).path);
            for (i, (tt, want)) in st.territories.iter().zip(mv["territories"].as_array().unwrap()).enumerate() {
                let refs: Vec<(u32, i32)> = want["armies"].as_array().unwrap().iter()
                    .map(|r| (r[0].as_u64().unwrap() as u32, r[1].as_i64().unwrap() as i32)).collect();
                let got: Vec<(u32, i32)> = tt.armies.iter().map(|r| (r.army.0, r.slot)).collect();
                assert_eq!(got, refs, "campaign {ci} territory {i}");
                assert_eq!(want["enemy_army_count"].as_i64().unwrap() as i32, tt.enemy_army_count);
            }
        }
    }
    assert!(truncations > 20, "{truncations}");
}
