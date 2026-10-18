//! Replays research/phase8-metagame/native-mgai-controller-c*.json: the
//! original MGAI controller fiber 945090 (players 944F90, PlayTurn 964D30,
//! yields 944CE0, queue 944F50 / flush 944E60) with the main fiber's work at
//! every switch: the flushed orders and scripted engine orders executed by
//! the original 9505F0 (951D70 + 954570), the waypoint mover 940020, actor
//! handle invalidation and kingdom army list rebuilds. The Rust side runs
//! `MgaiController::run` with an engine that replays the recorded switches
//! through `controller::execute_order` and `MgContext::step_orders`, and is
//! compared after every switch (orders, engine calls, state digest) and at
//! every PlayTurn stage (analysis, goals).
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::commands::{Command, CommandData};
use aa_metagame::data::{AiDefinition, MetaGameData};
use aa_metagame::mgai::controller::{execute_order, flush, MgaiController, MgaiEngine, OrderEffect, SaiResume};
use aa_metagame::mgai::{Analysis, Goal, KingdomArmy, MgaiPlayer, RecordRef, Stage};
use aa_metagame::resolve::{Engine, EngineCall, MgContext};
use aa_metagame::rng::MetaGameRng;
use aa_metagame::state::{ArmyId, MgState};
use aa_numeric::Precision;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-metagame")
}

struct Relations {
    enemy: RefCell<Vec<Vec<bool>>>,
    r138: RefCell<Vec<Vec<bool>>>,
}
impl KingdomRelations for Relations {
    fn enemy(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.enemy.borrow()[s.0 as usize][o.0 as usize]
    }
    fn relation_138(&self, s: KingdomActor, o: KingdomActor) -> bool {
        self.r138.borrow()[s.0 as usize][o.0 as usize]
    }
}
fn matrix(v: &Value) -> Vec<Vec<bool>> {
    v.as_array().unwrap().iter().map(|r| r.as_array().unwrap().iter().map(|x| x.as_bool().unwrap()).collect()).collect()
}
fn us(v: &Value) -> Vec<usize> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect()
}

/// Native army index <-> Rust army id bookkeeping (respawns keep the Rust id).
#[derive(Default)]
struct Ids {
    native: BTreeMap<usize, ArmyId>,
    serial: BTreeMap<ArmyId, u32>,
}

/// Engine calls of the resolution / mover code; busy answers shared with the MGAI.
struct Calls<'a> {
    busy: &'a [Value],
    bi: &'a Cell<usize>,
    calls: Vec<EngineCall>,
}
impl Engine for Calls<'_> {
    fn kingdom_field_19c(&self, _: KingdomActor) -> u32 {
        0
    }
    fn army_busy(&self, army: ArmyId) -> bool {
        busy_answer(self.busy, self.bi, army, None)
    }
    fn call(&mut self, call: EngineCall) {
        self.calls.push(call);
    }
}

fn busy_answer(busy: &[Value], bi: &Cell<usize>, army: ArmyId, ids: Option<&Ids>) -> bool {
    let b = &busy[bi.get()];
    bi.set(bi.get() + 1);
    if let Some(ids) = ids {
        assert_eq!(ids.native[&(b[0].as_u64().unwrap() as usize)], army, "busy query army");
    }
    b[1].as_u64().unwrap() != 0
}

/// Per goal: threshold bits, then items (territory, priority bits, assigned army handles).
type Goals = Vec<(u32, Vec<(usize, u32, Vec<Option<u32>>)>)>;

struct Snap {
    kingdom: usize,
    stage: Stage,
    yields: usize,
    analysis: Analysis,
    goals: Goals,
    flags: [bool; 2],
}

struct Host<'a> {
    data: &'a MetaGameData,
    rel: &'a Relations,
    precision: Precision,
    world: &'a Value,
    tgc: Vec<String>,
    lists: Vec<Vec<usize>>,
    ids: Ids,
    yi: usize,
    busy: &'a [Value],
    bi: &'a Cell<usize>,
    uninit: &'a [Value],
    ui: usize,
    tick: i32,
    rng: MetaGameRng,
    snaps: Vec<Snap>,
    stats: BTreeMap<String, usize>,
}

