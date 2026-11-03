//! Orders to regiments and actors, path following, formation slot
//! following, terrain speed limits, turning. Owner: movement.
//!
//! The host of the original regiment and element movement, on the exact ports:
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
//! The Mover (`0x577E50`, `0x5774A0`) runs on `aa_movement`, the element Mind on
//! the `aa_mind` and `aa_navigation` ports in the [`element`] module. The regiment
//! layer (`company_move`, `cai_sleep`, unpack, the fighting states) lives in the
//! [`regiment`] module. The ports of `0x53E1F0`, `0x53C170` and `0x534A40` are in
//! [`route`] and [`uai`]. **Rendition:** a lone actor's kill order is still
//! `order_think`, a walk to the target through `crate::path`.
use crate::data::{Def, GameData};
use crate::formation::{self, facing_angle, facing_vector};
use crate::path::{Grid, MoverClass, PathRules, Pathfinder, Request, Status, Waypoint};
use crate::scheduler::Owner as Sched;
use crate::world::{Activity, ActorId, Command, KingdomId, Order, OrgId, Unit, World};
use crate::minds::HostedKind;
use glam::Vec2;
use std::collections::VecDeque;

/// The element Mind on the exact ports (census M4).
mod element;
/// The regiment Mind on the exact ports (census M7).
mod regiment;
/// The tile lists and the collision queries (census M5).
mod space;
/// The route solver `0x53E1F0` on the exact ports (census M7).
mod route;
/// The element Move's collision results on the exact ports (census M5).
mod uai;

// Native binary32 constants.
/// 1e-4 (0x38D1B717).
const EPSILON: f32 = f32::from_bits(0x38d1_b717);
/// 0x52A1F0 arrival: squared distance below 0.25.
const ARRIVAL_RADIUS_SQUARED: f32 = 0.25;
/// 0x5276B0 target re-path ratio (0x3E15F96D).
const REPATH_RATIO: f32 = f32::from_bits(0x3e15_f96d);
/// 0x525740 / 0x5276B0 schedule(1.0).
const THINK_SECONDS: f32 = 1.0;

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
    /// The same path with its kinds, counts and +0x18 as the ports hold it; stale
    /// when its length differs from `path`'s (the earlier code cut `path`).
    route: Option<aa_navigation::OwnedPath>,
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
    /// Where the leader stood at the last re-plan of a cut route.
    capped_from: Option<Vec2>,
    /// (orders) Move state +0x24 / +0x28, the command's direction (Enter 0x523080
    /// writes it to Mind +0x2B4).
    auxiliary: Vec2,
    /// Move state +0x50, the formation it dispatched for.
    formation_word: u32,
    /// Counts the state's Enter and Leave, so a port call's copy of these fields is
    /// not written over what a callback did.
    generation: u32,
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
    mover: Mover,
    /// Overlapping-thing list (+0x110): (actor, until).
    ignore: Vec<(ActorId, f32)>,
    /// Orders of an actor outside any organization.
    order: OrderMove,
    seen: Option<Order>,
    /// Individual pursuit of the member's combat target during an attack
    /// hold: (target, weapon range, target position when planned).
    pursue: Option<(ActorId, f32, Vec2)>,
    /// Mover +0x44 / +0x48 (`0x576AD0`), from the Kill state's pursuit `0x52EF40`: it
    /// stops within this range of the thing, and the Mind thinks.
    chase: Option<(ActorId, f32)>,
    /// Element +0x24: when a detached element asleep teleports (`0x570B90`).
    teleport_at: f32,
    /// Mind +0x1B0: what its Move could not reach, and when (`0x532840`, `0x530E00`).
    bad_targets: Vec<(ActorId, f32)>,
    /// Effective speed of the last step (for `Motion`).
    step_speed: f32,
    /// Walking out of a recruiting structure (`walk_member`).
    walk: Option<MemberWalk>,
    /// Mind +0x184 is 2: the Reattach (+0x178) is current.
    reattaching: bool,
    /// GActor +0xF8 as the Mover reads it: the terrain speed cap and the
    /// tile (+0x44/+0x48) it was taken at (`0x4D4130`, see [`terrain_cap`]).
    terrain: Option<([i32; 2], f32)>,
    /// Walking to its organization's task goal ([`approach_task`]).
    task: Option<Vec2>,
    /// The hosted Mind's UAIStateMove fields ([`element`]); out while a port
    /// runs on them.
    payload: Option<Box<aa_mind::element_move::Payload>>,
    /// Actor +0x110: what the actor overlaps (`0x4D67F0`) and ignores for a while
    /// (`0x4D6940`, `0x534A40`), head first. Read by `0x4D5600`.
    overlaps: Vec<aa_spatial::overlaps::OverlapEntry>,
    /// KKC_UAIStateDeploy +10/+14 and KKC_UAIStateUnpack +10/+14.
    walk_to: Vec2,
    unpack_at: Vec2,
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
    formations: std::collections::HashMap<(crate::data::DefId, usize), f32>,
}
/// Mover +0x28: the actor's maximum speed with its technology modifiers.
fn max_speed(cache: &mut SpeedCache, world: &World, id: ActorId) -> f32 {
    max_speed_with(cache, world, id, true)
}

/// [`max_speed`]; `formed` false leaves the formation's property out, what a new
/// member's Mover has until `0x57BF50` gives it the property.
fn max_speed_with(cache: &mut SpeedCache, world: &World, id: ActorId, formed: bool) -> f32 {
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
    // The organization's formation property (0x57BF50 -> 0x4CEF10), which
    // reaches the member's 0x45D6F0 through its organization.
    let formed = if formed { formation_speed_amount(cache, world, id) } else { 0.0 };
    if enchanted != 0.0 || stance != 0.0 || formed != 0.0 {
        let clamp = world.data.rule_ci("ModifierAttributeClampMultiply").unwrap_or(0.05);
        return def.speed_max * (factor + enchanted + stance + formed).max(clamp);
    }
    def.speed_max * factor
}

