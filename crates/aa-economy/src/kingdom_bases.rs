//! Original Kingdom nearest home base `4E1D90` (thiscall Kingdom, `(actor)`,
//! `ret 4`), read by the aircraft join/landing Thinks through actor virtual
//! +108 (the owner Kingdom).
//!
//! A null actor, or one whose `+8` bit 0 (removed) is set, yields null. The
//! Kingdom `+284` linked list (node `+0` actor, `+4` next) is walked in list
//! order; a node is a candidate when its actor is nonnull, its `+8` bit 0 is
//! clear and its Denizen component `+88` is nonnull. The Denizen is asked
//! whether it takes the actor's `+4` word: `568120` when the candidate's
//! `+C0` word is nonzero, else `568EA0`. For an accepting candidate the
//! squared planar distance `dx*dx + dy*dy` (`dy` = candidate `+24` - actor
//! `+24`, `dx` likewise `+20`) is computed on the x87 stack. The first
//! accepting candidate is taken and its distance stored to binary32; a later
//! one replaces it only when its unstored distance compares strictly less
//! than that stored binary32, so ties keep the earlier list entry.
use crate::x87::Register;
use crate::{EconomyError, F32Bits, X87Precision};
use std::cmp::Ordering;

/// The fields and Denizen calls `4E1D90` reads.
pub trait BaseWorld {
    type Actor: Copy;
    type Error: From<EconomyError>;
    /// Fact: actor `+8` bit 0.
    fn removed(&mut self, actor: Self::Actor) -> Result<bool, Self::Error>;
    /// Fact: actor `+20`/`+24`.
    fn position(&mut self, actor: Self::Actor) -> Result<[F32Bits; 2], Self::Error>;
    /// Fact: candidate `+88` (Denizen) is nonnull.
    fn has_denizen(&mut self, base: Self::Actor) -> Result<bool, Self::Error>;
    /// Fact: candidate `+C0` word is nonzero.
    fn word_c0(&mut self, base: Self::Actor) -> Result<bool, Self::Error>;
    /// `568120(actor+4)` on the candidate's Denizen: AL.
    fn denizen_568120(&mut self, base: Self::Actor, actor: Self::Actor) -> Result<bool, Self::Error>;
    /// `568EA0(actor+4)` on the candidate's Denizen: AL.
    fn denizen_568ea0(&mut self, base: Self::Actor, actor: Self::Actor) -> Result<bool, Self::Error>;
}

/// `4E1D90`: `bases` is the Kingdom `+284` list in node order (a null node
/// value is `None`).
pub fn nearest_base<W: BaseWorld + ?Sized>(
    world: &mut W,
    bases: &[Option<W::Actor>],
    actor: Option<W::Actor>,
    precision: X87Precision,
) -> Result<Option<W::Actor>, W::Error> {
    let p = precision;
    let Some(actor) = actor else { return Ok(None) };
    if world.removed(actor)? {
        return Ok(None);
    }
    let mut best: Option<(W::Actor, F32Bits)> = None;
    for &base in bases {
        let Some(base) = base else { continue };
        if world.removed(base)? || !world.has_denizen(base)? {
            continue;
        }
        let accepts = if world.word_c0(base)? {
            world.denizen_568120(base, actor)?
        } else {
            world.denizen_568ea0(base, actor)?
        };
        if !accepts {
            continue;
        }
        let [bx, by] = world.position(base)?;
        let [ax, ay] = world.position(actor)?;
        let dy = Register::load(by)?.sub(&Register::load(ay)?, p);
        let dx = Register::load(bx)?.sub(&Register::load(ax)?, p);
        let sum = dx.mul(&dx, p).add(&dy.mul(&dy, p), p);
        let take = match best {
            None => true,
            Some((_, stored)) => sum.compare(&Register::load(stored)?) == Ordering::Less,
        };
        if take {
            best = Some((base, sum.store_f32()?));
        }
    }
    Ok(best.map(|(base, _)| base))
}
