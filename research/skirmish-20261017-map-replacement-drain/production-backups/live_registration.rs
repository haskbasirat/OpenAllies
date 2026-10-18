//! Native GWorld4F9510 registration and its4F4C40 available-city branch.
//!
//! This owner is moved with the registry, never reconstructed from its sorted
//! traversal. Object Init's World+8 list, Actor4D62F0 state/grid reentry and
//! full world startup are separate services. Completion here is not readiness.
use crate::{catalog::Catalog, names};
use aa_actors::{ActorId, IdentityRegistry, components::ComponentSlot};
use aa_mapworld::restore::{RestoredRecord, RuntimeDefinition};
use aa_numeric::Precision;

#[cfg(test)]
#[path = "live_registration_tests.rs"]
mod tests;

pub type Result<T> = std::result::Result<T, String>;

/// Pointer-bearing native lists use stable, generation-bearing object handles;
/// available cities already stores GTIDs natively. Head-first duplicates survive.
#[derive(Debug)]
pub struct State {
    names: names::Table,
    areas: Vec<ActorId>,            //World+188/+18C (virtual+BC)
    cameras: Vec<ActorId>,          //World+178/+17C (virtual+C0)
    teams: Vec<Option<ActorId>>,    //actual World+16C indexed array
    available_cities: Vec<ActorId>, //World+260/+264
    duplicate_assert_latch: bool,   //process-global B3CC40, explicitly supplied
}

impl State {
    ///4FCA7B..86 passes the decoded TEAMS count to430240, which initializes
    /// exactly that many null team pointers before loading the records.
    /// The constructor's capacity8 is not retained as a limit.
    pub fn from_saved_team_count(count: u32, duplicate_assert_latch: bool) -> Result<Self> {
        if count > i32::MAX as u32 {
            return Err(
                "430240 negative signed team count is outside the owned array domain".into(),
            );
        }
        let mut teams = Vec::new();
        teams
            .try_reserve_exact(count as usize)
            .map_err(|_| "430240 team array allocation failed")?;
        teams.resize(count as usize, None);
        Ok(Self::from_world_constructor(teams, duplicate_assert_latch))
    }

    ///4F7D00 initializes empty class/city lists and a59-bucket name table.
    /// The team array's constructor capacity8 is not its initialized length.
    /// Its actual initialized slots must be supplied by the world-load owner.
    /// B3CC40 is process-global, not reset by this world constructor.
    pub fn from_world_constructor(
        initialized_team_slots: Vec<Option<ActorId>>,
        duplicate_assert_latch: bool,
    ) -> Self {
        Self {
            names: names::Table::default(),
            areas: Vec::new(),
            cameras: Vec::new(),
            teams: initialized_team_slots,
            available_cities: Vec::new(),
            duplicate_assert_latch,
        }
    }
    pub fn names(&self) -> &names::Table {
        &self.names
    }
    pub fn names_mut(&mut self) -> &mut names::Table {
        &mut self.names
    }
    pub fn areas(&self) -> &[ActorId] {
        &self.areas
    }
    pub fn cameras(&self) -> &[ActorId] {
        &self.cameras
    }
    ///4F5490 removes only the first matching native classification node.
    /// The clock dispatcher borrows these same lists after loading.
    pub fn remove_classification(&mut self,list:aa_world::ListKind,id:ActorId)->Result<()> {
        let values=match list {
            aa_world::ListKind::Class178=>&mut self.cameras,
            aa_world::ListKind::Class188=>&mut self.areas,
            _=>return Err("classification removal requires native World178 or188 list".into()),
        };
        if let Some(index)=values.iter().position(|v|*v==id) {values.remove(index);}
        Ok(())
    }
    pub fn teams(&self) -> &[Option<ActorId>] {
        &self.teams
    }
    pub fn available_cities(&self) -> &[ActorId] {
        &self.available_cities
    }
    pub fn duplicate_assert_latch(&self) -> bool {
        self.duplicate_assert_latch
    }

    /// The same lists/name owner after the registry's values move into the
    /// ActorStore. This only resolves identities; it does not confer readiness.
    pub fn stored<'a>(&'a self, actors: &'a crate::actor_world::ActorStore) -> Stored<'a> {
        Stored {
            state: self,
            actors,
        }
    }

