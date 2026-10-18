//!4E44E0 on the same staged Actor and stored Kingdom allocations.
use super::*;
use aa_economy::{
    EconomyError, F32Bits, borrowed_flows,
    kingdom_registry::{
        self as native, ActorList, RegistryActor, RegistryWorld, RemoveRegistryWorld,
    },
};
use aa_mapworld::special_state::SpecialShell;
type EResult<T> = std::result::Result<T, EconomyError>;
fn e(value: impl std::fmt::Display) -> EconomyError {
    EconomyError::World(value.to_string())
}
pub(super) fn remove(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    actor: ObjectKey,
    pc: Precision,
) -> Result<()> {
    let runtime = canonical(staged, catalog, actor)?;
    if runtime.definition.native_class != 3 {
        return Err("4E44E0 removal requires ordinary Actor allocation".into());
    }
    let owner = ActorId::from_raw(
        *runtime
            .actor
            .as_ref()
            .unwrap()
            .words
            .get(&0xe8)
            .ok_or("4E44E0 actual ActorE8 absent")?,
    );
    let current = staged
        .actors
        .id(actor)
        .ok_or("4E44E0 current GTID absent")?;
    if owner.is_null() || owner == current {
        return Err("4E44E0 actual distinct owner required".into());
    }
    let owner_key = staged
        .actors
        .key(owner)
        .ok_or("4E44E0 stored owner generation absent")?;
    let mut world = Owned {
        staged,
        catalog,
        actor,
        current,
        owner,
        owner_key,
        pc,
    };
    world.kingdom().map_err(|v| v.to_string())?;
    native::remove_actor_stored(current.raw(), &mut world).map_err(|v| v.to_string())
}
struct Owned<'a, 's> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'s>,
    actor: ObjectKey,
    current: ActorId,
    owner: ActorId,
    owner_key: ObjectKey,
    pc: Precision,
}
impl Owned<'_, '_> {
    fn runtime(&self) -> EResult<&RuntimeObject> {
        canonical(self.staged, self.catalog, self.actor).map_err(e)
    }
    fn require_current(&self, id: u32) -> EResult<()> {
        if id == self.current.raw() {
            Ok(())
        } else {
            Err(e("4E44E0 another actor callback requires its real owner"))
        }
    }
    fn kingdom(&self) -> EResult<&SpecialShell> {
        let runtime = canonical(self.staged, self.catalog, self.owner_key).map_err(e)?;
        if runtime.definition.native_class != 4
            || self.staged.actors.id(self.owner_key) != Some(self.owner)
        {
            return Err(e("4E44E0 actual owner Kingdom source/generation differs"));
        }
        let crate::actor_world::Source::Special(k) =
            &self.staged.actors.entity(self.owner_key).map_err(e)?.source
        else {
            return Err(e("4E44E0 actual derived Kingdom allocation absent"));
        };
        if !matches!(k.source, crate::actor_world::SpecialSource::Kingdom(_)) {
            return Err(e("4E44E0 derived allocation is not Kingdom"));
        }
        let derived = k
            .derived
            .as_ref()
            .ok_or_else(|| e("4E44E0 Kingdom storage absent"))?;
        if derived.recruitment_owner != Some(self.owner) {
            return Err(e("4E44E0 Kingdom constructor owner differs"));
        }
        Ok(derived)
    }
    fn kingdom_mut(&mut self) -> EResult<&mut SpecialShell> {
        self.kingdom()?;
        let crate::actor_world::Source::Special(k) = &mut self
            .staged
            .actors
            .entity_mut(self.owner_key)
            .map_err(e)?
            .source
        else {
            unreachable!()
        };
        k.derived
            .as_mut()
            .ok_or_else(|| e("4E44E0 current Kingdom storage disappeared"))
    }
    fn word(&self, offset: u32) -> EResult<u32> {
        self.kingdom()?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| e(format!("4E44E0 actual Kingdom+{offset:X} unbound")))
    }
    fn offset(list: ActorList) -> EResult<u32> {
        match list {
            ActorList::Structures => Ok(0x2c4),
            ActorList::Settlements => Ok(0x2d0),
            ActorList::Hqs => Ok(0x2dc),
            ActorList::Organizations => Ok(0x2e8),
            ActorList::Flares => Err(e("4E44E0 flare is a linked list")),
        }
    }
    fn economy_remove_current(&mut self) -> EResult<()> {
        let source_pc = self
            .staged
            .definitions
            .combat_source()
            .map_err(e)?
            .source_precision();
        //Resource metadata is reached only after actual flag11/10 tests below.
        for provided in [false, true] {
            let flag = {
                let a = self.runtime()?.actor.as_ref().unwrap();
                let Some(RuntimeComponent::Economy(c)) = a.components.get(ComponentSlot::Economy)
                else {
                    return Err(e("56F4D0 actual Economy slot differs"));
                };
                if c.header.owner != self.current {
                    return Err(e("56F4D0 Economy owner differs"));
                }
                if provided { c.flag_10 } else { c.flag_11 }
            };
            if !flag {
                continue;
            }
            {
                let c = self
                    .staged
                    .actors
                    .object_mut(self.actor)
                    .map_err(e)?
                    .actor
                    .as_mut()
                    .unwrap()
                    .components
                    .get_mut(ComponentSlot::Economy)
                    .unwrap();
                let RuntimeComponent::Economy(c) = c else {
                    unreachable!()
                };
                if provided {
                    c.flag_10 = false
                } else {
                    c.flag_11 = false
                };
            }
            //Virtual+108 reads the current actual Actor+E8 on each removal.
            let current_owner = ActorId::from_raw(
                *self
                    .runtime()?
                    .actor
                    .as_ref()
                    .unwrap()
                    .words
                    .get(&0xe8)
                    .ok_or_else(|| e("56F4D0 current E8 absent"))?,
            );
            if current_owner.is_null() {
                continue;
            }
            if current_owner != self.owner {
                return Err(e("56F4D0 current owner changed"));
            }
            self.kingdom()?;
            let definitions = self.catalog.native_resource_economy(source_pc).map_err(e)?;
            let count = crate::saved_kingdom_flows::consumption_count(&definitions)?;
            let pc = self.pc;
            let catalog = self.catalog;
            self.staged
                .actors
                .with_two_entities_mut(self.actor, self.owner_key, |actor, kingdom| {
                    let component = actor
                        .runtime
                        .as_mut()
                        .and_then(|r| r.actor.as_mut())
                        .and_then(|a| a.components.get_mut(ComponentSlot::Economy))
                        .ok_or(crate::actor_world::Error::Missing(
                            "56F4D0 retained Economy",
                        ))?;
                    let RuntimeComponent::Economy(component) = component else {
                        return Err(crate::actor_world::Error::Missing("56F4D0 Economy class"));
                    };
                    let crate::actor_world::Source::Special(kingdom) = &mut kingdom.source else {
                        return Err(crate::actor_world::Error::Missing(
                            "56F4D0 Kingdom allocation",
                        ));
                    };
                    let kingdom = kingdom
                        .derived
                        .as_mut()
                        .ok_or(crate::actor_world::Error::Missing("56F4D0 Kingdom storage"))?;
                    let mut maximum = || crate::saved_kingdom_flows::provider_maximum(catalog, pc);
                    let mut flows = crate::saved_kingdom_flows::BorrowedKingdom {
                        kingdom,
                        resources: definitions.catalog(),
                        consumption_count: count,
                        provider_maximum: &mut maximum,
                    };
                    let vector = &mut component.resources[usize::from(!provided)];
                    let values = vector
                        .values
                        .iter()
                        .map(|v| F32Bits(v.0))
                        .collect::<Vec<_>>();
                    let precision = match pc {
                        Precision::Single24 => aa_economy::X87Precision::Single24,
                        Precision::Double53 => aa_economy::X87Precision::Double53,
                        Precision::Extended64 => aa_economy::X87Precision::Extended64,
                    };
                    let adjusted = if provided {
                        borrowed_flows::adjust_providers(&mut flows, &values, false, precision)
                    } else {
                        borrowed_flows::adjust_consumption(&mut flows, &values, false, precision)
                    };
                    adjusted.map_err(|v| crate::actor_world::Error::Source(v.to_string()))?;
                    let len = definitions.resource_count();
                    if vector.values.len() < len {
                        vector
                            .values
                            .try_reserve_exact(len - vector.values.len())
                            .map_err(|v| crate::actor_world::Error::Source(v.to_string()))?;
                    }
                    vector.values.resize(len, aa_body::F32::ZERO);
                    vector.values.fill(aa_body::F32::ZERO);
                    Ok(())
                })
                .map_err(e)?;
        }
        Ok(())
    }
}
impl RegistryWorld for Owned<'_, '_> {
    fn actor(&self, id: u32) -> EResult<RegistryActor> {
        self.require_current(id)?;
        let r = self.runtime()?;
        let a = r.actor.as_ref().unwrap();
        for (slot, c) in a.components.iter() {
            if c.slot() != slot {
                return Err(e("4E44E0 actual component slot/class differs"));
            }
        }
        let has = |s| a.components.present(s);
        let category = &r.definition.proof.category;
        Ok(RegistryActor {
            class_index: category.runtime_class,
            class_kind: category.registry_index,
            flags8: self
                .staged
                .actors
                .placement(self.actor, &ReadSpatial(self.staged))
                .map_err(e)?
                .flags,
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
    fn set_flags8(&mut self, id: u32, flags: u32) -> EResult<()> {
        self.require_current(id)?;
        let before = self.actor(id)?.flags8;
        let clear = before ^ flags;
        if flags & !before != 0 || clear & !0x18000000 != 0 {
            return Err(e("4E44E0 unexpected flag edit"));
        }
        if clear == 0 {
            return Ok(());
        }
        clear_registration_flags(self.staged, self.actor, clear).map_err(e)
    }
    fn structure_excluded(&mut self, id: u32) -> EResult<bool> {
        self.require_current(id)?;
        let c = &self.runtime()?.actor.as_ref().unwrap().components;
        if let Some(RuntimeComponent::Structure(s)) = c.get(ComponentSlot::Structure) {
            if s.header.owner != self.current {
                return Err(e("4CED60 actual Structure owner differs"));
            }
            if s.object_handle_10 != 0 {
                return Ok(true);
            }
        }
        if c.present(ComponentSlot::Wall) {
            return Err(e("4CED60 actual Wall getter"));
        }
        Ok(false)
    }
    fn economy_add(&mut self, _: u32) -> EResult<()> {
        Err(e("4E44E0 does not add Economy"))
    }
    fn economy_remove(&mut self, id: u32) -> EResult<()> {
        self.require_current(id)?;
        self.economy_remove_current()
    }
    fn refresh_organization(&mut self, _: u32) -> EResult<()> {
        Err(e("4D1350 current Organization refresh required"))
    }
    fn counted(&mut self, _: u32) -> EResult<bool> {
        Err(e("4E44E0 does not query counted predicate"))
    }
    fn statistics_present(&mut self) -> EResult<bool> {
        Ok(self.word(0x32c)? != 0)
    }
    fn statistics_track(&mut self, _: u32, _: bool) -> EResult<()> {
        Err(e("58EFE0 actual Kingdom statistics required"))
    }
    fn statistics_classified(&mut self, _: u32) -> EResult<bool> {
        Err(e("4E44E0 does not classify statistics"))
    }
    fn add_score(&mut self, _: usize, _: u32) -> EResult<()> {
        Err(e("4E44E0 does not add score"))
    }
    fn ownership_track(&mut self, id: u32, add: bool) -> EResult<()> {
        self.require_current(id)?;
        if add {
            return Err(e("4E44E0 ownership add unsupported"));
        }
        let r = self.runtime()?;
        let a = r.actor.as_ref().unwrap();
        let target = ActorId::from_raw(
            *a.words
                .get(&0x158)
                .ok_or_else(|| e("4E3C70 actual158 missing"))?,
        );
        if self.staged.actors.key(target).is_some() || a.components.present(ComponentSlot::City) {
            return Err(e("4E3C70 actual ownership/city list removal required"));
        }
        Ok(())
    }
    fn property_records(&mut self) -> EResult<Vec<native::PropertyApplication>> {
        if !self
            .kingdom()?
            .vectors
            .get(&0x318)
            .ok_or_else(|| e("4E44E0 actual property vector absent"))?
            .values
            .is_empty()
        {
            return Err(e("4CEF70 actual Kingdom property-record arena required"));
        }
        Ok(vec![])
    }
    fn apply_property(&mut self, _: u32, _: native::PropertyApplication) -> EResult<()> {
        Err(e("4E44E0 does not add property"))
    }
    fn remove_property_effect(&mut self, _: u32, _: native::PropertyApplication) -> EResult<()> {
        Err(e("4CEF70 actual property instance removal required"))
    }
    fn structure_properties(&mut self, id: u32) -> EResult<Vec<u32>> {
        self.require_current(id)?;
        crate::definition_technology_properties::bind(self.catalog, &self.runtime()?.definition)
            .map_err(e)
    }
    fn add_kingdom_property(&mut self, _: u32, _: u32) -> EResult<()> {
        Err(e("4E44E0 does not add research property"))
    }
    fn remove_kingdom_property(&mut self, _: u32, _: u32) -> EResult<()> {
        Err(e("4E23D0 actual research property removal required"))
    }
    fn max_flares(&mut self) -> EResult<i32> {
        Err(e("4E44E0 does not query maximum flares"))
    }
    fn flare_time(&mut self, _: u32) -> EResult<u32> {
        Err(e("4E44E0 does not query flare time"))
    }
    fn destroy_actor(&mut self, _: u32) -> EResult<()> {
        Err(e("4E44E0 does not destroy another actor"))
    }
}
impl RemoveRegistryWorld for Owned<'_, '_> {
    fn capital(&self) -> EResult<Option<u32>> {
        let k = self.kingdom()?;
        match k.identity_references.get(&0x1f8) {
            Some(v) => Ok(v.map(ActorId::raw)),
            None if self.word(0x1f8)? == 0 => Ok(None),
            None => Err(e("4E44E0 actual capital pointer unresolved")),
        }
    }
    fn clear_capital(&mut self) -> EResult<()> {
        let k = self.kingdom_mut()?;
        k.identity_references.insert(0x1f8, None);
        k.words.remove(&0x1f8);
        Ok(())
    }
    fn word_190(&self) -> EResult<u32> {
        self.word(0x190)
    }
    fn vector_remove(&mut self, list: ActorList, id: u32) -> EResult<()> {
        let off = Self::offset(list)?;
        let v = &mut self
            .kingdom_mut()?
            .vectors
            .get_mut(&off)
            .ok_or_else(|| e("4E44E0 actual vector absent"))?
            .values;
        let i = v
            .iter()
            .position(|v| *v == id)
            .ok_or_else(|| e("4E44E0 native vector removal of absent actor corrupts storage"))?;
        v.remove(i);
        Ok(())
    }
    fn organization_len(&self) -> EResult<usize> {
        Ok(self
            .kingdom()?
            .vectors
            .get(&0x2e8)
            .ok_or_else(|| e("4E44E0 actual organizations absent"))?
            .values
            .len())
    }
    fn organization_at(&self, i: usize) -> EResult<u32> {
        self.kingdom()?
            .vectors
            .get(&0x2e8)
            .ok_or_else(|| e("4E44E0 actual organizations absent"))?
            .values
            .get(i)
            .copied()
            .ok_or(EconomyError::ResourceIndex)
    }
    fn supply_remove_first(&mut self, id: u32) -> EResult<()> {
        let v = &mut self.kingdom_mut()?.supply_sources;
        if let Some(i) = v.iter().position(|v| v.raw() == id) {
            v.remove(i);
        }
        Ok(())
    }
    fn class_remove_first(&mut self, class: u32, id: u32) -> EResult<()> {
        let v = self
            .kingdom_mut()?
            .actor_lists
            .as_mut()
            .ok_or_else(|| e("4E44E0 actual class lists absent"))?
            .by_class
            .get_mut(class as usize)
            .ok_or_else(|| e("4E44E0 actual class list domain"))?;
        if let Some(i) = v.iter().position(|v| v.raw() == id) {
            v.remove(i);
        }
        Ok(())
    }
    fn decrement_counted(&mut self) -> EResult<()> {
        let v = self
            .kingdom_mut()?
            .words
            .get_mut(&0x2f4)
            .ok_or_else(|| e("4E44E0 actual counted scalar absent"))?;
        *v = v.wrapping_sub(1);
        Ok(())
    }
    fn flare_remove_first(&mut self, id: u32) -> EResult<()> {
        let v = &mut self
            .kingdom_mut()?
            .actor_lists
            .as_mut()
            .ok_or_else(|| e("4E44E0 actual flare list absent"))?
            .flares;
        if let Some(i) = v.iter().position(|v| v.raw() == id) {
            v.remove(i);
        }
        Ok(())
    }
}
