//! DetectionComponent scheduled Update on the live retained World.
//!
//! Detection+8 is its Updatable (vtable A3C540). Queue1 periodic 50B330 calls
//! +14 = 56F140(elapsed, data); queue3 deferred 4F05C0 calls +18 = 56E9F0,
//! which only calls this same Updatable's +14 with 0.0001 and its data. The
//! body is aa_zones::component::tick (dead actor; structure/Actor+138 ZOD;
//! ZOD 56EB40(6, 56EA30, +14), reveal mask 56EFC0 with 56EDC0 sources, RZOD
//! 56EB40(1, 56EC10, ActorData+1D0)).
//!
//! The ticking actor's zone vector (Actor+148) is lent to the kernel for the
//! call and stored back before return; its Detection+10 reveal mask is the
//! only component field the kernel writes. Every other actor is read from the
//! same ActorStore. Reached leaves without a live binding (zone creation,
//! solver reveal/hide callbacks, SAI hooks, positive engagement/defender/
//! denizen reveal sources, attached-actor parents) return explicit errors.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Source},
    catalog::Catalog,
    definitions::Bank,
    load_detection_activation::Effects,
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_numeric::Precision;
use aa_simulation::NativeClock;
use aa_world::queues::{Delivery, Error as QueueError};
use aa_zones::{
    Error, Result,
    component::{ActorZones, DetectionComponent, DetectionComponentWorld},
    flood::FloodWorld,
    query::{Kingdom, VisitWorld, ZoneActors},
    tile::{ComponentEffect, SaiHook, ZoneWorld},
    zone::ZoneObjectWorld,
};

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(format!("live Detection 56F140: {e}"))
}
fn required<T>(leaf: &str) -> Result<T> {
    Err(error(format!("required native {leaf} is not bound")))
}

struct SessionLoan<'a>(Option<&'a dyn crate::load_world_globals::SessionPhase>);
impl crate::load_world_globals::SessionPhase for SessionLoan<'_> {
    fn current_phase(&self) -> std::result::Result<u32, String> {
        self.0.ok_or("live zone requires actual Session+F8 owner")?.current_phase()
    }
    fn application_state(&self) -> std::result::Result<u32, String> {
        self.0.ok_or("live zone requires actual Session+E4 owner")?.application_state()
    }
}

