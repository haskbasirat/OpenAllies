//! Native Element ResultAbilityWait52D820 and ResultAbilityPursue52EF40.
//! Required services bind real ability, movement and decision implementations.
//! Errors preserve every earlier callback effect; these are not transactions.
use crate::{
    Arena, Error, MindId, Result, StateId,
    ability_decision::Decision,
    action,
    construct::{Installed, Kind},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub trait World: action::World {
    /// Actual4C83E0 (aa_combat::admission::recharge_remaining).
    fn recharge_remaining(&mut self, ability: u32, actor: u32) -> Result<Binary>;
    fn schedule_wait(
        &mut self,
        mind: MindId,
        delay_bits: u32,
        data: u32,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()>;
    fn bombard_mode(&self, mind: MindId) -> Result<u32>;
    fn local_definition(&self, ability: u32) -> Result<u32>;
    /// Actual529C30: definition of the current Mind actor.
    fn actor_definition(&mut self, mind: MindId, arena: &mut Arena) -> Result<u32>;
    /// Actual4BE6C0, aa_combat::weights::maximum with authored actor bonuses.
    fn local_weight(&mut self, local: u32, definition: u32) -> Result<Binary>;
    fn performer(&self, actor: u32) -> Result<u32>;
    /// Actual57E670 with excluded=None, not a clip duration.
    fn maximum_weight(&mut self, performer: u32) -> Result<Binary>;
    fn reconsider(
        &mut self,
        mind: MindId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()>;
    /// Actual embedded Move7C Think537B90. This is not ActionGo or a no-op.
    fn think_move(
        &mut self,
        state: StateId,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<()>;
    /// Actual4CF110 Properties lookup on the current source actor.
    fn modifier(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Actual4C8270, aa_combat::admission::effective_range.
    fn effective_range(&mut self, ability: u32, modifier: Option<u32>) -> Result<Binary>;
    /// Actual4CEDE0 / aa_spatial::moving_state::excluded, including its
    /// Element/Regiment semantics. This pure native getter requires an actor.
    fn target_is_moving(&mut self, target: u32) -> Result<bool>;
    fn facing(&self, actor: u32) -> Result<[u32; 2]>;
    fn world_delta(&self) -> Result<u32>;
    /// Target actor virtual+18. Delta was captured before this callback.
    fn target_speed(
        &mut self,
        actor: u32,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<Binary>;
    /// Actual576AD0, aa_movement::target::set_target with the current target ID
    /// and current world delta. It must commit to the real owned Mover state.
    fn set_target(&mut self, mover: u32, target: Option<u32>, range_bits: u32) -> Result<()>;
    /// Current Mind virtual+1AC. The signed native result is compared with5.
    fn target_decision(
        &mut self,
        mind: MindId,
        target: Option<u32>,
        decision: &mut Decision,
        arena: &mut Arena,
    ) -> Result<i32>;
}
fn n<T>(v: aa_numeric::Result<T>) -> Result<T> {
    v.map_err(|e| Error::World(format!("ability motion arithmetic: {e}")))
}
fn binding(installed: &Installed, arena: &Arena) -> Result<StateId> {
    if installed.kind != Kind::Element {
        return Err(Error::World(
            "ability motion requires an Element Mind".into(),
        ));
    }
    let state = *installed
        .state_bindings
        .get(&0x7c)
        .ok_or_else(|| Error::World("Element Move7C binding missing".into()))?;
    if arena.state(state)?.parent != installed.mind_id {
        return Err(Error::World("Element Move7C parent mismatch".into()));
    }
    Ok(state)
}
pub fn wait(
    installed: &Installed,
    decision: &mut Decision,
    arena: &mut Arena,
    world: &mut impl World,
) -> Result<()> {
    let state = binding(installed, arena)?;
    let mind = installed.mind_id;
    let remaining = world.recharge_remaining(decision.ability, arena.mind(mind)?.actor)?;
    let stored = n(remaining.store_f32())?;
    let mut delay = 0x3f19_999a;
    if remaining.compare(&Binary::from_i32(0)) == Ordering::Greater
        && n(Binary::from_f32_bits(stored))?.compare(&n(Binary::from_f32_bits(delay))?)
            == Ordering::Less
    {
        delay = stored;
    }
    action::stop(state, arena, world)?;
    world.schedule_wait(mind, delay, 0, decision, arena)?;
    if world.bombard_mode(mind)? == 2 {
        return Ok(());
    }
    let reconsider = if decision.kind == 6 {
        true
    } else {
        let local = world.local_definition(decision.ability)?;
        // Native EBX preserves this actor across definition/weight services.
        let actor = arena.mind(mind)?.actor;
        let definition = world.actor_definition(mind, arena)?;
        let weight = n(world.local_weight(local, definition)?.store_f32())?;
        let performer = world.performer(actor)?;
        world
            .maximum_weight(performer)?
            .compare(&n(Binary::from_f32_bits(weight))?)
            == Ordering::Greater
    };
    if reconsider {
        world.reconsider(mind, decision, arena)?;
    }
    Ok(())
}
pub fn pursue(
    installed: &Installed,
    decision: &mut Decision,
    arena: &mut Arena,
    world: &mut impl World,
    precision: Precision,
) -> Result<()> {
    let state = binding(installed, arena)?;
    let mind = installed.mind_id;
    world.think_move(state, decision, arena)?;
    let actor = arena.mind(mind)?.actor;
    let view = world.actor(actor)?;
    if view.active_component == view.mover {
        if view.mover.is_none() {
            return Err(Error::World("active Move7C branch requires a Mover".into()));
        }
        let modifier = world.modifier(actor)?;
        let range = world.effective_range(decision.ability, modifier)?;
        let mut range =
            n(n(range.sub(&n(Binary::from_f32_bits(0x38d1_b717))?, precision))?.store_f32())?;
        let target = decision
            .target
            .ok_or_else(|| Error::World("active pursuit requires a target actor".into()))?;
        if world.target_is_moving(target)? {
            let source_facing = world.facing(arena.mind(mind)?.actor)?;
            let target_facing = world.facing(target)?;
            let y = n(n(Binary::from_f32_bits(source_facing[1]))?
                .mul(&n(Binary::from_f32_bits(target_facing[1]))?, precision))?;
            let x = n(n(Binary::from_f32_bits(source_facing[0]))?
                .mul(&n(Binary::from_f32_bits(target_facing[0]))?, precision))?;
            if n(y.add(&x, precision))?.compare(&Binary::from_i32(0)) == Ordering::Greater {
                let delta = n(Binary::from_f32_bits(world.world_delta()?))?;
                let displacement = n(world
                    .target_speed(target, decision, arena)?
                    .mul(&delta, precision))?;
                range = n(
                    n(n(Binary::from_f32_bits(range))?.sub(&displacement, precision))?.store_f32(),
                )?;
            }
        }
        let mover = world
            .actor(arena.mind(mind)?.actor)?
            .mover
            .ok_or_else(|| Error::World("target assignment requires a Mover".into()))?;
        world.set_target(mover, decision.target, range)?;
    }
    if world.target_decision(mind, decision.target, decision, arena)? < 5 {
        world.reconsider(mind, decision, arena)?;
    }
    Ok(())
}
