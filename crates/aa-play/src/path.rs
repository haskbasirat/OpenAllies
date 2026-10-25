//! Pathfinding on the map's cell grid. Owner: movement.
//!
//! Plain `f32` reimplementation of the original tile route solver, following
//! the exact ports in aa-navigation:
//! - Solve flow `0x53E1F0` (`aa_navigation::solver::solve`): endpoint
//!   adjustment, minimum distance, start/goal cells, search, materialization;
//! - destination adjustment `0x53D340` and interior clamp `0x552640`
//!   (`aa_navigation::endpoint`);
//! - the tile search `0x53D8D0` (`aa_navigation::search`): parent-facing
//!   five-neighbour fan, diagonal corner rule, first discovery closes a tile,
//!   goal detected on discovery, octile heuristic `0x53C7A0`, the native heap
//!   tie rules `0x53D090`/`0x53D150`, depth cap 255, queue limit 0x1F9C and the
//!   partial route to the cell closest to the goal;
//! - the step cost `0x53CB70` (`aa_navigation::cost`) with the terrain change
//!   penalty;
//! - predecessor-chain materialization `0x53D500` / `0x53CF40`
//!   (`aa_navigation::materialize`): direction-run reduction and
//!   line-of-sight smoothing traced with the tile walk `0x4DD150`
//!   (`aa_spatial::tile_walk`).
//!
//! Tile passability follows `0x54F370` with the actor descriptor mask of
//! `0x5531F0` (`aa_spatial::query::PreparedQuery::for_actor`): flags20 bit0
//! always blocks; ground actors add 0x500 (not land passable, occupied),
//! water actors (actor flag bit21) 0x600 (not water passable, occupied).
//!
//! Not ported: the SAI region pre-pass `0x53DC00` (region topology is not
//! part of `GameMap`), wall cost (no wall actors own tiles here) and the
//! occupied-tile exception `0x54F190`. A search stopped by the depth cap or
//! queue limit is reported as [`Status::Capped`] so the caller can continue
//! from its end, which is where the original region pre-pass would have
//! placed the next tile search.
use crate::map::{BLOCKED, GameMap, NOT_LAND_PASSABLE, NOT_WATER_PASSABLE};
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Original flags20 bit10: the tile holds registered objects (0x54F370).
pub const OCCUPIED: u32 = 0x400;
/// Native binary32 sqrt(2) (0x3FB504F3) used by the heuristic and step cost.
pub const DIAGONAL: f32 = f32::from_bits(0x3fb5_04f3);
/// Native search depth rule (0x53D8D0: `depth >= 255`).
pub const DEPTH_LIMIT: i32 = 255;
/// Native constructor queue limit (0x53CDF0).
pub const QUEUE_LIMIT: usize = 0x1f9c;
/// Native binary32 1e-4 (0x38D1B717).
const EPSILON: f32 = f32::from_bits(0x38d1_b717);
/// Native binary32 2e-4 (0x3951B717): endpoint Manhattan acceptance.
const MANHATTAN: f32 = f32::from_bits(0x3951_b717);
/// Native binary32 sqrt(1/2) (0x3F3504F3) of the direction table.
const HALF_DIAGONAL: f32 = f32::from_bits(0x3f35_04f3);

/// Native direction order (0 = none, then counter-clockwise from (-1,-1)).
const DELTAS: [[i32; 2]; 9] = [[0, 0], [-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];
/// 0x53D500 unit directions for the same order.
const UNITS: [[f32; 2]; 9] = [
    [0.0, 0.0],
    [-HALF_DIAGONAL, -HALF_DIAGONAL],
    [-1.0, 0.0],
    [-HALF_DIAGONAL, HALF_DIAGONAL],
    [0.0, 1.0],
    [HALF_DIAGONAL, HALF_DIAGONAL],
    [1.0, 0.0],
    [HALF_DIAGONAL, -HALF_DIAGONAL],
    [0.0, -1.0],
];

/// Which tiles block an actor: the descriptor mask of `0x5531F0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoverClass {
    /// Ground actor: blocked by `NOT_LAND_PASSABLE` and occupied tiles.
    #[default]
    Land,
    /// Actor flag bit21 (boats): blocked by `NOT_WATER_PASSABLE` and occupied tiles.
    Water,
}
impl MoverClass {
    pub fn tile_mask(self) -> u32 {
        match self {
            MoverClass::Land => NOT_LAND_PASSABLE | OCCUPIED,
            MoverClass::Water => NOT_WATER_PASSABLE | OCCUPIED,
        }
    }
}

/// One path node: the point and the unit direction of the leg that ends at
/// it (original path node words 0..3).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Waypoint {
    pub pos: Vec2,
    pub dir: Vec2,
}

/// Native Solve result: 0 failure, 1 goal, 2 partial. Partial is split by
/// cause so callers can tell "goal unreachable" from "search capped".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// Native 0: no route (endpoint not acceptable, zero distance, start enclosed).
    Failed,
    /// Native 1: the route ends at the (possibly adjusted) goal.
    Reached,
    /// Native 2 after the queue ran empty: the goal is unreachable; the route
    /// ends at the reachable cell closest to it.
    Partial,
    /// Native 2 from the depth cap or queue limit: the route ends at the
    /// closest cell found so far; the goal may still be reachable.
    Capped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub status: Status,
    /// Nodes nearest first; the start point is not included.
    pub waypoints: Vec<Waypoint>,
    /// The goal after `0x53D340` adjustment.
    pub goal: Vec2,
    /// Tiles taken from the search queue (for the per-tick budget).
    pub expanded: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub from: Vec2,
    pub to: Vec2,
    pub class: MoverClass,
    /// Strategy +0x30: the mover's maximum speed; 0 disables terrain penalties.
    pub max_speed: f32,
}

