//! Actor64 (4EC010) over actual disjoint Object/Actor fields. The current
//! stored Zone remains borrowed by its caller; memory owns its original arena.
use crate::{
    catalog::Catalog,
    live_object_init::{MemoryServices, Source},
    load_detection_memory::Fields,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
};
use aa_mapworld::restore::RestoredRecord;
#[cfg(test)]
#[path = "load_actor_reveal_tests.rs"]
mod tests;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

fn kingdom(catalog: &Catalog<'_>, fields: &Fields<'_>, id: ActorId) -> Result<Option<u32>> {
    if id.is_null() {
        return Err(Error::NullDereference("54EC70/54ECB0 Kingdom+190"));
    }
    let entry = fields
        .registry
        .get(id)
        .ok_or_else(|| error("Actor64 Kingdom generation is not current"))?;
    let RestoredRecord::Kingdom(k) = &entry.value else {
        return Err(error("Actor64 target is not Kingdom"));
    };
    let runtime = k
        .runtime
        .as_ref()
        .ok_or_else(|| error("Actor64 Kingdom runtime absent"))?;
    Source::bind(catalog, &runtime.definition)?;
    if runtime.definition.native_class != 4
        || runtime.base.object.is_none()
        || runtime.base.actor.is_none()
        || runtime.base.kind != aa_actors::loading::LoadKind::Actor
        || !crate::load_spatial::same_proof(
            &runtime.base.thing.definition,
            &runtime.definition.proof,
        )
    {
        return Err(error("Actor64 Kingdom constructor/source storage differs"));
    }
    let derived = runtime
        .derived
        .as_ref()
        .ok_or_else(|| error("Actor64 Kingdom derived storage absent"))?;
    let word = |offset| {
        derived
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("Actor64 Kingdom+{offset:X} uninitialized")))
    };
    //54EC70/54ECB0 never read the index for a nonmajor Kingdom.
    if word(0x190)? != 0 {
        return Ok(None);
    }
    Ok(Some(word(0x18c)?))
}

pub(crate) trait World {
    fn shown(&mut self) -> Result<bool>;
    fn seen(&mut self) -> Result<bool>;
    fn show(&mut self) -> Result<()>;
    fn explore(&mut self) -> Result<()>;
    fn actor_b4(&mut self) -> Result<bool>;
    fn container(&mut self) -> Result<bool>;
    fn reveal_container(&mut self) -> Result<()>;
    fn memory(&mut self) -> Result<()>;
    fn announce(&mut self) -> Result<u8>;
    fn first_sighting(&mut self) -> Result<()>;
}
pub(crate) fn reveal_inner(world: &mut impl World, explored: bool, visible: bool) -> Result<()> {
    let update = if visible && !world.shown()? {
        true
    } else if !explored {
        false
    } else {
        !world.seen()?
    };
    if !update {
        return Ok(());
    }
    world.shown()?;
    let was_seen = world.seen()?;
    if visible {
        world.show()?;
    }
    if explored {
        world.explore()?;
        if world.actor_b4()? && world.container()? {
            world.reveal_container()?;
        }
    }
    world.memory()?;
    if world.announce()? != 0 && explored && !was_seen {
        world.first_sighting()?;
    }
    Ok(())
}
struct Owned<'a, 'fields, 'assets, S> {
    catalog: &'a Catalog<'assets>,
    fields: &'a mut Fields<'fields>,
    services: &'a mut S,
    kingdoms: &'a crate::load_kingdom_init_world::KingdomRegistryState,
    target: ActorId,
}
impl<S: MemoryServices> World for Owned<'_, '_, '_, S> {
    fn shown(&mut self) -> Result<bool> {
        Ok(
            kingdom(self.catalog, self.fields, self.target)?.is_none_or(|i| {
                self.fields.object.words_50_to_54[1] & 1u32.wrapping_shl(i.wrapping_add(16) & 31)
                    != 0
            }),
        )
    }
    fn seen(&mut self) -> Result<bool> {
        Ok(kingdom(self.catalog, self.fields, self.target)?
            .is_none_or(|i| self.fields.object.words_50_to_54[1] & 1u32.wrapping_shl(i & 31) != 0))
    }
    fn show(&mut self) -> Result<()> {
        if let Some(i) = kingdom(self.catalog, self.fields, self.target)? {
            self.fields.object.words_50_to_54[1] |= 1u32.wrapping_shl(i.wrapping_add(16) & 31);
        }
        Ok(())
    }
    fn explore(&mut self) -> Result<()> {
        if let Some(i) = kingdom(self.catalog, self.fields, self.target)? {
            self.fields.object.words_50_to_54[1] |= 1u32.wrapping_shl(i & 31);
        }
        Ok(())
    }
    fn actor_b4(&mut self) -> Result<bool> {
        Ok(matches!(self.fields.definition.native_class, 3 | 4))
    }
    fn container(&mut self) -> Result<bool> {
        Ok(self.fields.actor_container()?.is_some())
    }
    fn reveal_container(&mut self) -> Result<()> {
        Err(Error::Unsupported(
            "4EC010 current container recursive Actor64 owner",
        ))
    }
    fn memory(&mut self) -> Result<()> {
        let source = Source::bind(self.catalog, self.fields.definition)?;
        crate::load_detection_memory::refresh_fields(
            self.catalog,
            self.fields,
            &source,
            self.services,
            self.kingdoms,
        )
    }
    fn announce(&mut self) -> Result<u8> {
        self.services
            .definition_announce_byte(&self.fields.memory_record())
    }
    fn first_sighting(&mut self) -> Result<()> {
        Err(Error::Unsupported(
            "4EC010 actual first-sighting message/SAI owner",
        ))
    }
}
pub(crate) fn reveal(
    catalog: &Catalog<'_>,
    fields: &mut Fields<'_>,
    services: &mut impl MemoryServices,
    kingdoms: &crate::load_kingdom_init_world::KingdomRegistryState,
    target: ActorId,
    explored: bool,
    visible: bool,
) -> Result<()> {
    Source::bind(catalog, fields.definition)?.validate_memory_record(&fields.memory_record())?;
    reveal_inner(
        &mut Owned {
            catalog,
            fields,
            services,
            kingdoms,
            target,
        },
        explored,
        visible,
    )
}

pub(crate) trait Services {
    fn reveal(
        &mut self,
        catalog: &Catalog<'_>,
        fields: &mut Fields<'_>,
        kingdom: u32,
        explored: bool,
        visible: bool,
    ) -> Result<()>;
}
