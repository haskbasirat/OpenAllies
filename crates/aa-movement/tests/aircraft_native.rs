//! Replays research/phase8-movement/native-aircraft.json: original x86
//! execution (Unicorn) of AircraftComponent MoveTick 0x5612F0, path advance
//! 0x55E2F0, arc length 0x55DCA0, cubic evaluation 0x55DA10, holding circle
//! 0x560050/0x55FE60, follow 0x561080/0x5685E0, member reset 0x55EBE0, path
//! reset 0x55E1B0, leader 0x5685B0/0x567FF0, predicate 0x516B10 and Mover
//! speed clamp 0x576D20 under PC24/53/64. Every recorded callback with its
//! argument bits, every aircraft/path/segment field of this and the member
//! aircraft, the Mover words and actor+0x3C must match bit for bit; x87
//! returns are compared as exact 80-bit images.
use aa_movement::aircraft::*;
use aa_movement::mover::MoverLimits;
use aa_movement::{ActorMotion, MoverState};
use aa_numeric::{Binary, Precision};
use serde_json::{Value, json};
use std::collections::{BTreeMap, VecDeque};

fn document() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/phase8-movement/native-aircraft.json"
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn f(v: &Value) -> f32 {
    f32::from_bits(u(v))
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array().expect("array").iter().map(u).collect()
}
fn f3(v: &Value) -> [f32; 3] {
    let w = words(v);
    [
        f32::from_bits(w[0]),
        f32::from_bits(w[1]),
        f32::from_bits(w[2]),
    ]
}
fn bits3(v: [f32; 3]) -> Vec<u32> {
    v.map(f32::to_bits).to_vec()
}
fn key(v: &Value) -> Option<u32> {
    Some(u(v)).filter(|&k| k != 0)
}
fn f64_binary(v: &Value) -> Binary {
    let text = v.as_str().expect("hex");
    Binary::from_f64_bits(u64::from_str_radix(text.trim_start_matches("0x"), 16).unwrap()).unwrap()
}
fn f80_bits(hex: &str) -> u128 {
    (0..10).fold(0u128, |acc, i| {
        acc | (u128::from(u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()) << (8 * i))
    })
}
fn pose(v: &Value) -> Pose {
    let w = words(v);
    let g = |i: usize| f32::from_bits(w[i]);
    Pose {
        position: [g(0), g(1), g(2)],
        direction: [g(3), g(4), g(5)],
        speed: g(6),
    }
}
fn pose_bits(p: &Pose) -> Vec<u32> {
    let mut out = bits3(p.position);
    out.extend(bits3(p.direction));
    out.push(p.speed.to_bits());
    out
}

