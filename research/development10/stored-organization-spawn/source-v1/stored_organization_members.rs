//!57CBA0 over the installed Organization and same current ActorStore owners.
//!Short field loans end before native children; no component is removed.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeComponent},
    catalog::Catalog,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    components::ComponentSlot as Slot,
    ground_states::OrganizationState,
    organization_callees as callees,
    organization_state::{self as core, AddMemberFields},
    state,
};
use aa_numeric::Precision;
type Result<T> = std::result::Result<T, String>;
fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn error(e: impl std::fmt::Display) -> core::LoadError {
    core::LoadError::Unsupported(e.to_string())
}
fn required<T>(s: &str) -> core::Result<T> {
    Err(error(s))
}
fn org(actors: &ActorStore, owner: ActorId) -> Result<&OrganizationState<String>> {
    match actors.component(owner, Slot::Organization).map_err(text)? {
        Some(RuntimeComponent::Organization(v)) if v.header.owner == owner => Ok(v),
        _ => Err("57CBA0 actual Organization allocation/owner differs".into()),
    }
}
fn org_mut(actors: &mut ActorStore, owner: ActorId) -> Result<&mut OrganizationState<String>> {
    match actors
        .component_mut(owner, Slot::Organization)
        .map_err(text)?
    {
        Some(RuntimeComponent::Organization(v)) if v.header.owner == owner => Ok(v),
        _ => Err("57CBA0 actual Organization allocation/owner differs".into()),
    }
}
fn flags(staged: &Staged, key: ObjectKey) -> Result<u32> {
    if let Ok(v) = staged.actors.staged_placement(key) {
        return Ok(v.flags);
    }
    let world = staged
        .load_world
        .as_ref()
        .ok_or("57CBA0 actual World placement absent")?;
    staged
        .actors
        .placement(key, world.live_spatial()?)
        .map(|p| p.flags)
        .map_err(text)
}
struct Bound<'a, 'source> {
    catalog: &'a Catalog<'source>,
    staged: &'a mut Staged,
    key: ObjectKey,
    owner: ActorId,
    source_pc: Precision,
    live_pc: Precision,
}
impl Bound<'_, '_> {
    fn current_mind(&self) -> core::Result<&aa_mind::construct::Installed> {
        let Some(RuntimeComponent::Mind(m)) =
            self.component(self.owner, Slot::Mind).map_err(error)?
        else {
            return required("57CBA0 actual installed Mind absent");
        };
        let object = self.staged.actors.object(self.key).map_err(error)?;
        let expected = crate::catalog::mind_kind(
            object
                .definition
                .component_mask
                .ok_or_else(|| error("57CBA0 actual source mask absent"))?,
        )
        .ok_or_else(|| error("57CBA0 source has no concrete Mind kind"))?;
        if m.kind != expected
            || self
                .staged
                .actors
                .minds
                .mind(m.mind_id)
                .map_err(error)?
                .actor
                != self.owner.raw()
        {
            return required("57CBA0 same Arena/source Mind identity differs");
        }
        Ok(m)
    }
    fn state_status(&self, actor: ActorId, offset: u32) -> Result<u32> {
        let Some(RuntimeComponent::Mind(m)) = self.component(actor, Slot::Mind)? else {
            return Err("actual Mind state owner absent".into());
        };
        let id = *m
            .state_bindings
            .get(&offset)
            .ok_or("actual Mind embedded state binding absent")?;
        let state = self.staged.actors.minds.state(id).map_err(text)?;
        if state.parent != m.mind_id
            || self
                .staged
                .actors
                .minds
                .mind(m.mind_id)
                .map_err(text)?
                .actor
                != actor.raw()
        {
            return Err("actual Mind embedded state/Arena owner differs".into());
        }
        Ok(state.status)
    }
    fn key(&self, id: ActorId) -> Result<ObjectKey> {
        self.staged
            .actors
            .key(id)
            .ok_or_else(|| format!("57CBA0 current Actor {} generation absent", id.raw()))
    }
    fn component(&self, id: ActorId, slot: Slot) -> Result<Option<&RuntimeComponent>> {
        self.staged.actors.component(id, slot).map_err(text)
    }
    fn rebuild(&mut self, id: ActorId) -> Result<()> {
        let key = self.key(id)?;
        let w = self
            .staged
            .load_world
            .as_ref()
            .ok_or("57CBA0 action World absent")?;
        crate::stored_actor_actions::rebuild_with_variables(
            self.catalog,
            &mut self.staged.actors,
            &mut self.staged.definitions,
            key,
            &self.staged.inventory,
            w.live_spatial()?,
            Some(&w.variables),
            self.live_pc,
        )
    }
    fn economy(&mut self, id: ActorId) -> Result<()> {
        let w = self
            .staged
            .load_world
            .as_ref()
            .ok_or("57CBA0 economy World absent")?;
        let (_, mut access) = self.staged.actors.split_minds();
        crate::stored_economy_refresh::refresh(
            self.catalog,
            &mut access,
            &mut self.staged.definitions,
            w.live_spatial()?,
            &w.variables,
            id,
            self.live_pc,
        )
    }
    fn observer(&self) -> Result<&crate::restore_lifecycle::InactiveLoad> {
        self.staged
            .inactive_load
            .as_ref()
            .ok_or("5705A0 actual moved managed lifecycle absent".into())
    }
    fn org_word(&self, o: u32) -> Result<u32> {
        org(&self.staged.actors, self.owner)?
            .words
            .get(&o)
            .copied()
            .ok_or_else(|| format!("57CBA0 actual Organization+{o:X} unwritten"))
    }
    fn set_state(&mut self, index: u8, enabled: bool) -> Result<()> {
        if !matches!(index, 0x22 | 0x25) {
            return Err("57CBA0 unexpected state dispatch".into());
        }
        let s = self
            .staged
            .actors
            .object(self.key)
            .map_err(text)?
            .components()
            .map_err(text)?
            .state;
        let mut f = state::Fields {
            bits: u64::from(s.low) | (u64::from(s.high) << 32),
            thing_flags: flags(self.staged, self.key)?,
        };
        let result =
            state::set(&mut f, index, enabled, &mut StateEffects { world: self }).map_err(text);
        let s = &mut self
            .staged
            .actors
            .object_mut(self.key)
            .map_err(text)?
            .components_mut()
            .map_err(text)?
            .state;
        s.low = f.bits as u32;
        s.high = (f.bits >> 32) as u32;
        result
    }
}
impl aa_combat::organization::AttachWorld for Bound<'_, '_> {
    fn set_membership(&mut self, member: u32, owner: u32, slot: i32) -> aa_combat::Result<()> {
        let fail = |e: String| aa_combat::Error::World(e);
        if owner != self.owner.raw() {
            return Err(fail("5705A0 parent identity differs".into()));
        }
        let id = ActorId::from_raw(member);
        let Some(RuntimeComponent::Element(e)) = self
            .staged
            .actors
            .component_mut(id, Slot::Element)
            .map_err(|e| fail(text(e)))?
        else {
            return Err(fail("5705A0 actual member Element absent".into()));
        };
        if e.header.owner != id {
            return Err(fail("5705A0 member Element owner differs".into()));
        }
        e.slot_index = slot;
        e.organization = Some(self.owner);
        e.flag_1c = true;
        Ok(())
    }
    fn notify_member(&mut self, member: u32) -> aa_combat::Result<()> {
        self.rebuild(ActorId::from_raw(member))
            .map_err(aa_combat::Error::World)
    }
    fn refresh_member(&mut self, member: u32) -> aa_combat::Result<()> {
        self.economy(ActorId::from_raw(member))
            .map_err(aa_combat::Error::World)
    }
    fn observer_present(&self) -> aa_combat::Result<bool> {
        self.catalog
            .saved_observer_present_in_load(self.observer().map_err(aa_combat::Error::World)?)
            .map_err(aa_combat::Error::World)
    }
    fn notify_observer(&mut self, actor: u32) -> aa_combat::Result<()> {
        self.catalog
            .notify_saved_observer_in_load(self.observer().map_err(aa_combat::Error::World)?, actor)
            .map_err(aa_combat::Error::World)
    }
}
impl callees::ActorFacts for Bound<'_, '_> {
    fn experience_rank(&self, id: ActorId) -> callees::Result<Option<u32>> {
        let fail = |s: String| callees::Error::World(s);
        match self.component(id, Slot::Experience).map_err(fail)? {
            None => Ok(None),
            Some(RuntimeComponent::Experience(e)) if e.owner.0 == id.raw() => {
                Ok(Some(e.current_rank.map_or(0, |v| v.0)))
            }
            _ => Err(fail(
                "4CED10 actual Experience payload/owner differs".into(),
            )),
        }
    }
    fn element_organization(&self, id: ActorId) -> callees::Result<Option<ActorId>> {
        match self
            .component(id, Slot::Element)
            .map_err(callees::Error::World)?
        {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == id => Ok(e.organization),
            _ => Err(callees::Error::World(
                "4CED10 actual Element payload/owner differs".into(),
            )),
        }
    }
    fn has_performer(&self, id: ActorId) -> callees::Result<bool> {
        Ok(self
            .component(id, Slot::Performer)
            .map_err(callees::Error::World)?
            .is_some())
    }
    fn organization_members(&self, id: ActorId) -> callees::Result<Option<Vec<ActorId>>> {
        match self
            .component(id, Slot::Organization)
            .map_err(callees::Error::World)?
        {
            None => Ok(None),
            Some(RuntimeComponent::Organization(v)) if v.header.owner == id => Ok(Some(
                v.members.iter().copied().filter(|m| !m.is_null()).collect(),
            )),
            _ => Err(callees::Error::World(
                "57A330 actual Organization payload/owner differs".into(),
            )),
        }
    }
}
impl callees::RankWorld for Bound<'_, '_> {
    fn performer_rank_changed(&mut self, id: ActorId, rank: u32) -> callees::Result<()> {
        Err(callees::Error::World(format!(
            "57DCF0 actual rank definition/index owner required actor={} rank={rank}",
            id.raw()
        )))
    }
}
impl callees::BodyUpdatable for Bound<'_, '_> {
    fn body_has_scheduled(&mut self, id: ActorId, event: u8, data: u32) -> callees::Result<bool> {
        let key = self.key(id).map_err(callees::Error::World)?;
        let owner = self
            .staged
            .owners
            .get(crate::scheduler_owners::Target::Component {
                object: key,
                slot: Slot::Body.index() as u8,
            })
            .map_err(|e| callees::Error::World(text(e)))?;
        if self.staged.queues.precision != self.live_pc {
            return Err(callees::Error::World("561760 queue live PC differs".into()));
        }
        Ok(self.staged.queues.has_from_owner_head(owner, event, data))
    }
    fn body_unschedule(&mut self, id: ActorId, event: u8, data: u32) -> callees::Result<()> {
        let key = self.key(id).map_err(callees::Error::World)?;
        let owner = self
            .staged
            .owners
            .get(crate::scheduler_owners::Target::Component {
                object: key,
                slot: Slot::Body.index() as u8,
            })
            .map_err(|e| callees::Error::World(text(e)))?;
        self.staged
            .queues
            .remove_matching(owner, event, data)
            .map(|_| ())
            .map_err(|e| callees::Error::World(text(e)))
    }
}
impl AddMemberFields<String> for Bound<'_, '_> {
    fn layout_info(&mut self) -> core::Result<core::LayoutInfo> {
        aa_mapworld::restore::RestoreDefinitions::organization_layout_info(
            self.catalog,
            &org(&self.staged.actors, self.owner)
                .map_err(error)?
                .layout
                .definition,
            self.source_pc,
        )
        .map_err(error)
    }
    fn member_count(&mut self) -> core::Result<usize> {
        Ok(org(&self.staged.actors, self.owner)
            .map_err(error)?
            .members
            .len())
    }
    fn put_member(&mut self, i: usize, id: ActorId) -> core::Result<()> {
        *org_mut(&mut self.staged.actors, self.owner)
            .map_err(error)?
            .members
            .get_mut(i)
            .ok_or_else(|| error("57CBA0 actual member array bound changed"))? = id;
        Ok(())
    }
    fn word(&mut self, o: u32) -> core::Result<u32> {
        self.org_word(o).map_err(error)
    }
    fn put_word(&mut self, o: u32, v: u32) -> core::Result<()> {
        org_mut(&mut self.staged.actors, self.owner)
            .map_err(error)?
            .words
            .insert(o, v);
        Ok(())
    }
    fn element_set_organization(&mut self, id: ActorId, slot: i32) -> core::Result<()> {
        aa_combat::organization::attach_membership(id.raw(), self.owner.raw(), slot, self)
            .map_err(error)
    }
    fn member_has_body(&mut self, id: ActorId) -> core::Result<bool> {
        match self.component(id, Slot::Body).map_err(error)? {
            None => Ok(false),
            Some(RuntimeComponent::Body(v)) if v.header.owner == id => Ok(true),
            _ => required("561760 actual Body payload/owner differs"),
        }
    }
    fn body_set_organization(&mut self, id: ActorId) -> core::Result<()> {
        callees::body_set_organization(id, Some(self.owner), self).map_err(error)
    }
    fn member_has_experience(&mut self, id: ActorId) -> core::Result<bool> {
        match self.component(id, Slot::Experience).map_err(error)? {
            None => Ok(false),
            Some(RuntimeComponent::Experience(v)) if v.owner.0 == id.raw() => Ok(true),
            _ => required("57CBA0 actual Experience payload/owner differs"),
        }
    }
    fn owner_experience_token(&mut self) -> core::Result<u32> {
        callees::experience_source(self.owner, self).map_err(error)
    }
    fn set_experience_source(&mut self, id: ActorId, token: u32) -> core::Result<()> {
        callees::rank_changed(id, 0, token, self).map_err(error)
    }
    fn combat_value(&mut self) -> core::Result<()> {
        crate::stored_actor_value_refresh::refresh_organization(
            self.catalog,
            &mut self.staged.actors,
            &self.staged.definitions,
            self.key,
            self.live_pc,
        )
        .map_err(error)
    }
    fn member_mover_speed(&mut self, id: ActorId) -> core::Result<Option<u32>> {
        match self.component(id, Slot::Mover).map_err(error)? {
            None => Ok(None),
            Some(RuntimeComponent::Mover(v)) if v.header.owner == id => Ok(Some(v.speed.0)),
            _ => required("57CBA0 actual Mover payload/owner differs"),
        }
    }
    fn owner_has_mind(&mut self) -> core::Result<bool> {
        Ok(self
            .component(self.owner, Slot::Mind)
            .map_err(error)?
            .is_some())
    }
    fn mind_speed_changed(&mut self, old: u32, new: u32) -> core::Result<()> {
        let kind = self.current_mind()?.kind;
        crate::organization_mind_callbacks::speed_changed(kind, self, old, new).map_err(error)
    }
    fn member_flag_4d1080(&mut self, id: ActorId) -> core::Result<bool> {
        Ok(self
            .staged
            .actors
            .object(self.key(id).map_err(error)?)
            .map_err(error)?
            .components()
            .map_err(error)?
            .state
            .high
            & 4
            != 0)
    }
    fn set_owner_state(&mut self, index: u8, enabled: bool) -> core::Result<()> {
        self.set_state(index, enabled).map_err(error)
    }
    fn set_leader(&mut self, member: ActorId) -> core::Result<()> {
        aa_movement::borrowed_formation::set_leader(self, Some(member), self.live_pc).map_err(error)
    }
    fn mind_member_added(&mut self, member: ActorId) -> core::Result<()> {
        crate::stored_organization_member_mind::notify(
            self.catalog,
            self.staged,
            self.owner,
            member,
        )
        .map_err(error)
    }
    fn follow_leader(&mut self, member: ActorId) -> core::Result<()> {
        aa_actors::element_position::organization_moved(self.owner, member, self).map_err(error)
    }
    fn owner_4d4cf0(&mut self) -> core::Result<()> {
        crate::stored_actor_properties::refresh(
            self.catalog,
            &mut self.staged.actors,
            &self.staged.definitions,
            self.key,
        )
        .map_err(error)
    }
    fn owner_4cf0f0(&mut self) -> core::Result<()> {
        self.economy(self.owner).map_err(error)
    }
    fn owner_4d1350(&mut self) -> core::Result<()> {
        self.rebuild(self.owner).map_err(error)
    }
    fn member_added_5784c0(&mut self, member: ActorId) -> core::Result<()> {
        crate::saved_member_properties::propagate(self, member).map_err(error)
    }
    fn member_definition(&mut self, id: ActorId) -> core::Result<String> {
        Ok(self
            .staged
            .actors
            .object(self.key(id).map_err(error)?)
            .map_err(error)?
            .definition
            .proof
            .id
            .clone())
    }
    fn definition_list_2d8(&mut self, id: &String) -> core::Result<Vec<u32>> {
        aa_mapworld::restore::RestoreDefinitions::member_definition_properties(
            self.catalog,
            id,
            self.source_pc,
        )
        .map_err(error)
    }
    fn apply_property(&mut self, property: u32, kind: u32, source: ActorId) -> core::Result<()> {
        //57CCE2 loads ECX from Organization+4. The member is the third
        //argument (source), not the target of this supporting Property.
        let reference = aa_rules::native::layout_data::PropertyRef::from_handle(property)
            .ok_or_else(|| error("57CBA0 supporting Property reference is null"))?;
        crate::staged_property_admission::apply(
            self.staged,
            self.catalog,
            self.key,
            Some(reference),
            kind,
            Some(source),
            self.live_pc,
        )
        .map(|_| ())
        .map_err(error)
    }
    fn owner_object_10(&mut self) -> core::Result<Option<u32>> {
        Ok(self
            .staged
            .actors
            .object(self.key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or_else(|| error("57CBA0 owner Actor absent"))?
            .zones_148
            .zones()
            .slots
            .get(4)
            .and_then(Option::as_ref)
            .map(|z| z.key))
    }
    fn owner_object_notify(&mut self, _: u32) -> core::Result<()> {
        required("57CBA0 actual stored5147C0 Zone4 callback")
    }
}

impl Bound<'_, '_> {
    fn properties_state(
        &self,
        actor: ActorId,
    ) -> Result<Option<&aa_body::properties::PropertiesState>> {
        match self.component(actor, Slot::Properties)? {
            None => Ok(None),
            Some(RuntimeComponent::Properties(p)) if p.owner.0 == actor.raw() => Ok(Some(p)),
            _ => Err("5784C0 actual Properties payload/owner differs".into()),
        }
    }
    fn captured_properties(
        &self,
        receipt: (ActorId, usize),
    ) -> Result<&aa_body::properties::PropertiesState> {
        let p = self
            .properties_state(receipt.0)?
            .ok_or("5784C0 captured Properties allocation detached")?;
        //Identity-only receipt: never dereferenced, copied back or serialized.
        if p as *const _ as usize != receipt.1 {
            return Err("5784C0 captured Properties allocation replaced".into());
        }
        Ok(p)
    }
    fn property_instance(
        &self,
        id: aa_body::credit::Object,
    ) -> Result<&aa_body::property_effects::Instance> {
        let state = &self.staged.actors.property_allocations;
        state.require_in_bank(self.catalog, &self.staged.definitions)?;
        let instance = state
            .instances()
            .get(&id)
            .ok_or("5784C0 actual current instance allocation absent")?;
        if instance.handle != id {
            return Err("5784C0 instance handle/allocation identity differs".into());
        }
        Ok(instance)
    }
}
impl crate::saved_member_properties::LookupWorld for Bound<'_, '_> {
    type Properties = (ActorId, usize);
    fn properties(&self, actor: ActorId) -> Result<Option<Self::Properties>> {
        Ok(self
            .properties_state(actor)?
            .map(|p| (actor, p as *const _ as usize)))
    }
    fn count(&self, receipt: Self::Properties) -> Result<i32> {
        i32::try_from(self.captured_properties(receipt)?.effects.len())
            .map_err(|_| "5784C0 current count exceeds native signed count".into())
    }
    fn slot(&self, receipt: Self::Properties, index: i32) -> Result<aa_body::credit::Object> {
        let p = self.captured_properties(receipt)?;
        usize::try_from(index)
            .ok()
            .and_then(|i| p.effects.get(i).copied())
            .ok_or_else(|| format!("5784C0 current Properties slot {index} absent"))
    }
    fn instance_definition(
        &self,
        instance: aa_body::credit::Object,
    ) -> Result<aa_body::credit::Object> {
        Ok(self.property_instance(instance)?.definition)
    }
    fn instance_source(&self, instance: aa_body::credit::Object) -> Result<ActorId> {
        let id = ActorId::from_raw(self.property_instance(instance)?.source_id);
        Ok(if self.staged.actors.key(id).is_some() {
            id
        } else {
            ActorId::NULL
        })
    }
}
impl crate::saved_member_properties::World for Bound<'_, '_> {
    fn owner(&self) -> Result<ActorId> {
        Ok(org(&self.staged.actors, self.owner)?.header.owner)
    }
    fn pass_down_members(&self, definition: aa_body::credit::Object) -> Result<bool> {
        let state = &self.staged.actors.property_allocations;
        state.require_in_bank(self.catalog, &self.staged.definitions)?;
        Ok(state.facts(definition)?.pass_down_members())
    }
    fn has_experience(&self, actor: ActorId) -> Result<bool> {
        match self.component(actor, Slot::Experience)? {
            None => Ok(false),
            Some(RuntimeComponent::Experience(p)) if p.owner.0 == actor.raw() => Ok(true),
            _ => Err("5784C0 current Experience payload/owner differs".into()),
        }
    }
    fn instance_kind(&self, instance: aa_body::credit::Object) -> Result<u32> {
        Ok(self.property_instance(instance)?.kind)
    }
    fn instance_disabled(&self, instance: aa_body::credit::Object) -> Result<bool> {
        Ok(self.property_instance(instance)?.disabled)
    }
    fn apply(
        &mut self,
        actor: ActorId,
        property: aa_body::credit::Object,
        kind: u32,
        source: ActorId,
    ) -> Result<()> {
        let state = &self.staged.actors.property_allocations;
        state.require_in_bank(self.catalog, &self.staged.definitions)?;
        let reference = state.facts(property)?.reference();
        let key = self.key(actor)?;
        crate::staged_property_admission::apply(
            self.staged,
            self.catalog,
            key,
            Some(reference),
            kind,
            Some(source),
            self.live_pc,
        )
        .map(|_| ())
    }
}
struct StateEffects<'a, 'world, 'source> {
    world: &'a mut Bound<'world, 'source>,
}
impl StateEffects<'_, '_, '_> {
    fn commit(&mut self, f: &state::Fields) -> Result<()> {
        let s = &mut self
            .world
            .staged
            .actors
            .object_mut(self.world.key)
            .map_err(text)?
            .components_mut()
            .map_err(text)?
            .state;
        s.low = f.bits as u32;
        s.high = (f.bits >> 32) as u32;
        Ok(())
    }
    fn reread(&mut self, f: &mut state::Fields) -> Result<()> {
        let s = self
            .world
            .staged
            .actors
            .object(self.world.key)
            .map_err(text)?
            .components()
            .map_err(text)?
            .state;
        f.bits = u64::from(s.low) | (u64::from(s.high) << 32);
        f.thing_flags = flags(self.world.staged, self.world.key)?;
        Ok(())
    }
}
impl state::Effects for StateEffects<'_, '_, '_> {
    fn leave(&mut self, f: &mut state::Fields, d: &'static state::Descriptor) -> state::Result<()> {
        self.commit(f).map_err(state::Error::Callback)?;
        let r = crate::stored_actor_state_effects::leave_staged(
            self.world.staged,
            self.world.catalog,
            self.world.key,
            d.index,
            self.world.live_pc,
        );
        self.reread(f).map_err(state::Error::Callback)?;
        r.map_err(state::Error::Callback)
    }
    fn enter(
        &mut self,
        f: &mut state::Fields,
        d: &'static state::Descriptor,
        o: Option<&'static state::Descriptor>,
    ) -> state::Result<()> {
        self.commit(f).map_err(state::Error::Callback)?;
        let r = crate::stored_actor_state_effects::enter_staged(
            self.world.staged,
            self.world.catalog,
            self.world.key,
            d.index,
            o.map(|d| d.index),
            self.world.live_pc,
        );
        self.reread(f).map_err(state::Error::Callback)?;
        r.map_err(state::Error::Callback)
    }
    fn rebuild_abilities(&mut self, f: &mut state::Fields) -> state::Result<()> {
        self.commit(f).map_err(state::Error::Callback)?;
        let r = self.world.rebuild(self.world.owner);
        self.reread(f).map_err(state::Error::Callback)?;
        r.map_err(state::Error::Callback)
    }
}
///The native slot is supplied by57D950, not inferred from member count/order.
pub fn add_member(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    slot: i32,
    member: ActorId,
    live_pc: Precision,
) -> Result<()> {
    let runtime = crate::fresh_post_create_world::canonical(staged, catalog, key)?;
    let owner = staged
        .actors
        .id(key)
        .ok_or("57CBA0 actual Organization owner GTID absent")?;
    if runtime
        .definition
        .component_mask
        .is_none_or(|m| m & (1 << Slot::Organization.index()) == 0)
    {
        return Err("57CBA0 source has no Organization factory".into());
    }
    org(&staged.actors, owner)?;
    let member_key = staged
        .actors
        .key(member)
        .ok_or("57CBA0 actual member generation absent")?;
    crate::fresh_post_create_world::canonical(staged, catalog, member_key)?;
    let source_pc = staged.definitions.combat_source()?.source_precision();
    core::add_member_fields(
        &mut Bound {
            catalog,
            staged,
            key,
            owner,
            source_pc,
            live_pc,
        },
        slot,
        member,
    )
    .map_err(|e| {
        format!(
            "57CBA0(owner={}, slot={slot}, member={}): {e}",
            owner.raw(),
            member.raw()
        )
    })
}

