//! Element virtual160 at530AE0, with actual535E00 path ownership and52CDB0
//! head removal. Query and lifecycle effects remain explicit live services.
use crate::{
    Arena, Error, MindId, Result, StateId,
    element_move::{Payload, owner, parent},
    element_move_continuation,
};
use aa_numeric::{Binary, Precision};
#[derive(Clone, Copy, Debug)]
pub struct Actor {
    pub position: [u32; 2],
    pub leader_mind: Option<MindId>,
}
pub trait World {
    type Query;
    fn actor(&self, actor: u32) -> Result<Actor>;
    fn world_step_bits(&self) -> Result<u32>;
    fn kind54(&self, mind: MindId) -> Result<u32>;
    fn leader_node_reached(
        &mut self,
        mind: MindId,
        actor: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    fn waypoint_offset(&mut self, state: StateId, p: &mut Payload, a: &mut Arena) -> Result<()>;
    ///Actual554350(actor,false); preserve effects before fresh position reads.
    fn begin_query(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<Self::Query>;
    fn query_position(
        &mut self,
        q: &mut Self::Query,
        position: [u32; 2],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    fn query_facing(
        &mut self,
        q: &mut Self::Query,
        facing: [u32; 2],
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    ///GTID resolution is direct here, unlike531170's4CE6E0 replacement.
    fn query_ignore(
        &mut self,
        q: &mut Self::Query,
        target_id: u32,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<()>;
    fn query_kind2_flags(&mut self, q: &mut Self::Query) -> Result<()>;
    fn first_hit(
        &mut self,
        q: &mut Self::Query,
        p: &mut Payload,
        a: &mut Arena,
    ) -> Result<[u32; 4]>;
    fn collision_response(&mut self, mind: MindId, p: &mut Payload, a: &mut Arena) -> Result<bool>;
    fn default_stance(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<()>;
    fn schedule_zero(&mut self, mind: MindId, p: &mut Payload, a: &mut Arena) -> Result<()>;
}
fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(format!("Element node reached: {e}"))
}
fn b(v: u32) -> Result<Binary> {
    Binary::from_f32_bits(v).map_err(fail)
}
fn n<T>(v: aa_numeric::Result<T>) -> Result<T> {
    v.map_err(fail)
}
fn head(p: &Payload) -> Result<aa_navigation::PathNode> {
    p.plan
        .movement
        .path
        .nodes
        .first()
        .copied()
        .ok_or_else(|| fail("path head removed"))
}
///52CDB0 decrements the head's typed count before deleting it, and marks dirty.
pub fn remove_head(p: &mut Payload) -> Result<()> {
    let kind = head(p)?.words[4];
    let path = &mut p.plan.movement.path;
    if kind == 1 {
        path.kind1_count = path.kind1_count.wrapping_sub(1);
    } else if kind == 2 {
        path.kind2_count = path.kind2_count.wrapping_sub(1);
    }
    path.nodes.remove(0);
    path.dirty = true;
    Ok(())
}
fn normalize(v: [u32; 2], pc: Precision) -> Result<[u32; 2]> {
    let x = b(v[0])?;
    let y = b(v[1])?;
    let length = n(n(n(x.mul(&x, pc))?.add(&n(y.mul(&y, pc))?, pc))?.sqrt(pc))?;
    if !length.compare(&b(0x38d1b717)?).is_gt() {
        return Ok([0, 0]);
    }
    let inverse = n(Binary::from_i32(1).div(&length, pc))?;
    Ok([
        n(n(inverse.mul(&x, pc))?.store_f32())?,
        n(n(inverse.mul(&y, pc))?.store_f32())?,
    ])
}
///Ordinary Element concrete vtable only. No cloned path survives callbacks.
pub fn apply(
    state: StateId,
    p: &mut Payload,
    a: &mut Arena,
    pc: Precision,
    w: &mut impl World,
) -> Result<bool> {
    if !p.plan.movement.path.nodes.is_empty() {
        let initial_zero = head(p)?.words[4] == 0;
        let mut direction = [0, 0];
        loop {
            if head(p)?.words[4] != 0 {
                let actor = owner(state, a)?;
                if let Some(leader) = w.actor(actor)?.leader_mind {
                    w.leader_node_reached(leader, actor, p, a)?;
                }
            }
            if p.plan.movement.path.nodes.is_empty() {
                break;
            }
            remove_head(p)?;
            w.waypoint_offset(state, p, a)?;
            if p.plan.movement.path.nodes.is_empty() {
                break;
            }
            if initial_zero {
                let actor = owner(state, a)?;
                let node = head(p)?;
                let step = n(n(
                    b(w.world_step_bits()?)?.mul(&b(p.plan.movement.requested_speed_bits)?, pc)
                )?
                .store_f32())?;
                let position = w.actor(actor)?.position;
                let y = n(b(position[1])?.sub(&b(node.words[1])?, pc))?;
                let x = n(b(position[0])?.sub(&b(node.words[0])?, pc))?;
                let distance = n(n(x.mul(&x, pc))?.add(&n(y.mul(&y, pc))?, pc))?;
                if distance.compare(&n(b(step)?.mul(&b(step)?, pc))?).is_lt() {
                    continue;
                }
            }
            let node = head(p)?;
            direction = [node.words[2], node.words[3]];
            let position = w.actor(owner(state, a)?)?.position;
            let node = head(p)?;
            let x = b(n(
                n(b(node.words[0])?.sub(&b(position[0])?, pc))?.store_f32()
            )?)?;
            let y = n(b(node.words[1])?.sub(&b(position[1])?, pc))?;
            let dot =
                n(n(b(direction[1])?.mul(&y, pc))?.add(&n(x.mul(&b(direction[0])?, pc))?, pc))?;
            if dot.compare(&Binary::from_i32(0)).is_gt() {
                break;
            }
        }
        if !p.plan.movement.path.nodes.is_empty() && initial_zero {
            let step = n(n(
                b(w.world_step_bits()?)?.mul(&b(p.plan.movement.requested_speed_bits)?, pc)
            )?
            .store_f32())?;
            let mut query = w.begin_query(owner(state, a)?, p, a)?;
            let position = w.actor(owner(state, a)?)?.position;
            //Both products remain extended through their additions here (unlike531170).
            let point = [
                n(
                    n(n(b(direction[0])?.mul(&b(step)?, pc))?.add(&b(position[0])?, pc))?
                        .store_f32(),
                )?,
                n(
                    n(n(b(direction[1])?.mul(&b(step)?, pc))?.add(&b(position[1])?, pc))?
                        .store_f32(),
                )?,
            ];
            w.query_position(&mut query, point, p, a)?;
            w.query_facing(&mut query, direction, p, a)?;
            w.query_ignore(&mut query, p.plan.movement.target_id, p, a)?;
            if w.kind54(parent(state, a)?)? == 2 {
                w.query_kind2_flags(&mut query)?;
            }
            let position = w.actor(owner(state, a)?)?.position;
            let vector = normalize(
                [
                    n(n(b(point[0])?.sub(&b(position[0])?, pc))?.store_f32())?,
                    n(n(b(point[1])?.sub(&b(position[1])?, pc))?.store_f32())?,
                ],
                pc,
            )?;
            let collision = w.first_hit(&mut query, p, a)?;
            //52FD50 performs a second normalization, preserving the original stores.
            element_move_continuation::save_collision(p, collision, vector, pc)?;
            if p.collision[0] != 0 {
                return w.collision_response(parent(state, a)?, p, a);
            }
        }
    }
    if p.plan.movement.path.nodes.is_empty() {
        w.default_stance(owner(state, a)?, p, a)?;
    }
    w.schedule_zero(parent(state, a)?, p, a)?;
    Ok(false)
}
