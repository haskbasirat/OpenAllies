//! Original533C30 moving-target replan and its5317A0 projection for the
//! concrete Element Mind (virtual+A4 is52D780, testing native+184==2).
//! Mode2 formation placement remains an explicit unsupported boundary.
//! The shared Move payload is mutated
//! in place; no second route, guessed target center or clear-world callback.
use crate::{
    Arena, Error, Result, StateId,
    construct::{Installed, Kind},
    element_move::{self, Payload},
};
use aa_numeric::{Binary, Precision};
use aa_spatial::{
    collision::Shape,
    geometry::Placement,
    query::{Hit, PreparedQuery},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub position: [u32; 2],
    pub facing: [u32; 2],
    pub flags: u32,
    pub performer: Option<u32>,
}
pub trait World {
    fn resolve(&self, generation_id: u32) -> Result<Option<u32>>;
    fn actor(&self, actor: u32) -> Result<Actor>;
    /// Actual virtual+10 after the target transform snapshot. Ordinary source
    /// definitions are immutable; getter effects may change payload/Arena.
    fn soft_shape(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<Shape>;
    /// Native virtual0 retains the extended result.
    fn radius(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary>;
    /// Actual554350(actor,false), with no caller-added flags.
    fn actor_query(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<PreparedQuery>;
    /// Actual4CE6E0. Return the replacement's SpatialIndex ActorKey for the
    /// typed query, not a GTID or narrowed ActorStore ObjectKey.
    fn replacement_query_key(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<u32>;
    /// Actual552250 with source definition snapping through QueryPolicy.
    fn set_query_position(
        &mut self,
        query: &mut PreparedQuery,
        point: [u32; 2],
        arena: &Arena,
    ) -> Result<()>;
    /// Actual553BA0, normally aa_spatial::query::first_hit on the owned index.
    fn first_hit(&mut self, query: &PreparedQuery, p: &mut Payload, a: &mut Arena) -> Result<Hit>;
    /// Actual57EC80 / aa_combat::admission::maximum_range with live owned abilities.
    fn maximum_range(&mut self, performer: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary>;
    /// Native target virtual+18 is called even though its result is discarded.
    fn speed(&mut self, actor: u32, p: &mut Payload, a: &mut Arena) -> Result<Binary>;
}
fn err(message: impl std::fmt::Display) -> Error {
    Error::World(format!("moving target: {message}"))
}
fn n<T>(v: aa_numeric::Result<T>) -> Result<T> {
    v.map_err(err)
}
fn b(v: u32) -> Result<Binary> {
    n(Binary::from_f32_bits(v))
}
fn active(installed: &Installed, offset: u32, a: &Arena) -> Result<bool> {
    let state = *installed
        .state_bindings
        .get(&offset)
        .ok_or_else(|| err(format!("missing embedded state {offset:x}")))?;
    if a.state(state)?.parent != installed.mind_id {
        return Err(err("embedded state parent mismatch"));
    }
    Ok(a.mind(installed.mind_id)?.stack.front().copied().flatten() == Some(state))
}
fn owner(state: StateId, installed: &Installed, a: &Arena) -> Result<u32> {
    if a.state(state)?.parent != installed.mind_id {
        return Err(err("embedded Move parent changed"));
    }
    element_move::owner(state, a)
}
fn squared(delta: [Binary; 2], pc: Precision) -> Result<Binary> {
    n(n(delta[0].mul(&delta[0], pc))?.add(&n(delta[1].mul(&delta[1], pc))?, pc))
}
/// The radius-output form of5317A0 used by533C30. Installed definition/state
/// bindings and its+184 word must remain stable during this call; callbacks may
/// mutate the explicitly supplied Payload/Arena. The mode2 tail is rejected at
/// the actual branch after preceding query effects, never treated as false.
fn project(
    state: StateId,
    installed: &Installed,
    target: u32,
    p: &mut Payload,
    a: &mut Arena,
    pc: Precision,
    w: &mut impl World,
) -> Result<([u32; 2], u32)> {
    let transform = w.actor(target)?;
    let shape = w.soft_shape(target, p, a)?;
    if shape.kind > 3 {
        return Err(err("unsupported source soft shape kind"));
    }
    let mut radius = shape.radius_bits;
    let mut point = transform.position;
    if shape.kind >= 2 {
        let placement = Placement {
            shape: shape.clone(),
            facing: transform.facing,
            position: transform.position,
        };
        point = aa_combat::surface::placed_projection(
            &placement,
            w.actor(owner(state, installed, a)?)?.position,
            [0, 0],
            pc,
        )
        .map_err(err)?;
        let distance = n(squared(
            [
                n(b(transform.position[0])?.sub(&b(point[0])?, pc))?,
                n(b(transform.position[1])?.sub(&b(point[1])?, pc))?,
            ],
            pc,
        )?
        .store_f32())?;
        let join = active(installed, 0x164, a)?;
        let inner = n(b(shape.radius_bits)?.sub(&b(0x38d1b717)?, pc))?;
        if !join && !b(distance)?.compare(&n(inner.mul(&inner, pc))?).is_lt() {
            point = transform.position;
        } else {
            let mut query = w.actor_query(owner(state, installed, a)?, p, a)?;
            query.descriptor.ignore_moving = true;
            query.descriptor.ignore_actor = match w.resolve(p.plan.movement.target_id)? {
                Some(actor) => Some(w.replacement_query_key(actor, p, a)?),
                None => None,
            };
            w.set_query_position(&mut query, point, a)?;
            if w.first_hit(&query, p, a)?.kind != 0 {
                point = transform.position;
            } else {
                radius = 0;
            }
        }
    }
    if *installed
        .words
        .get(&0x184)
        .ok_or_else(|| err("missing Element Mind+184 mode"))?
        == 2
    {
        return Err(err(
            "Element mode2 formation placement45D4C0/556070 is not bound",
        ));
    }
    Ok((point, radius))
}
pub fn update(
    state: StateId,
    installed: &Installed,
    p: &mut Payload,
    a: &mut Arena,
    pc: Precision,
    w: &mut impl World,
) -> Result<Option<u32>> {
    if installed.kind != Kind::Element {
        return Err(err("requires concrete Element Mind"));
    }
    owner(state, installed, a)?;
    if installed.state_bindings.get(&0x7c) != Some(&state) {
        return Err(err("state is not embedded Move7C"));
    }
    let Some(target) = w.resolve(p.plan.movement.target_id)? else {
        return Ok(None);
    };
    let (point, target_radius) = project(state, installed, target, p, a, pc, w)?;
    let radius_actor = owner(state, installed, a)?;
    let position = w.actor(owner(state, installed, a)?)?.position;
    let distance = squared(
        [
            n(b(position[0])?.sub(&b(point[0])?, pc))?,
            n(b(position[1])?.sub(&b(point[1])?, pc))?,
        ],
        pc,
    )?;
    let distance = b(n(n(distance.sqrt(pc))?.store_f32())?)?;
    let radius = w.radius(radius_actor, p, a)?;
    let mut distance =
        n(n(n(distance.sub(&radius, pc))?.sub(&b(target_radius)?, pc))?.store_f32())?;
    if active(installed, 0xf8, a)? {
        let performer = w
            .actor(owner(state, installed, a)?)?
            .performer
            .ok_or_else(|| err("active Kill state requires Performer"))?;
        distance = n(n(b(distance)?.sub(&w.maximum_range(performer, p, a)?, pc))?.store_f32())?;
    }
    let live_target = w
        .resolve(p.plan.movement.target_id)?
        .ok_or_else(|| err("target retired before speed virtual"))?;
    w.speed(live_target, p, a)?;
    let previous = p.plan.previous_destination_bits;
    let moved = squared(
        [
            n(b(previous[0])?.sub(&b(point[0])?, pc))?,
            n(b(previous[1])?.sub(&b(point[1])?, pc))?,
        ],
        pc,
    )?;
    let threshold = if w.actor(target)?.flags & (1 << 29) == 0 {
        n(b(distance)?.mul(&b(0x3ec3f141)?, pc))?
    } else {
        b(0x38d1b717)?
    };
    if !moved.compare(&n(threshold.mul(&threshold, pc))?).is_lt() {
        element_move::clear_path(p);
        p.plan.movement.destination_bits = point;
        p.plan.attempts = 0;
        p.word78 = 0;
    }
    Ok(Some(target))
}
