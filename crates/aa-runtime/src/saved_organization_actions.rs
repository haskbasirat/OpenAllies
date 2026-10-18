//! Saved Element attachment (5705A0) over the actual in-progress owner and
//! restored member components. The 4D1350 coordinator and ground reads run
//! in native order; unbound contributors/publication/economy stay errors
//! after the proven prefix's writes. This is not a general live adapter.
use crate::catalog::Catalog;
use aa_actors::{
    ActorId, ability_flags as flags, action_publication, components::ComponentSlot,
    ground_states::OrganizationState, loading::ActorShell,
};
use aa_mapworld::restore::{
    OrganizationAttachContext, RestoredComponent, RestoredRecord, RuntimeDefinition,
};
use aa_numeric::Precision;

pub(crate) fn action_error(error: impl std::fmt::Display) -> flags::Error {
    flags::Error::World(error.to_string())
}
fn missing<T>(what: &'static str) -> flags::Result<T> {
    Err(flags::Error::Missing(what))
}
pub(crate) fn require_shared_ability_identity(
    catalog: &Catalog<'_>,
    value: &Option<aa_rules::native::TypedValue>,
) -> flags::Result<()> {
    let Some(value) = value else { return Ok(()) };
    let aa_rules::native::TypedValue::String { value, .. } = value else {
        return missing("shared AbilityDefinition IDS string");
    };
    let key = value.split('\0').next().unwrap_or_default();
    let found = catalog
        .actors
        .native_data_find("AbilityDefinition", key)
        .ok_or_else(|| action_error("shared AbilityDefinition is not registered"))?;
    // The completed-Ability producer currently resolves exact source IDS.
    // Refuse a translated/shadowed reference instead of using another
    // definition than native48D500 returns. This runs only at the reached
    // ability index: a matching earlier bombard ability short-circuits it.
    if key != value || !found.ids.eq_ignore_ascii_case(key) {
        return missing("translated shared AbilityDefinition source projection");
    }
    Ok(())
}

pub fn attach(
    catalog: &Catalog<'_>,
    context: OrganizationAttachContext<'_>,
    member: ActorId,
    slot: i32,
) -> Result<(), String> {
    if context.organization.header.owner != context.owner {
        return Err("active parent Organization owner identity mismatch".into());
    }
    let owner = context.owner;
    aa_combat::organization::attach_membership(
        member.raw(),
        owner.raw(),
        slot,
        &mut SavedActions {
            catalog,
            context: context.into(),
            active_mind: None,
        },
    )
    .map_err(|error| error.to_string())
}

pub(crate) struct SavedActions<'a, 'world, 'source> {
    pub(crate) catalog: &'a Catalog<'source>,
    pub(crate) context: SavedActionContext<'world>,
    // The same owned Mind is temporarily executing in aa_mind::Arena. Its
    // component remains logically present while its allocation is borrowed.
    pub(crate) active_mind: Option<ActorId>,
}

/// The same held Actor is usable with an Organization in its real slot, with
/// an actual Organization temporarily borrowed by Load, or with no Organization.
/// Presence is never inferred merely from the fact that this Actor is held.
pub(crate) struct SavedActionContext<'a> {
    pub(crate) inventory: &'a aa_mapworld::WorldInventory,
    pub(crate) owner: ActorId,
    pub(crate) owner_actor: &'a mut ActorShell<RestoredComponent>,
    pub(crate) owner_definition: &'a RuntimeDefinition,
    pub(crate) owner_thing_flags: u32,
    pub(crate) owner_thing_word_c: u32,
    pub(crate) owner_object_byte_4c: u8,
    pub(crate) organization: Option<&'a mut OrganizationState<String>>,
    pub(crate) registry: &'a mut aa_actors::IdentityRegistry<RestoredRecord>,
    pub(crate) precision: Precision,
    /// Definition-time reads use the actual Bank PC on the held live path.
    /// Existing standalone attachment diagnostics retain their explicit source PC.
    pub(crate) source_precision: Precision,
}

