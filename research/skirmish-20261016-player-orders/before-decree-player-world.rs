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
trait PermissionWorld {
    fn flags(&mut self, actor: ActorId) -> Result<u32>;
    fn parent(&mut self, actor: ActorId) -> Result<Option<ActorId>>;
    fn present(&mut self, actor: ActorId, slot: ComponentSlot) -> Result<bool>;
    fn gate(&mut self) -> Result<bool>;
    fn phase(&mut self) -> Result<u32>;
    fn owner(&mut self, actor: ActorId) -> Result<Option<ActorId>>;
    fn actions(&mut self, actor: ActorId) -> Result<u64>;
}
fn eligible_native(world: &mut impl PermissionWorld, actor: ActorId, argument: u8) -> Result<bool> {
    let mut actor = actor;
    let mut seen = Vec::new();
    loop {
        if seen.contains(&actor) {
            return required("finite4CF120 Element-owner chain (native would loop)");
        }
        seen.push(actor);
        if world.flags(actor)? & (1 << 2) != 0 {
            return Ok(false);
        }
        if let Some(parent) = world.parent(actor)? {
            actor = parent;
            continue;
        }
        if argument != 0 && world.gate()? && world.phase()? == 7 {
            return Ok(true);
        }
        return Ok(!world.present(actor, ComponentSlot::Mover)?
            || world.present(actor, ComponentSlot::MobileDeployment)?
            || world.present(actor, ComponentSlot::Recruit)?
            || world.present(actor, ComponentSlot::Denizen)?);
    }
}
fn permission_native(
    world: &mut impl PermissionWorld,
    actor: ActorId,
    kingdom: Option<ActorId>,
    resolved_type: Option<u32>,
    selected: u8,
) -> Result<bool> {
    let Some(index) = resolved_type else {
        return Ok(false);
    }; //4CF430 unresolved4DA570
    if selected != 0 {
        let owner = world.owner(actor)?;
        if owner.is_none() || owner != kingdom || !eligible_native(world, actor, 1)? {
            return Ok(false);
        }
    }
    Ok(native_command_bit(world.actions(actor)?, index))
}
struct PermissionBinding<'a, 'store, 'src, P, G> {
    catalog: &'a Catalog<'src>,
    actors: &'a ActorAccess<'store>,
    spatial: &'a P,
    globals: &'a mut G,
}
impl<P: Spatial, G: Globals> PermissionWorld for PermissionBinding<'_, '_, '_, P, G> {
    fn flags(&mut self, actor: ActorId) -> Result<u32> {
        object(self.catalog, self.actors, actor)?;
        let key = self
            .actors
            .key(actor)
            .ok_or_else(|| Error::Required("current actor generation".into()))?;
        Ok(check(self.actors.placement(key, self.spatial))?.flags)
    }
    fn parent(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        object(self.catalog, self.actors, actor)?;
        match check(self.actors.component(actor, ComponentSlot::Element))? {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == actor => Ok(e.organization),
            Some(_) => required("actual Element slot payload/owner identity"),
        }
    }
    fn present(&mut self, actor: ActorId, slot: ComponentSlot) -> Result<bool> {
        Ok(object(self.catalog, self.actors, actor)?
            .components()
            .map_err(|e| Error::Required(e.to_string()))?
            .present(slot))
    }
    fn gate(&mut self) -> Result<bool> {
        self.globals.can_process_40eaa0()
    }
    fn phase(&mut self) -> Result<u32> {
        self.globals.session_phase_76ac90()
    }
    fn owner(&mut self, actor: ActorId) -> Result<Option<ActorId>> {
        let raw = actor_word(self.catalog, self.actors, actor, 0xe8)?;
        Ok((raw != 0).then_some(ActorId::from_raw(raw)))
    }
    fn actions(&mut self, actor: ActorId) -> Result<u64> {
        Ok(
            u64::from(actor_word(self.catalog, self.actors, actor, 0xf0)?)
                | (u64::from(actor_word(self.catalog, self.actors, actor, 0xf4)?) << 32),
        )
    }
}
/// Actual4CF120 ordinary virtual+20 over authoritative live fields. Element+10
/// tail-calls preserve the argument and actual source identities.
pub fn eligible(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    spatial: &impl Spatial,
    actor: ActorId,
    argument: u8,
    globals: &mut impl Globals,
) -> Result<bool> {
    eligible_native(
        &mut PermissionBinding {
            catalog,
            actors,
            spatial,
            globals,
        },
        actor,
        argument,
    )
}
/// Concrete selected virtual+150 ->4CE680. Explicit commands bind canonical
/// definitions; actual default4DA570 remains a required source service.
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
    permission_native(
        &mut PermissionBinding {
            catalog,
            actors,
            spatial,
            globals,
        },
        actor,
        kingdom,
        Some(d.mind_type),
        1,
    )
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
fn command_fields(
    d: aa_rules::native::actor_command::ActorCommandDefinition,
    token: u32,
    input: Input,
) -> PlainCommand {
    let (flags, word10, position, auxiliary, scalar, trailing) = match input {
        Input::Move {
            destination,
            auxiliary,
            speed,
            trailing,
        } => (0x5c, 0, destination, auxiliary, speed, trailing),
        Input::Attack { target, trailing } => (
            0x40 | u32::from(!target.is_null()),
            target.raw(),
            [0; 2],
            [0; 2],
            0,
            trailing,
        ),
    };
    PlainCommand {
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
    }
}
/// Low-level native constructor fields. Resolved target identities are a
/// caller precondition here; issue_selected validates its actual target owner.
pub fn allocate(
    catalog: &Catalog<'_>,
    runtime: &mut mind_world::Runtime,
    input: Input,
) -> Result<u32> {
    check(catalog.require_source_owner())?;
    let name = match &input {
        Input::Move { .. } => "move",
        Input::Attack { .. } => "kill",
    };
    let d = check(aa_rules::native::actor_command::predefined_actor_command(
        name,
    ))?;
    let token = check(catalog.definition_bank.try_borrow_mut())?
        .intern("ActorCommand", d.id)
        .map_err(Error::Required)?;
    check(runtime.commands.allocate(command_fields(d, token, input)))
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
pub use crate::player_selection::Outcome as SelectionOutcome;
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
struct SelectionBinding<'a, 'store, 'src> {
    catalog: &'a Catalog<'src>,
    actors: &'a mut ActorAccess<'store>,
    session: Option<&'a dyn crate::load_world_globals::SessionPhase>,
}
impl crate::player_selection::World for SelectionBinding<'_, '_, '_> {
    fn application_state(&mut self) -> std::result::Result<u32, String> {
        self.session
            .ok_or("actual SessionE4 owner")?
            .application_state()
    }
    fn is_actor(&mut self, actor: ActorId) -> std::result::Result<bool, String> {
        object(self.catalog, self.actors, actor)
            .map(|_| true)
            .map_err(|e| e.to_string())
    }
    fn element_parent(&mut self, actor: ActorId) -> std::result::Result<Option<ActorId>, String> {
        match self
            .actors
            .component(actor, ComponentSlot::Element)
            .map_err(|e| e.to_string())?
        {
            None => Ok(None),
            Some(RuntimeComponent::Element(e)) if e.header.owner == actor => Ok(e.organization),
            Some(_) => Err("actual Element slot/owner".into()),
        }
    }
    fn change_byte_4c(
        &mut self,
        actor: ActorId,
        increment: bool,
    ) -> std::result::Result<(), String> {
        selection_byte(self.catalog, self.actors, actor, increment).map_err(|e| e.to_string())
    }
    fn update_group(
        &mut self,
        view: &mut impl crate::player_selection::View,
    ) -> std::result::Result<(), String> {
        update_group(view).map_err(|e| e.to_string())
    }
    fn rebuild_masks(
        &mut self,
        view: &mut impl crate::player_selection::View,
    ) -> std::result::Result<(), String> {
        rebuild_single_masks(self.catalog, self.actors, view).map_err(|e| e.to_string())
    }
}
/// Concrete4ECFF0 over actual ordinary Actor fields and the same scoped view.
/// Positive group/multiple services remain required at their reached owner;
/// this is no fixed selected-unit cap and earlier stores remain committed.
pub fn select(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    session: &dyn crate::load_world_globals::SessionPhase,
    actor: Option<ActorId>,
) -> Result<SelectionOutcome> {
    check(crate::player_selection::select(
        &mut SelectionBinding {
            catalog,
            actors,
            session: Some(session),
        },
        view,
        actor,
    ))
}
pub fn deselect(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
    actor: ActorId,
) -> Result<bool> {
    check(crate::player_selection::deselect(
        &mut SelectionBinding {
            catalog,
            actors,
            session: None,
        },
        view,
        actor,
    ))
}
pub fn clear_selection(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    view: &mut impl crate::player_selection::View,
) -> Result<()> {
    check(crate::player_selection::clear(
        &mut SelectionBinding {
            catalog,
            actors,
            session: None,
        },
        view,
    ))
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
    ordering_player_284: &u32,
    view: &mut impl crate::player_selection::View,
    command: u32,
) -> Result<SelectedDelivery> {
    if *ordering_player_284 == 0 || *ordering_player_284 != view.player_id() {
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
    ordering_player_284: &u32,
    view: &mut impl crate::player_selection::View,
    input: Input,
) -> Result<SelectedDelivery> {
    if let Input::Attack { target, .. } = &input
        && !target.is_null()
    {
        object(catalog, &world.actors, *target)?;
    }
    let command = allocate(catalog, world.runtime, input)?;
    let result = deliver_selected(
        catalog,
        world,
        arena,
        spatial,
        globals,
        ordering_player_284,
        view,
        command,
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

#[cfg(test)]
#[path = "player_order_world_tests.rs"]
mod tests;
