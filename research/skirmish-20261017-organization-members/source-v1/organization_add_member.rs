//! Storage-independent57CBA0. Each field loan ends before its reached child,
//! so a live adapter can keep the actual Organization installed in ActorStore.
use super::*;

pub trait AddMemberFields<D> {
    fn layout_info(&mut self) -> Result<LayoutInfo>;
    fn member_count(&mut self) -> Result<usize>;
    fn put_member(&mut self, index: usize, member: ActorId) -> Result<()>;
    fn word(&mut self, offset: u32) -> Result<u32>;
    fn put_word(&mut self, offset: u32, value: u32) -> Result<()>;
    fn element_set_organization(&mut self, member: ActorId, slot: i32) -> Result<()>;
    fn member_has_body(&mut self, member: ActorId) -> Result<bool>;
    fn body_set_organization(&mut self, member: ActorId) -> Result<()>;
    fn member_has_experience(&mut self, member: ActorId) -> Result<bool>;
    fn owner_experience_token(&mut self) -> Result<u32>;
    fn set_experience_source(&mut self, member: ActorId, token: u32) -> Result<()>;
    fn combat_value(&mut self) -> Result<()>;
    fn member_mover_speed(&mut self, member: ActorId) -> Result<Option<u32>>;
    fn owner_has_mind(&mut self) -> Result<bool>;
    fn mind_speed_changed(&mut self, old: u32, new: u32) -> Result<()>;
    fn member_flag_4d1080(&mut self, member: ActorId) -> Result<bool>;
    fn set_owner_state(&mut self, state: u8, enabled: bool) -> Result<()>;
    fn set_leader(&mut self, member: ActorId) -> Result<()>;
    fn mind_member_added(&mut self, member: ActorId) -> Result<()>;
    fn follow_leader(&mut self, member: ActorId) -> Result<()>;
    fn owner_4d4cf0(&mut self) -> Result<()>;
    fn owner_4cf0f0(&mut self) -> Result<()>;
    fn owner_4d1350(&mut self) -> Result<()>;
    fn member_added_5784c0(&mut self, member: ActorId) -> Result<()>;
    fn member_definition(&mut self, member: ActorId) -> Result<D>;
    fn definition_list_2d8(&mut self, definition: &D) -> Result<Vec<u32>>;
    fn apply_property(&mut self, property: u32, kind: u32, actor: ActorId) -> Result<()>;
    fn owner_object_10(&mut self) -> Result<Option<u32>>;
    fn owner_object_notify(&mut self, object: u32) -> Result<()>;
}

pub fn add_member_fields<D>(
    fields: &mut impl AddMemberFields<D>,
    slot: i32,
    member: ActorId,
) -> Result<()> {
    let info = fields.layout_info()?;
    let count = fields.member_count()?;
    let index = usize::try_from(slot)
        .ok()
        .filter(|&i| i < count)
        .ok_or_else(|| {
            LoadError::Unsupported(format!(
                "member slot {slot} outside the member array (native out-of-bounds store)"
            ))
        })?;
    fields.put_member(index, member)?;
    fields.element_set_organization(member, slot)?;
    if fields.member_has_body(member)? {
        fields.body_set_organization(member)?;
    }
    if !fields.member_has_experience(member)? {
        let token = fields.owner_experience_token()?;
        fields.set_experience_source(member, token)?;
    }
    fields.combat_value()?;
    if let Some(speed) = fields.member_mover_speed(member)? {
        let old = fields.word(0xd8)?;
        if compare32(speed, old) == Some(Ordering::Less) {
            fields.put_word(0xd8, speed)?;
            if fields.owner_has_mind()? {
                fields.mind_speed_changed(old, speed)?;
            }
        }
    }
    if info.primary_slot == slot {
        fields.put_word(0x44, member.raw())?;
        let flag = fields.member_flag_4d1080(member)?;
        fields.set_owner_state(0x22, flag)?;
    }
    if fields.word(0x44)? == member.raw() || fields.word(0x40)? == 0 {
        fields.set_leader(member)?;
    }
    if fields.owner_has_mind()? {
        fields.mind_member_added(member)?;
    }
    fields.follow_leader(member)?;
    fields.owner_4d4cf0()?;
    fields.owner_4cf0f0()?;
    fields.owner_4d1350()?;
    let group = *info
        .slot_groups
        .get(index)
        .ok_or_else(|| LoadError::Unsupported("slot beyond LayoutData +28".into()))?;
    let entry = info
        .entries
        .get(group as usize)
        .ok_or_else(|| LoadError::Unsupported("slot group beyond LayoutData +1C".into()))?;
    if entry.morale_line {
        let living = fields.word(0x64)?.wrapping_add(1);
        fields.put_word(0x64, living)?;
    }
    fields.member_added_5784c0(member)?;
    let definition = fields.member_definition(member)?;
    for value in fields.definition_list_2d8(&definition)? {
        fields.apply_property(value, 8, member)?;
    }
    if let Some(object) = fields.owner_object_10()? {
        fields.owner_object_notify(object)?;
    }
    let enabled = fields.word(0x40)? != 0 && fields.word(0x44)? == 0 && info.primary_slot >= 0;
    fields.set_owner_state(0x25, enabled)
}