impl<'a> From<OrganizationAttachContext<'a>> for SavedActionContext<'a> {
    fn from(context: OrganizationAttachContext<'a>) -> Self {
        Self {
            inventory: context.inventory,
            owner: context.owner,
            owner_actor: context.owner_actor,
            owner_definition: context.owner_definition,
            owner_thing_flags: context.owner_thing_flags,
            owner_thing_word_c: context.owner_thing_word_c,
            owner_object_byte_4c: context.owner_object_byte_4c,
            organization: Some(context.organization),
            registry: context.registry,
            precision: context.precision,
            source_precision: context.precision,
        }
    }
}
impl SavedActions<'_, '_, '_> {
    /// A live source owner wins over the standalone diagnostic input. This is
    /// lazy: no source Bank is read on a native branch that needs only fields.
    pub(crate) fn source_precision(&self) -> flags::Result<Precision> {
        let bank = self
            .catalog
            .definition_bank
            .try_borrow()
            .map_err(action_error)?;
        if let Some(source) = bank.combat_source.as_deref() {
            if source.source_identities() != self.catalog.actors.sources.as_slice() {
                return missing("saved actions source Bank/catalog identities differ");
            }
            Ok(source.source_precision())
        } else {
            // Existing standalone attachment proofs pass their source PC
            // explicitly. This branch does not confer startup completion.
            Ok(self.context.source_precision)
        }
    }
    fn group_layout(
        &self,
        actor: ActorId,
    ) -> flags::Result<(
        std::sync::Arc<aa_rules::native::layout_data::NativeLayoutData>,
        usize,
    )> {
        let org = self.org(actor)?;
        if org.header.owner != actor {
            return missing("578A30 Organization owner identity");
        }
        let data = self
            .catalog
            .layout_data(self.source_precision()?)
            .map_err(action_error)?;
        // This is an existing native object identity, not a second IDS lookup.
        let index = data
            .layouts
            .iter()
            .position(|layout| layout.ids.eq_ignore_ascii_case(&org.layout.definition))
            .ok_or_else(|| action_error("578A30 canonical OrganizationLayout is not registered"))?;
        if data.layouts[index].error.is_some() {
            return missing("578A30 completed OrganizationLayout");
        }
        Ok((data, index))
    }
    fn group_actor_binding(
        &self,
        actor: ActorId,
        index: usize,
    ) -> flags::Result<std::sync::Arc<crate::catalog::ActorBindings>> {
        let id = self
            .org(actor)?
            .layout
            .parameters
            .get(index)
            .ok_or_else(|| action_error("578A30 parameter index outside current Organization"))?
            .as_deref()
            .ok_or_else(|| action_error("578A30 required parameter ActorData is null"))?;
        self.catalog
            .require_untranslated_actor_identity(id)
            .map_err(action_error)?;
        self.catalog
            .actor_bindings(id, self.source_precision()?)
            .map_err(action_error)
    }
    pub(crate) fn source_id(&self, actor: ActorId) -> flags::Result<&str> {
        if actor == self.context.owner {
            return Ok(&self.context.owner_definition.proof.id);
        }
        self.context
            .registry
            .get(actor)
            .and_then(|entry| entry.value.loaded())
            .map(|entity| entity.definition.proof.id.as_str())
            .ok_or_else(|| action_error("restored ActorData identity missing"))
    }
    pub(crate) fn shell(&self, actor: ActorId) -> flags::Result<&ActorShell<RestoredComponent>> {
        if actor == self.context.owner {
            return Ok(self.context.owner_actor);
        }
        self.context
            .registry
            .get(actor)
            .and_then(|entry| entry.value.loaded())
            .and_then(|entity| entity.shell.actor.as_ref())
            .ok_or_else(|| action_error(format!("{:#x} is not a restored Actor", actor.raw())))
    }
    pub(crate) fn shell_mut(
        &mut self,
        actor: ActorId,
    ) -> flags::Result<&mut ActorShell<RestoredComponent>> {
        if actor == self.context.owner {
            return Ok(self.context.owner_actor);
        }
        self.context
            .registry
            .value_mut(actor)
            .and_then(RestoredRecord::loaded_mut)
            .and_then(|entity| entity.shell.actor.as_mut())
            .ok_or_else(|| action_error(format!("{:#x} is not a restored Actor", actor.raw())))
    }
    pub(crate) fn component(
        &self,
        actor: ActorId,
        slot: ComponentSlot,
    ) -> flags::Result<Option<&RestoredComponent>> {
        if slot == ComponentSlot::Mind && self.active_mind == Some(actor) {
            return missing("Mind is executing in its owned action arena");
        }
        Ok(self.shell(actor)?.components.get(slot))
    }
    pub(crate) fn org(&self, actor: ActorId) -> flags::Result<&OrganizationState<String>> {
        if actor == self.context.owner
            && let Some(organization) = self.context.organization.as_deref()
        {
            return Ok(organization);
        }
        match self.component(actor, ComponentSlot::Organization)? {
            Some(RestoredComponent::Organization(org)) => Ok(org),
            _ => missing("restored Organization component"),
        }
    }
    pub(crate) fn word(&self, actor: ActorId, offset: u32) -> flags::Result<u32> {
        self.shell(actor)?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| action_error(format!("Actor+{offset:X} is uninitialized")))
    }
    pub(crate) fn org_word(&self, actor: ActorId, offset: u32) -> flags::Result<u32> {
        self.org(actor)?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| action_error(format!("Organization+{offset:X} is uninitialized")))
    }
    fn thing_flags(&self, actor: ActorId) -> flags::Result<u32> {
        if actor == self.context.owner {
            return Ok(self.context.owner_thing_flags);
        }
        self.context
            .registry
            .get(actor)
            .and_then(|entry| entry.value.loaded())
            .map(|entity| entity.shell.thing.flags)
            .ok_or_else(|| action_error("Thing flags unavailable"))
    }
}

