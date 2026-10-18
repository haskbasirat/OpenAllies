//!4EBC70 over the actual held Object and live World+154 Kingdom vector.
//!Slot and record allocation have one owner in ObjectShell; required virtual
//!refresh/observer services stop at their native position after earlier stores.
use crate::{
    catalog::Catalog,
    live_object_init::{self, MemoryServices, Record, Source},
    load_kingdom_init_world::KingdomRegistryState,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    detection_memory::RecordId,
};
use aa_mapworld::restore::RestoredRecord;
#[path = "load_detection_memory_fields.rs"]
mod fields;
pub(crate) use fields::ActorFields;
pub use fields::Fields;
#[cfg(test)]
pub(crate) use fields::actor_container;

#[cfg(test)]
#[path = "load_detection_memory_tests.rs"]
mod tests;

#[cfg(test)]
thread_local! {
    static CAPTURE_ENABLED:std::cell::Cell<bool>=const {std::cell::Cell::new(false)};
    static ACTUAL_CAPTURE:std::cell::RefCell<Option<serde_json::Value>>=const {std::cell::RefCell::new(None)};
}

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
trait World {
    fn keeps(&mut self) -> Result<bool>;
    fn eligible(&mut self) -> Result<bool>;
    fn kingdom_count(&mut self) -> Result<i32>;
    fn kingdom_slot(&mut self, index: i32) -> Result<Option<ActorId>>;
    fn nonmajor(&mut self, kingdom: ActorId) -> Result<u32>;
    fn kingdom_index(&mut self, kingdom: ActorId) -> Result<u32>;
    fn slot(&mut self, index: u32) -> Result<Option<RecordId>>;
    fn shared_visible(&mut self, kingdom: ActorId) -> Result<bool>;
    fn shared_explored(&mut self, kingdom: ActorId) -> Result<bool>;
    fn flags(&mut self) -> Result<u32>;
    fn create(&mut self, kingdom: ActorId) -> Result<()>;
    fn delete(&mut self, kingdom: ActorId) -> Result<()>;
    fn count(&mut self) -> Result<i32>;
    fn clear(&mut self) -> Result<()>;
}
fn refresh_inner(world: &mut impl World) -> Result<()> {
    if !world.keeps()? && !world.eligible()? {
        return Ok(());
    }
    let mut deleted = false;
    let mut index = 0;
    //4F0680/4F06B0 skip current null slots; no GTID reconstruction/snapshot.
    while index < world.kingdom_count()? {
        let Some(kingdom) = world.kingdom_slot(index)? else {
            index += 1;
            continue;
        };
        let has = if world.nonmajor(kingdom)? == 0 && world.keeps()? {
            let slot = world.kingdom_index(kingdom)?;
            world.slot(slot)?.is_some()
        } else {
            false
        };
        let remembered = !world.shared_visible(kingdom)? && world.shared_explored(kingdom)?;
        let flags = world.flags()?;
        let alive = flags & 1 == 0 && flags & (1 << 7) == 0;
        if has {
            if !remembered {
                world.delete(kingdom)?;
                deleted = true;
            }
        } else if remembered && alive {
            world.create(kingdom)?;
        }
        index += 1;
    }
    if deleted && world.keeps()? {
        let count = world.count()?;
        for index in 0..count.max(0) {
            if world.slot(index as u32)?.is_some() {
                return Ok(());
            }
        }
        world.clear()?;
    }
    Ok(())
}

