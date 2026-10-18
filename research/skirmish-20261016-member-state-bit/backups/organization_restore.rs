//! Organization (slot 7) saved Load 57D130 at its actor stage, bound to the
//! restore world. The Load embeds complete member records (4FC200 ->
//! 4FAE80), so it runs on the constructed component instead of a pre-decoded
//! payload; members register and load re-entrantly while the owner is out of
//! the registry. Every native callback without a ported owner returns an
//! explicit error naming its address (never a silent success).
use crate::restore::{
    LoadContext, NativeMessage, OrganizationDefinitions, RestoreDefinitions, RestoredComponent,
    RestoredRecord, RuntimeDefinition, load_embedded_record,
};
use aa_actors::{
    ActorId, IdentityRegistry,
    components::ComponentSlot,
    loading::ActorShell,
    organization_callees,
    organization_state::{
        self, Definitions, Diagnostic, LayoutInfo, LoadError, MemberFacts, OrganizationWorld,
        OwnerFacts, StateWords, WorldFacts,
    },
};
use aa_numeric::Binary;

type Result<T> = std::result::Result<T, LoadError>;

fn unported<T>(what: &'static str) -> Result<T> {
    Err(LoadError::World(format!("{what} is unported")))
}

struct RestoreOrganizationWorld<'a, 'b, R: RestoreDefinitions> {
    definitions: &'a R,
    registry: &'a mut IdentityRegistry<RestoredRecord>,
    cx: &'a mut LoadContext<'b>,
    owner: ActorId,
    owner_actor: &'a mut ActorShell<RestoredComponent>,
    owner_definition: &'a RuntimeDefinition,
    owner_thing_flags: u32,
    owner_thing_word_c: u32,
    owner_object_byte_4c: u8,
}

impl<R: RestoreDefinitions> RestoreOrganizationWorld<'_, '_, R> {
    fn defs(&mut self) -> OrganizationDefinitions<'_, R> {
        OrganizationDefinitions {
            definitions: self.definitions,
            precision: self.cx.pc,
        }
    }
    fn world(e: String) -> LoadError {
        LoadError::World(e)
    }
}

impl<R: RestoreDefinitions> RestoreOrganizationWorld<'_, '_, R> {
    /// A component of a restored actor; the owner is out of the registry
    /// while its Organization loads, so it is read from its own shell.
    fn component(
        &self,
        actor: ActorId,
        slot: ComponentSlot,
    ) -> std::result::Result<Option<&RestoredComponent>, organization_callees::Error> {
        restored_component(self.owner, self.owner_actor, self.registry, actor, slot)
    }
}

/// The loading owner's shell is temporarily outside the identity registry;
/// member IDs still use full generation-checked GTIDs in that registry.
fn restored_component<'a>(
    owner: ActorId,
    owner_actor: &'a ActorShell<RestoredComponent>,
    registry: &'a IdentityRegistry<RestoredRecord>,
    actor: ActorId,
    slot: ComponentSlot,
) -> std::result::Result<Option<&'a RestoredComponent>, organization_callees::Error> {
    let shell = if actor == owner {
        owner_actor
    } else {
        registry
            .get(actor)
            .and_then(|entry| entry.value.loaded())
            .and_then(|entity| entity.shell.actor.as_ref())
            .ok_or_else(|| {
                organization_callees::Error::World(format!(
                    "actor {:#010x} is not a restored Actor",
                    actor.raw()
                ))
            })?
    };
    Ok(shell.components.get(slot))
}

fn restored_properties<'a>(
    owner: ActorId,
    owner_actor: &'a ActorShell<RestoredComponent>,
    registry: &'a IdentityRegistry<RestoredRecord>,
    actor: ActorId,
) -> Result<Option<&'a aa_body::properties::PropertiesState>> {
    match restored_component(
        owner,
        owner_actor,
        registry,
        actor,
        ComponentSlot::Properties,
    )
    .map_err(callee)?
    {
        Some(RestoredComponent::Properties(state)) if state.owner.0 == actor.raw() => {
            Ok(Some(state))
        }
        Some(RestoredComponent::Properties(_)) => Err(LoadError::World(
            "Properties context belongs to a different Actor".into(),
        )),
        Some(_) => Err(LoadError::World("Properties slot payload mismatch".into())),
        None => Ok(None),
    }
}

