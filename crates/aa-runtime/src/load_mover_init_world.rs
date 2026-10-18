//! Native Mover576700 InitExisting on its actual held saved allocation.
//! No-falling and already-active falling paths complete. A reached scheduler
//! or live state-effect service remains explicit, preserving earlier stores.
use crate::{
    catalog::Catalog,
    live_object_init::{Record, Source},
};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
    ground_states::MoverState,
    loading::LoadShell,
};
use aa_mapworld::restore::{ActorStateContext, RestoredComponent, RestoredRecord};
use aa_numeric::Precision;

fn required(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
fn mover<D>(shell: &LoadShell<D, RestoredComponent>) -> Result<&MoverState> {
    match shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("Mover Init Actor"))?
        .components
        .get(ComponentSlot::Mover)
    {
        Some(RestoredComponent::Mover(v)) => Ok(v),
        Some(_) => Err(required("Mover Init actual typed payload differs")),
        None => Err(Error::NullDereference("Mover Init slot3")),
    }
}
fn checked(catalog: &Catalog<'_>, record: &Record<'_>) -> Result<()> {
    if !record
        .registry
        .get(record.owner)
        .is_some_and(|e| matches!(e.value, RestoredRecord::Loading))
    {
        return Err(required(
            "Mover Init requires actual registered held Load shell",
        ));
    }
    if !matches!(record.definition.native_class, 3 | 4)
        || !crate::load_spatial::ordinary_layout(
            record.definition.native_class,
            record.shell.kind,
            record.shell.object.is_some(),
            record.shell.actor.is_some(),
        )
        || !crate::load_spatial::same_proof(
            &record.shell.thing.definition,
            &record.definition.proof,
        )
    {
        return Err(required(
            "Mover Init actual constructor/source proof differs",
        ));
    }
    let source = Source::bind(catalog, record.definition)?;
    let mask = catalog
        .canonical_component_mask(&record.definition.proof.id, source.source_precision())
        .map_err(required)?;
    if record.definition.component_mask != Some(mask)
        || mask & (1 << ComponentSlot::Mover.index()) == 0
    {
        return Err(required("Mover Init actual source factory mask differs"));
    }
    if mover(record.shell)?.header.owner != record.owner {
        return Err(required("Mover Init actual allocation owner differs"));
    }
    Ok(())
}

fn prefix<D>(
    shell: &mut LoadShell<D, RestoredComponent>,
    world_version: impl FnOnce() -> Result<u32>,
) -> Result<bool> {
    crate::mover_init_fields::initialize(&mut SavedFields{shell,version:Some(world_version)})
}
struct SavedFields<'a,D,F>{shell:&'a mut LoadShell<D,RestoredComponent>,version:Option<F>}
impl<D,F:FnOnce()->Result<u32>> crate::mover_init_fields::Fields for SavedFields<'_,D,F>{
 fn falling(&self)->Result<u8>{
    Ok(*mover(self.shell)?
        .bytes
        .get(&0x56)
        .ok_or_else(|| required("Mover Init actual+56 uninitialized"))?)
 }
 fn version(&mut self)->Result<u32>{self.version.take().ok_or(Error::Unsupported("576700 repeated actual world-version read"))?()}
 fn pending(&mut self)->Result<bool>{Ok(mover(self.shell)?.header.scheduled_head.is_some())}
 fn add_periodic(&mut self)->Result<()>{Err(required(
            "50AC00 Mover Init requires actual queue0 periodic scheduling(data0) before falling state activation",
        ))}
 fn activate_falling(&mut self)->Result<()>{
    //Use the existing actual saved state owner, including partial low-bit
    //commit before a reached4D4A90 effect. No raw Actor+100 mirror is used.
    let actor = self.shell
        .actor
        .as_mut()
        .ok_or(Error::NullDereference("Mover Init Actor state"))?;
    let owner = match actor.components.get(ComponentSlot::Mover) {
        Some(RestoredComponent::Mover(v)) => v.header.owner,
        _ => return Err(required("Mover Init actual allocation disappeared")),
    };
    crate::saved_actor_state::set(
        ActorStateContext {
            owner,
            owner_actor: actor,
            owner_thing_flags: &mut self.shell.thing.flags,
        },
        24,
        true,
    )
    .map_err(required)?;
    Ok(())
 }
}

/// The world-version getter must borrow the actual current World+2C word.
/// It is never invoked when the actual Mover+56 byte is zero.
pub fn init_existing(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    _pc: Precision,
    world_version: impl FnOnce() -> Result<u32>,
) -> Result<bool> {
    checked(catalog, record)?;
    prefix(record.shell, world_version)
}
/// Native Mover vtable+14=4B2830: return, with no additional mutable effects.
pub fn after_init(catalog: &Catalog<'_>, record: &mut Record<'_>) -> Result<()> {
    checked(catalog, record)?;
    Ok(())
}

#[cfg(test)]
#[path = "load_mover_init_world_tests.rs"]
mod tests;
