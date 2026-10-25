//! Regiment formations: slot positions around the anchor (the original
//! layouts and aa_movement::formation). Owner: movement.
//!
//! Plain `f32` versions of the original Organization.cpp / UAI.cpp formation
//! geometry, following the exact ports:
//! - offset rotation `0x578250` and ComputeSlotPosition `0x5797E0`
//!   (`aa_movement::formation::rotate_offset` / `compute_slot_position`);
//! - slot reassignment `0x57BA80` (`aa_movement::formation::reassign_slots`);
//! - the per-slot regiment path `0x52CBD0` with line intersection `0x445280`
//!   (`aa_navigation::company_move::offset_path_for_slot`);
//! - group offsets `0x4DE020` (the Game/group_formations.tgi positions turned
//!   by the group facing, moved back onto an open tile).
//!
//! Formation frame: X is to the right of the facing, Y is forward
//! (Organizations/Layouts/*.tgi `offset = x,y`).
use crate::path::{Grid, Waypoint};
use glam::Vec2;

/// Unit facing vector of an angle (0 = native +X).
pub fn facing_vector(angle: f32) -> Vec2 {
    let (s, c) = angle.sin_cos();
    Vec2::new(c, s)
}
/// Angle of a facing vector.
pub fn facing_angle(v: Vec2) -> f32 {
    v.y.atan2(v.x)
}

/// `0x578250`: formation offset (X right, Y forward) turned by the facing
/// `(fx, fy)`: `(fy*X + fx*Y, fy*Y - fx*X)`.
pub fn rotate_offset(offset: Vec2, facing: Vec2) -> Vec2 {
    Vec2::new(facing.y * offset.x + facing.x * offset.y, facing.y * offset.y - facing.x * offset.x)
}
/// Inverse of [`rotate_offset`] (the `A`/`B` terms of `0x52CBD0`/`0x52E400`).
pub fn unrotate_offset(world: Vec2, facing: Vec2) -> Vec2 {
    Vec2::new(world.x * facing.y - world.y * facing.x, world.y * facing.y + world.x * facing.x)
}

/// World position of slot `offset` for an organization at `anchor` facing `heading`.
pub fn slot_position(anchor: Vec2, heading: f32, offset: [f32; 2]) -> Vec2 {
    anchor + rotate_offset(Vec2::from(offset), facing_vector(heading))
}

/// `0x445280`: intersection of line p1->p2 with line p3->p4; None when parallel.
pub fn line_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let d1 = p2 - p1;
    let d2 = p4 - p3;
    let denominator = d2.y * d1.x - d1.y * d2.x;
    if denominator == 0.0 {
        return None;
    }
    let e = p1.y - p3.y;
    let g = d2.y * (p1.x - p3.x);
    let t = (d2.x * e - g) / denominator;
    Some(p1 + d1 * t)
}

/// `0x52CBD0`: the regiment path offset for one slot. Every node moves by the
/// formation offset turned to that node's leg direction; interior nodes then
/// become the intersection of the two offset legs (parallel legs keep the
/// plain offset point). The node directions are unchanged.
pub fn offset_path(path: &[Waypoint], formation_offset: Vec2) -> Vec<Waypoint> {
    let mut out: Vec<Waypoint> = path.to_vec();
    let Some(last) = path.last() else {
        return out;
    };
    let n = out.len();
    let mut previous_dir = last.dir;
    out[n - 1].pos = last.pos + rotate_offset(formation_offset, previous_dir);
    for i in (0..n - 1).rev() {
        let previous_point = out[i + 1].pos;
        let dir = out[i].dir;
        let point = path[i].pos + rotate_offset(formation_offset, dir);
        out[i].pos = point;
        if let Some(p) = line_intersection(point, point + dir, previous_point, previous_point + previous_dir) {
            out[i].pos = p;
        }
        previous_dir = dir;
    }
    out
}

/// The simulation thread's x87 precision control (PC53).
const PC: aa_numeric::Precision = aa_numeric::Precision::Double53;

/// A member standing in a slot, for [`organization_slots`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotMember {
    pub actor: u32,
    pub pos: Vec2,
}