impl aa_combat::organization::AttachWorld for SavedActions<'_, '_, '_> {
    fn set_membership(&mut self, member: u32, owner: u32, slot: i32) -> aa_combat::Result<()> {
        if owner != self.context.owner.raw() {
            return Err(aa_combat::Error::World("parent identity changed".into()));
        }
        let actor = ActorId::from_raw(member);
        let shell = self
            .shell_mut(actor)
            .map_err(|error| aa_combat::Error::World(error.to_string()))?;
        let Some(RestoredComponent::Element(element)) =
            shell.components.get_mut(ComponentSlot::Element)
        else {
            return Err(aa_combat::Error::World(
                "5705A0 member has no Element payload".into(),
            ));
        };
        if element.header.owner != actor {
            return Err(aa_combat::Error::World(
                "Element owner identity mismatch".into(),
            ));
        }
        // Preserve the native store order before any downstream callback.
        element.slot_index = slot;
        element.organization = (owner != 0).then_some(ActorId::from_raw(owner));
        element.flag_1c = true;
        Ok(())
    }
    fn notify_member(&mut self, member: u32) -> aa_combat::Result<()> {
        flags::rebuild(self, ActorId::from_raw(member))
            .map_err(|error| aa_combat::Error::World(error.to_string()))
    }
    fn refresh_member(&mut self, member: u32) -> aa_combat::Result<()> {
        crate::saved_economy_refresh::refresh_member(self, ActorId::from_raw(member))
    }
    fn observer_present(&self) -> aa_combat::Result<bool> {
        self.catalog
            .saved_observer_present()
            .map_err(aa_combat::Error::World)
    }
    fn notify_observer(&mut self, actor: u32) -> aa_combat::Result<()> {
        self.catalog
            .notify_saved_observer(actor)
            .map_err(aa_combat::Error::World)
    }
}