/// 501C90: +20 additive morale adjustment. The shared primitive performs
/// the native explicit binary32 store, floor-to-min(base, setting), then
/// floor-to-zero. Return that value on the numeric stack without a new
/// rounding step; 578300 owns the later sum's binary32 stores.
fn apply_morale_modifier(
    properties: &aa_body::properties::PropertiesState,
    value_bits: u32,
    minimum_bits: u32,
    pc: aa_numeric::Precision,
) -> Result<Binary> {
    let adjusted = properties
        .modifier
        .adjust_with_floor(
            0x20,
            aa_body::F32(value_bits),
            aa_body::F32(minimum_bits),
            pc,
        )
        .map_err(|error| LoadError::World(error.to_string()))?;
    Binary::from_f32_bits(adjusted.0).map_err(|error| LoadError::World(error.to_string()))
}
impl<R: RestoreDefinitions> RestoreOrganizationWorld<'_, '_, R> {
    /// Kingdom +1FC recruit item table of a restored Kingdom (raw ID).
    fn recruit_table(
        &mut self,
        kingdom: u32,
    ) -> Result<&mut aa_economy::kingdom_registry::RecruitItemTable> {
        let Some(RestoredRecord::Kingdom(record)) =
            self.registry.value_mut(ActorId::from_raw(kingdom))
        else {
            return Err(LoadError::World(format!(
                "owner {kingdom:#010x} is not a restored Kingdom"
            )));
        };
        record
            .runtime
            .as_mut()
            .and_then(|r| r.derived.as_mut())
            .map(|shell| &mut shell.recruit_items)
            .ok_or_else(|| LoadError::World("owner Kingdom has no derived state".into()))
    }
}
/// `4E1200` callbacks over the restore shells.
struct Pairing<'a> {
    registry: &'a IdentityRegistry<RestoredRecord>,
}
impl aa_economy::kingdom_registry::RecruitPairing for Pairing<'_> {
    fn member_gtid(&mut self, member: u32) -> std::result::Result<u32, aa_economy::EconomyError> {
        Ok(member)
    }
    fn member_element(
        &mut self,
        member: u32,
    ) -> std::result::Result<Option<u32>, aa_economy::EconomyError> {
        let present = self
            .registry
            .get(ActorId::from_raw(member))
            .and_then(|e| e.value.loaded())
            .and_then(|e| e.shell.actor.as_ref())
            .is_some_and(|a| a.components.present(ComponentSlot::Element));
        Ok(present.then_some(member))
    }
    fn link_element(
        &mut self,
        _element: u32,
        _item: usize,
    ) -> std::result::Result<(), aa_economy::EconomyError> {
        Err(aa_economy::EconomyError::World(
            "Element+14 recruit-single link has no ElementState field".into(),
        ))
    }
}
impl<R: RestoreDefinitions> organization_callees::ActorFacts
    for RestoreOrganizationWorld<'_, '_, R>
{
    fn experience_rank(
        &self,
        actor: ActorId,
    ) -> std::result::Result<Option<u32>, organization_callees::Error> {
        Ok(match self.component(actor, ComponentSlot::Experience)? {
            Some(RestoredComponent::Experience(e)) => Some(e.current_rank.map_or(0, |r| r.0)),
            Some(_) => {
                return Err(organization_callees::Error::World(
                    "Experience slot payload mismatch".into(),
                ));
            }
            None => None,
        })
    }
    fn element_organization(
        &self,
        actor: ActorId,
    ) -> std::result::Result<Option<ActorId>, organization_callees::Error> {
        Ok(match self.component(actor, ComponentSlot::Element)? {
            Some(RestoredComponent::Element(e)) => e.organization,
            Some(_) => {
                return Err(organization_callees::Error::World(
                    "Element slot payload mismatch".into(),
                ));
            }
            None => None,
        })
    }
    fn has_performer(
        &self,
        actor: ActorId,
    ) -> std::result::Result<bool, organization_callees::Error> {
        Ok(self.component(actor, ComponentSlot::Performer)?.is_some())
    }
    fn organization_members(
        &self,
        actor: ActorId,
    ) -> std::result::Result<Option<Vec<ActorId>>, organization_callees::Error> {
        if actor == self.owner {
            return Err(organization_callees::Error::World(
                "the owner's Organization is mid-Load (57A330 on the loading owner)".into(),
            ));
        }
        Ok(match self.component(actor, ComponentSlot::Organization)? {
            Some(RestoredComponent::Organization(o)) => {
                Some(o.members.iter().copied().filter(|m| !m.is_null()).collect())
            }
            Some(_) => {
                return Err(organization_callees::Error::World(
                    "Organization slot payload mismatch".into(),
                ));
            }
            None => None,
        })
    }
}
impl<R: RestoreDefinitions> organization_callees::RankWorld
    for RestoreOrganizationWorld<'_, '_, R>
{
    fn performer_rank_changed(
        &mut self,
        _actor: ActorId,
        _rank: u32,
    ) -> std::result::Result<(), organization_callees::Error> {
        Err(organization_callees::Error::World(
            "57DCF0 Performer rank gating is unported".into(),
        ))
    }
}
impl<R: RestoreDefinitions> organization_callees::BodyUpdatable
    for RestoreOrganizationWorld<'_, '_, R>
{
    fn body_has_scheduled(
        &mut self,
        member: ActorId,
        _event: u8,
        _data: u32,
    ) -> std::result::Result<bool, organization_callees::Error> {
        // During map Load the Body's 50BCD0 has cancelled its scheduled
        // entries and the saved queue is bound only after all objects load.
        match self.component(member, ComponentSlot::Body)? {
            Some(RestoredComponent::Body(body)) if body.header.scheduled_head.is_none() => {
                Ok(false)
            }
            Some(RestoredComponent::Body(_)) => Err(organization_callees::Error::World(
                "member Body has a live scheduled entry during restore".into(),
            )),
            Some(_) => Err(organization_callees::Error::World(
                "Body slot payload mismatch".into(),
            )),
            None => Err(organization_callees::Error::World(
                "561760 runs on the member's Body, which is absent".into(),
            )),
        }
    }
    fn body_unschedule(
        &mut self,
        _member: ActorId,
        _event: u8,
        _data: u32,
    ) -> std::result::Result<(), organization_callees::Error> {
        Err(organization_callees::Error::World(
            "Body Updatable 50B1C0 during restore has no scheduled entry to remove".into(),
        ))
    }
}
fn callee(e: organization_callees::Error) -> LoadError {
    LoadError::World(e.to_string())
}
impl<R: RestoreDefinitions> Definitions<String> for RestoreOrganizationWorld<'_, '_, R> {
    fn layout_data(&mut self, ids: &[u16]) -> Result<Option<String>> {
        self.defs().layout_data(ids)
    }
    fn actor_data(&mut self, ids: &[u16]) -> Result<Option<String>> {
        self.defs().actor_data(ids)
    }
    fn layout_info(&self, layout: &String) -> Result<LayoutInfo> {
        self.definitions
            .organization_layout_info(layout, self.cx.pc)
            .map_err(Self::world)
    }
    fn name(&self, definition: &String) -> Result<String> {
        Ok(definition.clone())
    }
}

