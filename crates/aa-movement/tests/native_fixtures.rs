use aa_movement::*;
use aa_numeric::Precision;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Clone, Deserialize)]
struct Input {
    name: String,
    operation: String,
    mover: MoverState,
    actor: ActorMotion,
    context: StepContext,
    precision_radius: f32,
    activation: bool,
    collision: bool,
    target_valid: bool,
    target_position: [f32; 2],
    shape_in_range: bool,
    notify_result: bool,
    order: GroundOrder,
    keep_scheduled: bool,
    reset_speed: bool,
}
#[derive(Deserialize)]
struct Fixture {
    input: Input,
    output: Value,
}
#[derive(Deserialize)]
struct Document {
    passed: bool,
    case_count: usize,
    fixtures: Vec<Fixture>,
}

struct World {
    input: Input,
    events: Vec<Value>,
}
impl World {
    fn record(&mut self, name: &str, bits: &[u32]) {
        self.events.push(json!({"name": name, "bits": bits}));
    }
}
impl GroundWorld for World {
    type Collision = ();
    fn activate(&mut self, _: &mut MoverState, _: &mut ActorMotion) -> Result<bool> {
        self.record("Activate", &[]);
        Ok(self.input.activation)
    }
    fn target_position(&mut self, _: u32) -> Result<Option<[f32; 2]>> {
        Ok(self
            .input
            .target_valid
            .then_some(self.input.target_position))
    }
    fn shapes_in_range(&mut self, _: &ActorMotion, _: u32, _: f32) -> Result<bool> {
        Ok(self.input.shape_in_range)
    }
    fn collision_at(&mut self, _: &ActorMotion, xy: [f32; 2]) -> Result<Option<()>> {
        self.record("CollisionQuery", &xy.map(f32::to_bits));
        Ok(self.input.collision.then_some(()))
    }
    fn set_facing(
        &mut self,
        _: &mut MoverState,
        actor: &mut ActorMotion,
        xy: [f32; 2],
    ) -> Result<()> {
        self.record("SetFacing", &xy.map(f32::to_bits));
        if actor.flags & (1 << 17) == 0 {
            actor.facing = [xy[0], xy[1], 0.0];
        }
        Ok(())
    }
    fn move_on_ground(
        &mut self,
        _: &mut MoverState,
        actor: &mut ActorMotion,
        xy: [f32; 2],
    ) -> Result<()> {
        self.record("MoveOnGround", &xy.map(f32::to_bits));
        // Synthetic terrain seam deliberately queries old XY, as the actual
        // 0x4eeb40 helper did in the original instruction fixture.
        let height =
            (f64::from(actor.position[0]) * 0.125 + f64::from(actor.position[1]) * 0.25) as f32;
        actor.position = [xy[0], xy[1], height];
        Ok(())
    }
    fn notify(
        &mut self,
        event: Event,
        _: &mut MoverState,
        actor: &mut ActorMotion,
    ) -> Result<bool> {
        self.record(&format!("{event:?}"), &[]);
        if event == Event::CharacterStop {
            actor.character_stopped = true;
        }
        if event == Event::CharacterResume {
            actor.character_stopped = false;
        }
        Ok(self.input.notify_result)
    }
    fn blocked(
        &mut self,
        _: &(),
        facing: [f32; 2],
        _: &mut MoverState,
        _: &mut ActorMotion,
    ) -> Result<()> {
        self.record("Blocked", &facing.map(f32::to_bits));
        Ok(())
    }
}
struct NoTurning;
impl GroundTurning for NoTurning {
    fn signed_angle(&mut self, _: [f32; 2], _: [f32; 2], _: Precision) -> Result<f32> {
        Err(Error::Turning(
            "fixture unexpectedly needs recovered turning provider".into(),
        ))
    }
    fn rotate(&mut self, _: [f32; 2], _: f32, _: bool, _: Precision) -> Result<[f32; 2]> {
        Err(Error::Turning(
            "fixture unexpectedly needs recovered turning provider".into(),
        ))
    }
}
fn document() -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase4-movement/native-movement-fixtures.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn turning_document() -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase4-movement/native-movement-turning-fixtures.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn mover_bits(m: &MoverState) -> Value {
    json!({"destination": m.destination.map(f32::to_bits), "last_step_speed": [m.last_step_speed.to_bits()],
           "speed": [m.speed.to_bits()], "requested_speed": [m.requested_speed.to_bits()],
           "acceleration": [m.acceleration.to_bits()], "turn_rate": [m.turn_rate.to_bits()],
           "desired_direction": m.desired_direction.map(f32::to_bits), "target_range": [m.target_range.map(f32::to_bits)],
           "speed_factor": [m.speed_factor.to_bits()], "target_id": m.target_id,
           "ignore_terrain_speed_limit": m.ignore_terrain_speed_limit, "instant_turn": m.instant_turn,
           "turning": m.turning, "falling": m.falling})
}

