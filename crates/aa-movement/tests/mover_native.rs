//! Replays research/phase8-movement/native-mover.json: original x86 execution
//! of Tick 577E50, 5768A0/5768F0, 577330, 4D03D0, 5767F0, 576B20, 576C70 /
//! 576850, 576740, 5770A0, 577A70 / 576BB0 / 5766A0 / 552640 / 4453A0,
//! 576700 and 577380 under PC24/53/64. Every recorded callback, its argument
//! bits and every Mover word/byte must match.
use aa_movement::falling::{self, FallContext, FallingState, FallingWorld, MapBounds};
use aa_movement::mover::{self, MaxSpeedWorld, MoverDefinition, MoverLimits, RestoreWorld, TickGate,
    TickWorld};
use aa_movement::*;
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::BTreeMap;

fn document() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-movement/native-mover.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn bits3(v: &Value) -> [f32; 3] {
    let a = v.as_array().unwrap();
    [f(u(&a[0])), f(u(&a[1])), f(u(&a[2]))]
}
fn precision(v: &Value) -> Precision {
    serde_json::from_value(v.clone()).unwrap()
}
fn f64_binary(hex: &str) -> Binary {
    Binary::from_f64_bits(u64::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap()).unwrap()
}
fn f80_binary(hex: &str) -> Binary {
    let raw: Vec<u8> = (0..10).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
    let mantissa = u64::from_le_bytes(raw[..8].try_into().unwrap());
    let top = u16::from_le_bytes([raw[8], raw[9]]);
    let p = Precision::Extended64;
    let mut value = Binary::from_u32((mantissa >> 32) as u32)
        .mul(&Binary::from_f64_bits(0x41f0_0000_0000_0000).unwrap(), p)
        .unwrap()
        .add(&Binary::from_u32(mantissa as u32), p)
        .unwrap();
    if mantissa != 0 {
        let mut shift = (top & 0x7fff) as i32 - 16383 - 63;
        while shift != 0 {
            let step = shift.clamp(-1000, 1000);
            let scale = f64::from_bits(((1023 + step) as u64) << 52);
            value = value.mul(&Binary::from_f64_bits(scale.to_bits()).unwrap(), p).unwrap();
            shift -= step;
        }
    }
    if top & 0x8000 != 0 { value.negated() } else { value }
}

