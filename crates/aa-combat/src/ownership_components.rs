//! Component callbacks of the kingdom change (`ownership::change_kingdom`):
//! each hands the new kingdom on through the actor virtual `+160`.
//!
//! - `0x589050` (component `+94`): the actor its `+34` handle names, when
//!   the handle is live.
//! - `0x579F10` (OrganizationComponent `+7C`): every member, in the
//!   member hash order (`0x4364C0`). Limitation: the port walks a snapshot
//!   of the members taken before the first call, while the original re-runs
//!   `0x4364C0` on the live hash after each `+160` call; a member whose
//!   `+160` releases a membership (`0x5714D0`, see `ownership.rs`) changes
//!   that hash, and then the two walks can differ.
//! - `0x584F10` (Settlement `+98`): the `+14` structure, then each filled
//!   entry of the `+18` / `+1C` structure array and of the `+24` / `+28`
//!   segment array; both counts are reread after every call.
//! - `0x585FF0` (Settlement `+70` / `+74` seen list): the kingdom is put at
//!   the head of the list unless it is already in it.
//!
//! Proven by research/phase8-combat/native_ownership_components.py
//! (tests/native_ownership_components.rs).
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// The two Settlement arrays `0x584F10` walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WallList {
    /// `+18` entries, `+1C` count.
    Structures,
    /// `+24` entries, `+28` count.
    Segments,
}

/// Services of the component callbacks.
pub trait ComponentKingdomWorld {
    /// The handle table (`[0xB364C0]`): the actor in the slot when the
    /// generation matches and the slot is filled.
    fn resolve(&self, handle: u32) -> Result<Option<u32>>;
    /// Actor virtual `+160(new)`.
    fn change_kingdom(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    /// OrganizationComponent members (`+28` entries) in `0x4364C0` order.
    fn organization_members(&mut self, component: u32) -> Result<Vec<u32>>;
    /// Settlement `+14`.
    fn wall_structure(&self, component: u32) -> Result<Option<u32>>;
    /// Settlement `+1C` or `+28`.
    fn wall_count(&self, component: u32, list: WallList) -> Result<i32>;
    /// Settlement `+18[index]` or `+24[index]` (None for an empty entry).
    fn wall_entry(&self, component: u32, list: WallList, index: i32) -> Result<Option<u32>>;
}

/// Native `0x589050(new)`: `handle` is the component's `+34` word.
pub fn linked_change_kingdom(
    handle: u32,
    new: Option<u32>,
    world: &mut impl ComponentKingdomWorld,
) -> Result<()> {
    if let Some(actor) = world.resolve(handle)? {
        world.change_kingdom(actor, new)?;
    }
    Ok(())
}

/// Native `0x579F10(new)` on the OrganizationComponent `component`, over a
/// snapshot of the members (see the module note on the live-hash walk).
pub fn organization_change_kingdom(
    component: u32,
    new: Option<u32>,
    world: &mut impl ComponentKingdomWorld,
) -> Result<()> {
    for member in world.organization_members(component)? {
        if member == 0 {
            return Err(Error::Invalid("organization member entry is empty"));
        }
        world.change_kingdom(member, new)?;
    }
    Ok(())
}

/// Native `0x584F10(new)` on the Settlement `component`.
pub fn wall_change_kingdom(
    component: u32,
    new: Option<u32>,
    world: &mut impl ComponentKingdomWorld,
) -> Result<()> {
    if let Some(structure) = world.wall_structure(component)? {
        world.change_kingdom(structure, new)?;
    }
    for list in [WallList::Structures, WallList::Segments] {
        let mut index = 0;
        while index < world.wall_count(component, list)? {
            if let Some(actor) = world.wall_entry(component, list, index)? {
                world.change_kingdom(actor, new)?;
            }
            index += 1;
        }
    }
    Ok(())
}

/// Native `0x585FF0(kingdom)`: `seen` is the `+70` list from its head (the
/// `+74` tail is its last entry).
pub fn seen_by(seen: &mut Vec<u32>, kingdom: u32) {
    if !seen.contains(&kingdom) {
        seen.insert(0, kingdom);
    }
}
