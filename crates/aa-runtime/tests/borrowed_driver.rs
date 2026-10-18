//! Actual native finite-input fixture, with the public borrowed Host boundary.
//! Network/audio/application owners are explicit laboratory services here.
use aa_numeric::Precision;
use aa_orders::{ClientState, Dispatch, History, Position};
use aa_runtime::simulation::{self, borrowed::*, *};
use aa_simulation::{FloatBits, NativeClock};
use serde_json::{Value, json};
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn fb(v: &Value) -> FloatBits {
    FloatBits(u(v))
}
fn pc(v: &Value) -> Precision {
    match v.as_u64().unwrap() {
        24 => Precision::Single24,
        53 => Precision::Double53,
        64 => Precision::Extended64,
        _ => panic!(),
    }
}
fn sim(v: &Value) -> Timeline {
    Timeline {
        begin_game: fb(&v["begin_game"]),
        begin_admin: fb(&v["begin_admin"]),
        end_game: fb(&v["end_game"]),
        end_admin: fb(&v["end_admin"]),
        speed: fb(&v["speed"]),
        max_speed: fb(&v["max_speed"]),
        paused: u(&v["paused"]) != 0,
        frozen: u(&v["frozen"]) != 0,
        game_per_admin_tick: fb(&v["game_per_admin_tick"]),
        avi_frames: u(&v["avi_frames"]) as i32,
    }
}
fn sim_json(v: &Timeline) -> Value {
    json!({"begin_game":v.begin_game.0,"begin_admin":v.begin_admin.0,"end_game":v.end_game.0,"end_admin":v.end_admin.0,"speed":v.speed.0,"max_speed":v.max_speed.0,"paused":u32::from(v.paused),"frozen":u32::from(v.frozen),"game_per_admin_tick":v.game_per_admin_tick.0,"avi_frames":v.avi_frames as u32})
}
struct World(NativeClock);
impl WorldSeams for World {
    fn world_clock(&self) -> Option<NativeClock> {
        Some(self.0)
    }
    fn world_update(&mut self) -> Result<NativeClock> {
        panic!("fixture reaches only admin tick")
    }
}
/// Real retained history/client kernel, with one+20C field. No second
/// Driver.processed exists to receive a copy or an extra increment.
struct Direct {
    client: ClientState,
    history: History,
    client_role: bool,
    admin: FloatBits,
    heartbeat: FloatBits,
    backup: bool,
    backup_since: FloatBits,
    fail: Option<String>,
    mutate_admin: bool,
    events: Vec<Value>,
}
impl Direct {
    fn push(&mut self, event: Value, identity: &str) -> Result<()> {
        self.events.push(event);
        if self.fail.as_deref() == Some(identity) {
            Err(simulation::seam_error(identity))
        } else {
            Ok(())
        }
    }
    fn event(&mut self, name: &str) -> Result<()> {
        self.push(json!({"event":name}), name)
    }
}
impl NetworkSeams for Direct {
    fn net_pump(&mut self) -> Result<()> {
        self.event("net_pump")
    }
    fn process_commands(&mut self) -> Result<()> {
        self.event("process_commands")
    }
    fn real_admin_clock(&mut self) -> Result<FloatBits> {
        Ok(FloatBits(0x42c80000))
    }
    fn avi_pump(&mut self) -> Result<i32> {
        panic!("no AVI fixture")
    }
    fn sync_log(&mut self, v: SyncLog) -> Result<()> {
        let (name, value) = match v {
            SyncLog::NextTickEventAdminTime(v) => ("GetNextTickEventAdminTime()", v.0),
            SyncLog::IsFrozen(v) => ("IsFrozen()", u32::from(v)),
            SyncLog::AdminTickEndGameTime(v) => ("m_admin_tick_end_game_time", v.0),
            SyncLog::AdminTickEndAdminTime(v) => ("m_admin_tick_end_admin_time", v.0),
            SyncLog::Frozen(v) => ("_frozen", u32::from(v)),
            SyncLog::GameTimeBeginTick { value, .. } => {
                ("g_gworld->GetGameTimeBeginTick()", value.0)
            }
            SyncLog::GameTimePerTick(v) => ("m_game_time_per_tick", v.0),
        };
        self.push(json!({"event":"log","name":name,"value":value}), name)
    }
    fn replay_record(&mut self) -> Result<()> {
        panic!("no recorder")
    }
    fn replay_playback(&mut self) -> Result<()> {
        panic!("no playback")
    }
    fn timing_smoothing(&mut self) -> Result<()> {
        panic!("no world tick")
    }
    fn end_condition_check(&mut self) -> Result<()> {
        panic!("no world tick")
    }
    fn client_sync_flush(&mut self) -> Result<()> {
        self.event("sync_flush_client")
    }
    fn host_sync_flush(&mut self) -> Result<()> {
        self.event("sync_flush_host")
    }
    fn admin_history(&mut self) -> Result<()> {
        self.event("admin_history_752DA0")
    }
    fn host_rate(&mut self, v: FloatBits) -> Result<()> {
        assert_eq!(v.0, 0x3dcccccd);
        self.event("host_rate_75FD80")
    }
    fn host_execute_requests(&mut self) -> Result<()> {
        self.event("host_execute_requests")
    }
    fn host_record_history(&mut self) -> Result<()> {
        if self.mutate_admin {
            self.admin = FloatBits(0x41200000)
        }
        self.event("host_record_history")
    }
    fn client_replay_history(&mut self, _: i32) -> Result<()> {
        panic!("borrowed whole74FD10 uses SAME client owner, not legacy prefix")
    }
    fn heartbeat(&mut self) -> Result<()> {
        self.event("heartbeat")
    }
    fn backup_clients(&mut self) -> Result<Vec<BackupClient>> {
        panic!("no server-tail fixture")
    }
    fn backup_mode_entered(&mut self) -> Result<()> {
        panic!("no backup entry")
    }
}
impl Host for Direct {
    //Historical direct-kernel fixture excludes the outer752AE0 timer/history.
    fn begin_world_duration(&mut self,_:Precision)->Result<Option<u32>>{Ok(None)}
    fn finish_world_duration(&mut self,_:u32,_:Precision)->Result<()>{panic!("outside this direct fixture")}
    fn role(&self) -> Result<Role> {
        Ok(if self.client_role {
            Role::Client
        } else {
            Role::Host
        })
    }
    fn flag(&self, v: NodeFlag) -> Result<bool> {
        Ok(match v {
            NodeFlag::Running | NodeFlag::HasAdminClock => true,
            NodeFlag::BackupMode => self.backup,
            _ => false,
        })
    }
    fn word(&self, v: NodeWord) -> Result<FloatBits> {
        Ok(match v {
            NodeWord::AdminTime => self.admin,
            NodeWord::HeartbeatAt => self.heartbeat,
            NodeWord::BackupSince => self.backup_since,
            NodeWord::DisconnectStarted => panic!("not disconnecting"),
        })
    }
    fn store_flag(&mut self, v: NodeFlag, value: bool) -> Result<()> {
        assert_eq!(v, NodeFlag::BackupMode);
        self.backup = value;
        Ok(())
    }
    fn store_word(&mut self, v: NodeWord, value: FloatBits) -> Result<()> {
        match v {
            NodeWord::AdminTime => self.admin = value,
            NodeWord::HeartbeatAt => self.heartbeat = value,
            NodeWord::BackupSince => self.backup_since = value,
            _ => panic!("unexpected write"),
        };
        Ok(())
    }
    fn processed(&self) -> Result<i32> {
        Ok(self.client.next_index)
    }
    fn received(&self) -> Result<i32> {
        Ok(self.history.end())
    }
    fn gate_flag(&self, v: GateFlag) -> Result<bool> {
        Ok(match v {
            GateFlag::SessionF4 => true,
            GateFlag::SinglePlayerAutoPause => panic!("native lazy gate should skip"),
            GateFlag::RenderDumpAvi => false,
        })
    }
    fn gate_word(&self, v: GateWord) -> Result<u32> {
        Ok(match v {
            GateWord::SessionMode => 2,
            _ => panic!("native lazy gate should skip"),
        })
    }
    fn async_ticks_per_second(&self) -> Result<FloatBits> {
        Ok(FloatBits(0x41200000))
    }
    fn replay_client_tick(&mut self) -> Result<()> {
        self.push(
            json!({"event":"client_replay_history","index":self.client.next_index as u32}),
            "client_replay_history",
        )?;
        aa_orders::consume_next(
            &mut self.history,
            &mut self.client,
            |packet, start, _, _| {
                assert!(packet.is_none());
                assert!(start.is_none());
                Ok(Dispatch {
                    events: Vec::new(),
                    position: Position::PACKET_START,
                    executed: 0,
                    stopped_before_execute: None,
                })
            },
        )
        .map_err(|e| simulation::seam_error(e.to_string()))?;
        Ok(())
    }
}
#[test]
fn borrowed_kernel_replays_both_native_images_all_pcs_and_error_prefixes_on_same_node() {
    let f: Value = serde_json::from_str(include_str!(
        "../../../research/development10/live-sim/native-borrowed-driver.json"
    ))
    .unwrap();
    assert_eq!(f["rows"].as_array().unwrap().len(), 90);
    for row in f["rows"].as_array().unwrap() {
        let n = &row["native"];
        let initial = &n["initial"];
        let final_ = &n["final"];
        let mut history = History::new(0, 16).unwrap();
        history.set_end(u(&initial["received"]) as i32).unwrap();
        let mut host = Direct {
            client: ClientState::new(u(&initial["processed"]) as i32),
            history,
            client_role: row["input"]["client"].as_bool().unwrap(),
            admin: fb(&initial["admin_time"]),
            heartbeat: fb(&initial["heartbeat_at"]),
            backup: u(&initial["backup_mode"]) != 0,
            backup_since: fb(&initial["backup_since"]),
            fail: row["input"]["stop"].as_str().map(str::to_owned),
            mutate_admin: row["input"]["mutate_admin"].as_bool().unwrap(),
            events: Vec::new(),
        };
        let mut timeline = sim(&initial["sim"]);
        let w = &initial["world"];
        let mut world = World(NativeClock {
            tick: u(&w["tick"]),
            begin: fb(&w["begin"]),
            end: fb(&w["end"]),
            step: fb(&w["step"]),
        });
        let mut counters = SyncCounters {
            client: u(&initial["client_sync_counter"]) as i32,
            host: u(&initial["host_sync_counter"]) as i32,
        };
        let mut stopped = false;
        let before_client = &host.client as *const ClientState;
        let result = Kernel {
            sim: &mut timeline,
            control: Control {
                counters: &mut counters,
                stopped: &mut stopped,
            },
            precision: pc(&row["precision"]),
        }
        .frame(FloatBits(0x3f800000), &mut host, &mut world);
        assert_eq!(result.is_err(), n["failed"].is_string(), "{}", row["name"]);
        assert_eq!(stopped, result.is_err());
        assert_eq!(before_client, &host.client as *const ClientState);
        let events: Vec<_> = n["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["event"] != "admin_tick")
            .cloned()
            .collect();
        assert_eq!(host.events, events, "{}", row["name"]);
        assert_eq!(sim_json(&timeline), final_["sim"], "{}", row["name"]);
        assert_eq!(host.admin.0, u(&final_["admin_time"]));
        assert_eq!(host.heartbeat.0, u(&final_["heartbeat_at"]));
        assert_eq!(host.client.next_index as u32, u(&final_["processed"]));
        assert_eq!(host.history.end() as u32, u(&final_["received"]));
        assert_eq!(counters.client as u32, u(&final_["client_sync_counter"]));
        assert_eq!(counters.host as u32, u(&final_["host_sync_counter"]));
        if result.is_err() {
            let count = host.events.len();
            assert!(
                Kernel {
                    sim: &mut timeline,
                    control: Control {
                        counters: &mut counters,
                        stopped: &mut stopped
                    },
                    precision: pc(&row["precision"])
                }
                .frame(FloatBits(0x3f800000), &mut host, &mut world)
                .is_err()
            );
            assert_eq!(host.events.len(), count);
        }
    }
}

#[test]
fn missing_end_admin_retains_both_begin_words_at_native_read_boundary() {
    use aa_runtime::live_sim::Sim;
    let proof: Value = serde_json::from_str(include_str!(
        "../../../research/development10/live-sim/native-admin-end-read-prefix.json"
    ))
    .unwrap();
    assert_eq!(proof["rows"].as_array().unwrap().len(), 6);
    for row in proof["rows"].as_array().unwrap() {
        assert_eq!(row["native"]["before"], row["native"]["after"]);
        assert_eq!(row["native"]["boundary"], "0x76b7a6");
        let mut sim = Sim::from_native_constructor_fields();
        sim.store_time(TimeWord::BeginGame, FloatBits(0x40a00000))
            .unwrap();
        sim.store_time(TimeWord::BeginAdmin, FloatBits(0x40c00000))
            .unwrap();
        sim.store_time(TimeWord::EndGame, FloatBits(0x41200000))
            .unwrap();
        sim.store_frozen(false);
        let mut history = History::new(0, 16).unwrap();
        history.set_end(10).unwrap();
        let mut host = Direct {
            client: ClientState::new(4),
            history,
            client_role: false,
            admin: FloatBits(0x40c00000),
            heartbeat: FloatBits::ZERO,
            backup: false,
            backup_since: FloatBits::ZERO,
            fail: None,
            mutate_admin: false,
            events: Vec::new(),
        };
        let before = sim.time_words();
        let mut counters = SyncCounters { client: 0, host: 0 };
        let mut stopped = false;
        let error = Kernel {
            sim: &mut sim,
            control: Control {
                counters: &mut counters,
                stopped: &mut stopped,
            },
            precision: pc(&row["precision"]),
        }
        .admin_tick(&mut host)
        .unwrap_err();
        assert!(error.to_string().contains("time word"));
        assert_eq!(sim.time_words(), before);
        assert_eq!(host.admin.0, 0x40c20000);
        assert_eq!(
            host.events.last(),
            Some(&json!({"event":"log","name":"IsFrozen()","value":0}))
        );
    }
}
