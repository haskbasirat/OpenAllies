//! Detection56EC90 on its same load component and Actor+148 zone owner.
//! Zone storage is borrowed directly through515F40/515970; missing native
//! world gates and outward callbacks are required services, never defaults.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{
    ActorId,
    components::{ComponentSlot, Components},
    loading::ActorShell,
};
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;
use aa_zones::{Error, Result, component::ActorZones, fpu, tile::ZoneMap, zone::Zone};
use std::collections::BTreeMap;

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
fn required<T>(name: &'static str) -> Result<T> {
    Err(Error::World(format!("required native service: {name}")))
}

/// Portable identities of actual zone allocations. This is a separate domain
/// from generation-bearing Actor IDs; no native pointer is guessed or cast.
#[derive(Debug, Default)]
pub struct Identities {
    allocations: BTreeMap<u32, (ActorId, u32)>,
    next: u32,
}
impl Identities {
    pub fn new() -> Self {
        Self::default()
    }
    fn allocate(&mut self, owner: ActorId, slot: u32) -> Result<u32> {
        if self.allocations.values().any(|v| *v == (owner, slot)) {
            return Err(error(
                "zone allocation identity already belongs to this Actor slot",
            ));
        }
        let key = self
            .next
            .checked_add(1)
            .ok_or_else(|| error("zone identity domain exhausted"))?;
        self.allocations.insert(key, (owner, slot));
        self.next = key;
        Ok(key)
    }
    pub fn allocation(&self, key: u32) -> Option<(ActorId, u32)> {
        self.allocations.get(&key).copied()
    }
    pub fn len(&self) -> usize {
        self.allocations.len()
    }
    pub fn is_empty(&self) -> bool {
        self.allocations.is_empty()
    }
}

/// These gates belong to the actual initialized World/presentation owners.
/// A caller must not turn a missing owner into an absent-pointer success.
pub trait Effects {
    fn fast_detection_zones(&mut self) -> Result<bool> {
        required("native ZoneBuilderFastDetectionZones SVar owner")
    }
    fn fog_enabled(&mut self) -> Result<bool> {
        required("native FogOfWar SVar owner")
    }
    fn solver_present(&mut self, _release: bool) -> Result<bool> {
        required("515A1A actual World+248 /514CE0 World+244 solver allocation owner")
    }
    fn presentation(
        &mut self,
        _owner: u32,
        _x: u32,
        _y: u32,
        _radius: u32,
        _flags: u16,
    ) -> Result<()> {
        required("50C4C0 actual World+D8/presentation+4/UI owner")
    }
}
pub struct RequiredEffects;
impl Effects for RequiredEffects {}

