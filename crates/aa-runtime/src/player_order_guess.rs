//! KKC_GActorCommand::GuessCommandData 0x4DA570 (Source\Sim\GActorCommands.cpp).
//!
//! A command without an explicit definition (flag bit6 clear, +08 null) asks
//! this routine which ActorCommand definition it means for one actor. It is
//! reached from 4CF430 (actor vt+150, the permission used by 4ED360, 61E220
//! and UIWorldState 436950), 61F210 (first selected actor, diagnostic name),
//! 4ED360 (each selected actor; argument2 = selection has distinct head and
//! tail actors) and the group path 4DE740. The kernel below is the exact
//! instruction order of 4DA570 with its inlined helpers 4D44E0 (construct
//! candidates), 579FD0/579F70/4364C0/4D1490 (Organization member predicates)
//! and 497220/405030/96CAD6 (ActorCommand enum B3CBA0 name lookup). Every
//! leaf the native reads through a virtual, a field or a variable record is
//! a [`World`] method; replay fixtures are in
//! `research/phase8-orders-input/guess-4da570/`.
//!
//! Native details kept exactly:
//! * mask 0x10/0x20 from flags bits2/3; with flag bit0 the target GTID is
//!   resolved through B364C0 (an invalid target logs 404F60 line 0x12A and
//!   returns null before any variable is read);
//! * target vt+108 kingdom, actor vt+140 (0x2000) then vt+138 (0x1000),
//!   Organization +7C: 579FD0 (0x200) and 579F70 (0x800); otherwise
//!   Construction +9C (0x200, plus 0x400 for Actor+100 bit21) and Body +60
//!   health < maximum as an ordered binary32 compare (0x800);
//! * the UVar SwapRightClickWithAltRightClick is read on every call (only the
//!   record pointer is cached by B3CB9C bit0); Alt = flag bit10 XOR swap; only
//!   then HotkeyAltRightClickCommand is read and looked up in B3CBA0 (-1 when
//!   absent);
//! * 39 definitions in order: vt+34(i), then the hot index filter, then
//!   def+3C when argument2, then def+2C against the mask: the location branch
//!   (matched bits 4..7) needs def+40 or the hot index, the actor branch needs
//!   def+3F or the hot index; type 4 (def+0C) scans the 4D44E0 list for
//!   candidate+4E0 == target+4 and, on success, sets flags|=2 and +14; every
//!   path that does not return reaches the vt+13C(kingdom) alternatives.
//!
//! Native faults (null dereferences, unbounded recursion) are explicit
//! [`Required`] errors; no default definition is invented.
use crate::{
    actor_world::{ActorAccess, RuntimeComponent},
    catalog::Catalog,
};
use aa_actors::{ActorId, components::ComponentSlot};

/// `cmp ebx, 0x27` at 4DA982: the ActorCommand definitions at [[B36530]+298].
pub const DEFINITION_COUNT: u32 = 0x27;

/// Requirement mask bits (EBP / [esp+10] of 4DA570) tested against def+2C.
pub mod requirement {
    /// Command flag bit2 (position).
    pub const POSITION: u32 = 0x10;
    /// Command flag bit3 (auxiliary / facing).
    pub const AUXILIARY: u32 = 0x20;
    /// Command flag bit0 with a resolvable target.
    pub const TARGET: u32 = 0x100;
    /// Target Organization 579FD0 or target Construction +9C.
    pub const CONSTRUCTION: u32 = 0x200;
    /// Non-Organization target with Construction and Actor+100 bit21.
    pub const CONSTRUCTION_STATE: u32 = 0x400;
    /// Organization 579F70 or Body +10 < +14.
    pub const DAMAGED: u32 = 0x800;
    /// Actor vt+138(target kingdom).
    pub const FRIENDLY: u32 = 0x1000;
    /// Actor vt+140(target kingdom).
    pub const HOSTILE: u32 = 0x2000;
}

/// A native fault or an unbound owner reached by this routine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Required(pub String);
impl From<Required> for super::Error {
    fn from(value: Required) -> Self {
        super::Error::Required(value.0)
    }
}
fn required<T, E: From<Required>>(text: impl Into<String>) -> Result<T, E> {
    Err(Required(text.into()).into())
}

/// The definition fields 4DA570 reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Definition {
    /// +0C: native type; 4B4E60 stores the registration index here.
    pub type_0c: u32,
    /// +2C: behavior/requirement flags (4740F0 third argument).
    pub requirements_2c: u32,
    /// +3C non_group_command.
    pub non_group_3c: bool,
    /// +3F can_hot_command_actor.
    pub hot_actor_3f: bool,
    /// +40 can_hot_command_location.
    pub hot_location_40: bool,
}

