//!Stored4EB6E0/4D67F0 use the SAME live index, clock and actual typed+110 list.
//!No shell or overlap snapshot is substituted for the mutable ActorStore owner.
use crate::{
    actor_world::{ObjectKey, Spatial},
    catalog::Catalog,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    overlap_list,
};
use aa_numeric::Precision;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required(s: &'static str) -> Error {
    Error::Unsupported(s)
}
fn flags(staged: &Staged, catalog: &Catalog<'_>, key: ObjectKey, pc: Precision) -> Result<u32> {
    let object = staged.actors.object(key).map_err(error)?;
    let source = crate::live_object_init::Source::bind_in_bank(
        catalog,
        &staged.definitions,
        &object.definition,
    )?;
    if !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
        || !crate::load_spatial::ordinary_layout(
            object.definition.native_class,
            object.kind,
            object.object.is_some(),
            object.actor.is_some(),
        )
    {
        return Err(required(
            "stored overlap actual constructor/source/layout differs",
        ));
    }
    let world = staged
        .load_world
        .as_ref()
        .ok_or(required("stored overlap actual World absent"))?;
    if world.terrain.source_precision() != source.source_precision()
        || world.source_fields.source_precision() != source.source_precision()
        || world.source_fields.receipt().sources != catalog.actors.sources
    {
        return Err(required("stored overlap World/source fields differ"));
    }
    let spatial = world.live_spatial().map_err(error)?;
    if spatial.index().precision() != pc {
        return Err(required("stored overlap actual live/index PC differs"));
    }
    let id = staged
        .actors
        .id(key)
        .ok_or(required("stored overlap current generation absent"))?;
    if spatial
        .index()
        .reference_for_key(spatial.key(key).map_err(error)?)
        .map_err(error)?
        != id.raw()
    {
        return Err(required("stored overlap spatial identity differs"));
    }
    Ok(staged.actors.placement(key, spatial).map_err(error)?.flags)
}
pub(crate) fn refresh(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    pc: Precision,
) -> Result<bool> {
    let flags = flags(staged, catalog, key, pc)?;
    let actor = staged
        .actors
        .object(key)
        .map_err(error)?
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("stored4D67F0 Actor"))?;
    if actor.words.contains_key(&0x110) || actor.words.contains_key(&0x114) {
        return Err(required("obsolete raw stored overlap-list authority"));
    }
    if flags & (1 << 29) != 0
        || !overlap_list::eligible(actor.components.state.low, actor.components.state.high)
        || flags & (1 << 10) == 0
    {
        return Ok(false);
    }
    //This is the same NativeClock moved from Attempt to Staged. Its actual+E0
    //word is read only after all native early-return gates above.
    let time = staged.clock.begin.0;
    staged
        .actors
        .object_mut(key)
        .map_err(error)?
        .actor
        .as_mut()
        .unwrap()
        .overlap_list_110
        .expire_at(time)
        .map_err(error)?;
    let found =
        crate::map_actor_placement::overlaps(staged, catalog, key, pc, false).map_err(error)?;
    let list = &mut staged
        .actors
        .object_mut(key)
        .map_err(error)?
        .actor
        .as_mut()
        .ok_or(Error::NullDereference("stored4D67F0 reread Actor"))?
        .overlap_list_110;
    for reference in found {
        list.prepend(overlap_list::Entry {
            reference,
            expiry_bits: 0,
        })
        .map_err(error)?;
    }
    Ok(true)
}
pub(crate) fn notify(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    pc: Precision,
) -> Result<()> {
    if flags(staged, catalog, key, pc)? & (1 << 10) == 0 {
        return Ok(());
    }
    let found =
        crate::map_actor_placement::overlaps(staged, catalog, key, pc, true).map_err(error)?;
    for raw in found {
        let child = staged
            .actors
            .key(ActorId::from_raw(raw))
            .ok_or(Error::NullDereference("stored4EB6E0 discovered generation"))?;
        match staged
            .actors
            .object(child)
            .map_err(error)?
            .definition
            .native_class
        {
            2 | 6 | 7 | 9 => continue,
            3 | 4 => {
                refresh(staged, catalog, child, pc)?;
            }
            _ => return Err(required("stored4EB6E0 actual discovered virtual+B4 class")),
        }
    }
    Ok(())
}
