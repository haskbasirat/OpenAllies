//! Native OrganizationComponent morale and attack-efficiency rules.
//!
//! Ported routines (original AA.exe addresses):
//! - `578140` morale state classification (0 exhausted, 1 wavering, 2 steady),
//! - `5786D0` gain, `579660` loss, `579750` signed modify,
//! - `578F10` slayer gain and its dispatcher `4CEB40` (selection only, see
//!   [`slayer_organization`]),
//! - `579010` breaker gain, `579AB0` retreat penalty,
//! - `57A0A0` morale tick (replenish/supply bonus and engagement loss),
//! - `5785D0` attack-efficiency approach toward its desired value,
//! - `4CA050` MORALE_GAIN / MORALE_LOSS ability effect (selection via
//!   [`effect_organization`], application via [`apply_effect`]),
//! - `579600` engagement experience tick and `4CEBE0` capture reward.
//!
//! These are the engine's suppression mechanics: morale exhaustion sets actor
//! state 5 (`mind_exhausted`) and notifies the company Mind (`51F9F0`
//! exhausted, `51F920` wavering). The Mind reactions, actor state machine
//! (`4D64F0`), modifier lookup (`4CF110`), current state descriptor (`4D49B0`),
//! actor value (`4D01E0`) and experience award (`4CE770`) are required world
//! services. Settings are the resolved settings-cache values; they are
//! immutable while a call runs. Callbacks receive the live Organization and
//! may change it; later native reads observe those changes.
//!
//! Domain: finite binary32 fields and explicit x87 precision. Native division
//! by zero (a zero member-slot count, or a zero killer value returned only by
//! the fourth slayer value call) produces x87 infinities; this API returns the
//! arithmetic error instead. A morale state query that reaches the Mind with
//! no Mind present is a native null dereference and is returned as an error.
use crate::{Error, Result};
use aa_body::{
    F32,
    credit::Object,
    damage::DamageRecord,
    experience::Request,
    experience_amounts::{self, Kind},
    properties::ModifierState,
};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// OrganizationComponent fields used by these rules (native offsets).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    /// Owner actor handle (`+4`).
    pub owner: u32,
    /// `+5C` current morale.
    pub morale: F32,
    /// `+60` maximum morale.
    pub maximum: F32,
    /// `+B8` morale breakpoint.
    pub breakpoint: F32,
    /// `+64` signed living-member count used by engagement loss.
    pub living: i32,
    /// `+4C` previous-tick engaged latch.
    pub engaged_latch: bool,
    /// `+4D` previous-tick engagement-experience latch.
    pub engage_latch: bool,
    /// `+84` current attack efficiency.
    pub attack_efficiency: F32,
    /// `+88` desired attack efficiency.
    pub attack_efficiency_desired: F32,
    /// `+8D` byte read by `579490`: when zero, a busy Mind blocks reactive
    /// engagement (`578E90`).
    pub byte8d: bool,
    /// `+90` seven-word copy of the last element damage record (`579490`).
    pub last_damage: DamageRecord,
    /// `+40` leader element and `+44` primary element actors.
    pub leader: Option<u32>,
    pub primary: Option<u32>,
    /// `+48` regroup start time, `+B4` entrenchment base delay.
    pub regroup_started: F32,
    pub entrench_base: F32,
    /// `+B0` flare pending and its `+AC` argument word; `+8E` marker pending.
    pub flare_pending: bool,
    pub flare_word: u32,
    pub marker_pending: bool,
    /// `+D4` watched target GTID (0 = none).
    pub watched_target: u32,
    /// `+68` current and `+6C` maximum combat value (`5788B0`).
    pub current_combat_value: F32,
    pub maximum_combat_value: F32,
    /// `+D8`, compared by member removal (`57CD70`) with the removed
    /// member's `+6C` component `+28` value.
    pub field_d8: F32,
}

