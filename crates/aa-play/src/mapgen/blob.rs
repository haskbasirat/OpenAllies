//! The blob solver of the terrain painters: the generic flood fill
//! (`.\Source\Sim\Solver\FloodFill.cpp`, solver vtable 0xA35EE4) with the
//! random map blob goal (vtable 0xA35E90), as 0x597090 drives it.
//!
//! Native objects: the painter embeds the solver at +4 (constructor
//! 0x5395C0): +0x94 tiles still to paint, +0xA0 radius, +0xA4 the tile
//! painter (painter +0xB8), +0xA8/+0xAC/+0xB0 the epoch state; its open
//! list (+0x80, class 0x53CDF0, 0x1F9C entries) is the binary heap of the
//! path search (`aa_navigation::search::NativeQueue`, push 0x53D090 / pop
//! 0x53D150); the goal (+0x44) holds the start tile (+0xC/+0x10), the shape
//! (+0x14 proximity, +0x18 consistency, +0x1C fuzzy, +0x20/+0x24 direction
//! bias, +0x28 its factor), the centre (+0x2C/+0x30) and the variation
//! (+0x34).
//!
//! One solve (0x539990 -> 0x539650): the GMap stamp advances and the epoch
//! state resets (goal +8, 0x538E30); the start tile is reached, scored and
//! pushed, then scored again (the best-tile bookkeeping, which nothing here
//! reads). Each pop scores the tile once more, advances the epoch counter
//! and paints it (0x5394D0); the solve ends when the count is painted or
//! the heap is empty or full. The popped tile's neighbours are visited in
//! direction order 1..=8 (0x554ED0 with this goal: +0x34 true, so all
//! eight; +0x40 true, so bounds only, no blocked test; the reached-from
//! direction is skipped, which is always a visited tile here); an unvisited
//! one is reached, scored (0x5392C0, one draw when fuzzy) and pushed with
//! priority binary32(0 + score) at depth + 1 (0x539190, 0x53D2A0).
//!
//! The score (0x5392C0) is evaluated on the x87 register at PC53, which is
//! binary64 here (every value stays far inside the normal binary64 range);
//! the stores the original makes to binary32 locals are kept.
use super::grid::World;
use super::rng::Rng;
use aa_navigation::search::{NativeQueue, QueueEntry};

