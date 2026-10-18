//! SummonEffect Apply 4CCA70 and its creators: SummonObject 4CC2C0,
//! SummonActor 4CC470 and SummonTransportedActor 4CB110, with the summoned
//! GTID list 4C89A0 and the FX event-4 helper 4CA7B0. World creation,
//! placement, commands, FX rendering and messages are explicit seams.
//! Proven by research/phase8-combat/native_summon.py.
use crate::{Result, ability::Ability, performer::Context, special_ops::EffectTarget};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn store(value: Binary) -> Result<F32> {
    Ok(F32(value.store_f32()?))
}

/// SummonEffect fields read at Apply time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SummonEffect {
    /// +14 object (ThingData).
    pub object: u32,
    /// +18 dynamic-campaign object.
    pub dynamic_object: u32,
    /// +24 property applied to summoned actors (4CEF10 kind 3).
    pub property: Option<u32>,
    /// +28.
    pub controllable: bool,
}

/// Object creation descriptor (vtable A25B1C): +4 definition, +8..+10
/// position, +14/+18 direction; +1C empty name, +20 -1, +24 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectDescriptor {
    pub definition: u32,
    pub position: [F32; 3],
    pub direction: [F32; 2],
}
/// Actor creation descriptor (vtable A25B28, the layout of
/// `regeneration::CreationDescriptor`): the object part, +28 allegiance,
/// +34 -1, +51 byte flag; other words zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorDescriptor {
    pub object: ObjectDescriptor,
    pub allegiance: Option<u32>,
    pub flag51: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Descriptor {
    Object(ObjectDescriptor),
    Actor(ActorDescriptor),
}
/// Creation entry points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Creator {
    /// 4F9C10 (objects and transports).
    Generic,
    /// 4FA030 (actors).
    Actor,
    /// 4FA150 (actors with an OrganizationComponent, ActorData+174 bit 7).
    Organization,
}