impl flags::World for SavedActions<'_, '_, '_> {
    fn actor_fields(&self, actor: ActorId) -> flags::Result<flags::ActorFields> {
        let state = self.shell(actor)?.components.state;
        Ok(flags::ActorFields {
            thing_flags: self.thing_flags(actor)?,
            state_bits: u64::from(state.low) | (u64::from(state.high) << 32),
            action_bits: u64::from(self.word(actor, 0xf0)?)
                | (u64::from(self.word(actor, 0xf4)?) << 32),
        })
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> flags::Result<()> {
        let shell = self.shell_mut(actor)?;
        if !shell.words.contains_key(&0xf0) || !shell.words.contains_key(&0xf4) {
            return missing("initialized Actor F0/F4 action fields");
        }
        shell.words.insert(0xf0, bits as u32);
        shell.words.insert(0xf4, (bits >> 32) as u32);
        Ok(())
    }
    fn has_component(&self, actor: ActorId, slot: ComponentSlot) -> flags::Result<bool> {
        if slot == ComponentSlot::Mind && self.active_mind == Some(actor) {
            return Ok(true);
        }
        if actor == self.context.owner
            && slot == ComponentSlot::Organization
            && self.context.organization.is_some()
        {
            return Ok(true);
        }
        Ok(self.component(actor, slot)?.is_some())
    }
    fn element(&self, actor: ActorId) -> flags::Result<flags::ElementView> {
        match self.component(actor, ComponentSlot::Element)? {
            Some(RestoredComponent::Element(element)) => Ok(flags::ElementView {
                owner: element.header.owner,
                definition_present: element.element_definition.is_some(),
                organization: element.organization,
                ready: element.flag_1c,
            }),
            _ => missing("restored Element component"),
        }
    }
    fn organization_member_count(&self, actor: ActorId) -> flags::Result<usize> {
        Ok(self.org(actor)?.members.len())
    }
    fn organization_member(&self, actor: ActorId, index: usize) -> flags::Result<Option<ActorId>> {
        let member = *self
            .org(actor)?
            .members
            .get(index)
            .ok_or_else(|| action_error("member index outside Organization slots"))?;
        Ok((!member.is_null()).then_some(member))
    }
    fn organization_member_44(&self, actor: ActorId) -> flags::Result<Option<ActorId>> {
        let word = self.org_word(actor, 0x44)?;
        Ok((word != 0).then_some(ActorId::from_raw(word)))
    }
    fn organization_reference_d0(&self, actor: ActorId) -> flags::Result<ActorId> {
        Ok(ActorId::from_raw(self.org_word(actor, 0xd0)?))
    }
    fn valid_reference(&self, actor: ActorId) -> flags::Result<bool> {
        Ok(actor == self.context.owner || self.context.registry.get(actor).is_some())
    }
    fn organization_group_count(&self, actor: ActorId) -> flags::Result<usize> {
        let (data, index) = self.group_layout(actor)?;
        Ok(data.layouts[index].entries.len())
    }
    fn organization_attack_group(
        &self,
        actor: ActorId,
        index: usize,
    ) -> flags::Result<flags::AttackGroup> {
        Ok(flags::AttackGroup {
            locked: self.organization_group_locked(actor, index)?,
            class_flags: (0..self.organization_group_ability_count(actor, index)?)
                .map(|ability| self.organization_group_ability_flags(actor, index, ability))
                .collect::<flags::Result<_>>()?,
        })
    }
    fn organization_group_locked(&self, actor: ActorId, index: usize) -> flags::Result<bool> {
        let (data, layout) = self.group_layout(actor)?;
        data.layouts[layout]
            .entries
            .get(index)
            .map(|entry| entry.locked)
            .ok_or_else(|| action_error("578A30 group index outside native layout"))
    }
    fn organization_group_ability_count(
        &self,
        actor: ActorId,
        index: usize,
    ) -> flags::Result<usize> {
        Ok(self
            .group_actor_binding(actor, index)?
            .performer
            .as_ref()
            .map_or(0, |p| p.abilities().len()))
    }
    fn organization_group_ability_flags(
        &self,
        actor: ActorId,
        index: usize,
        ability: usize,
    ) -> flags::Result<u32> {
        let binding = self.group_actor_binding(actor, index)?;
        let loaded = binding
            .performer
            .as_ref()
            .and_then(|p| p.abilities().get(ability))
            .ok_or_else(|| action_error("578A30 local AbilityData index outside parameter list"))?;
        require_shared_ability_identity(self.catalog, &loaded.authored.shared_definition_id)?;
        Ok(loaded.flags)
    }
    fn owner(&mut self, actor: ActorId) -> flags::Result<Option<ActorId>> {
        let word = self.word(actor, 0xe8)?;
        Ok((word != 0).then_some(ActorId::from_raw(word)))
    }
    fn kingdom_word_218(&self, actor: ActorId) -> flags::Result<u32> {
        let Some(RestoredRecord::Kingdom(record)) =
            self.context.registry.get(actor).map(|entry| &entry.value)
        else {
            return missing("restored owner Kingdom for +218");
        };
        let count = record
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.derived.as_ref())
            .map(|shell| shell.recruit_items.singles.len())
            .ok_or_else(|| action_error("Kingdom recruit item table is uninitialized"))?;
        // 218 is the owned singles vector count (214 is its pointer), not
        // an independent scalar word. Never fabricate a second count.
        u32::try_from(count).map_err(action_error)
    }
    fn state_resupply(&self, index: u8) -> flags::Result<bool> {
        let state = aa_actors::state::DESCRIPTORS
            .get(index as usize)
            .ok_or_else(|| action_error("ActorState index outside native descriptors"))?;
        self.catalog
            .actors
            .native_actor_state(state.name, self.source_precision()?)
            .map_err(action_error)?
            .mind
            .ok_or_else(|| action_error("ActorState has no Mind fields"))?
            .resupply
            .boolean()
            .map_err(action_error)
    }
    fn owner_can_recruit(&mut self, owner: ActorId, actor: ActorId) -> flags::Result<bool> {
        crate::saved_owner_recruitment::can_recruit(self, owner, actor)
    }
    fn owner_recruit_count(&mut self, owner: ActorId, actor: ActorId) -> flags::Result<i32> {
        crate::saved_owner_recruitment::recruit_count(self, owner, actor)
    }
    fn action_available(&mut self, actor: ActorId, index: u32) -> flags::Result<bool> {
        flags::available(self.actor_fields(actor)?.action_bits, index)
    }
    fn other_contributor(&mut self, actor: ActorId, slot: ComponentSlot) -> flags::Result<()> {
        if slot == ComponentSlot::Recruit {
            return crate::saved_recruit_actions::contribute(self, actor);
        }
        if slot == ComponentSlot::Research {
            return crate::saved_research_actions::contribute(self, actor);
        }
        if slot == ComponentSlot::Structure {
            return crate::saved_structure_actions::contribute(self, actor);
        }
        if slot == ComponentSlot::Denizen {
            return crate::saved_denizen_actions::contribute(self, actor);
        }
        if slot == ComponentSlot::Construction {
            return crate::saved_construction_actions::contribute(self, actor);
        }
        if slot == ComponentSlot::MobileDeployment {
            let precision = self.context.precision;
            return aa_actors::mobile_deployment_actions::contribute(self, actor, precision);
        }
        let native = match slot {
            ComponentSlot::Recruit => "582C40",
            ComponentSlot::Research => "583FC0",
            ComponentSlot::Settlement => "585AE0",
            ComponentSlot::Structure => "589A60",
            ComponentSlot::Denizen => "569C70",
            ComponentSlot::Construction => "565140",
            ComponentSlot::MobileDeployment => "575FB0",
            ComponentSlot::Flare => "virtual+38",
            _ => "unexpected contributor",
        };
        Err(action_error(format!(
            "saved {slot:?} {native} action contributor for actor {:#x} is unbound",
            actor.raw()
        )))
    }
    fn mind_contributor(&mut self, actor: ActorId) -> flags::Result<()> {
        crate::saved_mind_actions::contribute(self, actor).map_err(action_error)
    }
    fn changed(&mut self, actor: ActorId) -> flags::Result<()> {
        action_publication::changed(actor, self)
    }
}

