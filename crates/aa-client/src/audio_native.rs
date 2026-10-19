//! The application's one [B36548] audio manager: the ported
//! aa_runtime::audio_manager::Manager over the Miles platform
//! (audio_miles.rs), shared by the UI thread (menus, frame update) and the
//! simulation thread (FX PlaySFX, first-tick music, HUD voices) behind one
//! lock, as the native object is shared behind its critical sections.
//!
//! Construction follows KKC_Application::Init: 749130 then vt+4 (745510,
//! which runs Init 74AD70). The UVars it reads (user volumes, Miles*
//! channels, AudioMicrophoneHeight, AudioConsiderCenteredRadius,
//! AudioLatency*) are the source-created records of Data/UVars.tgi; the AVars
//! (AudioFadeTime, AudioFadeVolume, AudioDropTimeout, AudioRecoverFromFadeTime)
//! come from AVars.tgi (v14 overlay first). The QPC clock is the real
//! performance counter; B396E0 is anchored when this owner is created.
//!
//! Resources: 744EB0's sync load (5AAE10) and 59B030 GetRawData are served by
//! the 59B550 non-streaming load through the manager's vf1C 74A0D0
//! (AIL_file_read); 59AFE0's release (vf20, AIL_mem_free_lock) runs when the
//! request count the manager's vt+C/vt+14 calls keep reaches zero. This
//! request accounting is local to the audio owner: the resource depot's own
//! Data+20 counters are not updated from here (the depot lives on another
//! owner), and that is recorded as an open gap.
use crate::audio_miles::{Archives, MilesPlatform, PlayLog, SoundRef, SoundStore};
use aa_runtime::audio_manager::{self as am, AudioData, Camera, DataRef, Env, Manager, ObjectArg};
use aa_numeric::Precision;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type Result<T> = std::result::Result<T, String>;

/// The PlaySFX completion link (+54/+58): the FX graph node whose Chain
/// (+10) 428AF0 runs when the sound is removed on the view thread.
#[derive(Clone, Debug)]
pub struct FxLink {
    pub graph: Arc<aa_runtime::fx_graph::Graph>,
    pub node: aa_runtime::fx_graph::NodeId,
    pub context: Vec<(u32, u32)>,
}

/// The world/view reads the manager makes at its native points. The
/// simulation-thread integrator implements this over the live session;
/// [`NoWorld`] is the front end without a VWorld.
pub trait AudioWorld {
    fn ui_random_d4(&mut self) -> Result<u32>;
    fn vworld_ready_43aaf0(&mut self) -> Result<bool>;
    fn game_running_76aee0(&mut self) -> Result<bool>;
    fn cinematic_5f41f0(&mut self) -> Result<bool>;
    fn speech_gate(&mut self) -> Result<Option<(u32, u32)>>;
    fn application_kind(&mut self) -> Result<u32>;
    fn application_active(&mut self) -> Result<bool>;
    fn on_view_thread(&mut self) -> Result<bool>;
    fn object_position(&mut self, handle: u32) -> Result<Option<[u32; 3]>>;
    fn object_voice_owner(&mut self, handle: u32) -> Result<Option<u32>>;
    fn owner_relation(&mut self, handle: u32) -> Result<Option<u32>>;
    fn fog_audible(&mut self, xy: [u32; 2]) -> Result<bool>;
    fn camera(&mut self) -> Result<Option<Camera>>;
    fn fx_complete_428af0(&mut self, link: &FxLink) -> Result<()>;
}
/// The front end: no world, no camera ([B4004C] null), application active,
/// main thread. The UI+D4 draw comes from the caller's UI.
pub struct NoWorld<'a> {
    pub ui: &'a mut aa_ui::ui::Ui,
    pub active: bool,
}
impl AudioWorld for NoWorld<'_> {
    fn ui_random_d4(&mut self) -> Result<u32> { self.ui.next_view_random_word_d4() }
    fn vworld_ready_43aaf0(&mut self) -> Result<bool> { Ok(false) }
    fn game_running_76aee0(&mut self) -> Result<bool> { Ok(false) }
    fn cinematic_5f41f0(&mut self) -> Result<bool> { Ok(false) }
    fn speech_gate(&mut self) -> Result<Option<(u32, u32)>> { Ok(None) }
    fn application_kind(&mut self) -> Result<u32> { Ok(1) }
    fn application_active(&mut self) -> Result<bool> { Ok(self.active) }
    fn on_view_thread(&mut self) -> Result<bool> { Ok(true) }
    fn object_position(&mut self, _: u32) -> Result<Option<[u32; 3]>> { Ok(None) }
    fn object_voice_owner(&mut self, _: u32) -> Result<Option<u32>> { Ok(None) }
    fn owner_relation(&mut self, _: u32) -> Result<Option<u32>> { Ok(None) }
    fn fog_audible(&mut self, _: [u32; 2]) -> Result<bool> { Ok(true) }
    fn camera(&mut self) -> Result<Option<Camera>> { Ok(None) }
    fn fx_complete_428af0(&mut self, _: &FxLink) -> Result<()> {
        Err("428AF0 FX completion reached without a VWorld".into())
    }
}

