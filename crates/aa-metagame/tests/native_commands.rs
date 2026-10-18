//! Replays research/phase8-metagame/native-commands.json: original command
//! validation 951D70 (and validators) on perturbed InitCreate worlds.
//! The path search result of 93FDC0 is taken from the fixture (see path.rs).
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::commands::{Command, CommandData, CommandWorld, Validator};
use aa_metagame::data::MetaGameData;
use aa_metagame::state::{ArmyId, MgState};
use serde_json::Value;
use std::cell::RefCell;

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

struct Fixture<'a> {
    state: &'a MgState,
    path_found: RefCell<Option<bool>>,
    /// army vtable+0x134 forced result recorded by the producer (seam input)
    controllable: Option<bool>,
}
impl CommandWorld for Fixture<'_> {
    fn controllable(&self, army: ArmyId, issuer: KingdomActor) -> bool {
        self.controllable.unwrap_or(self.state.army(army).owner == issuer)
    }
    fn reach(&self, _: &MgState, _: ArmyId, _: usize) -> aa_metagame::Result<bool> {
        Ok(self.path_found.borrow().expect("fixture records the 93FDC0 result for this case"))
    }
}

#[test]
fn command_validation_matches_native_951d70() {
    let data_bytes = read("metagame-data.json");
    let data: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("native-commands.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    let mut checked = 0;
    let mut paths = 0;
    for camp in fx["campaigns"].as_array().unwrap() {
        let ci = camp["campaign"].as_u64().unwrap() as usize;
        let c = &data.campaigns[ci];
        let map = c.map.unwrap();
        let mut team = vec![None; c.kingdoms.len()];
        for (ti, t) in c.teams.iter().enumerate() {
            for &k in &t.kingdoms {
                team[k] = Some(ti);
            }
        }
        let rel = Teams(team);
        let mut st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        for round in camp["rounds"].as_array().unwrap() {
            for op in round["ops"].as_array().unwrap() {
                let v = |i: usize| op[i].as_i64().unwrap();
                match op[0].as_str().unwrap() {
                    "move" => {
                        st.enter_territory(ArmyId(v(1) as u32), v(2) as usize, &rel).unwrap();
                    }
                    "owner" => st.set_owner(v(1) as usize, v(2) as usize, &rel),
                    "resources" => st.kingdoms[v(1) as usize].resources = v(2) as i32,
                    "battles" => st.kingdoms[v(1) as usize].battles = v(2) as i32,
                    "current" => st.current = Some(v(1) as usize),
                    "battle_started" => st.battle_started = v(1) != 0,
                    "fought" => st.armies[v(1) as usize].as_mut().unwrap().fought = v(2) as i32,
                    other => panic!("op {other}"),
                }
            }
            for cs in round["commands"].as_array().unwrap() {
                let index = cs["index"].as_u64().unwrap() as u32;
                let mut cmd = Command::default();
                if cs.get("data").is_some() {
                    cmd.data = Some(if index < 12 { CommandData::builtin(index) } else { CommandData { index, flags: 0 } });
                }
                cmd.army = cs.get("army").map(|a| ArmyId(a.as_u64().unwrap() as u32));
                if let Some(d) = cs.get("territory_def") {
                    cmd.set_territory(&data, map, d.as_u64().unwrap() as usize);
                }
                cmd.technology = cs.get("tech").map(|t| t.as_str().unwrap().to_string());
                cmd.actor_definition = cs.get("actor_def").map(|a| {
                    data.armies.iter().position(|x| x.ids == a.as_str().unwrap()).unwrap()
                });
                // 952570 sets bit5 per pushed division: an empty list leaves it clear
                cmd.divisions = cs.get("divisions").map(|d| d.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect::<Vec<_>>()).filter(|v| !v.is_empty());
                assert_eq!(u64::from(cmd.presence()), cs["flags_word"].as_u64().unwrap() & 0x3f, "{cs}");
                let controllable = cs.get("controllable_override").and_then(|v| v.as_u64()).map(|v| v != 0);
                let world = Fixture { state: &st, path_found: RefCell::new(cs.get("path_found").map(|p| p.as_u64().unwrap() != 0)), controllable };
                let v = Validator { state: &st, data: &data, relations: &rel, world: &world, precision: aa_metagame::GAME_PRECISION };
                let issuer = KingdomActor(cs["issuer"].as_u64().unwrap() as u32);
                let got = v.validate(&cmd, issuer).unwrap();
                assert_eq!(u64::from(got), cs["code"].as_u64().unwrap(), "campaign {ci}: {cs}");
                // the ported path search agrees with the recorded 93FDC0 result
                if let Some(found) = cs.get("path_found") {
                    let a = cmd.army.unwrap();
                    let t = st.territories.iter().position(|t| Some(t.definition) == cmd.territory).unwrap();
                    let p = aa_metagame::path::army_path(&st, &data, &rel, a, t, aa_metagame::GAME_PRECISION).unwrap();
                    assert_eq!(p.is_some(), found.as_u64().unwrap() != 0, "{cs}");
                    if let Some(p) = p {
                        let want: Vec<usize> = cs["path"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect();
                        assert_eq!(p, want, "{cs}");
                    }
                    paths += 1;
                }
                // and the whole validation with the ported path search
                let native = aa_metagame::commands::NativePaths {
                    data: &data,
                    relations: &rel,
                    precision: aa_metagame::GAME_PRECISION,
                    controllable: |a: ArmyId, i: KingdomActor| controllable.unwrap_or(st.army(a).owner == i),
                };
                let v2 = Validator { state: &st, data: &data, relations: &rel, world: &native, precision: aa_metagame::GAME_PRECISION };
                assert_eq!(u64::from(v2.validate(&cmd, issuer).unwrap()), cs["code"].as_u64().unwrap(), "ported paths: {cs}");
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 1800);
    assert!(paths > 100, "{paths}");
}