fn segment(v: &Value) -> PathSegment {
    let c = words(&v["control"]);
    let p = |i: usize| {
        [
            f32::from_bits(c[3 * i]),
            f32::from_bits(c[3 * i + 1]),
            f32::from_bits(c[3 * i + 2]),
        ]
    };
    PathSegment {
        control: [p(0), p(1), p(2), p(3)],
        length: f(&v["length"]),
        field_3c: f(&v["field_3c"]),
        end_direction: f3(&v["end_direction"]),
        end_speed: f(&v["end_speed"]),
        start_speed: f(&v["start_speed"]),
        tolerance: f(&v["tolerance"]),
    }
}
fn segment_json(s: &PathSegment) -> Value {
    let control: Vec<u32> = s.control.iter().flat_map(|p| bits3(*p)).collect();
    json!({"control": control, "length": s.length.to_bits(), "field_3c": s.field_3c.to_bits(),
        "end_direction": bits3(s.end_direction), "end_speed": s.end_speed.to_bits(),
        "start_speed": s.start_speed.to_bits(), "tolerance": s.tolerance.to_bits()})
}
fn basis(v: &Value) -> [f32; 16] {
    let w = words(v);
    std::array::from_fn(|i| f32::from_bits(w[i]))
}
fn path(v: &Value) -> AircraftPath {
    AircraftPath {
        planning_speed: f(&v["planning_speed"]),
        turn_radius: f(&v["turn_radius"]),
        segments: v["segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(segment)
            .collect(),
        t: f(&v["t"]),
        basis: basis(&v["basis"]),
        altitude: f(&v["altitude"]),
        leg_speed: f(&v["leg_speed"]),
        total_length: f(&v["total_length"]),
        speed: f(&v["speed"]),
    }
}
fn aircraft(v: &Value) -> AircraftState {
    let g = |i: usize| key(&v["groups"][i]).map(GroupKey);
    AircraftState {
        on_ground: u(&v["on_ground"]) as u8,
        cruise_speed: f(&v["cruise_speed"]),
        cruise_altitude: 0.0,
        path: path(&v["path"]),
        circling: u(&v["circling"]) as u8,
        attack_target: u(&v["attack_target"]),
        attack_point: f3(&v["attack_point"]),
        pending_ability: key(&v["ability"]).map(AbilityKey),
        group_a: g(0),
        group_b: g(1),
        group_c: g(2),
    }
}
fn aircraft_json(a: &AircraftState) -> Value {
    let p = &a.path;
    let g = |k: Option<GroupKey>| k.map_or(0, |k| k.0);
    json!({
        "on_ground": a.on_ground, "cruise_speed": a.cruise_speed.to_bits(), "circling": a.circling,
        "attack_target": a.attack_target, "attack_point": bits3(a.attack_point),
        "ability": a.pending_ability.map_or(0, |k| k.0), "groups": [g(a.group_a), g(a.group_b), g(a.group_c)],
        "path": {"planning_speed": p.planning_speed.to_bits(), "turn_radius": p.turn_radius.to_bits(),
            "t": p.t.to_bits(), "basis": p.basis.map(f32::to_bits).to_vec(), "altitude": p.altitude.to_bits(),
            "leg_speed": p.leg_speed.to_bits(), "total_length": p.total_length.to_bits(),
            "speed": p.speed.to_bits(), "segments": p.segments.iter().map(segment_json).collect::<Vec<_>>()},
    })
}

struct Other {
    handle: u32,
    flags: u32,
    aircraft: Option<AircraftState>,
}

struct World {
    queues: BTreeMap<String, VecDeque<Value>>,
    events: Vec<Value>,
    others: BTreeMap<u32, Other>,
    groups: BTreeMap<u32, Value>,
    handles: BTreeMap<u16, HandleSlot>,
    mind_scheduled: bool,
    ability_word: u32,
}
impl World {
    fn new(case: &Value) -> Self {
        let mut queues = BTreeMap::new();
        if let Some(r) = case.get("responses").and_then(Value::as_object) {
            for (name, list) in r {
                queues.insert(
                    name.clone(),
                    list.as_array().unwrap().iter().cloned().collect(),
                );
            }
        }
        let mut others = BTreeMap::new();
        for o in case
            .get("others")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let aircraft = (!o["aircraft"].is_null()).then(|| aircraft(&o["aircraft"]));
            others.insert(
                u(&o["key"]),
                Other {
                    handle: u(&o["handle"]),
                    flags: u(&o["flags"]),
                    aircraft,
                },
            );
        }
        let mut groups = BTreeMap::new();
        for g in case
            .get("groups")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            groups.insert(u(&g["key"]), g.clone());
        }
        let mut handles = BTreeMap::new();
        for h in case
            .get("handles")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            handles.insert(
                u(&h[0]) as u16,
                HandleSlot {
                    generation: u(&h[1]) as u16,
                    actor: key(&h[2]).map(ActorKey),
                },
            );
        }
        let a = &case["actor"];
        World {
            queues,
            events: vec![],
            others,
            groups,
            handles,
            mind_scheduled: a
                .get("mind_scheduled")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            ability_word: a.get("ability_word_54").map_or(0, u),
        }
    }
    fn record(&mut self, name: &str, bits: &[u32]) {
        self.events.push(json!({"name": name, "bits": bits}));
    }
    fn response(&mut self, name: &str) -> Value {
        self.queues
            .get_mut(name)
            .and_then(VecDeque::pop_front)
            .unwrap_or_else(|| panic!("unexpected {name}"))
    }
    fn unused(&self) -> usize {
        self.queues.values().map(VecDeque::len).sum()
    }
}

