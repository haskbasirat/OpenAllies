//! UIWorldState command masks 0x437EB0 and the two simulation-side routines
//! it reaches through actor virtuals: actor vt+0x154 (0x4CE680) and the
//! Research component availability test 0x5845A0.
//!
//! 0x437EB0 (game UIWorldState, vtables 0xA61E40 / base 0x9FD648) rebuilds
//! the four DWORDs +0x20..+0x2C, two 64-bit sets over the 0x27 ActorCommand
//! indices that the HUD action buttons read:
//!
//! * `+0x20/+0x24` ([`ANY`]): some selected actor may issue the command;
//! * `+0x28/+0x2C` ([`ALL`]): every selected node may issue it.
//!
//! Callers (xrefs): 0x438370 (add), 0x438430 (remove), 0x438560 (group
//! +0x18 byte change), 0x438AD0 (deselect), 0x439E90 (SaveGroup),
//! 0x43A1C0 (the per-frame UIWorldState update, tail jump after it caches
//! the local player's kingdom in +0x4C) and 0x50C8E0 (simulation
//! notification from 0x4CFCC0 when a selected actor of the local kingdom
//! changes).
//!
//! Native order (every instruction of 0x437EB0 proved by the oracle in
//! `research/phase8-orders-input/command-masks/`, except four list
//! inconsistency branches that the UIWorldState list invariant excludes):
//! 1. the four words are zeroed; an empty list (+08 NULL) returns;
//! 2. `retreat_only`: every node's 0x45B7A0 actor has an Organization +7C,
//!    `Actor+0x100 & 0x10` (the 64-bit test's high half is the constant 0)
//!    and `Organization+0x8C` set; the first failing node ends the scan;
//! 3. for index 0..0x27: with distinct head/tail views the definition
//!    `[[B36530]+0x298][index]+0x3C` (non_group_command) skips the index;
//!    `retreat_only` skips every index but 7 (retreat) and 8 (rout); the bit
//!    (0x96D710) is set in ALL; per node: no 0x45B7A0 actor, or actor
//!    vt+0x154(0x5C0E30(), index, 1) false, clears it from ALL; true sets it
//!    in ANY, except index 0x23 (research), which also needs a Research
//!    component (+0xB4) whose 0x5845A0 holds: the actor's own when it has no
//!    Organization, otherwise the first such non-NULL member (0x4364C0,
//!    filter byte 0) of its Organization (+0x28/+0x2C);
//! 4. with a non-NULL kingdom +0x4C and distinct head/tail views, count (up
//!    to 2) the nodes whose view+0x14 thing has vt+0x20(1) and either not
//!    vt+0xB4 or vt+0x134(+0x4C); with 2, `[+0x40]+0x18` selects bit 0x26
//!    (group_behavior_disable, byte set) or 0x25 (enable), ORed into both
//!    sets. A NULL +0x40 there is a native fault.
//!
//! Every native read goes through [`MaskLeaves`]; nothing has a default.
use std::fmt::Debug;

pub type Result<T> = std::result::Result<T, String>;

/// `cmp ebx, 0x27` at 0x438083: the ActorCommand definitions.
pub const COMMAND_COUNT: u32 = 0x27;
/// retreat (7) and rout (8): the only indices offered to a retreating
/// selection.
pub const RETREAT: u32 = 7;
pub const ROUT: u32 = 8;
/// research (0x23): additionally needs an available Research item.
pub const RESEARCH: u32 = 0x23;
/// group_behavior_enable (0x25) / group_behavior_disable (0x26).
pub const GROUP_BEHAVIOR_ENABLE: u32 = 0x25;
pub const GROUP_BEHAVIOR_DISABLE: u32 = 0x26;
/// Word indices of `[u32; 4]` (UIWorldState+0x20 + 4 * i).
pub const ANY: [usize; 2] = [0, 1];
pub const ALL: [usize; 2] = [2, 3];