/// The organization slot table (+0x78) of an oriented formation, exactly
/// (`aa_movement::formation` at the simulation's PC53): ComputeSlotPosition
/// `0x5797E0` for every slot (zero for the leader's and the primary slot,
/// else the offset turned by `0x578250`), then with `reassign` the
/// reassignment `0x57BA80` (together `0x57BEF0`). `members[i]` stands in
/// slot i; `leader` is the leader's actor and slot. Inputs the native code
/// refuses (a non-finite value, slots outside the arrays) get the plain
/// `f32` table.
#[allow(clippy::too_many_arguments)]
pub fn organization_slots(offsets: &[[f32; 2]], facing: Vec2, center: Vec2, members: &[Option<SlotMember>], leader: Option<(u32, usize)>, primary_slot: Option<usize>, groups: &[usize], reassign: bool) -> Vec<Vec2> {
    use aa_movement::formation as native;
    struct NoDraws;
    impl native::FormationWorld for NoDraws {
        // Only an empty slot of a formation that is not oriented draws.
        fn next_random_u32(&mut self) -> aa_movement::Result<u32> {
            Err(aa_movement::Error::World("an oriented formation does not draw".into()))
        }
        fn mind_leader_changed(&mut self) -> aa_movement::Result<()> {
            Ok(())
        }
        fn detached_member(&mut self, _: u32, _: u32, _: [f32; 2]) -> aa_movement::Result<()> {
            Ok(())
        }
    }
    let n = offsets.len();
    let member = |slot: usize, m: &SlotMember| native::Member { id: m.actor, position: [m.pos.x, m.pos.y], slot: slot as i32, attached: true };
    let mut organization = native::Organization {
        owner_id: 0,
        facing: [facing.x, facing.y],
        owner_has_mind: false,
        center: [center.x, center.y],
        primary_slot: primary_slot.map_or(-1, |p| p as i32),
        group_sizes: groups.iter().map(|&g| g as i32).collect(),
        slot_count: n as i32,
        members: (0..n).map(|i| members.get(i).copied().flatten().map(|m| member(i, &m))).collect(),
        leader: leader.map(|(actor, slot)| {
            let pos = members.get(slot).copied().flatten().map_or(center, |m| m.pos);
            member(slot, &SlotMember { actor, pos })
        }),
        formation: Some(native::FormationShape { oriented: true, offsets: offsets.to_vec() }),
        slot_positions: vec![[0.0; 2]; n],
    };
    let result = if reassign {
        native::compute_all_slot_positions(&mut organization, PC, &mut NoDraws).map(|_| ())
    } else {
        (0..n as i32).try_for_each(|slot| native::compute_slot_position(&mut organization, slot, PC, &mut NoDraws).map(|_| ()))
    };
    match result {
        Ok(()) => organization.slot_positions.iter().map(|&p| Vec2::from(p)).collect(),
        Err(_) => {
            let leader_slot = leader.map(|l| l.1);
            let mut slots = compute_slot_positions(offsets, facing, leader_slot, primary_slot);
            if reassign {
                let positions: Vec<Option<Vec2>> = (0..n).map(|i| members.get(i).copied().flatten().map(|m| m.pos)).collect();
                reassign_slots(center, &mut slots, &positions, leader_slot, groups);
            }
            slots
        }
    }
}

/// `0x52CBD0` exactly (`aa_navigation::company_move::offset_path_for_slot`
/// at PC53): the regiment path offset for the slot whose world-frame
/// offset (+0x78) is `slot_offset` under the organization `facing`. Inputs
/// the native code refuses get the plain [`offset_path`].
pub fn slot_path(path: &[Waypoint], slot_offset: Vec2, facing: Vec2) -> Vec<Waypoint> {
    use aa_navigation::{OwnedPath, PathNode, company_move};
    let bits = |v: Vec2| [v.x.to_bits(), v.y.to_bits()];
    let node = |w: &Waypoint| {
        let [x, y] = bits(w.pos);
        let [dx, dy] = bits(w.dir);
        PathNode { words: [x, y, dx, dy, 1, 0, x, y] }
    };
    let source = OwnedPath { nodes: path.iter().map(node).collect(), kind1_count: path.len() as i32, kind2_count: 0, dirty: false, word18: u32::MAX };
    let mut target = source.clone();
    match company_move::offset_path_for_slot(&mut target, bits(slot_offset), bits(facing), &source, PC) {
        Ok(()) => target.nodes.iter().zip(path).map(|(n, w)| Waypoint { pos: Vec2::new(f32::from_bits(n.words[0]), f32::from_bits(n.words[1])), dir: w.dir }).collect(),
        Err(_) => offset_path(path, unrotate_offset(slot_offset, facing)),
    }
}

/// Slot offsets (organization +0x78) for a facing: zero for the leader's slot
/// and the primary slot, otherwise the oriented formation offset turned by
/// the facing (`0x5797E0` for an oriented formation), in plain `f32`.
pub fn compute_slot_positions(offsets: &[[f32; 2]], facing: Vec2, leader_slot: Option<usize>, primary_slot: Option<usize>) -> Vec<Vec2> {
    offsets
        .iter()
        .enumerate()
        .map(|(i, &o)| if Some(i) == leader_slot || Some(i) == primary_slot { Vec2::ZERO } else { rotate_offset(Vec2::from(o), facing) })
        .collect()
}