/// Original settings read by the solver.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathRules {
    /// SVar `PathingTerrainChangePenalty` (Data/Game/SVars.tgi line 89: 0.71).
    pub terrain_change_penalty: f32,
    /// Native queue limit policy (0x1F9C); `None` = unbounded.
    pub queue_limit: Option<usize>,
}
impl Default for PathRules {
    fn default() -> Self {
        Self { terrain_change_penalty: 0.71, queue_limit: Some(QUEUE_LIMIT) }
    }
}
impl PathRules {
    pub fn from_data(data: &crate::data::GameData) -> Self {
        Self { terrain_change_penalty: data.rule("PathingTerrainChangePenalty").unwrap_or(0.71), ..Self::default() }
    }
}

/// Tile view of the map for one mover class. Outside cells and the outer
/// ring are blocked (the original search requires a blocked outer ring).
pub struct Grid<'a> {
    map: &'a GameMap,
    mask: u32,
    width: i32,
    height: i32,
    cell: f32,
    /// Precomputed per-cell bits (1 blocked for the class, 2 flags20 bit0).
    table: Option<&'a [u8]>,
    /// Objects registered on each cell (flags20 bit10 while nonzero).
    occupied: Option<&'a [u16]>,
    /// Footprints (tile rectangles) the mover ignores: the structures it
    /// stands inside, like a recruit and its creator (+0x50 link, 0x4EB7D0).
    ignore: &'a [[i32; 4]],
}
impl<'a> Grid<'a> {
    pub fn new(map: &'a GameMap, class: MoverClass) -> Self {
        Self { map, mask: class.tile_mask(), width: map.width as i32, height: map.height as i32, cell: map.cell_size, table: None, occupied: None, ignore: &[] }
    }
    /// The map plus cells occupied by placed structures (`OCCUPIED`).
    pub fn with_occupancy(map: &'a GameMap, class: MoverClass, occupied: &'a [u16]) -> Self {
        Self { occupied: (occupied.len() == (map.width * map.height) as usize).then_some(occupied), ..Self::new(map, class) }
    }
    fn index(&self, c: [i32; 2]) -> Option<usize> {
        (c[0] >= 0 && c[1] >= 0 && c[0] < self.width && c[1] < self.height).then(|| (c[1] * self.width + c[0]) as usize)
    }
    fn flags(&self, c: [i32; 2]) -> u32 {
        if c[0] <= 0 || c[1] <= 0 || c[0] >= self.width - 1 || c[1] >= self.height - 1 {
            return BLOCKED;
        }
        let Some(i) = self.index(c) else {
            return BLOCKED;
        };
        let f = self.map.flags.get(i).copied().unwrap_or(0);
        let ignored = self.ignore.iter().filter(|r| c[0] >= r[0] && c[0] <= r[2] && c[1] >= r[1] && c[1] <= r[3]).count();
        if self.occupied.is_some_and(|o| usize::from(o[i]) > ignored) { f | OCCUPIED } else { f }
    }
    fn bits(&self, c: [i32; 2]) -> u8 {
        let f = self.flags(c);
        u8::from(f & (BLOCKED | self.mask) != 0) | (u8::from(f & BLOCKED != 0) << 1)
    }
    fn cached(&self, c: [i32; 2]) -> u8 {
        if self.ignore.iter().any(|r| c[0] >= r[0] && c[0] <= r[2] && c[1] >= r[1] && c[1] <= r[3]) {
            return self.bits(c);
        }
        match (self.table, self.index(c)) {
            (Some(t), Some(i)) => t[i],
            (Some(_), None) => 3,
            (None, _) => self.bits(c),
        }
    }
    /// flags20 bit0 only (the search's source-cell and discovery test).
    fn hard_blocked(&self, c: [i32; 2]) -> bool {
        self.cached(c) & 2 != 0
    }
    /// bit0 or the descriptor mask (`0x54F370` after the bit0 test).
    pub fn blocked(&self, c: [i32; 2]) -> bool {
        self.cached(c) & 1 != 0
    }
    pub fn blocked_at(&self, p: Vec2) -> bool {
        self.blocked(self.cell_of(p))
    }
    fn speed(&self, c: [i32; 2]) -> f32 {
        self.index(c).and_then(|i| self.map.speed_limit.get(i)).copied().unwrap_or(0.0)
    }
    /// Native `(int)x >> shift`; positions are clamped into the map first.
    pub fn cell_of(&self, p: Vec2) -> [i32; 2] {
        [(p.x.max(0.0) / self.cell) as i32, (p.y.max(0.0) / self.cell) as i32]
    }
    pub fn center(&self, c: [i32; 2]) -> Vec2 {
        let half = self.cell * 0.5;
        Vec2::new(c[0] as f32 * self.cell + half, c[1] as f32 * self.cell + half)
    }
    fn extent(&self) -> Vec2 {
        Vec2::new(self.width as f32 * self.cell, self.height as f32 * self.cell)
    }
    /// `0x552640`: keep a point one cell inside the map.
    fn clamp_interior(&self, p: Vec2) -> Vec2 {
        let min = self.cell;
        let max = self.extent() - Vec2::splat(self.cell + EPSILON);
        Vec2::new(if p.x < min { min } else { p.x.min(max.x) }, if p.y < min { min } else { p.y.min(max.y) })
    }
    /// `0x4DD150` tile walk from `from` to `to`.
    pub fn walk(&self, from: Vec2, to: Vec2) -> TileWalk {
        let current = self.cell_of(from);
        let end = self.cell_of(to);
        TileWalk {
            current,
            remaining: (end[0] - current[0]).abs() + (end[1] - current[1]).abs(),
            delta: to - from,
            point: from,
            started: false,
            cell: self.cell,
        }
    }
    /// False at the first blocked cell of the tile walk.
    pub fn line_clear(&self, from: Vec2, to: Vec2) -> bool {
        self.walk(from, to).all(|c| !self.blocked(c))
    }
}

