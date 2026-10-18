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

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase5-navigation/native-navigation-fixtures.json"
    ))
    .unwrap()
}

#[test]
fn original_path_events_and_action_go_match_all_native_cases() {
    let doc = fixture();
    assert_eq!(doc["passed"], true);
    for row in doc["fixtures"].as_array().unwrap() {
        let c: Input = serde_json::from_value(row["input"].clone()).unwrap();
        let mut state = c.state.clone();
        let mut actor = c.actor.clone();
        let mut world = World {
            input: c.clone(),
            events: vec![],
        };
        let result = match c.operation.as_str() {
            "copy" => {
                state.path.assign_from(&c.source).unwrap();
                None
            }
            "self_copy" => None, // Rust borrowing statically excludes mutable self-alias.
            "clear" => {
                state.path.clear_nodes();
                None
            }
            "prune" => {
                state.path.prune_kind2_tail().unwrap();
                None
            }
            "activate" => Some(activate_move(
                &mut state,
                &mut actor,
                c.activation,
                &mut world,
            )),
            "move_event" | "formation_event" => Some(
                move_path_event(
                    &mut state,
                    &mut actor,
                    &c.source,
                    c.speed_bits,
                    c.operation == "formation_event",
                    &mut world,
                )
                .unwrap(),
            ),
            "action" => Some(
                action_go(
                    &mut state,
                    &mut actor,
                    c.speed_bits,
                    ActionContext {
                        precision: c.precision,
                        group_speed_minimum_bits: c.group_speed_minimum_bits,
                        catchup_speed_factor_bits: c.catchup_speed_factor_bits,
                    },
                    &mut world,
                )
                .unwrap(),
            ),
            op => panic!("unknown {op}"),
        };
        assert_eq!(
            serde_json::to_value(&state).unwrap(),
            row["output"]["state"],
            "{} state",
            c.name
        );
        assert_eq!(
            serde_json::to_value(&actor).unwrap(),
            row["output"]["actor"],
            "{} actor",
            c.name
        );
        assert_eq!(
            world.events,
            row["output"]["events"].as_array().unwrap().clone(),
            "{} events",
            c.name
        );
        if let Some(r) = result {
            assert_eq!(
                r,
                row["output"]["native_success"].as_bool().unwrap(),
                "{} result",
                c.name
            );
        }
        assert_eq!(
            serde_json::to_value(&c.source).unwrap(),
            row["output"]["source_after"],
            "{} source ownership",
            c.name
        );
    }
}

#[test]
fn owned_copy_is_independent_and_clear_retains_metadata() {
    let c: Input = serde_json::from_value(fixture()["fixtures"][0]["input"].clone()).unwrap();
    let mut dest = c.state.path;
    let mut src = c.source;
    dest.assign_from(&src).unwrap();
    let copied = dest.clone();
    src.nodes[0].words.fill(0x12345678);
    assert_eq!(dest, copied);
    dest.clear_nodes();
    assert!(dest.nodes.is_empty());
    assert_eq!(dest.kind1_count, copied.kind1_count);
    assert_eq!(dest.kind2_count, copied.kind2_count);
    assert_eq!(dest.word18, copied.word18);
    assert_eq!(dest.dirty, copied.dirty);
}

#[test]
fn invalid_native_pointer_domains_are_errors_not_invented_routes() {
    let c: Input = serde_json::from_value(fixture()["fixtures"][0]["input"].clone()).unwrap();
    let mut bad = c.source.clone();
    bad.nodes.clear();
    bad.kind2_count = 1;
    assert!(matches!(
        bad.prune_kind2_tail(),
        Err(Error::InvalidPathCount)
    ));
    let mut state = c.state.clone();
    let mut actor = c.actor.clone();
    let mut world = World {
        input: c.clone(),
        events: vec![],
    };
    assert!(matches!(
        move_path_event(
            &mut state,
            &mut actor,
            &bad,
            c.speed_bits,
            false,
            &mut world
        ),
        Err(Error::EmptyEventPath)
    ));
    assert_eq!(
        world.events,
        vec![json!({"name":"SpecialStateActive","bits":[]})]
    );
    // An undefined/nonfinite node is rejected after native instant-turn write.
    state.path.nodes[0].words[0] = 0x7fc00000;
    state.path.nodes[0].words[4] = 0;
    assert!(
        action_go(
            &mut state,
            &mut actor,
            c.speed_bits,
            ActionContext {
                precision: c.precision,
                group_speed_minimum_bits: c.group_speed_minimum_bits,
                catchup_speed_factor_bits: c.catchup_speed_factor_bits
            },
            &mut world
        )
        .is_err()
    );
    assert!(actor.mover_instant_turn);
}

