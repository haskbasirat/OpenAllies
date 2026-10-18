//! Concrete4E6180 on the actual held Actor and its stored current Kingdom.
//! Class/list/scalar writes stay in their original owners on every error.
use crate::{
    catalog::Catalog,
    live_object_init::{Record, Source},
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_economy::{
    EconomyError,
    kingdom_registry::{
        self as native, ActorList, PropertyApplication, RegistryActor, RegistryWorld,
        StoredRegistryWorld,
    },
};
use aa_mapworld::{
    restore::{RestoredComponent, RestoredRecord},
    special_state::SpecialShell,
};
use aa_numeric::Precision;
type Result<T> = std::result::Result<T, EconomyError>;
fn error(e: impl std::fmt::Display) -> EconomyError {
    EconomyError::World(e.to_string())
}
fn required<T>(service: &'static str) -> Result<T> {
    Err(error(service))
}
fn actor_word(record: &Record<'_>, offset: u32) -> Result<u32> {
    record
        .shell
        .actor
        .as_ref()
        .and_then(|a| a.words.get(&offset))
        .copied()
        .ok_or_else(|| {
            error(format!(
                "4E6180 actual Actor+{offset:X} word is unavailable"
            ))
        })
}

/// Reached from4CF300 only after the actual Actor+E8 owner lookup. This does
/// not reconstruct Kingdom lists from a later GTID scan or clean diagnostic.
#[derive(Clone, Copy)]
enum EconomyBinding {
    Native,
    #[cfg(test)]
    RequiredStop,
}
pub fn add_actor(
    catalog: &Catalog<'_>,
    inventory: &aa_mapworld::WorldInventory,
    source_fields: &aa_rules::native_source_requirements::NativeSourceRequirements,
    record: &mut Record<'_>,
    owner: ActorId,
    pc: Precision,
) -> std::result::Result<(), String> {
    add_actor_bound(
        catalog,
        inventory,
        source_fields,
        record,
        owner,
        pc,
        EconomyBinding::Native,
    )
}
fn add_actor_bound(
    catalog: &Catalog<'_>,
    inventory: &aa_mapworld::WorldInventory,
    source_fields: &aa_rules::native_source_requirements::NativeSourceRequirements,
    record: &mut Record<'_>,
    owner: ActorId,
    pc: Precision,
    economy: EconomyBinding,
) -> std::result::Result<(), String> {
    let source = Source::bind(catalog, record.definition).map_err(|e| e.to_string())?;
    source.validate_record(record).map_err(|e| e.to_string())?;
    if !matches!(record.definition.native_class, 3 | 4) || record.shell.actor.is_none() {
        return Err("4E6180 actual Actor allocation is required".into());
    }
    if actor_word(record, 0xe8).map_err(|e| e.to_string())? != owner.raw()
        || owner.is_null()
        || owner == record.owner
    {
        return Err("4E6180 actual stored Kingdom owner differs/is held".into());
    }
    // The referenced stored Kingdom is bound independently of the held
    // Actor: agreeing mutable proof copies alone cannot replace source authority.
    {
        let Some(entry) = record.registry.get(owner) else {
            return Err("4E6180 stored Kingdom generation is absent".into());
        };
        let RestoredRecord::Kingdom(k) = &entry.value else {
            return Err("4E6180 owner is not a stored Kingdom".into());
        };
        let runtime = k
            .runtime
            .as_ref()
            .ok_or("4E6180 stored Kingdom runtime is absent")?;
        Source::bind(catalog, &runtime.definition).map_err(|e| e.to_string())?;
    }
    let current = record.owner;
    let mut world = World {
        catalog,
        inventory,
        source_fields,
        record,
        owner,
        pc,
        economy,
    };
    world.kingdom().map_err(|e| e.to_string())?;
    let result = native::add_actor_stored(current.raw(), &mut world).map_err(|e| e.to_string());
    let identity = source
        .validate_record(world.record)
        .map_err(|e| e.to_string())
        .and_then(|_| {
            if world.record.owner != current {
                return Err("4E6180 current held GTID changed".into());
            }
            world.kingdom().map(|_| ()).map_err(|e| e.to_string())
        });
    match (result, identity) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
        (Err(e), Err(owner)) => Err(format!("{e}; {owner}")),
    }
}
struct World<'a, 'r, 's> {
    catalog: &'a Catalog<'s>,
    inventory: &'a aa_mapworld::WorldInventory,
    source_fields: &'a aa_rules::native_source_requirements::NativeSourceRequirements,
    record: &'a mut Record<'r>,
    owner: ActorId,
    pc: Precision,
    economy: EconomyBinding,
}
impl World<'_, '_, '_> {
    fn kingdom(&self) -> Result<&SpecialShell> {
        let Some(entry) = self.record.registry.get(self.owner) else {
            return required("4E6180 current referenced Kingdom generation is absent");
        };
        let RestoredRecord::Kingdom(k) = &entry.value else {
            return required("4E6180 referenced owner is not a stored Kingdom");
        };
        if k.source.serialized_id != self.owner.raw() {
            return required("4E6180 stored Kingdom identity differs");
        }
        let runtime = k
            .runtime
            .as_ref()
            .ok_or_else(|| error("4E6180 Kingdom runtime is absent"))?;
        if runtime.definition.native_class != 4
            || runtime.base.kind != aa_actors::loading::LoadKind::Actor
            || !crate::load_spatial::same_proof(
                &runtime.base.thing.definition,
                &runtime.definition.proof,
            )
        {
            return required("4E6180 actual Kingdom runtime source/class differs");
        }
        let derived = runtime
            .derived
            .as_ref()
            .ok_or_else(|| error("4E6180 Kingdom derived storage is absent"))?;
        if derived.recruitment_owner != Some(self.owner) {
            return required("4E6180 stored Kingdom constructor owner differs");
        }
        Ok(derived)
    }
    fn kingdom_mut(&mut self) -> Result<&mut SpecialShell> {
        self.kingdom()?;
        let Some(RestoredRecord::Kingdom(k)) = self.record.registry.value_mut(self.owner) else {
            return required("4E6180 current Kingdom generation changed");
        };
        k.runtime
            .as_mut()
            .and_then(|r| r.derived.as_mut())
            .ok_or_else(|| error("4E6180 Kingdom derived owner disappeared"))
    }
    fn word(&self, offset: u32) -> Result<u32> {
        self.kingdom()?.words.get(&offset).copied().ok_or_else(|| {
            error(format!(
                "4E6180 actual Kingdom+{offset:X} scalar is unbound"
            ))
        })
    }
    fn vector_offset(list: ActorList) -> Result<u32> {
        match list {
            ActorList::Structures => Ok(0x2c4),
            ActorList::Settlements => Ok(0x2d0),
            ActorList::Hqs => Ok(0x2dc),
            ActorList::Organizations => Ok(0x2e8),
            ActorList::Flares => required("flare list is not a native vector"),
        }
    }
    fn require_current(&self, actor: u32) -> Result<()> {
        if actor != self.record.owner.raw() {
            return required("4E6180 callback requires another actual Actor allocation");
        }
        Ok(())
    }
}
impl RegistryWorld for World<'_, '_, '_> {
    fn actor(&self, actor: u32) -> Result<RegistryActor> {
        self.require_current(actor)?;
        let a = self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or_else(|| error("current Actor shell missing"))?;
        let category = self
            .catalog
            .actors
            .native_category(&self.record.definition.proof.id)
            .map_err(error)?;
        if category != self.record.definition.proof.category {
            return required("4E6180 canonical class descriptor differs");
        }
        let has = |s| a.components.present(s);
        Ok(RegistryActor {
            class_index: category.runtime_class,
            class_kind: category.registry_index,
            flags8: self.record.shell.thing.flags,
            states: u64::from(a.components.state.low) | (u64::from(a.components.state.high) << 32),
            economy: has(ComponentSlot::Economy),
            structure: has(ComponentSlot::Structure),
            settlement: has(ComponentSlot::Settlement),
            hq: has(ComponentSlot::Hq),
            organization: has(ComponentSlot::Organization),
            supply: has(ComponentSlot::Supply),
            properties: has(ComponentSlot::Properties),
            component_ac: has(ComponentSlot::MobileDeployment),
            flare: has(ComponentSlot::Flare),
        })
    }
    fn set_flags8(&mut self, actor: u32, flags: u32) -> Result<()> {
        self.require_current(actor)?;
        self.record.shell.thing.flags = flags;
        Ok(())
    }
    fn structure_excluded(&mut self, actor: u32) -> Result<bool> {
        self.require_current(actor)?;
        let components = &self
            .record
            .shell
            .actor
            .as_ref()
            .ok_or_else(|| error("current Actor shell"))?
            .components;
        if let Some(c) = components.get(ComponentSlot::Structure) {
            let RestoredComponent::Structure(s) = c else {
                return required("Structure payload differs");
            };
            if s.header.owner != self.record.owner {
                return required("Structure actual owner differs");
            }
            if s.object_handle_10 != 0 {
                return Ok(true);
            }
        }
        if components.present(ComponentSlot::Wall) {
            return required("4CED60/58CB30 actual Wall result service");
        }
        Ok(false)
    }
    fn economy_add(&mut self, actor: u32) -> Result<()> {
        self.require_current(actor)?;
        match self.economy {
            EconomyBinding::Native => crate::load_economy_refresh_world::refresh(
                self.catalog,
                self.inventory,
                self.record,
                self.pc,
            )
            .map_err(error),
            #[cfg(test)]
            EconomyBinding::RequiredStop => {
                required("5704C0/570020 explicit native-prefix test effect boundary")
            }
        }
    }
    fn economy_remove(&mut self, _: u32) -> Result<()> {
        required("56F4D0 remove is not reached by4E6180")
    }
    fn refresh_organization(&mut self, _: u32) -> Result<()> {
        required("4D1350 actual stored Organization refresh binding")
    }
    fn counted(&mut self, actor: u32) -> Result<bool> {
        self.require_current(actor)?;
        if let Some(c) = self
            .record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .get(ComponentSlot::Body)
        {
            let RestoredComponent::Body(b) = c else {
                return required("counted Body payload differs");
            };
            if b.header.owner != self.record.owner {
                return required("counted Body actual owner differs");
            }
            let v = f32::from_bits(b.body.dying_started.0);
            if !v.is_finite() {
                return required("4E1540 nonfinite Body dying marker domain");
            }
            if v >= 0.0 {
                return Ok(false);
            }
        }
        if self.record.definition.flags & ((1 << 7) | (1 << 16)) != 0 {
            return Ok(true);
        }
        if self
            .record
            .shell
            .actor
            .as_ref()
            .unwrap()
            .components
            .present(ComponentSlot::MobileDeployment)
        {
            return required("4E1540 actual deployment primary definition flags");
        }
        Ok(false)
    }
    fn statistics_present(&mut self) -> Result<bool> {
        Ok(self.word(0x32c)? != 0)
    }
    fn statistics_track(&mut self, _: u32, _: bool) -> Result<()> {
        required("58F010 actual current statistics owner")
    }
    fn statistics_classified(&mut self, _: u32) -> Result<bool> {
        required("58EA80 actual statistic classification")
    }
    fn add_score(&mut self, _: usize, _: u32) -> Result<()> {
        required("58E870 actual statistic score owner")
    }
    fn ownership_track(&mut self, actor: u32, add: bool) -> Result<()> {
        self.require_current(actor)?;
        crate::kingdom_ownership_track::update(self,add).map_err(error)
    }
    fn property_records(&mut self) -> Result<Vec<PropertyApplication>> {
        let v = self
            .kingdom()?
            .vectors
            .get(&0x318)
            .ok_or_else(|| error("actual Kingdom318 property table is missing"))?;
        if !v.values.is_empty() {
            return required("4E6180 actual Kingdom318 property-record arena");
        }
        Ok(Vec::new())
    }
    fn apply_property(&mut self, _: u32, _: PropertyApplication) -> Result<()> {
        required("4CEF10 actual property-instance effects")
    }
    fn remove_property_effect(&mut self, _: u32, _: PropertyApplication) -> Result<()> {
        required("4CEF70 is not reached by4E6180")
    }
    fn structure_properties(&mut self, actor: u32) -> Result<Vec<u32>> {
        self.require_current(actor)?;
        crate::definition_technology_properties::bind(self.catalog, self.record.definition)
            .map_err(error)
    }
    fn add_kingdom_property(&mut self, _: u32, _: u32) -> Result<()> {
        required("4E3AD0 actual Kingdom property-instance owner")
    }
    fn remove_kingdom_property(&mut self, _: u32, _: u32) -> Result<()> {
        required("4E23D0 is not reached by4E6180")
    }
    fn max_flares(&mut self) -> Result<i32> {
        required("4B2E60 actual MaxFlaresPerKingdom registered integer source")
    }
    fn flare_time(&mut self, _: u32) -> Result<u32> {
        required("4E1ED0 actual current flare allocation")
    }
    fn destroy_actor(&mut self, _: u32) -> Result<()> {
        required("4F6760 actual world destruction services")
    }
}