impl crate::organization_mind_callbacks::Fields for Bound<'_, '_> {
    fn embedded_status(&mut self, offset: u32) -> Result<u32> {
        self.current_mind().map_err(text)?;
        self.state_status(self.owner, offset)
    }
    fn move_speed_changed(&mut self, old: u32, new: u32) -> Result<()> {
        Err(format!(
            "actual active Mind move virtual24 speed callback old={old} new={new}"
        ))
    }
    fn put_word(&mut self, offset: u32, word: u32) -> Result<()> {
        self.current_mind().map_err(text)?;
        let Some(RuntimeComponent::Mind(m)) = self
            .staged
            .actors
            .component_mut(self.owner, Slot::Mind)
            .map_err(text)?
        else {
            return Err("actual Mind vanished".into());
        };
        if !m.words.contains_key(&offset) {
            return Err(format!("actual Mind+{offset:X} field storage absent"));
        }
        m.words.insert(offset, word);
        Ok(())
    }
}
impl aa_movement::borrowed_formation::World for Bound<'_, '_> {
    type Actor = ActorId;
    type Positions = ();
    type Formation = u32;
    type Mind = ();
    fn leader(&mut self) -> aa_movement::Result<Option<ActorId>> {
        let v = self.org_word(0x40).map_err(movement)?;
        Ok((v != 0).then_some(ActorId::from_raw(v)))
    }
    fn store_leader(&mut self, member: Option<ActorId>) -> aa_movement::Result<()> {
        org_mut(&mut self.staged.actors, self.owner)
            .map_err(movement)?
            .words
            .insert(0x40, member.map_or(0, ActorId::raw));
        Ok(())
    }
    fn element_slot(&mut self, id: ActorId) -> aa_movement::Result<i32> {
        match self.component(id, Slot::Element).map_err(movement)? {
            Some(RuntimeComponent::Element(v)) if v.header.owner == id => Ok(v.slot_index),
            _ => Err(movement("5797E0 actual Element payload/owner differs")),
        }
    }
    fn member(&mut self, slot: i32) -> aa_movement::Result<Option<ActorId>> {
        let id = *org(&self.staged.actors, self.owner)
            .map_err(movement)?
            .members
            .get(usize::try_from(slot).map_err(movement)?)
            .ok_or_else(|| movement("5797E0 actual member array bound"))?;
        Ok((!id.is_null()).then_some(id))
    }
    fn primary_slot(&mut self) -> aa_movement::Result<i32> {
        Ok(AddMemberFields::layout_info(self)
            .map_err(movement)?
            .primary_slot)
    }
    fn formation(&mut self) -> aa_movement::Result<Option<u32>> {
        let v = self.org_word(0x74).map_err(movement)?;
        Ok((v != 0).then_some(v))
    }
    fn formation_oriented(&mut self, formation: u32) -> aa_movement::Result<bool> {
        let data = self.catalog.layout_data(self.source_pc).map_err(movement)?;
        let f = data
            .layout_formation(formation)
            .ok_or_else(|| movement("5797E0 actual LayoutFormation identity absent"))?;
        Ok(f.formation
            .and_then(|i| data.formations.get(i as usize))
            .ok_or_else(|| movement("5797E0 FormationData source absent"))?
            .organized)
    }
    fn owner_facing(&mut self) -> aa_movement::Result<[f32; 2]> {
        let p = self.pose(self.owner).map_err(movement)?;
        Ok([p.facing_bits[0], p.facing_bits[1]].map(f32::from_bits))
    }
    fn formation_offset(&mut self, slot: i32) -> aa_movement::Result<[f32; 2]> {
        let formation = self.org_word(0x74).map_err(movement)?;
        let data = self.catalog.layout_data(self.source_pc).map_err(movement)?;
        let f = data
            .layout_formation(formation)
            .ok_or_else(|| movement("578250 actual LayoutFormation absent"))?;
        Ok(f.positions
            .get(usize::try_from(slot).map_err(movement)?)
            .ok_or_else(|| movement("578250 actual formation slot bound"))?
            .offset_bits
            .map(f32::from_bits))
    }
    fn positions(&mut self) -> aa_movement::Result<()> {
        org(&self.staged.actors, self.owner).map_err(movement)?;
        Ok(())
    }
    fn offset(&mut self, _: (), slot: i32, axis: usize) -> aa_movement::Result<f32> {
        if axis > 1 {
            return Err(movement("5797E0 axis bound"));
        }
        Ok(f32::from_bits(
            org(&self.staged.actors, self.owner)
                .map_err(movement)?
                .offsets_78
                .get(usize::try_from(slot).map_err(movement)?)
                .ok_or_else(|| movement("5797E0 actual offset array bound"))?[axis]
                .0,
        ))
    }
    fn store_offset(
        &mut self,
        _: (),
        slot: i32,
        axis: usize,
        value: f32,
    ) -> aa_movement::Result<()> {
        if axis > 1 {
            return Err(movement("5797E0 axis bound"));
        }
        org_mut(&mut self.staged.actors, self.owner)
            .map_err(movement)?
            .offsets_78
            .get_mut(usize::try_from(slot).map_err(movement)?)
            .ok_or_else(|| movement("5797E0 actual offset store array bound"))?[axis] =
            aa_body::F32(value.to_bits());
        Ok(())
    }
    fn element_attached(&mut self, id: ActorId) -> aa_movement::Result<bool> {
        match self.component(id, Slot::Element).map_err(movement)? {
            Some(RuntimeComponent::Element(v)) if v.header.owner == id => Ok(v.flag_1c),
            _ => Err(movement("5797E0 actual Element payload/owner differs")),
        }
    }
    fn actor_position(&mut self, id: ActorId, axis: usize) -> aa_movement::Result<f32> {
        if axis > 1 {
            return Err(movement("5797E0 position axis bound"));
        }
        Ok(f32::from_bits(
            self.pose(id).map_err(movement)?.position_bits[axis],
        ))
    }
    fn owner_id(&mut self) -> aa_movement::Result<u32> {
        Ok(self.owner.raw())
    }
    fn actor_id(&mut self, id: ActorId) -> aa_movement::Result<u32> {
        self.key(id).map_err(movement)?;
        Ok(id.raw())
    }
    fn owner_mind(&mut self) -> aa_movement::Result<Option<()>> {
        match self.component(self.owner, Slot::Mind).map_err(movement)? {
            None => Ok(None),
            Some(RuntimeComponent::Mind(_)) => Ok(Some(())),
            _ => Err(movement("579A00 actual Mind payload differs")),
        }
    }
    fn mind_leader_changed(&mut self, _: ()) -> aa_movement::Result<()> {
        let kind = self.current_mind().map_err(movement)?.kind;
        crate::organization_mind_callbacks::leader_changed(kind, self).map_err(movement)
    }
    fn next_random_u32(&mut self) -> aa_movement::Result<u32> {
        Ok(self.staged.rng.next_u32())
    }
    fn detached_member(&mut self, _: u32, _: u32, _: [f32; 2]) -> aa_movement::Result<()> {
        Err(movement(
            "5797E0 actual detached-member404F90 diagnostic owner",
        ))
    }
}
fn movement(e: impl std::fmt::Display) -> aa_movement::Error {
    aa_movement::Error::World(e.to_string())
}
impl Bound<'_, '_> {
    fn pose(&self, id: ActorId) -> Result<crate::actor_world::PlacementState> {
        let key = self.key(id)?;
        if let Ok(p) = self.staged.actors.staged_placement(key) {
            return Ok(p);
        }
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or("57CBA0 actual World placement absent")?;
        self.staged
            .actors
            .placement(key, world.live_spatial()?)
            .map_err(text)
    }
}

