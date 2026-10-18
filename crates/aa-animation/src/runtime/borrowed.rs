//! 563770/563BA0 coordinator over short loans into the actual Character owner.
//! The legacy public entry points use this same algorithm. No Character is
//! detached from its actor while activation, FX or queue callbacks run.
use super::{Animation, Character, Effect, Error, F32, Result, current, number};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub trait World<D = u32> {
    fn character(&self) -> Result<&Character<D>>;
    fn character_mut(&mut self) -> Result<&mut Character<D>>;
    /// Native Component+C. Live adapters read the authoritative queue owner
    /// head; the legacy fixture adapter uses its explicit raw-state field.
    fn scheduled_head(&self) -> Result<Option<u64>>;
    fn animation(&self, handle: D) -> Result<Animation>;
    fn looping_type(&self, kind: u32) -> Result<bool>;
    fn now(&self) -> Result<F32>;
    fn remap_variant(&mut self, variant: i32, requested: D) -> Result<D>;
    fn apply_variant(&mut self, variant: i32) -> Result<()>;
    fn movement_speed(&self) -> Result<Option<F32>>;
    fn visual_scale(&mut self) -> Result<F32>;
    fn effect(&mut self, animation: D, effect: Effect) -> Result<()>;
    fn cancel_events(&mut self) -> Result<()>;
    fn schedule_after(&mut self, delay: F32, event: u32) -> Result<()>;
}

pub trait StanceWorld<D = u32>: World<D> {
    fn active_component_is_character(&self) -> Result<bool>;
    fn activate(&mut self) -> Result<bool>;
    fn performer(&self) -> Result<Option<u32>>;
    fn component_is_active(&self, component: u32) -> Result<bool>;
    fn actor_flag(&self, flag: u32) -> Result<bool>;
    fn performer_ability(&self, performer: u32) -> Result<Option<u32>>;
    fn model_variant(&self) -> Result<u32>;
    fn ability_idle(&self, ability: u32, variant: u32) -> Result<Option<D>>;
    fn first_idle(&self) -> Result<Option<D>>;
    fn idle_repetitions(&mut self) -> Result<i32>;
}

/// Native callback boundaries keep their existing ordering. Mutable loans end
/// before every service call, and every later field read reaches the owner again.
pub fn play<D: Clone + PartialEq>(
    world: &mut impl World<D>,
    requested: D,
    duration: F32,
    p: Precision,
) -> Result<()> {
    let mut selected = requested.clone();
    let variant = world.character()?.variant;
    if variant != -2 {
        selected = world.remap_variant(variant, requested)?;
        let variant = world.character()?.variant;
        world.apply_variant(variant)?;
        world.character_mut()?.variant = -2;
    }
    let old = current(world.character()?)?;
    let selected_info = world.animation(selected.clone())?;
    let restarted = if selected_info.kind == 2 {
        let speed = world.movement_speed()?.unwrap_or(F32::ONE);
        if number(speed)?.compare(&Binary::from_f32_bits(0x38d1b717)?) == Ordering::Greater {
            let scale = world.visual_scale()?;
            let previous = world.animation(current(world.character()?)?)?.kind;
            let now = world.now()?;
            super::start_move(
                world.character_mut()?,
                selected,
                previous,
                speed,
                scale,
                selected_info.walk_distance,
                now,
                p,
            )?
        } else {
            false
        }
    } else {
        let looping = world.looping_type(selected_info.kind)?;
        let now = world.now()?;
        super::start_direct(world.character_mut()?, selected, looping, now)?
    };
    if restarted {
        world.effect(old, Effect::End)?;
        let selected = current(world.character()?)?;
        world.effect(selected, Effect::Start)?;
    }
    if !number(duration)?.is_zero() {
        world.cancel_events()?;
        let event = world.character()?.word_28;
        world.schedule_after(duration, event)?;
    }
    Ok(())
}

