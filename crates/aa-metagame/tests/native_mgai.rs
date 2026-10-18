//! Replays research/phase8-metagame/native-mgai-c*.json: the original
//! KKC_MGAIPlayer (constructor 963430, PlayTurn 964D30 with DoMapAnalysis
//! 962B70, SetGoalPriorities 9635A0, DoPurchasing 963B70, PossiblyAttack
//! 963FD0, 9649A0, Move 9649C0, EndTurn 962A40) under Unicorn on perturbed
//! InitCreate worlds. Every engine seam (yields, command queue, ticks, busy
//! armies, kingdom army lists, the uninitialised 96A5D0 block, the app RNG)
//! is replayed from the fixture; analyses, goals (f32 bits), issued commands
//! and the RNG word are compared after every PlayTurn stage.
use aa_metagame::battle::{KingdomActor, KingdomRelations};
use aa_metagame::commands::Command;
use aa_metagame::data::{AiDefinition, MetaGameData};
use aa_metagame::mgai::{Analysis, Goal, KingdomArmy, MgaiHost, MgaiPlayer, RecordRef, Stage, View};
use aa_metagame::rng::MetaGameRng;
use aa_metagame::state::{ArmyId, MgState};
use aa_numeric::Precision;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::BTreeSet;

fn dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-metagame")
}

// native_mgai.py writes native-mgai-c<campaign>.json. The controller oracle
// writes native-mgai-controller-c<campaign>.json with a different world schema;
// it has its own replay test and must not enter this PlayTurn fixture reader.
fn is_player_fixture(name: &str) -> bool {
    name.strip_prefix("native-mgai-c")
        .and_then(|suffix| suffix.strip_suffix(".json"))
        .is_some_and(|campaign| !campaign.is_empty() && campaign.bytes().all(|c| c.is_ascii_digit()))
}