#[derive(Default, Clone)]
struct Memory {
    words: BTreeMap<u32, u32>,
    bytes: BTreeMap<u32, u8>,
}
impl Memory {
    fn load(mover: &Value) -> Self {
        let mut m = Memory::default();
        for pair in mover["words"].as_array().unwrap() {
            m.words.insert(u(&pair[0]), u(&pair[1]));
        }
        for pair in mover["bytes"].as_array().unwrap() {
            m.bytes.insert(u(&pair[0]), u(&pair[1]) as u8);
        }
        m
    }
    fn w(&self, o: u32) -> u32 {
        *self.words.get(&o).unwrap_or(&0)
    }
    fn b(&self, o: u32) -> u8 {
        *self.bytes.get(&o).unwrap_or(&0)
    }
    fn split(&self) -> (MoverState, MoverLimits, FallingState) {
        let v = |o| f(self.w(o));
        (
            MoverState {
                destination: [v(0x10), v(0x14), v(0x18)],
                last_step_speed: v(0x1c),
                speed: v(0x20),
                requested_speed: v(0x24),
                acceleration: v(0x2c),
                turn_rate: v(0x34),
                desired_direction: [v(0x38), v(0x3c), v(0x40)],
                target_id: self.w(0x44),
                target_range: Some(v(0x48)),
                ignore_terrain_speed_limit: self.b(0x4c) != 0,
                speed_factor: v(0x50),
                instant_turn: self.b(0x54) != 0,
                turning: self.b(0x55) != 0,
                falling: self.b(0x56) != 0,
            },
            MoverLimits { max_speed: v(0x28), turn_radius: v(0x30) },
            FallingState {
                drag: v(0x58),
                orient_to_motion: self.b(0x5c),
                direction: [v(0x60), v(0x64), v(0x68)],
                aimed: self.b(0x6c),
                aim_point: [v(0x70), v(0x74), v(0x78)],
                aim_delta: [v(0x7c), v(0x80), v(0x84)],
                outside_map: self.b(0x88),
            },
        )
    }
    /// Merge Rust fields back; untouched native offsets keep their input bits.
    fn merge(&self, m: &MoverState, l: &MoverLimits, fs: &FallingState) -> Memory {
        let mut out = self.clone();
        let mut put = |o: u32, x: f32| {
            out.words.insert(o, x.to_bits());
        };
        for (i, x) in m.destination.iter().enumerate() {
            put(0x10 + 4 * i as u32, *x);
        }
        put(0x1c, m.last_step_speed);
        put(0x20, m.speed);
        put(0x24, m.requested_speed);
        put(0x28, l.max_speed);
        put(0x2c, m.acceleration);
        put(0x30, l.turn_radius);
        put(0x34, m.turn_rate);
        for (i, x) in m.desired_direction.iter().enumerate() {
            put(0x38 + 4 * i as u32, *x);
        }
        put(0x48, m.target_range.unwrap());
        put(0x50, m.speed_factor);
        put(0x58, fs.drag);
        for (i, x) in fs.direction.iter().enumerate() {
            put(0x60 + 4 * i as u32, *x);
        }
        for (i, x) in fs.aim_point.iter().enumerate() {
            put(0x70 + 4 * i as u32, *x);
        }
        for (i, x) in fs.aim_delta.iter().enumerate() {
            put(0x7c + 4 * i as u32, *x);
        }
        out.words.insert(0x44, m.target_id);
        for (o, x) in [
            (0x4c, m.ignore_terrain_speed_limit as u8),
            (0x54, m.instant_turn as u8),
            (0x55, m.turning as u8),
            (0x56, m.falling as u8),
            (0x5c, fs.orient_to_motion),
            (0x6c, fs.aimed),
            (0x88, fs.outside_map),
        ] {
            out.bytes.insert(o, x);
        }
        out
    }
}

