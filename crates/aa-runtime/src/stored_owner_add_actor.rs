//! Native4E6180 on the actual staged Actor/Kingdom; no copied Registry.
//! The existing add_actor_stored coordinator owns every branch/store order.
use crate::{
    actor_world::{
        ObjectKey, PlacementState, RuntimeComponent, RuntimeObject, Source, Spatial, SpecialSource,
    },
    catalog::Catalog,
    restoration::Staged,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_economy::{
    EconomyError,
    kingdom_registry::{
        self as native, ActorList, PropertyApplication, RegistryActor, RegistryWorld,
        StoredRegistryWorld,
    },
};
use aa_mapworld::special_state::SpecialShell;
use aa_numeric::Precision;
type Result<T> = std::result::Result<T, EconomyError>;
fn error(e: impl std::fmt::Display) -> EconomyError {
    EconomyError::World(e.to_string())
}
fn required<T>(e: &str) -> Result<T> {
    Err(error(e))
}

///Only the reached current Actor+E8 authorizes this same Kingdom allocation.
///Failed children retain native class/list/flag/count/economy prefixes.
pub fn add_actor(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    owner: ActorId,
    pc: Precision,
) -> std::result::Result<(), String> {
    let runtime = crate::fresh_post_create_world::canonical(staged, catalog, key)?;
    let id = staged
        .actors
        .id(key)
        .ok_or("4E6180 actual current GTID absent")?;
    if runtime.definition.native_class != 3
        || owner.is_null()
        || owner == id
        || runtime
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&0xe8))
            .copied()
            != Some(owner.raw())
    {
        return Err("4E6180 actual ordinary Actor/current distinct E8 owner differs".into());
    }
    let owner_key = staged
        .actors
        .key(owner)
        .ok_or("4E6180 current Kingdom generation absent")?;
    let mut world = Current {
        staged,
        catalog,
        key,
        id,
        owner,
        owner_key,
        pc,
    };
    world.kingdom().map_err(|e| e.to_string())?;
    native::add_actor_stored(id.raw(), &mut world).map_err(|e| e.to_string())
}
struct ReadSpatial<'a>(&'a Staged);
impl Spatial for ReadSpatial<'_> {
    fn placement(&self, id: ActorId) -> crate::actor_world::Result<PlacementState> {
        self.0
            .load_world
            .as_ref()
            .ok_or(crate::actor_world::Error::Missing(
                "4E6180 actual World absent",
            ))?
            .live_spatial()
            .map_err(crate::actor_world::Error::Source)?
            .placement(id)
    }
    fn publish(&mut self, _: ActorId, _: PlacementState) -> crate::actor_world::Result<()> {
        Err(crate::actor_world::Error::Missing(
            "read-only4E6180 placement loan",
        ))
    }
}
struct Current<'a, 's> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'s>,
    key: ObjectKey,
    id: ActorId,
    owner: ActorId,
    owner_key: ObjectKey,
    pc: Precision,
}
impl Current<'_, '_> {
    fn runtime(&self) -> Result<&RuntimeObject> {
        if self.staged.actors.key(self.id) != Some(self.key) {
            return required("4E6180 current generation changed");
        }
        self.staged.actors.object(self.key).map_err(error)
    }
    fn check(&self, id: u32) -> Result<()> {
        if id == self.id.raw() {
            Ok(())
        } else {
            required("4E6180 another actual Actor callback is required")
        }
    }
    fn kingdom(&self) -> Result<&SpecialShell> {
        if self.staged.actors.key(self.owner) != Some(self.owner_key) {
            return required("4E6180 current Kingdom generation changed");
        }
        let runtime =
            crate::fresh_post_create_world::canonical(self.staged, self.catalog, self.owner_key)
                .map_err(error)?;
        if runtime.definition.native_class != 4 {
            return required("4E6180 actual owner is not Kingdom");
        }
        let Source::Special(k) = &self
            .staged
            .actors
            .entity(self.owner_key)
            .map_err(error)?
            .source
        else {
            return required("4E6180 same derived Kingdom allocation absent");
        };
        if !matches!(k.source, SpecialSource::Kingdom(_)) {
            return required("4E6180 source is not Kingdom");
        }
        let derived = k
            .derived
            .as_ref()
            .ok_or_else(|| error("4E6180 derived Kingdom storage absent"))?;
        if derived.recruitment_owner != Some(self.owner) {
            return required("4E6180 Kingdom constructor owner differs");
        }
        Ok(derived)
    }
    fn kingdom_mut(&mut self) -> Result<&mut SpecialShell> {
        self.kingdom()?;
        let Source::Special(k) = &mut self
            .staged
            .actors
            .entity_mut(self.owner_key)
            .map_err(error)?
            .source
        else {
            unreachable!()
        };
        k.derived
            .as_mut()
            .ok_or_else(|| error("4E6180 Kingdom storage disappeared"))
    }
    fn word(&self, offset: u32) -> Result<u32> {
        self.kingdom()?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| error(format!("4E6180 actual Kingdom+{offset:X} unwritten")))
    }
    fn offset(list: ActorList) -> Result<u32> {
        match list {
            ActorList::Structures => Ok(0x2c4),
            ActorList::Settlements => Ok(0x2d0),
            ActorList::Hqs => Ok(0x2dc),
            ActorList::Organizations => Ok(0x2e8),
            ActorList::Flares => required("4E6180 flare list is not a native vector"),
        }
    }
}
impl RegistryWorld for Current<'_, '_> {
    fn actor(&self, id: u32) -> Result<RegistryActor> {
        self.check(id)?;
        let r = self.runtime()?;
        let a = r
            .actor
            .as_ref()
            .ok_or_else(|| error("4E6180 actual Actor storage absent"))?;
        let category = self
            .catalog
            .actors
            .native_category(&r.definition.proof.id)
            .map_err(error)?;
        if category != r.definition.proof.category {
            return required("4E6180 current class descriptor differs");
        }
        let has = |s| a.components.present(s);
        Ok(RegistryActor {
            class_index: category.runtime_class,
            class_kind: category.registry_index,
            flags8: self
                .staged
                .actors
                .placement(self.key, &ReadSpatial(self.staged))
                .map_err(error)?
                .flags,
            states: u64::from(a.components.state.low) | (u64::from(a.components.state.high) << 32),
            economy: has(Slot::Economy),
            structure: has(Slot::Structure),
            settlement: has(Slot::Settlement),
            hq: has(Slot::Hq),
            organization: has(Slot::Organization),
            supply: has(Slot::Supply),
            properties: has(Slot::Properties),
            component_ac: has(Slot::MobileDeployment),
            flare: has(Slot::Flare),
        })
    }
    fn set_flags8(&mut self, id: u32, flags: u32) -> Result<()> {
        self.check(id)?;
        let before = self.actor(id)?.flags8;
        let add = flags ^ before;
        if before & !flags != 0 || add & !0x18000000 != 0 {
            return required("4E6180 unexpected registration flag edit");
        }
        if add == 0 {
            return Ok(());
        }
        if self.staged.actors.staged_placement(self.key).is_ok() {
            self.staged
                .actors
                .staged_placement_mut(self.key)
                .map_err(error)?
                .flags |= add;
        } else {
            let w = self
                .staged
                .load_world
                .as_mut()
                .ok_or_else(|| error("4E6180 actual World absent"))?;
            let crate::load_world::SpatialOwner::Adopted(a) = &mut w.spatial else {
                return required("4E6180 same adopted placement required");
            };
            if a.error.is_some() || !a.pending.is_empty() {
                return required("4E6180 actual placement adoption incomplete");
            }
            a.world
                .or_registration_flags(self.key, add)
                .map_err(error)?;
        }
        Ok(())
    }
    fn structure_excluded(&mut self, id: u32) -> Result<bool> {
        self.check(id)?;
        let c = self.runtime()?.components().map_err(error)?;
        if let Some(c) = c.get(Slot::Structure) {
            let RuntimeComponent::Structure(s) = c else {
                return required("4CED60 Structure payload differs");
            };
            if s.header.owner != self.id {
                return required("4CED60 Structure owner differs");
            }
            if s.object_handle_10 != 0 {
                return Ok(true);
            }
        }
        if c.present(Slot::Wall) {
            return required("4CED60/58CB30 actual Wall getter required");
        }
        Ok(false)
    }
    fn economy_add(&mut self, id: u32) -> Result<()> {
        self.check(id)?;
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("570020 actual World absent"))?;
        let spatial = world.live_spatial().map_err(error)?;
        let (_, mut actors) = self.staged.actors.split_minds();
        crate::stored_economy_refresh::refresh(
            self.catalog,
            &mut actors,
            &mut self.staged.definitions,
            spatial,
            &world.variables,
            self.id,
            self.pc,
        )
        .map_err(error)
    }
    fn economy_remove(&mut self, _: u32) -> Result<()> {
        required("4E6180 does not remove Economy")
    }
    fn refresh_organization(&mut self, id: u32) -> Result<()> {
        let key = self
            .staged
            .actors
            .key(ActorId::from_raw(id))
            .ok_or_else(|| error("4D1350 current Organization absent"))?;
        crate::stored_actor_value_refresh::refresh_organization(
            self.catalog,
            &mut self.staged.actors,
            &self.staged.definitions,
            key,
            self.pc,
        )
        .map_err(error)
    }
    fn counted(&mut self, id: u32) -> Result<bool> {
        self.check(id)?;
        let r = self.runtime()?;
        let c = r.components().map_err(error)?;
        if let Some(c) = c.get(Slot::Body) {
            let RuntimeComponent::Body(b) = c else {
                return required("4E1540 Body payload differs");
            };
            if b.header.owner != self.id {
                return required("4E1540 Body owner differs");
            }
            let v = f32::from_bits(b.body.dying_started.0);
            if !v.is_finite() {
                return required("4E1540 finite Body dying marker domain");
            }
            if v >= 0.0 {
                return Ok(false);
            }
        }
        if r.definition.flags & ((1 << 7) | (1 << 16)) != 0 {
            return Ok(true);
        }
        if c.present(Slot::MobileDeployment) {
            return required("4E1540 actual Deployment primary definition required");
        }
        Ok(false)
    }
    fn statistics_present(&mut self) -> Result<bool> {
        Ok(self.word(0x32c)? != 0)
    }
    fn statistics_track(&mut self, _: u32, _: bool) -> Result<()> {
        required("58F010 actual statistics tracking required")
    }
    fn statistics_classified(&mut self, _: u32) -> Result<bool> {
        required("58EA80 actual statistics classification required")
    }
    fn add_score(&mut self, _: usize, _: u32) -> Result<()> {
        required("58E870 actual statistics score owner required")
    }
    fn ownership_track(&mut self, id: u32, add: bool) -> Result<()> {
        self.check(id)?;
        crate::kingdom_ownership_track::update(self, add).map_err(error)
    }
    fn property_records(&mut self) -> Result<Vec<PropertyApplication>> {
        if !self
            .kingdom()?
            .vectors
            .get(&0x318)
            .ok_or_else(|| error("4E6180 actual Kingdom318 owner absent"))?
            .values
            .is_empty()
        {
            return required("4E6180 positive stored Kingdom318 instance application required");
        }
        Ok(Vec::new())
    }
    fn apply_property(&mut self, _: u32, _: PropertyApplication) -> Result<()> {
        required("4CEF10 actual stored instance admission required")
    }
    fn remove_property_effect(&mut self, _: u32, _: PropertyApplication) -> Result<()> {
        required("4E6180 does not remove Property effects")
    }
    fn structure_properties(&mut self, id: u32) -> Result<Vec<u32>> {
        self.check(id)?;
        let source = crate::definition_fx::Source::bind_in_bank(
            self.catalog,
            &self.staged.definitions,
            &self.runtime()?.definition,
        )
        .map_err(error)?;
        let ids = self
            .catalog
            .actors
            .native_statistics_technology_properties(
                source.canonical_id(),
                source.source_precision(),
            )
            .map_err(error)?;
        ids.into_iter()
            .map(|id| {
                self.staged
                    .definitions
                    .intern("Property", &id)
                    .map_err(error)
            })
            .collect()
    }
    fn add_kingdom_property(&mut self, _: u32, _: u32) -> Result<()> {
        required("4E3AD0 positive stored Kingdom TechnologyProperty owner required")
    }
    fn remove_kingdom_property(&mut self, _: u32, _: u32) -> Result<()> {
        required("4E6180 does not remove TechnologyProperty")
    }
    fn max_flares(&mut self) -> Result<i32> {
        required("4B2E60 current MaxFlaresPerKingdom source required")
    }
    fn flare_time(&mut self, _: u32) -> Result<u32> {
        required("4E1ED0 actual stored flare time required")
    }
    fn destroy_actor(&mut self, _: u32) -> Result<()> {
        required("4F6760 actual stored actor destruction required")
    }
}
impl crate::kingdom_ownership_track::World for Current<'_, '_> {
    fn required_city_158(&mut self) -> std::result::Result<ActorId, String> {
        self.runtime()
            .map_err(|e| e.to_string())?
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&0x158))
            .copied()
            .map(ActorId::from_raw)
            .ok_or("4E3C70 actual Actor158 unbound".into())
    }
    fn resolve_current(&mut self, id: ActorId) -> std::result::Result<Option<ActorId>, String> {
        Ok(self.staged.actors.key(id).map(|_| id))
    }
    fn definition_24(&mut self, id: ActorId) -> std::result::Result<bool, String> {
        let key = self
            .staged
            .actors
            .key(id)
            .ok_or("4E3C70 selected generation changed")?;
        let r = self.staged.actors.object(key).map_err(|e| e.to_string())?;
        let bound = crate::live_object_init::Source::bind_in_bank(
            self.catalog,
            &self.staged.definitions,
            &r.definition,
        )
        .map_err(|e| e.to_string())?;
        if r.definition.native_class == 1 {
            return Err("4E3C70 ThingData vf24 remains required".into());
        }
        let fields = &self
            .staged
            .load_world
            .as_ref()
            .ok_or("4E3C70 actual World absent")?
            .source_fields;
        let data = fields
            .definition(&r.definition.proof.id)
            .map_err(|e| e.to_string())?;
        if fields.receipt().sources != self.catalog.actors.sources
            || fields.source_precision() != bound.source_precision()
            || data.token().source() != &r.definition.proof.source
            || data.native_kind() != r.definition.proof.category.native_kind
        {
            return Err("4E3C70 source/PC/definition owner differs".into());
        }
        fields
            .byte(&r.definition.proof.id, 0x34)
            .map(|v| v != 0)
            .map_err(|e| e.to_string())
    }
    fn current_actor(&self) -> ActorId {
        self.id
    }
    fn has_city_d4(&mut self) -> std::result::Result<bool, String> {
        match self
            .runtime()
            .map_err(|e| e.to_string())?
            .components()
            .map_err(|e| e.to_string())?
            .get(Slot::City)
        {
            None => Ok(false),
            Some(RuntimeComponent::City(c)) if c.header.owner == self.id => Ok(true),
            _ => Err("4E3C70 actual City allocation/owner differs".into()),
        }
    }
    fn list_30c(&mut self) -> std::result::Result<&mut Vec<ActorId>, String> {
        Ok(&mut self
            .kingdom_mut()
            .map_err(|e| e.to_string())?
            .actor_lists
            .as_mut()
            .ok_or("4E3C70 actual Kingdom30C list owner absent")?
            .city_ownership)
    }
}
impl StoredRegistryWorld for Current<'_, '_> {
    fn class_insert_head(&mut self, index: u32, id: u32) -> Result<()> {
        self.check(id)?;
        let list = self
            .kingdom_mut()?
            .actor_lists
            .as_mut()
            .ok_or_else(|| error("4E6180 class lists not constructed"))?
            .by_class
            .get_mut(index as usize)
            .ok_or_else(|| error("4E6180 native class index outside constructed enum"))?;
        list.try_reserve(1).map_err(error)?;
        list.insert(0, ActorId::from_raw(id));
        Ok(())
    }
    fn list_push(&mut self, list: ActorList, id: u32) -> Result<()> {
        let offset = Self::offset(list)?;
        self.kingdom_mut()?
            .vectors
            .get_mut(&offset)
            .ok_or_else(|| error("4E6180 actual Kingdom vector absent"))?
            .push(id)
            .map_err(error)
    }
    fn list_insert_head(&mut self, list: ActorList, id: u32) -> Result<()> {
        if list != ActorList::Flares {
            return required("4E6180 unexpected linked-list operation");
        }
        let v = &mut self
            .kingdom_mut()?
            .actor_lists
            .as_mut()
            .ok_or_else(|| error("4E6180 actual flare list absent"))?
            .flares;
        v.try_reserve(1).map_err(error)?;
        v.insert(0, ActorId::from_raw(id));
        Ok(())
    }
    fn list_len(&self, list: ActorList) -> Result<usize> {
        let k = self.kingdom()?;
        if list == ActorList::Flares {
            return Ok(k
                .actor_lists
                .as_ref()
                .ok_or_else(|| error("4E6180 flare list absent"))?
                .flares
                .len());
        }
        Ok(k.vectors
            .get(&Self::offset(list)?)
            .ok_or_else(|| error("4E6180 actual vector absent"))?
            .values
            .len())
    }
    fn list_at(&self, list: ActorList, index: usize) -> Result<u32> {
        let k = self.kingdom()?;
        if list == ActorList::Flares {
            return k
                .actor_lists
                .as_ref()
                .ok_or_else(|| error("4E6180 flare list absent"))?
                .flares
                .get(index)
                .map(|v| v.raw())
                .ok_or(EconomyError::ResourceIndex);
        }
        k.vectors
            .get(&Self::offset(list)?)
            .ok_or_else(|| error("4E6180 actual vector absent"))?
            .values
            .get(index)
            .copied()
            .ok_or(EconomyError::ResourceIndex)
    }
    fn list_contains(&self, list: ActorList, id: u32) -> Result<bool> {
        let k = self.kingdom()?;
        if list == ActorList::Flares {
            return Ok(k
                .actor_lists
                .as_ref()
                .ok_or_else(|| error("4E6180 flare list absent"))?
                .flares
                .contains(&ActorId::from_raw(id)));
        }
        Ok(k.vectors
            .get(&Self::offset(list)?)
            .ok_or_else(|| error("4E6180 actual vector absent"))?
            .values
            .contains(&id))
    }
    fn supply_contains(&self, id: u32) -> Result<bool> {
        Ok(self
            .kingdom()?
            .supply_sources
            .contains(&ActorId::from_raw(id)))
    }
    fn supply_insert_head(&mut self, id: u32) -> Result<()> {
        let v = &mut self.kingdom_mut()?.supply_sources;
        v.try_reserve(1).map_err(error)?;
        v.insert(0, ActorId::from_raw(id));
        Ok(())
    }
    fn increment_counted(&mut self) -> Result<()> {
        let v = self
            .kingdom_mut()?
            .words
            .get_mut(&0x2f4)
            .ok_or_else(|| error("4E6180 actual counted2F4 absent"))?;
        *v = v.wrapping_add(1);
        Ok(())
    }
    fn capital(&self) -> Result<Option<u32>> {
        match self.kingdom()?.identity_references.get(&0x1f8) {
            Some(v) => Ok(v.map(|id| id.raw())),
            None => {
                if self.word(0x1f8)? == 0 {
                    Ok(None)
                } else {
                    required("4E6180 actual capital pointer has no translated GTID")
                }
            }
        }
    }
    fn set_capital(&mut self, id: u32) -> Result<()> {
        let k = self.kingdom_mut()?;
        k.identity_references
            .insert(0x1f8, Some(ActorId::from_raw(id)));
        k.words.remove(&0x1f8);
        Ok(())
    }
    fn word_190(&self) -> Result<u32> {
        self.word(0x190)
    }
}