/// Command words 4DA570 reads (+0C flags, +10 target GTID). +14 is only
/// written, through [`Outcome::completed_object_14`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandFields {
    pub flags_0c: u32,
    pub target_10: u32,
}

/// Settlement +98: +14 parent and the +18/+1C object vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settlement<O> {
    pub parent_14: Option<O>,
    pub children_18: Vec<Option<O>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Guessed {
    /// Index into [[B36530]+298] (the returned definition pointer).
    pub index: u32,
    pub definition: Definition,
}

/// Result of one 4DA570 call. The native return is `guessed` (null = None).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome<D> {
    pub guessed: Option<Guessed>,
    /// 4DA994: the caller's command receives flags |= 2 and +14 = this.
    pub completed_object_14: Option<D>,
    /// 4DA613 404F60 "GuessCommandData - target actor %d is invalid"
    /// (GActorCommands.cpp line 0x12A): the unresolved +10 handle.
    pub invalid_target_logged: Option<u32>,
}
impl<D> Outcome<D> {
    /// The two 4DA994 stores, applied to a caller-owned flags word and +14.
    pub fn apply(self, flags_0c: &mut u32, object_14: &mut Option<D>) -> Option<Guessed> {
        if let Some(object) = self.completed_object_14 {
            *flags_0c |= 2;
            *object_14 = Some(object);
        }
        self.guessed
    }
}

/// Every native read of 4DA570 and its inlined helpers. Objects are any
/// B364C0 objects (target, actor, members, settlement children).
pub trait World {
    type Object: Copy + Eq;
    type Organization: Copy;
    type Kingdom: Copy;
    type Definition: Clone + Eq;
    type Error: From<Required>;
    /// B364C0: generation word table+4+2*slot, then object table+20004+4*slot
    /// (None for a different generation or a null object).
    fn resolve_10(&mut self, handle: u32) -> Result<Option<Self::Object>, Self::Error>;
    /// Target vt+108 (null kingdom = None).
    fn kingdom_108(&mut self, target: Self::Object) -> Result<Option<Self::Kingdom>, Self::Error>;
    /// Actor vt+140(kingdom), AL.
    fn hostile_140(&mut self, actor: Self::Object, kingdom: Option<Self::Kingdom>) -> Result<bool, Self::Error>;
    /// Actor vt+138(kingdom), AL.
    fn friendly_138(&mut self, actor: Self::Object, kingdom: Option<Self::Kingdom>) -> Result<bool, Self::Error>;
    /// Actor vt+13C(kingdom), AL; only reached with a non-null kingdom.
    fn neutral_13c(&mut self, actor: Self::Object, kingdom: Self::Kingdom) -> Result<bool, Self::Error>;
    /// Object +7C Organization component.
    fn organization_7c(&mut self, object: Self::Object) -> Result<Option<Self::Organization>, Self::Error>;
    /// Organization +28 member array of count +2C, null entries kept.
    fn members_28(&mut self, organization: Self::Organization) -> Result<Vec<Option<Self::Object>>, Self::Error>;
    /// Byte [[member+80]+1C]; None when the +80 Element component is null.
    fn element_flag_1c(&mut self, member: Self::Object) -> Result<Option<bool>, Self::Error>;
    /// Object +9C (Construction component) non-null.
    fn construction_9c(&mut self, object: Self::Object) -> Result<bool, Self::Error>;
    /// Actor+100 state word.
    fn state_100(&mut self, object: Self::Object) -> Result<u32, Self::Error>;
    /// Body +60 component: binary32 bits of +10 (health) and +14 (maximum).
    fn body_60(&mut self, object: Self::Object) -> Result<Option<[u32; 2]>, Self::Error>;
    /// 4BE150: byte at the SwapRightClickWithAltRightClick record, != 0.
    fn swap_right_click_4be150(&mut self) -> Result<bool, Self::Error>;
    /// [[B3CB94]+10]: the HotkeyAltRightClickCommand UTF-16 text.
    fn hotkey_alt_right_click(&mut self) -> Result<Vec<u16>, Self::Error>;
    /// Actor vt+34(index), AL.
    fn allowed_34(&mut self, actor: Self::Object, index: u32) -> Result<bool, Self::Error>;
    /// [[B36530]+298][index].
    fn definition(&mut self, index: u32) -> Result<Definition, Self::Error>;
    /// Object +A8 (Builder component) non-null.
    fn builder_a8(&mut self, object: Self::Object) -> Result<bool, Self::Error>;
    /// [[object+4]+504] values, head first.
    fn builder_targets_504(&mut self, object: Self::Object) -> Result<Vec<Self::Definition>, Self::Error>;
    /// Object +98 Settlement component.
    fn settlement_98(&mut self, object: Self::Object) -> Result<Option<Settlement<Self::Object>>, Self::Error>;
    /// Object +4 definition.
    fn definition_4(&mut self, object: Self::Object) -> Result<Option<Self::Definition>, Self::Error>;
    /// Definition +4E0.
    fn required_object_4e0(&mut self, definition: &Self::Definition) -> Result<Option<Self::Definition>, Self::Error>;
}

