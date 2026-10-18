//!Fresh56F280 over the SAME constructed Economy, source binding, Owners,
//!Queues and world RNG. The saved and fresh adapters share one native kernel.
//!Queue links own Economy+C; the constructor's head diagnostic is not a mirror.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    catalog::Catalog,
    definitions::Bank,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot as Slot,
};
use aa_numeric::Precision;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn checked(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    key: ObjectKey,
) -> Result<(ActorId, Precision)> {
    let (owner, pc) =
        crate::stored_component_init_source::checked(catalog, actors, bank, key, Slot::Economy)?;
    match actors.component(owner, Slot::Economy).map_err(error)? {
        Some(RuntimeComponent::Economy(e)) if e.header.owner == owner => {}
        _ => return Err(error("fresh56F280 actual Economy allocation/owner differs")),
    }
    let leaf = aa_actors::component_virtuals::leaves(
        aa_actors::init_create::FactoryKind::Standard(Slot::Economy),
    );
    if leaf.init_existing != 0x56f280 || leaf.vf14 != 0x4b2830 {
        return Err(error(
            "fresh Economy actual vtable InitExisting/AfterInit differs",
        ));
    }
    Ok((owner, pc))
}
struct Stored<'a> {
    world: crate::stored_component_schedule::World<'a>,
    owner: ActorId,
    source_pc: Precision,
}
impl crate::load_economy_init_world::World for Stored<'_> {
    fn scheduled(&mut self) -> Result<bool> {
        self.world.pending()
    }
    fn supply_required(&mut self) -> Result<bool> {
        //529C30 obtains the current component's actual ActorData. The exact cached
        //source allocation constructed this same fresh Actor; no live PC rebind.
        if self.world.staged.actors.id(self.world.key) != Some(self.owner) {
            return Err(error("56F280 current Actor generation changed"));
        }
        let binding = self
            .world
            .staged
            .actors
            .entity(self.world.key)
            .map_err(error)?
            .bindings
            .as_ref()
            .ok_or_else(|| error("56F280 actual constructor's source binding is absent"))?;
        if binding.precision != self.source_pc {
            return Err(error("56F280 immutable ActorData source precision changed"));
        }
        binding
            .source_ground
            .economy
            .as_ref()
            .ok_or_else(|| error("56F280 actual ActorData348 supply producer absent"))?
            .supply_required
            .boolean()
            .map_err(error)
    }
    fn add_periodic(&mut self) -> Result<()> {
        self.world.add_periodic(1)
    }
}
pub fn init_existing(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    pc: Precision,
) -> Result<bool> {
    let (owner, source_pc) = checked(catalog, &staged.actors, &staged.definitions, key)?;
    crate::load_economy_init_world::initialize(&mut Stored {
        world: crate::stored_component_schedule::World::new(staged, key, Slot::Economy, pc)?,
        owner,
        source_pc,
    })
}
///Native4B2830 is RET; preserve actual source/factory/owner admission.
pub fn after_init(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    bank: &Bank,
    key: ObjectKey,
) -> Result<()> {
    checked(catalog, actors, bank, key)?;
    Ok(())
}
