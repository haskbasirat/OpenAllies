//!Unpublished concrete522270 binder; crate writes are paused for packaging.
//!Ordinary Actor+2C stores reason4 only while178 is zero. Active4D7960
//!effects remain required after the proved initial prefix; no death is faked.
use crate::{
    actor_world::{ActorAccess, ObjectKey, RuntimeComponent, RuntimeObject},
    catalog::Catalog,
    definitions::Bank,
    spatial_world::SpatialWorld,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot, ground_states::OrganizationState};
use aa_mind::{Arena, MindId, construct::Kind, regiment_orphan as native};
use aa_numeric::Precision;
fn error(value: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(value.to_string())
}
fn required<T>(value: &str) -> aa_mind::Result<T> {
    Err(error(value))
}

struct Bound<'a, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    actors: &'a mut ActorAccess<'store>,
    arena: &'a Arena,
    bank: &'a Bank,
    spatial: &'a SpatialWorld,
    mind: MindId,
    object: ObjectKey,
}
impl Bound<'_, '_, '_> {
    fn validate(&self, mind: MindId) -> aa_mind::Result<()> {
        if mind != self.mind {
            return required("522270 captured Mind identity differs");
        }
        let object = self.actors.object(self.object).map_err(error)?;
        match object.components().map_err(error)?.get(Slot::Mind) {
            Some(RuntimeComponent::Mind(m)) if m.kind == Kind::Regiment && m.mind_id == mind => {
                Ok(())
            }
            _ => required("522270 actual captured Regiment allocation replaced"),
        }
    }
    fn actor(&self, actor: ActorId) -> aa_mind::Result<&RuntimeObject> {
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| error("522270 actual Actor generation absent"))?;
        let object = self.actors.object(key).map_err(error)?;
        if object.definition.native_class != 3 || object.actor.is_none() || object.object.is_none()
        {
            return required("522270 actual ordinary Actor virtual2C dispatch required");
        }
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, &object.definition)
            .map_err(error)?;
        if !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof) {
            return required("522270 actual Actor/source identity differs");
        }
        Ok(object)
    }
    fn organization(
        &self,
        receipt: (ActorId, usize),
    ) -> aa_mind::Result<&OrganizationState<String>> {
        self.actor(receipt.0)?;
        match self
            .actors
            .component(receipt.0, Slot::Organization)
            .map_err(error)?
        {
            Some(RuntimeComponent::Organization(org))
                if org.header.owner == receipt.0 && org as *const _ as usize == receipt.1 =>
            {
                Ok(org)
            }
            _ => required("522270 captured actual Organization allocation replaced"),
        }
    }
}
impl native::World for Bound<'_, '_, '_> {
    type Actor = ActorId;
    type Organization = (ActorId, usize);
    fn current_actor(&mut self, mind: MindId) -> aa_mind::Result<ActorId> {
        self.validate(mind)?;
        let actor = ActorId::from_raw(self.arena.mind(mind)?.actor);
        self.actor(actor)?;
        Ok(actor)
    }
    fn request_reason(&mut self, actor: ActorId, reason: u32) -> aa_mind::Result<()> {
        self.actor(actor)?;
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| error("4CEEF0 current Actor absent"))?;
        let word = self
            .actors
            .object_mut(key)
            .map_err(error)?
            .actor
            .as_mut()
            .unwrap()
            .words
            .get_mut(&0x178)
            .ok_or_else(|| error("4CEEF0 actual Actor178 uninitialized"))?;
        if let Some(store) = native::ordinary_reason(*word, reason) {
            *word = store;
        }
        Ok(())
    }
    fn organization(&mut self, actor: ActorId) -> aa_mind::Result<Self::Organization> {
        self.actor(actor)?;
        match self
            .actors
            .component(actor, Slot::Organization)
            .map_err(error)?
        {
            Some(RuntimeComponent::Organization(org)) if org.header.owner == actor => {
                Ok((actor, org as *const _ as usize))
            }
            _ => required("522270 native null/foreign Organization dereference"),
        }
    }
    fn member_count(&mut self, org: Self::Organization) -> aa_mind::Result<usize> {
        Ok(Bound::organization(self, org)?.members.len())
    }
    fn member(
        &mut self,
        org: Self::Organization,
        index: usize,
    ) -> aa_mind::Result<Option<ActorId>> {
        let member = *Bound::organization(self, org)?
            .members
            .get(index)
            .ok_or_else(|| error("522270 current Organization vector/index differs"))?;
        Ok((!member.is_null()).then_some(member))
    }
    fn death_4d7960(&mut self, actor: ActorId) -> aa_mind::Result<()> {
        native::death_prefix(self, actor)
    }
}
impl native::DeathPrefixWorld for Bound<'_, '_, '_> {
    type Actor = ActorId;
    type Credit = u32;
    fn flags(&mut self, actor: ActorId) -> aa_mind::Result<u32> {
        self.actor(actor)?;
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| error("4D7960 actual Actor generation absent"))?;
        Ok(self
            .actors
            .placement(key, self.spatial)
            .map_err(error)?
            .flags)
    }
    fn credit_4d57c0(&mut self, actor: ActorId) -> aa_mind::Result<u32> {
        use native::DeathPrefixWorld as _;
        //4D57DB..5805 only: first flags bit4, then lazy CURRENT SessionE4.
        if self.flags(actor)? & 16 != 0 {
            return Ok(0);
        }
        let session = self
            .catalog
            .load_session
            .ok_or_else(|| error("4D57C0 actual SessionE4 owner absent"))?;
        if session.application_state().map_err(error)? == 3 {
            return Ok(0);
        }
        required("4D57C0 positive actual actor credit/support callbacks required")
    }
    fn remaining_4d7993(&mut self, _: ActorId, _: u32) -> aa_mind::Result<()> {
        required(
            "4D7960 positive zone/warning/state/component/death suffix after actual credit prefix required",
        )
    }
}

///The captured scheduled allocation is independent of CURRENT Mind.actor.
///No component, member vector, Arena, queue or World state is removed/copied.
pub(crate) fn run(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    arena: &Arena,
    bank: &Bank,
    spatial: &SpatialWorld,
    mind: MindId,
    captured: ObjectKey,
    _pc: Precision,
) -> aa_mind::Result<()> {
    let mut bound = Bound {
        catalog,
        actors,
        arena,
        bank,
        spatial,
        mind,
        object: captured,
    };
    bound.validate(mind)?;
    native::run(&mut bound, mind)
}
