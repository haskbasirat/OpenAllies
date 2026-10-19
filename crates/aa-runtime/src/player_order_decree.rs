//! Concrete86/87 gameplay leaves after the native payload/decree-log boundary.
//! Decoded scalar commands enter the same Runtime.commands allocation owner;
//! actor/selection dispatch reuses61E220/61F210 and4D9B60/4ED360 kernels.
//! Positive definition/ParameterSet graphs, diagnostic sinks and group effects
//! remain required at their actual boundaries. This is not a network codec.
use crate::{
    actor_world::{ActorAccess, Spatial},
    catalog::Catalog,
    mind_world::{self, Commands, PlainCommand},
    player_order_world::{self as orders, Error, Globals, Result},
    player_selection::View,
};
use aa_actors::ActorId;
use aa_mind::Arena;
use aa_orders::{
    commands::{
        self, ActorCommandWire, ActorOutcome, ActorSlotReference, CommandWire, SelectedOutcome,
        TryActorCommandContext, TrySelectedCommandContext,
    },
    payload::{Field, Registry, ThingCheck},
};

#[path = "player_order_describe.rs"]
pub mod describe;
#[path = "player_order_object_reference.rs"]
pub mod object_reference;
#[cfg(test)]
#[path = "player_order_delivery_tests.rs"]
mod delivery_tests;

fn required<T>(text: impl Into<String>) -> Result<T> {
    Err(Error::Required(text.into()))
}
fn check<T>(r: std::result::Result<T, impl std::fmt::Display>) -> Result<T> {
    r.map_err(|e| Error::Required(e.to_string()))
}

/// This flat-field adapter consumes already decoded fields. It does not read
/// bytes or repeat registry normalization. Only native command-manager count39
/// is known here; other reference counts are never supplied as guessed zeros.
pub enum Decoded {
    Selected(CommandWire),
    Actor(ActorCommandWire),
}
struct Fields<'a>(std::slice::Iter<'a, Field>);
impl Fields<'_> {
    fn uint(&mut self, expected: u8) -> Result<u32> {
        match self.0.next() {
            Some(Field::Uint { width, value })
                if *width == expected && (expected == 32 || *value < (1u32 << expected)) =>
            {
                Ok(*value)
            }
            _ => required(format!("decoded native uint{expected} field")),
        }
    }
    fn command_reference(&mut self) -> Result<Option<u32>> {
        match self.0.next() {
            Some(Field::Def {
                registry: Registry(0xb3aa58),
                index,
            }) if *index <= 39 => Ok((*index != 39).then_some(*index)),
            _ => required("actual39-entry ActorCommand decoded reference"),
        }
    }
    /// 61EBA0 flag bit1: 76A960 value over `[B39F58]+28 + 1` possibilities;
    /// the count itself is the NULL sentinel (resolution happens later).
    fn object_reference(&mut self, count: usize) -> Result<Option<u32>> {
        match self.0.next() {
            Some(Field::Def {
                registry: Registry(0xb39f58),
                index,
            }) if (*index as usize) <= count => Ok(((*index as usize) != count).then_some(*index)),
            _ => required("actual B39F58 Object-definition decoded reference"),
        }
    }
}
pub fn from_fields(descriptor: usize, fields: &[Field]) -> Result<Decoded> {
    from_fields_with_objects(descriptor, fields, None)
}
/// [`from_fields`] with the current `[B39F58]+28` Thing-manager count, which
/// decodes flag bit1 (61EBA0's 76A960 Object-definition reference).
pub fn from_fields_with_objects(descriptor: usize, fields: &[Field], objects: Option<usize>) -> Result<Decoded> {
    if !matches!(descriptor, 86 | 87) {
        return required("86/87 accepted command decree boundary");
    }
    let mut fields = Fields(fields.iter());
    let flags = match fields.0.next() {
        Some(Field::Bounded { n: 0x800, value }) if *value < 0x800 => *value as u16,
        _ => return required("decoded native eleven-bit command flags"),
    };
    let mut wire = CommandWire {
        flags,
        ..CommandWire::default()
    };
    let has = |bit: u32| flags & (1u16 << bit) != 0;
    if has(6) {
        wire.command = fields.command_reference()?;
    }
    if has(0) {
        wire.word = fields.uint(32)?;
    }
    if has(1) {
        let Some(count) = objects else {
            return required("actual B39F58 Object-definition registry/null sentinel");
        };
        wire.object = fields.object_reference(count)?;
    }
    if has(2) {
        wire.position = [fields.uint(32)?, fields.uint(32)?];
    }
    if has(3) {
        wire.auxiliary = [fields.uint(32)?, fields.uint(32)?];
    }
    if has(4) {
        wire.scalar = fields.uint(32)?;
    }
    if has(7) {
        return required("actual B3AB08 definition registry/null sentinel");
    }
    if has(5) && fields.uint(1)? == 0 {
        return required("actual4C2C60 ParameterSet allocation/definition owner");
    }
    if has(8) {
        return required("actual B3AA2C definition registry/null sentinel");
    }
    if has(9) {
        wire.boolean = fields.uint(1)? == 1;
    }
    wire.trailing = fields.uint(1)? == 1;
    let out = if descriptor == 87 {
        let slot = match fields.0.next() {
            Some(Field::Thing {
                check: ThingCheck::B4,
                slot,
            }) => *slot,
            _ => return required("decoded76A5A0 B4 actor-slot suffix"),
        };
        Decoded::Actor(ActorCommandWire {
            command: wire,
            actor: ActorSlotReference(slot),
        })
    } else {
        Decoded::Selected(wire)
    };
    if fields.0.next().is_some() {
        return required("exact end of decoded command decree fields");
    }
    Ok(out)
}