/// SPEED_MAX MULTIPLY amount of the formation of the actor's organization,
/// by (organization definition, formation index): the data never changes.
fn formation_speed_amount(cache: &mut SpeedCache, world: &World, id: ActorId) -> f32 {
    let Some(org) = world.actors[id as usize].org else {
        return 0.0;
    };
    let Some(o) = world.orgs.get(org as usize) else {
        return 0.0;
    };
    // No formation: the layout's BrokenProperty (0x57BF50 with +0x74 null).
    let index = crate::player_orders::formation(world, org);
    *cache.formations.entry((o.def, index.unwrap_or(usize::MAX))).or_insert_with(|| {
        let def = world.data.def(o.def);
        let list = match index {
            Some(index) => def.formations.get(index).and_then(|f| world.data.formation_modifiers.get(&f.id.to_ascii_lowercase())),
            None => def.layout.as_ref().and_then(|l| world.data.formation_modifiers.get(&crate::data::broken_key(l))),
        };
        let Some(list) = list else {
            return 0.0;
        };
        list.iter().filter(|m| m.kind == "MULTIPLY" && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("SPEED_MAX"))).map(|m| m.amount).sum()
    })
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
    // The hosted element Mind ignores a structure only while it walks out of it
    // (KKC_UAIStateDeploy +1C, `solve_walking`). The Berlin film: a regiment standing
    // on the row of its HQ's tile rectangle first goes to the tile beside it.
    solve_job_ignoring(pathfinder, occupancy, map, rules, request, false)
}
/// [`solve_job`], a mover standing inside a structure's footprint ignoring it when
/// `ignore_start` (a recruit and the building that created it).
fn solve_job_ignoring(pathfinder: &mut Pathfinder, occupancy: &Occupancy, map: &crate::map::GameMap, rules: &PathRules, request: &Request, ignore_start: bool) -> crate::path::Route {
    let cell = [(request.from.x.max(0.0) / map.cell_size) as i32, (request.from.y.max(0.0) / map.cell_size) as i32];
    let ignore: Vec<[i32; 4]> = if ignore_start {
        occupancy.registered.iter().map(|r| r.1).filter(|r| cell[0] >= r[0] && cell[0] <= r[2] && cell[1] >= r[1] && cell[1] <= r[3]).collect()
    } else {
        Vec::new()
    };
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
    /// Movers stopped since the last [`flush`].
    unregister: Vec<ActorId>,
    /// Element minds re-scheduled since the last [`flush`] (delay after the
    /// update's begin, in order).
    thinks: Vec<(ActorId, f32)>,
    /// The world update [`MovementState::prepare`] last ran for, and its rules.
    prepared: Option<u64>,
    rules: Option<MoveRules>,
    /// Port errors met by the Movers (kept for diagnostics, at most 64).
    pub notes: Vec<String>,
    /// Trucks whose Unpack Think asked for `0x575C80` ([`element`]).
    unpacks: Vec<ActorId>,
    /// GWorld+0x30 and +0x23C ([`space`]); out of here while a query runs.
    space: Option<Box<space::Space>>,
    /// `[B3652C]`, the SAI regions of the map ([`route`]).
    regions: Option<Box<route::Regions>>,
}
impl MovementState {
    fn note(&mut self, what: String) {
        if self.notes.len() < 64 {
            self.notes.push(what);
        }
    }
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
        if let Some(mut s) = self.space.take() {
            s.add_new(self, world);
            self.space = Some(s);
        }
    }
    /// One complete route on the map plus the structures standing on it.
    fn solve(&mut self, map: &crate::map::GameMap, rules: &PathRules, request: &Request) -> crate::path::Route {
        let route = solve_job(&mut self.pathfinder.main, &self.occupancy, map, rules, request);
        self.count(&route);
        route
    }
    /// [`MovementState::solve`] for a mover walking out of its structure.
    fn solve_walking(&mut self, map: &crate::map::GameMap, rules: &PathRules, request: &Request) -> crate::path::Route {
        let route = solve_job_ignoring(&mut self.pathfinder.main, &self.occupancy, map, rules, request, true);
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
            let mut sp = match self.space.take() {
                Some(s) if s.fits(world) => Some(s),
                _ => {
                    self.regions = None;
                    space::Space::with_scenery(world)
                }
            };
            if let Some(s) = sp.as_mut() {
                s.sync(self, world);
            }
            self.space = sp;
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
/// The element minds' re-thinks go on update queue 4 in the order they were
/// made: the Mind's schedule (`0x52DC10` -> `0x50AD80`) first cancels its
/// pending think (its Updatable +10 `0x529CD0`: `0x50B250`), then adds one
/// `delay` after the update's begin (`0x50AF10`).
fn flush_minds(ms: &mut MovementState, world: &mut World) {
    for (id, delay) in ms.thinks.drain(..) {
        if world.is_scheduled(Sched::Mind(id)) {
            world.unschedule(Sched::Mind(id));
        }
        world.schedule_after(crate::scheduler::THINK, Sched::Mind(id), 0, delay);
    }
}
/// [`flush_movers`] and [`flush_minds`]: before the movement code returns
/// to the world.
fn flush(ms: &mut MovementState, world: &mut World) {
    flush_movers(ms, world);
    flush_minds(ms, world);
}
/// A Mind's schedule for minds outside the movement code (the aircraft
/// minds): `0x52DC10` -> `0x50AD80` cancels its pending think (`0x529CD0`:
/// `0x50B250`), then adds one on queue 4 `delay` after this update's begin
/// (`0x50AF10`). Immediate, as the native call is.
pub fn think_mind_in(world: &mut World, id: ActorId, delay: f32) {
    if world.is_scheduled(Sched::Mind(id)) {
        world.unschedule(Sched::Mind(id));
    }
    world.schedule_after(crate::scheduler::THINK, Sched::Mind(id), 0, delay);
}

// ---------------------------------------------------------------------------
// Small helpers.

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
/// (IN_PLACE, WHILE_MOVING) turn at max speed / turn radius. Mover +0x34 as
/// `0x576850` stores it (`aa_movement::mover::refresh_turn_rate`
/// at PC53: the binary64 quotient rounded to binary32).
fn turn_rate_native(def: &Def, max_speed: f32) -> f32 {
    match def.turn_type {
        1 | 2 if def.turn_radius != 0.0 => (f64::from(max_speed) / f64::from(def.turn_radius)) as f32,
        _ => 0.0,
    }
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
/// 0x5525D0: negative coordinates become 0, values beyond the extent minus 1e-4 are clamped.
pub(crate) fn clamp_extent(world: &World, p: Vec2) -> Vec2 {
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
    flush(&mut ms, world);
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
                // (combat, X1) 0x51D920 on the regiment's Mind: the Kill state, in mode 2.
                if let Owner::Org(o) = owner {
                    if regiment::fight::kill_command(ms, world, *rules, o, *target) {
                        continue;
                    }
                }
                // (orders) 0x51D920: the kill state already on this target only
                // reschedules its Think.
                let same = match owner {
                    Owner::Org(o) => world.orgs[o as usize].order == Order::Attack { target: *target },
                    Owner::Actor(a) => world.actors[a as usize].order == Order::Attack { target: *target },
                };
                if same {
                    continue;
                }
                // (orders) 0x51D920 -> 0x57A500 refuses: the handlers' Move at the thing
                // 0x520320, its place taken from the group (0x51E9B0).
                if let Owner::Org(o) = owner
                    && !crate::combat::kill_accepts(world, o, *target)
                {
                    let offset = crate::orders::group::member_offset(world, owner_unit(owner), to);
                    start(ms, world, owner, to, offset, Some(*target), false, None, None);
                    ms.order(owner).follow = true;
                    regiment_move_to_actor(ms, world, owner, *target, rules);
                    set_order(world, ms, owner, Order::Move { to: clamp_extent(world, to + offset), attack_move: false });
                    continue;
                }
                let offset = crate::orders::group::member_offset(world, owner_unit(owner), to);
                start(ms, world, owner, to, offset, Some(*target), false, None, None);
                set_order(world, ms, owner, Order::Attack { target: *target });
                // (combat, X1) 0x51D920 gives a regiment's members nothing: they take
                // targets from the engage state, once the target is in the ZOC.
                if let Owner::Actor(a) = owner {
                    world.actors[a as usize].combat.target = Some(*target);
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
                // (combat) 0x51D1C0: the sleep state replaces the engage state
                // (0x52A420), whose exit 0x520FE0 empties the list.
                if let Owner::Org(o) = owner
                    && let Some(s) = world.combat.orgs.get_mut(o as usize)
                {
                    s.engage.clear();
                }
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
                regiment_move_to_actor(ms, world, owner, *target, rules);
                set_order(world, ms, owner, Order::Move { to: clamp_extent(world, tp + offset), attack_move: false });
            }
            Ok(())
        }
        other => Err(format!("{other:?} is not a movement command")),
    }
}

/// `0x524C10` on the regiment's Mind, from the ZOC and from damage, with the combat
/// state out of the world. Inside the Mind's own world it waits for the next update.
pub(crate) fn mind_add_target(world: &mut World, st: &mut crate::combat::CombatState, o: OrgId, t: crate::combat::TargetRef) -> bool {
    let mut ms = std::mem::take(&mut world.movement);
    if ms.orgs.len() <= o as usize {
        world.movement = ms;
        st.orgs[o as usize].pending_targets.push(t);
        return false;
    }
    let rules = ms.prepare(world);
    std::mem::swap(&mut world.combat, st);
    let r = regiment::fight::add_target(&mut ms, world, rules, o, t);
    std::mem::swap(&mut world.combat, st);
    world.movement = ms;
    r
}

/// `0x51F920` (`exhausted` false) and `0x51F9F0` from the morale, with the combat state
/// out of the world. A regiment the movement pass holds has it asked at its next think.
pub(crate) fn mind_morale_reaction(world: &mut World, st: &mut crate::combat::CombatState, o: OrgId, exhausted: bool) -> bool {
    let mut ms = std::mem::take(&mut world.movement);
    if ms.orgs.len() <= o as usize {
        world.movement = ms;
        st.orgs[o as usize].pending_rout = Some(exhausted);
        return false;
    }
    let rules = ms.prepare(world);
    std::mem::swap(&mut world.combat, st);
    let r = regiment::fight::morale_reaction(&mut ms, world, rules, o, exhausted);
    std::mem::swap(&mut world.combat, st);
    world.movement = ms;
    r
}

/// Organization +0x74 as `0x57BFF0` would choose it.
pub(crate) fn required_formation(world: &World, o: OrgId) -> Option<usize> {
    regiment::fight::required_formation(world, o, &world.books.minds.arena)
}

/// The explore command `0x5249A0` on the regiment's Mind.
pub(crate) fn explore_command(world: &mut World, o: OrgId, at: Vec2, scalar: Option<f32>) -> bool {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    let rules = ms.prepare(world);
    let r = regiment::fight::explore_command(&mut ms, world, rules, o, at, scalar);
    flush(&mut ms, world);
    world.movement = ms;
    r
}

