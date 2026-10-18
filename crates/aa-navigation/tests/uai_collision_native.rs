//! Replays research/phase8-movement/native-uai-collision.json: the original
//! x86 UAIStateMove ResultAvoidStatic 0x534A40 (with 0x533140, 0x52E210,
//! 0x46E4F0/0x46E150, 0x46DAB0/0x46DC50/0x46DCB0, 0x51D510, 0x4CEFC0,
//! 0x52CF40/0x52CA90/0x52D180) and the moving-thing collision result 0x536AD0
//! (0x46EB30/0x46E0E0, 0x4CF210, 0x4EA0F0), both executing 0x531520 and the
//! whole low-path solver natively, plus the tile descriptor 0x52F3E0, under
//! PC24/53/64. Every recorded seam must be requested by the Rust port in the
//! same order with the same arguments (descriptor and shape words included);
//! native null dereferences must be Rust errors naming the same faulting
//! instruction after the same events; the route, +0x54, +0x78, the caller's
//! descriptor and the solver/window/goal/descriptor/stats state must match.
//! Timer (0x59AB60), SVar (0x4B2E60) and diagnostic log (0x404F90) events
//! have no Rust callback and are filtered.
use aa_navigation::low_path::{
    self, ActorPose, ActorShape, Continuation, ContinuationWorld, Environment, LowPathSettings,
    LowPathSolver, LowPathStats, LowPathWorld, Options, SpatialHit, WindowConfig,
};
use aa_navigation::result_plan::{CollisionContext, Object};
use aa_navigation::search::{SearchCell, SearchGrid};
use aa_navigation::uai_collision::{
    self, Context, Descriptor, IgnoredThing, MindTarget, Shape, StaticOutcome, ThingOutcome,
    UaiCollisionWorld,
};
use aa_navigation::{MoveState, OwnedPath, PathNode};
use aa_numeric::{Binary, Precision};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

const DYN: [usize; 9] = [8, 12, 14, 15, 17, 18, 19, 20, 21];
const N: usize = DYN.len();
const FILTERED: [&str; 3] = ["timer", "svar", "log"];

