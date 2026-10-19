//! UIWorldState 0x436950's per-actor call on the UI side: the 0x45B7A0
//! actor's vt+0x150(0x5C0E30() local kingdom, &cmd, 1), i.e. 0x4CF430
//! (GActor 0xA273F0 / GGroup 0xA29490 / GKingdom 0xA299D8) over the SAME
//! retained World between frames: its ActorStore, SpatialWorld and Bank are
//! only read (`World::presentation_parts`).
//!
//! The command is the UI's own KKC_GActorCommand (0x4D9C80 layout, here the
//! [`CommandWire`] the 0x620760/0x6210B0 paths build), presented to the
//! kernel as the PlainCommand its fields are.
use crate::{
    actor_world::{ActorStore, Spatial},
    catalog::Catalog,
    definitions::Bank,
    mind_world::PlainCommand,
    player_order_world::{self as orders, ReadBank},
};
use aa_actors::ActorId;
use aa_orders::commands::CommandWire;

pub type Result<T> = std::result::Result<T, String>;

/// The current identity of `actor+0x14`'s slot (the view leaves' 0x76A500
/// slot), or None when no live identity has it.
pub fn actor_for_slot(actors: &ActorStore, slot: u16) -> Option<ActorId> {
    actors.identities().iter().find(|e| e.id.slot() == slot).map(|e| e.id)
}

/// The UI command as the PlainCommand the actor virtuals read: fields the
/// flags do not name are the 0x4D9C80 zeros. An explicit command (flag bit
/// 6) names its canonical ActorCommand definition, whose token must already
/// be in the read-only Bank. Object, ParameterSet and extra references are
/// not built by the UI paths ported here, and are refused.
pub fn plain_from_ui_wire(bank: &Bank, wire: &CommandWire) -> Result<PlainCommand> {
    let has = |bit: u8| wire.flags & (1 << bit) != 0;
    if wire.object.is_some() || wire.parameter.is_some() || wire.parameters.is_some() || wire.extra.is_some() {
        return Err("UI command with an Object/ParameterSet/extra reference (not built by 0x620760/0x6210B0)".into());
    }
    let definition = if has(6) {
        let index = wire.command.ok_or("UI command flag bit 6 without its +0x08 definition")?;
        Some(
            aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS
                .get(index as usize)
                .copied()
                .ok_or("UI command +0x08 is not a canonical ActorCommand ordinal")?,
        )
    } else {
        None
    };
    let definition_id = match definition {
        Some(d) => bank
            .token("ActorCommand", d.id)
            .ok_or("UI explicit command definition is not interned in the World's Bank")?,
        None => 0,
    };
    Ok(PlainCommand {
        definition_id,
        definition: definition.map(|d| aa_mind::command::Definition {
            component_slot: d.component_slot,
            mind_type: d.mind_type,
            behavior_flags: d.behavior_flags,
        }),
        flags: u32::from(wire.flags),
        word10: if has(0) { wire.word } else { 0 },
        object_definition: None,
        position_bits: if has(2) { wire.position } else { [0; 2] },
        auxiliary_bits: if has(3) { wire.auxiliary } else { [0; 2] },
        scalar_bits: if has(4) { wire.scalar } else { 0 },
        parameter_definition: None,
        extra_definition: None,
        boolean: u8::from(has(9) && wire.boolean),
        trailing: u8::from(wire.trailing),
    })
}

/// vt+0x150(kingdom, &cmd, 1) of `actor` for 0x436950. `kingdom` is
/// 0x5C0E30 -> 0x76AC70 (the local player's kingdom); `globals` the same
/// application gate/phase/UVar reads the decree path uses. A 0x4DA570
/// completion of the UI command (type-4 construct) is refused explicitly.
#[allow(clippy::too_many_arguments)]
pub fn actor_virtual_150(
    catalog: &Catalog<'_>,
    actors: &ActorStore,
    spatial: &impl Spatial,
    bank: &Bank,
    actor: ActorId,
    kingdom: Option<ActorId>,
    wire: &mut CommandWire,
    globals: &mut impl orders::Globals,
) -> Result<bool> {
    let mut command = plain_from_ui_wire(bank, wire)?;
    let before = command.clone();
    let loan = ReadBank::new(bank);
    let allowed = orders::authorize_selected_command_read(catalog, &loan, actors, spatial, actor, kingdom, &mut command, globals)
        .map_err(|e| e.to_string())?;
    if command != before {
        return Err("0x4DA994 completion of the UI command inside 0x436950 (not mapped back to the UI command)".into());
    }
    Ok(allowed)
}
