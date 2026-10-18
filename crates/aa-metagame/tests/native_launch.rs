//! Replays research/phase8-metagame/native-launch.json: the original
//! metagame battle generation `KKC_MetaGameRTSLauncher::RequestGame` (948EF0,
//! participants 947560) and the RTS result decision 93A990, run under Unicorn
//! on perturbed InitCreate worlds of every shipped campaign at x87 PC53 (the
//! game precision), PC24 and PC64. Every battle compares the participant
//! records, the complete order sequence up to the RTS launch order (or a
//! native NULL dereference against `Error::Unsupported`) and, for each RTS
//! result, the ResolveBattle arguments, the play flag and the RNG state.
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::data::{MetaGameData, ResourceValues};
use aa_metagame::launch::{
    LaunchEnv, LaunchOrder, Participant, PlayerId, RmcSlider, RmcSliders, SessionPlayer, battle_setup,
    participants, rmc_settings, rts_outcome,
};
use aa_metagame::state::{ArmyId, MgState};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../../research/phase8-metagame/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

struct Matrix {
    enemy: Vec<Vec<bool>>,
    r138: Vec<Vec<bool>>,
}
impl KingdomRelations for Matrix {
    fn enemy(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.enemy[s.0 as usize][o.0 as usize]
    }
    fn relation_138(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.r138[s.0 as usize][o.0 as usize]
    }
}

struct Env {
    players: Vec<SessionPlayer>,
    local: Option<PlayerId>,
    names: Vec<String>,
    registry: Vec<String>,
    sliders: HashMap<String, RmcSliders>,
}
impl LaunchEnv for Env {
    fn players(&self) -> &[SessionPlayer] {
        &self.players
    }
    fn local_player(&self) -> Option<PlayerId> {
        self.local
    }
    fn kingdom_actor_name(&self, actor: KingdomActor) -> aa_metagame::Result<String> {
        Ok(self.names[actor.0 as usize].clone())
    }
    fn resource_registry(&self) -> &[String] {
        &self.registry
    }
    fn rmc_sliders(&self, rmc: &str) -> aa_metagame::Result<RmcSliders> {
        Ok(self.sliders[rmc].clone())
    }
}

