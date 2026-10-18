//! Owned `4F1E10` world-start binding over staged ActorStore and Kingdom state.
//! This is a startup operation, not publication or a runnable battle. Callers
//! supply the actual world+274 byte and world+154 kingdom order. This module
//! cannot substitute MAP_HEADER player slots or an owner scan for that list.
//! Tracking projects validated component/source facts before its first category
//! write. Rejection of an unsupported component or source is a preflight error;
//! its intermediate effects do not claim native invalid-pointer/error parity.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent, Source, SpecialSource},
    restoration::{KingdomEconomyLoad, StatisticsSnapshotEntry},
};
use aa_actors::{
    ActorId,
    components::{ComponentSlot, Components},
};
use aa_economy::{
    EconomyError, F32Bits, X87Precision,
    statistics_live::{self, DerivedCatalog, TrackingFacts, WeightedTerm},
    world_start::{self, OwnedWorldStart, WorldStart},
};
use aa_mapworld::special_state::SpecialShell;
use aa_numeric::Precision;
use aa_rules::actors::ActorCatalog;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, EconomyError>;
fn error(value: impl std::fmt::Display) -> EconomyError {
    EconomyError::World(value.to_string())
}
fn pc(value: Precision) -> X87Precision {
    match value {
        Precision::Single24 => X87Precision::Single24,
        Precision::Double53 => X87Precision::Double53,
        Precision::Extended64 => X87Precision::Extended64,
    }
}

/// Bind GameData+574 in native registration/formula order, checked against
/// the same resource registry used by the actual economy owner.
pub fn bind_formulas(
    source: &ActorCatalog,
    resources: &aa_economy::Catalog,
    precision: Precision,
) -> Result<DerivedCatalog> {
    let loaded = source.native_statistics_catalog(precision).map_err(error)?;
    if loaded
        .resource_ids
        .iter()
        .map(String::as_str)
        .ne(resources.resources.iter().map(|r| r.id.as_str()))
    {
        return Err(error(
            "statistics and economy resource registry identities/order differ",
        ));
    }
    Ok(DerivedCatalog {
        rows: loaded
            .derived
            .iter()
            .map(|row| {
                row.terms
                    .iter()
                    .map(|term| WeightedTerm {
                        index: term.stat_index as usize,
                        weight: F32Bits(term.weight_bits),
                    })
                    .collect()
            })
            .collect(),
    })
}

pub struct World<'a> {
    pub actors: &'a mut ActorStore,
    pub economy: &'a mut BTreeMap<u32, KingdomEconomyLoad>,
    /// Actual caller-owned world+274 state; no constructor value is guessed.
    pub started: &'a mut bool,
    /// Native world+154/+158, distinct from player/Kingdom slots at+148.
    pub major_kingdoms: &'a [Option<ActorId>],
    pub source: &'a ActorCatalog,
    pub resources: &'a aa_economy::Catalog,
    pub formulas: &'a DerivedCatalog,
    /// Source materialization uses the actual retained Bank's PC, separately
    /// from arithmetic performed by the running World below.
    pub source_precision: Precision,
    pub precision: Precision,
}