/// Resolved NULL is represented explicitly by None. A positive unowned
/// registry reference or ParameterSet cannot be recast as a portable token.
/// The input is already decoded/validated: this is not a partial61EBA0 reader.
pub(crate) fn allocate_decoded_with_bank(
    catalog: &Catalog<'_>,
    banks: &dyn orders::Banks,
    runtime: &mut mind_world::Runtime,
    wire: &CommandWire,
) -> Result<u32> {
    check(catalog.require_source_owner())?;
    if wire.flags >= 0x800 {
        return required("native eleven-bit command flags");
    }
    let has = |bit: u32| wire.flags & (1u16 << bit) != 0;
    if (has(7) && wire.parameter.is_some())
        || (has(8) && wire.extra.is_some())
        || (has(5) && wire.parameters.is_some())
    {
        return required("actual positive command definition/ParameterSet owners");
    }
    let definition = if has(6) {
        wire.command
            .map(|index| {
                aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS
                    .get(index as usize)
                    .copied()
                    .ok_or_else(|| Error::Required("canonical ActorCommand ordinal".into()))
            })
            .transpose()?
    } else {
        None
    };
    let token = match definition {
        Some(d) => {
            check(banks.borrow_bank_mut()?.intern("ActorCommand", d.id))?
        }
        None => 0,
    };
    // 61EBA0 bit1: 76A960 stores [B39F58]+24[v] only for an ActorData
    // definition (vt+38), NULL otherwise; the flag bit itself is kept.
    let object_definition = match (has(1), wire.object) {
        (true, Some(value)) => match check(object_reference::resolve_object_76a960(catalog, value))? {
            Some(d) => Some(check(object_reference::object_definition_token(banks, d))?),
            None => None,
        },
        _ => None,
    };
    let command = PlainCommand {
        definition_id: token,
        definition: definition.map(|d| aa_mind::command::Definition {
            component_slot: d.component_slot,
            mind_type: d.mind_type,
            behavior_flags: d.behavior_flags,
        }),
        flags: u32::from(wire.flags),
        word10: if has(0) { wire.word } else { 0 },
        object_definition,
        position_bits: if has(2) { wire.position } else { [0; 2] },
        auxiliary_bits: if has(3) { wire.auxiliary } else { [0; 2] },
        scalar_bits: if has(4) { wire.scalar } else { 0 },
        parameter_definition: None,
        extra_definition: None,
        boolean: u8::from(has(9) && wire.boolean),
        trailing: u8::from(wire.trailing),
    };
    check(runtime.commands.allocate(command))
}
fn release<T>(runtime: &mut mind_world::Runtime, command: u32, result: Result<T>) -> Result<T> {
    match (result, runtime.commands.release(command)) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(Error::Mind(error)),
        (Err(error), Err(release)) => required(format!(
            "{error}; decree command reference remains owned: {release}"
        )),
    }
}