#[test]
fn complete_orders_and_straight_ground_steps_match_original_x86_bits_and_callbacks() {
    let document = document();
    assert!(document.passed);
    assert_eq!(document.case_count, document.fixtures.len());
    for fixture in document.fixtures {
        let input = fixture.input;
        let mut mover = input.mover.clone();
        let mut actor = input.actor.clone();
        let mut world = World {
            input: input.clone(),
            events: Vec::new(),
        };
        let result = match input.operation.as_str() {
            "begin" => begin_ground_motion(
                &mut mover,
                &mut actor,
                input.order,
                input.precision_radius,
                input.context.precision,
                &mut world,
            )
            .unwrap(),
            "stop" => {
                stop_ground_motion(
                    &mut mover,
                    &mut actor,
                    input.keep_scheduled,
                    input.reset_speed,
                    &mut world,
                )
                .unwrap();
                true
            }
            _ => {
                ground_step(
                    &mut mover,
                    &mut actor,
                    input.context,
                    &mut world,
                    &mut NoTurning,
                )
                .unwrap()
                .native_success
            }
        };
        let actual = json!({"mover_bits": mover_bits(&mover), "position_bits": actor.position.map(f32::to_bits),
                            "facing_bits": actor.facing.map(f32::to_bits), "character_stopped": actor.character_stopped,
                            "native_success": result, "events": world.events, "abi_preserved": true});
        assert_eq!(actual, fixture.output, "native case {}", input.name);
    }
}

#[test]
fn constructor_unwritten_range_is_only_required_on_native_read_paths() {
    let reads: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-actors/mover-range-reads.json"
    ))
    .unwrap();
    assert_eq!(reads["native_executions"], 540);
    let cases = reads["cases"].as_array().unwrap();
    let fixtures = document().fixtures;
    let mut unread = 0;
    for row in cases {
        let fixture = fixtures
            .iter()
            .find(|f| f.input.name == row["input"]["name"])
            .unwrap();
        let input = &fixture.input;
        let mut mover = input.mover.clone();
        mover.target_range = None;
        let mut actor = input.actor.clone();
        let mut world = World {
            input: input.clone(),
            events: Vec::new(),
        };
        let result = match input.operation.as_str() {
            "begin" => begin_ground_motion(
                &mut mover,
                &mut actor,
                input.order,
                input.precision_radius,
                input.context.precision,
                &mut world,
            ),
            "stop" => stop_ground_motion(
                &mut mover,
                &mut actor,
                input.keep_scheduled,
                input.reset_speed,
                &mut world,
            )
            .map(|()| true),
            _ => ground_step(
                &mut mover,
                &mut actor,
                input.context,
                &mut world,
                &mut NoTurning,
            )
            .map(|o| o.native_success),
        };
        if row["range_read_addresses"].as_array().unwrap().is_empty() {
            unread += 1;
            let success = result.unwrap();
            assert_eq!(
                row["poison_values_proven_unread"].as_array().unwrap().len(),
                2
            );
            let mut expected = fixture.output.clone();
            expected["mover_bits"]["target_range"] = json!([null]);
            let actual = json!({"mover_bits":mover_bits(&mover),"position_bits":actor.position.map(f32::to_bits),"facing_bits":actor.facing.map(f32::to_bits),"character_stopped":actor.character_stopped,"native_success":success,"events":world.events,"abi_preserved":true});
            assert_eq!(actual, expected, "{}", input.name);
        } else {
            assert!(
                matches!(result, Err(Error::UninitializedTargetRange)),
                "{}",
                input.name
            );
        }
    }
    assert_eq!(unread, 177);
}

