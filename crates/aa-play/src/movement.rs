//! Orders to regiments and actors, path following, formation slot
//! following, terrain speed limits, turning. Owner: movement.
//!
//! Plain `f32` reimplementation of the original regiment and element
//! movement, following the exact ports:
//! - regiment move state `KKC_CAIStateMove` (`aa_navigation::company_move`):
//!   activation/Enter `0x520440`/`0x523080`, destination `0x51E9B0` with the
//!   group offset and extent clamp `0x5525D0`, speed `0x5205E0` capped by the
//!   slowest member (organization +0xD8, `0x57CBA0`), Think `0x5276B0`
//!   (target re-path ratio, arrival 0.5 m, move tries and
//!   `CAIMoveMinSecondsBeforeFail`), planning `0x51EA20` from the leader,
//!   member dispatch `0x525740` (facing along the first leg, slot positions
//!   `0x57BEF0`, per-slot paths `0x52CBD0`), and the leader node handler
//!   `0x51F7A0` that consumes the regiment path and turns the formation at
//!   every corner;
//! - element move `0x537B90` / continuation `0x5379C0` / ActionGo `0x5300A0`
//!   (`aa_mind::element_move*`, `aa_navigation::action_go`): node reached
//!   `0x530AE0`, waypoint turn offset `0x52FD90`, get-back-to-path `0x532B50`,
//!   the formation speed controller `0x52FF00` with slot lag `0x52E400`
//!   (`aa_navigation::formation_follow`), the re-think delays;
//! - Mover ground step `0x5774A0` and order `0x5773C0` (`aa_movement`):
//!   turning at max speed / turn radius (`0x576850`), acceleration, terrain
//!   speed limit with `TerrainMinimumMoveSpeed`, speed factor, crossing the
//!   destination along the requested direction, blocked steps;
//! - organization position interpolation `0x57C080` (the banner follows the
//!   leader at `MoverCatchupSpeed` times the regiment speed);
//! - collision exclusions `0x4D5600` / `0x552340` (friendly members of
//!   moving regiments pass each other; things already overlapped are
//!   ignored, `0x4D67F0`) and the 2 s ignore entry of `0x534A40`.
//!
//! Simplified (see the module report): obstacle avoidance after a blocked
//! step (the original low-level pather `0x53C170` / `0x534A40`) is a sidestep
//! node around the blocking actor, or a tile re-route with the tile solver.
use crate::data::{Def, GameData};
use crate::formation::{self, facing_angle, facing_vector};
use crate::path::{Grid, MoverClass, PathRules, Pathfinder, Request, Status, Waypoint};
use crate::scheduler::Owner as Sched;
use crate::world::{Activity, ActorId, Command, KingdomId, Order, OrgId, Unit, World};
use glam::Vec2;
use std::collections::VecDeque;

// Native binary32 constants.
/// 1e-4 (0x38D1B717).
const EPSILON: f32 = f32::from_bits(0x38d1_b717);
/// 0x52A1F0 arrival: squared distance below 0.25.
const ARRIVAL_RADIUS_SQUARED: f32 = 0.25;
/// 0x5276B0 target re-path ratio (0x3E15F96D).
const REPATH_RATIO: f32 = f32::from_bits(0x3e15_f96d);
/// 0x52FF00 minimum formation speed 0.2 (0x3E4CCCCD).
const MINIMUM_FORMATION_SPEED: f32 = f32::from_bits(0x3e4c_cccd);
/// 0x52FF00 kind-0 blend scale -0.25.
const PATH_DIFFERENCE_SCALE: f32 = -0.25;
/// 0x52E400 corner taper: 36.0, 6.0, 1/6.
const CORNER_RADIUS_SQUARED: f32 = 36.0;
const CORNER_RADIUS: f32 = 6.0;
const CORNER_SCALE: f32 = f32::from_bits(0x3e2a_aaab);
/// 0x525740 / 0x5276B0 schedule(1.0).
const THINK_SECONDS: f32 = 1.0;
/// 0x5300A0 re-think delays: 0.3 (kind 0) or 1.0 (kind 1) plus 0.3 * random.
const ACTION_DELAY_INSTANT: f32 = f32::from_bits(0x3e99_999a);
const ACTION_DELAY_PATH: f32 = 1.0;
const ACTION_JITTER: f32 = f32::from_bits(0x3e99_999a);
/// 0x536AD0: speed margin 0.25 (0xA033BC) and the 0.3 s stop delay.
const QUEUE_SPEED_MARGIN: f32 = 0.25;
const QUEUE_WAIT: f32 = f32::from_bits(0x3e99_999a);
/// 0x534A40 ignore entry: 2.0 s.
const IGNORE_SECONDS: f32 = 2.0;
/// 0x532B50: perpendicular distance squared that starts a turn recovery.
const RECOVERY_SQUARED: f32 = 1.0;
/// 0x532CAA default recovery angle pi/4 (0x3F490FDB).
const RECOVERY_ANGLE: f32 = f32::from_bits(0x3f49_0fdb);
/// 0x537B90 near destination: squared distance below 0.25 (0x3E800000).
const NEAR_POINT_SQUARED: f32 = 0.25;
/// 0x537B90 arrival of a mode 1 move: squared distance below 0.01
/// (0x3C23D70A).
const ARRIVED_SQUARED: f32 = f32::from_bits(0x3c23_d70a);
/// 0x532B50 / 0x5379C0 lead: 1.5 * movement radius (0x3FC00000).
const LEAD_SCALE: f32 = 1.5;

/// 0x52DE40 passes 4.0 (0x40800000) to FindOpenPositionLineThenRadius.
const DEPLOY_SEARCH_RADIUS: f32 = 4.0;
/// 0x52F7A0 KKC_UAIStateDeploy::EventReachedDestination: FindOpenPositionNear
/// within 8.0 (0x41000000); more than 5 redeploys and the actor dies.
const REDEPLOY_SEARCH_RADIUS: f32 = 8.0;
const REDEPLOY_TRIES: u8 = 5;

/// Tiles the path queue may expand per tick before later requests wait for
/// the next tick. A request always runs to completion once started, so the
/// budget only adds latency, never changes a route. Engineering limit, not
/// an original value.
pub const SEARCH_BUDGET: u32 = 12_000;
/// Spatial hash bucket size for blocking checks (engineering choice).
const HASH_CELL: f32 = 4.0;

/// Original settings used by movement, from `GameData::rules` with the
/// shipped values as fallback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveRules {
    /// `MoverCatchupSpeed` (Data/Game/SVars.tgi line 78).
    pub catchup: f32,
    /// `CAIMoveMinSecondsBeforeFail` (SVars.tgi line 82).
    pub min_seconds_before_fail: f32,
    /// `TAIReachedDestinationThreshold` (SVars.tgi line 86).
    pub reached_threshold: f32,
    /// `GroupSpeedMinimum` (Data/Game/SVars_AA.tgi line 77).
    pub group_speed_minimum: f32,
    /// `TerrainMinimumMoveSpeed` (AA.exe registration default 0x3F000000;
    /// no data file overrides it).
    pub terrain_minimum_speed: f32,
    /// `BannerMinCatchupSpeed` (SVars.tgi line 81).
    pub banner_min_catchup: f32,
    /// `LowPathingWaitMinimum` (SVars.tgi line 95): seconds between
    /// obstacle re-routes of one element.
    pub low_path_wait: f32,
    pub path: PathRules,
    /// `CAITargetInRangeZOCMultiplier` (SVars_AA.tgi line 88, 0.6; SVars.tgi
    /// 1): a followed actor is in range within this times the zone radius.
    pub target_in_range_zoc: f32,
}
impl MoveRules {
    pub fn from_data(data: &GameData) -> Self {
        let r = |name: &str, default: f32| data.rule(name).unwrap_or(default);
        Self {
            catchup: r("MoverCatchupSpeed", 1.75),
            min_seconds_before_fail: r("CAIMoveMinSecondsBeforeFail", 5.0),
            reached_threshold: r("TAIReachedDestinationThreshold", 0.1),
            group_speed_minimum: r("GroupSpeedMinimum", 1.0),
            terrain_minimum_speed: r("TerrainMinimumMoveSpeed", 0.5),
            banner_min_catchup: r("BannerMinCatchupSpeed", 0.5),
            low_path_wait: r("LowPathingWaitMinimum", 2.0),
            path: PathRules::from_data(data),
            target_in_range_zoc: r("CAITargetInRangeZOCMultiplier", 0.6),
        }
    }
}

/// Path node: point, leg direction, kind (0 = helper node with instant
/// turn, 1 = path node), as in the original path collection.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Node {
    pos: Vec2,
    dir: Vec2,
    kind: u8,
}
impl From<Waypoint> for Node {
    fn from(w: Waypoint) -> Self {
        Node { pos: w.pos, dir: w.dir, kind: 1 }
    }
}

/// Mover component fields (0x576920 layout).
#[derive(Clone, Debug)]
struct Mover {
    /// Scheduled (the Mover is the active component and ticking).
    active: bool,
    dest: Vec2,
    desired: Vec2,
    facing: Vec2,
    /// `Actor::facing` as last written, to notice outside changes.
    written_angle: f32,
    speed: f32,
    requested: f32,
    factor: f32,
    last_step: f32,
    turning: bool,
    instant: bool,
}
impl Default for Mover {
    fn default() -> Self {
        // 0x576920: everything zero except the speed factor +0x50 = 1.0.
        Self {
            active: false,
            dest: Vec2::ZERO,
            desired: Vec2::ZERO,
            facing: Vec2::X,
            written_angle: 0.0,
            speed: 0.0,
            requested: 0.0,
            factor: 1.0,
            last_step: 0.0,
            turning: false,
            instant: false,
        }
    }
}

/// Regiment (or single actor) move order state, `KKC_CAIStateMove`.
#[derive(Clone, Debug, Default)]
struct OrderMove {
    active: bool,
    target: Option<ActorId>,
    attack_move: bool,
    destination: Vec2,
    group_offset: Vec2,
    path_dest: Vec2,
    /// Regiment path (+0x34), consumed as the leader reaches its nodes.
    path: Vec<Waypoint>,
    /// The last route was cut by the depth cap or queue limit.
    capped: bool,
    speed: f32,
    group_speed: Option<f32>,
    tries: i32,
    start_time: f32,
    next_think: f32,
    job: Option<u64>,
    held: bool,
    deploy_facing: Option<f32>,
    /// A move to `target` (`0x520320`, `Command::Follow`), not an attack.
    follow: bool,
}

#[derive(Clone, Debug, Default)]
struct OrgMove {
    order: OrderMove,
    seen: Option<Order>,
    leader: Option<ActorId>,
    /// Organization owner facing (+0x2C).
    facing: Vec2,
    /// Slot offsets (+0x78) by slot index, world frame.
    slots: Vec<Vec2>,
    /// Members waiting for a new path (Mind +0x2BC).
    pending: Vec<ActorId>,
    /// Next idle check for members away from their slots.
    next_reform: f32,
    /// Organization +BC/+C0: the point the organization thing (`World`
    /// `orgs[o].anchor`) interpolates to (`0x57C680` / `0x57C080`); None
    /// until first used, then the anchor it started at.
    goal: Option<Vec2>,
}

