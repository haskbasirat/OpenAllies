//! Ground component InitCreate leaf methods and real periodic queue insertion.
//! World/component identity allocation remains caller owned; queue owners must
//! uniquely identify the actual Updatable, never just its actor.
use crate::components::ComponentSlot;
use aa_body::{Body, Environment, F32, HealthChange, Host};
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use aa_world::queues::{Owner, Queues};
use std::cmp::Ordering;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Body(#[from] aa_body::Error),
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Queue(#[from] aa_world::queues::Error),
    #[error("component {0:?} requires its concrete InitCreate implementation")]
    Unsupported(ComponentSlot),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug)]
pub struct CreationRecord {
    pub word40: u32,
    pub word48: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct BodySupply {
    /// Resolved original UnitSupplyEnable SVar; no compiled default assumed.
    pub unit_supply_enable: bool,
    /// Actual actor+7C Organization component presence.
    pub has_organization: bool,
    /// Definition+2A8 BodyComponent.unit_shortfall_damage_rate.
    pub unit_shortfall_damage_rate: F32,
}
/// Native 561B40; source definition/SVar are stable during this call. The float is read
/// only after the two original short-circuits have passed.
pub fn body_supply_enabled(input: BodySupply) -> Result<bool> {
    if !input.unit_supply_enable || input.has_organization {
        return Ok(false);
    }
    Ok(
        Binary::from_f32_bits(input.unit_shortfall_damage_rate.0)?.compare(&Binary::from_i32(0))
            == Ordering::Greater,
    )
}
/// Native 562310. Body health callbacks are synchronous and use the existing native
/// 561E50 port. Queue1 insertion executes the recovered50B2C0/RNG semantics.
/// The creation record and source supply inputs are immutable construction data;
/// callbacks may modify Body/Environment through Host's supported contract.
#[allow(clippy::too_many_arguments)]
pub fn body(
    body: &mut Body,
    env: &mut Environment,
    host: &mut impl Host,
    record: Option<CreationRecord>,
    supply: BodySupply,
    owner: Owner,
    queues: &mut Queues,
    rng: &mut NativeRng,
    pc: Precision,
) -> Result<bool> {
    if env.actor_flags & (1 << 21) != 0 {
        body.set_health_from_source(
            HealthChange {
                requested: F32::ONE,
                token: 0,
                notify: true,
                source_id: None,
            },
            env,
            host,
            pc,
        )?;
    }
    body_queue_after_health(record, supply, owner, queues, rng)
}
///562338..562374 after the optional health callback. A runtime with proved
///state21 clear can execute this exact suffix without inventing a Body host.
pub fn body_queue_after_health(
    record: Option<CreationRecord>,
    supply: BodySupply,
    owner: Owner,
    queues: &mut Queues,
    rng: &mut NativeRng,
) -> Result<bool> {
    if let Some(record) = record
        && body_supply_enabled(supply)?
        && record.word40 == 0
        && record.word48 == 0
        && !queues.has(owner, 1, 0)
    {
        queues.add_periodic(1, owner, 0, rng)?;
    }
    Ok(true)
}
/// Proven InitCreate methods for ordinary ground support components. Secondary
/// Updatable+8 is native6A71D0 (true) for every scheduling entry supported here.
/// Character/Mover/Experience's native438AC0 returns true without scheduling.
/// Performer, Mind and other component-specific behavior are explicit errors.
pub fn ground_support(
    slot: ComponentSlot,
    supply_required: bool,
    owner: Owner,
    queues: &mut Queues,
    rng: &mut NativeRng,
) -> Result<bool> {
    use ComponentSlot::*;
    match slot {
        Character | Mover | Experience => {}
        Properties | Organization | Element | Detection => {
            queues.add_periodic(1, owner, 0, rng)?;
        }
        Economy => {
            if supply_required {
                queues.add_periodic(1, owner, 0, rng)?;
            }
        }
        _ => return Err(Error::Unsupported(slot)),
    }
    Ok(true)
}

/// Constant native requirement virtuals for the verified ground constructors.
pub fn ground_requirement(kind: crate::init_create::FactoryKind) -> Result<Option<ComponentSlot>> {
    use crate::init_create::FactoryKind::*;
    use ComponentSlot::*;
    Ok(match kind {
        OrganizationMind => Some(Mind),
        AircraftMind => Some(Aircraft),
        ElementMind => Some(Element),
        Standard(Construction) => Some(Body), //4C9FA0
        Standard(Experience) => Some(Properties),
        Standard(Builder) => Some(Mind), //7C53E0
        Standard(Defender | Aircraft) => Some(Performer), //6B2450
        Standard(Transport) => Some(Mover), //7C7840 returns native slot3
        Standard(
            Body | Projectile | Character | Mover | Performer | Properties | Organization | Element | Economy
            | Denizen | Supply | Wall | Settlement | Recruit | MobileDeployment | Research | Detection
            | Lair | Flare | MetaGame | Portal | City | Hq,
        ) => None,
        Standard(slot) => return Err(Error::Unsupported(slot)),
    })
}

///589090's nonnull-owner Structure requirement.4D1050 reads the current
///Actor100/104 words; its returned predicate is discarded before-1. Callers
///must supply that SAME owner state; absent state is not a fake empty Actor.
pub fn ground_requirement_with_structure_state(
    kind:crate::init_create::FactoryKind,
    owner_state:Option<crate::components::StateBits>,
)->Result<Option<ComponentSlot>> {
    if kind==crate::init_create::FactoryKind::Standard(ComponentSlot::Structure) {
        let state=owner_state.ok_or(Error::Unsupported(ComponentSlot::Structure))?;
        let _native_predicate=state.low&(1<<26)!=0 || state.high&(1<<4)!=0;
        Ok(None)
    } else {ground_requirement(kind)}
}