struct View<'a, 'source, E> {
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    actors: &'a ActorStore,
    spatial: &'a SpatialWorld,
    variables: &'a crate::load_view_variables::State,
    kingdoms: &'a crate::load_kingdom_init_world::KingdomRegistryState,
    source_fields: &'a aa_rules::native_source_requirements::NativeSourceRequirements,
    effects: &'a mut E,
    width: i32,
    ticking: ActorId,
    zones: ActorZones,
    detection: DetectionComponent,
}
impl<E: Effects> View<'_, '_, E> {
    fn key(&self, raw: u32) -> Result<ObjectKey> {
        self.actors
            .key(ActorId::from_raw(raw))
            .ok_or_else(|| error(format!("{raw:#x} has no live GTID")))
    }
    fn object(&self, raw: u32) -> Result<&crate::actor_world::RuntimeObject> {
        self.actors.object(self.key(raw)?).map_err(error)
    }
    fn word(&self, raw: u32, offset: u32) -> Result<u32> {
        self.object(raw)?
            .actor
            .as_ref()
            .ok_or_else(|| error("Actor shell absent"))?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("Actor+{offset:X} is uninitialized")))
    }
    fn component_of(&self, raw: u32, slot: Slot) -> Result<Option<&RuntimeComponent>> {
        Ok(self.object(raw)?.components().map_err(error)?.get(slot))
    }
    fn pose(&self, raw: u32) -> Result<crate::actor_world::PlacementState> {
        self.actors.placement(self.key(raw)?, self.spatial).map_err(error)
    }
    fn special(&self, raw: u32) -> Result<&aa_mapworld::special_state::SpecialShell> {
        let key = self.key(raw)?;
        let Source::Special(s) = &self.actors.entity(key).map_err(error)?.source else {
            return required("special Kingdom/Team derived owner");
        };
        s.derived.as_ref().ok_or_else(|| error("special derived fields absent"))
    }
    fn kingdom_word(&self, raw: u32, offset: u32) -> Result<u32> {
        self.special(raw)?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("Kingdom+{offset:X} uninitialized")))
    }
    fn class(&self, raw: u32) -> Result<u32> {
        Ok(self.object(raw)?.definition.native_class)
    }
    fn related_code(&self, actor: u32, other: u32) -> Result<u32> {
        let other = (other != 0).then_some(ActorId::from_raw(other));
        match self.class(actor)? {
            3 => {
                let owner = self.word(actor, 0xe8)?;
                crate::relations::borrowed::actor(self, (owner != 0).then_some(ActorId::from_raw(owner)), other)
            }
            4 => crate::relations::borrowed::kingdom(self, ActorId::from_raw(actor), other),
            _ => required("virtual +144 relation for a non-Actor/Kingdom class"),
        }
    }
    /// 4CED60 (GActor virtual +100): Structure+10, else the Wall chain.
    fn parent(&self, raw: u32) -> Result<u32> {
        if self.class(raw)? != 3 {
            return required("virtual +100 for a non-Actor class");
        }
        match self.component_of(raw, Slot::Structure)? {
            Some(RuntimeComponent::Structure(v)) if v.object_handle_10 != 0 => {
                return required("4CED60 Structure+10 parent object binding");
            }
            Some(RuntimeComponent::Structure(_)) | None => {}
            Some(_) => return Err(error("Structure slot payload mismatch")),
        }
        if self.object(raw)?.components().map_err(error)?.present(Slot::Wall) {
            return required("4CED60 Wall+90 58CB30 parent chain");
        }
        Ok(0)
    }
    /// ActorData+1CC/+1D0 finalized ranges of the actor's canonical source,
    /// materialized at the same Bank source precision as its other fields.
    fn ground(&self, raw: u32) -> Result<aa_rules::ground::DetectionDefinition> {
        let definition = &self.object(raw)?.definition;
        let source = crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, definition)
            .map_err(error)?;
        self.catalog
            .actors
            .native_ground_definition(&definition.proof.id, source.source_precision())
            .map_err(error)?
            .detection
            .ok_or_else(|| error("actual Detection source definition absent"))
    }
}
impl<E: Effects> crate::relations::borrowed::Read for View<'_, '_, E> {
    type Id = ActorId;
    type Error = Error;
    fn kingdom_team(&self, id: ActorId) -> Result<Option<ActorId>> {
        let s = self.special(id.raw())?;
        match s.identity_references.get(&0x1ec) {
            Some(v) => Ok(*v),
            None if s.words.get(&0x1ec) == Some(&0) => Ok(None),
            _ => Err(error("Kingdom+1EC Team reference unbound")),
        }
    }
    fn kingdom_slot(&self, id: ActorId) -> Result<u32> {
        self.kingdom_word(id.raw(), 0x188)
    }
    fn team_slot(&self, id: ActorId) -> Result<u32> {
        self.special(id.raw())?
            .words
            .get(&0x38)
            .copied()
            .ok_or_else(|| error("Team+38 absent"))
    }
    fn kingdom_word(&self, id: ActorId, team: bool, slot: u32) -> Result<u32> {
        self.special(id.raw())?
            .vectors
            .get(&if team { 0x1dc } else { 0x1d0 })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| error("Kingdom relation vector slot absent"))
    }
    fn team_word(&self, id: ActorId, team: bool, slot: u32) -> Result<u32> {
        self.special(id.raw())?
            .vectors
            .get(&if team { 0x58 } else { 0x4c })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| error("Team relation vector slot absent"))
    }
    fn orphan_relation(&self) -> Result<u32> {
        required("global B28158 orphan relation")
    }
    fn team_null_relation(&self) -> Result<u32> {
        required("global B2815C team-null relation")
    }
}
impl<E: Effects> ZoneActors for View<'_, '_, E> {
    fn owner(&mut self, a: u32) -> Result<u32> {
        self.word(a, 0xe8)
    }
    fn allied(&mut self, a: u32, k: u32) -> Result<bool> {
        let same = match self.class(a)? {
            3 => self.word(a, 0xe8)? == k,
            4 => a == k,
            _ => return required("virtual +138 for a non-Actor/Kingdom class"),
        };
        Ok(same || self.related_code(a, k)? == 1)
    }
    fn enemy(&mut self, a: u32, k: u32) -> Result<bool> {
        Ok(self.related_code(a, k)? == 3)
    }
    fn zone_slot(&mut self, a: u32, i: u32) -> Result<u32> {
        let slots = if a == self.ticking.raw() {
            &self.zones.slots
        } else {
            &self
                .object(a)?
                .actor
                .as_ref()
                .ok_or_else(|| error("Actor shell absent"))?
                .zones_148
                .zones()
                .slots
        };
        Ok(slots.get(i as usize).copied().flatten().map_or(0, |z| z.key))
    }
    fn word4(&mut self, _: u32) -> Result<u32> {
        required("definition exclusion identity (actor+4)")
    }
    fn component_b0(&mut self, a: u32) -> Result<u32> {
        ZoneWorld::component(self, a, 0xb0)
    }
    fn reveal_mask(&mut self, a: u32) -> Result<Option<u32>> {
        if a == self.ticking.raw() {
            return Ok(Some(self.detection.reveal_mask));
        }
        match self.component_of(a, Slot::Detection)? {
            None => Ok(None),
            Some(RuntimeComponent::Detection(d)) => Ok(Some(d.word_10)),
            Some(_) => Err(error("Detection slot payload mismatch")),
        }
    }
}
impl<E: Effects> VisitWorld for View<'_, '_, E> {
    fn kingdom(&mut self, k: u32) -> Result<Kingdom> {
        Ok(Kingdom {
            key: k,
            index: self.kingdom_word(k, 0x18c)?,
            nonmajor: self.kingdom_word(k, 0x190)?,
            shared_mask: self.kingdom_word(k, 0x228)?,
            supply_mask: crate::load_detection_activation::kingdom_supply_mask(self.special(k)?)?,
        })
    }
    fn kingdom_slot(&mut self, i: u32) -> Result<u32> {
        self.kingdoms
            .majors()
            .get(i as usize)
            .map(|v| v.map_or(0, ActorId::raw))
            .ok_or_else(|| error("World+154 slot absent"))
    }
}
impl<E: Effects> ZoneWorld for View<'_, '_, E> {
    fn fog_enabled(&mut self) -> Result<bool> {
        self.effects.fog_enabled()
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        required("[B36528]+20 SAI ready byte (application SAI owner)")
    }
    fn sai(&mut self, _: SaiHook, _: usize, _: u32) -> Result<()> {
        required("SAI region-counter hook")
    }
    fn kingdom_list(&mut self) -> Result<Vec<u32>> {
        Ok(self.kingdoms.majors().iter().map(|v| v.map_or(0, ActorId::raw)).collect())
    }
    fn tile_actors(&mut self, t: usize) -> Result<Vec<u32>> {
        self.effects.tile_actors(t)
    }
    fn actor_reveal(&mut self, _: u32, _: u32, _: bool, _: bool) -> Result<()> {
        required("Actor virtual +64 reveal during a live Detection tick")
    }
    fn actor_hide(&mut self, _: u32, _: u32) -> Result<()> {
        required("Actor virtual +68 hide")
    }
    fn actor_unexplore(&mut self, _: u32, _: u32) -> Result<()> {
        required("Actor virtual +6C unexplore")
    }
    fn zone_effects_enabled(&mut self) -> Result<bool> {
        self.effects.zone_effects_enabled()
    }
    fn actor_b4(&mut self, a: u32) -> Result<bool> {
        Ok(matches!(self.class(a)?, 3 | 4))
    }
    fn actor_multi_tile(&mut self, a: u32) -> Result<bool> {
        let flags = self.pose(a)?.flags;
        if flags & 0x2000_0400 != 0x2000_0400 {
            return Ok(false);
        }
        let d = &self.object(a)?.definition;
        let s = crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, d).map_err(error)?;
        aa_spatial::geometry::marks_blocker(
            flags,
            s.blocking_shape().radius_bits,
            self.spatial.index().grid().cell_size_bits,
            self.spatial.index().precision(),
        )
        .map_err(error)
    }
    fn actor_tile(&mut self, a: u32) -> Result<i64> {
        let tile = self
            .pose(a)?
            .cached_tile
            .ok_or_else(|| error("actual cached tile absent"))?;
        Ok(i64::from(self.width) * i64::from(tile[1]) + i64::from(tile[0]))
    }
    fn actor_flags(&mut self, a: u32) -> Result<u32> {
        Ok(self.pose(a)?.flags)
    }
    fn actor_redirect(&mut self, _: u32) -> Result<Option<u32>> {
        required("Actor+80 zone-effect redirect")
    }
    fn component(&mut self, a: u32, offset: u32) -> Result<u32> {
        if offset == 0x100 {
            return Ok(self.object(a)?.components().map_err(error)?.state.low);
        }
        if offset == 0x138 {
            return self.word(a, offset);
        }
        let slot = aa_actors::components::ALL_SLOTS
            .iter()
            .copied()
            .find(|s| 0x60 + 4 * s.index() as u32 == offset)
            .ok_or_else(|| error(format!("unsupported component offset {offset:X}")))?;
        Ok(u32::from(self.component_of(a, slot)?.is_some()))
    }
    fn component_effect(&mut self, e: ComponentEffect, _: u32, _: u32) -> Result<()> {
        required(&format!("component zone effect {e:?}"))
    }
}
impl<E: Effects> FloodWorld for View<'_, '_, E> {
    fn tile_flags(&mut self, t: usize) -> Result<u32> {
        self.effects.tile_flags(t)
    }
    fn tile_blocker(&mut self, t: usize) -> Result<u32> {
        let raw = self.effects.tile_primary_reference(t)?;
        Ok(if self.actors.key(ActorId::from_raw(raw)).is_some() { raw } else { 0 })
    }
    fn assert_blocked_start(&mut self) -> Result<bool> {
        required("855D30 blocked-start assertion")
    }
}
impl<E: Effects> ZoneObjectWorld for View<'_, '_, E> {
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
        required("404F60 implicit zone-clear diagnostic")
    }
    fn actor_id(&mut self, a: u32) -> Result<u32> {
        self.key(a).map(|_| a)
    }
}
impl<E: Effects> DetectionComponentWorld for View<'_, '_, E> {
    fn detection_component(&mut self, actor: u32) -> Result<&mut DetectionComponent> {
        if actor != self.ticking.raw() {
            return required("another actor's Detection component during this tick");
        }
        Ok(&mut self.detection)
    }
    fn zones(&mut self, actor: u32) -> Result<&mut ActorZones> {
        if actor != self.ticking.raw() {
            return required("another actor's zone vector during this tick");
        }
        Ok(&mut self.zones)
    }
    fn new_zone_key(&mut self, _: u32, _: u32) -> Result<u32> {
        required("4D69A0 CreateZone 6F7810 allocation identity")
    }
    fn log_unexpected_zone(&mut self, _: u32, _: u32) -> Result<()> {
        required("404F60 unexpected-zone diagnostic")
    }
    fn actor_position(&mut self, a: u32) -> Result<[u32; 2]> {
        let p = self.pose(a)?;
        Ok([p.position_bits[0], p.position_bits[1]])
    }
    fn actor_virtual_100(&mut self, a: u32) -> Result<u32> {
        self.parent(a)
    }
    fn actor_virtual_158(&mut self, a: u32) -> Result<bool> {
        match self.component_of(a, Slot::Structure)? {
            None => Ok(false),
            Some(RuntimeComponent::Structure(v)) if v.object_handle_10 == 0 => Ok(false),
            Some(RuntimeComponent::Structure(_)) => required("588990 Structure+10 parent Settlement+14"),
            Some(_) => Err(error("Structure slot payload mismatch")),
        }
    }
    fn actor_80(&mut self, a: u32) -> Result<(u32, u32, u8)> {
        match self.component_of(a, Slot::Element)? {
            None => Ok((0, 0, 0)),
            Some(RuntimeComponent::Element(e)) => Ok((
                1,
                e.organization.map_or(0, ActorId::raw),
                u8::from(e.flag_1c),
            )),
            Some(_) => Err(error("Element slot payload mismatch")),
        }
    }
    fn data_word(&mut self, a: u32, offset: u32) -> Result<u32> {
        match offset {
            0x1cc => Ok(self.ground(a)?.finalized_range_bits),
            0x1d0 => Ok(self.ground(a)?.finalized_remote_range_bits),
            0x430 => {
                let id = self.object(a)?.definition.proof.id.clone();
                self.source_fields.word(&id, 0x430).map_err(error)
            }
            _ => required(&format!("ActorData+{offset:X}")),
        }
    }
    fn modified_detection_range(&mut self, _: u32, _: u32) -> Result<u32> {
        required("45D790 modified detection range")
    }
    /// 56EDC0.
    fn reveal_sources(&mut self, a: u32) -> Result<u32> {
        if self.word(a, 0x118)? != 0 || self.word(a, 0x130)? != 0 {
            return required("56EDC0 positive Actor+118/+130 engagement lists");
        }
        let reverse = {
            let ordinal = self
                .variables
                .registrations()
                .find(|r| r.name() == "EngagementShowReverseRZOD")
                .map(|r| r.ordinal())
                .ok_or_else(|| error("SVar EngagementShowReverseRZOD is not registered"))?;
            match self.variables.current(ordinal as usize).map_err(error)? {
                crate::load_view_variables::CurrentValue::Bool(v) => *v != 0,
                _ => return Err(error("EngagementShowReverseRZOD is not the Bool kind")),
            }
        };
        let mut mask = 0u32;
        if reverse {
            if let Some(mind) = self.component_of(a, Slot::Mind)? {
                let RuntimeComponent::Mind(m) = mind else {
                    return Err(error("Mind slot payload mismatch"));
                };
                //Element Mind virtual +90 = 535F10: 432E40 on Mind+13C.
                if m.kind != aa_mind::construct::Kind::Element {
                    return required("56EE9E virtual +90 of a non-Element Mind");
                }
                let target = m
                    .words
                    .get(&0x13c)
                    .copied()
                    .ok_or_else(|| error("Mind+13C is uninitialized"))?;
                if target != 0 && self.actors.key(ActorId::from_raw(target)).is_some() {
                    if self.class(target)? != 3 {
                        return required("56EEAC virtual +108 of a non-Actor engage target");
                    }
                    let owner = self.word(target, 0xe8)?;
                    if owner != 0 && self.kingdom_word(owner, 0x190)? == 0 {
                        mask |= 1u32.wrapping_shl(self.kingdom_word(owner, 0x18c)?);
                    }
                }
            }
        }
        match self.component_of(a, Slot::Defender)? {
            None => {}
            Some(RuntimeComponent::Defender(d)) if d.entries.is_empty() => {}
            Some(RuntimeComponent::Defender(_)) => {
                return required("56EEDC positive Defender+24 entries");
            }
            Some(_) => return Err(error("Defender slot payload mismatch")),
        }
        if self.actor_virtual_158(a)? && self.component_of(a, Slot::Denizen)?.is_some() {
            return required("569300 Denizen threats");
        }
        Ok(mask)
    }
    fn organization_slots(&mut self, a: u32) -> Result<Option<Vec<u32>>> {
        match self.component_of(a, Slot::Organization)? {
            None => Ok(None),
            Some(RuntimeComponent::Organization(o)) => {
                Ok(Some(o.members.iter().map(|m| m.raw()).collect()))
            }
            Some(_) => Err(error("Organization slot payload mismatch")),
        }
    }
    fn kingdom_enemy(&mut self, kingdom: u32, owner: u32) -> Result<bool> {
        //Kingdom virtual +140 = 4E52A0: virtual +144 (4E52D0) == 3.
        if self.class(kingdom)? != 4 {
            return required("Kingdom virtual +140 on a non-Kingdom");
        }
        Ok(self.related_code(kingdom, owner)? == 3)
    }
    fn kingdom_reveal_byte(&mut self, kingdom: u32) -> Result<Option<u8>> {
        //4CF110: Properties(+78)+1C, the ModifierState; its first byte.
        match self.component_of(kingdom, Slot::Properties)? {
            None => Ok(None),
            Some(RuntimeComponent::Properties(p)) => Ok(Some(p.modifier.flags as u8)),
            Some(_) => Err(error("Properties slot payload mismatch")),
        }
    }
    fn schedule_tick(&mut self, _: u32, _: u32) -> Result<()> {
        required("50ACC0 Detection tick scheduling from 56EC90")
    }
}