impl crate::kingdom_ownership_track::World for World<'_, '_, '_>{
    fn required_city_158(&mut self)->std::result::Result<ActorId,String>{
        actor_word(self.record,0x158).map(ActorId::from_raw).map_err(|e|e.to_string())
    }
    fn resolve_current(&mut self,id:ActorId)->std::result::Result<Option<ActorId>,String>{
        Ok(self.record.registry.get(id).map(|_|id))
    }
    fn definition_24(&mut self,id:ActorId)->std::result::Result<bool,String>{
        let definition=if id==self.record.owner{self.record.definition}else{
            let entry=self.record.registry.get(id).ok_or("4E3C70 selected required-city generation became stale")?;
            match &entry.value{
                RestoredRecord::Actor(v)|RestoredRecord::Object(v)|RestoredRecord::Thing(v)=>&v.definition,
                RestoredRecord::Kingdom(v)=>&v.runtime.as_ref().ok_or("4E3C70 selected Kingdom runtime absent")?.definition,
                RestoredRecord::Team(v)=>&v.runtime.as_ref().ok_or("4E3C70 selected Team runtime absent")?.definition,
                RestoredRecord::Loading=>return Err("4E3C70 another held required-city record is unbound".into()),
            }
        };
        if definition.native_class==1{return Err("4E3C70 actual ThingData virtual24 source binding is required".into())}
        let bound=Source::bind(self.catalog,definition).map_err(|e|e.to_string())?;
        //PostCreate already holds this same World's mutable field loans. The
        //source cache is borrowed disjointly, never reacquired via its RefCell.
        let fields=self.source_fields;
        let data=fields.definition(&definition.proof.id).map_err(|e|e.to_string())?;
        if fields.receipt().sources!=self.catalog.actors.sources||fields.source_precision()!=bound.source_precision()
            ||data.token().source()!=&definition.proof.source||data.native_kind()!=definition.proof.category.native_kind{
            return Err("4E3C70 selected actual source/FP/definition differs".into())
        }
        fields.byte(&definition.proof.id,0x34).map(|v|v!=0).map_err(|e|e.to_string())
    }
    fn current_actor(&self)->ActorId{self.record.owner}
    fn has_city_d4(&mut self)->std::result::Result<bool,String>{
        let Some(c)=self.record.shell.actor.as_ref().ok_or("4E3C70 current Actor allocation absent")?.components.get(ComponentSlot::City)
            else{return Ok(false)};
        match c{RestoredComponent::City(v)if v.header.owner==self.record.owner=>Ok(true),
            _=>Err("4E3C70 actual City component allocation/owner differs".into())}
    }
    fn list_30c(&mut self)->std::result::Result<&mut Vec<ActorId>,String>{
        Ok(&mut self.kingdom_mut().map_err(|e|e.to_string())?.actor_lists.as_mut()
            .ok_or("4E3C70 same Kingdom30C list owner absent")?.city_ownership)
    }
}
impl StoredRegistryWorld for World<'_, '_, '_> {
    fn class_insert_head(&mut self, index: u32, actor: u32) -> Result<()> {
        self.require_current(actor)?;
        let l = self
            .kingdom_mut()?
            .actor_lists
            .as_mut()
            .ok_or_else(|| error("Kingdom class-list constructor owner missing"))?;
        let v = l.by_class.get_mut(index as usize).ok_or_else(|| {
            error("native class-list index is outside its constructed enum domain")
        })?;
        v.try_reserve(1).map_err(error)?;
        v.insert(0, ActorId::from_raw(actor));
        Ok(())
    }
    fn list_push(&mut self, list: ActorList, actor: u32) -> Result<()> {
        let offset = Self::vector_offset(list)?;
        let v = self
            .kingdom_mut()?
            .vectors
            .get_mut(&offset)
            .ok_or_else(|| error("actual Kingdom vector is missing"))?;
        let count = u32::try_from(v.values.len())
            .map_err(error)?
            .checked_add(1)
            .ok_or_else(|| error("native vector count overflow"))?;
        if count > i32::MAX as u32 {
            return required("native vector signed-count domain");
        }
        //766DE0 compares the wrapped doubled capacity as signed x86.
        let twice = v.capacity_hint.wrapping_mul(2);
        let capacity = if (count as i32) > (twice as i32) {
            count
        } else if (count as i32) > (v.capacity_hint as i32) {
            twice
        } else {
            v.capacity_hint
        };

        v.values
            .try_reserve_exact((capacity as usize).saturating_sub(v.values.len()))
            .map_err(error)?;
        v.capacity_hint = capacity;
        v.values.push(actor);
        Ok(())
    }
    fn list_insert_head(&mut self, list: ActorList, actor: u32) -> Result<()> {
        if list != ActorList::Flares {
            return required("native head insertion requested for a vector");
        }
        let v = &mut self
            .kingdom_mut()?
            .actor_lists
            .as_mut()
            .ok_or_else(|| error("actual flare-list owner missing"))?
            .flares;
        v.try_reserve(1).map_err(error)?;
        v.insert(0, ActorId::from_raw(actor));
        Ok(())
    }
    fn list_len(&self, list: ActorList) -> Result<usize> {
        let k = self.kingdom()?;
        if list == ActorList::Flares {
            return Ok(k
                .actor_lists
                .as_ref()
                .ok_or_else(|| error("actual flare owner missing"))?
                .flares
                .len());
        }
        let offset = Self::vector_offset(list)?;
        Ok(k.vectors
            .get(&offset)
            .ok_or_else(|| error("actual Kingdom vector missing"))?
            .values
            .len())
    }
    fn list_at(&self, list: ActorList, index: usize) -> Result<u32> {
        let k = self.kingdom()?;
        if list == ActorList::Flares {
            return k
                .actor_lists
                .as_ref()
                .ok_or_else(|| error("actual flare owner missing"))?
                .flares
                .get(index)
                .map(|v| v.raw())
                .ok_or(EconomyError::ResourceIndex);
        }
        let offset = Self::vector_offset(list)?;
        k.vectors
            .get(&offset)
            .ok_or_else(|| error("actual Kingdom vector missing"))?
            .values
            .get(index)
            .copied()
            .ok_or(EconomyError::ResourceIndex)
    }
    fn list_contains(&self, list: ActorList, actor: u32) -> Result<bool> {
        let k = self.kingdom()?;
        if list == ActorList::Flares {
            return Ok(k
                .actor_lists
                .as_ref()
                .ok_or_else(|| error("actual flare owner missing"))?
                .flares
                .contains(&ActorId::from_raw(actor)));
        }
        let offset = Self::vector_offset(list)?;
        Ok(k.vectors
            .get(&offset)
            .ok_or_else(|| error("actual Kingdom vector missing"))?
            .values
            .contains(&actor))
    }
    fn supply_contains(&self, actor: u32) -> Result<bool> {
        Ok(self
            .kingdom()?
            .supply_sources
            .contains(&ActorId::from_raw(actor)))
    }
    fn supply_insert_head(&mut self, actor: u32) -> Result<()> {
        let v = &mut self.kingdom_mut()?.supply_sources;
        v.try_reserve(1).map_err(error)?;
        v.insert(0, ActorId::from_raw(actor));
        Ok(())
    }
    fn increment_counted(&mut self) -> Result<()> {
        let v = self
            .kingdom_mut()?
            .words
            .get_mut(&0x2f4)
            .ok_or_else(|| error("actual counted scalar2F4 missing"))?;
        *v = v.wrapping_add(1);
        Ok(())
    }
    fn capital(&self) -> Result<Option<u32>> {
        let k = self.kingdom()?;
        match k.identity_references.get(&0x1f8) {
            Some(v) => Ok(v.map(|i| i.raw())),
            None => {
                if self.word(0x1f8)? == 0 {
                    Ok(None)
                } else {
                    required("actual capital pointer has not been translated to its current GTID")
                }
            }
        }
    }
    fn set_capital(&mut self, actor: u32) -> Result<()> {
        let k = self.kingdom_mut()?;
        k.identity_references
            .insert(0x1f8, Some(ActorId::from_raw(actor)));
        k.words.remove(&0x1f8);
        Ok(())
    }
    fn word_190(&self) -> Result<u32> {
        self.word(0x190)
    }
}
#[cfg(test)]
#[path = "load_owner_add_actor_tests.rs"]
mod tests;
