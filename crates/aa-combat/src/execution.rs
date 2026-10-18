//! Native ability-use dispatcher4C9550 and area target traversal4C9C90.
//! Main/add damage call the real reconstructed hit calculation. Other effect
//! classes and projectile construction are required typed world operations.
use crate::{
    DamageRule, Error, Hit, Result, Weapon, ability::Ability, admission, performer::Context,
};
use aa_body::{F32, damage::WeaponPropagation};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

fn context_actor(context: &Context, world: &mut impl World) -> Result<Option<u32>> {
    match context.target {
        Some(target) if admission::World::is_actor(world, target)? => Ok(Some(target)),
        _ => Ok(None),
    }
}

/// Native57E030 performer delivery. The source impulse intentionally reads
/// definition+44 (authored impact_impulse), as the original instructions do.
/// The parsed recoil_impulse field+48 is not substituted as a design correction.
pub fn dispatch_from_performer<W: World>(
    performer: &mut crate::performer::Performer,
    handle: u32,
    ability: &mut Ability,
    context: &mut Context,
    world: &mut W,
    precision: Precision,
) -> Result<()> {
    let impact = world
        .execution_definition(ability.definition)?
        .impact_impulse;
    let _ = context_actor(context, world)?;
    if nonzero(impact)?
        && let Some(source) = context.source
        && world.actor_definition_flags168(source)? & 2 != 0
    {
        let source_position = admission::World::actor(world, source)?.position;
        let target = if context_actor(context, world)?.is_some() {
            let target = context_actor(context, world)?.ok_or(Error::Invalid(
                "target eligibility changed within impulse selection",
            ))?;
            admission::World::actor(world, target)?.position
        } else {
            context.target_position
        };
        world.apply_impulse(source, impulse(target, source_position, impact, precision)?)?;
    }
    let time = if performer.active_ability == Some(handle) {
        performer.started_at
    } else {
        admission::World::now(world)?
    };
    let _ = use_ability(handle, ability, context, time, world, precision)?;
    let cost = admission::World::definition(world, ability.definition)?.mana_cost;
    performer.mana = F32(n(performer.mana)?.sub(&n(cost)?, precision)?.store_f32()?);
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Definition {
    pub flags: u32,
    pub area: F32,
    pub volley_area: F32,
    pub projectile: Option<u32>,
    pub max_targets: i32,
    pub impact_impulse: F32,
    pub attenuation_minimum: F32,
    pub attenuation_factor: F32,
    pub impulse_area: F32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Effect {
    pub targets: admission::Filter,
    /// Present only for a source-verified DamageEffect implementation/vtable.
    pub damage: Option<DamageRule>,
}
pub trait World: admission::World + crate::World {
    fn execution_definition(&self, definition: u32) -> Result<Definition>;
    fn effect(&self, effect: u32) -> Result<Effect>;
    fn position_on_primary_island(&mut self, position: [F32; 2]) -> Result<bool>;
    fn clamp_map_position(&mut self, position: &mut [F32; 2]) -> Result<()>;
    fn ability_fx(
        &mut self,
        ability: &mut Ability,
        context: &mut Context,
        event: u32,
    ) -> Result<()>;
    fn pay_cost(&mut self, allegiance: u32, ability: &mut Ability) -> Result<()>;
    fn create_projectile(
        &mut self,
        ability: &mut Ability,
        context: &mut Context,
        definition: u32,
        precision: Precision,
    ) -> Result<Option<u32>>;
    fn begin_area_query(&mut self, center: [F32; 2], radius: F32) -> Result<u32>;
    fn next_area_target(&mut self, query: u32) -> Result<Option<u32>>;
    fn end_area_query(&mut self, query: u32) -> Result<()>;
    fn area_impulse(&mut self, position: [F32; 3], amount: F32, radius: F32) -> Result<()>;
    fn ability_is_structure(&self, definition: u32) -> Result<bool>;
    fn actor_definition_flags168(&self, actor: u32) -> Result<u32>;
    fn actor_radius(&mut self, actor: u32) -> Result<Binary>;
    fn apply_impulse(&mut self, actor: u32, impulse: [F32; 3]) -> Result<()>;
    /// Effects other than the verified DamageEffect must perform their original
    /// behavior here or return an error; no successful no-op fallback exists.
    /// Bind the ported Apply virtuals listed in [`crate::special_ops`] (morale,
    /// heal, enchantment, capture, steal resources, describe economy) and
    /// [`crate::summon::apply`].
    fn other_effect(
        &mut self,
        effect: u32,
        ability: &mut Ability,
        context: &mut Context,
        scale: F32,
        precision: Precision,
    ) -> Result<()>;
}
fn n(v: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(v.0)?)
}
fn positive(v: F32) -> Result<bool> {
    Ok(n(v)?.compare(&Binary::from_i32(0)) == Ordering::Greater)
}
fn nonzero(v: F32) -> Result<bool> {
    Ok(!n(v)?.is_zero())
}
fn squared(a: [F32; 2], b: [F32; 2], p: Precision) -> Result<Binary> {
    let dx = n(a[0])?.sub(&n(b[0])?, p)?;
    let dy = n(a[1])?.sub(&n(b[1])?, p)?;
    Ok(dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?)
}
fn impulse(from: [F32; 3], to: [F32; 3], amount: F32, p: Precision) -> Result<[F32; 3]> {
    let mut v = [F32::ZERO; 3];
    for i in 0..3 {
        v[i] = F32(n(to[i])?.sub(&n(from[i])?, p)?.store_f32()?);
    }
    let length = n(v[0])?
        .mul(&n(v[0])?, p)?
        .add(&n(v[1])?.mul(&n(v[1])?, p)?, p)?
        .add(&n(v[2])?.mul(&n(v[2])?, p)?, p)?
        .sqrt(p)?;
    if length.compare(&n(F32(0x38d1_b717))?) != Ordering::Greater {
        return Ok([F32::ZERO; 3]);
    }
    let reciprocal = Binary::from_u32(1).div(&length, p)?;
    for element in &mut v {
        *element = F32(n(*element)?.mul(&reciprocal, p)?.store_f32()?);
        *element = F32(n(*element)?.mul(&n(amount)?, p)?.store_f32()?);
    }
    Ok(v)
}
fn apply<W: World>(
    effect_id: u32,
    ability_handle: u32,
    ability: &mut Ability,
    context: &mut Context,
    scale: F32,
    world: &mut W,
    p: Precision,
) -> Result<()> {
    let effect = world.effect(effect_id)?;
    if let Some(rule) = effect.damage {
        let definition = world.execution_definition(ability.definition)?;
        let time = admission::World::now(world)?;
        crate::hit(
            Hit {
                rule,
                weapon: Some(Weapon {
                    id: ability_handle,
                    amount: ability.power,
                    flags: definition.flags,
                    propagation: WeaponPropagation {
                        definition_word38: definition.area,
                        definition_word4c: definition.volley_area,
                    },
                }),
                event_kind: context.event_kind,
                source_actor: context.source,
                allegiance_actor: context.allegiance,
                target_actor: context.target,
                scale,
                time,
            },
            world,
            p,
        )?;
    } else {
        world.other_effect(effect_id, ability, context, scale, p)?;
    }
    Ok(())
}
fn effect_accepts(effect: &Effect, context: &Context, world: &mut impl World) -> Result<bool> {
    let actor = match context.target {
        Some(t) if admission::World::is_actor(world, t)? => Some(t),
        _ => None,
    };
    let Some(actor) = actor else {
        return Ok(effect.targets.required == 0);
    };
    if !admission::World::is_actor(world, actor)? {
        return Ok(false);
    }
    admission::target_matches(effect.targets, context, actor, world)
}
/// Native linked mergesort: input was prepended, and the right half wins ties.
fn sort_targets(
    targets: &mut [u32],
    center: [F32; 2],
    world: &impl World,
    p: Precision,
) -> Result<()> {
    if targets.len() < 2 {
        return Ok(());
    }
    let mid = targets.len().div_ceil(2);
    let (left, right) = targets.split_at_mut(mid);
    sort_targets(left, center, world, p)?;
    sort_targets(right, center, world, p)?;
    let mut merged = Vec::new();
    merged
        .try_reserve_exact(targets.len())
        .map_err(|e| Error::World(e.to_string()))?;
    let (mut a, mut b) = (0, mid);
    while a < mid && b < targets.len() {
        let pa = admission::World::actor(world, targets[a])?.position;
        let pb = admission::World::actor(world, targets[b])?.position;
        if squared(center, [pa[0], pa[1]], p)?.compare(&squared(center, [pb[0], pb[1]], p)?)
            == Ordering::Less
        {
            merged.push(targets[a]);
            a += 1;
        } else {
            merged.push(targets[b]);
            b += 1;
        }
    }
    merged.extend_from_slice(&targets[a..mid]);
    merged.extend_from_slice(&targets[b..]);
    targets.copy_from_slice(&merged);
    Ok(())
}
/// Area iterator remains live across effect callbacks. Its end operation runs
/// on every Result exit, preserving primary/cleanup failures when both occur.
pub fn use_area<W: World>(
    handle: u32,
    ability: &mut Ability,
    context: &mut Context,
    radius: F32,
    time: F32,
    world: &mut W,
    p: Precision,
) -> Result<u32> {
    let center = [context.target_position[0], context.target_position[1]];
    let query = world.begin_area_query(center, radius)?;
    let result = (|| {
        let mut count = 0u32;
        let mut selected = Vec::new();
        loop {
            context.target = world.next_area_target(query)?;
            let Some(target) = context.target else {
                break;
            };
            let def = admission::World::definition(world, ability.definition)?;
            if admission::eligible(&def, context, true, world)? {
                if world.execution_definition(ability.definition)?.max_targets < 1 {
                    count =
                        count.wrapping_add(use_ability(handle, ability, context, time, world, p)?);
                } else {
                    selected
                        .try_reserve(1)
                        .map_err(|e| Error::World(e.to_string()))?;
                    selected.insert(0, target);
                }
            }
        }
        if world.execution_definition(ability.definition)?.max_targets > 0 {
            sort_targets(
                &mut selected,
                [context.target_position[0], context.target_position[1]],
                world,
                p,
            )?;
            let mut i = 0;
            while i < selected.len()
                && (i as i64)
                    < i64::from(world.execution_definition(ability.definition)?.max_targets)
            {
                context.target = Some(selected[i]);
                count = count.wrapping_add(use_ability(handle, ability, context, time, world, p)?);
                i += 1;
            }
        }
        Ok(count)
    })();
    let cleanup = world.end_area_query(query);
    match (result, cleanup) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(e),
        (Err(primary), Err(cleanup)) => Err(Error::World(format!(
            "{primary}; area query {query} cleanup also failed: {cleanup}"
        ))),
    }
}
/// Full finite4C9550 orchestration. Phase0 effects/payments happen before later
/// target/projectile failures, and last_use is the caller's captured start time.
pub fn use_ability<W: World>(
    handle: u32,
    ability: &mut Ability,
    context: &mut Context,
    time: F32,
    world: &mut W,
    p: Precision,
) -> Result<u32> {
    if ability.definition == 0 {
        return Ok(0);
    }
    let def = world.execution_definition(ability.definition)?;
    if def.flags & 0x400 != 0 {
        let position = match context.target {
            Some(t) => {
                let pos = admission::World::actor(world, t)?.position;
                [pos[0], pos[1]]
            }
            None => [context.target_position[0], context.target_position[1]],
        };
        if !world.position_on_primary_island(position)? {
            return Ok(0);
        }
    }
    let impact = def.impact_impulse;
    if context.phase == 0 {
        let source = context.source;
        if let Some(source) = source {
            let actor = admission::World::actor(world, source)?;
            context.source_position = [
                actor.position[0],
                actor.position[1],
                if actor.flags8 & 0x1000 != 0 {
                    actor.position[2]
                } else {
                    F32::ZERO
                },
            ];
        }
        let def = world.execution_definition(ability.definition)?;
        if def.flags & 0x10 != 0
            && (positive(def.area)? || positive(def.volley_area)? || def.flags & 0x20 != 0)
        {
            context.target_position = context.source_position;
            context.target = source;
        } else if let Some(target) = context.target {
            let actor = admission::World::actor(world, target)?;
            context.target_position = [
                actor.position[0],
                actor.position[1],
                if actor.flags8 & 0x1000 != 0 {
                    actor.position[2]
                } else {
                    F32::ZERO
                },
            ];
        }
        if let Some(source) = source
            && admission::World::is_kingdom(world, source)?
        {
            let mut point = [context.target_position[0], context.target_position[1]];
            world.clamp_map_position(&mut point)?;
            context.target_position[0] = point[0];
            context.target_position[1] = point[1];
            context.source_position = context.target_position;
        }
        world.ability_fx(ability, context, 1)?;
        ability.last_use = time;
        ability.use_count = ability.use_count.wrapping_add(1);
        for effect in admission::World::effects_for(world, ability.definition, context.source)? {
            if world.effect(effect)?.targets.required & 0x20 != 0 {
                let old = context.target;
                context.target = context.source;
                apply(effect, handle, ability, context, F32::ONE, world, p)?;
                context.target = old;
            }
        }
        if let Some(owner) = context.allegiance {
            world.pay_cost(owner, ability)?;
        }
    } else if admission::can_use(ability, context, true, world, p)? != 0 {
        return Ok(0);
    }
    let def = world.execution_definition(ability.definition)?;
    if (context.phase as i32) < 1 && positive(def.volley_area)? {
        context.phase = 1;
        return use_area(handle, ability, context, def.volley_area, time, world, p);
    }
    if (context.phase as i32) < 2
        && let Some(projectile) = world.execution_definition(ability.definition)?.projectile
    {
        return Ok(u32::from(
            world
                .create_projectile(ability, context, projectile, p)?
                .is_some(),
        ));
    }
    let def = world.execution_definition(ability.definition)?;
    if (context.phase as i32) < 3 && positive(def.area)? {
        context.phase = 3;
        let result = use_area(handle, ability, context, def.area, time, world, p)?;
        if nonzero(impact)? {
            let radius = world.execution_definition(ability.definition)?.impulse_area;
            world.area_impulse(context.target_position, impact, radius)?;
        }
        return Ok(result);
    }
    if context.target.is_none() && world.execution_definition(ability.definition)?.flags & 0x20 == 0
    {
        return Ok(0);
    }
    world.ability_fx(ability, context, 3)?;
    let target_actor = match context.target {
        Some(t) if admission::World::is_actor(world, t)? => Some(t),
        _ => None,
    };
    if nonzero(impact)?
        && !world.ability_is_structure(ability.definition)?
        && let Some(target) = target_actor
        && world.actor_definition_flags168(target)? & 4 != 0
    {
        let position = admission::World::actor(world, target)?.position;
        world.apply_impulse(
            target,
            impulse(context.source_position, position, impact, p)?,
        )?;
    }
    let mut scale = F32::ONE;
    let def = world.execution_definition(ability.definition)?;
    if context.phase == 3
        && n(def.attenuation_minimum)?.compare(&n(def.area)?) == Ordering::Less
        && n(def.attenuation_factor)?.compare(&n(F32(0x3f7f_f972))?) == Ordering::Less
    {
        let Some(target) = context.target else {
            return Ok(0);
        };
        let position = admission::World::actor(world, target)?.position;
        let distance = F32(squared(
            [context.target_position[0], context.target_position[1]],
            [position[0], position[1]],
            p,
        )?
        .sqrt(p)?
        .store_f32()?);
        let distance = n(distance)?.sub(&world.actor_radius(target)?, p)?;
        let distance = if distance.compare(&Binary::from_i32(0)) == Ordering::Less {
            Binary::from_i32(0)
        } else {
            distance
        };
        let def = world.execution_definition(ability.definition)?;
        let width = n(def.area)?.sub(&n(def.attenuation_minimum)?, p)?;
        if distance.compare(&n(def.attenuation_minimum)?) == Ordering::Greater
            && width.compare(&Binary::from_i32(0)) == Ordering::Greater
        {
            let ratio = distance
                .sub(&n(def.attenuation_minimum)?, p)?
                .div(&width, p)?;
            scale = F32(n(F32::ONE)?
                .sub(
                    &n(F32::ONE)?
                        .sub(&n(def.attenuation_factor)?, p)?
                        .mul(&ratio, p)?,
                    p,
                )?
                .store_f32()?);
            if n(scale)?.compare(&n(def.attenuation_factor)?) == Ordering::Less {
                scale = def.attenuation_factor;
            }
            if n(scale)?.compare(&n(F32::ONE)?) == Ordering::Greater {
                scale = F32::ONE;
            }
        }
    }
    for id in admission::World::effects_for(world, ability.definition, context.source)? {
        let effect = world.effect(id)?;
        if effect.targets.required & 0x20 == 0 && effect_accepts(&effect, context, world)? {
            apply(id, handle, ability, context, scale, world, p)?;
        }
    }
    Ok(1)
}
