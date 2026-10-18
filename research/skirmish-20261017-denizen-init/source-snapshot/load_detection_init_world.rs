//! Shared564D80 InitExisting on real Construction/Detection/Lair allocations.
//! Version22+ returns before queue reads; older maps use the actual owned
//! queue1/head and RNG. Detection AfterInit56EC90 needs its actual zone owner.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
};
use aa_mapworld::{restore::RestoredComponent, restore_components::UpdatableTarget};
use aa_numeric::Precision;
use aa_simulation::NativeRng;

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
pub fn supports(slot: ComponentSlot) -> bool {
    matches!(
        slot,
        ComponentSlot::Construction | ComponentSlot::Detection | ComponentSlot::Lair
    )
}
pub(crate) fn checked(catalog: &Catalog<'_>, record: &Record<'_>, slot: ComponentSlot) -> Result<()> {
    if !supports(slot) {
        return Err(Error::Unsupported("564D80 unsupported component class"));
    }
    crate::load_actor_init_world::validate(catalog, record)?;
    if record
        .definition
        .component_mask
        .is_none_or(|mask| mask & (1 << slot.index()) == 0)
    {
        return Err(error(
            "564D80 actual component lies outside canonical source factory",
        ));
    }
    let component = record
        .shell
        .actor
        .as_ref()
        .and_then(|a| a.components.get(slot));
    let owner = match (slot, component) {
        (ComponentSlot::Construction, Some(RestoredComponent::Construction(v))) => v.header.owner,
        (ComponentSlot::Detection, Some(RestoredComponent::Detection(v))) => v.header.owner,
        (ComponentSlot::Lair, Some(RestoredComponent::Lair(v))) => v.header.owner,
        (_, None) => {
            return Err(Error::NullDereference(
                "564D80 actual typed component allocation",
            ));
        }
        _ => return Err(error("564D80 actual typed component class differs")),
    };
    if owner != record.owner {
        return Err(error("564D80 actual component header owner differs"));
    }
    Ok(())
}

pub(crate) trait World {
    fn version(&mut self) -> Result<u32>;
    fn pending(&mut self) -> Result<bool>;
    fn add_periodic(&mut self) -> Result<()>;
}
fn initialize(world: &mut impl World) -> Result<bool> {
    initialize_when(world, || Ok(true))
}
///5687E0 adds its source predicate only after the shared version/head gates.
pub(crate) fn initialize_when(
    world: &mut impl World,
    condition: impl FnOnce() -> Result<bool>,
) -> Result<bool> {
    if world.version()? < 22 && !world.pending()? && condition()? {
        world.add_periodic()?;
    }
    Ok(true)
}
struct Owned<'a, 'assets, F> {
    catalog: &'a Catalog<'assets>,
    target: UpdatableTarget,
    owner: Option<aa_world::queues::Owner>,
    rng: &'a mut NativeRng,
    precision: Precision,
    version: Option<F>,
}
impl<F> Owned<'_, '_, F> {
    fn owner(&mut self) -> Result<aa_world::queues::Owner> {
        if let Some(owner) = self.owner {
            return Ok(owner);
        }
        let owner = self
            .catalog
            .load_owners
            .try_borrow_mut()
            .map_err(error)?
            .restoring(self.target)
            .map_err(error)?;
        self.owner = Some(owner);
        Ok(owner)
    }
}
impl<F: FnOnce() -> Result<u32>> World for Owned<'_, '_, F> {
    fn version(&mut self) -> Result<u32> {
        self.version
            .take()
            .ok_or(Error::Unsupported("564D80 repeated world-version read"))?()
    }
    fn pending(&mut self) -> Result<bool> {
        let owner = self.owner()?;
        let queues = self.catalog.load_queues.try_borrow().map_err(error)?;
        let queues = queues.as_deref().ok_or(Error::Unsupported(
            "564D8B actual load queue/intrusive-head owner",
        ))?;
        if queues.precision != self.precision {
            return Err(error("564D80 queue/runtime precision differs"));
        }
        //Queues owns actual Updatable+C. Constructor/saved header projection
        //does not substitute for this live scheduling owner.
        Ok(queues.owner_head(owner).is_some())
    }
    fn add_periodic(&mut self) -> Result<()> {
        let owner = self.owner()?;
        let mut queues = self.catalog.load_queues.try_borrow_mut().map_err(error)?;
        let queues = queues
            .as_deref_mut()
            .ok_or(Error::Unsupported("50AC60 actual load queue1 owner"))?;
        if queues.precision != self.precision {
            return Err(error("50AC60 queue/runtime precision differs"));
        }
        //The admitted embedded Updatable+8 entries are6A71D0(AL=true).
        queues.add_periodic(1, owner, 0, self.rng).map_err(error)?;
        Ok(())
    }
}
/// The supplied getter borrows the actual retained World+2C word. Queue/head
/// ownership and RNG remain unread when the version predicate skips them.
pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    slot: ComponentSlot,
    rng: &mut NativeRng,
    precision: Precision,
    world_version: impl FnOnce() -> Result<u32>,
) -> Result<bool> {
    checked(catalog, record, slot)?;
    schedule_old_existing(catalog,record,slot,rng,precision,world_version)
}
/// The identical version/head/50AC60 suffix also used by Element5704D0,
/// after that caller has validated its actual allocation and stored its rate.
pub(crate) fn schedule_old_existing(
    catalog:&Catalog<'_>, record:&Record<'_>, slot:ComponentSlot,
    rng:&mut NativeRng, precision:Precision, world_version:impl FnOnce()->Result<u32>,
)->Result<bool> {
    initialize(&mut Owned {
        catalog,
        target: UpdatableTarget::Component {
            owner: record.owner.raw(),
            slot: slot.index() as u8,
        },
        owner: None,
        rng,
        precision,
        version: Some(world_version),
    })
}

