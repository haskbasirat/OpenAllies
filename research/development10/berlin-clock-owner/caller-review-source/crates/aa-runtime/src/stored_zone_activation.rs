//!515F40/514CE0 over disjoint SAME ActorStore zones and current World owners.
//!No runtime allocation is removed; every current Actor lookup resolves its
//!actual borrowed fields. Unbound positive callbacks retain native prefixes.
use crate::{
    actor_world::{ActorStore, ObjectKey, Source, ZoneFields},
    catalog::Catalog,
    definitions::Bank,
    load_detection_activation::{self, Effects},
    load_world::World,
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_numeric::Precision;
use aa_zones::{
    Error, Result,
    flood::FloodWorld,
    query::{Kingdom, VisitWorld, ZoneActors},
    tile::{ComponentEffect, SaiHook, ZoneWorld},
    zone::ZoneObjectWorld,
};
#[path = "zone_primary_reference.rs"]
mod primary_reference;
#[path = "stored_zone_relations.rs"]
mod relations;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required<T>(s: &str) -> Result<T> {
    Err(error(format!("required native service: {s}")))
}
/// The gate lends the actual Session. Missing bindings remain errors only at
/// a reached Session read; neither phase nor application state is cached.
struct SessionLoan<'a>(Option<&'a dyn crate::load_world_globals::SessionPhase>);
impl crate::load_world_globals::SessionPhase for SessionLoan<'_> {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        self.0
            .ok_or("stored zone requires actual Session+F8 owner")?
            .current_phase()
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        self.0
            .ok_or("stored zone requires actual Session+E4 owner")?
            .application_state()
    }
}
struct View<'a, 'fields, 'source, E> {
    fields: &'a mut ZoneFields<'fields>,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    spatial: &'a SpatialWorld,
    effects: &'a mut E,
    inactive: Option<&'a crate::restore_lifecycle::InactiveLoad>,
    kingdoms: &'a crate::load_kingdom_init_world::KingdomRegistryState,
    memory_services: &'a crate::load_zone_services::Context<'a>,
    width: i32,
}
impl<E: Effects> View<'_, '_, '_, E> {
    fn id(&self, raw: u32) -> Result<ActorId> {
        let id = ActorId::from_raw(raw);
        if self.fields.present(id) {
            Ok(id)
        } else {
            Err(error("stored zone Actor generation absent"))
        }
    }
    fn checked(&self, raw: u32) -> Result<ActorId> {
        let id = self.id(raw)?;
        let d = self.fields.definition(id).map_err(error)?;
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, d).map_err(error)?;
        let (k, o, a) = self.fields.kind(id).map_err(error)?;
        if !self.fields.same_definition_proof(id).map_err(error)?
            || !crate::load_spatial::ordinary_layout(d.native_class, k, o, a)
        {
            return required("stored zone actual constructor layout");
        };
        Ok(id)
    }
    fn word(&self, raw: u32, offset: u32) -> Result<u32> {
        let id = self.checked(raw)?;
        self.fields
            .words(id)
            .map_err(error)?
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("stored zone Actor+{offset:X} uninitialized")))
    }
    fn special(&self, raw: u32) -> Result<&aa_mapworld::special_state::SpecialShell> {
        let id = self.checked(raw)?;
        if self.fields.definition(id).map_err(error)?.native_class != 4 {
            return required("stored zone actual Kingdom class");
        };
        let Source::Special(s) = self.fields.source(id).map_err(error)? else {
            return required("stored zone actual Kingdom derived owner");
        };
        s.derived
            .as_ref()
            .ok_or_else(|| error("stored zone Kingdom derived fields absent"))
    }
    fn kingdom_word(&self, raw: u32, offset: u32) -> Result<u32> {
        self.special(raw)?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("stored zone Kingdom+{offset:X} uninitialized")))
    }
    fn read_component(
        &self,
        raw: u32,
        slot: Slot,
    ) -> Result<Option<&crate::actor_world::RuntimeComponent>> {
        let id = self.checked(raw)?;
        Ok(self.fields.components(id).map_err(error)?.get(slot))
    }
    fn pose(&self, raw: u32) -> Result<crate::actor_world::PlacementState> {
        let id = self.checked(raw)?;
        self.fields.placement(id, self.spatial).map_err(error)
    }
}
impl<E: Effects> ZoneActors for View<'_, '_, '_, E> {
    fn owner(&mut self, a: u32) -> Result<u32> {
        self.word(a, 0xe8)
    }
    fn allied(&mut self, a: u32, k: u32) -> Result<bool> {
        let id = self.checked(a)?;
        let same = match self.fields.definition(id).map_err(error)?.native_class {
            3 => self.word(a, 0xe8)? == k,
            4 => a == k,
            _ => return required("stored zone Actor138 class dispatch"),
        };
        Ok(same || self.related_code(a, k)? == 1)
    }
    fn enemy(&mut self, a: u32, k: u32) -> Result<bool> {
        Ok(self.related_code(a, k)? == 3)
    }
    fn zone_slot(&mut self, a: u32, i: u32) -> Result<u32> {
        let id = self.checked(a)?;
        self.fields.zone_key(id, i as usize).map_err(error)
    }
    fn word4(&mut self, _a: u32) -> Result<u32> {
        required("stored zone actual definition exclusion identity")
    }
    fn component_b0(&mut self, a: u32) -> Result<u32> {
        ZoneWorld::component(self, a, 0xb0)
    }
    fn reveal_mask(&mut self, a: u32) -> Result<Option<u32>> {
        match self.read_component(a, Slot::Detection)? {
            None => Ok(None),
            Some(crate::actor_world::RuntimeComponent::Detection(d)) => Ok(Some(d.word_10)),
            _ => required("stored zone actual Detection component"),
        }
    }
}
impl<E: Effects> VisitWorld for View<'_, '_, '_, E> {
    fn kingdom(&mut self, k: u32) -> Result<Kingdom> {
        Ok(Kingdom {
            key: k,
            index: self.kingdom_word(k, 0x18c)?,
            nonmajor: self.kingdom_word(k, 0x190)?,
            shared_mask: self.kingdom_word(k, 0x228)?,
            supply_mask: load_detection_activation::kingdom_supply_mask(self.special(k)?)?,
        })
    }
    fn kingdom_slot(&mut self, i: u32) -> Result<u32> {
        self.kingdoms
            .majors()
            .get(i as usize)
            .map(|v| v.map_or(0, ActorId::raw))
            .ok_or_else(|| error("stored zone actual World major slot absent"))
    }
}
impl<E: Effects> ZoneWorld for View<'_, '_, '_, E> {
    fn fog_enabled(&mut self) -> Result<bool> {
        self.effects.fog_enabled()
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        let inactive = self.inactive.ok_or_else(|| {
            error("stored zone SAI active-byte read requires its same staged lifecycle owner")
        })?;
        self.catalog
            .saved_observer_active_in_load(inactive)
            .map_err(error)
    }
    fn sai(&mut self, _: SaiHook, _: usize, _: u32) -> Result<()> {
        required("stored zone actual SAI region-counter mutation owner")
    }
    fn kingdom_list(&mut self) -> Result<Vec<u32>> {
        Ok(self
            .kingdoms
            .majors()
            .iter()
            .map(|v| v.map_or(0, ActorId::raw))
            .collect())
    }
    fn tile_actors(&mut self, t: usize) -> Result<Vec<u32>> {
        self.effects.tile_actors(t)
    }
    fn actor_reveal(&mut self, actor: u32, kingdom: u32, explored: bool, visible: bool) -> Result<()> {
        let mut access=crate::actor_world::ZoneMemoryAccess{fields:self.fields,spatial:self.spatial};
        crate::stored_actor_reveal::reveal(self.catalog,self.bank,&mut access,
            self.memory_services,self.kingdoms,self.inactive,ActorId::from_raw(actor),
            ActorId::from_raw(kingdom),explored,visible).map_err(error)
    }
    fn actor_hide(&mut self, _: u32, _: u32) -> Result<()> {
        required("stored zone actual Actor68 hide")
    }
    fn actor_unexplore(&mut self, _: u32, _: u32) -> Result<()> {
        required("stored zone actual Actor6C unexplore")
    }
    fn zone_effects_enabled(&mut self) -> Result<bool> {
        self.effects.zone_effects_enabled()
    }
    fn actor_b4(&mut self, a: u32) -> Result<bool> {
        let id = self.checked(a)?;
        Ok(matches!(
            self.fields.definition(id).map_err(error)?.native_class,
            3 | 4
        ))
    }
    fn actor_multi_tile(&mut self, a: u32) -> Result<bool> {
        let flags = self.pose(a)?.flags;
        if flags & 0x20000400 != 0x20000400 {
            return Ok(false);
        };
        let id = self.checked(a)?;
        let d = self.fields.definition(id).map_err(error)?;
        let s = crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, d)
            .map_err(error)?;
        aa_spatial::geometry::marks_blocker(
            flags,
            s.blocking_shape().radius_bits,
            self.spatial.index().grid().cell_size_bits,
            self.spatial.index().precision(),
        )
        .map_err(error)
    }
    fn actor_tile(&mut self, a: u32) -> Result<i64> {
        let p = self.pose(a)?;
        let tile = p
            .cached_tile
            .ok_or_else(|| error("stored zone actual cached tile absent"))?;
        Ok(i64::from(self.width) * i64::from(tile[1]) + i64::from(tile[0]))
    }
    fn actor_flags(&mut self, a: u32) -> Result<u32> {
        Ok(self.pose(a)?.flags)
    }
    fn actor_redirect(&mut self, _: u32) -> Result<Option<u32>> {
        required("stored zone actual Denizen redirect")
    }
    fn component(&mut self, a: u32, offset: u32) -> Result<u32> {
        if offset == 0x100 {
            let id = self.checked(a)?;
            return Ok(self.fields.components(id).map_err(error)?.state.low);
        }
        if offset == 0x138 {
            return self.word(a, offset);
        }
        let slot = aa_actors::components::ALL_SLOTS
            .iter()
            .copied()
            .find(|s| 0x60 + 4 * s.index() as u32 == offset)
            .ok_or_else(|| error("stored zone unsupported component offset"))?;
        Ok(u32::from(self.read_component(a, slot)?.is_some()))
    }
    fn component_effect(&mut self, e: ComponentEffect, _: u32, _: u32) -> Result<()> {
        required(&format!("stored zone actual component effect {e:?}"))
    }
}
impl<E: Effects> FloodWorld for View<'_, '_, '_, E> {
    fn tile_flags(&mut self, t: usize) -> Result<u32> {
        self.effects.tile_flags(t)
    }
    fn tile_blocker(&mut self, t: usize) -> Result<u32> {
        let raw = self.effects.tile_primary_reference(t)?;
        Ok(primary_reference::resolve(raw, |raw| {
            self.fields.present(ActorId::from_raw(raw))
        }))
    }
    fn assert_blocked_start(&mut self) -> Result<bool> {
        required("stored zone actual blocked-start assertion")
    }
}
impl<E: Effects> ZoneObjectWorld for View<'_, '_, '_, E> {
    fn solver_present(&mut self, r: bool) -> Result<bool> {
        self.effects.solver_present(r)
    }
    fn fast_detection_zones(&mut self) -> Result<bool> {
        self.effects.fast_detection_zones()
    }
    fn presentation(&mut self, k: u32, x: u32, y: u32, r: u32, f: u16) -> Result<()> {
        self.effects.presentation(k, x, y, r, f)
    }
    fn log_implicit_clear(&mut self, _: u32, _: u16) -> Result<()> {
        required("stored zone actual implicit-clear diagnostic")
    }
    fn actor_id(&mut self, a: u32) -> Result<u32> {
        self.id(a).map(ActorId::raw)
    }
}
///Same zone allocation is mutably borrowed throughout solve and callbacks.
pub(crate) fn operate_slot(
    catalog: &Catalog<'_>,
    bank: &Bank,
    actors: &mut ActorStore,
    world: &mut World,
    key: ObjectKey,
    index: u32,
    movement: Option<([u32; 2], u32)>,
    inactive: Option<&crate::restore_lifecycle::InactiveLoad>,
    pc: Precision,
) -> Result<()> {
    let crate::load_world::SpatialOwner::Adopted(adoption) = &world.spatial else {
        return required("stored zone actual adopted spatial owner");
    };
    if adoption.error.is_some()
        || !adoption.pending.is_empty()
        || adoption.world.index().precision() != pc
    {
        return required("stored zone completed same index/live precision");
    }
    let spatial = &adoption.world;
    let phase = &world
        .dispatcher
        .as_ref()
        .ok_or_else(|| error("stored zone actual Dispatcher phase absent"))?
        .phase;
    let session = SessionLoan(catalog.load_session);
    let mut effects = world
        .zone_globals
        .bind_in_bank(
            catalog,
            bank,
            &world.zones,
            spatial.index(),
            phase,
            &world.presentation,
            &world.variables,
        )?
        .with_effect_state(&session, &world.mission_fields);
    let identities = &world.zone_identities;
    let kingdoms = &world.kingdoms;
    let memory_services=crate::load_zone_services::Context::with_kingdoms(
        &world.point,&session,kingdoms).with_source_fields(catalog,&world.source_fields);
    let map = &mut world.zones;
    let width = map.geometry.width;
    actors
        .with_zone_fields(key, index as usize, |fields, zone| {
            if identities.allocation(zone.key) != Some((fields.owner, index))
                || zone.actor != fields.owner.raw()
                || zone.flags != 1u16.wrapping_shl(index)
            {
                return Err(error("stored zone actual allocation/type/owner differs"));
            }
            let mut view = View {
                fields,
                catalog,
                bank,
                spatial,
                effects: &mut effects,
                inactive,
                kingdoms,
                memory_services: &memory_services,
                width,
            };
            if let Some((position, radius)) = movement {
                aa_zones::zone::move_to_position(map, &mut view, zone, position, radius, pc)
            } else {
                aa_zones::zone::release(map, &mut view, zone, pc)
            }
        })
        .map_err(error)?
}