/// Positive4D1900/4DFF30 uses the actual current generation and live storage.
/// No default implementation or inferred empty group is provided.
pub trait Groups {
    fn prepare(
        &mut self,
        catalog: &Catalog<'_>,
        actors: &mut ActorAccess<'_>,
        runtime: &mut mind_world::Runtime,
        arena: &mut Arena,
        group: ActorId,
        member: ActorId,
    ) -> Result<()>;
}
struct ActorBinding<'a, 'world, 'store, 'source, S, P, G, E> {
    catalog: &'a Catalog<'source>,
    banks: &'a dyn orders::Banks,
    world: &'a mut mind_world::World<'world, 'store, S>,
    arena: &'a mut Arena,
    spatial: &'a P,
    globals: &'a mut G,
    groups: &'a mut E,
}
impl<S: mind_world::Services, P: Spatial, G: Globals, E: Groups> TryActorCommandContext
    for ActorBinding<'_, '_, '_, '_, S, P, G, E>
{
    type Actor = ActorId;
    type Command = u32;
    type Group = (ActorId, ActorId);
    type Error = Error;
    fn resolve_slot(&mut self, slot: ActorSlotReference) -> Result<Option<ActorId>> {
        //76A5A0 receives only low16. Resolve the current slot, then preserve
        // its generation-bearing identity for every later Rust access.
        Ok(self
            .world
            .actors
            .identities()
            .iter()
            .find(|entry| entry.id.slot() == slot.0)
            .filter(|entry| entry.is_actor)
            .map(|entry| entry.id))
    }
    fn permit(&mut self, actor: &ActorId, command: &mut u32) -> Result<bool> {
        // 61E237 vt+150(NULL, &cmd, false) = 4CF430: a default command is
        // guessed by 4DA570 on this same arena command (completion persists).
        orders::authorize_actor_arena_with_bank(self.catalog, self.banks,
            &self.world.actors,
            self.spatial,
            &mut self.world.runtime.commands,
            *command,
            *actor,
            self.globals,
        )
    }
    fn has_valid_link(&mut self, actor: &ActorId) -> Result<bool> {
        let group = ActorId::from_raw(orders::actor_word_with_bank(self.catalog, self.banks,
            &self.world.actors,
            *actor,
            0x164,
        )?);
        Ok(self.world.actors.key(group).is_some())
    }
    fn linked_group(&mut self, actor: &ActorId) -> Result<Self::Group> {
        let group = ActorId::from_raw(orders::actor_word_with_bank(self.catalog, self.banks,
            &self.world.actors,
            *actor,
            0x164,
        )?);
        self.world.actors.key(group).ok_or_else(|| {
            Error::Required("4D1900 current group generation changed before reread".into())
        })?;
        Ok((group, *actor))
    }
    fn prepare_group(&mut self, (group, actor): Self::Group) -> Result<()> {
        self.groups.prepare(
            self.catalog,
            &mut self.world.actors,
            self.world.runtime,
            self.arena,
            group,
            actor,
        )
    }
    fn deliver(&mut self, actor: &ActorId, command: &mut u32) -> Result<()> {
        // 61E220 never stores the 4CF430-guessed definition (no 4D9A90), so a
        // permitted default command reaches 4D9B70 with command+08 null.
        if check(self.world.runtime.commands.value(*command))?.definition.is_none() {
            return required("4D9B70 [null command+08]+18: 61E220 delivers a 4DA570-guessed default command without its definition (native fault)");
        }
        orders::deliver_with_bank(self.catalog, self.banks, self.world, self.arena, *command, Some(*actor)).map(drop)
    }
}
/// Actor commands do not acquire selected-player authorization: native+150
/// receives NULL/false. This is the accepted-decree boundary after host checks.
#[allow(clippy::too_many_arguments)]
pub(crate) fn issue_actor_with_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    banks: &dyn orders::Banks,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    groups: &mut impl Groups,
    wire: &ActorCommandWire,
) -> Result<ActorOutcome> {
    let mut command = allocate_decoded_with_bank(catalog, banks, world.runtime, &wire.command)?;
    let result = commands::try_process_actor(
        wire.actor,
        &mut command,
        &mut ActorBinding {
            catalog,
            banks,
            world,
            arena,
            spatial,
            globals,
            groups,
        },
    );
    release(world.runtime, command, result)
}