fn stage_name(s: Stage) -> &'static str {
    match s {
        Stage::Analysis => "analysis",
        Stage::Goals1 => "goals1",
        Stage::Purchase => "purchase",
        Stage::Goals2 => "goals2",
        Stage::Attack => "attack",
        Stage::Goals3 => "goals3",
        Stage::Move => "move",
        Stage::End => "end",
    }
}

fn command_from(data: &MetaGameData, map: usize, v: &Value, ids: &Ids) -> Command {
    let index = v["index"].as_u64().unwrap() as u32;
    let mut c = Command { data: Some(CommandData::builtin(index)), ..Command::default() };
    if let Some(a) = v.get("army") {
        c.army = a.as_u64().map(|a| ids.native[&(a as usize)]);
    }
    if let Some(t) = v.get("territory_def") {
        c.set_territory(data, map, t.as_u64().unwrap() as usize);
    }
    if let Some(t) = v.get("tech") {
        c.technology = Some(t.as_str().unwrap().to_string());
    }
    if let Some(a) = v.get("actor_def") {
        c.actor_definition = Some(data.armies.iter().position(|x| x.ids == a.as_str().unwrap()).unwrap());
    }
    if let Some(d) = v.get("divisions") {
        c.divisions = Some(us(d));
    }
    c
}

fn check_command(data: &MetaGameData, got: &Command, want: &Value, ids: &Ids, ctx: &str) {
    assert_eq!(u64::from(got.presence()), want["presence"].as_u64().unwrap(), "{ctx} presence {got:?} {want}");
    assert_eq!(got.data.map(|d| u64::from(d.index)), want["index"].as_u64(), "{ctx} index");
    if let Some(a) = want.get("army") {
        assert_eq!(got.army, a.as_u64().map(|a| ids.native[&(a as usize)]), "{ctx} army");
    }
    if let Some(a) = want.get("actor_def") {
        assert_eq!(got.actor_definition.map(|d| data.armies[d].ids.as_str()), a.as_str(), "{ctx} actor def");
    }
    if let Some(t) = want.get("territory_def") {
        assert_eq!(got.territory.map(|d| d as u64), t.as_u64(), "{ctx} territory");
    }
    if let Some(t) = want.get("tech") {
        assert_eq!(got.technology.as_deref(), t.as_str(), "{ctx} tech");
    }
    if let Some(d) = want.get("divisions") {
        assert_eq!(got.divisions.clone().unwrap_or_default(), us(d), "{ctx} divisions");
    }
}

fn engine_call_json(c: &EngineCall, ids: &Ids) -> Value {
    let native = |a: ArmyId| -> Value {
        // the live native index of a Rust army (the newest mapping)
        json!(ids.native.iter().filter(|(_, v)| **v == a).map(|(k, _)| *k).max().unwrap())
    };
    match c {
        EngineCall::BattleMessages { attacker, defender, territory, attacker_won } => {
            json!(["messages", attacker, defender, territory, u8::from(*attacker_won)])
        }
        EngineCall::SetArmyOwner { army, owner } => json!(["set_owner", native(*army), owner.0]),
        EngineCall::KillArmy { army } => json!(["kill", native(*army)]),
        EngineCall::KingdomVictory { actor } => json!(["victory", actor.0]),
        EngineCall::GameOver => json!(["game_over"]),
        EngineCall::MoveVisual { army, from, to } => json!(["move_visual", native(*army), from, to]),
        EngineCall::EndTurnMessage { kingdom } => json!(["end_turn_message", kingdom]),
        other => json!(["unexpected", format!("{other:?}")]),
    }
}