pub trait SummonWorld: EffectTarget {
    /// AbilityDefinition `+54` flags of the ability's definition.
    fn definition_flags(&self, definition: u32) -> Result<u32>;
    /// `*[B39AB0]+128` byte (dynamic campaign).
    fn campaign_mode(&self) -> Result<bool>;
    /// Actor `+20` position, `+2C/+30` facing and `[+4]+54` extent.
    fn actor_position(&self, actor: u32) -> Result<[F32; 3]>;
    fn actor_facing(&self, actor: u32) -> Result<[F32; 2]>;
    fn actor_extent(&self, actor: u32) -> Result<F32>;
    /// 4F02B0 (world RNG `+D4`) then 444A90: `[B39AF8]` direction table
    /// entry at `rng.next() % count`. Bind `aa_movement::formation::random_direction`
    /// with a draw from the GWorld+0xD4 RNG (aa-runtime simulation).
    fn random_direction(&mut self) -> Result<[F32; 2]>;
    /// ThingData virtual `+38`, `+1C` bit 18 (ethereal), ActorData `+184`
    /// transport and `+174` bit 7 (OrganizationComponent).
    fn object_is_actor(&self, object: u32) -> Result<bool>;
    fn object_ethereal(&self, object: u32) -> Result<bool>;
    fn object_transport(&self, object: u32) -> Result<Option<u32>>;
    fn object_is_organization(&self, object: u32) -> Result<bool>;
    /// 554B30(map `[B36534]+30`; &position, &direction, object, 12.0, 0, 1):
    /// may move `position`; false when no open position exists. Bind
    /// `aa_spatial::placement::find_placement_within`.
    fn find_open_position(
        &mut self,
        position: &mut [F32; 2],
        direction: [F32; 2],
        object: u32,
    ) -> Result<bool>;
    /// 404F60 diagnostics of the failure paths.
    fn log(&mut self) -> Result<()>;
    /// 4CB5B0: "ability_effect_summon_failed" message 0x11 to the
    /// allegiance (nothing without one).
    fn summon_failed(&mut self, allegiance: Option<u32>) -> Result<()>;
    /// 4F9C10 / 4FA030 / 4FA150 on the world.
    fn create(&mut self, creator: Creator, descriptor: &Descriptor) -> Result<Option<u32>>;
    /// Actor `+8 |= 4` (not controllable).
    fn set_uncontrollable(&mut self, actor: u32) -> Result<()>;
    /// `[B36528]` present with byte `+20` set.
    fn sai_watching(&self) -> Result<bool>;
    /// 7A0C20 on the summoned actor.
    fn notify_sai(&mut self, actor: u32) -> Result<()>;
    /// Actor `+14` GTID.
    fn actor_id(&self, actor: u32) -> Result<u32>;
    /// 4CEF10(actor; property, 3, 0) (`aa_body::property_api::apply_property`).
    fn apply_property(&mut self, actor: u32, property: u32) -> Result<()>;
    /// Actor virtual `+B4` and `+7C` Organization: its `+28/+2C` members
    /// (`None` entries are null slots); `None` when not an organization.
    fn organization_members(&mut self, actor: u32) -> Result<Option<Vec<Option<u32>>>>;
    /// 4C8500(4, context) on the ability (`crate::fx`).
    fn emit_fx(&mut self, ability: &mut Ability, event: u32, context: &mut Context) -> Result<()>;
    /// Target virtual `+16C(summoned)`.
    fn target_attacks(&mut self, target: u32, summoned: u32) -> Result<bool>;
    /// Target `+60` Body's `+28` GTID, resolved (`None` without Body or
    /// when stale).
    fn body_attacker(&self, target: u32) -> Result<Option<u32>>;
    /// 4CA330(target) command, 4D9B60 on the summoned actor, 4CA2D0.
    fn order_attack(&mut self, summoned: u32, target: u32) -> Result<()>;
    /// `[B36534]+30` map `+C` width and `+10` height.
    fn map_size(&self) -> Result<[F32; 2]>;
    /// 552640 on the map: bind `aa_movement::falling::clamp_to_interior`.
    fn clamp_map_position(&mut self, position: &mut [F32; 2]) -> Result<()>;
    /// Transport ActorData `+414` altitude.
    fn transport_altitude(&self, transport: u32) -> Result<F32>;
    /// 4C2290(object `+2F0`) when the object is an organization, then
    /// 58BA90 on the transport's `+C0` component(object, "", data, start,
    /// drop).
    fn load_transport(
        &mut self,
        transport: u32,
        object: u32,
        organization_data: bool,
        start: [F32; 2],
        drop: [F32; 2],
    ) -> Result<()>;
    /// 4D9E20(5) command with flags `|0x1C`, the point and the startup
    /// globals B279C0/B279C4, issued by 4D9B60, then 4CA2D0.
    fn order_move(&mut self, transport: u32, point: [F32; 2]) -> Result<()>;
}

/// 409230: XY normalized in place; the returned length is zero when it is
/// not above 1e-4 (then the vector is zeroed).
fn normalize(v: &mut [F32; 2], p: Precision) -> Result<bool> {
    let x = n(v[0])?;
    let y = n(v[1])?;
    let length = y.mul(&y, p)?.add(&x.mul(&x, p)?, p)?.sqrt(p)?;
    if length.compare(&n(F32(0x38d1_b717))?) == Ordering::Greater {
        let inverse = Binary::from_u32(1).div(&length, p)?;
        v[0] = store(inverse.mul(&x, p)?)?;
        v[1] = store(inverse.mul(&y, p)?)?;
        Ok(true)
    } else {
        *v = [F32::ZERO; 2];
        Ok(false)
    }
}

