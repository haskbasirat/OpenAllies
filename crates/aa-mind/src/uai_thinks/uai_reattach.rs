//! Original `KKC_UAIStateReattach::Think` (`534000`, element state
//! `uai_reattach` at Mind+178, vtable A347BC slot +C): walk back to the
//! organization the unit is a member of.
//!
//! Order:
//! 1. `529E60` ([`crate::think_base`]).
//! 2. Without a membership (actor+80) or its organization (+10): Mind
//!    virtual +144 (tail call), done.
//! 3. `5709D0` on the membership (refreshes its detached byte +1C). The
//!    membership is reread; a set +1C byte goes to Mind virtual +144.
//! 4. When the move state (Mind+7C) is not active (status != 2), the
//!    membership (reread) organization is ordered on the move state with
//!    `532930(0, organization, 2, 0, [B279C0], [B279C4])`.
//! 5. The move state Thinks (tail call).
use super::{ThinkWorld, element, null, status_at};
use crate::{Arena, MindId, Result, StateId, think_base};

/// Services of `534000` in native order.
pub trait ReattachThinkWorld: ThinkWorld {
    /// Actor +80 (membership), `None` when null.
    fn membership(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// Membership +10 (organization), `None` when null.
    fn membership_organization(&mut self, membership: u32) -> Result<Option<u32>>;
    /// Membership +1C byte (detached).
    fn membership_detached(&mut self, membership: u32) -> Result<bool>;
    /// `5709D0` on the membership (result unused by the Think).
    fn membership_refresh(&mut self, membership: u32) -> Result<()>;
    /// Mind virtual +144.
    fn mind_virtual_144(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `532930(mode, object, a3, a4, point)` on the move state (Mind+7C),
    /// movement-owned; the result is unused here.
    #[allow(clippy::too_many_arguments)]
    fn move_to_object(
        &mut self,
        mind: MindId,
        mode: u32,
        object: Option<u32>,
        a3: u32,
        a4: u32,
        point: [u32; 2],
        arena: &mut Arena,
    ) -> Result<()>;
}

/// Complete `534000`. Returns the base Think's wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    world: &mut impl ReattachThinkWorld,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let organization = match world.membership(mind)? {
        Some(m) => world.membership_organization(m)?,
        None => None,
    };
    if organization.is_none() {
        world.mind_virtual_144(mind, arena)?;
        return Ok(wrong);
    }
    let membership = world
        .membership(mind)?
        .ok_or_else(|| null("5709D0 on a null membership"))?;
    world.membership_refresh(membership)?;
    let membership = world
        .membership(mind)?
        .ok_or_else(|| null("membership +1C read"))?;
    if world.membership_detached(membership)? {
        world.mind_virtual_144(mind, arena)?;
        return Ok(wrong);
    }
    if status_at(world, mind, element::MOVE, arena)? != 2 {
        let point = world.null_point()?;
        let organization = match world.membership(mind)? {
            Some(m) => world.membership_organization(m)?,
            None => None,
        };
        world.move_to_object(mind, 0, organization, 2, 0, point, arena)?;
    }
    world.embedded_think(mind, element::MOVE, arena)?;
    Ok(wrong)
}
