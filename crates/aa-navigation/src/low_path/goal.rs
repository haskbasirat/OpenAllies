//! KKC_LowPathGoal (solver+0x7C, vtable 0xA3634C) and its helpers.
use super::{
    ActorShape, EPSILON, Error, FLT_MAX, GOAL_EPSILON, LowPathSettings, LowPathWorld, NEGATIVE_ONE,
    ONE, Result, SQRT2, Window, ZERO, f, node, normalize, pop_front_kind0, push_front, set_byte_31,
    set_facing, source, st,
};
use crate::OwnedPath;
use crate::result_plan::{CollisionContext, Object};
use crate::search::{DIRECTIONS, QueueEntry, SearchGrid};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Goal words +4..+0x34. `path_linked`/`context_linked` mirror the +0x14/+0x18
/// pointers set by 0x53C170 and cleared by OnFailed / OnSolved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Goal {
    pub start_cell: [i32; 2],
    pub goal_cell: [i32; 2],
    pub path_linked: bool,
    pub context_linked: bool,
    pub start_bits: [u32; 2],
    /// Requested goal, moved in place by OnSolveStarted (0x53B490).
    pub goal_bits: [u32; 2],
    /// +0x30, set when a same-cell goal is reached by a clear sweep.
    pub direct: bool,
    /// +0x34, incremented by every Mark (0x53A610).
    pub examined: i32,
}

/// Direction code tables 0xAB78BC (x), 0xAB78C8 (y) and 0xAB78D0.
const DIRECTION_CODE: [u32; 13] = [0, 2, 8, 1, 6, 0, 7, 0, 4, 3, 0, 0, 5];

pub(crate) struct Run<'a, W> {
    pub window: &'a mut Window,
    pub goal: &'a mut Goal,
    pub grid: &'a mut SearchGrid,
    pub path: &'a mut OwnedPath,
    pub context: &'a mut CollisionContext,
    pub settings: LowPathSettings,
    pub p: Precision,
    pub world: &'a mut W,
}

fn less(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Less
}
fn absolute(value: Binary) -> Binary {
    if value.is_negative_nonzero() {
        value.negated()
    } else {
        value
    }
}

