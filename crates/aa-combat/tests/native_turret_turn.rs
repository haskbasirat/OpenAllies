//! 0x9C8660 arctangent table, 0x85D9F0/0x85D8D0 table arctangent, 0x57DEE0
//! turn step, 0x57E730 aim, 0x57EF00 reset update and 0x57EE40 stage-0 aim
//! against research/phase8-combat/native-turret-turn-fixtures.json.
use aa_body::F32;
use aa_combat::{
    Result,
    constructors::{OwnedPerformer, Turret},
    performer::Performer,
    turret::{
        TurnWorld, TurretDefinition, aim, atan_table, reset_update, stage0_aim, table_atan,
        turn_step,
    },
};
use aa_numeric::Precision;
use serde_json::{Value, json};

const OWNER: u32 = 0x5000;

fn bits(v: &Value) -> F32 {
    F32(v.as_u64().unwrap() as u32)
}
fn pair(v: &Value) -> [F32; 2] {
    [bits(&v[0]), bits(&v[1])]
}
fn precision(v: &Value) -> Precision {
    serde_json::from_value(v.clone()).unwrap()
}

struct Adapter<'a> {
    case: &'a Value,
    /// The live handle and the position the handle table gives for it.
    live: Option<(u32, [F32; 2])>,
    events: Vec<Value>,
}
impl TurnWorld for Adapter<'_> {
    fn time(&self) -> Result<F32> {
        Ok(bits(&self.case["time"]))
    }
    fn tick_length(&self) -> Result<F32> {
        Ok(bits(&self.case["tick"]))
    }
    fn turret_count(&self, owner: u32) -> Result<i32> {
        assert_eq!(owner, OWNER);
        Ok(self.case["turrets"].as_array().unwrap().len() as i32)
    }
    fn turret(&self, owner: u32, index: usize) -> Result<TurretDefinition> {
        assert_eq!(owner, OWNER);
        let t = &self.case["turrets"][index];
        Ok(TurretDefinition {
            yaw_start: bits(&t["yaw_start"]),
            rate: bits(&t["rate"]),
            offset: pair(&t["offset"]),
            reset_time: bits(&t["reset_time"]),
            can_reset: t["can_reset"].as_bool().unwrap(),
        })
    }
    fn position(&self, owner: u32) -> Result<[F32; 2]> {
        assert_eq!(owner, OWNER);
        Ok(pair(&self.case["position"]))
    }
    fn facing(&self, owner: u32) -> Result<[F32; 2]> {
        assert_eq!(owner, OWNER);
        Ok(pair(&self.case["facing"]))
    }
    fn live_actor(&self, handle: u32) -> Result<Option<[F32; 2]>> {
        Ok(self
            .live
            .filter(|(live, _)| handle != 0 && *live == handle)
            .map(|(_, position)| position))
    }
    fn yaw_changed(&mut self, owner: u32, index: usize) -> Result<()> {
        assert_eq!(owner, OWNER);
        self.events.push(json!({"yaw_changed": index}));
        Ok(())
    }
    fn activate_updates(&mut self) -> Result<()> {
        self.events.push(json!({"activate": 1}));
        Ok(())
    }
    fn deactivate_updates(&mut self) -> Result<()> {
        self.events.push(json!({"deactivate": [0, 1]}));
        Ok(())
    }
    fn post_stage(&mut self, stage: i32) -> Result<()> {
        self.events.push(json!({"post": [0, stage]}));
        Ok(())
    }
    fn in_arc(&mut self, owner: u32, index: usize, point: [F32; 2]) -> Result<bool> {
        assert_eq!(owner, OWNER);
        self.events
            .push(json!({"in_arc": index, "point": [point[0].0, point[1].0]}));
        Ok(self.case["arc"][index].as_u64().unwrap() != 0)
    }
}

fn performer(case: &Value) -> OwnedPerformer {
    let turrets = case["turrets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            let r = &t["record"];
            Turret {
                current_angle: bits(&r[0]),
                target_angle: bits(&r[1]),
                time: bits(&r[2]),
                pending: r[3].as_u64().unwrap() != 0,
                target: [bits(&r[4]), bits(&r[5]), bits(&r[6])],
            }
        })
        .collect();
    OwnedPerformer {
        state: Performer {
            owner: OWNER,
            stage: case["stage"].as_i64().unwrap_or(0) as i32,
            abilities: vec![],
            mana: F32::ZERO,
            target_id: 0,
            target_position: [F32::ZERO; 2],
            active_ability: None,
            active_index: -1,
            started_at: F32::ZERO,
            animation_length: F32::ZERO,
            event_kind: 0,
            animation: None,
            turret_index: case["index"].as_i64().unwrap_or(-1) as i32,
        },
        maximum_mana: F32::ZERO,
        turrets,
        turret_update_pending: case["latch"].as_bool().unwrap(),
        word60: 0,
    }
}

fn after(performer: &OwnedPerformer) -> Value {
    Value::Array(
        performer
            .turrets
            .iter()
            .map(|t| {
                json!([
                    t.current_angle.0,
                    t.target_angle.0,
                    t.time.0,
                    u32::from(t.pending),
                    t.target[0].0,
                    t.target[1].0,
                    t.target[2].0
                ])
            })
            .collect(),
    )
}