impl action_publication::World for SavedActions<'_, '_, '_> {
    fn element_organization(&self, actor: ActorId) -> flags::Result<Option<ActorId>> {
        match self.component(actor, ComponentSlot::Element)? {
            Some(RestoredComponent::Element(element)) => Ok(element.organization),
            None => Ok(None),
            _ => missing("Element slot payload mismatch"),
        }
    }
    fn organization_owner(&self, actor: ActorId) -> flags::Result<ActorId> {
        Ok(self.org(actor)?.header.owner)
    }
    fn rebuild_actions(&mut self, actor: ActorId) -> flags::Result<()> {
        flags::rebuild(self, actor)
    }
    fn owner(&mut self, actor: ActorId) -> flags::Result<Option<ActorId>> {
        flags::World::owner(self, actor)
    }
    fn object_byte_4c(&self, actor: ActorId) -> flags::Result<u8> {
        if actor == self.context.owner {
            return Ok(self.context.owner_object_byte_4c);
        }
        self.context
            .registry
            .get(actor)
            .and_then(|entry| entry.value.loaded())
            .and_then(|entity| entity.shell.object.as_ref())
            .map(|object| object.byte_4c)
            .ok_or_else(|| action_error("saved Object+4C unavailable"))
    }
    fn refresh_owner_views(&mut self, _: Option<ActorId>) -> flags::Result<()> {
        missing("76C7F0 saved owner-view publication")
    }
    fn renderer_flags(&self, actor: ActorId) -> flags::Result<Option<u32>> {
        let link = if actor == self.context.owner {
            self.context.owner_thing_word_c
        } else {
            self.context
                .registry
                .get(actor)
                .and_then(|entry| entry.value.loaded())
                .map(|entity| entity.shell.thing.word_c)
                .ok_or_else(|| action_error("saved Thing+C unavailable"))?
        };
        if link == 0 {
            Ok(None)
        } else {
            missing("saved renderer+54 flags")
        }
    }
    fn refresh_selected_view(&mut self, _: ActorId) -> flags::Result<()> {
        missing("50C8E0 saved selected-view publication")
    }
}

