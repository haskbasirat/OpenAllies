//! Replays research/phase8-tick/native-tick-round2.json: original 0x76F370 /
//! 0x76B230 / CRT _CIpow 0x96D890 (SSE2 and x87 bodies), 0x74B180 and 0x76DD90.
use aa_numeric::{Binary, Precision};
use aa_runtime::simulation::{
    AdminNode, BackupClient, Driver, Error, Gates, NetworkSeams, PowPath, Result, Role,
    SpeedContext, SpeedSettings, SyncLog, Timeline, WorldSeams,
    speed::{pow2_store_f32, pow2_value},
};
use aa_simulation::{FloatBits, NativeClock};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-tick/native-tick-round2.json"
    ))
    .unwrap()
}
fn pc(v: &Value) -> Precision {
    match v.as_u64().unwrap() {
        24 => Precision::Single24,
        53 => Precision::Double53,
        64 => Precision::Extended64,
        _ => panic!(),
    }
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn fb(v: &Value) -> FloatBits {
    FloatBits(u(v))
}
fn flag(v: &Value) -> bool {
    v.as_u64().unwrap() != 0
}
fn f80(hex: &str) -> u128 {
    let mut bytes = [0u8; 16];
    for i in 0..10 {
        bytes[i] = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
    }
    u128::from_le_bytes(bytes)
}
fn path(v: &Value) -> PowPath {
    if flag(v) { PowPath::Sse2 } else { PowPath::X87 }
}
fn timeline(s: &Value) -> Timeline {
    let g = |k: &str| FloatBits(s.get(k).map(u).unwrap_or(0));
    Timeline {
        begin_game: g("begin_game"),
        begin_admin: g("begin_admin"),
        end_game: g("end_game"),
        end_admin: g("end_admin"),
        speed: g("speed"),
        max_speed: g("max_speed"),
        paused: s.get("paused").map(flag).unwrap_or(false),
        frozen: s.get("frozen").map(flag).unwrap_or(false),
        game_per_admin_tick: g("game_per_admin_tick"),
        avi_frames: s.get("avi_frames").map(u).unwrap_or(0) as i32,
    }
}
fn sim_json(t: &Timeline) -> Value {
    json!({"begin_game":t.begin_game.0,"begin_admin":t.begin_admin.0,"end_game":t.end_game.0,
        "end_admin":t.end_admin.0,"speed":t.speed.0,"max_speed":t.max_speed.0,"paused":u8::from(t.paused),
        "frozen":u8::from(t.frozen),"game_per_admin_tick":t.game_per_admin_tick.0,"avi_frames":t.avi_frames as u32})
}

#[test]
fn crt_pow_two_matches_both_native_bodies() {
    let f = fixture();
    let rows = f["pow_rows"].as_array().unwrap();
    assert_eq!(rows.len(), 3132);
    let (mut exact, mut stored, mut refused) = (0, 0, 0);
    for row in rows {
        let (p, precision, y) = (
            path(&row["sse2_path"]),
            pc(&row["precision"]),
            u(&row["exponent"]),
        );
        let native =
            Binary::from_x87_extended_bits(f80(row["result_f80"].as_str().unwrap())).unwrap();
        match pow2_value(y, p, precision) {
            Ok(v) => {
                assert_eq!(
                    v.to_x87_extended_bits(),
                    native.to_x87_extended_bits(),
                    "{row}"
                );
                exact += 1;
            }
            Err(Error::Unsupported(_)) => {}
            Err(e) => panic!("{e} {row}"),
        }
        match pow2_store_f32(y, p, precision) {
            Ok(v) => {
                assert_eq!(v.0, native.store_f32().unwrap(), "{row}");
                stored += 1;
            }
            Err(Error::Unsupported(_)) => refused += 1,
            Err(e) => panic!("{e} {row}"),
        }
    }
    // Integral exponents are reproduced exactly in ST0; every row is
    // reproduced at the binary32 store unless explicitly refused.
    println!("pow rows: exact {exact}, stored {stored}, refused {refused}");
    assert_eq!(stored + refused, rows.len());
    assert!(refused <= 3, "refused {refused}");
    assert_eq!(exact, 2 * 3 * 14, "integral rows");
}

#[test]
fn set_game_speed_matches_native() {
    let f = fixture();
    let rows = f["game_speed_rows"].as_array().unwrap();
    assert_eq!(rows.len(), 96);
    for row in rows {
        let s = &row["svars"];
        let settings = SpeedSettings {
            game_speed_min: fb(&s["game_speed_min"]),
            game_speed_max: fb(&s["game_speed_max"]),
            film_min: fb(&s["film_min"]),
            film_max: fb(&s["film_max"]),
            admin_ticks_per_second: fb(&s["admin_ticks_per_second"]),
        };
        let context = SpeedContext {
            session_mode: row["session_mode"].as_i64().unwrap() as i32,
            session_f4: flag(&row["session_f4"]),
            world_cutscene: flag(&row["world_present"]).then(|| flag(&row["cutscene"])),
        };
        let mut t = timeline(&json!({"speed": row["old_speed"], "max_speed": row["max_speed"]}));
        let change = t
            .set_game_speed(
                fb(&row["argument"]),
                &settings,
                &context,
                path(&row["sse2_path"]),
                pc(&row["precision"]),
            )
            .unwrap();
        assert_eq!(t.speed.0, u(&row["speed"]), "{row}");
        assert_eq!(
            t.game_per_admin_tick.0,
            u(&row["game_per_admin_tick"]),
            "{row}"
        );
        let mut events = vec![json!({"event":"log","name":"m_game_time_per_tick",
            "value": match change.sync_log { SyncLog::GameTimePerTick(v) => v.0, _ => panic!() }})];
        if change.changed {
            events.push(json!({"event":"speed_changed_message"}));
        }
        assert_eq!(json!(events), row["events"], "{row}");
    }
}

struct World(NativeClock);
impl WorldSeams for World {
    fn world_clock(&self) -> Option<NativeClock> {
        Some(self.0)
    }
    fn world_update(&mut self) -> Result<NativeClock> {
        unreachable!()
    }
}
fn driver(precision: Precision, sim: Timeline, admin: FloatBits) -> Driver {
    let mut node = AdminNode::new(Role::Host);
    node.admin_time = admin;
    Driver::new(
        node,
        sim,
        Gates::default(),
        FloatBits::from_f32(10.0),
        precision,
    )
}

#[test]
fn admin_clock_estimate_matches_native_st0() {
    let f = fixture();
    let rows = f["admin_clock_estimate_rows"].as_array().unwrap();
    assert_eq!(rows.len(), 180);
    for row in rows {
        let d = driver(
            pc(&row["precision"]),
            timeline(&row["sim"]),
            fb(&row["admin_time"]),
        );
        let world = World(NativeClock {
            tick: 3,
            begin: FloatBits::ZERO,
            end: fb(&row["world_end"]),
            step: FloatBits::from_f32(0.0625),
        });
        let got = d
            .admin_clock_estimate(fb(&row["real_admin_clock"]), &world)
            .unwrap();
        assert_eq!(
            got.to_x87_extended_bits().unwrap(),
            f80(row["result_f80"].as_str().unwrap()),
            "{row}"
        );
    }
}

struct Net {
    real: FloatBits,
    log: Vec<Value>,
}
impl NetworkSeams for Net {
    fn net_pump(&mut self) -> Result<()> {
        unreachable!()
    }
    fn process_commands(&mut self) -> Result<()> {
        unreachable!()
    }
    fn real_admin_clock(&mut self) -> Result<FloatBits> {
        Ok(self.real)
    }
    fn avi_pump(&mut self) -> Result<i32> {
        unreachable!()
    }
    fn sync_log(&mut self, entry: SyncLog) -> Result<()> {
        match entry {
            SyncLog::Frozen(b) => self
                .log
                .push(json!({"event":"log","name":"_frozen","value":u32::from(b)})),
            other => panic!("{other:?}"),
        }
        Ok(())
    }
    fn replay_record(&mut self) -> Result<()> {
        unreachable!()
    }
    fn replay_playback(&mut self) -> Result<()> {
        unreachable!()
    }
    fn timing_smoothing(&mut self) -> Result<()> {
        unreachable!()
    }
    fn end_condition_check(&mut self) -> Result<()> {
        unreachable!()
    }
    fn client_sync_flush(&mut self) -> Result<()> {
        unreachable!()
    }
    fn host_sync_flush(&mut self) -> Result<()> {
        unreachable!()
    }
    fn admin_history(&mut self) -> Result<()> {
        unreachable!()
    }
    fn host_rate(&mut self, _: FloatBits) -> Result<()> {
        unreachable!()
    }
    fn host_execute_requests(&mut self) -> Result<()> {
        unreachable!()
    }
    fn host_record_history(&mut self) -> Result<()> {
        unreachable!()
    }
    fn client_replay_history(&mut self, _: i32) -> Result<()> {
        unreachable!()
    }
    fn heartbeat(&mut self) -> Result<()> {
        unreachable!()
    }
    fn backup_clients(&mut self) -> Result<Vec<BackupClient>> {
        unreachable!()
    }
    fn backup_mode_entered(&mut self) -> Result<()> {
        unreachable!()
    }
}

#[test]
fn set_frozen_matches_native() {
    let f = fixture();
    let rows = f["set_frozen_rows"].as_array().unwrap();
    assert_eq!(rows.len(), 48);
    for row in rows {
        let mut d = driver(
            pc(&row["precision"]),
            timeline(&row["sim"]),
            fb(&row["admin_time"]),
        );
        let world = World(NativeClock {
            tick: 9,
            begin: fb(&row["world_begin"]),
            end: fb(&row["world_end"]),
            step: FloatBits::from_f32(0.0625),
        });
        let mut net = Net {
            real: fb(&row["real_admin_clock"]),
            log: Vec::new(),
        };
        d.set_frozen(flag(&row["argument"]), &mut net, &world)
            .unwrap();
        assert_eq!(sim_json(&d.timeline), row["after"], "{row}");
        assert_eq!(json!(net.log), row["events"], "{row}");
    }
}