/// Resolved settings-cache values (`B39AB0` table offsets in comments).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// `+CC` MoraleEngagementLoss.
    pub engagement_loss: F32,
    /// `+D0` MoraleEngagementLossFactorMax.
    pub engagement_loss_factor_max: F32,
    /// `+D8` MoraleRetreatPenalty.
    pub retreat_penalty: F32,
    /// `+DC/+E0/+E4` MoraleSlayerBase/FactorMin/FactorMax.
    pub slayer_base: F32,
    pub slayer_min: F32,
    pub slayer_max: F32,
    /// `+E8/+EC/+F0` MoraleBreakerBase/FactorMin/FactorMax.
    pub breaker_base: F32,
    pub breaker_min: F32,
    pub breaker_max: F32,
    /// `+F4` MoraleReplenish and `+F8` MoraleSupplyBonus.
    pub replenish: F32,
    pub supply_bonus: F32,
    /// `+1AC/+1B0` AttackEfficiencyPointsPerSecondClimbing/Falling.
    pub attack_efficiency_climbing: F32,
    pub attack_efficiency_falling: F32,
    /// Experience settings `+160..+178` used by engagement/capture rewards.
    pub experience: experience_amounts::Settings,
}

/// Direct actor field views read at native points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    /// `+8`; bit0 is the destructed flag.
    pub flags8: u32,
    /// `+100`; bit4 engaged-in-combat, bit19 supply-bonus suppression.
    pub flags100: u32,
    /// `+104`; bit2 is tested by `4D1080`.
    pub flags104: u32,
    /// `4CFE90`: `+118` is nonnull.
    pub engaged: bool,
    /// `+7C` OrganizationComponent present.
    pub has_organization: bool,
    /// `+80` membership component; inner value is its `+10` organization actor.
    pub membership: Option<Option<u32>>,
}

/// Company Mind (`actor+70`) words `+28`/`+2C`; nonzero means busy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mind {
    pub word28: u32,
    pub word2c: u32,
}

/// `4D49B0` current state descriptor bytes `+1C/+1D/+1E`. Only `+1D`
/// (morale replenishment gate) is read by these rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDescriptor {
    pub byte1c: bool,
    pub byte1d: bool,
    pub byte1e: bool,
}

