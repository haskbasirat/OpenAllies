//! Replays research/phase8-metagame/native-process.json: original
//! KKC_MGCommand::Process 954570 (move/attack waypoints, battle flags,
//! purchases, upgrades incl. the army type change 951AA0) on perturbed
//! InitCreate worlds. Path results of 93FDC0 come from the fixture.
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::commands::{
    Command, CommandData, CommandWorld, ProcessWorld, Processor, Validator,
};
use aa_metagame::data::MetaGameData;
use aa_metagame::state::{ArmyId, MgState};
use serde_json::Value;
use std::collections::BTreeMap;

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

struct Recorded<'a> {
    state: &'a MgState,
    path: Option<Vec<usize>>,
}
impl CommandWorld for Recorded<'_> {
    fn controllable(&self, army: ArmyId, issuer: KingdomActor) -> bool {
        self.state.army(army).owner == issuer
    }
    fn reach(&self, _: &MgState, _: ArmyId, _: usize) -> aa_metagame::Result<bool> {
        Ok(self.path.is_some())
    }
}
struct Paths(Option<Vec<usize>>);
impl ProcessWorld for Paths {
    fn path(&self, _: &MgState, _: ArmyId, _: usize) -> aa_metagame::Result<Option<Vec<usize>>> {
        Ok(self.0.clone())
    }
}

fn usizes(v: &Value) -> Vec<usize> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect()
}