#[derive(Clone, Debug, Default)]
struct ActorMove {
    /// Element move state current.
    active: bool,
    /// Move state +0x30: 0 ordinary, 1 to the exact point, 3 formation
    /// following.
    mode: u8,
    nodes: Vec<Node>,
    /// State requested speed (+0x24).
    requested: f32,
    /// 0x52FD90 waypoint turn offset (+0x54).
    waypoint_offset: f32,
    next_think: f32,
    mover: Mover,
    /// Overlapping-thing list (+0x110): (actor, until).
    ignore: Vec<(ActorId, f32)>,
    last_reroute: f32,
    /// Orders of an actor outside any organization.
    order: OrderMove,
    seen: Option<Order>,
    /// Re-entry guard for node reached -> think -> ActionGo chains.
    depth: u8,
    /// Individual pursuit of the member's combat target during an attack
    /// hold: (target, weapon range, target position when planned).
    pursue: Option<(ActorId, f32, Vec2)>,
    /// Effective speed of the last step (for `Motion`).
    step_speed: f32,
    /// Walking out of a recruiting structure (`walk_member`).
    walk: Option<MemberWalk>,
    /// A move activated with a destination (the walk out, `0x52DF60`) whose
    /// first think is ResultPlan `0x5336B0`, not the continuation.
    plan: bool,
    /// What the Mover told the Mind during its queue-0 step (its arrival
    /// `0x5774A0`, or a blocked step), handled by the Mind's think in the
    /// same update (scheduled with delay 0), not inside the Mover's delivery.
    event: Option<MoverEvent>,
    /// GActor +0xF8 as the Mover reads it: the terrain speed cap and the
    /// tile (+0x44/+0x48) it was taken at (`0x4D4130`, see [`terrain_cap`]).
    terrain: Option<([i32; 2], f32)>,
    /// Walking to its organization's task goal ([`approach_task`]).
    task: Option<Vec2>,
}
#[derive(Clone, Copy, Debug)]
enum MoverEvent {
    Reached,
    Blocked(Blocker),
}
/// `KKC_UAIStateDeploy` (Mind +0x48): Activate `0x52DE40`, Enter `0x52F560`,
/// Exit `0x52F660`, EventReachedDestination `0x52F7A0`. While it is current
/// the actor is ethereal (Thing flag 0x40000) and does not block (vf50(0)):
/// it passes through others and others through it.
#[derive(Clone, Copy, Debug)]
struct MemberWalk {
    /// Redeploys so far (+0x18).
    tries: u8,
}
impl ActorMove {
    fn kind1_count(&self) -> i32 {
        self.nodes.iter().filter(|n| n.kind == 1).count() as i32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Owner {
    Org(OrgId),
    Actor(ActorId),
}

#[derive(Clone, Copy, Debug)]
struct Job {
    id: u64,
    owner: Owner,
    request: Request,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoveStats {
    pub searches: u64,
    pub expanded: u64,
    pub queued: usize,
}

#[derive(Default, Debug)]
struct SpatialHash {
    width: i32,
    height: i32,
    heads: Vec<u32>,
    next: Vec<u32>,
    items: Vec<(ActorId, Vec2, f32)>,
    max_radius: f32,
}
const NONE: u32 = u32::MAX;
impl SpatialHash {
    fn bucket(&self, p: Vec2) -> [i32; 2] {
        [((p.x / HASH_CELL) as i32).clamp(0, self.width - 1), ((p.y / HASH_CELL) as i32).clamp(0, self.height - 1)]
    }
    fn rebuild(&mut self, world: &World, radius: &[f32], movers: &[ActorMove]) {
        self.items.clear();
        self.max_radius = 0.0;
        let extent = Vec2::new(world.map.width as f32, world.map.height as f32) * world.map.cell_size;
        self.width = ((extent.x / HASH_CELL) as i32 + 1).max(1);
        self.height = ((extent.y / HASH_CELL) as i32 + 1).max(1);
        let buckets = (self.width * self.height) as usize;
        self.heads.clear();
        self.heads.resize(buckets, NONE);
        self.next.clear();
        for a in &world.actors {
            if !a.alive || a.removed {
                continue;
            }
            let r = radius.get(a.def as usize).copied().unwrap_or(0.0);
            if r <= 0.0 || !blocks_movers(world.data.def(a.def)) {
                continue;
            }
            // A member walking out of its structure does not block (0x52F560).
            if movers.get(a.id as usize).is_some_and(|m| m.walk.is_some()) {
                continue;
            }
            let p = a.pos.truncate();
            let b = self.bucket(p);
            let index = self.items.len() as u32;
            self.items.push((a.id, p, r));
            let head = &mut self.heads[(b[1] * self.width + b[0]) as usize];
            self.next.push(*head);
            *head = index;
            self.max_radius = self.max_radius.max(r);
        }
    }
    fn for_each_near(&self, p: Vec2, reach: f32, mut f: impl FnMut(ActorId, Vec2, f32) -> bool) -> Option<ActorId> {
        if self.items.is_empty() {
            return None;
        }
        let lo = self.bucket(p - Vec2::splat(reach));
        let hi = self.bucket(p + Vec2::splat(reach));
        for by in lo[1]..=hi[1] {
            for bx in lo[0]..=hi[0] {
                let mut i = self.heads[(by * self.width + bx) as usize];
                while i != NONE {
                    let (id, q, r) = self.items[i as usize];
                    if f(id, q, r) {
                        return Some(id);
                    }
                    i = self.next[i as usize];
                }
            }
        }
        None
    }
}

/// Cells covered by standing structures: the tile rectangle of the blocking
/// shape (`aa_spatial::geometry::tile_rectangle`, 0x5541F0) for actors that
/// mark tile blockers (`marks_blocker` 0x4EE700: bounding radius * 4 >= the
/// whole cell size). Movers never mark cells. Tiles with objects carry
/// flags20 bit10, which ground and water descriptors treat as blocked.
#[derive(Clone, Default, Debug)]
struct Occupancy {
    scanned: usize,
    cells: Vec<u16>,
    registered: Vec<(ActorId, [i32; 4])>,
    version: u64,
}
impl Occupancy {
    fn update(&mut self, world: &World) {
        let n = (world.map.width * world.map.height) as usize;
        if self.cells.len() != n {
            self.cells = vec![0; n];
            self.registered.clear();
            self.scanned = 0;
            self.version += 1;
        }
        let width = world.map.width as i32;
        let before = self.registered.len();
        let cells = &mut self.cells;
        self.registered.retain(|&(id, r)| {
            let keep = alive(world, id);
            if !keep {
                for y in r[1]..=r[3] {
                    for x in r[0]..=r[2] {
                        let c = &mut cells[(y * width + x) as usize];
                        *c = c.saturating_sub(1);
                    }
                }
            }
            keep
        });
        let mut changed = self.registered.len() != before;
        while self.scanned < world.actors.len() {
            let a = &world.actors[self.scanned];
            self.scanned += 1;
            if !a.alive || a.removed {
                continue;
            }
            let def = world.data.def(a.def);
            if !is_building(def) {
                continue;
            }
            if let Some(r) = footprint(def, a.pos.truncate(), a.facing, &world.map) {
                for y in r[1]..=r[3] {
                    for x in r[0]..=r[2] {
                        let c = &mut self.cells[(y * width + x) as usize];
                        *c = c.saturating_add(1);
                    }
                }
                self.registered.push((a.id, r));
                changed = true;
            }
        }
        if changed {
            self.version += 1;
        }
    }
    fn grid<'a>(&'a self, map: &'a crate::map::GameMap, class: MoverClass) -> Grid<'a> {
        Grid::with_occupancy(map, class, &self.cells)
    }
    /// The point stands on a building's footprint.
    fn inside(&self, world: &World, p: Vec2) -> bool {
        let cell = [(p.x.max(0.0) / world.map.cell_size) as i32, (p.y.max(0.0) / world.map.cell_size) as i32];
        self.registered.iter().any(|(_, r)| cell[0] >= r[0] && cell[0] <= r[2] && cell[1] >= r[1] && cell[1] <= r[3])
    }
}
/// Buildings (StructureComponent, CityComponent or packed-truck targets:
/// `DefClass::Building`) stand on tiles; doodads such as trees do not.
fn is_building(def: &Def) -> bool {
    def.class == Some(crate::data::DefClass::Building) && def.speed_max <= 0.0
}
/// Actors that block movers: other movers and buildings.
/// Actors that block movers by their shape: other movers. Buildings block
/// through the tiles of their footprint (`Occupancy`).
fn blocks_movers(def: &Def) -> bool {
    def.speed_max > 0.0
}
/// Tile rectangle (min x, min y, max x, max y) of a structure's blocking
/// shape, or None when it does not mark blockers.
fn footprint(def: &Def, pos: Vec2, facing: f32, map: &crate::map::GameMap) -> Option<[i32; 4]> {
    let size = def.blocking_size?;
    let cell = map.cell_size;
    if cell <= 0.0 || map.width == 0 || map.height == 0 {
        return None;
    }
    let (rx, ry, radius) = match def.blocking_shape.as_deref()?.to_ascii_uppercase().as_str() {
        "CIRCLE" => (size[0], size[0], size[0]),
        _ => {
            let (hw, hh) = (size[0] * 0.5, size[1] * 0.5);
            let f = facing_vector(facing);
            let (rx, ry) = if f.x.abs() < EPSILON {
                (hw, hh)
            } else if f.y.abs() < EPSILON {
                (hh, hw)
            } else {
                (hw * f.y.abs() + hh * f.x.abs(), hw * f.x.abs() + hh * f.y.abs())
            };
            (rx, ry, (hw * hw + hh * hh).sqrt())
        }
    };
    if radius * 4.0 < cell.trunc() {
        return None;
    }
    let span = |p: f32, r: f32, n: u32| -> (i32, i32) {
        let low = ((p - r).max(0.0) / cell) as i32;
        let high = if r < EPSILON { low } else { ((p + r - EPSILON).max(0.0) / cell) as i32 };
        (low.clamp(0, n as i32 - 1), high.clamp(0, n as i32 - 1))
    };
    let (x0, x1) = span(pos.x, rx, map.width);
    let (y0, y1) = span(pos.y, ry, map.height);
    Some([x0, y0, x1, y1])
}

/// SPEED_MAX technology factor per (kingdom, definition), refreshed every
/// tick: 0x576C70 sets the Mover maximum speed to the definition speed times
/// the modifier product (`production::multiply_factor`, 502A70).
#[derive(Default, Debug)]
struct SpeedCache {
    tick: u64,
    factors: std::collections::HashMap<(Option<KingdomId>, crate::data::DefId), f32>,
}
/// Mover +0x28: the actor's maximum speed with its technology modifiers.
fn max_speed(cache: &mut SpeedCache, world: &World, id: ActorId) -> f32 {
    let a = &world.actors[id as usize];
    let def = world.data.def(a.def);
    if def.speed_max <= 0.0 {
        return 0.0;
    }
    if cache.tick != world.tick {
        cache.factors.clear();
        cache.tick = world.tick;
    }
    let factor = *cache.factors.entry((a.kingdom, a.def)).or_insert_with(|| match a.kingdom {
        Some(k) => crate::production::multiply_factor(world, k, &def.properties, "SPEED_MAX"),
        None => 1.0,
    });
    // (combat owner) the actor's enchantments' SPEED_MAX amounts join the
    // same 502A70 sum (special operations: Blitzkrieg, Forced March...)
    let enchanted = crate::special_ops::mods(world, id).map_or(0.0, |m| m.speed);
    // The active stance's property (infantry_kneel -20%, infantry_prone -60%).
    let stance = stance_speed_amount(world, id);
    if enchanted != 0.0 || stance != 0.0 {
        let clamp = world.data.rule_ci("ModifierAttributeClampMultiply").unwrap_or(0.05);
        return def.speed_max * (factor + enchanted + stance).max(clamp);
    }
    def.speed_max * factor
}

/// 45E620: the actor's active stance, the last of its definition's
/// `[Stance]` list whose health level and actor state apply (re-evaluated as
/// the actor changes, 4D0050/4D16B0). `mind_engaged` is its organization's
/// Mind state; `under_siege` is not modelled.
pub fn stance(world: &World, id: ActorId) -> Option<&crate::data::StanceDef> {
    let actor = world.actors.get(id as usize)?;
    let def = world.data.def(actor.def);
    if def.stances.is_empty() {
        return None;
    }
    let max = world.combat.actor(id).map_or(0.0, |c| c.max_health);
    let max = if max > 0.0 { max } else { def.health.max(1.0) };
    let health = actor.health / max;
    let dead = !actor.alive || matches!(actor.activity, Activity::Dying | Activity::Dead);
    let org = actor.org.and_then(|o| world.combat.org(o));
    let state = |s: &str| match s {
        "mind_engaged" => org.map_or(actor.combat.target.is_some(), |o| !o.exhausted && o.engaging()),
        "dead" => dead,
        "structure_packing" => actor.activity == Activity::Packing,
        "structure_unpacking" => actor.activity == Activity::Unpacking,
        _ => false,
    };
    def.stances
        .iter()
        .rev()
        .find(|s| (health <= s.health + 1e-6 || dead) && if s.states.is_empty() { !dead } else { s.states.iter().any(|x| state(x)) })
}

/// SPEED_MAX MULTIPLY amount of the active stance's property.
fn stance_speed_amount(world: &World, id: ActorId) -> f32 {
    let Some(property) = stance(world, id).and_then(|s| s.property.as_deref()).and_then(|p| world.data.property(p)) else {
        return 0.0;
    };
    property.modifiers.iter().filter(|m| m.kind == "MULTIPLY" && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("SPEED_MAX"))).map(|m| m.amount).sum()
}

/// The game's path solvers: the exact native solver (`Pathfinder::exact`,
/// 53E1F0 at PC53) for the requests solved on the spot, and the planner
/// that solves the queued ones ahead on worker threads ([`run_jobs`]).
#[derive(Debug)]
struct Solvers {
    main: Pathfinder,
    planner: Planner,
}
impl Default for Solvers {
    fn default() -> Self {
        Self { main: Pathfinder::exact(), planner: Planner::default() }
    }
}

/// Most threads the path planner uses, the simulation thread included
/// (engineering limit).
const PATH_THREADS: usize = 8;

/// What a queued route depends on besides its request: the map, the
/// structures on it (one occupancy version) and the path rules.
struct Snapshot {
    map: std::sync::Arc<crate::map::GameMap>,
    occupancy: Occupancy,
    rules: PathRules,
}

#[derive(Default)]
struct PlannerQueue {
    /// Requests no worker has taken yet, in queue order.
    pending: VecDeque<(u64, Request)>,
    /// Finished routes by job id; None when the solve panicked (the
    /// simulation thread solves it again).
    done: std::collections::HashMap<u64, Option<crate::path::Route>>,
    /// Cancelled requests a worker may still be solving: drop their route.
    dropped: std::collections::HashSet<u64>,
    snapshot: Option<std::sync::Arc<Snapshot>>,
    /// Bumped by every restart; a route solved before one is dropped.
    generation: u64,
    stop: bool,
}
struct PlannerShared {
    queue: std::sync::Mutex<PlannerQueue>,
    work: std::sync::Condvar,
    finished: std::sync::Condvar,
}
impl PlannerShared {
    fn lock(&self) -> std::sync::MutexGuard<'_, PlannerQueue> {
        self.queue.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Background path planning (engineering, not native): worker threads
/// solve the queued requests in queue order, ahead of [`run_jobs`], which
/// takes each route when its walk reaches the request (solving it itself
/// when no worker has started it). A route is a pure function of the map,
/// the occupancy version and its request ([`solve_job`]), so the walk gets
/// exactly the routes it would solve one by one; only the waiting changes.
#[derive(Default)]
struct Planner {
    shared: Option<std::sync::Arc<PlannerShared>>,
    workers: Vec<std::thread::JoinHandle<()>>,
    /// Map and occupancy version of the current snapshot.
    key: (usize, u64),
    /// Requests handed to the workers (pending, being solved or done).
    submitted: std::collections::HashSet<u64>,
}
impl std::fmt::Debug for Planner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Planner").field("workers", &self.workers.len()).field("submitted", &self.submitted.len()).finish()
    }
}
impl Drop for Planner {
    fn drop(&mut self) {
        if let Some(shared) = &self.shared {
            shared.lock().stop = true;
            shared.work.notify_all();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
impl Planner {
    fn worker(shared: std::sync::Arc<PlannerShared>) {
        let mut pathfinder = Pathfinder::exact();
        let mut queue = shared.lock();
        loop {
            if queue.stop {
                return;
            }
            let Some((id, request)) = queue.pending.pop_front() else {
                queue = shared.work.wait(queue).unwrap_or_else(|e| e.into_inner());
                continue;
            };
            let Some(snapshot) = queue.snapshot.clone() else {
                continue;
            };
            let generation = queue.generation;
            drop(queue);
            let route = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| solve_job(&mut pathfinder, &snapshot.occupancy, &snapshot.map, &snapshot.rules, &request))).ok();
            if route.is_none() {
                pathfinder = Pathfinder::exact();
            }
            queue = shared.lock();
            if queue.generation == generation && !queue.dropped.remove(&id) {
                queue.done.insert(id, route);
                shared.finished.notify_all();
            }
        }
    }
    /// Use `threads` threads (none beside the simulation thread for 1) on
    /// the snapshot of map and occupancy `key`, restarting when it changed.
    fn prepare(&mut self, key: (usize, u64), threads: usize, snapshot: impl FnOnce() -> Snapshot) {
        if threads < 2 {
            return;
        }
        let shared = self.shared.get_or_insert_with(|| std::sync::Arc::new(PlannerShared { queue: Default::default(), work: Default::default(), finished: Default::default() })).clone();
        while self.workers.len() < threads - 1 {
            let shared = shared.clone();
            self.workers.push(std::thread::spawn(move || Planner::worker(shared)));
        }
        let mut queue = shared.lock();
        if self.key != key || queue.snapshot.is_none() {
            queue.pending.clear();
            queue.done.clear();
            queue.dropped.clear();
            queue.generation += 1;
            queue.snapshot = Some(std::sync::Arc::new(snapshot()));
            self.submitted.clear();
            self.key = key;
        }
    }
    fn submit(&mut self, id: u64, request: Request) {
        if let Some(shared) = &self.shared
            && self.submitted.insert(id)
        {
            shared.lock().pending.push_back((id, request));
            shared.work.notify_one();
        }
    }
    /// The request was cancelled before the walk reached it.
    fn forget(&mut self, id: u64) {
        if let Some(shared) = &self.shared
            && self.submitted.remove(&id)
        {
            let mut queue = shared.lock();
            let before = queue.pending.len();
            queue.pending.retain(|p| p.0 != id);
            if queue.pending.len() == before && queue.done.remove(&id).is_none() {
                queue.dropped.insert(id);
            }
        }
    }
    /// The route of a submitted request, waiting for its worker, or solved
    /// here with `main` when no worker has taken it. None when it was not
    /// submitted or its worker panicked: the caller solves it.
    fn take(&mut self, id: u64, main: &mut Pathfinder) -> Option<crate::path::Route> {
        let shared = self.shared.as_ref()?;
        if !self.submitted.remove(&id) {
            return None;
        }
        let mut queue = shared.lock();
        loop {
            if let Some(route) = queue.done.remove(&id) {
                return route;
            }
            if let Some(at) = queue.pending.iter().position(|p| p.0 == id) {
                let (_, request) = queue.pending.remove(at)?;
                let snapshot = queue.snapshot.clone()?;
                drop(queue);
                return Some(solve_job(main, &snapshot.occupancy, &snapshot.map, &snapshot.rules, &request));
            }
            queue = shared.finished.wait(queue).unwrap_or_else(|e| e.into_inner());
        }
    }
}

/// One complete route on the map plus the structures standing on it. A
/// pure function of its inputs: the pathfinder's caches and the native
/// grid's leftover visit epochs never change a route.
fn solve_job(pathfinder: &mut Pathfinder, occupancy: &Occupancy, map: &crate::map::GameMap, rules: &PathRules, request: &Request) -> crate::path::Route {
    // A mover standing inside a structure's footprint ignores it (a
    // recruit and the building that created it).
    let cell = [(request.from.x.max(0.0) / map.cell_size) as i32, (request.from.y.max(0.0) / map.cell_size) as i32];
    let ignore: Vec<[i32; 4]> = occupancy.registered.iter().map(|r| r.1).filter(|r| cell[0] >= r[0] && cell[0] <= r[2] && cell[1] >= r[1] && cell[1] <= r[3]).collect();
    pathfinder.solve_ignoring(map, Some((&occupancy.cells, occupancy.version)), &ignore, rules, request)
}

/// Movement state kept beside the battle state (`World::movement`).
#[derive(Default, Debug)]
pub struct MovementState {
    orgs: Vec<OrgMove>,
    actors: Vec<ActorMove>,
    /// Blocking radius (m) by `DefId` from the definitions' ObjectData
    /// blocking shape (e.g. infantry CIRCLE 0.35, medium tank 0.8 in
    /// Templates/template_unit_aa.tgi). Actors without one never block.
    blocking_radius: Vec<f32>,
    pathfinder: Solvers,
    jobs: VecDeque<Job>,
    next_job: u64,
    hash: SpatialHash,
    occupancy: Occupancy,
    speeds: SpeedCache,
    pub stats: MoveStats,
    /// Movers stopped since the last [`flush_movers`].
    unregister: Vec<ActorId>,
    /// The world update [`MovementState::prepare`] last ran for, and its rules.
    prepared: Option<u64>,
    rules: Option<MoveRules>,
}
impl MovementState {
    fn ensure(&mut self, world: &World) {
        if self.blocking_radius.len() != world.data.defs.len() {
            self.blocking_radius = world.data.defs.iter().map(blocking_radius).collect();
        }
        if self.orgs.len() < world.orgs.len() {
            self.orgs.resize_with(world.orgs.len(), OrgMove::default);
        }
        while self.actors.len() < world.actors.len() {
            let actor = &world.actors[self.actors.len()];
            let mut am = ActorMove::default();
            am.mover.facing = facing_vector(actor.facing);
            am.mover.written_angle = actor.facing;
            self.actors.push(am);
        }
        self.occupancy.update(world);
    }
    /// One complete route on the map plus the structures standing on it.
    fn solve(&mut self, map: &crate::map::GameMap, rules: &PathRules, request: &Request) -> crate::path::Route {
        let route = solve_job(&mut self.pathfinder.main, &self.occupancy, map, rules, request);
        self.count(&route);
        route
    }
    fn count(&mut self, route: &crate::path::Route) {
        self.stats.searches += 1;
        self.stats.expanded += u64::from(route.expanded);
    }
    fn order(&mut self, owner: Owner) -> &mut OrderMove {
        match owner {
            Owner::Org(o) => &mut self.orgs[o as usize].order,
            Owner::Actor(a) => &mut self.actors[a as usize].order,
        }
    }
    /// Once per world update, before its first Mover step or think: the
    /// state sizes, the rules and the blocking hash of this update.
    fn prepare(&mut self, world: &World) -> MoveRules {
        if self.prepared != Some(world.tick) || self.rules.is_none() {
            self.prepared = Some(world.tick);
            self.ensure(world);
            self.rules = Some(MoveRules::from_data(&world.data));
            if self.blocking_radius.iter().any(|&r| r > 0.0) {
                self.hash.rebuild(world, &self.blocking_radius, &self.actors);
            } else {
                self.hash.items.clear();
            }
        }
        self.rules.expect("prepared")
    }
    /// Stop the actor's Mover (it leaves update queue 0, `0x50B250`, when
    /// the movement code next reaches the world: [`flush_movers`]).
    fn halt(&mut self, id: ActorId) -> &mut ActorMove {
        self.unregister.push(id);
        let am = &mut self.actors[id as usize];
        am.mover.active = false;
        am
    }
}

/// The stopped Movers leave update queue 0 (`0x50B250` in the Mover stops
/// `0x5767C0` / `0x5768A0` / `0x5768F0` / `0x5774A0` and the component
/// deactivation `0x576B60`), in the order they stopped. Run before any
/// registration and before the movement code returns to the world, so each
/// cancel lands where the original makes it (inside a Mover's own delivery
/// when it stops there).
fn flush_movers(ms: &mut MovementState, world: &mut World) {
    for id in ms.unregister.drain(..) {
        // 0x50B250 on a Mover that is not registered changes nothing.
        if world.is_scheduled(Sched::Mover(id)) {
            world.unschedule(Sched::Mover(id));
        }
    }
}

// ---------------------------------------------------------------------------
// Small helpers.

fn normalize(v: Vec2) -> Vec2 {
    let length = v.length();
    if length <= EPSILON { Vec2::ZERO } else { v * (1.0 / length) }
}
fn alive(world: &World, id: ActorId) -> bool {
    world.actors.get(id as usize).is_some_and(|a| a.alive && !a.removed)
}
fn pos2(world: &World, id: ActorId) -> Vec2 {
    world.actors[id as usize].pos.truncate()
}
fn def_of(world: &World, id: ActorId) -> &Def {
    world.data.def(world.actors[id as usize].def)
}
/// 0x576850: turn type 0 (INSTANT) zeroes radius and rate; types 1 and 2
/// (IN_PLACE, WHILE_MOVING) turn at max speed / turn radius.
fn turn_rate(def: &Def, max_speed: f32) -> f32 {
    let radius = turn_radius(def);
    if radius > 0.0 { max_speed / radius } else { 0.0 }
}
/// Mover +0x30 after 0x576850.
fn turn_radius(def: &Def) -> f32 {
    if matches!(def.turn_type, 1 | 2) { def.turn_radius.max(0.0) } else { 0.0 }
}
/// ObjectData blocking shape as a circle: the circle radius, or the
/// inscribed circle of a square/rectangle footprint.
fn blocking_radius(def: &Def) -> f32 {
    let Some(size) = def.blocking_size else {
        return 0.0;
    };
    match def.blocking_shape.as_deref().map(str::to_ascii_uppercase).as_deref() {
        Some("CIRCLE") => size[0].max(0.0),
        Some(_) => 0.5 * size[0].min(size[1]).max(0.0),
        None => 0.0,
    }
}
fn signed_angle(from: Vec2, to: Vec2) -> f32 {
    (from.x * to.y - from.y * to.x).atan2(from.dot(to))
}
fn rotate(v: Vec2, angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    normalize(Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c))
}
/// 0x5525D0: negative coordinates become 0, values beyond the extent minus 1e-4 are clamped.
fn clamp_extent(world: &World, p: Vec2) -> Vec2 {
    let extent = Vec2::new(world.map.width as f32, world.map.height as f32) * world.map.cell_size - Vec2::splat(EPSILON);
    Vec2::new(if p.x < 0.0 { 0.0 } else { p.x.min(extent.x) }, if p.y < 0.0 { 0.0 } else { p.y.min(extent.y) })
}
/// Descriptor mask of 0x5531F0: water_based movers (Thing flag 0x200000,
/// actor flag bit 21) path on water tiles, everything else on land.
fn mover_class(def: &Def) -> MoverClass {
    if def.water_based || def.mover_class == crate::data::MoverClass::Water { MoverClass::Water } else { MoverClass::Land }
}
fn member_slot(world: &World, org: OrgId, member: ActorId) -> usize {
    let o = &world.orgs[org as usize];
    world.actors[member as usize].slot.map(usize::from).unwrap_or_else(|| o.members.iter().position(|&m| m == member).unwrap_or(0))
}
/// Layout +0x18 primary slot (the captain).
fn primary_slot(def: &Def) -> Option<usize> {
    def.leader_slot.map(usize::from)
}
/// Layout slot groups (+0x1C, one per [Slot] record) for 0x57BA80: runs of
/// expanded slots with the same layout entry.
fn slot_groups(def: &Def) -> Vec<usize> {
    let entries: Vec<u32> = if def.slot_entries.len() == def.slots.len() { def.slot_entries.clone() } else { (0..def.slots.len() as u32).collect() };
    let mut groups = Vec::new();
    let mut i = 0;
    while i < entries.len() {
        let mut j = i + 1;
        while j < entries.len() && entries[j] == entries[i] {
            j += 1;
        }
        groups.push(j - i);
        i = j;
    }
    groups
}
/// Slot offsets of the current formation: the layout's default formation
/// (Organizations/Layouts, e.g. regiment_defensive), else `Def::slots`.
/// UI owner hook: `current` is the formation the formation buttons chose
/// (`crate::player_orders::formation`, layout default otherwise).
fn formation_offsets(def: &Def, current: Option<usize>) -> Vec<[f32; 2]> {
    match current.or(def.default_formation).and_then(|i| def.formations.get(i)) {
        Some(f) if !f.positions.is_empty() => f.positions.clone(),
        _ => def.slots.iter().map(|s| s.1).collect(),
    }
}
/// The living member standing in each of `slots` slots (organization +0x28).
fn slot_members(world: &World, org: OrgId, slots: usize) -> Vec<Option<formation::SlotMember>> {
    let mut members = vec![None; slots];
    for m in alive_members(world, org) {
        let s = member_slot(world, org, m);
        if s < members.len() {
            members[s] = Some(formation::SlotMember { actor: m, pos: pos2(world, m) });
        }
    }
    members
}
fn alive_members(world: &World, org: OrgId) -> impl Iterator<Item = ActorId> + '_ {
    world.orgs[org as usize].members.iter().copied().filter(move |&m| alive(world, m))
}
/// Organization +0xD8: the slowest member's maximum speed.
fn speed_cap(cache: &mut SpeedCache, world: &World, owner: Owner) -> f32 {
    match owner {
        Owner::Org(o) => alive_members(world, o).map(|m| max_speed(cache, world, m)).fold(f32::INFINITY, f32::min),
        Owner::Actor(a) => max_speed(cache, world, a),
    }
    .clamp(0.0, f32::MAX)
}
fn pick_leader(world: &World, org: OrgId) -> Option<ActorId> {
    let primary = primary_slot(world.data.def(world.orgs[org as usize].def));
    let mut first = None;
    for m in alive_members(world, org) {
        if primary.is_some() && Some(member_slot(world, org, m)) == primary {
            return Some(m);
        }
        first = first.or(Some(m));
    }
    first
}
fn owner_position(world: &World, owner: Owner) -> Vec2 {
    match owner {
        Owner::Org(o) => world.orgs[o as usize].anchor,
        Owner::Actor(a) => pos2(world, a),
    }
}
fn kingdom_team(world: &World, kingdom: Option<KingdomId>) -> Option<u32> {
    let k = kingdom?;
    world.kingdoms.iter().find(|x| x.id == k).map(|x| x.team)
}
/// Shortest of the members' longest weapon ranges: where every member has
/// a weapon that reaches.
fn hold_range(world: &World, owner: Owner) -> f32 {
    let longest = |id: ActorId| def_of(world, id).weapons.iter().map(|w| w.range).fold(0.0f32, f32::max);
    let range = match owner {
        Owner::Org(o) => alive_members(world, o).map(longest).filter(|&r| r > 0.0).fold(f32::INFINITY, f32::min),
        Owner::Actor(a) => longest(a),
    };
    if range.is_finite() { range } else { 0.0 }
}
fn set_order(world: &mut World, ms: &mut MovementState, owner: Owner, order: Order) {
    match owner {
        Owner::Org(o) => {
            world.orgs[o as usize].order = order.clone();
            ms.orgs[o as usize].seen = Some(order);
        }
        Owner::Actor(a) => {
            world.actors[a as usize].order = order.clone();
            ms.actors[a as usize].seen = Some(order);
        }
    }
}