pub trait World {
    /// Native world `+D8`; mode 6 suppresses every morale loss.
    fn session_mode(&self) -> Result<u32>;
    fn actor(&self, actor: u32) -> Result<Actor>;
    fn mind(&self, actor: u32) -> Result<Option<Mind>>;
    /// Mind virtual `+5C` on the owner's Mind. True selects state 0.
    fn mind_declines(&mut self, organization: &mut Organization) -> Result<bool>;
    /// `51F9F0` on the owner's Mind after a transition to state 0.
    fn mind_exhausted(&mut self, organization: &mut Organization) -> Result<()>;
    /// `51F920` on the owner's Mind after a transition to state 1.
    fn mind_wavering(&mut self, organization: &mut Organization) -> Result<()>;
    /// `4D64F0(owner, state, enabled)` with all its live state effects.
    fn set_state(
        &mut self,
        organization: &mut Organization,
        state: u8,
        enabled: bool,
    ) -> Result<()>;
    /// `4CF110(owner)`.
    fn modifier(&mut self, organization: &mut Organization) -> Result<Option<ModifierState>>;
    /// `4D49B0(owner, group)`.
    fn state_group(
        &mut self,
        organization: &mut Organization,
        group: u32,
    ) -> Result<Option<StateDescriptor>>;
    /// `4D01E0(actor,0)` extended value.
    fn actor_value(
        &mut self,
        organization: &mut Organization,
        actor: u32,
        precision: Precision,
    ) -> Result<Binary>;
    /// Actor definition `+4F4`.
    fn definition4f4(&self, actor: u32) -> Result<F32>;
    /// Another actor's OrganizationComponent `+60` (`actor+7C`). Absent is an
    /// error: the native breaker dereferences it unconditionally.
    fn organization_maximum(&self, actor: u32) -> Result<F32>;
    /// Organization definition `+4C` (signed member-slot count).
    fn member_slots(&self, organization: &Organization) -> Result<i32>;
    /// Mind virtual `+90`: current engagement target actor.
    fn engage_target(&mut self, organization: &mut Organization) -> Result<Option<u32>>;
    /// Target actor virtual `+B4` used by `4C8330` eligibility.
    fn target_eligible(&mut self, actor: u32) -> Result<bool>;
    /// `4CE770(actor, amount, argument1, argument2)`.
    fn award(
        &mut self,
        organization: &mut Organization,
        actor: u32,
        request: Request,
    ) -> Result<()>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn zero() -> Binary {
    Binary::from_u32(0)
}
fn one() -> Binary {
    Binary::from_u32(1)
}

/// Native `578140`.
pub fn state(organization: &mut Organization, world: &mut impl World) -> Result<u8> {
    if let Some(mind) = world.mind(organization.owner)?
        && (mind.word28 != 0 || mind.word2c != 0)
    {
        return Ok(2);
    }
    if n(organization.maximum)?.compare(&zero()) != Ordering::Greater {
        return Ok(2);
    }
    if n(organization.morale)?.compare(&n(organization.breakpoint)?) == Ordering::Greater {
        return Ok(2);
    }
    if n(organization.morale)?.compare(&zero()) == Ordering::Equal {
        return Ok(0);
    }
    if world.mind(organization.owner)?.is_none() {
        return Err(Error::Invalid(
            "morale state query reached a null Mind (native null dereference)",
        ));
    }
    Ok(if world.mind_declines(organization)? {
        0
    } else {
        1
    })
}

/// Shared tail of `5786D0`/`579750`/`578F10`/`579010`: add, binary32 store,
/// clamp to the maximum, then clear state 5 unless the state is exhausted.
fn gain_amount(
    organization: &mut Organization,
    amount: Binary,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    organization.morale = F32(amount.add(&n(organization.morale)?, p)?.store_f32()?);
    if n(organization.maximum)?.compare(&n(organization.morale)?) == Ordering::Less {
        organization.morale = organization.maximum;
    }
    if state(organization, world)? != 0 {
        world.set_state(organization, 5, false)?;
    }
    Ok(())
}

/// Shared post-gate tail of `579660`/`579AB0`/`57A0A0`/`57CD70`.
pub(crate) fn lose_amount(
    organization: &mut Organization,
    amount: F32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let before = state(organization, world)?;
    organization.morale = F32(n(organization.morale)?.sub(&n(amount)?, p)?.store_f32()?);
    if n(organization.morale)?.compare(&zero()) == Ordering::Less {
        organization.morale = F32::ZERO;
    }
    let after = state(organization, world)?;
    if world.mind(organization.owner)?.is_some() && after != before {
        if after == 0 {
            world.mind_exhausted(organization)?;
            world.set_state(organization, 5, true)?;
            return Ok(());
        }
        if after == 1 {
            world.mind_wavering(organization)?;
            return Ok(());
        }
    }
    if after == 0 {
        world.set_state(organization, 5, true)?;
    }
    Ok(())
}

/// The loss gates shared by `579660`, `579AB0` and `57A0A0`. Only `579660`
/// lets kinds 3 and 4 bypass the modifier flag 8 (morale immunity) gate.
fn loss_allowed(
    organization: &mut Organization,
    kind: Option<u32>,
    world: &mut impl World,
) -> Result<bool> {
    if world.session_mode()? == 6 {
        return Ok(false);
    }
    let actor = world.actor(organization.owner)?;
    if actor.flags8 & 1 != 0 || actor.flags104 & 4 != 0 {
        return Ok(false);
    }
    if let Some(modifier) = world.modifier(organization)?
        && modifier.flags & 8 != 0
        && !matches!(kind, Some(3 | 4))
    {
        return Ok(false);
    }
    if let Some(mind) = world.mind(organization.owner)?
        && (mind.word28 != 0 || mind.word2c != 0)
    {
        return Ok(false);
    }
    Ok(true)
}

/// Native `5786D0`.
pub fn gain(
    organization: &mut Organization,
    amount: F32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    gain_amount(organization, n(amount)?, world, p)
}

/// Native `579660(amount, kind)`.
pub fn lose(
    organization: &mut Organization,
    amount: F32,
    kind: u32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    if loss_allowed(organization, Some(kind), world)? {
        lose_amount(organization, amount, world, p)?;
    }
    Ok(())
}

/// Native `579750`: nonpositive amounts are negated (exactly) and lost.
pub fn modify(
    organization: &mut Organization,
    amount: F32,
    kind: u32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    if n(amount)?.compare(&zero()) != Ordering::Greater {
        lose(organization, F32(amount.0 ^ 0x8000_0000), kind, world, p)
    } else {
        gain_amount(organization, n(amount)?, world, p)
    }
}

/// `4CEB40` selection: the killer's membership organization actor, when both
/// killer and victim have membership components. The caller then invokes
/// [`slayer`] on that actor's OrganizationComponent; nothing happens between.
pub fn slayer_organization(
    killer: u32,
    victim: u32,
    world: &mut impl World,
) -> Result<Option<u32>> {
    let Some(membership) = world.actor(killer)?.membership else {
        return Ok(None);
    };
    if world.actor(victim)?.membership.is_none() {
        return Ok(None);
    }
    Ok(membership)
}

/// Native `578F10(killer, victim)`. Four ordered actor-value calls: killer
/// (zero returns), killer (zero selects `2*max`), victim (binary32 store)
/// and killer (divisor). Min/max are cached before the later calls.
pub fn slayer(
    organization: &mut Organization,
    killer: u32,
    victim: u32,
    settings: &Settings,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    if world.actor_value(organization, killer, p)?.compare(&zero()) == Ordering::Equal {
        return Ok(());
    }
    let (minimum, maximum) = (settings.slayer_min, settings.slayer_max);
    let ratio = if world.actor_value(organization, killer, p)?.compare(&zero()) == Ordering::Equal {
        n(maximum)?.add(&n(maximum)?, p)?
    } else {
        let victim_value = F32(world.actor_value(organization, victim, p)?.store_f32()?);
        let killer_value = world.actor_value(organization, killer, p)?;
        n(victim_value)?.div(&killer_value, p)?
    };
    let half = Binary::from_f32_bits(0x3f00_0000)?;
    let mut factor = ratio.add(&one(), p)?.mul(&half, p)?;
    if factor.compare(&n(minimum)?) == Ordering::Less {
        factor = n(minimum)?;
    } else if factor.compare(&n(maximum)?) == Ordering::Greater {
        factor = n(maximum)?;
    }
    let amount = factor.mul(&n(settings.slayer_base)?, p)?;
    gain_amount(organization, amount, world, p)
}

/// Native `579010(routed_actor)`: ratio of the routed organization's
/// maximum morale to this organization's maximum, clamped, times the base.
pub fn breaker(
    organization: &mut Organization,
    routed: u32,
    settings: &Settings,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    if n(organization.maximum)?.compare(&zero()) == Ordering::Equal {
        return Ok(());
    }
    let (minimum, maximum) = (settings.breaker_min, settings.breaker_max);
    let ratio = if n(organization.maximum)?.compare(&zero()) == Ordering::Equal {
        n(maximum)?.add(&n(maximum)?, p)?
    } else {
        n(world.organization_maximum(routed)?)?.div(&n(organization.maximum)?, p)?
    };
    let factor = if ratio.compare(&n(minimum)?) == Ordering::Less {
        n(minimum)?
    } else if ratio.compare(&n(maximum)?) == Ordering::Greater {
        n(maximum)?
    } else {
        ratio
    };
    let amount = factor.mul(&n(settings.breaker_base)?, p)?;
    gain_amount(organization, amount, world, p)
}

/// Native `579AB0`. The penalty is read before the gates; no kind exception.
pub fn retreat_penalty(
    organization: &mut Organization,
    settings: &Settings,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let penalty = settings.retreat_penalty;
    if loss_allowed(organization, None, world)? {
        lose_amount(organization, penalty, world, p)?;
    }
    Ok(())
}

/// Native `57A0A0(delta)`. The engaged latch is always rewritten with the
/// fresh `4CFE90` result after the optional loss, including after a Mind
/// wavering callback.
pub fn tick(
    organization: &mut Organization,
    delta: F32,
    settings: &Settings,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let descriptor = world.state_group(organization, 4)?;
    let modifier = world.modifier(organization)?;
    if let Some(descriptor) = descriptor
        && descriptor.byte1d
    {
        let mut rate = n(settings.replenish)?;
        if world.actor(organization.owner)?.flags100 & 0x80000 == 0 {
            rate = rate.add(&n(settings.supply_bonus)?, p)?;
        }
        let mut amount = F32(rate.mul(&n(delta)?, p)?.store_f32()?);
        if let Some(modifier) = &modifier {
            amount = F32(modifier
                .multiply(0x44, amount, p)
                .map_err(body)?
                .store_f32()?);
        }
        gain_amount(organization, n(amount)?, world, p)?;
    }
    let engaged = world.actor(organization.owner)?.engaged;
    if organization.engaged_latch && engaged {
        let slots = world.member_slots(organization)?;
        let fraction = Binary::from_i32(organization.living).div(&Binary::from_i32(slots), p)?;
        let mut loss = F32(n(settings.engagement_loss_factor_max)?
            .sub(&one(), p)?
            .mul(&one().sub(&fraction, p)?, p)?
            .add(&one(), p)?
            .mul(&n(settings.engagement_loss)?, p)?
            .mul(&n(delta)?, p)?
            .store_f32()?);
        if let Some(modifier) = &modifier {
            loss = F32(modifier.divide(0x48, loss, p).map_err(body)?.store_f32()?);
        }
        if loss_allowed(organization, None, world)? {
            lose_amount(organization, loss, world, p)?;
        }
    }
    organization.engaged_latch = engaged;
    Ok(())
}

/// Native `5785D0(delta)`: move the current attack efficiency toward the
/// desired value without overshooting it. Equal values are left untouched.
pub fn attack_efficiency(
    organization: &mut Organization,
    delta: F32,
    settings: &Settings,
    p: Precision,
) -> Result<()> {
    let actual = n(organization.attack_efficiency)?;
    let desired = n(organization.attack_efficiency_desired)?;
    match actual.compare(&desired) {
        Ordering::Equal => {}
        Ordering::Greater => {
            let step = n(delta)?.mul(&n(settings.attack_efficiency_falling)?, p)?;
            organization.attack_efficiency = F32(actual.sub(&step, p)?.store_f32()?);
            if desired.compare(&n(organization.attack_efficiency)?) == Ordering::Greater {
                organization.attack_efficiency = organization.attack_efficiency_desired;
            }
        }
        Ordering::Less => {
            let step = n(delta)?.mul(&n(settings.attack_efficiency_climbing)?, p)?;
            organization.attack_efficiency = F32(step.add(&actual, p)?.store_f32()?);
            if desired.compare(&n(organization.attack_efficiency)?) == Ordering::Less {
                organization.attack_efficiency = organization.attack_efficiency_desired;
            }
        }
    }
    Ok(())
}

/// Effect fields read by `4CA050`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoraleEffect {
    /// Effect `+4`: 2 is MORALE_GAIN; every other value takes the loss path.
    pub kind: u32,
    /// Effect `+14`.
    pub amount: F32,
}