impl<R: RestoreDefinitions> OrganizationWorld<String> for RestoreOrganizationWorld<'_, '_, R> {
    fn updatable_table_word0(&self) -> Result<i32> {
        self.definitions
            .updatable_table_bound()
            .map_err(Self::world)
    }
    fn updatable_slot_occupied(&self, id: i32) -> Result<bool> {
        self.definitions
            .updatable_slot_occupied(id)
            .map_err(Self::world)
    }
    fn register_updatable(&mut self, id: i32) -> Result<()> {
        let target = crate::restore_components::UpdatableTarget::Component {
            owner: self.owner.raw(),
            slot: ComponentSlot::Organization.index() as u8,
        };
        self.definitions
            .register_updatable(id, &target)
            .map_err(Self::world)?;
        self.cx
            .updatable_registrations
            .push(crate::restore_components::UpdatableRegistration {
                serialized_id: id,
                target,
            });
        Ok(())
    }
    fn actor_modifier(&mut self, actor: ActorId) -> Result<Option<u32>> {
        // 4CF110 returns null only when Actor+78 lacks Properties. The
        // full Actor ID replaces the +1C pointer, validated again on use.
        Ok(
            restored_properties(self.owner, self.owner_actor, self.registry, actor)?
                .map(|_| actor.raw()),
        )
    }
    fn modifier_apply(&mut self, modifier: u32, value_bits: u32) -> Result<Binary> {
        let properties = restored_properties(
            self.owner,
            self.owner_actor,
            self.registry,
            ActorId::from_raw(modifier),
        )?
        .ok_or_else(|| {
            LoadError::World("Properties modifier context is no longer present".into())
        })?;
        let minimum = self
            .definitions
            .organization_modifier_minimum(self.cx.pc)
            .map_err(Self::world)?;
        apply_morale_modifier(properties, value_bits, minimum, self.cx.pc)
    }
    fn morale_contribution(&self, actor_data: &String) -> Result<u32> {
        self.definitions
            .organization_morale_contribution(actor_data, self.cx.pc)
            .map_err(Self::world)
    }
    fn has_property(&mut self, actor_data: &String, property: u32) -> Result<bool> {
        self.definitions
            .organization_has_property(actor_data, property, self.cx.pc)
            .map_err(Self::world)
    }
    fn log(&mut self, diagnostic: Diagnostic) -> Result<()> {
        self.cx.native_messages.push(NativeMessage {
            source_ordinal: self.cx.source_ordinal,
            function: diagnostic.function,
            line: diagnostic.line,
            text: diagnostic.message.to_string(),
        });
        Ok(())
    }
    fn load_member_record(&mut self, bytes: &[u8]) -> Result<(Option<ActorId>, usize)> {
        load_embedded_record(self.registry, bytes, self.owner, self.definitions, self.cx)
            .map_err(Self::world)
    }
    fn assert_latch(&self) -> Result<bool> {
        Ok(*self.cx.assert_latch)
    }
    fn is_actor(&mut self, thing: ActorId) -> Result<bool> {
        self.registry
            .get(thing)
            .map(|entry| entry.is_actor)
            .ok_or_else(|| LoadError::World("loaded member is not registered".into()))
    }
    fn assert_dialog(&mut self) -> Result<bool> {
        unported("855D30 `thing->IsGActor()` assertion dialog")
    }
    fn set_assert_latch(&mut self, value: bool) -> Result<()> {
        *self.cx.assert_latch = value;
        Ok(())
    }
    fn member_facts(&self, member: ActorId) -> Result<MemberFacts<String>> {
        let entity = self
            .registry
            .get(member)
            .and_then(|entry| entry.value.loaded())
            .ok_or_else(|| LoadError::World("member record is not a loaded entity".into()))?;
        let actor = entity
            .shell
            .actor
            .as_ref()
            .ok_or_else(|| LoadError::World("member is not an Actor".into()))?;
        let mover_speed = match actor.components.get(ComponentSlot::Mover) {
            Some(RestoredComponent::Mover(mover)) => Some(mover.speed.0),
            Some(_) => return Err(LoadError::World("Mover slot payload mismatch".into())),
            None => None,
        };
        Ok(MemberFacts {
            definition: entity.definition.proof.id.clone(),
            definition_flags: entity
                .definition
                .component_mask
                .ok_or_else(|| LoadError::World("member ActorData mask unresolved".into()))?,
            gtid: member.raw(),
            has_body: actor.components.present(ComponentSlot::Body),
            mover_speed,
            has_experience: actor.components.present(ComponentSlot::Experience),
        })
    }
    fn element_set_organization(
        &mut self,
        org: &mut aa_actors::ground_states::OrganizationState<String>,
        member: ActorId,
        slot: i32,
    ) -> Result<()> {
        self.definitions
            .organization_attach_member(
                crate::restore::OrganizationAttachContext {
                    inventory: self.cx.inventory,
                    owner: self.owner,
                    owner_actor: self.owner_actor,
                    owner_definition: self.owner_definition,
                    owner_thing_flags: self.owner_thing_flags,
                    owner_thing_word_c: self.owner_thing_word_c,
                    owner_object_byte_4c: self.owner_object_byte_4c,
                    organization: org,
                    registry: self.registry,
                    precision: self.cx.pc,
                },
                member,
                slot,
            )
            .map_err(Self::world)
    }
    fn body_set_organization(&mut self, member: ActorId) -> Result<()> {
        let owner = self.owner;
        organization_callees::body_set_organization(member, Some(owner), self).map_err(callee)
    }
    fn owner_experience_token(&mut self) -> Result<u32> {
        organization_callees::experience_source(self.owner, self).map_err(callee)
    }
    fn set_experience_source(&mut self, member: ActorId, token: u32) -> Result<()> {
        // 4CEBB0(member; 0, token).
        organization_callees::rank_changed(member, 0, token, self).map_err(callee)
    }
    fn combat_value(
        &mut self,
        org: &mut aa_actors::ground_states::OrganizationState<String>,
    ) -> Result<()> {
        self.definitions
            .organization_combat_value(crate::restore::OrganizationCombatContext {
                owner: self.owner,
                owner_actor: self.owner_actor,
                organization: org,
                registry: self.registry,
                precision: self.cx.pc,
            })
            .map_err(Self::world)
    }
    fn mind_speed_changed(&mut self, _old_bits: u32, _new_bits: u32) -> Result<()> {
        unported("owner Mind virtual +118 (speed changed)")
    }
    fn member_flag_4d1080(&mut self, _member: ActorId) -> Result<bool> {
        unported("4D1080 member state test")
    }
    fn set_owner_state(&mut self, _state: u8, _enabled: bool) -> Result<()> {
        unported("4D64F0 owner state change during restore")
    }
    fn set_leader(
        &mut self,
        _org: &mut aa_actors::ground_states::OrganizationState<String>,
        _member: ActorId,
    ) -> Result<()> {
        unported("579A00 set leader over the restore shells")
    }
    fn mind_member_added(&mut self, _member: ActorId) -> Result<()> {
        unported("524B80 owner Mind member added")
    }
    fn follow_leader(
        &mut self,
        _org: &mut aa_actors::ground_states::OrganizationState<String>,
        _member: ActorId,
    ) -> Result<()> {
        unported("57C800 follow leader")
    }
    fn owner_4d4cf0(&mut self) -> Result<()> {
        unported("4D4CF0 on the owner")
    }
    fn owner_4cf0f0(&mut self) -> Result<()> {
        unported("4CF0F0 on the owner")
    }
    fn owner_4d1350(&mut self) -> Result<()> {
        unported("4D1350 owner action-mask rebuild during restore")
    }
    fn member_added_5784c0(
        &mut self,
        _org: &mut aa_actors::ground_states::OrganizationState<String>,
        _member: ActorId,
    ) -> Result<()> {
        unported("5784C0 member added")
    }
    fn definition_list_2d8(&self, _actor_data: &String) -> Result<Vec<u32>> {
        unported("45EC60 ActorData+2D8 list")
    }
    fn apply_property(&mut self, _property: u32, _kind: u32, _actor: ActorId) -> Result<()> {
        unported("4CEF10 apply property")
    }
    fn remove_property(&mut self, _property: u32, _actor: ActorId) -> Result<()> {
        unported("4CEF70 remove property")
    }
    fn has_applied_property(&mut self, _property: u32, _actor: ActorId) -> Result<bool> {
        unported("4CEF80 applied-property test")
    }
    fn owner_object_notify(&mut self, _object: u32) -> Result<()> {
        unported("5147C0 owner object notify")
    }
    fn owner_kingdom(&mut self) -> Result<Option<u32>> {
        // GActor virtual +108 (4D1E80) returns the owner word +E8.
        let word = *self
            .owner_actor
            .words
            .get(&0xe8)
            .ok_or_else(|| LoadError::World("owner +E8 was not initialized".into()))?;
        Ok((word != 0).then_some(word))
    }
    // Recruit item handles are singles-vector index + 1 (0 = null).
    fn recruit_single_count(&self, kingdom: u32) -> Result<i32> {
        let Some(RestoredRecord::Kingdom(record)) = self
            .registry
            .get(ActorId::from_raw(kingdom))
            .map(|e| &e.value)
        else {
            return Err(LoadError::World(format!(
                "owner {kingdom:#010x} is not a restored Kingdom"
            )));
        };
        let shell = record
            .runtime
            .as_ref()
            .and_then(|r| r.derived.as_ref())
            .ok_or_else(|| LoadError::World("owner Kingdom has no derived state".into()))?;
        i32::try_from(shell.recruit_items.singles.len())
            .map_err(|_| LoadError::Unsupported("recruit singles exceed i32".into()))
    }
    fn recruit_single_at(&self, kingdom: u32, index: i32) -> Result<u32> {
        let count = self.recruit_single_count(kingdom)?;
        if index < 0 || index >= count {
            return Err(LoadError::Unsupported(format!(
                "4E13F0 reads recruit single {index} outside the +214 vector"
            )));
        }
        Ok(index as u32 + 1)
    }
    fn find_recruit_single(&mut self, kingdom: u32, actor_data: &String) -> Result<u32> {
        let id: Vec<u16> = actor_data.encode_utf16().collect();
        let token = self
            .definitions
            .actor_definition_token(&id, self.cx.pc)
            .map_err(Self::world)?;
        // 4E2A60(actor_data, 0): not limited to available singles.
        Ok(self
            .recruit_table(kingdom)?
            .find_single(token, false)
            .map_or(0, |index| index as u32 + 1))
    }
    fn recruit_item_words(&self, item: u32) -> Result<[u32; 2]> {
        let kingdom = self
            .owner_actor
            .words
            .get(&0xe8)
            .copied()
            .ok_or_else(|| LoadError::World("owner +E8 was not initialized".into()))?;
        let Some(RestoredRecord::Kingdom(record)) = self
            .registry
            .get(ActorId::from_raw(kingdom))
            .map(|e| &e.value)
        else {
            return Err(LoadError::World(
                "recruit item owner is not a restored Kingdom".into(),
            ));
        };
        let single = record
            .runtime
            .as_ref()
            .and_then(|r| r.derived.as_ref())
            .and_then(|s| s.recruit_items.singles.get((item as usize).wrapping_sub(1)))
            .ok_or_else(|| LoadError::World(format!("recruit item {item} is not live")))?;
        Ok([single.actor, single.organization])
    }
    fn pair_recruit_item(&mut self, item: u32, member: ActorId) -> Result<()> {
        let kingdom = self
            .owner_actor
            .words
            .get(&0xe8)
            .copied()
            .ok_or_else(|| LoadError::World("owner +E8 was not initialized".into()))?;
        // Take the table out so the pairing callbacks can read the registry.
        let mut table = std::mem::take(self.recruit_table(kingdom)?);
        let result = table.pair_single(
            (item as usize).wrapping_sub(1),
            member.raw(),
            &mut Pairing {
                registry: &*self.registry,
            },
        );
        *self.recruit_table(kingdom)? = table;
        result.map_err(|e| LoadError::World(format!("4E1200: {e:?}")))
    }
    fn damage_type(&mut self, _ids: &[u16]) -> Result<Option<u32>> {
        unported("48D500 KKC_DamageType handle binding")
    }
    fn formation_facts(&self, formation: u32) -> Result<(u8, u32)> {
        self.definitions
            .organization_formation_facts(formation, self.cx.pc)
            .map_err(Self::world)
    }
    fn state_group4(&mut self) -> Result<Option<StateWords>> {
        unported("4D49B0(owner, 4) state group")
    }
    fn formation_refresh(
        &mut self,
        _org: &mut aa_actors::ground_states::OrganizationState<String>,
    ) -> Result<()> {
        unported("57BEF0 formation refresh")
    }
    fn owner_object_164(&mut self) -> Result<Option<u32>> {
        unported("4D1900 owner +164 object")
    }
    fn object_refresh(&mut self, _object: u32) -> Result<()> {
        unported("4B2830 object refresh")
    }
}

