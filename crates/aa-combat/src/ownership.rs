//! Native `KKC_GActor::ChangeKingdom` (`4D8450`, actor virtual `+160`): the
//! ownership change performed by capture (`4D5230`) and by every other owner
//! transfer. The order of the component and kingdom hand-over is the
//! original's; every callee is a required world service:
//!
//! - kingdom economy/registration (aa-economy owner): `4E44E0` remove actor,
//!   `4E6180` add actor, recruit singles on kingdom `+1FC` (`4E2A60` find,
//!   `4E2CB0` remove, `4E4C30` add);
//! - components: `589CC0`/`589050` (`+94`), `565400`/`565700` (`+9C`
//!   construction/upgrade cancel), `4D19F0` (`+160`), `4DFF30` (`+164`
//!   GTID), `574590` (`+D8`), `584F10` (wall `+98`), `579F10`
//!   (OrganizationComponent `+7C`), `56ADA0` (Denizen `+88`), `5643A0`
//!   (`+D4`), `5714D0` (membership release);
//! - recursion: outer actor, membership organization actor and the Mind
//!   `+28` actor receive their own virtual `+160` (bind to this function or
//!   to a derived override);
//! - visibility: the 32-bit native mask of kingdoms that saw the actor
//!   (virtual `+70`), re-revealed through `4ECB90` and `585FF0`. Kingdom
//!   indices alias modulo 32 exactly as the native `shl` does.
use crate::{Error, Result};

