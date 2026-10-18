//! Native projectile activation580BC0, scheduled update580920 and impact580660.
//! Collision/target surface, actor creation and movement remain real world
//! services. Arrival timing and damage delivery are implemented here.
use crate::{Error, Result, ability::Ability, admission, execution, performer::Context};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Projectile {
    pub owner: u32,
    pub source_id: u32,
    pub spawn_bone: u32,
    pub target_id: u32,
    pub origin: [F32; 3],
    pub target: [F32; 3],
    pub ability: Ability,
    pub remaining: F32,
    pub duration: F32,
    pub speed: F32,
    pub started_at: F32,
    pub event_kind: u32,
}
impl Projectile {
    ///580ED0/4C8610 defaults. speed is the authored actor definition+2B0.
    pub fn new(owner: u32, speed: F32) -> Self {
        Self {
            owner,
            source_id: 0,
            spawn_bone: 0,
            target_id: 0,
            origin: [F32::ZERO; 3],
            target: [F32::ZERO; 3],
            ability: Ability::new(0, None, F32::ZERO),
            remaining: F32::ZERO,
            duration: F32::ZERO,
            speed,
            started_at: F32::ZERO,
            event_kind: 0,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Definition {
    pub kind: u32,
    pub life: F32,
}
pub trait World: execution::World {
    fn activate_projectile(&mut self, state: &mut Projectile) -> Result<bool>;
    fn schedule_projectile(&mut self, state: &mut Projectile, event: i32) -> Result<()>;
    fn projectile_active(&self, state: &Projectile) -> Result<bool>;
    ///4DB0B0/4D7960 deactivation; actor storage remains valid during impact.
    /// 4D7960 is [`crate::begin_death::begin_death`].
    fn deactivate_projectile(&mut self, state: &mut Projectile) -> Result<()>;
    fn projectile_definition(&self, owner: u32) -> Result<Definition>;
    fn position_in_map(&mut self, position: [F32; 2]) -> Result<bool>;
    fn target_height_ratio(&self, arcing: bool) -> Result<F32>;
    fn actor_height(&mut self, actor: u32) -> Result<Binary>;
    fn actor_speed(&mut self, actor: u32) -> Result<Binary>;
    fn actor_facing(&self, actor: u32) -> Result<[F32; 3]>;
    ///Native4EEEE0 surface/height selection for a ground target.
    fn target_hit_position(&mut self, actor: u32, direction: [F32; 3]) -> Result<[F32; 3]>;
    fn move_projectile(&mut self, owner: u32, position: [F32; 3]) -> Result<()>;
    ///World arena identity for the embedded ability, not the source ability.
    fn projectile_ability_handle(&self, owner: u32) -> Result<u32>;
}
fn n(v: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(v.0)?)
}
pub(crate) fn normalized_xy(from: [F32; 3], to: [F32; 3], p: Precision) -> Result<[F32; 3]> {
    let dx = F32(n(to[0])?.sub(&n(from[0])?, p)?.store_f32()?);
    let dy = F32(n(to[1])?.sub(&n(from[1])?, p)?.store_f32()?);
    let length = n(dx)?
        .mul(&n(dx)?, p)?
        .add(&n(dy)?.mul(&n(dy)?, p)?, p)?
        .sqrt(p)?;
    if length.compare(&n(F32(0x38d1_b717))?) != Ordering::Greater {
        return Ok([F32::ZERO; 3]);
    }
    let reciprocal = Binary::from_u32(1).div(&length, p)?;
    Ok([
        F32(reciprocal.mul(&n(dx)?, p)?.store_f32()?),
        F32(reciprocal.mul(&n(dy)?, p)?.store_f32()?),
        F32::ZERO,
    ])
}
fn predict(
    state: &mut Projectile,
    target: u32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let distance = world.actor_speed(target)?.mul(&n(state.remaining)?, p)?;
    let facing = world.actor_facing(target)?;
    let x = distance.mul(&n(facing[0])?, p)?;
    let y = F32(distance.mul(&n(facing[1])?, p)?.store_f32()?);
    let z = F32(distance.mul(&n(facing[2])?, p)?.store_f32()?);
    state.target[0] = F32(x.add(&n(state.target[0])?, p)?.store_f32()?);
    state.target[1] = F32(n(y)?.add(&n(state.target[1])?, p)?.store_f32()?);
    state.target[2] = F32(n(z)?.add(&n(state.target[2])?, p)?.store_f32()?);
    Ok(())
}
///580BC0. The source ability is copied using4C8140: definition/power only,
/// local-definition pointer cleared; existing embedded counters stay untouched.
pub fn start(
    state: &mut Projectile,
    context: &Context,
    source_ability: Option<&Ability>,
    spawn_bone: u32,
    world: &mut impl World,
    p: Precision,
) -> Result<bool> {
    if !world.activate_projectile(state)? {
        return Ok(false);
    }
    world.schedule_projectile(state, 0)?;
    state.source_id = match context.source {
        Some(s) => crate::World::actor_id(world, s)?,
        None => 0,
    };
    state.spawn_bone = spawn_bone;
    state.origin = admission::World::actor(world, state.owner)?.position;
    state.target_id = match context.target {
        Some(t) => crate::World::actor_id(world, t)?,
        None => 0,
    };
    state.target = context.target_position;
    if !world.position_in_map([state.target[0], state.target[1]])? {
        let mut xy = [state.target[0], state.target[1]];
        world.clamp_map_position(&mut xy)?;
        state.target[0] = xy[0];
        state.target[1] = xy[1];
    }
    if let Some(ability) = source_ability {
        state.ability.local_definition = None;
        state.ability.definition = ability.definition;
        state.ability.power = ability.power;
    }
    let kind = world.projectile_definition(state.owner)?.kind;
    let ratio = world.target_height_ratio(kind != 0)?;
    if let Some(target) = admission::World::resolve_id(world, state.target_id)? {
        state.target[2] = F32(world
            .actor_height(target)?
            .mul(&n(ratio)?, p)?
            .add(&n(state.target[2])?, p)?
            .store_f32()?);
    }
    state.duration = F32::ZERO;
    if n(world.projectile_definition(state.owner)?.life)?.compare(&Binary::from_i32(0))
        == Ordering::Greater
    {
        state.duration = world.projectile_definition(state.owner)?.life;
    } else if kind == 0 || kind == 1 {
        let from = admission::World::actor(world, state.owner)?.position;
        let dx = n(state.target[0])?.sub(&n(from[0])?, p)?;
        let dy = n(state.target[1])?.sub(&n(from[1])?, p)?;
        let dz = n(state.target[2])?.sub(&n(from[2])?, p)?;
        //580DF5 adds Z squared before Y squared.
        state.duration = F32(dx
            .mul(&dx, p)?
            .add(&dz.mul(&dz, p)?, p)?
            .add(&dy.mul(&dy, p)?, p)?
            .sqrt(p)?
            .div(&n(state.speed)?, p)?
            .store_f32()?);
    } else if kind == 2 {
        let source = admission::World::resolve_id(world, state.source_id)?
            .ok_or(Error::Invalid("type2 projectile requires source actor"))?;
        let from = admission::World::actor(world, source)?.position;
        let dx = F32(n(state.target[0])?.sub(&n(from[0])?, p)?.store_f32()?);
        let dy = F32(n(state.target[1])?.sub(&n(from[1])?, p)?.store_f32()?);
        let length = F32(n(dx)?
            .mul(&n(dx)?, p)?
            .add(&n(dy)?.mul(&n(dy)?, p)?, p)?
            .sqrt(p)?
            .store_f32()?);
        let source = admission::World::resolve_id(world, state.source_id)?
            .ok_or(Error::Invalid("projectile source changed during lookup"))?;
        state.duration = F32(n(length)?
            .div(&world.actor_speed(source)?, p)?
            .store_f32()?);
    }
    state.remaining = state.duration;
    state.started_at = admission::World::now(world)?;
    if let Some(target) = admission::World::resolve_id(world, state.target_id)? {
        predict(state, target, world, p)?;
    }
    state.event_kind = context.event_kind;
    Ok(true)
}
///580660 deactivates before resolving source/allegiance and delivery. The
/// admitted phase2 ability uses its copied power and current world time.
pub fn impact<W: World>(state: &mut Projectile, world: &mut W, p: Precision) -> Result<()> {
    world.deactivate_projectile(state)?;
    let mut context = Context::new(state.event_kind);
    context.phase = 2;
    if let Some(source) = admission::World::resolve_id(world, state.source_id)?
        && admission::World::is_actor(world, source)?
    {
        context.source = Some(source);
    }
    context.allegiance = admission::World::allegiance(world, state.owner)?;
    context.word2c = state.owner;
    context.source_position = admission::World::actor(world, state.owner)?.position;
    context.target = admission::World::resolve_id(world, state.target_id)?;
    context.target_position = state.target;
    if admission::can_use(&mut state.ability, &mut context, true, world, p)? == 0 {
        let handle = world.projectile_ability_handle(state.owner)?;
        let time = admission::World::now(world)?;
        let _ = execution::use_ability(handle, &mut state.ability, &mut context, time, world, p)?;
    }
    Ok(())
}
/// Native580920. Arrival triggers when the extended remaining-time subtraction is
/// strictly negative; exactly zero moves to the endpoint and waits for another
/// update. Do not replace this with a <=0 cooldown test.
pub fn advance<W: World>(
    state: &mut Projectile,
    delta: F32,
    world: &mut W,
    p: Precision,
) -> Result<()> {
    if !world.projectile_active(state)? {
        return Ok(());
    }
    if let Some(target) = admission::World::resolve_id(world, state.target_id)? {
        let position = admission::World::actor(world, target)?.position;
        let direction = normalized_xy(state.origin, position, p)?;
        state.target = if admission::World::actor(world, target)?.flags8 & 0x1000 != 0 {
            admission::World::actor(world, target)?.position
        } else {
            world.target_hit_position(target, direction)?
        };
    }
    let remaining = n(state.remaining)?.sub(&n(delta)?, p)?;
    state.remaining = F32(remaining.store_f32()?);
    if remaining.compare(&Binary::from_i32(0)) == Ordering::Less {
        world.move_projectile(state.owner, state.target)?;
        impact(state, world, p)?;
        return Ok(());
    }
    if let Some(target) = admission::World::resolve_id(world, state.target_id)? {
        predict(state, target, world, p)?;
    }
    let ratio = n(state.remaining)?.div(&n(state.duration)?, p)?;
    let inverse = Binary::from_u32(1).sub(&ratio, p)?;
    let target_x = F32(inverse.mul(&n(state.target[0])?, p)?.store_f32()?);
    let target_y = F32(inverse.mul(&n(state.target[1])?, p)?.store_f32()?);
    let target_z = inverse.mul(&n(state.target[2])?, p)?;
    let source_x = ratio.mul(&n(state.origin[0])?, p)?;
    let source_y = ratio.mul(&n(state.origin[1])?, p)?;
    let source_z = F32(ratio.mul(&n(state.origin[2])?, p)?.store_f32()?);
    let position = [
        F32(source_x.add(&n(target_x)?, p)?.store_f32()?),
        F32(source_y.add(&n(target_y)?, p)?.store_f32()?),
        F32(n(source_z)?.add(&target_z, p)?.store_f32()?),
    ];
    world.move_projectile(state.owner, position)?;
    Ok(())
}
