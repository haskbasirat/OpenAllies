//!Native4EA150's actual adopted-map occupancy and ordered live zone reads.
//!Positive Actor64 calls borrow the same stored memory/reveal owners.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Source as ActorSource},
    catalog::Catalog,
    definitions::Bank,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot,
};
use aa_numeric::Precision;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required(s: &'static str) -> Error {
    Error::Unsupported(s)
}
fn current<'a>(
    actors: &'a ActorStore,
    bank: &Bank,
    catalog: &Catalog<'_>,
    id: ActorId,
) -> Result<&'a crate::actor_world::RuntimeObject> {
    let key = actors
        .key(id)
        .ok_or(required("4EA150 zone Actor generation stale"))?;
    let object = actors.object(key).map_err(error)?;
    if !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
        || !crate::load_spatial::ordinary_layout(
            object.definition.native_class,
            object.kind,
            object.object.is_some(),
            object.actor.is_some(),
        )
    {
        return Err(required(
            "4EA150 actual zone allocation/source/layout differs",
        ));
    }
    crate::live_object_init::Source::bind_in_bank(
        catalog,
        bank,
        &object.definition,
    )?;
    Ok(object)
}
fn kingdom_word(actors: &ActorStore, bank: &Bank, catalog: &Catalog<'_>, id: ActorId, offset: u32) -> Result<u32> {
    if current(actors, bank, catalog, id)?.definition.native_class != 4 {
        return Err(required("4EA150 zone owner is not actual Kingdom"));
    }
    let key = actors
        .key(id)
        .ok_or(required("4EA150 Kingdom generation stale"))?;
    let ActorSource::Special(s) = &actors.entity(key).map_err(error)?.source else {
        return Err(required("4EA150 Kingdom derived owner absent"));
    };
    s.derived
        .as_ref()
        .and_then(|v| v.words.get(&offset))
        .copied()
        .ok_or(required("4EA150 actual Kingdom field absent"))
}
pub(crate) fn refresh(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    pc: Precision,
) -> Result<()> {
    let Staged { actors, definitions, load_world, inactive_load, .. } = staged;
    let id = actors
        .id(key)
        .ok_or(required("4EA150 current Actor identity absent"))?;
    let actual = current(actors, definitions, catalog, id)?;
    let source = crate::live_object_init::Source::bind_in_bank(
        catalog,
        definitions,
        &actual.definition,
    )?;
    let world = load_world
        .as_ref()
        .ok_or(required("4EA150 actual World absent"))?;
    if world.terrain.source_precision() != source.source_precision()
        || world.source_fields.source_precision() != source.source_precision()
        || world.source_fields.receipt().sources != catalog.actors.sources
    {
        return Err(required("4EA150 actual World/source Bank differs"));
    }
    world.point.validate_catalog(catalog).map_err(error)?;
    let spatial = world.live_spatial().map_err(error)?;
    if spatial.index().precision() != pc {
        return Err(required("4EA150 actual index/live PC differs"));
    }
    crate::load_object_init_world::map_geometry_index(
        &world.terrain,
        &world.header,
        spatial.index(),
        &world.zones,
        pc,
    )?;
    let pose = actors.placement(key, spatial).map_err(error)?;
    let map = &world.zones.geometry;
    let grid = aa_spatial::collision::Grid {
        dimensions: [map.width, map.height],
        coordinate_shift: map.shift,
        cell_size_bits: map.tile_size_bits,
    };
    let placement = aa_spatial::geometry::Placement {
        shape: source.blocking_shape().clone(),
        position: [pose.position_bits[0], pose.position_bits[1]],
        facing: [pose.facing_bits[0], pose.facing_bits[1]],
    };
    let tiles = aa_spatial::creation_placement::occupancy_tiles(
        &grid,
        map.extent_bits,
        world.point.shape(),
        &placement,
        pc,
    )
    .map_err(error)?;
    for [x, y] in tiles {
        let index = world
            .zones
            .index(x, y)
            .ok_or(required("4EA150 actual occupancy tile outside"))?;
        let Some(set) = world.zones.tile(index).map_err(error)?.set else {
            continue;
        };
        for entry in world.zones.pool.entries(Some(set)).map_err(error)? {
            let actor = ActorId::from_raw(entry.actor);
            if entry.flags & 0x40 != 0 {
                let target = current(actors, definitions, catalog, actor)?;
                let owner = match target.definition.native_class {
                    2 | 6 | 7 | 9 => 0,
                    3 | 4 => *target
                        .actor
                        .as_ref()
                        .and_then(|a| a.words.get(&0xe8))
                        .ok_or(required("4EA150 zone Actor+E8 uninitialized"))?,
                    _ => return Err(required("4EA150 actual owner108 dispatch")),
                };
                if owner != 0
                    && kingdom_word(actors, definitions, catalog, ActorId::from_raw(owner), 0x190)? == 0
                {
                    reveal_owners(catalog, definitions, actors, world, inactive_load.as_ref(),
                        id, ActorId::from_raw(owner), true, true)?;
                }
            }
            if entry.flags & 2 != 0 {
                let target = current(actors, definitions, catalog, actor)?;
                let Some(RuntimeComponent::Detection(detection)) = target
                    .components()
                    .map_err(error)?
                    .get(ComponentSlot::Detection)
                else {
                    return Err(Error::NullDereference(
                        "4EA150 actual zone Detection component",
                    ));
                };
                if detection.header.owner != actor {
                    return Err(required("4EA150 Detection allocation owner differs"));
                }
                let mask = detection.word_10;
                let mut index = 0;
                while index < aa_zones::query::KINGDOM_BITS && mask != 0 {
                    if mask & (1 << index) != 0 {
                        //The current World+154 slot is read before the reached Actor64 call.
                        let target = world
                            .kingdoms
                            .majors()
                            .get(index as usize)
                            .ok_or(required("4EA150 actual Kingdom vector slot absent"))?;
                        let target = match target { Some(target) => *target, None => ActorId::NULL };
                        reveal_owners(catalog, definitions, actors, world, inactive_load.as_ref(),
                            id, target, false, true)?;
                    }
                    index += 1;
                }
            }
        }
    }
    Ok(())
}

struct SessionLoan<'a>(Option<&'a dyn crate::load_world_globals::SessionPhase>);
impl crate::load_world_globals::SessionPhase for SessionLoan<'_> {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        self.0.ok_or("4EA150 memory requires actual Session+F8 owner")?.current_phase()
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        self.0.ok_or("4EA150 memory requires actual Session+E4 owner")?.application_state()
    }
}
#[allow(clippy::too_many_arguments)]
fn reveal_owners(catalog: &Catalog<'_>, bank: &Bank, actors: &mut ActorStore,
    world: &crate::load_world::World, inactive: Option<&crate::restore_lifecycle::InactiveLoad>,
    actor: ActorId, target: ActorId, explored: bool, visible: bool) -> Result<()> {
    let session = SessionLoan(catalog.load_session);
    let services = crate::load_zone_services::Context::with_kingdoms(
        &world.point, &session, &world.kingdoms).with_source_fields(catalog, &world.source_fields);
    let spatial = world.live_spatial().map_err(error)?;
    let mut access = crate::stored_memory_access::StoreAccess {
        actors, placement: crate::load_store_memory_refresh::PlacementRead::Published(spatial),
    };
    crate::stored_actor_reveal::reveal(catalog, bank, &mut access, &services,
        &world.kingdoms, inactive, actor, target, explored, visible)
}