/// The guard command `0x51F360` on the regiment's Mind.
pub(crate) fn guard_command(world: &mut World, o: OrgId, target: Option<Unit>, at: Vec2, aux: Vec2) -> bool {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    let rules = ms.prepare(world);
    let target = target.map(|t| match t {
        Unit::Org(g) => crate::combat::TargetRef::Org(g),
        Unit::Actor(a) => crate::combat::TargetRef::Actor(a),
    });
    let r = regiment::fight::guard_command(&mut ms, world, rules, o, target, at, aux);
    flush(&mut ms, world);
    world.movement = ms;
    r
}

/// `0x57C400` for the Mind states held elsewhere (guard, a task): the state's
/// formation_required.
pub(crate) fn formation_follow(world: &mut World, o: OrgId) {
    let required = regiment::fight::required_formation(world, o, &world.books.minds.arena);
    if crate::player_orders::formation(world, o) != required {
        set_formation(world, o, |world| {
            world.player_orders.current_formation.insert(o, required);
        });
    }
}

/// The fighting states leave for a Mind state held elsewhere (a task), with the combat
/// state out of the world.
pub(crate) fn fight_stand_down(world: &mut World, st: &mut crate::combat::CombatState, o: OrgId) {
    if regiment::fight::top_state(world, o).is_none() {
        return;
    }
    let mut ms = std::mem::take(&mut world.movement);
    if ms.orgs.len() > o as usize {
        let rules = ms.prepare(world);
        std::mem::swap(&mut world.combat, st);
        regiment::fight::stand_down(&mut ms, world, rules, o);
        std::mem::swap(&mut world.combat, st);
    }
    world.movement = ms;
}

/// The tail of the Element Tick `0x570B90`: the detached teleport.
pub(crate) fn element_tick_tail(world: &mut World, a: ActorId) {
    let mut ms = std::mem::take(&mut world.movement);
    if ms.actors.len() > a as usize {
        let rules = ms.prepare(world);
        element::fight::detached_tick(&mut ms, world, rules, a);
    }
    world.movement = ms;
}

/// The regiment Mind's fighting state on top of its stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FightState {
    Kill,
    Chasing,
    Engage,
    Reform,
}
pub(crate) fn fight_state(world: &World, o: OrgId) -> Option<FightState> {
    Some(match regiment::fight::top_state(world, o)? {
        regiment::fight::KILL => FightState::Kill,
        regiment::fight::CHASING => FightState::Chasing,
        regiment::fight::ENGAGE => FightState::Engage,
        _ => FightState::Reform,
    })
}
/// `0x4CEDE0` of a lone actor: its Mover is its active component.
pub(crate) fn actor_mover_active(world: &World, a: ActorId) -> bool {
    world.movement.actors.get(a as usize).is_some_and(|m| m.mover.active)
}

/// A regiment's Move command at an actor: `0x520320` on its Mind, which then thinks it
/// (`0x5276B0`). A lone actor's move to an actor stays with [`order_think`].
fn regiment_move_to_actor(ms: &mut MovementState, world: &mut World, owner: Owner, target: ActorId, rules: &MoveRules) {
    match owner {
        Owner::Org(o) => {
            regiment::activate_to_actor(ms, world, *rules, o, target);
        }
        // `0x538340` with a thing: 0x532930, mode 2.
        Owner::Actor(a) => {
            let speed = ms.order(owner).group_speed.map(f32::to_bits);
            element::deliver_lone_move(ms, world, *rules, a, bits2(pos2(world, target)), Some(target), speed);
        }
    }
}

/// (orders) `0x51D840`: where the group holds each thing at `to`. A command to
/// several things has been split by `crate::orders::group::selection`.
fn group_offsets(world: &World, _occupied: &[u16], owners: &[Owner], to: Vec2) -> Vec<Vec2> {
    owners.iter().map(|&o| crate::orders::group::member_offset(world, owner_unit(o), to)).collect()
}
fn owner_unit(owner: Owner) -> Unit {
    match owner {
        Owner::Org(o) => Unit::Org(o),
        Owner::Actor(a) => Unit::Actor(a),
    }
}

/// (orders) Organization +D8: the speed a group measures a member's time at.
pub(crate) fn org_top_speed(world: &mut World, o: OrgId) -> f32 {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    let cap = speed_cap(&mut ms.speeds, world, Owner::Org(o));
    world.movement = ms;
    cap
}
/// (orders) Mind vt+64 = 0x5247B0: the regiment's Move state runs.
pub(crate) fn org_move_active(world: &World, o: OrgId) -> bool {
    world.movement.orgs.get(o as usize).is_some_and(|m| m.order.active)
}
/// (orders) 0x52CE70 on the Move state's path (Mind vt+44), from the organization.
pub(crate) fn org_path_left(world: &World, o: OrgId) -> f32 {
    let Some(m) = world.movement.orgs.get(o as usize) else { return 0.0 };
    let mut from = crate::orders::guard_position(world, Unit::Org(o)).unwrap_or(Vec2::ZERO);
    let mut left = 0.0f64;
    for w in &m.order.path {
        let (dx, dy) = (f64::from(w.pos.x) - f64::from(from.x), f64::from(w.pos.y) - f64::from(from.y));
        left += (dy * dy + dx * dx).sqrt();
        from = w.pos;
    }
    left as f32
}
/// (orders) Mind vt+88 = 0x5205E0: capped by Organization +D8, a value below 1e-4
/// takes the cap; the members on a Move hear it (their Mind vt+88).
pub(crate) fn org_set_speed(world: &mut World, o: OrgId, speed: f32) {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    let cap = speed_cap(&mut ms.speeds, world, Owner::Org(o));
    let mut v = speed;
    if cap < v {
        v = cap;
    }
    if v < EPSILON {
        v = cap;
    }
    ms.orgs[o as usize].order.speed = v;
    let rules = MoveRules::from_data(&world.data);
    let members: Vec<ActorId> = alive_members(world, o).collect();
    for m in members {
        if element::move_active(&ms, world, m) {
            element::set_speed(&mut ms, world, rules, m, v.to_bits());
        }
    }
    world.movement = ms;
}
/// (orders) Mind vt+48 = 0x524870: the Move state plans again, here at the place
/// the group holds it now (0x51D840).
pub(crate) fn org_replan(world: &mut World, o: OrgId) {
    let (destination, active) = match world.movement.orgs.get(o as usize) {
        Some(m) => (m.order.destination, m.order.active),
        None => return,
    };
    if !active {
        return;
    }
    let offset = crate::orders::group::member_offset(world, Unit::Org(o), destination);
    let dest = clamp_extent(world, destination + offset);
    let now = world.time;
    let order = &mut world.movement.orgs[o as usize].order;
    order.group_offset = offset;
    order.path_dest = dest;
    order.path.clear();
    order.job = None;
    order.next_think = now;
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
    let generation = o.generation;
    *o = OrderMove {
        active: true,
        target,
        attack_move,
        destination,
        group_offset: offset,
        path_dest,
        path: Vec::new(),
        route: None,
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
        capped_from: None,
        auxiliary: world.orders.next_aux.unwrap_or(Vec2::ZERO),
        formation_word: 0,
        generation,
    };
    // The regiment's Move state, for all but a kill order. Its Enter thinks in this
    // update's queue 4.
    if let Owner::Org(org) = owner
        && target.is_none()
        && deploy_facing.is_none()
    {
        let rules = ms.rules.unwrap_or_else(|| MoveRules::from_data(&world.data));
        regiment::activate_move(ms, world, rules, org);
    }
    // A lone actor's: the Move command on its own Mind (`0x4CA760`, `0x4D9B60`, `0x52C320`,
    // `0x538340`), whose Think plans (`0x5336B0`) through the route solver.
    if let Owner::Actor(a) = owner
        && target.is_none()
        && deploy_facing.is_none()
    {
        let rules = ms.rules.unwrap_or_else(|| MoveRules::from_data(&world.data));
        element::deliver_lone_move(ms, world, rules, a, bits2(path_dest), None, group_speed.map(f32::to_bits));
    }
}