struct View<'a, E> {
    owner: ActorId,
    flags: &'a u32,
    components: &'a Components<RestoredComponent>,
    words: &'a BTreeMap<u32, u32>,
    registry: &'a aa_actors::IdentityRegistry<RestoredRecord>,
    effects: &'a mut E,
}
impl<E> View<'_, E> {
    fn prior(&self, actor: u32) -> Result<&ActorShell<RestoredComponent>> {
        if actor == self.owner.raw() {
            return required(
                "current Actor zone vector is exclusively leased to its native Zone call",
            );
        }
        let record = &self
            .registry
            .get(ActorId::from_raw(actor))
            .ok_or_else(|| error("zone Actor generation is not current"))?
            .value;
        let shell = match record {
            RestoredRecord::Actor(e) => &e.shell,
            RestoredRecord::Kingdom(e) => {
                &e.runtime
                    .as_ref()
                    .ok_or_else(|| error("zone Kingdom runtime absent"))?
                    .base
            }
            _ => return required("zone callback requires actual Actor constructor storage"),
        };
        shell
            .actor
            .as_ref()
            .ok_or_else(|| error("zone Actor allocation absent"))
    }
    fn word(&self, actor: u32, offset: u32) -> Result<u32> {
        let words = if actor == self.owner.raw() {
            self.words
        } else {
            &self.prior(actor)?.words
        };
        words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("zone actual Actor+{offset:X} is uninitialized")))
    }
    fn components(&self, actor: u32) -> Result<&Components<RestoredComponent>> {
        if actor == self.owner.raw() {
            Ok(self.components)
        } else {
            Ok(&self.prior(actor)?.components)
        }
    }
    fn kingdom_word(&self, key: u32, offset: u32) -> Result<u32> {
        let entry = self
            .registry
            .get(ActorId::from_raw(key))
            .ok_or_else(|| error("zone Kingdom generation is not current"))?;
        let RestoredRecord::Kingdom(e) = &entry.value else {
            return Err(error("zone owner is not an actual Kingdom"));
        };
        let runtime = e
            .runtime
            .as_ref()
            .ok_or_else(|| error("zone Kingdom runtime absent"))?;
        if runtime.definition.native_class != 4 || runtime.base.actor.is_none() {
            return Err(error("zone Kingdom constructor differs"));
        }
        runtime
            .derived
            .as_ref()
            .and_then(|d| d.words.get(&offset))
            .copied()
            .ok_or_else(|| error(format!("zone Kingdom+{offset:X} uninitialized")))
    }
}
impl<E: Effects> aa_zones::query::ZoneActors for View<'_, E> {
    fn owner(&mut self, a: u32) -> Result<u32> {
        self.word(a, 0xe8)
    }
    fn allied(&mut self, _a: u32, _k: u32) -> Result<bool> {
        required("zone Actor virtual138 actual relation owner")
    }
    fn enemy(&mut self, _a: u32, _k: u32) -> Result<bool> {
        required("zone Actor virtual140 actual relation owner")
    }
    fn zone_slot(&mut self, a: u32, i: u32) -> Result<u32> {
        Ok(self
            .prior(a)?
            .zones_148
            .zones()
            .slots
            .get(i as usize)
            .and_then(|z| z.as_ref())
            .map_or(0, |z| z.key))
    }
    fn word4(&mut self, _a: u32) -> Result<u32> {
        required("zone exclusion requires actual definition identity domain")
    }
    fn component_b0(&mut self, a: u32) -> Result<u32> {
        aa_zones::tile::ZoneWorld::component(self, a, 0xb0)
    }
    fn reveal_mask(&mut self, a: u32) -> Result<Option<u32>> {
        match self.components(a)?.get(ComponentSlot::Detection) {
            None => Ok(None),
            Some(RestoredComponent::Detection(d)) => Ok(Some(d.word_10)),
            _ => Err(error("zone Detection component type differs")),
        }
    }
}
impl<E: Effects> aa_zones::query::VisitWorld for View<'_, E> {
    fn kingdom(&mut self, k: u32) -> Result<aa_zones::query::Kingdom> {
        Ok(aa_zones::query::Kingdom {
            key: k,
            index: self.kingdom_word(k, 0x18c)?,
            nonmajor: self.kingdom_word(k, 0x190)?,
            shared_mask: self.kingdom_word(k, 0x228)?,
            supply_mask: self.kingdom_word(k, 0x22c)? as u16,
        })
    }
    fn kingdom_slot(&mut self, _i: u32) -> Result<u32> {
        required("zone actual World+154 Kingdom-vector borrow")
    }
}
impl<E: Effects> aa_zones::tile::ZoneWorld for View<'_, E> {
    fn fog_enabled(&mut self) -> Result<bool> {
        self.effects.fog_enabled()
    }
    fn sai_enabled(&mut self) -> Result<bool> {
        required("zone actual SAI settings+20 owner")
    }
    fn sai(&mut self, _h: aa_zones::tile::SaiHook, _t: usize, _k: u32) -> Result<()> {
        required("zone actual SAI region-counter owner")
    }
    fn kingdom_list(&mut self) -> Result<Vec<u32>> {
        required("zone actual World+154 Kingdom-vector borrow")
    }
    fn tile_actors(&mut self, _t: usize) -> Result<Vec<u32>> {
        required("zone same spatial-index actor traversal lease")
    }
    fn actor_reveal(&mut self, _a: u32, _k: u32, _e: bool, _v: bool) -> Result<()> {
        required("zone actual mutable Actor64 reveal services")
    }
    fn actor_hide(&mut self, _a: u32, _k: u32) -> Result<()> {
        required("zone actual mutable Actor68 hide services")
    }
    fn actor_unexplore(&mut self, _a: u32, _k: u32) -> Result<()> {
        required("zone actual mutable Actor6C unexplore services")
    }
    fn zone_effects_enabled(&mut self) -> Result<bool> {
        required("zone actual Session+E4 /World+48 owner")
    }
    fn actor_b4(&mut self, _a: u32) -> Result<bool> {
        required("zone actual Actor+B4 constructor dispatch")
    }
    fn actor_multi_tile(&mut self, _a: u32) -> Result<bool> {
        required("zone actual geometry/multi-tile source owner")
    }
    fn actor_tile(&mut self, _a: u32) -> Result<i64> {
        required("zone actual Actor tile/Map identity owner")
    }
    fn actor_flags(&mut self, a: u32) -> Result<u32> {
        if a == self.owner.raw() {
            return Ok(*self.flags);
        }
        match &self
            .registry
            .get(ActorId::from_raw(a))
            .ok_or_else(|| error("zone Actor generation is not current"))?
            .value
        {
            RestoredRecord::Actor(e) => Ok(e.shell.thing.flags),
            RestoredRecord::Kingdom(e) => Ok(e
                .runtime
                .as_ref()
                .ok_or_else(|| error("zone Kingdom runtime absent"))?
                .base
                .thing
                .flags),
            _ => required("zone actual Actor flags constructor"),
        }
    }
    fn actor_redirect(&mut self, _a: u32) -> Result<Option<u32>> {
        required("zone actual Denizen redirect owner")
    }
    fn component(&mut self, a: u32, offset: u32) -> Result<u32> {
        if offset == 0x100 {
            return Ok(self.components(a)?.state.low);
        }
        if offset == 0x138 {
            return self.word(a, offset);
        }
        let slot = aa_actors::components::ALL_SLOTS
            .iter()
            .copied()
            .find(|s| 0x60 + 4 * s.index() as u32 == offset)
            .ok_or_else(|| error("zone unknown component offset"))?;
        //Only native nullness is supplied; this is not a component pointer.
        Ok(u32::from(self.components(a)?.get(slot).is_some()))
    }
    fn component_effect(
        &mut self,
        _e: aa_zones::tile::ComponentEffect,
        _c: u32,
        _a: u32,
    ) -> Result<()> {
        required("zone actual component effect owner")
    }
}
impl<E: Effects> aa_zones::flood::FloodWorld for View<'_, E> {
    fn tile_flags(&mut self, _t: usize) -> Result<u32> {
        required("zone same spatial-index tile flags owner")
    }
    fn tile_blocker(&mut self, _t: usize) -> Result<u32> {
        required("zone same spatial-index blocker/registry owner")
    }
    fn assert_blocked_start(&mut self) -> Result<bool> {
        required("zone native assertion diagnostic owner")
    }
}
impl<E: Effects> aa_zones::zone::ZoneObjectWorld for View<'_, E> {
    fn solver_present(&mut self, r: bool) -> Result<bool> {
        self.effects.solver_present(r)
    }
    fn fast_detection_zones(&mut self) -> Result<bool> {
        self.effects.fast_detection_zones()
    }
    fn presentation(&mut self, k: u32, x: u32, y: u32, r: u32, f: u16) -> Result<()> {
        self.effects.presentation(k, x, y, r, f)
    }
    fn log_implicit_clear(&mut self, _a: u32, _f: u16) -> Result<()> {
        required("zone native diagnostic logging owner")
    }
    fn actor_id(&mut self, a: u32) -> Result<u32> {
        if a == self.owner.raw() {
            Ok(a)
        } else {
            self.prior(a).map(|_| a)
        }
    }
}