/// The two 64-bit sets of `+0x20..+0x2C`.
pub fn any_set(masks: &[u32; 4]) -> u64 {
    u64::from(masks[0]) | u64::from(masks[1]) << 32
}
pub fn all_set(masks: &[u32; 4]) -> u64 {
    u64::from(masks[2]) | u64::from(masks[3]) << 32
}

/// CRT `__allshl` 0x96D710 (EDX:EAX << CL): counts 64..255 give 0.
fn shl64_96d710(value: u64, count: u8) -> u64 {
    if count >= 64 { 0 } else { value << count }
}

/// Every native read of 0x437EB0 (and of the inlined 0x45B7A0 / 0x4364C0).
/// `Object` is a Thing (view+0x14), `Organization` an Organization
/// component, `Kingdom` a Kingdom pointer, `Research` a Research component.
pub trait MaskLeaves {
    type View: Copy + Eq + Debug;
    type Object: Copy + Eq + Debug;
    type Organization: Copy + Debug;
    type Kingdom: Copy + Eq + Debug;
    type Research: Copy + Debug;
    /// VThing+0x14 (NULL = None).
    fn thing_14(&mut self, view: Self::View) -> Result<Option<Self::Object>>;
    /// Thing vt+0xB4, AL (6A71D0 true for GActor/GKingdom/GGroup, 6A71F0
    /// false for the other Thing classes).
    fn virtual_b4(&mut self, thing: Self::Object) -> Result<bool>;
    /// Actor+0x7C Organization component.
    fn organization_7c(&mut self, actor: Self::Object) -> Result<Option<Self::Organization>>;
    /// Actor+0x100, the low state word.
    fn state_100(&mut self, actor: Self::Object) -> Result<u32>;
    /// Organization+0x8C byte != 0.
    fn organization_8c(&mut self, organization: Self::Organization) -> Result<bool>;
    /// `[[B36530]+0x298][index]+0x3C` (non_group_command) != 0; None when the
    /// definition pointer is NULL (0x437F5C then faults).
    fn non_group_3c(&mut self, index: u32) -> Result<Option<bool>>;
    /// 0x5C0E30: the local player's kingdom (`[B36550]` ? 0x76AC70 : NULL).
    fn local_kingdom_5c0e30(&mut self) -> Result<Option<Self::Kingdom>>;
    /// Actor vt+0x154(kingdom, index, selected), AL ([`virtual_154_4ce680`]
    /// for the three selectable vtables).
    fn virtual_154(&mut self, actor: Self::Object, kingdom: Option<Self::Kingdom>, index: u32, selected: u8) -> Result<bool>;
    /// Actor+0xB4 Research component.
    fn research_b4(&mut self, actor: Self::Object) -> Result<Option<Self::Research>>;
    /// 0x5845A0 on a Research component, AL ([`research_available_5845a0`]).
    fn research_available_5845a0(&mut self, research: Self::Research) -> Result<bool>;
    /// Organization+0x28 member array of count +0x2C (NULL entries kept).
    fn members_28(&mut self, organization: Self::Organization) -> Result<Vec<Option<Self::Object>>>;
    /// Thing vt+0x20(argument), AL.
    fn virtual_20(&mut self, thing: Self::Object, argument: u8) -> Result<bool>;
    /// Thing vt+0x134(kingdom), AL (GActor/GGroup 4D1E90, GKingdom 4E5240).
    fn virtual_134(&mut self, thing: Self::Object, kingdom: Self::Kingdom) -> Result<bool>;
}

/// 0x45B7A0: view+0x14 when it is non-NULL and its vt+0xB4 holds.
pub fn actor_45b7a0<L: MaskLeaves + ?Sized>(leaves: &mut L, view: L::View) -> Result<Option<L::Object>> {
    match leaves.thing_14(view)? {
        Some(thing) if leaves.virtual_b4(thing)? => Ok(Some(thing)),
        _ => Ok(None),
    }
}

