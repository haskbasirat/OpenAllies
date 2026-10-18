//!515620 slot4 ->514DA0/514E40 over current published actors and their zones.
//!All reads borrow ActorAccess and the SAME moved source Bank. A positive
//!component child remains required after its native guards and null checks.
use crate::{
    actor_world::{ActorAccess, ObjectKey, RuntimeComponent, Source},
    catalog::Catalog,
    definitions::Bank,
    spatial_world::SpatialWorld,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot as Slot,
};
use aa_zones::{
    tile::ComponentEffect,
    zone::{EnemyEnteredFields, Zone},
};

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn zone_error(e: impl std::fmt::Display) -> aa_zones::Error {
    aa_zones::Error::World(e.to_string())
}
struct Read<'a, 'store, 'source> {
    actors: &'a ActorAccess<'store>,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    spatial: &'a SpatialWorld,
    zone: Option<&'a Zone>,
}
impl Read<'_, '_, '_> {
    fn key(&self, id: ActorId) -> aa_zones::Result<ObjectKey> {
        let key = self.actors.key(id).ok_or_else(|| {
            zone_error(format!(
                "registration control Actor generation {id:?} absent"
            ))
        })?;
        if self.actors.id(key) != Some(id) {
            return Err(zone_error(
                "registration control reverse generation differs",
            ));
        }
        Ok(key)
    }
    fn ordinary(&self, id: ActorId) -> aa_zones::Result<ObjectKey> {
        let key = self.key(id)?;
        let object = self.actors.object(key).map_err(zone_error)?;
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, &object.definition)
            .map_err(zone_error)?;
        if !crate::load_spatial::ordinary_layout(
            object.definition.native_class,
            object.kind,
            object.object.is_some(),
            object.actor.is_some(),
        ) || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
        {
            return Err(zone_error(
                "registration control current source/constructor layout differs",
            ));
        }
        Ok(key)
    }
    fn word(&self, id: ActorId, offset: u32) -> aa_zones::Result<u32> {
        let key = self.ordinary(id)?;
        self.actors
            .object(key)
            .map_err(zone_error)?
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&offset))
            .copied()
            .ok_or_else(|| {
                zone_error(format!(
                    "registration control Actor {id:?}+{offset:X} absent"
                ))
            })
    }
    fn zone(&self) -> aa_zones::Result<&Zone> {
        self.zone
            .ok_or(aa_zones::Error::NativeNull("registration control zone4"))
    }
    fn special(
        &self,
        id: ActorId,
        class: u32,
    ) -> aa_zones::Result<&aa_mapworld::special_state::SpecialShell> {
        let key = self.key(id)?;
        let object = self.actors.object(key).map_err(zone_error)?;
        if object.definition.native_class != class
            || object.definition.proof.category.runtime_class != class
            || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
        {
            return Err(zone_error(
                "registration control relation source class differs",
            ));
        }
        if class == 4 {
            self.ordinary(id)?;
        } else if object.kind != aa_actors::loading::LoadKind::Thing
            || object.object.is_some()
            || object.actor.is_some()
        {
            return Err(zone_error(
                "registration control Team constructor layout differs",
            ));
        }
        let Source::Special(s) = &self.actors.entity(key).map_err(zone_error)?.source else {
            return Err(zone_error(
                "registration control relation derived owner absent",
            ));
        };
        if !matches!(
            (&s.source, class),
            (crate::actor_world::SpecialSource::Kingdom(_), 4)
                | (crate::actor_world::SpecialSource::Team(_), 5)
        ) {
            return Err(zone_error(
                "registration control relation source kind differs",
            ));
        }
        s.derived
            .as_ref()
            .ok_or_else(|| zone_error("registration control relation fields absent"))
    }
}
impl crate::relations::borrowed::Read for Read<'_, '_, '_> {
    type Id = ActorId;
    type Error = aa_zones::Error;
    fn kingdom_team(&self, id: ActorId) -> aa_zones::Result<Option<ActorId>> {
        self.special(id, 4)?
            .identity_references
            .get(&0x1ec)
            .copied()
            .ok_or_else(|| zone_error("registration control actual Kingdom1EC absent"))
    }
    fn kingdom_slot(&self, id: ActorId) -> aa_zones::Result<u32> {
        self.special(id, 4)?
            .words
            .get(&0x188)
            .copied()
            .ok_or_else(|| zone_error("registration control Kingdom188 absent"))
    }
    fn team_slot(&self, id: ActorId) -> aa_zones::Result<u32> {
        self.special(id, 5)?
            .words
            .get(&0x38)
            .copied()
            .ok_or_else(|| zone_error("registration control Team38 absent"))
    }
    fn kingdom_word(&self, id: ActorId, team: bool, slot: u32) -> aa_zones::Result<u32> {
        self.special(id, 4)?
            .vectors
            .get(&if team { 0x1dc } else { 0x1d0 })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| zone_error("registration control Kingdom relation slot absent"))
    }
    fn team_word(&self, id: ActorId, team: bool, slot: u32) -> aa_zones::Result<u32> {
        self.special(id, 5)?
            .vectors
            .get(&if team { 0x58 } else { 0x4c })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| zone_error("registration control Team relation slot absent"))
    }
    fn orphan_relation(&self) -> aa_zones::Result<u32> {
        Err(zone_error(
            "registration control actual mutable B28158 owner required",
        ))
    }
    fn team_null_relation(&self) -> aa_zones::Result<u32> {
        Err(zone_error(
            "registration control actual mutable B2815C owner required",
        ))
    }
}
impl EnemyEnteredFields for Read<'_, '_, '_> {
    type Component = (ActorId, Slot);
    fn actor_flags(&mut self, a: u32) -> aa_zones::Result<u32> {
        let id = ActorId::from_raw(a);
        let key = self.ordinary(id)?;
        self.actors
            .placement(key, self.spatial)
            .map(|p| p.flags)
            .map_err(zone_error)
    }
    fn zone_owner(&mut self) -> aa_zones::Result<u32> {
        let owner = self.zone()?.actor;
        self.word(ActorId::from_raw(owner), 0xe8)
    }
    fn enemy(&mut self, a: u32, k: u32) -> aa_zones::Result<bool> {
        let id = ActorId::from_raw(a);
        let key = self.ordinary(id)?;
        let other = (k != 0).then_some(ActorId::from_raw(k));
        let code = match self
            .actors
            .object(key)
            .map_err(zone_error)?
            .definition
            .native_class
        {
            3 => {
                let owner = self.word(id, 0xe8)?;
                crate::relations::borrowed::actor(
                    self,
                    (owner != 0).then_some(ActorId::from_raw(owner)),
                    other,
                )?
            }
            4 => crate::relations::borrowed::kingdom(self, id, other)?,
            _ => {
                return Err(zone_error(
                    "registration control actual Actor virtual140 dispatch required",
                ));
            }
        };
        Ok(code == 3)
    }
    fn actor_redirect(&mut self, a: u32) -> aa_zones::Result<Option<u32>> {
        let id = ActorId::from_raw(a);
        self.ordinary(id)?;
        match self
            .actors
            .component(id, Slot::Element)
            .map_err(zone_error)?
        {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == id => {
                //514DA0 reads Element10 first and only then byte1C.
                Ok(e.organization.filter(|_| e.flag_1c).map(ActorId::raw))
            }
            _ => Err(zone_error(
                "registration control actual Element allocation/owner differs",
            )),
        }
    }
    fn zone_flags(&mut self) -> aa_zones::Result<u16> {
        Ok(self.zone()?.flags)
    }
    fn component(&mut self, offset: u32) -> aa_zones::Result<Option<Self::Component>> {
        let zone_actor = ActorId::from_raw(self.zone()?.actor);
        self.ordinary(zone_actor)?;
        let slot = match offset {
            0x88 => Slot::Denizen,
            0x7c => Slot::Organization,
            0xb0 => Slot::Defender,
            _ => {
                return Err(zone_error(
                    "registration control native component offset differs",
                ));
            }
        };
        Ok(self
            .actors
            .component(zone_actor, slot)
            .map_err(zone_error)?
            .map(|_| (zone_actor, slot)))
    }
    fn effect(
        &mut self,
        e: ComponentEffect,
        c: Self::Component,
        target: u32,
    ) -> aa_zones::Result<()> {
        Err(zone_error(format!(
            "registration control actual component child {e:?} zone_actor={:?} slot={:?} target={target}",
            c.0, c.1
        )))
    }
}

pub(crate) fn entered(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    bank: &Bank,
    spatial: &SpatialWorld,
    zone_actor: ActorId,
    actor: ActorId,
) -> Result<()> {
    let key = actors.key(zone_actor).ok_or_else(|| {
        error(format!(
            "registration slot4 zone Actor {zone_actor:?} generation absent; entering={actor:?}"
        ))
    })?;
    let object = actors.object(key).map_err(error)?;
    let zones = object
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("registration zone Actor"))?
        .zones_148
        .zones();
    let zone = zones.slots.get(4).and_then(Option::as_ref);
    if zone.is_some_and(|z| z.actor != zone_actor.raw()) {
        return Err(error("registration slot4 actual zone owner differs"));
    }
    aa_zones::zone::enemy_entered_fields(
        &mut Read {
            actors,
            catalog,
            bank,
            spatial,
            zone,
        },
        actor.raw(),
    )
    .map_err(|e| {
        error(format!(
            "registration slot4 zone_actor={zone_actor:?} entering={actor:?}: {e}"
        ))
    })
}