impl Host<'_> {
    fn live_index(&self, id: ArmyId) -> usize {
        self.ids.native.iter().filter(|(_, v)| **v == id).map(|(k, _)| *k).max().unwrap()
    }

    fn digest(&self, st: &MgState, want: &Value, ctx: &str) {
        assert_eq!(st.current.map(|c| c as u64), want["current"].as_u64(), "{ctx} current");
        assert_eq!(i64::from(st.round), want["round"].as_i64().unwrap(), "{ctx} round");
        assert_eq!(u64::from(st.rng.state), want["rng"].as_u64().unwrap(), "{ctx} rng");
        let flags = [st.battle_started, st.battle_play_requested, st.battle_resolved, st.game_over].map(u64::from);
        let wf: Vec<u64> = want["flags"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect();
        assert_eq!(flags.to_vec(), wf, "{ctx} flags");
        for (k, (g, w)) in st.kingdoms.iter().zip(want["kingdoms"].as_array().unwrap()).enumerate() {
            assert_eq!(i64::from(g.resources), w[0].as_i64().unwrap(), "{ctx} kingdom {k} resources");
            assert_eq!(i64::from(g.battles), w[1].as_i64().unwrap(), "{ctx} kingdom {k} battles");
            assert_eq!(g.territories, us(&w[2]), "{ctx} kingdom {k} territories");
            let techs: Vec<String> = w[3].as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_string()).collect();
            assert_eq!(g.technologies, techs, "{ctx} kingdom {k} technologies");
        }
        for (t, (g, w)) in st.territories.iter().zip(want["territories"].as_array().unwrap()).enumerate() {
            assert_eq!(g.owner.map(|o| o as u64), w[0].as_u64(), "{ctx} territory {t} owner");
            assert_eq!(i64::from(g.enemy_army_count), w[1].as_i64().unwrap(), "{ctx} territory {t} +1C");
            let refs: Vec<(Option<u32>, i64)> = w[2]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| (r[0].as_u64().map(|a| self.ids.native[&(a as usize)].0), r[1].as_i64().unwrap()))
                .collect();
            let got: Vec<(Option<u32>, i64)> = g.armies.iter().map(|r| (Some(r.army.0), i64::from(r.slot))).collect();
            assert_eq!(got, refs, "{ctx} territory {t} armies");
        }
        let wa = want["armies"].as_array().unwrap();
        let mut seen = std::collections::BTreeSet::new();
        for (i, w) in wa.iter().enumerate() {
            if w.is_null() {
                continue;
            }
            let id = self.ids.native[&i];
            seen.insert(id);
            let a = st.armies[id.0 as usize].as_ref().unwrap_or_else(|| panic!("{ctx} army {i} dead in Rust"));
            assert_eq!(u64::from(a.owner.0), w[0].as_u64().unwrap(), "{ctx} army {i} owner");
            assert_eq!(a.territory.map(|t| t as u64), w[1].as_u64(), "{ctx} army {i} territory");
            assert_eq!(i64::from(a.fought), w[2].as_i64().unwrap(), "{ctx} army {i} fought");
            assert_eq!(a.path, us(&w[3]), "{ctx} army {i} path");
            assert_eq!(a.waypoints, us(&w[4]), "{ctx} army {i} waypoints");
            let divs: Vec<usize> = a.divisions.iter().map(|d| d.0 as usize).collect();
            assert_eq!(divs, us(&w[5]), "{ctx} army {i} divisions");
            assert_eq!(self.data.armies[a.definition].ids, w[6].as_str().unwrap(), "{ctx} army {i} type");
        }
        let live: std::collections::BTreeSet<ArmyId> =
            st.armies.iter().enumerate().filter(|(_, a)| a.is_some()).map(|(i, _)| ArmyId(i as u32)).collect();
        assert_eq!(live, seen, "{ctx} live armies");
    }
}