/// Stop the owner: a regiment's Stop command `0x51D1C0` (its members go to their
/// slots from its Sleep), a lone actor's Mind back to Sleep.
fn stop(ms: &mut MovementState, world: &mut World, owner: Owner) {
    let o = ms.order(owner);
    let held_kill = o.target.is_some() && !o.follow && o.active;
    o.active = false;
    o.job = None;
    o.held = false;
    o.path.clear();
    match owner {
        Owner::Org(org) => {
            if held_kill {
                for m in alive_members(world, org).collect::<Vec<_>>() {
                    complete_element(ms, world, m);
                }
            }
            let rules = ms.rules.unwrap_or_else(|| MoveRules::from_data(&world.data));
            regiment::stop(ms, world, rules, org);
        }
        Owner::Actor(a) => complete_element(ms, world, a),
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
        if world.orgs[o].alive {
            regiment::ensure(&mut ms, world, o as OrgId);
        }
        let om = &ms.orgs[o].order;
        let hosted = om.target.is_none() || om.follow;
        if !hosted && om.active && om.job.is_none() && om.next_think <= now && world.orgs[o].alive {
            order_think(&mut ms, world, Owner::Org(o as OrgId), &rules);
        }
    }
    for a in 0..world.actors.len() {
        let am = &ms.actors[a].order;
        // A lone actor's kill order; its Move is on its Mind.
        let hosted = am.target.is_none() || am.follow;
        if !hosted && am.active && am.job.is_none() && am.next_think <= now {
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
    // The element minds think on update queue 4 (`mind_tick`); a dead
    // element's state ends here.
    for a in 0..world.actors.len() {
        if ms.actors[a].active && !alive(world, a as ActorId) {
            complete_element(&mut ms, world, a as ActorId);
        }
    }
    publish(&mut ms, world);
    ms.stats.queued = ms.jobs.len();
    flush(&mut ms, world);
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
    flush(&mut ms, world);
    let unpacks = std::mem::take(&mut ms.unpacks);
    world.movement = ms;
    for truck in unpacks {
        crate::deploy::unpack_now(world, truck);
    }
}

// ---------------------------------------------------------------------------
// Aircraft flight (the AAI move state and AircraftComponent). Entry points
// for the aircraft minds; the flight (MoveTick 0x5612F0, legs 0x55F230,
// formation follow 0x560A70 / 0x561080) is not ported yet, so these do not
// move anything.

/// `0x516740(position, direction)`: the AAI move state's goal, and a leg
/// appended to the flight path (AircraftComponent `0x560DB0` -> `0x5601E0`,
/// from the pose when the path is empty, else from the last leg's end).
/// A transport's flight is `crate::transport`; for any other aircraft not ported yet: no
/// effect.
pub fn fly_to(world: &mut World, aircraft: ActorId, point: glam::Vec3, direction: glam::Vec3) {
    if crate::transport::flying(world, aircraft) {
        crate::transport::fly_to(world, aircraft, point, direction);
    }
}
/// `0x516720`: the cruise height (flight +0x18 / +0x80). Not ported yet: no
/// effect.
pub fn set_cruise(_world: &mut World, _aircraft: ActorId, _height: f32) {}
/// `0x5166F0`: the flight speed (flight +0x14 / +0x84). Not ported yet: no
/// effect.
pub fn set_flight_speed(_world: &mut World, _aircraft: ActorId, _speed: f32) {}
/// `0x516B10`: the aircraft still follows its flight path. Not ported yet:
/// false.
pub fn following_path(_world: &World, _aircraft: ActorId) -> bool {
    false
}


/// The position commit `0x4EE5C0` of a thing the Mover Tick does not step (a fall): the
/// tile lists follow it.
pub(crate) fn thing_moved(world: &mut World, id: ActorId) {
    let p = world.actors[id as usize].pos;
    if let Some(s) = world.movement.space.as_mut() {
        s.commit(id, [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]);
    }
}

/// The regiment Mind's queue-4 Update `0x524910`.
pub fn org_mind_tick(world: &mut World, o: OrgId) {
    let mut ms = std::mem::take(&mut world.movement);
    let rules = ms.prepare(world);
    if ms.orgs.len() < world.orgs.len() || ms.actors.len() < world.actors.len() {
        ms.ensure(world);
    }
    regiment::update(&mut ms, world, o, rules);
    flush(&mut ms, world);
    let unpacks = std::mem::take(&mut ms.unpacks);
    world.movement = ms;
    for truck in unpacks {
        crate::deploy::unpack_now(world, truck);
    }
}

/// The Load of the map's saved Minds: an organization's and its members' with their
/// Movers and Characters. `things` are the kept things by `GameMap::placed` index.
/// What the save holds and cannot be taken is noted and the thing gets a fresh Mind
/// when it first thinks.
pub fn restore_saved_minds(world: &mut World, things: &std::collections::BTreeMap<usize, crate::scheduler::Kept>) {
    let map = world.map.clone();
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    for (&index, &kept) in things {
        let Some(placed) = map.placed.get(index) else { continue };
        let Some(bytes) = placed.mind.as_ref() else { continue };
        let result = match kept {
            crate::scheduler::Kept::Org(o) => aa_mind::saved_load::decode(aa_mind::construct::Kind::Regiment, bytes)
                .map_err(|e| e.to_string())
                .and_then(|(saved, _)| regiment::restore(&mut ms, world, o, &saved, placed)),
            crate::scheduler::Kept::Actor(a) if crate::combat::air::is_aircraft(world, a) => Err("the Load of an aircraft Mind is not hosted".to_string()),
            crate::scheduler::Kept::Actor(a) => aa_mind::saved_load::decode(aa_mind::construct::Kind::Element, bytes)
                .map_err(|e| e.to_string())
                .and_then(|(saved, _)| element::restore(&mut ms, world, a, &saved, placed)),
        };
        if let Err(e) = result {
            ms.note(format!("placed thing {index} {}: saved Mind: {e}", placed.thing_id));
        }
    }
    world.movement = ms;
}

/// A saved word naming a thing by its GTID, as a hosted Mind names it: this world's actor
/// or `regiment::ORG | o`, plus one; 0 for none.
fn saved_thing(world: &World, gtid: u32) -> u32 {
    match world.ids.thing(gtid) {
        Some(crate::ids::Thing::Actor(a)) => a + 1,
        Some(crate::ids::Thing::Org(o)) => (crate::minds::ORGANIZATION_MIND | o) + 1,
        _ => 0,
    }
}

/// The targets of a restored Mind's pending and current commands, as [`saved_thing`].
fn saved_command_targets(world: &mut World, mind: u32) {
    let ids: Vec<u32> = world.books.minds.arena.minds.get(&crate::minds::mind_id(mind)).map_or_else(Vec::new, |m| m.pending_commands.iter().copied().chain(m.current_command).collect());
    for id in ids {
        let Some(c) = world.books.minds.commands.get(&id).filter(|c| c.flags & 1 != 0).cloned() else { continue };
        let thing = saved_thing(world, c.target);
        if let Some(c) = world.books.minds.commands.get_mut(&id) {
            c.target = thing;
        }
    }
}

/// The order a restored Mind stands on, as the layers above the Mind read it: nothing
/// is commanded, the Move state is the save's.
pub(crate) fn restored_order(world: &mut World, unit: Unit, order: Order) {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    let owner = match unit {
        Unit::Org(o) => Owner::Org(o),
        Unit::Actor(a) => Owner::Actor(a),
    };
    set_order(world, &mut ms, owner, order);
    world.movement = ms;
}

/// `0x57CBA0`'s `0x524B80` for a member that joined `o` (after `walk_member` when it
/// walks out of a structure).
pub fn member_added(world: &mut World, o: OrgId, member: ActorId) {
    let mut ms = std::mem::take(&mut world.movement);
    let rules = ms.prepare(world);
    ms.ensure(world);
    member_cap_steps(&mut ms, world, o, member);
    regiment::member_added(&mut ms, world, rules, o, member);
    flush(&mut ms, world);
    world.movement = ms;
}

/// The organization's +0xD8 as a member joins (`0x57D8A0` -> `0x57A640`): the least of
/// its Movers' maxima, first with the new Mover as its definition has it, then with
/// the formation's property on it (`0x57BF50`, `0x576C70` -> `0x57B250` -> `0x57A640`).
/// Each change goes through the regiment's virtual +0x118 to every member Mind.
fn member_cap_steps(ms: &mut MovementState, world: &World, o: OrgId, member: ActorId) {
    let mut cap = f32::INFINITY;
    for m in alive_members(world, o).collect::<Vec<_>>() {
        if m != member {
            cap = cap.min(max_speed(&mut ms.speeds, world, m));
        }
    }
    if !cap.is_finite() {
        return;
    }
    let bare = max_speed_with(&mut ms.speeds, world, member, false);
    let formed = max_speed(&mut ms.speeds, world, member);
    let first = cap.min(bare);
    if (first - cap).abs() >= EPSILON {
        org_cap_changed(ms, world, o, cap, first);
    }
    let second = cap.min(formed);
    if (second - first).abs() >= EPSILON {
        org_cap_changed(ms, world, o, first, second);
    }
}

/// Regiment virtual +0x118 (`0x51D290` -> CAIStateMove `0x520530`): its Move's speed
/// scales, and every member Mind hears the change (+0x118).
fn org_cap_changed(ms: &mut MovementState, world: &World, o: OrgId, old: f32, new: f32) {
    if !ms.orgs[o as usize].order.active || old == 0.0 {
        return;
    }
    let om = &mut ms.orgs[o as usize];
    om.order.speed = (f64::from(om.order.speed) / f64::from(old) * f64::from(new)) as f32;
    for m in alive_members(world, o).collect::<Vec<_>>() {
        let max = max_speed(&mut ms.speeds, world, m);
        max_speed_changed(ms, m, old, new, max, false);
    }
}

/// `0x5709D0` found a member attached that was not (`0x578ED0` -> `0x522840`).
pub fn member_attached(world: &mut World, o: OrgId, member: ActorId) {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    regiment::list_member(&mut ms, o, member);
    world.movement = ms;
}

/// The element Mind's queue-4 delivery (`0x531D30`, the Mind's +8
/// Updatable, newest-scheduled first): its current state thinks (movement's
/// element move state here), then a live Mind with no pending think re-thinks
/// in 2.0 s, unless its active Mover is registered (that wakes it).
pub fn mind_tick(world: &mut World, id: ActorId) {
    let mut ms = std::mem::take(&mut world.movement);
    let rules = ms.prepare(world);
    if ms.actors.len() < world.actors.len() {
        ms.ensure(world);
    }
    if (id as usize) < ms.actors.len() && alive(world, id) {
        element::update(&mut ms, world, id, rules);
    }
    flush(&mut ms, world);
    publish_state(&ms, world, id);
    let unpacks = std::mem::take(&mut ms.unpacks);
    world.movement = ms;
    for truck in unpacks {
        crate::deploy::unpack_now(world, truck);
    }
}

/// Orders set outside `command` (the core's `Order::Deploy` for trucks,
/// or a reset to `Idle`) start or stop movement.
fn sync_orders(ms: &mut MovementState, world: &mut World) {
    for o in 0..world.orgs.len() {
        if !world.orgs[o].alive {
            continue;
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
            // (combat, X1) An attack written as an order (a guard's Think): the Mind's Kill.
            if let Owner::Org(o) = owner
                && alive(world, *target)
            {
                let rules = ms.rules.unwrap_or_else(|| MoveRules::from_data(&world.data));
                if !regiment::fight::kill_command(ms, world, rules, o, *target) {
                    stop(ms, world, owner);
                    world.orgs[o as usize].order = Order::Idle;
                    ms.orgs[o as usize].seen = Some(Order::Idle);
                    return;
                }
            } else if alive(world, *target) {
                let to = pos2(world, *target);
                start(ms, world, owner, to, Vec2::ZERO, Some(*target), false, None, None);
            }
        }
        Order::Deploy { at, facing } => {
            let truck = match owner {
                Owner::Actor(a) => Some(a),
                Owner::Org(o) => pick_leader(world, o),
            };
            // The regiment's KKC_CAIStateUnpack 0x526E60 runs its embedded Move
            // first: the order's point, the path, the leader (below), then it
            // delivers the unpack action (`element::completed`). A refused spot
            // is native only when the building is made (`deploy::unpack_now`).
            let rules = ms.rules.unwrap_or_else(|| MoveRules::from_data(&world.data));
            match (owner, truck) {
                (Owner::Org(o), t) => {
                    ms.orgs[o as usize].leader = t;
                    start(ms, world, owner, *at, Vec2::ZERO, None, false, None, Some(*facing));
                    ms.orgs[o as usize].order.active = false;
                    regiment::activate_unpack(ms, world, rules, o, *at, *facing);
                    ms.orgs[o as usize].seen = Some(order);
                }
                (Owner::Actor(a), _) => {
                    element::begin_unpack(ms, world, rules, a, *at);
                    ms.actors[a as usize].seen = Some(order);
                }
            }
            return;
        }
    }
    match owner {
        // (combat) What the state that left put back (`reflect_order`) yields to the order.
        Owner::Org(o) => set_order(world, ms, Owner::Org(o), order),
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
fn stop_members(ms: &mut MovementState, world: &mut World, owner: Owner) {
    match owner {
        Owner::Org(o) => {
            for m in alive_members(world, o).collect::<Vec<_>>() {
                complete_element(ms, world, m);
            }
        }
        Owner::Actor(a) => complete_element(ms, world, a),
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
                complete_element(ms, world, m);
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
            complete_element(ms, world, a);
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
        // (combat, X1) A regiment's kill state walks on (0x520730 -> the Move state),
        // whatever its weapons' range, until the target is in its ZOC and the engage
        // state takes over (0x57AB70 -> 0x524A00).
        let hold = match owner {
            Owner::Org(o) => crate::combat::org_state(world, o).is_some_and(|c| c.engaging()),
            Owner::Actor(_) => (own - tp).length() <= hold_range(world, owner),
        };
        if hold {
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
    let leader_at = pos2(world, leader);
    let o = ms.order(owner);
    // A cut route (no region pre-pass here) that got nowhere since the last one is
    // not a hop: it counts as a failed try (0x526940).
    if o.capped && !leader_moving {
        if o.capped_from.is_some_and(|p| (p - leader_at).length_squared() < ARRIVAL_RADIUS_SQUARED) {
            o.capped = false;
            o.path.clear();
        }
        o.capped_from = Some(leader_at);
    }
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
fn send_member_path(ms: &mut MovementState, world: &mut World, org: OrgId, member: ActorId) {
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
    let standing: Vec<_> = members.iter().map(|m| m.map(|m| (m.actor, m.pos, element::element_attached(world, m.actor)))).collect();
    let om = &mut ms.orgs[org as usize];
    om.facing = facing;
    let previous = std::mem::replace(&mut om.slots, slots);
    world.orgs[org as usize].heading = facing_angle(facing);
    if let Some(table) = crate::creation::organization_scatter(world, org, facing, &standing, leader, &previous) {
        ms.orgs[org as usize].slots = table;
    }
}

/// `0x532170`: give an element a path in mode 0 or 3.
fn start_element(ms: &mut MovementState, world: &mut World, id: ActorId, nodes: Vec<Node>, mode: u8, speed: f32) {
    if !alive(world, id) {
        return;
    }
    let rules = ms.rules.unwrap_or_else(|| MoveRules::from_data(&world.data));
    ms.actors[id as usize].pursue = None;
    ms.actors[id as usize].task = None;
    element::give_path(ms, world, rules, id, nodes, mode, speed);
}
/// Element move complete: stop the Mover where it is (`0x52D660`; the Mover
/// keeps its speed value, `0x5767C0(0, 0)`).
fn complete_element(ms: &mut MovementState, world: &mut World, id: ActorId) {
    let rules = ms.rules.unwrap_or_else(|| MoveRules::from_data(&world.data));
    element::clear(ms, world, rules, id);
    let am = &mut ms.actors[id as usize];
    am.pursue = None;
    am.task = None;
}

// ---------------------------------------------------------------------------
// Element (member / single actor) mind.









// ---------------------------------------------------------------------------
// Mover.

#[derive(Clone, Copy, Debug)]
enum Blocker {
    /// The words of `0x553BA0`'s result.
    Hit([u32; 4]),
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
        complete_element(ms, world, id);
        return;
    }
    // (combat) 0x576AD0's target: within its range the Mover stops, the Mind thinks.
    if let Some((t, range)) = ms.actors[a].chase
        && (!alive(world, t) || crate::combat::in_range(world, id, t, range, false))
    {
        ms.actors[a].chase = None;
        ms.halt(id);
        think_mind_in(world, id, 0.0);
        return;
    }
    run_mover_tick(ms, world, id, rules);
}

/// The Mover fields of [`Mover`] in the port's layout (`0x576920`). The
/// maximum-speed derived rate +0x34 is computed as `0x576850` stores it.
fn mover_state(am: &ActorMove, def: &Def, max: f32) -> aa_movement::MoverState {
    let m = &am.mover;
    aa_movement::MoverState {
        destination: [m.dest.x, m.dest.y, 0.0],
        last_step_speed: m.last_step,
        speed: m.speed,
        requested_speed: m.requested,
        acceleration: def.acceleration,
        turn_rate: turn_rate_native(def, max),
        desired_direction: [m.desired.x, m.desired.y, 0.0],
        target_id: 0,
        target_range: None,
        ignore_terrain_speed_limit: false,
        speed_factor: m.factor,
        instant_turn: m.instant,
        turning: m.turning,
        falling: false,
    }
}

/// Mover Tick `0x577E50` on `aa_movement::mover::tick` and the ground step
/// `0x5774A0` on `aa_movement::ground_step_original`, with [`MoverHost`] as
/// the world. PC53, the precision-exception flag pending (the simulation
/// thread's x87 state: research/phase8-tick/fpu-precision.md).
fn run_mover_tick(ms: &mut MovementState, world: &mut World, id: ActorId, rules: &MoveRules) {
    let a = id as usize;
    let max = max_speed(&mut ms.speeds, world, id);
    let terrain = terrain_cap(ms, world, id);
    let def = def_of(world, id);
    let mut state = mover_state(&ms.actors[a], def, max);
    let f = ms.actors[a].mover.facing;
    let p = world.actors[a].pos;
    let mut motion = aa_movement::ActorMotion {
        position: [p.x, p.y, p.z],
        facing: [f.x, f.y, 0.0],
        // Thing +8: bit 18, ethereal while the Deploy is current, takes no collision.
        flags: space::thing_flags(ms, world, id) & (1 << 18),
        has_mind: true,
        has_character: false,
        character_stopped: false,
        terrain_speed_limit: terrain,
    };
    let gate = aa_movement::mover::TickGate { mover_is_active_component: ms.actors[a].mover.active, properties_flags_1c: None };
    let delta = f32::from_bits(world.clock.step.0);
    let mut host = MoverHost { ms, world, id, rules, delta };
    let result = aa_movement::mover::tick(&mut state, &mut motion, gate, delta, rules.reached_threshold, PC53, &mut host);
    let m = &mut ms.actors[a].mover;
    m.last_step = state.last_step_speed;
    m.speed = state.speed;
    m.requested = state.requested_speed;
    m.factor = state.speed_factor;
    m.turning = state.turning;
    m.instant = state.instant_turn;
    if let Err(e) = result {
        // A refused arithmetic domain stops this Mover: no retry (the port's
        // contract), and the error stays for the diagnostics.
        ms.note(format!("mover {id}: {e}"));
        let am = ms.halt(id);
        am.mover.turning = false;
    }
}

/// `aa_movement::GroundWorld` and `TickWorld` over [`MovementState`]: the
/// collision seam is [`collision`] until M5 of PARITY-CENSUS.md, the position
/// commit is the actor's `pos` plus the element notification `0x4D0F44`.
struct MoverHost<'a> {
    ms: &'a mut MovementState,
    world: &'a mut World,
    id: ActorId,
    rules: &'a MoveRules,
    delta: f32,
}
impl aa_movement::GroundWorld for MoverHost<'_> {
    type Collision = Blocker;
    fn activate(&mut self, _: &mut aa_movement::MoverState, _: &mut aa_movement::ActorMotion) -> aa_movement::Result<bool> {
        Ok(true)
    }
    fn target_position(&mut self, _: u32) -> aa_movement::Result<Option<[f32; 2]>> {
        // Mover +0x44 is never set by the ground orders hosted here.
        Ok(None)
    }
    fn shapes_in_range(&mut self, _: &aa_movement::ActorMotion, _: u32, _: f32) -> aa_movement::Result<bool> {
        Ok(false)
    }
    fn collision_at(&mut self, actor: &aa_movement::ActorMotion, xy: [f32; 2]) -> aa_movement::Result<Option<Blocker>> {
        Ok(element::ground_hit(self.ms, self.world, self.id, [xy[0].to_bits(), xy[1].to_bits()]).map(Blocker::Hit))
    }
    fn set_facing(&mut self, _: &mut aa_movement::MoverState, actor: &mut aa_movement::ActorMotion, xy: [f32; 2]) -> aa_movement::Result<()> {
        actor.facing = [xy[0], xy[1], 0.0];
        self.ms.actors[self.id as usize].mover.facing = Vec2::from(xy);
        write_facing(self.ms, self.world, self.id);
        Ok(())
    }
    fn move_on_ground(&mut self, _: &mut aa_movement::MoverState, actor: &mut aa_movement::ActorMotion, xy: [f32; 2]) -> aa_movement::Result<()> {
        let a = self.id as usize;
        let p = Vec2::from(xy);
        let from = self.world.actors[a].pos.truncate();
        // 0x4EEB40: the height query 0x4F0E50 reads the actor's position before
        // the commit, so Z lags one step (`aa_spatial::move_ground`).
        let z = self.world.map.height_at(from);
        self.ms.actors[a].step_speed = (p - from).length() / self.delta;
        self.world.actors[a].pos = glam::Vec3::new(p.x, p.y, z);
        actor.position = [p.x, p.y, z];
        if let Some(s) = self.ms.space.as_mut() {
            s.commit(self.id, [p.x.to_bits(), p.y.to_bits(), z.to_bits()]);
        }
        // The position commit's element notification (0x4D0F44 -> 0x5705E0):
        // the organization's goal follows its leader.
        if let Some(o) = self.world.actors[a].org.filter(|&o| self.world.orgs.get(o as usize).is_some_and(|x| x.alive)) {
            org_moved(self.ms, self.world, o, self.id, self.rules);
        }
        Ok(())
    }
    fn notify(&mut self, event: aa_movement::Event, mover: &mut aa_movement::MoverState, actor: &mut aa_movement::ActorMotion) -> aa_movement::Result<bool> {
        use aa_movement::Event as E;
        match event {
            // 0x50B250 inside the Mover's own delivery.
            E::CancelScheduled => {
                if self.world.is_scheduled(Sched::Mover(self.id)) {
                    self.world.unschedule(Sched::Mover(self.id));
                }
            }
            E::DestinationReached => self.call_mind(element::MoverCall::Reached, mover, actor),
            E::TurnCompleted => self.call_mind(element::MoverCall::TurnCompleted, mover, actor),
            E::DeactivateMover => self.call_mind(element::MoverCall::Deactivate, mover, actor),
            E::ScheduleImmediate | E::TargetInRange | E::SpeedChanged | E::CharacterStop | E::CharacterResume => {}
        }
        Ok(true)
    }
    fn blocked(&mut self, collision: &Blocker, facing: [f32; 2], mover: &mut aa_movement::MoverState, actor: &mut aa_movement::ActorMotion) -> aa_movement::Result<()> {
        self.call_mind(element::MoverCall::Blocked(collision, facing), mover, actor);
        Ok(())
    }
}
impl MoverHost<'_> {
    /// A direct Mind virtual from the Mover's delivery: the Mover's fields are
    /// published before it and read back after it.
    fn call_mind(&mut self, call: element::MoverCall<'_>, mover: &mut aa_movement::MoverState, actor: &mut aa_movement::ActorMotion) {
        let a = self.id as usize;
        {
            let m = &mut self.ms.actors[a].mover;
            m.last_step = mover.last_step_speed;
            m.speed = mover.speed;
            m.requested = mover.requested_speed;
            m.factor = mover.speed_factor;
            m.turning = mover.turning;
            m.instant = mover.instant_turn;
        }
        element::mover_call(self.ms, self.world, *self.rules, self.id, call);
        let m = &self.ms.actors[a].mover;
        mover.last_step_speed = m.last_step;
        mover.speed = m.speed;
        mover.requested_speed = m.requested;
        mover.speed_factor = m.factor;
        mover.turning = m.turning;
        mover.instant_turn = m.instant;
        mover.destination = [m.dest.x, m.dest.y, 0.0];
        mover.desired_direction = [m.desired.x, m.desired.y, 0.0];
        actor.facing = [m.facing.x, m.facing.y, 0.0];
    }
}
impl aa_movement::mover::TickWorld for MoverHost<'_> {
    fn fall(&mut self, _: &mut aa_movement::MoverState, _: &mut aa_movement::ActorMotion, _: f32) -> aa_movement::Result<()> {
        Err(aa_movement::Error::Falling)
    }
    /// `0x4D67F0`.
    fn refresh_overlaps(&mut self, _: &mut aa_movement::MoverState, _: &mut aa_movement::ActorMotion) -> aa_movement::Result<()> {
        if let Some(mut s) = self.ms.space.take() {
            space::refresh_overlaps(&mut s, self.ms, self.world, self.id);
            self.ms.space = Some(s);
        }
        Ok(())
    }
    fn aircraft_step(&mut self, _: &mut aa_movement::MoverState, _: &mut aa_movement::ActorMotion, _: f32) -> aa_movement::Result<()> {
        Ok(())
    }
    fn ground_step(&mut self, mover: &mut aa_movement::MoverState, actor: &mut aa_movement::ActorMotion, delta: f32) -> aa_movement::Result<aa_movement::StepOutcome> {
        let context = aa_movement::StepContext { acceleration_delta: delta, world_delta: self.delta, terrain_minimum_move_speed: self.rules.terrain_minimum_speed, precision: PC53 };
        aa_movement::ground_step_original(mover, actor, context, true, self)
    }
    fn movement_failed(&mut self, mover: &mut aa_movement::MoverState, actor: &mut aa_movement::ActorMotion) -> aa_movement::Result<()> {
        self.call_mind(element::MoverCall::Failed, mover, actor);
        Ok(())
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
/// tile +0x44/+0x48): a member attached to a living organization (Element +0x1C)
/// takes the organization thing's terrain (`0x570560`: the tile under the
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
        (Some(_), Some(o)) if element::element_attached(world, id) => tile_terrain(world, world.orgs[o as usize].anchor).1,
        _ => own,
    };
    ms.actors[id as usize].terrain = Some((cell, cap));
    cap
}

fn write_facing(ms: &mut MovementState, world: &mut World, id: ActorId) {
    let angle = facing_angle(ms.actors[id as usize].mover.facing);
    world.actors[id as usize].facing = angle;
    ms.actors[id as usize].mover.written_angle = angle;
    let f = ms.actors[id as usize].mover.facing;
    if let Some(s) = ms.space.as_mut() {
        s.set_facing(id, [f.x.to_bits(), f.y.to_bits()]);
    }
}



/// The tile's own flags as `0x54F370` reads them for this mover, without bit 10:
/// a tile that holds objects blocks only through their shapes (`0x553BA0`). Outside
/// the map counts as blocked.
fn terrain_blocked(world: &World, class: MoverClass, p: Vec2) -> bool {
    let mask = class.tile_mask() & !crate::path::OCCUPIED;
    match world.map.cell_of(p) {
        Some((x, y)) => world.map.flags.get((y * world.map.width + x) as usize).is_none_or(|&f| f & mask != 0),
        None => true,
    }
}
/// The mover's circle (radius `radius`) at `p` meets `a`'s placed blocking shape.
fn shape_meets(world: &World, a: &crate::world::Actor, p: Vec2, radius: f32) -> bool {
    let d = world.data.def(a.def);
    let Some(size) = d.blocking_size else { return false };
    let delta = p - a.pos.truncate();
    if d.blocking_shape.as_deref().is_some_and(|s| s.eq_ignore_ascii_case("CIRCLE")) {
        let reach = radius + size[0];
        return delta.length_squared() < reach * reach;
    }
    let f = facing_vector(a.facing);
    let across = (delta.x * f.y - delta.y * f.x).abs();
    let along = (delta.x * f.x + delta.y * f.y).abs();
    let (dx, dy) = ((across - size[0] * 0.5).max(0.0), (along - size[1] * 0.5).max(0.0));
    dx * dx + dy * dy < radius * radius
}
/// The structures whose tile rectangle lies under the circle at `p`; none without
/// looking at the list when no tile there holds a structure.
fn structures_near<'m>(ms: &'m MovementState, world: &World, p: Vec2, radius: f32) -> impl Iterator<Item = ActorId> + 'm {
    let cell = world.map.cell_size.max(f32::MIN_POSITIVE);
    let (width, height) = (world.map.width as i32, world.map.height as i32);
    let lo = [((p.x - radius).max(0.0) / cell) as i32, ((p.y - radius).max(0.0) / cell) as i32];
    let hi = [(((p.x + radius).max(0.0) / cell) as i32).min(width - 1), (((p.y + radius).max(0.0) / cell) as i32).min(height - 1)];
    let mut any = false;
    for y in lo[1]..=hi[1] {
        for x in lo[0]..=hi[0] {
            any |= ms.occupancy.cells.get((y * width + x) as usize).is_some_and(|&c| c > 0);
        }
    }
    let list: &[(ActorId, [i32; 4])] = if any { &ms.occupancy.registered } else { &[] };
    list.iter().filter(move |(_, r)| lo[0] <= r[2] && hi[0] >= r[0] && lo[1] <= r[3] && hi[1] >= r[1]).map(|&(b, _)| b)
}
/// The circle at `p` meets a structure other than `id`.
fn structure_overlap(ms: &MovementState, world: &World, id: ActorId, p: Vec2, radius: f32) -> bool {
    structures_near(ms, world, p, radius).any(|b| b != id && shape_meets(world, &world.actors[b as usize], p, radius))
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
    // 0x553BA0 (rendition, M5): the tiles' terrain flags, the structures' placed
    // shapes (a tile holding a structure blocks only where the shapes meet).
    let class = mover_class(def);
    let cell = world.map.cell_size;
    let lo = (p - Vec2::splat(r)).max(Vec2::ZERO) / cell;
    let hi = (p + Vec2::splat((r - EPSILON).max(0.0))).max(Vec2::ZERO) / cell;
    for y in (lo.y as i32)..=(hi.y as i32) {
        for x in (lo.x as i32)..=(hi.x as i32) {
            if terrain_blocked(world, class, Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * cell) {
                return false;
            }
        }
    }
    if structure_overlap(ms, world, mover, p, r) {
        return false;
    }
    !movers || ms.hash.for_each_near(p, r + ms.hash.max_radius, |o, q, rq| o != mover && (q - p).length_squared() < (r + rq) * (r + rq)).is_none()
}

