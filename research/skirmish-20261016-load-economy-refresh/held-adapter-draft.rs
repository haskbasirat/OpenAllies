//! Shared570020/5704C0 refresh over a held Actor and actual stored Kingdom.
//! Native value/refresh/borrowed-flow kernels are reused without an ActorStore
//! snapshot or replacement Kingdom. Every Result path returns the same Economy.
use crate::{
    catalog::Catalog,
    economy_refresh as refresh,
    live_object_init::{Record, Source},
};
use aa_actors::{
    ActorId,
    components::ComponentSlot,
    economy_values::{
        self as values, ActorSnapshot, Definition, GateWorld, Modifier, VectorKind,
        World as ValueWorld,
    },
    ground_states::{EconomyState, OrganizationState},
    loading::{ActorShell, LoadKind, LoadShell},
};
use aa_body::F32;
use aa_mapworld::{
    bootstrap::DefinitionProof,
    restore::{RestoredComponent, RestoredRecord, RuntimeDefinition},
    special_state::SpecialShell,
};
use aa_numeric::Precision;
use std::{borrow::Cow, sync::Arc};

#[cfg(test)]
#[path = "load_economy_refresh_world_tests.rs"]
mod tests;
type Result<T> = std::result::Result<T, String>;
fn value_error(e: impl std::fmt::Display) -> values::Error {
    values::Error::World(e.to_string())
}
fn refresh_error(e: impl std::fmt::Display) -> refresh::Error {
    refresh::Error::World(e.to_string())
}
type Shell = LoadShell<DefinitionProof, RestoredComponent>;

fn parts<'a>(record: &'a Record<'_>, id: ActorId) -> Result<(&'a Shell, &'a RuntimeDefinition)> {
    if id == record.owner {
        return Ok((&*record.shell, record.definition));
    }
    match record.registry.get(id).map(|e| &e.value) {
        Some(RestoredRecord::Actor(a)) if a.source.serialized_id == id.raw() => {
            Ok((&a.shell, &a.definition))
        }
        Some(RestoredRecord::Kingdom(k)) if k.source.serialized_id == id.raw() => {
            let r = k
                .runtime
                .as_ref()
                .ok_or("Economy stored Kingdom runtime is absent")?;
            Ok((&r.base, &r.definition))
        }
        _ => Err(format!(
            "Economy current Actor generation/storage absent: {:08x}",
            id.raw()
        )),
    }
}
fn shell_mut<'a>(record: &'a mut Record<'_>, id: ActorId) -> Result<&'a mut Shell> {
    if id == record.owner {
        return Ok(&mut *record.shell);
    }
    match record.registry.value_mut(id) {
        Some(RestoredRecord::Actor(a)) if a.source.serialized_id == id.raw() => Ok(&mut a.shell),
        Some(RestoredRecord::Kingdom(k)) if k.source.serialized_id == id.raw() => Ok(&mut k
            .runtime
            .as_mut()
            .ok_or("Economy stored Kingdom runtime is absent")?
            .base),
        _ => Err("Economy current Actor storage disappeared".into()),
    }
}
fn actor<'a>(record: &'a Record<'_>, id: ActorId) -> Result<&'a ActorShell<RestoredComponent>> {
    parts(record, id)?
        .0
        .actor
        .as_ref()
        .ok_or_else(|| "Economy actual Actor shell is absent".into())
}
fn word(record: &Record<'_>, id: ActorId, offset: u32) -> Result<u32> {
    actor(record, id)?
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| format!("Economy actual Actor+{offset:X} is unbound"))
}
fn validate(
    catalog: &Catalog<'_>,
    record: &Record<'_>,
    id: ActorId,
    source_pc: Precision,
) -> Result<()> {
    let (shell, definition) = parts(record, id)?;
    let source = Source::bind(catalog, definition).map_err(|e| e.to_string())?;
    if id == record.owner {
        source.validate_record(record).map_err(|e| e.to_string())?;
    }
    if !matches!(definition.native_class, 3 | 4)
        || shell.kind != LoadKind::Actor
        || shell.actor.is_none()
        || shell.object.is_none()
        || !crate::load_spatial::same_proof(&shell.thing.definition, &definition.proof)
        || source.source_precision() != source_pc
        || definition.component_mask
            != Some(catalog.canonical_component_mask(&definition.proof.id, source_pc)?)
    {
        return Err("Economy canonical Actor source/constructor/component-mask differs".into());
    }
    Ok(())
}

