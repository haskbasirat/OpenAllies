//! Native 4D6660 (under-siege refresh) over the live retained owners.
//!
//! The arithmetic/decision body is aa_combat::siege::refresh_under_siege
//! (oracle-tested in aa-combat/tests/native_siege.rs). This binds its reads:
//! Thing+8 flags, Actor+100 state bits, the Body+28 damage record time (+10),
//! Actor+98 Settlement / +90 Wall presence, virtual +100 (4CED60) and +158
//! (4CE980/588990), the Actor+128 engagement head, City(+D4)+20 and the
//! Denizen(+88) gate. Every reached leaf without a live binding (Settlement,
//! Wall, Organization/wall damage records, Denizen predicates, state 20
//! transition and 4D1350) returns an explicit error at that call.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_body::F32;
use aa_combat::{Error, Result};
use aa_numeric::Precision;

fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(format!("live 4D6660: {e}"))
}
fn required<T>(leaf: &str) -> Result<T> {
    Err(fail(format!("required native {leaf} is not bound")))
}

pub(crate) struct Siege<'a> {
    pub actors: &'a ActorStore,
    pub spatial: &'a SpatialWorld,
    /// World+E0.
    pub now: F32,
}
impl Siege<'_> {
    fn key(&self, actor: u32) -> Result<ObjectKey> {
        self.actors
            .key(ActorId::from_raw(actor))
            .ok_or_else(|| fail(format!("actor {actor:#x} has no live GTID")))
    }
    fn components(&self, actor: u32) -> Result<&aa_actors::components::Components<RuntimeComponent>> {
        self.actors
            .object(self.key(actor)?)
            .map_err(fail)?
            .components()
            .map_err(fail)
    }
    fn word(&self, actor: u32, offset: u32) -> Result<u32> {
        self.actors
            .object(self.key(actor)?)
            .map_err(fail)?
            .actor
            .as_ref()
            .ok_or_else(|| fail("Actor shell absent"))?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| fail(format!("Actor+{offset:X} is uninitialized")))
    }
    /// 4CED60 (GActor virtual +100): Structure+10, else the Wall 58CB30 chain.
    fn outer_of(&self, actor: u32) -> Result<Option<u32>> {
        let key = self.key(actor)?;
        let object = self.actors.object(key).map_err(fail)?;
        if object.definition.native_class != 3 {
            return required("virtual +100 for a non-Actor class");
        }
        let components = object.components().map_err(fail)?;
        match components.get(ComponentSlot::Structure) {
            Some(RuntimeComponent::Structure(v)) if v.object_handle_10 != 0 => {
                return required("4CED60 Structure+10 parent object binding");
            }
            Some(RuntimeComponent::Structure(_)) | None => {}
            Some(_) => return Err(fail("Structure slot payload mismatch")),
        }
        if components.present(ComponentSlot::Wall) {
            return required("4CED60 Wall+90 58CB30 parent chain");
        }
        Ok(None)
    }
}
impl aa_combat::siege::World for Siege<'_> {
    fn flags8(&self, actor: u32) -> Result<u32> {
        Ok(self
            .actors
            .placement(self.key(actor)?, self.spatial)
            .map_err(fail)?
            .flags)
    }
    fn flags100(&self, actor: u32) -> Result<u32> {
        Ok(self.components(actor)?.state.low)
    }
    fn last_damage_time(&self, actor: u32) -> Result<Option<F32>> {
        let components = self.components(actor)?;
        match components.get(ComponentSlot::Body) {
            //Body+28 record, its +10 time word (Body+38).
            Some(RuntimeComponent::Body(v)) => return Ok(Some(F32(v.body.last_damage[4]))),
            Some(_) => return Err(fail("Body slot payload mismatch")),
            None => {}
        }
        if components.present(ComponentSlot::Organization) {
            return required("Organization+90 damage record time");
        }
        if components.present(ComponentSlot::Wall) {
            return required("Wall+4C damage record time");
        }
        Ok(None)
    }
    fn has_wall(&self, actor: u32) -> Result<bool> {
        if self.components(actor)?.present(ComponentSlot::Settlement) {
            return required("Actor+98 Settlement owner (not a runtime component)");
        }
        Ok(false)
    }
    fn has_wall_link(&self, actor: u32) -> Result<bool> {
        Ok(self.components(actor)?.present(ComponentSlot::Wall))
    }
    fn outer(&mut self, actor: u32) -> Result<Option<u32>> {
        self.outer_of(actor)
    }
    fn delegates(&mut self, actor: u32) -> Result<bool> {
        //4CE980: Structure+94 present and 588990 (Structure+10 parent's
        //Settlement+14 equals this actor). A null +10 returns false.
        match self.components(actor)?.get(ComponentSlot::Structure) {
            None => Ok(false),
            Some(RuntimeComponent::Structure(v)) if v.object_handle_10 == 0 => Ok(false),
            Some(RuntimeComponent::Structure(_)) => required("588990 Structure+10 parent Settlement+14"),
            Some(_) => Err(fail("Structure slot payload mismatch")),
        }
    }
    fn engaged(&self, actor: u32) -> Result<bool> {
        Ok(self.word(actor, 0x128)? != 0)
    }
    fn watched_flag(&self, actor: u32) -> Result<bool> {
        match self.components(actor)?.get(ComponentSlot::City) {
            None => Ok(false),
            Some(RuntimeComponent::City(c)) => Ok(c.byte_20 != 0),
            Some(_) => Err(fail("City slot payload mismatch")),
        }
    }
    fn component88_gate(&self, actor: u32) -> Result<bool> {
        //4D66F3: Denizen+88 non-null, zone count +14C > 5, zone[5] (+148)+14
        //non-null and Denizen+24 non-null.
        let denizen = match self.components(actor)?.get(ComponentSlot::Denizen) {
            None => return Ok(false),
            Some(RuntimeComponent::Denizen(v)) => v,
            Some(_) => return Err(fail("Denizen slot payload mismatch")),
        };
        let object = self.actors.object(self.key(actor)?).map_err(fail)?;
        let zones = &object
            .actor
            .as_ref()
            .ok_or_else(|| fail("Actor shell absent"))?
            .zones_148
            .zones()
            .slots;
        if zones.len() <= 5 || zones[5].is_none() {
            return Ok(false);
        }
        let word_24 = denizen
            .words
            .get(&0x24)
            .copied()
            .ok_or_else(|| fail("Denizen+24 is uninitialized"))?;
        Ok(word_24 != 0)
    }
    fn wall_predicate(&mut self, _: u32) -> Result<bool> {
        required("585160 wall-complete predicate")
    }
    fn component88_predicate(&mut self, _: u32, _: u32) -> Result<bool> {
        required("569540 intruder predicate")
    }
    fn wall_update(&mut self, _: u32) -> Result<()> {
        required("586370 propagate under siege")
    }
    fn set_state(&mut self, actor: u32, _: u8, enabled: bool) -> Result<()> {
        Err(fail(format!(
            "required native 4D64F0 state 20 transition ({enabled}) for {actor:#x} is not bound"
        )))
    }
    fn notify(&mut self, _: u32) -> Result<()> {
        required("4D1350 actor refresh after state 20")
    }
    fn now(&self) -> Result<F32> {
        Ok(self.now)
    }
}

/// Native 4D6660(actor) with SVars+40 UnderSiegeRecentlyDamagedSeconds.
pub(crate) fn refresh(
    actors: &ActorStore,
    spatial: &SpatialWorld,
    variables: &crate::load_view_variables::State,
    now: F32,
    actor: ActorId,
    precision: Precision,
) -> Result<()> {
    let ordinal = variables
        .registrations()
        .find(|r| r.name() == "UnderSiegeRecentlyDamagedSeconds")
        .map(|r| r.ordinal())
        .ok_or_else(|| fail("SVar UnderSiegeRecentlyDamagedSeconds is not registered"))?;
    let threshold = match variables.current(ordinal as usize).map_err(fail)? {
        crate::load_view_variables::CurrentValue::Fixed(v) => F32(*v),
        _ => return Err(fail("UnderSiegeRecentlyDamagedSeconds is not the float kind")),
    };
    let mut world = Siege { actors, spatial, now };
    aa_combat::siege::refresh_under_siege(actor.raw(), threshold, &mut world, precision)
}
