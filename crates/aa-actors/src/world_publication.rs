//! Original Actor PostCreate4CF300, including Object4EBA40 and fixed properties.
//! World services are synchronous and fallible. Earlier publication effects are
//! retained on error; retrying a partially published actor is not automatic.
use crate::{
    ActorId,
    components::{ALL_SLOTS, ComponentSlot},
};
use aa_body::credit::Object;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("publication requires missing or unsupported operation: {0}")]
    Unsupported(&'static str),
    #[error("publication callback failed: {0}; prior effects remain committed")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;
pub trait CreateWorld {
    type Record;
    fn has_component(&self, actor: ActorId, slot: ComponentSlot) -> Result<bool>;
    /// Native GWORLD+28 (map settings) +A8 word, read only for flag low-byte zero.
    fn map_mode_a8(&self) -> Result<u32>;
    /// Actual4EB500 ground deformation/paint, including all ordinary early-outs.
    fn object_ground_post_create(&mut self, actor: ActorId, record: &Self::Record) -> Result<()>;
    fn component_post_create(
        &mut self,
        actor: ActorId,
        slot: ComponentSlot,
        record: &Self::Record,
        flag: u32,
    ) -> Result<()>;
    /// Direct current Actor+E8 owner field; not its virtual allegiance query.
    fn stored_owner(&self, actor: ActorId) -> Result<Option<ActorId>>;
    fn owner_post_create_4e6180(&mut self, owner: ActorId, actor: ActorId) -> Result<()>;
    fn definition_mask(&self, actor: ActorId) -> Result<u32>;
    fn city_predicate_45d6a0(&mut self, actor: ActorId) -> Result<bool>;
    fn city_argument_34(&self, record: &Self::Record) -> Result<u32>;
    fn city_post_create_5644f0(&mut self, actor: ActorId, argument: u32) -> Result<()>;
}
/// Full4CF300 and4EBA40 caller. Native base Thing4D1E10 is literally RET8.
/// Every component slot is reread after prior callbacks. The full native flag
/// word is forwarded; only the native low byte controls the two outer gates.
pub fn post_create<W: CreateWorld>(
    world: &mut W,
    actor: ActorId,
    record: &W::Record,
    flag: u32,
) -> Result<()> {
    if flag as u8 == 0 && world.map_mode_a8()? == 0 {
        world.object_ground_post_create(actor, record)?;
    }
    for slot in ALL_SLOTS {
        if world.has_component(actor, slot)? {
            world.component_post_create(actor, slot, record, flag)?;
        }
    }
    if let Some(owner) = world.stored_owner(actor)? {
        world.owner_post_create_4e6180(owner, actor)?;
    }
    if flag as u8 == 0
        && world.definition_mask(actor)? & (1 << 29) != 0
        && !world.city_predicate_45d6a0(actor)?
    {
        let value = world.city_argument_34(record)?;
        world.city_post_create_5644f0(actor, value)?;
    }
    Ok(())
}
/// Native ground-component PostCreate vtable methods. Property source lists are
/// immutable loaded definitions; handles remain ordered and null entries fail
/// at the same required insertion boundary instead of being silently dropped.
pub trait GroundPostCreate {
    fn initial_properties(&self, actor: ActorId) -> Result<Vec<Option<Object>>>;
    ///5817A0 with kind0 and sourceNone, followed by actual581630 insertion.
    fn add_initial_property(&mut self, actor: ActorId, property: Option<Object>) -> Result<()>;
}
pub fn ground_post_create(
    world: &mut impl GroundPostCreate,
    actor: ActorId,
    slot: ComponentSlot,
) -> Result<()> {
    match slot {
        ComponentSlot::Properties => {
            for property in world.initial_properties(actor)? {
                world.add_initial_property(actor, property)?;
            }
        }
        ComponentSlot::Body
        | ComponentSlot::Character
        | ComponentSlot::Mover
        | ComponentSlot::Mind
        | ComponentSlot::Performer
        | ComponentSlot::Organization
        | ComponentSlot::Element
        | ComponentSlot::Economy
        | ComponentSlot::Experience
        | ComponentSlot::Detection => {}
        _ => return Err(Error::Unsupported("component-specific PostCreate")),
    }
    Ok(())
}
