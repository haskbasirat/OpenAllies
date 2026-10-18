//! Resource getters over the same ActorStore components used by gameplay.
//! Opaque getter context IDs are current actor GTIDs (one component per slot),
//! not process pointers or queue owners. No mutable actor snapshot is retained.
use crate::{
    actor_world::{ActorStore, RuntimeComponent, Spatial},
    catalog::Catalog,
};
use aa_actors::{
    ActorId,
    components::ComponentSlot,
    economy_values::{
        self as values, ActorSnapshot, Definition, Error, GateWorld, Modifier, Result, VectorKind,
        World,
    },
};
use aa_body::F32;
use aa_numeric::Precision;
use std::collections::BTreeMap;

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
pub struct View<'a, 'source, S> {
    pub actors: &'a mut ActorStore,
    pub spatial: &'a S,
    pub catalog: &'a Catalog<'source>,
    pub precision: Precision,
    /// Completed native4605B0 inverse bindings. A missing key is unresolved;
    /// Some(None) is a proven native null. Do not invert filesystem scan order.
    pub packed_definitions: &'a BTreeMap<String, Option<String>>,
}
impl<S: Spatial> View<'_, '_, S> {
    fn component(&self, id: ActorId, slot: ComponentSlot) -> Result<Option<&RuntimeComponent>> {
        self.actors.component(id, slot).map_err(error)
    }
    fn organization(
        &self,
        id: u32,
    ) -> Result<&aa_actors::ground_states::OrganizationState<String>> {
        match self.component(ActorId::from_raw(id), ComponentSlot::Organization)? {
            Some(RuntimeComponent::Organization(value)) => Ok(value),
            _ => Err(error("live Organization context is missing")),
        }
    }
    /// The component stores a resolved native identity. Reading it must not
    /// apply the OrganizationLayout translation table a second time.
    fn layout(&self, organization: u32) -> Result<(std::sync::Arc<aa_rules::native::layout_data::NativeLayoutData>, usize)> {
        let org = self.organization(organization)?;
        let data = self.catalog.layout_data(self.precision).map_err(error)?;
        let index = data.layouts.iter()
            .position(|row| row.ids.eq_ignore_ascii_case(&org.layout.definition))
            .ok_or_else(|| error("live Economy canonical OrganizationLayout missing"))?;
        if data.layouts[index].error.is_some() {
            return Err(error("live Economy OrganizationLayout load failed"));
        }
        Ok((data, index))
    }
}
impl<S: Spatial> GateWorld for View<'_, '_, S> {
    type Definition = String;
    fn actor(&self, id: ActorId) -> Result<ActorSnapshot<String>> {
        let key = self
            .actors
            .key(id)
            .ok_or_else(|| error("economy actor GTID is stale"))?;
        let object = self.actors.object(key).map_err(error)?;
        let components = object.components().map_err(error)?;
        let element_organization = match components.get(ComponentSlot::Element) {
            Some(RuntimeComponent::Element(value)) => value.organization,
            None => None,
            _ => return Err(error("wrong component in Element slot")),
        };
        // ConstructionComponent is not ConstructorComponent. Native4D0C90
        // reads the latter supply component, not Construction's vector metadata.
        // Legacy complete-snapshot field is not used by this adapter. The
        //supplied override below is the authority for that conditional read.
        let constructor18 = None;
        let state = components.state;
        Ok(ActorSnapshot {
            definition: object.definition.proof.id.clone(),
            flags: self
                .actors
                .placement(key, self.spatial)
                .map_err(error)?
                .flags,
            state_bits: u64::from(state.low) | (u64::from(state.high) << 32),
            element_organization,
            organization: components
                .present(ComponentSlot::Organization)
                .then_some(id.raw()),
            economy: components
                .present(ComponentSlot::Economy)
                .then_some(id.raw()),
            constructor18,
            transport: components
                .present(ComponentSlot::Transport)
                .then_some(id.raw()),
        })
    }
    fn definition(&self, id: &String) -> Result<Definition<String>> {
        self.catalog.require_untranslated_actor_identity(id).map_err(error)?;
        let binding = self
            .catalog
            .actor_bindings(id, self.precision)
            .map_err(error)?;
        let source = binding
            .source_economy
            .as_ref()
            .ok_or_else(|| error("resolved Economy source fields missing"))?;
        let resource = source
            .resources
            .as_ref()
            .ok_or_else(|| error("resource getter requires resolved Economy source fields"))?;
        let mask = binding
            .definition
            .component_mask
            .ok_or_else(|| error("completed ActorData component flags missing"))?;
        Ok(Definition {
            flags174: mask,
            class178: source.recruitable_word,
            // The actual+17C reverse recruitment list is queried lazily by
            // recruit_backref_present only on501730's reached Cost branch.
            // +184 transport_actor is a different native field.
            word17c: 0,
            layout_kind18: binding
                .source_ground
                .organization
                .as_ref()
                .map(|o| o.layout.primary_index),
            supply_required: resource.supply_required.boolean().map_err(error)?,
            // Legacy snapshot field is deliberately unused: the override below
            // is the sole authoritative inverse-reference getter for this view.
            packed_definition: None,
            cost: resource.cost_bits.iter().copied().map(F32).collect(),
            upkeep: resource.upkeep_bits.iter().copied().map(F32).collect(),
            production: resource.production_bits.iter().copied().map(F32).collect(),
        })
    }
    fn packed_definition(&self, id: &String) -> Result<Option<String>> {
        self.catalog.require_untranslated_actor_identity(id).map_err(error)?;
        let canonical = self
            .catalog
            .actor_bindings(id, self.precision)
            .map_err(error)?
            .definition
            .proof
            .id
            .to_ascii_lowercase();
        self.packed_definitions
            .get(&canonical)
            .cloned()
            .ok_or_else(|| {
                error(format!(
                    "native inverse unpack binding unresolved for {canonical}"
                ))
            })
    }
    fn supplied(&self, id: ActorId) -> Result<bool> {
        let key = self
            .actors
            .key(id)
            .ok_or_else(|| error("supply actor GTID is stale"))?;
        let components = self
            .actors
            .object(key)
            .map_err(error)?
            .components()
            .map_err(error)?;
        if components.state.low & (1 << 19) == 0 {
            return Ok(true);
        }
        if components.present(ComponentSlot::Supply) {
            return Err(error("live Supply field18 getter is not bound"));
        }
        Ok(false)
    }
}
impl<S: Spatial> World for View<'_, '_, S> {
    fn recruit_backref_present(&self, id: &String) -> Result<bool> {
        self.catalog.native_recruit_backrefs(self.precision).map_err(error)?
            .present(id).map_err(error)
    }
    fn resource_count(&self) -> Result<usize> {
        self.catalog
            .actors
            .native_resource_registry(self.precision)
            .map(|r| r.ids.len())
            .map_err(error)
    }
    fn actor_modifier(&self, id: ActorId) -> Result<Option<u32>> {
        Ok(self
            .component(id, ComponentSlot::Properties)?
            .map(|_| id.raw()))
    }
    fn modifier(&self, context: u32) -> Result<Modifier> {
        let Some(RuntimeComponent::Properties(value)) =
            self.component(ActorId::from_raw(context), ComponentSlot::Properties)?
        else {
            return Err(error("live Properties context is missing"));
        };
        Ok(Modifier {
            flags: value.modifier.flags,
            factor4c: value.modifier.scalar(0x4c)?,
            factor60: value.modifier.scalar(0x60)?,
            production_adjustments: Vec::new(),
        })
    }
    fn production_delta(&self, context: u32, index: usize) -> Result<aa_numeric::Binary> {
        let Some(RuntimeComponent::Properties(value)) =
            self.component(ActorId::from_raw(context), ComponentSlot::Properties)?
        else {
            return Err(error("live Properties context is missing"));
        };
        // ModifierState +80 production table (typed since combat round 6);
        // 5019D0 returns zero when its size differs from the registry.
        aa_body::economy_modifiers::production_delta(
            &value.modifier.production,
            self.resource_count()?,
            index,
        )
        .map_err(Error::Body)
    }
    fn transport_vector(&self, _: u32, _: VectorKind) -> Result<Vec<F32>> {
        Err(error("live Transport resource cache is not bound"))
    }
    fn layout_slot_count(&self, organization: u32) -> Result<usize> {
        let (data, index) = self.layout(organization)?;
        Ok(data.layouts[index].slot_groups.len())
    }
    fn layout_slot_definition(&self, organization: u32, index: usize) -> Result<Option<String>> {
        let (data, layout) = self.layout(organization)?;
        let group = *data.layouts[layout]
            .slot_groups
            .get(index)
            .ok_or_else(|| error("layout slot outside native expanded groups"))?;
        self.organization(organization)?.layout
            .parameters
            .get(group as usize)
            .cloned()
            .ok_or_else(|| error("layout group outside live parameter array"))
    }
    fn member_count(&self, organization: u32) -> Result<usize> {
        Ok(self.organization(organization)?.members.len())
    }
    fn member(&self, organization: u32, index: usize) -> Result<Option<ActorId>> {
        let id = *self
            .organization(organization)?
            .members
            .get(index)
            .ok_or_else(|| error("native Organization member index"))?;
        Ok((id != ActorId::NULL).then_some(id))
    }
    fn store_member_production(&mut self, economy: u32, values: &[F32]) -> Result<()> {
        let Some(RuntimeComponent::Economy(component)) = self
            .actors
            .component_mut(ActorId::from_raw(economy), ComponentSlot::Economy)
            .map_err(error)?
        else {
            return Err(error("live Economy member cache is missing"));
        };
        let out = &mut component.resources[3].values;
        out.try_reserve(values.len().saturating_sub(out.len()))
            .map_err(|_| Error::Allocation)?;
        out.clear();
        out.extend_from_slice(values);
        Ok(())
    }
}
impl<S: Spatial> View<'_, '_, S> {
    pub fn cost_or_upkeep(&self, actor: ActorId, kind: VectorKind) -> Result<Vec<F32>> {
        let mut out = Vec::new();
        values::cost_or_upkeep(&mut out, Some(actor), kind, self, self.precision)?;
        Ok(out)
    }
    pub fn production(&mut self, actor: ActorId, cache_members: bool) -> Result<Vec<F32>> {
        let mut out = Vec::new();
        values::production(&mut out, Some(actor), cache_members, self, self.precision)?;
        Ok(out)
    }
}
