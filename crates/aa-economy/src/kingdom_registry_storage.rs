//! Native4E6180 over actual borrowed storage, with no detached Registry copy.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorList {
    Structures,
    Settlements,
    Hqs,
    Organizations,
    Flares,
}
/// Storage and effects share one owner. Each required access observes current
/// fields, including earlier list writes before a required child stops.
pub trait StoredRegistryWorld: RegistryWorld {
    fn class_insert_head(&mut self, class_index: u32, actor: u32) -> Result<(), EconomyError>;
    fn list_push(&mut self, list: ActorList, actor: u32) -> Result<(), EconomyError>;
    fn list_insert_head(&mut self, list: ActorList, actor: u32) -> Result<(), EconomyError>;
    fn list_len(&self, list: ActorList) -> Result<usize, EconomyError>;
    fn list_at(&self, list: ActorList, index: usize) -> Result<u32, EconomyError>;
    fn list_contains(&self, list: ActorList, actor: u32) -> Result<bool, EconomyError>;
    fn supply_contains(&self, actor: u32) -> Result<bool, EconomyError>;
    fn supply_insert_head(&mut self, actor: u32) -> Result<(), EconomyError>;
    fn increment_counted(&mut self) -> Result<(), EconomyError>;
    fn capital(&self) -> Result<Option<u32>, EconomyError>;
    fn set_capital(&mut self, actor: u32) -> Result<(), EconomyError>;
    fn word_190(&self) -> Result<u32, EconomyError>;
}
///4E44E0 storage lease. Each reached mutation updates its actual owner before
///the next fallible effect; no complete KingdomRegistry projection is needed.
pub trait RemoveRegistryWorld: RegistryWorld {
    fn capital(&self) -> Result<Option<u32>, EconomyError>;
    fn clear_capital(&mut self) -> Result<(), EconomyError>;
    fn word_190(&self) -> Result<u32, EconomyError>;
    fn vector_remove(&mut self, list: ActorList, actor: u32) -> Result<(), EconomyError>;
    fn organization_len(&self) -> Result<usize, EconomyError>;
    fn organization_at(&self, index: usize) -> Result<u32, EconomyError>;
    fn supply_remove_first(&mut self, actor: u32) -> Result<(), EconomyError>;
    fn class_remove_first(&mut self, class: u32, actor: u32) -> Result<(), EconomyError>;
    fn decrement_counted(&mut self) -> Result<(), EconomyError>;
    fn flare_remove_first(&mut self, actor: u32) -> Result<(), EconomyError>;
}
pub fn remove_actor_stored(
    actor: u32,
    w: &mut impl RemoveRegistryWorld,
) -> Result<(), EconomyError> {
    let a = w.actor(actor)?;
    if a.class_kind == CITY_CLASS && w.capital()? == Some(actor) {
        w.clear_capital()?;
    }
    if a.flags8 & FLAG_REGISTERED == 0 {
        return Ok(());
    }
    if w.statistics_present()? && !bit(a.states, 21) {
        w.statistics_track(actor, false)?;
    }
    w.ownership_track(actor, false)?;
    if w.word_190()? == 0 {
        if a.structure {
            for definition in w.structure_properties(actor)? {
                w.remove_kingdom_property(definition, actor)?;
            }
        }
        if a.properties {
            for record in w.property_records()? {
                w.remove_property_effect(actor, record)?;
            }
        }
    }
    if a.settlement {
        w.vector_remove(ActorList::Settlements, actor)?;
    }
    if a.hq {
        w.vector_remove(ActorList::Hqs, actor)?;
        let count = w.organization_len()?;
        for index in 0..count {
            let org = w.organization_at(index)?;
            w.refresh_organization(org)?;
        }
    }
    if a.organization {
        w.vector_remove(ActorList::Organizations, actor)?;
    }
    if a.structure && !w.structure_excluded(actor)? {
        w.vector_remove(ActorList::Structures, actor)?;
    }
    if a.supply {
        w.supply_remove_first(actor)?;
    }
    if a.economy {
        w.economy_remove(actor)?;
    }
    w.class_remove_first(a.class_index, actor)?;
    let flags = w.actor(actor)?.flags8 & !FLAG_REGISTERED;
    //Actual4E47EE/4E47FB store flags before counted scalar4E47FE.
    w.set_flags8(actor, flags)?;
    if flags & FLAG_COUNTED != 0 {
        w.set_flags8(actor, flags & !FLAG_COUNTED)?;
        w.decrement_counted()?;
    }
    if a.flare {
        w.flare_remove_first(actor)?;
    }
    Ok(())
}
fn count_stored(actor: u32, w: &mut impl StoredRegistryWorld) -> Result<(), EconomyError> {
    let flags = w.actor(actor)?.flags8;
    if flags & FLAG_COUNTED == 0 && w.counted(actor)? {
        w.increment_counted()?;
        w.set_flags8(actor, flags | FLAG_COUNTED)?;
    }
    Ok(())
}
fn completed_stored(actor: u32, w: &mut impl StoredRegistryWorld) -> Result<(), EconomyError> {
    count_stored(actor, w)?;
    if w.statistics_present()? {
        w.statistics_track(actor, true)?;
        if w.statistics_classified(actor)? {
            w.add_score(14, 0x3f800000)?;
        }
    }
    w.ownership_track(actor, true)
}
/// Native AddActor storage order, shared with the existing Registry API.
pub fn add_actor_stored(actor: u32, w: &mut impl StoredRegistryWorld) -> Result<(), EconomyError> {
    let a = w.actor(actor)?;
    w.class_insert_head(a.class_index, actor)?;
    w.set_flags8(actor, a.flags8 | FLAG_REGISTERED)?;
    if a.economy {
        w.economy_add(actor)?;
    }
    if a.structure && !w.structure_excluded(actor)? {
        w.list_push(ActorList::Structures, actor)?;
    }
    if a.settlement {
        w.list_push(ActorList::Settlements, actor)?;
    }
    if a.hq {
        w.list_push(ActorList::Hqs, actor)?;
        let count = w.list_len(ActorList::Organizations)?;
        for index in 0..count {
            let organization = w.list_at(ActorList::Organizations, index)?;
            w.refresh_organization(organization)?;
        }
    }
    if a.organization {
        w.list_push(ActorList::Organizations, actor)?;
    }
    if a.supply && !w.supply_contains(actor)? {
        w.supply_insert_head(actor)?;
    }
    let states = w.actor(actor)?.states;
    if !bit(states, 21) && !(bit(states, 26) || bit(states, 36)) {
        completed_stored(actor, w)?;
    }
    if a.class_kind == CITY_CLASS {
        let capital = w.capital()?;
        if capital != Some(actor) {
            if let Some(previous) = capital {
                w.destroy_actor(previous)?;
            }
            w.set_capital(actor)?;
        }
    }
    if w.word_190()? == 0 {
        if a.properties {
            for record in w.property_records()? {
                w.apply_property(actor, record)?;
            }
        }
        if a.structure {
            for definition in w.structure_properties(actor)? {
                w.add_kingdom_property(definition, actor)?;
            }
        }
    }
    if a.component_ac {
        count_stored(actor, w)?;
    }
    if a.flare && !w.list_contains(ActorList::Flares, actor)? {
        let maximum = w.max_flares()?;
        w.list_insert_head(ActorList::Flares, actor)?;
        if w.list_len(ActorList::Flares)? as i64 > i64::from(maximum) {
            let mut oldest: Option<(u32, u32)> = None;
            for index in 0..w.list_len(ActorList::Flares)? {
                let flare = w.list_at(ActorList::Flares, index)?;
                let time = w.flare_time(flare)?;
                oldest = match oldest {
                    Some((best, best_time)) if f32_le(best_time, time)? => Some((best, best_time)),
                    _ => Some((flare, time)),
                };
            }
            if let Some((oldest, _)) = oldest
                && oldest != actor
            {
                w.destroy_actor(oldest)?;
            }
        }
    }
    Ok(())
}
struct Joined<'a, W> {
    registry: &'a mut KingdomRegistry,
    world: &'a mut W,
}
impl<W: RegistryWorld> RegistryWorld for Joined<'_, W> {
    fn actor(&self, actor: u32) -> Result<RegistryActor, EconomyError> {
        self.world.actor(actor)
    }
    fn set_flags8(&mut self, actor: u32, flags: u32) -> Result<(), EconomyError> {
        self.world.set_flags8(actor, flags)
    }
    fn structure_excluded(&mut self, actor: u32) -> Result<bool, EconomyError> {
        self.world.structure_excluded(actor)
    }
    fn economy_add(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.world.economy_add(actor)
    }
    fn economy_remove(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.world.economy_remove(actor)
    }
    fn refresh_organization(&mut self, organization: u32) -> Result<(), EconomyError> {
        self.world.refresh_organization(organization)
    }
    fn counted(&mut self, actor: u32) -> Result<bool, EconomyError> {
        self.world.counted(actor)
    }
    fn statistics_present(&mut self) -> Result<bool, EconomyError> {
        self.world.statistics_present()
    }
    fn statistics_track(&mut self, actor: u32, add: bool) -> Result<(), EconomyError> {
        self.world.statistics_track(actor, add)
    }
    fn statistics_classified(&mut self, actor: u32) -> Result<bool, EconomyError> {
        self.world.statistics_classified(actor)
    }
    fn add_score(&mut self, index: usize, amount_bits: u32) -> Result<(), EconomyError> {
        self.world.add_score(index, amount_bits)
    }
    fn ownership_track(&mut self, actor: u32, add: bool) -> Result<(), EconomyError> {
        self.world.ownership_track(actor, add)
    }
    fn property_records(&mut self) -> Result<Vec<PropertyApplication>, EconomyError> {
        self.world.property_records()
    }
    fn apply_property(
        &mut self,
        actor: u32,
        record: PropertyApplication,
    ) -> Result<(), EconomyError> {
        self.world.apply_property(actor, record)
    }
    fn remove_property_effect(
        &mut self,
        actor: u32,
        record: PropertyApplication,
    ) -> Result<(), EconomyError> {
        self.world.remove_property_effect(actor, record)
    }
    fn structure_properties(&mut self, actor: u32) -> Result<Vec<u32>, EconomyError> {
        self.world.structure_properties(actor)
    }
    fn add_kingdom_property(&mut self, definition: u32, actor: u32) -> Result<(), EconomyError> {
        self.world.add_kingdom_property(definition, actor)
    }
    fn remove_kingdom_property(&mut self, definition: u32, actor: u32) -> Result<(), EconomyError> {
        self.world.remove_kingdom_property(definition, actor)
    }
    fn max_flares(&mut self) -> Result<i32, EconomyError> {
        self.world.max_flares()
    }
    fn flare_time(&mut self, actor: u32) -> Result<u32, EconomyError> {
        self.world.flare_time(actor)
    }
    fn destroy_actor(&mut self, actor: u32) -> Result<(), EconomyError> {
        self.world.destroy_actor(actor)
    }
}
impl<W: RegistryWorld> Joined<'_, W> {
    fn list(&self, l: ActorList) -> &Vec<u32> {
        match l {
            ActorList::Structures => &self.registry.structures,
            ActorList::Settlements => &self.registry.settlements,
            ActorList::Hqs => &self.registry.hqs,
            ActorList::Organizations => &self.registry.organizations,
            ActorList::Flares => &self.registry.flares,
        }
    }
    fn list_mut(&mut self, l: ActorList) -> &mut Vec<u32> {
        match l {
            ActorList::Structures => &mut self.registry.structures,
            ActorList::Settlements => &mut self.registry.settlements,
            ActorList::Hqs => &mut self.registry.hqs,
            ActorList::Organizations => &mut self.registry.organizations,
            ActorList::Flares => &mut self.registry.flares,
        }
    }
}
impl<W: RegistryWorld> StoredRegistryWorld for Joined<'_, W> {
    fn class_insert_head(&mut self, c: u32, a: u32) -> Result<(), EconomyError> {
        self.registry.class_lists.entry(c).or_default().insert(0, a);
        Ok(())
    }
    fn list_push(&mut self, l: ActorList, a: u32) -> Result<(), EconomyError> {
        self.list_mut(l).push(a);
        Ok(())
    }
    fn list_insert_head(&mut self, l: ActorList, a: u32) -> Result<(), EconomyError> {
        self.list_mut(l).insert(0, a);
        Ok(())
    }
    fn list_len(&self, l: ActorList) -> Result<usize, EconomyError> {
        Ok(self.list(l).len())
    }
    fn list_at(&self, l: ActorList, i: usize) -> Result<u32, EconomyError> {
        self.list(l)
            .get(i)
            .copied()
            .ok_or(EconomyError::ResourceIndex)
    }
    fn list_contains(&self, l: ActorList, a: u32) -> Result<bool, EconomyError> {
        Ok(self.list(l).contains(&a))
    }
    fn supply_contains(&self, a: u32) -> Result<bool, EconomyError> {
        Ok(self.registry.supply.contains(&a))
    }
    fn supply_insert_head(&mut self, a: u32) -> Result<(), EconomyError> {
        self.registry.supply.insert(0, a);
        Ok(())
    }
    fn increment_counted(&mut self) -> Result<(), EconomyError> {
        self.registry.counted = self.registry.counted.wrapping_add(1);
        Ok(())
    }
    fn capital(&self) -> Result<Option<u32>, EconomyError> {
        Ok(self.registry.capital)
    }
    fn set_capital(&mut self, a: u32) -> Result<(), EconomyError> {
        self.registry.capital = Some(a);
        Ok(())
    }
    fn word_190(&self) -> Result<u32, EconomyError> {
        Ok(self.registry.word_190)
    }
}
pub(super) fn add_compat<W: RegistryWorld>(
    registry: &mut KingdomRegistry,
    actor: u32,
    world: &mut W,
) -> Result<(), EconomyError> {
    add_actor_stored(actor, &mut Joined { registry, world })
}
impl<W: RegistryWorld> RemoveRegistryWorld for Joined<'_, W> {
    fn capital(&self) -> Result<Option<u32>, EconomyError> {
        Ok(self.registry.capital)
    }
    fn clear_capital(&mut self) -> Result<(), EconomyError> {
        self.registry.capital = None;
        Ok(())
    }
    fn word_190(&self) -> Result<u32, EconomyError> {
        Ok(self.registry.word_190)
    }
    fn vector_remove(&mut self, l: ActorList, a: u32) -> Result<(), EconomyError> {
        remove_value(self.list_mut(l), a, "Kingdom native vector")
    }
    fn organization_len(&self) -> Result<usize, EconomyError> {
        Ok(self.registry.organizations.len())
    }
    fn organization_at(&self, i: usize) -> Result<u32, EconomyError> {
        self.registry
            .organizations
            .get(i)
            .copied()
            .ok_or(EconomyError::ResourceIndex)
    }
    fn supply_remove_first(&mut self, a: u32) -> Result<(), EconomyError> {
        if let Some(i) = self.registry.supply.iter().position(|v| *v == a) {
            self.registry.supply.remove(i);
        }
        Ok(())
    }
    fn class_remove_first(&mut self, c: u32, a: u32) -> Result<(), EconomyError> {
        if let Some(v) = self.registry.class_lists.get_mut(&c)
            && let Some(i) = v.iter().position(|v| *v == a)
        {
            v.remove(i);
        }
        Ok(())
    }
    fn decrement_counted(&mut self) -> Result<(), EconomyError> {
        self.registry.counted = self.registry.counted.wrapping_sub(1);
        Ok(())
    }
    fn flare_remove_first(&mut self, a: u32) -> Result<(), EconomyError> {
        if let Some(i) = self.registry.flares.iter().position(|v| *v == a) {
            self.registry.flares.remove(i);
        }
        Ok(())
    }
}
pub(super) fn remove_compat<W: RegistryWorld>(
    registry: &mut KingdomRegistry,
    actor: u32,
    world: &mut W,
) -> Result<(), EconomyError> {
    remove_actor_stored(actor, &mut Joined { registry, world })
}