/// Actual first CreateZone success owns both the vector and Zone value before
/// later move/solver errors. There is no retained copy of the component/pose.
pub fn after_init(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    map: &mut ZoneMap,
    identities: &mut Identities,
    pc: Precision,
    effects: &mut impl Effects,
) -> aa_actors::actor_creation::Result<()> {
    super::load_detection_init_world::checked(catalog, record, ComponentSlot::Detection)?;
    let actor = record.shell.actor.as_ref().unwrap();
    let word138 = *actor.words.get(&0x138).ok_or_else(|| {
        aa_actors::actor_creation::Error::World("56EC90 actual Actor138 absent".into())
    })?;
    let structure = actor.components.state.low & 0x20_0000 != 0;
    if structure && word138 != 0 {
        return Ok(());
    }
    let source = crate::live_object_init::Source::bind(catalog, record.definition)?;
    let radius = if structure {
        catalog
            .actors
            .native_ground_definition(&record.definition.proof.id, source.source_precision())
            .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))?
            .detection
            .ok_or_else(|| {
                aa_actors::actor_creation::Error::World(
                    "56EC90 actual Detection source absent".into(),
                )
            })?
            .finalized_remote_range_bits
    } else {
        if word138 != 0 {
            return Err(aa_actors::actor_creation::Error::Unsupported(
                "56EC90 Actor138 path requires actual source430/Denizen/scheduling services",
            ));
        }
        let Some(RestoredComponent::Detection(d)) = actor.components.get(ComponentSlot::Detection)
        else {
            unreachable!()
        };
        d.radius.0
    };
    let position = record.shell.thing.position_bits;
    let flags = &record.shell.thing.flags;
    let actor = record.shell.actor.as_mut().unwrap();
    let enabled = structure
        || (*flags & 1 == 0
            && fpu::less(
                &fpu::load(0)
                    .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))?,
                &fpu::load(radius)
                    .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))?,
            ));
    zone_change(
        record.owner,
        actor,
        flags,
        position,
        radius,
        enabled,
        map,
        identities,
        pc,
        &*record.registry,
        effects,
    )
    .map_err(|e| aa_actors::actor_creation::Error::World(e.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn zone_change(
    owner: ActorId,
    actor: &mut ActorShell<RestoredComponent>,
    flags: &u32,
    position: [u32; 3],
    radius: u32,
    enabled: bool,
    map: &mut ZoneMap,
    identities: &mut Identities,
    pc: Precision,
    registry: &aa_actors::IdentityRegistry<RestoredRecord>,
    effects: &mut impl Effects,
) -> Result<()> {
    use aa_zones::{fpu, zone};
    let mut world = View {
        owner,
        flags,
        components: &actor.components,
        words: &actor.words,
        registry,
        effects,
    };
    let zones: &mut ActorZones = actor.zones_148.zones_mut();
    if enabled {
        if zones.slots.is_empty() {
            zones.slots.try_reserve_exact(9).map_err(error)?;
            zones.slots.resize(9, None);
        }
        if zones.slots.len() <= 6 {
            return Err(error("native CreateZone6 vector overrun"));
        }
        if zones.slots[6].is_none() {
            let key = identities.allocate(owner, 6)?;
            zones.slots[6] = Some(Zone::new(key, owner.raw(), 0x40));
        }
        let zone = zones.slots[6].as_mut().unwrap();
        if identities.allocation(zone.key) != Some((owner, 6))
            || zone.actor != owner.raw()
            || zone.flags != 0x40
        {
            return Err(error(
                "Detection Zone6 allocation identity/owner/type differs",
            ));
        }
        if zone.active() && fpu::equal(&fpu::load(zone.radius_bits)?, &fpu::load(radius)?) {
            return Ok(());
        }
        //Borrow the actual stored Zone through all callbacks; no copy is
        //temporarily removed and no stale zone is readable through View.
        zone::move_to_position(
            map,
            &mut world,
            zone,
            [position[0], position[1]],
            radius,
            pc,
        )
    } else if let Some(Some(zone)) = zones.slots.get_mut(6) {
        zone::release(map, &mut world, zone, pc)
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "load_detection_activation_tests.rs"]
mod tests;