/// Real61F210 logging and4D9A40 synchronizer writes remain required sinks.
/// These read-only loans preserve the actual command/selection/actor owners.
/// Source getters in this bounded domain cannot mutate selection or commands.
pub trait Diagnostics {
    fn selection(
        &mut self,
        catalog: &Catalog<'_>,
        actors: &ActorAccess<'_>,
        commands: &Commands,
        view: &impl View,
        command: u32,
        empty: bool,
    ) -> Result<()>;
    fn command_sync(&mut self, commands: &Commands, command: u32) -> Result<()>;
    /// 61F210's description with the 4DA570 ordinal the kernel guessed on
    /// this command (None: explicit, not guessed or NULL). The default keeps
    /// the original `selection` sink.
    #[allow(clippy::too_many_arguments)]
    fn describe(
        &mut self,
        catalog: &Catalog<'_>,
        actors: &ActorAccess<'_>,
        commands: &Commands,
        view: &impl View,
        command: u32,
        empty: bool,
        guessed: Option<u32>,
    ) -> Result<()> {
        let _ = guessed;
        self.selection(catalog, actors, commands, view, command, empty)
    }
}
struct SelectionBinding<'a, 'world, 'store, 'source, S, P, G, D, V> {
    catalog: &'a Catalog<'source>,
    banks: &'a dyn orders::Banks,
    world: &'a mut mind_world::World<'world, 'store, S>,
    arena: &'a mut Arena,
    spatial: &'a P,
    globals: &'a mut G,
    diagnostics: &'a mut D,
    slot: &'a u32,
    view: &'a mut V,
}
impl<S: mind_world::Services, P: Spatial, G: Globals, D: Diagnostics, V: View>
    TrySelectedCommandContext for SelectionBinding<'_, '_, '_, '_, S, P, G, D, V>
{
    type Selection = ();
    type Command = u32;
    type Error = Error;
    fn current_selection(&mut self) -> Result<Option<()>> {
        if *self.slot == 0 || *self.slot != self.view.player_id() {
            return required("actual current Admin284 and same player's VWorldState");
        }
        Ok(Some(()))
    }
    fn is_empty(&mut self, _: &()) -> Result<bool> {
        Ok(self.view.list().head().is_none())
    }
    fn inspect_empty(&mut self, _: &(), command: &mut u32) -> Result<()> {
        self.diagnostics.describe(
            self.catalog,
            &self.world.actors,
            &self.world.runtime.commands,
            self.view,
            *command,
            true,
            None,
        )
    }
    fn inspect_selected(&mut self, _: &(), command: &mut u32) -> Result<()> {
        let head = self
            .view
            .list()
            .head()
            .ok_or_else(|| Error::Required("actual nonempty selection head".into()))?;
        let actor = check(self.view.list().node(head))?.actor;
        orders::object_with_bank(self.catalog, self.banks, &self.world.actors, actor)?;
        // Native+B4 is true for the admitted classes; +108 then reads E8
        // before the optional default-definition lookup and diagnostics.
        orders::actor_word_with_bank(self.catalog, self.banks, &self.world.actors, actor, 0xe8)?;
        let mut guessed = None;
        if check(self.world.runtime.commands.value(*command))?.flags & (1 << 6) == 0 {
            // 61F3D8: 4DA570(cmd, first actor, false). The definition only
            // names the diagnostic; its 4DA994 completion stays on the command.
            guessed = orders::guess_arena_with_bank(self.catalog, self.banks, &self.world.actors,
                &mut self.world.runtime.commands, *command, actor, false, self.globals)?
                .map(|g| g.index);
        }
        self.diagnostics.describe(
            self.catalog,
            &self.world.actors,
            &self.world.runtime.commands,
            self.view,
            *command,
            false,
            guessed,
        )
    }
    fn deliver_selection(&mut self, _: &(), command: &mut u32) -> Result<()> {
        self.diagnostics
            .command_sync(&self.world.runtime.commands, *command)?;
        orders::deliver_selected_with_bank(self.catalog, self.banks,
            self.world,
            self.arena,
            self.spatial,
            self.globals,
            self.slot,
            self.view,
            *command,
        )
        .map(drop)
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn issue_selected_with_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    banks: &dyn orders::Banks,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    diagnostics: &mut impl Diagnostics,
    ordering_player_284: &u32,
    view: &mut impl View,
    wire: &CommandWire,
) -> Result<SelectedOutcome> {
    let mut command = allocate_decoded_with_bank(catalog, banks, world.runtime, wire)?;
    let result = commands::try_process_selected(
        &mut command,
        &mut SelectionBinding {
            catalog,
            banks,
            world,
            arena,
            spatial,
            globals,
            diagnostics,
            slot: ordering_player_284,
            view,
        },
    );
    release(world.runtime, command, result)
}

//Legacy per-read Catalog APIs and explicit actual moved-Bank entry points.
pub fn allocate_decoded(
    catalog: &Catalog<'_>,
    runtime: &mut mind_world::Runtime,
    wire: &CommandWire,
) -> Result<u32> {
    allocate_decoded_with_bank(catalog, &catalog.definition_bank, runtime, wire)
}

///Same decree algorithm with the actual moved Bank; no Catalog fallback.
pub fn allocate_decoded_in_bank(
    catalog: &Catalog<'_>,
    bank: &orders::DefinitionBankLoan<'_>,
    runtime: &mut mind_world::Runtime,
    wire: &CommandWire,
) -> Result<u32> {
    allocate_decoded_with_bank(catalog, bank, runtime, wire)
}

pub fn issue_actor<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    groups: &mut impl Groups,
    wire: &ActorCommandWire,
) -> Result<ActorOutcome> {
    issue_actor_with_bank(catalog, &catalog.definition_bank, world, arena, spatial, globals, groups, wire)
}