// ---------------------------------------------------------------------------
// Commands.

/// Apply a player or computer command; Err is the refusal reason.
pub fn command(world: &mut World, command: &Command) -> Result<(), String> {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    let rules = MoveRules::from_data(&world.data);
    let result = apply_command(&mut ms, world, command, &rules);
    flush_movers(&mut ms, world);
    world.movement = ms;
    result
}

fn resolve(world: &World, kingdom: KingdomId, units: &[Unit]) -> Vec<Owner> {
    let mut out: Vec<Owner> = Vec::new();
    // A routing regiment does not take orders (morale.rs rout_think).
    let org_ok = |o: OrgId| {
        world.orgs.get(o as usize).is_some_and(|x| x.alive && x.kingdom == kingdom)
            && !routing(world, o)
            && alive_members(world, o).any(|m| def_of(world, m).speed_max > 0.0)
    };
    for unit in units {
        let owner = match *unit {
            Unit::Org(o) => org_ok(o).then_some(Owner::Org(o)),
            Unit::Actor(a) => {
                if !alive(world, a) || world.actors[a as usize].kingdom != Some(kingdom) {
                    None
                } else if let Some(o) = world.actors[a as usize].org {
                    org_ok(o).then_some(Owner::Org(o))
                } else {
                    (def_of(world, a).speed_max > 0.0).then_some(Owner::Actor(a))
                }
            }
        };
        if let Some(owner) = owner
            && !out.contains(&owner)
        {
            out.push(owner);
        }
    }
    out
}

fn routing(world: &World, org: OrgId) -> bool {
    crate::combat::org_state(world, org).is_some_and(|c| c.routing.is_some())
}
/// The Move order of a routing regiment (its rout destination).
fn rout_order(world: &World, org: OrgId) -> Option<Order> {
    crate::combat::org_state(world, org)?.routing.as_ref()?.destination.map(|to| Order::Move { to, attack_move: false })
}

fn apply_command(ms: &mut MovementState, world: &mut World, command: &Command, rules: &MoveRules) -> Result<(), String> {
    match command {
        Command::Move { kingdom, units, to, attack_move } => {
            let owners = resolve(world, *kingdom, units);
            if owners.is_empty() {
                return Err("no unit of this kingdom can move".into());
            }
            let offsets = group_offsets(world, &ms.occupancy.cells, &owners, *to);
            let mut slowest = f32::INFINITY;
            for &o in &owners {
                slowest = slowest.min(speed_cap(&mut ms.speeds, world, o));
            }
            let group_speed = (owners.len() > 1).then(|| slowest.max(rules.group_speed_minimum));
            for (&owner, &offset) in owners.iter().zip(&offsets) {
                start(ms, world, owner, *to, offset, None, *attack_move, group_speed, None);
                set_order(world, ms, owner, Order::Move { to: clamp_extent(world, *to + offset), attack_move: *attack_move });
            }
            Ok(())
        }
        Command::Attack { kingdom, units, target } => {
            if !alive(world, *target) {
                return Err("the target is gone".into());
            }
            if world.actors[*target as usize].kingdom == Some(*kingdom) {
                return Err("cannot attack an own unit".into());
            }
            let owners = resolve(world, *kingdom, units);
            if owners.is_empty() {
                return Err("no unit of this kingdom can move".into());
            }
            let to = pos2(world, *target);
            for &owner in &owners {
                start(ms, world, owner, to, Vec2::ZERO, Some(*target), false, None, None);
                set_order(world, ms, owner, Order::Attack { target: *target });
                let members: Vec<ActorId> = match owner {
                    Owner::Org(o) => alive_members(world, o).collect(),
                    Owner::Actor(a) => vec![a],
                };
                for m in members {
                    world.actors[m as usize].combat.target = Some(*target);
                }
            }
            Ok(())
        }
        Command::Stop { kingdom, units } => {
            let owners = resolve(world, *kingdom, units);
            if owners.is_empty() {
                return Err("no unit of this kingdom to stop".into());
            }
            for &owner in &owners {
                stop(ms, world, owner);
                set_order(world, ms, owner, Order::Idle);
            }
            Ok(())
        }
        // The move command with a target actor (CAIStateMove activation
        // `0x520320`; a lone actor: KKC_UAIStateMove to an actor `0x532930`):
        // the group offsets around the target, the group speed.
        Command::Follow { kingdom, units, target } => {
            let Some(tp) = world.actor(*target).filter(|a| !a.removed).map(|a| a.pos.truncate()) else {
                return Err("the target is gone".into());
            };
            let owners = resolve(world, *kingdom, units);
            if owners.is_empty() {
                return Err("no unit of this kingdom can move".into());
            }
            let offsets = group_offsets(world, &ms.occupancy.cells, &owners, tp);
            let mut slowest = f32::INFINITY;
            for &o in &owners {
                slowest = slowest.min(speed_cap(&mut ms.speeds, world, o));
            }
            let group_speed = (owners.len() > 1).then(|| slowest.max(rules.group_speed_minimum));
            for (&owner, &offset) in owners.iter().zip(&offsets) {
                start(ms, world, owner, tp, offset, Some(*target), false, group_speed, None);
                ms.order(owner).follow = true;
                set_order(world, ms, owner, Order::Move { to: clamp_extent(world, tp + offset), attack_move: false });
            }
            Ok(())
        }
        other => Err(format!("{other:?} is not a movement command")),
    }
}

/// Group formation offsets for a multi-unit move (`0x4DF840` facing toward
/// the destination, `0x4DE020` offsets; positions from group_defensive).
fn group_offsets(world: &World, occupied: &[u16], owners: &[Owner], to: Vec2) -> Vec<Vec2> {
    if owners.len() < 2 {
        return vec![Vec2::ZERO; owners.len()];
    }
    let positions: Vec<Vec2> = owners.iter().map(|&o| owner_position(world, o)).collect();
    let center = positions.iter().copied().sum::<Vec2>() / positions.len() as f32;
    let facing = normalize(to - center);
    let mut offsets: Vec<Vec2> = (0..owners.len())
        .map(|i| formation::GROUP_DEFENSIVE.get(i).map_or(Vec2::ZERO, |&p| formation::rotate_offset(Vec2::from(p), facing)))
        .collect();
    let used = owners.len().min(formation::GROUP_DEFENSIVE.len());
    let members: Vec<Option<Vec2>> = positions.iter().map(|&p| Some(p)).collect();
    formation::reassign_slots(to, &mut offsets[..used], &members[..used], None, &[used]);
    let grid = Grid::with_occupancy(&world.map, MoverClass::Land, occupied);
    offsets.iter().map(|&o| formation::group_offset(&grid, to, o)).collect()
}

/// Activation and Enter (`0x520440` / `0x523080`).
#[allow(clippy::too_many_arguments)]
fn start(ms: &mut MovementState, world: &mut World, owner: Owner, to: Vec2, offset: Vec2, target: Option<ActorId>, attack_move: bool, group_speed: Option<f32>, deploy_facing: Option<f32>) {
    let cap = speed_cap(&mut ms.speeds, world, owner);
    let now = world.time;
    let mut to = to;
    // A unit leaving the building it stands in (a recruit going to its
    // deployment point): KKC_UAIStateDeploy::Activate 0x52DE40 moves the
    // point to a free spot, FindOpenPositionLineThenRadius 0x554BF0 (4 m).
    let mover = match owner {
        Owner::Actor(a) => Some(a),
        Owner::Org(o) => pick_leader(world, o),
    };
    if target.is_none()
        && deploy_facing.is_none()
        && let Some(m) = mover
        && ms.occupancy.inside(world, pos2(world, m))
    {
        to = open_position(ms, world, m, to, DEPLOY_SEARCH_RADIUS);
    }
    let destination = clamp_extent(world, to);
    let path_dest = clamp_extent(world, to + offset);
    let o = ms.order(owner);
    *o = OrderMove {
        active: true,
        target,
        attack_move,
        destination,
        group_offset: offset,
        path_dest,
        path: Vec::new(),
        capped: false,
        // 0x5205E0: a zero request takes the cap; groups move at their slowest.
        speed: group_speed.map_or(cap, |g| g.min(cap)),
        group_speed,
        tries: 0,
        start_time: now,
        next_think: now,
        job: None,
        held: false,
        deploy_facing,
        follow: false,
    };
}

/// Stop the owner and every member where they stand.
fn stop(ms: &mut MovementState, world: &mut World, owner: Owner) {
    let o = ms.order(owner);
    o.active = false;
    o.job = None;
    o.held = false;
    o.path.clear();
    match owner {
        Owner::Org(org) => {
            let members: Vec<ActorId> = world.orgs[org as usize].members.clone();
            for m in members {
                if (m as usize) < ms.actors.len() {
                    complete_element(ms, m);
                }
            }
        }
        Owner::Actor(a) => complete_element(ms, a),
    }
}

// ---------------------------------------------------------------------------
// Tick.

/// The movement minds of one world update (after the periodic queues, whose
/// queue 0 stepped the registered Movers through [`mover_tick`]).
pub fn tick(world: &mut World) {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    let rules = ms.prepare(world);
    for a in 0..world.actors.len() {
        take_over_facing(&mut ms, world, a as ActorId);
        // A removed actor's Mover left the queues with it.
        if ms.actors[a].mover.active && !alive(world, a as ActorId) {
            ms.halt(a as ActorId);
        }
    }
    sync_orders(&mut ms, world);
    let now = world.time;
    for o in 0..world.orgs.len() {
        let om = &ms.orgs[o].order;
        if om.active && om.job.is_none() && om.next_think <= now && world.orgs[o].alive {
            order_think(&mut ms, world, Owner::Org(o as OrgId), &rules);
        }
    }
    for a in 0..world.actors.len() {
        let am = &ms.actors[a].order;
        if am.active && am.job.is_none() && am.next_think <= now {
            if alive(world, a as ActorId) {
                order_think(&mut ms, world, Owner::Actor(a as ActorId), &rules);
            } else {
                ms.actors[a].order.active = false;
            }
        }
    }
    run_jobs(&mut ms, world, &rules);
    for o in 0..world.orgs.len() {
        if world.orgs[o].alive && !ms.orgs[o].order.active {
            reform(&mut ms, world, o as OrgId, &rules);
        }
    }
    for a in 0..world.actors.len() {
        let am = &mut ms.actors[a];
        am.depth = 0;
        if am.active && am.next_think <= now {
            if alive(world, a as ActorId) {
                element_think(&mut ms, world, a as ActorId, &rules);
            } else {
                complete_element(&mut ms, a as ActorId);
            }
        }
    }
    publish(&mut ms, world);
    ms.stats.queued = ms.jobs.len();
    flush_movers(&mut ms, world);
    world.movement = ms;
}