/// 497220 on the B3CBA0 enum built by 4DA9D0(1)/4DA2F0: the value of the
/// first registered name equal to `text` (up to its first NUL) under the
/// CRT "C" locale wcsicmp (only 'A'..'Z' fold). Names and values are the
/// original 4DA2F0 registrations (aa-rules code-predefined table).
pub fn hot_command_497220(text: &[u16]) -> Option<u32> {
    let end = text.iter().position(|&c| c == 0).unwrap_or(text.len());
    let text = &text[..end];
    let fold = |c: u16| if (0x41..=0x5a).contains(&c) { c + 0x20 } else { c };
    aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS
        .iter()
        .find(|d| {
            let name: Vec<u16> = d.id.encode_utf16().collect();
            name.len() == text.len() && name.iter().zip(text).all(|(&a, &b)| fold(a) == fold(b))
        })
        .map(|d| d.mind_type)
}

/// 579FD0: any non-null Organization member with a +9C Construction.
pub fn organization_construction_579fd0<W: World + ?Sized>(
    world: &mut W,
    organization: W::Organization,
) -> Result<bool, W::Error> {
    for member in world.members_28(organization)?.into_iter().flatten() {
        if world.construction_9c(member)? {
            return Ok(true);
        }
    }
    Ok(false)
}
/// 4D1490: Body present, health < maximum (ordered), Actor+100 bit21 clear.
pub fn damaged_4d1490<W: World + ?Sized>(world: &mut W, object: W::Object) -> Result<bool, W::Error> {
    let Some([health, maximum]) = world.body_60(object)? else {
        return Ok(false);
    };
    if !(f32::from_bits(health) < f32::from_bits(maximum)) {
        return Ok(false);
    }
    Ok(world.state_100(object)? & 0x0020_0000 == 0)
}
/// 579F70: any non-null Organization member satisfying 4D1490.
pub fn organization_damaged_579f70<W: World + ?Sized>(
    world: &mut W,
    organization: W::Organization,
) -> Result<bool, W::Error> {
    for member in world.members_28(organization)?.into_iter().flatten() {
        if damaged_4d1490(world, member)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// 4D44E0(object, &list): the head-first construct candidate list. Builder
/// targets are prepended without deduplication; filtered Organization
/// members (4364C0 with [[member+80]+1C]), Settlement+18 children and the
/// Settlement+14 parent recurse and prepend values not already present.
pub fn candidates_4d44e0<W: World + ?Sized>(
    world: &mut W,
    object: W::Object,
) -> Result<Vec<W::Definition>, W::Error> {
    let mut active = Vec::new();
    candidates(world, object, &mut active)
}
fn merge<D: Clone + Eq>(out: &mut Vec<D>, values: Vec<D>) {
    for value in values {
        if !out.contains(&value) {
            out.insert(0, value);
        }
    }
}
fn candidates<W: World + ?Sized>(
    world: &mut W,
    object: W::Object,
    active: &mut Vec<W::Object>,
) -> Result<Vec<W::Definition>, W::Error> {
    if active.contains(&object) {
        return required("4D44E0 recursion revisits an active object (native recursion is unbounded)");
    }
    active.push(object);
    let mut out = Vec::new();
    if world.builder_a8(object)? {
        for value in world.builder_targets_504(object)? {
            out.insert(0, value);
        }
    }
    if let Some(organization) = world.organization_7c(object)? {
        for member in world.members_28(organization)?.into_iter().flatten() {
            match world.element_flag_1c(member)? {
                None => return required("4364E6 filtered Organization member has a null +80 Element (native [null+1C])"),
                Some(false) => continue,
                Some(true) => {}
            }
            let values = candidates(world, member, active)?;
            merge(&mut out, values);
        }
    }
    if let Some(settlement) = world.settlement_98(object)? {
        for child in settlement.children_18 {
            let Some(child) = child else {
                return required("4D46E3 null Settlement+18 child (native 4D44E0 this=null)");
            };
            let values = candidates(world, child, active)?;
            merge(&mut out, values);
        }
        if let Some(parent) = settlement.parent_14 {
            let values = candidates(world, parent, active)?;
            merge(&mut out, values);
        }
    }
    active.pop();
    Ok(out)
}

/// KKC_GActorCommand::GuessCommandData 0x4DA570(this=command, actor,
/// non_group_excluded). See the module documentation for the exact order.
pub fn guess_command_data<W: World + ?Sized>(
    world: &mut W,
    command: CommandFields,
    actor: W::Object,
    non_group_excluded: bool,
) -> Result<Outcome<W::Definition>, W::Error> {
    let flags = command.flags_0c;
    let mut mask = 0;
    if (flags >> 2) & 1 != 0 {
        mask = requirement::POSITION;
    }
    if (flags >> 3) & 1 != 0 {
        mask |= requirement::AUXILIARY;
    }
    let mut kingdom = None;
    if flags & 1 != 0 {
        let Some(target) = world.resolve_10(command.target_10)? else {
            return Ok(Outcome { guessed: None, completed_object_14: None, invalid_target_logged: Some(command.target_10) });
        };
        let target_kingdom = world.kingdom_108(target)?;
        mask |= requirement::TARGET;
        kingdom = target_kingdom;
        if world.hostile_140(actor, target_kingdom)? {
            mask |= requirement::HOSTILE;
        }
        if world.friendly_138(actor, target_kingdom)? {
            mask |= requirement::FRIENDLY;
        }
        if let Some(organization) = world.organization_7c(target)? {
            if organization_construction_579fd0(world, organization)? {
                mask |= requirement::CONSTRUCTION;
            }
            if organization_damaged_579f70(world, organization)? {
                mask |= requirement::DAMAGED;
            }
        } else {
            if world.construction_9c(target)? {
                mask |= requirement::CONSTRUCTION;
                if world.state_100(target)? & 0x0020_0000 != 0 {
                    mask |= requirement::CONSTRUCTION_STATE;
                }
            }
            if let Some([health, maximum]) = world.body_60(target)? {
                if f32::from_bits(health) < f32::from_bits(maximum) {
                    mask |= requirement::DAMAGED;
                }
            }
        }
    }
    let swap = world.swap_right_click_4be150()?;
    // 4DA777 rereads +0C; 4DA77C..4DA786 Alt = bit10 XOR swap.
    let alt = ((command.flags_0c >> 10) & 1 != 0) != swap;
    let mut hot: i32 = -1;
    if alt {
        let text = world.hotkey_alt_right_click()?;
        hot = hot_command_497220(&text).map_or(-1, |v| v as i32);
    }
    for index in 0..DEFINITION_COUNT {
        if !world.allowed_34(actor, index)? {
            continue;
        }
        if hot >= 0 && index as i32 != hot {
            continue;
        }
        let definition = world.definition(index)?;
        if non_group_excluded && definition.non_group_3c {
            continue;
        }
        let requirements = definition.requirements_2c;
        let matched = requirements & mask;
        let hot_index = hot == index as i32;
        let found = if matched & 0xf0 != 0 {
            let wanted = requirements & 0xffff_00ff;
            (definition.hot_location_40 || hot_index) && matched == wanted
        } else {
            let wanted = requirements & 0xffff_ff0f;
            if !definition.hot_actor_3f && !hot_index {
                false
            } else if definition.type_0c != 4 {
                matched == wanted
            } else {
                let list = candidates_4d44e0(world, actor)?;
                let Some(target) = world.resolve_10(command.target_10)? else {
                    return required("4DA8EB type-4 target re-resolution is null (native [null+4])");
                };
                let target_definition = world.definition_4(target)?;
                for value in list {
                    if world.required_object_4e0(&value)? == target_definition
                        && (mask | 0x50000) & wanted == wanted
                    {
                        return Ok(Outcome {
                            guessed: Some(Guessed { index, definition }),
                            completed_object_14: Some(value),
                            invalid_target_logged: None,
                        });
                    }
                }
                false
            }
        };
        if found {
            return Ok(Outcome { guessed: Some(Guessed { index, definition }), completed_object_14: None, invalid_target_logged: None });
        }
        if let Some(kingdom) = kingdom
            && world.neutral_13c(actor, kingdom)?
            && (matched | requirement::FRIENDLY == mask || matched | requirement::HOSTILE == mask)
        {
            return Ok(Outcome { guessed: Some(Guessed { index, definition }), completed_object_14: None, invalid_target_logged: None });
        }
    }
    Ok(Outcome { guessed: None, completed_object_14: None, invalid_target_logged: None })
}

// ---------------------------------------------------------------------------
// Live binding over the same owners as the permission code.

/// [`World`] over the actual ActorStore, canonical ActorCommand definitions
/// and the same Bank used by the permission code.
/// The two UVars come from [`super::Globals`] (default: required owner).
pub(crate) struct Live<'a, 'src, A: ?Sized, G: ?Sized> {
    pub catalog: &'a Catalog<'src>,
    pub banks: &'a dyn super::Banks,
    pub actors: &'a A,
    pub globals: &'a mut G,
}
impl<A: super::ActorRead + ?Sized, G: ?Sized> Live<'_, '_, A, G> {
    fn class(&self, id: ActorId) -> super::Result<u32> {
        let key = self.actors.key(id).ok_or_else(|| super::Error::Required("4DA570 current object generation".into()))?;
        Ok(super::check(self.actors.object(key))?.definition.native_class)
    }
    fn ordinary(&self, id: ActorId, leaf: &str) -> super::Result<&crate::actor_world::RuntimeObject> {
        let class = self.class(id)?;
        if !matches!(class, 3 | 4) {
            return required(format!("4DA570 {leaf} on native class {class} (only ordinary Actor/Kingdom layouts are bound)"));
        }
        super::object_with_bank(self.catalog, self.banks, self.actors, id)
    }
    fn component(&self, id: ActorId, slot: ComponentSlot, leaf: &str) -> super::Result<Option<&RuntimeComponent>> {
        Ok(self.ordinary(id, leaf)?.components().map_err(|e| super::Error::Required(e.to_string()))?.get(slot))
    }
    fn relation(&self, actor: ActorId, kingdom: Option<ActorId>) -> super::Result<u32> {
        let read = Relations { catalog: self.catalog, banks: self.banks, actors: self.actors };
        match self.class(actor)? {
            3 => {
                let owner = super::actor_word_with_bank(self.catalog, self.banks, self.actors, actor, 0xe8)?;
                crate::relations::borrowed::actor(&read, (owner != 0).then_some(ActorId::from_raw(owner)), kingdom)
            }
            4 => {
                self.ordinary(actor, "4E52D0 Kingdom vt+144")?;
                crate::relations::borrowed::kingdom(&read, actor, kingdom)
            }
            class => required(format!("4DA570 actor relation virtual on native class {class}")),
        }
    }
    fn precision(&self) -> super::Result<aa_numeric::Precision> {
        let bank = self.banks.borrow_bank()?;
        let source = bank.combat_source().map_err(super::Error::Required)?;
        if source.source_identities() != self.catalog.actors.sources.as_slice() {
            return required("4DA570 ActorCommand source Bank FP owner differs from Catalog");
        }
        Ok(source.source_precision())
    }
}
impl<A: super::ActorRead + ?Sized, G: super::Globals + ?Sized> World for Live<'_, '_, A, G> {
    type Object = ActorId;
    type Organization = ActorId;
    type Kingdom = ActorId;
    type Definition = String;
    type Error = super::Error;
    fn resolve_10(&mut self, handle: u32) -> super::Result<Option<ActorId>> {
        Ok(self.actors.identities().get(ActorId::from_raw(handle)).map(|e| e.id))
    }
    fn kingdom_108(&mut self, target: ActorId) -> super::Result<Option<ActorId>> {
        self.ordinary(target, "target vt+108 and +7C/+9C/+100/+60 reads")?;
        let raw = super::actor_word_with_bank(self.catalog, self.banks, self.actors, target, 0xe8)?;
        Ok((raw != 0).then_some(ActorId::from_raw(raw)))
    }
    fn hostile_140(&mut self, actor: ActorId, kingdom: Option<ActorId>) -> super::Result<bool> {
        // 4E52A0: vt+144 == 3.
        Ok(self.relation(actor, kingdom)? == 3)
    }
    fn friendly_138(&mut self, actor: ActorId, kingdom: Option<ActorId>) -> super::Result<bool> {
        // Actor 4D1EB0: Actor+E8 == kingdom (null == null) or vt+144 == 1.
        // Kingdom 4E5250: this == kingdom or vt+144 == 1.
        let same = match self.class(actor)? {
            3 => {
                let owner = super::actor_word_with_bank(self.catalog, self.banks, self.actors, actor, 0xe8)?;
                (owner != 0).then_some(ActorId::from_raw(owner)) == kingdom
            }
            4 => Some(actor) == kingdom,
            class => return required(format!("4DA570 actor vt+138 on native class {class}")),
        };
        Ok(same || self.relation(actor, kingdom)? == 1)
    }
    fn neutral_13c(&mut self, actor: ActorId, kingdom: ActorId) -> super::Result<bool> {
        // 4E5280: vt+144 == 2.
        Ok(self.relation(actor, Some(kingdom))? == 2)
    }
    fn organization_7c(&mut self, object: ActorId) -> super::Result<Option<ActorId>> {
        match self.component(object, ComponentSlot::Organization, "+7C Organization")? {
            None => Ok(None),
            Some(RuntimeComponent::Organization(o)) if o.header.owner == object => Ok(Some(object)),
            Some(_) => required("4DA682 actual Organization slot payload/owner"),
        }
    }
    fn members_28(&mut self, organization: ActorId) -> super::Result<Vec<Option<ActorId>>> {
        match self.component(organization, ComponentSlot::Organization, "Organization+28 members")? {
            Some(RuntimeComponent::Organization(o)) if o.header.owner == organization => {
                Ok(o.members.iter().map(|m| (!m.is_null()).then_some(*m)).collect())
            }
            _ => required("579FD0/4364C0 actual Organization+28/+2C member array"),
        }
    }
    fn element_flag_1c(&mut self, member: ActorId) -> super::Result<Option<bool>> {
        match self.component(member, ComponentSlot::Element, "+80 Element")? {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == member => Ok(Some(e.flag_1c)),
            Some(_) => required("4364E6 actual Element slot payload/owner"),
        }
    }
    fn construction_9c(&mut self, object: ActorId) -> super::Result<bool> {
        Ok(self.component(object, ComponentSlot::Construction, "+9C Construction")?.is_some())
    }
    fn state_100(&mut self, object: ActorId) -> super::Result<u32> {
        Ok(self.ordinary(object, "Actor+100")?.components().map_err(|e| super::Error::Required(e.to_string()))?.state.low)
    }
    fn body_60(&mut self, object: ActorId) -> super::Result<Option<[u32; 2]>> {
        match self.component(object, ComponentSlot::Body, "+60 Body")? {
            None => Ok(None),
            Some(RuntimeComponent::Body(b)) if b.header.owner == object => Ok(Some([b.body.health.0, b.body.maximum.0])),
            Some(_) => required("4DA6DA actual Body slot payload/owner"),
        }
    }
    fn swap_right_click_4be150(&mut self) -> super::Result<bool> {
        self.globals.swap_right_click_with_alt_right_click_4be150()
    }
    fn hotkey_alt_right_click(&mut self) -> super::Result<Vec<u16>> {
        self.globals.hotkey_alt_right_click_command_4b2e60()
    }
    fn allowed_34(&mut self, actor: ActorId, index: u32) -> super::Result<bool> {
        self.ordinary(actor, "actor vt+34")?;
        super::allows_with_bank(self.catalog, self.banks, self.actors, actor, index)
    }
    fn definition(&mut self, index: u32) -> super::Result<Definition> {
        let d = aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS
            .get(index as usize)
            .copied()
            .ok_or_else(|| super::Error::Required("[[B36530]+298] definition index".into()))?;
        // 4B4E60 replaced +0C by the registration index (warning on mismatch).
        if d.mind_type != index {
            return required("4B4E60 registration index differs from the predefined type");
        }
        let controls = controls_in_bank(self.catalog, d, self.precision()?)?;
        Ok(Definition {
            type_0c: index,
            requirements_2c: d.behavior_flags,
            non_group_3c: controls.non_group_command(),
            hot_actor_3f: controls.can_hot_command_actor(),
            hot_location_40: controls.can_hot_command_location(),
        })
    }
    fn builder_a8(&mut self, object: ActorId) -> super::Result<bool> {
        Ok(self.component(object, ComponentSlot::Builder, "+A8 Builder")?.is_some())
    }
    fn builder_targets_504(&mut self, object: ActorId) -> super::Result<Vec<String>> {
        let id = self.ordinary(object, "ActorData+504")?.thing.definition.id.clone();
        let source = self.catalog.native_builder_data(self.precision()?).map_err(super::Error::Required)?;
        Ok(super::check(source.definition(&id))?.ids.clone())
    }
    fn settlement_98(&mut self, object: ActorId) -> super::Result<Option<Settlement<ActorId>>> {
        match self.component(object, ComponentSlot::Settlement, "+98 Settlement")? {
            None => Ok(None),
            Some(_) => required("4D44E0 actual Settlement+14/+18 owner"),
        }
    }
    fn definition_4(&mut self, object: ActorId) -> super::Result<Option<String>> {
        let key = self.actors.key(object).ok_or_else(|| super::Error::Required("4DA8EB current target generation".into()))?;
        Ok(Some(super::check(self.actors.object(key))?.thing.definition.id.clone()))
    }
    fn required_object_4e0(&mut self, _: &String) -> super::Result<Option<String>> {
        required("4DA8F4 ActorData+4E0 completed source-field owner (NativeSourceRequirements) on the order path")
    }
}

/// The 48CF0A..48CF97 controls of one predefined command, bound in the
/// supplied Bank (the moved Frame Bank or the Catalog Bank), with the same
/// actor_command_ui getter/store order.
fn controls_in_bank(
    catalog: &Catalog<'_>,
    definition: aa_rules::native::actor_command::ActorCommandDefinition,
    precision: aa_numeric::Precision,
) -> super::Result<crate::actor_command_ui::Controls> {
    use aa_rules::native::TypedValue;
    super::check(catalog.require_source_owner())?;
    let registry = catalog
        .actors
        .native_data_registry("ActorCommand")
        .ok_or_else(|| super::Error::Required("actual ActorCommand manager".into()))?;
    if registry.mode != Some(2) {
        return required("ActorCommand manager mode differs from native47F7C0");
    }
    let entry = catalog
        .actors
        .native_data_find("ActorCommand", definition.id)
        .ok_or_else(|| super::Error::Required("ActorCommand authored UI root".into()))?;
    if !entry.ids.eq_ignore_ascii_case(definition.id) {
        return required("canonical ActorCommand identity is shadowed by translation");
    }
    let node = super::check(catalog.actors.native_data_node(entry))?;
    match super::check(node.field("IDS", precision))? {
        Some(TypedValue::String { value, .. }) if value.eq_ignore_ascii_case(definition.id) => {}
        _ => return required("ActorCommand node IDS differs from canonical registration"),
    }
    if node.sources().first() != Some(&entry.source) {
        return required("ActorCommand registered source/node provenance differs");
    }
    struct Node<'a> {
        node: aa_rules::actors::ResolvedNode<'a>,
        precision: aa_numeric::Precision,
    }
    impl crate::actor_command_ui::Fields for Node<'_> {
        fn boolean(&mut self, name: &'static str) -> crate::actor_command_ui::Result<bool> {
            self.node.field(name, self.precision).map_err(|e| e.to_string())?
                .ok_or_else(|| format!("ActorCommand node does not contain {name}"))?
                .boolean().map_err(|e| e.to_string())
        }
        fn integer(&mut self, name: &'static str) -> crate::actor_command_ui::Result<i32> {
            self.node.field(name, self.precision).map_err(|e| e.to_string())?
                .ok_or_else(|| format!("ActorCommand node does not contain {name}"))?
                .integer().map_err(|e| e.to_string())
        }
    }
    let mut state = crate::actor_command_ui::LoadState::default();
    crate::actor_command_ui::load_fields(&mut state, &mut Node { node, precision }, definition.mind_type, definition.behavior_flags)
        .map_err(super::Error::Required)?;
    state.completed().map_err(super::Error::Required)
}

