//! Original regiment (company) Mind virtual +14C `51F7A0(leader)`: the
//! organization's leader reached a node of its own path, and the regiment
//! path follows it.
//!
//! Order of the original:
//! 1. The state at Mind+1CC is the current state (`52A000`): nothing.
//! 2. The leader (organization +7C component +40) is not the caller's actor:
//!    nothing. The state at Mind+34 is current: nothing.
//! 3. The regiment path (Mind virtual +44) is empty: nothing. The leader's
//!    path (leader +70 virtual +44) has more nodes (`51E8D0`) than the
//!    regiment path's kind-1 count (+C): nothing.
//! 4. `52CDB0` removes the regiment path's head. A next head turns the
//!    formation toward its point (`57BF30(node +8, node +C)` on the
//!    organization component).
//! 5. With the path now empty and the move state (Mind+68) active (status
//!    Mind+74 == 2), arrival within 0.5 of the move goal (state +14/+18,
//!    `51D7D0` -> `52A1F0`) returns Mind virtual +144's result.
//!
//! Every other exit returns false.
use crate::{OwnedPath, PathNode, Result, company_move::arrived};
use aa_numeric::Precision;

/// Regiment Mind offsets of the states `51F7A0` asks about.
pub const FIRST_STATE: u32 = 0x1cc;
pub const SECOND_STATE: u32 = 0x34;

/// Services of `51F7A0`, called in native order. The host is bound to one
/// regiment Mind.
pub trait LeaderNodeWorld {
    /// `52A000` on the state at Mind+`offset`: it is the current state.
    fn state_is_current(&mut self, offset: u32) -> Result<bool>;
    /// Organization (Mind+4) +7C component +40: the leader.
    fn leader(&mut self) -> Result<u32>;
    /// `51E8D0` on the leader's path (leader +70 virtual +44): its node
    /// count.
    fn leader_path_nodes(&mut self, leader: u32) -> Result<i32>;
    /// `57BF30(x, y)` on the organization component.
    fn face(&mut self, point: [u32; 2]) -> Result<()>;
    /// Mind+74 (status of the move state at Mind+68).
    fn move_status(&mut self) -> Result<u32>;
    /// Move state +14/+18 (binary32).
    fn move_goal(&mut self) -> Result<[u32; 2]>;
    /// The organization (Mind+4) +20/+24 (binary32).
    fn organization_position(&mut self) -> Result<[u32; 2]>;
    /// Mind virtual +144 and its result.
    fn mind_virtual_144(&mut self) -> Result<bool>;
}

/// `52CDB0`: the head's kind count (+C for kind 1, +10 for kind 2) is
/// decremented, the head is unlinked and freed, and the path is marked
/// dirty (also when it was empty).
pub fn remove_head(path: &mut OwnedPath) {
    if let Some(head) = path.nodes.first().copied() {
        match head.words[4] {
            1 => path.kind1_count = path.kind1_count.wrapping_sub(1),
            2 => path.kind2_count = path.kind2_count.wrapping_sub(1),
            _ => {}
        }
        path.nodes.remove(0);
    }
    path.dirty = true;
}

/// Point (+8/+C) of a path node.
fn point(node: &PathNode) -> [u32; 2] {
    [node.words[2], node.words[3]]
}

/// Complete `51F7A0(leader)`; `path` is the regiment path (Mind virtual
/// +44). Returns the Mind virtual +144 result on arrival, otherwise false.
pub fn leader_node_reached(
    leader: u32,
    path: &mut OwnedPath,
    world: &mut impl LeaderNodeWorld,
    p: Precision,
) -> Result<bool> {
    if world.state_is_current(FIRST_STATE)? || world.leader()? != leader {
        return Ok(false);
    }
    if world.state_is_current(SECOND_STATE)? || path.nodes.is_empty() {
        return Ok(false);
    }
    let count = path.kind1_count;
    if world.leader_path_nodes(leader)? > count {
        return Ok(false);
    }
    remove_head(path);
    if let Some(head) = path.nodes.first() {
        world.face(point(head))?;
    }
    if path.nodes.is_empty()
        && world.move_status()? == 2
        && arrived(world.move_goal()?, world.organization_position()?, p)?
    {
        return world.mind_virtual_144();
    }
    Ok(false)
}
