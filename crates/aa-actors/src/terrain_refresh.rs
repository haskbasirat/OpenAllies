//! Native actor terrain refresh `4D4130` (GActor.cpp UpdateTerrain).
//!
//! Skipped when actor `+8` bit 29 or bit 12 or `+100` bit 24 is set, or
//! `4D10A0` refuses. The terrain (`+F8`) is the membership's (`570560` on
//! `+80`) or the map tile's at `+44/+48` (`54ECE0` on the `[B36534]+30` grid
//! record). When it changed: the OrganizationComponent's `579EB0`, then, with
//! `+78` properties, every property of the old terrain's `+B0` list is
//! removed (`581810(property, actor)`, null entries included) and every
//! non-null property of the new terrain's list that applies (`5035E0`) is
//! added with kind 5 (`5817A0`). A null new terrain is logged and then
//! dereferenced by the native; the port reports it as an error.
//!
//! Oracle: research/phase8-combat/native_terrain_refresh.py.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("terrain refresh callback failed: {0}")]
    World(String),
    #[error("4D4130 null terrain (the native logs it, then dereferences it)")]
    NullTerrain,
}
pub type Result<T> = std::result::Result<T, Error>;

pub trait World {
    /// Actor `+8` word.
    fn actor_flags(&mut self, actor: u32) -> Result<u32>;
    /// `4D10A0` on the actor.
    fn check_4d10a0(&mut self, actor: u32) -> Result<bool>;
    /// Actor `+100` word.
    fn word100(&mut self, actor: u32) -> Result<u32>;
    /// Actor `+F8` terrain.
    fn terrain(&mut self, actor: u32) -> Result<Option<u32>>;
    fn set_terrain(&mut self, actor: u32, terrain: Option<u32>) -> Result<()>;
    /// Actor `+80` membership.
    fn membership(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `570560` on the membership.
    fn membership_terrain(&mut self, membership: u32) -> Result<Option<u32>>;
    /// Actor `+44/+48` tile.
    fn tile(&mut self, actor: u32) -> Result<[u32; 2]>;
    /// `54ECE0` on the grid record of the tile.
    fn tile_terrain(&mut self, tile: [u32; 2]) -> Result<Option<u32>>;
    /// Actor `+7C` OrganizationComponent.
    fn organization(&mut self, actor: u32) -> Result<Option<u32>>;
    /// `579EB0` on the OrganizationComponent.
    fn organization_refresh(&mut self, organization: u32) -> Result<()>;
    /// Actor `+78` properties.
    fn properties(&mut self, actor: u32) -> Result<Option<u32>>;
    /// The values of the terrain's `+B0` property list, head first.
    fn terrain_properties(&mut self, terrain: u32) -> Result<Vec<Option<u32>>>;
    /// `581810(property, actor)` on the properties.
    fn remove(&mut self, properties: u32, property: Option<u32>, actor: u32) -> Result<()>;
    /// `5035E0(actor, property)`.
    fn applicable(&mut self, actor: u32, property: u32) -> Result<bool>;
    /// `5817A0(property, kind, actor)` on the properties.
    fn add(&mut self, properties: u32, property: u32, kind: u32, actor: u32) -> Result<()>;
}

/// Native `4D4130` on `actor`.
pub fn refresh_terrain(world: &mut impl World, actor: u32) -> Result<()> {
    let flags = world.actor_flags(actor)?;
    if flags >> 29 & 1 != 0
        || !world.check_4d10a0(actor)?
        || world.word100(actor)? & 0x0100_0000 != 0
        || flags >> 12 & 1 != 0
    {
        return Ok(());
    }
    let old = world.terrain(actor)?;
    let new = match world.membership(actor)? {
        Some(membership) => world.membership_terrain(membership)?,
        None => {
            let tile = world.tile(actor)?;
            world.tile_terrain(tile)?
        }
    };
    world.set_terrain(actor, new)?;
    if new == old {
        return Ok(());
    }
    if let Some(organization) = world.organization(actor)? {
        world.organization_refresh(organization)?;
    }
    if world.properties(actor)?.is_none() {
        return Ok(());
    }
    if let Some(old) = old {
        for property in world.terrain_properties(old)? {
            let properties = world
                .properties(actor)?
                .ok_or_else(|| Error::World("4D4130 properties vanished".into()))?;
            world.remove(properties, property, actor)?;
        }
    }
    let new = world.terrain(actor)?.ok_or(Error::NullTerrain)?;
    for property in world.terrain_properties(new)?.into_iter().flatten() {
        if let Some(properties) = world.properties(actor)?
            && world.applicable(actor, property)?
        {
            world.add(properties, property, 5, actor)?;
        }
    }
    Ok(())
}
