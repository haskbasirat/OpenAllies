//! Original Element command6 handler52D910: activate embedded Sleep68 in
//! mode2, then match a present Element leader's facing through4EEBE0.
use crate::{
    Arena, Error, Result,
    construct::{Installed, Kind},
};
pub trait World: crate::World {
    /// Actual actor+80 Element, then its+10 leader. Absence is a native branch.
    fn element_leader(&self, actor: u32) -> Result<Option<u32>>;
    fn actor_facing(&self, actor: u32) -> Result<[u32; 2]>;
    /// Must call the concrete actor/spatial4EEBE0 service (SpatialIndex::set_facing)
    /// with its real membership/presentation effects, not only change a UI angle.
    fn set_actor_facing(&mut self, actor: u32, facing: [u32; 2], arena: &mut Arena) -> Result<()>;
}
pub fn apply(installed: &Installed, arena: &mut Arena, world: &mut impl World) -> Result<bool> {
    if installed.kind != Kind::Element {
        return Err(Error::World("52D910 requires an Element Mind".into()));
    }
    let mind = installed.mind_id;
    let sleep = *installed
        .state_bindings
        .get(&0x68)
        .ok_or_else(|| Error::World("Element Sleep68 binding missing".into()))?;
    if arena.state(sleep)?.parent != mind {
        return Err(Error::World(
            "Element Sleep68 parent binding mismatch".into(),
        ));
    }
    if !crate::activate(sleep, 2, arena, world)? {
        return Ok(false);
    }
    // Activation callbacks may replace the actor; this native read is afterwards.
    let actor = arena.mind(mind)?.actor;
    if let Some(leader) = world.element_leader(actor)? {
        let facing = world.actor_facing(leader)?;
        world.set_actor_facing(actor, facing, arena)?;
    }
    Ok(true)
}
