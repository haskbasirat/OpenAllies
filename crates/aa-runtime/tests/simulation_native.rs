//! Replays research/phase8-tick/native-tick-driver.json: the original
//! 0x75CA70/0x755290/0x76B780/0x76AF90/0x76AF30/0x76DCE0 code run under Unicorn.
use aa_numeric::{Binary, Precision};
use aa_runtime::simulation::{
    Action, AdminNode, BackupClient, Driver, Gates, NetworkSeams, Result, Role, SyncLog, Timeline,
    WorldSeams,
};
use aa_simulation::{FloatBits, NativeClock};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-tick/native-tick-driver.json"
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
fn gates(g: &Value) -> Gates {
    Gates {
        session_f4: flag(&g["session_f4"]),
        session_mode: g["session_mode"].as_i64().unwrap_or(0) as i32,
        flags_b396c4: g["flags_b396c4"].as_u64().unwrap_or(0) as u32,
        single_player_auto_pause: flag(&g["single_player_auto_pause"]),
        render_dump_avi: g.get("avi").map(flag).unwrap_or(false),
        avi_frames_per_tick: g
            .get("avi_frames_per_tick")
            .map(|v| v.as_i64().unwrap() as i32)
            .unwrap_or(0),
    }
}
fn sim_json(t: &Timeline) -> Value {
    json!({"begin_game":t.begin_game.0,"begin_admin":t.begin_admin.0,"end_game":t.end_game.0,
        "end_admin":t.end_admin.0,"speed":t.speed.0,"max_speed":t.max_speed.0,"paused":u8::from(t.paused),
        "frozen":u8::from(t.frozen),"game_per_admin_tick":t.game_per_admin_tick.0,"avi_frames":t.avi_frames as u32})
}
fn timeline_from(s: &Value) -> Timeline {
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

type Log = Rc<RefCell<Vec<Value>>>;
struct Net {
    log: Log,
    real: FloatBits,
    pump: i32,
    clients: Vec<BackupClient>,
    gamespy: bool,
}
impl Net {
    fn push(&self, v: Value) -> Result<()> {
        self.log.borrow_mut().push(v);
        Ok(())
    }
    fn ev(&self, name: &str) -> Result<()> {
        self.push(json!({"event": name}))
    }
}
impl NetworkSeams for Net {
    fn net_pump(&mut self) -> Result<()> {
        self.ev("net_pump")
    }
    fn process_commands(&mut self) -> Result<()> {
        self.ev("process_commands")
    }
    fn real_admin_clock(&mut self) -> Result<FloatBits> {
        Ok(self.real)
    }
    fn avi_pump(&mut self) -> Result<i32> {
        Ok(self.pump)
    }
    fn sync_log(&mut self, entry: SyncLog) -> Result<()> {
        let (name, value) = match entry {
            SyncLog::NextTickEventAdminTime(v) => ("GetNextTickEventAdminTime()", v.0),
            SyncLog::GameTimeBeginTick { value, .. } => {
                ("g_gworld->GetGameTimeBeginTick()", value.0)
            }
            SyncLog::IsFrozen(b) => ("IsFrozen()", u32::from(b)),
            SyncLog::AdminTickEndGameTime(v) => ("m_admin_tick_end_game_time", v.0),
            SyncLog::AdminTickEndAdminTime(v) => ("m_admin_tick_end_admin_time", v.0),
            SyncLog::GameTimePerTick(v) => ("m_game_time_per_tick", v.0),
            SyncLog::Frozen(b) => ("_frozen", u32::from(b)),
        };
        self.push(json!({"event":"log","name":name,"value":value}))
    }
    fn replay_record(&mut self) -> Result<()> {
        self.ev("replay_record")
    }
    fn replay_playback(&mut self) -> Result<()> {
        self.ev("replay_playback")
    }
    fn timing_smoothing(&mut self) -> Result<()> {
        self.ev("timing_smoothing")
    }
    fn end_condition_check(&mut self) -> Result<()> {
        self.ev("end_condition_check")
    }
    fn client_sync_flush(&mut self) -> Result<()> {
        self.ev("sync_flush_client")
    }
    fn host_sync_flush(&mut self) -> Result<()> {
        self.ev("sync_flush_host")
    }
    fn admin_history(&mut self) -> Result<()> {
        self.ev("admin_history_752DA0")
    }
    fn host_rate(&mut self, seconds: FloatBits) -> Result<()> {
        assert_eq!(seconds.0, 0x3dcc_cccd);
        self.ev("host_rate_75FD80")
    }
    fn host_execute_requests(&mut self) -> Result<()> {
        self.ev("host_execute_requests")
    }
    fn host_record_history(&mut self) -> Result<()> {
        self.ev("host_record_history")
    }
    fn client_replay_history(&mut self, index: i32) -> Result<()> {
        self.push(json!({"event":"client_replay_history","index":index as u32}))
    }
    fn heartbeat(&mut self) -> Result<()> {
        self.ev("heartbeat")
    }
    fn backup_clients(&mut self) -> Result<Vec<BackupClient>> {
        Ok(self.clients.clone())
    }
    fn backup_mode_entered(&mut self) -> Result<()> {
        self.ev("backup_mode_entered")?;
        if self.gamespy {
            self.ev("gamespy_backup_report")?;
        }
        Ok(())
    }
}
struct World {
    log: Log,
    clock: Option<NativeClock>,
    precision: Precision,
}
impl WorldSeams for World {
    fn world_clock(&self) -> Option<NativeClock> {
        self.clock
    }
    fn world_update(&mut self) -> Result<NativeClock> {
        let clock = self.clock.as_mut().unwrap();
        self.log
            .borrow_mut()
            .push(json!({"event":"world_update","tick_before":clock.tick}));
        // 0x4F9A07..0x4F9A3B, already proven by phase2/phase7 world fixtures.
        clock.advance_fragment_at_precision(self.precision)?;
        Ok(*clock)
    }
}

#[test]
fn native_frame_driver_cases_replay_exactly() {
    let f = fixture();
    assert_eq!(f["passed"], true);
    let cases = f["driver_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 31);
    let mut frames = 0;
    let mut world_ticks = 0;
    let mut admin_ticks = 0;
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let precision = pc(&case["precision"]);
        let init = &case["initial"];
        let log: Log = Rc::default();
        let w0 = &init["world"];
        let mut world = World {
            log: log.clone(),
            clock: Some(NativeClock {
                tick: u(&w0["tick"]),
                begin: fb(&w0["begin"]),
                end: fb(&w0["end"]),
                step: fb(&w0["step"]),
            }),
            precision,
        };
        // Case setup ran native 0x76DCE0 on a zeroed, unfrozen timeline.
        let mut timeline = timeline_from(&json!({"game_per_admin_tick": case["rate_bits"]}));
        timeline
            .reset_admin_time(
                fb(&case["reset_admin_time_bits"]),
                Some(fb(&w0["begin"])),
                precision,
            )
            .unwrap();
        assert_eq!(sim_json(&timeline), init["sim"], "{name} reset");
        let mut node = AdminNode::new(match case["role"].as_str().unwrap() {
            "host" => Role::Host,
            _ => Role::Client,
        });
        node.admin_time = fb(&case["reset_admin_time_bits"]);
        node.running = true;
        node.has_admin_clock = true;
        node.processed = u(&init["processed"]) as i32;
        node.received = u(&init["received"]) as i32;
        node.heartbeat_at = fb(&init["heartbeat_at"]);
        node.is_server = case["server"].as_bool().unwrap();
        node.backup_mode = flag(&init["backup_mode"]);
        node.backup_since = fb(&init["backup_since"]);
        let mut driver = Driver::new(
            node,
            timeline,
            gates(&case["gates"]),
            FloatBits::from_f32(10.0),
            precision,
        );
        for row in case["frames"].as_array().unwrap() {
            let inputs = &row["inputs"];
            driver.node.received = u(&inputs["received"]) as i32;
            // Fixture writes +0x2D4 directly in the old playback-gate case.
            driver.node.backup_mode = flag(&inputs["playback"]);
            driver.timeline.paused = flag(&inputs["paused"]);
            driver.timeline.frozen = flag(&inputs["frozen"]);
            driver.gates.render_dump_avi = flag(&inputs["avi"]);
            driver.gates.avi_frames_per_tick = u(&inputs["avi_frames_per_tick"]) as i32;
            let clients = row["clients"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| BackupClient {
                    flag_30: flag(&c["f30"]),
                    flag_48: flag(&c["f48"]),
                    flag_49: flag(&c["f49"]),
                    flag_4a: flag(&c["f4a"]),
                    flag_4b: flag(&c["f4b"]),
                })
                .collect();
            let mut net = Net {
                log: log.clone(),
                real: fb(&row["real_admin_clock"]),
                pump: u(&row["pump"]) as i32,
                clients,
                gamespy: name.contains("gamespy"),
            };
            log.borrow_mut().clear();
            let action = driver.frame(fb(&row["now"]), &mut net, &mut world).unwrap();
            let native: Vec<Value> = row["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| {
                    !matches!(
                        e["event"].as_str(),
                        Some(
                            "world_tick"
                                | "admin_tick"
                                | "server_backup_check"
                                | "set_backup_mode"
                                | "diagnostic"
                        )
                    )
                })
                .cloned()
                .collect();
            assert_eq!(*log.borrow(), native, "{name} frame {}", row["frame"]);
            let markers: Vec<&str> = row["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|e| e["event"].as_str())
                .filter(|e| matches!(*e, "world_tick" | "admin_tick"))
                .collect();
            match action {
                Action::WorldTick => {
                    world_ticks += 1;
                    assert_eq!(markers, ["world_tick"], "{name}")
                }
                Action::AdminTick => {
                    admin_ticks += 1;
                    assert_eq!(markers, ["admin_tick"], "{name}")
                }
                _ => assert!(markers.is_empty(), "{name} {action:?}"),
            }
            let s = &row["state"];
            assert_eq!(driver.node.admin_time.0, u(&s["admin_time"]), "{name}");
            assert_eq!(driver.node.processed as u32, u(&s["processed"]), "{name}");
            assert_eq!(driver.node.heartbeat_at.0, u(&s["heartbeat_at"]), "{name}");
            assert_eq!(
                u8::from(driver.node.backup_mode) as u32,
                u(&s["backup_mode"]),
                "{name}"
            );
            assert_eq!(driver.node.backup_since.0, u(&s["backup_since"]), "{name}");
            assert_eq!(driver.counters.client as u32, u(&s["client_sync_counter"]));
            assert_eq!(driver.counters.host as u32, u(&s["host_sync_counter"]));
            assert_eq!(
                sim_json(&driver.timeline),
                s["sim"],
                "{name} frame {}",
                row["frame"]
            );
            let c = world.clock.unwrap();
            assert_eq!(
                json!({"tick":c.tick,"begin":c.begin.0,"end":c.end.0,"step":c.step.0}),
                s["world"],
                "{name}"
            );
            frames += 1;
        }
    }
    assert_eq!(frames, 776);
    assert!(
        world_ticks > 250 && admin_ticks > 200,
        "{world_ticks} {admin_ticks}"
    );
}

#[test]
fn native_timeline_arithmetic_matches_x87_results() {
    let f = fixture();
    let rows = f["pure_cases"].as_array().unwrap();
    assert_eq!(rows.len(), 180);
    for row in rows {
        let precision = pc(&row["precision"]);
        let g = gates(&row["gates"]);
        let t = timeline_from(&row["sim"]);
        let world_end = flag(&json!(u8::from(row["world_present"].as_bool().unwrap())))
            .then(|| fb(&row["world_end"]));
        let next = t
            .next_tick_event_admin_time(&g, world_end, precision)
            .unwrap();
        assert_eq!(
            next.to_x87_extended_bits().unwrap(),
            f80(row["next_tick_event_f80"].as_str().unwrap()),
            "{row}"
        );
        let rate = Binary::from_f32_bits(t.game_time_per_admin_tick(&g).0).unwrap();
        assert_eq!(
            rate.to_x87_extended_bits().unwrap(),
            f80(row["game_per_admin_tick_f80"].as_str().unwrap())
        );
        let end = &row["admin_tick_end"];
        assert_eq!(sim_json(&t)["begin_game"], end["before"]["begin_game"]);
        let mut after = t;
        after
            .end_admin_tick(fb(&end["argument"]), &g, precision)
            .unwrap();
        assert_eq!(sim_json(&after), end["after"], "{row}");
        let reset = &row["reset_admin_time"];
        let mut fresh = timeline_from(&json!({}));
        fresh
            .reset_admin_time(
                fb(&reset["argument"]),
                Some(fb(&reset["world_begin"])),
                precision,
            )
            .unwrap();
        assert_eq!(sim_json(&fresh), reset["after"], "{row}");
        let inc = &row["admin_increment"];
        let mut node = AdminNode::new(Role::Host);
        node.admin_time = fb(&inc["before"]);
        node.processed = 0;
        let mut driver = Driver::new(node, t, g, FloatBits::from_f32(10.0), precision);
        let mut net = Net {
            log: Rc::default(),
            real: FloatBits::ZERO,
            pump: 0,
            clients: Vec::new(),
            gamespy: false,
        };
        driver.admin_tick(&mut net).unwrap();
        assert_eq!(driver.node.admin_time.0, u(&inc["after"]), "{row}");
        assert_eq!(sim_json(&driver.timeline), inc["timeline"], "{row}");
    }
}

#[test]
fn failed_seam_stops_the_driver_without_retry() {
    struct Failing(Net);
    impl NetworkSeams for Failing {
        fn net_pump(&mut self) -> Result<()> {
            Ok(())
        }
        fn process_commands(&mut self) -> Result<()> {
            Ok(())
        }
        fn real_admin_clock(&mut self) -> Result<FloatBits> {
            Ok(FloatBits::from_f32(1.0e6))
        }
        fn avi_pump(&mut self) -> Result<i32> {
            self.0.avi_pump()
        }
        fn sync_log(&mut self, e: SyncLog) -> Result<()> {
            self.0.sync_log(e)
        }
        fn replay_record(&mut self) -> Result<()> {
            Ok(())
        }
        fn replay_playback(&mut self) -> Result<()> {
            Ok(())
        }
        fn timing_smoothing(&mut self) -> Result<()> {
            Ok(())
        }
        fn end_condition_check(&mut self) -> Result<()> {
            Ok(())
        }
        fn client_sync_flush(&mut self) -> Result<()> {
            Ok(())
        }
        fn host_sync_flush(&mut self) -> Result<()> {
            Ok(())
        }
        fn admin_history(&mut self) -> Result<()> {
            Ok(())
        }
        fn host_rate(&mut self, _: FloatBits) -> Result<()> {
            Ok(())
        }
        fn host_execute_requests(&mut self) -> Result<()> {
            Err(aa_runtime::simulation::seam_error("decree handler failed"))
        }
        fn host_record_history(&mut self) -> Result<()> {
            Ok(())
        }
        fn client_replay_history(&mut self, _: i32) -> Result<()> {
            Ok(())
        }
        fn heartbeat(&mut self) -> Result<()> {
            Ok(())
        }
        fn backup_clients(&mut self) -> Result<Vec<BackupClient>> {
            Ok(Vec::new())
        }
        fn backup_mode_entered(&mut self) -> Result<()> {
            Ok(())
        }
    }
    let precision = Precision::Extended64;
    let mut timeline = Timeline::new(FloatBits::from_f32(16.0).0, precision).unwrap();
    timeline.frozen = false;
    timeline
        .reset_admin_time(FloatBits::ZERO, Some(FloatBits::ZERO), precision)
        .unwrap();
    let mut node = AdminNode::new(Role::Host);
    node.running = true;
    node.has_admin_clock = true;
    let mut driver = Driver::new(
        node,
        timeline,
        Gates::default(),
        FloatBits::from_f32(10.0),
        precision,
    );
    let log: Log = Rc::default();
    let mut world = World {
        log: log.clone(),
        clock: Some(NativeClock::stock()),
        precision,
    };
    let mut net = Failing(Net {
        log,
        real: FloatBits::ZERO,
        pump: 0,
        clients: Vec::new(),
        gamespy: false,
    });
    assert!(driver.frame(FloatBits::ZERO, &mut net, &mut world).is_err());
    assert!(driver.stopped());
    // The admin time was not advanced: the failure happened before 0x755465.
    assert_eq!(driver.node.admin_time, FloatBits::ZERO);
    assert!(matches!(
        driver.frame(FloatBits::ZERO, &mut net, &mut world),
        Err(aa_runtime::simulation::Error::Stopped)
    ));
}

#[test]
fn constructor_rate_is_reciprocal_admin_ticks_per_second() {
    // 0x76F240: +0x2C = 1.0 / AdminTicksPerSecond (registered default 16.0).
    let t = Timeline::new(FloatBits::from_f32(16.0).0, Precision::Extended64).unwrap();
    assert_eq!(t.game_per_admin_tick, FloatBits::from_f32(0.0625));
    assert!(t.frozen && !t.paused);
    assert_eq!(t.max_speed.0, 0x4280_0000);
}
