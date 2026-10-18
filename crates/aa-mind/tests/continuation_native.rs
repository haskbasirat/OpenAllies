use aa_mind::element_move::Payload;
use aa_mind::element_move_continuation::waypoint_offset;
use aa_navigation::result_plan;
use serde_json::Value;
#[test]
fn whole_original_waypoint_offsets_match_x87_and_stores() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-continuation/native-offset.json"
    ))
    .unwrap();
    assert_eq!(doc["passed"], true);
    for row in doc["cases"].as_array().unwrap() {
        let movement = serde_json::from_value(row["state"].clone()).unwrap();
        let mut p = Payload {
            plan: result_plan::State {
                movement,
                attempts: 0,
                previous_destination_bits: [0, 0],
            },
            collision: [0; 4],
            collision_vector: [0; 2],
            detached_time_bits: 0,
            word78: 0,
        };
        waypoint_offset(
            &mut p,
            row["radius"].as_u64().unwrap() as u32,
            serde_json::from_value(row["precision"].clone()).unwrap(),
            row["pending"].as_bool().unwrap(),
        )
        .unwrap();
        assert_eq!(
            p.plan.movement.waypoint_offset_bits,
            row["output"].as_u64().unwrap() as u32,
            "{row}"
        );
    }
}

use aa_mind::element_move_continuation::{self as kernel, Actor, Mover, Obstacle, World};
use aa_mind::{Arena, Error, Mind, MindId, Result, State, StateId};
use aa_numeric::{Binary, Precision};
use serde_json::json;
struct Fixture {
    c: Value,
    events: Vec<Value>,
    obstacle: Option<Obstacle>,
    fail_at: Option<usize>,
    stopped: Option<Value>,
    recovery: Option<kernel::TurnRecovery>,
}
impl Fixture {
    fn n(&self, k: &str) -> u32 {
        self.c[k].as_u64().unwrap() as u32
    }
    fn flag(&self, k: &str) -> bool {
        self.c[k].as_bool().unwrap()
    }
    fn ev(&mut self, name: &str, bits: &[u32], p: &mut Payload) -> Result<()> {
        self.events.push(json!({"name":name,"bits":bits}));
        let m = self.c["changes"][name].clone();
        if let Some(v) = m["phase"].as_u64() {
            p.plan.movement.phase = v as u32;
        }
        if let Some(v) = m["head_kind"].as_u64() {
            p.plan.movement.path.nodes[0].words[4] = v as u32;
        }
        if m["clear_path"] == true {
            p.plan.movement.path.clear_nodes();
        }
        if let Some(v) = m.get("facing") {
            self.c["facing"] = v.clone();
        }
        if self.fail_at == Some(self.events.len() - 1) {
            self.stopped = Some(serde_json::to_value(p).unwrap());
            return Err(Error::World("injected callback failure".into()));
        }
        Ok(())
    }
}
impl World for Fixture {
    fn actor(&self, id: u32) -> Result<Actor> {
        assert!(id == 1 || id == 2);
        Ok(Actor {
            flags: if id == 1 {
                self.c["actor"]["flags"].as_u64().unwrap() as u32
            } else {
                self.n("target_flags")
            },
            position: serde_json::from_value(if id == 1 {
                self.c["actor"]["position_bits"].clone()
            } else {
                self.c["target_position"].clone()
            })
            .unwrap(),
            facing: serde_json::from_value(self.c["facing"].clone()).unwrap(),
            mover: Some(Mover {
                turn_radius: self.n("radius"),
                requested_direction: serde_json::from_value(self.c["direction"].clone()).unwrap(),
            }),
        })
    }
    fn resolve(&self, id: u32) -> Result<Option<u32>> {
        Ok(if id == 0x10007 && self.flag("target_valid") {
            Some(2)
        } else {
            None
        })
    }
    fn world_step_bits(&self) -> Result<u32> {
        Ok(self.n("step"))
    }
    fn precision_exception_pending(&self) -> bool {
        false
    }
    fn actor_is_moving(&mut self, id: u32, p: &mut Payload, _: &mut Arena) -> Result<bool> {
        self.ev("Moving", &[id], p)?;
        Ok(self.flag("moving"))
    }
    fn action_go(
        &mut self,
        _: StateId,
        speed: u32,
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<bool> {
        self.ev("ActionGo", &[speed], p)?;
        Ok(self.flag("action_success"))
    }
    fn schedule(&mut self, _: MindId, delay: u32, p: &mut Payload, _: &mut Arena) -> Result<()> {
        self.ev("Schedule", &[delay, 0], p)
    }
    fn get_back_to_path(&mut self, _: StateId, p: &mut Payload, _: &mut Arena) -> Result<bool> {
        self.ev("BackToPath", &[], p)?;
        Ok(self.flag("back_result"))
    }
    fn movement_radius(&mut self, id: u32, p: &mut Payload, _: &mut Arena) -> Result<Binary> {
        self.ev("MovementRadius", &[id], p)?;
        Ok(Binary::from_f32_bits(self.n("movement_radius")).unwrap())
    }
    fn forward_probe(
        &mut self,
        _: StateId,
        direction: [u32; 2],
        distance: u32,
        result: &mut [u32; 4],
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<()> {
        self.ev("ForwardProbe", &[direction[0], direction[1], distance], p)?;
        *result = serde_json::from_value(self.c["probe_result"].clone()).unwrap();
        Ok(())
    }
    fn node_reached(&mut self, _: MindId, p: &mut Payload, _: &mut Arena) -> Result<()> {
        self.ev("NodeReached", &[], p)
    }
    fn avoid_actor(
        &mut self,
        _: StateId,
        target: u32,
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<bool> {
        self.ev("Avoid", &[target], p)?;
        Ok(self.flag("avoid"))
    }
    fn refresh_visibility(&mut self, id: u32, p: &mut Payload, _: &mut Arena) -> Result<()> {
        self.ev("Refresh", &[id], p)
    }
    fn overlaps_actor(
        &mut self,
        id: u32,
        target: u32,
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<bool> {
        self.ev("Overlap", &[id, target], p)?;
        Ok(self.flag("overlap"))
    }
    fn overlap_diagnostic(
        &mut self,
        id: u32,
        target: u32,
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<()> {
        self.ev("Diagnostic", &[id, target], p)
    }
    fn bounds_radius(&mut self, id: u32, p: &mut Payload, _: &mut Arena) -> Result<Binary> {
        self.ev("BoundsRadius", &[id], p)?;
        Ok(Binary::from_f32_bits(self.n(if id == 1 {
            "own_radius"
        } else {
            "target_radius"
        }))
        .unwrap())
    }
    fn stop_distance(
        &mut self,
        _: StateId,
        target: u32,
        distance: u32,
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<()> {
        self.ev("StopDistance", &[target, distance], p)
    }
    fn result_path(&mut self, _: StateId, p: &mut Payload, _: &mut Arena) -> Result<u32> {
        self.ev("ResultPath", &[], p)?;
        Ok(self.n("path_result"))
    }
    fn wait(&mut self, _: StateId, p: &mut Payload, _: &mut Arena) -> Result<()> {
        self.ev("Wait", &[], p)
    }
    fn collision_plan(
        &mut self,
        _: StateId,
        obstacle: Obstacle,
        direction: [u32; 2],
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<()> {
        self.obstacle = Some(obstacle);
        self.ev("CollisionPlan", &direction, p)
    }
}
impl kernel::TurnRecoveryWorld for Fixture {
    fn turn_recovery(
        &mut self,
        _: StateId,
        input: kernel::TurnRecovery,
        p: &mut Payload,
        _a: &mut Arena,
    ) -> Result<bool> {
        if self.c["op"] == "fullback" {
            let pc = serde_json::from_value(self.c["precision"].clone()).unwrap();
            return kernel::recover_turn(StateId(1), input, p, _a, pc, self);
        }
        self.recovery = Some(input);
        self.ev("TurnRecovery", &[], p)?;
        Ok(self.flag("back_result"))
    }
}

impl kernel::RecoveryGeometryWorld for Fixture {
    fn recovery_placement_blocked(
        &mut self,
        _: StateId,
        endpoint: [u32; 2],
        direction: [u32; 2],
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<bool> {
        self.ev(
            "RecoveryPlacement",
            &[endpoint[0], endpoint[1], direction[0], direction[1]],
            p,
        )?;
        Ok(self.c["placement_blocked"] == true)
    }
    fn recovery_probe(
        &mut self,
        _: StateId,
        origin: [u32; 2],
        direction: [u32; 2],
        distance: u32,
        result: &mut [u32; 4],
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<bool> {
        self.ev(
            "RecoveryProbe",
            &[origin[0], origin[1], direction[0], direction[1], distance],
            p,
        )?;
        *result = serde_json::from_value(self.c["probe_result"].clone()).unwrap();
        Ok(self.c["recovery_probe_blocked"] == true)
    }
    fn recovery_diagnostic(
        &mut self,
        actor: u32,
        distance: u32,
        offset: u32,
        p: &mut Payload,
        _: &mut Arena,
    ) -> Result<()> {
        self.ev("RecoveryDiagnostic", &[actor, distance, offset], p)
    }
}

fn setup(c: &Value) -> (Payload, Arena, Fixture) {
    let p = Payload {
        plan: result_plan::State {
            movement: serde_json::from_value(c["state"].clone()).unwrap(),
            attempts: 0,
            previous_destination_bits: [0; 2],
        },
        collision: serde_json::from_value(c["collision"].clone()).unwrap(),
        collision_vector: serde_json::from_value(c["collision_vector"].clone()).unwrap(),
        detached_time_bits: 0,
        word78: 0,
    };
    let mut a = Arena::default();
    a.minds.insert(MindId(1), Mind::base(1));
    a.states.insert(
        StateId(1),
        State {
            parent: MindId(1),
            discard_on_suspend: false,
            status: 2,
        },
    );
    (
        p,
        a,
        Fixture {
            c: c.clone(),
            events: vec![],
            obstacle: None,
            fail_at: None,
            stopped: None,
            recovery: None,
        },
    )
}
fn execute(c: &Value, p: &mut Payload, a: &mut Arena, w: &mut Fixture) -> Result<()> {
    let pc: Precision = serde_json::from_value(c["precision"].clone()).unwrap();
    match c["op"].as_str().unwrap() {
        "continue" => kernel::continue_path(StateId(1), p, a, pc, w),
        "forward" => kernel::forward_check(StateId(1), p, a, w.n("distance"), pc, w),
        "collision" => kernel::collision_result(StateId(1), p, a, pc, w),
        "save" => kernel::save_collision(p, p.collision, w.actor(1)?.facing, pc),
        "back" | "fullback" => {
            let value = kernel::get_back_to_path(StateId(1), p, a, pc, w)?;
            w.c["success"] = json!(value);
            Ok(())
        }
        _ => panic!(),
    }
}
#[test]
fn original_continuation_collision_decisions_and_partial_failures_match() {
    let doc: Value = serde_json::from_str(include_str!(
        "../../../research/phase7-continuation/native-continuation.json"
    ))
    .unwrap();
    for row in doc["cases"].as_array().unwrap() {
        let c = &row["input"];
        let (p0, _, _) = setup(c);
        let (mut p, mut a, mut w) = setup(c);
        execute(c, &mut p, &mut a, &mut w).unwrap();
        let mut actual = json!({"events":w.events,"phase":p.plan.movement.phase,"offset":p.plan.movement.waypoint_offset_bits,"collision":p.collision,"collision_vector":p.collision_vector,"path":p.plan.movement.path,"facing":w.c["facing"],"obstacle":w.obstacle});
        if c["op"] == "back" || c["op"] == "fullback" {
            actual["recovery"] = json!(w.recovery);
            actual["success"] = w.c["success"].clone();
        }
        assert_eq!(actual, row["output"], "{}", c["name"]);
        assert_eq!(p.plan.movement.target_id, p0.plan.movement.target_id);
        for index in 0..w.events.len() {
            let (mut p, mut a, mut bad) = setup(c);
            bad.fail_at = Some(index);
            assert!(execute(c, &mut p, &mut a, &mut bad).is_err());
            assert_eq!(bad.events, w.events[..=index]);
            assert_eq!(
                serde_json::to_value(&p).unwrap(),
                bad.stopped.unwrap(),
                "no post-error writes {} callback{index}",
                c["name"]
            );
        }
    }
}