/// `0x4DD030`/`0x4DD150`: the 4-connected tiles a segment crosses, start
/// tile first; exact boundary ties step in Y.
pub struct TileWalk {
    current: [i32; 2],
    remaining: i32,
    delta: Vec2,
    point: Vec2,
    started: bool,
    cell: f32,
}
impl Iterator for TileWalk {
    type Item = [i32; 2];
    fn next(&mut self) -> Option<[i32; 2]> {
        if !self.started {
            self.started = true;
            return Some(self.current);
        }
        if self.remaining <= 0 {
            return None;
        }
        self.remaining -= 1;
        let delta = self.delta;
        let axis = if delta.x.abs() < EPSILON {
            1
        } else if delta.y.abs() < EPSILON {
            0
        } else {
            let by = (self.current[1] + i32::from(delta.y >= 0.0)) as f32 * self.cell;
            let bx = (self.current[0] + i32::from(delta.x >= 0.0)) as f32 * self.cell;
            let tx = (bx - self.point.x) / delta.x;
            let ty = (by - self.point.y) / delta.y;
            if tx < ty {
                self.point = Vec2::new(bx, tx * delta.y + self.point.y);
                0
            } else {
                self.point = Vec2::new(ty * delta.x + self.point.x, by);
                1
            }
        };
        self.current[axis] += if delta[axis] < 0.0 { -1 } else { 1 };
        Some(self.current)
    }
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    cell: [i32; 2],
    depth: i32,
    priority: f32,
    cost: f32,
}

/// Blocked bits of one map for one mover class.
#[derive(Default, Debug)]
struct Table {
    key: (usize, u32, u32, usize, MoverClass, u64),
    cells: Vec<u8>,
}

/// Reusable search scratch (visit epochs, predecessor directions, queue)
/// and cached per-class tile tables.
#[derive(Default, Debug)]
pub struct Pathfinder {
    epoch: u32,
    visit: Vec<u32>,
    predecessor: Vec<u8>,
    heap: Vec<Entry>,
    width: i32,
    tables: Vec<Table>,
    /// The native search grid of the map (see [`exact`]).
    native: Option<exact::NativeGrid>,
    /// Requests the exact solver could not take (non-power-of-two cells or
    /// an exact-path error), solved by the f32 reference instead.
    pub fallbacks: u64,
    /// Solve with the exact native solver (see [`Pathfinder::exact`]). Off
    /// by default, the f32 reference kept for comparison; the game's
    /// movement solves with the exact solver.
    pub exact: bool,
}

fn normalize(v: Vec2) -> Vec2 {
    let length = v.length();
    if length <= EPSILON { Vec2::ZERO } else { v * (1.0 / length) }
}
fn next_direction(d: u8) -> u8 {
    if d == 8 { 1 } else { d + 1 }
}
fn previous_direction(d: u8) -> u8 {
    if d == 1 { 8 } else { d - 1 }
}
fn step(c: [i32; 2], d: u8) -> [i32; 2] {
    [c[0] + DELTAS[d as usize][0], c[1] + DELTAS[d as usize][1]]
}
fn parent(c: [i32; 2], d: u8) -> [i32; 2] {
    [c[0] - DELTAS[d as usize][0], c[1] - DELTAS[d as usize][1]]
}

