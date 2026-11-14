//! Original timed actor state 26 of a component whose +4 is its actor
//! (`575B20` / `575A30`), and the ratio helper `571700`.
//!
//! * `575B20` on the component: `4D1350` on the actor; then, with actor
//!   +100 bit 25 (0x2000000) set, the unstored health fraction
//!   `4D0230(0)` ([`crate::health_fraction`]) not above the definition
//!   (actor +4) +54C threshold (or unordered) runs `575A30`.
//! * `575A30` on the component: its timer list (+8) is cleared (`50B250`),
//!   actor state 26 is set (`4D64F0(0x1A, 1)`), and the delay
//!   `stored(def+550 - (deadline - time))` (deadline = component +14, time
//!   = world +E0, both differences unstored at the thread precision) is
//!   capped at definition +554; `50ACC0(delay, 0)` schedules it on the list
//!   and the deadline becomes `stored(delay + time)`.
//! * `571700(a, b)`: `(b / a) * 0.5 + 0.5` unstored when `a` is above 0.0,
//!   else 0.0.
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error("timed state input unavailable: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Actor +100 bit tested by `575B20`.
pub const TIMED_FLAG: u32 = 0x0200_0000;
/// The actor state `575A30` sets.
pub const TIMED_STATE: u32 = 0x1a;
/// `[9FBDB8]` binary32 0.5.
const HALF: u32 = 0x3f00_0000;

/// Services of `575B20` / `575A30`, in native order.
pub trait TimedStateWorld {
    /// `4D1350` on the actor.
    fn actor_refresh(&mut self) -> Result<()>;
    /// Actor +100.
    fn actor_flags(&mut self) -> Result<u32>;
    /// Actor definition (+4) word at `offset` (+54C, +550, +554).
    fn definition_word(&mut self, offset: u32) -> Result<u32>;
    /// `4D0230(0)` on the actor (ST0, unstored).
    fn health_fraction(&mut self) -> Result<Binary>;
    /// `50B250` on the component timer list (+8).
    fn clear_timers(&mut self) -> Result<()>;
    /// `4D64F0(state, on)` on the actor.
    fn set_actor_state(&mut self, state: u32, on: bool) -> Result<()>;
    /// World (`[B36534]`) +E0 time.
    fn time(&mut self) -> Result<u32>;
    /// Component +14 (deadline) and its write.
    fn deadline(&mut self) -> Result<u32>;
    fn set_deadline(&mut self, bits: u32) -> Result<()>;
    /// `50ACC0(delay, data)` on the component timer list (+8).
    fn add_timer(&mut self, delay_bits: u32, data: u32) -> Result<()>;
}

fn n(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

/// Complete `575A30`.
pub fn start_timed_state(world: &mut impl TimedStateWorld, p: Precision) -> Result<()> {
    world.clear_timers()?;
    world.set_actor_state(TIMED_STATE, true)?;
    let left = n(world.deadline()?)?.sub(&n(world.time()?)?, p)?;
    let mut delay = n(world.definition_word(0x550)?)?.sub(&left, p)?.store_f32()?;
    let cap = world.definition_word(0x554)?;
    // fcomp: neither C0 nor C3 (above the cap) takes the cap.
    if n(delay)?.compare(&n(cap)?) == Ordering::Greater {
        delay = n(cap)?.store_f32()?;
    }
    world.add_timer(delay, 0)?;
    let deadline = n(delay)?.add(&n(world.time()?)?, p)?.store_f32()?;
    world.set_deadline(deadline)
}

/// Complete `575B20`; returns true when `575A30` ran.
pub fn update_timed_state(world: &mut impl TimedStateWorld, p: Precision) -> Result<bool> {
    world.actor_refresh()?;
    if world.actor_flags()? & TIMED_FLAG == 0 {
        return Ok(false);
    }
    let threshold = world.definition_word(0x54c)?;
    if world.health_fraction()?.compare(&n(threshold)?) == Ordering::Greater {
        return Ok(false);
    }
    start_timed_state(world, p)?;
    Ok(true)
}

/// `571700(a, b)`, unstored.
pub fn ratio(a: u32, b: u32, p: Precision) -> Result<Binary> {
    if n(a)?.compare(&Binary::from_i32(0)) != Ordering::Greater {
        return Ok(Binary::from_i32(0));
    }
    let half = n(HALF)?;
    Ok(n(b)?.div(&n(a)?, p)?.mul(&half, p)?.add(&half, p)?)
}
