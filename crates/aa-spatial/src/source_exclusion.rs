//! Ordinary Actor virtual+30 (4D5600) and base4EB7D0 exclusion rules.
//! Component/owner dispatch remains explicit and ordered; a missing field is
//! not interpreted as friendly, empty, or collision-free.
use crate::collision::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[cfg(test)]
#[path = "source_exclusion_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub definition: u32,
    pub reference: u32,
    pub flags: u32,
    pub position: [u32; 2],
    pub linked_50: u32,
    pub linked_168: u32,
    pub has_element: bool,
    pub has_wall: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Definition {
    /// ConstructionComponent.required_object resolved identity, native+4E0.
    pub required_object: Option<u32>,
    /// Global Settlement loader marks referenced wall definitions at+1A8.
    pub settlement_wall: bool,
    /// Native definition+42C WallComponent.gate.
    pub gate: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ExclusionNode {
    pub reference: u32,
    pub next: Option<u32>,
}
pub trait World {
    type Actor: Copy + Eq;
    type Mind: Copy;
    type AcceptanceDispatch: Copy;
    type Owner: Copy;
    fn actor(&self, actor: Self::Actor) -> Result<Actor>;
    fn definition(&self, definition: u32) -> Result<Definition>;
    fn resolve(&self, reference: u32) -> Result<Option<Self::Actor>>;
    fn exclusion_head(&self, source: Self::Actor) -> Result<Option<u32>>;
    fn exclusion_node(&self, node: u32) -> Result<ExclusionNode>;
    /// Actual candidate virtual+B4, evaluated after base/list/definition checks.
    fn actor_category(&mut self, actor: Self::Actor) -> Result<bool>;
    /// Capture the candidate vtable identity BEFORE source virtual+108. The
    /// +138 entry is read from that retained table when accepts_owner executes.
    fn acceptance_dispatch(&self, actor: Self::Actor) -> Result<Self::AcceptanceDispatch>;
    fn owner(&mut self, actor: Self::Actor) -> Result<Self::Owner>;
    fn accepts_owner(
        &mut self,
        dispatch: Self::AcceptanceDispatch,
        actor: Self::Actor,
        owner: Self::Owner,
    ) -> Result<bool>;
    /// Element+10, read only after has_element. Null component is an error.
    fn organization(&self, actor: Self::Actor) -> Result<Option<Self::Actor>>;
    fn mind(&self, actor: Self::Actor) -> Result<Option<Self::Mind>>;
    fn mind_ready_64(&mut self, mind: Self::Mind) -> Result<bool>;
    ///58CBC0 on the freshly read candidate Wall, with source actor argument.
    fn wall_allows_source(&mut self, wall_actor: Self::Actor, source: Self::Actor) -> Result<bool>;
    /// Actual candidate virtual+100 then4D36F0(source,parent).
    fn parent(&mut self, actor: Self::Actor) -> Result<Option<Self::Actor>>;
    fn parent_collision_required(
        &mut self,
        source: Self::Actor,
        parent: Option<Self::Actor>,
    ) -> Result<bool>;
}
///4EB7D0 self or source+50 generation-valid link. Zero link skips lookup.
pub fn base_excludes<W: World>(source: W::Actor, candidate: W::Actor, w: &W) -> Result<bool> {
    if source == candidate {
        return Ok(true);
    }
    let link = w.actor(source)?.linked_50;
    Ok(link != 0 && w.resolve(link)? == Some(candidate))
}
///4EE780 is definition identity and ordered XY equality, not a range check.
///Signed zeros compare equal; unordered NaNs never match, as in FUCOMPP.
pub fn same_definition_position(
    candidate: &Actor,
    definition: Option<u32>,
    point: [u32; 2],
) -> bool {
    definition == Some(candidate.definition)
        && candidate
            .position
            .into_iter()
            .zip(point)
            .all(|(a, b)| f32::from_bits(a) == f32::from_bits(b))
}
fn accepts<W: World>(source: W::Actor, candidate: W::Actor, w: &mut W) -> Result<bool> {
    let dispatch = w.acceptance_dispatch(candidate)?;
    let owner = w.owner(source)?;
    w.accepts_owner(dispatch, candidate, owner)
}
/// Ordinary4D1EB0: pointer equality (including null/null) succeeds without a
/// relation callback; otherwise virtual+144 must return exactly1.
pub fn ordinary_accepts_owner<T: Copy + Eq>(
    candidate_owner: Option<T>,
    owner: Option<T>,
    relation: impl FnOnce() -> Result<u32>,
) -> Result<bool> {
    if candidate_owner == owner {
        Ok(true)
    } else {
        Ok(relation()? == 1)
    }
}
///4D5600. The exclusion list captures candidate GTID once; every field after a
///virtual call is freshly read at its original boundary. Errors retain effects.
pub fn excludes<W: World>(source: W::Actor, candidate: W::Actor, w: &mut W) -> Result<bool> {
    if base_excludes(source, candidate, w)? {
        return Ok(true);
    }
    let mut node = w.exclusion_head(source)?;
    if node.is_some() {
        let reference = w.actor(candidate)?.reference;
        let mut seen = BTreeSet::new();
        while let Some(key) = node {
            if !seen.insert(key) {
                return Err(Error::World("cyclic native source exclusion list".into()));
            }
            let entry = w.exclusion_node(key)?;
            if entry.reference == reference {
                return Ok(true);
            }
            node = entry.next;
        }
    }
    let required = w.definition(w.actor(source)?.definition)?.required_object;
    let point = w.actor(source)?.position;
    if same_definition_position(&w.actor(candidate)?, required, point) {
        return Ok(true);
    }
    if !w.actor_category(candidate)? {
        return Ok(false);
    }
    if w.actor(source)?.has_element
        && w.actor(candidate)?.has_element
        && accepts(source, candidate, w)?
    {
        let own = w.organization(source)?;
        let other = w.organization(candidate)?;
        if let Some(own) = own
            && let Some(other) = other
            && let Some(mind) = w.mind(own)?
            && w.mind_ready_64(mind)?
            && let Some(mind) = w.mind(other)?
            && w.mind_ready_64(mind)?
        {
            return Ok(true);
        }
    }
    if w.definition(w.actor(source)?.definition)?.settlement_wall {
        return Ok(true);
    }
    let other_definition = w.actor(candidate)?.definition;
    let wall_mark = w.definition(other_definition)?.settlement_wall;
    if wall_mark {
        let own = w.actor(source)?;
        if own.flags & (1 << 10) == 0 || own.flags & (1 << 29) == 0 || own.has_wall {
            return Ok(true);
        }
    } else if w.actor(source)?.has_wall {
        if w.actor(candidate)?.has_wall {
            return Ok(true);
        }
        //4D571F bypasses the gate path when only the source has a Wall.
        return Ok(w.resolve(w.actor(source)?.linked_168)? == Some(candidate));
    }
    if w.actor(candidate)?.has_wall
        && w.definition(other_definition)?.gate
        && (accepts(source, candidate, w)? || w.wall_allows_source(candidate, source)?)
    {
        let parent = w.parent(candidate)?;
        if !w.parent_collision_required(source, parent)? {
            return Ok(true);
        }
    }
    Ok(w.resolve(w.actor(source)?.linked_168)? == Some(candidate))
}