impl Pathfinder {
    fn prepare(&mut self, grid: &Grid) {
        let cells = (grid.width.max(0) * grid.height.max(0)) as usize;
        if self.visit.len() != cells || self.width != grid.width {
            self.visit = vec![0; cells];
            self.predecessor = vec![0; cells];
            self.epoch = 0;
            self.width = grid.width;
        }
        // 0x552440 epoch with its zero-generation rollover.
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch >= 0x7fff_ffff {
            self.epoch = 1;
            self.visit.iter_mut().for_each(|v| *v = 0);
        }
        self.heap.clear();
    }
    fn mark(&mut self, grid: &Grid, to: [i32; 2], direction: u8) {
        if let Some(i) = grid.index(to) {
            self.visit[i] = self.epoch;
            self.predecessor[i] = direction;
        }
    }
    fn visited(&self, grid: &Grid, c: [i32; 2]) -> bool {
        grid.index(c).is_some_and(|i| self.visit[i] == self.epoch)
    }
    fn predecessor(&self, grid: &Grid, c: [i32; 2]) -> u8 {
        grid.index(c).map_or(0, |i| self.predecessor[i])
    }
    /// 0x53D090: ties rise above their parent.
    fn push(&mut self, entry: Entry, limit: Option<usize>) -> bool {
        if limit.is_some_and(|l| l > 0 && self.heap.len() == l) {
            return false;
        }
        let mut i = self.heap.len();
        self.heap.push(entry);
        while i > 0 {
            let p = (i - 1) / 2;
            if entry.priority > self.heap[p].priority {
                break;
            }
            self.heap[i] = self.heap[p];
            i = p;
        }
        self.heap[i] = entry;
        true
    }
    /// 0x53D150: the right child wins ties; stop when the tail is not greater.
    fn pop(&mut self) -> Option<Entry> {
        let first = *self.heap.first()?;
        let last = self.heap.pop()?;
        if self.heap.is_empty() {
            return Some(first);
        }
        let n = self.heap.len();
        let mut i = 0;
        while i * 2 + 1 < n {
            let l = i * 2 + 1;
            let r = l + 1;
            let child = if r < n && self.heap[l].priority >= self.heap[r].priority { r } else { l };
            if last.priority <= self.heap[child].priority {
                break;
            }
            self.heap[i] = self.heap[child];
            i = child;
        }
        self.heap[i] = last;
        Some(first)
    }

    /// Complete Solve for one request on the static map.
    pub fn solve(&mut self, map: &GameMap, rules: &PathRules, request: &Request) -> Route {
        self.solve_with(map, None, rules, request)
    }

    /// Complete Solve with cells occupied by structures: `occupancy` is the
    /// per-cell object count and a version that changes with it.
    pub fn solve_with(&mut self, map: &GameMap, occupancy: Option<(&[u16], u64)>, rules: &PathRules, request: &Request) -> Route {
        self.solve_ignoring(map, occupancy, &[], rules, request)
    }

    /// A pathfinder that solves with the exact native solver
    /// (`aa_navigation::solver` 53E1F0 at PC53), falling back to the f32
    /// reference only where the native code refuses (a cell size that is not
    /// a power of two, or an exact-path error; counted in `fallbacks`).
    pub fn exact() -> Self {
        Self { exact: true, ..Self::default() }
    }
    /// As `solve_with`, with structure footprints the mover ignores; exact
    /// when `self.exact`, else the f32 reference.
    pub fn solve_ignoring(&mut self, map: &GameMap, occupancy: Option<(&[u16], u64)>, ignore: &[[i32; 4]], rules: &PathRules, request: &Request) -> Route {
        let exact = self.exact;
        self.solve_by(map, occupancy, ignore, rules, request, exact)
    }
    /// The f32 reimplementation, kept as the reference the exact solver is
    /// compared with.
    pub fn solve_reference(&mut self, map: &GameMap, occupancy: Option<(&[u16], u64)>, ignore: &[[i32; 4]], rules: &PathRules, request: &Request) -> Route {
        self.solve_by(map, occupancy, ignore, rules, request, false)
    }
    fn solve_by(&mut self, map: &GameMap, occupancy: Option<(&[u16], u64)>, ignore: &[[i32; 4]], rules: &PathRules, request: &Request, native: bool) -> Route {
        let key = (map as *const GameMap as usize, map.width, map.height, map.flags.len(), request.class, occupancy.map_or(0, |o| o.1));
        let mut tables = std::mem::take(&mut self.tables);
        let index = match tables.iter().position(|t| t.key == key) {
            Some(i) => i,
            None => {
                let plain = match occupancy {
                    Some((cells, _)) => Grid::with_occupancy(map, request.class, cells),
                    None => Grid::new(map, request.class),
                };
                let mut cells = Vec::with_capacity((map.width * map.height) as usize);
                for y in 0..map.height as i32 {
                    for x in 0..map.width as i32 {
                        cells.push(plain.bits([x, y]));
                    }
                }
                if tables.len() >= 4 {
                    tables.remove(0);
                }
                tables.push(Table { key, cells });
                tables.len() - 1
            }
        };
        let mut grid = match occupancy {
            Some((cells, _)) => Grid::with_occupancy(map, request.class, cells),
            None => Grid::new(map, request.class),
        };
        grid.table = Some(&tables[index].cells);
        grid.ignore = ignore;
        let route = match native.then(|| exact::solve(&mut self.native, &grid, rules, request)).flatten() {
            Some(route) => route,
            None => {
                self.fallbacks += u64::from(native);
                self.solve_on(&grid, rules, request)
            }
        };
        self.tables = tables;
        route
    }

    fn solve_on(&mut self, grid: &Grid, rules: &PathRules, request: &Request) -> Route {
        let failed = |goal| Route { status: Status::Failed, waypoints: Vec::new(), goal, expanded: 0 };
        if grid.width < 3 || grid.height < 3 || grid.cell <= 0.0 {
            return failed(request.to);
        }
        let start = request.from;
        let mut goal = request.to;
        if !adjust_goal(grid, start, &mut goal) {
            return failed(goal);
        }
        if (goal - start).length() <= EPSILON {
            return failed(goal);
        }
        let start_cell = grid.cell_of(start);
        let goal_cell = grid.cell_of(goal);
        let (status, end, expanded) = self.search(grid, rules, request.max_speed, start_cell, goal_cell, goal);
        let Some(end) = end else {
            return Route { status: Status::Failed, waypoints: Vec::new(), goal, expanded };
        };
        let waypoints = self.materialize(grid, start_cell, goal_cell, start, goal, end);
        Route { status, waypoints, goal, expanded }
    }