/// 4CF040/4E52D0/4E10F0 reads over the live Kingdom/Team derived fields.
struct Relations<'a, 'src, A: ?Sized> {
    catalog: &'a Catalog<'src>,
    banks: &'a dyn super::Banks,
    actors: &'a A,
}
impl<A: super::ActorRead + ?Sized> Relations<'_, '_, A> {
    fn special(&self, id: ActorId, class: u32) -> super::Result<&aa_mapworld::special_state::SpecialShell> {
        let key = self.actors.key(id).ok_or_else(|| super::Error::Required("4DA570 relation current Kingdom/Team generation".into()))?;
        let entity = super::check(self.actors.entity(key))?;
        let runtime = entity.runtime.as_ref().ok_or_else(|| super::Error::Required("4DA570 relation runtime".into()))?;
        let d = &runtime.definition;
        if d.native_class != class
            || d.proof.category.runtime_class != class
            || !crate::load_spatial::same_proof(&runtime.thing.definition, &d.proof)
            || (class == 4 && !crate::load_spatial::ordinary_layout(class, runtime.kind, runtime.object.is_some(), runtime.actor.is_some()))
            || (class == 5 && (runtime.kind != aa_actors::loading::LoadKind::Thing || runtime.object.is_some() || runtime.actor.is_some()))
        {
            return required("4DA570 relation actual Kingdom/Team class/layout");
        }
        if class == 4 {
            super::object_with_bank(self.catalog, self.banks, self.actors, id)?;
        }
        let crate::actor_world::Source::Special(special) = &entity.source else {
            return required("4DA570 relation actual derived Kingdom/Team owner");
        };
        if !matches!((&special.source, class),
            (crate::actor_world::SpecialSource::Kingdom(_), 4) | (crate::actor_world::SpecialSource::Team(_), 5)) {
            return required("4DA570 relation actual source kind");
        }
        special.derived.as_ref().ok_or_else(|| super::Error::Required("4DA570 relation derived fields".into()))
    }
    fn word(shell: &aa_mapworld::special_state::SpecialShell, offset: u32) -> super::Result<u32> {
        shell.words.get(&offset).copied().ok_or_else(|| super::Error::Required(format!("4DA570 relation +{offset:X} word")))
    }
    fn vector(shell: &aa_mapworld::special_state::SpecialShell, offset: u32, slot: u32) -> super::Result<u32> {
        shell.vectors.get(&offset).and_then(|v| v.values.get(slot as usize)).copied()
            .ok_or_else(|| super::Error::Required(format!("4DA570 relation +{offset:X} vector slot {slot}")))
    }
}
impl<A: super::ActorRead + ?Sized> crate::relations::borrowed::Read for Relations<'_, '_, A> {
    type Id = ActorId;
    type Error = super::Error;
    fn kingdom_team(&self, id: ActorId) -> super::Result<Option<ActorId>> {
        let shell = self.special(id, 4)?;
        match shell.identity_references.get(&0x1ec) {
            Some(team) => Ok(*team),
            None if shell.words.get(&0x1ec) == Some(&0) => Ok(None),
            None => required("4E52EB actual Kingdom+1EC Team reference"),
        }
    }
    fn kingdom_slot(&self, id: ActorId) -> super::Result<u32> {
        Self::word(self.special(id, 4)?, 0x188)
    }
    fn team_slot(&self, id: ActorId) -> super::Result<u32> {
        Self::word(self.special(id, 5)?, 0x38)
    }
    fn kingdom_word(&self, id: ActorId, team: bool, slot: u32) -> super::Result<u32> {
        Self::vector(self.special(id, 4)?, if team { 0x1dc } else { 0x1d0 }, slot)
    }
    fn team_word(&self, id: ActorId, team: bool, slot: u32) -> super::Result<u32> {
        Self::vector(self.special(id, 5)?, if team { 0x58 } else { 0x4c }, slot)
    }
    fn orphan_relation(&self) -> super::Result<u32> {
        required("4CF069/4E52E3 current mutable global B28158 owner")
    }
    fn team_null_relation(&self) -> super::Result<u32> {
        required("4E10F0 current mutable global B2815C owner")
    }
}