fn wrapping_factorial(count: i32) -> i32 {
    let mut value: i32 = 1;
    let mut factor = count;
    while factor > 1 {
        value = value.wrapping_mul(factor);
        factor -= 1;
    }
    value
}

/// `0x57BA80`: within each layout slot group of more than one slot, swap
/// slot positions pairwise while the total squared member-to-slot distance
/// drops by more than 1e-4 (the last improving partner wins, as natively).
/// `members[i]` is the position of the member in slot i (None = empty slot,
/// distance 0). The leader's slot never moves.
pub fn reassign_slots(center: Vec2, slots: &mut [Vec2], members: &[Option<Vec2>], leader_slot: Option<usize>, groups: &[usize]) {
    const TOLERANCE: f32 = f32::from_bits(0x38d1_b717);
    let mut start = 0usize;
    for &count in groups {
        let end = start + count;
        if count > 1 && end <= slots.len() && end <= members.len() {
            let size = count;
            let max_swaps = wrapping_factorial(count as i32);
            let mut matrix = vec![0f32; size * size];
            let mut total = 0f32;
            for i in 0..size {
                for j in 0..size {
                    let distance = members[start + i].map_or(0.0, |m| (center + slots[start + j] - m).length_squared());
                    matrix[j * size + i] = distance;
                    if i == j {
                        total += distance;
                    }
                }
            }
            let mut pending = vec![true; size];
            if let Some(l) = leader_slot.filter(|&l| l >= start && l < end) {
                pending[l - start] = false;
            }
            let mut swaps: i32 = 0;
            while let Some(i) = pending.iter().rposition(|p| *p) {
                let mut best = None;
                let mut best_value = f32::MAX;
                for c in 0..size {
                    if Some(start + c) == leader_slot {
                        continue;
                    }
                    let candidate = total - matrix[i * size + i] - matrix[c * size + c] + matrix[i * size + c] + matrix[c * size + i];
                    if TOLERANCE + candidate < total {
                        best_value = candidate;
                        best = Some(c);
                    }
                }
                match best {
                    Some(c) if c != i => {
                        total = best_value;
                        slots.swap(start + c, start + i);
                        for k in 0..size {
                            matrix.swap(c * size + k, i * size + k);
                        }
                        pending[c] = false;
                        let previous = swaps;
                        swaps = swaps.wrapping_add(1);
                        if previous >= max_swaps {
                            break;
                        }
                    }
                    _ => pending[i] = false,
                }
            }
        }
        start = end;
    }
}

/// Data/Game/group_formations.tgi `group_defensive`: the group formation of
/// the default regiment formation `regiment_defensive`
/// (Organizations/Formations/regiment_formations.tgi `group_formation_ids`).
pub const GROUP_DEFENSIVE: [[f32; 2]; 6] = [[6.5, 10.0], [-6.5, 10.0], [6.5, 0.0], [-6.5, 0.0], [6.5, -10.0], [-6.5, -10.0]];
/// `group_aggressive` (formation `regiment_aggressive`).
pub const GROUP_AGGRESSIVE: [[f32; 2]; 6] = [[0.0, 6.0], [-13.5, 6.0], [13.5, 6.0], [0.0, -6.0], [-13.5, -6.0], [13.5, -6.0]];
/// `group_column` (formation `regiment_column`).
pub const GROUP_COLUMN: [[f32; 2]; 6] = [[0.0, 8.0], [-9.0, 8.0], [9.0, 8.0], [0.0, -8.0], [-9.0, -8.0], [9.0, -8.0]];

