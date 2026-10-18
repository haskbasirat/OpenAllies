//! Native 4CEDE0 and its 578D00/5780F0/578E10 leaves. This evaluates existing
//! movement state; it does not advance orders or synthesize component state.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("movement state world failed: {0}")]
    World(String),
    #[error("native moving-state actor links contain a cycle at {0}")]
    Cycle(u32),
    #[error("could not allocate movement traversal storage")]
    Allocation,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub component7c: Option<u32>,
    pub component6c: Option<u32>,
    pub active_component_e4: Option<u32>,
    pub active_state70: Option<u32>,
    pub position: [u32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Component {
    /// Component+4, used for both active-state and position queries.
    pub owner: u32,
    pub linked_actor40: Option<u32>,
    pub comparison_position: [u32; 2],
}
/// Pure coherent field snapshots. Native helpers perform no callbacks and do
/// not mutate the world. Missing components must produce errors, not defaults.
pub trait World {
    fn actor(&self, id: u32) -> Result<Actor>;
    fn component(&self, id: u32) -> Result<Component>;
    fn state_kind74(&self, id: u32) -> Result<u32>;
}

/// x87 ordered equality: opposite signed zero is equal, NaNs never are. No
/// arithmetic is performed, so this also preserves infinity and subnormal bits.
fn equal(a: u32, b: u32) -> bool {
    let nan = |x: u32| x & 0x7fff_ffff > 0x7f80_0000;
    !nan(a) && !nan(b) && (a == b || a & 0x7fff_ffff == 0 && b & 0x7fff_ffff == 0)
}
fn displaced<W: World>(component: u32, world: &W) -> Result<bool> {
    let c = world.component(component)?;
    let position = world.actor(c.owner)?.position;
    Ok(!equal(position[0], c.comparison_position[0])
        || !equal(position[1], c.comparison_position[1]))
}

/// Evaluate 4CEDE0. Iterative traversal avoids an artificial Rust recursion cap;
/// a native infinite recursive link is reported explicitly as an invalid cycle.
pub fn excluded<W: World>(actor: u32, world: &W) -> Result<bool> {
    let mut current = actor;
    let mut fallback = Vec::new();
    let mut visited = BTreeSet::new();
    let result = loop {
        if !visited.insert(current) {
            return Err(Error::Cycle(current));
        }
        let a = world.actor(current)?;
        let Some(component) = a.component7c else {
            break a.component6c.is_some() && a.component6c == a.active_component_e4;
        };
        let c = world.component(component)?;
        let Some(state) = world.actor(c.owner)?.active_state70 else {
            break false;
        };
        if world.state_kind74(state)? == 2
            && let Some(link) = c.linked_actor40
        {
            fallback.try_reserve(1).map_err(|_| Error::Allocation)?;
            fallback.push(component);
            current = link;
        } else {
            break displaced(component, world)?;
        }
    };
    if result {
        return Ok(true);
    }
    while let Some(component) = fallback.pop() {
        if displaced(component, world)? {
            return Ok(true);
        }
    }
    Ok(false)
}
