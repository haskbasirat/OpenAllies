//! Original Character/Animator timing and selection, distinct from KF sampling.
//! Authoritative scheduling, animation definitions and FX are required services.
use aa_actors::ground_states::CharacterState;
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde::{Deserialize, Serialize};
pub mod borrowed;

///D must identify the native definition object, not only its display/clip name.
///The default preserves existing numeric-oracle adapters; live factories use
///scoped canonical String keys in the same authoritative CharacterState box.
pub type Character<D = u32> = CharacterState<D>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("invalid native animation runtime: {0}")]
    Invalid(&'static str),
    #[error("required animation service failed: {0}")]
    Service(String),
}
pub type Result<T> = std::result::Result<T, Error>;
fn number(v: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(v.0)?)
}
fn current<D: Clone>(state: &Character<D>) -> Result<D> {
    state
        .animation
        .clone()
        .ok_or(Error::Invalid("Character has no animation"))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Animation {
    pub kind: u32,
    /// Native Animation+10, authored desired gameplay seconds (not KF duration).
    pub length: F32,
    pub use_fraction: F32,
    pub walk_distance: F32,
    /// Native head-first linked-list order, reverse of authored WhenDoAtTime list.
    pub cues: Vec<F32>,
}
///499FF0/4970D0 authored built-in type policy: idle and move loop; the other
///five built-in types restart. Custom type IDs require an explicit provider.
pub fn canonical_looping_type(kind: u32) -> Result<bool> {
    match kind {
        0 | 2 => Ok(true),
        1 | 3..=6 => Ok(false),
        _ => Err(Error::Invalid("unknown native animation type")),
    }
}

///4BE5C0/4BE610/4BE660 consume already resolved native variant lists. Variant
///indices are never clamped or wrapped into a different model's animations.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityAnimations<D = u32> {
    pub variants: Vec<Vec<D>>,
    pub idle: Vec<Option<D>>,
    pub flags: u32,
}
impl<D: Clone + PartialEq> AbilityAnimations<D> {
    pub fn select(&self, variant: u32, random: &mut NativeRng) -> Result<Option<D>> {
        if self.variants.is_empty() {
            return Ok(None);
        }
        let choices = self
            .variants
            .get(variant as usize)
            .ok_or(Error::Invalid("animation variant outside resolved list"))?;
        if choices.is_empty() {
            return Ok(None);
        }
        let count = u32::try_from(choices.len())
            .map_err(|_| Error::Invalid("animation list exceeds native u32 count"))?;
        Ok(Some(choices[(random.next_u32() % count) as usize].clone()))
    }
    pub fn contains(&self, variant: u32, animation: D) -> Result<bool> {
        if self.variants.is_empty() {
            return Ok(false);
        }
        Ok(self
            .variants
            .get(variant as usize)
            .ok_or(Error::Invalid("animation variant outside resolved list"))?
            .contains(&animation))
    }
    pub fn idle(&self, variant: u32) -> Result<Option<D>> {
        if self.idle.is_empty() {
            return Ok(None);
        }
        self.idle
            .get(variant as usize)
            .cloned()
            .ok_or(Error::Invalid("idle variant outside resolved list"))
    }
    ///4BE590 uses the complete masked field, not merely one present flag bit.
    pub fn has_mode(&self, mode: u8) -> bool {
        self.flags & 0x1ff800 == 1_u32.wrapping_shl(u32::from(mode).wrapping_add(11))
    }
    pub fn keep_previous(&self) -> bool {
        self.has_mode(3)
    }
}

///49B4D0. The integer interval excludes its upper end; native wrapping addition
///and unsigned divisor are retained. Zero width is rejected before RNG mutation.
pub fn idle_repetitions(min: i32, max: i32, random: &mut NativeRng) -> Result<i32> {
    let width = max.wrapping_sub(min) as u32;
    if width == 0 {
        return Err(Error::Invalid("zero native idle-repeat interval"));
    }
    Ok((random.next_u32() % width).wrapping_add(min as u32) as i32)
}

