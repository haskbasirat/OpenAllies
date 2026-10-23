//! Land reachability for the unpack site search: which tiles a ground
//! truck can drive to from where it stands, on the map's land tiles plus
//! the footprints of standing structures (the movement owner's occupancy:
//! static actors with a blocking shape, `0x53D8D0` tile rules).
//!
//! The original's site search only offers spots its truck can reach (the
//! deploy order of an unreachable spot fails in the mover as "the deploy
//! spot cannot be reached"). The native search moves diagonally only when
//! both orthogonal neighbours are open, so 4-connected components of the
//! open tiles are exactly the reachable sets; the components are labelled
//! once per change of the structures and shared by every truck.
use crate::path::{Grid, MoverClass};
use crate::world::World;
use glam::Vec2;

const NONE: u32 = u32::MAX;
/// Native binary32 epsilon of the movement owner's footprint rule.
const EPSILON: f32 = f32::from_bits(0x38d1_b717);

#[derive(Default)]
pub struct Reach {
    width: i32,
    height: i32,
    cell: f32,
    /// Map-blocked tiles (terrain, outer ring).
    terrain: Vec<bool>,
    /// Structure footprints (tile rectangles).
    rects: Vec<[i32; 4]>,
    /// Component of every open tile (NONE when blocked).
    label: Vec<u32>,
    key: (usize, usize, usize),
}

impl Reach {
    /// The cache key: the map, the actor count and the live structures.
    fn key_of(world: &World) -> (usize, usize, usize) {
        let live = world.actors.iter().filter(|a| a.alive && !a.removed && world.data.def(a.def).speed_max <= 0.0).count();
        (std::sync::Arc::as_ptr(&world.map) as usize, world.actors.len(), live)
    }

    /// Rebuild when the map or the structures changed.
    pub fn update(&mut self, world: &World) {
        let key = Self::key_of(world);
        if key == self.key && !self.label.is_empty() {
            return;
        }
        *self = Self::build(world);
        self.key = key;
    }

    pub fn build(world: &World) -> Self {
        let map = &world.map;
        let (w, h) = (map.width as i32, map.height as i32);
        let n = (w.max(0) * h.max(0)) as usize;
        let grid = Grid::new(map, MoverClass::Land);
        let mut terrain = vec![false; n];
        for y in 0..h {
            for x in 0..w {
                terrain[(y * w + x) as usize] = grid.blocked([x, y]);
            }
        }
        let mut rects = Vec::new();
        let mut covered = vec![0u16; n];
        for a in world.actors.iter().filter(|a| a.alive && !a.removed) {
            let def = world.data.def(a.def);
            if def.speed_max > 0.0 {
                continue;
            }
            if let Some(r) = footprint(def, a.pos.truncate(), a.facing, map) {
                for y in r[1]..=r[3] {
                    for x in r[0]..=r[2] {
                        let c = &mut covered[(y * w + x) as usize];
                        *c = c.saturating_add(1);
                    }
                }
                rects.push(r);
            }
        }
        let mut label = vec![NONE; n];
        let mut next = 0u32;
        let mut stack = Vec::new();
        for start in 0..n {
            if label[start] != NONE || terrain[start] || covered[start] > 0 {
                continue;
            }
            label[start] = next;
            stack.push(start);
            while let Some(i) = stack.pop() {
                let (x, y) = ((i as i32) % w, (i as i32) / w);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= w || ny >= h {
                        continue;
                    }
                    let j = (ny * w + nx) as usize;
                    if label[j] == NONE && !terrain[j] && covered[j] == 0 {
                        label[j] = next;
                        stack.push(j);
                    }
                }
            }
            next += 1;
        }
        Reach { width: w, height: h, cell: map.cell_size, terrain, rects, label, key: (0, 0, 0) }
    }

    fn cell_of(&self, p: Vec2) -> Option<(i32, i32)> {
        if self.cell <= 0.0 {
            return None;
        }
        let (x, y) = ((p.x.max(0.0) / self.cell) as i32, (p.y.max(0.0) / self.cell) as i32);
        (x < self.width && y < self.height).then_some((x, y))
    }

    /// The components a mover at `from` can enter: its own tile's, or, when
    /// it stands inside structure footprints (a truck leaving the corps),
    /// those touching the open border of the footprints it stands in (the
    /// movement owner lets it ignore them).
    fn start_labels(&self, from: Vec2) -> Vec<u32> {
        let Some((x, y)) = self.cell_of(from) else { return Vec::new() };
        let i = (y * self.width + x) as usize;
        if self.label[i] != NONE {
            return vec![self.label[i]];
        }
        if self.terrain[i] {
            return Vec::new();
        }
        let inside: Vec<[i32; 4]> = self.rects.iter().copied().filter(|r| x >= r[0] && x <= r[2] && y >= r[1] && y <= r[3]).collect();
        let mut out = Vec::new();
        for r in inside {
            for yy in (r[1] - 1)..=(r[3] + 1) {
                for xx in (r[0] - 1)..=(r[2] + 1) {
                    if xx < 0 || yy < 0 || xx >= self.width || yy >= self.height {
                        continue;
                    }
                    let l = self.label[(yy * self.width + xx) as usize];
                    if l != NONE && !out.contains(&l) {
                        out.push(l);
                    }
                }
            }
        }
        out
    }

    /// A ground mover at `from` can drive to the open tile at `to`.
    pub fn reachable(&self, from: Vec2, to: Vec2) -> bool {
        if self.label.is_empty() {
            return true;
        }
        let Some((x, y)) = self.cell_of(to) else { return false };
        let goal = self.label[(y * self.width + x) as usize];
        goal != NONE && self.start_labels(from).contains(&goal)
    }
}

/// The movement owner's structure footprint: the tile rectangle of a
/// static actor's blocking shape (CIRCLE radius, else the rotated box), or
/// None when it is too small to mark tiles.
fn footprint(def: &crate::data::Def, pos: Vec2, facing: f32, map: &crate::map::GameMap) -> Option<[i32; 4]> {
    let size = def.blocking_size?;
    let cell = map.cell_size;
    if cell <= 0.0 || map.width == 0 || map.height == 0 {
        return None;
    }
    let (rx, ry, radius) = match def.blocking_shape.as_deref()?.to_ascii_uppercase().as_str() {
        "CIRCLE" => (size[0], size[0], size[0]),
        _ => {
            let (hw, hh) = (size[0] * 0.5, size[1] * 0.5);
            let f = crate::formation::facing_vector(facing);
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
