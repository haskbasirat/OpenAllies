//! Element Mind ResultAbilityDecision5307A0. This connects the chosen decision
//! to the real Performer lifecycle; scoring and pursuit remain required services.
use crate::{Arena, Error, MindId, Result};
use aa_numeric::Binary;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub ability: u32,
    pub kind: u32,
    /// Native decision+8: 0 ready, 1 waiting, 3 failed; other values pursue.
    pub status: u32,
    pub target: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Start {
    pub ability: u32,
    pub event_kind: u32,
    pub target: Option<u32>,
    pub point: Option<[u32; 2]>,
}
pub trait World {
    /// Pure current snapshots. +118 is the Element bombard mode word.
    fn bombard_mode(&self, mind: MindId) -> Result<u32>;
    fn performer(&self, actor: u32) -> Result<u32>;
    fn actor_id(&self, actor: u32) -> Result<u32>;
    /// Actual aa_combat::performer::start /57F290, not a synthetic attack event.
    fn start(
        &mut self,
        performer: u32,
        request: Start,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Full52ED30 decision utility. It may return an extended x87 value; the
    /// caller stores to binary32 only after this callback completes.
    fn decision_weight(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<Binary>;
    /// 57E670: maximum utility of enabled abilities except the supplied ability's
    /// target-mask class. This value is not an animation duration or cooldown.
    fn maximum_other_ability_weight(
        &mut self,
        performer: u32,
        ability: u32,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<Binary>;
    /// Mind virtual+17C; may mutate the current decision and actor components.
    fn reconsider(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()>;
    fn cancel_update(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()>;
    /// Mind virtual+140 receives the target's current generation-tagged ID.
    fn failed_target(
        &mut self,
        mind: MindId,
        target_id: u32,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()>;
    /// Mind virtual+144 when failed decision has no target.
    fn failed_without_target(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()>;
    fn wait(&mut self, mind: MindId, decision: &mut Decision, arena: &mut Arena) -> Result<()>;
    fn pursue(&mut self, mind: MindId, decision: &mut Decision, arena: &mut Arena) -> Result<()>;
}
fn numeric<T>(value: aa_numeric::Result<T>) -> Result<T> {
    value.map_err(|e| Error::World(format!("Mind decision weight: {e}")))
}
pub fn apply(
    mind: MindId,
    decision: &mut Decision,
    point: Option<[u32; 2]>,
    arena: &mut Arena,
    world: &mut impl World,
) -> Result<()> {
    if decision.status == 0 {
        let event_kind = if world.bombard_mode(mind)? == 2 { 3 } else { 1 };
        let performer = world.performer(arena.mind(mind)?.actor)?;
        let request = Start {
            ability: decision.ability,
            event_kind,
            target: decision.target,
            point,
        };
        if world.start(performer, request, decision, arena)? {
            decision.status = 1;
            let weight = world.decision_weight(mind, decision, arena)?;
            let stored = numeric(weight.store_f32())?;
            // Both the actor component and ability are fetched again after
            // scoring; synchronous world services can change either value.
            let performer = world.performer(arena.mind(mind)?.actor)?;
            let other =
                world.maximum_other_ability_weight(performer, decision.ability, decision, arena)?;
            if other.compare(&numeric(Binary::from_f32_bits(stored))?) == Ordering::Greater {
                world.reconsider(mind, decision, arena)?;
            }
            world.cancel_update(mind, decision, arena)?;
            return Ok(());
        }
        decision.status = 3;
    }
    if decision.status == 3 {
        match decision.target {
            Some(target) => world.failed_target(mind, world.actor_id(target)?, decision, arena),
            None => world.failed_without_target(mind, decision, arena),
        }
    } else if decision.status == 1 {
        world.wait(mind, decision, arena)
    } else {
        world.pursue(mind, decision, arena)
    }
}