/// Dispatch the actual held Actor's present Economy. A null native allegiance
/// returns before Resource/Kingdom/vector access, as570041..57004B require.
pub fn refresh(catalog: &Catalog<'_>, record: &mut Record<'_>, pc: Precision) -> Result<()> {
    let source_pc = {
        catalog.require_source_owner()?;
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|e| e.to_string())?;
        let source = bank.combat_source()?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err("Economy source Bank identities differ".into());
        }
        source.completed_means()?;
        source.source_precision()
    };
    refresh_actor(catalog, record, record.owner, &mut [], source_pc, pc)
}
fn refresh_actor(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    id: ActorId,
    held: &mut [(ActorId, &mut EconomyState)],
    source_pc: Precision,
    pc: Precision,
) -> Result<()> {
    if held.iter().any(|(owner, _)| *owner == id) {
        return Err("570020 recursive Economy ancestor cycle".into());
    }
    validate(catalog, record, id, source_pc)?;
    let present = actor(record, id)?.components.get(ComponentSlot::Economy);
    let Some(RestoredComponent::Economy(component)) = present else {
        return if present.is_none() {
            Ok(())
        } else {
            Err("Economy slot payload differs".into())
        };
    };
    if component.header.owner != id {
        return Err("Economy header/current Actor owner differs".into());
    }
    let owner = word(record, id, 0xe8)?;
    if owner == 0 {
        return Ok(());
    }
    let count = catalog
        .actors
        .native_resource_registry(source_pc)
        .map_err(|e| e.to_string())?
        .ids
        .len();
    let Some(RestoredComponent::Economy(mut component)) = shell_mut(record, id)?
        .actor
        .as_mut()
        .ok_or("Economy Actor disappeared")?
        .components
        .remove(ComponentSlot::Economy)
    else {
        unreachable!("validated Economy")
    };
    let result = refresh::refresh(
        &mut component,
        &mut Refresh {
            catalog,
            record,
            held,
            storage: id,
            first_owner: Some(ActorId::from_raw(owner)),
            source_pc,
            pc,
            count,
        },
        pc,
    )
    .map_err(|e| e.to_string());
    let replaced = shell_mut(record, id)?
        .actor
        .as_mut()
        .ok_or("Economy Actor disappeared")?
        .components
        .insert(
            ComponentSlot::Economy,
            RestoredComponent::Economy(component),
        );
    assert!(
        replaced.is_none(),
        "concrete Economy getters cannot replace the borrowed allocation"
    );
    result
}