#[test]
fn player_fixture_discovery_excludes_controller_oracles() {
    assert!(is_player_fixture("native-mgai-c0.json"));
    assert!(is_player_fixture("native-mgai-c42.json"));
    assert!(!is_player_fixture("native-mgai-controller-c0.json"));
    assert!(!is_player_fixture("native-mgai-c.json"));
    assert!(!is_player_fixture("native-mgai-c0.json.bak"));
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

struct Snapshot {
    stage: Stage,
    analysis: Analysis,
    goals: Vec<Goal>,
    flags: [bool; 2],
    issued: usize,
    yields: usize,
}

struct Host<'a> {
    relations: &'a Relations,
    lists: &'a [Vec<usize>],
    deleted: &'a BTreeSet<usize>,
    yields: &'a [Value],
    busy: &'a [Value],
    uninit: &'a [Value],
    yi: usize,
    bi: usize,
    ui: usize,
    tick: i32,
    rng: MetaGameRng,
    issued: Vec<Command>,
    snaps: Vec<Snapshot>,
}
impl MgaiHost for Host<'_> {
    fn kingdom_armies(&self, _: &MgState, actor: KingdomActor) -> aa_metagame::Result<Vec<KingdomArmy>> {
        Ok(self.lists[actor.0 as usize]
            .iter()
            .map(|&i| KingdomArmy { army: ArmyId(i as u32), deleted: self.deleted.contains(&i) })
            .collect())
    }
    fn yield_turn(&mut self, state: &mut MgState, arg: u8) -> aa_metagame::Result<bool> {
        let y = &self.yields[self.yi];
        self.yi += 1;
        assert_eq!(y[0].as_u64().unwrap(), u64::from(arg), "yield {} argument", self.yi - 1);
        self.tick = y[3].as_u64().unwrap() as u32 as i32;
        if y[2].as_u64().unwrap() != 0 {
            state.battle_resolved = true;
        }
        // scripted engine progress: relocation by 940100
        if let Some(a) = y.get(4).filter(|a| !a.is_null()) {
            state.enter_territory(ArmyId(a[0].as_u64().unwrap() as u32), a[1].as_u64().unwrap() as usize, self.relations)?;
        }
        Ok(y[1].as_u64().unwrap() != 0)
    }
    fn issue(&mut self, _: &MgState, command: Command) -> aa_metagame::Result<()> {
        self.issued.push(command);
        Ok(())
    }
    fn tick(&self) -> i32 {
        self.tick
    }
    fn army_busy(&mut self, _: &MgState, army: ArmyId) -> aa_metagame::Result<bool> {
        let b = &self.busy[self.bi];
        self.bi += 1;
        assert_eq!(b[0].as_u64().unwrap(), u64::from(army.0), "busy query army");
        Ok(b[1].as_u64().unwrap() != 0)
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
    fn stage(&mut self, stage: Stage, player: &MgaiPlayer, _: &MgState) {
        self.snaps.push(Snapshot {
            stage,
            analysis: player.analysis.clone(),
            goals: player.goals.to_vec(),
            flags: [player.attacked, player.turn_ended],
            issued: self.issued.len(),
            yields: self.yi,
        });
    }
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

fn record_label(r: RecordRef) -> Value {
    match r {
        RecordRef::Default => serde_json::json!(["default", null]),
        RecordRef::Capital => serde_json::json!(["capital", null]),
        RecordRef::FriendlyCapital => serde_json::json!(["friendly_capital", null]),
        RecordRef::EnemyCapital => serde_json::json!(["enemy_capital", null]),
        RecordRef::Territory(i) => serde_json::json!(["territory", i]),
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
    let us = |v: &Value| -> Vec<usize> { v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect() };
    assert_eq!(got.borders, us(&want["borders"]), "{ctx} borders");
    assert_eq!(got.fronts, us(&want["fronts"]), "{ctx} fronts");
    assert_eq!(u64::from(got.proximity_done), want["proximity_done"].as_u64().unwrap(), "{ctx} proximity done");
}

fn check_goals(got: &[Goal], want: &Value, ctx: &str) {
    let wg = want.as_array().unwrap();
    assert_eq!(got.len(), wg.len());
    for (gi, (g, w)) in got.iter().zip(wg).enumerate() {
        assert_eq!(u64::from(g.threshold_bits), w["threshold"].as_u64().unwrap(), "{ctx} goal {gi} threshold");
        let wi = w["items"].as_array().unwrap();
        let got_items: Vec<(usize, u32, Vec<Option<u32>>)> =
            g.items.iter().map(|it| (it.territory, it.priority_bits, it.armies.iter().map(|a| Some(a.army.0)).collect())).collect();
        let want_items: Vec<(usize, u32, Vec<Option<u32>>)> = wi
            .iter()
            .map(|it| {
                (
                    it["territory"].as_u64().unwrap() as usize,
                    it["priority"].as_u64().unwrap() as u32,
                    it["armies"].as_array().unwrap().iter().map(|a| a.as_u64().map(|v| v as u32)).collect(),
                )
            })
            .collect();
        assert_eq!(got_items, want_items, "{ctx} goal {gi} items");
    }
}

fn check_command(data: &MetaGameData, got: &Command, want: &Value, ctx: &str) {
    assert_eq!(u64::from(got.presence()), want["presence"].as_u64().unwrap(), "{ctx} presence {got:?} {want}");
    assert_eq!(got.data.map(|d| u64::from(d.index)), want["index"].as_u64(), "{ctx} index");
    if let Some(a) = want.get("army") {
        assert_eq!(got.army.map(|x| u64::from(x.0)), a.as_u64(), "{ctx} army");
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
        let want_d: Vec<usize> = d.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect();
        assert_eq!(got.divisions.clone().unwrap_or_default(), want_d, "{ctx} divisions");
    }
}

#[test]
fn mgai_matches_native_964d30() {
    let data_bytes = std::fs::read(dir().join("metagame-data.json")).unwrap();
    let base: MetaGameData = serde_json::from_slice(&data_bytes).unwrap();
    let mut files: Vec<_> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            is_player_fixture(&n)
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no native-mgai-c*.json fixture");
    let (mut turns, mut stages, mut commands, mut yields_total) = (0usize, 0usize, 0usize, 0usize);
    let (mut arrivals, mut busy_total, mut blocks, mut capital_lost, mut no_upgrade) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut results = std::collections::BTreeMap::<String, usize>::new();
    for file in &files {
        let fx: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        assert_eq!(fx["data_sha256"].as_str().unwrap(), aa_rules::sources::sha256(&data_bytes), "{file:?} data");
        for world in fx["worlds"].as_array().unwrap() {
            let ci = world["campaign"].as_u64().unwrap() as usize;
            let variant = world["variant"].as_u64().unwrap();
            let mut data = base.clone();
            for (k, v) in world["ai"].as_object().unwrap() {
                let ai: AiDefinition = serde_json::from_value(v.clone()).unwrap();
                let i: usize = k.parse().unwrap();
                if variant == 0 {
                    assert_eq!(ai, base.ai[i], "unperturbed AI values differ from metagame-data.json");
                }
                data.ai[i] = ai;
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
            let mut lists: Vec<Vec<usize>> = vec![Vec::new(); nk];
            let mut deleted = BTreeSet::new();
            let mut players: Vec<MgaiPlayer> = Vec::new();
            let fx_turns = world["turns"].as_array().unwrap();
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
                    "fought" => st.armies[v(1) as usize].as_mut().unwrap().fought = v(2) as i32,
                    "divisions" => {
                        st.armies[v(1) as usize].as_mut().unwrap().divisions = op[2]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|d| aa_metagame::battle::DivisionRef(d.as_u64().unwrap() as u32))
                            .collect();
                    }
                    "resources" => st.kingdoms[v(1) as usize].resources = v(2) as i32,
                    "battles" => st.kingdoms[v(1) as usize].battles = v(2) as i32,
                    "flag46" => st.battle_resolved = v(1) != 0,
                    "flag51" => st.fast_ai = v(1) as u8,
                    "deleted" => {
                        deleted.insert(v(1) as usize);
                    }
                    "lists" => {
                        lists = op[1]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|l| l.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).collect())
                            .collect();
                    }
                    "construct" => {
                        for k in 0..nk {
                            let p = MgaiPlayer::new(&st, &data, &rel, aa_metagame::GAME_PRECISION, k).unwrap();
                            check_analysis(&p.analysis, &world["construct"][k.to_string()], &format!("c{ci} v{variant} construct k{k}"));
                            players.push(p);
                        }
                    }
                    "turn" => {
                        let t = &fx_turns[v(1) as usize];
                        let k = t["kingdom"].as_u64().unwrap() as usize;
                        let precision = match t["cw"].as_str().unwrap() {
                            "pc24" => Precision::Single24,
                            "pc53" => Precision::Double53,
                            "pc64" => Precision::Extended64,
                            other => panic!("{other}"),
                        };
                        assert_eq!(u64::from(st.battle_resolved), t["flag46_in"].as_u64().unwrap());
                        let mut host = Host {
                            relations: &rel,
                            lists: &lists,
                            deleted: &deleted,
                            yields: t["yields"].as_array().unwrap(),
                            busy: t["busy"].as_array().unwrap(),
                            uninit: t["uninit"].as_array().unwrap(),
                            yi: 0,
                            bi: 0,
                            ui: 0,
                            tick: t["tick_in"].as_u64().unwrap() as u32 as i32,
                            rng: MetaGameRng::new(t["rng_in"].as_u64().unwrap() as u32),
                            issued: Vec::new(),
                            snaps: Vec::new(),
                        };
                        let ctx = format!("c{ci} v{variant} turn {} k{k} {}", v(1), t["cw"]);
                        capital_lost += usize::from(st.capital_lost(k, &rel).unwrap());
                        {
                            let probe = Host {
                                relations: &rel, lists: &lists, deleted: &deleted, yields: &[], busy: &[], uninit: &[],
                                yi: 0, bi: 0, ui: 0, tick: 0, rng: MetaGameRng::new(0), issued: Vec::new(), snaps: Vec::new(),
                            };
                            let view = View::new(&st, &data, &rel, precision, k).unwrap();
                            no_upgrade += usize::from(!view.upgrade_possible(&probe).unwrap());
                        }
                        let r = players[k].play_turn(&mut st, &data, &rel, precision, &mut host).unwrap();
                        let ws = t["stages"].as_array().unwrap();
                        assert_eq!(host.snaps.len(), ws.len(), "{ctx} stage count");
                        for (s, w) in host.snaps.iter().zip(ws) {
                            let sctx = format!("{ctx} stage {}", stage_name(s.stage));
                            assert_eq!(stage_name(s.stage), w["stage"].as_str().unwrap(), "{sctx}");
                            if !w["analysis"].is_null() {
                                check_analysis(&s.analysis, &w["analysis"], &sctx);
                            }
                            check_goals(&s.goals, &w["goals"], &sctx);
                            let flags: Vec<u64> = w["flags"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect();
                            assert_eq!(flags, [u64::from(s.flags[0]), u64::from(s.flags[1])], "{sctx} flags");
                            assert_eq!(s.issued as u64, w["issued"].as_u64().unwrap(), "{sctx} issued count");
                            assert_eq!(s.yields as u64, w["yields"].as_u64().unwrap(), "{sctx} yield count");
                            stages += 1;
                        }
                        assert_eq!(u64::from(r), t["result"].as_u64().unwrap(), "{ctx} result");
                        let wi = t["issued"].as_array().unwrap();
                        assert_eq!(host.issued.len(), wi.len(), "{ctx} issued {:?}", host.issued);
                        for (i, (g, w)) in host.issued.iter().zip(wi).enumerate() {
                            check_command(&data, g, w, &format!("{ctx} command {i}"));
                        }
                        assert_eq!(host.yi, host.yields.len(), "{ctx} yields consumed");
                        assert_eq!(host.bi, host.busy.len(), "{ctx} busy consumed");
                        assert_eq!(host.ui, host.uninit.len(), "{ctx} 96A5D0 blocks consumed");
                        assert_eq!(u64::from(host.rng.state), t["rng_out"].as_u64().unwrap(), "{ctx} app rng");
                        assert_eq!(u64::from(st.battle_resolved), t["flag46_out"].as_u64().unwrap(), "{ctx} flag46");
                        turns += 1;
                        commands += wi.len();
                        yields_total += host.yi;
                        busy_total += host.bi;
                        blocks += host.ui;
                        arrivals += host.yields.iter().filter(|y| y.get(4).is_some_and(|a| !a.is_null())).count();
                        let last = ws.last().map(|s| s["stage"].as_str().unwrap().to_string()).unwrap_or_else(|| "start".into());
                        *results.entry(format!("{}@{last}", t["result"])).or_default() += 1;
                    }
                    other => panic!("op {other}"),
                }
            }
        }
    }
    eprintln!("mgai: {turns} turns, {stages} stages, {commands} commands, {yields_total} yields, {arrivals} arrivals, {busy_total} busy queries, {blocks} 96A5D0 blocks, {capital_lost} turns with a lost capital, {no_upgrade} turns without possible upgrade, endings {results:?}");
    assert!(turns > 0);
}