/// Mirror of the oracle's seam effect on this aircraft.
fn mutate(a: &mut AircraftState, m: &Value) {
    if m.get("clear").and_then(Value::as_bool) == Some(true) {
        a.path.segments.clear();
        a.path.t = 0.0;
        a.path.total_length = 0.0;
    }
    for s in m
        .get("append")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        a.path.segments.push_back(segment(s));
    }
    for w in m
        .get("words")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let value = f(&w[1]);
        match u(&w[0]) {
            0x14 => a.cruise_speed = value,
            0x2c => a.path.planning_speed = value,
            0x30 => a.path.turn_radius = value,
            0x3c => a.path.t = value,
            0x80 => a.path.altitude = value,
            0x84 => a.path.leg_speed = value,
            0x88 => a.path.total_length = value,
            0x8c => a.path.speed = value,
            other => panic!("word {other:#x}"),
        }
    }
    for b in m
        .get("bytes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match u(&b[0]) {
            0x10 => a.on_ground = u(&b[1]) as u8,
            0x90 => a.circling = u(&b[1]) as u8,
            other => panic!("byte {other:#x}"),
        }
    }
    if let Some(g) = m.get("groups") {
        a.group_a = key(&g[0]).map(GroupKey);
        a.group_b = key(&g[1]).map(GroupKey);
        a.group_c = key(&g[2]).map(GroupKey);
    }
}

fn descriptor_bits(d: &AttackDescriptor) -> Vec<u32> {
    let mut out = vec![d.source.0];
    match d.target {
        AttackTarget::Actor(actor) => out.extend([1, actor.0, 0, 0]),
        AttackTarget::Point(p) => {
            out.push(2);
            out.extend(bits3(p));
        }
    }
    out
}

impl FormationWorld for World {
    fn handle_slot(&mut self, index: u16) -> AircraftResult<HandleSlot> {
        Ok(self.handles.get(&index).copied().unwrap_or(HandleSlot {
            generation: 0,
            actor: None,
        }))
    }
    fn actor_handle(&mut self, actor: ActorKey) -> AircraftResult<u32> {
        Ok(self.others[&actor.0].handle)
    }
    fn actor_flags(&mut self, actor: ActorKey) -> AircraftResult<u32> {
        Ok(self.others[&actor.0].flags)
    }
    fn group_members(&mut self, group: GroupKey) -> AircraftResult<Vec<u32>> {
        Ok(words(&self.groups[&group.0]["members"]))
    }
    fn group_layout(&mut self, group: GroupKey) -> AircraftResult<GroupLayout> {
        let g = &self.groups[&group.0];
        let definition = (!g["layout"].is_null()).then(|| FormationDefinition {
            slot_word: u(&g["layout"]["slot_word"]),
            offsets: g["layout"]["offsets"]
                .as_array()
                .unwrap()
                .iter()
                .map(f3)
                .collect(),
        });
        Ok(GroupLayout {
            definition,
            anchor: f3(&g["anchor"]),
        })
    }
    fn member_aircraft(&mut self, actor: ActorKey) -> AircraftResult<&mut AircraftState> {
        self.others
            .get_mut(&actor.0)
            .and_then(|o| o.aircraft.as_mut())
            .ok_or(AircraftError::NullDereference("member actor+0xBC"))
    }
}