impl MgaiEngine for Host<'_> {
    fn kingdom_armies(&self, _: &MgState, actor: KingdomActor) -> aa_metagame::Result<Vec<KingdomArmy>> {
        Ok(self.lists[actor.0 as usize]
            .iter()
            .map(|&i| KingdomArmy { army: self.ids.native[&i], deleted: false })
            .collect())
    }
    fn army_serial(&self, _: &MgState, army: ArmyId) -> u32 {
        self.ids.serial.get(&army).copied().unwrap_or(0)
    }
    fn tick(&self) -> i32 {
        self.tick
    }
    fn army_busy(&mut self, _: &MgState, army: ArmyId) -> aa_metagame::Result<bool> {
        Ok(busy_answer(self.busy, self.bi, army, Some(&self.ids)))
    }
    fn uninitialized_nonzero(&mut self, len: usize) -> aa_metagame::Result<usize> {
        let u = &self.uninit[self.ui];
        self.ui += 1;
        assert_eq!(u[0].as_u64().unwrap() as usize, len, "96A5D0 block length");
        Ok(u[1].as_u64().unwrap() as usize)
    }
    fn app_rng(&mut self) -> &mut MetaGameRng {
        &mut self.rng
    }
    fn kingdom_player(&self, actor: Option<KingdomActor>) -> Option<bool> {
        match self.tgc[actor.expect("kingdom actor").0 as usize].as_str() {
            "none" => None,
            "computer" => Some(true),
            "human" => Some(false),
            other => panic!("{other}"),
        }
    }
    fn sai_yield(&mut self, state: &mut MgState, arg: u8, queue: &mut Vec<Command>) -> aa_metagame::Result<SaiResume> {
        let rec = &self.world["yields"][self.yi];
        let ctx = format!("c{} yield {}", state.campaign, self.yi);
        self.yi += 1;
        assert_eq!(rec["arg"].as_u64().unwrap(), u64::from(arg), "{ctx} argument");
        if rec.get("stop").is_some() {
            return Ok(SaiResume::Abandoned);
        }
        if rec["switched"].as_u64().unwrap() != 0 {
            *self.stats.entry("switches".into()).or_default() += 1;
            self.tick = rec["tick"].as_u64().unwrap() as u32 as i32;
            let flushed = flush(queue);
            let map = self.data.campaigns[state.campaign].map.unwrap();
            let mut ai = flushed.into_iter();
            for (oi, o) in rec["orders"].as_array().unwrap().iter().enumerate() {
                let octx = format!("{ctx} order {oi}");
                let cmd = if o["source"] == "ai" {
                    let c = ai.next().unwrap_or_else(|| panic!("{octx}: fewer flushed commands"));
                    check_command(self.data, &c, &o["command"], &self.ids, &octx);
                    c
                } else {
                    command_from(self.data, map, &o["command"], &self.ids)
                };
                let issuer = KingdomActor(o["issuer"].as_u64().unwrap() as u32);
                let owners: Vec<Option<KingdomActor>> = state.armies.iter().map(|a| a.as_ref().map(|a| a.owner)).collect();
                let controllable = move |a: ArmyId, i: KingdomActor| owners[a.0 as usize] == Some(i);
                let mut calls = Calls { busy: self.busy, bi: self.bi, calls: Vec::new() };
                let r = execute_order(state, self.data, self.rel, self.precision, &cmd, issuer, controllable, &mut calls)
                    .unwrap_or_else(|e| panic!("{octx}: {e}"));
                assert_eq!(u64::from(r.code), o["code"].as_u64().unwrap(), "{octx} code {cmd:?}");
                *self.stats.entry(format!("order {} code {}", cmd.data.unwrap().index, r.code)).or_default() += 1;
                let mut messages: Vec<u32> = Vec::new();
                match r.effect {
                    None => assert!(o.get("result").is_none(), "{octx}"),
                    Some(OrderEffect::Process(out)) => {
                        assert_eq!(u64::from(out.result), o["result"].as_u64().unwrap(), "{octx} result");
                        messages = out.messages.clone();
                        let created = us(&o["created"]);
                        match out.created {
                            Some(id) => {
                                assert_eq!(created.len(), 1, "{octx} created");
                                self.ids.native.insert(created[0], id);
                            }
                            None => assert!(created.is_empty(), "{octx} created"),
                        }
                        match (out.respawned, o.get("respawned")) {
                            (Some(id), Some(pair)) => {
                                assert_eq!(self.ids.native[&(pair[0].as_u64().unwrap() as usize)], id, "{octx} respawn");
                                self.ids.native.insert(pair[1].as_u64().unwrap() as usize, id);
                                *self.ids.serial.entry(id).or_default() += 1;
                            }
                            (None, None) => {}
                            other => panic!("{octx} respawn {other:?}"),
                        }
                    }
                    Some(OrderEffect::EndTurn(b)) | Some(OrderEffect::QuickBattle(b)) => {
                        assert_eq!(u64::from(b), o["result"].as_u64().unwrap(), "{octx} result");
                        assert!(us(&o["created"]).is_empty() && o.get("respawned").is_none(), "{octx}");
                    }
                }
                if r.code == 0 {
                    let wm: Vec<u32> = o["messages"].as_array().unwrap().iter().map(|m| m.as_u64().unwrap() as u32).collect();
                    // only the command processors report message categories
                    if matches!(cmd.data.unwrap().index, 0..=4 | 7..=10) {
                        assert_eq!(messages, wm, "{octx} messages");
                    }
                }
                let got: Vec<Value> = calls.calls.iter().map(|c| engine_call_json(c, &self.ids)).collect();
                assert_eq!(Value::Array(got), o["engine"], "{octx} engine calls");
            }
            assert!(ai.next().is_none(), "{ctx}: more flushed commands than recorded orders");
            for s in rec["steps"].as_array().unwrap() {
                let a = self.ids.native[&(s[0].as_u64().unwrap() as usize)];
                let mut calls = Calls { busy: self.busy, bi: self.bi, calls: Vec::new() };
                MgContext { state: &mut *state, data: self.data, relations: self.rel, engine: &mut calls, precision: self.precision }
                    .step_orders(a)
                    .unwrap();
                let got: Vec<Value> = calls.calls.iter().map(|c| engine_call_json(c, &self.ids)).collect();
                assert_eq!(Value::Array(got), s[1], "{ctx} step");
                *self.stats.entry("steps".into()).or_default() += 1;
            }
            self.lists = rec["lists"].as_array().unwrap().iter().map(us).collect();
            if let Some(d) = rec.get("digest") {
                self.digest(state, d, &ctx);
            }
        }
        let replaced = rec["replaced"].as_u64().unwrap() != 0;
        if replaced {
            *self.stats.entry("replaced".into()).or_default() += 1;
        }
        Ok(SaiResume::Resumed { state_replaced: replaced })
    }
    fn stage(&mut self, stage: Stage, player: &MgaiPlayer, state: &MgState) {
        let goals = player
            .goals
            .iter()
            .map(|g: &Goal| {
                (
                    g.threshold_bits,
                    g.items
                        .iter()
                        .map(|it| {
                            let armies = it
                                .armies
                                .iter()
                                .map(|h| {
                                    (state.armies.get(h.army.0 as usize).is_some_and(Option::is_some)
                                        && self.army_serial(state, h.army) == h.serial)
                                        .then(|| self.live_index(h.army) as u32)
                                })
                                .collect();
                            (it.territory, it.priority_bits, armies)
                        })
                        .collect(),
                )
            })
            .collect();
        self.snaps.push(Snap {
            kingdom: player.kingdom,
            stage,
            yields: self.yi,
            analysis: player.analysis.clone(),
            goals,
            flags: [player.attacked, player.turn_ended],
        });
    }
}