impl World<'_> {
    pub fn start(&mut self) -> Result<()> {
        world_start::world_start_owned(self.resources.resources.len(), self)
    }
    fn key(&self, id: u32) -> Result<ObjectKey> {
        self.actors
            .key(ActorId::from_raw(id))
            .ok_or_else(|| error(format!("world-start object {id:#x} is not live")))
    }
    fn kingdom(&self, id: u32) -> Result<&SpecialShell> {
        match &self.actors.entity(self.key(id)?).map_err(error)?.source {
            Source::Special(s) if matches!(s.source, SpecialSource::Kingdom(_)) => s
                .derived
                .as_ref()
                .ok_or_else(|| error("Kingdom derived state missing")),
            _ => Err(error("world-start owner is not a Kingdom")),
        }
    }
    fn kingdom_mut(&mut self, id: u32) -> Result<&mut SpecialShell> {
        let key = self.key(id)?;
        match &mut self.actors.entity_mut(key).map_err(error)?.source {
            Source::Special(s) if matches!(s.source, SpecialSource::Kingdom(_)) => s
                .derived
                .as_mut()
                .ok_or_else(|| error("Kingdom derived state missing")),
            _ => Err(error("world-start owner is not a Kingdom")),
        }
    }
    fn economy(&self, id: usize) -> Result<&KingdomEconomyLoad> {
        self.economy
            .get(&(id as u32))
            .ok_or_else(|| error("Kingdom statistics owner missing"))
    }
    fn economy_mut(&mut self, id: usize) -> Result<&mut KingdomEconomyLoad> {
        self.economy
            .get_mut(&(id as u32))
            .ok_or_else(|| error("Kingdom statistics owner missing"))
    }
    fn tracking_facts(&self, actor: u32) -> Result<TrackingFacts> {
        let object = self.actors.object(self.key(actor)?).map_err(error)?;
        let components = object.components().map_err(error)?;
        // Settlement is not a concrete owned component in this runtime yet.
        if components.present(ComponentSlot::Settlement) {
            return Err(error("58F010 Settlement+10 level owner is unavailable"));
        }
        let mut facts = TrackingFacts {
            component_7c: components.present(ComponentSlot::Organization),
            component_d8: components.present(ComponentSlot::Hq),
            structure_94: components.present(ComponentSlot::Structure),
            component_98_level: None,
            construction_9c: components.present(ComponentSlot::Construction),
            component_88: components.present(ComponentSlot::Denizen),
            component_d4: components.present(ComponentSlot::City),
            // These words are never read on a path without the components below.
            definition_4d0: false,
            definition_43c_nonempty: false,
            definition_4e0: 0,
        };
        if facts.structure_94 || facts.construction_9c {
            let canonical = &object.definition.proof.id;
            if !self
                .source
                .find(canonical)
                .is_some_and(|d| d.id.eq_ignore_ascii_case(canonical))
            {
                return Err(error(
                    "canonical statistics ActorData is shadowed by a translation",
                ));
            }
            let definition = self
                .source
                .native_statistics_tracking_definition(canonical, self.source_precision)
                .map_err(error)?;
            facts.definition_4d0 = definition.predecessor_nonempty;
            facts.definition_43c_nonempty = !definition.technology_property_ids.is_empty();
            facts.definition_4e0 = u32::from(definition.required_object_ids.is_some());
        }
        Ok(facts)
    }
}

/// GActor/Kingdom/Group virtual+100 is4CED60: non-null Structure+10,
/// otherwise Wall+10 via58CB30. No simulation callback mutates components
/// during58F010, so this borrows those same component boxes beside Actor+13C.
fn parent_present(components: &Components<RuntimeComponent>) -> Result<bool> {
    match components.get(ComponentSlot::Structure) {
        Some(RuntimeComponent::Structure(v)) if v.object_handle_10 != 0 => return Ok(true),
        Some(RuntimeComponent::Structure(_)) | None => {}
        Some(_) => return Err(error("Structure slot payload mismatch")),
    }
    if components.present(ComponentSlot::Wall) {
        return Err(error("4CED60 Wall+10 concrete owner is unavailable"));
    }
    Ok(false)
}