/// 0x53CDF0: the open list holds 0x1F9C entries (push fails when full).
const QUEUE_LIMIT: usize = 0x1F9C;
/// 0xB322E0: (dx, dy) per direction 1..=8.
const DELTA: [[i32; 2]; 9] = [[0, 0], [-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];

/// The goal's shape fields (+0x14..+0x28), copied from the brush
/// (+0x40 proximity, +0x3C consistency, +0x44 fuzzy factor, +0x48/+0x4C
/// direction bias, +0x50 its factor) by 0x597090.
#[derive(Clone, Copy, Debug)]
pub(super) struct Shape {
    pub proximity: f32,
    pub consistency: f32,
    pub fuzzy: f32,
    pub dir_bias: [f32; 2],
    pub dir_bias_factor: f32,
}

/// The tile painter the solver calls for every popped tile (solver +0xA4
/// vtable +8): it records the tile and returns whether it counts.
pub(super) trait TilePainter {
    /// Vtable +4, called by 0x539990 once the start tile is inside the map:
    /// the solve's variation, centre and radius.
    fn begin(&mut self, _var: u32, _center: [f32; 2], _radius: f32) {}
    fn paint(&mut self, world: &mut World, tile: usize, var: u32) -> bool;
}

/// The solver state kept between solves (per painter natively; only the
/// stamps matter across solves, and a fresh stamp starts every solve).
pub(super) struct Solver {
    /// Tile +0x28 for this solver's purposes: the stamp of the solve that
    /// last reached the tile.
    visit: Vec<u32>,
    /// The stamp of the solve that painted the tile (tile +0x24 then holds
    /// the solve's variation; reaching a tile resets +0x24 to its committed
    /// variation, so an older paint never counts).
    painted: Vec<u32>,
    stamp: u32,
    queue: NativeQueue,
}

struct Goal {
    center: [f32; 2],
    radius: f32,
    var: u32,
    shape: Shape,
    epoch: i32,
}

impl Solver {
    pub(super) fn new(world: &World) -> Self {
        let n = world.tw * world.th;
        Self { visit: vec![0; n], painted: vec![0; n], stamp: 0, queue: NativeQueue::new(Some(QUEUE_LIMIT)) }
    }

    /// 0x538F40: how many of the eight neighbours (direction order) hold
    /// `var` (+0x24 when the tile was reached in this solve, else +0).
    fn same_neighbours(&self, world: &World, x: i32, y: i32, var: u32) -> i32 {
        let mut n = 0;
        for d in &DELTA[1..] {
            let (nx, ny) = (x + d[0], y + d[1]);
            if nx < 0 || ny < 0 || nx >= world.tw as i32 || ny >= world.th as i32 {
                continue;
            }
            let t = ny as usize * world.tw + nx as usize;
            let v = if self.visit[t] == self.stamp && self.painted[t] == self.stamp { var } else { world.var[t] };
            if v == var {
                n += 1;
            }
        }
        n
    }

    /// 0x5392C0, the extended result (callers store binary32).
    fn score(&self, world: &World, goal: &Goal, rng: &mut Rng, x: i32, y: i32) -> f64 {
        let shift = world.tile_bits;
        let half = f64::from(world.tile) * 0.5;
        let px = f64::from(x << shift) + half;
        let py = (f64::from(y << shift) + half) as f32;
        let mut dx = (px - f64::from(goal.center[0])) as f32;
        let dy_d = f64::from(py) - f64::from(goal.center[1]);
        let mut dy = dy_d as f32;
        let d = (dy_d * f64::from(dy) + f64::from(dx) * f64::from(dx)).sqrt() as f32;
        if d != 0.0 {
            let inv = 1.0 / f64::from(d);
            dx = (f64::from(dx) * inv) as f32;
            dy = (inv * f64::from(dy)) as f32;
        }
        let s = goal.shape;
        let fuzzy = if s.fuzzy != 0.0 { rng.at(0x53937D).unit_d() * f64::from(s.fuzzy) } else { 0.0 };
        let consistency = if s.consistency != 0.0 {
            f64::from(self.same_neighbours(world, x, y, goal.var)) * 0.125 * f64::from(s.consistency)
        } else {
            0.0
        };
        let proximity = if s.proximity != 0.0 { f64::from(d) * f64::from(s.proximity) / (f64::from(goal.radius) * 1.5) } else { 0.0 };
        let dir = if s.dir_bias_factor != 0.0 && d != 0.0 {
            let t = f64::from(dy) * f64::from(s.dir_bias[1]) + f64::from(dx) * f64::from(s.dir_bias[0]);
            t.abs() * f64::from(s.dir_bias_factor)
        } else {
            0.0
        };
        f64::from(goal.epoch * 10) - (((dir + consistency) + fuzzy) - proximity)
    }

    /// 0x539990: paint up to `n` tiles of `var` around `center`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn solve(
        &mut self,
        world: &mut World,
        painter: &mut dyn TilePainter,
        rng: &mut Rng,
        center: [f32; 2],
        n: i32,
        radius: f32,
        var: u32,
        shape: Shape,
    ) {
        let shift = world.tile_bits;
        let (sx, sy) = (ftol(center[0]) >> shift, ftol(center[1]) >> shift);
        if sx < 0 || sy < 0 || sx >= world.tw as i32 || sy >= world.th as i32 {
            return;
        }
        painter.begin(var, center, radius);
        // Goal +8 (0x538E30): 0x552440 and the epoch state reset.
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp >= 0x7FFF_FFFF {
            self.stamp = 0;
            self.visit.iter_mut().for_each(|v| *v = 0);
            self.painted.iter_mut().for_each(|v| *v = u32::MAX);
        }
        self.queue = NativeQueue::new(Some(QUEUE_LIMIT));
        let mut goal = Goal { center, radius, var, shape, epoch: 0 };
        let (mut state, mut counter) = (0, 0);
        // 0x5394D0's per-solve bound: ceil(radius / tile) tiles per epoch.
        let r_tiles = ftol_d((f64::from(radius) / f64::from(world.tile)).ceil());
        let mut remaining = n;
        let tw = world.tw;
        let start = sy as usize * tw + sx as usize;
        self.visit[start] = self.stamp;
        let h = self.score(world, &goal, rng, sx, sy) as f32;
        if !self.push(sx, sy, 0, h) {
            return;
        }
        let _best = self.score(world, &goal, rng, sx, sy);
        // The neighbour score's draw site is 0x53937D too; the three calls
        // differ in their return addresses (0x539725 start, 0x539789 best,
        // 0x5397DE pop, 0x53924C neighbour), which the trace keeps as the
        // chain, not the site.
        while let Some(node) = self.queue.pop() {
            let [x, y] = node.cell;
            let _ = self.score(world, &goal, rng, x, y);
            counter += 1;
            if state == 0 {
                if counter > r_tiles {
                    goal.epoch += 1;
                    state = 1;
                    counter = 0;
                }
            } else if counter > r_tiles * 4 {
                state = 0;
                counter = 0;
            }
            let t = y as usize * tw + x as usize;
            if painter.paint(world, t, var) {
                self.painted[t] = self.stamp;
                remaining -= 1;
            }
            if remaining == 0 {
                return;
            }
            for d in &DELTA[1..] {
                let (nx, ny) = (x + d[0], y + d[1]);
                if nx < 0 || ny < 0 || nx >= tw as i32 || ny >= world.th as i32 {
                    continue;
                }
                let nt = ny as usize * tw + nx as usize;
                if self.visit[nt] == self.stamp {
                    continue;
                }
                self.visit[nt] = self.stamp;
                let h = self.score(world, &goal, rng, nx, ny) as f32;
                if !self.push(nx, ny, node.depth + 1, h) {
                    return;
                }
            }
        }
    }

    /// 0x53D2A0: priority = binary32(cost + score) with cost 0 (goal +0x30).
    fn push(&mut self, x: i32, y: i32, depth: i32, h: f32) -> bool {
        let priority = (0.0f64 + f64::from(h)) as f32;
        self.queue
            .push(QueueEntry { cell: [x, y], depth, priority_bits: priority.to_bits(), cost_bits: 0 })
            .unwrap_or(false)
    }
}