/// 0x437EB0 on UIWorldState: the returned words are +0x20, +0x24, +0x28,
/// +0x2C. `selected` is the +0x08 list head first (+0x0C is its last node),
/// `kingdom_4c` the +0x4C word and `group_40_byte_18` the byte +0x18 of the
/// +0x40 group (None for a NULL +0x40).
pub fn command_masks_437eb0<L: MaskLeaves + ?Sized>(
    leaves: &mut L,
    selected: &[L::View],
    kingdom_4c: Option<L::Kingdom>,
    group_40_byte_18: Option<bool>,
) -> Result<[u32; 4]> {
    let mut masks = [0; 4];
    command_masks_in_place_437eb0(leaves, selected, kingdom_4c, group_40_byte_18, &mut masks)?;
    Ok(masks)
}

/// [`command_masks_437eb0`] storing into the caller's +0x20..+0x2C in native
/// order, so a failing leaf leaves the native partial stores committed.
pub fn command_masks_in_place_437eb0<L: MaskLeaves + ?Sized>(
    leaves: &mut L,
    selected: &[L::View],
    kingdom_4c: Option<L::Kingdom>,
    group_40_byte_18: Option<bool>,
    masks: &mut [u32; 4],
) -> Result<()> {
    *masks = [0; 4];
    let (Some(&head), Some(&tail)) = (selected.first(), selected.last()) else {
        return Ok(());
    };
    let mut retreat_only = true;
    for &view in selected {
        let passes = match actor_45b7a0(leaves, view)? {
            None => false,
            Some(actor) => match leaves.organization_7c(actor)? {
                None => false,
                Some(organization) => leaves.state_100(actor)? & 0x10 != 0 && leaves.organization_8c(organization)?,
            },
        };
        if !passes {
            retreat_only = false;
            break;
        }
    }
    let multiple = head != tail;
    for index in 0..COMMAND_COUNT {
        if multiple {
            match leaves.non_group_3c(index)? {
                None => {
                    return Err(format!(
                        "0x437F5C reads [NULL+0x3C]: [[B36530]+0x298][{index:#x}] is NULL (native fault)"
                    ))
                }
                Some(true) => continue,
                Some(false) => {}
            }
        }
        if retreat_only && index != RETREAT && index != ROUT {
            continue;
        }
        let bit = shl64_96d710(1, index as u8);
        let (low, high) = (bit as u32, (bit >> 32) as u32);
        masks[ALL[0]] |= low;
        masks[ALL[1]] |= high;
        for &view in selected {
            if node_grants_437fb0(leaves, view, index)? {
                masks[ANY[0]] |= low;
                masks[ANY[1]] |= high;
            } else {
                masks[ALL[0]] &= !low;
                masks[ALL[1]] &= !high;
            }
        }
    }
    let Some(kingdom) = kingdom_4c else {
        return Ok(());
    };
    if !multiple {
        return Ok(());
    }
    let mut count = 0;
    for &view in selected {
        if count >= 2 {
            break;
        }
        let Some(thing) = leaves.thing_14(view)? else { continue };
        if !leaves.virtual_20(thing, 1)? {
            continue;
        }
        if !leaves.virtual_b4(thing)? || leaves.virtual_134(thing, kingdom)? {
            count += 1;
        }
    }
    if count > 1 {
        let byte = group_40_byte_18.ok_or(
            "0x438142 reads [NULL+0x18]: UIWorldState+0x40 is NULL with two commandable selected objects (native fault)",
        )?;
        let bit = if byte { 1 << (GROUP_BEHAVIOR_DISABLE - 32) } else { 1 << (GROUP_BEHAVIOR_ENABLE - 32) };
        masks[ANY[1]] |= bit;
        masks[ALL[1]] |= bit;
    }
    Ok(())
}