/// FindOpenPositionLineThenRadius `0x554BF0` for `mover`, from its own position, along its
/// facing ([`space::open_line_then_radius`]). A map whose tile grid has no lists keeps the point.
fn open_position(ms: &mut MovementState, world: &World, mover: ActorId, point: Vec2, radius: f32) -> Vec2 {
    let def = world.actors[mover as usize].def;
    let (facing, from) = (ms.actors[mover as usize].mover.facing, pos2(world, mover));
    let Some(mut s) = ms.space.take() else { return point };
    let at = {
        let mut policy = space::Policy { ms: &*ms, world };
        space::open_line_then_radius(&mut s, &mut policy, def, bits2(point), bits2(facing), bits2(from), radius.to_bits())
    };
    ms.space = Some(s);
    Vec2::new(f32::from_bits(at[0]), f32::from_bits(at[1]))
}
/// `0x554BF0` for a definition that is no actor yet (a recruit's organization, `0x582E80`;
/// a dropped regiment). None: the map's tile grid has no lists.
pub(crate) fn open_position_for(world: &mut World, def: crate::data::DefId, point: Vec2, from: Vec2, facing: Vec2, radius: f32) -> Option<Vec2> {
    let mut ms = std::mem::take(&mut world.movement);
    if ms.space.is_none() {
        ms.prepare(world);
    }
    let mut at = None;
    if let Some(mut s) = ms.space.take() {
        s.add_new(&ms, world);
        let mut policy = space::Policy { ms: &ms, world };
        let found = space::open_line_then_radius(&mut s, &mut policy, def, bits2(point), bits2(facing), bits2(from), radius.to_bits());
        at = Some(Vec2::new(f32::from_bits(found[0]), f32::from_bits(found[1])));
        ms.space = Some(s);
    }
    world.movement = ms;
    at
}
fn bits2(v: Vec2) -> [u32; 2] {
    [v.x.to_bits(), v.y.to_bits()]
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
        if ms.space.is_none() {
            ms.prepare(world);
        }
        let at = open_position(&mut ms, world, member, to, DEPLOY_SEARCH_RADIUS);
        begin_walk(&mut ms, world, member, at, 0, &rules);
        // An actor without an organization keeps the activation's schedule 0 and
        // thinks in this update's queue-4 pass (the Berlin film: a truck created in
        // 1002 starts in 1002); a member's join delivers a command the Deploy defers
        // (`member_added`), whose 1e-4 puts the first think in the next update (trucks
        // created in 402 start in 403).
    }
    flush(&mut ms, world);
    world.movement = ms;
}
/// Enter `0x52F560`: the move to `at` (mode 0, own top speed, no facing).
fn begin_walk(ms: &mut MovementState, world: &mut World, member: ActorId, at: Vec2, tries: u8, rules: &MoveRules) {
    element::begin_deploy(ms, world, *rules, member, at, tries);
}