    fn heuristic(&self, grid: &Grid, cell: [i32; 2], goal_cell: [i32; 2], goal: Vec2) -> f32 {
        if cell == goal_cell {
            return 0.0;
        }
        let d = (grid.center(cell) - goal).abs();
        let (small, large) = if d.x < d.y { (d.x, d.y) } else { (d.y, d.x) };
        small * DIAGONAL + (large - small)
    }

    /// 0x53CB70 for flat tiles (no wall actors): step plus terrain change penalty.
    fn cost(&self, grid: &Grid, rules: &PathRules, max_speed: f32, current: &Entry, next: [i32; 2], direction: u8) -> f32 {
        let mut step = if direction & 1 != 0 { DIAGONAL } else { 1.0 } * grid.cell;
        if max_speed > 0.0 {
            let source = grid.speed(current.cell);
            let reference = if source > 0.0 { max_speed.min(source) } else { max_speed };
            let destination = grid.speed(next);
            if destination > 0.0 && destination < reference {
                step += rules.terrain_change_penalty / destination * reference;
            }
        }
        step + current.cost
    }

    /// 0x554ED0 neighbour policy for this search: (first, last, reverse).
    fn fan(&self, grid: &Grid, cell: [i32; 2]) -> (u8, u8, u8) {
        if grid.hard_blocked(cell) {
            return (0, 0, 0);
        }
        let parent = self.predecessor(grid, cell);
        if parent == 0 {
            return (1, 8, 0);
        }
        (
            previous_direction(previous_direction(parent)),
            next_direction(next_direction(parent)),
            next_direction(next_direction(next_direction(next_direction(parent)))),
        )
    }
    fn allowed(&self, grid: &Grid, cell: [i32; 2], direction: u8, reverse: u8) -> bool {
        if direction == reverse {
            return false;
        }
        let candidate = step(cell, direction);
        if grid.blocked(candidate) {
            return false;
        }
        if direction & 1 != 0 {
            // Diagonals need both adjacent orthogonals open.
            if grid.blocked(step(cell, previous_direction(direction))) || grid.blocked(step(cell, next_direction(direction))) {
                return false;
            }
        }
        true
    }

    /// 0x53D8D0. Returns (status, end cell, tiles expanded).
    fn search(&mut self, grid: &Grid, rules: &PathRules, max_speed: f32, start: [i32; 2], goal_cell: [i32; 2], goal: Vec2) -> (Status, Option<[i32; 2]>, u32) {
        self.prepare(grid);
        if grid.index(start).is_none() || grid.index(goal_cell).is_none() {
            return (Status::Failed, None, 0);
        }
        self.mark(grid, start, 0);
        if start == goal_cell {
            return (Status::Reached, Some(start), 0);
        }
        let mut closest = start;
        let mut closest_distance = self.heuristic(grid, start, goal_cell, goal);
        let mut expanded = 0u32;
        self.push(Entry { cell: start, depth: 0, priority: closest_distance, cost: 0.0 }, rules.queue_limit);
        while let Some(current) = self.pop() {
            let distance = self.heuristic(grid, current.cell, goal_cell, goal);
            if distance < closest_distance {
                closest = current.cell;
                closest_distance = distance;
            }
            expanded += 1;
            if current.depth >= DEPTH_LIMIT {
                return if closest == start { (Status::Failed, None, expanded) } else { (Status::Capped, Some(closest), expanded) };
            }
            let (mut direction, last, reverse) = self.fan(grid, current.cell);
            while direction != 0 {
                if self.allowed(grid, current.cell, direction, reverse) {
                    let candidate = step(current.cell, direction);
                    if !self.visited(grid, candidate) {
                        self.mark(grid, candidate, direction);
                        if candidate == goal_cell {
                            return (Status::Reached, Some(candidate), expanded);
                        }
                        let remaining = self.heuristic(grid, candidate, goal_cell, goal);
                        let cost = self.cost(grid, rules, max_speed, &current, candidate, direction);
                        let entry = Entry { cell: candidate, depth: current.depth + 1, priority: cost + remaining, cost };
                        if !self.push(entry, rules.queue_limit) {
                            return (Status::Capped, Some(closest), expanded);
                        }
                    }
                }
                if direction == last {
                    break;
                }
                direction = next_direction(direction);
            }
        }
        if closest == start {
            // The original still materializes the start cell (a same-cell
            // route); with nothing reachable that is no route at all.
            return (Status::Failed, None, expanded);
        }
        (Status::Partial, Some(closest), expanded)
    }

