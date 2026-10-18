//! Structure enable/disable and construction command availability.
//!
//! * `0x565400` enable (cancel command on a disabled structure; also called by
//!   KKC_GActor::ChangeKingdom `0x4D8450` for a disabled structure):
//!   state 0x1B on, `0x565140` recompute, then the settlement refresh
//!   `0x584E10` when the actor is in a settlement.
//! * `0x564DC0` disable (command 0x16): state 0x1E on, `0x4D1350`, then in a
//!   settlement `0x584DE0` (clear every structure production queue,
//!   `0x589CC0`) and `0x584E10`.
//! * `0x565140` recomputes the construction commands in the 64-bit actor
//!   command mask at +0xF0 (`0x517FA0` sets/clears bit n): 0x13 cancel, 0x14
//!   sell, 0x15 upgrade (`0x565090`), 0x16 disable. The structure mode is the
//!   lowest actor state in state group 0xD (`0x4D14C0`/`0x4D0820`): 0x1B
//!   operational, 0x1D upgrading, 0x1E disabled.
//!
//! Actor state and command-mask writes, the state-group table (runtime data
//! at 0xB3AC40), virtual calls, the settlement center lookup `0x4CEDA0`, the
//! option "AllowSellUnderSiege" (`0x4B2E60`, looked up once and cached),
//! `0x4D1350` refreshes and queue clearing are explicit [`StructureState`]
//! callbacks.
use crate::Result;
use crate::construction::{ACTOR_CONSTRUCTING, ACTOR_STATE_20, Construction};

pub const COMMAND_CANCEL: u32 = 0x13;
pub const COMMAND_SELL: u32 = 0x14;
pub const COMMAND_UPGRADE: u32 = 0x15;
pub const COMMAND_DISABLE: u32 = 0x16;
/// State group of the structure mode.
pub const MODE_GROUP: u32 = 0xD;
pub const MODE_OPERATIONAL: u32 = 0x1B;
pub const MODE_UPGRADING: u32 = 0x1D;
pub const MODE_DISABLED: u32 = 0x1E;
/// Actor state 0x13 (out of supply) blocks selling.
pub const STATE_OUT_OF_SUPPLY: u32 = 0x13;
/// Modifier-token word bit that blocks the sell and disable commands.
pub const TOKEN_LOCKED: u32 = 0x800;

/// `0x4D14C0(group)` + `0x4D0820`: the lowest set state of `states` inside
/// the group mask, or None (-1 natively).
pub fn state_in_group(states: u64, group_mask: u64) -> Option<u32> {
    let masked = states & group_mask;
    (masked != 0).then(|| masked.trailing_zeros())
}

/// `0x585F00`: the settlement holds exactly its limit of structures and none
/// is under construction (actor+0x100 bit 21).
pub fn settlement_ready_for_upgrade(count: i32, limit: i32, constructing: &[bool]) -> bool {
    count == limit && !constructing.iter().any(|&c| c)
}

/// Settlement membership walked by `0x584E10`/`0x584DE0`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettlementMembers {
    /// SettlementComponent+4.
    pub settlement_actor: u32,
    /// +0x14 center.
    pub center: Option<u32>,
    /// +0x18/+0x1C structures.
    pub structures: Vec<u32>,
    /// +0x24/+0x28 members (null entries are skipped).
    pub members: Vec<Option<u32>>,
}

/// Facts `0x565090` needs about the settlement (read through virtual +0x15C
/// and its SettlementComponent).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SettlementUpgradeFacts {
    /// +0x1C structure count.
    pub count: i32,
    /// ActorData+0x464 limit of the settlement level.
    pub limit: i32,
    /// Bit 21 of each listed structure.
    pub constructing: Vec<bool>,
}

pub trait StructureState: Construction {
    /// Runtime state-group table 0xB3AC40[group].
    fn state_group_mask(&mut self, group: u32) -> Result<u64>;
    /// Actor+0xF0 command mask.
    fn command_mask(&mut self) -> Result<u64>;
    fn set_command_mask(&mut self, mask: u64) -> Result<()>;
    /// `0x4CEDA0` settlement center (of the actor or its virtual +0x100
    /// ancestors) with actor+0x100 bit 30 (disabled).
    fn settlement_center_disabled(&mut self) -> Result<bool>;
    /// Actor virtual +0x100 (non-null).
    fn actor_virtual_100(&mut self) -> Result<bool>;
    /// Actor virtual +0x158 (in a settlement).
    fn in_settlement(&mut self) -> Result<bool>;
    /// (virtual +0x15C)+0x98 facts for `0x585F00`.
    fn settlement_upgrade_facts(&mut self) -> Result<SettlementUpgradeFacts>;
    /// (virtual +0x15C)+0x98 membership for the refreshes.
    fn settlement_members(&mut self) -> Result<SettlementMembers>;
    /// Actor+0x90 component present.
    fn has_component_90(&mut self) -> Result<bool>;
    /// ActorData+0x4E5: the structure can be disabled.
    fn can_disable(&mut self) -> Result<bool>;
    /// ActorData+0x4E4: the structure can be sold.
    fn can_sell(&mut self) -> Result<bool>;
    /// `0x4CF110` modifier token first word, None without a token.
    fn token_word(&mut self) -> Result<Option<u32>>;
    /// Option "AllowSellUnderSiege" (`0x4B2E60` on 0xB39AA8, cached pointer).
    fn allow_sell_under_siege(&mut self) -> Result<bool>;
    /// `actor.0x4D1350`.
    fn refresh_actor_organization(&mut self, actor: u32) -> Result<()>;
    /// `structure+0x94 .0x589CC0`: cancel every queue item (refunds).
    fn clear_queue(&mut self, structure: u32) -> Result<()>;
    /// This actor (for `0x4D1350` in `0x564DC0`).
    fn this_actor(&mut self) -> Result<u32>;
}