/// `4CA050` selection: the effect applies to the organization actor of an
/// eligible target (`4C8330`, then `4CE6C0`), when it has an
/// OrganizationComponent. `None` is the native `false` return.
pub fn effect_organization(target: Option<u32>, world: &mut impl World) -> Result<Option<u32>> {
    let Some(target) = target else {
        return Ok(None);
    };
    if !world.target_eligible(target)? {
        return Ok(None);
    }
    let organization_actor = organization_actor(target, world)?;
    Ok(world
        .actor(organization_actor)?
        .has_organization
        .then_some(organization_actor))
}

/// Native `4CE6C0`: an actor without its own OrganizationComponent but with
/// a membership organization resolves to that organization actor.
pub fn organization_actor(actor: u32, world: &impl World) -> Result<u32> {
    let view = world.actor(actor)?;
    if !view.has_organization
        && let Some(Some(organization)) = view.membership
    {
        return Ok(organization);
    }
    Ok(actor)
}

/// `4CA050` application after [`effect_organization`] returned an actor.
pub fn apply_effect(
    organization: &mut Organization,
    effect: MoraleEffect,
    scale: F32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let amount = F32(n(scale)?.mul(&n(effect.amount)?, p)?.store_f32()?);
    if effect.kind == 2 {
        gain(organization, amount, world, p)
    } else {
        lose(organization, amount, 0, world, p)
    }
}