pub fn default_stance<D: Clone + PartialEq>(
    world: &mut impl StanceWorld<D>,
    p: Precision,
) -> Result<()> {
    if world.active_component_is_character()?
        && matches!(world.character()?.word_28, 0 | 1)
        && world.scheduled_head()?.is_some()
    {
        return Ok(());
    }
    if !world.activate()? {
        return Ok(());
    }
    let mut ability = None;
    if let Some(performer) = world.performer()?
        && (world.component_is_active(performer)? || world.actor_flag(4)? || world.actor_flag(1)?)
    {
        let performer = world
            .performer()?
            .ok_or(Error::Invalid("Performer removed during stance query"))?;
        ability = world.performer_ability(performer)?;
    }
    let (animation, delay) = if let Some(ability) = ability {
        let id = world
            .ability_idle(ability, world.model_variant()?)?
            .ok_or(Error::Invalid(
                "active ability has no native idle animation",
            ))?;
        (id.clone(), world.animation(id)?.length)
    } else {
        let id = world
            .first_idle()?
            .ok_or(Error::Invalid("Character view has no first idle"))?;
        let length = world.animation(id.clone())?.length;
        let count = world.idle_repetitions()?;
        (
            id,
            F32(Binary::from_i32(count)
                .mul(&number(length)?, p)?
                .store_f32()?),
        )
    };
    play(world, animation, F32::ZERO, p)?;
    world.character_mut()?.word_28 = 0;
    world.schedule_after(delay, 0)
}

pub(super) struct Legacy<'a, D, W> {
    pub state: &'a mut Character<D>,
    pub world: &'a mut W,
}
impl<D, W: super::World<D>> World<D> for Legacy<'_, D, W> {
    fn character(&self) -> Result<&Character<D>> {
        Ok(self.state)
    }
    fn character_mut(&mut self) -> Result<&mut Character<D>> {
        Ok(self.state)
    }
    fn scheduled_head(&self) -> Result<Option<u64>> {
        Ok(self.state.header.scheduled_head)
    }
    fn animation(&self, h: D) -> Result<Animation> {
        self.world.animation(h)
    }
    fn looping_type(&self, k: u32) -> Result<bool> {
        self.world.looping_type(k)
    }
    fn now(&self) -> Result<F32> {
        self.world.now()
    }
    fn remap_variant(&mut self, v: i32, h: D) -> Result<D> {
        self.world.remap_variant(self.state, v, h)
    }
    fn apply_variant(&mut self, v: i32) -> Result<()> {
        self.world.apply_variant(self.state, v)
    }
    fn movement_speed(&self) -> Result<Option<F32>> {
        self.world.movement_speed(self.state)
    }
    fn visual_scale(&mut self) -> Result<F32> {
        self.world.visual_scale(self.state)
    }
    fn effect(&mut self, h: D, e: Effect) -> Result<()> {
        self.world.effect(self.state, h, e)
    }
    fn cancel_events(&mut self) -> Result<()> {
        self.world.cancel_events(self.state)
    }
    fn schedule_after(&mut self, d: F32, e: u32) -> Result<()> {
        self.world.schedule_after(self.state, d, e)
    }
}
impl<D, W: super::StanceWorld<D>> StanceWorld<D> for Legacy<'_, D, W> {
    fn active_component_is_character(&self) -> Result<bool> {
        self.world.active_component_is_character(self.state)
    }
    fn activate(&mut self) -> Result<bool> {
        self.world.activate(self.state)
    }
    fn performer(&self) -> Result<Option<u32>> {
        self.world.performer(self.state)
    }
    fn component_is_active(&self, c: u32) -> Result<bool> {
        self.world.component_is_active(c)
    }
    fn actor_flag(&self, f: u32) -> Result<bool> {
        self.world.actor_flag(self.state, f)
    }
    fn performer_ability(&self, c: u32) -> Result<Option<u32>> {
        self.world.performer_ability(c)
    }
    fn model_variant(&self) -> Result<u32> {
        self.world.model_variant(self.state)
    }
    fn ability_idle(&self, a: u32, v: u32) -> Result<Option<D>> {
        self.world.ability_idle(a, v)
    }
    fn first_idle(&self) -> Result<Option<D>> {
        self.world.first_idle(self.state)
    }
    fn idle_repetitions(&mut self) -> Result<i32> {
        self.world.idle_repetitions(self.state)
    }
}