#[test]
fn command_processing_matches_native_954570() {
    let data_bytes = read("metagame-data.json");
    let data: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("native-process.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    let mut processed = 0;
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
        // native army index -> Rust army id
        let mut ids: BTreeMap<usize, ArmyId> = (0..st.armies.len()).map(|i| (i, ArmyId(i as u32))).collect();
        let mut native_count = st.armies.len();
        for round in camp["rounds"].as_array().unwrap() {
            let cur = round["ops"].as_array().unwrap().iter().find(|o| o[0] == "current").unwrap()[1].as_u64().unwrap() as u32;
            for op in round["ops"].as_array().unwrap() {
                let v = |i: usize| op[i].as_i64().unwrap();
                match op[0].as_str().unwrap() {
                    "move" => {
                        st.enter_territory(ids[&(v(1) as usize)], v(2) as usize, &rel).unwrap();
                    }
                    "resources" => st.kingdoms[v(1) as usize].resources = v(2) as i32,
                    "battles" => st.kingdoms[v(1) as usize].battles = v(2) as i32,
                    "current" => st.current = Some(v(1) as usize),
                    "fought_all" => {
                        for a in st.armies.iter_mut().flatten() {
                            a.fought = 0;
                        }
                    }
                    other => panic!("op {other}"),
                }
            }
            for cs in round["commands"].as_array().unwrap() {
                if cs["code"].as_u64().unwrap() != 0 {
                    continue;
                }
                let index = cs["index"].as_u64().unwrap() as u32;
                let mut cmd = Command { data: Some(CommandData::builtin(index)), ..Command::default() };
                cmd.army = cs.get("army").map(|a| ids[&(a.as_u64().unwrap() as usize)]);
                if let Some(d) = cs.get("territory_def") {
                    cmd.set_territory(&data, map, d.as_u64().unwrap() as usize);
                }
                cmd.technology = cs.get("tech").map(|t| t.as_str().unwrap().to_string());
                cmd.actor_definition = cs.get("actor_def").map(|a| data.armies.iter().position(|x| x.ids == a.as_str().unwrap()).unwrap());
                cmd.divisions = cs.get("divisions").map(usizes).filter(|v| !v.is_empty());
                let path = cs.get("path").map(usizes);
                let issuer = KingdomActor(cur);
                let world = Recorded { state: &st, path: path.clone() };
                let v = Validator { state: &st, data: &data, relations: &rel, world: &world, precision: aa_metagame::GAME_PRECISION };
                assert_eq!(v.validate(&cmd, issuer).unwrap(), 0, "{cs}");
                if let (Some(want), Some(a), Some(_)) = (&path, cmd.army, cmd.territory) {
                    let t = st.territories.iter().position(|t| Some(t.definition) == cmd.territory).unwrap();
                    let got = aa_metagame::path::army_path(&st, &data, &rel, a, t, aa_metagame::GAME_PRECISION).unwrap();
                    assert_eq!(got.as_ref(), Some(want), "ported path vs recorded: {cs}");
                }
                let mut p = Processor { state: &mut st, data: &data, relations: &rel, precision: aa_metagame::GAME_PRECISION };
                let out = p.process(&cmd, issuer, &Paths(path)).unwrap();
                assert_eq!(u64::from(out.result), cs["result"].as_u64().unwrap(), "{cs}");
                let msgs: Vec<u32> = cs["messages"].as_array().unwrap().iter().map(|m| m.as_u64().unwrap() as u32).collect();
                assert_eq!(out.messages, msgs, "{cs}");
                // index bookkeeping
                let created = cs["created"].as_u64().unwrap() as usize;
                let respawned = cs["respawned"].as_array().unwrap();
                if let Some(id) = out.created {
                    assert_eq!(created, 1);
                    ids.insert(native_count, id);
                } else if let Some(id) = out.respawned {
                    assert_eq!(respawned.len(), 1, "{cs}");
                    assert_eq!(ids[&(respawned[0][0].as_u64().unwrap() as usize)], id);
                    ids.insert(respawned[0][1].as_u64().unwrap() as usize, id);
                } else {
                    assert_eq!(created, 0, "{cs}");
                }
                native_count += created;
                // state after
                let after = &cs["after"];
                for (k, want) in st.kingdoms.iter().zip(after["kingdoms"].as_array().unwrap()) {
                    assert_eq!(i64::from(k.resources), want["resources"].as_i64().unwrap(), "{cs}");
                    let techs: Vec<String> = want["technologies"].as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_string()).collect();
                    assert_eq!(k.technologies, techs);
                }
                let native_to_rust = |i: usize| ids.get(&i).copied();
                for (i, want) in after["armies"].as_array().unwrap().iter().enumerate() {
                    let Some(id) = native_to_rust(i) else { continue };
                    // a replaced native actor maps to the same id as its replacement; check the live one
                    if ids.range(i + 1..).any(|(_, &x)| x == id) {
                        continue;
                    }
                    let a = st.army(id);
                    assert_eq!(want["definition"].as_str().unwrap(), data.armies[a.definition].ids, "{cs}");
                    assert_eq!(want["territory"].as_u64().map(|v| v as usize), a.territory);
                    let divs: Vec<usize> = a.divisions.iter().map(|d| d.0 as usize).collect();
                    assert_eq!(usizes(&want["divisions"]), divs, "{cs}");
                    assert_eq!(usizes(&want["waypoints"]), a.waypoints, "army {i} cmd {} army {:?} resp {:?} tdef {:?} path {:?}", cs["index"], cs.get("army"), cs["respawned"], cs.get("territory_def"), cs.get("path"));
                }
                for (t, want) in st.territories.iter().zip(after["territories"].as_array().unwrap()) {
                    let refs: Vec<(u32, i32)> = want.as_array().unwrap().iter()
                        .map(|r| (native_to_rust(r[0].as_u64().unwrap() as usize).unwrap().0, r[1].as_i64().unwrap() as i32)).collect();
                    let got: Vec<(u32, i32)> = t.armies.iter().map(|r| (r.army.0, r.slot)).collect();
                    assert_eq!(got, refs, "{cs}");
                }
                let flags: Vec<u64> = after["flags_44_45_46"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
                assert_eq!(flags, [u64::from(st.battle_started), u64::from(st.battle_play_requested), u64::from(st.battle_resolved)]);
                assert_eq!(after["last_purchased"].as_u64().and_then(|i| native_to_rust(i as usize)), st.last_purchased);
                processed += 1;
            }
        }
    }
    assert!(processed > 200, "{processed}");
}