struct Values<'a, W> {
    world: &'a mut W,
    organization: &'a mut Organization,
}
impl<W: World> experience_amounts::World for Values<'_, W> {
    fn actor_value(&mut self, actor: Object, p: Precision) -> aa_body::Result<Binary> {
        self.world
            .actor_value(self.organization, actor.0, p)
            .map_err(|e| aa_body::Error::Callback(e.to_string()))
    }
    fn definition4f4(&self, actor: Object) -> aa_body::Result<F32> {
        self.world
            .definition4f4(actor.0)
            .map_err(|e| aa_body::Error::Callback(e.to_string()))
    }
}
fn body(error: aa_body::Error) -> Error {
    Error::World(format!("body service: {error}"))
}

/// Native `579600(delta)`: while the owner's `+100` bit4 is set, a second
/// consecutive engaged tick awards `5718B0(owner, target)*delta` (binary32)
/// through `4CE770(amount, 0, 0)`.
pub fn engagement_experience(
    organization: &mut Organization,
    delta: F32,
    settings: &Settings,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let owner = organization.owner;
    if world.actor(owner)?.flags100 & 0x10 == 0 {
        organization.engage_latch = false;
        return Ok(());
    }
    if organization.engage_latch && world.mind(owner)?.is_some() {
        let target = world.engage_target(organization)?;
        let rate = {
            let mut values = Values {
                world: &mut *world,
                organization: &mut *organization,
            };
            experience_amounts::engagement(
                Object(owner),
                target.map(Object),
                &settings.experience,
                &mut values,
                p,
            )
            .map_err(body)?
        };
        let amount = F32(rate.mul(&n(delta)?, p)?.store_f32()?);
        world.award(
            organization,
            owner,
            Request {
                amount,
                argument1: false,
                argument2: false,
            },
        )?;
    }
    organization.engage_latch = true;
    Ok(())
}

