//! Shared564D80 on the same fresh Construction/Detection/Lair allocations.
//! The original version/head/queue coordinator is reused unchanged.
use crate::{
    actor_world::{ObjectKey, RuntimeComponent},
    catalog::Catalog,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot as Slot,
};
use aa_numeric::Precision;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn checked(catalog: &Catalog<'_>, staged: &Staged, key: ObjectKey, slot: Slot) -> Result<ActorId> {
    if !crate::load_detection_init_world::supports(slot) {
        return Err(error("stored564D80 unsupported actual component class"));
    }
    let binding =
        crate::map_component_init_create::actual_binding(staged, catalog, key).map_err(error)?;
    let owner = staged
        .actors
        .id(key)
        .ok_or_else(|| error("stored564D80 actual GTID absent"))?;
    if staged.actors.key(owner) != Some(key)
        || binding
            .definition
            .component_mask
            .is_none_or(|m| m & (1 << slot.index()) == 0)
    {
        return Err(error("stored564D80 actual generation/source mask differs"));
    }
    let header = match (slot, staged.actors.component(owner, slot).map_err(error)?) {
        (Slot::Construction, Some(RuntimeComponent::Construction(v))) => &v.header,
        (Slot::Detection, Some(RuntimeComponent::Detection(v))) => &v.header,
        (Slot::Lair, Some(RuntimeComponent::Lair(v))) => &v.header,
        _ => {
            return Err(error(
                "stored564D80 actual typed component allocation differs",
            ));
        }
    };
    let leaf =
        aa_actors::component_virtuals::leaves(aa_actors::init_create::FactoryKind::Standard(slot));
    if header.owner != owner || leaf.init_existing != 0x564d80 {
        return Err(error(
            "stored564D80 actual component owner/virtual8 differs",
        ));
    }
    Ok(owner)
}
pub fn init_existing(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    slot: Slot,
    pc: Precision,
) -> Result<bool> {
    checked(catalog, staged, key, slot)?;
    let mut world = crate::stored_component_schedule::World::new(staged, key, slot, pc)?;
    crate::load_detection_init_world::initialize(&mut world)
}

struct After<'a, 'source> {
    catalog: &'a Catalog<'source>,
    staged: &'a mut Staged,
    key: ObjectKey,
    owner: ActorId,
    pc: Precision,
}
impl After<'_, '_> {
    fn pose(&self) -> Result<crate::actor_world::PlacementState> {
        if let Ok(p) = self.staged.actors.staged_placement(self.key) {
            return Ok(p);
        }
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("stored56EC90 actual World absent"))?;
        self.staged
            .actors
            .placement(self.key, world.live_spatial().map_err(error)?)
            .map_err(error)
    }
    fn zone(&self) -> Result<Option<&aa_zones::zone::Zone>> {
        Ok(self
            .staged
            .actors
            .object(self.key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or_else(|| error("stored56EC90 actual Actor storage absent"))?
            .zones_148
            .zones()
            .slots
            .get(6)
            .and_then(Option::as_ref))
    }
}
impl crate::load_detection_activation::after_core::Fields for After<'_, '_> {
    type Error = Error;
    fn word138(&mut self) -> Result<u32> {
        self.staged
            .actors
            .object(self.key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or_else(|| error("stored56EC90 actual Actor absent"))?
            .words
            .get(&0x138)
            .copied()
            .ok_or_else(|| error("stored56EC90 actual Actor138 uninitialized"))
    }
    fn structure(&mut self) -> Result<bool> {
        Ok(self
            .staged
            .actors
            .object(self.key)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state
            .low
            & 0x20_0000
            != 0)
    }
    fn remote_range(&mut self) -> Result<u32> {
        let binding =
            crate::map_component_init_create::actual_binding(self.staged, self.catalog, self.key)
                .map_err(error)?;
        self.catalog
            .actors
            .native_ground_definition(&binding.definition.proof.id, binding.precision)
            .map_err(error)?
            .detection
            .map(|v| v.finalized_remote_range_bits)
            .ok_or_else(|| error("stored56EC90 actual source remote detection range absent"))
    }
    fn attached_path(&mut self) -> Result<()> {
        Err(Error::Unsupported(
            "56EC90 Actor138 path requires actual source430/Denizen/scheduling services",
        ))
    }
    fn radius(&mut self) -> Result<u32> {
        match self
            .staged
            .actors
            .component(self.owner, Slot::Detection)
            .map_err(error)?
        {
            Some(RuntimeComponent::Detection(d)) if d.header.owner == self.owner => Ok(d.radius.0),
            _ => Err(error(
                "stored56EC90 actual Detection allocation/owner differs",
            )),
        }
    }
    fn flags(&mut self) -> Result<u32> {
        Ok(self.pose()?.flags)
    }
    fn positive(&mut self, radius: u32) -> Result<bool> {
        Ok(aa_zones::fpu::less(
            &aa_zones::fpu::load(0).map_err(error)?,
            &aa_zones::fpu::load(radius).map_err(error)?,
        ))
    }
    fn zone_change(&mut self, enabled: bool, radius: u32) -> Result<()> {
        if enabled {
            let flags = self.pose()?.flags;
            let world = self
                .staged
                .load_world
                .as_mut()
                .ok_or_else(|| error("stored56EB40 actual World/zone identity owner absent"))?;
            let actor = self
                .staged
                .actors
                .object_mut(self.key)
                .map_err(error)?
                .actor
                .as_mut()
                .ok_or_else(|| error("stored56EB40 actual Actor absent"))?;
            crate::load_detection_activation::create_slot_fields(
                self.owner,
                flags,
                actor.zones_148.zones_mut(),
                &mut world.zone_identities,
                6,
            )
            .map_err(error)?;
            let zone = self
                .zone()?
                .ok_or_else(|| error("stored56EB40 actual newly created Zone6 absent"))?;
            if zone.active()
                && aa_zones::fpu::equal(
                    &aa_zones::fpu::load(zone.radius_bits).map_err(error)?,
                    &aa_zones::fpu::load(radius).map_err(error)?,
                )
            {
                return Ok(());
            }
            let pose = self.pose()?;
            let world = self
                .staged
                .load_world
                .as_mut()
                .ok_or_else(|| error("stored515F40 actual World absent"))?;
            crate::stored_zone_activation::operate_slot(
                self.catalog,
                &self.staged.definitions,
                &mut self.staged.actors,
                world,
                &self.staged.clock,
                self.key,
                6,
                Some(([pose.position_bits[0], pose.position_bits[1]], radius)),
                self.staged.inactive_load.as_ref(),
                self.pc,
            )
            .map_err(error)
        } else if self.zone()?.is_some() {
            let world = self
                .staged
                .load_world
                .as_mut()
                .ok_or_else(|| error("stored514CE0 actual World absent"))?;
            crate::stored_zone_activation::operate_slot(
                self.catalog,
                &self.staged.definitions,
                &mut self.staged.actors,
                world,
                &self.staged.clock,
                self.key,
                6,
                None,
                self.staged.inactive_load.as_ref(),
                self.pc,
            )
            .map_err(error)
        } else {
            Ok(())
        }
    }
}
pub fn after_init(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    pc: Precision,
) -> Result<()> {
    let owner = checked(catalog, staged, key, Slot::Detection)?;
    crate::load_detection_activation::after_core::after(&mut After {
        catalog,
        staged,
        key,
        owner,
        pc,
    })
}
