//! Replays research/phase8-movement/native-low-path-{solve,continuation,
//! result-plan}.json: original x86 KKC_LowPathSolver::Solve 0x53C170
//! (constructor 0x53BB90, core 0x53BE30, goal vtable 0xA3634C, window,
//! neighbour iterator, path building), UAIStateMove 0x531520 and ResultPlan
//! 0x5336B0 reaching 0x531520, under PC24/53/64. Every recorded world seam
//! must be requested by the Rust port in the same order with the same
//! arguments and descriptor words; every popped queue entry, the final
//! solver/window/goal/path/descriptor/stats state and the world epoch and
//! tile stamps must match. Timer (0x59AB60), SVar lookup (0x4B2E60) and
//! diagnostic log (0x404F60) events have no Rust callback: their values are
//! explicit inputs or they have no state effect, so they are filtered.
use aa_navigation::low_path::{
    self, ActorPose, ActorShape, Continuation, ContinuationInput, ContinuationOutcome,
    ContinuationWorld, Environment, LowPathSettings, LowPathSolver, LowPathStats, LowPathWorld,
    Options, Request, SpatialHit, WindowConfig,
};
use aa_navigation::result_plan::{
    self, Actor as PlanActor, CollisionContext, Object, State as PlanState,
};
use aa_navigation::search::{SearchCell, SearchGrid};
use aa_navigation::{MoveState, OwnedPath, PathNode};
use aa_numeric::Precision;
use serde_json::Value;
use std::collections::HashMap;

const DYN: [usize; 9] = [8, 12, 14, 15, 17, 18, 19, 20, 21];
const N: usize = DYN.len();
const FILTERED: [&str; 3] = ["timer", "svar", "log"];