fn set_bit(mask: u64, bit: u32, enabled: bool) -> u64 {
    if enabled {
        mask | 1 << bit
    } else {
        mask & !(1 << bit)
    }
}

fn structure_mode(world: &mut impl StructureState) -> Result<Option<u32>> {
    let states = world.actor_flags()?;
    let group = world.state_group_mask(MODE_GROUP)?;
    Ok(state_in_group(states, group))
}

/// `0x565090(check_settlement)`: the upgrade command is available.
pub fn upgrade_available(check_settlement: bool, world: &mut impl StructureState) -> Result<bool> {
    if world.has_component_90()? {
        return Ok(false);
    }
    let flags = world.actor_flags()?;
    if flags & (ACTOR_STATE_20 | ACTOR_CONSTRUCTING) != 0 {
        return Ok(false);
    }
    if structure_mode(world)? != Some(MODE_OPERATIONAL) || world.upgrade_list()?.is_empty() {
        return Ok(false);
    }
    if check_settlement && world.in_settlement()? {
        let facts = world.settlement_upgrade_facts()?;
        return Ok(settlement_ready_for_upgrade(
            facts.count,
            facts.limit,
            &facts.constructing,
        ));
    }
    Ok(true)
}

/// `0x565140`: recompute the construction commands in the command mask.
pub fn recompute_commands(world: &mut impl StructureState) -> Result<()> {
    let mut mode = structure_mode(world)?;
    let mut operational = mode == Some(MODE_OPERATIONAL);
    let flags = world.actor_flags()?;
    let besieged = flags & ACTOR_STATE_20 != 0;
    if world.settlement_center_disabled()? {
        mode = Some(MODE_DISABLED);
        operational = false;
    }
    if flags & ACTOR_CONSTRUCTING != 0 {
        let mut mask = world.command_mask()?;
        mask &= !(1 << COMMAND_UPGRADE);
        mask &= !(1 << COMMAND_SELL);
        mask &= !(1 << COMMAND_DISABLE);
        mask |= 1 << COMMAND_CANCEL;
        return world.set_command_mask(mask);
    }
    let upgrade = upgrade_available(true, world)?;
    let mask = world.command_mask()?;
    world.set_command_mask(set_bit(mask, COMMAND_UPGRADE, upgrade))?;

    let mut disable = false;
    if world.can_disable()? && !besieged {
        let placed = !world.actor_virtual_100()? || world.in_settlement()?;
        disable = placed && mode != Some(MODE_UPGRADING) && operational;
    }
    let token = world.token_word()?;
    let locked = token.is_some_and(|t| t & TOKEN_LOCKED != 0);
    if locked {
        disable = false;
    }
    let mask = world.command_mask()?;
    world.set_command_mask(set_bit(mask, COMMAND_DISABLE, disable))?;

    let mut cancel = false;
    if mode == Some(MODE_UPGRADING) {
        cancel = !world.has_component_90()?;
    } else if mode == Some(MODE_DISABLED) {
        cancel = !world.actor_virtual_100()? || world.in_settlement()?;
    }
    let mask = world.command_mask()?;
    world.set_command_mask(set_bit(mask, COMMAND_CANCEL, cancel))?;

    let mut sell = world.can_sell()? && !world.state_active(STATE_OUT_OF_SUPPLY)?;
    if sell && besieged && !world.allow_sell_under_siege()? {
        sell = false;
    }
    if locked {
        sell = false;
    }
    let mask = world.command_mask()?;
    world.set_command_mask(set_bit(mask, COMMAND_SELL, sell))
}

/// `0x584E10`: `0x4D1350` on the settlement actor, the center, every
/// structure and every non-null member, in that order.
pub fn refresh_settlement(
    members: &SettlementMembers,
    world: &mut impl StructureState,
) -> Result<()> {
    world.refresh_actor_organization(members.settlement_actor)?;
    if let Some(center) = members.center {
        world.refresh_actor_organization(center)?;
    }
    for &structure in &members.structures {
        world.refresh_actor_organization(structure)?;
    }
    for member in members.members.iter().flatten() {
        world.refresh_actor_organization(*member)?;
    }
    Ok(())
}

/// `0x565400` enable structure (the API ChangeKingdom `0x4D8450` uses for a
/// disabled structure).
pub fn enable_structure(world: &mut impl StructureState) -> Result<()> {
    world.construction_actor_state(MODE_OPERATIONAL, true)?;
    recompute_commands(world)?;
    if world.in_settlement()? {
        let members = world.settlement_members()?;
        refresh_settlement(&members, world)?;
    }
    Ok(())
}

/// `0x564DC0` disable structure (command 0x16).
pub fn disable_structure(world: &mut impl StructureState) -> Result<()> {
    world.construction_actor_state(MODE_DISABLED, true)?;
    let actor = world.this_actor()?;
    world.refresh_actor_organization(actor)?;
    if world.in_settlement()? {
        let members = world.settlement_members()?;
        for &structure in &members.structures {
            world.clear_queue(structure)?;
        }
        refresh_settlement(&members, world)?;
    }
    Ok(())
}