pub(crate) struct Context<'a, 'source> {
    pub catalog: &'a Catalog<'source>,
    pub bank: &'a Bank,
    pub actors: &'a mut ActorStore,
    pub world: &'a mut crate::load_world::World,
    pub clock: &'a NativeClock,
    pub phase: u32,
    pub precision: Precision,
}

/// One delivery to Detection+8: queue1 +14 (56F140) or queue3 +18 (56E9F0,
/// which forwards to its own +14 with 0x38D1B717 and the same data).
pub(crate) fn run(key: ObjectKey, event: Delivery, cx: &mut Context<'_, '_>) -> aa_world::queues::Result<()> {
    let fail = |e: Error| QueueError::Callback(e.to_string());
    if event.queue == 4 {
        return Err(QueueError::Callback("live Detection: +1C is 943800 RET4; queue4 delivery is invalid".into()));
    }
    let _ = cx.clock;
    let id = cx.actors.id(key).ok_or_else(|| fail(error("owner has no live GTID")))?;
    let detection = match cx.actors.component(id, Slot::Detection).map_err(|e| fail(error(e)))? {
        Some(RuntimeComponent::Detection(d)) if d.header.owner == id => DetectionComponent {
            actor: id.raw(),
            reveal_mask: d.word_10,
            zod_radius_bits: d.radius.0,
        },
        _ => return Err(fail(error("scheduled Detection allocation/owner differs"))),
    };
    let world = &mut *cx.world;
    let crate::load_world::SpatialOwner::Adopted(adoption) = &world.spatial else {
        return Err(fail(error("adopted spatial owner absent")));
    };
    if adoption.error.is_some() || !adoption.pending.is_empty() || adoption.world.index().precision() != cx.precision {
        return Err(fail(error("completed same-precision spatial index required")));
    }
    let spatial = &adoption.world;
    let session = SessionLoan(cx.catalog.load_session);
    let phase = cx.phase;
    let mut effects = world
        .zone_globals
        .bind_in_bank(
            cx.catalog,
            cx.bank,
            &world.zones,
            spatial.index(),
            &phase,
            &world.presentation,
            &world.variables,
        )
        .map_err(|e| fail(error(e)))?
        .with_effect_state(&session, &world.mission_fields);
    //Lend Actor+148 for the call; restored below on every exit.
    let zones = {
        let actor = cx
            .actors
            .object_mut(key)
            .map_err(|e| fail(error(e)))?
            .actor
            .as_mut()
            .ok_or_else(|| fail(error("Actor shell absent")))?;
        std::mem::take(actor.zones_148.zones_mut())
    };
    let width = world.zones.geometry.width;
    let mut view = View {
        catalog: cx.catalog,
        bank: cx.bank,
        actors: &*cx.actors,
        spatial,
        variables: &world.variables,
        kingdoms: &world.kingdoms,
        source_fields: &world.source_fields,
        effects: &mut effects,
        width,
        ticking: id,
        zones,
        detection,
    };
    let result = aa_zones::component::tick(&mut world.zones, &mut view, id.raw(), cx.precision);
    let View { zones, detection, .. } = view;
    let actor = cx
        .actors
        .object_mut(key)
        .map_err(|e| fail(error(e)))?
        .actor
        .as_mut()
        .ok_or_else(|| fail(error("Actor shell absent")))?;
    *actor.zones_148.zones_mut() = zones;
    match cx.actors.component_mut(id, Slot::Detection).map_err(|e| fail(error(e)))? {
        Some(RuntimeComponent::Detection(d)) => d.word_10 = detection.reveal_mask,
        _ => return Err(fail(error("Detection disappeared during 56F140"))),
    }
    result.map_err(fail)
}
