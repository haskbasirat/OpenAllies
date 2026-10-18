//! Native under-siege refresh `4D6660` (the Body-damage response that
//! `aa_body::damage::Action::ResetRecovery` dispatches) and the
//! recently-damaged test `4D0440`. State 20 is `under_siege`.
use crate::Result;
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub trait World {
    fn flags8(&self, actor: u32) -> Result<u32>;
    fn flags100(&self, actor: u32) -> Result<u32>;
    /// Time word (`+10`) of the actor's last damage record: Body `+28` when a
    /// Body exists, else OrganizationComponent `+90`, else wall component `+4C`.
    fn last_damage_time(&self, actor: u32) -> Result<Option<F32>>;
    /// Component presence: `+98` wall, `+90` wall link.
    fn has_wall(&self, actor: u32) -> Result<bool>;
    fn has_wall_link(&self, actor: u32) -> Result<bool>;
    /// Virtual `+100` outer actor and `+158` delegation predicate.
    fn outer(&mut self, actor: u32) -> Result<Option<u32>>;
    fn delegates(&mut self, actor: u32) -> Result<bool>;
    /// `+128` engagement list nonempty.
    fn engaged(&self, actor: u32) -> Result<bool>;
    /// `+D4` object present with byte `+20` set.
    fn watched_flag(&self, actor: u32) -> Result<bool>;
    /// `+88` component88 present, `+14C > 5`, `(+148)+14` nonnull and
    /// component88 `+24` nonnull.
    fn component88_gate(&self, actor: u32) -> Result<bool>;
    /// `585160` on `outer+98` and `569540(outer)` on component88; bind
    /// [`crate::walls::wall_complete`] and [`crate::walls::intruder_near_wall`].
    fn wall_predicate(&mut self, outer: u32) -> Result<bool>;
    fn component88_predicate(&mut self, actor: u32, outer: u32) -> Result<bool>;
    /// `586370` on `outer+98`; bind [`crate::walls::propagate_under_siege`].
    fn wall_update(&mut self, outer: u32) -> Result<()>;
    /// `4D64F0(actor, 20, enabled)` and `4D1350(actor)`.
    fn set_state(&mut self, actor: u32, state: u8, enabled: bool) -> Result<()>;
    fn notify(&mut self, actor: u32) -> Result<()>;
    fn now(&self) -> Result<F32>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `4D0440`: a nonzero record time with `now - time` strictly below
/// `threshold` (UnderSiegeRecentlyDamagedSeconds, settings cache `+40`).
pub fn recently_damaged(
    actor: u32,
    threshold: F32,
    world: &impl World,
    p: Precision,
) -> Result<bool> {
    let Some(time) = world.last_damage_time(actor)? else {
        return Ok(false);
    };
    if n(time)?.is_zero() {
        return Ok(false);
    }
    Ok(n(world.now()?)?.sub(&n(time)?, p)?.compare(&n(threshold)?) == Ordering::Less)
}

/// Native `4D6660`.
pub fn refresh_under_siege(
    actor: u32,
    threshold: F32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let was = world.flags100(actor)? & 0x10_0000 != 0;
    let check_delegation = if world.has_wall(actor)? {
        true
    } else {
        world.outer(actor)?.is_some() && !world.has_wall_link(actor)?
    };
    if check_delegation && !world.delegates(actor)? {
        return Ok(());
    }
    let mut siege = recently_damaged(actor, threshold, world, p)?
        || world.engaged(actor)?
        || world.watched_flag(actor)?;
    if world.component88_gate(actor)? {
        match world.outer(actor)? {
            None => siege = true,
            Some(outer) => {
                if !world.wall_predicate(outer)? {
                    let outer = world.outer(actor)?.ok_or(crate::Error::Invalid(
                        "outer actor vanished between native calls",
                    ))?;
                    if world.component88_predicate(actor, outer)? {
                        siege = true;
                    }
                }
            }
        }
    }
    if !siege && world.delegates(actor)? {
        let outer = world.outer(actor)?.ok_or(crate::Error::Invalid(
            "delegating actor has no outer actor (native null dereference)",
        ))?;
        if recently_damaged(outer, threshold, world, p)? {
            siege = true;
        } else {
            let outer = world.outer(actor)?.ok_or(crate::Error::Invalid(
                "outer actor vanished between native calls",
            ))?;
            if world.engaged(outer)? {
                siege = true;
            }
        }
    }
    if world.flags8(actor)? & 1 != 0 {
        siege = false;
    }
    if siege == was {
        return Ok(());
    }
    world.set_state(actor, 20, siege)?;
    if world.delegates(actor)? {
        let outer = world.outer(actor)?.ok_or(crate::Error::Invalid(
            "delegating actor has no outer actor (native null dereference)",
        ))?;
        world.wall_update(outer)?;
    }
    world.notify(actor)
}