/// `0x4DE020`: a group member's offset from the group destination (the group
/// formation position already turned by the group facing); when that point
/// is on a blocked tile, the first open tile on the line back to the
/// destination (its centre) is used instead.
pub fn group_offset(grid: &Grid, destination: Vec2, offset: Vec2) -> Vec2 {
    let point = destination + offset;
    if !grid.blocked_at(point) {
        return offset;
    }
    // Walk the tiles from the offset point back toward the destination.
    for cell in grid.walk(point, destination) {
        if !grid.blocked(cell) {
            return grid.center(cell) - destination;
        }
    }
    Vec2::ZERO
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_matches_original_frame() {
        // Facing +Y: X stays right (+X), Y forward (+Y).
        assert_eq!(rotate_offset(Vec2::new(2.0, 3.0), Vec2::new(0.0, 1.0)), Vec2::new(2.0, 3.0));
        // Facing +X: forward is +X, right is -Y.
        assert_eq!(rotate_offset(Vec2::new(2.0, 3.0), Vec2::new(1.0, 0.0)), Vec2::new(3.0, -2.0));
        let v = Vec2::new(0.6, 0.8);
        let w = rotate_offset(Vec2::new(-1.5, 4.0), v);
        assert!((unrotate_offset(w, v) - Vec2::new(-1.5, 4.0)).length() < 1e-5);
    }

    #[test]
    fn offset_path_keeps_parallel_legs() {
        // An L-shaped path east then north; slot 2 m to the right.
        let path = [
            Waypoint { pos: Vec2::new(10.0, 0.0), dir: Vec2::new(1.0, 0.0) },
            Waypoint { pos: Vec2::new(10.0, 10.0), dir: Vec2::new(0.0, 1.0) },
        ];
        let out = offset_path(&path, Vec2::new(2.0, 0.0));
        // Last node: right of north is +X.
        assert_eq!(out[1].pos, Vec2::new(12.0, 10.0));
        // Corner: intersection of y = -2 (right of east) and x = 12.
        assert!((out[0].pos - Vec2::new(12.0, -2.0)).length() < 1e-5, "{:?}", out[0].pos);
    }

    #[test]
    fn exact_slot_table_and_slot_path_agree_with_the_plain_versions() {
        let offsets = [[-3.0, 2.5], [0.0, 3.25], [2.75, 2.75], [0.25, -3.0], [-3.25, -2.75], [3.25, -2.75], [0.0, 0.0]];
        let facing = Vec2::new(0.6, 0.8);
        let center = Vec2::new(100.0, 50.0);
        // Members of slots 1 and 2 stand at each other's slot.
        let mut members: Vec<Option<SlotMember>> = (0..offsets.len()).map(|i| Some(SlotMember { actor: i as u32 + 10, pos: center + rotate_offset(Vec2::from(offsets[i]), facing) })).collect();
        members.swap(1, 2);
        members[1].as_mut().unwrap().actor = 11;
        members[2].as_mut().unwrap().actor = 12;
        let groups = [3, 3, 1];
        let exact = organization_slots(&offsets, facing, center, &members, Some((16, 6)), Some(6), &groups, true);
        let mut plain = compute_slot_positions(&offsets, facing, Some(6), Some(6));
        let positions: Vec<Option<Vec2>> = members.iter().map(|m| m.map(|m| m.pos)).collect();
        reassign_slots(center, &mut plain, &positions, Some(6), &groups);
        for (e, p) in exact.iter().zip(&plain) {
            assert!((*e - *p).length() < 1e-5, "{e:?} vs {p:?}");
        }
        // The swap happened: slot 1 now holds slot 2's offset.
        assert!((exact[1] - rotate_offset(Vec2::from(offsets[2]), facing)).length() < 1e-5);
        assert_eq!(exact[6], Vec2::ZERO);
        let path = [
            Waypoint { pos: Vec2::new(10.0, 0.0), dir: Vec2::new(1.0, 0.0) },
            Waypoint { pos: Vec2::new(10.0, 10.0), dir: Vec2::new(0.0, 1.0) },
            Waypoint { pos: Vec2::new(30.0, 30.0), dir: Vec2::new(0.70710677, 0.70710677) },
        ];
        let offset = rotate_offset(Vec2::new(2.0, -1.0), facing);
        let exact = slot_path(&path, offset, facing);
        let plain = offset_path(&path, unrotate_offset(offset, facing));
        for (e, p) in exact.iter().zip(&plain) {
            assert!((e.pos - p.pos).length() < 1e-4, "{:?} vs {:?}", e.pos, p.pos);
            assert_eq!(e.dir, p.dir);
        }
    }

    #[test]
    fn reassignment_swaps_to_nearer_slots() {
        let center = Vec2::ZERO;
        let mut slots = vec![Vec2::new(-3.0, 0.0), Vec2::new(3.0, 0.0), Vec2::ZERO];
        // Member of slot 0 stands right, member of slot 1 stands left.
        let members = [Some(Vec2::new(3.0, 0.0)), Some(Vec2::new(-3.0, 0.0)), Some(Vec2::ZERO)];
        reassign_slots(center, &mut slots, &members, Some(2), &[2, 1]);
        assert_eq!(slots[0], Vec2::new(3.0, 0.0));
        assert_eq!(slots[1], Vec2::new(-3.0, 0.0));
        assert_eq!(slots[2], Vec2::ZERO);
    }
}