pub trait World {
    /// Actor `+8` bit23 (ownable).
    fn ownable(&self, actor: u32) -> Result<bool>;
    /// GActor.cpp line 0x69D error log.
    fn not_ownable(&mut self, actor: u32) -> Result<()>;
    /// Actor `+E8`.
    fn kingdom(&self, actor: u32) -> Result<Option<u32>>;
    fn set_kingdom(&mut self, actor: u32, kingdom: Option<u32>) -> Result<()>;
    fn flags100(&self, actor: u32) -> Result<u32>;
    /// `+90` wall link: when absent the change is logged (line 0x6A5).
    fn has_wall_link(&self, actor: u32) -> Result<bool>;
    fn change_log(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    /// SAI global `B36528` with byte `+20`, then `7A0A40(actor, +140, old, new)`.
    fn sai_active(&self) -> Result<bool>;
    fn sai_change(&mut self, actor: u32, old: Option<u32>, new: Option<u32>) -> Result<()>;
    /// World `+158` count and `+154` table.
    fn kingdom_count(&self) -> Result<i32>;
    fn kingdom_at(&self, index: i32) -> Result<Option<u32>>;
    /// Actor virtual `+70(kingdom, 1)`.
    fn visible_to(&mut self, actor: u32, kingdom: u32) -> Result<bool>;
    /// Kingdom `+18C` index when `+190` is zero (otherwise `None`).
    fn own_index(&self, kingdom: u32) -> Result<Option<u32>>;
    /// `4D5DE0`, `4CFD30` and the final free of the snapshot list.
    fn snapshot_begin(&mut self, actor: u32) -> Result<()>;
    fn snapshot_end(&mut self, actor: u32) -> Result<()>;
    fn snapshot_release(&mut self, actor: u32) -> Result<()>;
    fn has_component(&self, actor: u32, offset: u32) -> Result<bool>;
    fn research_detach(&mut self, actor: u32) -> Result<()>;
    fn construction_cancel(&mut self, actor: u32) -> Result<()>;
    fn upgrade_cancel(&mut self, actor: u32) -> Result<()>;
    fn object160(&mut self, actor: u32) -> Result<()>;
    /// `+164` GTID resolves, then `4DFF30(actor)` on it.
    fn object164_live(&self, actor: u32) -> Result<bool>;
    fn object164(&mut self, actor: u32) -> Result<()>;
    /// Kingdom `+1FC` recruit singles for the actor definition.
    fn find_single(&mut self, kingdom: u32, actor: u32) -> Result<Option<u32>>;
    fn remove_single(&mut self, kingdom: u32, single: u32) -> Result<()>;
    fn add_single(&mut self, kingdom: u32, single: u32) -> Result<()>;
    fn free_single(&mut self, single: u32) -> Result<()>;
    fn duplicate_single(&mut self, actor: u32, kingdom: u32) -> Result<()>;
    fn kingdom_remove_actor(&mut self, kingdom: u32, actor: u32) -> Result<()>;
    fn kingdom_add_actor(&mut self, kingdom: u32, actor: u32) -> Result<()>;
    fn component_d8(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    /// Membership `+10` organization actor (outer `None` = no membership).
    fn membership_organization(&self, actor: u32) -> Result<Option<Option<u32>>>;
    fn membership_release(&mut self, actor: u32) -> Result<()>;
    fn request_removal(&mut self, actor: u32) -> Result<()>;
    /// `50C2B0(world+130 manager; actor, had_object160)`.
    fn manager(&mut self, actor: u32, had_object160: bool) -> Result<()>;
    fn outer(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Another actor's virtual `+160(new)`.
    fn change_kingdom(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    fn wall_kingdom(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    fn organization_kingdom(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    /// Mind present and its `+28` GTID: `Some(Some(object))` when it resolves.
    fn mind_object(&self, actor: u32) -> Result<Option<Option<u32>>>;
    fn denizen_kingdom(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    fn object_d4_kingdom(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    fn research_kingdom(&mut self, actor: u32, new: Option<u32>) -> Result<()>;
    fn notify(&mut self, actor: u32) -> Result<()>;
    fn update_0ea0(&mut self, actor: u32) -> Result<()>;
    /// Virtuals `+8C` (re-reveal wanted) and `+15C` (wall-revealed object).
    fn virtual8c(&mut self, actor: u32) -> Result<bool>;
    fn virtual15c(&mut self, actor: u32) -> Result<Option<u32>>;
    fn reveal(&mut self, actor: u32, kingdom: Option<u32>) -> Result<()>;
    fn wall_reveal(&mut self, object: u32, kingdom: Option<u32>) -> Result<()>;
}

/// Native `4D8450(new)` on `actor`.
pub fn change_kingdom(actor: u32, new: Option<u32>, world: &mut impl World) -> Result<()> {
    let new = if world.ownable(actor)? {
        new
    } else {
        world.not_ownable(actor)?;
        None
    };
    if world.kingdom(actor)? == new {
        return Ok(());
    }
    if !world.has_wall_link(actor)? {
        world.change_log(actor, new)?;
    }
    if world.sai_active()? {
        let old = world.kingdom(actor)?;
        world.sai_change(actor, old, new)?;
    }
    let mut mask = 0u32;
    let mut index = 0;
    while index < world.kingdom_count()? - 1 {
        if index < world.kingdom_count()?
            && let Some(kingdom) = world.kingdom_at(index)?
            && world.visible_to(actor, kingdom)?
        {
            mask |= 1u32 << (index as u32 & 31);
        }
        index += 1;
    }
    if let Some(kingdom) = new
        && let Some(own) = world.own_index(kingdom)?
    {
        mask &= !(1u32 << (own & 31));
    }
    world.snapshot_begin(actor)?;
    if world.has_component(actor, 0x94)? {
        world.research_detach(actor)?;
    }
    if world.flags100(actor)? & 0x4000_0000 != 0 {
        world.construction_cancel(actor)?;
    }
    if world.flags100(actor)? & 0x2000_0000 != 0 {
        world.upgrade_cancel(actor)?;
    }
    let had_object160 = world.has_component(actor, 0x160)?;
    if had_object160 {
        world.object160(actor)?;
    }
    if world.object164_live(actor)? {
        world.object164(actor)?;
    }
    let mut single = None;
    if let Some(old) = world.kingdom(actor)? {
        single = world.find_single(old, actor)?;
        if let Some(s) = single {
            world.remove_single(old, s)?;
        }
    }
    if let Some(old) = world.kingdom(actor)? {
        world.kingdom_remove_actor(old, actor)?;
    }
    if world.has_component(actor, 0xd8)? {
        world.component_d8(actor, new)?;
    }
    world.set_kingdom(actor, new)?;
    if let (Some(kingdom), Some(s)) = (new, single) {
        if world.find_single(kingdom, actor)?.is_some() {
            world.duplicate_single(actor, kingdom)?;
            if matches!(world.membership_organization(actor)?, Some(Some(_))) {
                world.membership_release(actor)?;
            } else {
                world.request_removal(actor)?;
            }
            world.free_single(s)?;
            return world.snapshot_release(actor);
        }
        world.add_single(kingdom, s)?;
    }
    if let Some(kingdom) = world.kingdom(actor)? {
        world.kingdom_add_actor(kingdom, actor)?;
    }
    world.manager(actor, had_object160)?;
    if world.outer(actor)?.is_some() {
        let outer = world
            .outer(actor)?
            .ok_or(Error::Invalid("outer actor vanished between native calls"))?;
        let current = world.kingdom(actor)?;
        world.change_kingdom(outer, current)?;
    } else if world.has_component(actor, 0x98)? {
        let current = world.kingdom(actor)?;
        world.wall_kingdom(actor, current)?;
    }
    if let Some(Some(organization)) = world.membership_organization(actor)? {
        let current = world.kingdom(actor)?;
        world.change_kingdom(organization, current)?;
    } else if world.has_component(actor, 0x7c)? {
        let current = world.kingdom(actor)?;
        world.organization_kingdom(actor, current)?;
    }
    match world.mind_object(actor)? {
        Some(Some(object)) => {
            let current = world.kingdom(actor)?;
            world.change_kingdom(object, current)?;
        }
        _ => {
            if world.has_component(actor, 0x88)? {
                let current = world.kingdom(actor)?;
                world.denizen_kingdom(actor, current)?;
            }
        }
    }
    if world.has_component(actor, 0xd4)? {
        let current = world.kingdom(actor)?;
        world.object_d4_kingdom(actor, current)?;
    }
    if world.has_component(actor, 0x94)? {
        let current = world.kingdom(actor)?;
        world.research_kingdom(actor, current)?;
    }
    world.snapshot_end(actor)?;
    world.notify(actor)?;
    world.update_0ea0(actor)?;
    if world.virtual8c(actor)? && mask != 0 {
        let mut index = 0;
        while index < world.kingdom_count()? - 1 {
            if mask & (1u32 << (index as u32 & 31)) != 0 {
                let kingdom = if index < world.kingdom_count()? {
                    world.kingdom_at(index)?
                } else {
                    None
                };
                world.reveal(actor, kingdom)?;
            }
            index += 1;
        }
    }
    if let Some(object) = world.virtual15c(actor)? {
        let mut index = 0;
        while index < world.kingdom_count()? - 1 {
            if mask & (1u32 << (index as u32 & 31)) != 0 {
                let kingdom = if index < world.kingdom_count()? {
                    world.kingdom_at(index)?
                } else {
                    None
                };
                world.wall_reveal(object, kingdom)?;
            }
            index += 1;
        }
    }
    world.snapshot_release(actor)
}