/// Services for `4CEBE0`, which operates on the capturing actor rather than
/// on an OrganizationComponent.
pub trait CaptureWorld: experience_amounts::World {
    fn actor(&self, actor: u32) -> Result<Actor>;
    fn has_mind(&self, actor: u32) -> Result<bool>;
    fn award(&mut self, actor: u32, request: Request) -> Result<()>;
    /// Mind virtual `+188(captured)`.
    fn notify_capture(&mut self, actor: u32, captured: u32) -> Result<()>;
}

/// Native `4CEBE0(captured, record)`. A record reason word (+18) of 7 skips the reward; the
/// Mind notification happens either way when the capturer has a Mind.
pub fn capture_reward(
    capturer: u32,
    captured: u32,
    record_kind: u32,
    settings: &experience_amounts::Settings,
    world: &mut impl CaptureWorld,
    p: Precision,
) -> Result<()> {
    if record_kind != 7 {
        let view = CaptureWorld::actor(world, capturer)?;
        let source = match view.membership {
            Some(Some(organization)) if !view.has_organization => organization,
            _ => capturer,
        };
        let amount = experience_amounts::clamped(
            Kind::Capture,
            Object(source),
            Some(Object(captured)),
            settings,
            world,
            p,
        )
        .map_err(body)?;
        world.award(
            capturer,
            Request {
                amount,
                argument1: false,
                argument2: true,
            },
        )?;
    }
    if world.has_mind(capturer)? {
        world.notify_capture(capturer, captured)?;
    }
    Ok(())
}

/// Services for the rout handler `4D3FA0`, which runs on the routed actor.
pub trait RoutWorld: experience_amounts::World {
    /// Name virtual `+24`, allegiance `+108` and `42D490(manager, 0xC, ...)`.
    fn rout_message(&mut self, routed: u32) -> Result<()>;
    /// Actor `+118` engaged-by GTID list, in node order. The walk reads each
    /// node's successor after its callbacks; list mutation by those callbacks
    /// (freed or inserted nodes) is outside this API's supported contract.
    fn engaged_by(&self, routed: u32) -> Result<Vec<u32>>;
    /// Generation-checked registry lookup.
    fn resolve(&self, gtid: u32) -> Result<Option<u32>>;
    /// Actor `+7C` OrganizationComponent present.
    fn has_organization(&self, actor: u32) -> Result<bool>;
    /// [`breaker`] (`579010`) on `attacker`'s OrganizationComponent.
    fn breaker(&mut self, attacker: u32, routed: u32) -> Result<()>;
    /// `4CE770(actor, amount, argument1, argument2)`.
    fn award(&mut self, actor: u32, request: Request) -> Result<()>;
}

/// Native `4D3FA0`: after the rout message, every live engaged-by attacker
/// gains breaker morale (when it is an organization) and is awarded the rout
/// experience `5718E0(attacker, routed)` with arguments `(0, 1)`.
pub fn rout(
    routed: u32,
    settings: &experience_amounts::Settings,
    world: &mut impl RoutWorld,
    p: Precision,
) -> Result<()> {
    world.rout_message(routed)?;
    for gtid in world.engaged_by(routed)? {
        let Some(attacker) = world.resolve(gtid)? else {
            continue;
        };
        if world.has_organization(attacker)? {
            world.breaker(attacker, routed)?;
        }
        let amount = experience_amounts::clamped(
            Kind::Rout,
            Object(attacker),
            Some(Object(routed)),
            settings,
            world,
            p,
        )
        .map_err(body)?;
        world.award(
            attacker,
            Request {
                amount,
                argument1: false,
                argument2: true,
            },
        )?;
    }
    Ok(())
}