fn u(v: &Value) -> u32 {
    v.as_u64().unwrap_or_else(|| panic!("u32 expected: {v}")) as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn ipair(v: &Value) -> [i32; 2] {
    [u(&v[0]) as i32, u(&v[1]) as i32]
}
fn words<const N: usize>(v: &Value) -> [u32; N] {
    let list: Vec<u32> = v.as_array().unwrap().iter().map(u).collect();
    list.try_into().unwrap()
}
fn path(v: &Value) -> OwnedPath {
    OwnedPath {
        nodes: v["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| PathNode { words: words(n) })
            .collect(),
        kind1_count: u(&v["kind1"]) as i32,
        kind2_count: u(&v["kind2"]) as i32,
        dirty: false,
        word18: 0x1357_2468,
    }
}
fn path_value(p: &OwnedPath) -> Value {
    serde_json::json!({
        "nodes": p.nodes.iter().map(|n| n.words.to_vec()).collect::<Vec<_>>(),
        "kind1": p.kind1_count as u32,
        "kind2": p.kind2_count as u32,
        "dirty": u32::from(p.dirty),
        "word18": p.word18,
    })
}

struct Actor {
    pose: ActorPose,
    word90: u32,
}

struct Replay {
    name: String,
    events: Vec<Value>,
    next: usize,
    registry: Vec<[u32; 3]>,
    actors: HashMap<u32, Actor>,
    context: [u32; 22],
    owner_word_54: u32,
    mover: u32,
    mover_position: [u32; 2],
}
impl Replay {
    fn take(&mut self, kind: &str) -> Value {
        let event = self
            .events
            .get(self.next)
            .unwrap_or_else(|| panic!("{}: no native event left for {kind}", self.name))
            .clone();
        assert_eq!(
            event[0].as_str().unwrap(),
            kind,
            "{}: event {} is {event}",
            self.name,
            self.next
        );
        self.next += 1;
        event
    }
    fn check(&self, event: &Value, at: usize, context: &CollisionContext) {
        for (k, &i) in DYN.iter().enumerate() {
            assert_eq!(
                u(&event[at + k]),
                context.words[i],
                "{}: descriptor word {i} at event {}: {event}",
                self.name,
                self.next - 1
            );
        }
    }
}
impl LowPathWorld for Replay {
    fn tile_blocked(
        &mut self,
        tile: [i32; 2],
        context: &CollisionContext,
    ) -> low_path::Result<bool> {
        let e = self.take("tile");
        assert_eq!([u(&e[1]) as i32, u(&e[2]) as i32], tile, "{}", self.name);
        self.check(&e, 3, context);
        Ok(u(&e[3 + N]) != 0)
    }
    fn set_context_position(
        &mut self,
        context: &mut CollisionContext,
        position: [u32; 2],
    ) -> low_path::Result<()> {
        let e = self.take("pos");
        assert_eq!([u(&e[1]), u(&e[2])], position, "{}", self.name);
        // 0x552250 with no source definition stores the position unchanged.
        context.words[19] = position[0];
        context.words[20] = position[1];
        Ok(())
    }
    fn first_hit(&mut self, context: &CollisionContext) -> low_path::Result<SpatialHit> {
        let e = self.take("hit");
        self.check(&e, 1, context);
        Ok(SpatialHit {
            kind: u(&e[1 + N]),
            reference: u(&e[2 + N]),
        })
    }
    fn sweep(
        &mut self,
        context: &CollisionContext,
        goal: [u32; 2],
    ) -> low_path::Result<SpatialHit> {
        let e = self.take("sweep");
        assert_eq!([u(&e[1]), u(&e[2])], goal, "{}", self.name);
        self.check(&e, 3, context);
        Ok(SpatialHit {
            kind: u(&e[3 + N]),
            reference: u(&e[4 + N]),
        })
    }
    fn placement_result(
        &mut self,
        source: Option<Object>,
        context: &CollisionContext,
    ) -> low_path::Result<u32> {
        let e = self.take("place");
        assert_eq!(u(&e[1]), source.map_or(0, |o| o.0), "{}", self.name);
        self.check(&e, 2, context);
        Ok(u(&e[2 + N]))
    }
    fn resolve_actor(&mut self, reference: u32) -> Option<Object> {
        self.registry
            .iter()
            .find(|[index, generation, object]| {
                *index == reference & 0xffff && *generation == reference >> 16 && *object != 0
            })
            .map(|entry| Object(entry[2]))
    }
    fn actor_virtual_b4(&mut self, actor: Object) -> low_path::Result<bool> {
        let e = self.take("vb4");
        assert_eq!(u(&e[1]), actor.0, "{}", self.name);
        Ok(u(&e[2]) != 0)
    }
    fn actor_word_90_present(&mut self, actor: Object) -> low_path::Result<bool> {
        Ok(self.actors[&actor.0].word90 != 0)
    }
    fn actor_virtual_100(&mut self, actor: Object) -> low_path::Result<u32> {
        let e = self.take("v100");
        assert_eq!(u(&e[1]), actor.0, "{}", self.name);
        Ok(u(&e[2]))
    }
    fn context_excludes(
        &mut self,
        context: &CollisionContext,
        actor: Object,
    ) -> low_path::Result<bool> {
        let e = self.take("excl");
        assert_eq!(u(&e[1]), actor.0, "{}", self.name);
        self.check(&e, 2, context);
        Ok(u(&e[2 + N]) != 0)
    }
    fn actor_pose(&mut self, actor: Object) -> low_path::Result<ActorPose> {
        Ok(self.actors[&actor.0].pose)
    }
    fn source_virtual_0(&mut self, source: Object) -> low_path::Result<u32> {
        let e = self.take("v0");
        assert_eq!(u(&e[1]), source.0, "{}", self.name);
        Ok(u(&e[2]))
    }
    fn actor_shape(&mut self, actor: Object) -> low_path::Result<ActorShape> {
        let e = self.take("shape");
        assert_eq!(u(&e[1]), actor.0, "{}", self.name);
        Ok(ActorShape {
            kind: u(&e[2]),
            word1_bits: u(&e[3]),
            word5_bits: u(&e[4]),
        })
    }
    fn refresh_overlaps(&mut self, source: Object) -> low_path::Result<()> {
        let e = self.take("refresh");
        assert_eq!(u(&e[1]), source.0, "{}", self.name);
        Ok(())
    }
}
impl ContinuationWorld for Replay {
    fn collision_context(&mut self, _: &mut MoveState) -> low_path::Result<CollisionContext> {
        self.take("context");
        Ok(CollisionContext {
            words: self.context,
        })
    }
    fn replacement_actor(&mut self, object: Object, _: &mut MoveState) -> low_path::Result<Object> {
        let e = self.take("replace");
        assert_eq!(u(&e[1]), object.0, "{}", self.name);
        Ok(Object(u(&e[2])))
    }
    fn owner_word_54(&mut self, _: &mut MoveState) -> low_path::Result<u32> {
        Ok(self.owner_word_54)
    }
    fn actor_position(&mut self, _: &mut MoveState) -> low_path::Result<[u32; 2]> {
        Ok(self.mover_position)
    }
    fn stop_action(&mut self, _: &mut MoveState) -> low_path::Result<()> {
        self.take("stop");
        Ok(())
    }
    fn actor_stop_animation(&mut self, _: &mut MoveState) -> low_path::Result<()> {
        let e = self.take("anim");
        assert_eq!(u(&e[1]), self.mover, "{}", self.name);
        Ok(())
    }
    fn schedule(&mut self, delay_bits: u32, _: &mut MoveState) -> low_path::Result<()> {
        let e = self.take("schedule");
        assert_eq!([u(&e[1]), u(&e[2])], [delay_bits, 0], "{}", self.name);
        Ok(())
    }
    fn path_failed(&mut self, _: &mut MoveState) -> low_path::Result<()> {
        let e = self.take("failed");
        assert_eq!(u(&e[1]), 1, "{}", self.name);
        Ok(())
    }
    fn action_go(&mut self, speed_bits: u32, state: &mut MoveState) -> low_path::Result<()> {
        let e = self.take("action_go");
        assert_eq!(u(&e[1]), speed_bits, "{}", self.name);
        assert_eq!(u(&e[2]), state.waypoint_offset_bits, "{}", self.name);
        assert_eq!(
            e[3],
            path_value(&state.path),
            "{}: path at ActionGo",
            self.name
        );
        Ok(())
    }
}

struct Setup {
    precision: Precision,
    grid: SearchGrid,
    solver: LowPathSolver,
    busy: bool,
    stats: Option<LowPathStats>,
    settings: LowPathSettings,
    queue_limit: Option<usize>,
    world: Replay,
}

fn setup(case: &Value) -> Setup {
    let name =
        case["name"].as_str().unwrap().to_owned() + "/" + case["precision"].as_str().unwrap();
    let precision: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
    let g = &case["grid"];
    let (width, height) = (u(&g["width"]) as i32, u(&g["height"]) as i32);
    let mut cells = vec![SearchCell::default(); (width * height) as usize];
    for entry in g["flags"].as_array().unwrap() {
        cells[u(&entry[0]) as usize].flags20 = u(&entry[1]);
    }
    for entry in g["stamps"].as_array().unwrap() {
        cells[u(&entry[0]) as usize].visit_epoch = u(&entry[1]);
    }
    let grid = SearchGrid {
        width,
        height,
        coordinate_shift: u(&g["shift"]),
        cell_size_bits: u(&g["cell_size"]),
        epoch: u(&g["epoch"]),
        cells,
    };
    let mut solver = LowPathSolver::new(&grid, WindowConfig::NATIVE, precision).unwrap();
    let constructed = &case["output"]["constructed"];
    assert_eq!(
        window_words(&solver),
        words::<12>(&constructed["window"]),
        "{name}: constructor"
    );
    solver.queue_remaining = u(&case["queue_count"]) as usize;
    for entry in case["pre_cells"].as_array().unwrap() {
        let cell = &mut solver.window.cells[u(&entry[0]) as usize];
        cell.stamp = u(&entry[1]);
        cell.direction = u(&entry[2]);
    }
    let s = &case["settings"];
    let settings = LowPathSettings {
        examined_max: u(&s["examined_max"]) as i32,
        ignore_moving_depth: u(&s["ignore_moving_depth"]) as i32,
        accuracy_depth: u(&s["accuracy_depth"]) as i32,
    };
    let limit = u(&case["queue_limit"]) as usize;
    let stats = case["stats"].as_array().map(|w| LowPathStats {
        words: words(&Value::Array(w.clone())),
    });
    let mut actors = HashMap::new();
    for a in case["actors"].as_array().unwrap() {
        actors.insert(
            u(&a["object"]),
            Actor {
                pose: ActorPose {
                    position_bits: pair(&a["position"]),
                    facing_bits: pair(&a["facing"]),
                },
                word90: u(&a["word90"]),
            },
        );
    }
    let events = case["output"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| !FILTERED.contains(&e[0].as_str().unwrap()))
        .cloned()
        .collect();
    let (mover, owner_word_54) = match case.get("continuation") {
        Some(c) => (
            u(&case["actors"][c["actor"].as_u64().unwrap() as usize]["object"]),
            u(&c["owner_word_54"]),
        ),
        None => (0, 0),
    };
    let mover_position = if mover != 0 {
        actors[&mover].pose.position_bits
    } else {
        [0, 0]
    };
    let world = Replay {
        name,
        events,
        next: 0,
        registry: case["registry"]
            .as_array()
            .unwrap()
            .iter()
            .map(words::<3>)
            .collect(),
        actors,
        context: words(&case["context"]),
        owner_word_54,
        mover,
        mover_position,
    };
    Setup {
        precision,
        grid,
        solver,
        busy: case["busy"].as_bool().unwrap(),
        stats,
        settings,
        queue_limit: (limit != 0).then_some(limit),
        world,
    }
}

fn window_words(solver: &LowPathSolver) -> [u32; 12] {
    let w = &solver.window;
    [
        w.origin_tile[0] as u32,
        w.origin_tile[1] as u32,
        w.origin_bits[0],
        w.origin_bits[1],
        w.offset_bits[0],
        w.offset_bits[1],
        w.sub_shift,
        w.mask as u32,
        w.size[0] as u32,
        w.size[1] as u32,
        w.extent_bits[0],
        w.extent_bits[1],
    ]
}

fn compare_final(
    case: &Value,
    setup: &Setup,
    path: &OwnedPath,
    context: &CollisionContext,
    pre: &HashMap<usize, (u32, u32)>,
) {
    let o = &case["output"];
    let name = &setup.world.name;
    assert_eq!(
        setup.world.next,
        setup.world.events.len(),
        "{name}: unreplayed native events"
    );
    assert_eq!(setup.grid.epoch, u(&o["epoch"]), "{name}: epoch");
    let stamps: Vec<Value> = setup
        .grid
        .cells
        .iter()
        .enumerate()
        .filter(|(_, c)| c.visit_epoch != 0)
        .map(|(i, c)| serde_json::json!([i, c.visit_epoch]))
        .collect();
    assert_eq!(
        Value::Array(stamps),
        o["tile_stamps"],
        "{name}: tile stamps"
    );
    let solver = &setup.solver;
    assert_eq!(
        window_words(solver),
        words::<12>(&o["window"]),
        "{name}: window"
    );
    let changed: Vec<Value> = solver
        .window
        .cells
        .iter()
        .enumerate()
        .filter(|(i, c)| (c.stamp, c.direction) != *pre.get(i).unwrap_or(&(0, 0)))
        .map(|(i, c)| serde_json::json!([i, c.stamp, c.direction]))
        .collect();
    assert_eq!(
        Value::Array(changed),
        o["changed_cells"],
        "{name}: window cells"
    );
    let g = &o["goal"];
    let goal = &solver.goal;
    assert_eq!(
        goal.start_cell,
        ipair(&g["start_cell"]),
        "{name}: goal start cell"
    );
    assert_eq!(goal.goal_cell, ipair(&g["goal_cell"]), "{name}: goal cell");
    assert_eq!(
        goal.path_linked,
        g["path_linked"].as_bool().unwrap(),
        "{name}: path link"
    );
    assert_eq!(
        goal.context_linked,
        g["context_linked"].as_bool().unwrap(),
        "{name}: context link"
    );
    assert_eq!(
        goal.start_bits,
        pair(&g["start_bits"]),
        "{name}: start bits"
    );
    assert_eq!(goal.goal_bits, pair(&g["goal_bits"]), "{name}: goal bits");
    assert_eq!(
        goal.direct,
        g["direct"].as_bool().unwrap(),
        "{name}: direct"
    );
    assert_eq!(goal.examined as u32, u(&g["examined"]), "{name}: examined");
    let s = &solver.search;
    let search = [
        s.closest_cell[0] as u32,
        s.closest_cell[1] as u32,
        s.closest_distance_bits,
        s.current_cell[0] as u32,
        s.current_cell[1] as u32,
        s.entry_words[0],
        s.entry_words[1],
        s.entry_words[2],
        s.entry_words[3],
        s.entry_words[4],
        s.candidate_cell[0] as u32,
        s.candidate_cell[1] as u32,
    ];
    assert_eq!(search, words::<12>(&o["search"]), "{name}: search state");
    assert_eq!(
        solver.queue_remaining as u32,
        u(&o["queue_count"]),
        "{name}: queue count"
    );
    assert_eq!(path_value(path), o["path"], "{name}: path");
    assert_eq!(
        context.words,
        words::<22>(&o["context"]),
        "{name}: descriptor"
    );
    assert_eq!(u32::from(setup.busy), u(&o["busy"]), "{name}: busy flag");
    match (&setup.stats, o["stats"].as_array()) {
        (Some(stats), Some(w)) => {
            assert_eq!(
                stats.words,
                words::<13>(&Value::Array(w.clone())),
                "{name}: stats"
            )
        }
        (None, None) => {}
        _ => panic!("{name}: stats presence"),
    }
}

/// Every popped queue entry (native solver +0x24 after each successful pop).
fn compare_popped(case: &Value, name: &str, expanded: &[aa_navigation::search::QueueEntry]) {
    let native: Vec<[u32; 5]> = case["output"]["popped"]
        .as_array()
        .unwrap()
        .iter()
        .map(words::<5>)
        .collect();
    let rust: Vec<[u32; 5]> = expanded.iter().map(|e| e.words()).collect();
    assert_eq!(rust, native, "{name}: popped queue entries");
}

fn load(file: &str) -> Value {
    let path = format!(
        "{}/../../research/phase8-movement/{file}",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(
        doc["cases"].as_array().unwrap().len(),
        doc["case_count"].as_u64().unwrap() as usize
    );
    doc
}

fn pre_cells(case: &Value) -> HashMap<usize, (u32, u32)> {
    case["pre_cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (u(&e[0]) as usize, (u(&e[1]), u(&e[2]))))
        .collect()
}

#[test]
fn native_low_path_solve_matches_original_x86() {
    let doc = load("native-low-path-solve.json");
    let mut statuses = [0usize; 3];
    for case in doc["cases"].as_array().unwrap() {
        let mut setup = setup(case);
        let mut path = path(&case["path"]);
        let mut context = CollisionContext {
            words: words(&case["context"]),
        };
        let request = Request {
            start_bits: pair(&case["request"]["start"]),
            goal_bits: pair(&case["request"]["goal"]),
        };
        assert!(
            case["output"]["fault"].is_null(),
            "{}: unexpected native fault",
            setup.world.name
        );
        let mut env = Environment {
            grid: &mut setup.grid,
            search_busy: &mut setup.busy,
            stats: setup.stats.as_mut(),
            settings: setup.settings,
            options: Options {
                precision: setup.precision,
                queue_limit: setup.queue_limit,
            },
        };
        let outcome = low_path::solve(
            &mut setup.solver,
            &mut env,
            request,
            &mut path,
            &mut context,
            &mut setup.world,
        )
        .unwrap_or_else(|e| panic!("{}: {e}", setup.world.name));
        assert_eq!(
            outcome.status,
            u(&case["output"]["status"]),
            "{}: status",
            setup.world.name
        );
        compare_popped(case, &setup.world.name, &outcome.expanded);
        statuses[outcome.status as usize] += 1;
        compare_final(case, &setup, &path, &context, &pre_cells(case));
    }
    // Every native result kind is represented.
    assert!(
        statuses.iter().all(|&n| n > 0),
        "status coverage {statuses:?}"
    );
}

#[test]
fn native_low_path_continuation_matches_original_x86() {
    let doc = load("native-low-path-continuation.json");
    let mut outcomes = HashMap::new();
    for case in doc["cases"].as_array().unwrap() {
        let mut setup = setup(case);
        let c = &case["continuation"];
        let mut state = MoveState {
            target_id: u(&c["target_id"]),
            destination_bits: pair(&c["destination"]),
            auxiliary_bits: [0, 0],
            requested_speed_bits: 0,
            phase: 0,
            mode: 0,
            waypoint_offset_bits: u(&c["waypoint_offset"]),
            path: path(&case["path"]),
        };
        let mut last_time = u(&c["last_time"]);
        let input = ContinuationInput {
            argument: u(&c["argument"]),
            world_time_bits: u(&case["world_time"]),
            wait_minimum_bits: u(&case["svars"]["LowPathingWaitMinimum"][1]),
        };
        let mut env = Environment {
            grid: &mut setup.grid,
            search_busy: &mut setup.busy,
            stats: setup.stats.as_mut(),
            settings: setup.settings,
            options: Options {
                precision: setup.precision,
                queue_limit: setup.queue_limit,
            },
        };
        let result = low_path::continue_current(
            input,
            &mut state,
            &mut last_time,
            &mut setup.solver,
            &mut env,
            &mut setup.world,
        );
        let name = setup.world.name.clone();
        let fault = &case["output"]["fault"];
        let native_context = words::<22>(&case["output"]["context"]);
        let (key, context) = match result {
            Ok(Continuation {
                outcome,
                context,
                solve,
            }) => {
                assert!(
                    fault.is_null(),
                    "{name}: native faulted ({fault}) but Rust returned {outcome:?}"
                );
                let expanded = solve.map(|o| o.expanded).unwrap_or_default();
                compare_popped(case, &name, &expanded);
                let key = match outcome {
                    ContinuationOutcome::Waited => "waited",
                    ContinuationOutcome::Failed => "failed",
                    ContinuationOutcome::Solved(_) => "solved",
                };
                (key, context)
            }
            Err(low_path::Error::EmptySolvedPath) => {
                assert!(!fault.is_null(), "{name}: Rust error without native fault");
                // The local descriptor is unobservable after the error; every
                // world event above already checked its solver-written words.
                (
                    "null_tail",
                    CollisionContext {
                        words: native_context,
                    },
                )
            }
            Err(e) => panic!("{name}: {e}"),
        };
        *outcomes.entry(key).or_insert(0usize) += 1;
        let s = &case["output"]["state"];
        assert_eq!(last_time, u(&s["last_time"]), "{name}: +0x78");
        assert_eq!(
            state.waypoint_offset_bits,
            u(&s["waypoint_offset"]),
            "{name}: +0x54"
        );
        // The descriptor is the 0x554350 result as modified by 0x531520 and Solve.
        compare_final(case, &setup, &state.path, &context, &pre_cells(case));
    }
    for key in ["waited", "failed", "solved", "null_tail"] {
        assert!(
            outcomes.contains_key(key),
            "missing continuation outcome {key}: {outcomes:?}"
        );
    }
}

/// Host world for ResultPlan: only the near/same-region branch is expected,
/// whose `continue_current` boundary (0x531520) runs the Rust continuation.
struct PlanHost {
    setup: Setup,
    input: ContinuationInput,
    last_time: u32,
    continuation: Option<Continuation>,
}
fn unexpected<T>(what: &str) -> T {
    panic!("ResultPlan composition reached unexpected boundary {what}")
}
impl result_plan::World for PlanHost {
    fn special_type(
        &mut self,
        _: Object,
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<bool> {
        unexpected("special_type")
    }
    fn has_special_component(&self, _: Object) -> bool {
        unexpected("has_special_component")
    }
    fn region_at(
        &mut self,
        point: [u32; 2],
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<i32> {
        let e = self.setup.world.take("region");
        assert_eq!([u(&e[1]), u(&e[2])], point, "{}", self.setup.world.name);
        Ok(u(&e[3]) as i32)
    }
    fn collision_context(
        &mut self,
        _: Object,
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<CollisionContext> {
        unexpected("collision_context")
    }
    fn resolve_actor(&self, _: u32) -> Option<Object> {
        unexpected("resolve_actor")
    }
    fn replacement_actor(&self, _: Object) -> Object {
        unexpected("replacement_actor")
    }
    fn object_definition(&self, _: Object) -> u32 {
        unexpected("object_definition")
    }
    fn sweep(
        &mut self,
        _: &mut CollisionContext,
        _: [u32; 2],
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<result_plan::CollisionHit> {
        unexpected("sweep")
    }
    fn continue_turn(
        &mut self,
        _: bool,
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<()> {
        unexpected("continue_turn")
    }
    fn continue_current(
        &mut self,
        argument: u32,
        state: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<()> {
        let Setup {
            precision,
            grid,
            solver,
            busy,
            stats,
            settings,
            queue_limit,
            world,
        } = &mut self.setup;
        let mut env = Environment {
            grid,
            search_busy: busy,
            stats: stats.as_mut(),
            settings: *settings,
            options: Options {
                precision: *precision,
                queue_limit: *queue_limit,
            },
        };
        let input = ContinuationInput {
            argument,
            ..self.input
        };
        let done = low_path::continue_current(
            input,
            &mut state.movement,
            &mut self.last_time,
            solver,
            &mut env,
            world,
        )
        .map_err(|e| result_plan::Error::World(e.to_string()))?;
        self.continuation = Some(done);
        Ok(())
    }
    fn property(
        &mut self,
        _: Object,
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<u32> {
        unexpected("property")
    }
    fn property_active(
        &mut self,
        _: u32,
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<bool> {
        unexpected("property_active")
    }
    fn solve(
        &mut self,
        _: &mut CollisionContext,
        _: aa_navigation::solver::Request,
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<i32> {
        unexpected("solve")
    }
    fn update_waypoint_offset(
        &mut self,
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<()> {
        unexpected("update_waypoint_offset")
    }
    fn action_go(
        &mut self,
        _: u32,
        _: &mut PlanState,
        _: &mut PlanActor,
    ) -> result_plan::Result<()> {
        unexpected("plan action_go")
    }
    fn failed(&mut self, _: &mut PlanState, _: &mut PlanActor) -> result_plan::Result<()> {
        self.setup.world.take("plan_failed");
        Ok(())
    }
}

/// Original ResultPlan 0x5336B0 executing 0x531520 natively, replayed through
/// result_plan::result_plan with the low-path continuation as its boundary.
#[test]
fn native_result_plan_reaches_low_path_continuation() {
    let doc = load("native-low-path-result-plan.json");
    let mut continued = 0;
    for case in doc["cases"].as_array().unwrap() {
        let setup = setup(case);
        let c = &case["continuation"];
        let rp = &case["result_plan"];
        let mut state = PlanState {
            movement: MoveState {
                target_id: u(&c["target_id"]),
                destination_bits: pair(&c["destination"]),
                auxiliary_bits: [0, 0],
                requested_speed_bits: 0,
                phase: u(&rp["phase"]),
                mode: u(&rp["mode"]),
                waypoint_offset_bits: u(&c["waypoint_offset"]),
                path: path(&case["path"]),
            },
            attempts: u(&rp["attempts"]) as i32,
            previous_destination_bits: pair(&rp["previous_destination"]),
        };
        let mut actor = PlanActor {
            object: Object(setup.world.mover),
            position_bits: setup.world.mover_position,
            facing_bits: [0, 0],
            turn_radius_bits: 0,
        };
        let precision = setup.precision;
        let mut host = PlanHost {
            setup,
            input: ContinuationInput {
                argument: 0,
                world_time_bits: u(&case["world_time"]),
                wait_minimum_bits: u(&case["svars"]["LowPathingWaitMinimum"][1]),
            },
            last_time: u(&c["last_time"]),
            continuation: None,
        };
        let name = host.setup.world.name.clone();
        assert!(
            case["output"]["fault"].is_null(),
            "{name}: unexpected native fault"
        );
        result_plan::result_plan(
            &mut state,
            &mut actor,
            None,
            u(&c["argument"]),
            precision,
            &mut host,
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let s = &case["output"]["state"];
        assert_eq!(state.attempts as u32, u(&s["attempts"]), "{name}: attempts");
        assert_eq!(
            state.previous_destination_bits,
            pair(&s["previous_destination"]),
            "{name}: +0x34"
        );
        assert_eq!(host.last_time, u(&s["last_time"]), "{name}: +0x78");
        assert_eq!(
            state.movement.waypoint_offset_bits,
            u(&s["waypoint_offset"]),
            "{name}: +0x54"
        );
        let context = match host.continuation.take() {
            Some(done) => {
                continued += 1;
                let expanded = done.solve.map(|o| o.expanded).unwrap_or_default();
                compare_popped(case, &name, &expanded);
                done.context
            }
            // ResultPlan returned before 0x531520 built a descriptor.
            None => CollisionContext {
                words: words(&case["output"]["context"]),
            },
        };
        compare_final(
            case,
            &host.setup,
            &state.movement.path,
            &context,
            &pre_cells(case),
        );
    }
    assert!(continued > 0, "no ResultPlan case reached 0x531520");
}