fn position_error(e: impl std::fmt::Display) -> aa_actors::element_position::Error {
    aa_actors::element_position::Error::World(e.to_string())
}
impl aa_actors::element_position::World for Bound<'_, '_> {
    type Definition = String;
    fn world_mode(&self) -> aa_actors::element_position::Result<u32> {
        self.staged
            .load_world
            .as_ref()
            .ok_or_else(|| position_error("57C800 actual World absent"))?
            .map_mode
            .as_ref()
            .ok_or_else(|| position_error("57C800 actual World+D8 mode absent"))
    }
    fn actor(
        &self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<aa_actors::element_position::Actor> {
        let p = self.pose(id).map_err(position_error)?;
        Ok(aa_actors::element_position::Actor {
            flags: p.flags,
            position: [
                aa_body::F32(p.position_bits[0]),
                aa_body::F32(p.position_bits[1]),
            ],
            mind: self
                .component(id, Slot::Mind)
                .map_err(position_error)?
                .is_some()
                .then_some(id.raw()),
        })
    }
    fn organization(
        &self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<&OrganizationState<String>> {
        if id != self.owner {
            return Err(position_error(
                "57C800 current Organization identity differs",
            ));
        }
        org(&self.staged.actors, id).map_err(position_error)
    }
    fn organization_mut(
        &mut self,
        id: ActorId,
    ) -> aa_actors::element_position::Result<&mut OrganizationState<String>> {
        if id != self.owner {
            return Err(position_error(
                "57C800 current Organization identity differs",
            ));
        }
        org_mut(&mut self.staged.actors, id).map_err(position_error)
    }
    fn selected_member(&self, id: ActorId) -> aa_actors::element_position::Result<Option<ActorId>> {
        if id != self.owner {
            return Err(position_error(
                "57C800 current Organization identity differs",
            ));
        }
        let value = self.org_word(0x40).map_err(position_error)?;
        Ok((value != 0).then_some(ActorId::from_raw(value)))
    }
    fn mind_predicate(
        &mut self,
        mind: u32,
        slot: u32,
    ) -> aa_actors::element_position::Result<bool> {
        if mind != self.owner.raw() {
            return Err(position_error(
                "578E90 actual current Mind identity differs",
            ));
        }
        let kind = self.current_mind().map_err(position_error)?.kind;
        crate::organization_mind_callbacks::position_active(kind, slot, self)
            .map_err(position_error)
    }
    fn relocate_owner(
        &mut self,
        _: ActorId,
        _: [aa_body::F32; 2],
    ) -> aa_actors::element_position::Result<()> {
        Err(position_error(
            "57C680 actual stored4EEB40 relocation callbacks",
        ))
    }
    fn interpolate(&mut self, _: ActorId) -> aa_actors::element_position::Result<bool> {
        Err(position_error(
            "57C080 actual stored Organization interpolation",
        ))
    }
    fn has_scheduled(
        &mut self,
        id: ActorId,
        event: u32,
        data: u32,
    ) -> aa_actors::element_position::Result<bool> {
        let key = self.key(id).map_err(position_error)?;
        let owner = self
            .staged
            .owners
            .get(crate::scheduler_owners::Target::Component {
                object: key,
                slot: Slot::Organization.index() as u8,
            })
            .map_err(position_error)?;
        let event = u8::try_from(event).map_err(position_error)?;
        Ok(self.staged.queues.has_from_owner_head(owner, event, data))
    }
    fn schedule(&mut self, _: ActorId, _: u32) -> aa_actors::element_position::Result<()> {
        Err(position_error("578E10 actual50AC00 scheduler publication"))
    }
}
