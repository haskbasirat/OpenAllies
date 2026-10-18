//! Concrete post-record tail over the same retained World/ActorStore/Bank.
//! This assembles checked read-only bindings, then executes the existing native
//! interval4FD3F3..4FD47C. It does not perform conversion/publication or start
//! a battle. Binding refusals are preflight errors, not native unwind parity.
use crate::{
    actor_world::{Source, SpecialSource},
    catalog::Catalog,
    load_tail_source,
    load_tail_world::{self, SessionRead, WorldFacts},
    restoration::Staged,
};
use aa_numeric::{Binary, Precision};
use std::collections::BTreeMap;

pub(crate) fn gmap_extent_words(extents: [i32; 2]) -> Result<[u32; 2], String> {
    Ok([
        Binary::from_i32(extents[0])
            .store_f32()
            .map_err(|e| e.to_string())?,
        Binary::from_i32(extents[1])
            .store_f32()
            .map_err(|e| e.to_string())?,
    ])
}

/// Execute only after Staged::post_load. Every mutable write goes into the
/// actual retained TailState; errors leave those earlier writes observable.
/// Session getters remain live and their opaque handles stay owner checked.
pub fn after_post_load<S: SessionRead + ?Sized>(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    session: &S,
    precision: Precision,
) -> Result<(), String> {
    catalog.require_source_owner()?;
    let source = staged.definitions.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("load tail moved source Bank belongs to another Catalog".into());
    }
    if staged.report.stopped_before.is_some()
        || staged.report.events.iter().any(|e| e.error.is_some())
    {
        return Err("load tail requires completed native record sections".into());
    }
    let world = staged
        .load_world
        .as_mut()
        .ok_or("load tail has no actual retained World")?;
    let actual_pc = match &world.spatial {
        crate::load_world::SpatialOwner::Loading(Some(v)) => v.index().precision(),
        crate::load_world::SpatialOwner::Adopted(v) => v.world.index().precision(),
        _ => return Err("load tail spatial owner is between transfers".into()),
    };
    if actual_pc != precision {
        return Err("load tail arithmetic PC differs from actual World".into());
    }
    let mode = crate::live_placement::read_settings_mode(
        true,
        staged.native_load_mode,
        world.global_settings_mode,
        Some(world.map_mode.as_ref()),
    )
    .map_err(|e| e.to_string())?
    .ok_or("load tail actual World settings alias is null")?;
    let flag = if matches!(mode, 1 | 4) {
        Some(crate::map_resource_mode::world_start_flag(&world.header)?)
    } else {
        None
    };
    //5565E1..556665 ceils the actual world extents and stores those integers
    //as binary32 at GMap+C/+10. Grid width/height are different (shifted) words.
    let extents = world.terrain.bound.layout.extent_i32;
    let map_size = gmap_extent_words(extents)?;
    let mut members = BTreeMap::new();
    for kingdom in world.kingdoms.all().iter().flatten() {
        let key = staged
            .actors
            .key(*kingdom)
            .ok_or("load tail retained Kingdom slot has no current GTID")?;
        let entity = staged.actors.entity(key).map_err(|e| e.to_string())?;
        let Source::Special(s) = &entity.source else {
            return Err("load tail Kingdom slot is not its retained special allocation".into());
        };
        if !matches!(s.source, SpecialSource::Kingdom(_)) {
            return Err("load tail Kingdom slot resolves to another class".into());
        }
        let lists = s
            .derived
            .as_ref()
            .and_then(|v| v.actor_lists.as_ref())
            .ok_or("load tail Kingdom+284 has no constructed list owner")?;
        let list = lists
            .by_class
            .get(3)
            .ok_or("load tail Kingdom class-list3 is unavailable")?;
        // Copy only the read-only traversal snapshot. Membership and order are
        // read from the actual stored list, never reconstructed by an owner scan.
        members.insert(*kingdom, list.clone());
    }
    let mut priorities = BTreeMap::new();
    // Bind immutable source words only. This scan does not construct any
    // membership; the native camera walk still uses the retained lists above.
    for entry in staged.actors.identities().iter() {
        let entity = staged
            .actors
            .entity(entry.value)
            .map_err(|e| e.to_string())?;
        if let Some(object) = entity.runtime.as_ref().filter(|v| v.actor.is_some()) {
            let value = load_tail_source::starting_location_priority_in_bank(
                catalog,
                &object.definition,
                &staged.definitions,
            )?;
            let id = object.definition.proof.id.clone();
            if let Some(old) = priorities.insert(id, value.value()) {
                if old != value.value() {
                    return Err(
                        "one canonical camera definition has inconsistent immutable priority"
                            .into(),
                    );
                }
            }
        }
    }
    let mut tail = load_tail_world::World {
        actors: &mut staged.actors,
        session,
        facts: WorldFacts {
            map_settings_mode: mode,
            world_start_flag: flag,
            world_word_48: staged.inventory.world.word_48,
            map_size_bits: Some(map_size),
            kingdom_slots: Some(world.kingdoms.all()),
            kingdom_actors: &members,
            definition_priorities: &priorities,
        },
        camera: &mut world.tail.camera,
        precision,
        //Fresh mode0 map headers skip this service at the native flag branch.
        //A reached save/metagame world-start needs the real statistics/economy
        //adapter; absence remains an explicit native-position refusal.
        world_start: None,
    };
    tail.after_post_load().map_err(|e| e.to_string())
}

/// Read the camera after the actual owner has been initialized or failed.
pub fn camera(staged: &Staged) -> Result<&load_tail_world::CameraState, String> {
    Ok(&staged
        .load_world
        .as_ref()
        .ok_or("load tail World is absent")?
        .tail
        .camera)
}