    /// 0x53D500 / 0x53CF40. Nodes are collected tail first, then reversed.
    fn materialize(&self, grid: &Grid, start_cell: [i32; 2], goal_cell: [i32; 2], start: Vec2, goal: Vec2, end: [i32; 2]) -> Vec<Waypoint> {
        let mut nodes: Vec<Waypoint> = Vec::new();
        if end == start_cell {
            let dir = normalize(goal - start);
            if goal != start && dir != Vec2::ZERO {
                nodes.push(Waypoint { pos: goal, dir });
            }
            return nodes;
        }
        let update_head = |nodes: &mut Vec<Waypoint>, from: Vec2| {
            if let Some(head) = nodes.last_mut() {
                head.dir = normalize(head.pos - from);
            }
        };
        let mut last_dir = self.predecessor(grid, end);
        let initial = Vec2::from(UNITS[last_dir as usize]);
        let xy = if end == goal_cell { goal } else { grid.center(end) };
        nodes.push(Waypoint { pos: xy, dir: initial });
        let mut anchor = end;
        let mut previous = end;
        let mut current = parent(end, last_dir);
        // Each tile of the chain is visited a bounded number of times.
        let mut guard = (grid.width * grid.height) as usize * 4 + 16;
        loop {
            guard -= 1;
            if guard == 0 {
                break;
            }
            let current_dir = self.predecessor(grid, current);
            if previous == anchor {
                if current == start_cell {
                    break;
                }
                if current_dir != last_dir {
                    previous = current;
                }
            } else {
                let head = nodes.last().map_or(goal, |h| h.pos);
                if !grid.line_clear(grid.center(current), head) {
                    let point = grid.center(previous);
                    update_head(&mut nodes, point);
                    nodes.push(Waypoint { pos: point, dir: initial });
                    anchor = previous;
                    last_dir = self.predecessor(grid, previous);
                    continue;
                }
                if current == start_cell {
                    break;
                }
                previous = current;
            }
            if current_dir == 0 {
                break;
            }
            current = parent(current, current_dir);
        }
        update_head(&mut nodes, start);
        nodes.reverse();
        nodes
    }
}

/// The exact native solver over the same tile view: `aa_navigation::solver`
/// 53E1F0 with its search 53D8D0, step cost 53CB70, endpoint adjustment
/// 53D340/552640, materialization 53D500/53CF40 and the tile walk 4DD150
/// (`aa_spatial::tile_walk`), all at the simulation's PC53, with
/// `aa_numeric::Fast` arithmetic. Not modelled here, as in the f32
/// reference: wall actors on tiles (53CB70's wall cost), the object
/// displacement 53CA30 and the region pre-pass 53DC00 (no region topology).
mod exact {
    use super::{Grid, PathRules, Request, Route, Status, Waypoint};
    use aa_navigation::{OwnedPath, cost, endpoint, materialize, search, solver};
    use aa_numeric::{Fast, Precision};
    use aa_spatial::tile_walk::{TileGrid, TileWalk};
    use glam::Vec2;
    use std::cmp::Ordering;

    /// The simulation thread's precision control (research/phase8-tick/fpu-precision.md).
    const PC: Precision = Precision::Double53;

    /// The native search grid of one map: flags20 bit0 (hard-blocked tiles and
    /// the outer ring) and the visit epochs and predecessor directions that
    /// the searches leave behind, which the native grid keeps between them.
    #[derive(Debug)]
    pub struct NativeGrid {
        key: (usize, i32, i32, u32),
        grid: search::SearchGrid,
    }

    /// The host side of one solve: tile predicates from the class view
    /// (0x54F370 with the descriptor mask), terrain speeds, the tile walk.
    struct Host<'g, 'a> {
        grid: &'g Grid<'a>,
        tile: TileGrid,
        extent: [u32; 2],
        max_speed_bits: u32,
        penalty_bits: u32,
        walk: Option<TileWalk>,
    }