pub(super) struct Held<'a, D, W> {
    pub org: &'a mut OrganizationState<D>,
    pub owner: &'a OwnerFacts<D>,
    pub world: &'a mut W,
}
impl<D: Clone, W: OrganizationWorld<D>> AddMemberFields<D> for Held<'_, D, W> {
    fn layout_info(&mut self) -> Result<LayoutInfo> {
        self.world.layout_info(&self.org.layout.definition)
    }
    fn member_count(&mut self) -> Result<usize> {
        Ok(self.org.members.len())
    }
    fn put_member(&mut self, i: usize, a: ActorId) -> Result<()> {
        self.org.members[i] = a;
        Ok(())
    }
    fn word(&mut self, o: u32) -> Result<u32> {
        word(self.org, o)
    }
    fn put_word(&mut self, o: u32, v: u32) -> Result<()> {
        self.org.words.insert(o, v);
        Ok(())
    }
    fn element_set_organization(&mut self, a: ActorId, s: i32) -> Result<()> {
        self.world.element_set_organization(self.org, a, s)
    }
    fn member_has_body(&mut self, a: ActorId) -> Result<bool> {
        Ok(self.world.member_facts(a)?.has_body)
    }
    fn body_set_organization(&mut self, a: ActorId) -> Result<()> {
        self.world.body_set_organization(a)
    }
    fn member_has_experience(&mut self, a: ActorId) -> Result<bool> {
        Ok(self.world.member_facts(a)?.has_experience)
    }
    fn owner_experience_token(&mut self) -> Result<u32> {
        self.world.owner_experience_token()
    }
    fn set_experience_source(&mut self, a: ActorId, t: u32) -> Result<()> {
        self.world.set_experience_source(a, t)
    }
    fn combat_value(&mut self) -> Result<()> {
        self.world.combat_value(self.org)
    }
    fn member_mover_speed(&mut self, a: ActorId) -> Result<Option<u32>> {
        Ok(self.world.member_facts(a)?.mover_speed)
    }
    fn owner_has_mind(&mut self) -> Result<bool> {
        Ok(self.owner.has_mind)
    }
    fn mind_speed_changed(&mut self, a: u32, b: u32) -> Result<()> {
        self.world.mind_speed_changed(a, b)
    }
    fn member_flag_4d1080(&mut self, a: ActorId) -> Result<bool> {
        self.world.member_flag_4d1080(a)
    }
    fn set_owner_state(&mut self, s: u8, v: bool) -> Result<()> {
        self.world.set_owner_state(s, v)
    }
    fn set_leader(&mut self, a: ActorId) -> Result<()> {
        self.world.set_leader(self.org, a)
    }
    fn mind_member_added(&mut self, a: ActorId) -> Result<()> {
        self.world.mind_member_added(a)
    }
    fn follow_leader(&mut self, a: ActorId) -> Result<()> {
        self.world.follow_leader(self.org, a)
    }
    fn owner_4d4cf0(&mut self) -> Result<()> {
        self.world.owner_4d4cf0(self.org)
    }
    fn owner_4cf0f0(&mut self) -> Result<()> {
        self.world.owner_4cf0f0(self.org)
    }
    fn owner_4d1350(&mut self) -> Result<()> {
        self.world.owner_4d1350(self.org)
    }
    fn member_added_5784c0(&mut self, a: ActorId) -> Result<()> {
        self.world.member_added_5784c0(self.org, a)
    }
    fn member_definition(&mut self, a: ActorId) -> Result<D> {
        Ok(self.world.member_facts(a)?.definition)
    }
    fn definition_list_2d8(&mut self, d: &D) -> Result<Vec<u32>> {
        self.world.definition_list_2d8(d)
    }
    fn apply_property(&mut self, p: u32, k: u32, a: ActorId) -> Result<()> {
        self.world.apply_property(p, k, a)
    }
    fn owner_object_10(&mut self) -> Result<Option<u32>> {
        Ok(self.owner.object_10)
    }
    fn owner_object_notify(&mut self, o: u32) -> Result<()> {
        self.world.owner_object_notify(o)
    }
}