fn u(v: &Value) -> u32 {
    v.as_u64()
        .or_else(|| v.as_i64().map(|x| x as u64))
        .unwrap_or_else(|| panic!("u32 expected: {v}")) as u32
}
fn pair(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn ipair(v: &Value) -> [i32; 2] {
    [u(&v[0]) as i32, u(&v[1]) as i32]
}
fn words<const M: usize>(v: &Value) -> [u32; M] {
    let list: Vec<u32> = v.as_array().unwrap().iter().map(u).collect();
    list.try_into().unwrap()
}
fn shape(w: [u32; 6]) -> Shape {
    Shape {
        kind: w[0],
        radius_bits: w[1],
        bounds_bits: [w[2], w[3], w[4], w[5]],
    }
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
/// Native event layout of a descriptor: shape words (null when the pointer
/// is null), facing, position, alignment word.
fn descriptor_value(d: &Descriptor) -> Vec<Value> {
    let mut out: Vec<Value> = match d.shape {
        Some(s) => s.words().iter().map(|&w| Value::from(w)).collect(),
        None => vec![Value::Null; 6],
    };
    for w in [
        d.facing_bits[0],
        d.facing_bits[1],
        d.position_bits[0],
        d.position_bits[1],
        d.alignment,
    ] {
        out.push(Value::from(w));
    }
    out
}

struct Actor {
    pose: ActorPose,
    word90: u32,
    flags: u32,
    gtid: u32,
    mind: u32,
    /// actor+0x80 present, and its organization thing (0 when null).
    organization: Option<u32>,
}
struct Thing {
    mind: u32,
    component: u32,
}

struct Replay {
    name: String,
    events: Vec<Value>,
    next: usize,
    registry: Vec<[u32; 3]>,
    actors: HashMap<u32, Actor>,
    things: HashMap<u32, Thing>,
    leaders: HashMap<u32, u32>,
    context: [u32; 22],
    owner_word_54: u32,
    owner: u32,
    mover: u32,
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
    fn actor(&self, object: Object) -> &Actor {
        self.actors
            .get(&object.0)
            .unwrap_or_else(|| panic!("{}: unknown actor {}", self.name, object.0))
    }
    fn object(&self, event: &Value, at: usize, object: Object) {
        assert_eq!(u(&event[at]), object.0, "{}: object in {event}", self.name);
    }
    fn descriptor(&self, event: &Value, at: usize, d: &Descriptor) {
        let native: Vec<Value> = event.as_array().unwrap()[at..at + 11].to_vec();
        assert_eq!(
            native,
            descriptor_value(d),
            "{}: descriptor in {event}",
            self.name
        );
    }
    fn float(&mut self, kind: &str, object: Object) -> low_path::Result<Binary> {
        let e = self.take(kind);
        self.object(&e, 1, object);
        Ok(Binary::from_f32_bits(u(&e[2])).unwrap())
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
        self.object(&e, 1, actor);
        Ok(u(&e[2]) != 0)
    }
    fn actor_word_90_present(&mut self, actor: Object) -> low_path::Result<bool> {
        Ok(self.actor(actor).word90 != 0)
    }
    fn actor_virtual_100(&mut self, actor: Object) -> low_path::Result<u32> {
        let e = self.take("v100");
        self.object(&e, 1, actor);
        Ok(u(&e[2]))
    }
    fn context_excludes(
        &mut self,
        context: &CollisionContext,
        actor: Object,
    ) -> low_path::Result<bool> {
        let e = self.take("excl");
        self.object(&e, 1, actor);
        self.check(&e, 2, context);
        Ok(u(&e[2 + N]) != 0)
    }
    fn actor_pose(&mut self, actor: Object) -> low_path::Result<ActorPose> {
        Ok(self.actor(actor).pose)
    }
    fn source_virtual_0(&mut self, source: Object) -> low_path::Result<u32> {
        let e = self.take("v0");
        self.object(&e, 1, source);
        Ok(u(&e[2]))
    }
    fn actor_shape(&mut self, actor: Object) -> low_path::Result<ActorShape> {
        let e = self.take("shape");
        self.object(&e, 1, actor);
        Ok(ActorShape {
            kind: u(&e[2]),
            word1_bits: u(&e[3]),
            word5_bits: u(&e[7]),
        })
    }
    fn refresh_overlaps(&mut self, source: Object) -> low_path::Result<()> {
        let e = self.take("refresh");
        self.object(&e, 1, source);
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
        self.object(&e, 1, object);
        Ok(Object(u(&e[2])))
    }
    fn owner_word_54(&mut self, _: &mut MoveState) -> low_path::Result<u32> {
        Ok(self.owner_word_54)
    }
    fn actor_position(&mut self, _: &mut MoveState) -> low_path::Result<[u32; 2]> {
        Ok(self.actor(Object(self.mover)).pose.position_bits)
    }
    fn stop_action(&mut self, _: &mut MoveState) -> low_path::Result<()> {
        self.take("stop");
        Ok(())
    }
    fn actor_stop_animation(&mut self, _: &mut MoveState) -> low_path::Result<()> {
        let e = self.take("anim");
        self.object(&e, 1, Object(self.mover));
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
impl UaiCollisionWorld for Replay {
    fn state_actor(&mut self, _: &mut MoveState) -> low_path::Result<Object> {
        Ok(Object(self.mover))
    }
    fn actor_flags(&mut self, actor: Object) -> low_path::Result<u32> {
        Ok(self.actor(actor).flags)
    }
    fn actor_gtid(&mut self, actor: Object) -> low_path::Result<u32> {
        Ok(self.actor(actor).gtid)
    }
    fn organization_thing(&mut self, actor: Object) -> low_path::Result<Option<Object>> {
        Ok(self
            .actor(actor)
            .organization
            .filter(|&t| t != 0)
            .map(Object))
    }
    fn organization_component(&mut self, thing: Object) -> low_path::Result<Option<Object>> {
        Ok(Some(self.things[&thing.0].component)
            .filter(|&c| c != 0)
            .map(Object))
    }
    fn organization_leader(&mut self, component: Object) -> low_path::Result<Option<Object>> {
        Ok(Some(self.leaders[&component.0])
            .filter(|&l| l != 0)
            .map(Object))
    }
    fn thing_mind(&mut self, thing: Object) -> low_path::Result<Option<Object>> {
        let mind = match self.things.get(&thing.0) {
            Some(t) => t.mind,
            None => self.actor(thing).mind,
        };
        Ok(Some(mind).filter(|&m| m != 0).map(Object))
    }
    fn mind_virtual_44(&mut self, mind: Object) -> low_path::Result<MindTarget> {
        let e = self.take("v44");
        self.object(&e, 1, mind);
        Ok(match u(&e[2]) {
            0 => MindTarget::Null,
            1 => MindTarget::NoPoint,
            _ => MindTarget::Point([u(&e[3]), u(&e[4])]),
        })
    }
    fn mind_virtual_68(&mut self, mind: Object) -> low_path::Result<bool> {
        let e = self.take("v68");
        self.object(&e, 1, mind);
        assert_ne!(mind.0, self.owner, "{}", self.name);
        Ok(u(&e[2]) != 0)
    }
    fn owner_virtual_68(&mut self, _: &mut MoveState) -> low_path::Result<bool> {
        let e = self.take("v68");
        self.object(&e, 1, Object(self.owner));
        Ok(u(&e[2]) != 0)
    }
    fn actor_moving(&mut self, actor: Object) -> low_path::Result<bool> {
        let e = self.take("moving");
        self.object(&e, 1, actor);
        Ok(u(&e[2]) != 0)
    }
    fn actor_virtual_0(&mut self, actor: Object) -> low_path::Result<Binary> {
        self.float("v0", actor)
    }
    fn actor_virtual_4(&mut self, actor: Object) -> low_path::Result<Binary> {
        self.float("v4", actor)
    }
    fn actor_collision_shape(&mut self, actor: Object) -> low_path::Result<Option<Shape>> {
        let e = self.take("shape");
        self.object(&e, 1, actor);
        let list = e.as_array().unwrap();
        Ok((list.len() > 2).then(|| shape(words(&Value::Array(list[2..8].to_vec())))))
    }
    fn actor_soft_shape(&mut self, actor: Object) -> low_path::Result<Option<Shape>> {
        let e = self.take("soft");
        self.object(&e, 1, actor);
        let list = e.as_array().unwrap();
        Ok(Some(shape(words(&Value::Array(list[2..8].to_vec())))))
    }
    fn actor_virtual_18(&mut self, actor: Object) -> low_path::Result<Binary> {
        self.float("v18", actor)
    }
    fn actor_virtual_20(&mut self, actor: Object, argument: u32) -> low_path::Result<bool> {
        let e = self.take("v20");
        self.object(&e, 1, actor);
        assert_eq!(u(&e[2]), argument, "{}", self.name);
        Ok(u(&e[3]) != 0)
    }
    fn actor_virtual_16c(&mut self, actor: Object, other: Object) -> low_path::Result<bool> {
        let e = self.take("v16c");
        self.object(&e, 1, actor);
        self.object(&e, 2, other);
        Ok(u(&e[3]) != 0)
    }
    fn ignore_thing(&mut self, actor: Object, entry: IgnoredThing) -> low_path::Result<()> {
        let e = self.take("ignore");
        self.object(&e, 1, actor);
        assert_eq!([u(&e[2]), u(&e[3])], [entry.gtid, entry.until_bits], "{}", self.name);
        Ok(())
    }
    fn forward_probe(
        &mut self,
        _: &mut MoveState,
        direction: [u32; 2],
        distance_bits: u32,
    ) -> low_path::Result<[u32; 4]> {
        let e = self.take("fprobe");
        assert_eq!(
            [u(&e[1]), u(&e[2]), u(&e[3])],
            [direction[0], direction[1], distance_bits],
            "{}: forward probe",
            self.name
        );
        Ok([u(&e[4]), u(&e[5]), u(&e[6]), u(&e[7])])
    }
    fn origin_probe(
        &mut self,
        _: &mut MoveState,
        origin: [u32; 2],
        direction: [u32; 2],
        distance_bits: u32,
    ) -> low_path::Result<[u32; 4]> {
        let e = self.take("oprobe");
        assert_eq!(
            [u(&e[1]), u(&e[2]), u(&e[3]), u(&e[4]), u(&e[5])],
            [
                origin[0],
                origin[1],
                direction[0],
                direction[1],
                distance_bits
            ],
            "{}: origin probe",
            self.name
        );
        Ok([u(&e[6]), u(&e[7]), u(&e[8]), u(&e[9])])
    }
    fn shapes_overlap(&mut self, a: &Descriptor, b: &Descriptor) -> low_path::Result<bool> {
        let e = self.take("overlap");
        self.descriptor(&e, 1, a);
        self.descriptor(&e, 12, b);
        Ok(u(&e[23]) != 0)
    }
    fn shapes_in_range(
        &mut self,
        a: &Descriptor,
        range_bits: u32,
        b: &Descriptor,
        inverted: bool,
    ) -> low_path::Result<bool> {
        let e = self.take("range");
        self.descriptor(&e, 1, a);
        assert_eq!(u(&e[12]), range_bits, "{}: range", self.name);
        self.descriptor(&e, 13, b);
        assert_eq!(u(&e[24]) != 0, inverted, "{}: inverted", self.name);
        Ok(u(&e[25]) != 0)
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
    assert_eq!(
        window_words(&solver),
        words::<12>(&case["output"]["constructed"]["window"]),
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
    let mut things = HashMap::new();
    let mut leaders = HashMap::new();
    for a in case["actors"].as_array().unwrap() {
        let organization = match &a["org_objects"] {
            Value::Null => None,
            o => {
                let thing = u(&o["thing"]);
                if thing != 0 {
                    let component = u(&o["component"]);
                    things.insert(
                        thing,
                        Thing {
                            mind: u(&o["thing_mind"]),
                            component,
                        },
                    );
                    if component != 0 {
                        leaders.insert(component, u(&o["leader"]));
                    }
                }
                Some(thing)
            }
        };
        actors.insert(
            u(&a["object"]),
            Actor {
                pose: ActorPose {
                    position_bits: pair(&a["position"]),
                    facing_bits: pair(&a["facing"]),
                },
                word90: u(&a["word90"]),
                flags: u(&a["flags"]),
                gtid: u(&a["gtid"]),
                mind: u(&a["mind_object"]),
                organization,
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
    let c = &case["continuation"];
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
        things,
        leaders,
        context: words(&case["context"]),
        owner_word_54: u(&c["owner_word_54"]),
        owner: u(&case["owner_object"]),
        mover: u(&case["actors"][c["actor"].as_u64().unwrap() as usize]["object"]),
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

/// Final solver/world state (as in low_path_native.rs) and the UAIStateMove.
fn compare_final(case: &Value, setup: &Setup, state: &MoveState, last_time: u32, context: &[u32; 22]) {
    let o = &case["output"];
    let name = &setup.world.name;
    assert_eq!(
        setup.world.next,
        setup.world.events.len(),
        "{name}: unreplayed native events"
    );
    assert_eq!(path_value(&state.path), o["path"], "{name}: path");
    assert_eq!(last_time, u(&o["state"]["last_time"]), "{name}: +0x78");
    assert_eq!(
        state.waypoint_offset_bits,
        u(&o["state"]["waypoint_offset"]),
        "{name}: +0x54"
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
    assert_eq!(Value::Array(stamps), o["tile_stamps"], "{name}: tile stamps");
    let solver = &setup.solver;
    assert_eq!(window_words(solver), words::<12>(&o["window"]), "{name}: window");
    let pre: HashMap<usize, (u32, u32)> = case["pre_cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (u(&e[0]) as usize, (u(&e[1]), u(&e[2]))))
        .collect();
    let changed: Vec<Value> = solver
        .window
        .cells
        .iter()
        .enumerate()
        .filter(|(i, c)| (c.stamp, c.direction) != *pre.get(i).unwrap_or(&(0, 0)))
        .map(|(i, c)| serde_json::json!([i, c.stamp, c.direction]))
        .collect();
    assert_eq!(Value::Array(changed), o["changed_cells"], "{name}: window cells");
    let g = &o["goal"];
    let goal = &solver.goal;
    assert_eq!(goal.start_cell, ipair(&g["start_cell"]), "{name}: goal start cell");
    assert_eq!(goal.goal_cell, ipair(&g["goal_cell"]), "{name}: goal cell");
    assert_eq!(goal.path_linked, g["path_linked"].as_bool().unwrap(), "{name}: path link");
    assert_eq!(
        goal.context_linked,
        g["context_linked"].as_bool().unwrap(),
        "{name}: context link"
    );
    assert_eq!(goal.start_bits, pair(&g["start_bits"]), "{name}: start bits");
    assert_eq!(goal.goal_bits, pair(&g["goal_bits"]), "{name}: goal bits");
    assert_eq!(goal.direct, g["direct"].as_bool().unwrap(), "{name}: direct");
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
    assert_eq!(*context, words::<22>(&o["context"]), "{name}: low-path descriptor");
    assert_eq!(u32::from(setup.busy), u(&o["busy"]), "{name}: busy flag");
    match (&setup.stats, o["stats"].as_array()) {
        (Some(stats), Some(w)) => {
            assert_eq!(stats.words, words::<13>(&Value::Array(w.clone())), "{name}: stats")
        }
        (None, None) => {}
        _ => panic!("{name}: stats presence"),
    }
}

fn compare_popped(case: &Value, name: &str, continuation: Option<&Continuation>) {
    let native: Vec<[u32; 5]> = case["output"]["popped"]
        .as_array()
        .unwrap()
        .iter()
        .map(words::<5>)
        .collect();
    let rust: Vec<[u32; 5]> = continuation
        .and_then(|c| c.solve.as_ref())
        .map(|o| o.expanded.iter().map(|e| e.words()).collect())
        .unwrap_or_default();
    assert_eq!(rust, native, "{name}: popped queue entries");
}

fn load() -> Value {
    let path = format!(
        "{}/../../research/phase8-movement/native-uai-collision.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let total: usize = ["static", "thing", "tile"]
        .iter()
        .map(|g| doc[*g].as_array().unwrap().len())
        .sum();
    assert_eq!(total, doc["case_count"].as_u64().unwrap() as usize);
    doc
}

fn move_state(case: &Value) -> MoveState {
    let c = &case["continuation"];
    let uai = &case["uai"];
    MoveState {
        target_id: u(&c["target_id"]),
        destination_bits: pair(&c["destination"]),
        auxiliary_bits: [0, 0],
        requested_speed_bits: u(&uai["speed"]),
        phase: u(&uai["phase"]),
        mode: u(&uai["mode"]),
        waypoint_offset_bits: u(&c["waypoint_offset"]),
        path: path(&case["path"]),
    }
}

enum Outcome {
    Static(StaticOutcome),
    Thing(ThingOutcome),
}

/// Runs one 0x534A40 or 0x536AD0 case and checks every observable result.
fn replay(case: &Value, counts: &mut BTreeMap<String, usize>) {
    let mut setup = setup(case);
    let precision = setup.precision;
    let uai = &case["uai"];
    let mut state = move_state(case);
    let mut last_time = u(&case["continuation"]["last_time"]);
    let collision = pair(&uai["cv"]);
    let output = &case["output"];
    let name = setup.world.name.clone();
    let Setup {
        grid,
        solver,
        busy,
        stats,
        settings,
        queue_limit,
        world,
        ..
    } = &mut setup;
    let mut environment = Environment {
        grid,
        search_busy: busy,
        stats: stats.as_mut(),
        settings: *settings,
        options: Options {
            precision,
            queue_limit: *queue_limit,
        },
    };
    let mut ctx = Context {
        world_step_bits: u(&uai["step"]),
        world_time_bits: u(&case["world_time"]),
        wait_minimum_bits: u(&case["svars"]["LowPathingWaitMinimum"][1]),
        last_solve_time_bits: &mut last_time,
        solver,
        environment: &mut environment,
    };
    let result = if uai["entry"] == "static" {
        let d = &uai["descriptor"];
        let mut descriptor = if d["source"] == "tile" {
            // GMap+0x30, built natively by 0x46EB60 from the cell size.
            let cell = u(&case["grid"]["cell_size"]);
            let tile_shape = uai_collision::square_shape(cell, precision).unwrap();
            assert_eq!(
                tile_shape.words(),
                words::<6>(&output["tile_shape"]),
                "{name}: 0x46EB60 tile shape"
            );
            uai_collision::tile_descriptor(
                u(&case["grid"]["shift"]),
                cell,
                tile_shape,
                ipair(&d["tile"]),
                precision,
            )
            .unwrap()
        } else {
            let obstacle = u(&case["actors"][d["actor"].as_u64().unwrap() as usize]["object"]);
            uai_collision::actor_descriptor(Object(obstacle), world).unwrap()
        };
        descriptor.alignment = d.get("alignment").map_or(0, u);
        let native_in = words::<6>(&output["descriptor_in"]);
        assert_eq!(
            [
                descriptor.facing_bits[0],
                descriptor.facing_bits[1],
                descriptor.position_bits[0],
                descriptor.position_bits[1],
                descriptor.alignment
            ],
            native_in[1..],
            "{name}: descriptor built by 0x4CF210/0x52F3E0"
        );
        let other = uai["other"]
            .as_u64()
            .map(|i| Object(u(&case["actors"][i as usize]["object"])));
        let result = uai_collision::avoid_static(
            &mut descriptor,
            collision,
            other,
            &mut state,
            &mut ctx,
            world,
        );
        if result.is_ok() {
            assert_eq!(
                descriptor.alignment,
                u(&output["descriptor_out"][5]),
                "{name}: descriptor alignment cache"
            );
        }
        result.map(Outcome::Static)
    } else {
        let thing = Object(u(&case["actors"][uai["other"].as_u64().unwrap() as usize]["object"]));
        uai_collision::avoid_thing(
            thing,
            u(&uai["distance"]),
            collision,
            &mut state,
            &mut ctx,
            world,
        )
        .map(Outcome::Thing)
    };
    let fault = &output["fault"];
    let (key, continuation) = match result {
        Ok(outcome) => {
            assert!(fault.is_null(), "{name}: native faulted ({fault}) but Rust succeeded");
            let static_key = |o: StaticOutcome| match o {
                StaticOutcome::Continued(c) => ("continued".to_owned(), Some(c)),
                StaticOutcome::Avoided { index } => (format!("avoided_{index}"), None),
                StaticOutcome::Proceeded => ("proceeded".to_owned(), None),
                StaticOutcome::IgnoredThing(_) => ("ignored".to_owned(), None),
            };
            match outcome {
                Outcome::Static(o) => {
                    let (k, c) = static_key(o);
                    (format!("static/{k}"), c)
                }
                Outcome::Thing(ThingOutcome::Static(o)) => {
                    let (k, c) = static_key(o);
                    (format!("thing/static/{k}"), c)
                }
                Outcome::Thing(ThingOutcome::Stopped) => ("thing/stopped".to_owned(), None),
                Outcome::Thing(ThingOutcome::Continued(c)) => {
                    ("thing/continued".to_owned(), Some(c))
                }
                Outcome::Thing(ThingOutcome::Proceeded) => ("thing/proceeded".to_owned(), None),
            }
        }
        Err(uai_collision::Error::NullDereference { address }) => {
            assert!(!fault.is_null(), "{name}: Rust null dereference without native fault");
            assert_eq!(
                address,
                u(&output["fault_eip"]),
                "{name}: faulting instruction"
            );
            (format!("fault/{address:#x}"), None)
        }
        Err(uai_collision::Error::LowPath(low_path::Error::EmptySolvedPath)) => {
            assert!(!fault.is_null(), "{name}: Rust error without native fault");
            ("fault/empty_solved_path".to_owned(), None)
        }
        Err(e) => panic!("{name}: {e}"),
    };
    *counts.entry(key).or_insert(0) += 1;
    compare_popped(case, &name, continuation.as_ref());
    let context = continuation.map_or_else(|| words::<22>(&output["context"]), |c| c.context.words);
    compare_final(case, &setup, &state, last_time, &context);
}

#[test]
fn native_avoid_static_matches_original_x86() {
    let doc = load();
    let mut counts = BTreeMap::new();
    for case in doc["static"].as_array().unwrap() {
        replay(case, &mut counts);
    }
    eprintln!("0x534A40 outcomes: {counts:?}");
    for key in [
        "static/continued",
        "static/avoided_0",
        "static/avoided_1",
        "static/proceeded",
        "static/ignored",
        "fault/0x534b78",
        "fault/0x534af2",
        "fault/0x534afc",
        "fault/0x534b01",
        "fault/0x4cede0",
        "fault/0x534bb3",
        "fault/0x534bc2",
        "fault/0x534bc5",
        "fault/0x534c30",
    ] {
        assert!(counts.contains_key(key), "missing outcome {key}: {counts:?}");
    }
}

#[test]
fn native_avoid_thing_matches_original_x86() {
    let doc = load();
    let mut counts = BTreeMap::new();
    for case in doc["thing"].as_array().unwrap() {
        replay(case, &mut counts);
    }
    eprintln!("0x536AD0 outcomes: {counts:?}");
    for key in [
        "thing/static/avoided_0",
        "thing/static/continued",
        "thing/static/ignored",
        "thing/static/proceeded",
        "thing/stopped",
        "thing/continued",
        "thing/proceeded",
        "fault/0x534c30",
    ] {
        assert!(counts.contains_key(key), "missing outcome {key}: {counts:?}");
    }
}

#[test]
fn native_tile_descriptor_matches_original_x86() {
    let doc = load();
    for case in doc["tile"].as_array().unwrap() {
        let precision: Precision = serde_json::from_value(case["precision"].clone()).unwrap();
        let tile_shape = uai_collision::square_shape(u(&case["cell_size"]), precision).unwrap();
        let d = uai_collision::tile_descriptor(
            u(&case["shift"]),
            u(&case["cell_size"]),
            tile_shape,
            ipair(&case["tile"]),
            precision,
        )
        .unwrap();
        assert_eq!(d.shape, Some(tile_shape));
        assert_eq!(
            [
                d.facing_bits[0],
                d.facing_bits[1],
                d.position_bits[0],
                d.position_bits[1],
                d.alignment
            ],
            words::<5>(&case["output"]["words"]),
            "{}/{}",
            case["name"],
            case["precision"]
        );
    }
}