/// 57D130 on the Organization the stage has just constructed. The component
/// stays in its slot afterwards whether or not the Load completed (native
/// partial state). Returns the bytes consumed.
pub(crate) fn load_organization<R: RestoreDefinitions>(
    actor: &mut ActorShell<RestoredComponent>,
    owner: ActorId,
    definition: &RuntimeDefinition,
    owner_thing_flags: u32,
    owner_thing_word_c: u32,
    owner_object_byte_4c: u8,
    registry: &mut IdentityRegistry<RestoredRecord>,
    definitions: &R,
    cx: &mut LoadContext<'_>,
    bytes: &[u8],
) -> std::result::Result<usize, String> {
    let default_layout = definition
        .ground
        .organization
        .as_ref()
        .map(|o| o.layout.clone())
        .ok_or("ActorData+2F0 default Organization layout is unresolved")?;
    // (+14C > 4) && [+148]+10: the +148 object is not restored yet.
    let word_14c = *actor
        .words
        .get(&0x14c)
        .ok_or("owner +14C was not initialized")?;
    if word_14c > 4 {
        return Err("Organization owner +148 object binding is unported".into());
    }
    let facts = OwnerFacts {
        definition: definition.proof.id.clone(),
        gtid: owner.raw(),
        has_mind: actor.components.present(ComponentSlot::Mind),
        object_10: None,
        default_layout,
    };
    let world_facts = WorldFacts {
        world_version: *cx.world_version,
        time_e4: cx.inventory.world.word_e4,
    };
    let Some(RestoredComponent::Organization(mut org)) =
        actor.components.remove(ComponentSlot::Organization)
    else {
        return Err("constructed Organization component is missing".into());
    };
    let pc = cx.pc;
    let result = {
        let mut world = RestoreOrganizationWorld {
            definitions,
            registry,
            cx,
            owner,
            owner_actor: actor,
            owner_definition: definition,
            owner_thing_flags,
            owner_thing_word_c,
            owner_object_byte_4c,
        };
        organization_state::load(&mut org, bytes, &facts, world_facts, &mut world, pc)
    };
    actor.components.insert(
        ComponentSlot::Organization,
        RestoredComponent::Organization(org),
    );
    result.map_err(|e| match e {
        // 858A00 / 858AD0 / 41DFB0 all throw TGC_LoadWarningException.
        LoadError::Warning(_) | LoadError::Fatal(_) | LoadError::Exception(_) => {
            crate::restore::load_warning_exception(e)
        }
        other => other.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        bootstrap::DefinitionProof,
        inventory::{ByteSpan, Placement, ThingState},
        restore::{LoadCompletion, LoadedEntity},
    };
    use aa_actors::loading::{LoadKind, construct_shell_with_components};
    use aa_body::{F32, credit::Object, properties::PropertiesState};
    use aa_numeric::Precision;

    fn shell(owner: ActorId, delta: f32) -> ActorShell<RestoredComponent> {
        let mut actor = construct_shell_with_components(LoadKind::Actor, ())
            .actor
            .unwrap();
        let mut properties = PropertiesState::new(Object(owner.raw()), 2).unwrap();
        properties.modifier.scalars[0x20 / 4 - 1] = F32(delta.to_bits());
        actor.components.insert(
            ComponentSlot::Properties,
            RestoredComponent::Properties(Box::new(properties)),
        );
        actor
    }

    fn record(id: ActorId, actor: ActorShell<RestoredComponent>) -> RestoredRecord {
        let proof = DefinitionProof {
            id: "fixture_member".into(),
            source: aa_rules::resources::Provenance {
                path: "fixture".into(),
                sha256: String::new(),
                span: aa_rules::tgi::Span { start: 0, end: 0 },
            },
            template_chain: vec![],
            category: *aa_rules::native::CATEGORIES
                .iter()
                .find(|c| c.name == "ACTOR")
                .unwrap(),
            base_flag_inputs: vec![],
        };
        let mut base = construct_shell_with_components(LoadKind::Actor, proof.clone());
        base.actor = Some(actor);
        RestoredRecord::Actor(Box::new(LoadedEntity {
            source: Placement {
                record_ordinal: 0,
                source_span: ByteSpan {
                    offset: 0,
                    length: 0,
                },
                thing_id_utf16: vec![],
                class_code: 3,
                serialized_id: id.raw(),
                thing: ThingState {
                    version: 2,
                    name_utf16: vec![],
                    word_1c: 0,
                    position_bits: [0; 3],
                    facing_bits: [0; 3],
                    serialized_flags: 0,
                    legacy_version_one_word: None,
                },
                object: None,
                actor: None,
                opaque_tail_span: ByteSpan {
                    offset: 0,
                    length: 0,
                },
                opaque_tail: vec![],
            },
            definition: RuntimeDefinition {
                proof,
                native_class: 3,
                flags: 0,
                object: None,
                component_mask: None,
                ground: Default::default(),
            },
            shell: base,
            completion: LoadCompletion::ComponentsLoaded,
            owner_source_slot: None,
            saved_components: vec![],
            actor_tail: None,
            doodad: None,
            special_object: None,
            failure: None,
            active_component: None,
        }))
    }

    #[test]
    fn morale_modifier_uses_loading_owner_and_generation_checked_member_contexts() {
        let owner = ActorId::from_raw(0x10001);
        let member = ActorId::from_raw(0x20002);
        let owner_actor = shell(owner, 2.0);
        let mut registry = IdentityRegistry::new();
        registry.seed_saved_generation(owner).unwrap();
        registry
            .register_saved(owner, true, RestoredRecord::Loading)
            .unwrap();
        registry.seed_saved_generation(member).unwrap();
        registry
            .register_saved(member, true, record(member, shell(member, -10.0)))
            .unwrap();
        for pc in [
            Precision::Single24,
            Precision::Double53,
            Precision::Extended64,
        ] {
            let own = restored_properties(owner, &owner_actor, &registry, owner)
                .unwrap()
                .unwrap();
            let other = restored_properties(owner, &owner_actor, &registry, member)
                .unwrap()
                .unwrap();
            assert_eq!(
                apply_morale_modifier(own, 4.0f32.to_bits(), 1.0f32.to_bits(), pc)
                    .unwrap()
                    .store_f32()
                    .unwrap(),
                6.0f32.to_bits()
            );
            assert_eq!(
                apply_morale_modifier(other, 4.0f32.to_bits(), 1.0f32.to_bits(), pc)
                    .unwrap()
                    .store_f32()
                    .unwrap(),
                1.0f32.to_bits()
            );
            assert_eq!(
                apply_morale_modifier(other, 0.5f32.to_bits(), 1.0f32.to_bits(), pc)
                    .unwrap()
                    .store_f32()
                    .unwrap(),
                0.5f32.to_bits()
            );
        }
        // Same slot, wrong generation must never alias the real member.
        assert!(
            restored_properties(owner, &owner_actor, &registry, ActorId::from_raw(0x10002))
                .is_err()
        );
        registry.remove(member).unwrap();
        assert!(restored_properties(owner, &owner_actor, &registry, member).is_err());
    }

    #[test]
    fn absent_properties_is_null_but_wrong_payload_or_owner_is_an_error() {
        let owner = ActorId::from_raw(1);
        let registry = IdentityRegistry::new();
        let mut owner_actor = shell(owner, 0.0);
        owner_actor.components.remove(ComponentSlot::Properties);
        assert!(
            restored_properties(owner, &owner_actor, &registry, owner)
                .unwrap()
                .is_none()
        );
        owner_actor.components.insert(
            ComponentSlot::Properties,
            RestoredComponent::Detection(Box::new(aa_actors::ground_states::DetectionState::new(
                owner,
                F32::ZERO,
            ))),
        );
        assert!(
            restored_properties(owner, &owner_actor, &registry, owner)
                .unwrap_err()
                .to_string()
                .contains("payload mismatch")
        );
        let wrong = shell(ActorId::from_raw(2), 0.0);
        assert!(
            restored_properties(owner, &wrong, &registry, owner)
                .unwrap_err()
                .to_string()
                .contains("different Actor")
        );
    }
}