/// Source-created UVars/AVars records the manager reads.
#[derive(Clone, Debug, Default)]
pub struct Vars {
    values: HashMap<String, String>,
}
impl Vars {
    fn parse(text: &str, into: &mut HashMap<String, String>) {
        for line in text.lines() {
            let line = line.split(";;").next().unwrap_or("").trim();
            let mut parts = line.splitn(2, '=');
            let (Some(left), Some(value)) = (parts.next(), parts.next()) else { continue };
            let mut words = left.split_whitespace();
            let (Some(kind), Some(name), None) = (words.next(), words.next(), words.next()) else { continue };
            if !["int", "float", "flag", "string", "fixed"].contains(&kind.to_ascii_lowercase().as_str()) {
                continue;
            }
            into.insert(name.to_ascii_lowercase(), value.trim().trim_matches('"').to_owned());
        }
    }
    /// UVars.tgi (+ UVars_Restored.tgi) and AVars.tgi (+ AVars_AA.tgi) from
    /// the v14 overlay first, then Data.rwd.
    pub fn load(install: &std::path::Path, archives: &mut Archives) -> Self {
        let mut values = HashMap::new();
        let overlay = install.join("reconstruction/native-game/runtime-expanded-v14/Data");
        for name in ["UVars.tgi", "UVars_Restored.tgi", "AVars.tgi", "AVars_AA.tgi"] {
            let text = std::fs::read(overlay.join(name)).ok().or_else(|| archives.read(name).ok());
            if let Some(bytes) = text {
                Self::parse(&String::from_utf8_lossy(&bytes), &mut values);
            }
        }
        Self { values }
    }
    fn raw(&self, name: &str) -> Result<&str> {
        self.values.get(&name.to_ascii_lowercase()).map(String::as_str).ok_or_else(|| format!("4B2E60 Requested non-existent Var: {name}"))
    }
    pub fn i32(&self, name: &str) -> Result<i32> {
        self.raw(name)?.trim().parse().map_err(|_| format!("Var {name} is not an int"))
    }
    pub fn f32_bits(&self, name: &str) -> Result<u32> {
        let raw = self.raw(name)?.trim();
        let value: f32 = match raw.strip_suffix('%') {
            Some(p) => p.trim().parse::<f32>().map_err(|_| format!("Var {name} percent"))? / 100.0,
            None => raw.parse().map_err(|_| format!("Var {name} is not a float"))?,
        };
        Ok(value.to_bits())
    }
}

