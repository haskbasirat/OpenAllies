//!522270 and529D80: request reason4, current live members, current-actor death.
//!This coordinator is an unpublished draft during the release source freeze.
use crate::{MindId, Result};

pub trait World {
    type Actor: Copy;
    type Organization: Copy;
    fn current_actor(&mut self, mind: MindId) -> Result<Self::Actor>;
    ///Actual virtual2C; concrete ordinary Actor dispatch uses4CEEF0 below.
    fn request_reason(&mut self, actor: Self::Actor, reason: u32) -> Result<()>;
    ///52227B..81 captures this allocation ONCE, after529D80 returns.
    fn organization(&mut self, actor: Self::Actor) -> Result<Self::Organization>;
    ///Current captured Organization+2C, reread after each virtual callback.
    fn member_count(&mut self, org: Self::Organization) -> Result<usize>;
    ///Current captured Organization+28 vector/index; null entries are skipped.
    fn member(&mut self, org: Self::Organization, index: usize) -> Result<Option<Self::Actor>>;
    fn death_4d7960(&mut self, actor: Self::Actor) -> Result<()>;
}
///4CEEF0 preserves every existing nonzero reason, including signed-bit values.
pub fn ordinary_reason(current: u32, request: u32) -> Option<u32> {
    if current == 0 { Some(request) } else { None }
}
pub fn run<W: World>(world: &mut W, mind: MindId) -> Result<()> {
    let actor = world.current_actor(mind)?;
    world.request_reason(actor, 4)?; //529D80
    let actor = world.current_actor(mind)?;
    let org = world.organization(actor)?;
    let mut index = 0;
    while index < world.member_count(org)? {
        if let Some(member) = world.member(org, index)? {
            world.request_reason(member, 4)?;
        }
        index += 1;
    }
    let actor = world.current_actor(mind)?;
    world.death_4d7960(actor)
}

///4D7960's proved initial prefix only. The positive remainder is a mandatory
///service, not an acknowledgement that an actor was killed or unregistered.
pub trait DeathPrefixWorld {
    type Actor: Copy;
    type Credit: Copy;
    fn flags(&mut self, actor: Self::Actor) -> Result<u32>;
    fn credit_4d57c0(&mut self, actor: Self::Actor) -> Result<Self::Credit>;
    fn remaining_4d7993(&mut self, actor: Self::Actor, credit: Self::Credit) -> Result<()>;
}
pub fn death_prefix<W: DeathPrefixWorld>(world: &mut W, actor: W::Actor) -> Result<()> {
    if world.flags(actor)? & 1 != 0 {
        return Ok(());
    }
    let credit = world.credit_4d57c0(actor)?;
    world.remaining_4d7993(actor, credit)
}
