//! Area4DB200, Camera4DBCF0 and DynamicDoodad4DC730 saved Init(NULL).
//! Actual held derived payload, current Object service and persistent World
//! counters/queues are borrowed; no pose/component/queue copyback exists.
use crate::{catalog::Catalog, live_object_init::Record, load_object_init_world};
use aa_actors::{
    actor_creation::{Error, Result},
    loading::LoadKind,
};
use aa_mapworld::{
    dynamic_doodad::{Component, DynamicDoodad},
    map_objects::{Area, Camera},
    restore::RestoredRecord,
    restore_components::UpdatableTarget,
};
use aa_numeric::{Binary, Precision};
use aa_simulation::NativeRng;
use serde::Serialize;

#[cfg(test)]
#[path = "load_special_object_init_tests.rs"]
mod tests;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

/// Mutation generations beside the native camera/area lists, not list lengths.
///4F7F15/+2D initialize them to zero. Direction/shape edits use the same owner.
#[derive(Debug, Serialize)]
pub struct State {
    camera_generation: u32,
    area_generation: u32,
}
impl State {
    pub const fn from_world_constructor() -> Self {
        Self {
            camera_generation: 0,
            area_generation: 0,
        }
    }
    pub fn camera_generation(&self) -> u32 {
        self.camera_generation
    }
    pub fn area_generation(&self) -> u32 {
        self.area_generation
    }
    ///4F0340, also used by actual Camera direction and destructor services.
    pub fn camera_changed(&mut self) {
        self.camera_generation = self.camera_generation.wrapping_add(1);
    }
    ///4F0350, also used by actual Area shape and destructor services.
    pub fn area_changed(&mut self) {
        self.area_generation = self.area_generation.wrapping_add(1);
    }
}

