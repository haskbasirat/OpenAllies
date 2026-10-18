//! Original Element decision utility52ED30, including health and player terms.
//! All floating store boundaries are explicit; this function returns x87 Binary.
use crate::{Arena, Error, MindId, Result, ability_decision::Decision};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct LocalWeights {
    pub shared_definition: u32,
    pub priority: u32,
    /// Move, use, wait at local+24/28/2C.
    pub factors: [u32; 3],
    pub bonuses: [u32; 3],
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Shared {
    pub flags54: u32,
    pub word64_bits: u32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Health {
    pub current_bits: u32,
    pub maximum_bits: u32,
}
pub trait World {
    /// Pure snapshots of the actual registered local/shared ability definitions.
    fn local_weights(&self, ability: u32) -> Result<LocalWeights>;
    fn shared(&self, definition: u32) -> Result<Shared>;
    /// Actual actor definition+204/208/20C, index move/use/wait.
    fn actor_bonus(&self, actor: u32, index: usize) -> Result<u32>;
    /// 4C8A40 boolean, with the cached ability and current actor.
    fn movement_wait_weight(
        &mut self,
        ability: u32,
        actor: u32,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<bool>;
    fn target_health(&self, target: u32) -> Result<Health>;
    /// Native4BE8E0 lookup and scalar at returned effect+14. Kind3 is optional;
    /// kind4 must exist when the positive shared+64/player branch requires it.
    fn effect_scalar(&self, definition: u32, kind: u32) -> Result<Option<u32>>;
    fn health_bonus(&self) -> Result<u32>;
    fn actor_uses_player_weight(&self, actor: u32) -> Result<bool>;
    fn target_player(&self, target: Option<u32>) -> Result<Option<u32>>;
    /// Native501650 result stays extended until multiplied by the stored utility.
    fn player_weight(&mut self, player: u32, effect_scalar: u32) -> Result<Binary>;
}
fn num<T>(value: aa_numeric::Result<T>) -> Result<T> {
    value.map_err(|e| Error::World(format!("decision utility: {e}")))
}
fn n(bits: u32) -> Result<Binary> {
    num(Binary::from_f32_bits(bits))
}
pub fn score(
    mind: MindId,
    decision: &mut Decision,
    arena: &mut Arena,
    world: &mut impl World,
    p: Precision,
) -> Result<Binary> {
    let ability = decision.ability;
    let initial = world.local_weights(ability)?;
    // Original caches both the local and shared definition pointers across the
    // movement predicate, even if that callback changes the current decision.
    let shared_id = initial.shared_definition;
    let mut stored = initial.priority;
    match decision.status {
        0 | 1 => {
            let index = if decision.status == 0 { 1 } else { 2 };
            let actor = arena.mind(mind)?.actor;
            let value = num(n(initial.factors[index])?.mul(&n(initial.priority)?, p))?;
            let value = num(value.add(&n(initial.bonuses[index])?, p))?;
            let value = num(value.add(&n(world.actor_bonus(actor, index)?)?, p))?;
            stored = num(value.store_f32())?;
        }
        2 => {
            let actor = arena.mind(mind)?.actor;
            let wait = world.movement_wait_weight(ability, actor, decision, arena)?;
            let current = world.local_weights(ability)?;
            let index = if wait { 2 } else { 0 };
            let value = num(n(current.factors[index])?.mul(&n(current.priority)?, p))?;
            let value = num(value.add(&n(current.bonuses[index])?, p))?;
            stored = num(value.store_f32())?;
            // This branch always adds the move bonus, including when the local
            // factor/bonus came from wait. The intermediate store is observable.
            let actor = arena.mind(mind)?.actor;
            stored = num(num(n(world.actor_bonus(actor, 0)?)?.add(&n(stored)?, p))?.store_f32())?;
        }
        _ => {}
    }
    if world.shared(shared_id)?.flags54 & (1 << 6) != 0 {
        let target = decision
            .target
            .ok_or_else(|| Error::World("health-weighted ability requires a target body".into()))?;
        let health = world.target_health(target)?;
        let missing =
            num(num(n(health.maximum_bits)?.sub(&n(health.current_bits)?, p))?.store_f32())?;
        if decision.status != 1
            && let Some(amount) = world.effect_scalar(shared_id, 3)?
        {
            let amount = n(amount)?;
            let missing = n(missing)?;
            let selected = if missing.compare(&amount) == Ordering::Less {
                missing
            } else {
                amount.clone()
            };
            stored = num(num(num(selected.div(&amount, p))?.mul(&n(stored)?, p))?.store_f32())?;
        }
        let target = decision
            .target
            .ok_or_else(|| Error::World("health-weighted ability lost its target".into()))?;
        let maximum = world.target_health(target)?.maximum_bits;
        let value = num(n(missing)?.div(&n(maximum)?, p))?;
        let value = num(value.mul(&n(world.health_bonus()?)?, p))?;
        stored = num(num(value.add(&n(stored)?, p))?.store_f32())?;
    }
    if n(world.shared(shared_id)?.word64_bits)?.compare(&Binary::from_i32(0)) == Ordering::Greater
        && world.actor_uses_player_weight(arena.mind(mind)?.actor)?
        && let Some(player) = world.target_player(decision.target)?
    {
        let scalar = world
            .effect_scalar(shared_id, 4)?
            .ok_or_else(|| Error::World("player-weighted ability lacks native effect4".into()))?;
        let factor = world.player_weight(player, scalar)?;
        return num(factor.mul(&n(stored)?, p));
    }
    n(stored)
}