/// `_ftol` 0x96CC4C on a binary32 value: truncation toward zero (the
/// original's out-of-range result is 0x80000000).
pub(super) fn ftol(v: f32) -> i32 {
    ftol_d(f64::from(v))
}

/// `_ftol` of a register value.
pub(super) fn ftol_d(v: f64) -> i32 {
    let t = v.trunc();
    if t.is_finite() && t >= -2_147_483_648.0 && t <= 2_147_483_647.0 { t as i32 } else { i32::MIN }
}

/// The 0xB39AF8 direction table (0x408ED0(0x800) at startup): entry i is
/// (cos a, sin a) of a = binary32(i * 2pi / 2048) (fcos/fsin stored to
/// binary32), normalised by 0x409230.
pub(super) fn direction_table() -> &'static [[f32; 2]] {
    static TABLE: std::sync::OnceLock<Vec<[f32; 2]>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let tau = f64::from(f32::from_bits(0x40C9_0FDB));
        (0..0x800)
            .map(|i| {
                let a = (f64::from(i) * tau / 2048.0) as f32;
                let c = f64::from(a).cos() as f32;
                let s = f64::from(a).sin() as f32;
                normalized([c, s])
            })
            .collect()
    })
}

/// 0x409230: scale to unit length when longer than 0.0001, else zero.
pub(super) fn normalized(v: [f32; 2]) -> [f32; 2] {
    let len = (f64::from(v[0]) * f64::from(v[0]) + f64::from(v[1]) * f64::from(v[1])).sqrt();
    if len <= f64::from(f32::from_bits(0x38D1_B717)) {
        return [0.0, 0.0];
    }
    let inv = 1.0 / len;
    [(inv * f64::from(v[0])) as f32, (inv * f64::from(v[1])) as f32]
}
