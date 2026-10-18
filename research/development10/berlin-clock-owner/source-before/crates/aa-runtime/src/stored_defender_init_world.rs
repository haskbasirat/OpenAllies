//!566DB0 on the SAME fresh Defender, Zone4, completed source Bank and current
//!Performer owners. The shared coordinator owns branch/store ordering.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    catalog::Catalog,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    component_virtuals::defender as native,
    components::ComponentSlot as Slot,
};
use aa_numeric::Precision;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn defender(
    actors: &ActorStore,
    owner: ActorId,
) -> Result<&aa_actors::defender_state::DefenderState> {
    match actors.component(owner, Slot::Defender).map_err(error)? {
        Some(RuntimeComponent::Defender(v)) if v.header.owner == owner => Ok(v),
        _ => Err(error(
            "stored566DB0 actual Defender allocation/owner differs",
        )),
    }
}
fn missing_environment() -> std::result::Result<aa_numeric::crt_pow::CrtEnvironment, String> {
    Err("required native service: Defender57E430 current thread CRT environment".into())
}
struct Stored<'a, 'source> {
    catalog: &'a Catalog<'source>,
    staged: &'a mut Staged,
    key: ObjectKey,
    owner: ActorId,
    source_pc: Precision,
    live_pc: Precision,
}
impl Stored<'_, '_> {
    fn current(&self) -> Result<()> {
        if self.staged.actors.id(self.key) != Some(self.owner)
            || self.staged.actors.key(self.owner) != Some(self.key)
        {
            return Err(error("566DB0 current Actor generation changed"));
        }
        defender(&self.staged.actors, self.owner)?;
        Ok(())
    }
    fn zone(&self) -> Result<Option<&aa_zones::zone::Zone>> {
        self.current()?;
        Ok(self
            .staged
            .actors
            .object(self.key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("566DB0 Actor"))?
            .zones_148
            .zones()
            .slots
            .get(4)
            .and_then(Option::as_ref))
    }
    fn pose(&self) -> Result<crate::actor_world::PlacementState> {
        self.current()?;
        if let Ok(value) = self.staged.actors.staged_placement(self.key) {
            return Ok(value);
        }
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("566DB0 actual placement World absent"))?;
        self.staged
            .actors
            .placement(self.key, world.live_spatial().map_err(error)?)
            .map_err(error)
    }
}
impl native::World for Stored<'_, '_> {
    fn zone_present(&mut self) -> Result<bool> {
        Ok(self.zone()?.is_some())
    }
    fn definition_mask(&mut self) -> Result<u32> {
        self.current()?;
        let id = &self
            .staged
            .actors
            .object(self.key)
            .map_err(error)?
            .definition
            .proof
            .id;
        self.catalog
            .canonical_component_mask(id, self.source_pc)
            .map_err(error)
    }
    fn range(&mut self, organization: bool) -> Result<u32> {
        self.current()?;
        let id = &self
            .staged
            .actors
            .object(self.key)
            .map_err(error)?
            .definition
            .proof
            .id;
        let binding = self
            .catalog
            .actor_bindings(id, self.source_pc)
            .map_err(error)?;
        let bits = if organization {
            binding
                .source_ground
                .organization
                .as_ref()
                .ok_or_else(|| error("45D4C0 actual ActorData+2FC Organization source absent"))?
                .control_range
                .bits
        } else {
            //4657A0..465937 reads this fixed field into ActorData+218. Do not
            //rebuild unrelated Ability children at the reached range getter.
            let actor = self.catalog.actors.resolved_actor(id).map_err(error)?;
            let node = actor
                .child("DefenderComponent")
                .map_err(error)?
                .ok_or_else(|| error("45D6C0 actual Defender source absent"))?;
            node.field("control_range", self.source_pc)
                .map_err(error)?
                .ok_or_else(|| error("45D6C0 actual ActorData+218 field absent"))?
                .fixed_bits()
                .map_err(error)?
        };
        //FLD32/FSTP32 is bit-preserving except that an sNaN is quieted. The
        //Unicorn oracle excludes signalling NaNs (documented emulator limit).
        Ok(
            if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
                bits | 0x0040_0000
            } else {
                bits
            },
        )
    }
    fn store_range(&mut self, bits: u32) -> Result<()> {
        self.current()?;
        match self
            .staged
            .actors
            .component_mut(self.owner, Slot::Defender)
            .map_err(error)?
        {
            Some(RuntimeComponent::Defender(v)) if v.header.owner == self.owner => {
                v.word_2c = bits;
                Ok(())
            }
            _ => Err(error("566E08 actual Defender changed before+2C store")),
        }
    }
    fn create_zone(&mut self) -> Result<bool> {
        let flags = self.pose()?.flags;
        let identities = &mut self
            .staged
            .load_world
            .as_mut()
            .ok_or_else(|| error("566DB0 actual zone identities absent"))?
            .zone_identities;
        let actor = self
            .staged
            .actors
            .object_mut(self.key)
            .map_err(error)?
            .actor
            .as_mut()
            .ok_or(Error::NullDereference("4D69A0 Actor"))?;
        crate::load_detection_activation::create_slot_fields(
            self.owner,
            flags,
            actor.zones_148.zones_mut(),
            identities,
            4,
        )
        .map_err(error)
    }
    fn aircraft_present(&mut self) -> Result<bool> {
        self.current()?;
        Ok(self
            .staged
            .actors
            .component(self.owner, Slot::Aircraft)
            .map_err(error)?
            .is_some())
    }
    fn aircraft_zone_flag(&mut self) -> Result<()> {
        let zone = self
            .zone()?
            .ok_or(Error::NullDereference("515940 actual Zone4"))?;
        let identities = &self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("515940 actual identities absent"))?
            .zone_identities;
        if identities.allocation(zone.key) != Some((self.owner, 4))
            || zone.actor != self.owner.raw()
            || zone.flags != 0x10
        {
            return Err(error("515940 Zone4 allocation/type/owner differs"));
        }
        self.staged
            .actors
            .object_mut(self.key)
            .map_err(error)?
            .actor
            .as_mut()
            .ok_or(Error::NullDereference("515940 Actor"))?
            .zones_148
            .zones_mut()
            .slots[4]
            .as_mut()
            .ok_or(Error::NullDereference("515940 Zone4"))?
            .set_state_bit(1, true);
        Ok(())
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        Ok(self.pose()?.position_bits[..2]
            .try_into()
            .expect("two coordinates"))
    }
    fn move_zone(&mut self, position: [u32; 2], range: u32) -> Result<()> {
        let staged = &mut *self.staged;
        let world = staged
            .load_world
            .as_mut()
            .ok_or_else(|| error("515F40 actual persistent World absent"))?;
        crate::stored_zone_activation::operate_slot(
            self.catalog,
            &staged.definitions,
            &mut staged.actors,
            world,
            self.key,
            4,
            Some((position, range)),
            staged.inactive_load.as_ref(),
            self.live_pc,
        )
        .map_err(error)
    }
    fn performer_present(&mut self) -> Result<bool> {
        self.current()?;
        Ok(self
            .staged
            .actors
            .component(self.owner, Slot::Performer)
            .map_err(error)?
            .is_some())
    }
    fn refresh_performer(&mut self) -> Result<()> {
        let staged = &mut *self.staged;
        let world = staged
            .load_world
            .as_mut()
            .ok_or_else(|| error("57E430 actual current World absent"))?;
        let crate::load_world::SpatialOwner::Adopted(adoption) = &world.spatial else {
            return Err(error("57E430 actual adopted spatial owner absent"));
        };
        if adoption.error.is_some() || !adoption.pending.is_empty() {
            return Err(error("57E430 spatial adoption incomplete"));
        }
        crate::stored_performer_refresh::refresh_value_on_owners(
            self.catalog,
            &mut staged.actors,
            &mut staged.definitions,
            Some(&adoption.world),
            self.key,
            &mut world.variables,
            &staged.inventory,
            self.catalog
                .runtime_environment
                .unwrap_or(missing_environment),
            self.live_pc,
        )
        .map_err(error)
    }
}
pub fn init_existing(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    pc: Precision,
) -> Result<bool> {
    let (owner, source_pc) = crate::stored_component_init_source::checked(
        catalog,
        &staged.actors,
        &staged.definitions,
        key,
        Slot::Defender,
    )?;
    defender(&staged.actors, owner)?;
    let leaf = aa_actors::component_virtuals::leaves(
        aa_actors::init_create::FactoryKind::Standard(Slot::Defender),
    );
    if leaf.init_existing != 0x566db0 || leaf.vf14 != 0x4b2830 {
        return Err(error("566DB0 actual Defender vtable differs"));
    }
    native::initialize(&mut Stored {
        catalog,
        staged,
        key,
        owner,
        source_pc,
        live_pc: pc,
    })
}