impl WorldStart for World<'_> {
    fn objects(&mut self) -> Result<Vec<u32>> {
        Ok(self
            .actors
            .identities()
            .iter()
            .map(|entry| entry.id.raw())
            .collect())
    }
    fn object_active(&mut self, id: u32) -> Result<bool> {
        // Registry classification is made by the native class constructor;
        // saved classes are also checked against the recovered vtables.
        let key = self.key(id)?;
        Ok(
            crate::post_load_world::object_class(self.actors.entity(key).map_err(error)?)
                .map_err(error)?
                .is_actor(),
        )
    }
    fn field_13c(&mut self, actor: u32) -> Result<u32> {
        self.actors
            .object(self.key(actor)?)
            .map_err(error)?
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&0x13c))
            .copied()
            .ok_or_else(|| error("Actor+13C is uninitialized"))
    }
    fn set_field_13c(&mut self, actor: u32, value: u32) -> Result<()> {
        let key = self.key(actor)?;
        let a = self
            .actors
            .object_mut(key)
            .map_err(error)?
            .actor
            .as_mut()
            .ok_or_else(|| error("Actor shell missing"))?;
        let field = a
            .words
            .get_mut(&0x13c)
            .ok_or_else(|| error("Actor+13C is uninitialized"))?;
        *field = value;
        Ok(())
    }
    fn owner(&mut self, actor: u32) -> Result<Option<usize>> {
        Ok(self
            .actors
            .ordinary_owner(ActorId::from_raw(actor))
            .map_err(error)?
            .map(|id| id.raw() as usize))
    }
    fn kingdom_word_190(&mut self, id: usize) -> Result<u32> {
        self.kingdom(id as u32)?
            .words
            .get(&0x190)
            .copied()
            .ok_or_else(|| error("Kingdom+190 is uninitialized"))
    }
    fn statistics_track(&mut self, kingdom: usize, actor: u32) -> Result<()> {
        let facts = self.tracking_facts(actor)?;
        let key = self.key(actor)?;
        let a = self
            .actors
            .object_mut(key)
            .map_err(error)?
            .actor
            .as_mut()
            .ok_or_else(|| error("Actor shell missing"))?;
        let field = a
            .words
            .get_mut(&0x13c)
            .ok_or_else(|| error("Actor+13C is uninitialized"))?;
        let stats = self
            .economy
            .get_mut(&(kingdom as u32))
            .and_then(|e| e.statistics.active.as_mut())
            .ok_or_else(|| error("58F010 active statistics are null"))?;
        statistics_live::track_actor(
            stats,
            field,
            &facts,
            *self.started,
            pc(self.precision),
            || parent_present(&a.components),
        )
    }
    fn statistics_snapshot(&mut self, id: usize) -> Result<()> {
        let kingdom = self.kingdom(id as u32)?;
        let vector = |offset| {
            kingdom.vectors.get(&offset).map(|v| v.values.iter().copied().map(F32Bits).collect::<Vec<_>>())
            .ok_or_else(|| error(format!("Kingdom vector{offset:X} has moved or is missing; bind its actual economy owner")))
        };
        let provided = vector(0x1a0)?;
        let consumed = vector(0x1b8)?;
        // +214/+218 is the singles vector, already moved into its typed owner.
        // No duplicate scalar count is retained in SpecialShell.words.
        let count = i32::try_from(kingdom.recruit_items.singles.len()).map_err(|_| {
            error("recruit count exceeds the native signed statistic representation")
        })?;
        let economy = self.economy(id)?;
        let stats = economy
            .statistics
            .active
            .as_ref()
            .ok_or_else(|| error("58F530 active statistics are null"))?;
        // The active relation object was just promoted for this Kingdom. Its
        // owner identity remains stable throughout this one world-start pass.
        let relation = economy.statistics.relations.as_ref().map(|_| id as u32);
        let snapshot = statistics_live::snapshot(
            stats,
            self.resources,
            &provided,
            &consumed,
            count,
            relation,
            self.formulas,
            pc(self.precision),
        )?;
        // 45F240 appends after the current tail; snapshots are chronological.
        self.economy_mut(id)?
            .snapshots
            .push(StatisticsSnapshotEntry::Live(snapshot));
        Ok(())
    }
}
impl OwnedWorldStart for World<'_> {
    fn started(&mut self) -> Result<bool> {
        Ok(*self.started)
    }
    fn set_started(&mut self, value: bool) -> Result<()> {
        *self.started = value;
        Ok(())
    }
    fn kingdoms(&mut self) -> Result<Vec<Option<usize>>> {
        Ok(self
            .major_kingdoms
            .iter()
            .map(|id| id.map(|id| id.raw() as usize))
            .collect())
    }
    fn promote_and_reset_statistics(&mut self, id: usize, count: usize) -> Result<()> {
        self.kingdom(id as u32)?;
        self.economy_mut(id)?.statistics.start_world(count);
        Ok(())
    }
    fn has_snapshots(&mut self, id: usize) -> Result<bool> {
        Ok(!self.economy(id)?.snapshots.is_empty())
    }
    fn reset_snapshot_timer(&mut self, id: usize) -> Result<()> {
        let timer = self
            .kingdom_mut(id as u32)?
            .words
            .get_mut(&0x334)
            .ok_or_else(|| error("Kingdom+334 is uninitialized"))?;
        *timer = 0;
        Ok(())
    }
}
