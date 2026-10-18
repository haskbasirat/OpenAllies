//!Shared4D7580 coordinator. Each service acts on its actual retained storage;
//!no temporary ActorStore or class-changing shell conversion is permitted.
use aa_actors::components::{ComponentSlot,ALL_SLOTS};
pub(crate) type Result<T>=std::result::Result<T,String>;
pub(crate) trait World {
    fn or_flags(&mut self,bits:u32)->Result<()>;
    fn timed_list(&mut self)->Result<()>;
    fn allocation_144(&mut self)->Result<()>;
    fn observer_transition(&mut self)->Result<()>;
    fn set_category_140(&mut self)->Result<()>;
    fn companion_158(&mut self)->Result<()>;
    fn zones(&mut self)->Result<()>;
    fn state_bit25(&mut self)->Result<bool>;
    fn linked_160(&mut self)->Result<()>;
    fn linked_164(&mut self)->Result<()>;
    fn mind(&mut self)->Result<()>;
    fn element(&mut self)->Result<()>;
    fn owner(&mut self)->Result<()>;
    fn linked_168(&mut self)->Result<()>;
    fn structure(&mut self)->Result<()>;
    fn component(&mut self,slot:ComponentSlot)->Result<()>;
    fn base_object(&mut self)->Result<()>;
}
pub(crate) fn run(world:&mut impl World)->Result<()> {
    world.or_flags(1)?;
    world.timed_list()?;
    world.allocation_144()?;
    world.observer_transition()?;
    world.set_category_140()?;
    world.companion_158()?;
    world.zones()?;
    if world.state_bit25()? {world.or_flags(2)?;}
    world.linked_160()?;
    world.linked_164()?;
    world.mind()?;
    world.element()?;
    world.owner()?;
    world.linked_168()?;
    world.structure()?;
    for slot in std::iter::once(ComponentSlot::Mind).chain(ALL_SLOTS.into_iter().rev()) {
        world.component(slot)?;
    }
    world.or_flags(2)?;
    world.base_object()
}
