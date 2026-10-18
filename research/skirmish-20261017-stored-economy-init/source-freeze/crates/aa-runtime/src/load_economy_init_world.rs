//!56F280 Economy InitExisting over its held allocation and real load queues.
//!Scheduling uses the same embedded Updatable identity, world RNG and queue1.
//!This entry does not refresh Economy values/Kingdom flows (570020).
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
};
use aa_mapworld::{restore::RestoredComponent, restore_components::UpdatableTarget};
use aa_numeric::Precision;
use aa_simulation::NativeRng;

#[cfg(test)]
#[path = "load_economy_init_world_tests.rs"]
mod tests;

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}

fn checked(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<Precision> {
    let pc = crate::load_actor_init_world::validate(catalog, record)?;
    if record
        .definition
        .component_mask
        .is_none_or(|mask| mask & (1 << ComponentSlot::Economy.index()) == 0)
    {
        return Err(error(
            "56F280 Economy allocation is outside canonical source factory",
        ));
    }
    let economy = record
        .shell
        .actor
        .as_ref()
        .and_then(|a| a.components.get(ComponentSlot::Economy));
    match economy {
        Some(RestoredComponent::Economy(e)) if e.header.owner == record.owner => Ok(pc),
        Some(_) => Err(error("56F280 held Economy class/owner differs")),
        None => Err(Error::NullDereference("56F280 actual Economy allocation")),
    }
}

pub(crate) trait World {
    fn scheduled(&mut self) -> Result<bool>;
    fn supply_required(&mut self) -> Result<bool>;
    fn add_periodic(&mut self) -> Result<()>;
}
pub(crate) fn initialize(world: &mut impl World) -> Result<bool> {
    if !world.scheduled()? && world.supply_required()? {
        world.add_periodic()?;
    }
    Ok(true)
}

struct Owned<'a, 'record, 'assets> {
    catalog: &'a Catalog<'assets>,
    record: &'a Record<'record>,
    rng: &'a mut NativeRng,
    owner: aa_world::queues::Owner,
    source_pc: Precision,
    precision: Precision,
}
impl World for Owned<'_, '_, '_> {
    fn scheduled(&mut self) -> Result<bool> {
        let queues = self.catalog.load_queues.try_borrow().map_err(error)?;
        let queues = queues
            .as_deref()
            .ok_or(Error::Unsupported("56F283 actual load queue owner"))?;
        if queues.precision != self.precision {
            return Err(error("56F280 current queue/runtime precision differs"));
        }
        //The persistent Queues intrusive chain is authoritative Updatable+4
        //(Economy+C); the saved constructor head projection is not reread.
        Ok(queues.owner_head(self.owner).is_some())
    }
    fn supply_required(&mut self) -> Result<bool> {
        let source = self
            .catalog
            .actors
            .native_economy_inputs(&self.record.definition.proof.id, self.source_pc)
            .map_err(error)?;
        source
            .resources
            .as_ref()
            .ok_or(Error::Unsupported("56F28F actual EconomyData348 producer"))?
            .supply_required
            .boolean()
            .map_err(error)
    }
    fn add_periodic(&mut self) -> Result<()> {
        let mut queues = self.catalog.load_queues.try_borrow_mut().map_err(error)?;
        let queues = queues
            .as_deref_mut()
            .ok_or(Error::Unsupported("50AC60 actual load queue1 owner"))?;
        if queues.precision != self.precision {
            return Err(error("50AC60 current queue/runtime precision differs"));
        }
        //Actual embedded Economy Updatable+8 is6A71D0(AL=true).50AC60 then
        //uses queue1/data0; the shared kernel consumes the same one RNG draw.
        queues
            .add_periodic(1, self.owner, 0, self.rng)
            .map_err(error)?;
        Ok(())
    }
}

pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    rng: &mut NativeRng,
    precision: Precision,
) -> Result<bool> {
    let source_pc = checked(catalog, record)?;
    let owner = catalog
        .load_owners
        .try_borrow_mut()
        .map_err(error)?
        .restoring(UpdatableTarget::Component {
            owner: record.owner.raw(),
            slot: ComponentSlot::Economy.index() as u8,
        })
        .map_err(error)?;
    initialize(&mut Owned {
        catalog,
        record,
        rng,
        owner,
        source_pc,
        precision,
    })
}

/// Actual Economy AfterInit vtable entry4B2830 is a single RET.
pub fn after_init(catalog: &Catalog<'_>, record: &mut Record<'_>) -> Result<()> {
    checked(catalog, record)?;
    Ok(())
}
