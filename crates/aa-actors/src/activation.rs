//! Native active-component switch `4CFC10(component)` (actor `+E4`): the
//! Character and Mover take turns being the active component whenever a
//! unit starts or stops moving.
//!
//! With actor `+8` bit 0 set, a new component must accept through its
//! virtual `+44`. The previous active component gets `+40(same)`, then the
//! component now in `+E4` (reread: the `+40` callee may change it) gets
//! `+3C(same)`; `same` is true when the component was already active. The
//! native passes `same` as the low byte of a word whose upper bytes are left
//! over from the component pointer; only the byte is meaningful.
//!
//! Oracle: research/phase8-combat/native_activation.py.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("active component callback failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub trait World {
    /// Actor `+8` word.
    fn actor_flags(&mut self, actor: u32) -> Result<u32>;
    /// Actor `+E4`.
    fn active_component(&mut self, actor: u32) -> Result<Option<u32>>;
    fn set_active_component(&mut self, actor: u32, component: Option<u32>) -> Result<()>;
    /// Component virtual `+44`.
    fn accepts_activation(&mut self, component: u32) -> Result<bool>;
    /// Component virtual `+40(same)`.
    fn deactivated(&mut self, component: u32, same: bool) -> Result<()>;
    /// Component virtual `+3C(same)`.
    fn activated(&mut self, component: u32, same: bool) -> Result<()>;
}

/// Native `4CFC10(component)` on `actor`; true when a component is active
/// afterwards and was told so.
pub fn activate(world: &mut impl World, actor: u32, component: Option<u32>) -> Result<bool> {
    let mut same = false;
    if let Some(component) = component {
        if world.actor_flags(actor)? & 1 != 0 && !world.accepts_activation(component)? {
            return Ok(false);
        }
        same = world.active_component(actor)? == Some(component);
    }
    let previous = world.active_component(actor)?;
    world.set_active_component(actor, component)?;
    if let Some(previous) = previous {
        world.deactivated(previous, same)?;
    }
    let Some(current) = world.active_component(actor)? else {
        return Ok(false);
    };
    world.activated(current, same)?;
    Ok(true)
}
