//! Pinned native navigation diagnostics. World callbacks below are fixture-only.
//! This module does not implement routes, actor-world integration, or gameplay.
use aa_navigation::*;
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Deserialize)]
struct Input {
    name: String,
    operation: String,
    precision: Precision,
    state: MoveState,
    actor: ActorState,
    source: OwnedPath,
    activation: Activation,
    speed_bits: u32,
    group_speed_minimum_bits: u32,
    catchup_speed_factor_bits: u32,
    random_u32: u32,
    special_state_active: bool,
    can_activate: bool,
    activate_result: bool,
    organization_ready: bool,
    single_node_placement: bool,
    actor_is_moving: bool,
    leader_is_moving: bool,
    formation_ready: bool,
    actor_or_leader_handle_valid: bool,
    begin_result: bool,
    #[serde(default)]
    mutations: std::collections::BTreeMap<String, Value>,
}
struct World {
    input: Input,
    events: Vec<Value>,
}
impl World {
    fn event(&mut self, name: &str, bits: &[u32]) {
        self.events.push(json!({"name":name,"bits":bits}));
    }
    fn mutate(&self, name: &str, state: &mut MoveState, actor: &mut ActorState) {
        let Some(m) = self.input.mutations.get(name) else {
            return;
        };
        if let Some(fields) = m.get("state") {
            let mut value = serde_json::to_value(&*state).unwrap();
            for (k, v) in fields.as_object().unwrap() {
                value[k] = v.clone();
            }
            *state = serde_json::from_value(value).unwrap();
        }
        if let Some(fields) = m.get("actor") {
            let mut value = serde_json::to_value(&*actor).unwrap();
            for (k, v) in fields.as_object().unwrap() {
                value[k] = v.clone();
            }
            *actor = serde_json::from_value(value).unwrap();
        }
        if let Some(kind) = m.get("first_kind") {
            state.path.nodes[0].words[4] = kind.as_u64().unwrap() as u32;
        }
    }
}
impl NavigationWorld for World {
    fn special_state_active(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
        self.event("SpecialStateActive", &[]);
        self.mutate("SpecialStateActive", state, actor);
        self.input.special_state_active
    }
    fn can_activate(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
        self.event("CanActivate", &[]);
        self.mutate("CanActivate", state, actor);
        self.input.can_activate
    }
    fn activate(&mut self, mode: u32, state: &mut MoveState, actor: &mut ActorState) -> bool {
        self.event("Activate", &[mode]);
        self.mutate("Activate", state, actor);
        self.input.activate_result
    }
    fn node_reached(&mut self, state: &mut MoveState, actor: &mut ActorState) {
        self.event("NodeReached", &[]);
        self.mutate("NodeReached", state, actor);
    }
    fn single_node_placement(
        &mut self,
        xy: [u32; 2],
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> bool {
        self.event("SingleNodePlacement", &xy);
        self.mutate("SingleNodePlacement", state, actor);
        self.input.single_node_placement
    }
    fn actor_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
        self.event("ActorIsMoving", &[]);
        self.mutate("ActorIsMoving", state, actor);
        self.input.actor_is_moving
    }
    fn formation_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
        self.event("FormationReady", &[]);
        self.mutate("FormationReady", state, actor);
        self.input.formation_ready
    }
    fn actor_or_leader_handle_valid(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> bool {
        self.event("ActorOrLeaderHandleValid", &[]);
        self.mutate("ActorOrLeaderHandleValid", state, actor);
        self.input.actor_or_leader_handle_valid
    }
    fn organization_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
        self.event("OrganizationReady", &[]);
        self.mutate("OrganizationReady", state, actor);
        self.input.organization_ready
    }
    fn leader_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> bool {
        self.event("LeaderIsMoving", &[]);
        self.mutate("LeaderIsMoving", state, actor);
        self.input.leader_is_moving
    }
    fn stop_action(&mut self, state: &mut MoveState, actor: &mut ActorState) {
        self.event("StopAction", &[]);
        self.mutate("StopAction", state, actor);
    }
    fn begin_mover(
        &mut self,
        r: MoverRequest,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> bool {
        self.event(
            "BeginMover",
            &[
                r.direction_bits[0],
                r.direction_bits[1],
                r.destination_bits[0],
                r.destination_bits[1],
                r.requested_speed_bits,
                r.speed_factor_bits,
            ],
        );
        self.mutate("BeginMover", state, actor);
        self.input.begin_result
    }
    fn next_world_random_u32(&mut self, state: &mut MoveState, actor: &mut ActorState) -> u32 {
        self.event("RandomDraw", &[]);
        self.mutate("RandomDraw", state, actor);
        self.input.random_u32
    }
    fn schedule(&mut self, delay: u32, state: &mut MoveState, actor: &mut ActorState) {
        self.event("Schedule", &[delay, 0]);
        self.mutate("Schedule", state, actor);
    }
}


/// Run explicitly pinned original-executable samples through Rust leaf functions.
/// `document` is the bundled oracle selection, not an untrusted user payload.
pub fn check_navigation(document: &Value) -> anyhow::Result<Vec<Value>> {
    let fixtures = document["fixtures"].as_array()
        .ok_or_else(|| anyhow::anyhow!("navigation bundle requires fixtures"))?;
    let mut rows = Vec::with_capacity(fixtures.len());
    for row in fixtures {
        let c: Input = serde_json::from_value(row["input"].clone())?;
        let mut state = c.state.clone();
        let mut actor = c.actor.clone();
        let mut world = World { input: c.clone(), events: vec![] };
        let result = match c.operation.as_str() {
            "activate" => activate_move(&mut state, &mut actor, c.activation, &mut world),
            "move_event" | "formation_event" => move_path_event(
                &mut state, &mut actor, &c.source, c.speed_bits,
                c.operation == "formation_event", &mut world)?,
            "action" => action_go(&mut state, &mut actor, c.speed_bits,
                ActionContext {
                    precision: c.precision,
                    group_speed_minimum_bits: c.group_speed_minimum_bits,
                    catchup_speed_factor_bits: c.catchup_speed_factor_bits,
                }, &mut world)?,
            operation => anyhow::bail!("unsupported pinned navigation sample {operation}"),
        };
        let actual = json!({
            "state": state, "actor": actor, "events": world.events,
            "native_success": result, "source_after": c.source,
        });
        let expected = json!({
            "state": row["output"]["state"],
            "actor": row["output"]["actor"],
            "events": row["output"]["events"],
            "native_success": row["output"]["native_success"],
            "source_after": row["output"]["source_after"],
        });
        rows.push(json!({"name": c.name, "passed": actual == expected,
            "actual": actual, "expected": expected}));
    }
    Ok(rows)
}
