//! Special operations: Commander (Faction) abilities and the ability-effect
//! Apply virtuals they reach.
//!
//! * `KKC_FactionUseAbilityOrder` (61E190, orders owner) calls
//!   [`use_ability`] = `KKC_Faction::UseAbility` 4CDE00, which admits through
//!   4C8F20 ([`crate::admission::can_use`]) and uses through 4C9EF0 ->
//!   4C9550 ([`crate::execution::use_ability`]) at world time.
//! * Effect Apply is vtable +8 of each effect class (called by 4C9550 and the
//!   area loop 4C9C90 with the ability, the context and the attenuation
//!   scale); `execution::World::other_effect` binds these per kind:
//!   - MORALE_LOSS/MORALE_GAIN 4CA050: `morale::{effect_organization, apply_effect}`;
//!   - MAIN_DAMAGE/ADD_DAMAGE 4CA400: the execution hit path;
//!   - HEAL 4CA0B0: [`apply_heal`] (with [`heal_modifier`] 501B70);
//!   - ENCHANTMENT 4CA130: [`apply_enchantment`] (with [`add_enchantment`] 581700);
//!   - CAPTURE 4CA260: [`apply_capture`];
//!   - STEAL_RESOURCES 4CA680: `aa_economy::transfers::steal_resources`
//!     after the target kingdom (virtual +108) resolved;
//!   - DESCRIBE_ECONOMY 4CC050: [`apply_describe_economy`];
//!   - SUMMON 4CCA70: [`crate::summon`].
//!
//! Proven by research/phase8-combat/native_special_ops_effects.py.
use crate::{
    Error, Result,
    capture::{self, AttemptWorld},
    commander,
    performer::Context,
};
use aa_body::{F32, damage::DamageRecord};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Services of 4CDE00.
pub trait FactionWorld {
    /// 4C8F20 on the faction ability `index` with check_range = true.
    fn can_use(
        &mut self,
        state: &mut commander::State,
        index: usize,
        context: &mut Context,
    ) -> Result<u32>;
    /// 4C9550 on that ability with `time`; the native int result.
    fn use_ability(
        &mut self,
        state: &mut commander::State,
        index: usize,
        context: &mut Context,
        time: F32,
    ) -> Result<i32>;
    /// `[B36534]+E0` world time (read by 4C9EF0).
    fn world_time(&self) -> Result<F32>;
    /// Target actor `+20` position.
    fn actor_position(&self, actor: u32) -> Result<[F32; 3]>;
    /// 404F60 "Faction '%s' unable to use ability '%s'" (diagnostic only).
    fn unable(&mut self, state: &commander::State, definition: u32) -> Result<()>;
}

/// Native `KKC_Faction::UseAbility` 4CDE00(definition, target, &point): the
/// first faction ability (+10/+14) whose definition (+4) matches; a
/// kind-3 context (4C8300) from the faction's kingdom (+8, source and
/// allegiance) at the target's x/y (z 0) or the point; admission with the
/// range check; then 4C9EF0 (4C9550 at world time). True when the use
/// returned a positive value.
pub fn use_ability(
    state: &mut commander::State,
    definition: u32,
    target: Option<u32>,
    point: [F32; 2],
    world: &mut impl FactionWorld,
) -> Result<bool> {
    let Some(index) = state
        .abilities
        .iter()
        .position(|a| a.definition == definition)
    else {
        return Ok(false);
    };
    let mut context = Context::new(3);
    context.source = Some(state.kingdom);
    context.allegiance = Some(state.kingdom);
    context.target = target;
    let xy = match target {
        Some(actor) => {
            let p = world.actor_position(actor)?;
            [p[0], p[1]]
        }
        None => point,
    };
    context.target_position = [xy[0], xy[1], F32::ZERO];
    if world.can_use(state, index, &mut context)? != 0 {
        world.unable(state, definition)?;
        return Ok(false);
    }
    let time = world.world_time()?;
    Ok(world.use_ability(state, index, &mut context, time)? > 0)
}