    impl endpoint::World for Host<'_, '_> {
        /// 0x53CE40 as the tile test: inside the map and the tile open.
        fn acceptable(&mut self, _: [u32; 2], point: [u32; 2]) -> endpoint::Result<bool> {
            for (axis, &value) in point.iter().enumerate() {
                let v = Fast::from_f32_bits(value)?;
                if v.is_negative_nonzero() || v.compare(&Fast::from_f32_bits(self.extent[axis])?) != Ordering::Less {
                    return Ok(false);
                }
            }
            let shift = self.tile.coordinate_shift & 31;
            let cell = [Fast::from_f32_bits(point[0])?.truncate_i32()? >> shift, Fast::from_f32_bits(point[1])?.truncate_i32()? >> shift];
            Ok(!self.grid.blocked(cell))
        }
        /// 0x53CA30 moves the point around a blocking object; objects do not
        /// own tiles here.
        fn displace_blocked(&mut self, _: [u32; 2], _: &mut [u32; 2]) -> endpoint::Result<bool> {
            Ok(false)
        }
    }
    impl search::SearchWorld for Host<'_, '_> {
        fn collision_blocked(&mut self, _: &search::SearchGrid, cell: [i32; 2]) -> search::Result<bool> {
            Ok(self.grid.blocked(cell))
        }
        fn accumulated_cost(&mut self, grid: &search::SearchGrid, current: search::QueueEntry, next: [i32; 2], p: Precision) -> search::Result<u32> {
            let mut state = cost::State { current, maximum_speed_bits: self.max_speed_bits, collision_owner: None };
            cost::accumulated_cost(grid.cell_size_bits, next, &mut state, p, self).map_err(|e| search::SearchError::World(e.to_string()))
        }
    }
    impl cost::World for Host<'_, '_> {
        fn tile_actor(&mut self, _: [i32; 2]) -> cost::Result<Option<u32>> {
            Ok(None)
        }
        fn has_wall_component(&self, _: u32) -> cost::Result<bool> {
            Ok(false)
        }
        fn outer_actor(&mut self, _: u32, _: &mut cost::State) -> cost::Result<Option<u32>> {
            Ok(None)
        }
        fn pathing_wall_cost_bits(&mut self) -> cost::Result<u32> {
            Err(cost::Error::World("no wall actors own tiles".into()))
        }
        fn body_health_bits(&mut self, _: u32) -> cost::Result<Option<[u32; 2]>> {
            Ok(None)
        }
        fn prepare_terrain_penalty(&mut self) -> cost::Result<()> {
            Ok(())
        }
        fn terrain_speed_bits(&mut self, cell: [i32; 2]) -> cost::Result<u32> {
            Ok(self.grid.speed(cell).to_bits())
        }
        fn terrain_change_penalty_bits(&mut self) -> cost::Result<u32> {
            Ok(self.penalty_bits)
        }
    }
    impl materialize::World for Host<'_, '_> {
        fn begin_trace(&mut self, from: [u32; 2], to: [u32; 2], _: &mut materialize::State, _: &mut OwnedPath) -> materialize::Result<()> {
            self.walk = Some(TileWalk::new(self.tile, from, to, PC).map_err(|e| materialize::Error::World(e.to_string()))?);
            Ok(())
        }
        fn next_trace_cell(&mut self, _: &mut materialize::State, _: &mut OwnedPath) -> materialize::Result<Option<[i32; 2]>> {
            let Some(walk) = self.walk.as_mut() else {
                return Ok(None);
            };
            let mut budget = walk.remaining_candidates();
            walk.next_cell_with_budget(&mut budget).map_err(|e| materialize::Error::World(e.to_string()))
        }
        fn cell_blocked(&mut self, cell: [i32; 2], _: &mut materialize::State, _: &mut OwnedPath) -> materialize::Result<bool> {
            Ok(self.grid.blocked(cell))
        }
    }
    impl solver::World for Host<'_, '_> {
        fn disable_path_logging(&mut self) {}
        fn use_region_pather(&self) -> bool {
            false
        }
        fn collision_actor_flags(&self) -> Option<u32> {
            None
        }
        fn begin_region_probe(&mut self) -> solver::Result<()> {
            Err(solver::Error::World("the region pre-pass is not modelled".into()))
        }
        fn region_line_clear(&mut self, _: [u32; 2], _: [u32; 2]) -> solver::Result<bool> {
            Err(solver::Error::World("the region pre-pass is not modelled".into()))
        }
        fn region_path(&mut self, _: [u32; 2], _: [u32; 2], _: &mut OwnedPath) -> solver::Result<i32> {
            Err(solver::Error::World("the region pre-pass is not modelled".into()))
        }
        fn collision_actor_id(&self) -> Option<u32> {
            None
        }
        fn collision_target_id(&self) -> Option<u32> {
            None
        }
        fn collision_word28(&self) -> u32 {
            0
        }
    }

    /// One native Solve; None when the native code refuses the request (a
    /// cell size that is not a power of two, or an exact-path error), for the
    /// f32 reference to take.
    pub fn solve(native: &mut Option<NativeGrid>, grid: &Grid, rules: &PathRules, request: &Request) -> Option<Route> {
        let cell = grid.cell;
        let cell_bits = cell.to_bits();
        // The native grid addresses tiles as `(int)x >> shift`.
        if !(cell >= 1.0 && cell_bits & 0x7f_ffff == 0) || grid.width < 3 || grid.height < 3 {
            return None;
        }
        let shift = (cell_bits >> 23) - 127;
        let key = (grid.map as *const crate::map::GameMap as usize, grid.width, grid.height, cell_bits);
        if native.as_ref().is_none_or(|n| n.key != key) {
            let mut cells = Vec::with_capacity((grid.width * grid.height) as usize);
            for y in 0..grid.height {
                for x in 0..grid.width {
                    cells.push(search::SearchCell { flags20: u32::from(grid.hard_blocked([x, y])), visit_epoch: 0, predecessor_direction: 0 });
                }
            }
            *native = Some(NativeGrid { key, grid: search::SearchGrid { width: grid.width, height: grid.height, coordinate_shift: shift, cell_size_bits: cell_bits, epoch: 0, cells } });
        }
        let native = native.as_mut()?;
        let extent = [(grid.width as f32 * cell).to_bits(), (grid.height as f32 * cell).to_bits()];
        let mut host = Host {
            grid,
            tile: TileGrid { width: grid.width, height: grid.height, coordinate_shift: shift, cell_size_bits: cell_bits },
            extent,
            max_speed_bits: request.max_speed.to_bits(),
            penalty_bits: rules.terrain_change_penalty.to_bits(),
            walk: None,
        };
        let mut path = OwnedPath { nodes: Vec::new(), kind1_count: 0, kind2_count: 0, dirty: false, word18: 0 };
        let mut state = solver::StrategyState {
            route: materialize::State { start_cell: [0, 0], goal_cell: [0, 0], start_bits: [0, 0], goal_bits: [0, 0], path_linked: false, collision_linked: false, collision_word28: 0 },
            actor_id: 0,
            target_id: 0,
            option_word: 0,
        };
        let options = solver::Options { bounds: endpoint::Bounds { cell_size_bits: cell_bits, extent_bits: extent }, precision: PC, queue_limit: rules.queue_limit };
        let bits = |p: Vec2| [p.x.to_bits(), p.y.to_bits()];
        let request_bits = solver::Request { start_bits: bits(request.from), goal_bits: bits(request.to), option_word: 0 };
        let mut summary = solver::Summary::default();
        let status = solver::solve_detailed(&mut native.grid, request_bits, &mut state, &mut path, options, &mut host, &mut summary).ok()?;
        let at = |x: u32, y: u32| Vec2::new(f32::from_bits(x), f32::from_bits(y));
        let waypoints = path.nodes.iter().map(|n| Waypoint { pos: at(n.words[0], n.words[1]), dir: at(n.words[2], n.words[3]) }).collect();
        let goal = if summary.searched { at(state.route.goal_bits[0], state.route.goal_bits[1]) } else { request.to };
        let status = match status {
            1 => Status::Reached,
            2 if summary.capped => Status::Capped,
            2 => Status::Partial,
            _ => Status::Failed,
        };
        Some(Route { status, waypoints, goal, expanded: summary.expanded as u32 })
    }
}

