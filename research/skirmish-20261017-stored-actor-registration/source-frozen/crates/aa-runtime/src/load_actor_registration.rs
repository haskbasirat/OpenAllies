//! Concrete saved Actor registration4D62F0 over its current held allocation.
//! State count/order belongs to the native ActorState manager, state words are
//! reread after each entry, and the grid half uses the same persistent map.
//! Positive4D4A90 effects remain required until their actual owners are bound.
use crate::{
    catalog::Catalog,
    live_object_init::{Record, Source},
    load_spatial::LoadSpatial,
    load_world_globals::{PointShape, SessionPhase},
};
use aa_actors::{
    ActorId, IdentityRegistry,
    actor_creation::{Error, Result},
    actor_registration,
    components::StateBits,
};
use aa_mapworld::{restore::RestoredRecord, world::WorldState};
use aa_numeric::Precision;
use aa_zones::tile::ZoneMap;

#[cfg(test)]
#[path = "load_actor_registration_tests.rs"]
mod tests;

pub struct GridContext<'a> {
    pub zones: &'a mut ZoneMap,
    pub spatial: &'a LoadSpatial,
    pub point: &'a PointShape,
    pub session: &'a dyn SessionPhase,
    pub world: &'a WorldState,
    pub city: Option<crate::load_zone_entry::CityContext<'a>>,
    pub live_world48: Option<&'a crate::mission_fields::State>,
}
/// The payload is the reached actual native registry entry, with its source
/// descriptor fields. Previous state is NULL in every4D62F0 reentry call.
pub trait Reentry {
    fn state_reenter(
        &mut self,
        _catalog: &Catalog<'_>,
        _record: &mut Record<'_>,
        _index: u32,
        _source: aa_rules::states::ActorStateInputs,
        _pc: Precision,
    ) -> Result<()> {
        Err(Error::Unsupported(
            "4D4A90 actual saved Actor state-entry effects",
        ))
    }
}
fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}

/// Native477FC0 pre-registers this exact descriptor sequence into the embedded
/// ActorState manager (GameData+2C0, array+2E4/count+2E8). Source LoadData runs
/// in mode2 and cannot append new identities. The source registry records
/// authored load order, which is deliberately NOT the native descriptor order.
fn state_ids(catalog: &Catalog<'_>) -> Result<Vec<String>> {
    catalog.require_source_owner().map_err(error)?;
    {
        let bank = catalog.definition_bank.try_borrow().map_err(error)?;
        let source = bank.combat_source().map_err(error)?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err(Error::Unsupported("ActorState native source owner differs"));
        }
    }
    registered_state_ids(catalog)
}

///The same native table after the completed source Bank has moved into the
///ActorStore's mission. Never substitute the now-empty Catalog Bank.
pub(crate) fn state_ids_in_bank(catalog:&Catalog<'_>,bank:&crate::definitions::Bank)->Result<Vec<String>>{
    catalog.require_source_owner().map_err(error)?;
    let source=bank.combat_source().map_err(error)?;
    source.completed_means().map_err(error)?;
    if source.source_identities()!=catalog.actors.sources.as_slice(){
        return Err(Error::Unsupported("ActorState native source owner differs"));
    }
    registered_state_ids(catalog)
}

fn registered_state_ids(catalog:&Catalog<'_>)->Result<Vec<String>>{
    let manager = catalog
        .actors
        .native_data_registry("ActorState")
        .ok_or(Error::Unsupported("actual native ActorState manager"))?;
    if manager.mode != Some(2) || !manager.manager.eq_ignore_ascii_case("ActorState") {
        return Err(Error::Unsupported(
            "native ActorState manager mode/identity differs",
        ));
    }
    for entry in &manager.entries {
        if !aa_actors::state::DESCRIPTORS
            .iter()
            .any(|descriptor| entry.ids.eq_ignore_ascii_case(descriptor.name))
        {
            return Err(Error::Unsupported(
                "mode2 source contains a non-native ActorState identity",
            ));
        }
    }
    // The constructor's actual fixed semantic table is proved by executing
    //477FC0+474270+473490, including each DataManager+2C0 insertion receiver.
    // Optional/unreached authored descriptor payloads are not read here.
    Ok(aa_actors::state::DESCRIPTORS
        .iter()
        .map(|d| d.name.to_owned())
        .collect())
}

