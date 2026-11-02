//! Which film decrees have an aa-play equivalent (independent of aa-play,
//! so the reader and its reports build without it).
use crate::decree::Decree;

/// ActorCommand registry index (`[0xB3AA58]`, 0x475870 order) → the aa-play
/// `Command` it maps to (`mapping::Mapper`), `None` for no equivalent.
pub const ACTOR_COMMAND_MAPPING: [(&str, Option<&str>); 39] = [
    ("kill", Some("Attack")),
    ("bombard", None),
    ("guard", None),
    ("repair", None),
    ("construct", None),
    (
        "move",
        Some("Move (Follow when the command has a target word)"),
    ),
    ("stop", Some("Stop")),
    ("retreat", None),
    ("rout", None),
    ("explore", None),
    ("disband", Some("Disband")),
    ("unpack", Some("Deploy (facing from auxiliary: inferred)")),
    ("pack_organization", None),
    ("build_structure", None),
    ("commission", None),
    ("recruit", Some("Recruit")),
    ("set_rally_point", Some("SetRallyPoint")),
    ("recruit_location", None),
    ("structure_cancel_queued", None),
    ("construction_cancel", None),
    ("sell", Some("Sell")),
    ("upgrade", None),
    ("raze", None),
    ("sally_forth", None),
    ("recall", None),
    ("air_attack", None),
    ("air_recon", None),
    ("escort_on", None),
    ("escort_off", None),
    ("attach", Some("Attach")),
    ("detach", None),
    ("hq_attach", None),
    ("hq_detach", Some("Detach")),
    ("set_formation", Some("SetFormation")),
    ("pack", Some("Pack")),
    ("research", Some("Research")),
    ("cancel_flare", None),
    ("group_behavior_enable", None),
    ("group_behavior_disable", None),
];

/// Coarse class of a decree for reports: `state` (ordering player,
/// selection), `sync`, `timeline`, `names`, `command` (has an aa-play
/// mapping path) or `other`.
pub fn decree_class(decree: &Decree) -> &'static str {
    match decree {
        Decree::SetPlayer { .. }
        | Decree::SelectionSet { .. }
        | Decree::SelectionAdd { .. }
        | Decree::SelectionRemove { .. }
        | Decree::SelectionClear
        | Decree::SelectionClearNamed { .. } => "state",
        Decree::SimSyncValue { .. } | Decree::SyncValue { .. } => "sync",
        Decree::SetGameSpeed { .. }
        | Decree::TogglePaused
        | Decree::SetPaused { .. }
        | Decree::SpeedUp
        | Decree::SpeedDown => "timeline",
        Decree::UpdateKingdomName { .. } => "names",
        Decree::TellSelectedCommand(_)
        | Decree::TellActorCommand { .. }
        | Decree::TellActorAttack { .. }
        | Decree::TellActorMove { .. }
        | Decree::FactionUseAbility { .. } => "command",
        Decree::TellActorAbility { .. }
        | Decree::GamePlayerKingdom { .. }
        | Decree::Other { .. } => "other",
    }
}

/// The ActorCommand of a command decree with its aa-play mapping, for reports.
pub fn actor_command(decree: &Decree) -> Option<(u32, &'static str, Option<&'static str>)> {
    let wire = match decree {
        Decree::TellSelectedCommand(w) => w,
        Decree::TellActorCommand { command, .. } => command,
        _ => return None,
    };
    let index = wire.command?;
    let (name, to) = ACTOR_COMMAND_MAPPING
        .get(index as usize)
        .copied()
        .unwrap_or(("unknown", None));
    Some((index, name, to))
}

/// A generic command without an ActorCommand (flag bit 6 clear): the player's
/// plain click. The kernel picks the command from the selected actor and these
/// fields (0x4DA570, 0x61F210). Named by the fields it carries.
fn default_order(decree: &Decree) -> String {
    let wire = match decree {
        Decree::TellSelectedCommand(wire) => wire,
        Decree::TellActorCommand { command, .. } => command,
        _ => return "default order".into(),
    };
    const FIELDS: [(u16, &str); 9] = [
        (0, "target"),
        (1, "object"),
        (2, "position"),
        (3, "auxiliary"),
        (4, "scalar"),
        (5, "parameters"),
        (7, "formation"),
        (8, "property"),
        (9, "boolean"),
    ];
    let names: Vec<&str> = FIELDS
        .iter()
        .filter(|(bit, _)| wire.flags >> bit & 1 != 0)
        .map(|(_, name)| *name)
        .collect();
    format!("default order ({})", names.join(", "))
}

/// The order a gameplay decree gives, for coverage reports: the ActorCommand of
/// an 86/87, the definition of a 23 and the decree itself otherwise, with what
/// aa-play does with it (`mapping::Mapper`). `None` for bookkeeping decrees.
pub fn order_kind(descriptor: usize, decree: &Decree) -> Option<String> {
    let mapped = |name: &str, to: Option<&str>| {
        format!("{name} -> {}", to.unwrap_or("NO EQUIVALENT"))
    };
    Some(match decree {
        Decree::TellSelectedCommand(_) | Decree::TellActorCommand { .. } => {
            match actor_command(decree) {
                Some((index, name, to)) => mapped(&format!("command {index:02} {name}"), to),
                None => mapped(&default_order(decree), None),
            }
        }
        Decree::TellActorAttack { .. } => mapped("65 TellActorAttack", Some("Attack")),
        Decree::TellActorMove { .. } => mapped("66 TellActorMove", Some("Move")),
        Decree::TellActorAbility { .. } => mapped("67 TellActorAbility", None),
        Decree::FactionUseAbility { .. } => mapped("70 FactionUseAbility", Some("UseAbility")),
        Decree::GamePlayerKingdom { .. } => mapped("68 GamePlayerKingdom", None),
        Decree::Other { fields } if descriptor == crate::decree::id::TEAM_COMMAND => {
            let name = match fields.first() {
                Some(aa_orders::payload::Field::Uint { value, .. }) => crate::registries::TEAM_COMMANDS
                    .get(*value as usize)
                    .map_or("unknown", |(name, _)| name),
                _ => "unknown",
            };
            mapped(&format!("23 TeamCommand {name}"), None)
        }
        _ => return None,
    })
}