fn fixtures() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-combat/native-turret-turn-fixtures.json"
    ))
    .unwrap()
}

#[test]
fn arctangent_table_matches_native() {
    let doc = fixtures();
    let table = &doc["table"];
    let native = table["entries"].as_str().unwrap();
    let port = atan_table();
    assert_eq!(port.count as u64, table["count"].as_u64().unwrap());
    assert_eq!(port.limit.0 as u64, table["maximum"].as_u64().unwrap());
    assert_eq!(native.len(), port.entries.len() * 8);
    for (i, entry) in port.entries.iter().enumerate() {
        let word = u32::from_str_radix(&native[i * 8..i * 8 + 8], 16)
            .unwrap()
            .swap_bytes();
        assert_eq!(entry.0, word, "entry {i}");
        // Each entry is far from a binary32 rounding boundary: any arctangent
        // within 2^-40 (relative) of the host one stores the same word.
        let exact = (f64::from(i as i32) * 100.0 / 16384.0).atan();
        let stored = f64::from(f32::from_bits(entry.0));
        for neighbour in [entry.0.wrapping_sub(1), entry.0 + 1] {
            if i == 0 && neighbour == u32::MAX {
                continue;
            }
            let boundary = (stored + f64::from(f32::from_bits(neighbour))) / 2.0;
            assert!(
                (exact - boundary).abs() > exact.abs() * 2f64.powi(-40),
                "entry {i} is near a rounding boundary"
            );
        }
    }
}

#[test]
fn table_arctangent_matches_native() {
    let doc = fixtures();
    let cases = doc["cases"]["atan"].as_array().unwrap();
    assert!(cases.len() > 1000);
    for case in cases {
        let p = precision(&case["precision"]);
        let value = table_atan(bits(&case["x"]), bits(&case["y"]), p).unwrap();
        let native = u128::from(case["result"][0].as_u64().unwrap())
            | (u128::from(case["result"][1].as_u64().unwrap()) << 64);
        assert_eq!(value.to_x87_extended_bits(), native, "{case}");
    }
}

#[test]
fn turn_step_matches_native() {
    let doc = fixtures();
    let cases = doc["cases"]["step"].as_array().unwrap();
    assert!(cases.len() > 600);
    for case in cases {
        let p = precision(&case["precision"]);
        let mut perf = performer(&case["case"]);
        let mut world = Adapter { case: &case["case"], live: None, events: vec![] };
        let reached = turn_step(&mut perf, 0, bits(&case["target"]), &mut world, p).unwrap();
        assert_eq!(u64::from(reached), case["result"].as_u64().unwrap(), "{case}");
        assert_eq!(after(&perf), case["after"], "{case}");
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}

#[test]
fn aim_matches_native() {
    let doc = fixtures();
    let cases = doc["cases"]["aim"].as_array().unwrap();
    assert!(cases.len() > 500);
    for case in cases {
        let p = precision(&case["precision"]);
        let mut perf = performer(&case["case"]);
        let handle = case["handle"].as_u64().unwrap() as u32;
        let live = (case["handle_kind"] == "live").then_some((handle, [F32::ZERO; 2]));
        let mut world = Adapter { case: &case["case"], live, events: vec![] };
        let index = case["index"].as_u64().unwrap() as usize;
        let reached = aim(&mut perf, index, pair(&case["point"]), handle, &mut world, p).unwrap();
        assert_eq!(u64::from(reached), case["result"].as_u64().unwrap(), "{case}");
        assert_eq!(after(&perf), case["after"], "{case}");
        assert_eq!(
            u64::from(perf.turret_update_pending),
            case["latch_after"].as_u64().unwrap(),
            "{case}"
        );
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}

#[test]
fn reset_update_matches_native() {
    let doc = fixtures();
    let cases = doc["cases"]["reset"].as_array().unwrap();
    assert!(cases.len() > 250);
    for case in cases {
        let p = precision(&case["precision"]);
        let mut perf = performer(&case["case"]);
        let mut world = Adapter { case: &case["case"], live: None, events: vec![] };
        reset_update(&mut perf, &mut world, p).unwrap();
        assert_eq!(after(&perf), case["after"], "{case}");
        assert_eq!(
            u64::from(perf.turret_update_pending),
            case["latch_after"].as_u64().unwrap(),
            "{case}"
        );
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}

#[test]
fn stage0_aim_matches_native() {
    let doc = fixtures();
    let cases = doc["cases"]["stage0"].as_array().unwrap();
    assert!(cases.len() > 200);
    for case in cases {
        let p = precision(&case["precision"]);
        let c = &case["case"];
        let mut perf = performer(c);
        let handle = case["handle"].as_u64().unwrap() as u32;
        perf.state.target_id = handle;
        perf.state.target_position = pair(&c["point"]);
        let live = (c["handle_kind"] == "live").then_some((handle, pair(&c["target_position"])));
        let mut world = Adapter { case: c, live, events: vec![] };
        stage0_aim(&mut perf, &mut world, p).unwrap();
        assert_eq!(after(&perf), case["after"], "{case}");
        assert_eq!(
            u64::from(perf.turret_update_pending),
            case["latch_after"].as_u64().unwrap(),
            "{case}"
        );
        assert_eq!(Value::Array(world.events), case["events"], "{case}");
    }
}