/// `0x53D340` with `0x53CE40` reduced to the tile test: walk the goal back
/// toward the start one whole cell at a time until it stands on an open tile.
fn adjust_goal(grid: &Grid, start: Vec2, goal: &mut Vec2) -> bool {
    let step = grid.cell.trunc();
    if step <= 0.0 {
        return false;
    }
    let delta = *goal - start;
    let distance = delta.length();
    let mut remaining = distance;
    let direction = if distance > 0.0 { delta * (step / distance) } else { delta };
    let success = loop {
        let candidate = grid.clamp_interior(*goal);
        if !grid.blocked_at(candidate) {
            *goal = candidate;
            break true;
        }
        if remaining <= 0.0 {
            break false;
        }
        *goal -= direction;
        let next = remaining - step;
        if next <= 0.0 {
            *goal = start;
            remaining = 0.0;
        } else {
            remaining = next;
        }
    };
    success && (goal.x - start.x).abs() + (goal.y - start.y).abs() >= MANHATTAN
}

/// Waypoints from `from` to `to` for a ground actor, nearest first; empty
/// when no route exists. Partial routes end at the closest reachable point.
pub fn find(map: &GameMap, from: Vec2, to: Vec2) -> Vec<Vec2> {
    let request = Request { from, to, class: MoverClass::Land, max_speed: 0.0 };
    Pathfinder::default().solve(map, &PathRules::default(), &request).waypoints.into_iter().map(|w| w.pos).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_map(size: u32, cell: f32) -> GameMap {
        GameMap {
            id: "test".into(),
            width: size,
            height: size,
            cell_size: cell,
            flags: vec![0; (size * size) as usize],
            speed_limit: vec![0.0; (size * size) as usize],
            ..Default::default()
        }
    }
    fn set(map: &mut GameMap, x: u32, y: u32, flags: u32) {
        let w = map.width;
        map.flags[(y * w + x) as usize] |= flags;
    }

    #[test]
    fn heap_keeps_native_tie_order() {
        let mut p = Pathfinder::default();
        for (i, prio) in [3.0f32, 1.0, 1.0, 2.0, 1.0].into_iter().enumerate() {
            p.push(Entry { cell: [i as i32, 0], depth: 0, priority: prio, cost: 0.0 }, None);
        }
        let order: Vec<i32> = std::iter::from_fn(|| p.pop()).map(|e| e.cell[0]).collect();
        // Equal priorities: the newest rises above its parent on insertion.
        assert_eq!(order[0], 4);
        assert_eq!(order.len(), 5);
        assert_eq!(*order.last().unwrap(), 0);
    }

    #[test]
    fn straight_open_route_is_one_leg() {
        let map = open_map(32, 4.0);
        let r = Pathfinder::default().solve(&map, &PathRules::default(), &Request { from: Vec2::new(10.0, 10.0), to: Vec2::new(100.0, 90.0), class: MoverClass::Land, max_speed: 0.0 });
        assert_eq!(r.status, Status::Reached);
        assert_eq!(r.waypoints.len(), 1, "{:?}", r.waypoints);
        assert_eq!(r.waypoints[0].pos, Vec2::new(100.0, 90.0));
        assert!((r.waypoints[0].dir - (Vec2::new(90.0, 80.0)).normalize()).length() < 1e-5);
    }

    #[test]
    fn border_ring_and_blocked_goal_are_adjusted() {
        let mut map = open_map(32, 4.0);
        for y in 0..32 {
            set(&mut map, 20, y, NOT_LAND_PASSABLE);
        }
        // Goal inside the water column: walked back toward the start.
        let r = Pathfinder::default().solve(&map, &PathRules::default(), &Request { from: Vec2::new(10.0, 60.0), to: Vec2::new(82.0, 60.0), class: MoverClass::Land, max_speed: 0.0 });
        assert_eq!(r.status, Status::Reached);
        assert!(r.goal.x < 80.0 && r.goal.x > 70.0, "{:?}", r.goal);
    }
}