struct FallibleWorld {
    inner: World,
    fail_at: Option<usize>,
    calls: usize,
    at_failure: Option<(MoveState, ActorState)>,
}
impl FallibleWorld {
    fn after<T>(&mut self, value: T, state: &MoveState, actor: &ActorState) -> Result<T> {
        let index = self.calls;
        self.calls += 1;
        if self.fail_at == Some(index) {
            self.at_failure = Some((state.clone(), actor.clone()));
            return Err(Error::World("injected live callback failure".into()));
        }
        Ok(value)
    }
}
impl TryActionWorld for FallibleWorld {
    fn node_reached(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()> {
        self.inner.node_reached(state, actor);
        self.after((), state, actor)
    }
    fn single_node_placement(
        &mut self,
        waypoint: [u32; 2],
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool> {
        let value = self.inner.single_node_placement(waypoint, state, actor);
        self.after(value, state, actor)
    }
    fn actor_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool> {
        let value = self.inner.actor_is_moving(state, actor);
        self.after(value, state, actor)
    }
    fn formation_ready(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool> {
        let value = self.inner.formation_ready(state, actor);
        self.after(value, state, actor)
    }
    fn actor_or_leader_handle_valid(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool> {
        let value = self.inner.actor_or_leader_handle_valid(state, actor);
        self.after(value, state, actor)
    }
    fn organization_ready(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool> {
        let value = self.inner.organization_ready(state, actor);
        self.after(value, state, actor)
    }
    fn leader_is_moving(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<bool> {
        let value = self.inner.leader_is_moving(state, actor);
        self.after(value, state, actor)
    }
    fn stop_action(&mut self, state: &mut MoveState, actor: &mut ActorState) -> Result<()> {
        self.inner.stop_action(state, actor);
        self.after((), state, actor)
    }
    fn begin_mover(
        &mut self,
        request: MoverRequest,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<bool> {
        let value = self.inner.begin_mover(request, state, actor);
        self.after(value, state, actor)
    }
    fn next_world_random_u32(
        &mut self,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<u32> {
        let value = self.inner.next_world_random_u32(state, actor);
        self.after(value, state, actor)
    }
    fn schedule(
        &mut self,
        delay_bits: u32,
        state: &mut MoveState,
        actor: &mut ActorState,
    ) -> Result<()> {
        self.inner.schedule(delay_bits, state, actor);
        self.after((), state, actor)
    }
}
#[test]
fn fallible_action_matches_native_and_stops_at_each_callback_failure() {
    let doc = fixture();
    let mut checked = 0;
    for row in doc["fixtures"].as_array().unwrap() {
        let c: Input = serde_json::from_value(row["input"].clone()).unwrap();
        if c.operation != "action" {
            continue;
        }
        let expected_events = row["output"]["events"].as_array().unwrap();
        for fail_at in std::iter::once(None).chain((0..expected_events.len()).map(Some)) {
            let mut state = c.state.clone();
            let mut actor = c.actor.clone();
            let mut world = FallibleWorld {
                inner: World {
                    input: c.clone(),
                    events: vec![],
                },
                fail_at,
                calls: 0,
                at_failure: None,
            };
            let result = try_action_go(
                &mut state,
                &mut actor,
                c.speed_bits,
                ActionContext {
                    precision: c.precision,
                    group_speed_minimum_bits: c.group_speed_minimum_bits,
                    catchup_speed_factor_bits: c.catchup_speed_factor_bits,
                },
                &mut world,
            );
            if let Some(index) = fail_at {
                assert!(
                    matches!(result, Err(Error::World(_))),
                    "{} callback{index}",
                    c.name
                );
                assert_eq!(
                    world.inner.events,
                    expected_events[..=index],
                    "{} callback order",
                    c.name
                );
                assert_eq!(
                    (state, actor),
                    world.at_failure.unwrap(),
                    "{} no writes after failure",
                    c.name
                );
                checked += 1;
            } else {
                assert_eq!(
                    result.unwrap(),
                    row["output"]["native_success"].as_bool().unwrap()
                );
                assert_eq!(serde_json::to_value(state).unwrap(), row["output"]["state"]);
                assert_eq!(serde_json::to_value(actor).unwrap(), row["output"]["actor"]);
                assert_eq!(&world.inner.events, expected_events);
            }
        }
    }
    assert!(
        checked > 100,
        "every reached callback boundary must be tested"
    );
}