/// Native 501B70(modifier; amount): `binary32(amount + modifier+14)`, at
/// least `min(amount, cap)` where cap is `*[B39AB0]+1A4`, and never below
/// zero. The value is binary32-exact (returned in ST0).
pub fn heal_modifier(amount: F32, bonus: F32, cap: F32, p: Precision) -> Result<F32> {
    let mut local = F32(n(amount)?.add(&n(bonus)?, p)?.store_f32()?);
    let floor = if n(amount)?.compare(&n(cap)?) == Ordering::Less {
        amount
    } else {
        cap
    };
    if n(local)?.compare(&n(floor)?) == Ordering::Less {
        local = floor;
    }
    if n(local)?.compare(&Binary::from_i32(0)) == Ordering::Less {
        return Ok(F32::ZERO);
    }
    Ok(local)
}

/// Target resolution shared by the Apply virtuals.
pub trait EffectTarget {
    /// 4C8330 on the context: the eligible target actor (virtual +B4).
    fn effect_target(&mut self, context: &Context) -> Result<Option<u32>>;
}

/// Services of 4CA0B0.
pub trait HealWorld: EffectTarget {
    /// Actor `+60` Body present.
    fn has_body(&self, actor: u32) -> Result<bool>;
    /// 4CF110 on the context source (`+8`).
    fn source_modifier(&mut self, source: Option<u32>) -> Result<Option<u32>>;
    /// Modifier `+14` (heal bonus).
    fn heal_bonus(&self, modifier: u32) -> Result<F32>;
    /// `*[B39AB0]+1A4`.
    fn heal_cap(&self) -> Result<F32>;
    /// 562BA0 on the target's Body with the record `{source, amount}`
    /// (bind `aa_body::Body::heal_plain`).
    fn heal(&mut self, target: u32, source: Option<u32>, amount: F32) -> Result<()>;
}

/// Native HealEffect Apply 4CA0B0(ability, context, scale): an eligible
/// target with a Body; amount = effect+14 (through 501B70 when the source
/// has a modifier) times scale, stored binary32; then 562BA0.
pub fn apply_heal(
    amount: F32,
    context: &Context,
    scale: F32,
    world: &mut impl HealWorld,
    p: Precision,
) -> Result<bool> {
    let Some(target) = world.effect_target(context)? else {
        return Ok(false);
    };
    if !world.has_body(target)? {
        return Ok(false);
    }
    let base = match world.source_modifier(context.source)? {
        Some(modifier) => {
            let bonus = world.heal_bonus(modifier)?;
            heal_modifier(amount, bonus, world.heal_cap()?, p)?
        }
        None => amount,
    };
    let healed = F32(n(base)?.mul(&n(scale)?, p)?.store_f32()?);
    world.heal(target, context.source, healed)?;
    Ok(true)
}

/// Services of 4CA130 / 581700.
pub trait EnchantmentWorld: EffectTarget {
    /// Actor `+78` Properties component.
    fn properties_component(&self, actor: u32) -> Result<Option<u32>>;
    /// 581280(property, 0): any instance of `property` (5032D0 without a
    /// source filter; bind `aa_body::property_api::find_instance`).
    fn has_property(&mut self, component: u32, property: u32) -> Result<bool>;
    /// Owner (component `+4`) virtual `+138(allegiance)`.
    fn allied(&mut self, owner: u32, allegiance: Option<u32>) -> Result<bool>;
    /// Property `+64` enchantment type.
    fn enchantment_type(&self, property: u32) -> Result<u32>;
    /// 501610 on the component's `+1C` modifier state
    /// (`aa_body::properties::ModifierState::immune_to`).
    fn immune(&self, component: u32, enchantment_type: u32) -> Result<bool>;
    /// `[B36534]+E0`.
    fn world_time(&self) -> Result<F32>;
    /// 503140 (0x1C instance, vtable A2EBE0: +4 time, +8 = 1, +C property,
    /// +10 byte 0, +14 source GTID, +18 allegiance GTID) then 581630.
    fn insert_enchantment(
        &mut self,
        component: u32,
        property: u32,
        source: Option<u32>,
        allegiance: Option<u32>,
        time: F32,
    ) -> Result<()>;
}