    ///4F9510 then4F4C40(insert=1), with immediate stores and no rollback.
    /// World supplies live fields at each native boundary, including after
    /// Actor registration and after the duplicate assertion callback.
    pub fn register(&mut self, world: &mut impl World) -> Result<()> {
        let name = world.capture_name()?; //4BEDF0 precedes the flags store
        world.clear_loading_flag()?;
        if !name.is_empty() {
            self.names.remove_first(&name).map_err(|e| e.to_string())?;
            self.names
                .alias(&name, world.id()?)
                .map_err(|e| e.to_string())?;
        }
        if world.predicate(0xbc)? {
            prepend(&mut self.areas, world.id()?)?;
        } else if world.predicate(0xc0)? {
            prepend(&mut self.cameras, world.id()?)?;
        } else if world.predicate(0xac)? {
            let slot = usize::try_from(world.team_slot()?).map_err(|_| "negative Team+38")?;
            let id = world.id()?;
            let current = self
                .teams
                .get_mut(slot)
                .ok_or("Team+38 outside initialized World+16C array")?;
            if current.is_some_and(|old| old != id) {
                return Err("4F9510 Loading Exception GWorld.cpp:3329: m_all_teams[id]==NULL || m_all_teams[id]==team".into());
            }
            *current = Some(id);
        } else if world.predicate(0xb4)? {
            world.register_actor()?; //mandatory4D62F0: state reentry then grid
        }
        self.publish_available_city(world)
    }

    fn publish_available_city(&mut self, world: &mut impl World) -> Result<()> {
        let direct = world.predicate(0xb0)? && world.settlement_space()?;
        if !direct {
            if !world.predicate(0xb4)? || !world.city_present()? {
                return Ok(());
            }
            if world.required_city_live()? && world.required_city_settlement_space()? {
                return Ok(());
            }
        }
        if !self.duplicate_assert_latch {
            let id = world.id()?;
            if self.available_cities.contains(&id) {
                self.duplicate_assert_latch = world.assert_duplicate_available_city()?;
            }
        }
        //4F4D15 rereads GTID after the assertion, even if it returned false.
        prepend(&mut self.available_cities, world.id()?)
    }
}

pub struct Stored<'a> {
    state: &'a State,
    actors: &'a crate::actor_world::ActorStore,
}
impl Stored<'_> {
    pub fn key(&self, id: ActorId) -> Result<crate::actor_world::ObjectKey> {
        self.actors.key(id).ok_or_else(|| {
            format!(
                "4F9510 retained object {:#010x} is no longer current",
                id.raw()
            )
        })
    }
    /// Native name lookup rejects a stale first alias without trying older ones.
    pub fn named(&self, name: &[u16]) -> Result<Option<crate::actor_world::ObjectKey>> {
        Ok(self
            .state
            .names
            .lookup(name)
            .map_err(|e| e.to_string())?
            .and_then(|id| self.actors.key(id)))
    }
    pub fn areas(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<crate::actor_world::ObjectKey>> + '_ {
        self.state.areas.iter().map(|id| self.key(*id))
    }
    pub fn cameras(
        &self,
    ) -> impl ExactSizeIterator<Item = Result<crate::actor_world::ObjectKey>> + '_ {
        self.state.cameras.iter().map(|id| self.key(*id))
    }
    pub fn team(&self, slot: usize) -> Result<Option<crate::actor_world::ObjectKey>> {
        self.state
            .teams
            .get(slot)
            .ok_or("World+16C team slot outside initialized array")?
            .map(|id| self.key(id))
            .transpose()
    }
}

fn prepend(list: &mut Vec<ActorId>, id: ActorId) -> Result<()> {
    list.try_reserve(1)
        .map_err(|_| "4F9510 list allocation failed")?;
    list.insert(0, id);
    Ok(())
}

/// Native service boundaries; no method has an implicit successful default.
pub trait World {
    fn capture_name(&mut self) -> Result<Vec<u16>>;
    fn clear_loading_flag(&mut self) -> Result<()>;
    fn id(&mut self) -> Result<ActorId>;
    fn predicate(&mut self, offset: u32) -> Result<bool>;
    fn team_slot(&mut self) -> Result<i32>;
    fn register_actor(&mut self) -> Result<()>;
    fn settlement_space(&mut self) -> Result<bool>;
    fn city_present(&mut self) -> Result<bool>;
    fn required_city_live(&mut self) -> Result<bool>;
    fn required_city_settlement_space(&mut self) -> Result<bool>;
    fn assert_duplicate_available_city(&mut self) -> Result<bool>;
}

/// Required effect owner.4D62F0 must run real current-state reentry and grid
/// insertion; merely transferring a placement into SpatialWorld is insufficient.
/// The held allocation/GTID/class/source identity remains stable throughout the
/// callback; mutable live state, flags and references may change and are reread.
pub trait Effects {
    fn register_actor(
        &mut self,
        id: ActorId,
        record: &mut RestoredRecord,
        registry: &mut IdentityRegistry<RestoredRecord>,
        catalog: &Catalog<'_>,
        pc: Precision,
    ) -> Result<()>;
    fn assert_duplicate_available_city(&mut self) -> Result<bool>;
}

