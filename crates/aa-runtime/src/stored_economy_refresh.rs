//!570020 on the same Economy focus, stored Kingdom vectors and current Vars.
//! All arithmetic/order lives in existing economy_refresh/economy_values and
//! aa_economy::borrowed_flows kernels; these are field bindings only.
use crate::{
    actor_world::{ActorAccess, EconomyAccess, ObjectKey, RuntimeComponent, Source},
    catalog::Catalog,
    definitions::Bank,
    economy_refresh as native,
    load_view_variables::{CurrentValue, State},
    spatial_world::SpatialWorld,
    stored_economy_values::View,
};
use aa_actors::{
    ActorId, components::ComponentSlot as Slot, economy_values::VectorKind,
    ground_states::EconomyState,
};
use aa_body::F32;
use aa_numeric::Precision;
type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> native::Error {
    native::Error::World(e.to_string())
}
fn word(access: &EconomyAccess<'_>, actor: ActorId, offset: u32) -> native::Result<u32> {
    let key = access
        .key(actor)
        .ok_or_else(|| error("570020 actual current Actor generation absent"))?;
    access
        .object(key)
        .map_err(error)?
        .actor
        .as_ref()
        .and_then(|a| a.words.get(&offset))
        .copied()
        .ok_or_else(|| error(format!("570020 actual Actor+{offset:X} unwritten")))
}
fn cap(variables: &State) -> std::result::Result<i32, aa_economy::EconomyError> {
    let fail = |s: &str| aa_economy::EconomyError::World(s.into());
    let index = variables
        .registrations()
        .find(|v| v.name().eq_ignore_ascii_case("EconomyLimitedResourceMax"))
        .ok_or_else(|| fail("current EconomyLimitedResourceMax registration absent"))?
        .ordinal() as usize;
    match variables.current(index).map_err(|e| fail(&e))? {
        CurrentValue::Int(v) => Ok(*v as i32),
        _ => Err(fail("current EconomyLimitedResourceMax is not native Int")),
    }
}
pub fn refresh(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    spatial: &SpatialWorld,
    variables: &State,
    actor: ActorId,
    pc: Precision,
) -> Result<()> {
    let key = actors
        .key(actor)
        .ok_or("570020 actual current GTID absent")?;
    let component = actors
        .component(actor, Slot::Economy)
        .map_err(|e| e.to_string())?;
    match component {
        None => return Ok(()),
        Some(RuntimeComponent::Economy(e)) if e.header.owner == actor => {}
        _ => return Err("570020 actual Economy allocation/owner differs".into()),
    }
    //Nativevirtual108 is the first meaningful read. A null owner skips source,
    //flow, values, current Vars and all component vector reads.
    let owner = *actors
        .object(key)
        .map_err(|e| e.to_string())?
        .actor
        .as_ref()
        .ok_or("570020 actual Actor absent")?
        .words
        .get(&0xe8)
        .ok_or("570020 Actor+E8 unwritten")?;
    if owner == 0 {
        return Ok(());
    }
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("570020 source Bank universe differs".into());
    }
    let source_pc = source.source_precision();
    let object = actors.object(key).map_err(|e| e.to_string())?;
    crate::live_object_init::Source::bind_in_bank(catalog, bank, &object.definition)
        .map_err(|e| e.to_string())?;
    if object.definition.native_class != 3 {
        return Err("570020 actual ordinary Actor dispatch required".into());
    }
    let mut access = actors.economy_access().map_err(|e| e.to_string())?;
    refresh_focus(
        catalog,
        &mut access,
        bank,
        spatial,
        variables,
        actor,
        pc,
        source_pc,
        &mut Vec::new(),Some(ActorId::from_raw(owner)),
    )
}
fn refresh_focus(
    catalog: &Catalog<'_>,
    access: &mut EconomyAccess<'_>,
    bank: &Bank,
    spatial: &SpatialWorld,
    variables: &State,
    actor: ActorId,
    pc: Precision,
    source_pc: Precision,
    path: &mut Vec<ActorId>,
    captured_owner:Option<ActorId>,
) -> Result<()> {
    if path.contains(&actor) {
        return Err("570020 invalid cyclic parent Economy graph".into());
    }
    let key = access
        .key(actor)
        .ok_or("570020 focus current GTID absent")?;
    if !access
        .object(key)
        .map_err(|e| e.to_string())?
        .components()
        .map_err(|e| e.to_string())?
        .present(Slot::Economy)
    {
        return Ok(());
    }
    let first_owner=match captured_owner{Some(owner)=>owner,None=>ActorId::from_raw(word(access,actor,0xe8).map_err(|e|e.to_string())?)};
    if first_owner.is_null(){return Ok(())}
    let component = access.take_focus(key).map_err(|e| e.to_string())?;
    if component.header.owner != actor {
        return Err("570020 focus Economy owner differs".into());
    }
    path.push(actor);
    let count = catalog.native_resource_economy(source_pc)?.resource_count();
    let result = native::refresh(
        component,
        &mut Refresh {
            catalog,
            access,
            bank,
            spatial,
            variables,
            key,
            source_pc,
            pc,
            path,
            count,
            first_owner:Some(first_owner),
        },
        pc,
    )
    .map_err(|e| e.to_string());
    path.pop();
    result
}
struct Refresh<'a, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    access: &'a mut EconomyAccess<'store>,
    bank: &'a Bank,
    spatial: &'a SpatialWorld,
    variables: &'a State,
    key: ObjectKey,
    source_pc: Precision,
    pc: Precision,
    path: &'a mut Vec<ActorId>,
    count: usize,
    first_owner:Option<ActorId>,
}
impl<'a,'store,'source> Refresh<'a,'store,'source> {
    fn values<'b>(&'b mut self, current: &'b mut EconomyState) -> View<'b,'store,'source,SpatialWorld> {
        View {
            actors: &mut *self.access,
            focused: current,
            spatial: self.spatial,
            catalog: self.catalog,
            precision: self.pc,
            source_pc: self.source_pc,
        }
    }
    fn flows(
        &mut self,
        owner: ActorId,
        values: &[aa_economy::F32Bits],
        add: bool,
        providers: bool,
    ) -> native::Result<()> {
        let definitions = self
            .catalog
            .native_resource_economy(self.source_pc)
            .map_err(error)?;
        let count = crate::saved_kingdom_flows::consumption_count(&definitions)?;
        let key = self
            .access
            .key(owner)
            .ok_or_else(|| error("570020 actual owner Kingdom generation absent"))?;
        if self
            .access
            .object(key)
            .map_err(error)?
            .definition
            .native_class
            != 4
        {
            return Err(error("570020 owner actual Kingdom dispatch differs"));
        }
        let entity = self.access.entity_mut(key).map_err(error)?;
        let Source::Special(special) = entity.source else {
            return Err(error("570020 actual owned Kingdom derived storage absent"));
        };
        let kingdom = special
            .derived
            .as_mut()
            .ok_or_else(|| error("570020 actual Kingdom flow arrays absent"))?;
        let mut current_cap = || {
            self.variables
                .validate_in_bank(self.catalog, self.bank)
                .map_err(aa_economy::EconomyError::World)?;
            cap(self.variables)
        };
        let mut world = crate::saved_kingdom_flows::BorrowedKingdom {
            kingdom,
            resources: definitions.catalog(),
            consumption_count: count,
            provider_maximum: &mut current_cap,
        };
        let pc = match self.pc {
            Precision::Single24 => aa_economy::X87Precision::Single24,
            Precision::Double53 => aa_economy::X87Precision::Double53,
            Precision::Extended64 => aa_economy::X87Precision::Extended64,
        };
        if providers {
            aa_economy::borrowed_flows::adjust_providers(&mut world, values, add, pc)?
        } else {
            aa_economy::borrowed_flows::adjust_consumption(&mut world, values, add, pc)?
        }
        Ok(())
    }
}
impl native::World for Refresh<'_, '_, '_> {
    type Kingdom = ActorId;
    fn resource_count(&self) -> usize {
        self.count
    }
    fn owner(&mut self, a: ActorId, current: &mut EconomyState) -> native::Result<Option<ActorId>> {
        if current.header.owner != a {
            return Err(error("570020 current Economy header changed"));
        }
        if let Some(owner)=self.first_owner.take(){return Ok(Some(owner))}
        let id = ActorId::from_raw(word(self.access, a, 0xe8)?);
        Ok((!id.is_null()).then_some(id))
    }
    fn upkeep_blocked(&mut self, a: ActorId, c: &mut EconomyState) -> native::Result<bool> {
        self.values(c).upkeep_blocked(a).map_err(error)
    }
    fn upkeep_values(
        &mut self,
        a: ActorId,
        c: &mut EconomyState,
        _: Precision,
    ) -> native::Result<Vec<F32>> {
        self.values(c)
            .cost_or_upkeep(a, VectorKind::Upkeep)
            .map_err(error)
    }
    fn provides(&mut self, a: ActorId, c: &mut EconomyState) -> native::Result<bool> {
        self.values(c).provides(a).map_err(error)
    }
    fn provided_values(
        &mut self,
        a: ActorId,
        c: &mut EconomyState,
        cache: bool,
        _: Precision,
    ) -> native::Result<Vec<F32>> {
        self.values(c).production(a, cache).map_err(error)
    }
    fn cost_values(
        &mut self,
        a: ActorId,
        c: &mut EconomyState,
        _: Precision,
    ) -> native::Result<Vec<F32>> {
        self.values(c)
            .cost_or_upkeep(a, VectorKind::Cost)
            .map_err(error)
    }
    fn kingdom_economy(&mut self, _: ActorId) -> native::Result<&mut aa_economy::Economy> {
        Err(error(
            "570020 uses same Kingdom arrays, never a replacement Economy",
        ))
    }
    fn adjust_consumption(
        &mut self,
        k: ActorId,
        v: &[aa_economy::F32Bits],
        add: bool,
    ) -> native::Result<()> {
        self.flows(k, v, add, false)
    }
    fn adjust_providers(
        &mut self,
        k: ActorId,
        v: &[aa_economy::F32Bits],
        add: bool,
    ) -> native::Result<()> {
        self.flows(k, v, add, true)
    }
    fn parent_organization(
        &mut self,
        a: ActorId,
        _: &mut EconomyState,
    ) -> native::Result<Option<ActorId>> {
        match self.access.component(a, Slot::Element).map_err(error)? {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == a => Ok(e.organization),
            _ => Err(error("570020 actual Element parent field differs")),
        }
    }
    fn refresh_parent(
        &mut self,
        a: ActorId,
        current: &mut EconomyState,
        pc: Precision,
    ) -> native::Result<()> {
        let mut children = self
            .access
            .reborrow_with(self.key, current)
            .map_err(error)?;
        refresh_focus(
            self.catalog,
            &mut children,
            self.bank,
            self.spatial,
            self.variables,
            a,
            pc,
            self.source_pc,
            self.path,None,
        )
        .map_err(error)
    }
}
