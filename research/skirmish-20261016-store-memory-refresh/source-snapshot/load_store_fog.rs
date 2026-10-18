//!551F90/551FD0 over the original post-load ActorStore, before placement adoption.
//!Native tile nodes retain their identities; actor flags, detection words and
//!memory arenas are mutated in their sole staged owners. Reached unsupported
//!virtuals preserve preceding tile/actor stores and return an explicit error.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeObject, Source as ActorSource},
    catalog::Catalog,
    live_object_init::{self, Source},
    load_detection_memory,
    load_kingdom_init_world::KingdomRegistryState,
    load_zone_services,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    detection_memory::RecordId,
    loading::LoadKind,
};
use aa_spatial::index::{NodeKey, SpatialIndex};
use aa_zones::tile::ZoneMap;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
#[cfg(test)]
thread_local! {
    static ACTUAL_MEMORY_STOP:std::cell::RefCell<Option<serde_json::Value>>=const {std::cell::RefCell::new(None)};
}

trait World {
    type Node: Copy + Ord;
    fn tile_count(&mut self) -> Result<i32>;
    fn tile_explored(&mut self, tile: i32) -> Result<()>;
    fn tile_visible(&mut self, tile: i32) -> Result<()>;
    fn first(&mut self, tile: i32) -> Result<Option<Self::Node>>;
    fn next(&mut self, node: Self::Node) -> Result<Option<Self::Node>>;
    fn actor(&mut self, node: Self::Node) -> Result<ActorId>;
    fn flags(&mut self, actor: ActorId) -> Result<u32>;
    fn set_flags(&mut self, actor: ActorId, flags: u32) -> Result<()>;
    fn is_object(&mut self, actor: ActorId) -> Result<bool>;
    fn associated_current(&mut self, actor: ActorId) -> Result<bool>;
    fn explore(&mut self, actor: ActorId) -> Result<()>;
    fn show(&mut self, actor: ActorId) -> Result<()>;
}

///54F400 and54F4B0 reread node.next after each callback. A Rust refusal at a
///callback does not run the later native bit30 restoration or next-node read.
fn tile_explore(w: &mut impl World, tile: i32) -> Result<()> {
    w.tile_explored(tile)?;
    let mut node = w.first(tile)?;
    let mut visited = std::collections::BTreeSet::new();
    while let Some(n) = node {
        if !visited.insert(n) {
            return Err(error("54F400 corrupt cyclic native node list"));
        }
        let actor = w.actor(n)?;
        let flags = w.flags(actor)?;
        if flags & (1 << 20) != 0 || flags & (1 << 11) != 0 && flags & (1 << 23) == 0 {
            let temporary_clear = w.is_object(actor)?
                && w.associated_current(actor)?
                && w.flags(actor)? & (1 << 30) != 0;
            if temporary_clear {
                let flags = w.flags(actor)?;
                w.set_flags(actor, flags & !(1 << 30))?;
            }
            w.explore(actor)?;
            if temporary_clear {
                let flags = w.flags(actor)?;
                w.set_flags(actor, flags | (1 << 30))?;
            }
        }
        node = w.next(n)?;
    }
    Ok(())
}
fn tile_show(w: &mut impl World, tile: i32) -> Result<()> {
    w.tile_visible(tile)?;
    let mut node = w.first(tile)?;
    let mut visited = std::collections::BTreeSet::new();
    while let Some(n) = node {
        if !visited.insert(n) {
            return Err(error("54F4B0 corrupt cyclic native node list"));
        }
        let actor = w.actor(n)?;
        w.show(actor)?;
        node = w.next(n)?;
    }
    Ok(())
}
fn all(w: &mut impl World, visible: bool) -> Result<()> {
    let mut tile = 0_i32;
    while tile < w.tile_count()? {
        tile_explore(w, tile)?;
        if visible {
            tile_show(w, tile)?;
        }
        tile = tile.wrapping_add(1);
    }
    Ok(())
}