/// Borrows the actual held record while its registry entry may be Loading.
/// Neither the record nor its shell/components/source are cloned. The current mutable
/// registry and immutable Catalog remain the exact current owners for referenced cities.
pub fn register_restored(
    state: &mut State,
    id: ActorId,
    record: &mut RestoredRecord,
    registry: &mut IdentityRegistry<RestoredRecord>,
    catalog: &Catalog<'_>,
    effects: &mut impl Effects,
    pc: Precision,
) -> Result<()> {
    if !registry
        .get(id)
        .is_some_and(|entry| matches!(&entry.value, RestoredRecord::Loading))
    {
        return Err("4F9510 held record has no current GTID Loading owner".into());
    }
    let saved_id = match &*record {
        RestoredRecord::Team(v) => v.source.serialized_id,
        RestoredRecord::Kingdom(v) => v.source.serialized_id,
        RestoredRecord::Thing(v) | RestoredRecord::Object(v) | RestoredRecord::Actor(v) => {
            v.source.serialized_id
        }
        RestoredRecord::Loading => return Err("4F9510 actual held record is absent".into()),
    };
    if saved_id != id.raw() {
        return Err("4F9510 held record saved identity mismatch".into());
    }
    state.register(&mut Restored {
        id,
        record,
        registry,
        catalog,
        effects,
        pc,
    })
}

struct Restored<'a, 'source, E> {
    id: ActorId,
    record: &'a mut RestoredRecord,
    registry: &'a mut IdentityRegistry<RestoredRecord>,
    catalog: &'a Catalog<'source>,
    effects: &'a mut E,
    pc: Precision,
}

fn definition(record: &RestoredRecord) -> Result<&RuntimeDefinition> {
    match record {
        RestoredRecord::Thing(v) | RestoredRecord::Object(v) | RestoredRecord::Actor(v) => {
            Ok(&v.definition)
        }
        RestoredRecord::Team(v) => Ok(&v.runtime.as_ref().ok_or("Team runtime absent")?.definition),
        RestoredRecord::Kingdom(v) => Ok(&v
            .runtime
            .as_ref()
            .ok_or("Kingdom runtime absent")?
            .definition),
        RestoredRecord::Loading => Err("4F9510 actual held record is absent".into()),
    }
}
fn shell(
    record: &RestoredRecord,
) -> Result<
    &aa_actors::loading::LoadShell<
        aa_mapworld::bootstrap::DefinitionProof,
        aa_mapworld::restore::RestoredComponent,
    >,