struct World {
    case: Value,
    responses: Vec<Value>,
    events: Vec<Value>,
}
impl World {
    fn new(case: &Value) -> Self {
        World { case: case.clone(), responses: case["responses"].as_array().unwrap().clone(), events: vec![] }
    }
    fn record(&mut self, name: &str, bits: &[u32]) {
        self.events.push(json!({"name": name, "bits": bits}));
    }
    fn response(&mut self, name: &str) -> Value {
        assert!(!self.responses.is_empty(), "unexpected {name}");
        let r = self.responses.remove(0);
        assert_eq!(r["name"], name);
        r
    }
}
impl GroundWorld for World {
    type Collision = ();
    fn activate(&mut self, _: &mut MoverState, _: &mut ActorMotion) -> Result<bool> {
        self.record("Activate", &[]);
        Ok(self.response("Activate")["value"].as_bool().unwrap())
    }
    fn target_position(&mut self, _: u32) -> Result<Option<[f32; 2]>> {
        unreachable!()
    }
    fn shapes_in_range(&mut self, _: &ActorMotion, _: u32, _: f32) -> Result<bool> {
        unreachable!()
    }
    fn collision_at(&mut self, _: &ActorMotion, _: [f32; 2]) -> Result<Option<()>> {
        unreachable!()
    }
    fn set_facing(&mut self, _: &mut MoverState, _: &mut ActorMotion, _: [f32; 2]) -> Result<()> {
        unreachable!()
    }
    fn move_on_ground(&mut self, _: &mut MoverState, _: &mut ActorMotion, _: [f32; 2]) -> Result<()> {
        unreachable!()
    }
    fn notify(&mut self, event: Event, _: &mut MoverState, _: &mut ActorMotion) -> Result<bool> {
        match event {
            Event::CancelScheduled => self.record("CancelScheduled", &[]),
            Event::DeactivateMover => self.record("DeactivateMover", &[]),
            Event::DestinationReached => self.record("MindDestinationReached", &[]),
            Event::ScheduleImmediate => self.record("ScheduleImmediate", &[0]),
            other => panic!("unexpected {other:?}"),
        }
        Ok(true)
    }
    fn blocked(&mut self, _: &(), _: [f32; 2], _: &mut MoverState, _: &mut ActorMotion) -> Result<()> {
        unreachable!()
    }
}
impl TickWorld for World {
    fn fall(&mut self, _: &mut MoverState, _: &mut ActorMotion, delta: f32) -> Result<()> {
        self.record("Fall", &[delta.to_bits()]);
        Ok(())
    }
    fn refresh_overlaps(&mut self, _: &mut MoverState, actor: &mut ActorMotion) -> Result<()> {
        self.record("RefreshOverlaps", &[]);
        let r = self.response("RefreshOverlaps");
        if let Some(flags) = r.get("flags") {
            actor.flags = u(flags);
        }
        Ok(())
    }
    fn aircraft_step(&mut self, _: &mut MoverState, _: &mut ActorMotion, delta: f32) -> Result<()> {
        self.record("AircraftStep", &[delta.to_bits()]);
        Ok(())
    }
    fn ground_step(&mut self, _: &mut MoverState, _: &mut ActorMotion, delta: f32) -> Result<StepOutcome> {
        self.record("GroundStep", &[delta.to_bits()]);
        Ok(StepOutcome { kind: StepKind::Advanced, native_success: true })
    }
    fn movement_failed(&mut self, _: &mut MoverState, _: &mut ActorMotion) -> Result<()> {
        self.record("MindMovementFailed", &[]);
        Ok(())
    }
}
impl MaxSpeedWorld for World {
    fn has_organization(&mut self, _: &mut ActorMotion) -> Result<bool> {
        Ok(self.case["actor"]["element_organization"].as_bool() == Some(true))
    }
    fn organization_speed_changed(&mut self, old: u32, _: &mut ActorMotion) -> Result<()> {
        self.record("OrganizationSpeedChanged", &[old]);
        Ok(())
    }
    fn mind_max_speed_changed(&mut self, old: u32, new: u32, _: &mut ActorMotion) -> Result<()> {
        self.record("MindMaxSpeedChanged", &[old, new]);
        Ok(())
    }
}
impl RestoreWorld for World {
    fn schedule_immediate(&mut self) -> Result<()> {
        self.record("ScheduleImmediate", &[0]);
        Ok(())
    }
    fn actor_state(&mut self, state: u32, value: bool) -> Result<()> {
        self.record("ActorState", &[state, value as u32]);
        Ok(())
    }
}
impl FallingWorld for World {
    fn has_aircraft(&mut self, _: &mut ActorMotion) -> Result<bool> {
        Ok(self.case["actor"]["has_aircraft"].as_bool().unwrap())
    }
    fn aircraft_height_offset(&mut self, _: &mut ActorMotion) -> Result<f32> {
        Ok(f(u(&self.case["definition"]["aircraft_height_offset"])))
    }
    fn ground_height(
        &mut self,
        xy: [f32; 2],
        _: &mut MoverState,
        _: &mut FallingState,
        _: &mut ActorMotion,
    ) -> Result<Binary> {
        self.record("GroundHeight", &[xy[0].to_bits(), xy[1].to_bits(), 0x122]);
        Ok(f64_binary(self.response("GroundHeight")["f64"].as_str().unwrap()))
    }
    fn left_map(&mut self, _: &mut ActorMotion) -> Result<()> {
        self.record("LeftMap", &[]);
        Ok(())
    }
    fn entered_map(&mut self, _: &mut ActorMotion) -> Result<()> {
        self.record("EnteredMap", &[]);
        Ok(())
    }
    fn commit_position(&mut self, p: [f32; 3], actor: &mut ActorMotion) -> Result<()> {
        self.record("CommitPosition", &p.map(f32::to_bits));
        actor.position = p;
        Ok(())
    }
    fn set_facing(&mut self, facing: [f32; 3], actor: &mut ActorMotion) -> Result<()> {
        self.record("SetFacing", &facing.map(f32::to_bits));
        if actor.flags & (1 << 17) == 0 {
            actor.facing = facing;
        }
        Ok(())
    }
    fn actor_state(&mut self, state: u32, value: bool, _: &mut ActorMotion) -> Result<()> {
        self.record("ActorState", &[state, value as u32]);
        Ok(())
    }
    fn schedule_immediate(&mut self, _: &mut ActorMotion) -> Result<()> {
        self.record("ScheduleImmediate", &[0]);
        Ok(())
    }
    fn cancel_scheduled(&mut self, _: &mut ActorMotion) -> Result<()> {
        self.record("CancelScheduled", &[]);
        Ok(())
    }
    fn landed(&mut self, _: &mut ActorMotion) -> Result<()> {
        self.record("Landed", &[]);
        Ok(())
    }
}

