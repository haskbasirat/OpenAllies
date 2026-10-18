//! Native Performer start and scheduled animation/use/reload lifecycle.
//! Event deadlines come from the authored animation, not a replacement cooldown.
use crate::{Error, Result};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub event_kind: u32,
    pub phase: u32,
    pub source: Option<u32>,
    pub allegiance: Option<u32>,
    pub source_position: [F32; 3],
    pub target: Option<u32>,
    pub target_position: [F32; 3],
    pub word2c: u32,
}
impl Context {
    pub fn new(event_kind: u32) -> Self {
        Self {
            event_kind,
            phase: 0,
            source: None,
            allegiance: None,
            source_position: [F32::ZERO; 3],
            target: None,
            target_position: [F32::ZERO; 3],
            word2c: 0,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Performer {
    pub owner: u32,
    /// Native +10: 0 turret wait, 1 use frame, 2 animation end, 3 reload, 4 done.
    pub stage: i32,
    pub abilities: Vec<u32>,
    pub mana: F32,
    pub target_id: u32,
    pub target_position: [F32; 2],
    pub active_ability: Option<u32>,
    pub active_index: i32,
    pub started_at: F32,
    pub animation_length: F32,
    pub event_kind: u32,
    pub animation: Option<u32>,
    pub turret_index: i32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Start {
    pub ability: u32,
    pub event_kind: u32,
    pub target: Option<u32>,
    pub point: Option<[F32; 2]>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub id: u32,
    pub position: [F32; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Animation {
    pub length: F32,
    pub use_fraction: F32,
    /// Native linked animation cue times, in original order.
    pub cues: Vec<F32>,
}

/// Required live services. Callbacks may mutate Performer fields; subsequent
/// reads happen at native boundaries. Actor/ability handles remain alive for
/// the call. Recursive same-Performer dispatch and arbitrary definition/list
/// replacement during inspection are outside the supported contract.
pub trait World {
    fn enabled(&self, ability: u32) -> Result<bool>;
    fn actor(&self, handle: u32) -> Result<Actor>;
    fn resolve_id(&self, id: u32) -> Result<Option<u32>>;
    /// 4C8F20. Zero means usable; the native rejection value is retained.
    fn can_use(&mut self, ability: u32, context: &mut Context, check_range: bool) -> Result<u32>;
    fn activate(&mut self, state: &mut Performer) -> Result<bool>;
    /// 57DD90 target/turret intention, before turret-start selection; bind
    /// [`track_target`] on the owning `OwnedPerformer`.
    fn track_target(
        &mut self,
        state: &mut Performer,
        request: Start,
        point: [F32; 2],
    ) -> Result<()>;
    fn ability_turret(&self, ability: u32) -> Result<i32>;
    fn ability_uses_turret(&self, ability: u32) -> Result<bool>;
    fn turret_turn_required(&self, owner: u32, index: i32) -> Result<bool>;
    fn clear_turret_pending(&mut self, state: &mut Performer, index: i32) -> Result<()>;
    fn has_mover(&self, owner: u32) -> Result<bool>;
    fn face_direction(&mut self, owner: u32, direction: [F32; 2]) -> Result<()>;
    fn now(&self) -> Result<F32>;
    /// Native ability FX event0 (start) or2 (end), with mutable actual context.
    fn ability_event(
        &mut self,
        state: &mut Performer,
        ability: u32,
        event: u32,
        context: &mut Context,
    ) -> Result<()>;
    fn model_variant(&self, owner: u32) -> Result<u32>;
    /// Must preserve 4C8170/4BE5C0 RNG consumption, even if an old animation wins.
    fn select_animation(&mut self, ability: u32, variant: u32) -> Result<Option<u32>>;
    fn keep_previous_animation(&self, ability: u32) -> Result<bool>;
    fn animation_belongs_to(&self, ability: u32, variant: u32, animation: u32) -> Result<bool>;
    fn animation(&self, animation: u32) -> Result<Animation>;
    fn has_animator(&self, owner: u32) -> Result<bool>;
    fn play_animation(&mut self, state: &mut Performer, animation: u32) -> Result<()>;
    fn animator_begin(&mut self, state: &mut Performer) -> Result<()>;
    fn animator_use(&mut self, state: &mut Performer) -> Result<()>;
    fn animator_cue(&mut self, state: &mut Performer, cue: i32) -> Result<()>;
    /// 57E030/4C9550 delivery includes original recoil and mana debit semantics.
    fn use_ability(
        &mut self,
        state: &mut Performer,
        ability: u32,
        context: &mut Context,
        precision: Precision,
    ) -> Result<()>;
    fn reload_time(&self, ability: u32) -> Result<F32>;
    fn idle_animation(&self, ability: u32, variant: u32) -> Result<Option<u32>>;
    fn play_idle(&mut self, state: &mut Performer, animation: u32, duration: F32) -> Result<()>;
    fn active_component_is_self(&self, state: &Performer) -> Result<bool>;
    fn actor_paused(&self, owner: u32) -> Result<bool>;
    fn notify_finished(&mut self, state: &mut Performer) -> Result<()>;
    fn schedule_immediate(&mut self, state: &mut Performer, event: i32) -> Result<()>;
    fn schedule_after(&mut self, state: &mut Performer, delay: F32, event: i32) -> Result<()>;
    fn schedule_at(&mut self, state: &mut Performer, time: F32, event: i32) -> Result<()>;
    fn cancel_event(&mut self, state: &mut Performer, event: i32, periodic: bool) -> Result<()>;
}
fn number(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn active(state: &Performer) -> Result<u32> {
    state
        .active_ability
        .ok_or(Error::Invalid("Performer has no active ability"))
}

/// Native57E360. Context initialization4C8300 remains separate from these writes.
pub fn fill_context(state: &Performer, context: &mut Context, world: &impl World) -> Result<()> {
    context.phase = 0;
    context.source = Some(state.owner);
    context.target = world.resolve_id(state.target_id)?;
    let position = match world.resolve_id(state.target_id)? {
        Some(actor) => {
            let p = world.actor(actor)?.position;
            [p[0], p[1]]
        }
        None => state.target_position,
    };
    context.target_position = [position[0], position[1], F32::ZERO];
    Ok(())
}
/// Native57DC90 notifies Mind before publishing stage4.
pub fn finish(state: &mut Performer, world: &mut impl World) -> Result<()> {
    world.notify_finished(state)?;
    state.stage = 4;
    Ok(())
}
/// Native57ED20 begins the chosen animation and publishes exact event deadlines.
pub fn begin(state: &mut Performer, world: &mut impl World, precision: Precision) -> Result<()> {
    state.started_at = world.now()?;
    let mut context = Context::new(state.event_kind);
    fill_context(state, &mut context, world)?;
    world.ability_event(state, active(state)?, 0, &mut context)?;
    let variant = world.model_variant(state.owner)?;
    let mut selected = world.select_animation(active(state)?, variant)?;
    if state.animation.is_some() && world.keep_previous_animation(active(state)?)? {
        let old = state
            .animation
            .ok_or(Error::Invalid("animation changed during pure query"))?;
        let variant = world.model_variant(state.owner)?;
        if world.animation_belongs_to(active(state)?, variant, old)? {
            selected = state.animation;
        }
    }
    state.animation = selected;
    if let Some(animation) = selected {
        state.animation_length = world.animation(animation)?.length;
        if world.has_animator(state.owner)? {
            world.play_animation(state, animation)?;
            world.animator_begin(state)?;
        }
        state.stage = 1;
        // The native animation is read after the animator callbacks.
        let info = world.animation(animation)?;
        let deadline = F32(number(info.use_fraction)?
            .mul(&number(info.length)?, precision)?
            .add(&number(state.started_at)?, precision)?
            .store_f32()?);
        world.schedule_at(state, deadline, 1)?;
        for (index, time) in world.animation(animation)?.cues.into_iter().enumerate() {
            let event = i32::try_from(index)
                .map_err(|_| Error::Invalid("animation cue index overflow"))?
                .checked_add(6)
                .ok_or(Error::Invalid("animation cue event overflow"))?;
            world.schedule_after(state, time, event)?;
        }
    } else {
        state.animation_length = F32::ZERO;
        state.stage = 1;
        world.schedule_after(state, F32::ZERO, 1)?;
    }
    Ok(())
}
/// Native57F290. Target handles and the ability list remain stable during admission.
pub fn start(
    state: &mut Performer,
    request: Start,
    world: &mut impl World,
    precision: Precision,
) -> Result<bool> {
    if !world.enabled(request.ability)? {
        return Ok(false);
    }
    let index = state
        .abilities
        .iter()
        .position(|&a| a == request.ability)
        .map(|i| i32::try_from(i).map_err(|_| Error::Invalid("ability index overflow")))
        .transpose()?
        .unwrap_or(-1);
    let target_position = match request.target {
        Some(target) => world.actor(target)?.position,
        None => {
            let point = request
                .point
                .ok_or(Error::Invalid("ability requires a target or point"))?;
            [point[0], point[1], F32::ZERO]
        }
    };
    let mut context = Context::new(request.event_kind);
    context.source = Some(state.owner);
    context.target = request.target;
    context.target_position = target_position;
    if world.can_use(request.ability, &mut context, true)? != 0 {
        return Ok(false);
    }
    if !world.activate(state)? {
        return Ok(false);
    }
    state.event_kind = request.event_kind;
    state.active_ability = Some(request.ability);
    state.target_id = match request.target {
        Some(t) => world.actor(t)?.id,
        None => 0,
    };
    state.target_position = if let Some(point) = request.point {
        point
    } else {
        let p = world
            .actor(request.target.ok_or(Error::Invalid("target disappeared"))?)?
            .position;
        [p[0], p[1]]
    };
    state.active_index = index;
    let track_point = if let Some(point) = request.point {
        point
    } else {
        let p = world
            .actor(request.target.ok_or(Error::Invalid("target disappeared"))?)?
            .position;
        [p[0], p[1]]
    };
    world.track_target(state, request, track_point)?;
    state.turret_index = world.ability_turret(request.ability)?;
    if world.ability_uses_turret(request.ability)?
        && world.turret_turn_required(state.owner, state.turret_index)?
    {
        state.stage = 0;
        world.schedule_immediate(state, 0)?;
        world.clear_turret_pending(state, state.turret_index)?;
        return Ok(true);
    }
    if world.has_mover(state.owner)? {
        let p = match request.target {
            Some(t) => {
                let p = world.actor(t)?.position;
                [p[0], p[1]]
            }
            None => request
                .point
                .ok_or(Error::Invalid("target point missing"))?,
        };
        let owner = world.actor(state.owner)?.position;
        let dy = number(p[1])?.sub(&number(owner[1])?, precision)?;
        let dx = F32(number(p[0])?
            .sub(&number(owner[0])?, precision)?
            .store_f32()?);
        let distance = number(dx)?
            .mul(&number(dx)?, precision)?
            .add(&dy.mul(&dy, precision)?, precision)?;
        if distance.compare(&Binary::from_f64_bits(0x3f84_7ae1_47ae_147b)?) == Ordering::Greater {
            world.face_direction(state.owner, [dx, F32(dy.store_f32()?)])?;
        }
    }
    begin(state, world, precision)?;
    Ok(true)
}
/// Native57E8C0 revalidates the target at the use frame, without range checking.
pub fn use_frame(
    state: &mut Performer,
    world: &mut impl World,
    precision: Precision,
) -> Result<()> {
    let mut context = Context::new(state.event_kind);
    fill_context(state, &mut context, world)?;
    if world.can_use(active(state)?, &mut context, false)? == 0 {
        if world.has_animator(state.owner)? {
            world.animator_use(state)?;
        }
        world.use_ability(state, active(state)?, &mut context, precision)?;
    }
    Ok(())
}
/// Native57F040 is called with the Performer scheduler subobject; this safe API
/// accepts the containing state and preserves its native signed event dispatch.
pub fn timed_event(
    state: &mut Performer,
    event: i32,
    world: &mut impl World,
    precision: Precision,
) -> Result<()> {
    if !world.active_component_is_self(state)? {
        return Ok(());
    }
    if world.actor_paused(state.owner)? {
        world.schedule_after(state, F32(0x38d1_b717), event)?;
        return Ok(());
    }
    if event >= 6 {
        if world.has_animator(state.owner)? {
            world.animator_cue(state, event - 6)?;
        }
        return Ok(());
    }
    if state.stage != event {
        return Ok(());
    }
    match event {
        0 => {
            state.turret_index = -1;
            world.cancel_event(state, 0, false)?;
            begin(state, world, precision)?;
        }
        1 => {
            use_frame(state, world, precision)?;
            let time = F32(number(state.animation_length)?
                .add(&number(state.started_at)?, precision)?
                .store_f32()?);
            state.stage = 2;
            world.schedule_at(state, time, 2)?;
        }
        2 => {
            let mut context = Context::new(state.event_kind);
            fill_context(state, &mut context, world)?;
            world.ability_event(state, active(state)?, 2, &mut context)?;
            if number(world.reload_time(active(state)?)?)?.compare(&number(F32(0x38d1_b717))?)
                == Ordering::Less
            {
                finish(state, world)?;
            } else {
                let variant = world.model_variant(state.owner)?;
                if let Some(animation) = world.idle_animation(active(state)?, variant)? {
                    let duration = world.reload_time(active(state)?)?;
                    world.play_idle(state, animation, duration)?;
                }
                state.stage = 3;
                let deadline = F32(number(world.reload_time(active(state)?)?)?
                    .add(&number(state.animation_length)?, precision)?
                    .add(&number(state.started_at)?, precision)?
                    .store_f32()?);
                world.schedule_at(state, deadline, state.stage)?;
            }
        }
        3 => finish(state, world)?,
        _ => {}
    }
    Ok(())
}

/// Services of native `57DD90` (Performer turret intention).
pub trait TrackWorld {
    /// `4C81E0`: ability `[+8]+54` turret index (`-1` none).
    fn ability_turret(&self, ability: u32) -> Result<i32>;
    /// `45DD30` on the owner definition (`529C30`): `+1F8[index]` byte `+31`.
    fn turret_tracks_target(&self, owner: u32, index: i32) -> Result<bool>;
    /// Live actor `+20` position, reread for every ability.
    fn actor_position(&self, actor: u32) -> Result<[F32; 3]>;
    /// `4C8F20(context, check_range)`; zero means usable.
    fn can_use(&mut self, ability: u32, context: &mut Context, check_range: bool) -> Result<u32>;
    /// `50AC00(1)` on the Performer `+8` Updatable.
    fn activate_updates(
        &mut self,
        performer: &mut crate::constructors::OwnedPerformer,
    ) -> Result<()>;
}

/// Native `57DD90(event_kind, target, &point)`: every other ability whose turret
/// tracks targets and which admits the target (without the range check) aims
/// its turret at `point`; the first aim activates the Performer updates.
/// `World::track_target` binds this.
pub fn track_target(
    performer: &mut crate::constructors::OwnedPerformer,
    event_kind: u32,
    target: Option<u32>,
    point: [F32; 2],
    world: &mut impl TrackWorld,
) -> Result<()> {
    let mut index = 0;
    while index < performer.state.abilities.len() {
        let ability = performer.state.abilities[index];
        index += 1;
        let turret = world.ability_turret(ability)?;
        if performer.state.active_ability == Some(ability) || turret == -1 {
            continue;
        }
        if !world.turret_tracks_target(performer.state.owner, turret)? {
            continue;
        }
        // 4C8300 then the source, target and position writes.
        let mut context = Context::new(event_kind);
        context.source = Some(performer.state.owner);
        context.target = target;
        context.target_position = match target {
            Some(actor) => world.actor_position(actor)?,
            None => [point[0], point[1], F32::ZERO],
        };
        if world.can_use(ability, &mut context, false)? != 0 {
            continue;
        }
        let record = usize::try_from(turret)
            .ok()
            .and_then(|turret| performer.turrets.get_mut(turret))
            .ok_or(Error::Invalid(
                "ability turret index outside the Performer +4C records",
            ))?;
        record.pending = true;
        record.target[0] = point[0];
        record.target[1] = point[1];
        if !performer.turret_update_pending {
            performer.turret_update_pending = true;
            world.activate_updates(performer)?;
        }
    }
    Ok(())
}

/// Native `57DEA0`: clear the pending flag of the first `turret_count`
/// (owner definition `+1FC`) turret records.
pub fn clear_turret_intentions(
    performer: &mut crate::constructors::OwnedPerformer,
    turret_count: i32,
) -> Result<()> {
    for index in 0..usize::try_from(turret_count.max(0)).unwrap_or(0) {
        performer
            .turrets
            .get_mut(index)
            .ok_or(Error::Invalid(
                "definition +1FC exceeds the Performer +4C records",
            ))?
            .pending = false;
    }
    Ok(())
}

/// Native `0x4BE4F0(rank)` with `0x4BE4D0`: a local definition's `+60`
/// minimum and `+64` maximum rank (`-1` unbounded). `None` when the
/// definition has no limits (4BE4D0 false).
pub fn rank_allows(limits: [i32; 2], rank: i32) -> Option<bool> {
    let [minimum, maximum] = limits;
    if minimum == -1 && maximum == -1 {
        return None;
    }
    Some((minimum == -1 || minimum <= rank) && (maximum == -1 || rank <= maximum))
}

pub trait RankWorld {
    /// Ability `+8` local definition.
    fn ability_local_definition(&self, ability: u32) -> Result<Option<u32>>;
    /// Local definition `+60` / `+64`.
    fn rank_limits(&self, local_definition: u32) -> Result<[i32; 2]>;
    /// Ability `+18` enabled byte.
    fn ability_enabled(&self, ability: u32) -> Result<bool>;
    fn set_ability_enabled(&mut self, ability: u32, enabled: bool) -> Result<()>;
    /// `0x4D1350` on the owner.
    fn notify(&mut self, owner: u32) -> Result<()>;
}

/// Native `0x57DCF0(rank object)`: the rank is the object's `+4` (0 without
/// one); every rank-limited ability's enabled byte follows
/// [`rank_allows`]; the owner is notified once when any byte changed.
pub fn gate_by_rank(
    state: &Performer,
    rank: Option<i32>,
    world: &mut impl RankWorld,
) -> Result<()> {
    let rank = rank.unwrap_or(0);
    let mut changed = false;
    for &ability in &state.abilities {
        let Some(local) = world.ability_local_definition(ability)? else {
            continue;
        };
        let Some(allowed) = rank_allows(world.rank_limits(local)?, rank) else {
            continue;
        };
        if world.ability_enabled(ability)? != allowed {
            world.set_ability_enabled(ability, allowed)?;
            changed = true;
        }
    }
    if changed {
        world.notify(state.owner)?;
    }
    Ok(())
}