/// Mover Tick `0x577E50` for one delivery of the actor's Mover on update
/// queue 0 (`crate::scheduler`, newest registration first, interleaved with
/// the production queues and the kingdoms): the ground step `0x5774A0`. A
/// Mover that stops here leaves the queue inside its own delivery, so the
/// dispatcher passes over its successor this update (`0x50B371`).
pub fn mover_tick(world: &mut World, id: ActorId) {
    let mut ms = std::mem::take(&mut world.movement);
    let rules = ms.prepare(world);
    if ms.actors.len() < world.actors.len() {
        ms.ensure(world);
    }
    if (id as usize) < ms.actors.len() {
        step_mover(&mut ms, world, id, &rules);
    }
    flush_movers(&mut ms, world);
    world.movement = ms;
}

/// Orders set outside `command` (the core's `Order::Deploy` for trucks,
/// or a reset to `Idle`) start or stop movement.
fn sync_orders(ms: &mut MovementState, world: &mut World) {
    for o in 0..world.orgs.len() {
        if !world.orgs[o].alive {
            continue;
        }
        // While routing the rout Move stays imposed (Idle = rout arrived).
        if let Some(rout) = rout_order(world, o as OrgId)
            && world.orgs[o].order != rout
            && world.orgs[o].order != Order::Idle
        {
            world.orgs[o].order = rout;
        }
        let order = &world.orgs[o].order;
        if ms.orgs[o].seen.as_ref() != Some(order) {
            // A new regiment's Idle has nothing to stop (a member may
            // already be walking out of its structure).
            if ms.orgs[o].seen.is_none() && *order == Order::Idle {
                ms.orgs[o].seen = Some(Order::Idle);
                continue;
            }
            let order = order.clone();
            apply_order(ms, world, Owner::Org(o as OrgId), order);
        }
    }
    for a in 0..world.actors.len() {
        let actor = &world.actors[a];
        if actor.org.is_some() || !actor.alive || actor.removed {
            continue;
        }
        if ms.actors[a].seen.as_ref() != Some(&actor.order) {
            let order = actor.order.clone();
            if ms.actors[a].seen.is_none() && order == Order::Idle {
                ms.actors[a].seen = Some(order);
                continue;
            }
            if world.data.def(actor.def).speed_max > 0.0 {
                apply_order(ms, world, Owner::Actor(a as ActorId), order);
            } else {
                ms.actors[a].seen = Some(order);
            }
        }
    }
}
fn apply_order(ms: &mut MovementState, world: &mut World, owner: Owner, order: Order) {
    match &order {
        Order::Idle => stop(ms, world, owner),
        Order::Move { to, attack_move } => start(ms, world, owner, *to, Vec2::ZERO, None, *attack_move, None, None),
        Order::Attack { target } => {
            if alive(world, *target) {
                let to = pos2(world, *target);
                start(ms, world, owner, to, Vec2::ZERO, Some(*target), false, None, None);
            }
        }
        Order::Deploy { at, facing } => {
            let truck = match owner {
                Owner::Actor(a) => Some(a),
                Owner::Org(o) => pick_leader(world, o),
            };
            if let Some(reason) = truck.and_then(|t| spot_problem(&ms.occupancy, world, t, *at)) {
                refuse(world, owner, reason);
                stop(ms, world, owner);
                set_order(world, ms, owner, Order::Idle);
                return;
            }
            start(ms, world, owner, *at, Vec2::ZERO, None, false, None, Some(*facing))
        }
    }
    match owner {
        Owner::Org(o) => ms.orgs[o as usize].seen = Some(order),
        Owner::Actor(a) => ms.actors[a as usize].seen = Some(order),
    }
}
/// Solve the queued requests in queue order within the tick's budget. Every
/// queued request that is still current goes to the [`Planner`], whose
/// workers solve them ahead in queue order, across ticks; the walk takes
/// each route when it reaches its request, exactly the route it would solve
/// itself. The planner restarts when the occupancy version changes.
fn run_jobs(ms: &mut MovementState, world: &mut World, rules: &MoveRules) {
    static THREADS: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    let threads = *THREADS.get_or_init(|| std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, PATH_THREADS));
    if ms.jobs.is_empty() {
        return;
    }
    let key = (&*world.map as *const crate::map::GameMap as usize, ms.occupancy.version);
    let map = world.map.clone();
    let occupancy = &ms.occupancy;
    ms.pathfinder.planner.prepare(key, threads, || Snapshot { map, occupancy: occupancy.clone(), rules: rules.path });
    for &job in &ms.jobs {
        let o = match job.owner {
            Owner::Org(o) => &ms.orgs[o as usize].order,
            Owner::Actor(a) => &ms.actors[a as usize].order,
        };
        if o.active && o.job == Some(job.id) {
            ms.pathfinder.planner.submit(job.id, job.request);
        }
    }
    let mut budget = SEARCH_BUDGET;
    let mut first = true;
    while let Some(job) = ms.jobs.front().copied() {
        if !first && budget == 0 {
            break;
        }
        ms.jobs.pop_front();
        let o = ms.order(job.owner);
        if !o.active || o.job != Some(job.id) {
            ms.pathfinder.planner.forget(job.id);
            continue;
        }
        first = false;
        let Solvers { main, planner } = &mut ms.pathfinder;
        let route = match planner.take(job.id, main) {
            Some(route) => route,
            None => solve_job(main, &ms.occupancy, &world.map, &rules.path, &job.request),
        };
        ms.count(&route);
        budget = budget.saturating_sub(route.expanded.max(1));
        let o = ms.order(job.owner);
        o.job = None;
        match route.status {
            Status::Failed => fail(ms, world, job.owner),
            status => {
                if route.waypoints.is_empty() {
                    finish(ms, world, job.owner);
                    continue;
                }
                let o = ms.order(job.owner);
                o.path = route.waypoints;
                o.capped = status == Status::Capped;
                dispatch(ms, world, job.owner);
            }
        }
    }
}

/// `0x526940`: count a move try; fail after the third once the minimum
/// seconds have passed since Enter.
fn increment_tries(o: &mut OrderMove, now: f32, rules: &MoveRules) -> bool {
    o.tries += 1;
    !(o.tries > 2 && rules.min_seconds_before_fail + o.start_time <= now)
}
/// Mind +0x144: the move is complete.
fn finish(ms: &mut MovementState, world: &mut World, owner: Owner) {
    let now = world.time;
    let o = ms.order(owner);
    // A move to a living actor stays current and thinks again in 1 s
    // (`0x5222E0` / `0x5324A0` with `0x52FD00`): the units keep following.
    if o.follow
        && let Some(t) = o.target
        && owner != Owner::Actor(t)
        && world.actor(t).is_some_and(|a| a.alive && !a.removed)
    {
        o.next_think = now + THINK_SECONDS;
        return;
    }
    o.active = false;
    o.held = false;
    let deploy = o.deploy_facing;
    match owner {
        Owner::Org(org) => {
            let now = world.time;
            let om = &mut ms.orgs[org as usize];
            om.pending.clear();
            om.next_reform = now + THINK_SECONDS;
            // A deploying HQ regiment keeps Order::Deploy for the deploy
            // system. The Mover never turns on the spot; the building takes
            // the commanded facing.
            if deploy.is_none() {
                set_order(world, ms, owner, Order::Idle);
            }
        }
        Owner::Actor(_) => {
            if deploy.is_none() {
                set_order(world, ms, owner, Order::Idle);
            }
        }
    }
}
/// Mind +0x124: the move failed; the units stay where they are.
fn fail(ms: &mut MovementState, world: &mut World, owner: Owner) {
    if ms.order(owner).deploy_facing.is_some() {
        refuse(world, owner, "the deploy spot cannot be reached".into());
    }
    stop(ms, world, owner);
    set_order(world, ms, owner, Order::Idle);
}
/// A refused order, reported like a refused command.
fn refuse(world: &mut World, owner: Owner, reason: String) {
    let kingdom = match owner {
        Owner::Org(o) => Some(world.orgs[o as usize].kingdom),
        Owner::Actor(a) => world.actors[a as usize].kingdom,
    };
    if let Some(kingdom) = kingdom {
        world.events.push(crate::world::Event::Refused { kingdom, reason });
    }
}
/// Why a mover cannot drive to `at`: outside the map or on a tile blocked
/// for it (terrain, water for ground units, or a standing structure).
fn spot_problem(occupancy: &Occupancy, world: &World, mover: ActorId, at: Vec2) -> Option<String> {
    let extent = Vec2::new(world.map.width as f32, world.map.height as f32) * world.map.cell_size;
    if !(at.x >= 0.0 && at.y >= 0.0 && at.x < extent.x && at.y < extent.y) {
        return Some("the deploy spot is outside the map".into());
    }
    let grid = occupancy.grid(&world.map, mover_class(def_of(world, mover)));
    grid.blocked_at(at).then(|| "the deploy spot is blocked".into())
}
/// The Mover is driving this tick (scheduled).
fn is_moving(ms: &MovementState, id: ActorId) -> bool {
    ms.actors.get(id as usize).is_some_and(|a| a.mover.active)
}
/// 0x4CEDE0 "moving": the Mover is the active component, which it stays
/// while the element's move state is current, including its waits (queued
/// behind another mover, between obstacle re-routes).
fn element_moving(ms: &MovementState, id: ActorId) -> bool {
    ms.actors.get(id as usize).is_some_and(|a| a.active || a.mover.active)
}
fn has_combat_target(world: &World, owner: Owner) -> bool {
    let valid = |id: ActorId| world.actors[id as usize].combat.target.is_some_and(|t| alive(world, t) && world.actors[t as usize].kingdom != world.actors[id as usize].kingdom);
    match owner {
        Owner::Org(o) => alive_members(world, o).any(valid),
        Owner::Actor(a) => valid(a),
    }
}
fn stop_members(ms: &mut MovementState, world: &World, owner: Owner) {
    match owner {
        Owner::Org(o) => {
            for m in alive_members(world, o).collect::<Vec<_>>() {
                complete_element(ms, m);
            }
        }
        Owner::Actor(a) => complete_element(ms, a),
    }
}

fn longest_range(def: &Def) -> f32 {
    def.weapons.iter().map(|w| w.range).fold(0.0f32, f32::max)
}
fn enemy_target(world: &World, id: ActorId) -> Option<ActorId> {
    world.actors[id as usize].combat.target.filter(|&t| alive(world, t) && world.actors[t as usize].kingdom != world.actors[id as usize].kingdom)
}
/// Members pursue individually while their regiment holds at an attack
/// target: a member whose combat target is beyond its longest weapon range
/// walks toward it (ordinary move) and stops once in range; the route is
/// renewed with the 0x5276B0 ratio when the target moves.
fn pursue_members(ms: &mut MovementState, world: &mut World, org: OrgId) {
    let rules = MoveRules::from_data(&world.data);
    let members: Vec<ActorId> = alive_members(world, org).collect();
    for m in members {
        let Some(t) = enemy_target(world, m) else {
            continue;
        };
        let range = longest_range(def_of(world, m));
        if range <= 0.0 {
            continue;
        }
        let p = pos2(world, m);
        let tp = pos2(world, t);
        if (tp - p).length() <= range {
            if ms.actors[m as usize].pursue.is_some() {
                complete_element(ms, m);
            }
            continue;
        }
        if let Some((pt, _, goal)) = ms.actors[m as usize].pursue
            && pt == t
            && ms.actors[m as usize].active
            && (p - tp).length_squared() * REPATH_RATIO > (goal - tp).length_squared()
        {
            continue;
        }
        let def = def_of(world, m);
        let speed = max_speed(&mut ms.speeds, world, m);
        let request = Request { from: p, to: tp, class: mover_class(def), max_speed: speed };
        let route = ms.solve(&world.map, &rules.path, &request);
        if route.status != Status::Failed && !route.waypoints.is_empty() {
            start_element(ms, world, m, route.waypoints.into_iter().map(Node::from).collect(), 0, speed);
            ms.actors[m as usize].pursue = Some((t, range, tp));
        }
    }
}

/// 0x537B90 mode 2: a lone actor following another stops once their shapes
/// are less than 5.0 (0x40A00000) apart.
const FOLLOW_REACH: f32 = 5.0;
/// 0x533C30: a lone actor re-targets when the target moved by more than
/// 0.3827 (0xA3547C) times the gap between them.
const ELEMENT_RETARGET: f32 = f32::from_bits(0x3ec3_f141);

/// The move to an actor in `0x5276B0` (regiment) / `0x537B90` mode 2 with
/// `0x533C30` (lone actor). True when this think is over.
/// - The target is gone (EventTargetGone `0x521F90` pops the move), or is
///   the mover itself: the order ends where the units stand.
/// - In range: a regiment within `CAITargetInRangeZOCMultiplier` times its
///   zone radius (control_range, x0.75 exhausted) of the target
///   (`0x51FBC0`), a lone actor less than 5 m from it shape to shape. While
///   the target lives the move stays and holds (vf144 `0x5222E0` /
///   `0x5324A0` with `0x52FD00`: think again in 1 s; a lone actor's Mover
///   stops), so the units go after it again once it leaves the range; a
///   dead target ends the order. A friendly target with a Portal would be
///   joined instead (EventJoin `0x51E5D0`; nothing here has one).
/// - Otherwise the route is renewed when the target moved away from the
///   last goal: regiment `|goal - last|^2 >= 0.146 |goal - own|^2`, lone
///   actor `|goal - last| >= 0.3827 x gap`.
fn follow_think(ms: &mut MovementState, world: &mut World, owner: Owner, target: ActorId, own: Vec2, rules: &MoveRules) -> bool {
    let now = world.time;
    let Some((tp, living)) = world.actor(target).filter(|a| !a.removed).map(|a| (a.pos.truncate(), a.alive)) else {
        finish(ms, world, owner);
        return true;
    };
    if owner == Owner::Actor(target) {
        finish(ms, world, owner);
        return true;
    }
    let radius = |id: ActorId| ms.blocking_radius.get(world.actors[id as usize].def as usize).copied().unwrap_or(0.0);
    let (in_range, gap) = match owner {
        Owner::Org(o) => {
            let mut zone = world.data.def(world.orgs[o as usize].def).control_range;
            if crate::combat::org_state(world, o).is_some_and(|c| c.exhausted) {
                zone *= 0.75;
            }
            ((own - tp).length() <= rules.target_in_range_zoc * zone, 0.0)
        }
        Owner::Actor(a) => {
            let gap = (own - tp).length() - radius(a) - radius(target);
            (gap < FOLLOW_REACH, gap)
        }
    };
    if in_range {
        if !living {
            finish(ms, world, owner);
            return true;
        }
        if let Owner::Actor(a) = owner {
            complete_element(ms, a);
        }
        ms.order(owner).next_think = now + THINK_SECONDS;
        return true;
    }
    let o = ms.order(owner);
    let goal = tp + o.group_offset;
    let moved = (o.path_dest - goal).length_squared();
    let renew = match owner {
        Owner::Org(_) => (own - goal).length_squared() * REPATH_RATIO <= moved,
        Owner::Actor(_) => (gap * ELEMENT_RETARGET).powi(2) <= moved,
    };
    if renew {
        o.path.clear();
        o.tries = 0;
        o.destination = clamp_extent(world, tp);
        o.path_dest = clamp_extent(world, goal);
        let to = o.path_dest;
        set_order(world, ms, owner, Order::Move { to, attack_move: false });
    }
    false
}

