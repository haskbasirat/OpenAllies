//! Original Recruit action contributor582C40. Writes actor action bits14
//! (commission),15 (recruit),17 (recruit at location),16 (rally), in that order.
//! The bit14 write precedes recruitment eligibility calls; failures retain it.
//! Source definitions and eligibility5827B0 are required live dependencies.
use crate::{
    ActorId,
    ability_flags::{ActorFields, Error, Result, set_action},
    state,
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry<D, L> {
    pub definition: D,
    /// ActorData+184 !=0 distinguishes the location-recruit count.
    pub location_class: bool,
    /// Entry ActorData+2F0 default LayoutParameters (null is permitted).
    pub default_layout: L,
}

/// Typed field operations must read authoritative current state. They do not
/// introduce gameplay callbacks; setters only write the supplied action mask.
pub trait World {
    type Cursor: Clone + Ord;
    type Definition;
    type Layout;
    /// Stable Recruit component identity is represented by its storage owner;
    /// return its current header+4 owner on every read, including after5827B0.
    fn recruit_owner(&self, recruit: ActorId) -> Result<ActorId>;
    fn fields(&self, actor: ActorId) -> Result<ActorFields>;
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> Result<()>;
    /// Actual ActorStateData+28 for the selected native state index.
    fn state_type(&self, state_index: u8) -> Result<u32>;
    /// Outer None: no Settlement component; inner None: null Settlement+14.
    fn settlement_reference(&self, actor: ActorId) -> Result<Option<Option<ActorId>>>;
    /// Recruit+11 (ordinary) or+10 (location); nonzero means enabled.
    fn recruit_enabled(&self, recruit: ActorId, location: bool) -> Result<u8>;
    /// Current owner's ActorData+4EC. Called twice for separate admission
    /// gates and again immediately before traversal, exactly as native.
    fn recruit_list_head(&self, actor: ActorId) -> Result<Option<Self::Cursor>>;
    fn recruit_entry(&self, cursor: &Self::Cursor)
    -> Result<Entry<Self::Definition, Self::Layout>>;
    /// The current node's next pointer is read after its eligibility callback.
    fn next_recruit_entry(&self, cursor: &Self::Cursor) -> Result<Option<Self::Cursor>>;
    /// Original5827B0(definition, default_layout,1,0): check requirements,
    /// no diagnostic destination. Only result2 excludes an entry. All other
    /// values count; missing concrete eligibility must return an error.
    fn eligibility(
        &mut self,
        recruit: ActorId,
        definition: &Self::Definition,
        layout: &Self::Layout,
    ) -> Result<u32>;
}

fn selected_state(world: &impl World, actor: ActorId) -> Result<u8> {
    state::group(world.fields(actor)?.state_bits, 13)
        .map_err(|e| Error::World(e.to_string()))?
        .map(|d| d.index)
        .ok_or(Error::Missing(
            "582C40 group13 selected -1; native ActorState lookup is invalid",
        ))
}
fn write(world: &mut impl World, recruit: ActorId, bit: u32, enabled: bool) -> Result<()> {
    let actor = world.recruit_owner(recruit)?;
    let mut bits = world.fields(actor)?.action_bits;
    set_action(&mut bits, bit, enabled)?;
    world.set_action_bits(actor, bits)
}

/// Original582C40, using the caller's stable Recruit component storage owner.
pub fn contribute(world: &mut impl World, recruit: ActorId) -> Result<()> {
    let actor = world.recruit_owner(recruit)?;
    let mut selected = selected_state(world, actor)?;
    let mut blocked = world.state_type(selected)? == 1;
    let under_siege = world.fields(world.recruit_owner(recruit)?)?.state_bits & (1 << 20) != 0;
    if !blocked {
        if let Some(Some(target)) = world.settlement_reference(world.recruit_owner(recruit)?)? {
            selected = selected_state(world, target)?;
            blocked = world.state_type(selected)? == 1;
        }
    }
    let commission = (!blocked || selected == 29)
        && !under_siege
        && world
            .settlement_reference(world.recruit_owner(recruit)?)?
            .is_some();
    write(world, recruit, 14, commission)?;
    let mut ordinary = false;
    let mut location = false;
    if !blocked {
        ordinary = world
            .recruit_list_head(world.recruit_owner(recruit)?)?
            .is_some()
            && world.recruit_enabled(recruit, false)? != 0;
        location = world
            .recruit_list_head(world.recruit_owner(recruit)?)?
            .is_some()
            && world.recruit_enabled(recruit, true)? != 0;
    }
    if ordinary || location {
        let mut cursor = world.recruit_list_head(world.recruit_owner(recruit)?)?;
        let mut ordinary_count = 0i32;
        let mut location_count = 0i32;
        let mut visited = BTreeSet::new();
        while let Some(current) = cursor {
            if !visited.insert(current.clone()) {
                return Err(Error::Missing(
                    "582C40 recruit list cycle; native traversal does not terminate",
                ));
            }
            let entry = world.recruit_entry(&current)?;
            let eligible = world.eligibility(recruit, &entry.definition, &entry.default_layout)?;
            if eligible != 2 {
                if entry.location_class {
                    location_count = location_count.wrapping_add(1);
                } else {
                    ordinary_count = ordinary_count.wrapping_add(1);
                }
            }
            cursor = world.next_recruit_entry(&current)?;
        }
        ordinary &= ordinary_count > 0;
        location &= location_count > 0;
    }
    write(world, recruit, 15, ordinary)?;
    write(world, recruit, 17, location)?;
    write(
        world,
        recruit,
        16,
        (!blocked || selected == 29) && (commission || ordinary),
    )
}
