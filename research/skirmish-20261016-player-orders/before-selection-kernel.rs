//! Native actor authorization and command delivery over the actual live owners.
//! Source selection is supplied by the real player's VWorldState. Commands
//! enter52C320 in the same Mind/command/queue arenas; positions are never moved
//! directly by this adapter.
use crate::{
    actor_world::{ActorAccess, RuntimeComponent, RuntimeObject, Spatial},
    catalog::Catalog,
    live_object_init::Source,
    mind_world::{self, PlainCommand},
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_mind::{Arena, MindId};
use aa_orders::commands::{self, CommandDelivery, TryCommandRouting};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("player order requires {0}")]
    Required(String),
    #[error(transparent)]
    Order(#[from] aa_orders::Error),
    #[error(transparent)]
    Mind(#[from] aa_mind::Error),
}
pub type Result<T> = std::result::Result<T, Error>;
fn required<T>(s: impl Into<String>) -> Result<T> {
    Err(Error::Required(s.into()))
}
fn check<T>(r: std::result::Result<T, impl std::fmt::Display>) -> Result<T> {
    r.map_err(|e| Error::Required(e.to_string()))
}
/// Actual40EAA0 and76AC90 are deliberately separate reached reads. A missing
/// registered application variable cannot become true from an inferred mode.
pub trait Globals {
    fn can_process_40eaa0(&mut self) -> Result<bool>;
    fn session_phase_76ac90(&mut self) -> Result<u32>;
}
fn object<'a>(
    catalog: &Catalog<'_>,
    actors: &'a ActorAccess<'_>,
    actor: ActorId,
) -> Result<&'a RuntimeObject> {
    let key = actors
        .key(actor)
        .ok_or_else(|| Error::Required("current actor generation".into()))?;
    let o = check(actors.object(key))?;
    if !matches!(o.definition.native_class, 3 | 4) {
        return required("actual ordinary Actor/Kingdom virtual dispatch");
    }
    check(Source::bind(catalog, &o.definition))?;
    if !crate::load_spatial::same_proof(&o.thing.definition, &o.definition.proof) {
        return required("actual actor shell/source identity agreement");
    }
    Ok(o)
}
fn actor_word(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    actor: ActorId,
    offset: u32,
) -> Result<u32> {
    object(catalog, actors, actor)?
        .actor
        .as_ref()
        .and_then(|a| a.words.get(&offset))
        .copied()
        .ok_or_else(|| Error::Required(format!("actual Actor+{offset:X} word")))
}
fn definition(
    catalog: &Catalog<'_>,
    command: &PlainCommand,
) -> Result<aa_rules::native::actor_command::ActorCommandDefinition> {
    let bank = check(catalog.definition_bank.try_borrow())?;
    let entry = check(bank.require_usable(command.definition_id))?;
    if entry.key.domain != "actorcommand" {
        return required("canonical ActorCommand definition domain");
    }
    let d = check(aa_rules::native::actor_command::predefined_actor_command(
        &entry.key.name,
    ))?;
    let projected = aa_mind::command::Definition {
        component_slot: d.component_slot,
        mind_type: d.mind_type,
        behavior_flags: d.behavior_flags,
    };
    if command.definition != Some(projected) {
        return required("canonical command type/component/behavior identity");
    }
    Ok(d)
}
///4D1F90/96D710 reads CL; shifts64..255 return zero,256 wraps to zero.
pub fn native_command_bit(bits: u64, index: u32) -> bool {
    let shift = index as u8;
    let mask = if shift < 64 { 1u64 << shift } else { 0 };
    bits & mask != 0
}
fn allows(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    actor: ActorId,
    index: u32,
) -> Result<bool> {
    let low = actor_word(catalog, actors, actor, 0xf0)?;
    let high = actor_word(catalog, actors, actor, 0xf4)?;
    Ok(native_command_bit(
        u64::from(low) | (u64::from(high) << 32),
        index,
    ))
}
/// Actual4CF120 ordinary virtual+20. The Element+10 owning Actor tail-call
/// retains the caller's byte argument. A malformed recursion cycle is an error,
/// not an invented false eligibility result.
pub fn eligible(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    argument: u8,
    globals: &mut impl Globals,
) -> Result<bool> {
    let mut current = actor;
    let mut seen = Vec::new();
    loop {
        if seen.contains(&current) {
            return required("finite4CF120 Element-owner chain (native would loop)");
        }
        seen.push(current);
        let o = object(catalog, actors, current)?;
        let key = actors
            .key(current)
            .ok_or_else(|| Error::Required("current actor generation".into()))?;
        if check(actors.placement(key, spatial))?.flags & (1 << 2) != 0 {
            return Ok(false);
        }
        let a = o
            .actor
            .as_ref()
            .ok_or_else(|| Error::Required("actual Actor components".into()))?;
        if let Some(component) = a.components.get(ComponentSlot::Element) {
            let RuntimeComponent::Element(e) = component else {
                return required("actual Element slot payload");
            };
            if e.header.owner != current {
                return required("actual Element owner identity");
            }
            if let Some(parent) = e.organization {
                current = parent;
                continue;
            }
        }
        if argument != 0 && globals.can_process_40eaa0()? && globals.session_phase_76ac90()? == 7 {
            return Ok(true);
        }
        return Ok(!a.components.present(ComponentSlot::Mover)
            || a.components.present(ComponentSlot::MobileDeployment)
            || a.components.present(ComponentSlot::Recruit)
            || a.components.present(ComponentSlot::Denizen));
    }
}
/// Concrete selected-command virtual+150 ->4CE680. Explicit commands have
/// flag6 and a canonical definition; unresolved default4DA570 remains required.
/// Kingdom is the actual scoped VWorldState+0 owner, never a map-header label.
pub fn authorize_selected(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    kingdom: Option<ActorId>,
    command: &PlainCommand,
    globals: &mut impl Globals,
) -> Result<bool> {
    if command.flags & (1 << 6) == 0 {
        return required("4DA570 actual default command selection");
    }
    let d = definition(catalog, command)?;
    let owner = ActorId::from_raw(actor_word(catalog, actors, actor, 0xe8)?);
    if owner.is_null() || Some(owner) != kingdom {
        return Ok(false);
    }
    if !eligible(catalog, actors, spatial, actor, 1, globals)? {
        return Ok(false);
    }
    allows(catalog, actors, actor, d.mind_type)
}
/// Original4D9E20/4CA760 fields. Inputs are raw caller words; no target/path,
/// direction, speed or Shift policy is invented by this constructor adapter.
pub enum Input {
    Move {
        destination: [u32; 2],
        auxiliary: [u32; 2],
        speed: u32,
        trailing: u8,
    },
    Attack {
        target: ActorId,
        trailing: u8,
    },
}
pub fn allocate(
    catalog: &Catalog<'_>,
    runtime: &mut mind_world::Runtime,
    input: Input,
) -> Result<u32> {
    check(catalog.require_source_owner())?;
    let (name, flags, word10, position, auxiliary, scalar, trailing) = match input {
        Input::Move {
            destination,
            auxiliary,
            speed,
            trailing,
        } => ("move", 0x5c, 0, destination, auxiliary, speed, trailing),
        Input::Attack { target, trailing } => (
            "kill",
            0x40 | u32::from(!target.is_null()),
            target.raw(),
            [0; 2],
            [0; 2],
            0,
            trailing,
        ),
    };
    let d = check(aa_rules::native::actor_command::predefined_actor_command(
        name,
    ))?;
    let token = check(catalog.definition_bank.try_borrow_mut())?
        .intern("ActorCommand", d.id)
        .map_err(Error::Required)?;
    check(runtime.commands.allocate(PlainCommand {
        definition_id: token,
        definition: Some(aa_mind::command::Definition {
            component_slot: d.component_slot,
            mind_type: d.mind_type,
            behavior_flags: d.behavior_flags,
        }),
        flags,
        word10,
        object_definition: None,
        position_bits: position,
        auxiliary_bits: auxiliary,
        scalar_bits: scalar,
        parameter_definition: None,
        extra_definition: None,
        boolean: 0,
        trailing,
    }))
}
struct Routing<'a, 'w, 'store, 'src, S> {
    catalog: &'a Catalog<'src>,
    world: &'a mut mind_world::World<'w, 'store, S>,
    arena: &'a mut Arena,
    command: u32,
}
impl<S: mind_world::Services> TryCommandRouting for Routing<'_, '_, '_, '_, S> {
    type Actor = ActorId;
    type Handler = (ActorId, ComponentSlot);
    type Error = Error;
    fn handler(&mut self, actor: ActorId, slot: u32) -> Result<Option<Self::Handler>> {
        let o = object(self.catalog, &self.world.actors, actor)?;
        let slot = ComponentSlot::from_index(slot as usize)
            .ok_or_else(|| Error::Required("native valid component dispatch slot".into()))?;
        Ok(o.components()
            .map_err(|e| Error::Required(e.to_string()))?
            .present(slot)
            .then_some((actor, slot)))
    }
    fn accepts(&mut self, actor: ActorId) -> Result<bool> {
        let command = check(self.world.runtime.commands.value(self.command))?;
        let d = definition(self.catalog, command)?;
        allows(self.catalog, &self.world.actors, actor, d.mind_type)
    }
    fn handle(&mut self, (actor, slot): Self::Handler) -> Result<u8> {
        if slot != ComponentSlot::Mind {
            return required(format!("actual component{slot:?} command virtual+24"));
        }
        let Some(RuntimeComponent::Mind(installed)) =
            check(self.world.actors.component(actor, slot))?
        else {
            return required("actual Mind slot payload");
        };
        let mind: MindId = installed.mind_id;
        if check(self.arena.mind(mind))?.actor != actor.raw() {
            return required("same current Mind/Actor owner");
        }
        Ok(u8::from(self.world.process_command(
            mind,
            self.command,
            self.arena,
        )?))
    }
    fn redirect(&mut self, actor: ActorId) -> Result<Option<Option<ActorId>>> {
        object(self.catalog, &self.world.actors, actor)?;
        match check(self.world.actors.component(actor, ComponentSlot::Structure))? {
            None => Ok(None),
            Some(RuntimeComponent::Structure(s))
                if s.header.owner == actor && s.object_handle_10 == 0 =>
            {
                Ok(None)
            }
            Some(RuntimeComponent::Structure(_)) => {
                required("4CE980/588990 actual parent Settlement+14 attachment owner")
            }
            Some(_) => required("actual Structure slot payload"),
        }
    }
    fn container_member(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        object(self.catalog, &self.world.actors, actor)?;
        match check(
            self.world
                .actors
                .component(actor, ComponentSlot::Organization),
        )? {
            None => Ok(None),
            Some(RuntimeComponent::Organization(o)) if o.header.owner == actor => {
                let raw = *o
                    .words
                    .get(&0x44)
                    .ok_or_else(|| Error::Required("actual Organization+44 member".into()))?;
                Ok((raw != 0).then_some(ActorId::from_raw(raw)))
            }
            Some(_) => required("actual Organization slot payload/owner"),
        }
    }
}
///4D9B60 itself does not authorize player ownership. Call after the native
/// selected/actor decree boundary; its returned byte is diagnostic to4ED360.
pub fn deliver<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    command: u32,
    actor: Option<ActorId>,
) -> Result<CommandDelivery> {
    // Native null-actor branch never dereferences the command definition.
    if actor.is_none() {
        return Ok(CommandDelivery {
            result: 0,
            null_actor_logged: true,
        });
    }
    let d = definition(catalog, check(world.runtime.commands.value(command))?)?;
    commands::try_deliver_actor_command(
        &mut Routing {
            catalog,
            world,
            arena,
            command,
        },
        d.component_slot,
        actor,
    )
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionOutcome {
    Null,
    AlreadySelected,
    AttachedElement,
    Selected,
}
fn selection_byte(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    actor: ActorId,
    increment: bool,
) -> Result<()> {
    object(catalog, actors, actor)?;
    let key = actors
        .key(actor)
        .ok_or_else(|| Error::Required("selected actor generation".into()))?;
    let o = check(actors.object_mut(key))?
        .object
        .as_mut()
        .ok_or_else(|| Error::Required("actual Object4C byte".into()))?;
    o.byte_4c = if increment {
        o.byte_4c.wrapping_add(1)
    } else {
        o.byte_4c.wrapping_sub(1)
    };
    Ok(())
}
fn update_group(view: &mut impl crate::player_selection::View) -> Result<()> {
    let list = view.list();
    let single = match (list.head(), list.tail()) {
        (None, None) => true,
        (Some(h), Some(t)) => check(list.node(h))?.actor == check(list.node(t))?.actor,
        _ => return required("consistent actual selection head/tail owner"),
    };
    if single {
        view.clear_group();
        Ok(())
    } else {
        required(
            "4ECF00 same-owner wildcard/static-group storage at the first positive group store",
        )
    }
}
fn rebuild_single_masks(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
) -> Result<()> {
    // Four immediate zero stores of4ECD70 precede its first actor callback.
    view.mask_words_mut().fill(0);
    let Some(head) = view.list().head() else {
        return Ok(());
    };
    let tail = view
        .list()
        .tail()
        .ok_or_else(|| Error::Required("actual selection tail".into()))?;
    if check(view.list().node(head))?.actor != check(view.list().node(tail))?.actor {
        return required("4ECD70 actual ActorCommandData+3C/group owners for multiple selection");
    }
    // The real singleton branch bypasses ActorCommandData+3C. Both admitted
    // class3/4 vtables dispatch+40 to887B50 (false, ret8), never F0/F4 here.
    for index in 0..39 {
        let mask = 1u64 << index;
        let low = mask as u32;
        let high = (mask >> 32) as u32;
        view.mask_words_mut()[2] |= low;
        view.mask_words_mut()[3] |= high;
        let mut node = view.list().head();
        while let Some(id) = node {
            let entry = check(view.list().node(id))?;
            object(catalog, actors, entry.actor)?;
            view.mask_words_mut()[2] &= !low;
            view.mask_words_mut()[3] &= !high;
            node = entry.next;
        }
    }
    Ok(())
}
/// Concrete4ECFF0 for ordinary Actor/Kingdom candidates. This is the native
/// add-selection function, not permission to replace picking/ownership logic.
/// Positive multi-selection group/source services remain required at their
/// reached store. Earlier Object4C/list changes are retained on failure.
pub fn select(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    session: &dyn crate::load_world_globals::SessionPhase,
    actor: Option<ActorId>,
) -> Result<SelectionOutcome> {
    let Some(actor) = actor else {
        return Ok(SelectionOutcome::Null);
    };
    if check(view.list().find(actor))?.is_some() {
        return Ok(SelectionOutcome::AlreadySelected);
    }
    object(catalog, actors, actor)?;
    if check(session.application_state())? != 3 {
        if let Some(component) = check(actors.component(actor, ComponentSlot::Element))? {
            let RuntimeComponent::Element(e) = component else {
                return required("actual Element slot payload");
            };
            if e.header.owner != actor {
                return required("actual Element owner");
            }
            if e.organization.is_some() {
                return Ok(SelectionOutcome::AttachedElement);
            }
        }
    }
    selection_byte(catalog, actors, actor, true)?;
    check(view.list_mut().prepend(actor))?;
    update_group(view)?;
    rebuild_single_masks(catalog, actors, view)?;
    Ok(SelectionOutcome::Selected)
}
///4ED080 removes the current allocation after decrementing its Object4C byte.
pub fn deselect(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    actor: ActorId,
) -> Result<bool> {
    let Some(node) = check(view.list().find(actor))? else {
        return Ok(false);
    };
    selection_byte(catalog, actors, actor, false)?;
    check(view.list_mut().remove(node))?;
    update_group(view)?;
    rebuild_single_masks(catalog, actors, view)?;
    Ok(true)
}
///4ED0D0 decrements every Object4C before retiring any selection node, then
/// refreshes its null group and zero masks. Partial errors do not reset the list.
pub fn clear_selection(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
) -> Result<()> {
    let mut current = view.list().head();
    while let Some(node) = current {
        let entry = check(view.list().node(node))?;
        current = entry.next;
        selection_byte(catalog, actors, entry.actor, false)?;
    }
    while let Some(node) = view.list().head() {
        check(view.list_mut().remove(node))?;
    }
    update_group(view)?;
    view.mask_words_mut().fill(0);
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedDelivery {
    Empty,
    Delivered,
}
/// Explicit-command/null-group4ED360 branch after its diagnostic4D9A40.
/// The real current order-node284 must match this actual scoped player's view.
/// Default commands, positive group formation and live-group preparation remain
/// required. Return means delivery was attempted; native ignores acceptance.
pub fn deliver_selected<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    admin: &aa_orders::net::admin::AdminNode,
    view: &mut impl crate::player_selection::View,
    command: u32,
) -> Result<SelectedDelivery> {
    if admin.ordering_player != Some(view.player_id()) {
        return required("actual current admin284 player and same scoped selection owner");
    }
    let cmd = check(world.runtime.commands.value(command))?;
    if cmd.flags & (1 << 6) == 0 {
        return required("4DA570 default selected-command source");
    }
    let selected_definition = definition(catalog, cmd)?;
    if !matches!(selected_definition.mind_type, 0 | 5) {
        return required("bounded selected move5/attack0 command entry");
    }
    if !check(view.group_is_null())? {
        return required("4ED360 actual positive selection-group/formation services");
    }
    let mut node = view.list().head();
    if node.is_none() {
        return Ok(SelectedDelivery::Empty);
    }
    while let Some(id) = node {
        // Native captures the next node before virtual+B4/authorization/delivery.
        let entry = check(view.list().node(id))?;
        node = entry.next;
        object(catalog, &world.actors, entry.actor)?;
        let cmd = check(world.runtime.commands.value(command))?;
        if authorize_selected(
            catalog,
            &world.actors,
            spatial,
            entry.actor,
            view.kingdom(),
            cmd,
            globals,
        )? {
            let group = ActorId::from_raw(actor_word(catalog, &world.actors, entry.actor, 0x164)?);
            if world.actors.key(group).is_some() {
                return required("4D1900/4DFF30 actual live command-group preparation");
            }
            let _ = deliver(catalog, world, arena, command, Some(entry.actor))?;
        }
    }
    Ok(SelectedDelivery::Delivered)
}
/// Own the one original temporary command reference through synchronous selected
/// delivery, then release it as4CA2D0 does, including on a required-child error.
#[allow(clippy::too_many_arguments)]
pub fn issue_selected<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    admin: &aa_orders::net::admin::AdminNode,
    view: &mut impl crate::player_selection::View,
    input: Input,
) -> Result<SelectedDelivery> {
    let command = allocate(catalog, world.runtime, input)?;
    let result = deliver_selected(
        catalog, world, arena, spatial, globals, admin, view, command,
    );
    let released = world.runtime.commands.release(command);
    match (result, released) {
        (Ok(v), Ok(())) => Ok(v),
        (Err(e), Ok(())) => Err(e),
        (Ok(_), Err(e)) => Err(Error::Mind(e)),
        (Err(e), Err(release)) => required(format!(
            "{e}; original command reference remains owned: {release}"
        )),
    }
}