/// Regiment move Think `0x5276B0` (also used for single actors).
fn order_think(ms: &mut MovementState, world: &mut World, owner: Owner, rules: &MoveRules) {
    let now = world.time;
    let leader = match owner {
        Owner::Org(o) => {
            let leader = pick_leader(world, o);
            ms.orgs[o as usize].leader = leader;
            match leader {
                Some(l) => l,
                None => {
                    ms.orgs[o as usize].order.active = false;
                    return;
                }
            }
        }
        Owner::Actor(a) => a,
    };
    let own = owner_position(world, owner);
    if ms.order(owner).follow
        && let Some(target) = ms.order(owner).target
    {
        if follow_think(ms, world, owner, target, own, rules) {
            return;
        }
    } else if let Some(target) = ms.order(owner).target {
        if !alive(world, target) {
            fail(ms, world, owner);
            return;
        }
        let tp = pos2(world, target);
        if (own - tp).length() <= hold_range(world, owner) {
            if !ms.order(owner).held {
                stop_members(ms, world, owner);
            }
            let o = ms.order(owner);
            o.held = true;
            o.path.clear();
            o.next_think = now + THINK_SECONDS;
            if let Owner::Org(org) = owner {
                pursue_members(ms, world, org);
            }
            return;
        }
        let o = ms.order(owner);
        o.held = false;
        let goal = tp + o.group_offset;
        let moved = (o.path_dest - goal).length_squared();
        let reach = (own - goal).length_squared() * REPATH_RATIO;
        if reach <= moved {
            o.path.clear();
            o.tries = 0;
            o.destination = clamp_extent(world, tp);
            o.path_dest = clamp_extent(world, tp + o.group_offset);
        }
    }
    if ms.order(owner).attack_move {
        if has_combat_target(world, owner) {
            if !ms.order(owner).held {
                stop_members(ms, world, owner);
            }
            let o = ms.order(owner);
            o.held = true;
            o.next_think = now + THINK_SECONDS;
            return;
        }
        let o = ms.order(owner);
        if o.held {
            o.held = false;
            o.path.clear();
            o.tries = 0;
        }
    }
    let leader_moving = element_moving(ms, leader);
    let o = ms.order(owner);
    let mut replan = o.path.is_empty();
    if replan && o.capped {
        // Region-hop equivalent (0x52CAE0 -> tries = 0): continue the cut route.
        o.tries = 0;
    }
    if !replan && o.capped && !leader_moving {
        o.tries = 0;
        replan = true;
    }
    if replan {
        if (o.path_dest - own).length_squared() < ARRIVAL_RADIUS_SQUARED {
            finish(ms, world, owner);
            return;
        }
        if o.target.is_none() && !o.capped && !increment_tries(o, now, rules) {
            fail(ms, world, owner);
            return;
        }
        let destination = o.destination;
        let offset = o.group_offset;
        o.path_dest = clamp_extent(world, destination + offset);
        let from = pos2(world, leader);
        let to = o.path_dest;
        let request = Request { from, to, class: mover_class(def_of(world, leader)), max_speed: max_speed(&mut ms.speeds, world, leader) };
        let id = ms.next_job;
        ms.next_job += 1;
        let o = ms.order(owner);
        o.job = Some(id);
        o.capped = false;
        ms.jobs.push_back(Job { id, owner, request });
        return;
    }
    if leader_moving {
        o.next_think = now + THINK_SECONDS;
        // 0x523130 for every pending member (+0x2BC drain).
        if let Owner::Org(org) = owner {
            let pending = std::mem::take(&mut ms.orgs[org as usize].pending);
            for m in pending {
                if alive(world, m) && world.actors[m as usize].org == Some(org) && m != leader {
                    send_member_path(ms, world, org, m);
                }
            }
        }
        return;
    }
    if (o.path_dest - own).length_squared() < ARRIVAL_RADIUS_SQUARED {
        finish(ms, world, owner);
        return;
    }
    if o.group_speed.is_some() && o.speed < rules.group_speed_minimum {
        o.tries = 0;
        o.next_think = now + THINK_SECONDS;
        return;
    }
    if !increment_tries(o, now, rules) {
        fail(ms, world, owner);
        return;
    }
    dispatch(ms, world, owner);
}

/// `0x525740`: face along the first leg, place the slots, send every member
/// its path, think again in 1 s.
fn dispatch(ms: &mut MovementState, world: &mut World, owner: Owner) {
    let now = world.time;
    let o = ms.order(owner);
    o.next_think = now + THINK_SECONDS;
    let path = o.path.clone();
    let speed = o.speed;
    let Some(first) = path.first() else {
        return;
    };
    match owner {
        Owner::Actor(a) => start_element(ms, world, a, path.iter().map(|&w| Node::from(w)).collect(), 0, speed),
        Owner::Org(org) => {
            let Some(leader) = pick_leader(world, org) else {
                return;
            };
            ms.orgs[org as usize].leader = Some(leader);
            ms.orgs[org as usize].pending.clear();
            if first.dir != Vec2::ZERO {
                set_org_facing(ms, world, org, first.dir);
            }
            let members: Vec<ActorId> = alive_members(world, org).collect();
            for m in members {
                if m == leader {
                    start_element(ms, world, m, path.iter().map(|&w| Node::from(w)).collect(), 0, speed);
                } else {
                    send_member_path(ms, world, org, m);
                }
            }
        }
    }
}

/// `0x523130`: a member's copy of the regiment path offset for its slot
/// (`0x52CBD0`), followed in formation mode 3 at the regiment speed.
fn send_member_path(ms: &mut MovementState, world: &World, org: OrgId, member: ActorId) {
    let om = &ms.orgs[org as usize];
    if om.order.path.is_empty() {
        return;
    }
    let slot = member_slot(world, org, member);
    let offset = om.slots.get(slot).copied().unwrap_or(Vec2::ZERO);
    let nodes = formation::slot_path(&om.order.path, offset, om.facing).into_iter().map(Node::from).collect();
    let speed = om.order.speed;
    start_element(ms, world, member, nodes, 3, speed);
}

/// `0x4EEBE0` + `0x57BEF0`: set the organization facing and recompute the
/// slot offsets (oriented formation) with reassignment.
fn set_org_facing(ms: &mut MovementState, world: &mut World, org: OrgId, facing: Vec2) {
    let def = world.data.def(world.orgs[org as usize].def);
    let leader = ms.orgs[org as usize].leader;
    let offsets = formation_offsets(def, crate::player_orders::formation(world, org));
    let members = slot_members(world, org, offsets.len());
    let center = world.orgs[org as usize].anchor;
    let leader = leader.map(|l| (l, member_slot(world, org, l)));
    let slots = formation::organization_slots(&offsets, facing, center, &members, leader, primary_slot(def), &slot_groups(def), true);
    let om = &mut ms.orgs[org as usize];
    om.facing = facing;
    om.slots = slots;
    world.orgs[org as usize].heading = facing_angle(facing);
}

/// `0x532170`: give an element a path in mode 0 or 3.
fn start_element(ms: &mut MovementState, world: &World, id: ActorId, nodes: Vec<Node>, mode: u8, speed: f32) {
    if !alive(world, id) {
        return;
    }
    let am = &mut ms.actors[id as usize];
    am.active = true;
    am.mode = mode;
    am.nodes = nodes;
    am.requested = speed;
    am.next_think = world.time;
    am.pursue = None;
    // Any other path (the regiment's EventMoveFormationPath `0x532310`
    // activates the move with mode 2, clearing the state stack) ends a walk
    // out of the structure (Exit `0x52F660`).
    am.walk = None;
    // A path event (`0x532170` / `0x532310`) thinks with the continuation.
    am.plan = false;
    am.event = None;
    am.task = None;
}
/// Element move complete: stop the Mover where it is (`0x52D660`; the Mover
/// keeps its speed value, `0x5767C0(0, 0)`).
fn complete_element(ms: &mut MovementState, id: ActorId) {
    let am = ms.halt(id);
    am.active = false;
    am.nodes.clear();
    am.mover.turning = false;
    am.mover.last_step = 0.0;
    am.pursue = None;
    am.walk = None;
    am.plan = false;
    am.event = None;
    am.task = None;
}

// ---------------------------------------------------------------------------
// Element (member / single actor) mind.

fn element_think(ms: &mut MovementState, world: &mut World, id: ActorId, rules: &MoveRules) {
    let now = world.time;
    let pos = pos2(world, id);
    let am = &mut ms.actors[id as usize];
    if am.depth > 16 {
        // A node chain that keeps resolving itself: continue next tick.
        am.next_think = now + EPSILON;
        return;
    }
    am.depth += 1;
    // The Mover's event of this update's queue-0 step.
    if let Some(event) = am.event.take() {
        match event {
            MoverEvent::Reached => node_reached(ms, world, id, rules),
            MoverEvent::Blocked(blocker) => blocked(ms, world, id, blocker, rules),
        }
        return;
    }
    let am = &mut ms.actors[id as usize];
    let near = if am.mode == 1 { ARRIVED_SQUARED } else { NEAR_POINT_SQUARED };
    let done = am.nodes.last().is_none_or(|last| (last.pos - pos).length_squared() < near);
    if done {
        let walk = am.walk;
        complete_element(ms, id);
        if let Some(walk) = walk {
            walk_arrived(ms, world, id, walk, rules);
            return;
        }
        // A single actor's order checks its arrival on the next tick.
        if world.actors[id as usize].org.is_none() && ms.actors[id as usize].order.active {
            ms.actors[id as usize].order.next_think = now;
        }
        return;
    }
    // ResultPlan 0x5336B0 for a move activated with a destination: the
    // route, the 52FD90 offset and ActionGo(0), whose delay is the next think
    // (the film: a walking truck's ActionGo in 403, the next one in 421).
    if am.plan {
        am.plan = false;
        let radius = turn_radius(def_of(world, id));
        update_waypoint_offset(am, radius);
        action_go(ms, world, id, 0.0, rules);
        return;
    }
    // 0x5379C0 continuation.
    if !am.mover.active {
        let radius = turn_radius(def_of(world, id));
        update_waypoint_offset(am, radius);
        let speed = am.requested;
        action_go(ms, world, id, speed, rules);
        let am = &mut ms.actors[id as usize];
        if let Some(first) = am.nodes.first() {
            am.next_think = now + if first.kind == 0 { 1.0 } else { EPSILON };
        }
        return;
    }
    let head = am.nodes[0];
    if am.mover.desired != head.dir {
        if action_go(ms, world, id, 0.0, rules) {
            ms.actors[id as usize].next_think = now + EPSILON;
        }
        return;
    }
    if get_back_to_path(ms, world, id, rules) {
        return;
    }
    // 0x537150 forward check (the forward probe itself is the step collision).
    let am = &ms.actors[id as usize];
    if am.mover.facing == head.dir && (head.pos - pos).dot(head.dir) < EPSILON {
        node_reached(ms, world, id, rules);
    } else {
        action_go(ms, world, id, 0.0, rules);
    }
}

/// `0x52FD90`: start turning `r * tan(angle/2)` before a corner.
fn update_waypoint_offset(am: &mut ActorMove, radius: f32) {
    am.waypoint_offset = 0.0;
    if radius == 0.0 || am.nodes.len() < 2 || am.nodes[0].kind == 0 {
        return;
    }
    let first = am.nodes[0];
    let mut next = am.nodes[1];
    if (next.pos - first.pos).abs().element_sum() < f32::from_bits(0x3951_b717) {
        if am.nodes.len() == 2 {
            return;
        }
        next = am.nodes[2];
    }
    let angle = signed_angle(first.dir, next.dir).abs();
    am.waypoint_offset = (angle * 0.5).tan() * radius;
}

/// `0x5300A0` ActionGo: order the Mover toward the head node.
fn action_go(ms: &mut MovementState, world: &mut World, id: ActorId, speed_arg: f32, rules: &MoveRules) -> bool {
    let now = world.time;
    let pos = pos2(world, id);
    if ms.actors[id as usize].nodes.is_empty() {
        // 0x52D660 stop, think again in 1 s.
        let am = ms.halt(id);
        am.mover.last_step = 0.0;
        am.next_think = now + 1.0;
        return false;
    }
    let am = &mut ms.actors[id as usize];
    let head = am.nodes[0];
    am.mover.instant = head.kind == 0;
    let d = head.pos - pos;
    if d.length() <= EPSILON || d.dot(head.dir) <= 0.0 {
        node_reached(ms, world, id, rules);
        return false;
    }
    let mut speed = speed_arg;
    if speed < EPSILON {
        speed = if am.mover.active { am.mover.requested } else { am.requested };
    }
    let mut factor = 1.0;
    if am.mode == 3 {
        if !formation_speed(ms, world, id, rules) {
            return false;
        }
        factor = ms.actors[id as usize].mover.factor;
    }
    let am = &mut ms.actors[id as usize];
    let dest = head.pos - head.dir * am.waypoint_offset;
    begin_mover(&mut am.mover, pos, head.dir, dest, speed, factor, rules.reached_threshold);
    // 0x5773C0: the Mover becomes the actor's active component (vtbl+38 ->
    // 0x4CFC10: its deactivate 0x576B60 and activate 0x576790 each cancel
    // its queue entry, 0x50B250), then it registers on update queue 0
    // (0x50AC00 -> 0x50B2C0: the getter, one draw for its bucket and a head
    // insertion), so it is on the queue once, delivered first from the next
    // queue-0 pass. The early out of other mover modes ([mover+0x56]) is not
    // modelled.
    flush_movers(ms, world);
    if world.is_scheduled(Sched::Mover(id)) {
        world.unschedule(Sched::Mover(id));
    }
    world.schedule_periodic(crate::scheduler::EVERY_TICK, Sched::Mover(id), 0);
    let am = &mut ms.actors[id as usize];
    let base = if head.kind == 0 { ACTION_DELAY_INSTANT } else { ACTION_DELAY_PATH };
    // The world stream (getter 0x4F02B0 + unit float 0x45D400, one draw),
    // then f32(base + u * 0.3) at PC53: binary64 arithmetic rounds the same
    // for these magnitudes before the binary32 store.
    let u = world.random();
    let delay = (f64::from(base) + f64::from(u) * f64::from(ACTION_JITTER)) as f32;
    am.next_think = now + delay;
    true
}

/// `0x5773C0` ground order.
fn begin_mover(m: &mut Mover, pos: Vec2, direction: Vec2, dest: Vec2, requested: f32, factor: f32, radius: f32) {
    m.dest = dest;
    m.desired = direction;
    if (dest - pos).length_squared() < radius * radius {
        m.speed = 0.0;
    } else {
        m.factor = factor;
        m.requested = requested;
    }
    m.active = true;
}

/// `0x52FF00`: formation speed from the slot lag (mode 3).
fn formation_speed(ms: &mut MovementState, world: &World, id: ActorId, rules: &MoveRules) -> bool {
    let max = max_speed(&mut ms.speeds, world, id);
    let org = world.actors[id as usize].org.filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive));
    let Some(org) = org else {
        let am = &mut ms.actors[id as usize];
        am.mode = 0;
        am.requested = max;
        finish_speed(am, max, max, max);
        return true;
    };
    let requested = ms.actors[id as usize].requested;
    let head = ms.actors[id as usize].nodes[0];
    let catchup = rules.catchup * max;
    let speed = if head.kind != 0 {
        let mut speed = slot_lag(ms, world, id, org) * 0.5 + requested;
        let too_slow = if speed < MINIMUM_FORMATION_SPEED {
            true
        } else {
            if speed > catchup {
                speed = catchup;
            }
            speed <= MINIMUM_FORMATION_SPEED
        };
        if too_slow {
            let leader_moving = ms.orgs[org as usize].leader.is_some_and(|l| element_moving(ms, l));
            if leader_moving {
                // 0x52D660 stop, think again in 1 s.
                let am = ms.halt(id);
                am.mover.turning = false;
                am.mover.last_step = 0.0;
                am.next_think = world.time + 1.0;
            } else {
                // 0x52A3C0 finish; the regiment re-sends its path (+0x2BC).
                complete_element(ms, id);
                let om = &mut ms.orgs[org as usize];
                if om.order.active && !om.pending.contains(&id) {
                    om.pending.push(id);
                }
            }
            return false;
        }
        speed
    } else {
        let blend = path_length_difference(ms, world, id, org) * PATH_DIFFERENCE_SCALE;
        let weight = if blend < 0.0 { 0.0 } else { blend.min(1.0) };
        catchup * weight + (1.0 - weight) * requested
    };
    finish_speed(&mut ms.actors[id as usize], speed, requested, max);
    true
}
/// `0x576D20` with the factor of `0x52FF00`.
fn finish_speed(am: &mut ActorMove, speed: f32, requested: f32, max: f32) {
    am.mover.requested = speed.min(max);
    am.mover.factor = if requested != 0.0 { speed / requested } else { 1.0 };
}
fn path_length(nodes: &[Node], from: Vec2) -> f32 {
    let mut total = 0.0;
    let mut previous = from;
    for n in nodes {
        total += (n.pos - previous).length();
        previous = n.pos;
    }
    total
}
/// `0x52E300`: regiment remaining path length minus the member's.
fn path_length_difference(ms: &MovementState, world: &World, id: ActorId, org: OrgId) -> f32 {
    let am = &ms.actors[id as usize];
    let member = path_length(&am.nodes, pos2(world, id));
    let mut organization = 0.0;
    let mut previous = world.orgs[org as usize].anchor;
    for w in &ms.orgs[org as usize].order.path {
        organization += (w.pos - previous).length();
        previous = w.pos;
    }
    organization - member
}
/// `0x52E400`: how far the member trails its slot along the path.
fn slot_lag(ms: &MovementState, world: &World, id: ActorId, org: OrgId) -> f32 {
    let am = &ms.actors[id as usize];
    let om = &ms.orgs[org as usize];
    let pos = pos2(world, id);
    let member_count = am.kind1_count();
    if member_count != om.order.path.len() as i32 {
        let Some(leader) = om.leader.filter(|&l| alive(world, l)) else {
            return 0.0;
        };
        let la = &ms.actors[leader as usize];
        let leader_length = path_length(&la.nodes, pos2(world, leader));
        let difference = path_length(&am.nodes, pos) - leader_length;
        let count = (member_count - la.kind1_count()) as f32;
        if difference * count < 0.0 {
            return count * 0.5;
        }
        return difference;
    }
    let slot = om.slots.get(member_slot(world, org, id)).copied().unwrap_or(Vec2::ZERO);
    let f = om.facing;
    let lateral = f.y * slot.x - f.x * slot.y;
    let forward = f.y * slot.y + f.x * slot.x;
    let head = am.nodes[0];
    let ahead = world.orgs[org as usize].anchor - pos;
    let value = ahead.dot(head.dir) + forward;
    if am.nodes.len() <= 1 {
        return value;
    }
    let second = am.nodes[1];
    if second.kind == 0 || head.dir == second.dir {
        return value;
    }
    let distance = (pos - head.pos).length_squared();
    if CORNER_RADIUS_SQUARED <= distance {
        return value;
    }
    let cross = head.dir.y * second.dir.x - second.dir.y * head.dir.x;
    let side = if cross >= 0.0 { -lateral } else { lateral };
    side * (CORNER_RADIUS - distance.sqrt()) * CORNER_SCALE + value
}