struct Owned<'a, 'source, 'services> {
    catalog: &'a Catalog<'source>,
    actors: &'a mut ActorStore,
    zones: &'a mut ZoneMap,
    index: &'a SpatialIndex,
    services: &'a load_zone_services::Context<'services>,
    kingdoms: &'a KingdomRegistryState,
}
impl Owned<'_, '_, '_> {
    fn key(&self, actor: ActorId) -> Result<ObjectKey> {
        self.actors
            .key(actor)
            .ok_or_else(|| error("global Fog native node has no current ActorStore generation"))
    }
    fn object(&self, actor: ActorId) -> Result<&RuntimeObject> {
        self.actors.object(self.key(actor)?).map_err(error)
    }
    fn validate_object(&self, actor: ActorId) -> Result<()> {
        let object = self.object(actor)?;
        Source::bind(self.catalog, &object.definition)?;
        let valid = match object.definition.native_class {
            2 | 6 | 7 | 9 => {
                object.kind == LoadKind::Object && object.object.is_some() && object.actor.is_none()
            }
            3 | 4 => {
                object.kind == LoadKind::Actor && object.object.is_some() && object.actor.is_some()
            }
            _ => false,
        };
        if !valid
            || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
        {
            return Err(error(
                "global Fog actual staged constructor/source storage differs",
            ));
        }
        //An adopted actor must use its actual SpatialWorld flag owner instead.
        self.actors
            .staged_placement(self.key(actor)?)
            .map_err(error)?;
        Ok(())
    }
    fn object_word(&self, actor: ActorId, offset: u32) -> Result<u32> {
        let o = self
            .object(actor)?
            .object
            .as_ref()
            .ok_or(Error::NullDereference("global Fog Object"))?;
        match offset {
            0x50 => Ok(o.words_50_to_54[0]),
            0x54 => Ok(o.words_50_to_54[1]),
            _ => Err(Error::Unsupported("global Fog object word")),
        }
    }
    fn kingdom_word(&self, actor: ActorId, offset: u32) -> Result<u32> {
        self.validate_object(actor)?;
        if self.object(actor)?.definition.native_class != 4 {
            return Err(error(
                "4EBC70 staged Kingdom vector entry is not actual Kingdom",
            ));
        }
        let entity = self.actors.entity(self.key(actor)?).map_err(error)?;
        let ActorSource::Special(special) = &entity.source else {
            return Err(error("4EBC70 staged Kingdom has no actual derived owner"));
        };
        if !matches!(
            special.source,
            crate::actor_world::SpecialSource::Kingdom(_)
        ) {
            return Err(error(
                "4EBC70 staged Kingdom derived owner is not a Kingdom allocation",
            ));
        }
        special
            .derived
            .as_ref()
            .and_then(|d| d.words.get(&offset))
            .copied()
            .ok_or_else(|| error(format!("4EBC70 staged Kingdom+{offset:X} is uninitialized")))
    }
    fn memory(&mut self, actor: ActorId) -> Result<()> {
        load_detection_memory::refresh_inner(&mut Memory { world: self, actor })
    }
    fn set_actor_word(&mut self, actor: ActorId, visible: bool) -> Result<()> {
        self.validate_object(actor)?;
        let key = self.key(actor)?;
        let object = self
            .actors
            .object_mut(key)
            .map_err(error)?
            .object
            .as_mut()
            .ok_or(Error::NullDereference("global Fog actual Object"))?;
        let word = &mut object.words_50_to_54[1];
        *word = if visible {
            (*word & 0xffff) | 0xffff0000
        } else {
            (*word & 0xffff0000) | 0xffff
        };
        self.memory(actor)
    }
}
impl World for Owned<'_, '_, '_> {
    type Node = NodeKey;
    fn tile_count(&mut self) -> Result<i32> {
        Ok(self
            .zones
            .geometry
            .width
            .wrapping_mul(self.zones.geometry.height))
    }
    fn tile_explored(&mut self, tile: i32) -> Result<()> {
        let tile = self
            .zones
            .tiles
            .get_mut(usize::try_from(tile).map_err(error)?)
            .ok_or(Error::Unsupported("54F400 actual tile absent"))?;
        tile.fog = (tile.fog & 0xffff0000) | 0xffff;
        Ok(())
    }
    fn tile_visible(&mut self, tile: i32) -> Result<()> {
        let tile = self
            .zones
            .tiles
            .get_mut(usize::try_from(tile).map_err(error)?)
            .ok_or(Error::Unsupported("54F4B0 actual tile absent"))?;
        tile.fog = (tile.fog & 0xffff) | 0xffff0000;
        Ok(())
    }
    fn first(&mut self, tile: i32) -> Result<Option<NodeKey>> {
        let width = self.zones.geometry.width;
        if width <= 0 {
            return Err(error("global Fog native tile width nonpositive"));
        }
        Ok(self
            .index
            .tile([tile % width, tile / width])
            .map_err(error)?
            .first_node)
    }
    fn next(&mut self, node: NodeKey) -> Result<Option<NodeKey>> {
        Ok(self.index.node(node).map_err(error)?.next)
    }
    fn actor(&mut self, node: NodeKey) -> Result<ActorId> {
        let actor = self.index.node(node).map_err(error)?.actor;
        Ok(ActorId::from_raw(
            self.index.reference_for_key(actor).map_err(error)?,
        ))
    }
    fn flags(&mut self, actor: ActorId) -> Result<u32> {
        Ok(self
            .actors
            .staged_placement(self.key(actor)?)
            .map_err(error)?
            .flags)
    }
    fn set_flags(&mut self, actor: ActorId, flags: u32) -> Result<()> {
        let key = self.key(actor)?;
        self.actors.staged_placement_mut(key).map_err(error)?.flags = flags;
        Ok(())
    }
    fn is_object(&mut self, actor: ActorId) -> Result<bool> {
        self.validate_object(actor)?;
        //These source-proven constructor classes all inherit TRUE6A71D0+B0.
        Ok(true)
    }
    fn associated_current(&mut self, actor: ActorId) -> Result<bool> {
        Ok(self
            .actors
            .key(ActorId::from_raw(self.object_word(actor, 0x50)?))
            .is_some())
    }
    fn explore(&mut self, actor: ActorId) -> Result<()> {
        self.set_actor_word(actor, false)
    }
    fn show(&mut self, actor: ActorId) -> Result<()> {
        self.set_actor_word(actor, true)
    }
}