/// Native 581700(property, source, allegiance) on `component` (owned by
/// `owner`): an enemy (owner virtual +138 false) immune to the property's
/// enchantment type is skipped; otherwise a new instance is inserted.
/// Returns whether it inserted.
pub fn add_enchantment(
    component: u32,
    owner: u32,
    property: u32,
    source: Option<u32>,
    allegiance: Option<u32>,
    world: &mut impl EnchantmentWorld,
) -> Result<bool> {
    if !world.allied(owner, allegiance)? {
        let kind = world.enchantment_type(property)?;
        if world.immune(component, kind)? {
            return Ok(false);
        }
    }
    let time = world.world_time()?;
    world.insert_enchantment(component, property, source, allegiance, time)?;
    Ok(true)
}

/// Native EnchantmentEffect Apply 4CA130(ability, context, scale): an
/// eligible target with Properties that does not already hold the
/// property (from any source) gets 581700 with the context source and
/// allegiance; scale is unused. True whenever 581700 was reached.
pub fn apply_enchantment(
    property: u32,
    context: &Context,
    world: &mut impl EnchantmentWorld,
) -> Result<bool> {
    let Some(target) = world.effect_target(context)? else {
        return Ok(false);
    };
    let Some(component) = world.properties_component(target)? else {
        return Ok(false);
    };
    if world.has_property(component, property)? {
        return Ok(false);
    }
    add_enchantment(
        component,
        target,
        property,
        context.source,
        context.allegiance,
        world,
    )?;
    Ok(true)
}

/// The record 4CA260 passes to 4D5230: event kind 1, reason 7, rest zero.
pub const CAPTURE_EFFECT_RECORD: DamageRecord = DamageRecord {
    source_id: 0,
    allegiance_id: 0,
    damage_type: 0,
    amount: F32::ZERO,
    time: F32::ZERO,
    event_kind: 1,
    reason: 7,
};

/// Native CaptureEffect Apply 4CA260(ability, context, scale): the eligible
/// target's 4D5230 capture attempt by the context source; its result.
pub fn apply_capture<W: AttemptWorld + EffectTarget>(
    context: &Context,
    world: &mut W,
) -> Result<bool> {
    let Some(target) = world.effect_target(context)? else {
        return Ok(false);
    };
    capture::attempt(target, context.source, &CAPTURE_EFFECT_RECORD, world)
}

/// Services of 4CC050.
pub trait DescribeWorld: EffectTarget {
    /// Target virtual `+108` (its kingdom).
    fn kingdom_of(&mut self, actor: u32) -> Result<Option<u32>>;
    /// 404F60 "Describe economy ability effect failed: target has no kingdom".
    fn no_kingdom(&mut self) -> Result<()>;
    /// 4E7200 economy text of `kingdom`, 4B9F40 "ability_effect_describe_economy|ss"
    /// with the kingdom name (virtual +24) and that text, then
    /// 40BF70(B399B0, 0xF, allegiance, 0, message). UI/economy owners.
    fn post_economy_description(&mut self, kingdom: u32, allegiance: Option<u32>) -> Result<()>;
}

/// Native DescribeEconomyEffect Apply 4CC050(ability, context, scale).
pub fn apply_describe_economy(context: &Context, world: &mut impl DescribeWorld) -> Result<bool> {
    let kingdom = match world.effect_target(context)? {
        Some(target) => world.kingdom_of(target)?,
        None => None,
    };
    let Some(kingdom) = kingdom else {
        world.no_kingdom()?;
        return Ok(false);
    };
    world.post_economy_description(kingdom, context.allegiance)?;
    Ok(true)
}

/// Guard for callers mapping a missing faction ability to an error.
pub fn require_ability(state: &commander::State, definition: u32) -> Result<usize> {
    state
        .abilities
        .iter()
        .position(|a| a.definition == definition)
        .ok_or(Error::Invalid(
            "faction has no ability with this definition",
        ))
}

/// Property-instance expiry: instance vtable slot 0, called by the
/// Properties tick 5818D0 (binds `periodic::World::expired`). Enchantment
/// instances (503140, vtable A2EBE0 -> 5031B0) expire once the extended
/// `world time - created` is not below the property's `+60` duration (an
/// unordered comparison does not expire); plain instances (5030F0, vtable
/// A29678 -> 6A71F0) never expire.
pub fn enchantment_expired(now: F32, created: F32, duration: F32, p: Precision) -> Result<bool> {
    let elapsed = n(now)?.sub(&n(created)?, p)?;
    Ok(elapsed.compare(&n(duration)?) != Ordering::Less)
}