///Same decree algorithm with the actual moved Bank; no Catalog fallback.
pub fn issue_actor_in_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    bank: &orders::DefinitionBankLoan<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    groups: &mut impl Groups,
    wire: &ActorCommandWire,
) -> Result<ActorOutcome> {
    issue_actor_with_bank(catalog, bank, world, arena, spatial, globals, groups, wire)
}

pub fn issue_selected<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    diagnostics: &mut impl Diagnostics,
    ordering_player_284: &u32,
    view: &mut impl View,
    wire: &CommandWire,
) -> Result<SelectedOutcome> {
    issue_selected_with_bank(catalog, &catalog.definition_bank, world, arena, spatial, globals, diagnostics, ordering_player_284, view, wire)
}

///Same decree algorithm with the actual moved Bank; no Catalog fallback.
pub fn issue_selected_in_bank<S: mind_world::Services>(
    catalog: &Catalog<'_>,
    bank: &orders::DefinitionBankLoan<'_>,
    world: &mut mind_world::World<'_, '_, S>,
    arena: &mut Arena,
    spatial: &impl Spatial,
    globals: &mut impl Globals,
    diagnostics: &mut impl Diagnostics,
    ordering_player_284: &u32,
    view: &mut impl View,
    wire: &CommandWire,
) -> Result<SelectedOutcome> {
    issue_selected_with_bank(catalog, bank, world, arena, spatial, globals, diagnostics, ordering_player_284, view, wire)
}
