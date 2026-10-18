//! Kingdom/team definition restriction lists: `0x4E1910` (actor restriction
//! used by BuildActor, upgrades, settlements and AI), `0x4E1810` and the
//! technology variant `0x4E1890`.
//!
//! The world manager at world+0x12C (`0x4F0400` -> `0x504560`) maps profile
//! names to profiles. A kingdom is checked against the profile named by its
//! team (kingdom+0x1EC, name at team+0x64) and then the profile named by its
//! own +0x348 string. Each profile has two maps keyed by definition ID string
//! (ActorData+8): restricted actors at +0x34 (`0x504500`) and restricted
//! technologies at +0x44 (`0x504530`). An entry counts only when its value is
//! non-null. Keys hash (`0x405030`, a-z folded) and compare (`0x96CAD6`
//! `_wcsicmp`, C-locale path: A-Z folded) case-insensitively on ASCII letters
//! only. Profiles come from the rules/mission data (loader `0x505760`); they
//! are inputs here.

/// One named profile (world+0x12C manager value). Map entries are kept in
/// insertion order as (definition ID, value non-null); the native chains are
/// head-inserted, so the most recently inserted matching key decides.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RestrictionProfile {
    /// +0x34 map.
    pub actors: Vec<(String, bool)>,
    /// +0x44 map.
    pub technologies: Vec<(String, bool)>,
}

/// World+0x12C profile map (`0x504560`), insertion order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RestrictionProfiles {
    pub profiles: Vec<(String, RestrictionProfile)>,
}

impl RestrictionProfiles {
    /// `0x4F0400`/`0x504560(name)`.
    pub fn find(&self, name: &str) -> Option<&RestrictionProfile> {
        self.profiles
            .iter()
            .rev()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, profile)| profile)
    }
}

fn restricted(list: &[(String, bool)], id: &str) -> bool {
    list.iter()
        .rev()
        .find(|(key, _)| key.eq_ignore_ascii_case(id))
        .is_some_and(|(_, value)| *value)
}

/// `0x4E1810(definition)` (and `0x4E1910(actor, definition)`, which repeats
/// the team check first with the same result): false when the team profile
/// or the kingdom profile restricts the definition ID.
pub fn actor_allowed(
    team: Option<&RestrictionProfile>,
    kingdom: Option<&RestrictionProfile>,
    id: &str,
) -> bool {
    !team.is_some_and(|p| restricted(&p.actors, id))
        && !kingdom.is_some_and(|p| restricted(&p.actors, id))
}

/// `0x4E1890(definition)`: the same over the technology maps.
pub fn technology_allowed(
    team: Option<&RestrictionProfile>,
    kingdom: Option<&RestrictionProfile>,
    id: &str,
) -> bool {
    !team.is_some_and(|p| restricted(&p.technologies, id))
        && !kingdom.is_some_and(|p| restricted(&p.technologies, id))
}