/// Native SummonEffect Apply 4CCA70(ability, context, scale).
pub fn apply(
    effect: &SummonEffect,
    ability: &mut Ability,
    context: &mut Context,
    world: &mut impl SummonWorld,
    p: Precision,
) -> Result<bool> {
    let (mut position, mut direction);
    if world.definition_flags(ability.definition)? & 0x20 != 0 {
        position = [context.target_position[0], context.target_position[1]];
        direction = [F32::ZERO, F32(0xbf80_0000)];
    } else if let Some(source) = context.source {
        direction = world.actor_facing(source)?;
        let extent = n(world.actor_extent(source)?)?;
        let at = world.actor_position(source)?;
        let dx = store(n(direction[0])?.mul(&extent, p)?)?;
        let dy = store(n(direction[1])?.mul(&extent, p)?)?;
        position = [
            store(n(dx)?.add(&n(at[0])?, p)?)?,
            store(n(dy)?.add(&n(at[1])?, p)?)?,
        ];
    } else {
        position = [context.source_position[0], context.source_position[1]];
        direction = [
            store(n(context.target_position[0])?.sub(&n(context.source_position[0])?, p)?)?,
            store(n(context.target_position[1])?.sub(&n(context.source_position[1])?, p)?)?,
        ];
        if !normalize(&mut direction, p)? {
            direction = world.random_direction()?;
        }
    }
    let object = if world.campaign_mode()? {
        effect.dynamic_object
    } else {
        effect.object
    };
    let actor = world.object_is_actor(object)?;
    if !world.object_ethereal(object)?
        && !world.find_open_position(&mut position, direction, object)?
    {
        world.log()?;
        world.summon_failed(context.allegiance)?;
        return Ok(false);
    }
    if actor {
        if world.object_transport(object)?.is_some() {
            return summon_transported(effect, position, context, world, p);
        }
        return summon_actor(effect, position, direction, ability, context, world);
    }
    summon_object(effect, position, direction, ability, context, world)
}

fn object_descriptor(definition: u32, position: [F32; 2], direction: [F32; 2]) -> ObjectDescriptor {
    ObjectDescriptor {
        definition,
        position: [position[0], position[1], F32::ZERO],
        direction,
    }
}

/// 4C89A0: the summoned actor's GTID head-inserted into the ability's
/// `+24` list (`+28` tail).
fn add_summoned(ability: &mut Ability, id: u32) {
    ability.summoned_ids.insert(0, id);
}

/// 4CA7B0(summoned, ability, context): FX event 4 with the context target
/// set to every non-null organization member, or to the summoned actor
/// itself; the previous target is restored.
fn summoned_fx(
    summoned: u32,
    ability: &mut Ability,
    context: &mut Context,
    world: &mut impl SummonWorld,
) -> Result<()> {
    let saved = context.target;
    context.target = Some(summoned);
    match world.organization_members(summoned)? {
        Some(members) => {
            for member in members.into_iter().flatten() {
                context.target = Some(member);
                world.emit_fx(ability, 4, context)?;
            }
        }
        None => world.emit_fx(ability, 4, context)?,
    }
    context.target = saved;
    Ok(())
}

/// Native SummonObject 4CC2C0.
fn summon_object(
    effect: &SummonEffect,
    position: [F32; 2],
    direction: [F32; 2],
    ability: &mut Ability,
    context: &mut Context,
    world: &mut impl SummonWorld,
) -> Result<bool> {
    let definition = if world.campaign_mode()? {
        effect.dynamic_object
    } else {
        effect.object
    };
    let descriptor = Descriptor::Object(object_descriptor(definition, position, direction));
    let Some(created) = world.create(Creator::Generic, &descriptor)? else {
        world.log()?;
        world.summon_failed(context.allegiance)?;
        return Ok(false);
    };
    add_summoned(ability, world.actor_id(created)?);
    summoned_fx(created, ability, context, world)?;
    Ok(true)
}

