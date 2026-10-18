//! Original SAI actor (re)classification `7A0C20(actor)` on the SAI object
//! `[B36528]`: called when an actor joins an organization (`5705A0`) and by
//! the combat agent's special-operation / summon path.
//!
//! Nothing happens before the SAI is ready (+20 byte). The actor's SAI
//! category is its +140 word (5 = none):
//! 1. an actor with +C8 set becomes category 4;
//! 2. else one passing `79E300` becomes category 3;
//! 3. else one that is not `4CEFF0` and whose virtual +20(0) holds is
//!    registered through `7A0A40(actor, 79E080(actor), 0, 0)`;
//! 4. else (SAI still ready) category 2.
//!
//! "Becomes category k" is: nothing when +140 already is k; otherwise
//! `79F6E0(actor, 0)` unless +140 is 5, then `7A0390(actor, k, 0)` unless the
//! actor is removed (+8 bit 0).
use crate::Result;

/// Services of `7A0C20` in native order (the actor is an opaque handle).
pub trait ActorNotifyWorld {
    /// SAI +20 (ready byte).
    fn sai_ready(&mut self) -> Result<bool>;
    /// Actor +C8 is non-null.
    fn actor_c8(&mut self, actor: u32) -> Result<bool>;
    /// Actor +140 (SAI category).
    fn actor_category(&mut self, actor: u32) -> Result<u32>;
    /// Actor +8 bit 0 (removed).
    fn actor_removed(&mut self, actor: u32) -> Result<bool>;
    /// `79E300(actor)`.
    fn predicate_79e300(&mut self, actor: u32) -> Result<bool>;
    /// `4CEFF0` on the actor.
    fn actor_4ceff0(&mut self, actor: u32) -> Result<bool>;
    /// Actor virtual +20(0).
    fn actor_virtual_20(&mut self, actor: u32) -> Result<bool>;
    /// `79E080(actor)`.
    fn predicate_79e080(&mut self, actor: u32) -> Result<bool>;
    /// `79F6E0(actor, 0)` on the SAI (leave the current category).
    fn unregister(&mut self, actor: u32) -> Result<()>;
    /// `7A0390(actor, category, 0)` on the SAI.
    fn register(&mut self, actor: u32, category: u32) -> Result<()>;
    /// `7A0A40(actor, flag, 0, 0)` on the SAI.
    fn register_unit(&mut self, actor: u32, flag: bool) -> Result<()>;
}

fn become_category(w: &mut impl ActorNotifyWorld, actor: u32, category: u32) -> Result<()> {
    let current = w.actor_category(actor)?;
    if current == category {
        return Ok(());
    }
    if current != 5 {
        w.unregister(actor)?;
    }
    if !w.actor_removed(actor)? {
        w.register(actor, category)?;
    }
    Ok(())
}

/// Complete `7A0C20(actor)`.
pub fn notify(w: &mut impl ActorNotifyWorld, actor: u32) -> Result<()> {
    if !w.sai_ready()? {
        return Ok(());
    }
    if w.actor_c8(actor)? {
        return become_category(w, actor, 4);
    }
    if w.predicate_79e300(actor)? {
        return become_category(w, actor, 3);
    }
    if !w.actor_4ceff0(actor)? && w.actor_virtual_20(actor)? {
        let flag = w.predicate_79e080(actor)?;
        return w.register_unit(actor, flag);
    }
    if w.sai_ready()? {
        become_category(w, actor, 2)?;
    }
    Ok(())
}