struct Clock {
    frequency: i64,
}
fn qpc() -> Result<i64> {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn QueryPerformanceCounter(value: *mut i64) -> i32;
        }
        let mut v = 0i64;
        // SAFETY: QueryPerformanceCounter writes one i64 through a valid pointer.
        if unsafe { QueryPerformanceCounter(&mut v) } == 0 {
            return Err("QueryPerformanceCounter failed".into());
        }
        Ok(v)
    }
    #[cfg(not(windows))]
    {
        Err("native QPC timer requires Windows".into())
    }
}
fn qpf() -> Result<i64> {
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn QueryPerformanceFrequency(value: *mut i64) -> i32;
        }
        let mut v = 0i64;
        // SAFETY: QueryPerformanceFrequency writes one i64 through a valid pointer.
        if unsafe { QueryPerformanceFrequency(&mut v) } == 0 || v == 0 {
            return Err("QueryPerformanceFrequency failed".into());
        }
        Ok(v)
    }
    #[cfg(not(windows))]
    {
        Err("native QPC timer requires Windows".into())
    }
}

/// Resource accounting for the manager's vt+C/vt+14/vt+18 and 59B030.
#[derive(Default)]
struct Loads {
    memory: HashMap<String, u32>,
    requests: HashMap<String, i32>,
}

struct Frame<'a, W: AudioWorld + ?Sized> {
    world: &'a mut W,
    vars: &'a Vars,
    clock: &'a Clock,
    anchor: i64,
    loads: &'a mut Loads,
    store: &'a Mutex<SoundStore>,
}
impl<W: AudioWorld + ?Sized> am::Clock for Frame<'_, W> {
    fn qpc(&mut self) -> Result<i64> { qpc() }
    fn qpc_frequency(&mut self) -> Result<i64> { Ok(self.clock.frequency) }
}
impl<W: AudioWorld + ?Sized> Env<SoundRef, FxLink> for Frame<'_, W> {
    fn b396e0_ms(&mut self) -> Result<u32> {
        Ok((qpc()?.wrapping_sub(self.anchor).wrapping_mul(1000) / self.clock.frequency) as u32)
    }
    fn ui_random_d4(&mut self) -> Result<u32> { self.world.ui_random_d4() }
    fn ui_present(&mut self) -> Result<bool> { Ok(true) }
    fn vworld_ready_43aaf0(&mut self) -> Result<bool> { self.world.vworld_ready_43aaf0() }
    fn game_running_76aee0(&mut self) -> Result<bool> { self.world.game_running_76aee0() }
    fn cinematic_5f41f0(&mut self) -> Result<bool> { self.world.cinematic_5f41f0() }
    fn speech_gate(&mut self) -> Result<Option<(u32, u32)>> { self.world.speech_gate() }
    fn subtitle_owner(&mut self) -> Result<bool> { Ok(false) }
    fn application_kind(&mut self) -> Result<u32> { self.world.application_kind() }
    fn application_active(&mut self) -> Result<bool> { self.world.application_active() }
    fn dev_mode_40eaa0(&mut self) -> Result<bool> { Ok(false) }
    fn audio_debug_ids(&mut self) -> Result<Option<String>> { Ok(None) }
    fn on_view_thread(&mut self) -> Result<bool> { self.world.on_view_thread() }
    fn object_position(&mut self, h: u32) -> Result<Option<[u32; 3]>> { self.world.object_position(h) }
    fn object_voice_owner(&mut self, h: u32) -> Result<Option<u32>> { self.world.object_voice_owner(h) }
    fn owner_relation(&mut self, h: u32) -> Result<Option<u32>> { self.world.owner_relation(h) }
    fn fog_audible(&mut self, xy: [u32; 2]) -> Result<bool> { self.world.fog_audible(xy) }
    fn camera(&mut self) -> Result<Option<Camera>> { self.world.camera() }
    fn uvar_i32(&mut self, name: &str) -> Result<i32> { self.vars.i32(name) }
    fn uvar_f32(&mut self, name: &str) -> Result<u32> { self.vars.f32_bits(name) }
    fn avar_f32(&mut self, name: &str) -> Result<u32> { self.vars.f32_bits(name) }
    fn user_volume(&mut self, index: u32) -> Result<u32> {
        let name = match index {
            0x8f => "AudioMainVolume",
            0x90 => "Audio2DVolume",
            0x91 => "Audio3DVolume",
            0x92 => "AudioSpeechVolume",
            0x93 => "AudioMusicVolume",
            _ => return Err(format!("UVar index {index:#x} is not an audio volume")),
        };
        self.vars.f32_bits(name)
    }
    fn resource_request(&mut self, r: &SoundRef, delta: i32, _mode: u32) -> Result<()> {
        *self.loads.requests.entry(r.path.to_ascii_lowercase()).or_default() += delta;
        Ok(())
    }
    fn resource_release(&mut self, r: &SoundRef, delta: i32) -> Result<()> {
        let key = r.path.to_ascii_lowercase();
        let count = self.loads.requests.entry(key.clone()).or_default();
        *count -= delta;
        if *count <= 0 {
            if let Some(memory) = self.loads.memory.remove(&key) {
                //59AFE0 -> vf20 748290 AIL_mem_free_lock.
                self.store.lock().map_err(|_| "audio store poisoned")?.free(memory);
            }
        }
        Ok(())
    }
    fn resource_sync_load(&mut self, r: &SoundRef) -> Result<()> {
        let streaming = r.path.to_ascii_lowercase().ends_with(".mp3");
        if streaming {
            return Ok(());
        }
        let key = r.path.to_ascii_lowercase();
        if !self.loads.memory.contains_key(&key) {
            //59B550 non-streaming: vf1C 74A0D0 AIL_file_read into +3C.
            let loaded = self.store.lock().map_err(|_| "audio store poisoned")?.load(&r.path);
            match loaded {
                Ok(memory) => {
                    self.loads.memory.insert(key, memory);
                }
                Err(e) => log::warn!("audio: 59B550 load of {} failed ({e}); +3C stays null", r.path),
            }
        }
        Ok(())
    }
    fn raw_data_59b030(&mut self, r: &SoundRef) -> Result<Option<u32>> {
        Ok(self.loads.memory.get(&r.path.to_ascii_lowercase()).copied())
    }
    fn fx_complete_428af0(&mut self, link: &FxLink) -> Result<()> { self.world.fx_complete_428af0(link) }
    fn music_user_changed(&mut self) -> Result<()> { Ok(()) }
}

