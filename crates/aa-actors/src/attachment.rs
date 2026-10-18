//! Ordinary Structure/Settlement attachment predicates (4CE980/588990/4CE9A0).
//! Object identities refer to the single world arena, not native pointers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("native attachment requires {0}")]
    Missing(&'static str),
    #[error("attachment lookup failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Structure<A> {
    /// Native component+4.
    pub owner: A,
    /// Native component+10.
    pub parent: Option<A>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settlement<A> {
    /// Native component+14.
    pub attached_actor: Option<A>,
}
pub trait World {
    type Actor: Copy + Eq;
    /// Actor+94; null is a valid absence.
    fn structure(&self, actor: Self::Actor) -> Result<Option<Structure<Self::Actor>>>;
    /// Actor+98; native588990 requires nonnull after a nonnull Structure parent.
    fn settlement(&self, actor: Self::Actor) -> Result<Option<Settlement<Self::Actor>>>;
    /// Actual actor virtual+158 dispatch; derived overrides must be honored.
    fn is_attached_virtual(&mut self, actor: Self::Actor) -> Result<bool>;
}
pub fn structure_is_attached<W: World>(structure: Structure<W::Actor>, world: &W) -> Result<bool> {
    let Some(parent) = structure.parent else {
        return Ok(false);
    };
    let settlement = world.settlement(parent)?.ok_or(Error::Missing(
        "parent Settlement for nonnull Structure parent",
    ))?;
    Ok(settlement.attached_actor == Some(structure.owner))
}
pub fn is_attached<W: World>(actor: W::Actor, world: &W) -> Result<bool> {
    match world.structure(actor)? {
        Some(s) => structure_is_attached(s, world),
        None => Ok(false),
    }
}
/// Rereads actual Structure after the virtual callback, as native4CE9A0 does.
/// Returning true then removing Structure is an invalid native read, not None.
pub fn parent<W: World>(actor: W::Actor, world: &mut W) -> Result<Option<W::Actor>> {
    if !world.is_attached_virtual(actor)? {
        return Ok(None);
    }
    Ok(world
        .structure(actor)?
        .ok_or(Error::Missing(
            "Structure after successful attachment virtual",
        ))?
        .parent)
}