impl AircraftWorld for World {
    fn mind_movement_blocked(&mut self, _: &mut AircraftUnit) -> AircraftResult<bool> {
        self.record("MindBlocked", &[]);
        Ok(self.response("MindBlocked")["value"].as_bool().unwrap())
    }
    fn ability_definition_word(&mut self, ability: AbilityKey) -> AircraftResult<u32> {
        assert_eq!(ability, AbilityKey(1));
        Ok(self.ability_word)
    }
    fn attack_descriptor(&mut self, flags: u32, _: &mut AircraftUnit) -> AircraftResult<()> {
        self.record("AttackDescriptor", &[flags]);
        Ok(())
    }
    fn ability_check(
        &mut self,
        ability: AbilityKey,
        d: &AttackDescriptor,
        _: &mut AircraftUnit,
    ) -> AircraftResult<u32> {
        let mut bits = vec![ability.0, 1];
        bits.extend(descriptor_bits(d));
        self.record("AbilityCheck", &bits);
        Ok(u(&self.response("AbilityCheck")["value"]))
    }
    fn fire_ability(
        &mut self,
        ability: Option<AbilityKey>,
        d: &AttackDescriptor,
        _: &mut AircraftUnit,
    ) -> AircraftResult<()> {
        let mut bits = vec![ability.map_or(0, |k| k.0)];
        bits.extend(descriptor_bits(d));
        self.record("FireAbility", &bits);
        Ok(())
    }
    fn actor_speed(&mut self, _: &mut AircraftUnit) -> AircraftResult<Binary> {
        self.record("ActorSpeed", &[]);
        Ok(f64_binary(&self.response("ActorSpeed")["f64"]))
    }
    fn formation_speed(&mut self, _: &mut AircraftUnit) -> AircraftResult<Binary> {
        self.record("FormationSpeed", &[]);
        Ok(f64_binary(&self.response("FormationSpeed")["f64"]))
    }
    fn group_disbanded(&mut self, group: GroupKey, _: &mut AircraftUnit) -> AircraftResult<bool> {
        self.record("GroupDisbanded", &[group.0]);
        Ok(self.groups[&group.0]["disbanded"].as_bool().unwrap())
    }
    fn leave_formation(&mut self, unit: &mut AircraftUnit) -> AircraftResult<()> {
        self.record("LeaveFormation", &[]);
        mutate(
            &mut unit.aircraft,
            &self.response("LeaveFormation")["mutation"],
        );
        Ok(())
    }
    fn plan_follow_path(
        &mut self,
        unit: &mut AircraftUnit,
        p: &Pose,
        leader: ActorKey,
        offset: [f32; 3],
        delta: f32,
    ) -> AircraftResult<()> {
        let mut bits = pose_bits(p);
        bits.push(leader.0);
        bits.extend(bits3(offset));
        bits.push(delta.to_bits());
        self.record("FollowPath", &bits);
        mutate(&mut unit.aircraft, &self.response("FollowPath")["mutation"]);
        Ok(())
    }
    fn build_leg(
        &mut self,
        unit: &mut AircraftUnit,
        from: &Pose,
        target: [f32; 3],
        side: [f32; 3],
        flag: u32,
    ) -> AircraftResult<Pose> {
        let mut bits = pose_bits(from);
        bits.extend(bits3(target));
        bits.extend(bits3(side));
        bits.extend([flag, 0]);
        self.record("BuildLeg", &bits);
        let r = self.response("BuildLeg");
        mutate(&mut unit.aircraft, &r["mutation"]);
        Ok(pose(&r["out"]))
    }
    fn mind_scheduled(&mut self, _: &mut AircraftUnit) -> AircraftResult<bool> {
        Ok(self.mind_scheduled)
    }
    fn report_stopped_thinking(&mut self, line: u32, handle: u32) -> AircraftResult<()> {
        self.record("Diagnostic", &[line, handle]);
        Ok(())
    }
    fn mind_reschedule(
        &mut self,
        delay: u32,
        argument: u32,
        _: &mut AircraftUnit,
    ) -> AircraftResult<()> {
        self.record("MindReschedule", &[delay, argument]);
        Ok(())
    }
    fn commit_pose(
        &mut self,
        position: [f32; 3],
        facing: [f32; 3],
        speed: f32,
        _: &mut AircraftUnit,
    ) -> AircraftResult<()> {
        let mut bits = bits3(position);
        bits.extend(bits3(facing));
        bits.push(speed.to_bits());
        self.record("CommitPose", &bits);
        Ok(())
    }
    fn ground_height(
        &mut self,
        xy: [f32; 2],
        flags: u32,
        _: &mut AircraftUnit,
    ) -> AircraftResult<Binary> {
        self.record("GroundHeight", &[xy[0].to_bits(), xy[1].to_bits(), flags]);
        Ok(f64_binary(&self.response("GroundHeight")["f64"]))
    }
    fn mind_path_exhausted(&mut self, _: &mut AircraftUnit) -> AircraftResult<()> {
        self.record("MindPathExhausted", &[]);
        Ok(())
    }
}