///Same real queue/head/RNG suffix with5687E0's additional lazy source test.
pub(crate) fn schedule_old_existing_when(
    catalog:&Catalog<'_>, record:&Record<'_>, slot:ComponentSlot,
    rng:&mut NativeRng, precision:Precision, world_version:impl FnOnce()->Result<u32>,
    condition:impl FnOnce()->Result<bool>,
)->Result<bool> {
    initialize_when(&mut Owned {
        catalog,
        target:UpdatableTarget::Component{owner:record.owner.raw(),slot:slot.index()as u8},
        owner:None,rng,precision,version:Some(world_version),
    },condition)
}

/// Construction/Lair vtable+14 is4B2830 RET. Detection uses56EC90 instead.
pub fn after_constant(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    slot: ComponentSlot,
) -> Result<()> {
    checked(catalog, record, slot)?;
    if !matches!(slot, ComponentSlot::Construction | ComponentSlot::Lair) {
        return Err(Error::Unsupported(
            "56EC90 actual Detection/zone activation services",
        ));
    }
    Ok(())
}

/// Read-only observation of the real held owner at this exact native boundary.
/// Test-only artifact I/O; it neither substitutes a component nor skips effects.
#[cfg(test)]
pub(crate) fn capture_after_entry(catalog: &Catalog<'_>, record: &Record<'_>) {
    let Some(folder) = std::env::var_os("AA_DETECTION_BOUNDARY_CAPTURE_DIR") else {
        return;
    };
    if !matches!(record.owner.raw(), 2696 | 6477) {
        return;
    }
    checked(catalog, record, ComponentSlot::Detection).unwrap();
    let actor = record.shell.actor.as_ref().unwrap();
    let Some(RestoredComponent::Detection(detection)) =
        actor.components.get(ComponentSlot::Detection)
    else {
        unreachable!()
    };
    let pc = crate::live_object_init::Source::bind(catalog, record.definition)
        .unwrap()
        .source_precision();
    let value = serde_json::json!({"scope":"Read-only actual held owner immediately before56EC90, after real preceding component Init/After prefixes. Diagnostic census does not certify earlier unrelated records.",
        "owner":record.owner.raw(),"definition":record.definition.proof,"source_precision":pc,
        "definition_flags":record.definition.flags,"component_mask":record.definition.component_mask,
        "thing_flags":record.shell.thing.flags,"position_bits":record.shell.thing.position_bits,
        "actor_state":actor.components.state,"actor_words":actor.words,"detection":detection,
        "components":actor.components.iter().map(|(s,_)|format!("{s:?}")).collect::<Vec<_>>()});
    let path = std::path::PathBuf::from(folder)
        .join(format!("actual-{}-before-56ec90.json", record.owner.raw()));
    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap(),
        &value,
    )
    .unwrap();
}

#[cfg(test)]
#[path = "load_detection_init_world_tests.rs"]
mod tests;