> {
    match record {
        RestoredRecord::Thing(v) | RestoredRecord::Object(v) | RestoredRecord::Actor(v) => {
            Ok(&v.shell)
        }
        RestoredRecord::Team(v) => Ok(&v.runtime.as_ref().ok_or("Team runtime absent")?.base),
        RestoredRecord::Kingdom(v) => Ok(&v.runtime.as_ref().ok_or("Kingdom runtime absent")?.base),
        RestoredRecord::Loading => Err("4F9510 actual held shell is absent".into()),
    }
}
fn flags(record: &mut RestoredRecord) -> Result<&mut u32> {
    match record {
        RestoredRecord::Thing(v) | RestoredRecord::Object(v) | RestoredRecord::Actor(v) => {
            Ok(&mut v.shell.thing.flags)
        }
        RestoredRecord::Team(v) => Ok(&mut v
            .runtime
            .as_mut()
            .ok_or("Team runtime absent")?
            .base
            .thing
            .flags),
        RestoredRecord::Kingdom(v) => Ok(&mut v
            .runtime
            .as_mut()
            .ok_or("Kingdom runtime absent")?
            .base
            .thing
            .flags),
        RestoredRecord::Loading => Err("4F9510 actual held flags are absent".into()),
    }
}
/// Original saved-load constructors install these vtables; pure class leaves
/// are6A71D0(true)/6A71F0(false). Group8 has no owned restore constructor yet.
pub fn class_predicate(class: u32, offset: u32) -> Result<bool> {
    if !matches!(class, 1..=7 | 9) {
        return Err(format!("4F9510 unsupported runtime class {class}"));
    }
    Ok(match offset {
        0xbc => class == 6,
        0xc0 => class == 7,
        0xac => class == 5,
        0xb0 => matches!(class, 2 | 3 | 4 | 6 | 7 | 9),
        0xb4 => matches!(class, 3 | 4),
        _ => return Err(format!("4F9510 unproved predicate +{offset:X}")),
    })
}
fn settlement_space(catalog: &Catalog<'_>, record: &RestoredRecord, pc: Precision) -> Result<bool> {
    let id = &definition(record)?.proof.id;
    catalog.require_untranslated_actor_identity(id)?;
    catalog
        .actors
        .native_field(id, "needs_space_for_settlement_zone", pc)
        .map_err(|e| e.to_string())?
        .ok_or("ObjectData+34 settlement-space source field absent")?
        .boolean()
        .map_err(|e| e.to_string())
}
impl<E: Effects> World for Restored<'_, '_, E> {
    fn capture_name(&mut self) -> Result<Vec<u16>> {
        Ok(shell(self.record)?.thing.name_utf16.clone())
    }
    fn clear_loading_flag(&mut self) -> Result<()> {
        *flags(self.record)? &= 0x7fff_ffff;
        Ok(())
    }
    fn id(&mut self) -> Result<ActorId> {
        Ok(self.id)
    }
    fn predicate(&mut self, offset: u32) -> Result<bool> {
        class_predicate(definition(self.record)?.native_class, offset)
    }
    fn team_slot(&mut self) -> Result<i32> {
        let RestoredRecord::Team(team) = &*self.record else {
            return Err("virtual+AC has no owned Team".into());
        };
        Ok(*team
            .runtime
            .as_ref()
            .and_then(|v| v.derived.as_ref())
            .and_then(|v| v.words.get(&0x38))
            .ok_or("Team+38 is uninitialized")? as i32)
    }
    fn register_actor(&mut self) -> Result<()> {
        let expected = {
            let d = definition(self.record)?;
            (
                d.proof.id.clone(),
                d.proof.source.clone(),
                d.proof.template_chain.clone(),
                d.proof.category,
                d.native_class,
                d.flags,
            )
        };
        let result =
            self.effects
                .register_actor(self.id, self.record, self.registry, self.catalog, self.pc);
        let ownership = (|| -> Result<()> {
            if !self
                .registry
                .get(self.id)
                .is_some_and(|entry| matches!(&entry.value, RestoredRecord::Loading))
            {
                return Err("4D62F0 current GTID Loading ownership changed".into());
            }
            let id = match &*self.record {
                RestoredRecord::Team(e) => e.source.serialized_id,
                RestoredRecord::Kingdom(e) => e.source.serialized_id,
                RestoredRecord::Thing(e) | RestoredRecord::Object(e) | RestoredRecord::Actor(e) => {
                    e.source.serialized_id
                }
                RestoredRecord::Loading => {
                    return Err("4D62F0 held allocation was replaced by Loading".into());
                }
            };
            let d = definition(self.record)?;
            if id != self.id.raw()
                || d.proof.id != expected.0
                || d.proof.source != expected.1
                || d.proof.template_chain != expected.2
                || d.proof.category != expected.3
                || d.native_class != expected.4
                || d.flags != expected.5
            {
                return Err("4D62F0 current held class/source identity changed".into());
            }
            Ok(())
        })();
        match (result, ownership) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), Ok(())) => Err(error),
            (Ok(()), Err(error)) => Err(error),
            (Err(prior), Err(error)) => Err(format!("{prior}; {error}")),
        }
    }
    fn settlement_space(&mut self) -> Result<bool> {
        settlement_space(self.catalog, self.record, self.pc)
    }
    fn city_present(&mut self) -> Result<bool> {
        Ok(shell(self.record)?
            .actor
            .as_ref()
            .ok_or("Actor shell absent")?
            .components
            .get(ComponentSlot::City)
            .is_some())
    }
    fn required_city_live(&mut self) -> Result<bool> {
        let word = *shell(self.record)?
            .actor
            .as_ref()
            .ok_or("Actor shell absent")?
            .words
            .get(&0x158)
            .ok_or("Actor+158 is uninitialized")?;
        Ok(self.registry.get(ActorId::from_raw(word)).is_some())
    }
    fn required_city_settlement_space(&mut self) -> Result<bool> {
        let word = *shell(self.record)?
            .actor
            .as_ref()
            .ok_or("Actor shell absent")?
            .words
            .get(&0x158)
            .ok_or("Actor+158 is uninitialized")?;
        let other = if ActorId::from_raw(word) == self.id {
            &*self.record
        } else {
            &self
                .registry
                .get(ActorId::from_raw(word))
                .ok_or("required city no longer current")?
                .value
        };
        settlement_space(self.catalog, other, self.pc)
    }
    fn assert_duplicate_available_city(&mut self) -> Result<bool> {
        self.effects.assert_duplicate_available_city()
    }
}