fn unit(case: &Value) -> AircraftUnit {
    let a = &case["actor"];
    let d = &a["definition"];
    let m = &case["mover"];
    AircraftUnit {
        actor: AircraftActor {
            key: ActorKey(1),
            handle: u(&a["handle"]),
            scale: f(&a["scale"]),
            definition: AircraftDefinition {
                taxi_scale: f(&d["taxi_scale"]),
                taxi_height: f(&d["taxi_height"]),
                operational_height: f(&d["operational_height"]),
            },
        },
        motion: ActorMotion {
            position: f3(&a["position"]),
            facing: f3(&a["facing"]),
            flags: u(&a["flags"]),
            has_mind: a["has_mind"].as_bool().unwrap(),
            has_character: false,
            character_stopped: false,
            terrain_speed_limit: 0.0,
        },
        mover: MoverState {
            destination: [0.0; 3],
            last_step_speed: 0.0,
            speed: 0.0,
            requested_speed: f(&m["requested_speed"]),
            acceleration: 0.0,
            turn_rate: 0.0,
            desired_direction: [0.0; 3],
            target_id: 0,
            target_range: None,
            ignore_terrain_speed_limit: false,
            speed_factor: f(&m["speed_factor"]),
            instant_turn: false,
            turning: false,
            falling: false,
        },
        mover_limits: MoverLimits {
            max_speed: f(&m["max_speed"]),
            turn_radius: 0.0,
        },
        aircraft: aircraft(&case["aircraft"]),
    }
}

fn precision(v: &Value) -> Precision {
    serde_json::from_value(v.clone()).unwrap()
}

fn x87(value: &Binary) -> u128 {
    value.to_x87_extended_bits().expect("x87 image")
}