/// 4DA570 over one live PlainCommand: the 4DA994 completion needs a
/// command+14 Object-definition token (B39F58 registry), which has no
/// portable owner yet, so a completion is an explicit error.
pub(crate) fn guess_plain_with_bank<A: super::ActorRead + ?Sized, G: super::Globals + ?Sized>(
    catalog: &Catalog<'_>,
    banks: &dyn super::Banks,
    actors: &A,
    globals: &mut G,
    command: &mut crate::mind_world::PlainCommand,
    actor: ActorId,
    non_group_excluded: bool,
) -> super::Result<Option<Guessed>> {
    let outcome = guess_command_data(
        &mut Live { catalog, banks, actors, globals },
        CommandFields { flags_0c: command.flags, target_10: command.word10 },
        actor,
        non_group_excluded,
    )?;
    if outcome.completed_object_14.is_some() {
        return required("4DA994 command+14 Object-definition token (B39F58 registry) for the type-4 completion");
    }
    Ok(outcome.guessed)
}

/// Public 4DA570 over a PlainCommand and the actual moved Bank loan
/// (UIWorldState 436950 / 61E150 callers use the same kernel).
pub fn guess_plain_in_bank<G: super::Globals + ?Sized>(
    catalog: &Catalog<'_>,
    bank: &super::DefinitionBankLoan<'_>,
    actors: &ActorAccess<'_>,
    globals: &mut G,
    command: &mut crate::mind_world::PlainCommand,
    actor: ActorId,
    non_group_excluded: bool,
) -> super::Result<Option<Guessed>> {
    guess_plain_with_bank(catalog, bank, actors, globals, command, actor, non_group_excluded)
}

