//!4EC4A0 over the sole staged ActorStore object and detection record owner.
//!Each store follows its native callback immediately; failures keep its prefix.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeObject, Source as ActorSource, SpecialSource},
    catalog::Catalog,
    live_object_init::Source,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot,
    detection_memory::{Record, RecordId},
    owned,
};
use aa_mapworld::{map_objects::SpecialObject, restore::RestoreDefinitions};
#[cfg(test)]
thread_local! {pub(crate) static ACTUAL_INPUT:std::cell::RefCell<Option<serde_json::Value>>=const{std::cell::RefCell::new(None)};}
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
trait World {
    fn time(&mut self) -> Result<u32>;
    fn name(&mut self) -> Result<Vec<u16>>;
    fn position(&mut self) -> Result<[u32; 2]>;
    fn definition(&mut self) -> Result<String>;
    fn owner(&mut self) -> Result<ActorId>;
    fn aspect(&mut self) -> Result<u32>;
    fn stat(&mut self, slot: u32) -> Result<u32>;
    fn flags(&mut self) -> Result<u32>;
    fn record(&mut self) -> Result<&mut Record>;
}
fn refresh_inner(w: &mut impl World) -> Result<()> {
    let time = w.time()?;
    w.record()?.timestamp_bits = time;
    let name = w.name()?;
    w.record()?.name_utf16 = name;
    let position = w.position()?;
    w.record()?.position_bits = position;
    let definition = w.definition()?;
    w.record()?.definition = Some(definition);
    let owner = w.owner()?;
    w.record()?.owner = owner;
    let aspect = w.aspect()?;
    w.record()?.aspect = aspect;
    //Native calls114 before118/11C; its destination is record+2C (index4).
    for (slot, index) in [(0x10c, 0), (0x110, 1), (0x114, 4), (0x118, 2), (0x11c, 3)] {
        let value = w.stat(slot)?;
        w.record()?.stat_words[index] = Some(value);
    }
    let flags = w.flags()?;
    w.record()?.byte_30 = ((flags >> 30) & 1) as u8;
    let flags = w.flags()?;
    w.record()?.flags = flags;
    Ok(())
}
struct Owned<'a, 'source> {
    catalog: &'a Catalog<'source>,
    bank: &'a crate::definitions::Bank,
    actors: &'a mut ActorStore,
    actor: ActorId,
    record: RecordId,
}
impl Owned<'_, '_> {
    fn key(&self) -> Result<ObjectKey> {
        self.actors
            .key(self.actor)
            .ok_or_else(|| error("4EC4A0 current ActorStore generation absent"))
    }
    fn object(&self) -> Result<&RuntimeObject> {
        self.actors.object(self.key()?).map_err(error)
    }
    fn source(&self) -> Result<Source> {
        let o = self.object()?;
        let source = Source::bind_in_bank(self.catalog, self.bank, &o.definition)?;
        let valid = match o.definition.native_class {
            2 | 6 | 7 | 9 => {
                o.kind == aa_actors::loading::LoadKind::Object
                    && o.object.is_some()
                    && o.actor.is_none()
            }
            3 | 4 => {
                o.kind == aa_actors::loading::LoadKind::Actor
                    && o.object.is_some()
                    && o.actor.is_some()
            }
            _ => false,
        };
        if !valid || !crate::load_spatial::same_proof(&o.thing.definition, &o.definition.proof) {
            return Err(error("4EC4A0 actual constructor/source owner differs"));
        }
        Ok(source)
    }
}
impl World for Owned<'_, '_> {
    fn time(&mut self) -> Result<u32> {
        Ok(self
            .catalog
            .load_clock
            .ok_or(Error::Unsupported("4EC4C8 actual GWorld+E0 clock owner"))?
            .begin
            .0)
    }
    fn name(&mut self) -> Result<Vec<u16>> {
        let source = self.source()?;
        let o = self.object()?;
        match o.definition.native_class {
            2 | 9 => self
                .catalog
                .actor_display_name(&o.definition.proof.id, source.source_precision())
                .map_err(error),
            3 => {
                let a = o
                    .actor
                    .as_ref()
                    .ok_or(Error::NullDereference("4D99D0 actual Actor"))?;
                if a.components.get(ComponentSlot::Settlement).is_some() {
                    return Err(Error::Unsupported(
                        "4D99D0 actual Settlement+14 parent naming owner",
                    ));
                }
                if !a.string_dc_utf16.is_empty() {
                    return Ok(a.string_e0_utf16.clone());
                }
                let number = a
                    .words
                    .get(&0x170)
                    .copied()
                    .ok_or(Error::Unsupported("4D8CA0 actual Actor+170"))?;
                if number != 0 {
                    return Err(Error::Unsupported(
                        "4D8AB0 staged numbered-name localization owner",
                    ));
                }
                self.catalog
                    .actor_display_name(&o.definition.proof.id, source.source_precision())
                    .map_err(error)
            }
            4 => {
                let ActorSource::Special(s) =
                    &self.actors.entity(self.key()?).map_err(error)?.source
                else {
                    return Err(error("4E5340 actual Kingdom derived owner absent"));
                };
                if !matches!(s.source, SpecialSource::Kingdom(_)) {
                    return Err(error("4E5340 actual derived owner is not Kingdom"));
                }
                s.derived
                    .as_ref()
                    .and_then(|s| s.strings_utf16.get(&0x198))
                    .cloned()
                    .ok_or(Error::Unsupported("4E5340 actual Kingdom+198 name"))
            }
            6 | 7 => {
                let ActorSource::Loaded(s) =
                    &self.actors.entity(self.key()?).map_err(error)?.source
                else {
                    return Err(error("map object derived name owner absent"));
                };
                match (&s.special_object, o.definition.native_class) {
                    (Some(SpecialObject::Area(a)), 6) => Ok(a.label_utf16.clone()),
                    (Some(SpecialObject::Camera(c)), 7) => Ok(c.label_utf16.clone()),
                    _ => Err(Error::Unsupported("actual Area+80/Camera+64 label owner")),
                }
            }
            _ => Err(Error::Unsupported("4EC4D3 actual Object virtual24")),
        }
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        let p = self
            .actors
            .staged_placement(self.key()?)
            .map_err(error)?
            .position_bits;
        Ok([p[0], p[1]])
    }
    fn definition(&mut self) -> Result<String> {
        Ok(self.object()?.definition.proof.id.clone())
    }
    fn owner(&mut self) -> Result<ActorId> {
        let o = self.object()?;
        match o.definition.native_class {
            2 | 6 | 7 | 9 => Ok(ActorId::NULL),
            3 | 4 => Ok(ActorId::from_raw(
                o.actor
                    .as_ref()
                    .ok_or(Error::NullDereference("4D1E80 actual Actor"))?
                    .words
                    .get(&0xe8)
                    .copied()
                    .ok_or(Error::Unsupported("4D1E80 actual Actor+E8"))?,
            )),
            _ => Err(Error::Unsupported("4EC4A0 actual virtual108")),
        }
    }
    fn aspect(&mut self) -> Result<u32> {
        let source = self.source()?;
        let o = self.object()?;
        if matches!(o.definition.native_class, 3 | 4) {
            let a = o
                .actor
                .as_ref()
                .ok_or(Error::NullDereference("4EAAA0 actual Actor"))?;
            if a.components.state.construction() {
                return Err(Error::Unsupported(
                    "4EAAA0 actual Data+11C/+120 construction aspect list",
                ));
            }
            if a.words
                .get(&0x15c)
                .copied()
                .ok_or(Error::Unsupported("4CF100 actual Actor+15C stance"))?
                != u32::MAX
            {
                return Err(Error::Unsupported("4D0700 actual staged stance aspect"));
            }
        }
        let variant = o
            .object
            .as_ref()
            .ok_or(Error::NullDereference("4EAAA0 actual Object+40"))?
            .variant;
        //DoodadData virtual40 is the actual true leaf for native class9.
        let enabled = if o.definition.native_class == 9 {
            true
        } else {
            self.catalog
                .actors
                .native_field(
                    &o.definition.proof.id,
                    "View.aspects_as_variations",
                    source.source_precision(),
                )
                .map_err(error)?
                .ok_or(Error::Unsupported(
                    "4EAAA0 actual ObjectData+108 source field",
                ))?
                .boolean()
                .map_err(error)?
        };
        if enabled && variant != -1 {
            return Ok(variant as u32);
        }
        Err(Error::Unsupported(
            "49B480 actual ObjectData+118 default-aspect source owner",
        ))
    }
    fn stat(&mut self, slot: u32) -> Result<u32> {
        let o = self.object()?;
        if matches!(o.definition.native_class, 2 | 6 | 7 | 9) {
            return Ok(0);
        } //4DB6B0 fld[9F32DC].
        let a = o
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("4CEC50 actual Actor"))?;
        match slot {
            0x10c | 0x110 | 0x114 => match a.components.get(ComponentSlot::Body) {
                None => Ok(0),
                Some(owned::Component::Body(b)) => match slot {
                    0x10c => Ok(b.body.health.0),
                    0x110 => Ok(b.body.maximum.0),
                    _ => Err(Error::Unsupported(
                        "4CEC90 actual529C30/45D730 modified maximum health",
                    )),
                },
                Some(_) => Err(error("actual Body slot has another payload")),
            },
            0x118 | 0x11c => {
                if a.components.get(ComponentSlot::Denizen).is_none() {
                    Ok(0)
                } else {
                    Err(Error::Unsupported(
                        "4CECB0/4CECE0 actual Denizen Data+424/current inventory",
                    ))
                }
            }
            _ => Err(Error::Unsupported("4EC4A0 statistic virtual")),
        }
    }
    fn flags(&mut self) -> Result<u32> {
        Ok(self
            .actors
            .staged_placement(self.key()?)
            .map_err(error)?
            .flags)
    }
    fn record(&mut self) -> Result<&mut Record> {
        let key = self.key()?;
        self.actors
            .object_mut(key)
            .map_err(error)?
            .object
            .as_mut()
            .ok_or(Error::NullDereference("4EC4A0 actual Object memory"))?
            .memory
            .record_mut(self.record)
            .map_err(error)
    }
}
pub(crate) fn refresh(
    catalog: &Catalog<'_>,
    bank: &crate::definitions::Bank,
    actors: &mut ActorStore,
    actor: ActorId,
    record: RecordId,
) -> Result<()> {
    let mut world = Owned {
        catalog,
        bank,
        actors,
        actor,
        record,
    };
    #[cfg(test)]
    if actor.raw() == 28 {
        let input = (|| -> Result<serde_json::Value> {
            let source = world.source()?;
            let object = world.object()?;
            Ok(
                serde_json::json!({"class":object.definition.native_class,"definition":object.definition.proof,
                "source_precision":source.source_precision(),"time":world.catalog.load_clock.ok_or(Error::Unsupported("actual clock"))?.begin.0,
                "flags":world.actors.staged_placement(world.key()?).map_err(error)?.flags,
                "position":world.actors.staged_placement(world.key()?).map_err(error)?.position_bits[..2],
                "variant":object.object.as_ref().unwrap().variant,
                "display":world.catalog.actor_display_name(&object.definition.proof.id,source.source_precision()).map_err(error)?}),
            )
        })();
        if let Ok(input) = input {
            ACTUAL_INPUT.with(|v| {
                let mut v = v.borrow_mut();
                if v.is_none() {
                    *v = Some(input);
                }
            });
        }
    }
    refresh_inner(&mut world)
}
#[cfg(test)]
#[path = "load_store_memory_refresh_tests.rs"]
mod tests;