#[test]
fn native_aircraft_matches_original_x86() {
    let doc = document();
    let cases = doc["cases"].as_array().unwrap();
    assert_eq!(cases.len(), doc["case_count"].as_u64().unwrap() as usize);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut unsupported = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let op = case["operation"].as_str().unwrap();
        let pc = precision(&case["precision"]);
        let context = format!("{name} {pc:?}");
        let args = &case["args"];
        let out = &case["output"];
        let mut w = World::new(case);
        match op {
            "evaluate" => {
                let c = words(&args["control"]);
                let p = |i: usize| {
                    [
                        f32::from_bits(c[3 * i]),
                        f32::from_bits(c[3 * i + 1]),
                        f32::from_bits(c[3 * i + 2]),
                    ]
                };
                let point = evaluate_curve(
                    &basis(&args["basis"]),
                    &[p(0), p(1), p(2), p(3)],
                    f(&args["t"]),
                    pc,
                )
                .unwrap();
                assert_eq!(bits3(point), words(&out["point"]), "{context}");
            }
            "arc_length" => {
                let length = arc_length(
                    &basis(&args["basis"]),
                    &segment(&args["segment"]),
                    f(&args["t0"]),
                    f(&args["t1"]),
                    pc,
                )
                .unwrap();
                assert_eq!(
                    x87(&length),
                    f80_bits(out["length_f80"].as_str().unwrap()),
                    "{context}"
                );
            }
            "advance_path" => {
                let mut a = aircraft(&case["aircraft"]);
                let result = advance_path(&mut a.path, f(&args["delta"]), &pose(&args["pose"]), pc);
                if case.get("unsupported").is_some() {
                    // The original continues with a NaN pose (recorded in the
                    // fixture); the port refuses the nonfinite parameter.
                    assert!(
                        words(&out["pose"])[..3]
                            .iter()
                            .all(|&w| w & 0x7f80_0000 == 0x7f80_0000),
                        "{context}"
                    );
                    assert!(
                        matches!(result, Err(AircraftError::NonFinite(_))),
                        "{context}: {result:?}"
                    );
                    unsupported += 1;
                    *counts.entry(op.to_string()).or_default() += 1;
                    continue;
                }
                let (remaining, result) = result.unwrap();
                let remaining = Binary::from_f32_bits(remaining.to_bits()).unwrap();
                assert_eq!(
                    x87(&remaining),
                    f80_bits(out["remaining_f80"].as_str().unwrap()),
                    "{context} remaining"
                );
                assert_eq!(pose_bits(&result), words(&out["pose"]), "{context} pose");
                assert_eq!(aircraft_json(&a), out["aircraft"], "{context} aircraft");
            }
            _ => {
                let mut un = unit(case);
                let group = |n: &str| GroupKey(u(&args[n]));
                let result: AircraftResult<()> = match op {
                    "move_tick" => move_tick(&mut un, f(&args["delta"]), pc, &mut w).map(drop),
                    "plan_circle" => plan_holding_circle(&mut un, &pose(&args["pose"]), pc, &mut w),
                    "reset_members" => reset_member_paths(&mut un, &mut w),
                    "follow_leader" => {
                        follow_leader(&mut un, &pose(&args["pose"]), f(&args["delta"]), pc, &mut w)
                            .map(drop)
                    }
                    "clear_path" => {
                        clear_path(&mut un.aircraft.path);
                        Ok(())
                    }
                    "set_mover_speed" => {
                        let limits = un.mover_limits;
                        set_mover_speed(
                            &mut un.mover,
                            &limits,
                            f(&args["speed"]),
                            f(&args["factor"]),
                        )
                    }
                    "following_path" => {
                        assert_eq!(
                            following_path(&un.aircraft) as u64,
                            out["return_al"].as_u64().unwrap(),
                            "{context}"
                        );
                        Ok(())
                    }
                    "formation_offset" => {
                        let relative = u(&args["relative"]) & 0xff != 0;
                        let offset = formation_offset(
                            &mut w,
                            group("group"),
                            u(&args["handle"]),
                            relative,
                            pc,
                        )
                        .unwrap();
                        assert_eq!(bits3(offset), words(&out["offset"]), "{context}");
                        Ok(())
                    }
                    "leader" => {
                        let leader = group_leader(&mut w, group("group")).unwrap();
                        assert_eq!(leader.map_or(0, |k| k.0), u(&out["leader"]), "{context}");
                        Ok(())
                    }
                    "leader_is" => {
                        let is =
                            group_leader_is(&mut w, group("group"), u(&args["handle"])).unwrap();
                        assert_eq!(is as u64, out["return_al"].as_u64().unwrap(), "{context}");
                        Ok(())
                    }
                    other => panic!("{other}"),
                };
                assert_eq!(
                    Value::Array(w.events.clone()),
                    out["events"],
                    "{context} events"
                );
                if out.get("null_dereference").is_some() {
                    assert!(
                        matches!(result, Err(AircraftError::NullDereference(_))),
                        "{context}: {result:?}"
                    );
                } else {
                    result.unwrap_or_else(|e| panic!("{context}: {e}"));
                    assert_eq!(
                        aircraft_json(&un.aircraft),
                        out["aircraft"],
                        "{context} aircraft"
                    );
                    let others: Vec<Value> = w
                        .others
                        .iter()
                        .filter_map(|(k, o)| {
                            o.aircraft
                                .as_ref()
                                .map(|a| json!({"key": k, "aircraft": aircraft_json(a)}))
                        })
                        .collect();
                    assert_eq!(Value::Array(others), out["others"], "{context} others");
                    assert_eq!(
                        vec![
                            un.mover.requested_speed.to_bits(),
                            un.mover_limits.max_speed.to_bits(),
                            un.mover.speed_factor.to_bits()
                        ],
                        words(&out["mover"]),
                        "{context} mover"
                    );
                    assert_eq!(
                        un.actor.scale.to_bits(),
                        u(&out["actor_scale"]),
                        "{context} scale"
                    );
                    assert_eq!(
                        bits3(un.motion.position),
                        words(&out["actor_position"]),
                        "{context} position"
                    );
                    assert_eq!(
                        bits3(un.motion.facing),
                        words(&out["actor_facing"]),
                        "{context} facing"
                    );
                }
            }
        }
        assert_eq!(w.unused(), 0, "{context} unused responses");
        *counts.entry(op.to_string()).or_default() += 1;
    }
    eprintln!(
        "native aircraft cases by operation: {counts:?} (unsupported nonfinite: {unsupported})"
    );
    assert_eq!(unsupported, 3);
    assert_eq!(
        serde_json::to_value(&counts).unwrap(),
        doc["operation_counts"]
    );
    for pc in ["Single24", "Double53", "Extended64"] {
        assert!(
            cases.iter().filter(|c| c["precision"] == pc).count() >= 400,
            "{pc} coverage"
        );
    }
}

