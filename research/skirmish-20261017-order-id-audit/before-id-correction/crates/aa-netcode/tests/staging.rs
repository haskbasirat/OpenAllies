//! Replays research/phase8-net/native-launch.json: the original launch
//! countdown processes (0x774A50, 0x772350 -> 0x76CFA0, 0x7723F0), the
//! readiness tests (0x765FC0, 0x76C960), KKC_Session::Launch 0x77A950,
//! ChangeToState 0x77A750 and AutoAssignKingdomToPlayer 0x778D10 on native
//! KKC_Session objects. Every seam event (UI messages, request emission,
//! AVar, timer, world steps, state event, settings copies, kingdom setters)
//! is compared in native order, with the request bytes and the session
//! fields afterwards.
use aa_netcode::lobby::{NodeView, PeerDirectory, PeerIdentity, PeerLink, PeerView, Session};
use aa_netcode::session::PeerAddress;
use aa_netcode::staging::{
    all_ready, auto_assign_kingdom, change_to_state, countdown_progress, launch, player_ready, start_countdown,
    stop_countdown, KingdomHost, LaunchStep, SettingsSlot, StagingHost, StagingNode, StateEvent,
    LAUNCH_COUNTDOWN_PROGRESS_REQUEST,
};
use aa_orders::commands::WireWriter;
use aa_orders::payload::Field;
use aa_orders::Position;
use serde_json::{json, Value};

fn de<T: serde::de::DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

struct Peers {
    node: PeerIdentity,
    conns: Vec<(PeerAddress, PeerIdentity)>,
}
impl PeerDirectory for Peers {
    fn peer(&self, link: &PeerLink) -> aa_netcode::Result<PeerView> {
        match link {
            PeerLink::Node => Ok(PeerView { address: None, identity: self.node.clone() }),
            PeerLink::Conn(a) => {
                let identity = self.conns.iter().find(|(x, _)| x == a).map(|(_, i)| i.clone()).unwrap_or_default();
                Ok(PeerView { address: Some(*a), identity })
            }
            PeerLink::Null => panic!("null peer"),
        }
    }
}