fn matrix(v: &Value) -> Vec<Vec<bool>> {
    v.as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(|b| b.as_bool().unwrap()).collect()).collect()
}
fn s(v: &Value) -> String {
    v.as_str().unwrap().to_string()
}
fn u32v(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn opt_s(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}
fn values(v: &Value) -> ResourceValues {
    ResourceValues { values: v.as_array().unwrap().iter().map(|p| (s(&p[0]), u32v(&p[1]))).collect() }
}
fn precision(name: &str) -> Precision {
    match name {
        "single24" => Precision::Single24,
        "double53" => Precision::Double53,
        "extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}

/// Data perturbations of the oracle world (applied to a clone of the shipped data).
fn apply_overrides(data: &mut MetaGameData, ci: usize, ov: &Value) {
    for (ids, o) in ov["armies"].as_object().unwrap() {
        let a = data.armies.iter_mut().find(|a| &a.ids == ids).unwrap();
        for (k, v) in o.as_object().unwrap() {
            match k.as_str() {
                "rts_actor_ids" => a.rts_actor_ids = s(v),
                "rts_water_actor_ids" => a.rts_water_actor_ids = s(v),
                "stockpile_bonus" => a.stockpile_bonus = values(v),
                "water_stockpile_bonus" => a.water_stockpile_bonus = values(v),
                other => panic!("army override {other}"),
            }
        }
    }
    for (ids, o) in ov["divisions"].as_object().unwrap() {
        let d = data.divisions.iter_mut().find(|d| &d.ids == ids).unwrap();
        for (k, v) in o.as_object().unwrap() {
            match k.as_str() {
                "rts_actor_ids" => d.rts_actor_ids = s(v),
                "rts_water_actor_ids" => d.rts_water_actor_ids = s(v),
                "technologies" => d.technologies = v.as_array().unwrap().iter().map(opt_s).collect(),
                other => panic!("division override {other}"),
            }
        }
    }
    for (index, o) in ov["kingdoms"].as_object().unwrap() {
        let k = &mut data.campaigns[ci].kingdoms[index.parse::<usize>().unwrap()];
        for (key, v) in o.as_object().unwrap() {
            match key.as_str() {
                "stockpile_bonus" => k.stockpile_bonus = values(v),
                "water_stockpile_bonus" => k.water_stockpile_bonus = values(v),
                "color_ids" => k.color_ids = s(v),
                "nation_ids" => k.nation_ids = s(v),
                other => panic!("kingdom override {other}"),
            }
        }
    }
}

fn sliders(v: &Value) -> RmcSliders {
    let list = |l: &Value| -> Vec<RmcSlider> {
        l.as_array()
            .unwrap()
            .iter()
            .map(|e| RmcSlider { name: s(&e[0]), min_bits: u32v(&e[1]), max_bits: u32v(&e[2]) })
            .collect()
    };
    RmcSliders {
        first: list(&v["list1"]),
        second: list(&v["list2"]),
        third: list(&v["list3"]),
        int_min: v["int_range"][0].as_i64().unwrap() as i32,
        int_max: v["int_range"][1].as_i64().unwrap() as i32,
    }
}

fn participant_json(p: &Participant, data: &MetaGameData) -> Value {
    json!({
        "rmc_id": p.rmc_id,
        "kind": p.kind,
        "player": p.player.map(|x| x.0),
        "kingdom": p.kingdom,
        "merge_into": p.merge_into,
        "armies": p.armies.iter().map(|&a| data.armies[a].ids.clone()).collect::<Vec<_>>(),
        "divisions": p.divisions,
    })
}

/// Oracle record of an order (set_session_rmc is checked separately).
fn order_json(o: &LaunchOrder) -> Value {
    match o {
        LaunchOrder::SetSessionRmc { rmc } => json!(["set_session_rmc", rmc, rmc]),
        LaunchOrder::SetRmcSettings { settings } => json!([
            "set_rmc_settings",
            settings.total_bits,
            settings.first.iter().map(|(n, b)| json!([n, b])).collect::<Vec<_>>(),
            settings.second.iter().map(|(n, b)| json!([n, b])).collect::<Vec<_>>(),
            settings.midpoint
        ]),
        LaunchOrder::SetMapSize { width_bits, height_bits } => json!(["set_map_size", width_bits, height_bits]),
        LaunchOrder::SetNewState { state, game_state: g } => json!([
            "set_new_state",
            state,
            [g.kind, u8::from(g.flag_04), g.text, g.field_0c, g.field_10, u8::from(g.flag_14), u8::from(g.flag_15),
             u8::from(g.flag_16), g.field_18, g.field_1c, u8::from(g.flag_20), g.field_24]
        ]),
        LaunchOrder::SetMapType { map_type } => json!(["set_map_type", map_type]),
        LaunchOrder::SetPlayerKingdom { player, kingdom } => json!(["set_player_kingdom", player.map(|p| p.0), kingdom]),
        LaunchOrder::CreateExplicitAi { kingdom, second, name } => json!(["create_explicit_ai", kingdom, second, name]),
        LaunchOrder::SetKingdomMergeInto { kingdom, merge_into } => json!(["set_merge_into", kingdom, merge_into]),
        LaunchOrder::SetKingdomColor { kingdom, color } => json!(["set_color", kingdom, color]),
        LaunchOrder::SetKingdomNation { kingdom, nation } => json!(["set_nation", kingdom, nation]),
        LaunchOrder::SetKingdomFaction { kingdom, faction } => json!(["set_faction", kingdom, faction]),
        LaunchOrder::AddKingdomStartProperty { kingdom, property } => json!(["add_start_property", kingdom, property]),
        LaunchOrder::SetKingdomBonusStockpile { kingdom, resource, amount_bits } => {
            json!(["set_bonus_stockpile", kingdom, resource, amount_bits])
        }
        LaunchOrder::ClearKingdomAdditionalActors { kingdom } => json!(["clear_additional_actors", kingdom]),
        LaunchOrder::AddKingdomAdditionalActor { kingdom, actor } => json!(["add_additional_actor", kingdom, actor]),
        LaunchOrder::Launch => json!(["launch"]),
    }
}

#[test]
fn native_launch_replays_every_world() {
    let data_bytes = read("metagame-data.json");
    let base: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read("native-launch.json")).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut bump = |k: String| *counts.entry(k).or_default() += 1;
    for (wi, world) in fx["worlds"].as_array().unwrap().iter().enumerate() {
        let ci = world["campaign"].as_u64().unwrap() as usize;
        let pc = precision(world["precision"].as_str().unwrap());
        bump(format!("world {}", world["precision"].as_str().unwrap()));
        let mut data = base.clone();
        apply_overrides(&mut data, ci, &world["data_overrides"]);
        let rel = Matrix { enemy: matrix(&world["enemy_140"]), r138: matrix(&world["relation_138"]) };
        let registry: Vec<String> = world["resource_registry"].as_array().unwrap().iter().map(s).collect();
        let slider_map: HashMap<String, RmcSliders> = world["rmc_sliders"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), sliders(v)))
            .collect();
        let mut st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        for (bi, battle) in world["battles"].as_array().unwrap().iter().enumerate() {
            let what = format!("world {wi} battle {bi}");
            for op in battle["ops"].as_array().unwrap() {
                let o = op.as_array().unwrap();
                let v = |i: usize| o[i].as_u64().unwrap() as usize;
                match o[0].as_str().unwrap() {
                    "move" => {
                        st.enter_territory(ArmyId(v(1) as u32), v(2), &rel).unwrap();
                    }
                    "owner" => st.set_owner(v(1), v(2), &rel),
                    "tech" => {
                        let ids = s(&o[2]);
                        let list = &mut st.kingdoms[v(1)].technologies;
                        if !list.contains(&ids) {
                            list.push(ids);
                        }
                    }
                    other => panic!("op {other}"),
                }
            }
            let session = &battle["session"];
            let env = Env {
                players: session["players"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| SessionPlayer {
                        kingdom_name: s(&p["kingdom_name"]),
                        ai: p["ai"].as_u64().unwrap() as u8,
                        kingdom_actor: p["kingdom_actor"].as_u64().map(|a| KingdomActor(a as u32)),
                    })
                    .collect(),
                local: session["local"].as_u64().map(|p| PlayerId(p as u32)),
                names: battle["actor_names"].as_array().unwrap().iter().map(s).collect(),
                registry: registry.clone(),
                sliders: slider_map.clone(),
            };
            // MGState+0x0C, sent with SetKingdomFaction
            st.faction = opt_s(&session["faction"]);
            let t = battle["territory"].as_u64().unwrap() as usize;
            // 947560
            let want = &battle["participants"];
            match participants(&st, &data, &rel, t, &env, pc) {
                Ok(list) => {
                    assert!(want.get("fault").is_none(), "{what}: native faulted ({}) but Rust built {list:?}", want["fault"]);
                    let got: Vec<Value> = list.iter().map(|p| participant_json(p, &data)).collect();
                    assert_eq!(Value::Array(got), want["participants"], "{what}: participants");
                    bump("participants built".into());
                    for p in &list {
                        bump(format!("participant kind={} player={} merge={}", p.kind, p.player.is_some(), !p.merge_into.is_empty()));
                    }
                }
                Err(e) => {
                    assert!(want.get("fault").is_some(), "{what}: Rust error {e} but native built {}", want["participants"]);
                    bump("participants unsupported".into());
                }
            }
            // 948EF0
            let want = &battle["request"];
            match battle_setup(&st, &data, &rel, t, &env, pc) {
                Ok(setup) => {
                    assert!(want.get("fault").is_none(), "{what}: native faulted ({}) but Rust launched", want["fault"]);
                    let recs = want["orders"].as_array().unwrap();
                    assert_eq!(setup.orders.len(), recs.len(), "{what}: order count\n{:#?}\n{recs:?}", setup.orders);
                    for (oi, (o, rec)) in setup.orders.iter().zip(recs).enumerate() {
                        let got = order_json(o);
                        if let LaunchOrder::SetSessionRmc { rmc } = o {
                            let index = data.rmcs.iter().position(|r| &r.ids == rmc).unwrap();
                            assert_eq!(rec[1], json!(rmc), "{what}: order {oi} rmc");
                            assert_eq!(rec[2], json!(rmc), "{what}: order {oi} session rmc name");
                            assert_eq!(rec[3], json!(0x5000 + index), "{what}: order {oi} settings word");
                            assert_eq!(rec[4], json!(0x5000 + index), "{what}: order {oi} settings copy");
                            assert_eq!(rec[5], json!([0, 0, 0, 0]), "{what}: order {oi} map settings reset");
                        } else {
                            assert_eq!(&got, rec, "{what}: order {oi}");
                        }
                        bump(format!("order {}", rec[0].as_str().unwrap()));
                    }
                    assert_eq!(setup.participants.len(), want_participants(battle), "{what}");
                    let rmc = data.rmcs.iter().find(|r| r.ids == setup.rmc).unwrap();
                    assert_eq!(Some(&setup.map_size), rmc.map_sizes.first(), "{what}: map size");
                    // the slider midpoints are precision dependent: count worlds where it shows
                    let sl = &env.sliders[&setup.rmc];
                    if rmc_settings(sl, Precision::Single24).unwrap() != rmc_settings(sl, Precision::Double53).unwrap() {
                        bump(format!("rmc settings PC24 != PC53 (proven at {:?})", pc));
                    }
                    bump("launched".into());
                }
                Err(e) => {
                    assert!(want.get("fault").is_some(), "{what}: Rust error {e} but native launched");
                    bump("launch unsupported".into());
                }
            }
            // 93A990
            for (ri, r) in battle["rts"].as_array().unwrap().iter().enumerate() {
                let what = format!("{what} rts {ri}");
                st.battle_play_requested = r["play_requested"].as_u64().unwrap() != 0;
                let res = &r["result"];
                let before = st.rng.state;
                // MGState+0x48 (written by 60A140 at RTS game over)
                st.rts_winner = r["winner"].as_str().unwrap().to_owned();
                match rts_outcome(&mut st, &data, &rel, &env, u32v(&r["app_state"]), pc) {
                    Ok(decision) => {
                        assert!(res.get("fault").is_none(), "{what}: native faulted ({})", res["fault"]);
                        assert_eq!(json!(before), res["rng_before"], "{what}: rng before");
                        let got = match &decision {
                            None => json!([]),
                            Some(d) => json!([[d.attacker, d.defender, d.territory, u8::from(d.attacker_won)]]),
                        };
                        assert_eq!(got, res["resolved"], "{what}: resolved");
                        assert_eq!(json!(u8::from(st.battle_play_requested)), res["play_requested_after"], "{what}: flag");
                        assert_eq!(json!(st.rng.state), res["rng_after"], "{what}: rng after");
                        match &decision {
                            None => bump("rts guard".into()),
                            Some(d) if d.solved.is_some() => bump("rts solved".into()),
                            Some(_) if r["winner"].as_str().unwrap().is_empty() => bump("rts local side rule".into()),
                            Some(_) => bump("rts winner name".into()),
                        }
                    }
                    Err(e) => {
                        assert!(res.get("fault").is_some(), "{what}: Rust error {e} but native decided {res}");
                        bump("rts unsupported".into());
                    }
                }
            }
        }
    }
    eprintln!("{counts:#?}");
    for key in ["launched", "launch unsupported", "participants unsupported", "rts solved", "rts local side rule",
                "rts winner name", "rts guard", "order set_merge_into", "order create_explicit_ai",
                "order set_faction", "order add_additional_actor", "world single24", "world extended64",
                "rmc settings PC24 != PC53 (proven at Single24)", "rmc settings PC24 != PC53 (proven at Double53)"] {
        assert!(counts.get(key).copied().unwrap_or(0) > 0, "no coverage of {key}");
    }
}

fn want_participants(battle: &Value) -> usize {
    battle["participants"]["participants"].as_array().map_or(0, |a| a.len())
}

/// 506370 on hand-made sliders: the PC24 / PC53 split of the midpoint
/// arithmetic and the integer midpoint truncation (cdq / sar).
#[test]
fn rmc_settings_integer_midpoint_truncates_toward_zero() {
    let s = RmcSliders { int_min: -7, int_max: 0, ..Default::default() };
    assert_eq!(rmc_settings(&s, Precision::Double53).unwrap().midpoint, -4);
    let s = RmcSliders { int_min: 0, int_max: 7, ..Default::default() };
    assert_eq!(rmc_settings(&s, Precision::Double53).unwrap().midpoint, 3);
}