/// `0x532B50` / `0x532C64`: when the element has finished turning onto the
/// leg but stands 1 m or more beside it, insert nodes that turn back onto
/// the leg (two arcs of the turn radius, at most pi/4).
fn get_back_to_path(ms: &mut MovementState, world: &mut World, id: ActorId, rules: &MoveRules) -> bool {
    let pos = pos2(world, id);
    let radius = turn_radius(def_of(world, id));
    // Actor virtual +0x18: the current speed (0x536AD0 compares it as one).
    let actor_speed = ms.actors[id as usize].mover.speed;
    let am = &mut ms.actors[id as usize];
    let head = am.nodes[0];
    let facing = am.mover.facing;
    let request = am.mover.desired;
    if facing != head.dir {
        return false;
    }
    let v = head.pos - pos;
    let along = request.dot(v);
    let perpendicular = v - request * along;
    let squared = perpendicular.length_squared();
    if squared < RECOVERY_SQUARED {
        return false;
    }
    if head.kind == 0 {
        while am.nodes.first().is_some_and(|n| n.kind == 0) {
            am.nodes.remove(0);
        }
        action_go(ms, world, id, 0.0, rules);
        return true;
    }
    let left = v.x * request.y - v.y * request.x < 0.0;
    // 0x532CAA: pi/4 unless the offset is small enough for two arcs.
    let mut angle = RECOVERY_ANGLE;
    let limit = 2.0 * radius * (1.0 - RECOVERY_ANGLE.cos());
    if squared < limit * limit && radius != 0.0 {
        angle = (1.0 - squared.sqrt() / (2.0 * radius)).clamp(-1.0, 1.0).acos();
    }
    let (sin, cos) = angle.sin_cos();
    let base = pos + facing * (radius * sin);
    let side = if left { Vec2::new(-facing.y, facing.x) } else { Vec2::new(facing.y, -facing.x) };
    let mut arc = base + side * (radius * (1.0 - cos));
    let mut direction = rotate(facing, if left { angle } else { -angle });
    let denominator = request.y * direction.x - direction.y * request.x;
    let mut t = angle;
    if denominator != 0.0 {
        let n = head.pos - arc;
        t = (n.x * direction.y - n.y * direction.x) / denominator;
    }
    let straight = t > 0.0;
    let endpoint;
    let distance;
    if straight {
        arc = pos;
        endpoint = head.pos;
        distance = (endpoint - pos).length();
        direction = normalize(endpoint - pos);
    } else {
        endpoint = head.pos + request * t;
        distance = (endpoint - arc).length();
    }
    let lead = distance.min(actor_speed * LEAD_SCALE);
    let mut add_kind1 = true;
    let mut inserted = Vec::new();
    if straight {
        inserted.push(Node { pos: arc + direction * lead, dir: direction, kind: 0 });
        add_kind1 = lead + EPSILON < distance;
    }
    if add_kind1 {
        inserted.insert(0, Node { pos: endpoint, dir: direction, kind: 1 });
    }
    // Prepend: the last pushed ends up first (kind 0 lead, then endpoint).
    for n in inserted {
        am.nodes.insert(0, n);
    }
    update_waypoint_offset(am, radius);
    action_go(ms, world, id, 0.0, rules);
    true
}

/// `0x530AE0`: drop the reached node (and nodes already passed), tell the
/// regiment when the leader reached one, and continue.
fn node_reached(ms: &mut MovementState, world: &mut World, id: ActorId, rules: &MoveRules) {
    let pos = pos2(world, id);
    let radius = turn_radius(def_of(world, id));
    let step = crate::DT * ms.actors[id as usize].requested;
    let initial_zero = ms.actors[id as usize].nodes.first().is_some_and(|n| n.kind == 0);
    while let Some(&head) = ms.actors[id as usize].nodes.first() {
        if head.kind != 0 {
            leader_node_reached(ms, world, id);
        }
        let am = &mut ms.actors[id as usize];
        am.nodes.remove(0);
        update_waypoint_offset(am, radius);
        let Some(&next) = am.nodes.first() else {
            break;
        };
        if initial_zero && (pos - next.pos).length_squared() < step * step {
            continue;
        }
        if (next.pos - pos).dot(next.dir) > 0.0 {
            break;
        }
    }
    element_think(ms, world, id, rules);
}

/// `0x51F7A0`: the regiment consumes its path with the leader and turns the
/// formation onto the next leg; the move finishes when the path is used up
/// at the destination.
fn leader_node_reached(ms: &mut MovementState, world: &mut World, id: ActorId) {
    let Some(org) = world.actors[id as usize].org else {
        return;
    };
    let om = &ms.orgs[org as usize];
    if !om.order.active || om.leader != Some(id) || om.order.path.is_empty() {
        return;
    }
    if (om.order.path.len() as i32) < ms.actors[id as usize].nodes.len() as i32 {
        return;
    }
    ms.orgs[org as usize].order.path.remove(0);
    if let Some(head) = ms.orgs[org as usize].order.path.first() {
        let dir = head.dir;
        if dir != Vec2::ZERO {
            set_org_facing(ms, world, org, dir);
        }
    }
    let om = &ms.orgs[org as usize].order;
    if om.path.is_empty() && !om.capped && (om.path_dest - world.orgs[org as usize].anchor).length_squared() < ARRIVAL_RADIUS_SQUARED {
        finish(ms, world, Owner::Org(org));
    }
}

// ---------------------------------------------------------------------------
// Mover.

#[derive(Clone, Copy, Debug)]
enum Blocker {
    Actor(ActorId, Vec2, f32),
    Tile,
}
enum Step {
    Arrived,
    Blocked(Blocker),
    Moved(Vec2, bool),
}

/// Another system turned the actor (combat aiming, setup): the Mover takes
/// the new facing over.
fn take_over_facing(ms: &mut MovementState, world: &World, id: ActorId) {
    let a = id as usize;
    if world.actors[a].facing != ms.actors[a].mover.written_angle {
        let f = world.actors[a].facing;
        ms.actors[a].mover.facing = facing_vector(f);
        ms.actors[a].mover.written_angle = f;
    }
}

/// One Mover delivery (`0x577E50`): the actor's step this update.
fn step_mover(ms: &mut MovementState, world: &mut World, id: ActorId, rules: &MoveRules) {
    let now = world.time;
    let a = id as usize;
    take_over_facing(ms, world, id);
    // 0x577E50 returns when the Mover is not the actor's active component
    // (it stays registered; a destroyed actor's entries left with it).
    if !alive(world, id) || !ms.actors[a].mover.active {
        return;
    }
    ms.actors[a].ignore.retain(|&(_, until)| until > now);
    if let Some((t, range, _)) = ms.actors[a].pursue
        && (!alive(world, t) || (pos2(world, t) - pos2(world, id)).length() <= range)
    {
        complete_element(ms, id);
        return;
    }
    // 0x577E50: both speeds zero -> arrival or failure stop (0x5768A0 /
    // 0x5768F0).
    let (speed, requested, dest) = {
        let m = &ms.actors[a].mover;
        (m.speed, m.requested, m.dest)
    };
    if speed == 0.0 && requested == 0.0 {
        let near = (dest - pos2(world, id)).length_squared() < rules.reached_threshold * rules.reached_threshold;
        let am = ms.halt(id);
        am.mover.turning = false;
        am.next_think = now;
        if near {
            am.event = Some(MoverEvent::Reached);
        }
        return;
    }
    let before = ms.actors[a].mover.facing;
    let outcome = ground_step(ms, world, id, rules);
    if ms.actors[a].mover.facing != before {
        write_facing(ms, world, id);
    }
    match outcome {
        Step::Arrived => tell_mind(ms, id, MoverEvent::Reached, now),
        Step::Blocked(blocker) => tell_mind(ms, id, MoverEvent::Blocked(blocker), now),
        Step::Moved(p, crossed) => {
            let z = world.map.height_at(p);
            world.actors[a].pos = glam::Vec3::new(p.x, p.y, z);
            // The position commit's element notification (0x4D0F44 ->
            // 0x5705E0): the organization's goal follows its leader.
            if let Some(o) = world.actors[a].org.filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive)) {
                org_moved(ms, world, o, id, rules);
            }
            if crossed {
                tell_mind(ms, id, MoverEvent::Reached, now);
            }
        }
    }
}
/// The tile under `p` and its terrain speed cap (TerrainData +0x38 through
/// the tile's variation, `0x54ECE0`; 0 when uncapped data says 0 or off the
/// map).
fn tile_terrain(world: &World, p: Vec2) -> ([i32; 2], f32) {
    match world.map.cell_of(p) {
        Some((x, y)) => ([x as i32, y as i32], world.map.speed_limit.get((y * world.map.width + x) as usize).copied().unwrap_or(0.0)),
        None => ([-1, -1], 0.0),
    }
}
/// The Mover's terrain speed cap, GActor +0xF8 (KKC_GActor::UpdateTerrain
/// `0x4D4130`, re-evaluated when a position commit changes the actor's
/// tile +0x44/+0x48): a member attached to a living organization takes the
/// organization thing's terrain (`0x570560`: the tile under the
/// organization's position), anything else its own tile. Its first value,
/// at the creation placement, is its own tile (the Berlin film: a ger
/// truck capped at 2.0 from its first tile change, by its regiment's
/// forest, while its own tile is grass).
fn terrain_cap(ms: &mut MovementState, world: &World, id: ActorId) -> f32 {
    let (cell, own) = tile_terrain(world, pos2(world, id));
    let cached = ms.actors[id as usize].terrain;
    if let Some((at, cap)) = cached
        && at == cell
    {
        return cap;
    }
    let org = world.actors[id as usize].org.filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive));
    let cap = match (cached, org) {
        (Some(_), Some(o)) => tile_terrain(world, world.orgs[o as usize].anchor).1,
        _ => own,
    };
    ms.actors[id as usize].terrain = Some((cell, cap));
    cap
}

/// The Mover's event for its Mind (arrival `0x5774A0` -> Mind, scheduled
/// with delay 0): the element thinks this update, after queue 0.
fn tell_mind(ms: &mut MovementState, id: ActorId, event: MoverEvent, now: f32) {
    let am = &mut ms.actors[id as usize];
    am.event = Some(event);
    am.next_think = now;
}
fn write_facing(ms: &mut MovementState, world: &mut World, id: ActorId) {
    let angle = facing_angle(ms.actors[id as usize].mover.facing);
    world.actors[id as usize].facing = angle;
    ms.actors[id as usize].mover.written_angle = angle;
}


/// `0x5774A0` ground step.
fn ground_step(ms: &mut MovementState, world: &World, id: ActorId, rules: &MoveRules) -> Step {
    let pos = pos2(world, id);
    let def = def_of(world, id);
    let dt = crate::DT;
    let terrain = terrain_cap(ms, world, id);
    let max = max_speed(&mut ms.speeds, world, id);
    if (ms.actors[id as usize].mover.dest - pos).length_squared() < EPSILON {
        // 0x5774A0 arrival: speed and +0x1C zeroed, the entry leaves the
        // queue (0x50B250), then the Mind's event.
        let m = &mut ms.halt(id).mover;
        m.turning = false;
        m.speed = 0.0;
        m.last_step = 0.0;
        return Step::Arrived;
    }
    let am = &mut ms.actors[id as usize];
    let m = &mut am.mover;
    let mut facing = m.facing;
    m.turning = false;
    if facing != m.desired {
        let amount = dt * turn_rate(def, max);
        if amount != 0.0 && !m.instant {
            let angle = signed_angle(facing, m.desired);
            if angle.abs() <= amount.abs() {
                facing = m.desired;
            } else {
                m.turning = true;
                let ccw = facing.x * m.desired.y - facing.y * m.desired.x >= 0.0;
                facing = rotate(facing, if ccw { amount } else { -amount });
            }
        } else {
            facing = m.desired;
        }
        m.facing = facing;
    }
    if m.speed != m.requested {
        if def.acceleration != 0.0 && m.requested > m.speed {
            m.speed = (dt * def.acceleration + m.speed).min(m.requested);
        } else {
            m.speed = m.requested;
        }
    }
    if terrain > 0.0 {
        let limit = terrain.max(rules.terrain_minimum_speed);
        if limit < m.speed {
            m.speed = limit;
        }
    }
    // The stored speed is the factored speed (native FST of the product).
    m.speed *= m.factor;
    let mut travel = m.speed * dt;
    let projection = (m.dest - pos).dot(m.desired);
    let crossed = projection <= travel;
    if crossed && projection > 0.0 && m.desired == facing && projection < travel {
        travel = projection;
    }
    let candidate = pos + facing * travel;
    let speed = m.speed;
    if let Some(blocker) = collision(ms, world, id, pos, candidate) {
        let m = &mut ms.actors[id as usize].mover;
        m.last_step = 0.0;
        return Step::Blocked(blocker);
    }
    let am = &mut ms.actors[id as usize];
    am.mover.last_step = speed;
    am.step_speed = travel / dt;
    if crossed {
        // At the destination: the 0x5774A0 arrival stop (speed +0x20 and
        // +0x1C zeroed).
        let am = ms.halt(id);
        am.mover.turning = false;
        am.mover.speed = 0.0;
        am.mover.last_step = 0.0;
    }
    Step::Moved(candidate, crossed)
}

/// Tile and actor blocking at a step's end point, with the original
/// exclusions.
fn collision(ms: &MovementState, world: &World, id: ActorId, pos: Vec2, candidate: Vec2) -> Option<Blocker> {
    let actor = &world.actors[id as usize];
    let def = world.data.def(actor.def);
    let grid = ms.occupancy.grid(&world.map, mover_class(def));
    let from = grid.cell_of(pos);
    let to = grid.cell_of(candidate);
    if from != to && grid.blocked(to) && !grid.blocked(from) {
        return Some(Blocker::Tile);
    }
    let radius = ms.blocking_radius.get(actor.def as usize).copied().unwrap_or(0.0);
    if radius <= 0.0 || ms.hash.items.is_empty() {
        return None;
    }
    let am = &ms.actors[id as usize];
    // Ethereal while walking out of its structure (0x52F560, flag 0x40000).
    if am.walk.is_some() {
        return None;
    }
    let now = world.time;
    let my_team = kingdom_team(world, actor.kingdom);
    let my_org_moving = actor.org.is_some_and(|o| ms.orgs.get(o as usize).is_some_and(|x| x.order.active));
    // An attack target does not block the approach; an actor being
    // followed does.
    let target = match actor.org {
        Some(o) => ms.orgs.get(o as usize).filter(|x| !x.order.follow).and_then(|x| x.order.target),
        None => Some(&am.order).filter(|x| !x.follow).and_then(|x| x.target),
    };
    let mut found = None;
    ms.hash.for_each_near(candidate, radius + ms.hash.max_radius, |other, q, r| {
        if other == id || Some(other) == target {
            return false;
        }
        let reach = radius + r;
        if (candidate - q).length_squared() >= reach * reach {
            return false;
        }
        // 0x4D67F0: things already overlapped are ignored.
        if (pos - q).length_squared() < reach * reach {
            return false;
        }
        if am.ignore.iter().any(|&(x, until)| x == other && until > now) {
            return false;
        }
        let o = &world.actors[other as usize];
        // 0x4D5600: friendly elements of two moving organizations pass.
        if my_org_moving && kingdom_team(world, o.kingdom) == my_team && o.org.is_some_and(|x| ms.orgs.get(x as usize).is_some_and(|y| y.order.active)) {
            return false;
        }
        found = Some(Blocker::Actor(other, q, r));
        true
    });
    found
}

