//! Small original Mind-state helpers reached from the regiment (CAI) and
//! element (UAI) states:
//!
//! * `51E1D0(mode, target, flag)` on a regiment state: activate on a target
//!   ([`activate_with_target`]).
//! * `52D6D0(mode)` on an element state: activate only for a member of an
//!   organization ([`activate_if_member`]).
//! * `51D3F0` on a regiment Mind: the engage state (Mind+1CC) is active and
//!   its +1C byte is set ([`engage_flag`]).
//! * `51EB80` on a regiment state: unless its +20 byte is set, one of the
//!   GTIDs of its +18 list resolves ([`any_resolves`]).
//! * `52E1D0` on the element move state: the actor is within 3 of the
//!   state's destination +14/+18 ([`near_destination`]).
//! * `51E030(&out)` on a regiment state: the first word of its +14 array, or
//!   0 when its +10 count is 0 ([`first_or_zero`]).
use crate::{
    Arena, Result, StateId, World, activate,
    x87::{n, num},
};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// `[A0A9F0]` (binary32 9.0): squared distance of [`near_destination`].
pub const NEAR_DESTINATION_SQUARED_BITS: u32 = 0x4110_0000;

/// Services of [`activate_with_target`].
pub trait TargetActivateWorld: World {
    /// Object +14 (its GTID).
    fn target_gtid(&mut self, object: u32) -> Result<u32>;
    /// State +10 (target GTID) and +15 byte writes.
    fn set_state_target(&mut self, state: StateId, gtid: u32) -> Result<()>;
    fn set_state_byte15(&mut self, state: StateId, value: u8) -> Result<()>;
}

/// `51E1D0(mode, target, flag)`: a null target fails; `529C00(mode)` (modes
/// other than 0 and 1 ask the state's virtual +4) must hold; then state+10 =
/// the target's GTID, state+15 = `flag` and `52A810(mode)` gives the result.
pub fn activate_with_target(
    state: StateId,
    mode: u32,
    target: Option<u32>,
    flag: u8,
    arena: &mut Arena,
    world: &mut impl TargetActivateWorld,
) -> Result<bool> {
    let Some(target) = target else {
        return Ok(false);
    };
    if mode != 0 && mode != 1 && !world.can_activate(state, arena)? {
        return Ok(false);
    }
    let gtid = world.target_gtid(target)?;
    world.set_state_target(state, gtid)?;
    world.set_state_byte15(state, flag)?;
    activate(state, mode, arena, world)
}

/// Services of [`activate_if_member`].
pub trait MemberActivateWorld: World {
    /// Actor +80 (membership) -> +10 (organization) present.
    fn has_organization(&mut self, state: StateId) -> Result<bool>;
}

/// `52D6D0(mode)`: `52A810(mode)` for a member of an organization, false
/// otherwise.
pub fn activate_if_member(
    state: StateId,
    mode: u32,
    arena: &mut Arena,
    world: &mut impl MemberActivateWorld,
) -> Result<bool> {
    if !world.has_organization(state)? {
        return Ok(false);
    }
    activate(state, mode, arena, world)
}

/// `51D3F0`: Mind+1D8 (engage status) is 2 and Mind+1E8 (engage +1C byte)
/// is nonzero.
pub fn engage_flag(engage_status: u32, engage_byte1c: u8) -> bool {
    engage_status == 2 && engage_byte1c != 0
}

/// `51EB80`: false when the +20 byte is set; otherwise true when one GTID of
/// the +18 list (in list order) resolves to a non-null registry entry.
pub fn any_resolves(
    byte20: bool,
    gtids: &[u32],
    mut resolve: impl FnMut(u32) -> Result<Option<u32>>,
) -> Result<bool> {
    if byte20 {
        return Ok(false);
    }
    for &gtid in gtids {
        if resolve(gtid)?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `52E1D0`: `(actor - destination)` squared length (x87 at `p`, unstored)
/// is below 9 (ordered).
pub fn near_destination(actor: [u32; 2], destination: [u32; 2], p: Precision) -> Result<bool> {
    let dy = num(n(actor[1])?.sub(&n(destination[1])?, p))?;
    let dx = num(n(actor[0])?.sub(&n(destination[0])?, p))?;
    let sum = num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?;
    Ok(sum.compare(&n(NEAR_DESTINATION_SQUARED_BITS)?) == Ordering::Less)
}

/// `51E030(&out)`: 0 when the +10 count is 0, otherwise the first word of
/// the +14 array.
pub fn first_or_zero(count: u32, first: impl FnOnce() -> Result<u32>) -> Result<u32> {
    if count == 0 { Ok(0) } else { first() }
}
