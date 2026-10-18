//! Replays research/phase8-metagame/native-turn.json: original turn advance
//! (952790 ProcessEndTurn, 9391F0, 938DA0, 938C90 with the territory turn
//! start 9556D0 auto-captures, 93FE10, income 959FB0) interleaved with
//! battle resolutions 954130 on perturbed InitCreate worlds of every shipped
//! campaign, with and without synthetic campaign events (95ADD0 rolls,
//! 95BE20 / 95BC80 fires, effects 95B220 / 95B400 / 95B630 / 95B7F0 /
//! 95B9B0). After every operation the whole metagame state (event states
//! included), the return value and the engine seam calls are compared with
//! the original.
use aa_metagame::battle::{DestroyBehavior, KingdomActor, KingdomRelations};
use aa_metagame::data::{CampaignEvent, EffectKind, MetaGameData};
use aa_metagame::resolve::{Engine, EngineCall, MgContext};
use aa_metagame::state::{ArmyId, MgState};
use aa_numeric::Precision;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

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

#[derive(Default)]
struct TestEngine {
    field19c: HashMap<u32, u32>,
    busy: HashSet<u32>,
    log: Vec<EngineCall>,
}
impl Engine for TestEngine {
    fn kingdom_field_19c(&self, actor: KingdomActor) -> u32 {
        self.field19c.get(&actor.0).copied().unwrap_or(0)
    }
    fn army_busy(&self, army: ArmyId) -> bool {
        self.busy.contains(&army.0)
    }
    fn call(&mut self, call: EngineCall) {
        self.log.push(call);
    }
}

fn matrix(v: &Value) -> Vec<Vec<bool>> {
    v.as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(|b| b.as_bool().unwrap()).collect()).collect()
}
fn opt(v: &Value) -> Option<usize> {
    v.as_u64().map(|x| x as usize)
}

fn dump(st: &MgState) -> Value {
    json!({
        "current": st.current,
        "round": st.round,
        "rng": st.rng.state,
        "flags": [u8::from(st.battle_started), u8::from(st.battle_play_requested), u8::from(st.battle_resolved), u8::from(st.game_over)],
        "results": st.results.iter().map(|r| json!([r.territory, r.attacker, r.defender, u8::from(r.attacker_won)])).collect::<Vec<_>>(),
        "kingdoms": st.kingdoms.iter().map(|k| json!([k.resources, k.battles, k.territories, k.technologies])).collect::<Vec<_>>(),
        "events": st.events.iter().map(|e| json!([u8::from(e.triggered), e.kingdom])).collect::<Vec<_>>(),
        "territories": st.territories.iter().map(|t| json!([t.owner, t.enemy_army_count,
            t.armies.iter().map(|r| json!([r.army.0, r.slot])).collect::<Vec<_>>()])).collect::<Vec<_>>(),
        "armies": st.armies.iter().map(|a| match a {
            None => Value::Null,
            Some(a) => json!([a.owner.0, a.territory, a.fought, a.path, a.waypoints]),
        }).collect::<Vec<_>>(),
    })
}

fn log_json(calls: &[EngineCall], data: &MetaGameData) -> Value {
    Value::Array(
        calls
            .iter()
            .map(|c| match c {
                EngineCall::BattleMessages { attacker, defender, territory, attacker_won } => {
                    json!(["messages", attacker, defender, territory, u8::from(*attacker_won)])
                }
                EngineCall::SetArmyOwner { army, owner } => json!(["set_owner", army.0, owner.0]),
                EngineCall::KillArmy { army } => json!(["kill", army.0]),
                EngineCall::KingdomVictory { actor } => json!(["victory", actor.0]),
                EngineCall::GameOver => json!(["game_over"]),
                EngineCall::MoveVisual { army, from, to } => json!(["move_visual", army.0, from, to]),
                EngineCall::EndTurnMessage { kingdom } => json!(["end_turn_message", kingdom]),
                EngineCall::CreateArmy { army, definition, owner } => {
                    json!(["create_army", army.0, data.armies[*definition].ids, owner.0])
                }
                EngineCall::EventMessage { event, kingdom } => json!(["event_message", event, kingdom]),
                EngineCall::EventEffectMessage { event, effect, kind, kingdom, army, territory, technology, source } => {
                    let army = army.map(|a| a.0);
                    match kind {
                        EffectKind::AddActor => json!(["effect_message", event, effect, "add_actor", army, kingdom, territory]),
                        EffectKind::RemoveActor => json!(["effect_message", event, effect, "remove_actor", army, kingdom, territory]),
                        EffectKind::AddTech => json!(["effect_message", event, effect, "add_tech", kingdom, technology]),
                        EffectKind::RemoveTech => json!(["effect_message", event, effect, "remove_tech", kingdom, technology]),
                        EffectKind::ChangeTerritory => {
                            json!(["effect_message", event, effect, "change_territory", kingdom, territory, source])
                        }
                        EffectKind::Unknown => panic!("message for an unknown effect"),
                    }
                }
            })
            .collect(),
    )
}

