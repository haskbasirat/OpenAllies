//! `531D30` Element Mind (UAI) deferred Update: the Mind+8 Updatable (vtable
//! A357E8) slot +1C, reached from World+208 (queue4) through `4F05B0`.
//!
//! Native order (`this` = Mind+8):
//! 1. `529D00` base update ([`crate::regiment_update::base_update`]);
//! 2. a removed (`+8` bit 0) actor, `+8` bit 6, or `+104` bit 1 returns;
//! 3. the static SVar `UAIBadTargetExpirationSeconds` (`4B2E60`) is bound on
//!    first use; the Mind+1B0 bad-target list drops expired or moved entries
//!    (`52F370`);
//! 4. `529F70` current state, its virtual +C (Think);
//! 5. an active, unlinked Actor+6C Mover with Mind+88 != 2 runs `4CFC80`;
//! 6. a live actor whose Mind+C scheduling head is null is rescheduled through
//!    Updatable virtual +4 (`52DC10`) with 2.0 s, unless its active component
//!    (Actor+E4, other than Actor+68) is itself linked, or its Actor+C0
//!    Transport has byte +38 while the Join state (Mind+164) is not current;
//! 7. a "summoned" actor (Properties+1C flags bit 4) whose Mind virtual +94
//!    (in combat) is false logs `summoned %d no longer in combat - dieing`
//!    and runs `4D7960`.
use crate::{MindId, Result, regiment_update::BaseWorld};

/// binary32 2.0: the re-think delay passed to Updatable virtual +4.
pub const RESCHEDULE_DELAY_BITS: u32 = 0x4000_0000;

pub trait World: BaseWorld {
    type State: Copy;
    /// Actor +8 flags.
    fn actor_flags8(&mut self, actor: Self::Actor) -> Result<u32>;
    /// Actor +104 (state bits 32..63).
    fn actor_state_high(&mut self, actor: Self::Actor) -> Result<u32>;
    /// Step 3: walk Mind+1B0 with the SVar (`52F370` unlinks); the binding
    /// owns both the static SVar read and the list.
    fn expire_bad_targets(&mut self, mind: MindId) -> Result<()>;
    /// `529F70`: a null current state is a native fault.
    fn current_state(&mut self, mind: MindId) -> Result<Self::State>;
    /// Current state virtual +C.
    fn think(&mut self, state: Self::State) -> Result<()>;
    /// Actor+6C non-null, `4DB0F0` (it is Actor+E4) and its +C link null.
    fn mover_active_unlinked(&mut self, actor: Self::Actor) -> Result<bool>;
    /// Mind+88.
    fn mind_word88(&mut self, mind: MindId) -> Result<u32>;
    /// `4CFC80` on the actor.
    fn deactivate_4cfc80(&mut self, actor: Self::Actor) -> Result<()>;
    /// Mind+C (this Updatable's own +4 event head) non-null.
    fn updatable_linked(&mut self, mind: MindId) -> Result<bool>;
    /// Actor+E4 non-null, not Actor+68, and its +C link non-null.
    fn active_component_linked(&mut self, actor: Self::Actor) -> Result<bool>;
    /// Actor+C0 non-null, `52A000` on Mind+164 false and Transport+38 set.
    fn transport_blocks(&mut self, mind: MindId, actor: Self::Actor) -> Result<bool>;
    /// Updatable virtual +4 (`52DC10`) with (delay, data 0).
    fn reschedule(&mut self, mind: MindId, delay_bits: u32) -> Result<()>;
    /// Actor+78 non-null and its +1C flags bit 4.
    fn summoned(&mut self, actor: Self::Actor) -> Result<bool>;
    /// Mind virtual +94 (`52D730`).
    fn in_combat(&mut self, mind: MindId) -> Result<bool>;
    /// `404F60` diagnostic then `4D7960` on the actor.
    fn summoned_dies(&mut self, actor: Self::Actor) -> Result<()>;
}

/// Complete `531D30`.
pub fn update<W: World>(w: &mut W, mind: MindId) -> Result<()> {
    crate::regiment_update::base_update(w, mind)?;
    let actor = w.actor(mind)?;
    let flags = w.actor_flags8(actor)?;
    if flags & 1 != 0 || flags & (1 << 6) != 0 {
        return Ok(());
    }
    if w.actor_state_high(actor)? & 2 != 0 {
        return Ok(());
    }
    w.expire_bad_targets(mind)?;
    let state = w.current_state(mind)?;
    w.think(state)?;
    let actor = w.actor(mind)?;
    if w.mover_active_unlinked(actor)? && w.mind_word88(mind)? != 2 {
        w.deactivate_4cfc80(actor)?;
    }
    let actor = w.actor(mind)?;
    if w.actor_flags8(actor)? & 1 == 0
        && !w.updatable_linked(mind)?
        && !w.active_component_linked(actor)?
        && !w.transport_blocks(mind, actor)?
    {
        w.reschedule(mind, RESCHEDULE_DELAY_BITS)?;
    }
    let actor = w.actor(mind)?;
    if w.summoned(actor)? && !w.in_combat(mind)? {
        w.summoned_dies(actor)?;
    }
    Ok(())
}