struct Refresh<'a, 'r, 's, 'h> {
    catalog: &'a Catalog<'s>,
    record: &'a mut Record<'r>,
    held: &'a mut [(ActorId, &'h mut EconomyState)],
    storage: ActorId,
    first_owner: Option<ActorId>,
    source_pc: Precision,
    pc: Precision,
    count: usize,
}
impl<'r, 's, 'h> Refresh<'_, 'r, 's, 'h> {
    fn view<'a>(&'a mut self, focused: &'a mut EconomyState) -> View<'a, 'r, 's, 'h> {
        View {
            catalog: self.catalog,
            record: self.record,
            focused,
            held: self.held,
            source_pc: self.source_pc,
            pc: self.pc,
        }
    }
    fn adjust(
        &mut self,
        owner: ActorId,
        values: &[aa_economy::F32Bits],
        add: bool,
        providers: bool,
    ) -> refresh::Result<()> {
        let definitions = self
            .catalog
            .native_resource_economy(self.source_pc)
            .map_err(refresh_error)?;
        let count = crate::saved_kingdom_flows::consumption_count(&definitions)?;
        validate(self.catalog, self.record, owner, self.source_pc).map_err(refresh_error)?;
        let catalog = self.catalog;
        let pc = self.pc;
        let kingdom = kingdom_mut(self.record, owner).map_err(refresh_error)?;
        let mut cap = || crate::saved_kingdom_flows::provider_maximum(catalog, pc);
        let mut world = crate::saved_kingdom_flows::BorrowedKingdom {
            kingdom,
            resources: definitions.catalog(),
            consumption_count: count,
            provider_maximum: &mut cap,
        };
        let p = match pc {
            Precision::Single24 => aa_economy::X87Precision::Single24,
            Precision::Double53 => aa_economy::X87Precision::Double53,
            Precision::Extended64 => aa_economy::X87Precision::Extended64,
        };
        if providers {
            aa_economy::borrowed_flows::adjust_providers(&mut world, values, add, p)?;
        } else {
            aa_economy::borrowed_flows::adjust_consumption(&mut world, values, add, p)?;
        }
        Ok(())
    }
}
fn kingdom_mut<'a>(record: &'a mut Record<'_>, id: ActorId) -> Result<&'a mut SpecialShell> {
    if id == record.owner {
        return Err(
            "Economy flow owner is held; actual Kingdom derived allocation is required".into(),
        );
    }
    let Some(RestoredRecord::Kingdom(k)) = record.registry.value_mut(id) else {
        return Err("Economy stored Kingdom generation is absent".into());
    };
    if k.source.serialized_id != id.raw() {
        return Err("Economy stored Kingdom identity differs".into());
    }
    let d = k
        .runtime
        .as_mut()
        .and_then(|r| r.derived.as_mut())
        .ok_or("Economy actual Kingdom derived storage is absent")?;
    if d.recruitment_owner != Some(id) {
        return Err("Economy actual Kingdom constructor owner differs".into());
    }
    Ok(d)
}
impl refresh::World for Refresh<'_, '_, '_, '_> {
    type Kingdom = ActorId;
    fn resource_count(&self) -> usize {
        self.count
    }
    fn owner(&mut self, id: ActorId, c: &mut EconomyState) -> refresh::Result<Option<ActorId>> {
        if id != self.storage || c.header.owner != id {
            return Err(refresh_error("focused Economy owner differs"));
        }
        if let Some(owner) = self.first_owner.take() {
            return Ok(Some(owner));
        }
        let v = word(self.record, id, 0xe8).map_err(refresh_error)?;
        Ok((v != 0).then_some(ActorId::from_raw(v)))
    }
    fn upkeep_blocked(&mut self, id: ActorId, c: &mut EconomyState) -> refresh::Result<bool> {
        self.view(c).upkeep_blocked(id).map_err(refresh_error)
    }
    fn upkeep_values(
        &mut self,
        id: ActorId,
        c: &mut EconomyState,
        p: Precision,
    ) -> refresh::Result<Vec<F32>> {
        self.view(c)
            .cost_or_upkeep(id, VectorKind::Upkeep, p)
            .map_err(refresh_error)
    }
    fn provides(&mut self, id: ActorId, c: &mut EconomyState) -> refresh::Result<bool> {
        values::eligible(id, &self.view(c)).map_err(refresh_error)
    }
    fn provided_values(
        &mut self,
        id: ActorId,
        c: &mut EconomyState,
        cache: bool,
        p: Precision,
    ) -> refresh::Result<Vec<F32>> {
        self.view(c).production(id, cache, p).map_err(refresh_error)
    }
    fn cost_values(
        &mut self,
        id: ActorId,
        c: &mut EconomyState,
        p: Precision,
    ) -> refresh::Result<Vec<F32>> {
        self.view(c)
            .cost_or_upkeep(id, VectorKind::Cost, p)
            .map_err(refresh_error)
    }
    fn kingdom_economy(&mut self, _: ActorId) -> refresh::Result<&mut aa_economy::Economy> {
        Err(refresh_error(
            "held refresh mutates actual borrowed Kingdom arrays only",
        ))
    }
    fn adjust_consumption(
        &mut self,
        o: ActorId,
        v: &[aa_economy::F32Bits],
        add: bool,
    ) -> refresh::Result<()> {
        self.adjust(o, v, add, false)
    }
    fn adjust_providers(
        &mut self,
        o: ActorId,
        v: &[aa_economy::F32Bits],
        add: bool,
    ) -> refresh::Result<()> {
        self.adjust(o, v, add, true)
    }
    fn parent_organization(
        &mut self,
        id: ActorId,
        c: &mut EconomyState,
    ) -> refresh::Result<Option<ActorId>> {
        self.view(c).parent(id).map_err(refresh_error)
    }
    fn refresh_parent(
        &mut self,
        id: ActorId,
        current: &mut EconomyState,
        p: Precision,
    ) -> refresh::Result<()> {
        let mut held = Vec::new();
        held.try_reserve_exact(self.held.len() + 1)
            .map_err(|_| refresh::Error::Allocation)?;
        for (id, c) in self.held.iter_mut() {
            held.push((*id, &mut **c));
        }
        held.push((self.storage, current));
        refresh_actor(self.catalog, self.record, id, &mut held, self.source_pc, p)
            .map_err(refresh_error)
    }
}