impl<W: LowPathWorld> Run<'_, W> {
    /// 0x53AF10: exact start / goal positions for their cells, else centers.
    pub(crate) fn position(&self, cell: [i32; 2]) -> Result<[u32; 2]> {
        if cell == self.goal.start_cell {
            Ok(self.goal.start_bits)
        } else if cell == self.goal.goal_cell {
            Ok(self.goal.goal_bits)
        } else {
            self.window.center(self.grid, cell, self.p)
        }
    }
    /// 0x53ADB0, extended result (octile distance to the exact goal).
    pub(crate) fn heuristic(&self, cell: [i32; 2]) -> Result<Binary> {
        let p = self.p;
        if cell == self.goal.goal_cell {
            return f(ZERO);
        }
        let point = if cell == self.goal.start_cell {
            self.goal.start_bits
        } else {
            self.window.center(self.grid, cell, p)?
        };
        let x = absolute(f(point[0])?.sub(&f(self.goal.goal_bits[0])?, p)?);
        let y = absolute(f(point[1])?.sub(&f(self.goal.goal_bits[1])?, p)?);
        let (small, large) = if less(&x, &y) { (x, y) } else { (y, x) };
        Ok(large.sub(&small, p)?.add(&small.mul(&f(SQRT2)?, p)?, p)?)
    }
    /// 0x53AD00: accumulated cost from a popped entry to `cell`.
    pub(crate) fn cost(&self, entry: &QueueEntry, cell: [i32; 2]) -> Result<Binary> {
        let p = self.p;
        let from = if entry.cell == self.goal.start_cell {
            self.goal.start_bits
        } else {
            self.window.center(self.grid, entry.cell, p)?
        };
        if entry.cell == cell {
            return f(ZERO)?.add(&f(entry.cost_bits)?, p).map_err(Error::from);
        }
        let to = self.window.center(self.grid, cell, p)?;
        let dy = f(to[1])?.sub(&f(from[1])?, p)?;
        let dx = f(to[0])?.sub(&f(from[0])?, p)?;
        Ok(dx
            .mul(&dx, p)?
            .add(&dy.mul(&dy, p)?, p)?
            .sqrt(p)?
            .add(&f(entry.cost_bits)?, p)?)
    }
    /// 0x53A610 Mark: stamp, predecessor direction, examined count.
    pub(crate) fn mark(&mut self, from: [i32; 2], to: [i32; 2], index: usize) -> Result<()> {
        self.window.cells[index].stamp = self.grid.epoch;
        let dx = to[0].wrapping_sub(from[0]);
        let dy = to[1].wrapping_sub(from[1]);
        let code = |d: i32, negative: usize, positive: usize| match d {
            -1 => Some(negative),
            0 => Some(0),
            1 => Some(positive),
            _ => None,
        };
        let (Some(x), Some(y)) = (code(dx, 1, 4), code(dy, 2, 8)) else {
            return Err(Error::InvalidDelta([dx, dy]));
        };
        self.window.cells[index].direction = DIRECTION_CODE[x | y];
        self.goal.examined = self.goal.examined.wrapping_add(1);
        Ok(())
    }
    /// 0x539EE0.
    pub(crate) fn visited(&self, index: usize) -> bool {
        self.window.cells[index].stamp == self.grid.epoch
    }
    /// 0x539F10 -> 0x54F370 on the cell's world tile.
    pub(crate) fn blocked(&mut self, index: usize) -> Result<bool> {
        let tile = self.window.tile(self.grid, index)?;
        self.world.tile_blocked(tile, self.context)
    }
    /// 0x53A660 with the LowPathingExaminedMax SVar.
    pub(crate) fn limit_reached(&self, entry: &QueueEntry) -> bool {
        entry.depth >= super::DEPTH_LIMIT || self.goal.examined >= self.settings.examined_max
    }
    /// 0x539F00.
    pub(crate) fn on_failed(&mut self) {
        self.goal.path_linked = false;
        self.goal.context_linked = false;
    }
    /// 0x53B720 OnSolveStarted.
    pub(crate) fn on_solve_started(&mut self) -> Result<bool> {
        let p = self.p;
        self.grid.advance_epoch();
        self.goal.direct = false;
        self.goal.examined = 0;
        // 0x53C170 always links the path and context before the search.
        if !self.window.contains(self.goal.start_bits, p)? {
            return Ok(false);
        }
        pop_front_kind0(self.path);
        if !self.adjust_goal()? {
            return Ok(false);
        }
        self.goal.goal_cell = self
            .window
            .world_to_cell(self.grid, self.goal.goal_bits, p)?;
        let [sx, sy] = self.goal.start_bits;
        let [gx, gy] = self.goal.goal_bits;
        let dy = f(gy)?.sub(&f(sy)?, p)?;
        let dx = f(gx)?.sub(&f(sx)?, p)?;
        let squared = dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?;
        if less(&squared, &f(EPSILON)?) {
            // "fixed destination too close to source - give up" (diagnostic).
            return Ok(false);
        }
        if self.goal.goal_cell == self.goal.start_cell {
            self.world
                .set_context_position(self.context, self.goal.start_bits)?;
            let mut direction = [st(&f(gx)?.sub(&f(sx)?, p)?)?, st(&f(gy)?.sub(&f(sy)?, p)?)?];
            normalize(&mut direction, p)?;
            set_facing(self.context, direction);
            if !self.reachable(self.goal.goal_bits)? {
                return Ok(false);
            }
            self.world
                .set_context_position(self.context, self.goal.start_bits)?;
            set_facing(self.context, direction);
            if self.world.sweep(self.context, self.goal.goal_bits)?.kind != 0 {
                return Ok(false);
            }
            self.goal.direct = true;
        }
        if let Some(actor) = source(self.context)
            && self.world.actor_virtual_b4(actor)?
        {
            self.world.refresh_overlaps(actor)?;
        }
        Ok(true)
    }
    /// 0x53B490: move the goal (in place) to a reachable point inside the
    /// window, stepping back towards the start one sub-cell at a time and
    /// trying to step past a blocking actor (0x53AB90).
    fn adjust_goal(&mut self) -> Result<bool> {
        let p = self.p;
        let start = self.goal.start_bits;
        if !self.window.contains(self.goal.goal_bits, p)? {
            let mut goal = self.goal.goal_bits;
            self.window.clip(start, &mut goal, p)?;
            self.goal.goal_bits = goal;
        }
        let dy_extended = f(self.goal.goal_bits[1])?.sub(&f(start[1])?, p)?;
        let mut step = [
            st(&f(self.goal.goal_bits[0])?.sub(&f(start[0])?, p)?)?,
            st(&dy_extended)?,
        ];
        let step_length = st(&f(self.grid.cell_size_bits)?.div(
            &Binary::from_i32(1i32.wrapping_shl(self.window.sub_shift & 31)),
            p,
        )?)?;
        let length = f(step[0])?
            .mul(&f(step[0])?, p)?
            .add(&dy_extended.mul(&dy_extended, p)?, p)?
            .sqrt(p)?;
        let mut remaining = st(&length)?;
        if length.compare(&f(ZERO)?) == Ordering::Greater {
            let scale = f(step_length)?.div(&f(remaining)?, p)?;
            step[0] = st(&f(step[0])?.mul(&scale, p)?)?;
            step[1] = st(&dy_extended.mul(&scale, p)?)?;
        }
        let mut success = true;
        loop {
            if self.reachable(self.goal.goal_bits)? {
                break;
            }
            self.world
                .set_context_position(self.context, self.goal.goal_bits)?;
            let hit = self.world.first_hit(self.context)?;
            if hit.kind == 2 {
                let mut moved = self.goal.goal_bits;
                let actor = self.world.resolve_actor(hit.reference);
                if self.displace(actor, start, &mut moved)? && self.reachable(moved)? {
                    self.goal.goal_bits = moved;
                    break;
                }
            }
            self.goal.goal_bits[0] = st(&f(self.goal.goal_bits[0])?.sub(&f(step[0])?, p)?)?;
            self.goal.goal_bits[1] = st(&f(self.goal.goal_bits[1])?.sub(&f(step[1])?, p)?)?;
            remaining = st(&f(remaining)?.sub(&f(step_length)?, p)?)?;
            if f(remaining)?.compare(&f(ZERO)?) != Ordering::Greater {
                success = false;
                break;
            }
        }
        let x = absolute(f(self.goal.goal_bits[0])?.sub(&f(start[0])?, p)?);
        let y = absolute(f(self.goal.goal_bits[1])?.sub(&f(start[1])?, p)?);
        if less(&y.add(&x, p)?, &f(GOAL_EPSILON)?) {
            return Ok(false);
        }
        Ok(success)
    }
    /// 0x53A6F0: inside the window and 0x4F3C40 reports a clear placement.
    fn reachable(&mut self, point: [u32; 2]) -> Result<bool> {
        if !self.window.contains(point, self.p)? {
            return Ok(false);
        }
        self.world.set_context_position(self.context, point)?;
        let source = source(self.context);
        Ok(self.world.placement_result(source, self.context)? == 0)
    }
    /// 0x53AB90: push `point` past a blocking actor along its facing normal.
    fn displace(
        &mut self,
        actor: Option<Object>,
        start: [u32; 2],
        point: &mut [u32; 2],
    ) -> Result<bool> {
        let p = self.p;
        let Some(actor) = actor else {
            return Ok(false);
        };
        if !self.world.actor_virtual_b4(actor)?
            || !self.world.actor_word_90_present(actor)?
            || self.world.actor_virtual_100(actor)? == 0
            || self.world.context_excludes(self.context, actor)?
        {
            return Ok(false);
        }
        let pose = self.world.actor_pose(actor)?;
        let [ax, ay] = pose.position_bits;
        let [fx, fy] = pose.facing_bits;
        let along = f(start[1])?
            .sub(&f(ay)?, p)?
            .mul(&f(fy)?, p)?
            .add(&f(start[0])?.sub(&f(ax)?, p)?.mul(&f(fx)?, p)?, p)?;
        let mut normal = [fx, fy];
        if less(&along, &f(ZERO)?) {
            normal = [
                st(&f(fx)?.mul(&f(NEGATIVE_ONE)?, p)?)?,
                st(&f(fy)?.mul(&f(NEGATIVE_ONE)?, p)?)?,
            ];
        }
        let reach = match source(self.context) {
            Some(source) => self.world.source_virtual_0(source)?,
            None => ZERO,
        };
        let pose = self.world.actor_pose(actor)?;
        let tangent = [pose.facing_bits[1] ^ 0x8000_0000, pose.facing_bits[0]];
        let shape: ActorShape = self.world.actor_shape(actor)?;
        let distance = f(shape.radius_bits())?
            .add(&f(reach)?, p)?
            .add(&f(EPSILON)?, p)?;
        let pose = self.world.actor_pose(actor)?;
        let y_offset = st(&f(normal[1])?.mul(&distance, p)?)?;
        let anchor = [
            st(&f(normal[0])?
                .mul(&distance, p)?
                .add(&f(pose.position_bits[0])?, p)?)?,
            st(&f(y_offset)?.add(&f(pose.position_bits[1])?, p)?)?,
        ];
        let gap = st(&line_distance(*point, anchor, tangent, p)?)?;
        let push = absolute(f(gap)?).add(&f(EPSILON)?, p)?;
        let y_push = st(&f(normal[1])?.mul(&push, p)?)?;
        point[0] = st(&f(normal[0])?.mul(&push, p)?.add(&f(point[0])?, p)?)?;
        point[1] = st(&f(y_push)?.add(&f(point[1])?, p)?)?;
        Ok(true)
    }
    /// 0x53AF70: neighbour line test between two cells; a full sweep below
    /// the accuracy depth, an end-point query otherwise. Byte +0x31 carries
    /// the ignore-moving flag during the query.
    pub(crate) fn line_clear(
        &mut self,
        from: [i32; 2],
        to: [i32; 2],
        ignore_moving: bool,
        accuracy: bool,
    ) -> Result<bool> {
        let p = self.p;
        let a = self.position(from)?;
        let b = self.position(to)?;
        if f(a[0])?.compare(&f(b[0])?) == Ordering::Equal
            && f(a[1])?.compare(&f(b[1])?) == Ordering::Equal
        {
            return Ok(true);
        }
        set_byte_31(self.context, u8::from(ignore_moving));
        let hit = if accuracy {
            self.world.set_context_position(self.context, a)?;
            let mut direction = [
                st(&f(b[0])?.sub(&f(a[0])?, p)?)?,
                st(&f(b[1])?.sub(&f(a[1])?, p)?)?,
            ];
            normalize(&mut direction, p)?;
            set_facing(self.context, direction);
            self.world.sweep(self.context, b)?
        } else {
            self.world.set_context_position(self.context, b)?;
            self.world.first_hit(self.context)?
        };
        set_byte_31(self.context, 0);
        Ok(hit.kind == 0)
    }
    /// 0x53B960 OnSolved: prepend kind-0 waypoints from `end` back to the
    /// start, one per change of normalized direction.
    pub(crate) fn on_solved(&mut self, end: [i32; 2]) -> Result<()> {
        let p = self.p;
        if self.goal.direct {
            return self.direct_path();
        }
        if end == self.goal.start_cell {
            return Ok(());
        }
        let mut previous = self.position(end)?;
        let mut last = [FLT_MAX, FLT_MAX];
        let mut cell = end;
        while cell != self.goal.start_cell {
            let direction = self.window.cells[self.window.cell_index(cell)?].direction;
            let delta = DIRECTIONS
                .get(direction as usize)
                .ok_or(Error::InvalidDirection(direction))?;
            cell = [
                cell[0].wrapping_sub(delta[0]),
                cell[1].wrapping_sub(delta[1]),
            ];
            let current = self.position(cell)?;
            let dy = f(previous[1])?.sub(&f(current[1])?, p)?;
            let dx = f(previous[0])?.sub(&f(current[0])?, p)?;
            let dx_stored = st(&dx)?;
            let length = dx
                .mul(&f(dx_stored)?, p)?
                .add(&dy.mul(&dy, p)?, p)?
                .sqrt(p)?;
            let heading = if length.compare(&f(EPSILON)?) != Ordering::Greater {
                [ZERO, ZERO]
            } else {
                let inverse = f(ONE)?.div(&length, p)?;
                [
                    st(&f(dx_stored)?.mul(&inverse, p)?)?,
                    st(&dy.mul(&inverse, p)?)?,
                ]
            };
            if f(heading[0])?.compare(&f(last[0])?) != Ordering::Equal
                || f(heading[1])?.compare(&f(last[1])?) != Ordering::Equal
            {
                push_front(self.path, node(previous, heading, 0))?;
                last = heading;
            }
            previous = current;
        }
        self.goal.path_linked = false;
        self.goal.context_linked = false;
        Ok(())
    }
    /// 0x53AE60: single waypoint at the exact goal for a direct line.
    fn direct_path(&mut self) -> Result<()> {
        let p = self.p;
        let [sx, sy] = self.goal.start_bits;
        let [gx, gy] = self.goal.goal_bits;
        let same = f(sx)?.compare(&f(gx)?) == Ordering::Equal
            && f(sy)?.compare(&f(gy)?) == Ordering::Equal;
        if !same {
            let mut heading = [st(&f(gx)?.sub(&f(sx)?, p)?)?, st(&f(gy)?.sub(&f(sy)?, p)?)?];
            if !normalize(&mut heading, p)?.is_zero() {
                push_front(self.path, node([gx, gy], heading, 0))?;
            }
        }
        self.goal.path_linked = false;
        self.goal.context_linked = false;
        Ok(())
    }
    /// Shared tile flag read of 0x53B3C0 (+0x20 bit 0 of the world tile).
    pub(crate) fn tile_open(&self, index: usize) -> Result<bool> {
        Ok(self.window.tile_flags(self.grid, index)? & 1 == 0)
    }
}

/// 0x53A930: distance from `b` to the line through `a` with direction `d`.
fn line_distance(a: [u32; 2], b: [u32; 2], d: [u32; 2], p: Precision) -> Result<Binary> {
    let x = f(b[0])?.sub(&f(a[0])?, p)?;
    let y = f(b[1])?.sub(&f(a[1])?, p)?;
    let t = x.mul(&f(d[0])?, p)?.add(&y.mul(&f(d[1])?, p)?, p)?;
    let ty = st(&t.mul(&f(d[1])?, p)?)?;
    let ex = st(&t.mul(&f(d[0])?, p)?.add(&f(a[0])?, p)?.sub(&f(b[0])?, p)?)?;
    let ey = f(ty)?.add(&f(a[1])?, p)?.sub(&f(b[1])?, p)?;
    Ok(ey.mul(&ey, p)?.add(&f(ex)?.mul(&f(ex)?, p)?, p)?.sqrt(p)?)
}
