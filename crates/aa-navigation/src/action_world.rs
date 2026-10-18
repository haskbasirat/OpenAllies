//! Fallible ActionGo callbacks and compatibility with the original fixture API.
use crate::{ActorState, MoveState, MoverRequest, NavigationWorld, Result};
/// Required actual5300A0 world services. Failures retain already-applied effects;
/// the enclosing runtime must latch failure rather than automatically retry.
pub trait TryActionWorld {
    fn node_reached(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()>;
    fn single_node_placement(
        &mut self,
        waypoint: [u32; 2],
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool>;
    fn actor_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool>;
    fn formation_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool>;
    fn actor_or_leader_handle_valid(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool>;
    fn organization_ready(&mut self, state: &mut MoveState, actor: &mut ActorState)
    -> Result<bool>;
    fn leader_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool>;
    fn stop_action(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()>;
    fn begin_mover(
        &mut self,
        request: MoverRequest,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool>;
    fn next_world_random_u32(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<u32>;
    fn schedule(
        &mut self,
        delay_bits: u32,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<()>;
}
pub(crate) struct Legacy<'a, W>(pub &'a mut W);
impl<W: NavigationWorld> TryActionWorld for Legacy<'_, W> {
    fn node_reached(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()> {
        self.0.node_reached(state, actor);
        Ok(())
    }
    fn single_node_placement(
        &mut self,
        waypoint: [u32; 2],
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool> {
        Ok(self.0.single_node_placement(waypoint, state, actor))
    }
    fn actor_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool> {
        Ok(self.0.actor_is_moving(state, actor))
    }
    fn formation_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool> {
        Ok(self.0.formation_ready(state, actor))
    }
    fn actor_or_leader_handle_valid(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool> {
        Ok(self.0.actor_or_leader_handle_valid(state, actor))
    }
    fn organization_ready(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool> {
        Ok(self.0.organization_ready(state, actor))
    }
    fn leader_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool> {
        Ok(self.0.leader_is_moving(state, actor))
    }
    fn stop_action(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()> {
        self.0.stop_action(state, actor);
        Ok(())
    }
    fn begin_mover(
        &mut self,
        request: MoverRequest,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool> {
        Ok(self.0.begin_mover(request, state, actor))
    }
    fn next_world_random_u32(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<u32> {
        Ok(self.0.next_world_random_u32(state, actor))
    }
    fn schedule(
        &mut self,
        delay_bits: u32,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<()> {
        self.0.schedule(delay_bits, state, actor);
        Ok(())
    }
}