struct View<'a, 'r, 's, 'h> {
    catalog: &'a Catalog<'s>,
    record: &'a mut Record<'r>,
    focused: &'a mut EconomyState,
    held: &'a mut [(ActorId, &'h mut EconomyState)],
    source_pc: Precision,
    pc: Precision,
}
impl View<'_, '_, '_, '_> {
    fn component(
        &self,
        id: ActorId,
        s: ComponentSlot,
    ) -> values::Result<Option<&RestoredComponent>> {
        Ok(actor(self.record, id)
            .map_err(value_error)?
            .components
            .get(s))
    }
    fn org(&self, id: ActorId) -> values::Result<&OrganizationState<String>> {
        match self.component(id, ComponentSlot::Organization)? {
            Some(RestoredComponent::Organization(v)) if v.header.owner == id => Ok(v),
            _ => Err(value_error("actual Organization payload/header differs")),
        }
    }
    fn parent(&self, id: ActorId) -> values::Result<Option<ActorId>> {
        match self.component(id, ComponentSlot::Element)? {
            None => Ok(None),
            Some(RestoredComponent::Element(v)) if v.header.owner == id => Ok(v.organization),
            _ => Err(value_error("actual Element payload/header differs")),
        }
    }
    fn economy_present(&self, id: ActorId) -> values::Result<bool> {
        if self.focused.header.owner == id {
            return Ok(true);
        }
        if let Some((_, c)) = self.held.iter().find(|(owner, _)| *owner == id) {
            if c.header.owner != id {
                return Err(value_error("held ancestor Economy owner differs"));
            }
            return Ok(true);
        }
        match self.component(id, ComponentSlot::Economy)? {
            None => Ok(false),
            Some(RestoredComponent::Economy(v)) if v.header.owner == id => Ok(true),
            _ => Err(value_error("actual Economy payload/header differs")),
        }
    }
    fn economy_mut(&mut self, id: ActorId) -> values::Result<&mut EconomyState> {
        if self.focused.header.owner == id {
            return Ok(self.focused);
        }
        if let Some((_, c)) = self.held.iter_mut().find(|(owner, _)| *owner == id) {
            if c.header.owner != id {
                return Err(value_error("held ancestor Economy owner differs"));
            }
            return Ok(c);
        }
        match shell_mut(self.record, id)
            .map_err(value_error)?
            .actor
            .as_mut()
            .ok_or_else(|| value_error("actual Actor missing"))?
            .components
            .get_mut(ComponentSlot::Economy)
        {
            Some(RestoredComponent::Economy(v)) if v.header.owner == id => Ok(v),
            _ => Err(value_error("actual Economy cache payload/header differs")),
        }
    }
    fn properties(&self, id: ActorId) -> values::Result<&aa_body::properties::PropertiesState> {
        match self.component(id, ComponentSlot::Properties)? {
            Some(RestoredComponent::Properties(v)) if v.owner.0 == id.raw() => Ok(v),
            _ => Err(value_error("actual Properties owner/payload differs")),
        }
    }
    fn layout(
        &self,
        id: ActorId,
    ) -> values::Result<(Arc<aa_rules::native::layout_data::NativeLayoutData>, usize)> {
        let org = self.org(id)?;
        let data = self
            .catalog
            .layout_data(self.source_pc)
            .map_err(value_error)?;
        let index = data
            .layouts
            .iter()
            .position(|d| d.ids.eq_ignore_ascii_case(&org.layout.definition))
            .ok_or_else(|| value_error("actual OrganizationLayout is absent"))?;
        if data.layouts[index].error.is_some() {
            return Err(value_error("actual OrganizationLayout failed"));
        }
        Ok((data, index))
    }
    fn upkeep_blocked(&self, id: ActorId) -> values::Result<bool> {
        if self.parent(id)?.is_some()
            || parts(self.record, id).map_err(value_error)?.0.thing.flags & 1 != 0
        {
            return Ok(true);
        }
        if self.component(id, ComponentSlot::Organization)?.is_some() {
            let name = parts(self.record, id)
                .map_err(value_error)?
                .1
                .proof
                .id
                .clone();
            return Ok(!values::recruitable(&self.definition(&name)?)?);
        }
        Ok(false)
    }
    fn cost_or_upkeep(
        &self,
        id: ActorId,
        kind: VectorKind,
        p: Precision,
    ) -> values::Result<Vec<F32>> {
        if p != self.pc {
            return Err(value_error("Economy arithmetic PC differs"));
        }
        let mut out = Vec::new();
        values::cost_or_upkeep(&mut out, Some(id), kind, self, p)?;
        Ok(out)
    }
    fn production(&mut self, id: ActorId, cache: bool, p: Precision) -> values::Result<Vec<F32>> {
        if p != self.pc {
            return Err(value_error("Economy arithmetic PC differs"));
        }
        let mut out = Vec::new();
        values::production(&mut out, Some(id), cache, self, p)?;
        Ok(out)
    }
}
impl GateWorld for View<'_, '_, '_, '_> {
    type Definition = String;
    fn actor(&self, id: ActorId) -> values::Result<ActorSnapshot<String>> {
        validate(self.catalog, self.record, id, self.source_pc).map_err(value_error)?;
        let (shell, definition) = parts(self.record, id).map_err(value_error)?;
        let bits = shell
            .actor
            .as_ref()
            .ok_or_else(|| value_error("actual Actor shell is absent"))?
            .components
            .state;
        Ok(ActorSnapshot {
            definition: definition.proof.id.clone(),
            flags: shell.thing.flags,
            state_bits: u64::from(bits.low) | (u64::from(bits.high) << 32),
            element_organization: self.parent(id)?,
            organization: self
                .component(id, ComponentSlot::Organization)?
                .map(|_| id.raw()),
            economy: self.economy_present(id)?.then_some(id.raw()),
            constructor18: None,
            transport: self
                .component(id, ComponentSlot::Transport)?
                .map(|_| id.raw()),
        })
    }
    fn definition(&self, id: &String) -> values::Result<Definition<String>> {
        self.catalog
            .require_untranslated_actor_identity(id)
            .map_err(value_error)?;
        let binding = self
            .catalog
            .actor_bindings(id, self.source_pc)
            .map_err(value_error)?;
        let mask = binding
            .definition
            .component_mask
            .ok_or_else(|| value_error("ActorData component mask absent"))?;
        let source = match &binding.source_economy {
            Some(v) => Cow::Borrowed(v),
            None => Cow::Owned(
                self.catalog
                    .actors
                    .native_economy_inputs(id, self.source_pc)
                    .map_err(value_error)?,
            ),
        };
        if (mask & (1 << 9) != 0) != source.resources.is_some() {
            return Err(value_error(
                "ActorData Economy source/mask presence differs",
            ));
        }
        let layout_kind18 = if let Some(org) = &binding.definition.ground.organization {
            let data = self
                .catalog
                .layout_data(self.source_pc)
                .map_err(value_error)?;
            let layout = data
                .layouts
                .iter()
                .find(|d| d.ids.eq_ignore_ascii_case(&org.layout.definition))
                .ok_or_else(|| value_error("ActorData OrganizationLayout absent"))?;
            if layout.error.is_some() {
                return Err(value_error("ActorData OrganizationLayout failed"));
            }
            Some(layout.primary_slot)
        } else {
            None
        };
        // Actual4628D1..462943 null/count stores, already proved by saved values.
        let (supply_required, cost, upkeep, production) = match &source.resources {
            Some(r) => (
                r.supply_required.boolean().map_err(value_error)?,
                r.cost_bits.iter().copied().map(F32).collect(),
                r.upkeep_bits.iter().copied().map(F32).collect(),
                r.production_bits.iter().copied().map(F32).collect(),
            ),
            None => (false, Vec::new(), Vec::new(), Vec::new()),
        };
        Ok(Definition {
            flags174: mask,
            class178: source.recruitable_word,
            word17c: 0,
            layout_kind18,
            supply_required,
            packed_definition: None,
            cost,
            upkeep,
            production,
        })
    }
    fn packed_definition(&self, id: &String) -> values::Result<Option<String>> {
        self.catalog
            .require_untranslated_actor_identity(id)
            .map_err(value_error)?;
        Ok(self
            .catalog
            .native_mobile_deployment_data(self.source_pc)
            .map_err(value_error)?
            .definition(id)
            .map_err(value_error)?
            .pack_to
            .clone())
    }
    fn supplied(&self, id: ActorId) -> values::Result<bool> {
        let mut snapshot = self.actor(id)?;
        if snapshot.state_bits & (1 << 19) == 0 {
            return Ok(true);
        }
        snapshot.constructor18 = match self.component(id, ComponentSlot::Supply)? {
            None => None,
            Some(RestoredComponent::Supply(v)) if v.header.owner == id => Some(v.stockpile),
            _ => return Err(value_error("actual Supply payload/header differs")),
        };
        Ok(values::supplied(&snapshot))
    }
}
impl ValueWorld for View<'_, '_, '_, '_> {
    fn resource_count(&self) -> values::Result<usize> {
        Ok(self
            .catalog
            .actors
            .native_resource_registry(self.source_pc)
            .map_err(value_error)?
            .ids
            .len())
    }
    fn actor_modifier(&self, id: ActorId) -> values::Result<Option<u32>> {
        Ok(self
            .component(id, ComponentSlot::Properties)?
            .map(|_| id.raw()))
    }
    fn modifier(&self, id: u32) -> values::Result<Modifier> {
        let p = self.properties(ActorId::from_raw(id))?;
        Ok(Modifier {
            flags: p.modifier.flags,
            factor4c: p.modifier.scalar(0x4c)?,
            factor60: p.modifier.scalar(0x60)?,
            production_adjustments: Vec::new(),
        })
    }
    fn recruit_backref_present(&self, id: &String) -> values::Result<bool> {
        self.catalog
            .native_recruit_backrefs(self.source_pc)
            .map_err(value_error)?
            .present(id)
            .map_err(value_error)
    }
    fn production_delta(&self, id: u32, index: usize) -> values::Result<aa_numeric::Binary> {
        aa_body::economy_modifiers::production_delta(
            &self.properties(ActorId::from_raw(id))?.modifier.production,
            self.resource_count()?,
            index,
        )
        .map_err(values::Error::Body)
    }
    fn transport_vector(&self, id: u32, kind: VectorKind) -> values::Result<Vec<F32>> {
        let actor = ActorId::from_raw(id);
        let Some(RestoredComponent::Transport(v)) =
            self.component(actor, ComponentSlot::Transport)?
        else {
            return Err(value_error("actual Transport cache absent"));
        };
        if v.owner != actor {
            return Err(value_error("actual Transport owner differs"));
        }
        match kind {
            VectorKind::Cost => Ok(v.cost.values.clone()),
            VectorKind::Upkeep => Ok(v.upkeep.values.clone()),
            VectorKind::Production => Err(value_error("Transport has no native production cache")),
        }
    }
    fn layout_slot_count(&self, id: u32) -> values::Result<usize> {
        let (d, i) = self.layout(ActorId::from_raw(id))?;
        Ok(d.layouts[i].slot_groups.len())
    }
    fn layout_slot_definition(&self, id: u32, slot: usize) -> values::Result<Option<String>> {
        let actor = ActorId::from_raw(id);
        let (d, i) = self.layout(actor)?;
        let group = *d.layouts[i]
            .slot_groups
            .get(slot)
            .ok_or_else(|| value_error("actual layout slot index"))? as usize;
        self.org(actor)?
            .layout
            .parameters
            .get(group)
            .cloned()
            .ok_or_else(|| value_error("actual layout parameter index"))
    }
    fn member_count(&self, id: u32) -> values::Result<usize> {
        Ok(self.org(ActorId::from_raw(id))?.members.len())
    }
    fn member(&self, id: u32, index: usize) -> values::Result<Option<ActorId>> {
        let id = *self
            .org(ActorId::from_raw(id))?
            .members
            .get(index)
            .ok_or_else(|| value_error("actual Organization member index"))?;
        Ok((!id.is_null()).then_some(id))
    }
    fn store_member_production(&mut self, id: u32, values: &[F32]) -> values::Result<()> {
        let out = &mut self.economy_mut(ActorId::from_raw(id))?.resources[3].values;
        out.try_reserve(values.len().saturating_sub(out.len()))
            .map_err(|_| values::Error::Allocation)?;
        out.clear();
        out.extend_from_slice(values);
        Ok(())
    }
}