#[test]
fn invalid_or_wrong_mode_inputs_do_not_silently_invoke_an_invented_world() {
    let input = document().fixtures.remove(0).input;
    let mut world = World {
        input: input.clone(),
        events: Vec::new(),
    };
    let mut mover = input.mover.clone();
    let mut actor = input.actor.clone();
    mover.falling = true;
    assert!(matches!(
        ground_step(
            &mut mover,
            &mut actor,
            input.context,
            &mut world,
            &mut NoTurning
        ),
        Err(Error::Falling)
    ));
    assert!(world.events.is_empty());
    mover.falling = false;
    mover.destination[0] = f32::NAN;
    assert!(matches!(
        ground_step(
            &mut mover,
            &mut actor,
            input.context,
            &mut world,
            &mut NoTurning
        ),
        Err(Error::NonFinite)
    ));
    assert!(world.events.is_empty());
}

#[test]
fn pc53_concrete_original_turning_matches_complete_native_ground_steps() {
    let document = turning_document();
    assert!(document.passed);
    let mut count = 0;
    for fixture in document.fixtures {
        let input = fixture.input;
        if input.context.precision != Precision::Double53 || input.operation != "step" {
            continue;
        }
        let mut mover = input.mover.clone();
        let mut actor = input.actor.clone();
        let mut world = World {
            input: input.clone(),
            events: Vec::new(),
        };
        let result =
            ground_step_original_pc53(&mut mover, &mut actor, input.context, &mut world).unwrap();
        let actual = json!({"mover_bits": mover_bits(&mover), "position_bits": actor.position.map(f32::to_bits),
                            "facing_bits": actor.facing.map(f32::to_bits), "character_stopped": actor.character_stopped,
                            "native_success": result.native_success, "events": world.events, "abi_preserved": true});
        let mut expected = fixture.output;
        // Diagnostic emulator status is not a reliable real-CPU sticky PE proof.
        expected.as_object_mut().unwrap().remove("turning_trace");
        assert_eq!(
            actual, expected,
            "native concrete turning case {}",
            input.name
        );
        count += 1;
    }
    assert_eq!(count, 90);
}

#[test]
fn concrete_provider_rejects_other_precision_before_world_or_state_mutation() {
    for pc in [Precision::Single24, Precision::Extended64] {
        let mut input = document().fixtures.remove(0).input;
        input.context.precision = pc;
        let mut world = World {
            input: input.clone(),
            events: Vec::new(),
        };
        let mut mover = input.mover.clone();
        let mut actor = input.actor.clone();
        assert!(matches!(
            ground_step_original_pc53(&mut mover, &mut actor, input.context, &mut world),
            Err(Error::UnsupportedOriginalTurningPrecision)
        ));
        assert_eq!(mover, input.mover);
        assert_eq!(actor, input.actor);
        assert!(world.events.is_empty());
    }
}

#[derive(Clone, Copy)]
enum Failure {
    Collision,
    GroundBefore,
    GroundAfter,
    Cancel,
}
struct FallibleWorld {
    inner: World,
    failure: Failure,
}
impl GroundWorld for FallibleWorld {
    type Collision = ();
    fn activate(&mut self, m: &mut MoverState, a: &mut ActorMotion) -> Result<bool> {
        self.inner.activate(m, a)
    }
    fn target_position(&mut self, id: u32) -> Result<Option<[f32; 2]>> {
        self.inner.target_position(id)
    }
    fn shapes_in_range(&mut self, a: &ActorMotion, id: u32, range: f32) -> Result<bool> {
        self.inner.shapes_in_range(a, id, range)
    }
    fn collision_at(&mut self, a: &ActorMotion, xy: [f32; 2]) -> Result<Option<()>> {
        if matches!(self.failure, Failure::Collision) {
            return Err(Error::World("collision query failed".into()));
        }
        self.inner.collision_at(a, xy)
    }
    fn set_facing(&mut self, m: &mut MoverState, a: &mut ActorMotion, xy: [f32; 2]) -> Result<()> {
        self.inner.set_facing(m, a, xy)
    }
    fn move_on_ground(
        &mut self,
        m: &mut MoverState,
        a: &mut ActorMotion,
        xy: [f32; 2],
    ) -> Result<()> {
        if matches!(self.failure, Failure::GroundBefore) {
            return Err(Error::World("height query failed".into()));
        }
        self.inner.move_on_ground(m, a, xy)?;
        if matches!(self.failure, Failure::GroundAfter) {
            return Err(Error::World("position notification failed".into()));
        }
        Ok(())
    }
    fn notify(&mut self, event: Event, m: &mut MoverState, a: &mut ActorMotion) -> Result<bool> {
        let result = self.inner.notify(event, m, a)?;
        if matches!(self.failure, Failure::Cancel) && event == Event::CancelScheduled {
            return Err(Error::World("cancel callback failed".into()));
        }
        Ok(result)
    }
    fn blocked(
        &mut self,
        c: &(),
        f: [f32; 2],
        m: &mut MoverState,
        a: &mut ActorMotion,
    ) -> Result<()> {
        self.inner.blocked(c, f, m, a)
    }
}

