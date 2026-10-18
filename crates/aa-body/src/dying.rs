//! Original562380 corpse initialization, terrain adjustment and timer ordering.
//! Ability execution, ownership and world removal remain required world services.
use crate::{Body, Environment, Error, F32, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Definition {
    pub death_ability: Option<u32>, // actor definition+2A4
    pub rot_time: F32,              // +27C
    pub sink_time: F32,             // +280
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityInvocation {
    pub actor: u32,
    pub allegiance: Option<u32>,
    pub position: [F32; 3],
    /// Original action context kind=2; all other unrepresented slots are zero.
    pub kind: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    RemovedImmediately,
    ScheduledDecay,
}

pub trait World {
    /// Body virtual+38; do not substitute a guessed destructible/sink predicate.
    fn body_virtual38(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<bool, String>;
    fn definition(&self, actor: u32) -> std::result::Result<Definition, String>;
    /// Actual owner+8 flags, distinct from Environment.actor_flags at+100.
    fn actor_flags(&self, actor: u32) -> std::result::Result<u32, String>;
    ///4C8650 lifetime begins before building the action context. Native ability
    /// constructor/getter behavior (including any randomness) is the host's job.
    /// The world owns the handle until release succeeds. Failed construction
    /// must clean up its own partial allocation before returning an error.
    fn begin_ability(
        &mut self,
        definition: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<u32, String>;
    /// Actor virtual+108. Repeated calls cannot be cached across callbacks.
    fn allegiance(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<Option<u32>, String>;
    fn actor_modifier(&self, actor: u32) -> std::result::Result<Option<u32>, String>;
    fn invoke_ability(
        &mut self,
        ability: u32,
        modifier: u32,
        request: AbilityInvocation,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<(), String>;
    /// Called once on every Result exit after a successful begin, including a
    /// failed callback or invalid callback state. On failure the world retains
    /// ownership for explicit recovery; the reported error identifies the handle.
    fn release_ability(
        &mut self,
        ability: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<(), String>;
    ///4F0E50(actor.position,0x122), retaining the native extended return until
    /// this routine's explicit binary32 store. Both calls are made independently.
    fn terrain_height(
        &mut self,
        flags: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<Binary, String>;
    /// Actor virtual+8. Called once for eligibility and twice for the formula.
    fn actor_dimension(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<Binary, String>;
    fn schedule_decay(
        &mut self,
        delay: F32,
        argument: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<(), String>;
    fn remove_from_allegiance(
        &mut self,
        allegiance: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<(), String>;
    fn remove_actor(
        &mut self,
        argument: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<(), String>;
}

/// Finite valid-object562380 path. A scheduled decay is not actor destruction;
/// callbacks must supply actual ability/ownership/timer semantics. Errors are
/// nontransactional and do not authorize retrying or silently losing an ability.
pub fn start_dying(
    body: &mut Body,
    env: &mut Environment,
    world: &mut impl World,
    precision: Precision,
) -> Result<Outcome> {
    body.validate_fields(env)?;
    let retained = world.body_virtual38(body, env).map_err(Error::Callback)?;
    body.validate_fields(env)?;
    if !retained {
        world.remove_actor(0, body, env).map_err(Error::Callback)?;
        body.validate_fields(env)?;
        return Ok(Outcome::RemovedImmediately);
    }
    if let Some(definition) = world
        .definition(env.actor_id)
        .map_err(Error::Callback)?
        .death_ability
    {
        let ability = world
            .begin_ability(definition, body, env)
            .map_err(Error::Callback)?;
        let invoked = (|| -> Result<()> {
            body.validate_fields(env)?;
            let allegiance = world.allegiance(body, env).map_err(Error::Callback)?;
            body.validate_fields(env)?;
            let request = AbilityInvocation {
                actor: env.actor_id,
                allegiance,
                position: [env.owner_position[0], env.owner_position[1], F32::ZERO],
                kind: 2,
            };
            if let Some(modifier) = world
                .actor_modifier(env.actor_id)
                .map_err(Error::Callback)?
            {
                world
                    .invoke_ability(ability, modifier, request, body, env)
                    .map_err(Error::Callback)?;
                body.validate_fields(env)?;
            }
            Ok(())
        })();
        let released = world.release_ability(ability, body, env).map_err(|error| {
            Error::Callback(format!(
                "ability {ability} release failed; world retains ownership: {error}"
            ))
        });
        match (invoked, released) {
            (Err(primary), Err(cleanup)) => {
                return Err(Error::Callback(format!(
                    "{primary}; cleanup also failed: {cleanup}"
                )));
            }
            (Err(primary), Ok(())) => return Err(primary),
            (Ok(()), Err(cleanup)) => return Err(cleanup),
            (Ok(()), Ok(())) => {}
        }
        body.validate_fields(env)?;
    }
    body.dying_started = env.world_time;
    body.sink_duration = world
        .definition(env.actor_id)
        .map_err(Error::Callback)?
        .sink_time;
    body.sink_duration.number()?;
    if world.actor_flags(env.actor_id).map_err(Error::Callback)? & (1 << 21) != 0
        && body.sink_duration.compare(F32::ZERO)? == Ordering::Greater
    {
        let first = F32(world
            .terrain_height(0x122, body, env)
            .map_err(Error::Callback)?
            .store_f32()?);
        body.validate_fields(env)?;
        let second = F32(world
            .terrain_height(0x122, body, env)
            .map_err(Error::Callback)?
            .store_f32()?);
        body.validate_fields(env)?;
        if first.compare(second)? == Ordering::Greater {
            let dimension = world.actor_dimension(body, env).map_err(Error::Callback)?;
            body.validate_fields(env)?;
            if dimension.compare(&Binary::from_u32(0)) == Ordering::Greater {
                let dimension = world.actor_dimension(body, env).map_err(Error::Callback)?;
                body.validate_fields(env)?;
                let speed = F32(dimension
                    .div(&body.sink_duration.number()?, precision)?
                    .store_f32()?);
                let dimension = world.actor_dimension(body, env).map_err(Error::Callback)?;
                body.validate_fields(env)?;
                body.sink_duration = F32(dimension
                    .add(&first.number()?, precision)?
                    .sub(&second.number()?, precision)?
                    .div(&speed.number()?, precision)?
                    .store_f32()?);
            }
        }
    }
    let delay = world
        .definition(env.actor_id)
        .map_err(Error::Callback)?
        .rot_time
        .number()?
        .add(&body.sink_duration.number()?, precision)?;
    let ten = F32(0x4120_0000);
    let delay = if delay.compare(&ten.number()?) == Ordering::Greater {
        ten
    } else {
        F32(delay.store_f32()?)
    };
    world
        .schedule_decay(delay, 0, body, env)
        .map_err(Error::Callback)?;
    body.validate_fields(env)?;
    let allegiance = world.allegiance(body, env).map_err(Error::Callback)?;
    body.validate_fields(env)?;
    if allegiance.is_some() {
        let allegiance = world
            .allegiance(body, env)
            .map_err(Error::Callback)?
            .ok_or_else(|| {
                Error::Callback("allegiance disappeared between native virtual calls".into())
            })?;
        body.validate_fields(env)?;
        world
            .remove_from_allegiance(allegiance, body, env)
            .map_err(Error::Callback)?;
        body.validate_fields(env)?;
    }
    Ok(Outcome::ScheduledDecay)
}