struct Owned<'a, 'record, 'assets, S> {
    catalog: &'a Catalog<'assets>,
    record: &'a mut Fields<'record>,
    source: &'a Source,
    services: &'a mut S,
    kingdoms: &'a KingdomRegistryState,
}
impl<S: MemoryServices> Owned<'_, '_, '_, S> {
    fn actor_container(&self) -> Result<Option<ActorId>> {
        self.record.actor_container()
    }
    fn current_record(&mut self, kingdom: ActorId) -> Result<Option<RecordId>> {
        if self.nonmajor(kingdom)? != 0 || !self.keeps()? {
            return Ok(None);
        }
        let index = self.kingdom_index(kingdom)?;
        self.slot(index)
    }
    fn memory(&self) -> Result<&aa_actors::detection_memory::State> {
        Ok(&self.record.object.memory)
    }
    fn memory_mut(&mut self) -> Result<&mut aa_actors::detection_memory::State> {
        Ok(&mut self.record.object.memory)
    }
    fn kingdom_word(&self, kingdom: ActorId, offset: u32) -> Result<u32> {
        if kingdom == self.record.owner {
            return Err(Error::Unsupported("held Kingdom derived detection fields"));
        }
        let entry = self
            .record
            .registry
            .get(kingdom)
            .ok_or_else(|| error("4EBC70 actual Kingdom generation is absent"))?;
        let RestoredRecord::Kingdom(kingdom) = &entry.value else {
            return Err(error("4EBC70 vector entry is not a current Kingdom"));
        };
        let runtime = kingdom
            .runtime
            .as_ref()
            .ok_or_else(|| error("4EBC70 Kingdom runtime is absent"))?;
        Source::bind(self.catalog, &runtime.definition)?;
        if runtime.definition.native_class != 4
            || runtime.base.kind != aa_actors::loading::LoadKind::Actor
            || runtime.base.object.is_none()
            || runtime.base.actor.is_none()
            || !crate::load_spatial::same_proof(
                &runtime.base.thing.definition,
                &runtime.definition.proof,
            )
        {
            return Err(error("4EBC70 actual stored Kingdom source/storage differs"));
        }
        runtime
            .derived
            .as_ref()
            .and_then(|s| s.words.get(&offset))
            .copied()
            .ok_or_else(|| error(format!("4EBC70 actual Kingdom+{offset:X} field is absent")))
    }
}
impl<S: MemoryServices> World for Owned<'_, '_, '_, S> {
    fn keeps(&mut self) -> Result<bool> {
        Ok(self.memory()?.count() > 0)
    }
    fn eligible(&mut self) -> Result<bool> {
        live_object_init::memory_eligible_fields(
            &self.record.memory_record(),
            self.source,
            self.services,
        )
    }
    fn kingdom_count(&mut self) -> Result<i32> {
        i32::try_from(self.kingdoms.majors().len()).map_err(error)
    }
    fn kingdom_slot(&mut self, index: i32) -> Result<Option<ActorId>> {
        self.kingdoms
            .majors()
            .get(usize::try_from(index).map_err(error)?)
            .copied()
            .ok_or_else(|| error("4EBC70 current Kingdom vector index is absent"))
    }
    fn nonmajor(&mut self, kingdom: ActorId) -> Result<u32> {
        self.kingdom_word(kingdom, 0x190)
    }
    fn kingdom_index(&mut self, kingdom: ActorId) -> Result<u32> {
        self.kingdom_word(kingdom, 0x18c)
    }
    fn slot(&mut self, index: u32) -> Result<Option<RecordId>> {
        self.memory()?.slot(index).map_err(error)
    }
    fn shared_visible(&mut self, kingdom: ActorId) -> Result<bool> {
        if self.nonmajor(kingdom)? != 0 {
            return Ok(true);
        }
        Ok(self.kingdom_word(kingdom, 0x228)? & self.record.word(0x54)? & 0xffff0000 != 0)
    }
    fn shared_explored(&mut self, kingdom: ActorId) -> Result<bool> {
        if self.nonmajor(kingdom)? != 0 {
            return Ok(true);
        }
        Ok(self.kingdom_word(kingdom, 0x228)? & self.record.word(0x54)? & 0xffff != 0)
    }
    fn flags(&mut self) -> Result<u32> {
        self.record.word(8)
    }
    fn create(&mut self, kingdom: ActorId) -> Result<()> {
        if self.nonmajor(kingdom)? != 0 {
            return Ok(());
        }
        let flags = self.flags()?;
        if flags & (1 << 7) != 0 {
            return Err(Error::Unsupported(
                "4ECA80 dead Object potential-visible virtual owner",
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
            self.memory_mut()?.allocate_slots(count).map_err(error)?;
        }
        let record = self.memory_mut()?.allocate_record().map_err(error)?;
        let clock = self
            .catalog
            .load_clock
            .ok_or(Error::Unsupported("4EC4C8 current GWorld+E0 clock owner"))?;
        self.memory_mut()?
            .record_mut(record)
            .map_err(error)?
            .timestamp_bits = clock.begin.0;
        // Native has already allocated the table/record and stored time before
        // asking the ACTUAL virtual24 name (Area/Camera override this getter).
        Err(Error::Unsupported(
            "4EC4D3 actual current Object virtual24 memory-name/refresh owner",
        ))
    }
    fn delete(&mut self, kingdom: ActorId) -> Result<()> {
        //Actor/Kingdom virtual14C is4CE8C0, not Object4EB970. Its current
        //container getter precedes every base deletion guard.
        if matches!(self.record.definition.native_class, 3 | 4)
            && self.actor_container()?.is_some()
            && self.current_record(kingdom)?.is_some()
        {
            let record = self
                .current_record(kingdom)?
                .ok_or(Error::NullDereference("4CE8C0 reread memory record"))?;
            let owner = self
                .record
                .actor
                .as_ref()
                .and_then(|a| a.words.get(&0xe8))
                .copied()
                .ok_or(Error::Unsupported("4CE8C0 current Actor+E8 owning Kingdom"))?;
            if self.memory()?.record(record).map_err(error)?.owner.raw() != owner {
                return Err(Error::Unsupported(
                    "4CE905 current container Settlement585FF0 memory notification",
                ));
            }
        }
        if self.nonmajor(kingdom)? != 0 || !self.keeps()? {
            return Ok(());
        }
        let index = self.kingdom_index(kingdom)?;
        let Some(record) = self.slot(index)? else {
            return Ok(());
        };
        if self.catalog.saved_observer_active().map_err(error)? {
            return Err(Error::Unsupported(
                "4EB970 active observer79DEB0 memory deletion",
            ));
        }
        self.memory_mut()?.free_record(record).map_err(error)?;
        self.memory_mut()?.set_slot(index, None).map_err(error)
    }
    fn count(&mut self) -> Result<i32> {
        Ok(self.memory()?.count())
    }
    fn clear(&mut self) -> Result<()> {
        if !self.keeps()? {
            return Ok(());
        }
        let mut index = 0;
        while index < self.count()? {
            if let Some(record) = self.slot(index as u32)? {
                if self.catalog.saved_observer_present().map_err(error)? {
                    return Err(Error::Unsupported(
                        "4EBB80 present observer79DEB0 memory clear",
                    ));
                }
                self.memory_mut()?.free_record(record).map_err(error)?;
                self.memory_mut()?
                    .set_slot(index as u32, None)
                    .map_err(error)?;
            }
            index += 1;
        }
        if self.count()? > 0 {
            self.memory_mut()?.release_slots().map_err(error)?;
        }
        Ok(())
    }
}
pub fn refresh(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    source: &Source,
    services: &mut impl MemoryServices,
    kingdoms: &KingdomRegistryState,
) -> Result<()> {
    source.validate_record(record)?;
    refresh_fields(
        catalog,
        &mut Fields::from_record(record)?,
        source,
        services,
        kingdoms,
    )
}

pub(crate) fn refresh_fields(
    catalog: &Catalog<'_>,
    record: &mut Fields<'_>,
    source: &Source,
    services: &mut impl MemoryServices,
    kingdoms: &KingdomRegistryState,
) -> Result<()> {
    source.validate_memory_record(&record.memory_record())?;
    let mut world = Owned {
        catalog,
        record,
        source,
        services,
        kingdoms,
    };
    #[cfg(test)]
    let capture = if CAPTURE_ENABLED.get()
        && world.record.owner.raw() == 28
        && matches!(world.record.definition.native_class, 6 | 7 | 9)
    {
        Some(capture_entry(&mut world))
    } else {
        None
    };
    let result = refresh_inner(&mut world);
    #[cfg(test)]
    if let Some(capture) = capture {
        let value = match capture {
            Ok(input) => {
                serde_json::json!({"input":input,"result":result.as_ref().map(|_|()).map_err(|e|e.to_string()),
                "after":{"count":world.memory().unwrap().count(),"allocation":world.memory().unwrap().allocation_present(),
                    "flags":world.record.word(8).unwrap(),"word":world.record.word(0x54).unwrap(),"allocated_records":world.memory().unwrap().allocated_records()}})
            }
            Err(error) => serde_json::json!({"capture_error":error.to_string()}),
        };
        ACTUAL_CAPTURE.with(|capture| {
            *capture.borrow_mut() = Some(value);
        });
    }
    result
}

#[cfg(test)]
fn capture_entry<S: MemoryServices>(world: &mut Owned<'_, '_, '_, S>) -> Result<serde_json::Value> {
    let mut kingdoms = Vec::new();
    for index in 0..world.kingdom_count()? {
        let value = match world.kingdom_slot(index)? {
            Some(k) => {
                serde_json::json!({"id":k.raw(),"index":world.kingdom_index(k)?,"nonmajor":world.nonmajor(k)?,"shared":world.kingdom_word(k,0x228)?})
            }
            None => serde_json::Value::Null,
        };
        kingdoms.push(value);
    }
    let slots = if world.memory()?.allocation_present() {
        Some(
            (0..world.memory()?.count())
                .map(|i| world.memory().unwrap().slot(i as u32).unwrap().is_some())
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    Ok(
        serde_json::json!({"name":"actual_saved_Object28","owner":world.record.owner.raw(),"native_class":world.record.definition.native_class,
        "definition":world.record.definition.proof,"source_precision":world.source.source_precision(),
        "eligible":world.eligible()?,"word":world.record.word(0x54)?,"flags":world.record.word(8)?,"kingdoms":kingdoms,
        "slots":slots,"fail":null}),
    )
}