///4DAF20. Legacy CharacterState.active is the native animator paused byte(+24),
///elapsed is start timestamp(+14), resumed_at is pause timestamp(+18).
pub fn pause<D>(state: &mut Character<D>, now: F32) -> Result<()> {
    number(now)?;
    if !state.active {
        state.resumed_at = now;
        state.active = true;
    }
    Ok(())
}
///4DAF40 shifts the start timestamp by time spent paused, with one f32 store.
pub fn resume<D>(state: &mut Character<D>, now: F32, p: Precision) -> Result<()> {
    if state.active {
        let shifted = number(now)?
            .sub(&number(state.resumed_at)?, p)?
            .add(&number(state.elapsed)?, p)?
            .store_f32()?;
        state.active = false;
        state.elapsed = F32(shifted);
    }
    Ok(())
}
///4DB120 returns whether native end/start FX must run. Repeated looping
///animation preserves start time; every call clears pause and resets rate to1.
pub fn start_direct<D: PartialEq>(
    state: &mut Character<D>,
    animation: D,
    looping: bool,
    now: F32,
) -> Result<bool> {
    number(now)?;
    let restarted = state.animation.as_ref() != Some(&animation) || !looping;
    if restarted {
        state.elapsed = now;
    }
    state.animation = Some(animation);
    state.active = false;
    state.rate = F32::ONE;
    Ok(restarted)
}
///4DB170. The native denominator stays in x87 precision until division/store.
#[allow(clippy::too_many_arguments)] //Original operands and explicit x87 precision.
pub fn start_move<D>(
    state: &mut Character<D>,
    animation: D,
    previous_kind: u32,
    speed: F32,
    visual_scale: F32,
    walk_distance: F32,
    now: F32,
    p: Precision,
) -> Result<bool> {
    //Compute before publication so an unsupported finite-domain operation fails
    //without a partially changed portable state.
    let rate = F32(number(speed)?
        .div(&number(visual_scale)?.mul(&number(walk_distance)?, p)?, p)?
        .store_f32()?);
    resume(state, now, p)?;
    let restarted = previous_kind != 2;
    if restarted {
        number(now)?;
        state.elapsed = now;
    }
    state.rate = rate;
    state.animation = Some(animation);
    Ok(restarted)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaybackParameters {
    pub start: F32,
    pub rate: F32,
}
///44F7B0 presentation boundary: client-time pause offset and native controller
///duration/gameplay-length ratio. This does not invent an authoritative clock.
pub fn playback_parameters<D>(
    state: &Character<D>,
    client_time: F32,
    kf_duration: F32,
    gameplay_length: F32,
    p: Precision,
) -> Result<PlaybackParameters> {
    let start = if state.active {
        F32(number(client_time)?
            .sub(&number(state.resumed_at)?, p)?
            .add(&number(state.elapsed)?, p)?
            .store_f32()?)
    } else {
        state.elapsed
    };
    let rate = F32(number(kf_duration)?
        .div(&number(gameplay_length)?, p)?
        .mul(&number(state.rate)?, p)?
        .store_f32()?);
    Ok(PlaybackParameters { start, rate })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    Start,
    Use,
    End,
    Cue(usize),
}

///Services can mutate Character at the same callback boundaries as the native
///engine. Pure definition queries remain stable for the call; scheduler, actor
///virtual operations and FX are not silently replaced by no-ops.
pub trait World<D = u32> {
    fn animation(&self, handle: D) -> Result<Animation>;
    fn looping_type(&self, kind: u32) -> Result<bool> {
        canonical_looping_type(kind)
    }
    fn now(&self) -> Result<F32>;
    ///Resolve requested name in pending variant view, with native type fallback.
    fn remap_variant(&mut self, state: &mut Character<D>, variant: i32, requested: D) -> Result<D>;
    fn apply_variant(&mut self, state: &mut Character<D>, variant: i32) -> Result<()>;
    fn movement_speed(&self, state: &Character<D>) -> Result<Option<F32>>;
    ///Actor virtual+F4, converted to f32 before start_move in the original.
    fn visual_scale(&mut self, state: &mut Character<D>) -> Result<F32>;
    fn effect(&mut self, state: &mut Character<D>, animation: D, effect: Effect) -> Result<()>;
    fn cancel_events(&mut self, state: &mut Character<D>) -> Result<()>;
    fn schedule_after(&mut self, state: &mut Character<D>, delay: F32, event: u32) -> Result<()>;
}

///563770: original Character start, pending variant remap, locomotion rate,
///old-end/new-start FX ordering, and optional rescheduling.
pub fn play<D: Clone + PartialEq>(
    state: &mut Character<D>,
    requested: D,
    duration: F32,
    world: &mut impl World<D>,
    p: Precision,
) -> Result<()> {
    borrowed::play(&mut borrowed::Legacy { state, world }, requested, duration, p)
}
///563640.
pub fn begin<D: Clone + PartialEq>(
    state: &mut Character<D>,
    world: &mut impl World<D>,
) -> Result<()> {
    state.word_28 = 0;
    world.cancel_events(state)?;
    let delay = world.animation(current(state)?)?.length;
    world.schedule_after(state, delay, state.word_28)
}
///4DB080 deactivation cancels scheduled entries. It does not implicitly pause
///or reset the Animator; the native reason argument is unused by this leaf.
pub fn stop<D>(state: &mut Character<D>, _reason: u32, world: &mut impl World<D>) -> Result<()> {
    world.cancel_events(state)
}
///563620.
pub fn use_animation<D: Clone + PartialEq>(
    state: &mut Character<D>,
    world: &mut impl World<D>,
) -> Result<()> {
    world.effect(state, current(state)?, Effect::Use)
}
///563AA0. Missing/negative indices walk to the end and have no effect.
pub fn cue<D: Clone + PartialEq>(
    state: &mut Character<D>,
    index: i32,
    world: &mut impl World<D>,
) -> Result<()> {
    let animation = current(state)?;
    if let Ok(index) = usize::try_from(index)
        && index < world.animation(animation.clone())?.cues.len()
    {
        world.effect(state, animation, Effect::Cue(index))?;
    }
    Ok(())
}

pub trait StanceWorld<D = u32>: World<D> {
    fn active_component_is_character(&self, state: &Character<D>) -> Result<bool>;
    fn activate(&mut self, state: &mut Character<D>) -> Result<bool>;
    fn performer(&self, state: &Character<D>) -> Result<Option<u32>>;
    fn component_is_active(&self, component: u32) -> Result<bool>;
    fn actor_flag(&self, state: &Character<D>, flag: u32) -> Result<bool>;
    fn performer_ability(&self, performer: u32) -> Result<Option<u32>>;
    fn model_variant(&self, state: &Character<D>) -> Result<u32>;
    fn ability_idle(&self, ability: u32, variant: u32) -> Result<Option<D>>;
    fn first_idle(&self, state: &Character<D>) -> Result<Option<D>>;
    ///4F02B0 +49B4D0: shared RNG modulo the definition's native interval.
    fn idle_repetitions(&mut self, state: &mut Character<D>) -> Result<i32>;
}
///563BA0. Eligibility and native short-circuit query order remain observable.
pub fn default_stance<D: Clone + PartialEq>(
    state: &mut Character<D>,
    world: &mut impl StanceWorld<D>,
    p: Precision,
) -> Result<()> {
    borrowed::default_stance(&mut borrowed::Legacy { state, world }, p)
}

///4CFC80's actor dispatch. The no-Character path clears the active pointer
///before old virtual+40 and rereads it before new virtual+3C.
pub trait ActorStanceWorld {
    fn character(&self, actor: u32) -> Result<Option<u32>>;
    fn character_default_stance(&mut self, character: u32) -> Result<()>;
    fn active_component(&self, actor: u32) -> Result<Option<u32>>;
    fn clear_active_component(&mut self, actor: u32) -> Result<()>;
    fn deactivate(&mut self, component: u32, reason: u32) -> Result<()>;
    fn resume_component(&mut self, component: u32, reason: u32) -> Result<()>;
}
pub fn actor_default_stance(actor: u32, world: &mut impl ActorStanceWorld) -> Result<()> {
    if let Some(character) = world.character(actor)? {
        return world.character_default_stance(character);
    }
    let old = world.active_component(actor)?;
    world.clear_active_component(actor)?;
    if let Some(old) = old {
        world.deactivate(old, 0)?;
    }
    if let Some(new) = world.active_component(actor)? {
        world.resume_component(new, 0)?;
    }
    Ok(())
}