/// Actual separately owned derived allocation of the same held saved record.
pub enum Derived<'a> {
    Area(&'a mut Area),
    Camera(&'a mut Camera),
    Doodad(&'a mut DynamicDoodad),
}
impl Derived<'_> {
    fn class(&self) -> u32 {
        match self {
            Self::Area(_) => 6,
            Self::Camera(_) => 7,
            Self::Doodad(_) => 9,
        }
    }
}

///5904A0 never spills the RNG fraction before multiplying the component period.
///The period bits were read before this draw; unsupported numeric inputs still
///retain the mandatory RNG prefix, matching the reached arithmetic boundary.
fn delay(rng: &mut NativeRng, period_bits: u32, pc: Precision) -> Result<u32> {
    let draw = rng.next_u32();
    let signed = Binary::from_i32(draw as i32);
    let value = if draw & 0x8000_0000 != 0 {
        signed
            .add(
                &Binary::from_f64_bits(0x41f0_0000_0000_0000).map_err(error)?,
                pc,
            )
            .map_err(error)?
    } else {
        signed
    };
    value
        .mul(
            &Binary::from_f64_bits(0x3df0_0000_0010_0000).map_err(error)?,
            pc,
        )
        .map_err(error)?
        .mul(&Binary::from_f32_bits(period_bits).map_err(error)?, pc)
        .map_err(error)?
        .store_f32()
        .map_err(error)
}

trait TailServices {
    fn period(&mut self, slot: u8, component: &Component) -> Result<u32>;
    fn schedule(&mut self, slot: u8, component: &mut Component, delay_bits: u32) -> Result<()>;
    fn clear(&mut self, slot: u8, component: &mut Component) -> Result<()>;
    fn legacy_area(&mut self, area: &mut Area) -> Result<()>;
}

/// Native tail only; `initialize` calls it only after the real Object succeeds.
fn tail(
    derived: Derived<'_>,
    state: &mut State,
    version: &u32,
    rng: &mut NativeRng,
    pc: Precision,
    services: &mut impl TailServices,
) -> Result<bool> {
    match derived {
        Derived::Area(area) => {
            state.area_changed(); // before the version/head guard and any error.
            if *version < 22 && area.scheduled_head.is_none() {
                services.legacy_area(area)?;
            }
        }
        Derived::Camera(_) => state.camera_changed(),
        Derived::Doodad(doodad) => {
            for slot in 0..2u8 {
                //4DC750 rereads the actual current slot at each iteration.
                let Some(component) = doodad.components[usize::from(slot)].as_mut() else {
                    continue;
                };
                let period = services.period(slot, component)?; //5904B5 before85D840.
                let delay_bits = delay(rng, period, pc)?;
                services.schedule(slot, component, delay_bits)?;
                //58F960 rereads state only after5904A0 completed scheduling.
                if slot == 1
                    && matches!(
                        *component
                            .words
                            .first()
                            .ok_or(Error::Unsupported("58F960 current Breakable+18 storage"))?,
                        0 | 1
                    )
                {
                    services.clear(slot, component)?;
                }
            }
        }
    }
    Ok(true)
}

struct Owned<'a, 'assets> {
    catalog: &'a Catalog<'assets>,
    owner: aa_actors::ActorId,
    canonical: &'a str,
    live_pc: Precision,
}
impl Owned<'_, '_> {
    fn target(&self, slot: u8) -> UpdatableTarget {
        UpdatableTarget::DynamicDoodad {
            owner: self.owner.raw(),
            slot,
        }
    }
    fn check(&self, slot: u8, component: &Component) -> Result<()> {
        if component.owner != self.owner.raw()
            || component.slot != slot
            || !component
                .definition_owner
                .eq_ignore_ascii_case(self.canonical)
        {
            return Err(error(
                "Doodad Init current component owner/source/slot differs",
            ));
        }
        Ok(())
    }
}
impl TailServices for Owned<'_, '_> {
    fn period(&mut self, slot: u8, component: &Component) -> Result<u32> {
        self.check(slot, component)?;
        self.catalog
            .require_untranslated_actor_identity(self.canonical)
            .map_err(error)?;
        let source_pc = {
            let bank = self.catalog.definition_bank.try_borrow().map_err(error)?;
            bank.combat_source().map_err(error)?.source_precision()
        };
        let data = self
            .catalog
            .actors
            .native_dynamic_doodad(self.canonical, source_pc)
            .map_err(error)?;
        let definition = data
            .components
            .get(usize::from(slot))
            .and_then(|v| v.as_ref())
            .ok_or(Error::Unsupported(
                "5904A0 captured Doodad component definition+8",
            ))?;
        if definition.kind != u32::from(slot) {
            return Err(error("Doodad source component vtable kind differs"));
        }
        definition
            .fields
            .get("update_time")
            .ok_or(Error::Unsupported("5904B5 actual component definition+10"))?
            .fixed_bits()
            .map_err(error)
    }
    fn schedule(&mut self, slot: u8, component: &mut Component, delay_bits: u32) -> Result<()> {
        self.check(slot, component)?;
        // Embedded Updatable+C is6A71D0(true) for both real component vtables.
        let mut queues = self.catalog.load_queues.try_borrow_mut().map_err(error)?;
        let queues = queues.as_deref_mut().ok_or(Error::Unsupported(
            "50ACC0 actual load GWorld+1E8 queue owner",
        ))?;
        if queues.precision != self.live_pc {
            return Err(error(
                "50ACC0 queue arithmetic PC differs from current Init PC",
            ));
        }
        let owner = self
            .catalog
            .load_owners
            .try_borrow_mut()
            .map_err(error)?
            .restoring(self.target(slot))
            .map_err(error)?;
        queues.add_after(3, owner, 0, delay_bits).map_err(error)?;
        Ok(())
    }
    fn clear(&mut self, slot: u8, component: &mut Component) -> Result<()> {
        self.check(slot, component)?;
        let owner = self
            .catalog
            .load_owners
            .try_borrow()
            .map_err(error)?
            .get(crate::scheduler_owners::Target::Restoring(
                self.target(slot),
            ))
            .map_err(error)?;
        let mut queues = self.catalog.load_queues.try_borrow_mut().map_err(error)?;
        let queues = queues
            .as_deref_mut()
            .ok_or(Error::Unsupported("50B250 actual load queues"))?;
        // Queues owns the current Updatable head; constructor/load receipt
        // fields cannot substitute for it or manufacture a native pointer.
        if queues.has_owner(owner) {
            queues.cancel_owner(owner).map_err(error)?;
        }
        Ok(())
    }
    fn legacy_area(&mut self, _: &mut Area) -> Result<()> {
        Err(Error::Unsupported(
            "4DB238 legacy Area50AC00 periodic scheduling",
        ))
    }
}

/// Actual native class dispatch. Source, terrain and object storage are checked
/// by the real base service; all earlier mutations survive a later tail error.
#[allow(clippy::too_many_arguments)]
pub fn initialize(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    derived: Derived<'_>,
    object: &mut load_object_init_world::Context<'_, '_>,
    state: &mut State,
    world_version: &u32,
    rng: &mut NativeRng,
    pc: Precision,
) -> Result<bool> {
    if record.definition.native_class != derived.class()
        || record.shell.kind != LoadKind::Object
        || record.shell.object.is_none()
        || record.shell.actor.is_some()
        || !record
            .registry
            .get(record.owner)
            .is_some_and(|v| matches!(v.value, RestoredRecord::Loading))
    {
        return Err(error(
            "special Object Init actual held class/storage/derived allocation differs",
        ));
    }
    if !load_object_init_world::initialize(catalog, record, object, pc)? {
        return Ok(false);
    }
    let mut services = Owned {
        catalog,
        owner: record.owner,
        canonical: &record.definition.proof.id,
        live_pc: pc,
    };
    tail(derived, state, world_version, rng, pc, &mut services)
}