impl SavedActions<'_, '_, '_> {
    fn mobile_source(
        &self,
        actor: ActorId,
    ) -> flags::Result<(
        std::sync::Arc<aa_rules::native_mobile_deployment::NativeMobileDeploymentData>,
        String,
    )> {
        match self.component(actor, ComponentSlot::MobileDeployment)? {
            Some(RestoredComponent::MobileDeployment(value)) if value.header.owner == actor => (),
            _ => return missing("saved MobileDeployment owner/payload"),
        }
        let id = self.source_id(actor)?.to_owned();
        let data = self
            .catalog
            .native_mobile_deployment_data(self.source_precision()?)
            .map_err(action_error)?;
        if !data.definition(&id).map_err(action_error)?.mobile {
            return missing("saved MobileDeployment ActorData component");
        }
        Ok((data, id))
    }
}
impl aa_actors::mobile_deployment_actions::World for SavedActions<'_, '_, '_> {
    fn fields(&self, actor: ActorId) -> flags::Result<flags::ActorFields> {
        flags::World::actor_fields(self, actor)
    }
    fn set_action_bits(&mut self, actor: ActorId, bits: u64) -> flags::Result<()> {
        flags::World::set_action_bits(self, actor, bits)
    }
    fn has_pack_reference(&self, actor: ActorId) -> flags::Result<bool> {
        let (data, id) = self.mobile_source(actor)?;
        Ok(data
            .definition(&id)
            .map_err(action_error)?
            .pack_to
            .is_some())
    }
    fn has_unpack_reference(&self, actor: ActorId) -> flags::Result<bool> {
        let (data, id) = self.mobile_source(actor)?;
        Ok(data
            .definition(&id)
            .map_err(action_error)?
            .unpack_to
            .is_some())
    }
    fn pack_health_min_bits(&self, actor: ActorId) -> flags::Result<u32> {
        let (data, id) = self.mobile_source(actor)?;
        Ok(data
            .definition(&id)
            .map_err(action_error)?
            .pack_health_min_bits)
    }
    fn properties_flags(&self, actor: ActorId) -> flags::Result<Option<u32>> {
        match self.component(actor, ComponentSlot::Properties)? {
            None => Ok(None),
            Some(RestoredComponent::Properties(value)) if value.owner.0 == actor.raw() => {
                Ok(Some(value.modifier.flags))
            }
            _ => missing("saved MobileDeployment Properties owner/payload"),
        }
    }
    fn body_health(
        &self,
        actor: ActorId,
    ) -> flags::Result<Option<aa_actors::mobile_deployment_actions::BodyHealth>> {
        match self.component(actor, ComponentSlot::Body)? {
            None => Ok(None),
            Some(RestoredComponent::Body(value)) if value.header.owner == actor => {
                Ok(Some(aa_actors::mobile_deployment_actions::BodyHealth {
                    health_bits: value.body.health.0,
                    maximum_bits: value.body.maximum.0,
                }))
            }
            _ => missing("saved MobileDeployment Body owner/payload"),
        }
    }
    fn has_mind(&self, actor: ActorId) -> flags::Result<bool> {
        flags::World::has_component(self, actor, ComponentSlot::Mind)
    }
}

#[cfg(test)]
#[path = "saved_group_actions_tests.rs"]
mod group_tests;