/// A member's own approach to its organization's task (the element states
/// of `combat::tasks`: UAI repair `0x5359D0` status 2 follows its object,
/// UAI bombard `0x5353E0` relation 1): an ordinary move (mode 0) at its own
/// top speed to `combat::tasks::approach_goal`, renewed with the `0x5276B0`
/// ratio when that goal moves, stopped once the goal is gone (in range).
fn approach_task(ms: &mut MovementState, world: &mut World, m: ActorId, rules: &MoveRules) {
    let Some((point, _range)) = crate::combat::tasks::approach_goal(world, m) else {
        if ms.actors[m as usize].task.take().is_some() && ms.actors[m as usize].active {
            complete_element(ms, world, m);
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
    // The Mind's Sleep sends the members to their slots (`0x522F60`); what stays
    // here is the approach of `crate::combat::tasks`.
    ms.orgs[org as usize].next_reform = now + THINK_SECONDS;
    for m in alive_members(world, org).collect::<Vec<_>>() {
        if crate::combat::tasks::busy(world, m) || ms.actors[m as usize].task.is_some() {
            approach_task(ms, world, m, rules);
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
            world.schedule_periodic_at(crate::trace::sites::ORGANIZATION_GOAL, crate::scheduler::EVERY_TICK, owner, GOAL_DATA);
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
    // A building leader (the unpacked HQ) does not collide with its own shape:
    // its footprint is left out of the tile test (`0x553BA0` excludes the source).
    let blocked = |ms: &MovementState, world: &World, at: Vec2| {
        leader.is_some_and(|l| {
            if is_building(def_of(world, l)) {
                terrain_blocked(world, mover_class(def_of(world, l)), at)
            } else {
                !spot_free(ms, world, l, at, true)
            }
        })
    };
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
    flush(&mut ms, world);
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
    flush(&mut ms, world);
    world.movement = ms;
}

/// Element +1C of a member.
pub(crate) fn element_is_attached(world: &World, id: ActorId) -> bool {
    element::element_attached(world, id)
}

/// `0x57BFF0` chose another formation (`change` writes it): the members' Mover
/// maximum refresh `0x576C70` runs. A lower maximum caps the Mover's request;
/// an Element Mind with its Move current scales the state speed by new over old
/// (Mind +0x118 `0x52DBE0` -> `0x52D620`, the Mover's request is that times its
/// factor, clamped by `0x576D20`). Then `0x57B250` -> `0x57A640`: Organization
/// +0xD8 changed, so the regiment's virtual +0x118 (`0x51D290` -> CAIStateMove
/// `0x520530`) scales its move's speed and offers the change to every member
/// Mind. Last, the slot table `0x57BEF0` for the organization's facing.
pub fn set_formation(world: &mut World, o: OrgId, change: impl FnOnce(&mut World)) {
    let mut ms = std::mem::take(&mut world.movement);
    ms.ensure(world);
    set_formation_in(&mut ms, world, o, change);
    flush(&mut ms, world);
    world.movement = ms;
}

/// [`set_formation`] with the movement state out of the world.
fn set_formation_in(ms: &mut MovementState, world: &mut World, o: OrgId, change: impl FnOnce(&mut World)) {
    let members: Vec<ActorId> = alive_members(world, o).collect();
    let old: Vec<f32> = members.iter().map(|&m| max_speed(&mut ms.speeds, world, m)).collect();
    let old_cap = speed_cap(&mut ms.speeds, world, Owner::Org(o));
    change(world);
    for (&m, &before) in members.iter().zip(&old) {
        let new = max_speed(&mut ms.speeds, world, m);
        if (before - new).abs() >= EPSILON {
            max_speed_changed(ms, m, before, new, new, true);
        }
    }
    let cap = speed_cap(&mut ms.speeds, world, Owner::Org(o));
    if (cap - old_cap).abs() >= EPSILON {
        org_cap_changed(ms, world, o, old_cap, cap);
    }
    if !ms.orgs[o as usize].slots.is_empty() {
        let facing = ms.orgs[o as usize].facing;
        set_org_facing(ms, world, o, facing);
    }
}

/// The Element Mind's virtual +0x118 (`0x52D620` while its Move is current),
/// after the Mover's own `0x576C70` clamp when `own` is set. `max` is the
/// Mover's maximum after the change.
fn max_speed_changed(ms: &mut MovementState, id: ActorId, old: f32, new: f32, max: f32, own: bool) {
    let am = &mut ms.actors[id as usize];
    if own && new < am.mover.requested {
        am.mover.requested = new;
    }
    if am.active && old != 0.0 {
        am.requested = (f64::from(am.requested) / f64::from(old) * f64::from(new)) as f32;
        if let Some(p) = am.payload.as_mut() {
            p.plan.movement.requested_speed_bits = am.requested.to_bits();
        }
        // 0x576D20: the Mover's request, kept to its maximum.
        let factor = am.mover.factor;
        let want = (f64::from(factor) * f64::from(am.requested)) as f32;
        am.mover.requested = want.min(max);
    }
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
    flush(&mut ms, world);
    world.movement = ms;
}

/// Copy the movement results into the shared `Actor` fields.
fn publish(ms: &mut MovementState, world: &mut World) {
    for a in 0..world.actors.len() {
        // This update's step (queue 0, `mover_tick`); none until the next.
        let step_speed = std::mem::take(&mut ms.actors[a].step_speed);
        let am = &ms.actors[a];
        let actor = &mut world.actors[a];
        if !actor.alive || actor.removed {
            continue;
        }
        // The Mover's speed (+0x20: zeroed by an arrival, kept by other
        // stops) and this update's displacement.
        actor.motion.speed = am.mover.speed;
        actor.motion.velocity = am.mover.facing * step_speed;
        publish_state(ms, world, a as ActorId);
    }
}
/// The element's path, goal and activity (also after its queue-4 think).
fn publish_state(ms: &MovementState, world: &mut World, id: ActorId) {
    let time = world.time;
    let am = &ms.actors[id as usize];
    let actor = &mut world.actors[id as usize];
    if !actor.alive || actor.removed {
        return;
    }
    let moving = am.mover.active;
    if am.active || !actor.motion.path.is_empty() {
        actor.motion.path.clear();
        if am.active {
            actor.motion.path.extend(am.nodes.iter().map(|n| n.pos));
        }
    }
    actor.motion.goal = if am.active { am.nodes.last().map(|n| n.pos) } else { None };
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

// ---------------------------------------------------------------------------
// Queries for other systems.

/// True while the actor's Mover is driving it.
pub fn actor_moving(world: &World, id: ActorId) -> bool {
    world.movement.actors.get(id as usize).is_some_and(|a| a.mover.active)
}
/// `0x4CEDE0` on an organization: its Mind's state is active (Mind +0x74 == 2) with
/// a moving leader (`0x5780F0`), or it stands off its goal (`0x578E10`).
pub fn org_moving(world: &World, o: OrgId) -> bool {
    let ms = &world.movement;
    let Some(om) = ms.orgs.get(o as usize) else { return false };
    // 0x4CEDE0's first half: the regiment's Move with a moving leader. Its second
    // half, the thing away from its goal (`!0x578E10`), would disturb a doodad at
    // tick 599 of the Berlin film, where the banner still walks to the unpacked HQ
    // and the original draws nothing: left out on that evidence, unexplained.
    org_active(ms, world, o) && om.leader.is_some_and(|l| actor_moving(world, l))
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

/// The hosted element Mind is in use (see the element module's switch).
/// `0x552EE0` -> `0x558040` mode 0 -> `0x558580` (`aa_spatial::rectangle_actors`): the
/// actors listed in the tile rectangle of a circle, in the original's order. There is
/// no test against the circle itself.
pub(crate) fn actors_in_tile_rectangle(world: &mut World, at: Vec2, radius: f32) -> Vec<ActorId> {
    let mut ms = std::mem::take(&mut world.movement);
    if ms.space.is_none() {
        ms.prepare(world);
    }
    let mut found = Vec::new();
    if let Some(mut s) = ms.space.take() {
        s.add_new(&ms, world);
        let mut policy = space::Policy { ms: &ms, world };
        let space::Space { index, geometry, .. } = &mut *s;
        if let Ok(references) = aa_spatial::rectangle_actors::collect(index, geometry, [at.x.to_bits(), at.y.to_bits()], radius.to_bits(), [0, 0], &mut policy) {
            found = references.into_iter().map(space::key_actor).filter(|&t| !space::is_scenery(t)).collect();
        }
        ms.space = Some(s);
    }
    world.movement = ms;
    found
}


/// Number of path requests waiting for a later tick.
pub fn queued_requests(world: &World) -> usize {
    world.movement.jobs.len()
}
