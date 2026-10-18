//! Original Organization add-member (`57CBA0`) callees that had no port:
//! GActor `4CED10` (experience rank source), GActor `4CEBB0` with
//! Organization `57A330` and Performer `57E010` (rank-change propagation),
//! and Body `561760` (organization owner set: cancel the Body's scheduled
//! (1, 0) Updatable entry). Oracle:
//! research/phase8-saved-components/native-organization-callees.json.
//!
//! Not ported here (other areas; see the requests file): Element `5705A0`,
//! Organization `5784C0`, GActor `4CEF80`, Mind `524B80`, recruit item
//! `4E1200`, and Performer `57DCF0` (called by `57E010`).
use crate::ActorId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("organization callee world failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Actor fields read by `4CED10`/`4CEBB0`/`57A330` (memory facts).
pub trait ActorFacts {
    /// `+A4` ExperienceComponent: its `+C` word (current rank) when present.
    fn experience_rank(&self, actor: ActorId) -> Result<Option<u32>>;
    /// `+80` Element `+10` owning organization actor (None when the actor
    /// has no Element or the Element has no organization).
    fn element_organization(&self, actor: ActorId) -> Result<Option<ActorId>>;
    /// `+74` PerformerComponent present.
    fn has_performer(&self, actor: ActorId) -> Result<bool>;
    /// `+7C` OrganizationComponent: its non-null members `+28[0..+2C]` in
    /// slot order (`4364C0` with the attached-only byte clear), or None.
    fn organization_members(&self, actor: ActorId) -> Result<Option<Vec<ActorId>>>;
}

/// `4CED10`: the actor's Experience `+C`, else the Experience `+C` of the
/// organization its Element belongs to, else 0.
pub fn experience_source(actor: ActorId, facts: &impl ActorFacts) -> Result<u32> {
    if let Some(rank) = facts.experience_rank(actor)? {
        return Ok(rank);
    }
    if let Some(org) = facts.element_organization(actor)?
        && let Some(rank) = facts.experience_rank(org)?
    {
        return Ok(rank);
    }
    Ok(0)
}

pub trait RankWorld: ActorFacts {
    /// `57DCF0(rank)` on the actor's Performer (ability availability by rank).
    fn performer_rank_changed(&mut self, actor: ActorId, rank: u32) -> Result<()>;
}

/// `4CEBB0(actor; old, new)`: Performer `57E010` (only a nonzero `new`
/// reaches `57DCF0`), then Organization `57A330`, which repeats this for every
/// member without its own Experience, in slot order. `old` is passed through
/// unused by both natives.
#[allow(clippy::only_used_in_recursion)] // native signature: `old` is passed through
pub fn rank_changed(actor: ActorId, old: u32, new: u32, world: &mut impl RankWorld) -> Result<()> {
    if world.has_performer(actor)? && new != 0 {
        world.performer_rank_changed(actor, new)?;
    }
    if let Some(members) = world.organization_members(actor)? {
        for member in members {
            if world.experience_rank(member)?.is_none() {
                rank_changed(member, old, new, world)?;
            }
        }
    }
    Ok(())
}

pub trait BodyUpdatable {
    /// `50B220(event, data)` on the Body Updatable (`Body+8`).
    fn body_has_scheduled(&mut self, member: ActorId, event: u8, data: u32) -> Result<bool>;
    /// `50B1C0(event, data)`: remove every matching entry.
    fn body_unschedule(&mut self, member: ActorId, event: u8, data: u32) -> Result<()>;
}

/// `561760(owner)` on the member's Body: with a nonnull owner, a scheduled
/// (1, 0) Body entry is removed.
pub fn body_set_organization(member: ActorId, owner: Option<ActorId>, world: &mut impl BodyUpdatable) -> Result<()> {
    if owner.is_some() && world.body_has_scheduled(member, 1, 0)? {
        world.body_unschedule(member, 1, 0)?;
    }
    Ok(())
}