pub struct NativeAudio {
    manager: Option<Manager<SoundRef, FxLink>>,
    platform: MilesPlatform,
    store: Arc<Mutex<SoundStore>>,
    vars: Vars,
    clock: Clock,
    anchor: i64,
    loads: Loads,
    data: HashMap<String, DataRef<SoundRef>>,
    errors: Vec<String>,
    pub log: Vec<String>,
}
pub type Shared = Arc<Mutex<NativeAudio>>;

impl NativeAudio {
    /// 749130 + 745510 at application start. A missing output device keeps
    /// the manager (Miles opens a null driver) with no audible output.
    pub fn new(install: &std::path::Path, mixer: Option<rodio::mixer::Mixer>) -> Result<Self> {
        let mut archives = Archives::open(install)?;
        let vars = Vars::load(install, &mut archives);
        let store = Arc::new(Mutex::new(SoundStore::new(Some(archives))));
        let clock = Clock { frequency: qpf()? };
        let anchor = qpc()?;
        let mut audio = Self {
            manager: None,
            platform: MilesPlatform::new(mixer, Arc::clone(&store)),
            store,
            vars,
            clock,
            anchor,
            loads: Loads::default(),
            data: HashMap::new(),
            errors: Vec::new(),
            log: Vec::new(),
        };
        if let Err(e) = audio.construct() {
            audio_error(&mut audio.errors, &e);
            return Err(e);
        }
        Ok(audio)
    }
    fn construct(&mut self) -> Result<()> {
        struct Init;
        impl AudioWorld for Init {
            fn ui_random_d4(&mut self) -> Result<u32> { Err("UI+D4 draw during 749130".into()) }
            fn vworld_ready_43aaf0(&mut self) -> Result<bool> { Ok(false) }
            fn game_running_76aee0(&mut self) -> Result<bool> { Ok(false) }
            fn cinematic_5f41f0(&mut self) -> Result<bool> { Ok(false) }
            fn speech_gate(&mut self) -> Result<Option<(u32, u32)>> { Ok(None) }
            fn application_kind(&mut self) -> Result<u32> { Ok(1) }
            fn application_active(&mut self) -> Result<bool> { Ok(true) }
            fn on_view_thread(&mut self) -> Result<bool> { Ok(true) }
            fn object_position(&mut self, _: u32) -> Result<Option<[u32; 3]>> { Ok(None) }
            fn object_voice_owner(&mut self, _: u32) -> Result<Option<u32>> { Ok(None) }
            fn owner_relation(&mut self, _: u32) -> Result<Option<u32>> { Ok(None) }
            fn fog_audible(&mut self, _: [u32; 2]) -> Result<bool> { Ok(true) }
            fn camera(&mut self) -> Result<Option<Camera>> { Ok(None) }
            fn fx_complete_428af0(&mut self, _: &FxLink) -> Result<()> { Ok(()) }
        }
        let mut world = Init;
        let mut frame = Frame { world: &mut world, vars: &self.vars, clock: &self.clock, anchor: self.anchor, loads: &mut self.loads, store: &self.store };
        let mut manager = Manager::construct_749130(&mut frame, Precision::Double53)?;
        //745510: disabled = 0, then Init 74AD70.
        manager.enable_745510(&mut frame, &mut self.platform)?;
        log::info!("audio: [B36548] TGC_MilesAudioManager constructed and initialized (2D/3D/stream channels {:?})", manager.miles.available);
        self.manager = Some(manager);
        Ok(())
    }
    /// Run `f` with the manager, a world, and the platform.
    fn with<R, W: AudioWorld + ?Sized>(&mut self, world: &mut W, f: impl FnOnce(&mut Manager<SoundRef, FxLink>, &mut Frame<'_, W>, &mut MilesPlatform) -> Result<R>) -> Result<R> {
        let manager = self.manager.as_mut().ok_or("the [B36548] audio manager was not constructed")?;
        let mut frame = Frame { world, vars: &self.vars, clock: &self.clock, anchor: self.anchor, loads: &mut self.loads, store: &self.store };
        f(manager, &mut frame, &mut self.platform)
    }
    /// 747A80 Update with the end-of-sample callbacks the mixer queued since
    /// the last frame (749510/749550/749590) first.
    pub fn update_747a80(&mut self, world: &mut (impl AudioWorld + ?Sized)) -> Result<()> {
        let events = self.platform.take_events();
        let result = self.with(world, |m, env, p| {
            for (kind, handle) in events {
                match kind {
                    0 => m.eos_2d_749510(env, p, handle)?,
                    1 => m.eos_3d_749550(env, p, handle)?,
                    _ => m.eos_stream_749590(env, p, handle)?,
                }
            }
            m.update_747a80(env, p)
        });
        self.drain_logs();
        result
    }
    fn drain_logs(&mut self) {
        let events = self.manager.as_mut().map(|m| m.take_events()).unwrap_or_default();
        for e in events {
            let line = format!("{e:?}");
            log::info!("audio [B36548]: {line}");
            self.log.push(line);
        }
        for PlayLog { kind, handle, file } in std::mem::take(&mut self.platform.played) {
            let line = format!("Miles {kind} handle {handle:#x} plays {file}");
            log::info!("audio [B36548]: {line}");
            self.log.push(line);
        }
    }
    /// The KKC_AudioData for `ids`, built once from its definition.
    pub fn data(&mut self, catalog: &aa_runtime::catalog::Catalog<'_>, ids: &str) -> Result<DataRef<SoundRef>> {
        if let Some(d) = self.data.get(&ids.to_ascii_lowercase()) {
            return Ok(Arc::clone(d));
        }
        let fields = aa_runtime::audio_data_fields::read(catalog, ids, Precision::Double53)?;
        let lists = fields.lists.clone().map(|l| l.into_iter().map(|path| SoundRef { path, handle: None }).collect());
        let data = Arc::new(AudioData::new(fields.ids.clone(), fields.category, fields.priority, fields.flags, fields.volume, fields.range,
            fields.simultaneous_limit, lists, fields.fade_time_s, fields.delay_ms, fields.pitch_adjust, fields.volume_adjust, fields.ldf_length, 0));
        self.data.insert(ids.to_ascii_lowercase(), Arc::clone(&data));
        Ok(data)
    }
    /// 747630 (4F96D0 first-tick music, 541F20, 546A30, 5C08E0, 600D40).
    pub fn play_747630(&mut self, world: &mut (impl AudioWorld + ?Sized), data: &DataRef<SoundRef>) -> Result<u32> {
        let r = self.with(world, |m, env, p| m.play_747630(env, p, data));
        self.drain_logs();
        r
    }
    /// 7474F0 PlaySound.
    #[allow(clippy::too_many_arguments)]
    pub fn play_7474f0(&mut self, world: &mut (impl AudioWorld + ?Sized), data: &DataRef<SoundRef>, loop_38: bool,
        a: Option<ObjectArg>, b: Option<ObjectArg>, owner: u32, position: Option<[u32; 3]>, fx: Option<FxLink>) -> Result<u32> {
        let r = self.with(world, |m, env, p| m.play_7474f0(env, p, data, loop_38, a, b, owner, position, fx));
        self.drain_logs();
        r
    }
    /// 747150/747180/7471B0 by category mask.
    pub fn queue(&mut self, kind: u32, mask: u32) -> Result<()> {
        let m = self.manager.as_mut().ok_or("audio manager absent")?;
        match kind {
            1 => m.queue_pause_747150(mask),
            2 => m.queue_resume_747180(mask),
            3 => m.queue_stop_7471b0(mask),
            _ => return Err("audio queue kind".into()),
        }
        Ok(())
    }
    /// 744F90 user volume (mode 0) for one group.
    pub fn set_user_volume(&mut self, group: u32, value: f32) -> Result<()> {
        let mut world = Silent;
        self.with(&mut world, |m, env, _| m.set_volume_744f90(env, group, value.to_bits(), 0))
    }
    pub fn take_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.errors)
    }
}
fn audio_error(errors: &mut Vec<String>, e: &str) {
    log::error!("audio [B36548]: {e}");
    errors.push(e.to_owned());
}
/// For calls that reach no world read (volume writes).
struct Silent;
impl AudioWorld for Silent {
    fn ui_random_d4(&mut self) -> Result<u32> { Err("no UI+D4 owner for this audio call".into()) }
    fn vworld_ready_43aaf0(&mut self) -> Result<bool> { Ok(false) }
    fn game_running_76aee0(&mut self) -> Result<bool> { Ok(false) }
    fn cinematic_5f41f0(&mut self) -> Result<bool> { Ok(false) }
    fn speech_gate(&mut self) -> Result<Option<(u32, u32)>> { Ok(None) }
    fn application_kind(&mut self) -> Result<u32> { Ok(1) }
    fn application_active(&mut self) -> Result<bool> { Ok(true) }
    fn on_view_thread(&mut self) -> Result<bool> { Ok(true) }
    fn object_position(&mut self, _: u32) -> Result<Option<[u32; 3]>> { Ok(None) }
    fn object_voice_owner(&mut self, _: u32) -> Result<Option<u32>> { Ok(None) }
    fn owner_relation(&mut self, _: u32) -> Result<Option<u32>> { Ok(None) }
    fn fog_audible(&mut self, _: [u32; 2]) -> Result<bool> { Ok(true) }
    fn camera(&mut self) -> Result<Option<Camera>> { Ok(None) }
    fn fx_complete_428af0(&mut self, _: &FxLink) -> Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vars_read_source_created_records_and_percent() {
        let mut v = HashMap::new();
        Vars::parse("\t\tint\t\tMiles2DChannels = 8\t\t\t;; how many\n float AudioMusicVolume = 70%\n string AudioMusicSilent = music_silent\n", &mut v);
        let vars = Vars { values: v };
        assert_eq!(vars.i32("Miles2DChannels").unwrap(), 8);
        assert_eq!(vars.f32_bits("AudioMusicVolume").unwrap(), 0.7f32.to_bits());
        assert!(vars.i32("Missing").is_err());
    }
}
