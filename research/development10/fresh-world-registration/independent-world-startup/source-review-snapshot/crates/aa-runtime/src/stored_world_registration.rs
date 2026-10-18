//! Fresh4F9510 over the SAME World registration lists and published Actor.
//! State reentry and zone entry use disjoint loans of these retained owners;
//! no restored shell, replacement registry slot or copied World is installed.
use crate::{actor_world::{ObjectKey, Source}, catalog::Catalog, restoration::Staged};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_numeric::Precision;
type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> String { e.to_string() }

pub(crate) fn register(staged: &mut Staged, catalog: &Catalog<'_>, key: ObjectKey, pc: Precision) -> Result<()> {
    let owner = staged.actors.id(key).ok_or("fresh4F9510 actual GTID absent")?;
    let object = staged.actors.object(key).map_err(error)?;
    if object.definition.native_class != 3 || object.actor.is_none() || object.object.is_none()
        || !matches!(&staged.actors.entity(key).map_err(error)?.source, Source::New { .. }) {
        return Err("fresh4F9510 requires the actual newly constructed ordinary Actor".into());
    }
    crate::live_object_init::Source::bind_in_bank(catalog, &staged.definitions, &object.definition).map_err(error)?;
    let world = staged.load_world.as_mut().ok_or("fresh4F9510 actual World absent")?;
    let crate::load_world::SpatialOwner::Adopted(spatial) = &mut world.spatial else {
        return Err("fresh4F9510 actual adopted spatial owner absent".into());
    };
    if spatial.error.is_some() || !spatial.pending.is_empty() || spatial.world.index().precision() != pc {
        return Err("fresh4F9510 actual spatial adoption/live precision differs".into());
    }
    //Fresh4D7230 has already transferred the actual placement into this
    //index; this is a portable ownership preflight, not native registration.
    spatial.world.key(key).map_err(error)?;
    let session = catalog.load_session.ok_or("fresh4F9510 actual application Session loan absent")?;
    let (arena, mut actors) = staged.actors.split_minds();
    let mut context = crate::stored_actor_registration::Context {
        state: crate::stored_actor_state_effects::Context {
            actors: &mut actors, arena, bank: &mut staged.definitions, spatial: &mut spatial.world,
            variables: &mut world.variables, clock: &staged.clock, inventory: &staged.inventory,
            presentation: &mut world.presentation, queues: &mut staged.queues, owners: &staged.owners,
        },
        zones: &mut world.zones, point: &world.point, session, mission: &world.mission_fields,
    };
    world.registration.register(&mut Current { catalog, context: &mut context, key, owner, pc })
}

struct Current<'a, 'fields, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    context: &'a mut crate::stored_actor_registration::Context<'fields, 'store>,
    key: ObjectKey, owner: ActorId, pc: Precision,
}
impl Current<'_, '_, '_, '_> {
    fn current(&self) -> Result<&crate::actor_world::RuntimeObject> {
        if self.context.state.actors.id(self.key) != Some(self.owner) {
            return Err("fresh4F9510 current generation/allocation changed".into());
        }
        let object = self.context.state.actors.object(self.key).map_err(error)?;
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.context.state.bank, &object.definition).map_err(error)?;
        if object.definition.native_class != 3 || object.actor.is_none() || object.object.is_none() {
            return Err("fresh4F9510 current Actor constructor/source layout changed".into());
        }
        Ok(object)
    }
    fn settlement_space_for(&self, key: ObjectKey) -> Result<bool> {
        let object = self.context.state.actors.object(key).map_err(error)?;
        let source = crate::live_object_init::Source::bind_in_bank(
            self.catalog, self.context.state.bank, &object.definition,
        ).map_err(error)?;
        self.catalog.actors.native_field(&object.definition.proof.id, "needs_space_for_settlement_zone", source.source_precision())
            .map_err(error)?.ok_or("fresh4F4C40 actual settlement-space source absent")?.boolean().map_err(error)
    }
    fn required_city(&self) -> Result<u32> {
        self.current()?.actor.as_ref().and_then(|actor| actor.words.get(&0x158)).copied()
            .ok_or_else(|| "fresh4F4C40 actual Actor158 is uninitialized".into())
    }
}
impl crate::live_registration::World for Current<'_, '_, '_, '_> {
    fn capture_name(&mut self) -> Result<Vec<u16>> { Ok(self.current()?.thing.name_utf16.clone()) }
    fn clear_loading_flag(&mut self) -> Result<()> {
        self.current()?;
        self.context.state.spatial.clear_loading_flag(self.key).map_err(error)
    }
    fn id(&mut self) -> Result<ActorId> { self.current()?; Ok(self.owner) }
    fn predicate(&mut self, offset: u32) -> Result<bool> {
        crate::live_registration::class_predicate(self.current()?.definition.native_class, offset)
    }
    fn team_slot(&mut self) -> Result<i32> { Err("fresh ordinary Actor has no Team virtualAC branch".into()) }
    fn register_actor(&mut self) -> Result<()> {
        self.current()?;
        let result = crate::stored_actor_registration::register(self.catalog, self.context, self.key, self.pc).map_err(error);
        let owner = self.current().map(|_| ());
        match (result, owner) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(prior), Ok(())) => Err(prior),
            (Ok(()), Err(owner)) => Err(owner),
            (Err(prior), Err(owner)) => Err(format!("{prior}; {owner}")),
        }
    }
    fn settlement_space(&mut self) -> Result<bool> { self.current()?; self.settlement_space_for(self.key) }
    fn city_present(&mut self) -> Result<bool> {
        Ok(self.current()?.components().map_err(error)?.get(ComponentSlot::City).is_some())
    }
    fn required_city_live(&mut self) -> Result<bool> {
        let city = ActorId::from_raw(self.required_city()?);
        Ok(self.context.state.actors.key(city).is_some())
    }
    fn required_city_settlement_space(&mut self) -> Result<bool> {
        let city = ActorId::from_raw(self.required_city()?);
        let key = self.context.state.actors.key(city).ok_or("fresh4F4C40 required City generation disappeared")?;
        self.settlement_space_for(key)
    }
    fn assert_duplicate_available_city(&mut self) -> Result<bool> {
        Err("fresh4F4C40 actual duplicate available-City assertion owner is required".into())
    }
}
