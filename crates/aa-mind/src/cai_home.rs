//! Original Mind home setter `52C280(home)` (the Mind +28 home GTID that
//! aircraft and denizen Minds return to).
//!
//! Order:
//! 1. A null home, one with +8 bit 0 set (being removed), or one refusing
//!    the Mind's actor (home virtual +174(actor)) clears Mind +28.
//! 2. The home's +14 GTID equal to Mind +28 changes nothing.
//! 3. Otherwise Mind +28 = that GTID, actor +8 |= 4, the pending commands are
//!    released (`52C0D0`), and an actor with a zero +BC word clears the
//!    state stack (`52A3C0`).
//! 4. When Mind +28 was zero before and is not now, the strategic AI
//!    (`[B36528]` present with its +20 byte set) reclassifies the actor
//!    (`7A0C20(actor)`, tail call).
use crate::{Arena, MindId, Result, World, clear};

/// Services in native order.
pub trait HomeWorld: World {
    /// Home +8.
    fn home_flags8(&mut self, home: u32) -> Result<u32>;
    /// Home virtual +174(actor).
    fn home_accepts(&mut self, home: u32, actor: u32, arena: &mut Arena) -> Result<bool>;
    /// Home +14 (its GTID).
    fn home_gtid(&mut self, home: u32) -> Result<u32>;
    /// Actor +8, read and stored.
    fn actor_flags8(&mut self, actor: u32) -> Result<u32>;
    fn set_actor_flags8(&mut self, actor: u32, value: u32) -> Result<()>;
    /// `52C0D0` on the Mind; bind [`crate::command::clear_pending`].
    fn release_pending(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Actor +BC word.
    fn actor_word_bc(&mut self, actor: u32) -> Result<u32>;
    /// `[B36528]` non-null and its +20 byte set.
    fn sai_active(&mut self) -> Result<bool>;
    /// `7A0C20(actor)` on `[B36528]`; bind
    /// [`crate::sai::actor_notify`].
    fn sai_notify(&mut self, actor: u32, arena: &mut Arena) -> Result<()>;
}

fn home_word(mind: MindId, arena: &mut Arena) -> Result<&mut u32> {
    Ok(&mut arena.mind_mut(mind)?.words_24_30[1])
}

/// Complete `52C280(home)` on `mind`.
pub fn set_home(
    mind: MindId,
    home: Option<u32>,
    arena: &mut Arena,
    world: &mut impl HomeWorld,
) -> Result<()> {
    let actor = arena.mind(mind)?.actor;
    let accepted = match home {
        Some(home) => world.home_flags8(home)? & 1 == 0 && world.home_accepts(home, actor, arena)?,
        None => false,
    };
    let Some(home) = home.filter(|_| accepted) else {
        *home_word(mind, arena)? = 0;
        return Ok(());
    };
    let old = *home_word(mind, arena)?;
    let gtid = world.home_gtid(home)?;
    if old == gtid {
        return Ok(());
    }
    *home_word(mind, arena)? = gtid;
    let actor = arena.mind(mind)?.actor;
    let flags = world.actor_flags8(actor)?;
    world.set_actor_flags8(actor, flags | 4)?;
    world.release_pending(mind, arena)?;
    let actor = arena.mind(mind)?.actor;
    if world.actor_word_bc(actor)? == 0 {
        clear(mind, arena, world)?;
    }
    if old != 0 || *home_word(mind, arena)? == 0 || !world.sai_active()? {
        return Ok(());
    }
    let actor = arena.mind(mind)?.actor;
    world.sai_notify(actor, arena)
}