/// Same algorithm over the UI wire (0x620760 command, CommandWire layout).
pub fn guess_wire_in_bank<G: super::Globals + ?Sized>(
    catalog: &Catalog<'_>,
    bank: &super::DefinitionBankLoan<'_>,
    actors: &ActorAccess<'_>,
    globals: &mut G,
    wire: &aa_orders::commands::CommandWire,
    actor: ActorId,
    non_group_excluded: bool,
) -> super::Result<Option<Guessed>> {
    let outcome = guess_command_data(
        &mut Live { catalog, banks: bank, actors, globals },
        CommandFields { flags_0c: u32::from(wire.flags), target_10: wire.word },
        actor,
        non_group_excluded,
    )?;
    if outcome.completed_object_14.is_some() {
        return required("4DA994 command+14 Object-definition token (B39F58 registry) for the type-4 completion");
    }
    Ok(outcome.guessed)
}

/// [Kingdom+188] (the kingdom number 61F210 prints as `K%d`) over the same
/// live Kingdom derived fields the 4E52D0 relation reads use.
pub(crate) fn kingdom_slot_188<A: super::ActorRead + ?Sized>(
    catalog: &Catalog<'_>,
    banks: &dyn super::Banks,
    actors: &A,
    kingdom: ActorId,
) -> super::Result<u32> {
    use crate::relations::borrowed::Read as _;
    Relations { catalog, banks, actors }.kingdom_slot(kingdom)
}

#[cfg(test)]
#[path = "player_order_guess_tests.rs"]
mod tests;
