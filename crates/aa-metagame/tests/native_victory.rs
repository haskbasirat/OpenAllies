//! Replays research/phase8-metagame/native-victory.json: original set-owner
//! 955620 sequences on the InitCreate worlds with the victory tests 95A6B0 /
//! 95A350 evaluated after every step.
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::data::MetaGameData;
use aa_metagame::state::MgState;
use serde_json::Value;

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../research/phase8-metagame/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

struct Matrix(Vec<Vec<bool>>);
impl KingdomRelations for Matrix {
    fn enemy(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.0[s.0 as usize][o.0 as usize]
    }
    fn relation_138(&self, _: KingdomActor, _: KingdomActor) -> bool {
        unreachable!("not used by owner changes or victory")
    }
}

fn usizes(v: &Value) -> Vec<Option<usize>> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().map(|u| u as usize)).collect()
}

#[test]
fn owner_changes_and_victory_match_native_955620_95a6b0_95a350() {
    let data_bytes = read("metagame-data.json");
    let data: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("native-victory.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    let mut winners = 0;
    for seq in fx["sequences"].as_array().unwrap() {
        let ci = seq["campaign"].as_u64().unwrap() as usize;
        let rel = Matrix(
            seq["enemy_140"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.as_array().unwrap().iter().map(|b| b.as_bool().unwrap()).collect())
                .collect(),
        );
        let mut st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        for step in seq["steps"].as_array().unwrap() {
            let t = step["territory"].as_u64().unwrap() as usize;
            let k = step["new_owner"].as_u64().unwrap() as usize;
            st.set_owner(t, k, &rel);
            let wins: Vec<u64> = (0..st.kingdoms.len())
                .map(|k| u64::from(st.has_won(&data, k, &rel).unwrap()))
                .collect();
            let lost: Vec<u64> = (0..st.kingdoms.len())
                .map(|k| u64::from(st.capital_lost(k, &rel).unwrap()))
                .collect();
            let want = |key: &str| -> Vec<u64> {
                step[key].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect()
            };
            assert_eq!(wins, want("wins"), "campaign {ci} step {step}");
            assert_eq!(lost, want("capital_lost"));
            winners += usize::from(wins.contains(&1));
            for (kk, list) in st.kingdoms.iter().zip(step["kingdom_territories"].as_array().unwrap()) {
                let want: Vec<usize> = list.as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
                assert_eq!(kk.territories, want, "owned-territory list order");
            }
            let owners: Vec<Option<usize>> = st.territories.iter().map(|t| t.owner).collect();
            assert_eq!(owners, usizes(&step["owners"]));
            let counts: Vec<i64> = st.territories.iter().map(|t| i64::from(t.enemy_army_count)).collect();
            let want_counts: Vec<i64> = step["enemy_counts"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
            assert_eq!(counts, want_counts);
        }
    }
    assert!(winners > 50, "{winners}");
}