/// Mind +0x154 after a blocked step: ignore the blocking actor for 2 s
/// (`0x534A40`) and go around it; around blocked tiles, re-route to the
/// head node with the tile solver.
fn blocked(ms: &mut MovementState, world: &mut World, id: ActorId, blocker: Blocker, rules: &MoveRules) {
    let now = world.time;
    let pos = pos2(world, id);
    let radius = ms.blocking_radius.get(world.actors[id as usize].def as usize).copied().unwrap_or(0.0);
    match blocker {
        Blocker::Actor(other, q, r) => {
            // 0x536AD0 with a moving thing heading the same way: proceed
            // while it pulls away; queue behind it (stop, think again in
            // 0.3 s) unless clearly faster (by more than 0.25 m/s); a faster
            // mover, or one met head-on, goes around it.
            if is_moving(ms, other) {
                let own = &ms.actors[id as usize].mover;
                let them = &ms.actors[other as usize].mover;
                let heading = own.facing.dot(them.facing);
                if heading > 0.0 {
                    let own_speed = own.speed;
                    let other_speed = them.speed;
                    if own_speed <= other_speed * heading {
                        return;
                    }
                    if own_speed - QUEUE_SPEED_MARGIN <= other_speed {
                        let am = ms.halt(id);
                        am.mover.last_step = 0.0;
                        am.next_think = now + QUEUE_WAIT;
                        return;
                    }
                }
            }
            let am = &mut ms.actors[id as usize];
            am.ignore.push((other, now + IGNORE_SECONDS));
            if let Some(&head) = am.nodes.first() {
                // Step aside by both radii, pass the obstacle, rejoin the path.
                // Avoidance nodes are kind 0 (instant turn), like the
                // original low-level pather's nodes.
                let dir = am.mover.facing;
                let left = Vec2::new(-dir.y, dir.x);
                let side = if (head.pos - q).dot(left) >= 0.0 { 1.0 } else { -1.0 };
                let reach = radius + r;
                let lateral = left * side * reach;
                let aside = pos + lateral;
                let past = q + dir * reach + lateral;
                let (d1, d2, d3) = (normalize(aside - pos), normalize(past - aside), normalize(head.pos - past));
                if d1 != Vec2::ZERO && d2 != Vec2::ZERO {
                    // Rejoin at the head itself, so it is reached, not skipped.
                    if d3 != Vec2::ZERO {
                        am.nodes.insert(0, Node { pos: head.pos, dir: d3, kind: 0 });
                    }
                    am.nodes.insert(0, Node { pos: past, dir: d2, kind: 0 });
                    am.nodes.insert(0, Node { pos: aside, dir: d1, kind: 0 });
                }
            }
        }
        Blocker::Tile => {
            let def = def_of(world, id);
            let request_head = ms.actors[id as usize].nodes.first().copied();
            let Some(head) = request_head else {
                complete_element(ms, id);
                return;
            };
            // A node standing on a blocked tile (a slot offset point inside a
            // building) cannot be reached: drop it and go for the next one,
            // as the low-level pather clips its goal (0x53A990).
            let grid = ms.occupancy.grid(&world.map, mover_class(def));
            if grid.blocked_at(head.pos) {
                // Keep the node (formation paths stay comparable with the
                // regiment's): move it to the nearest open tile centre.
                match nearest_open(&grid, head.pos, NODE_SHIFT_CELLS) {
                    Some(p) => ms.actors[id as usize].nodes[0].pos = p,
                    None if ms.actors[id as usize].nodes.len() > 1 => {
                        ms.actors[id as usize].nodes.remove(0);
                    }
                    None => {}
                }
                ms.halt(id);
                let radius = turn_radius(def_of(world, id));
                update_waypoint_offset(&mut ms.actors[id as usize], radius);
                let speed = ms.actors[id as usize].requested;
                action_go(ms, world, id, speed, rules);
                return;
            }
            let am = &ms.actors[id as usize];
            if am.last_reroute > 0.0 && now - am.last_reroute < rules.low_path_wait {
                // LowPathingWaitMinimum: stand and try again after the wait.
                let am = ms.halt(id);
                am.mover.last_step = 0.0;
                am.next_think = am.last_reroute + rules.low_path_wait;
                return;
            }
            let request = Request { from: pos, to: head.pos, class: mover_class(def), max_speed: max_speed(&mut ms.speeds, world, id) };
            let route = ms.solve(&world.map, &rules.path, &request);
            let am = &mut ms.actors[id as usize];
            am.last_reroute = now.max(EPSILON);
            if route.status == Status::Failed || route.waypoints.is_empty() {
                am.nodes.remove(0);
            } else {
                // The goal had to move off a blocked head: the route's end
                // replaces the head.
                if (route.goal - head.pos).length_squared() > EPSILON && am.nodes.len() > 1 {
                    am.nodes.remove(0);
                }
                // The route ends at the head (or as near as it gets); a
                // duplicate head is dropped on arrival by ActionGo.
                // Avoidance nodes are kind 0 (instant turn), like the original
                // low-level pather's, so a vehicle turns away from the
                // obstacle instead of arcing into it.
                for w in route.waypoints.iter().rev() {
                    am.nodes.insert(0, Node { pos: w.pos, dir: w.dir, kind: 0 });
                }
                // Routes are smoothed from tile centres: when the first leg
                // from an off-centre position clips a blocked tile, step to
                // the own tile's centre first.
                let grid = ms.occupancy.grid(&world.map, mover_class(def));
                let first = ms.actors[id as usize].nodes[0].pos;
                if !grid.line_clear(pos, first) {
                    let center = grid.center(grid.cell_of(pos));
                    let dir = normalize(center - pos);
                    if dir != Vec2::ZERO {
                        ms.actors[id as usize].nodes.insert(0, Node { pos: center, dir, kind: 0 });
                    }
                }
            }
        }
    }
    ms.halt(id);
    let radius = turn_radius(def_of(world, id));
    update_waypoint_offset(&mut ms.actors[id as usize], radius);
    if ms.actors[id as usize].nodes.is_empty() {
        element_failed(ms, world, id, rules);
        return;
    }
    let speed = ms.actors[id as usize].requested;
    action_go(ms, world, id, speed, rules);
}

/// The mover's blocking circle fits at `p`: no blocked tile under its
/// bounding square (tiles under the shape, like 0x553BA0) and, with
/// `movers`, no other mover's circle overlapping. The native queries count
/// things whose flags hold their mask: 0x400 (blocking) for the ring search
/// of FindOpenPositionLineThenRadius (`movers`), 0x20000400 (blocking and
/// static, Thing flag bit 29, cleared for movers with speed) for its line
/// search, the deploy arrival check and the redeploy search. Static
/// blockers here are the tile footprints.
fn spot_free(ms: &MovementState, world: &World, mover: ActorId, p: Vec2, movers: bool) -> bool {
    let def = def_of(world, mover);
    let r = ms.blocking_radius.get(world.actors[mover as usize].def as usize).copied().unwrap_or(0.0);
    let grid = ms.occupancy.grid(&world.map, mover_class(def));
    let lo = grid.cell_of(p - Vec2::splat(r));
    let hi = grid.cell_of(p + Vec2::splat((r - EPSILON).max(0.0)));
    for y in lo[1]..=hi[1] {
        for x in lo[0]..=hi[0] {
            if grid.blocked([x, y]) {
                return false;
            }
        }
    }
    !movers || ms.hash.for_each_near(p, r + ms.hash.max_radius, |o, q, rq| o != mover && (q - p).length_squared() < (r + rq) * (r + rq)).is_none()
}

/// FindOpenPositionLineThenRadius 0x554BF0: walk from `point` back toward
/// the mover in steps of its radius (at least a quarter cell, 0x553FA0,
/// static blockers) until the spot is free, then search rings of eight
/// directions within `radius` around it (0x554B30 / 0x554850 with flags 0:
/// every blocking thing, starting along the facing). Keeps the line result
/// when the ring search fails.
fn open_position(ms: &MovementState, world: &World, mover: ActorId, point: Vec2, radius: f32) -> Vec2 {
    let start = pos2(world, mover);
    let cell = world.map.cell_size;
    let r = ms.blocking_radius.get(world.actors[mover as usize].def as usize).copied().unwrap_or(0.0);
    let step = r.max(cell * 0.25);
    let mut p = point;
    let delta = p - start;
    let mut remaining = delta.length();
    let back = if remaining > 0.0 { delta * (step / remaining) } else { Vec2::ZERO };
    let mut found = false;
    loop {
        let candidate = clamp_extent(world, p);
        if spot_free(ms, world, mover, candidate, false) {
            p = candidate;
            found = true;
            break;
        }
        if remaining <= 0.0 {
            break;
        }
        p -= back;
        remaining -= step;
        if remaining <= 0.0 {
            p = start;
            remaining = 0.0;
        }
    }
    // The ring search always follows, from the line result, and also counts
    // movers; the line result stays when the rings find nothing. Natively
    // a line that finds nothing fails the whole search; here that happens
    // when a point touches a structure's tile footprint (coarser than the
    // native shape test), so the rings start at the requested point instead.
    let base = if found { p } else { point };
    open_position_near(ms, world, mover, base, radius, true).unwrap_or(p)
}
/// FindOpenPositionNear 0x554B30: `base` itself, else rings within `radius`
/// (steps of the mover's radius, at least a quarter cell), eight rotations
/// by -pi/4 starting along the mover's facing (0x554850); `movers` as in
/// [`spot_free`] (its flag argument 0 counts them, 1 does not).
fn open_position_near(ms: &MovementState, world: &World, mover: ActorId, base: Vec2, radius: f32, movers: bool) -> Option<Vec2> {
    let r = ms.blocking_radius.get(world.actors[mover as usize].def as usize).copied().unwrap_or(0.0);
    let step = r.max(world.map.cell_size * 0.25);
    let rings = ((radius / step) as i32).clamp(1, 20);
    let ring_step = radius / rings as f32;
    let facing = ms.actors[mover as usize].mover.facing;
    if spot_free(ms, world, mover, base, movers) {
        return Some(base);
    }
    for k in 1..=rings {
        let mut offset = facing * (k as f32 * ring_step);
        for _ in 0..8 {
            let candidate = clamp_extent(world, base + offset);
            if spot_free(ms, world, mover, candidate, movers) {
                return Some(candidate);
            }
            offset = rotate(offset, -std::f32::consts::FRAC_PI_4) * offset.length();
        }
    }
    None
}

/// Rings searched around a blocked node for an open tile (engineering bound).
const NODE_SHIFT_CELLS: i32 = 4;
/// Centre of the open tile nearest to `p` within `rings` tiles.
fn nearest_open(grid: &Grid, p: Vec2, rings: i32) -> Option<Vec2> {
    let c = grid.cell_of(p);
    for ring in 1..=rings {
        let mut best: Option<(f32, Vec2)> = None;
        for dy in -ring..=ring {
            for dx in -ring..=ring {
                if dx.abs() != ring && dy.abs() != ring {
                    continue;
                }
                let cell = [c[0] + dx, c[1] + dy];
                if !grid.blocked(cell) {
                    let q = grid.center(cell);
                    let d = (q - p).length_squared();
                    if best.is_none_or(|b| d < b.0) {
                        best = Some((d, q));
                    }
                }
            }
        }
        if let Some((_, q)) = best {
            return Some(q);
        }
    }
    None
}

/// An element gave up its path: a regiment member waits for the regiment to
/// send it a path again (+0x2BC), or to call it back to its slot.
fn element_failed(ms: &mut MovementState, world: &mut World, id: ActorId, rules: &MoveRules) {
    let walk = ms.actors[id as usize].walk;
    complete_element(ms, id);
    // A walk out of the structure that cannot go on ends where it stands:
    // the arrival check takes over (0x52F7A0).
    if let Some(walk) = walk {
        walk_arrived(ms, world, id, walk, rules);
        return;
    }
    if let Some(org) = world.actors[id as usize].org {
        let om = &mut ms.orgs[org as usize];
        if om.order.active && !om.pending.contains(&id) {
            om.pending.push(id);
        }
    }
}

/// A new regiment member walks out of its recruiting structure: member
/// creation `4FACA0` -> Mind vf198 `0x52F320` -> `KKC_UAIStateDeploy`.
/// `to` (the structure's deployment point) first moves to a free spot,
/// FindOpenPositionLineThenRadius `0x554BF0` within 4 m (Activate
/// `0x52DE40`). The member walks there on its own while its regiment stands
/// (Enter `0x52F560` -> `KKC_UAIStateMove::Activate` `0x52DF60`): an
/// ordinary move (mode 0) at its own top speed (speed 0 -> Mover +0x28,
/// with its technology and stance factors, `0x52FC50`), no final facing,
/// passing through others (ethereal, not blocking). On arrival (`0x52F7A0`)
/// a member overlapping something walks on to a free spot within 8 m
/// (FindOpenPositionNear `0x554B30`); after 5 such redeploys, or with no
/// free spot, it dies. Then it rejoins its regiment (`0x5324A0`, reattach
/// `0x534000`): a moving regiment sends it its formation path at its next
/// think, an idle one calls it back to its slot. A new regiment order in
/// the meantime replaces the walk.
pub fn walk_member(world: &mut World, member: ActorId, to: Vec2) {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    if alive(world, member) && def_of(world, member).speed_max > 0.0 {
        let rules = MoveRules::from_data(&world.data);
        ms.hash.rebuild(world, &ms.blocking_radius, &ms.actors);
        let at = open_position(&ms, world, member, clamp_extent(world, to), DEPLOY_SEARCH_RADIUS);
        begin_walk(&mut ms, world, member, at, 0, &rules);
        // The new Mind first updates in the next world update (this
        // update's Mind queue has run): the walk's first ActionGo, and its
        // Mover registration and delay draws, come then (the Berlin film:
        // trucks created in 402 start in 403).
        let am = &mut ms.actors[member as usize];
        if am.active && am.walk.is_some() {
            am.next_think = world.time + EPSILON;
        }
    }
    flush_movers(&mut ms, world);
    world.movement = ms;
}
/// Enter `0x52F560`: the move to `at` (mode 0, own top speed, no facing).
fn begin_walk(ms: &mut MovementState, world: &mut World, member: ActorId, at: Vec2, tries: u8, rules: &MoveRules) {
    let speed = max_speed(&mut ms.speeds, world, member);
    let request = Request { from: pos2(world, member), to: at, class: mover_class(def_of(world, member)), max_speed: speed };
    let route = ms.solve(&world.map, &rules.path, &request);
    let walk = MemberWalk { tries };
    if route.status == Status::Failed || route.waypoints.is_empty() {
        // Already there, or nowhere to go: the arrival check decides.
        complete_element(ms, member);
        walk_arrived(ms, world, member, walk, rules);
        return;
    }
    // Mode 1 (+0x30 = 1 from 0x52F560): done only at the point itself.
    start_element(ms, world, member, route.waypoints.into_iter().map(Node::from).collect(), 1, speed);
    let am = &mut ms.actors[member as usize];
    am.walk = Some(walk);
    // Activated with a destination, not given a path: its first think is
    // ResultPlan (the route here is solved at activation).
    am.plan = true;
}
/// `0x52F7A0` EventReachedDestination: the actor's shape must be clear where
/// it stands (`0x553BA0`), else it redeploys.
fn walk_arrived(ms: &mut MovementState, world: &mut World, id: ActorId, walk: MemberWalk, rules: &MoveRules) {
    let p = pos2(world, id);
    // 0x553BA0 with 0x552210(1, ..): static blocking things only.
    if spot_free(ms, world, id, p, false) {
        rejoin(ms, world, id);
        return;
    }
    let tries = walk.tries + 1;
    // FindOpenPositionNear(.., 8, 1, 0): static blocking things only.
    let next = if tries > REDEPLOY_TRIES { None } else { open_position_near(ms, world, id, p, REDEPLOY_SEARCH_RADIUS, false) };
    match next {
        Some(q) => begin_walk(ms, world, id, q, tries, rules),
        // "redeploy failed %d times - dying" / "FindOpenPositionNear failed
        // for redeploy - dying" (0x4D7960): zero health, combat's death path.
        None => world.actors[id as usize].health = 0.0,
    }
}
/// `0x5324A0` after a clear arrival: the reattach state ends for a member
/// inside its regiment's zone and the regiment lists it (+0x2BC via
/// `0x5709D0` -> `0x522840`). A moving regiment sends it its formation path
/// at its next think; an idle one calls it back to its slot (`reform`).
fn rejoin(ms: &mut MovementState, world: &World, id: ActorId) {
    let Some(org) = world.actors[id as usize].org.filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive)) else {
        return;
    };
    let om = &mut ms.orgs[org as usize];
    if om.order.active && !om.pending.contains(&id) {
        om.pending.push(id);
    }
}

/// A member's own approach to its organization's task (the element states
/// of `combat::tasks`: UAI repair `0x5359D0` status 2 follows its object,
/// UAI bombard `0x5353E0` relation 1): an ordinary move (mode 0) at its own
/// top speed to `combat::tasks::approach_goal`, renewed with the `0x5276B0`
/// ratio when that goal moves, stopped once the goal is gone (in range).
fn approach_task(ms: &mut MovementState, world: &mut World, m: ActorId, rules: &MoveRules) {
    let Some((point, _range)) = crate::combat::tasks::approach_goal(world, m) else {
        if ms.actors[m as usize].task.take().is_some() && ms.actors[m as usize].active {
            complete_element(ms, m);
        }
        return;
    };
    let p = pos2(world, m);
    let am = &ms.actors[m as usize];
    if let Some(goal) = am.task
        && am.active
        && (p - point).length_squared() * REPATH_RATIO > (goal - point).length_squared()
    {
        return;
    }
    let speed = max_speed(&mut ms.speeds, world, m);
    if speed <= 0.0 {
        return;
    }
    let request = Request { from: p, to: point, class: mover_class(def_of(world, m)), max_speed: speed };
    let route = ms.solve(&world.map, &rules.path, &request);
    if route.status != Status::Failed && !route.waypoints.is_empty() {
        start_element(ms, world, m, route.waypoints.into_iter().map(Node::from).collect(), 0, speed);
        ms.actors[m as usize].task = Some(point);
    }
}

/// `0x52E9D0` / `0x52E900` return to slot: members of an idle regiment that
/// stand still away from their slot (and have no combat target) walk back
/// to it with an ordinary move (mode 0) at their own top speed (speed 0 ->
/// Mover +0x28, `0x52FC50`), checked once per regiment think.
fn reform(ms: &mut MovementState, world: &mut World, org: OrgId, rules: &MoveRules) {
    let now = world.time;
    let om = &ms.orgs[org as usize];
    if om.order.active || now < om.next_reform {
        return;
    }
    if om.slots.is_empty() {
        // A regiment that has not moved yet: its slot table (+0x78) for its
        // heading, as its members were placed (no reassignment).
        let Some(leader) = pick_leader(world, org) else {
            return;
        };
        let def = world.data.def(world.orgs[org as usize].def);
        let facing = facing_vector(world.orgs[org as usize].heading);
        let offsets = formation_offsets(def, crate::player_orders::formation(world, org));
        let members = slot_members(world, org, offsets.len());
        let center = world.orgs[org as usize].anchor;
        let slots = formation::organization_slots(&offsets, facing, center, &members, Some((leader, member_slot(world, org, leader))), primary_slot(def), &slot_groups(def), false);
        let om = &mut ms.orgs[org as usize];
        om.leader = Some(leader);
        om.facing = facing;
        om.slots = slots;
    }
    let center = world.orgs[org as usize].anchor;
    let leader = ms.orgs[org as usize].leader;
    ms.orgs[org as usize].next_reform = now + THINK_SECONDS;
    let members: Vec<ActorId> = alive_members(world, org).collect();
    for m in members {
        // A member busy with its organization's task walks to its own goal
        // and is not called back to its slot.
        if crate::combat::tasks::busy(world, m) || ms.actors[m as usize].task.is_some() {
            approach_task(ms, world, m, rules);
            continue;
        }
        if Some(m) == leader || ms.actors[m as usize].active || world.actors[m as usize].combat.target.is_some() {
            continue;
        }
        let speed = max_speed(&mut ms.speeds, world, m);
        if speed <= 0.0 {
            continue;
        }
        let slot = member_slot(world, org, m);
        let goal = center + ms.orgs[org as usize].slots.get(slot).copied().unwrap_or(Vec2::ZERO);
        let p = pos2(world, m);
        if (goal - p).length_squared() < ARRIVAL_RADIUS_SQUARED {
            continue;
        }
        let def = def_of(world, m);
        let request = Request { from: p, to: goal, class: mover_class(def), max_speed: max_speed(&mut ms.speeds, world, m) };
        let route = ms.solve(&world.map, &rules.path, &request);
        if route.status != Status::Failed && !route.waypoints.is_empty() {
            start_element(ms, world, m, route.waypoints.into_iter().map(Node::from).collect(), 0, speed);
        }
    }
}