struct Host<'a> {
    launch: &'a Value,
    events: Vec<Value>,
    request: Option<(Vec<u8>, Position)>,
}
fn slot(s: SettingsSlot) -> &'static str {
    match s {
        SettingsSlot::Launch => "launch",
        SettingsSlot::Launch88 => "launch_88",
        SettingsSlot::StoreSingle => "store_single",
        SettingsSlot::StoreMulti => "store_multi",
    }
}
impl StagingHost for Host<'_> {
    fn ui_message(&mut self, kind: u32, key: &'static str, number: Option<i32>) -> aa_netcode::Result<()> {
        let args: Vec<u32> = number.map(|n| n as u32).into_iter().collect();
        self.events.push(json!({"event": "ui_message", "kind": kind, "text": units(key), "args": args}));
        Ok(())
    }
    fn request(&mut self, descriptor: usize, fields: Vec<Field>) -> aa_netcode::Result<()> {
        assert_eq!(descriptor, LAUNCH_COUNTDOWN_PROGRESS_REQUEST);
        self.events.push(json!({"event": "request_gate"}));
        self.events.push(json!({"event": "request_player", "player": 0}));
        let [Field::Uint { width: 32, value }] = fields[..] else { panic!("{fields:?}") };
        let mut w = WireWriter::new(Position::PACKET_START).unwrap();
        w.write_uint(descriptor as u32, aa_orders::registry_bits(357).unwrap()).unwrap();
        w.write_uint(value, 32).unwrap();
        let end = w.position();
        let bits = end.absolute().unwrap();
        assert!(self.request.is_none());
        self.request = Some((w.bytes()[..bits.div_ceil(8)].to_vec(), end));
        Ok(())
    }
    fn countdown_time(&mut self) -> aa_netcode::Result<i32> {
        self.events.push(json!({"event": "avar", "name": "LaunchCountdownTime"}));
        Ok(self.launch["countdown_time"].as_i64().unwrap() as i32)
    }
    fn start_timer(&mut self, _s: &mut Session) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "start_timer", "this_is_110": true}));
        Ok(())
    }
    fn not_ready(&mut self, _s: &mut Session) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "not_ready"}));
        Ok(())
    }
    fn launch_step(&mut self, _s: &mut Session, step: LaunchStep) -> aa_netcode::Result<()> {
        self.events.push(match step {
            LaunchStep::Step775DB0 => json!({"event": "launch_step", "fn": "0x775db0", "arg": 1}),
            LaunchStep::Step770500 => json!({"event": "launch_step", "fn": "0x770500"}),
            LaunchStep::Step776B20 => json!({"event": "launch_step", "fn": "0x776b20"}),
        });
        Ok(())
    }
    fn load(&mut self, _s: &mut Session) -> aa_netcode::Result<bool> {
        self.events.push(json!({"event": "launch_step", "fn": "0x775ee0"}));
        Ok(self.launch["launch_byte"].as_u64().unwrap() != 0)
    }
    fn dump_launch_settings(&mut self, _s: &mut Session, label: &'static str) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "dump_settings", "label": label, "settings": "launch"}));
        Ok(())
    }
    fn timeline_freeze(&mut self, flag: bool) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "timeline_freeze", "arg": u8::from(flag)}));
        Ok(())
    }
    fn reset_order_version(&mut self) -> aa_netcode::Result<()> {
        self.events.push(json!({"event": "reset_order_version", "arg": 0}));
        Ok(())
    }
    fn state_changed(&mut self, _s: &mut Session, e: StateEvent) -> aa_netcode::Result<()> {
        // Oracle names: +0x34 (the flag byte) is "field_34", +0x38 (0) is "flag".
        self.events.push(json!({"event": "state_event", "field_0": 0, "text": e.text, "state": e.state,
            "launch_kind": e.launch_state.kind, "launch_field_18": e.launch_state.field_18,
            "field_34": u8::from(e.flag), "flag": 0}));
        Ok(())
    }
    fn copy_launch_settings(&mut self, _s: &mut Session, dst: SettingsSlot, src: SettingsSlot) -> aa_netcode::Result<()> {
        // 0x779F00 is a recording seam in the oracle (no copy is made there).
        self.events.push(json!({"event": "copy_settings", "dst": slot(dst), "src": slot(src)}));
        Ok(())
    }
}
impl KingdomHost for Host<'_> {
    fn kingdom_record_info(&mut self, session: &Session, record: usize) -> aa_netcode::Result<Option<bool>> {
        let name = session.launch.active_map().header.kingdoms[record].name.clone();
        self.events.push(json!({"event": "kingdom_info", "kingdom": name}));
        let open = &self.launch["kingdom_open"][String::from_utf16(&name).unwrap()];
        Ok(open.as_bool())
    }
    fn assign_kingdom(&mut self, session: &mut Session, player: u32, record: usize) -> aa_netcode::Result<()> {
        let name = session.launch.active_map().header.kingdoms[record].name.clone();
        self.events.push(json!({"event": "set_kingdom", "player": player, "kingdom": name, "report": 0}));
        self.events.push(json!({"event": "set_kingdom_30", "record": record, "player": player}));
        self.events.push(json!({"event": "set_kingdom_34", "record": record, "player": player}));
        Ok(())
    }
}

fn link(v: &Value) -> PeerLink {
    de(v)
}

