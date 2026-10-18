//! Native death-queue producers and wall-component death/damage leaves.
//!
//! - `4F1A90`: Body damage queues a zero-health actor by pushing it at the
//!   HEAD of the world list (`world+100`, tail `+104`).
//! - `584E70`: a wall component whose owner definition `+4A8` is set queues
//!   every nonnull segment in array order instead of the dying actor.
//! - `584CD0`: copies the seven-word damage record into component `+4C`.
//!
//! There is ONE death list and ONE drain: the world's `+100` list is
//! `aa_world::ListKind::Pending100`, drained head first (LIFO) by
//! `aa_world::World::transition` (`4F65B0`, ported and proven by the tick
//! owner in research/phase8-tick). [`DeathQueue`] is the producer-side seam
//! the runtime binds to `enqueue_front(ListKind::Pending100, actor)`; this
//! crate keeps no list of its own. The phase8-combat drain fixtures remain as
//! independent cross-evidence for that drain.
use crate::Result;
use aa_body::damage::DamageRecord;

/// World `+100` death list producer (`4F1A90` push at the head).
pub trait DeathQueue {
    fn push_front(&mut self, actor: u32) -> Result<()>;
}

/// Native `4F1A90`.
pub fn queue_death(actor: u32, queue: &mut impl DeathQueue) -> Result<()> {
    queue.push_front(actor)
}

/// Native `584E70(actor)`. `all_segments` is the wall owner's definition
/// `+4A8` byte; `segments` is the component `+24` array (count `+28`).
pub fn wall_member_killed(
    actor: u32,
    all_segments: bool,
    segments: &[Option<u32>],
    queue: &mut impl DeathQueue,
) -> Result<()> {
    if !all_segments {
        return queue_death(actor, queue);
    }
    for segment in segments.iter().flatten() {
        queue_death(*segment, queue)?;
    }
    Ok(())
}

/// Native `584CD0`: the outer/wall component keeps the last damage record.
pub fn outer_damaged(last_damage: &mut DamageRecord, record: &DamageRecord) {
    *last_damage = *record;
}
