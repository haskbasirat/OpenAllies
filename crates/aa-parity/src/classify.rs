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