#[test]
fn bezier_basis_matches_the_fixture_paths() {
    let doc = document();
    let path_case = doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["operation"] == "move_tick")
        .unwrap();
    assert_eq!(
        words(&path_case["aircraft"]["path"]["basis"]),
        BEZIER_BASIS_BITS.to_vec()
    );
}

#[test]
fn self_repeating_subdivision_is_an_explicit_error() {
    // A negative tolerance makes the zero-width subdivision recurse on itself.
    let s = PathSegment {
        control: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
        ],
        length: 3.0,
        field_3c: 0.0,
        end_direction: [1.0, 0.0, 0.0],
        end_speed: 1.0,
        start_speed: 1.0,
        tolerance: -1.0,
    };
    let basis = BEZIER_BASIS_BITS.map(f32::from_bits);
    let error = arc_length(&basis, &s, 0.25, 0.25, Precision::Double53).unwrap_err();
    assert!(
        matches!(error, AircraftError::UnboundedRecursion),
        "{error}"
    );
}

#[test]
fn infinite_negative_segment_step_is_unsupported() {
    let s = PathSegment {
        control: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
        ],
        length: -0.0,
        field_3c: 0.0,
        end_direction: [1.0, 0.0, 0.0],
        end_speed: 1.0,
        start_speed: 1.0,
        tolerance: 0.1,
    };
    let mut p = AircraftPath {
        planning_speed: 1.0,
        turn_radius: 1.0,
        segments: VecDeque::from([s]),
        t: 0.0,
        basis: BEZIER_BASIS_BITS.map(f32::from_bits),
        altitude: 0.0,
        leg_speed: 1.0,
        total_length: 0.0,
        speed: 1.0,
    };
    let start = Pose {
        position: [0.0; 3],
        direction: [1.0, 0.0, 0.0],
        speed: 1.0,
    };
    let error = advance_path(&mut p, 0.5, &start, Precision::Double53).unwrap_err();
    assert!(matches!(error, AircraftError::NonFinite(_)), "{error}");
}