struct Memory<'a, 'world, 'source, 'services> {
    world: &'a mut Owned<'world, 'source, 'services>,
    actor: ActorId,
}
impl Memory<'_, '_, '_, '_> {
    fn state(&self) -> Result<&aa_actors::detection_memory::State> {
        Ok(&self
            .world
            .object(self.actor)?
            .object
            .as_ref()
            .ok_or(Error::NullDereference("4EBC70 staged Object"))?
            .memory)
    }
    fn state_mut(&mut self) -> Result<&mut aa_actors::detection_memory::State> {
        let key = self.world.key(self.actor)?;
        Ok(&mut self
            .world
            .actors
            .object_mut(key)
            .map_err(error)?
            .object
            .as_mut()
            .ok_or(Error::NullDereference("4EBC70 staged Object"))?
            .memory)
    }
    fn current_record(&mut self, kingdom: ActorId) -> Result<Option<RecordId>> {
        if load_detection_memory::World::nonmajor(self, kingdom)? != 0
            || !load_detection_memory::World::keeps(self)?
        {
            return Ok(None);
        }
        let index = load_detection_memory::World::kingdom_index(self, kingdom)?;
        self.state()?.slot(index).map_err(error)
    }
}
impl live_object_init::MemoryFields for Memory<'_, '_, '_, '_> {
    fn phase(&mut self) -> Result<u32> {
        self.world.services.native_memory_phase()
    }
    fn mask(&mut self) -> Result<u32> {
        let definition = &self.world.object(self.actor)?.definition;
        let source = Source::bind(self.world.catalog, definition)?;
        let actual = self
            .world
            .catalog
            .canonical_component_mask(&definition.proof.id, source.source_precision())
            .map_err(error)?;
        if definition.component_mask != Some(actual) {
            return Err(error(
                "staged memory ActorData+174 differs from actual source owner",
            ));
        }
        Ok(actual)
    }
    fn flags(&mut self) -> Result<u32> {
        self.world.flags(self.actor)
    }
    fn announce(&mut self) -> Result<u8> {
        self.world.services.native_announce_byte(
            self.world.catalog,
            &self.world.object(self.actor)?.definition,
        )
    }
}
impl load_detection_memory::World for Memory<'_, '_, '_, '_> {
    fn keeps(&mut self) -> Result<bool> {
        Ok(self.state()?.count() > 0)
    }
    fn eligible(&mut self) -> Result<bool> {
        let class = self.world.object(self.actor)?.definition.native_class;
        live_object_init::memory_eligibility(class, self)
    }
    fn kingdom_count(&mut self) -> Result<i32> {
        i32::try_from(self.world.kingdoms.majors().len()).map_err(error)
    }
    fn kingdom_slot(&mut self, index: i32) -> Result<Option<ActorId>> {
        self.world
            .kingdoms
            .majors()
            .get(usize::try_from(index).map_err(error)?)
            .copied()
            .ok_or_else(|| error("4EBC70 staged Kingdom vector index absent"))
    }
    fn nonmajor(&mut self, kingdom: ActorId) -> Result<u32> {
        self.world.kingdom_word(kingdom, 0x190)
    }
    fn kingdom_index(&mut self, kingdom: ActorId) -> Result<u32> {
        self.world.kingdom_word(kingdom, 0x18c)
    }
    fn slot(&mut self, index: u32) -> Result<Option<RecordId>> {
        self.state()?.slot(index).map_err(error)
    }
    fn shared_visible(&mut self, kingdom: ActorId) -> Result<bool> {
        Ok(self.world.kingdom_word(kingdom, 0x190)? != 0
            || self.world.kingdom_word(kingdom, 0x228)?
                & self.world.object_word(self.actor, 0x54)?
                & 0xffff0000
                != 0)
    }
    fn shared_explored(&mut self, kingdom: ActorId) -> Result<bool> {
        Ok(self.world.kingdom_word(kingdom, 0x190)? != 0
            || self.world.kingdom_word(kingdom, 0x228)?
                & self.world.object_word(self.actor, 0x54)?
                & 0xffff
                != 0)
    }
    fn flags(&mut self) -> Result<u32> {
        self.world.flags(self.actor)
    }
    fn create(&mut self, kingdom: ActorId) -> Result<()> {
        use load_detection_memory::World;
        if self.nonmajor(kingdom)? != 0 {
            return Ok(());
        }
        if World::flags(self)? & (1 << 7) != 0 {
            return Err(Error::Unsupported(
                "4ECA80 dead staged Object potential-visible owner",
            ));
        }
        if !self.eligible()? {
            return Ok(());
        }
        if self.keeps()? {
            let index = self.kingdom_index(kingdom)?;
            if self.slot(index)?.is_some() {
                return Ok(());
            }
        }
        if !self.keeps()? {
            let count = self.kingdom_count()?;
            self.state_mut()?.allocate_slots(count).map_err(error)?;
        }
        let record = self.state_mut()?.allocate_record().map_err(error)?;
        let refreshed = crate::load_store_memory_refresh::refresh(
            self.world.catalog,
            self.world.actors,
            self.actor,
            record,
        );
        #[cfg(test)]
        {
            let state = self.state()?;
            let value = state.record(record).map_err(error)?;
            ACTUAL_MEMORY_STOP.with(|capture|*capture.borrow_mut()=Some(serde_json::json!({
                "actor":self.actor.raw(),"class":self.world.object(self.actor).unwrap().definition.native_class,
                "definition":self.world.object(self.actor).unwrap().definition.proof,
                "kingdom":kingdom.raw(),"count":state.count(),"allocation_present":state.allocation_present(),
                "slots":(0..state.count()).map(|i|state.slot(i as u32).unwrap().map(|id|id.raw())).collect::<Vec<_>>(),
                "records":state.allocated_records(),"record_id":record.raw(),"timestamp_bits":value.timestamp_bits,
                "name":value.name_utf16,"position":value.position_bits,"record_definition":value.definition,
                "record_owner":value.owner.raw(),"aspect":value.aspect,"stat_words":value.stat_words,
                "byte30":value.byte_30,"record_flags":value.flags
            })));
        }
        refreshed?;
        let index = self.kingdom_index(kingdom)?;
        self.state_mut()?
            .set_slot(index, Some(record))
            .map_err(error)?;
        if self.world.catalog.saved_observer_active().map_err(error)? {
            return Err(Error::Unsupported(
                "4ECB73 active observer79E7B0 staged memory creation",
            ));
        }
        Ok(())
    }
    fn delete(&mut self, kingdom: ActorId) -> Result<()> {
        let object = self.world.object(self.actor)?;
        if matches!(object.definition.native_class, 3 | 4) {
            let actor = object
                .actor
                .as_ref()
                .ok_or(Error::NullDereference("4CE8C0 staged Actor"))?;
            if load_detection_memory::actor_container(self.actor, &actor.components)?.is_some()
                && self.current_record(kingdom)?.is_some()
            {
                let record = self
                    .current_record(kingdom)?
                    .ok_or(Error::NullDereference("4CE8C0 staged reread memory record"))?;
                let owner = self
                    .world
                    .object(self.actor)?
                    .actor
                    .as_ref()
                    .and_then(|a| a.words.get(&0xe8))
                    .copied()
                    .ok_or(Error::Unsupported("4CE8C0 staged Actor+E8 owning Kingdom"))?;
                if self.state()?.record(record).map_err(error)?.owner.raw() != owner {
                    return Err(Error::Unsupported(
                        "4CE905 current staged container Settlement585FF0 memory notification",
                    ));
                }
            }
        }
        if self.nonmajor(kingdom)? != 0 || !self.keeps()? {
            return Ok(());
        }
        let index = self.kingdom_index(kingdom)?;
        let Some(record) = self.slot(index)? else {
            return Ok(());
        };
        if self.world.catalog.saved_observer_active().map_err(error)? {
            return Err(Error::Unsupported(
                "4EB970 active observer79DEB0 staged memory deletion",
            ));
        }
        self.state_mut()?.free_record(record).map_err(error)?;
        self.state_mut()?.set_slot(index, None).map_err(error)
    }
    fn count(&mut self) -> Result<i32> {
        Ok(self.state()?.count())
    }
    fn clear(&mut self) -> Result<()> {
        if !self.keeps()? {
            return Ok(());
        }
        let mut i = 0;
        while i < self.count()? {
            if let Some(record) = self.slot(i as u32)? {
                if self.world.catalog.saved_observer_present().map_err(error)? {
                    return Err(Error::Unsupported(
                        "4EBB80 present observer79DEB0 staged memory clear",
                    ));
                }
                self.state_mut()?.free_record(record).map_err(error)?;
                self.state_mut()?.set_slot(i as u32, None).map_err(error)?;
            }
            i += 1;
        }
        if self.count()? > 0 {
            self.state_mut()?.release_slots().map_err(error)?;
        }
        Ok(())
    }
}

/// Actual mission-boundary551F90 (visible=false) or551FD0 (visible=true).
/// Caller supplies this map's paired native index, preserved through staging.
/// After spatial adoption this API refuses the actor's unavailable staged flags.
pub(crate) fn reveal_all(
    catalog: &Catalog<'_>,
    actors: &mut ActorStore,
    zones: &mut ZoneMap,
    globals: &crate::load_zone_globals::State,
    index: &SpatialIndex,
    services: &load_zone_services::Context<'_>,
    kingdoms: &KingdomRegistryState,
    visible: bool,
) -> Result<()> {
    globals.validate_map(zones).map_err(error)?;
    let grid = index.grid();
    if grid.dimensions != [zones.geometry.width, zones.geometry.height]
        || grid.coordinate_shift != zones.geometry.shift
        || grid.cell_size_bits != zones.geometry.tile_size_bits
    {
        return Err(error("global Fog actual paired map/index geometry differs"));
    }
    all(
        &mut Owned {
            catalog,
            actors,
            zones,
            index,
            services,
            kingdoms,
        },
        visible,
    )
}

#[cfg(test)]
#[path = "load_store_fog_tests.rs"]
mod tests;
