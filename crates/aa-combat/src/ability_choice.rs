//! Original Performer target check `57E920(target, kind, strict)` (thiscall
//! Performer, `ret 0xC`): the best code over the Performer's abilities for
//! using one of them on a target. It is the Performer step of the relation
//! `4D4FF0` ([`crate::relation`], `performer_relation`).
//!
//! Order of the original:
//! 1. A null target, or one whose virtual +84(kingdom) is false, answers 6.
//!    The kingdom is the Performer actor's (+4) virtual +108.
//! 2. Kind 3 (bombard): the target's virtual +5C(kingdom, 1) must hold,
//!    else 5.
//! 3. A context is built (`4C8300(kind)`, +8 = actor, +1C = target) and the
//!    best code starts at 4. The abilities (Performer +14 array, +18 count)
//!    are walked by `517F60`/`516AD0`: only abilities whose +18 byte is set,
//!    the count reread at every step. With kind 3 an ability whose
//!    definition (+4) +54 bit 3 is clear is skipped. `4C8740(&context, 0)`
//!    must hold; then with the target's +20/+24 point:
//!    `4C84B0(actor, &point)` false is code 3, `4C8460(actor, &point)` false
//!    is code 2. Otherwise a non-strict call answers 0 at once; a strict one
//!    stores `dx^2 + dy^2` (target minus actor, binary32 store) and answers
//!    0 when it is not above `range^2` (`4C8270(4CF110())`, unstored), else
//!    code 1. The smallest code is kept.
//! 4. The walk's end answers the best code.
//!
//! All x87 arithmetic follows the original sequence at precision `p`; a
//! non-finite operand is an error.
use crate::Result;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// The `4C8300` context words `57E920` fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    /// +0: the kind argument.
    pub kind: u32,
    /// +8: the Performer actor.
    pub actor: u32,
    /// +1C: the target.
    pub target: u32,
}

/// Services of `57E920`, in native order. Ability indices are Performer
/// +14 array positions.
pub trait AbilityChoiceWorld {
    /// Performer +4 (its actor).
    fn actor(&mut self) -> Result<u32>;
    /// Actor virtual +108 (its kingdom).
    fn kingdom(&mut self, actor: u32) -> Result<u32>;
    /// Target virtual +84(kingdom).
    fn target_accepts(&mut self, target: u32, kingdom: u32) -> Result<bool>;
    /// Target virtual +5C(kingdom, flag).
    fn target_bombardable(&mut self, target: u32, kingdom: u32, flag: u32) -> Result<bool>;
    /// Performer +18 (signed count).
    fn ability_count(&mut self) -> Result<i32>;
    /// Ability +18 byte is set.
    fn ability_enabled(&mut self, index: i32) -> Result<bool>;
    /// Ability definition (+4) +54.
    fn ability_flags54(&mut self, index: i32) -> Result<u32>;
    /// `4C8740(&context, flag)` on the ability.
    fn ability_applies(&mut self, index: i32, context: &Context, flag: u32) -> Result<bool>;
    /// Target +20/+24 (binary32).
    fn target_position(&mut self, target: u32) -> Result<[u32; 2]>;
    /// `4C84B0(actor, &point)` on the ability.
    fn ability_reaches(&mut self, index: i32, actor: u32, point: [u32; 2]) -> Result<bool>;
    /// `4C8460(actor, &point)` on the ability.
    fn ability_ready(&mut self, index: i32, actor: u32, point: [u32; 2]) -> Result<bool>;
    /// Actor +20/+24 (binary32).
    fn actor_position(&mut self, actor: u32) -> Result<[u32; 2]>;
    /// `4CF110` on the actor.
    fn modifier(&mut self, actor: u32) -> Result<u32>;
    /// `4C8270(modifier)` on the ability (ST0, unstored).
    fn ability_range(&mut self, index: i32, modifier: u32) -> Result<Binary>;
}

/// The bombard kind.
pub const BOMBARD: u32 = 3;

fn n(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}

/// `517F60` (from 0) and `516AD0` (from `index + 1`): the next ability whose
/// +18 byte is set, or the end position.
fn next_enabled(world: &mut impl AbilityChoiceWorld, mut index: i32) -> Result<i32> {
    while index < world.ability_count()? && index >= 0 && !world.ability_enabled(index)? {
        index += 1;
    }
    Ok(index)
}

/// Complete `57E920(target, kind, strict)`; returns the original's code.
pub fn choose(
    target: Option<u32>,
    kind: u32,
    strict: bool,
    world: &mut impl AbilityChoiceWorld,
    p: Precision,
) -> Result<u32> {
    let Some(target) = target else {
        return Ok(6);
    };
    let actor = world.actor()?;
    let kingdom = world.kingdom(actor)?;
    if !world.target_accepts(target, kingdom)? {
        return Ok(6);
    }
    let bombard = kind == BOMBARD;
    if bombard {
        let kingdom = world.kingdom(actor)?;
        if !world.target_bombardable(target, kingdom, 1)? {
            return Ok(5);
        }
    }
    let context = Context { kind, actor, target };
    let mut best = 4;
    let mut index = next_enabled(world, 0)?;
    loop {
        if index >= world.ability_count()? || index < 0 {
            return Ok(best);
        }
        let considered = !bombard || (world.ability_flags54(index)? >> 3) & 1 != 0;
        if considered && world.ability_applies(index, &context, 0)? {
            let point = world.target_position(target)?;
            let code = if !world.ability_reaches(index, actor, point)? {
                3
            } else if !world.ability_ready(index, actor, point)? {
                2
            } else {
                if !strict {
                    return Ok(0);
                }
                let t = world.target_position(target)?;
                let a = world.actor_position(actor)?;
                let dy = n(t[1])?.sub(&n(a[1])?, p)?;
                let dx = n(t[0])?.sub(&n(a[0])?, p)?;
                let sum = dx.mul(&dx, p)?.add(&dy.mul(&dy, p)?, p)?.store_f32()?;
                let modifier = world.modifier(actor)?;
                let range = world.ability_range(index, modifier)?;
                let reach = range.mul(&range, p)?;
                // fcompp ST0 = stored sum, ST1 = range^2: C0 or C3 alone is
                // within range.
                if n(sum)?.compare(&reach) != Ordering::Greater {
                    return Ok(0);
                }
                1
            };
            if best > code {
                best = code;
            }
        }
        index = next_enabled(world, index + 1)?;
    }
}