/// Native SummonActor 4CC470.
fn summon_actor(
    effect: &SummonEffect,
    position: [F32; 2],
    direction: [F32; 2],
    ability: &mut Ability,
    context: &mut Context,
    world: &mut impl SummonWorld,
) -> Result<bool> {
    let definition = if world.campaign_mode()? {
        effect.dynamic_object
    } else {
        effect.object
    };
    let descriptor = Descriptor::Actor(ActorDescriptor {
        object: object_descriptor(definition, position, direction),
        allegiance: context.allegiance,
        flag51: true,
    });
    let creator = if world.object_is_organization(definition)? {
        Creator::Organization
    } else {
        Creator::Actor
    };
    let Some(created) = world.create(creator, &descriptor)? else {
        world.log()?;
        world.summon_failed(context.allegiance)?;
        return Ok(false);
    };
    if !effect.controllable {
        world.set_uncontrollable(created)?;
    }
    if world.sai_watching()? {
        world.notify_sai(created)?;
    }
    add_summoned(ability, world.actor_id(created)?);
    if let Some(property) = effect.property {
        world.apply_property(created, property)?;
    }
    summoned_fx(created, ability, context, world)?;
    if let Some(target) = world.effect_target(context)? {
        let victim = if world.target_attacks(target, created)? {
            Some(target)
        } else {
            world.body_attacker(target)?
        };
        if let Some(victim) = victim {
            world.order_attack(created, victim)?;
        }
    }
    Ok(true)
}

/// 4CB110's entry edge: the farther x edge distance `max(x, W - x)` (stored
/// binary32) against the farther y edge distance `max(y, H - y)`
/// (extended); the x edge only when strictly farther. The transport enters
/// from that far edge, heading inward. Returns (spawn, direction).
pub fn transport_entry(
    point: [F32; 2],
    map: [F32; 2],
    p: Precision,
) -> Result<([F32; 2], [F32; 2])> {
    let [x, y] = point;
    let [width, height] = map;
    let right = n(width)?.sub(&n(x)?, p)?;
    let ex = if right.compare(&n(x)?) == Ordering::Less {
        x
    } else {
        store(right)?
    };
    let top = n(height)?.sub(&n(y)?, p)?;
    let ey = if top.compare(&n(y)?) == Ordering::Less {
        n(y)?
    } else {
        top
    };
    let mut spawn = point;
    let mut direction = [F32::ZERO; 2];
    if n(ex)?.compare(&ey) == Ordering::Greater {
        if n(ex)?.compare(&n(x)?) == Ordering::Equal {
            spawn[0] = F32::ZERO;
            direction[0] = F32::ONE;
        } else {
            spawn[0] = width;
            direction[0] = F32(0xbf80_0000);
        }
    } else if ey.compare(&n(y)?) == Ordering::Equal {
        spawn[1] = F32::ZERO;
        direction[1] = F32::ONE;
    } else {
        spawn[1] = height;
        direction[1] = F32(0xbf80_0000);
    }
    Ok((spawn, direction))
}

/// Native SummonTransportedActor 4CB110(&point, context).
fn summon_transported(
    effect: &SummonEffect,
    point: [F32; 2],
    context: &mut Context,
    world: &mut impl SummonWorld,
    p: Precision,
) -> Result<bool> {
    let object = if world.campaign_mode()? {
        effect.dynamic_object
    } else {
        effect.object
    };
    let transport = world
        .object_transport(object)?
        .ok_or(crate::Error::Invalid(
            "transported summon without transport",
        ))?;
    let (mut spawn, direction) = transport_entry(point, world.map_size()?, p)?;
    world.clamp_map_position(&mut spawn)?;
    let altitude = store(n(world.transport_altitude(transport)?)?.add(&Binary::from_i32(0), p)?)?;
    let descriptor = Descriptor::Actor(ActorDescriptor {
        object: ObjectDescriptor {
            definition: transport,
            position: [spawn[0], spawn[1], altitude],
            direction,
        },
        allegiance: context.allegiance,
        flag51: false,
    });
    let Some(created) = world.create(Creator::Generic, &descriptor)? else {
        world.log()?;
        return Ok(false);
    };
    if !effect.controllable {
        world.set_uncontrollable(created)?;
    }
    let organization = world.object_is_organization(object)?;
    world.load_transport(created, object, organization, spawn, point)?;
    world.order_move(created, point)?;
    Ok(true)
}
