//! Code-predefined ActorCommandData fields used by saved Mind commands.
//!
//! Original enum constructor4DA9D0/4DA2F0 binds names to native type IDs.
//! GameData475870 constructs every definition with4740F0(type, component,
//! behavior_flags), then4B4E60 registers it in B3AA58 (GameData+274).
//! This table comes from execution of those original functions, not TGI order.
//! ActorCommand LoadData48CD00 changes presentation/control fields but never
//! these three words. A command need not have a TGI UI declaration to exist.
//!
//! Native lookup463FA0/4915F0 compares the built-in ASCII names without
//! regard to ASCII case. No whitespace trimming, aliases, unknown-ID default,
//! subtype command handlers, or whole ActorCommand UI loader is implied.
use crate::native::{Error, Result};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ActorCommandDefinition {
    /// Canonical built-in IDS, from4DA2F0; not an authored source alias.
    pub id: &'static str,
    /// Definition+0C, the native type consumed by Mind dispatch.
    pub mind_type: u32,
    /// Definition+18. Mind is slot4; other component slots remain distinct.
    pub component_slot: u32,
    /// Definition+2C, consumed by52C320 and other native command consumers.
    pub behavior_flags: u32,
}

const fn entry(
    id: &'static str,
    mind_type: u32,
    component_slot: u32,
    behavior_flags: u32,
) -> ActorCommandDefinition {
    ActorCommandDefinition {
        id,
        mind_type,
        component_slot,
        behavior_flags,
    }
}

/// Complete original475870 registration order. These are game command
/// identities, not a fixed runtime allocation pool or count of live commands.
pub const PREDEFINED_ACTOR_COMMANDS: &[ActorCommandDefinition] = &[
    entry("kill", 0, 4, 8496),
    entry("bombard", 1, 4, 8528),
    entry("guard", 2, 4, 4400),
    entry("repair", 3, 4, 6912),
    entry("construct", 4, 4, 327728),
    entry("move", 5, 4, 304),
    entry("stop", 6, 4, 0),
    entry("retreat", 7, 4, 304),
    entry("rout", 8, 4, 0),
    entry("explore", 9, 4, 16),
    entry("disband", 10, 4, 1),
    entry("unpack", 11, 4, 48),
    entry("pack_organization", 12, 4, 0),
    entry("build_structure", 13, 14, 589840),
    entry("commission", 14, 16, 1114112),
    entry("recruit", 15, 16, 2162688),
    entry("set_rally_point", 16, 16, 16),
    entry("recruit_location", 17, 16, 8454160),
    entry("structure_cancel_queued", 18, 13, 0),
    entry("construction_cancel", 19, 15, 0),
    entry("sell", 20, 15, 1),
    entry("upgrade", 21, 15, 196608),
    entry("raze", 22, 15, 1),
    entry("sally_forth", 23, 10, 0),
    entry("recall", 24, 10, 0),
    entry("air_attack", 25, 10, 8528),
    entry("air_recon", 26, 10, 16),
    entry("escort_on", 27, 10, 0),
    entry("escort_off", 28, 10, 0),
    entry("attach", 29, 7, 4259840),
    entry("detach", 30, 7, 0),
    entry("hq_attach", 31, 7, 0),
    entry("hq_detach", 32, 7, 0),
    entry("set_formation", 33, 7, 0),
    entry("pack", 34, 19, 0),
    entry("research", 35, 21, 16777216),
    entry("cancel_flare", 36, 26, 0),
    entry("group_behavior_enable", 37, u32::MAX, 0),
    entry("group_behavior_disable", 38, u32::MAX, 0),
];

/// Resolve one canonical saved command IDS against the original built-ins.
/// This does not apply source Translate aliases again. Empty saved references
/// are handled by the saved reader, not converted into a default command here.
pub fn predefined_actor_command(canonical_id: &str) -> Result<ActorCommandDefinition> {
    PREDEFINED_ACTOR_COMMANDS
        .iter()
        .find(|d| d.id.eq_ignore_ascii_case(canonical_id))
        .copied()
        .ok_or_else(|| {
            Error::Unsupported(format!(
                "unknown predefined ActorCommandData IDS {canonical_id:?}"
            ))
        })
}