#[test]
fn native_launch_processes() {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-net/native-launch.json");
    let v: Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
    let cases = v["cases"].as_array().unwrap();
    let mut counts = std::collections::BTreeMap::<String, (usize, usize)>::new();
    for (n, c) in cases.iter().enumerate() {
        let case = &c["case"];
        let r = &c["result"];
        let l = &case["launch"];
        let kind = l["kind"].as_str().unwrap();
        assert_eq!(r["status"], "ok", "{n}");
        let mut session: Session = de(&case["session"]);
        session.launch_countdown = l["countdown_118"].as_i64().unwrap() as i32;
        session.host_countdown = l["countdown_11c"].as_i64().unwrap() as i32;
        session.flag_169 = l["flag_169"].as_bool().unwrap();
        (session.flag_16a, session.flag_16b, session.flag_16c) = (false, true, true);
        (session.field_f8, session.field_100) = (0x55, 0x77);
        session.launch.text_30 = units("pending text");
        let nd = &case["node"];
        let node = StagingNode {
            view: NodeView { host: link(&nd["host"]) },
            took_over: l["migrated"].as_i64().unwrap() as i32,
            ordering_player: l["ordering"].as_u64().map(|x| x as u32),
            local_player: l["local"].as_u64().map(|x| x as u32),
            internet: l["internet"].as_bool().unwrap(),
        };
        let mut conns: Vec<(PeerAddress, PeerIdentity)> =
            nd["peers"].as_array().unwrap().iter().map(|p| (de(&p["address"]), de(&p["identity"]))).collect();
        conns.push((de(&nd["conn_address"]), de(&nd["conn_identity"])));
        let peers = Peers { node: de(&nd["identity"]), conns };
        let mut host = Host { launch: l, events: Vec::new(), request: None };
        let mut result = Value::Null;
        match kind {
            "start" => start_countdown(&mut session, &node, &peers, &mut host).unwrap(),
            "stop" => stop_countdown(&mut session, &node, &mut host).unwrap(),
            "progress" => countdown_progress(&mut session, l["value"].as_i64().unwrap() as i32, &node, &mut host).unwrap(),
            "launch" => launch(&mut session, &node, &mut host).unwrap(),
            "change_state" => {
                let st = l["new_state"].as_u64().unwrap() as u32;
                change_to_state(&mut session, st, l["new_flag"].as_u64().unwrap() != 0, &node, &mut host).unwrap()
            }
            "auto_assign" => {
                let id = l["assign_player"].as_u64().unwrap() as u32;
                result = json!(u8::from(auto_assign_kingdom(&mut session, id, &mut host).unwrap()));
            }
            "all_ready" => result = json!(u8::from(all_ready(&session, &node, &peers).unwrap())),
            "ready" => {
                result = json!((0..session.players.len())
                    .map(|i| u8::from(player_ready(&session, i, &node, &peers).unwrap()))
                    .collect::<Vec<_>>())
            }
            k => panic!("{k}"),
        }
        assert_eq!(result, r["result"], "{n} {kind}: result");
        // The oracle seam records ui args for "|d" formats only; others carry [].
        let mut native = r["events"].as_array().unwrap().clone();
        for e in native.iter_mut() {
            if e["event"] == "ui_message" && e.get("args").is_none() {
                e["args"] = json!([]);
            }
        }
        assert_eq!(Value::Array(host.events.clone()), Value::Array(native), "{n} {kind}: events");
        match &host.request {
            Some((bytes, end)) => {
                assert_eq!(json!(bytes), r["request"]["bytes"], "{n} {kind}: request bytes");
                assert_eq!(json!(end), r["request"]["end"], "{n} {kind}: request end");
            }
            None => assert_eq!(r["request"]["end"], json!({"byte": 0, "bit": 3}), "{n} {kind}: no request"),
        }
        let s = &r["session"];
        let mine = json!({
            "countdown_118": session.launch_countdown, "countdown_11c": session.host_countdown,
            "field_164": session.field_164, "flag_169": u8::from(session.flag_169), "flag_16a": u8::from(session.flag_16a),
            "flag_16b": u8::from(session.flag_16b), "flag_16c": u8::from(session.flag_16c), "state": session.state,
            "game_type": session.launch.game_type, "field_f8": session.field_f8, "field_100": session.field_100,
            "changed": u8::from(session.changed), "changed_c9": u8::from(session.changed_c9), "text_84": session.launch.text_30,
            "players": session.players.iter().map(|p| json!({"id": p.id, "kingdom": p.kingdom, "flag_2c": u8::from(p.flag_2c)})).collect::<Vec<_>>(),
        });
        assert_eq!(&mine, s, "{n} {kind}: session after");
        let e = counts.entry(kind.to_string()).or_default();
        e.0 += 1;
        e.1 += usize::from(!host.events.is_empty() || host.request.is_some());
    }
    assert_eq!(cases.len(), 700);
    for (k, (total, active)) in &counts {
        assert!(*active > 0 || k == "ready" || k == "all_ready", "{k}: {active}/{total}");
    }
}
