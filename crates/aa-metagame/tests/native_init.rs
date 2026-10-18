//! Replays research/phase8-metagame/native-init.json (original
//! KKC_MGState::InitCreate 939B90 under Unicorn on every shipped campaign)
//! against MgState::init_create. Input definitions: metagame-data.json, the
//! loader export pinned by the fixture's data_sha256.
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::data::MetaGameData;
use aa_metagame::state::{INITIAL_RNG_STATE, MgState};
use serde_json::Value;

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../research/phase8-metagame/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
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

fn sha256_hex(bytes: &[u8]) -> String {
    aa_rules::sources::sha256(bytes)
}

#[test]
fn init_create_matches_native_939b90() {
    let data_bytes = read("metagame-data.json");
    let data: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("native-init.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), sha256_hex(&data_bytes));
    let rows = fx["campaigns"].as_array().unwrap();
    assert_eq!(rows.len(), data.campaigns.len());
    for (ci, row) in rows.iter().enumerate() {
        let c = &data.campaigns[ci];
        assert_eq!(row["campaign"].as_str().unwrap(), c.ids);
        let mut team = vec![None; c.kingdoms.len()];
        for (ti, t) in c.teams.iter().enumerate() {
            for &k in &t.kingdoms {
                team[k] = Some(ti);
            }
        }
        let rel = Teams(team);
        let st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        assert_eq!(row["current"].as_u64().map(|v| v as usize), st.current);
        assert_eq!(row["rng_state"].as_u64().unwrap(), u64::from(INITIAL_RNG_STATE));
        assert_eq!(st.rng.state, INITIAL_RNG_STATE);
        assert_eq!(row["state_byte_50"].as_u64().unwrap(), 0);
        assert_eq!(row["round"].as_i64().unwrap() as i32, st.round);
        let flags: Vec<u64> = row["flags_44_45_46_50_51"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
        assert_eq!(flags, [u64::from(st.battle_started), u64::from(st.battle_play_requested), u64::from(st.battle_resolved), u64::from(st.game_over), u64::from(st.fast_ai)]);
        assert!(row["last_purchased"].is_null() && st.last_purchased.is_none());
        assert!(row["results"].as_array().unwrap().is_empty() && st.results.is_empty());
        // kingdoms
        let ks = row["kingdoms"].as_array().unwrap();
        assert_eq!(ks.len(), st.kingdoms.len());
        for (k, want) in st.kingdoms.iter().zip(ks) {
            assert!(want["back_pointer_is_state"].as_bool().unwrap());
            assert_eq!(want["data"].as_u64().unwrap() as usize, k.data);
            assert_eq!(want["actor"].as_u64().unwrap() as u32, k.actor.unwrap().0);
            assert_eq!(want["capital"].as_u64().map(|v| v as usize), k.capital);
            let terr: Vec<usize> = want["territories"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
            assert_eq!(terr, k.territories);
            assert_eq!(want["technologies"].as_u64().unwrap() as usize, k.technologies.len());
            assert_eq!(want["list_2c"].as_u64().unwrap(), 0);
            assert_eq!(want["resources"].as_i64().unwrap() as i32, k.resources);
            assert_eq!(want["battles"].as_i64().unwrap() as i32, k.battles);
        }
        // territories
        let ts = row["territories"].as_array().unwrap();
        assert_eq!(ts.len(), st.territories.len());
        for (i, (t, want)) in st.territories.iter().zip(ts).enumerate() {
            assert_eq!(want["index"].as_u64().unwrap() as usize, i);
            let record = want["record"].as_array().map(|r| (r[0].as_u64().unwrap() as usize, r[1].as_u64().unwrap() as usize));
            assert_eq!(record, t.record, "territory {i}");
            assert_eq!(want["definition"].as_u64().unwrap() as usize, t.definition);
            assert_eq!(want["owner"].as_u64().map(|v| v as usize), t.owner);
            assert_eq!(want["enemy_army_count"].as_i64().unwrap() as i32, t.enemy_army_count);
            let refs: Vec<(u32, i32)> = want["armies"].as_array().unwrap().iter()
                .map(|r| (r["army"].as_u64().unwrap() as u32, r["slot"].as_i64().unwrap() as i32)).collect();
            let got: Vec<(u32, i32)> = t.armies.iter().map(|r| (r.army.0, r.slot)).collect();
            assert_eq!(got, refs, "territory {i} armies");
            assert!(want["visual_handle_set"].as_bool().unwrap());
        }
        // armies (creation order = ids)
        let arm = row["armies"].as_array().unwrap();
        assert_eq!(arm.len(), st.armies.len());
        for (a, want) in st.armies.iter().map(|a| a.as_ref().unwrap()).zip(arm) {
            assert_eq!(want["definition"].as_str().unwrap(), data.armies[a.definition].ids);
            assert_eq!(want["owner"].as_u64().unwrap() as u32, a.owner.0);
            assert_eq!(want["territory"].as_u64().map(|v| v as usize), a.territory);
            let path: Vec<usize> = want["path"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as usize).collect();
            assert_eq!(path, a.path);
        }
    }
}