/// Called by4F9510's real Actor branch after its own loading/name writes.
/// This helper does not repeat4F9510, class lists, names or4F4C40 publication.
#[allow(clippy::too_many_arguments)]
pub fn register(
    catalog: &Catalog<'_>,
    id: ActorId,
    record: &mut RestoredRecord,
    registry: &mut IdentityRegistry<RestoredRecord>,
    grid: GridContext<'_>,
    effects: &mut impl Reentry,
    pc: Precision,
) -> Result<()> {
    if id.is_null()
        || !registry
            .get(id)
            .is_some_and(|e| matches!(&e.value, RestoredRecord::Loading))
    {
        return Err(Error::Unsupported("4D62F0 current GTID Loading owner"));
    }
    let (shell, definition, saved_id) = match record {
        RestoredRecord::Actor(e) => (&mut e.shell, &e.definition, e.source.serialized_id),
        RestoredRecord::Kingdom(e) => {
            let r = e
                .runtime
                .as_mut()
                .ok_or(Error::Unsupported("Kingdom runtime shell"))?;
            (&mut r.base, &r.definition, e.source.serialized_id)
        }
        _ => {
            return Err(Error::Unsupported(
                "4D62F0 requires actual Actor/Kingdom allocation",
            ));
        }
    };
    if saved_id != id.raw()
        || !matches!(definition.native_class, 3 | 4)
        || shell.kind != aa_actors::loading::LoadKind::Actor
        || shell.actor.is_none()
        || shell.object.is_none()
        || !crate::load_spatial::same_proof(&shell.thing.definition, &definition.proof)
    {
        return Err(Error::Unsupported(
            "4D62F0 held Actor class/identity differs",
        ));
    }
    let ids = state_ids(catalog)?;
    actor_registration::register(&mut Borrowed {
        catalog,
        record: Record {
            owner: id,
            shell,
            definition,
            registry,
        },
        ids,
        grid,
        effects,
        pc,
    })
}

struct Borrowed<'c, 's, 'r, 'g, 'e, E> {
    catalog: &'c Catalog<'s>,
    record: Record<'r>,
    ids: Vec<String>,
    grid: GridContext<'g>,
    effects: &'e mut E,
    pc: Precision,
}
impl<E: Reentry> actor_registration::World for Borrowed<'_, '_, '_, '_, '_, E> {
    fn registered_state_count(&mut self) -> Result<i32> {
        i32::try_from(self.ids.len()).map_err(error)
    }
    fn state_bits(&mut self) -> Result<StateBits> {
        Ok(self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or(Error::Unsupported("current Actor shell"))?
            .components
            .state)
    }
    fn state_reenter(&mut self, index: u32) -> Result<()> {
        let name = self
            .ids
            .get(index as usize)
            .ok_or(Error::Unsupported("current ActorState index"))?;
        let source_pc = {
            let bank = self.catalog.definition_bank.try_borrow().map_err(error)?;
            let state = bank.combat_source().map_err(error)?;
            if state.source_identities() != self.catalog.actors.sources.as_slice() {
                return Err(Error::Unsupported("state reentry source FP owner differs"));
            }
            state.source_precision()
        };
        if !self
            .catalog
            .actors
            .native_data_find("ActorState", name)
            .is_some_and(|entry| entry.ids.eq_ignore_ascii_case(name))
        {
            return Err(Error::Unsupported(
                "native indexed ActorState source identity is absent/shadowed",
            ));
        }
        let source = self
            .catalog
            .actors
            .native_actor_state(name, source_pc)
            .map_err(error)?;
        if !source.id.eq_ignore_ascii_case(name) {
            return Err(Error::Unsupported(
                "ActorState canonical identity is shadowed",
            ));
        }
        let expected_owner = self.record.owner;
        let result =
            self.effects
                .state_reenter(self.catalog, &mut self.record, index, source, self.pc);
        // The concrete callback may mutate other current records. The held
        // allocation's generation/class/source remain the same owner, including
        // when the child stopped after earlier native stores.
        let owner = if self.record.owner != expected_owner
            || !self
                .record
                .registry
                .get(self.record.owner)
                .is_some_and(|entry| matches!(&entry.value, RestoredRecord::Loading))
            || self.record.shell.kind != aa_actors::loading::LoadKind::Actor
            || self.record.shell.actor.is_none()
            || self.record.shell.object.is_none()
            || !crate::load_spatial::same_proof(
                &self.record.shell.thing.definition,
                &self.record.definition.proof,
            ) {
            Err(Error::Unsupported(
                "4D4A90 current held generation/class/source ownership changed",
            ))
        } else {
            Ok(())
        };
        match (result, owner) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(prior), Ok(())) => Err(prior),
            (Ok(()), Err(owner)) => Err(owner),
            (Err(prior), Err(owner)) => Err(error(format!("{prior}; {owner}"))),
        }
    }
    fn grid_register(&mut self) -> Result<()> {
        let source = Source::bind(self.catalog, self.record.definition)?;
        crate::load_zone_entry::grid_register_bound(
            self.catalog,
            &mut self.record,
            &source,
            self.grid.zones,
            self.grid.spatial,
            self.grid.point,
            self.grid.session,
            self.grid.world,
            self.grid.city.as_mut(),
            self.grid.live_world48,
            self.pc,
        )
    }
}
