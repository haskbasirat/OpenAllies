//! Original Mover order/ground-step control flow (0x5773c0 / 0x5774a0).
//!
//! World collision, terrain-aware position commits, component activation and
//! Mind/Character notifications are explicit required boundaries. This crate
//! neither invents a route nor substitutes a collision solver. Original turning
//! uses quantized native tables and is also an explicit provider, never host
//! `sin`/`cos`. A complete actor/world integration must implement these contracts.
#![forbid(unsafe_code)]

pub mod falling;
pub mod formation;
pub mod mover;
pub mod target;

use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const MOVER_COMPONENT_SLOT: usize = 3;
pub const MIND_COMPONENT_SLOT: usize = 4;
pub const NATIVE_EPSILON_BITS: u32 = 0x38d1_b717;
pub const BYPASS_COLLISION_FLAG: u32 = 1 << 18;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoverState {
    /// Native +0x10; Z is explicitly zeroed by the ground-order operation.
    pub destination: [f32; 3],
    /// Native +0x1c, distinct from stored speed +0x20.
    pub last_step_speed: f32,
    pub speed: f32,
    pub requested_speed: f32,
    /// Native +0x2c.
    pub acceleration: f32,
    /// Native +0x34, initialized elsewhere from maximum speed / turn radius.
    pub turn_rate: f32,
    /// Native +0x38; provided by the native path/AI layer, not normalized here.
    pub desired_direction: [f32; 3],
    /// Native +0x44 generation-bearing actor handle, zero normally means none.
    pub target_id: u32,
    /// Native +48 is not initialized by576920. Ordinary ground orders do not
    /// consume it; target-range branches require an explicit written value.
    pub target_range: Option<f32>,
    /// Native +0x4c.
    pub ignore_terrain_speed_limit: bool,
    /// Native +0x50; multiplication is performed on every step.
    pub speed_factor: f32,
    /// Native +0x54: bypass gradual turn when set.
    pub instant_turn: bool,
    /// Native +0x55.
    pub turning: bool,
    /// Native +0x56; falling is handled by a separate routine, not ground step.
    pub falling: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorMotion {
    /// Native X/Y ground, Z up; no renderer-coordinate conversion.
    pub position: [f32; 3],
    pub facing: [f32; 3],
    /// Native actor+8 flags (bit18 collision bypass, bit17 facing lock),
    /// distinct from the body/property flag word at actor+0x100.
    pub flags: u32,
    pub has_mind: bool,
    pub has_character: bool,
    /// Character+0x24, tested before the resume callback.
    pub character_stopped: bool,
    /// Native actor+0xf8 pointee+0x38; interpretation supplied by actor/world layer.
    pub terrain_speed_limit: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepContext {
    /// The argument to 0x5774a0, used for acceleration only.
    pub acceleration_delta: f32,
    /// GWorld+0xe8, used for turning and displacement.
    pub world_delta: f32,
    pub terrain_minimum_move_speed: f32,
    /// Explicit x87 PC; no host or engine default is assumed.
    pub precision: Precision,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GroundOrder {
    pub direction: [f32; 2],
    pub destination: [f32; 2],
    pub requested_speed: f32,
    pub speed_factor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    ScheduleImmediate,
    CancelScheduled,
    DeactivateMover,
    /// Mind virtual+0x12c.
    DestinationReached,
    /// Mind virtual+0x130.
    TargetInRange,
    /// Mind virtual+0x134.
    TurnCompleted,
    /// The native variation/speed visual update; host may find no variation.
    SpeedChanged,
    CharacterStop,
    CharacterResume,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepKind {
    AlreadyAtDestination,
    TargetAlreadyInRange,
    Blocked,
    DestinationCrossed,
    TargetReachedAfterMove,
    Advanced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepOutcome {
    pub kind: StepKind,
    /// Initial shape-in-range returns the Mind callback's AL; other paths true.
    pub native_success: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("nonfinite movement state/input")]
    NonFinite,
    #[error("native target-in-range path requires a Mind component")]
    MissingMind,
    #[error("native Mover target range was read before initialization")]
    UninitializedTargetRange,
    #[error("ground step cannot process the falling state")]
    Falling,
    #[error("turning provider: {0}")]
    Turning(String),
    #[error("the concrete original turning provider supports only explicit x87 PC53")]
    UnsupportedOriginalTurningPrecision,
    #[error("world boundary failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Required original geometry/world boundaries, with no permissive defaults.
/// Callback mutations are visible at the same subsequent field reads as native.
/// `set_facing` owns actor flag bit17/dirty-notification semantics (0x4eebe0).
/// `move_on_ground` owns height flags, tile locking and spatial updates (0x4eeb40).
/// A boundary failure stops the enclosing operation at that call. Earlier native
/// field writes/callback effects remain applied; these operations do not roll
/// back and callers must not automatically retry the step.
pub trait GroundWorld {
    type Collision;
    fn activate(&mut self, mover: &mut MoverState, actor: &mut ActorMotion) -> Result<bool>;
    fn target_position(&mut self, id: u32) -> Result<Option<[f32; 2]>>;
    fn shapes_in_range(&mut self, actor: &ActorMotion, id: u32, range: f32) -> Result<bool>;
    fn collision_at(
        &mut self,
        actor: &ActorMotion,
        xy: [f32; 2],
    ) -> Result<Option<Self::Collision>>;
    /// Storage-aware form for a live world that projects canonical component
    /// fields into MoverState. Such an adapter publishes current writes before
    /// the callback and refreshes both snapshots afterwards, including errors.
    /// The default is valid when the borrowed state is already authoritative.
    fn target_position_with_state(
        &mut self,
        id: u32,
        _mover: &mut MoverState,
        _actor: &mut ActorMotion,
    ) -> Result<Option<[f32; 2]>> {
        self.target_position(id)
    }
    fn shapes_in_range_with_state(
        &mut self,
        id: u32,
        range: f32,
        _mover: &mut MoverState,
        actor: &mut ActorMotion,
    ) -> Result<bool> {
        self.shapes_in_range(actor, id, range)
    }
    fn collision_at_with_state(
        &mut self,
        xy: [f32; 2],
        _mover: &mut MoverState,
        actor: &mut ActorMotion,
    ) -> Result<Option<Self::Collision>> {
        self.collision_at(actor, xy)
    }
    fn set_facing(
        &mut self,
        mover: &mut MoverState,
        actor: &mut ActorMotion,
        xy: [f32; 2],
    ) -> Result<()>;
    fn move_on_ground(
        &mut self,
        mover: &mut MoverState,
        actor: &mut ActorMotion,
        xy: [f32; 2],
    ) -> Result<()>;
    fn notify(
        &mut self,
        event: Event,
        mover: &mut MoverState,
        actor: &mut ActorMotion,
    ) -> Result<bool>;
    /// Mind virtual+0x154; caller only invokes when the actor has a Mind.
    fn blocked(
        &mut self,
        collision: &Self::Collision,
        facing: [f32; 2],
        mover: &mut MoverState,
        actor: &mut ActorMotion,
    ) -> Result<()>;
}

/// 0x445400 and 0x4454a0/e0 use original CRT/lookup semantics. Implementations
/// must be independently grounded; standard host trigonometry is not a default.
pub trait GroundTurning {
    fn signed_angle(&mut self, from: [f32; 2], to: [f32; 2], precision: Precision) -> Result<f32>;
    fn rotate(
        &mut self,
        from: [f32; 2],
        amount: f32,
        counterclockwise: bool,
        precision: Precision,
    ) -> Result<[f32; 2]>;
}

/// Original quantized/CRT turning in the explicitly supported PC53 domain.
/// The CRT precision-exception branch is skipped under PC53, so an inherited
/// sticky PE bit does not select a different intermediate rounding path.
/// This is not a claim that native game/thread defaults are always PC53.
#[derive(Debug, Default)]
pub struct OriginalTurning53;
impl GroundTurning for OriginalTurning53 {
    fn signed_angle(&mut self, from: [f32; 2], to: [f32; 2], precision: Precision) -> Result<f32> {
        if precision != Precision::Double53 {
            return Err(Error::UnsupportedOriginalTurningPrecision);
        }
        aa_trig::signed_angle(
            from.map(f32::to_bits),
            to.map(f32::to_bits),
            precision,
            false,
        )
        .map(f32::from_bits)
        .map_err(|error| Error::Turning(error.to_string()))
    }
    fn rotate(
        &mut self,
        from: [f32; 2],
        amount: f32,
        counterclockwise: bool,
        precision: Precision,
    ) -> Result<[f32; 2]> {
        if precision != Precision::Double53 {
            return Err(Error::UnsupportedOriginalTurningPrecision);
        }
        aa_trig::rotate(
            from.map(f32::to_bits),
            amount.to_bits(),
            !counterclockwise,
            precision,
        )
        .map(|bits| bits.map(f32::from_bits))
        .map_err(|error| Error::Turning(error.to_string()))
    }
}

/// Original quantized/CRT turning for any explicit x87 precision. The CRT
/// asin (`0x96d780`) takes its qword round trip (`0x977837`) when PC is not 53
/// and the sticky precision-exception flag is set; that flag, as pending at the
/// `0x445400` call inside the ground step, is an explicit caller input rather
/// than an assumed engine state (phase4-trig-angle covers both flag states).
#[derive(Debug, Clone, Copy)]
pub struct OriginalTurning {
    pub precision_exception_pending: bool,
}
impl GroundTurning for OriginalTurning {
    fn signed_angle(&mut self, from: [f32; 2], to: [f32; 2], precision: Precision) -> Result<f32> {
        aa_trig::signed_angle(
            from.map(f32::to_bits),
            to.map(f32::to_bits),
            precision,
            self.precision_exception_pending,
        )
        .map(f32::from_bits)
        .map_err(|error| Error::Turning(error.to_string()))
    }
    fn rotate(
        &mut self,
        from: [f32; 2],
        amount: f32,
        counterclockwise: bool,
        precision: Precision,
    ) -> Result<[f32; 2]> {
        aa_trig::rotate(
            from.map(f32::to_bits),
            amount.to_bits(),
            !counterclockwise,
            precision,
        )
        .map(|bits| bits.map(f32::from_bits))
        .map_err(|error| Error::Turning(error.to_string()))
    }
}

/// Concrete original-trig ground step for any explicit precision, with the
/// sticky precision-exception state at the `0x445400` call supplied
/// explicitly (the simulation thread runs PC53 with PE set every tick).
pub fn ground_step_original<W: GroundWorld>(
    mover: &mut MoverState,
    actor: &mut ActorMotion,
    context: StepContext,
    precision_exception_pending: bool,
    world: &mut W,
) -> Result<StepOutcome> {
    ground_step(
        mover,
        actor,
        context,
        world,
        &mut OriginalTurning {
            precision_exception_pending,
        },
    )
}

/// Concrete original-trig ground step. Rejects PC24/64 before any mutation or
/// world callback, rather than guessing their preceding x87 exception state.
pub fn ground_step_original_pc53<W: GroundWorld>(
    mover: &mut MoverState,
    actor: &mut ActorMotion,
    context: StepContext,
    world: &mut W,
) -> Result<StepOutcome> {
    if context.precision != Precision::Double53 {
        return Err(Error::UnsupportedOriginalTurningPrecision);
    }
    ground_step(mover, actor, context, world, &mut OriginalTurning53)
}

fn b(value: f32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.to_bits())?)
}
fn stored(value: &Binary) -> Result<f32> {
    Ok(f32::from_bits(value.store_f32()?))
}
fn absolute(value: Binary) -> Binary {
    if value.is_negative_nonzero() {
        value.negated()
    } else {
        value
    }
}
fn xy(values: [f32; 3]) -> [f32; 2] {
    [values[0], values[1]]
}
fn validate(m: &MoverState, a: &ActorMotion) -> Result<()> {
    if m.destination
        .into_iter()
        .chain(m.desired_direction)
        .chain(a.position)
        .chain(a.facing)
        .chain([
            m.last_step_speed,
            m.speed,
            m.requested_speed,
            m.acceleration,
            m.turn_rate,
            m.speed_factor,
            a.terrain_speed_limit,
        ])
        .all(f32::is_finite)
    {
        Ok(())
    } else {
        Err(Error::NonFinite)
    }
}
fn squared_distance(a: [f32; 2], c: [f32; 2], pc: Precision) -> Result<Binary> {
    let dx = b(a[0])?.sub(&b(c[0])?, pc)?;
    let dy = b(a[1])?.sub(&b(c[1])?, pc)?;
    Ok(dx.mul(&dx, pc)?.add(&dy.mul(&dy, pc)?, pc)?)
}
fn target_range(m: &MoverState) -> Result<f32> {
    let range = m.target_range.ok_or(Error::UninitializedTargetRange)?;
    if !range.is_finite() {
        return Err(Error::NonFinite);
    }
    Ok(range)
}

/// Complete 0x5773c0 operation. The precision radius comes from SVar cache+0x54;
/// callers supply the recovered value instead of assuming a numeric default.
pub fn begin_ground_motion<W: GroundWorld>(
    m: &mut MoverState,
    a: &mut ActorMotion,
    order: GroundOrder,
    precision_radius: f32,
    pc: Precision,
    world: &mut W,
) -> Result<bool> {
    validate(m, a)?;
    if !order
        .direction
        .into_iter()
        .chain(order.destination)
        .chain([order.requested_speed, order.speed_factor, precision_radius])
        .all(f32::is_finite)
    {
        return Err(Error::NonFinite);
    }
    if m.falling || !world.activate(m, a)? {
        return Ok(false);
    }
    m.destination = [order.destination[0], order.destination[1], 0.0];
    m.target_id = 0;
    m.desired_direction = [order.direction[0], order.direction[1], 0.0];
    let radius = b(precision_radius)?;
    if squared_distance(xy(m.destination), xy(a.position), pc)?.compare(&radius.mul(&radius, pc)?)
        == Ordering::Less
    {
        // Deliberately retain old requested_speed and speed_factor in this path.
        m.speed = 0.0;
    } else {
        m.speed_factor = order.speed_factor;
        m.requested_speed = order.requested_speed;
    }
    world.notify(Event::ScheduleImmediate, m, a)?;
    Ok(true)
}

/// Complete 0x5767c0 stop operation; flags are native arguments, not guessed modes.
pub fn stop_ground_motion<W: GroundWorld>(
    m: &mut MoverState,
    a: &mut ActorMotion,
    keep_scheduled: bool,
    reset_speed: bool,
    world: &mut W,
) -> Result<()> {
    m.turning = false;
    if reset_speed {
        m.speed = 0.0;
    }
    m.last_step_speed = 0.0;
    if !keep_scheduled {
        world.notify(Event::CancelScheduled, m, a)?;
    }
    Ok(())
}

fn arrived<W: GroundWorld>(m: &mut MoverState, a: &mut ActorMotion, world: &mut W) -> Result<()> {
    world.notify(Event::CancelScheduled, m, a)?;
    world.notify(
        if a.has_mind {
            Event::DestinationReached
        } else {
            Event::DeactivateMover
        },
        m,
        a,
    )?;
    Ok(())
}

/// Entire native 0x5774a0 ground-step branch structure. The outer scheduler,
/// falling/aircraft dispatch and path/AI destination producer are not included.
pub fn ground_step<W: GroundWorld, T: GroundTurning>(
    m: &mut MoverState,
    a: &mut ActorMotion,
    context: StepContext,
    world: &mut W,
    turning: &mut T,
) -> Result<StepOutcome> {
    validate(m, a)?;
    if m.falling {
        return Err(Error::Falling);
    }
    if ![
        context.acceleration_delta,
        context.world_delta,
        context.terrain_minimum_move_speed,
    ]
    .into_iter()
    .all(f32::is_finite)
    {
        return Err(Error::NonFinite);
    }
    let pc = context.precision;
    let zero = b(0.0)?;
    let epsilon = Binary::from_f32_bits(NATIVE_EPSILON_BITS)?;
    let outcome = |kind| StepOutcome {
        kind,
        native_success: true,
    };
    if squared_distance(xy(m.destination), xy(a.position), pc)?.compare(&epsilon) == Ordering::Less
    {
        m.turning = false;
        m.speed = 0.0;
        m.last_step_speed = 0.0;
        arrived(m, a, world)?;
        return Ok(outcome(StepKind::AlreadyAtDestination));
    }
    if world.target_position_with_state(m.target_id, m, a)?.is_some()
        && world.shapes_in_range_with_state(m.target_id, target_range(m)?, m, a)?
    {
        if !a.has_mind {
            return Err(Error::MissingMind);
        }
        let native_success = world.notify(Event::TargetInRange, m, a)?;
        return Ok(StepOutcome {
            kind: StepKind::TargetAlreadyInRange,
            native_success,
        });
    }
    let mut facing = xy(a.facing);
    m.turning = false;
    if facing != xy(m.desired_direction) {
        let angle = turning.signed_angle(facing, xy(m.desired_direction), pc)?;
        let amount = stored(&b(context.world_delta)?.mul(&b(m.turn_rate)?, pc)?)?;
        if !angle.is_finite() {
            return Err(Error::NonFinite);
        }
        if amount != 0.0 && !m.instant_turn {
            if absolute(b(angle)?).compare(&absolute(b(amount)?)) != Ordering::Greater {
                facing = xy(m.desired_direction);
                if a.has_mind {
                    world.notify(Event::TurnCompleted, m, a)?;
                }
            } else {
                m.turning = true;
                let cross = b(facing[0])?.mul(&b(m.desired_direction[1])?, pc)?.add(
                    &b(facing[1])?
                        .negated()
                        .mul(&b(m.desired_direction[0])?, pc)?,
                    pc,
                )?;
                facing =
                    turning.rotate(facing, amount, cross.compare(&zero) != Ordering::Less, pc)?;
                if !facing.into_iter().all(f32::is_finite) {
                    return Err(Error::NonFinite);
                }
            }
        } else {
            facing = xy(m.desired_direction);
        }
        world.set_facing(m, a, facing)?;
    }
    if m.speed != m.requested_speed {
        if m.acceleration != 0.0 && m.requested_speed > m.speed {
            m.speed = stored(
                &b(context.acceleration_delta)?
                    .mul(&b(m.acceleration)?, pc)?
                    .add(&b(m.speed)?, pc)?,
            )?;
            if m.speed > m.requested_speed {
                m.speed = m.requested_speed;
            }
        } else {
            m.speed = m.requested_speed;
        }
    }
    if !m.ignore_terrain_speed_limit && a.terrain_speed_limit > 0.0 {
        let limit = a
            .terrain_speed_limit
            .max(context.terrain_minimum_move_speed);
        if limit < m.speed {
            m.speed = limit;
        }
    }
    // Native FST stores speed but retains the extended product for both the
    // displacement calculation and the change comparison. Do not reload it.
    let scaled = b(m.speed_factor)?.mul(&b(m.speed)?, pc)?;
    m.speed = stored(&scaled)?;
    let mut travel = stored(&scaled.mul(&b(context.world_delta)?, pc)?)?;
    if absolute(b(m.last_step_speed)?.sub(&scaled, pc)?).compare(&epsilon) != Ordering::Less {
        world.notify(Event::SpeedChanged, m, a)?;
    }
    m.last_step_speed = m.speed;
    let dx = b(m.destination[0])?.sub(&b(a.position[0])?, pc)?;
    let dy = b(m.destination[1])?.sub(&b(a.position[1])?, pc)?;
    let projection = stored(
        &dy.mul(&b(m.desired_direction[1])?, pc)?
            .add(&dx.mul(&b(m.desired_direction[0])?, pc)?, pc)?,
    )?;
    let crossed = projection <= travel;
    if crossed && projection > 0.0 && xy(m.desired_direction) == facing && projection < travel {
        travel = projection;
    }
    let candidate = [
        stored(
            &b(facing[0])?
                .mul(&b(travel)?, pc)?
                .add(&b(a.position[0])?, pc)?,
        )?,
        stored(
            &b(travel)?
                .mul(&b(facing[1])?, pc)?
                .add(&b(a.position[1])?, pc)?,
        )?,
    ];
    if a.flags & BYPASS_COLLISION_FLAG == 0
        && let Some(collision) = world.collision_at_with_state(candidate, m, a)?
    {
        if a.has_character {
            world.notify(Event::CharacterStop, m, a)?;
        }
        if a.has_mind {
            world.blocked(&collision, facing, m, a)?;
        }
        m.last_step_speed = 0.0;
        return Ok(outcome(StepKind::Blocked));
    }
    if a.has_character && a.character_stopped {
        world.notify(Event::CharacterResume, m, a)?;
    }
    world.move_on_ground(m, a, candidate)?;
    m.last_step_speed = m.speed;
    if crossed {
        m.turning = false;
        m.last_step_speed = 0.0;
        arrived(m, a, world)?;
        return Ok(outcome(StepKind::DestinationCrossed));
    }
    if let Some(position) = world.target_position_with_state(m.target_id, m, a)? {
        let range = b(target_range(m)?)?;
        if range
            .mul(&range, pc)?
            .compare(&squared_distance(position, xy(a.position), pc)?)
            != Ordering::Less
        {
            if !a.has_mind {
                return Err(Error::MissingMind);
            }
            world.notify(Event::TargetInRange, m, a)?;
            return Ok(outcome(StepKind::TargetReachedAfterMove));
        }
    }
    Ok(outcome(StepKind::Advanced))
}
pub mod aircraft;
pub mod borrowed_formation;
