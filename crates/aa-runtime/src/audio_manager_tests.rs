//! Replays research/phase8-audio/native-audio-manager.json: the original
//! 749130/74AD70/7474F0/747630/7476E0/747A80/7495x0/744F90/7450E0/747150..
//! executed under Unicorn (native_audio_manager.py), step by step, with the
//! full manager/instance/pool state and the ordered Miles, resource, UI draw,
//! fog and FX-completion calls after every step.
use super::*;
use serde_json::{json, Value};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

type Trace = Rc<RefCell<Vec<Value>>>;

fn bits(v: &Value) -> u32 {
    (v.as_f64().expect("number") as f32).to_bits()
}
fn word(v: &Value) -> u32 {
    v.as_u64().expect("word") as u32
}

struct TestEnv {
    trace: Trace,
    now: i64,
    anchor: i64,
    case: Value,
    env: serde_json::Map<String, Value>,
    draws: Vec<u32>,
    voice_ids: Vec<String>,
}
impl TestEnv {
    fn flag(&self, k: &str) -> bool {
        self.env[k].as_bool().expect(k)
    }
    fn object_by_handle(&self, handle: u32) -> Option<(String, Value)> {
        let objects = self.case["objects"].as_object()?;
        objects.iter().find(|(_, o)| word(&o["handle"]) == handle).map(|(k, o)| (k.clone(), o.clone()))
    }
    fn alive(&self, name: &str) -> bool {
        self.env.get("alive").and_then(|a| a.get(name)).and_then(Value::as_bool).unwrap_or(true)
    }
    fn var(&self, name: &str) -> Value {
        self.case["vars"][name].clone()
    }
}
impl Clock for TestEnv {
    fn qpc(&mut self) -> Result<i64> {
        Ok(self.now)
    }
    fn qpc_frequency(&mut self) -> Result<i64> {
        Ok(1000)
    }
}
impl Env<String, String> for TestEnv {
    fn b396e0_ms(&mut self) -> Result<u32> {
        Ok((self.now - self.anchor) as u32)
    }
    fn ui_random_d4(&mut self) -> Result<u32> {
        if self.draws.is_empty() {
            return Err("scripted UI draws exhausted".into());
        }
        let v = self.draws.remove(0);
        self.trace.borrow_mut().push(json!({"op": "draw", "value": v}));
        Ok(v)
    }
    fn ui_present(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn vworld_ready_43aaf0(&mut self) -> Result<bool> {
        Ok(self.flag("vworld_ready"))
    }
    fn game_running_76aee0(&mut self) -> Result<bool> {
        Ok(self.flag("running"))
    }
    fn cinematic_5f41f0(&mut self) -> Result<bool> {
        Ok(self.flag("cinematic"))
    }
    fn speech_gate(&mut self) -> Result<Option<(u32, u32)>> {
        let g = &self.env["speech_gate"];
        Ok(Some((bits(&g[0]), bits(&g[1]))))
    }
    fn subtitle_owner(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn application_kind(&mut self) -> Result<u32> {
        Ok(word(&self.env["application_kind"]))
    }
    fn application_active(&mut self) -> Result<bool> {
        Ok(self.flag("active"))
    }
    fn dev_mode_40eaa0(&mut self) -> Result<bool> {
        Ok(self.flag("dev_mode"))
    }
    fn audio_debug_ids(&mut self) -> Result<Option<String>> {
        Ok(self.env.get("debug_ids").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_owned))
    }
    fn on_view_thread(&mut self) -> Result<bool> {
        Ok(self.flag("view_thread"))
    }
    fn object_position(&mut self, handle: u32) -> Result<Option<[u32; 3]>> {
        let Some((name, spec)) = self.object_by_handle(handle) else { return Ok(None) };
        if !self.alive(&name) {
            return Ok(None);
        }
        let p = self.env.get("positions").and_then(|p| p.get(&name)).cloned().unwrap_or_else(|| spec.get("position").cloned().unwrap_or(json!([0, 0, 0])));
        Ok(Some([bits(&p[0]), bits(&p[1]), bits(&p[2])]))
    }
    fn object_voice_owner(&mut self, handle: u32) -> Result<Option<u32>> {
        let Some((name, spec)) = self.object_by_handle(handle) else { return Ok(None) };
        if !self.alive(&name) {
            return Ok(None);
        }
        let Some(voice) = spec.get("voice").and_then(Value::as_str) else { return Ok(Some(0)) };
        Ok(Some(self.voice_ids.iter().position(|v| v == voice).unwrap() as u32 + 1))
    }
    fn owner_relation(&mut self, handle: u32) -> Result<Option<u32>> {
        let Some((name, spec)) = self.object_by_handle(handle) else { return Ok(None) };
        if !self.alive(&name) {
            return Ok(None);
        }
        Ok(Some(if self.env["local"].is_null() { 2 } else { word(&spec["relation"]) }))
    }
    fn fog_audible(&mut self, xy: [u32; 2]) -> Result<bool> {
        self.trace.borrow_mut().push(json!({"op": "fog_tile", "x": xy[0], "y": xy[1]}));
        if self.env["fog_tile"].is_null() || self.flag("fog_off") {
            return Ok(true);
        }
        Ok(self.flag("fog_visible"))
    }
    fn camera(&mut self) -> Result<Option<Camera>> {
        let c = &self.env["camera"];
        if c.is_null() {
            return Ok(None);
        }
        let v = |k: &str| [bits(&c[k][0]), bits(&c[k][1]), bits(&c[k][2])];
        Ok(Some(Camera { zoom: bits(&c["zoom"]), eye: v("eye"), center: v("center") }))
    }
    fn uvar_i32(&mut self, name: &str) -> Result<i32> {
        let v = self.var(name);
        Ok(v[1].as_i64().ok_or(format!("{name} int"))? as i32)
    }
    fn uvar_f32(&mut self, name: &str) -> Result<u32> {
        Ok(bits(&self.var(name)[1]))
    }
    fn avar_f32(&mut self, name: &str) -> Result<u32> {
        Ok(bits(&self.var(name)[1]))
    }
    fn user_volume(&mut self, index: u32) -> Result<u32> {
        Ok(bits(&self.case["user_volumes"][format!("{index:x}")]))
    }
    fn resource_request(&mut self, r: &String, delta: i32, mode: u32) -> Result<()> {
        self.trace.borrow_mut().push(json!({"op": "request", "resource": r, "delta": delta, "mode": mode}));
        Ok(())
    }
    fn resource_release(&mut self, r: &String, delta: i32) -> Result<()> {
        self.trace.borrow_mut().push(json!({"op": "release", "resource": r, "delta": delta}));
        Ok(())
    }
    fn resource_sync_load(&mut self, r: &String) -> Result<()> {
        self.trace.borrow_mut().push(json!({"op": "sync_load", "resource": r}));
        Ok(())
    }
    fn raw_data_59b030(&mut self, r: &String) -> Result<Option<u32>> {
        let memory = self.case["resources"][r].get("memory").map(word).unwrap_or(0);
        Ok((memory != 0).then_some(memory))
    }
    fn fx_complete_428af0(&mut self, link: &String) -> Result<()> {
        self.trace.borrow_mut().push(json!({"op": "fx_complete", "node": link}));
        Ok(())
    }
    fn music_user_changed(&mut self) -> Result<()> {
        Ok(())
    }
}

struct TestMiles {
    trace: Trace,
    case: Value,
    next: [u32; 3],
}
impl TestMiles {
    fn ail(&self, name: &str, args: Value) {
        self.trace.borrow_mut().push(json!({"op": "ail", "name": name, "args": args}));
    }
    fn cb(enabled: bool, name: &str) -> &str {
        if enabled { name } else { "none" }
    }
}
impl Miles<String> for TestMiles {
    fn set_redist_directory(&mut self, d: &str) -> Result<()> { self.ail("set_redist_directory", json!([d])); Ok(()) }
    fn startup(&mut self) -> Result<()> { self.ail("startup", json!([])); Ok(()) }
    fn open_digital_driver(&mut self, f: i32, b: i32, c: i32, fl: u32) -> Result<u32> { self.ail("open_digital_driver", json!([f, b, c, fl])); Ok(0x1000) }
    fn enumerate_3d_providers(&mut self, cursor: &mut u32) -> Result<Option<(u32, String)>> {
        self.ail("enumerate_3d_providers", json!([]));
        let providers = self.case["providers"].as_array().unwrap();
        let Some(p) = providers.get(*cursor as usize) else { return Ok(None) };
        *cursor += 1;
        Ok(Some((word(&p[0]), p[1].as_str().unwrap().to_owned())))
    }
    fn open_3d_provider(&mut self, p: u32) -> Result<u32> { self.ail("open_3d_provider", json!([p])); Ok(self.case.get("open_provider_result").map(word).unwrap_or(0)) }
    fn close_3d_provider(&mut self, p: u32) -> Result<()> { self.ail("close_3d_provider", json!([p])); Ok(()) }
    fn set_3d_speaker_type(&mut self, p: u32, s: u32) -> Result<()> { self.ail("set_3d_speaker_type", json!([p, s])); Ok(()) }
    fn open_3d_listener(&mut self, p: u32) -> Result<u32> { self.ail("open_3d_listener", json!([p])); Ok(0x4000) }
    fn close_3d_listener(&mut self, l: u32) -> Result<()> { self.ail("close_3d_listener", json!([l])); Ok(()) }
    fn set_file_callbacks(&mut self) -> Result<()> { self.ail("set_file_callbacks", json!([])); Ok(()) }
    fn set_preference(&mut self, n: u32, v: i32) -> Result<()> { self.ail("set_preference", json!([n, v as u32])); Ok(()) }
    fn close_digital_driver(&mut self, d: u32) -> Result<()> { self.ail("close_digital_driver", json!([d])); Ok(()) }
    fn shutdown(&mut self) -> Result<()> { self.ail("shutdown", json!([])); Ok(()) }
    fn allocate_sample_handle(&mut self, d: u32) -> Result<u32> {
        self.ail("allocate_sample_handle", json!([d]));
        let h = 0x2000 + self.next[0];
        self.next[0] += 1;
        let fails = self.case.get("sample_failures").and_then(Value::as_array).cloned().unwrap_or_default();
        Ok(if fails.iter().any(|f| word(f) == h - 0x2000) { 0 } else { h })
    }
    fn init_sample(&mut self, s: u32) -> Result<()> { self.ail("init_sample", json!([s])); Ok(()) }
    fn release_sample_handle(&mut self, s: u32) -> Result<()> { self.ail("release_sample_handle", json!([s])); Ok(()) }
    fn set_sample_file(&mut self, s: u32, m: u32, b: u32) -> Result<()> { self.ail("set_sample_file", json!([s, m, b])); Ok(()) }
    fn register_eos_callback(&mut self, s: u32, e: bool) -> Result<()> { self.ail("register_eos_callback", json!([s, Self::cb(e, "eos_2d")])); Ok(()) }
    fn start_sample(&mut self, s: u32) -> Result<()> { self.ail("start_sample", json!([s])); Ok(()) }
    fn stop_sample(&mut self, s: u32) -> Result<()> { self.ail("stop_sample", json!([s])); Ok(()) }
    fn resume_sample(&mut self, s: u32) -> Result<()> { self.ail("resume_sample", json!([s])); Ok(()) }
    fn sample_status(&mut self, s: u32) -> Result<u32> { self.ail("sample_status", json!([s])); Ok(4) }
    fn sample_playback_rate(&mut self, s: u32) -> Result<i32> { self.ail("sample_playback_rate", json!([s])); Ok(self.rate()) }
    fn set_sample_playback_rate(&mut self, s: u32, r: i32) -> Result<()> { self.ail("set_sample_playback_rate", json!([s, r as u32])); Ok(()) }
    fn set_sample_volume_pan(&mut self, s: u32, v: u32, p: u32) -> Result<()> { self.ail("set_sample_volume_pan", json!([s, v, p])); Ok(()) }
    fn allocate_3d_sample_handle(&mut self, p: u32) -> Result<u32> {
        self.ail("allocate_3d_sample_handle", json!([p]));
        let h = 0x3000 + self.next[1];
        self.next[1] += 1;
        Ok(h)
    }
    fn release_3d_sample_handle(&mut self, s: u32) -> Result<()> { self.ail("release_3d_sample_handle", json!([s])); Ok(()) }
    fn set_3d_sample_file(&mut self, s: u32, m: u32) -> Result<()> { self.ail("set_3d_sample_file", json!([s, m])); Ok(()) }
    fn register_3d_eos_callback(&mut self, s: u32, e: bool) -> Result<()> { self.ail("register_3d_eos_callback", json!([s, Self::cb(e, "eos_3d")])); Ok(()) }
    fn start_3d_sample(&mut self, s: u32) -> Result<()> { self.ail("start_3d_sample", json!([s])); Ok(()) }
    fn stop_3d_sample(&mut self, s: u32) -> Result<()> { self.ail("stop_3d_sample", json!([s])); Ok(()) }
    fn resume_3d_sample(&mut self, s: u32) -> Result<()> { self.ail("resume_3d_sample", json!([s])); Ok(()) }
    fn sample_3d_status(&mut self, s: u32) -> Result<u32> { self.ail("sample_3d_status", json!([s])); Ok(4) }
    fn sample_3d_playback_rate(&mut self, s: u32) -> Result<i32> { self.ail("sample_3d_playback_rate", json!([s])); Ok(self.rate()) }
    fn set_3d_sample_playback_rate(&mut self, s: u32, r: i32) -> Result<()> { self.ail("set_3d_sample_playback_rate", json!([s, r as u32])); Ok(()) }
    fn set_3d_sample_volume(&mut self, s: u32, v: u32) -> Result<()> { self.ail("set_3d_sample_volume", json!([s, v])); Ok(()) }
    fn set_3d_sample_distances(&mut self, s: u32, max: u32, min: u32) -> Result<()> { self.ail("set_3d_sample_distances", json!([s, max, min])); Ok(()) }
    fn set_3d_position(&mut self, o: u32, p: [u32; 3]) -> Result<()> { self.ail("set_3d_position", json!([o, p[0], p[1], p[2]])); Ok(()) }
    fn set_3d_orientation(&mut self, o: u32, f: [u32; 3], u: [u32; 3]) -> Result<()> { self.ail("set_3d_orientation", json!([o, f[0], f[1], f[2], u[0], u[1], u[2]])); Ok(()) }
    fn open_stream(&mut self, d: u32, r: &String) -> Result<u32> {
        self.ail("open_stream", json!([d, r, 0]));
        let h = 0x5000 + self.next[2];
        self.next[2] += 1;
        Ok(h)
    }
    fn close_stream(&mut self, s: u32) -> Result<()> { self.ail("close_stream", json!([s])); Ok(()) }
    fn start_stream(&mut self, s: u32) -> Result<()> { self.ail("start_stream", json!([s])); Ok(()) }
    fn pause_stream(&mut self, s: u32, on: i32) -> Result<()> { self.ail("pause_stream", json!([s, on as u32])); Ok(()) }
    fn stream_status(&mut self, s: u32) -> Result<u32> { self.ail("stream_status", json!([s])); Ok(4) }
    fn register_stream_callback(&mut self, s: u32, e: bool) -> Result<()> { self.ail("register_stream_callback", json!([s, Self::cb(e, "eos_stream")])); Ok(()) }
    fn set_stream_volume_pan(&mut self, s: u32, v: u32, p: u32) -> Result<()> { self.ail("set_stream_volume_pan", json!([s, v, p])); Ok(()) }
    fn stream_length_ms(&mut self, _: u32) -> Result<i32> { Err("unused".into()) }
    fn file_read(&mut self, _: &String) -> Result<u32> { Err("unused".into()) }
    fn mem_free_lock(&mut self, m: u32) -> Result<()> { self.ail("mem_free_lock", json!([m])); Ok(()) }
    fn wav_info(&mut self, _: u32) -> Result<WavInfo> { Err("unused".into()) }
}
impl TestMiles {
    fn rate(&self) -> i32 {
        self.case.get("playback_rate").map(|v| v.as_i64().unwrap() as i32).unwrap_or(22050)
    }
}

fn pair(d: &Value, k: &str) -> [u32; 2] {
    match d.get(k) {
        Some(v) => [bits(&v[0]), bits(&v[1])],
        None => [0, 0],
    }
}
fn names(d: &Value, k: &str) -> Vec<String> {
    d.get(k).and_then(Value::as_array).map(|a| a.iter().map(|v| v.as_str().unwrap().to_owned()).collect()).unwrap_or_default()
}
fn build_data(case: &Value) -> BTreeMap<String, DataRef<String>> {
    case["data"].as_object().unwrap().iter().map(|(name, d)| {
        let data = AudioData::new(
            d["ids"].as_str().unwrap().to_owned(),
            d["category"].as_i64().unwrap() as i32,
            2,
            [word(&d["control"]) as u8, word(&d["world"]) as u8, word(&d["who"]) as u8],
            bits(&d["volume"]),
            [bits(&d["range"][0]), bits(&d["range"][1])],
            d["limit"].as_i64().unwrap() as i32,
            [names(d, "leadin"), names(d, "files"), names(d, "fallout")],
            pair(d, "fade"),
            pair(d, "delay"),
            pair(d, "pitch"),
            d.get("volume_adjust").map(bits).unwrap_or(0),
            d.get("ldf").map(bits).unwrap_or(0),
            d.get("subtitle").map(bits).unwrap_or(0),
        );
        (name.clone(), Arc::new(data))
    }).collect()
}

fn channel_value(m: &Manager<String, String>, c: Option<usize>) -> Value {
    match c {
        None => Value::Null,
        Some(c) => {
            let c = &m.miles.channels[c];
            json!([c.kind, c.index as u32, c.sample_0c, c.sample3d_10, c.stream_14, if c.kind == 2 { c.driver_18 } else { 0 }])
        }
    }
}
fn state(m: &Manager<String, String>, data: &BTreeMap<String, DataRef<String>>, voices: &[String], guard: u32) -> Value {
    let data_name = |d: &DataRef<String>| data.iter().find(|(_, v)| Arc::ptr_eq(v, d)).map(|(k, _)| k.clone()).unwrap();
    let mut instances = serde_json::Map::new();
    for (id, i) in &m.instances {
        let voice = i.voice_5c.map(|v| voices[(v - 1) as usize].clone());
        instances.insert(id.to_string(), json!({
            "id": i.id, "data": data_name(&i.data), "interrupts_10": i.interrupts_10, "claimed_by_14": i.claimed_by_14,
            "channel": channel_value(m, i.channel_18), "mode_1c": i.mode_1c as u32, "object_20": i.object_20,
            "owner_24": i.owner_24, "position": i.position_28, "state_34": i.state_34 as u32, "loop_38": i.loop_38 as u32,
            "released_39": i.released_39 as u32, "current": i.current_3c, "leadin": i.leadin_40, "fallout": i.fallout_44,
            "main": i.main_48, "start_4c": i.start_4c, "fx": i.fx_54, "voice_5c": voice,
            "fade_timer": i.fade_timer_60.map(|t| t.anchor as u32), "fade_64": i.fade_64 as u32,
            "delay_68": i.delay_68, "pitch_6c": i.pitch_6c, "volume_70": i.volume_70,
        }));
    }
    let groups: Vec<Value> = m.volumes.groups.iter().map(|g| json!([g.user, g.game, g.fade, g.target, g.remaining, g.timer.anchor as u32, g.changed as u32])).collect();
    let free: Vec<Value> = m.miles.free.iter().map(|l| Value::Array(l.iter().map(|&c| channel_value(m, Some(c))).collect())).collect();
    let queue: Vec<Value> = m.queue.iter().map(|c| json!([c.kind, c.audio.unwrap_or(0), c.id, c.time, c.mask])).collect();
    json!({
        "lists": m.lists, "music_38": m.music_38 as u32, "speech_3c": m.speech_3c as u32, "queue": queue,
        "next_id_9c": m.next_id_9c, "groups": groups, "recover_168": m.recover_168.anchor as u32,
        "mic": m.mic_170, "orient": m.orient_17c,
        "flags": [m.paused_188 as u32, m.disabled_189 as u32, m.fade_18a as u32, m.recover_18b as u32, m.latency_18c as u32],
        "hold_190": m.hold_190 as u32, "initialized": m.miles.initialized_198 as u32, "fast_2d": m.miles.fast_2d_19c as u32,
        "open": m.miles.open_1a0 as u32, "driver": m.miles.driver_3a8, "listener": m.miles.listener_3d0, "free": free,
        "available": [m.miles.available[0] as u32, m.miles.available[1] as u32, m.miles.available[2] as u32],
        "deferred": m.miles.deferred, "voices": voices.iter().map(|v| (v.clone(), json!(m.miles.voices_5bc.contains(&((voices.iter().position(|x| x == v).unwrap() + 1) as u32)) as u32))).collect::<serde_json::Map<_, _>>(),
        "instances": instances, "guard": guard,
    })
}
fn object_arg(case: &Value, name: &Value) -> Option<ObjectArg> {
    let name = name.as_str()?;
    let o = &case["objects"][name];
    let owner = o.get("owner").and_then(Value::as_str).map(|k| word(&case["objects"][k]["handle"])).unwrap_or(0);
    Some(ObjectArg { handle: word(&o["handle"]), owner_108: owner })
}
fn object_handle(case: &Value, name: &Value) -> u32 {
    name.as_str().map(|n| word(&case["objects"][n]["handle"])).unwrap_or(0)
}
fn position(v: &Value) -> Option<[u32; 3]> {
    v.as_array().map(|p| [bits(&p[0]), bits(&p[1]), bits(&p[2])])
}
fn without_logs(trace: &Value) -> Value {
    Value::Array(trace.as_array().unwrap().iter().filter(|t| t["op"] != "log").cloned().collect())
}

fn replay(row: &Value) {
    let pc = match row["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        _ => Precision::Extended64,
    };
    let case = &row["case"];
    let trace: Trace = Rc::new(RefCell::new(Vec::new()));
    let voices: Vec<String> = case["voice_owners"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_owned()).collect();
    let mut env = TestEnv {
        trace: Rc::clone(&trace),
        now: case.get("start_ms").map(|v| v.as_i64().unwrap()).unwrap_or(1000),
        anchor: case["env"].get("timer_anchor").map(|v| v.as_i64().unwrap()).unwrap_or(0),
        case: case.clone(),
        env: case["env"].as_object().unwrap().clone(),
        draws: case["draws"].as_array().unwrap().iter().map(word).collect(),
        voice_ids: voices.clone(),
    };
    let mut miles = TestMiles { trace: Rc::clone(&trace), case: case.clone(), next: [0; 3] };
    let data = build_data(case);
    let name = row["name"].as_str().unwrap();
    let mut m = Manager::<String, String>::construct_749130(&mut env, pc).unwrap();
    let steps = row["steps"].as_array().unwrap();
    let check = |i: usize, m: &Manager<String, String>, result: Value, trace: &Trace| {
        let expected = &steps[i];
        let got_trace = Value::Array(std::mem::take(&mut *trace.borrow_mut()));
        assert_eq!(without_logs(&got_trace), without_logs(&expected["trace"]), "{name} {pc:?} step {i} trace");
        let got = state(m, &data, &voices, 1);
        assert_eq!(got, expected["state"], "{name} {pc:?} step {i} state");
        assert_eq!(result, expected["result"], "{name} {pc:?} step {i} result");
    };
    check(0, &m, Value::Null, &trace);
    for (i, step) in case["steps"].as_array().unwrap().iter().enumerate() {
        let mut result = Value::Null;
        match step["op"].as_str().unwrap() {
            "enable" => m.enable_745510(&mut env, &mut miles).unwrap(),
            "disable" => m.disable_745530(&mut env, &mut miles).unwrap(),
            "update" => m.update_747a80(&mut env, &mut miles).unwrap(),
            "play" => {
                let fx = step.get("fx").and_then(Value::as_str).map(str::to_owned);
                let id = m.play_7474f0(&mut env, &mut miles, &data[step["data"].as_str().unwrap()],
                    step.get("loop").and_then(Value::as_bool).unwrap_or(false),
                    object_arg(case, step.get("a").unwrap_or(&Value::Null)), object_arg(case, step.get("b").unwrap_or(&Value::Null)),
                    object_handle(case, step.get("owner").unwrap_or(&Value::Null)), position(step.get("position").unwrap_or(&Value::Null)), fx).unwrap();
                result = json!(id);
            }
            "music" => result = json!(m.play_747630(&mut env, &mut miles, &data[step["data"].as_str().unwrap()]).unwrap()),
            "speech" => {
                let id = m.play_7476e0(&mut env, &mut miles, &data[step["data"].as_str().unwrap()],
                    object_arg(case, step.get("a").unwrap_or(&Value::Null)), object_arg(case, step.get("b").unwrap_or(&Value::Null)),
                    object_handle(case, step.get("owner").unwrap_or(&Value::Null)), position(step.get("position").unwrap_or(&Value::Null))).unwrap();
                result = json!(id);
            }
            "eos_2d" => m.eos_2d_749510(&mut env, &mut miles, word(&step["handle"])).unwrap(),
            "eos_3d" => m.eos_3d_749550(&mut env, &mut miles, word(&step["handle"])).unwrap(),
            "eos_stream" => m.eos_stream_749590(&mut env, &mut miles, word(&step["handle"])).unwrap(),
            "set_volume" => m.set_volume_744f90(&mut env, word(&step["index"]), bits(&step["value"]), bits(&step["duration"])).unwrap(),
            "set_volumes" => m.set_volumes_7450e0(&mut env, word(&step["mask"]), bits(&step["value"]), bits(&step["duration"])).unwrap(),
            "pause" => m.queue_pause_747150(word(&step["mask"])),
            "resume" => m.queue_resume_747180(word(&step["mask"])),
            "stop" => m.queue_stop_7471b0(word(&step["mask"])),
            "hold" => m.hold_745580(),
            "unhold" => m.release_hold_7455b0(),
            "advance" => env.now += step["ms"].as_i64().unwrap(),
            "env" => {
                for (k, v) in step["set"].as_object().unwrap() {
                    env.env.insert(k.clone(), v.clone());
                }
            }
            other => panic!("{other}"),
        }
        check(i + 1, &m, result, &trace);
    }
}

#[test]
fn native_audio_manager_fixture_replays() {
    let path = std::env::var("AA_AUDIO_FIXTURE")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-audio/native-audio-manager.json").into());
    let text = std::fs::read_to_string(&path).expect("research/phase8-audio/native-audio-manager.json");
    let fixture: Value = serde_json::from_str(&text).unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 63);
    for row in rows {
        replay(row);
    }
}