fn compare_state(got: &Value, want: &Value, what: &str) {
    for key in ["current", "round", "rng", "flags", "results", "kingdoms", "events"] {
        assert_eq!(got[key], want[key], "{what}: {key}");
    }
    for (i, (g, w)) in got["territories"].as_array().unwrap().iter().zip(want["territories"].as_array().unwrap()).enumerate() {
        assert_eq!(g, w, "{what}: territory {i}");
    }
    for (i, (g, w)) in got["armies"].as_array().unwrap().iter().zip(want["armies"].as_array().unwrap()).enumerate() {
        assert_eq!(g, w, "{what}: army {i}");
    }
    assert_eq!(got, want, "{what}");
}

fn precision(name: &str) -> Precision {
    match name {
        "single24" => Precision::Single24,
        "double53" => Precision::Double53,
        "extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}

/// Replays every world of a fixture; returns per-operation counts.
fn replay(fixture: &str) -> HashMap<String, usize> {
    let data_bytes = read("metagame-data.json");
    let base: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let fx: Value = serde_json::from_slice(&read(fixture)).unwrap();
    assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes));
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (wi, world) in fx["worlds"].as_array().unwrap().iter().enumerate() {
        let ci = world["campaign"].as_u64().unwrap() as usize;
        let mut data = base.clone();
        // synthetic campaign events (loader model), as the oracle laid them out
        let events: Vec<CampaignEvent> = serde_json::from_value(world["events"].clone()).unwrap();
        *counts.entry("events".into()).or_default() += events.len();
        data.campaigns[ci].events = events;
        let rel = Matrix { enemy: matrix(&world["enemy_140"]), r138: matrix(&world["relation_138"]) };
        let pc = precision(world["precision"].as_str().unwrap());
        *counts.entry(format!("world:{}", world["precision"].as_str().unwrap())).or_default() += 1;
        let mut st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
        let mut engine = TestEngine::default();
        for (oi, op) in world["ops"].as_array().unwrap().iter().enumerate() {
            if let Some(o) = op.as_array() {
                let v = |i: usize| o[i].as_i64().unwrap();
                match o[0].as_str().unwrap() {
                    "move" => {
                        st.enter_territory(ArmyId(v(1) as u32), v(2) as usize, &rel).unwrap();
                    }
                    "owner" => match opt(&o[2]) {
                        Some(k) => st.set_owner(v(1) as usize, k, &rel),
                        // synthetic unowned setup (955620(NULL) crashes natively)
                        None => st.clear_owner(v(1) as usize, &rel),
                    },
                    "fought" => st.armies[v(1) as usize].as_mut().unwrap().fought = v(2) as i32,
                    "battles" => st.kingdoms[v(1) as usize].battles = v(2) as i32,
                    "resources" => st.kingdoms[v(1) as usize].resources = v(2) as i32,
                    "current" => st.current = Some(v(1) as usize),
                    "rng" => st.rng.state = v(1) as u32,
                    "game_over" => st.game_over = v(1) != 0,
                    "behaviours" => {
                        let b = |x: &Value| -> DestroyBehavior { serde_json::from_value(x.clone()).unwrap() };
                        data.campaigns[ci].attacking_destroy_behavior = b(&o[1]);
                        data.campaigns[ci].defending_destroy_behavior = b(&o[2]);
                    }
                    "field19c" => {
                        engine.field19c.insert(v(1) as u32, v(2) as u32);
                    }
                    "busy" => {
                        if v(2) != 0 {
                            engine.busy.insert(v(1) as u32);
                        } else {
                            engine.busy.remove(&(v(1) as u32));
                        }
                    }
                    "push_waypoint" => st.armies[v(1) as usize].as_mut().unwrap().waypoints.push(v(2) as usize),
                    "technologies" => {
                        st.kingdoms[v(1) as usize].technologies =
                            o[2].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_string()).collect();
                    }
                    "event_state" => {
                        let e = &mut st.events[v(1) as usize];
                        e.triggered = v(2) != 0;
                        e.kingdom = opt(&o[3]);
                    }
                    "round" => st.round = v(1) as i32,
                    "army_owner" => {
                        let a = st.armies[v(1) as usize].as_mut().unwrap();
                        a.owner = KingdomActor(v(2) as u32);
                        let t = a.territory.unwrap();
                        st.recount_enemies(t, &rel);
                    }
                    other => panic!("op {other}"),
                }
                continue;
            }
            let o = op["op"].as_array().unwrap();
            let kind = o[0].as_str().unwrap();
            let what = format!("world {wi} (campaign {ci}) op {oi}: {}", Value::Array(o.clone()));
            engine.log.clear();
            let mut ctx = MgContext { state: &mut st, data: &data, relations: &rel, engine: &mut engine, precision: pc };
            let result: Result<Value, aa_metagame::Error> = match kind {
                "resolve" => ctx
                    .resolve_battle(opt(&o[1]), opt(&o[2]), o[3].as_u64().unwrap() as usize, o[4].as_u64().unwrap() != 0)
                    .map(|r| json!(u8::from(r))),
                "quick" => ctx.process_quick_battle().map(|r| json!(u8::from(r))),
                "step" => ctx.step_orders(ArmyId(o[1].as_u64().unwrap() as u32)).map(|_| Value::Null),
                "order" => Ok(json!(u8::from(ctx.set_order(ArmyId(o[1].as_u64().unwrap() as u32), o[2].as_u64().unwrap() as usize)))),
                "end_turn" => ctx.process_end_turn(KingdomActor(o[1].as_u64().unwrap() as u32)).map(|r| json!(u8::from(r))),
                "advance" => ctx.next_kingdom().map(|_| Value::Null),
                "advance_playable" => ctx.advance_turn().map(|_| Value::Null),
                other => panic!("op {other}"),
            };
            if let Some(why) = op.get("unsupported") {
                match result {
                    Err(aa_metagame::Error::Unsupported(_)) => {}
                    other => panic!("{what}: expected Unsupported ({why}), got {other:?}"),
                }
                *counts.entry("native_crash_unsupported".into()).or_default() += 1;
                break;
            }
            if op.get("null_order").is_some() {
                match result {
                    Err(aa_metagame::Error::Unsupported(_)) => {}
                    other => panic!("{what}: expected Unsupported for a null retreat order, got {other:?}"),
                }
                *counts.entry("null_order_unsupported".into()).or_default() += 1;
                break;
            }
            let result = result.unwrap_or_else(|e| panic!("{what}: {e}"));
            assert_eq!(result, op["result"], "{what}: result");
            assert_eq!(log_json(&engine.log, &data), op["log"], "{what}: engine calls");
            compare_state(&dump(&st), &op["state"], &what);
            *counts.entry(kind.to_string()).or_default() += 1;
            for c in &engine.log {
                let tag = match c {
                    EngineCall::KillArmy { .. } => "kill",
                    EngineCall::SetArmyOwner { .. } => "capture",
                    EngineCall::GameOver => "game_over",
                    EngineCall::CreateArmy { .. } => "create_army",
                    EngineCall::EventMessage { .. } => "event_message",
                    EngineCall::EventEffectMessage { .. } => "effect_message",
                    _ => continue,
                };
                *counts.entry(tag.into()).or_default() += 1;
            }
        }
    }
    counts
}

#[test]
fn turn_advance_matches_native_952790_9391f0_938c90_9556d0() {
    let counts = replay("native-turn.json");
    eprintln!("{counts:?}");
    for (key, min) in [
        ("end_turn", 100),
        ("advance", 50),
        ("advance_playable", 40),
        ("resolve", 20),
        ("events", 40),
        ("create_army", 50),
        ("effect_message", 100),
        ("event_message", 50),
        ("native_crash_unsupported", 1),
    ] {
        assert!(counts.get(key).copied().unwrap_or(0) >= min, "{key}: {counts:?}");
    }
}