// ---------------------------------------------------------------------------
// Organization position and presentation.

// The organization thing's position (`World::orgs[o].anchor`, the banner)
// moves only through KKC_OrganizationComponent's goal (+BC/+C0): the leader's
// position commits (0x4D0F44 -> 0x5705E0) and the maintenance tick 0x57C8E0
// call 0x57C800, which sets the goal (0x57C680); the goal is approached by
// 0x57C080, at once and then on update queue 0 (the component's Updatable
// +8 with data 1, the same Updatable as its queue-1 maintenance) until it
// arrives (0x57D820 removes that entry, 0x50B1C0). Natively the
// organization's Mind virtual +64 / +7C gate the goal; here they are its
// move order (`OrderMove::active`).

/// aa-rules COMPONENTS slot of the Organization component, whose Updatable
/// carries the queue-1 maintenance (data 0) and the queue-0 goal (data 1).
const ORGANIZATION_SLOT: u8 = 7;
/// Data word of the queue-0 goal registration (`0x50AC00(1)`).
const GOAL_DATA: u32 = 1;
/// The simulation thread's x87 precision control (PC53).
const PC53: aa_numeric::Precision = aa_numeric::Precision::Double53;

/// The organization Mind's virtual +64 / +7C (Regiment `0x5247B0` /
/// `0x524800`): its embedded Move state is active (here its move order),
/// or its deploy state (Mind +0x280, status +0xC == 2: the Berlin film's HQ
/// regiment holds it, with the unpack point, while its truck drives there
/// and unpacks) is.
fn org_active(ms: &MovementState, world: &World, o: OrgId) -> bool {
    ms.orgs.get(o as usize).is_some_and(|om| om.order.active) || world.orgs.get(o as usize).is_some_and(|x| matches!(x.order, Order::Deploy { .. }))
}

/// `0x57C800` (`aa_actors::element_position::organization_moved`): a member
/// moved. Only the leader (+0x40) advances the goal, while the organization
/// moves, from a point other than where the organization stands.
fn org_moved(ms: &mut MovementState, world: &mut World, o: OrgId, member: ActorId, rules: &MoveRules) {
    if ms.orgs.get(o as usize).and_then(|om| om.leader) != Some(member) || !org_active(ms, world, o) {
        return;
    }
    let point = pos2(world, member);
    if world.orgs[o as usize].anchor == point {
        return;
    }
    set_goal(ms, world, o, point, rules);
}

/// `0x57C680` with `0x578E10`: store a changed goal; only an organization
/// that had reached its previous goal steps at once (`0x57C080`) and, not
/// there yet, registers on queue 0 unless it already is (`0x50B220(0, 1)`).
fn set_goal(ms: &mut MovementState, world: &mut World, o: OrgId, point: Vec2, rules: &MoveRules) {
    let anchor = world.orgs[o as usize].anchor;
    let goal = *ms.orgs[o as usize].goal.get_or_insert(anchor);
    if !org_active(ms, world, o) || point == goal {
        return;
    }
    let reached = anchor == goal;
    ms.orgs[o as usize].goal = Some(point);
    if reached && !interpolate(ms, world, o, rules) {
        let owner = Sched::OrgComponent(o, ORGANIZATION_SLOT);
        if !world.sched.queues.has_from_owner_head(owner.key(), crate::scheduler::EVERY_TICK, GOAL_DATA) {
            world.schedule_periodic(crate::scheduler::EVERY_TICK, owner, GOAL_DATA);
        }
    }
}

/// `0x57C080` (`aa_actors::organization_interpolation::interpolate`): one
/// step of the organization thing toward its goal at `MoverCatchupSpeed` ×
/// Organization+D8, at least `BannerMinCatchupSpeed`, with the native
/// binary32 stores at PC53; each point is probed with the leader's blocking
/// shape (`0x553BA0`, mask 0x400). True when it stands at the goal (an
/// arithmetic refusal ends the interpolation as arrived).
fn interpolate(ms: &mut MovementState, world: &mut World, o: OrgId, rules: &MoveRules) -> bool {
    interpolate_native(ms, world, o, rules).unwrap_or(true)
}
fn interpolate_native(ms: &mut MovementState, world: &mut World, o: OrgId, rules: &MoveRules) -> Option<bool> {
    use aa_numeric::Fast;
    use std::cmp::Ordering;
    let p = PC53;
    let n = |v: f32| Fast::from_f32(v).ok();
    let st = |v: Fast| v.to_f32().ok();
    let zero = Fast::from_i32(0);
    let goal = ms.orgs[o as usize].goal.unwrap_or(world.orgs[o as usize].anchor);
    let mut point = world.orgs[o as usize].anchor;
    let dy = st(n(goal.y)?.sub(&n(point.y)?, p).ok()?)?;
    let dx_wide = n(goal.x)?.sub(&n(point.x)?, p).ok()?;
    let dx = st(dx_wide)?;
    // Native FST, not FSTP: the first multiplicand keeps the extended delta.
    let square = dx_wide.mul(&n(dx)?, p).ok()?.add(&n(dy)?.mul(&n(dy)?, p).ok()?, p).ok()?;
    let square_stored = st(square)?;
    if square.compare(&zero) != Ordering::Greater {
        return Some(true);
    }
    // Organization+D8: the slowest member's speed.
    let cap = speed_cap(&mut ms.speeds, world, Owner::Org(o));
    let local = if cap.is_finite() { cap } else { 0.0 };
    let mut speed = st(n(rules.catchup)?.mul(&n(local)?, p).ok()?)?;
    let minimum = n(rules.banner_min_catchup)?;
    if n(speed)?.compare(&minimum) == Ordering::Less {
        speed = st(minimum)?;
    }
    let step = st(n(speed)?.mul(&n(crate::DT)?, p).ok()?)?;
    let mut distance = st(n(square_stored)?.sqrt(p).ok()?)?;
    let ratio = n(step)?.div(&n(distance)?, p).ok()?;
    let delta = [st(n(dx)?.mul(&ratio, p).ok()?)?, st(n(dy)?.mul(&ratio, p).ok()?)?];
    let near = n(step)?.add(&n(EPSILON)?, p).ok()?;
    if near.compare(&n(distance)?) != Ordering::Less {
        world.orgs[o as usize].anchor = goal;
        return Some(true);
    }
    let leader = ms.orgs[o as usize].leader.filter(|&l| alive(world, l));
    // 0x4CEDE0 and the leader Mind's virtual +80 point (its destination):
    // the organization waits while it stands (almost) there.
    if let Some(l) = leader
        && is_moving(ms, l)
        && let Some(destination) = ms.actors[l as usize].nodes.last().map(|node| node.pos)
    {
        let a = n(point.x)?.sub(&n(destination.x)?, p).ok()?;
        let b = n(point.y)?.sub(&n(destination.y)?, p).ok()?;
        let abs = |v: Fast| if v.is_negative_nonzero() { v.negated() } else { v };
        let gap = abs(a).add(&abs(b), p).ok()?;
        if gap.compare(&n(f32::from_bits(0x3951_b717))?) == Ordering::Less {
            return Some(false);
        }
    }
    if n(step)?.compare(&zero) != Ordering::Greater {
        return None;
    }
    let blocked = |ms: &MovementState, world: &World, at: Vec2| leader.is_some_and(|l| !spot_free(ms, world, l, at, true));
    loop {
        point = Vec2::new(st(n(point.x)?.add(&n(delta[0])?, p).ok()?)?, st(n(point.y)?.add(&n(delta[1])?, p).ok()?)?);
        let remaining = n(distance)?.sub(&n(step)?, p).ok()?;
        distance = st(remaining)?;
        if remaining.compare(&zero) != Ordering::Greater {
            point = goal;
        }
        if !blocked(ms, world, point) {
            world.orgs[o as usize].anchor = point;
            return Some(n(distance)?.compare(&zero) != Ordering::Greater);
        }
        if n(distance)?.compare(&zero) == Ordering::Greater {
            continue;
        }
        if blocked(ms, world, goal) {
            return Some(false);
        }
        world.orgs[o as usize].anchor = goal;
        return Some(true);
    }
}

/// The organization's queue-0 goal delivery (`0x57D820` with data 1): one
/// `0x57C080` step; at the goal its entry leaves the queue (`0x50B1C0(0,
/// 1)`, inside its own delivery).
pub fn org_tick(world: &mut World, o: OrgId) {
    let mut ms = std::mem::take(&mut world.movement);
    let rules = ms.prepare(world);
    if ms.orgs.len() <= o as usize {
        ms.ensure(world);
    }
    if (o as usize) < world.orgs.len() && world.orgs[o as usize].alive && interpolate(&mut ms, world, o, &rules) {
        let owner = Sched::OrgComponent(o, ORGANIZATION_SLOT);
        let _ = world.sched.queues.remove_matching(owner.key(), crate::scheduler::EVERY_TICK, GOAL_DATA);
    }
    flush_movers(&mut ms, world);
    world.movement = ms;
}

/// `new` takes `old`'s slot in the organization (the in-place unpack's
/// `0x4D6174` -> `0x57D8A0`: remove `0x57CD70`, add `0x57CBA0`): the added
/// member becomes the leader when it replaces it or there is none
/// (`0x579A00`), then the goal follows it (`0x57C800` at `0x57CC8B`, which
/// can register the organization's queue-0 goal and its draw).
pub fn member_replaced(world: &mut World, o: OrgId, old: ActorId, new: ActorId) {
    let mut ms = std::mem::take(&mut world.movement);
    let rules = ms.prepare(world);
    ms.ensure(world);
    if let Some(om) = ms.orgs.get_mut(o as usize) {
        if om.leader.is_none() || om.leader == Some(old) {
            om.leader = Some(new);
        }
        if alive(world, new) {
            org_moved(&mut ms, world, o, new, &rules);
        }
    }
    flush_movers(&mut ms, world);
    world.movement = ms;
}

/// The maintenance tick's `0x57C800(leader)` (`0x57C8E0`, queue 1): while
/// the organization moves, its goal follows the leader.
pub fn org_follow_leader(world: &mut World, o: OrgId) {
    let mut ms = std::mem::take(&mut world.movement);
    let rules = ms.prepare(world);
    if ms.orgs.len() <= o as usize {
        ms.ensure(world);
    }
    if let Some(leader) = ms.orgs.get(o as usize).and_then(|om| om.leader).filter(|&l| alive(world, l)) {
        org_moved(&mut ms, world, o, leader, &rules);
    }
    flush_movers(&mut ms, world);
    world.movement = ms;
}

/// Copy the movement results into the shared `Actor` fields.
fn publish(ms: &mut MovementState, world: &mut World) {
    let time = world.time;
    for a in 0..world.actors.len() {
        // This update's step (queue 0, `mover_tick`); none until the next.
        let step_speed = std::mem::take(&mut ms.actors[a].step_speed);
        let am = &ms.actors[a];
        let actor = &mut world.actors[a];
        if !actor.alive || actor.removed {
            continue;
        }
        let moving = am.mover.active;
        if am.active || !actor.motion.path.is_empty() {
            actor.motion.path.clear();
            if am.active {
                actor.motion.path.extend(am.nodes.iter().map(|n| n.pos));
            }
        }
        actor.motion.goal = if am.active { am.nodes.last().map(|n| n.pos) } else { None };
        // The Mover's speed (+0x20: zeroed by an arrival, kept by other
        // stops) and this update's displacement.
        actor.motion.speed = am.mover.speed;
        actor.motion.velocity = am.mover.facing * step_speed;
        let next = if moving {
            match actor.activity {
                Activity::Idle | Activity::Attacking => Some(Activity::Moving),
                _ => None,
            }
        } else if actor.activity == Activity::Moving {
            Some(Activity::Idle)
        } else {
            None
        };
        if let Some(next) = next
            && actor.activity != next
        {
            actor.activity = next;
            actor.activity_since = time;
        }
    }
}

// ---------------------------------------------------------------------------
// Queries for other systems.

/// True while the actor's Mover is driving it.
pub fn actor_moving(world: &World, id: ActorId) -> bool {
    world.movement.actors.get(id as usize).is_some_and(|a| a.mover.active)
}
/// True when an actor with `Order::Deploy` stands at its spot (within the
/// original 0.5 m arrival) and no longer moves. Its facing is whatever it
/// arrived with: the building takes the commanded facing.
pub fn deploy_ready(world: &World, id: ActorId) -> bool {
    let Some(actor) = world.actor(id) else {
        return false;
    };
    let Order::Deploy { at, .. } = actor.order else {
        return false;
    };
    let ms = &world.movement;
    let Some(am) = ms.actors.get(id as usize) else {
        return false;
    };
    !am.order.active && !actor_moving(world, id) && (actor.pos.truncate() - at).length_squared() < ARRIVAL_RADIUS_SQUARED
}
/// Why `truck` cannot drive to the deploy spot `at` (outside the map, or a
/// tile blocked for it by terrain, water or a standing structure); None when
/// the spot is drivable. `Order::Deploy` to such a spot is refused with this
/// reason (`Event::Refused`) and reset to `Order::Idle`.
pub fn deploy_spot_problem(world: &World, truck: ActorId, at: Vec2) -> Option<String> {
    world.actor(truck)?;
    spot_problem(&world.movement.occupancy, world, truck, at)
}
/// How the renderer places an actor's model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orientation {
    /// Model origin (native X/Y ground, Z up).
    pub position: glam::Vec3,
    /// Unit forward vector (the facing, tilted onto the ground plane).
    pub forward: glam::Vec3,
    /// Unit up vector (the ground plane normal, or +Z).
    pub up: glam::Vec3,
}

/// Things with `conform_to_terrain` (Thing flag 0x1000000; set by the
/// AAUnitVehicle and AATrailer templates).
pub fn conforms_to_terrain(def: &Def) -> bool {
    def.conform_to_terrain
}

/// The model placement of the original view (`0x458710` -> virtual +0x128
/// `0x457920` -> `0x715E90`). For a terrain-conforming actor the four
/// corners of its blocking shape (the square of its radius for a circle),
/// turned by its facing, and its centre are set on the ground; the up
/// vector is the normalized sum of the triangle normals (corner 0, centre,
/// corner 3) and (corner 1, corner 2, centre); that plane is raised to its
/// highest corner, the model stands on it but never below the ground under
/// its centre, and forward is the facing laid onto the plane. No smoothing:
/// it follows the ground every frame. Other actors stand upright at their
/// position, facing their facing.
pub fn orientation(world: &World, id: ActorId) -> Option<Orientation> {
    let actor = world.actors.get(id as usize)?;
    let def = world.data.def(actor.def);
    let f = facing_vector(actor.facing);
    let upright = Orientation { position: actor.pos, forward: glam::Vec3::new(f.x, f.y, 0.0), up: glam::Vec3::Z };
    if !conforms_to_terrain(def) {
        return Some(upright);
    }
    let (min, max) = match (def.blocking_shape.as_deref().map(str::to_ascii_uppercase).as_deref(), def.blocking_size) {
        (Some("CIRCLE"), Some(s)) => (Vec2::splat(-s[0]), Vec2::splat(s[0])),
        (Some(_), Some(s)) => (Vec2::new(-s[0], -s[1]) * 0.5, Vec2::new(s[0], s[1]) * 0.5),
        _ => return Some(upright),
    };
    let p = actor.pos.truncate();
    let on_ground = |local: Vec2| {
        let q = p + formation::rotate_offset(local, f);
        glam::Vec3::new(q.x, q.y, world.map.height_at(q))
    };
    let c0 = on_ground(min);
    let c1 = on_ground(Vec2::new(max.x, min.y));
    let c2 = on_ground(max);
    let c3 = on_ground(Vec2::new(min.x, max.y));
    let centre = on_ground((min + max) * 0.5);
    let n1 = (centre - c0).cross(c3 - centre);
    let n2 = (c2 - c1).cross(centre - c2);
    let n = (n1 + n2).normalize_or_zero();
    if n.z <= f32::EPSILON {
        return Some(upright);
    }
    let mut d = n.dot(centre);
    for c in [c3, c0, c2, c1] {
        d = d.max(n.dot(c));
    }
    let plane = |q: Vec2| (d - n.x * q.x - n.y * q.y) / n.z;
    let z = plane(p).max(world.map.height_at(p));
    let ahead = p + f;
    let forward = glam::Vec3::new(f.x, f.y, plane(ahead) - z).normalize_or_zero();
    Some(Orientation { position: glam::Vec3::new(p.x, p.y, z), forward, up: n })
}

/// Number of path requests waiting for a later tick.
pub fn queued_requests(world: &World) -> usize {
    world.movement.jobs.len()
}