/// 0x437FB0..0x438077: one node's verdict for `index` (true = 0x4380C3,
/// ANY |= bit; false = 0x43805B, ALL &= !bit).
fn node_grants_437fb0<L: MaskLeaves + ?Sized>(leaves: &mut L, view: L::View, index: u32) -> Result<bool> {
    let Some(actor) = actor_45b7a0(leaves, view)? else {
        return Ok(false);
    };
    let kingdom = leaves.local_kingdom_5c0e30()?;
    if !leaves.virtual_154(actor, kingdom, index, 1)? {
        return Ok(false);
    }
    if index != RESEARCH {
        return Ok(true);
    }
    match leaves.organization_7c(actor)? {
        None => match leaves.research_b4(actor)? {
            None => Ok(false),
            Some(research) => leaves.research_available_5845a0(research),
        },
        Some(organization) => {
            // 0x4364C0 with filter byte 0 stops at every non-NULL member.
            for member in leaves.members_28(organization)?.into_iter().flatten() {
                if let Some(research) = leaves.research_b4(member)?
                    && leaves.research_available_5845a0(research)?
                {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

/// The reads of actor vt+0x154 = 0x4CE680, the slot of all three selectable
/// vtables (GActor 0xA273F0, GGroup 0xA29490, GKingdom 0xA299D8).
pub trait PermissionLeaves {
    type Actor: Copy;
    type Kingdom: Copy + Eq;
    /// Actor+0xE8 owner kingdom (NULL = None).
    fn owner_e8(&mut self, actor: Self::Actor) -> Result<Option<Self::Kingdom>>;
    /// Actor vt+0x20(argument), AL (0x4CF120 for the three vtables).
    fn virtual_20(&mut self, actor: Self::Actor, argument: u8) -> Result<bool>;
    /// Actor vt+0x34(index), AL (0x4D1F90: Actor+0xF0/+0xF4 bit, CL count).
    fn virtual_34(&mut self, actor: Self::Actor, index: u32) -> Result<bool>;
}

/// 0x4CE680(kingdom, index, selected): a nonzero `selected` byte requires a
/// non-NULL Actor+0xE8 equal to `kingdom` and vt+0x20(1); the answer is
/// vt+0x34(index) (the whole index DWORD is passed on).
pub fn virtual_154_4ce680<P: PermissionLeaves + ?Sized>(
    leaves: &mut P,
    actor: P::Actor,
    kingdom: Option<P::Kingdom>,
    index: u32,
    selected: u8,
) -> Result<bool> {
    if selected != 0 {
        let owner = leaves.owner_e8(actor)?;
        if owner.is_none() || owner != kingdom {
            return Ok(false);
        }
        if !leaves.virtual_20(actor, 1)? {
            return Ok(false);
        }
    }
    leaves.virtual_34(actor, index)
}

/// The reads of 0x5845A0 on a Research component (this).
pub trait ResearchLeaves {
    type Research: Copy;
    type Item: Copy + Eq + Debug;
    /// `[[[this+4]+4]+0x558]`: the owner's ActorData ResearchItem records,
    /// head first (read once).
    fn items_558(&mut self, research: Self::Research) -> Result<Vec<Self::Item>>;
    /// 0x5844A0(item, notify) on this component: the raw EAX reason
    /// (0 = allowed).
    fn blocked_5844a0(&mut self, research: Self::Research, item: Self::Item, notify: u32) -> Result<u32>;
    /// The this+0x10 record list, head first (re-read for every candidate).
    fn listed_10(&mut self, research: Self::Research) -> Result<Vec<Self::Item>>;
}

/// 0x5845A0: true at the first ActorData research item whose 0x5844A0(item,
/// 0) is 0 and which is not on the component's +0x10 list.
pub fn research_available_5845a0<R: ResearchLeaves + ?Sized>(leaves: &mut R, research: R::Research) -> Result<bool> {
    for item in leaves.items_558(research)? {
        if leaves.blocked_5844a0(research, item, 0)? != 0 {
            continue;
        }
        if !leaves.listed_10(research)?.contains(&item) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "command_masks_tests.rs"]
mod tests;
