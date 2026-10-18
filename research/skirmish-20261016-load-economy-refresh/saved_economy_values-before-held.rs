//! Native resource values over authoritative saved actors. The focused Economy
//! and held ancestor components are real borrowed allocations temporarily
//! outside their slots; they remain present and receive actual cache writes.
use crate::{catalog::Catalog, saved_organization_actions::SavedActions};
use aa_actors::{
    ActorId,
    components::ComponentSlot,
    economy_values::{
        self as values, ActorSnapshot, Definition, Error, GateWorld, Modifier, Result, VectorKind,
        World,
    },
    ground_states::EconomyState,
};
use aa_body::F32;
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::Precision;
use aa_rules::native::layout_data::NativeLayoutData;
use std::{borrow::Cow, sync::Arc};

fn error(message: impl std::fmt::Display) -> Error {
    Error::World(message.to_string())
}

pub(crate) struct View<'a, 'held, 'catalog, 'world, 'source> {
    actions: &'a mut SavedActions<'catalog, 'world, 'source>,
    focused: &'a mut EconomyState,
    held: &'a mut [(ActorId, &'held mut EconomyState)],
    precision: Precision,
}
impl<'a, 'held, 'catalog, 'world, 'source> View<'a, 'held, 'catalog, 'world, 'source> {
    pub(crate) fn new(
        actions: &'a mut SavedActions<'catalog, 'world, 'source>,
        focused: &'a mut EconomyState,
        held: &'a mut [(ActorId, &'held mut EconomyState)],
        precision: Precision,
    ) -> Self {
        Self {
            actions,
            focused,
            held,
            precision,
        }
    }

    fn catalog(&self) -> &Catalog<'source> {
        self.actions.catalog
    }
    fn check_precision(&self) -> Result<()> {
        if self.precision != self.actions.context.precision {
            return Err(error(
                "saved Economy value precision differs from restore context",
            ));
        }
        Ok(())
    }
    fn component(&self, actor: ActorId, slot: ComponentSlot) -> Result<Option<&RestoredComponent>> {
        self.actions.component(actor, slot).map_err(error)
    }
    fn organization_present(&self, actor: ActorId) -> Result<bool> {
        Ok(actor == self.actions.context.owner && self.actions.context.organization.is_some()
            || self
                .component(actor, ComponentSlot::Organization)?
                .is_some())
    }
    fn economy_present(&self, actor: ActorId) -> Result<bool> {
        if actor == self.focused.header.owner {
            return Ok(true);
        }
        if let Some((_, component)) = self.held.iter().find(|(id, _)| *id == actor) {
            if component.header.owner != actor {
                return Err(error("held Economy header owner identity mismatch"));
            }
            return Ok(true);
        }
        match self.component(actor, ComponentSlot::Economy)? {
            None => Ok(false),
            Some(RestoredComponent::Economy(value)) if value.header.owner == actor => Ok(true),
            Some(_) => Err(error("saved Economy payload/header identity mismatch")),
        }
    }
    fn economy_mut(&mut self, actor: ActorId) -> Result<&mut EconomyState> {
        if actor == self.focused.header.owner {
            return Ok(self.focused);
        }
        if let Some((_, component)) = self.held.iter_mut().find(|(id, _)| *id == actor) {
            if component.header.owner != actor {
                return Err(error("held Economy cache owner identity mismatch"));
            }
            return Ok(component);
        }
        match self
            .actions
            .shell_mut(actor)
            .map_err(error)?
            .components
            .get_mut(ComponentSlot::Economy)
        {
            Some(RestoredComponent::Economy(value)) if value.header.owner == actor => Ok(value),
            _ => Err(error("saved Economy cache payload/header identity missing")),
        }
    }
    fn thing_flags(&self, actor: ActorId) -> Result<u32> {
        if actor == self.actions.context.owner {
            return Ok(self.actions.context.owner_thing_flags);
        }
        self.actions
            .context
            .registry
            .get(actor)
            .and_then(|e| e.value.loaded())
            .map(|e| e.shell.thing.flags)
            .ok_or_else(|| error("saved Economy Thing flags missing"))
    }
    fn properties(&self, actor: ActorId) -> Result<&aa_body::properties::PropertiesState> {
        match self.component(actor, ComponentSlot::Properties)? {
            Some(RestoredComponent::Properties(value)) if value.owner.0 == actor.raw() => Ok(value),
            _ => Err(error(
                "saved Economy Properties context missing or owner mismatched",
            )),
        }
    }
    fn layout(&self, organization: ActorId) -> Result<(Arc<NativeLayoutData>, usize)> {
        let org = self.actions.org(organization).map_err(error)?;
        if org.header.owner != organization {
            return Err(error("saved Economy Organization owner mismatch"));
        }
        let data = self.catalog().layout_data(self.precision).map_err(error)?;
        let index = data
            .layouts
            .iter()
            .position(|d| d.ids.eq_ignore_ascii_case(&org.layout.definition))
            .ok_or_else(|| error("saved Economy canonical OrganizationLayout missing"))?;
        if data.layouts[index].error.is_some() {
            return Err(error("saved Economy OrganizationLayout load failed"));
        }
        Ok((data, index))
    }
    pub(crate) fn parent_organization(&self, actor: ActorId) -> Result<Option<ActorId>> {
        match self.component(actor, ComponentSlot::Element)? {
            None => Ok(None),
            Some(RestoredComponent::Element(value)) if value.header.owner == actor => {
                Ok(value.organization)
            }
            Some(_) => Err(error("saved Economy Element+10 payload/header mismatch")),
        }
    }
    ///570020 inline upkeep gate, before56FB10. Preserve native early exits.
    pub(crate) fn upkeep_blocked(&self, actor: ActorId) -> Result<bool> {
        self.check_precision()?;
        if self.parent_organization(actor)?.is_some() || self.thing_flags(actor)? & 1 != 0 {
            return Ok(true);
        }
        if self.organization_present(actor)? {
            let id = self.actions.source_id(actor).map_err(error)?.to_owned();
            return Ok(!values::recruitable(&self.definition(&id)?)?);
        }
        Ok(false)
    }
    pub(crate) fn provides(&self, actor: ActorId) -> Result<bool> {
        self.check_precision()?;
        values::eligible(actor, self)
    }
    pub(crate) fn cost_or_upkeep(&self, actor: ActorId, kind: VectorKind) -> Result<Vec<F32>> {
        self.check_precision()?;
        let mut out = Vec::new();
        values::cost_or_upkeep(&mut out, Some(actor), kind, self, self.precision)?;
        Ok(out)
    }
    pub(crate) fn production(&mut self, actor: ActorId, cache_members: bool) -> Result<Vec<F32>> {
        self.check_precision()?;
        let mut out = Vec::new();
        values::production(&mut out, Some(actor), cache_members, self, self.precision)?;
        Ok(out)
    }
}
impl GateWorld for View<'_, '_, '_, '_, '_> {
    type Definition = String;
    fn actor(&self, actor: ActorId) -> Result<ActorSnapshot<String>> {
        let shell = self.actions.shell(actor).map_err(error)?;
        let bits = shell.components.state;
        Ok(ActorSnapshot {
            definition: self.actions.source_id(actor).map_err(error)?.to_owned(),
            flags: self.thing_flags(actor)?,
            state_bits: u64::from(bits.low) | (u64::from(bits.high) << 32),
            element_organization: self.parent_organization(actor)?,
            organization: self.organization_present(actor)?.then_some(actor.raw()),
            economy: self.economy_present(actor)?.then_some(actor.raw()),
            // The lazy supplied override below is the authoritative+18 read.
            constructor18: None,
            transport: self
                .component(actor, ComponentSlot::Transport)?
                .map(|_| actor.raw()),
        })
    }
    fn definition(&self, id: &String) -> Result<Definition<String>> {
        self.catalog()
            .require_untranslated_actor_identity(id)
            .map_err(error)?;
        let binding = self
            .catalog()
            .actor_bindings(id, self.precision)
            .map_err(error)?;
        let mask = binding
            .definition
            .component_mask
            .ok_or_else(|| error("saved Economy ActorData mask missing"))?;
        let source = match &binding.source_economy {
            Some(source) => Cow::Borrowed(source),
            None => Cow::Owned(
                self.catalog()
                    .actors
                    .native_economy_inputs(id, self.precision)
                    .map_err(error)?,
            ),
        };
        if (mask & (1 << 9) != 0) != source.resources.is_some() {
            return Err(error(
                "saved Economy component/source resource presence mismatch",
            ));
        }
        let layout_kind18 = if let Some(org) = &binding.definition.ground.organization {
            let data = self.catalog().layout_data(self.precision).map_err(error)?;
            let layout = data
                .layouts
                .iter()
                .find(|d| d.ids.eq_ignore_ascii_case(&org.layout.definition))
                .ok_or_else(|| error("ActorData canonical OrganizationLayout missing"))?;
            if layout.error.is_some() {
                return Err(error("ActorData OrganizationLayout load failed"));
            }
            Some(layout.primary_slot)
        } else {
            None
        };
        // ActorData ctor4628D1..462943 initializes all three vector counts
        // to0 and supply_required(+348) to0. Absent Economy leaves them so.
        let (supply_required, cost, upkeep, production) = match &source.resources {
            Some(r) => (
                r.supply_required.boolean().map_err(error)?,
                r.cost_bits.iter().copied().map(F32).collect(),
                r.upkeep_bits.iter().copied().map(F32).collect(),
                r.production_bits.iter().copied().map(F32).collect(),
            ),
            None => (false, Vec::new(), Vec::new(), Vec::new()),
        };
        Ok(Definition {
            flags174: mask,
            class178: source.recruitable_word,
            // The lazy override is authoritative; this legacy snapshot is
            // never consulted by this View and is not a guessed backref.
            word17c: 0,
            layout_kind18,
            supply_required,
            packed_definition: None,
            cost,
            upkeep,
            production,
        })
    }
    fn packed_definition(&self, id: &String) -> Result<Option<String>> {
        self.catalog()
            .require_untranslated_actor_identity(id)
            .map_err(error)?;
        Ok(self
            .catalog()
            .native_mobile_deployment_data(self.precision)
            .map_err(error)?
            .definition(id)
            .map_err(error)?
            .pack_to
            .clone())
    }
    fn supplied(&self, actor: ActorId) -> Result<bool> {
        let mut snapshot = self.actor(actor)?;
        if snapshot.state_bits & (1 << 19) == 0 {
            return Ok(true);
        }
        snapshot.constructor18 = match self.component(actor, ComponentSlot::Supply)? {
            None => None,
            Some(RestoredComponent::Supply(value)) if value.header.owner == actor => {
                Some(value.stockpile)
            }
            Some(_) => return Err(error("saved Supply+18 payload/header mismatch")),
        };
        Ok(values::supplied(&snapshot))
    }
}
impl World for View<'_, '_, '_, '_, '_> {
    fn resource_count(&self) -> Result<usize> {
        self.catalog()
            .actors
            .native_resource_registry(self.precision)
            .map(|r| r.ids.len())
            .map_err(error)
    }
    fn actor_modifier(&self, actor: ActorId) -> Result<Option<u32>> {
        Ok(self
            .component(actor, ComponentSlot::Properties)?
            .map(|_| actor.raw()))
    }
    fn modifier(&self, context: u32) -> Result<Modifier> {
        let value = self.properties(ActorId::from_raw(context))?;
        Ok(Modifier {
            flags: value.modifier.flags,
            factor4c: value.modifier.scalar(0x4c)?,
            factor60: value.modifier.scalar(0x60)?,
            production_adjustments: Vec::new(),
        })
    }
    fn recruit_backref_present(&self, id: &String) -> Result<bool> {
        self.catalog()
            .native_recruit_backrefs(self.precision)
            .map_err(error)?
            .present(id)
            .map_err(error)
    }
    fn production_delta(&self, context: u32, index: usize) -> Result<aa_numeric::Binary> {
        aa_body::economy_modifiers::production_delta(
            &self
                .properties(ActorId::from_raw(context))?
                .modifier
                .production,
            self.resource_count()?,
            index,
        )
        .map_err(Error::Body)
    }
    fn transport_vector(&self, transport: u32, kind: VectorKind) -> Result<Vec<F32>> {
        let actor = ActorId::from_raw(transport);
        let Some(RestoredComponent::Transport(value)) =
            self.component(actor, ComponentSlot::Transport)?
        else {
            return Err(error("saved Transport resource cache missing"));
        };
        if value.owner != actor {
            return Err(error("saved Transport cache owner mismatch"));
        }
        match kind {
            VectorKind::Cost => Ok(value.cost.values.clone()),
            VectorKind::Upkeep => Ok(value.upkeep.values.clone()),
            VectorKind::Production => Err(error("native Transport has no production cache")),
        }
    }
    fn layout_slot_count(&self, organization: u32) -> Result<usize> {
        let (data, index) = self.layout(ActorId::from_raw(organization))?;
        Ok(data.layouts[index].slot_groups.len())
    }
    fn layout_slot_definition(&self, organization: u32, slot: usize) -> Result<Option<String>> {
        let actor = ActorId::from_raw(organization);
        let (data, index) = self.layout(actor)?;
        let group = *data.layouts[index]
            .slot_groups
            .get(slot)
            .ok_or_else(|| error("saved layout slot index"))? as usize;
        self.actions
            .org(actor)
            .map_err(error)?
            .layout
            .parameters
            .get(group)
            .cloned()
            .ok_or_else(|| error("saved layout group parameter index"))
    }
    fn member_count(&self, organization: u32) -> Result<usize> {
        Ok(self
            .actions
            .org(ActorId::from_raw(organization))
            .map_err(error)?
            .members
            .len())
    }
    fn member(&self, organization: u32, index: usize) -> Result<Option<ActorId>> {
        let id = *self
            .actions
            .org(ActorId::from_raw(organization))
            .map_err(error)?
            .members
            .get(index)
            .ok_or_else(|| error("saved Organization member index"))?;
        Ok((id != ActorId::NULL).then_some(id))
    }
    fn store_member_production(&mut self, economy: u32, values: &[F32]) -> Result<()> {
        let out = &mut self.economy_mut(ActorId::from_raw(economy))?.resources[3].values;
        out.try_reserve(values.len().saturating_sub(out.len()))
            .map_err(|_| Error::Allocation)?;
        out.clear();
        out.extend_from_slice(values);
        Ok(())
    }
}

#[cfg(test)]
#[path = "saved_economy_values_tests.rs"]
mod tests;