fn record_label(r: RecordRef) -> Value {
    match r {
        RecordRef::Default => json!(["default", null]),
        RecordRef::Capital => json!(["capital", null]),
        RecordRef::FriendlyCapital => json!(["friendly_capital", null]),
        RecordRef::EnemyCapital => json!(["enemy_capital", null]),
        RecordRef::Territory(i) => json!(["territory", i]),
    }
}

fn check_analysis(got: &Analysis, want: &Value, ctx: &str) {
    let wt = want["territories"].as_array().unwrap();
    assert_eq!(got.territories.len(), wt.len(), "{ctx}");
    for (i, (g, w)) in got.territories.iter().zip(wt).enumerate() {
        assert_eq!(record_label(g.record), w["record"], "{ctx} territory {i} record");
        let infl: Vec<u32> = w["influence"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
        assert_eq!(g.influence, infl, "{ctx} territory {i} influence");
        assert_eq!(i64::from(g.importance), w["importance"].as_i64().unwrap(), "{ctx} territory {i} importance");
        assert_eq!(u64::from(g.border), w["border"].as_u64().unwrap(), "{ctx} territory {i} border");
        assert_eq!(u64::from(g.front), w["front"].as_u64().unwrap(), "{ctx} territory {i} front");
        assert_eq!(i64::from(g.proximity), w["proximity"].as_i64().unwrap(), "{ctx} territory {i} proximity");
    }
    assert_eq!(got.borders, us(&want["borders"]), "{ctx} borders");
    assert_eq!(got.fronts, us(&want["fronts"]), "{ctx} fronts");
    assert_eq!(u64::from(got.proximity_done), want["proximity_done"].as_u64().unwrap(), "{ctx} proximity done");
}

#[test]
fn mgai_controller_matches_native_945090() {
    let data_bytes = std::fs::read(dir().join("metagame-data.json")).unwrap();
    let base: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let mut files: Vec<_> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().to_string();
            n.starts_with("native-mgai-controller-c") && n.ends_with(".json")
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no native-mgai-controller-c*.json fixture");
    let mut stats: BTreeMap<String, usize> = BTreeMap::new();
    for file in &files {
        let fx: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes), "{file:?} data");
        for (wi, world) in fx["worlds"].as_array().unwrap().iter().enumerate() {
            let ci = world["campaign"].as_u64().unwrap() as usize;
            let precision = match world["cw"].as_str().unwrap() {
                "pc24" => Precision::Single24,
                "pc53" => Precision::Double53,
                "pc64" => Precision::Extended64,
                other => panic!("{other}"),
            };
            let mut data = base.clone();
            for (k, v) in world["ai"].as_object().unwrap() {
                let ai: AiDefinition = serde_json::from_value(v.clone()).unwrap();
                data.ai[k.parse::<usize>().unwrap()] = ai;
            }
            let c = &data.campaigns[ci];
            let mut team = vec![None; c.kingdoms.len()];
            for (ti, t) in c.teams.iter().enumerate() {
                for &k in &t.kingdoms {
                    team[k] = Some(ti);
                }
            }
            let nk = c.kingdoms.len();
            let rel = Relations {
                enemy: RefCell::new((0..nk).map(|a| (0..nk).map(|b| team[a] != team[b]).collect()).collect()),
                r138: RefCell::new((0..nk).map(|a| (0..nk).map(|b| team[a] == team[b]).collect()).collect()),
            };
            let mut st = MgState::init_create(&data, ci, |i| Some(KingdomActor(i as u32)), &rel).unwrap();
            let mut lists = vec![Vec::new(); nk];
            for op in world["ops"].as_array().unwrap() {
                let v = |i: usize| op[i].as_i64().unwrap();
                match op[0].as_str().unwrap() {
                    "relations" => {
                        *rel.enemy.borrow_mut() = matrix(&op[1]);
                        *rel.r138.borrow_mut() = matrix(&op[2]);
                    }
                    "owner" => st.set_owner(v(1) as usize, v(2) as usize, &rel),
                    "move" => {
                        st.enter_territory(ArmyId(v(1) as u32), v(2) as usize, &rel).unwrap();
                    }
                    "turn_start" => {
                        let a = st.armies[v(1) as usize].as_mut().unwrap();
                        a.fought = 0;
                        a.path = vec![a.territory.unwrap()];
                    }
                    "resources" => st.kingdoms[v(1) as usize].resources = v(2) as i32,
                    "battles" => st.kingdoms[v(1) as usize].battles = v(2) as i32,
                    "current" => st.current = Some(v(1) as usize),
                    "flag51" => st.fast_ai = v(1) as u8,
                    "lists" => lists = op[1].as_array().unwrap().iter().map(us).collect(),
                    other => panic!("op {other}"),
                }
            }
            let busy = world["busy"].as_array().unwrap();
            let bi = Cell::new(0usize);
            let mut ids = Ids::default();
            for i in 0..st.armies.len() {
                ids.native.insert(i, ArmyId(i as u32));
            }
            let mut host = Host {
                data: &data,
                rel: &rel,
                precision,
                world,
                tgc: world["tgc"].as_array().unwrap().iter().map(|t| t.as_str().unwrap().to_string()).collect(),
                lists,
                ids,
                yi: 0,
                busy,
                bi: &bi,
                uninit: world["uninit"].as_array().unwrap(),
                ui: 0,
                tick: world["tick_in"].as_u64().unwrap() as u32 as i32,
                rng: MetaGameRng::new(world["rng_in"].as_u64().unwrap() as u32),
                snaps: Vec::new(),
                stats: BTreeMap::new(),
            };
            let ctx = format!("{file:?} world {wi}");
            let mut controller = MgaiController::new();
            controller.run(&mut st, &data, &rel, precision, &mut host).unwrap_or_else(|e| panic!("{ctx}: {e}"));
            assert_eq!(host.yi, world["yields"].as_array().unwrap().len(), "{ctx} yields consumed");
            assert_eq!(bi.get(), busy.len(), "{ctx} busy consumed");
            assert_eq!(host.ui, host.uninit.len(), "{ctx} 96A5D0 blocks consumed");
            assert_eq!(u64::from(host.rng.state), world["rng_out"].as_u64().unwrap(), "{ctx} app rng");
            let ws = world["stages"].as_array().unwrap();
            assert_eq!(host.snaps.len(), ws.len(), "{ctx} stage count");
            for (si, (s, w)) in host.snaps.iter().zip(ws).enumerate() {
                let sctx = format!("{ctx} stage {si} {} k{}", stage_name(s.stage), s.kingdom);
                assert_eq!(stage_name(s.stage), w["stage"].as_str().unwrap(), "{sctx}");
                assert_eq!(s.kingdom as u64, w["kingdom"].as_u64().unwrap(), "{sctx} kingdom");
                assert_eq!(s.yields as u64, w["yield"].as_u64().unwrap(), "{sctx} yield count");
                if !w["analysis"].is_null() {
                    check_analysis(&s.analysis, &w["analysis"], &sctx);
                }
                let want: Goals = w["goals"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|g| {
                        (
                            g["threshold"].as_u64().unwrap() as u32,
                            g["items"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|it| {
                                    (
                                        it["territory"].as_u64().unwrap() as usize,
                                        it["priority"].as_u64().unwrap() as u32,
                                        it["armies"].as_array().unwrap().iter().map(|a| a.as_u64().map(|v| v as u32)).collect(),
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect();
                assert_eq!(s.goals, want, "{sctx} goals");
                let flags: Vec<u64> = w["flags"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect();
                assert_eq!(flags, [u64::from(s.flags[0]), u64::from(s.flags[1])], "{sctx} flags");
            }
            host.digest(&st, &world["final"], &format!("{ctx} final"));
            let players: Vec<Vec<u64>> = controller.players.iter().map(|p| vec![u64::from(p.attacked), u64::from(p.turn_ended)]).collect();
            let wp: Vec<Vec<u64>> = world["players"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p.as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect())
                .collect();
            assert_eq!(players, wp, "{ctx} players");
            assert_eq!(controller.queue.len() as u64, world["queue"].as_u64().unwrap(), "{ctx} queue");
            *stats.entry("worlds".into()).or_default() += 1;
            *stats.entry("stages".into()).or_default() += ws.len();
            *stats.entry("yields".into()).or_default() += host.yi;
            for (k, v) in host.stats {
                *stats.entry(k).or_default() += v;
            }
        }
    }
    eprintln!("mgai controller: {stats:?}");
}