#[test]
fn fallible_world_stops_scheduling_after_partial_ground_commit() {
    let input = document()
        .fixtures
        .into_iter()
        .find(|f| f.input.name == "cross_destination_Double53")
        .unwrap()
        .input;
    for failure in [Failure::GroundBefore, Failure::GroundAfter] {
        let mut mover = input.mover.clone();
        let mut actor = input.actor.clone();
        let mut world = FallibleWorld {
            inner: World {
                input: input.clone(),
                events: Vec::new(),
            },
            failure,
        };
        assert!(matches!(
            ground_step(
                &mut mover,
                &mut actor,
                input.context,
                &mut world,
                &mut NoTurning
            ),
            Err(Error::World(_))
        ));
        assert!(
            !world
                .inner
                .events
                .iter()
                .any(|e| e["name"] == "CancelScheduled" || e["name"] == "DestinationReached")
        );
        if matches!(failure, Failure::GroundBefore) {
            assert_eq!(actor.position, input.actor.position);
        } else {
            assert_ne!(actor.position, input.actor.position);
            assert_eq!(world.inner.events.last().unwrap()["name"], "MoveOnGround");
        }
    }
}

#[test]
fn collision_and_notification_failures_preserve_prior_effects_without_fake_success() {
    for (name, failure) in [
        ("accelerate_Double53", Failure::Collision),
        ("already_at_destination_Double53", Failure::Cancel),
    ] {
        let input = document()
            .fixtures
            .into_iter()
            .find(|f| f.input.name == name)
            .unwrap()
            .input;
        let mut mover = input.mover.clone();
        let mut actor = input.actor.clone();
        let mut world = FallibleWorld {
            inner: World {
                input: input.clone(),
                events: Vec::new(),
            },
            failure,
        };
        assert!(matches!(
            ground_step(
                &mut mover,
                &mut actor,
                input.context,
                &mut world,
                &mut NoTurning
            ),
            Err(Error::World(_))
        ));
        assert_eq!(actor.position, input.actor.position);
        assert!(
            !world
                .inner
                .events
                .iter()
                .any(|e| e["name"] == "MoveOnGround" || e["name"] == "DestinationReached")
        );
        if matches!(failure, Failure::Collision) {
            assert_ne!(mover.speed, input.mover.speed);
        } else {
            assert_eq!(mover.speed, 0.0);
            assert_eq!(
                world.inner.events.last().unwrap()["name"],
                "CancelScheduled"
            );
        }
    }
}

#[test]
fn explicit_status_original_turning_matches_pc24_and_pc64_native_ground_steps() {
    let document = turning_document();
    let mut count = 0;
    for fixture in document.fixtures {
        let input = fixture.input;
        if input.context.precision == Precision::Double53 || input.operation != "step" {
            continue;
        }
        // The recorded 0x445400-entry status from the original execution.
        let pending = fixture.output["turning_trace"]
            .as_array()
            .map(|trace| {
                trace
                    .iter()
                    .filter(|t| t["entry"] == "0x445400")
                    .any(|t| t["precision_flag"] == true)
            })
            .unwrap_or(false);
        let mut mover = input.mover.clone();
        let mut actor = input.actor.clone();
        let mut world = World {
            input: input.clone(),
            events: Vec::new(),
        };
        let result =
            ground_step_original(&mut mover, &mut actor, input.context, pending, &mut world).unwrap();
        let actual = json!({"mover_bits": mover_bits(&mover), "position_bits": actor.position.map(f32::to_bits),
                            "facing_bits": actor.facing.map(f32::to_bits), "character_stopped": actor.character_stopped,
                            "native_success": result.native_success, "events": world.events, "abi_preserved": true});
        let mut expected = fixture.output;
        expected.as_object_mut().unwrap().remove("turning_trace");
        assert_eq!(actual, expected, "explicit-status turning case {}", input.name);
        count += 1;
    }
    assert_eq!(count, 180);
}