fn actor(case: &Value) -> ActorMotion {
    let a = &case["actor"];
    ActorMotion {
        position: bits3(&a["position"]),
        facing: bits3(&a["facing"]),
        flags: u(&a["flags"]),
        has_mind: a["has_mind"].as_bool().unwrap(),
        has_character: false,
        character_stopped: false,
        terrain_speed_limit: 0.0,
    }
}

#[test]
fn native_mover_operations_match_original_x86() {
    let doc = document();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), doc["case_count"].as_u64().unwrap() as usize);
    let mut counts = BTreeMap::<String, usize>::new();
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let op = case["operation"].as_str().unwrap();
        let pc = precision(&case["precision"]);
        let memory = Memory::load(&case["mover"]);
        let (mut m, mut limits, mut fs) = memory.split();
        let mut a = actor(case);
        let mut w = World::new(case);
        let s = &case["settings"];
        let d = &case["definition"];
        let definition = MoverDefinition {
            speed: f(u(&d["speed"])),
            turn_radius: f(u(&d["turn_radius"])),
            turn_type: u(&d["turn_type"]) as i32,
        };
        let map = MapBounds {
            cell_size: f(u(&s["map_cell_size"])),
            extent: [f(u(&s["map_extent"][0])), f(u(&s["map_extent"][1]))],
        };
        let args: Vec<u32> = case["args"].as_array().unwrap().iter().map(u).collect();
        let al: Option<u8> = match op {
            "tick" => {
                let gate = TickGate {
                    mover_is_active_component: case["actor"]["mover_active"].as_bool().unwrap(),
                    properties_flags_1c: case["actor"]["properties_flags"].as_u64().map(|v| v as u8),
                };
                mover::tick(&mut m, &mut a, gate, f(u(&case["arg"])), f(u(&s["precision_radius"])), pc, &mut w)
                    .unwrap();
                None
            }
            "stop_turning" => Some(mover::stop_turning(&mut m, &mut a, &mut w).unwrap() as u8),
            "set_speed" => {
                Some(mover::set_requested_speed(&mut m, &mut a, f(args[0]), f(args[1]), &mut w).unwrap() as u8)
            }
            "refresh_max_speed" => {
                let multiplier = case.get("speed_multiplier").and_then(|v| v.as_u64()).map(|v| f(v as u32));
                mover::refresh_max_speed(&mut m, &mut limits, &mut a, definition, multiplier, pc, &mut w)
                    .unwrap();
                None
            }
            "begin_fall" => {
                Some(falling::begin_fall(&mut m, &mut fs, &mut a, f(args[0]), args[1] as u8, &mut w).unwrap() as u8)
            }
            "begin_aimed_fall" => Some(
                falling::begin_aimed_fall(&mut m, &mut fs, &mut a, f(args[0]), [f(args[1]), f(args[2])],
                    args[3] as u8, pc, &mut w)
                .unwrap() as u8,
            ),
            "fall" => {
                let context = FallContext {
                    delta: f(u(&case["arg"])),
                    gravity: f(u(&s["gravity"])),
                    plane_fall_gravity_tweak: Some(f(u(&s["plane_fall_gravity_tweak"]))),
                    map,
                    precision: pc,
                };
                Some(falling::fall_step(&mut m, &mut fs, &mut a, &context, &mut w).unwrap() as u8)
            }
            "restore_falling" => Some(
                mover::restore_falling(&m, u(&s["world_version"]), u(&case["updatable_word_0c"]) != 0, &mut w)
                    .unwrap() as u8,
            ),
            "interior" => Some(falling::inside_map(map, [f(args[0]), f(args[1])], pc).unwrap() as u8),
            other => panic!("{other}"),
        };
        let out = &case["output"];
        let context = format!("{name} {:?}", pc);
        if let Some(al) = al {
            assert_eq!(al as u64, out["return_al"].as_u64().unwrap() & 255, "{context} return");
        }
        assert!(w.responses.is_empty(), "{context} unused responses");
        assert_eq!(Value::Array(w.events.clone()), out["events"], "{context} events");
        if op != "interior" {
            let merged = memory.merge(&m, &limits, &fs);
            for pair in out["mover_words"].as_array().unwrap() {
                let o = u(&pair[0]);
                assert_eq!(merged.w(o), u(&pair[1]), "{context} word {o:#x}");
            }
            for pair in out["mover_bytes"].as_array().unwrap() {
                let o = u(&pair[0]);
                assert_eq!(merged.b(o) as u32, u(&pair[1]), "{context} byte {o:#x}");
            }
            assert_eq!(a.position.map(f32::to_bits).to_vec(), out["actor_position"].as_array().unwrap()
                .iter().map(u).collect::<Vec<_>>(), "{context} position");
            assert_eq!(a.facing.map(f32::to_bits).to_vec(), out["actor_facing"].as_array().unwrap()
                .iter().map(u).collect::<Vec<_>>(), "{context} facing");
            assert_eq!(a.flags, u(&out["actor_flags"]), "{context} flags");
        }
        *counts.entry(op.to_string()).or_default() += 1;
    }
    eprintln!("native mover cases by operation: {counts:?}");
    let launch = doc["launch_speed"].as_array().unwrap();
    for row in launch {
        let pc = precision(&row["precision"]);
        let value = mover::launch_speed(f(u(&row["distance"])), f(u(&row["factor"])), f(u(&row["offset"])),
            f(u(&row["gravity"])), pc).unwrap();
        let expected = f80_binary(row["result_f80"].as_str().unwrap());
        assert_eq!(value.compare(&expected), Ordering::Equal, "launch {row}");
        assert_eq!(value.is_negative_nonzero(), expected.is_negative_nonzero());
    }
    assert_eq!(launch.len(), 24);
}

#[test]
fn failure_without_mind_errors_after_native_effects() {
    let mut m = Memory::default().split().0;
    let mut a = ActorMotion {
        position: [0.0; 3],
        facing: [1.0, 0.0, 0.0],
        flags: 0,
        has_mind: false,
        has_character: false,
        character_stopped: false,
        terrain_speed_limit: 0.0,
    };
    m.destination = [5.0, 0.0, 0.0];
    let mut w = World::new(&json!({"responses": []}));
    let gate = TickGate { mover_is_active_component: true, properties_flags_1c: None };
    let error = mover::tick(&mut m, &mut a, gate, 0.0625, 0.1, Precision::Double53, &mut w).unwrap_err();
    assert!(matches!(error, Error::MissingMind));
    assert_eq!(w.events, vec![json!({"name":"CancelScheduled","bits":[]}),
        json!({"name":"DeactivateMover","bits":[]})]);
}
