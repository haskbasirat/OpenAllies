//! The application audio manager [B36548]: TGC_AudioManager (vtable A8B1B0,
//! base constructor 746470) and its only concrete class TGC_MilesAudioManager
//! (vtable A8B3A8, constructor 749130, Init 74AD70), with the playing-audio
//! objects TGC_Audio (A8AFE0, 7457D0) / TGC_MilesAudio (A8B2D0, 7482A0).
//!
//! Native object fields map to Rust fields named after their offsets. The
//! Miles Sound System calls (mss32.dll `_AIL_*` imports) go through the
//! [`Miles`] platform trait with the same arguments, in the same order. All
//! other world/application reads go through [`Env`] at their native points.
//!
//! Threading: natively every public entry takes the manager's critical
//! sections (+4 lists, +48 command queue, +64 id counter, +80 hold counter,
//! Miles +3D4 pools and +3F0 deferred streams). The Rust owner is one value
//! behind the caller's lock, so those sections are implicit.
//!
//! Removed native caps: none of the lists or pools has a fixed size here; the
//! channel counts come from the UVars Miles2D/3D/StreamingChannels (defaults
//! 8/21/3) exactly as natively, because they are user audio settings, not
//! engine limits.
use aa_numeric::{Binary, Precision};
use std::{collections::{BTreeMap, VecDeque}, sync::{Arc, Mutex}};

pub type Result<T> = std::result::Result<T, String>;

#[path = "audio_manager_volume.rs"]
pub mod volume;
#[path = "audio_manager_instance.rs"]
mod instance;
#[path = "audio_manager_miles.rs"]
mod miles;
pub use miles::{Channel, Miles, WavInfo, PROVIDER_FAST_2D};
use volume::{e, f, fild_unsigned, greater, same, st, MILLI, ONE, THOUSAND, ZERO};

/// QueryPerformanceCounter / QueryPerformanceFrequency (59AA70/59AB60/59ABA0).
pub trait Clock {
    fn qpc(&mut self) -> Result<i64>;
    fn qpc_frequency(&mut self) -> Result<i64>;
}
/// An 8-byte native timer object: 59AB60 stores the counter, 59ABA0 returns
/// `(now - anchor) * 1000 / frequency` (64-bit __allmul/__alldiv), truncated
/// to its 32-bit EAX result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Timer {
    pub anchor: i64,
}
impl Timer {
    pub fn start(clock: &mut impl Clock) -> Result<Self> {
        Ok(Self { anchor: clock.qpc()? })
    }
    pub fn elapsed_59aba0(&self, clock: &mut impl Clock) -> Result<u32> {
        let now = clock.qpc()?;
        let frequency = clock.qpc_frequency()?;
        if frequency == 0 {
            return Err("59ABA0 performance frequency is zero".into());
        }
        Ok((now.wrapping_sub(self.anchor).wrapping_mul(1000) / frequency) as u32)
    }
}

/// KKC_AudioData fields the manager reads (loader 46C3A0). Floats are binary32
/// bits. `R` identifies one shared KKC_AudioResource allocation.
#[derive(Debug)]
pub struct AudioData<R> {
    /// +8 IDS.
    pub ids: String,
    /// +18: 0 SOUND, 1 MUSIC, 2 SPEECH (category registry 46BD40); -1 when
    /// the category lookup failed.
    pub category: i32,
    /// +1C priority (LOWEST..CRITICAL); the manager never reads it.
    pub priority: i32,
    /// +20 control_flags: 1 INTERRUPT, 2 SEQUENTIAL, 4 ALL, 8 VOICE, 0x10 WORLD,
    /// 0x20 CAUSEFADE, 0x40 TESTSOUND.
    pub control: u8,
    /// +21 world_flags: 1 GLOBAL, 2 FOGGED.
    pub world: u8,
    /// +22 who_flags: 1 LOCAL, 2 PLAYER, 4 ALLIES, 8 NEUTRALS, 0x10 ENEMIES,
    /// 0x20 EVERYONE.
    pub who: u8,
    /// +24 volume.
    pub volume: u32,
    /// +28 range[0] (Miles min distance), +2C range[1] (max, audible range).
    pub range: [u32; 2],
    /// +30 simultaneous_limit.
    pub simultaneous_limit: i32,
    /// +34/+38 leadin, +40/+44 files, +4C/+50 fallout.
    pub leadin: Vec<R>,
    pub files: Vec<R>,
    pub fallout: Vec<R>,
    /// +58/+5C fade_time_s (in, out).
    pub fade_time_s: [u32; 2],
    /// +60/+64 delay_ms.
    pub delay_ms: [u32; 2],
    /// +68/+6C pitch_adjust.
    pub pitch_adjust: [u32; 2],
    /// +70 volume_adjust.
    pub volume_adjust: u32,
    /// +78 ldf_length.
    pub ldf_length: u32,
    /// +7C the subtitle duration 74A270 and 7477F0 test against 0.
    pub subtitle_7c: u32,
    /// +74 the last SEQUENTIAL pick (46B110). Mutable shared definition state.
    sequential_74: Mutex<Option<R>>,
}
pub type DataRef<R> = Arc<AudioData<R>>;
impl<R> AudioData<R> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(ids: String, category: i32, priority: i32, flags: [u8; 3], volume: u32, range: [u32; 2], simultaneous_limit: i32,
        lists: [Vec<R>; 3], fade_time_s: [u32; 2], delay_ms: [u32; 2], pitch_adjust: [u32; 2], volume_adjust: u32,
        ldf_length: u32, subtitle_7c: u32) -> Self {
        let [leadin, files, fallout] = lists;
        Self { ids, category, priority, control: flags[0], world: flags[1], who: flags[2], volume, range, simultaneous_limit,
            leadin, files, fallout, fade_time_s, delay_ms, pitch_adjust, volume_adjust, ldf_length, subtitle_7c,
            sequential_74: Mutex::new(None) }
    }
    pub fn sequential_cursor(&self) -> Option<R> where R: Clone {
        self.sequential_74.lock().ok().and_then(|c| c.clone())
    }
}
impl<R: Clone + PartialEq> AudioData<R> {
    /// 46B0D0 leadin: one entry without a draw; otherwise one UI+D4 draw
    /// modulo (count - 1), so the last leadin is never chosen.
    fn leadin_46b0d0(&self, draw: &mut impl FnMut() -> Result<u32>) -> Result<Option<R>> {
        pick_minus_one(&self.leadin, draw)
    }
    /// 46B190 fallout: the same (count - 1) modulo.
    fn fallout_46b190(&self, draw: &mut impl FnMut() -> Result<u32>) -> Result<Option<R>> {
        pick_minus_one(&self.fallout, draw)
    }
    /// 46B110 files: SEQUENTIAL advances from the first index of the last
    /// pick (46B030, -1 when absent); otherwise a draw modulo count.
    fn main_46b110(&self, draw: &mut impl FnMut() -> Result<u32>) -> Result<Option<R>> {
        let n = self.files.len();
        match n {
            0 => return Ok(None),
            1 => return Ok(Some(self.files[0].clone())),
            _ => {}
        }
        if self.control & 2 != 0 {
            let mut cursor = self.sequential_74.lock().map_err(|_| "AudioData+74 cursor poisoned")?;
            let pick = match cursor.as_ref() {
                None => self.files[0].clone(),
                Some(last) => {
                    let index = self.files.iter().position(|r| r == last).map_or(-1i64, |i| i as i64);
                    self.files[((index + 1) % n as i64) as usize].clone()
                }
            };
            *cursor = Some(pick.clone());
            return Ok(Some(pick));
        }
        let value = draw()?;
        Ok(Some(self.files[(value % n as u32) as usize].clone()))
    }
}
fn pick_minus_one<R: Clone>(list: &[R], draw: &mut impl FnMut() -> Result<u32>) -> Result<Option<R>> {
    match list.len() {
        0 => Ok(None),
        1 => Ok(Some(list[0].clone())),
        n => {
            let value = draw()?;
            Ok(Some(list[(value % (n as u32 - 1)) as usize].clone()))
        }
    }
}
/// 46B2E0 / 46B340: equal bounds return the first; otherwise
/// `(b - a) * (draw * 2^-32) + a` at the current precision (ST0).
fn random_range(bounds: [u32; 2], draw: &mut impl FnMut() -> Result<u32>, pc: Precision) -> Result<Binary> {
    if same(bounds[0], bounds[1]) {
        return f(bounds[0]);
    }
    let r = unit(draw()?, pc)?;
    let span = e(f(bounds[1])?.sub(&f(bounds[0])?, pc))?;
    e(e(r.mul(&span, pc))?.add(&f(bounds[0])?, pc))
}
/// fild + qword 2^32 when negative, then fmul qword [9FDA98] (2^-32).
fn unit(value: u32, pc: Precision) -> Result<Binary> {
    let mut v = Binary::from_i32(value as i32);
    if (value as i32) < 0 {
        v = e(v.add(&e(Binary::from_f64_bits(0x41f0_0000_0000_0000))?, pc))?;
    }
    e(v.mul(&e(Binary::from_f64_bits(0x3df0_0000_0000_0000))?, pc))
}

/// Native `+1C` position modes of TGC_Audio.
pub const MODE_OBJECT: i32 = 0;
pub const MODE_SECONDARY_OBJECT: i32 = 1;
pub const MODE_GLOBAL: i32 = 2;
pub const MODE_LOCATION: i32 = 3;
pub const MODE_NONE: i32 = 4;

/// One TGC_MilesAudio (0x74 bytes).
#[derive(Debug)]
pub struct Instance<R, X> {
    /// +4.
    pub data: DataRef<R>,
    /// +C (745550 counter, first value 2).
    pub id: u32,
    /// +10 id of the audio this one replaces (interrupt target).
    pub interrupts_10: u32,
    /// +14 id of the audio that claimed this one.
    pub claimed_by_14: u32,
    /// +18 Miles channel (index into the manager's channel arena).
    pub channel_18: Option<usize>,
    /// +1C position mode, +20 object handle, +24 owner object handle.
    pub mode_1c: i32,
    pub object_20: u32,
    pub owner_24: u32,
    /// +28..+30.
    pub position_28: [u32; 3],
    /// +34 0 Start, 1 Leadin, 2 Main, 3 Fallout, 4 Complete.
    pub state_34: i32,
    /// +38 loop (745840 clears it), +39 released.
    pub loop_38: bool,
    pub released_39: bool,
    /// +3C current part, +40 leadin, +44 fallout, +48 main (null: draw each time).
    pub current_3c: Option<R>,
    pub leadin_40: Option<R>,
    pub fallout_44: Option<R>,
    pub main_48: Option<R>,
    /// +4C B396E0 milliseconds at 747390.
    pub start_4c: u32,
    /// +54 FXC PlaySFX node and +58 its 0x6C-byte FX context copy (745CE0).
    pub fx_54: Option<X>,
    /// +5C voice owner while this VOICE audio holds it (+5BC byte).
    pub voice_5c: Option<u32>,
    /// +60 fade timer (8-byte object), +64 0 fading in, 1 playing, 2 fading
    /// out, 3 faded out.
    pub fade_timer_60: Option<Timer>,
    pub fade_64: i32,
    /// +68 delay (ms), +6C pitch, +70 volume.
    pub delay_68: u32,
    pub pitch_6c: u32,
    pub volume_70: u32,
}

/// One 0x14-byte command (746E70 node, next at +14).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// 0 start, 1 pause, 2 resume, 3 stop/fade, 4 release, 5 subtitle release.
    pub kind: u32,
    pub audio: Option<u32>,
    pub id: u32,
    /// B396E0 seconds (binary32), 0.0 for 747630.
    pub time: u32,
    /// Category mask for kinds 1..3 (bit = 1 << category).
    pub mask: u32,
}

/// The camera fields 745FA0/7485B0 read through [B40048].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Camera {
    /// +208.
    pub zoom: u32,
    /// +240..+248 (eye) and +24C..+254 (look-at).
    pub eye: [u32; 3],
    pub center: [u32; 3],
}
/// A game object argument of 7474F0/7476E0: its +14 handle and the +14
/// handle of its vt+108 object (null = 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectArg {
    pub handle: u32,
    pub owner_108: u32,
}

/// Application/world reads at their native points.
pub trait Env<R, X>: Clock {
    /// 59ABA0 on the application timer B396E0 (= B39690+50, which 40B000
    /// also reads).
    fn b396e0_ms(&mut self) -> Result<u32>;
    /// 85D840 on [B3653C]+D4.
    fn ui_random_d4(&mut self) -> Result<u32>;
    /// [B3653C] != 0 (747390 returns early without a UI).
    fn ui_present(&mut self) -> Result<bool>;
    /// [B36534] != 0 and 43AAF0([B36538]).
    fn vworld_ready_43aaf0(&mut self) -> Result<bool>;
    /// 76AEE0([B36554]).
    fn game_running_76aee0(&mut self) -> Result<bool>;
    /// 5F41F0([B3E0A8], 0).
    fn cinematic_5f41f0(&mut self) -> Result<bool>;
    /// None when [B36554] is null, else ([B36554]+20, [[B39AB0]+114]).
    fn speech_gate(&mut self) -> Result<Option<(u32, u32)>>;
    /// [B3974C] != 0 (subtitle owner).
    fn subtitle_owner(&mut self) -> Result<bool>;
    /// [B396C0].
    fn application_kind(&mut self) -> Result<u32>;
    /// [B396C4] bit 1.
    fn application_active(&mut self) -> Result<bool>;
    /// 40EAA0(B39690).
    fn dev_mode_40eaa0(&mut self) -> Result<bool>;
    /// 4B2C30("AudioDebugIDS", 4) text; None when absent or empty.
    fn audio_debug_ids(&mut self) -> Result<Option<String>>;
    /// GetCurrentThreadId() == [B364D4]+48.
    fn on_view_thread(&mut self) -> Result<bool>;
    /// B364C0 handle -> object +20..+28; None when stale.
    fn object_position(&mut self, handle: u32) -> Result<Option<[u32; 3]>>;
    /// B364C0 handle -> object +4 (voice owner identity); None when stale.
    fn object_voice_owner(&mut self, handle: u32) -> Result<Option<u32>>;
    /// B364C0 handle -> None when stale; else Some(relation) where relation
    /// is 2 without a 5C0E50 local kingdom, else object vt+144(local).
    fn owner_relation(&mut self, handle: u32) -> Result<Option<u32>>;
    /// 748790 FOGGED test: 5543F0([B36534]+30, xy) tile; false when the tile
    /// exists, 40CFA0 is false and 54F3E0(tile, local) is false.
    fn fog_audible(&mut self, xy: [u32; 2]) -> Result<bool>;
    /// [B4004C] and [B40048] current camera.
    fn camera(&mut self) -> Result<Option<Camera>>;
    /// UVar record value (4B2E60 on B39938) or AVar (B39988/B3999C).
    fn uvar_i32(&mut self, name: &str) -> Result<i32>;
    fn uvar_f32(&mut self, name: &str) -> Result<u32>;
    fn avar_f32(&mut self, name: &str) -> Result<u32>;
    /// [B39940] user volume by registration index 0x8F..0x93.
    fn user_volume(&mut self, index: u32) -> Result<u32>;
    /// Resource vt+C(delta, mode), vt+14(delta), vt+18 (5AAE10 sync load).
    fn resource_request(&mut self, resource: &R, delta: i32, mode: u32) -> Result<()>;
    fn resource_release(&mut self, resource: &R, delta: i32) -> Result<()>;
    fn resource_sync_load(&mut self, resource: &R) -> Result<()>;
    /// 59B030 GetRawData (+3C Miles memory), None when null.
    fn raw_data_59b030(&mut self, resource: &R) -> Result<Option<u32>>;
    /// 428AF0(node, context) for a removed audio with +54 (on the view thread).
    fn fx_complete_428af0(&mut self, link: &X) -> Result<()>;
    /// [B3656C] vt+4(1) after a user music volume change.
    fn music_user_changed(&mut self) -> Result<()>;
}

/// What the manager did, in order, for logs and tests (no native effect).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Queued { id: u32, ids: String },
    Rejected { id: u32, ids: String },
    Started { id: u32, ids: String, kind: u32, channel: i32 },
    Part { id: u32, state: i32 },
    Dropped { id: u32, ids: String },
    Removed { id: u32, ids: String },
}

pub struct Manager<R, X> {
    pub pc: Precision,
    /// +20/+28/+30: 2-D, 3-D, streaming lists of instance ids (45F240 order).
    pub lists: [Vec<u32>; 3],
    /// +38 active music streams, +3C active speech streams.
    pub music_38: i32,
    pub speech_3c: i32,
    /// +40/+44 command queue.
    pub queue: VecDeque<Command>,
    /// +9C next id.
    pub next_id_9c: u32,
    pub volumes: volume::Volumes,
    /// +168 fade recovery timer.
    pub recover_168: Timer,
    /// +170 microphone, +17C orientation.
    pub mic_170: [u32; 3],
    pub orient_17c: [u32; 3],
    /// +188 game paused, +189 disabled, +18A fade active, +18B recovering,
    /// +18C high latency, +190 hold count.
    pub paused_188: bool,
    pub disabled_189: bool,
    pub fade_18a: bool,
    pub recover_18b: bool,
    pub latency_18c: bool,
    pub hold_190: i32,
    /// 747A80's InterlockedDecrement guard B28A40 (initial 1).
    guard_b28a40: i32,
    pub instances: BTreeMap<u32, Instance<R, X>>,
    pub miles: miles::State,
    pub events: Vec<Event>,
}

impl<R: Clone + PartialEq + std::fmt::Debug, X> Manager<R, X> {
    /// 749130 (-> 746470): lists empty, +9C = 2, five 745790(1,1,1) groups
    /// whose user volume is then [B39940+{90,91,93,92,8F}*4], +168 started,
    /// microphone (0,0,0), orientation (0,-1,0), flags clear; Miles fields
    /// +198 0, +19C/+1A0 -1, pools empty, listener 0.
    pub fn construct_749130(env: &mut impl Env<R, X>, pc: Precision) -> Result<Self> {
        let mut groups = Vec::with_capacity(5);
        for _ in 0..5 {
            groups.push(volume::Group::constructed(env)?);
        }
        let recover_168 = Timer::start(env)?;
        let mut volumes = volume::Volumes { groups: groups.try_into().map_err(|_| "five volume groups")? };
        for (i, index) in [0x90, 0x91, 0x93, 0x92, 0x8f].into_iter().enumerate() {
            volumes.groups[i].user = env.user_volume(index)?;
        }
        Ok(Self {
            pc,
            lists: [Vec::new(), Vec::new(), Vec::new()],
            music_38: 0,
            speech_3c: 0,
            queue: VecDeque::new(),
            next_id_9c: 2,
            volumes,
            recover_168,
            mic_170: [ZERO; 3],
            orient_17c: [ZERO, 0xbf80_0000, ZERO],
            paused_188: false,
            disabled_189: false,
            fade_18a: false,
            recover_18b: false,
            latency_18c: false,
            hold_190: 0,
            guard_b28a40: 1,
            instances: BTreeMap::new(),
            miles: miles::State::constructed(),
            events: Vec::new(),
        })
    }
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }
    pub fn initialized(&self) -> bool {
        self.miles.initialized_198
    }

    /// 745510 (vt+4): enable; Init (vt+48) when not yet initialized.
    pub fn enable_745510(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        self.disabled_189 = false;
        if !self.miles.initialized_198 {
            self.init_74ad70(env, platform)?;
        }
        Ok(())
    }
    /// 745530 (vt+C): disable; Shutdown (vt+4C) when initialized.
    pub fn disable_745530(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        self.disabled_189 = true;
        if self.miles.initialized_198 {
            self.shutdown_749460(env, platform)?;
        }
        Ok(())
    }
    /// 745580 / 7455B0: the loading-screen hold count (+190).
    pub fn hold_745580(&mut self) {
        self.hold_190 = self.hold_190.wrapping_add(1);
    }
    pub fn release_hold_7455b0(&mut self) {
        self.hold_190 = self.hold_190.wrapping_sub(1);
    }
    /// 745550: next instance id under +64.
    fn next_id_745550(&mut self) -> u32 {
        let id = self.next_id_9c;
        self.next_id_9c = id.wrapping_add(1);
        id
    }
    /// 744F90 / 7450E0 public volume writes.
    pub fn set_volume_744f90(&mut self, env: &mut impl Env<R, X>, index_mode: u32, value: u32, duration: u32) -> Result<()> {
        let mut notify = false;
        self.volumes.set_one_744f90(index_mode, value, duration, env, || {
            notify = true;
            Ok(())
        })?;
        if notify {
            env.music_user_changed()?;
        }
        Ok(())
    }
    pub fn set_volumes_7450e0(&mut self, env: &mut impl Env<R, X>, mask_mode: u32, value: u32, duration: u32) -> Result<()> {
        self.volumes.set_mask_7450e0(mask_mode, value, duration, env)
    }

    /// 7482A0 (vt+58): a new TGC_MilesAudio for `data`, or None when the
    /// Miles manager is not initialized.
    fn create_7482a0(&mut self, data: &DataRef<R>) -> Option<u32> {
        if !self.miles.initialized_198 {
            return None;
        }
        let id = self.next_id_745550();
        self.instances.insert(id, Instance {
            data: Arc::clone(data),
            id,
            interrupts_10: 0,
            claimed_by_14: 0,
            channel_18: None,
            mode_1c: MODE_NONE,
            object_20: 0,
            owner_24: 0,
            position_28: [ZERO; 3],
            state_34: 0,
            loop_38: false,
            released_39: false,
            current_3c: None,
            leadin_40: None,
            fallout_44: None,
            main_48: None,
            start_4c: 0,
            fx_54: None,
            voice_5c: None,
            fade_timer_60: None,
            fade_64: 0,
            delay_68: 0,
            pitch_6c: 0,
            volume_70: 0,
        });
        Some(id)
    }
    fn inst(&self, id: u32) -> Result<&Instance<R, X>> {
        self.instances.get(&id).ok_or_else(|| format!("audio instance {id} is not owned by the manager"))
    }
    fn inst_mut(&mut self, id: u32) -> Result<&mut Instance<R, X>> {
        self.instances.get_mut(&id).ok_or_else(|| format!("audio instance {id} is not owned by the manager"))
    }

    /// 7474F0 PlaySound: data, loop byte (+38), object a (mode 0), object b
    /// (mode 1), owner object handle, location, FXC node + context (+54/+58).
    /// Returns the new id, or 0 when no instance was created.
    #[allow(clippy::too_many_arguments)]
    pub fn play_7474f0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, data: &DataRef<R>, loop_38: bool,
        a: Option<ObjectArg>, b: Option<ObjectArg>, owner: u32, position: Option<[u32; 3]>, fx: Option<X>) -> Result<u32> {
        let Some(id) = self.create_7482a0(data) else { return Ok(0) };
        {
            let inst = self.inst_mut(id)?;
            inst.fx_54 = fx;
            inst.loop_38 = loop_38;
        }
        self.place(id, a, b, owner, position)?;
        self.queue_or_release(env, platform, id, true)?;
        Ok(id)
    }
    /// 7476E0 (541F00): as 7474F0 without loop/FX; a refused audio is queued
    /// as command 5 (subtitle release) instead of being released now.
    pub fn play_7476e0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, data: &DataRef<R>,
        a: Option<ObjectArg>, b: Option<ObjectArg>, owner: u32, position: Option<[u32; 3]>) -> Result<u32> {
        let Some(id) = self.create_7482a0(data) else { return Ok(0) };
        self.place(id, a, b, owner, position)?;
        let accepted = self.can_play_7471e0(env, platform, id)?;
        let time = self.now_seconds(env)?;
        self.push_746e70(Command { kind: if accepted { 0 } else { 5 }, audio: Some(id), id: 0, time, mask: 0 });
        self.events.push(if accepted { Event::Queued { id, ids: data.ids.clone() } } else { Event::Rejected { id, ids: data.ids.clone() } });
        Ok(id)
    }
    /// 747630: looping (+38 = 1), no position; queued with time 0.0 so the
    /// drop timeout never applies.
    pub fn play_747630(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, data: &DataRef<R>) -> Result<u32> {
        let Some(id) = self.create_7482a0(data) else { return Ok(0) };
        self.inst_mut(id)?.loop_38 = true;
        if self.can_play_7471e0(env, platform, id)? {
            self.push_746e70(Command { kind: 0, audio: Some(id), id: 0, time: ZERO, mask: 0 });
            self.events.push(Event::Queued { id, ids: data.ids.clone() });
        } else {
            self.events.push(Event::Rejected { id, ids: data.ids.clone() });
            self.release_now(env, platform, id)?;
        }
        Ok(id)
    }
    /// 747150 / 747180 / 7471B0: pause, resume or stop by category mask or id.
    pub fn queue_pause_747150(&mut self, mask: u32) {
        self.push_746e70(Command { kind: 1, audio: None, id: 0, time: ZERO, mask });
    }
    pub fn queue_resume_747180(&mut self, mask: u32) {
        self.push_746e70(Command { kind: 2, audio: None, id: 0, time: ZERO, mask });
    }
    pub fn queue_stop_7471b0(&mut self, mask: u32) {
        self.push_746e70(Command { kind: 3, audio: None, id: 0, time: ZERO, mask });
    }
    /// The by-id forms used by FXC StopSFX and other callers.
    pub fn queue_by_id(&mut self, kind: u32, id: u32) -> Result<()> {
        if !(1..=3).contains(&kind) {
            return Err("audio by-id command kind outside pause/resume/stop".into());
        }
        self.push_746e70(Command { kind, audio: None, id, time: ZERO, mask: 0 });
        Ok(())
    }
    /// 7474F0/7476E0 position setup: 745970(a, 0) or 745970(b, 1) or
    /// 745890(position) or 745970(null).
    fn place(&mut self, id: u32, a: Option<ObjectArg>, b: Option<ObjectArg>, owner: u32, position: Option<[u32; 3]>) -> Result<()> {
        let mut owner = owner;
        if let Some(a) = a {
            if owner == 0 {
                owner = a.owner_108;
            }
        } else if let Some(b) = b {
            if owner == 0 {
                owner = b.owner_108;
            }
        }
        let inst = self.inst_mut(id)?;
        if let Some(a) = a {
            set_object_745970(inst, Some(a.handle), MODE_OBJECT, owner);
        } else if let Some(b) = b {
            set_object_745970(inst, Some(b.handle), MODE_SECONDARY_OBJECT, owner);
        } else if let Some(position) = position {
            if inst.mode_1c > MODE_LOCATION {
                inst.mode_1c = MODE_LOCATION;
                inst.position_28 = position;
                inst.owner_24 = owner;
            }
        } else {
            set_object_745970(inst, None, MODE_OBJECT, owner);
        }
        Ok(())
    }
    fn now_seconds(&mut self, env: &mut impl Env<R, X>) -> Result<u32> {
        let ms = fild_unsigned(env.b396e0_ms()?, self.pc)?;
        st(&e(ms.mul(&f(MILLI)?, self.pc))?)
    }
    fn queue_or_release(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32, timed: bool) -> Result<()> {
        let ids = self.inst(id)?.data.ids.clone();
        if self.can_play_7471e0(env, platform, id)? {
            let time = if timed { self.now_seconds(env)? } else { ZERO };
            self.push_746e70(Command { kind: 0, audio: Some(id), id: 0, time, mask: 0 });
            self.events.push(Event::Queued { id, ids });
        } else {
            self.events.push(Event::Rejected { id, ids });
            self.release_now(env, platform, id)?;
        }
        Ok(())
    }
    /// The refused branch of 7474F0/747630 and command 4: release the voice
    /// owner (7458F0), +39 = 1, vt+5C (748330).
    fn release_now(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        self.release_voice_7458f0(id)?;
        self.inst_mut(id)?.released_39 = true;
        self.destroy_748330(env, platform, id)
    }
    /// 746E70: append at the queue tail.
    fn push_746e70(&mut self, command: Command) {
        self.queue.push_back(command);
    }

    /// 745850: the channel kind (channel +4, else 2 for MUSIC/SPEECH, 1 for a
    /// positioned WORLD sound, else 0).
    pub fn kind_745850(&self, id: u32) -> Result<u32> {
        let inst = self.inst(id)?;
        if let Some(channel) = inst.channel_18 {
            return Ok(self.miles.channels[channel].kind);
        }
        Ok(data_kind(&inst.data, inst.mode_1c))
    }

    /// 7471E0 CanPlay.
    fn can_play_7471e0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<bool> {
        if !self.audible_746730(env, platform, id)? {
            return Ok(false);
        }
        if !self.reserve_voice_746410(env, id)? {
            return Ok(false);
        }
        if env.dev_mode_40eaa0()? {
            if let Some(debug) = env.audio_debug_ids()? {
                //4097F0 __wcsicmp: a nonzero difference refuses the audio.
                if !debug.eq_ignore_ascii_case(&self.inst(id)?.data.ids) {
                    return Ok(false);
                }
            }
        }
        if env.cinematic_5f41f0()? {
            match env.speech_gate()? {
                None => return Ok(false),
                Some((current, required)) => {
                    if self.inst(id)?.data.category == 2 && !same(current, required) {
                        return Ok(false);
                    }
                }
            }
        }
        if !self.over_limit_746a80(id)? && self.reserve_channel_7480a0(id)? {
            return Ok(true);
        }
        self.interrupt_746f40(id)
    }
    /// 746730: ShouldPlayLocally and 745C20 range at the microphone.
    fn audible_746730(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<bool> {
        if !self.should_play_locally_746300(env, id)? {
            return Ok(false);
        }
        let mic = self.mic_170;
        self.in_range_745c20(env, platform, id, mic)
    }
    /// 746300 TGC_Audio::ShouldPlayLocally.
    fn should_play_locally_746300(&mut self, env: &mut impl Env<R, X>, id: u32) -> Result<bool> {
        let inst = self.inst(id)?;
        if inst.mode_1c == MODE_NONE || inst.data.who & 0x21 != 0 {
            return Ok(true);
        }
        let relation = if inst.mode_1c != MODE_LOCATION && inst.owner_24 != 0 { env.owner_relation(inst.owner_24)? } else { None };
        let Some(relation) = relation else {
            //404ED0 "Audio '%s' is being used without available kingdom
            //information ..." (Audio.cpp 0xC1), then true.
            return Ok(true);
        };
        let who = inst.data.who;
        Ok(match relation {
            0 => who >> 1 & 1 != 0,
            1 => who >> 2 & 1 != 0,
            2 => who >> 3 & 1 != 0,
            3 => who >> 4 & 1 != 0,
            _ => false,
        })
    }
    /// 745C20: non-WORLD or GLOBAL audio is always in range; otherwise the
    /// position (refreshed through vt+20 when it is (0,0,0)) must be closer
    /// than range[1].
    fn in_range_745c20(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32, at: [u32; 3]) -> Result<bool> {
        let (world, global) = {
            let d = &self.inst(id)?.data;
            (d.control & 0x10 != 0, d.world & 1 != 0)
        };
        if !world || global {
            return Ok(true);
        }
        let p = self.inst(id)?.position_28;
        if p.iter().all(|&c| same(c, ZERO)) {
            self.update_3d_7485b0(env, platform, id)?;
        }
        let pc = self.pc;
        let inst = self.inst(id)?;
        let p = inst.position_28;
        let dx = st(&e(f(at[0])?.sub(&f(p[0])?, pc))?)?;
        let dy = e(f(at[1])?.sub(&f(p[1])?, pc))?;
        let dz = e(f(at[2])?.sub(&f(p[2])?, pc))?;
        let sum = e(e(dz.mul(&dz, pc))?.add(&e(dy.mul(&dy, pc))?, pc))?;
        let sum = e(sum.add(&e(f(dx)?.mul(&f(dx)?, pc))?, pc))?;
        let distance = e(sum.sqrt(pc))?;
        Ok(distance.compare(&f(inst.data.range[1])?) == std::cmp::Ordering::Less)
    }
    /// 746410: an object-attached VOICE audio takes its owner's voice.
    fn reserve_voice_746410(&mut self, env: &mut impl Env<R, X>, id: u32) -> Result<bool> {
        let (mode, object, voice) = {
            let inst = self.inst(id)?;
            (inst.mode_1c, inst.object_20, inst.data.control & 8 != 0)
        };
        if mode != MODE_OBJECT && mode != MODE_SECONDARY_OBJECT {
            return Ok(true);
        }
        let Some(owner) = env.object_voice_owner(object)? else { return Ok(false) };
        if voice {
            self.inst_mut(id)?.voice_5c = Some(owner);
            if self.miles.voices_5bc.contains(&owner) {
                return Ok(false);
            }
            self.miles.voices_5bc.insert(owner);
        }
        Ok(true)
    }
    /// 7458F0 (inline in several callers): give the voice back.
    fn release_voice_7458f0(&mut self, id: u32) -> Result<()> {
        let inst = self.inst_mut(id)?;
        if (inst.mode_1c == MODE_OBJECT || inst.mode_1c == MODE_SECONDARY_OBJECT) && inst.data.control & 8 != 0 {
            if let Some(owner) = inst.voice_5c.take() {
                self.miles.voices_5bc.remove(&owner);
            }
        }
        Ok(())
    }
    fn list_for_kind(kind: u32) -> Option<usize> {
        (kind < 3).then_some(kind as usize)
    }
    /// 746130: counts same-data audio in `list` (into `count`) and finds the
    /// oldest unclaimed (or self-claimed) one; INTERRUPT non-music audio may
    /// take it (`commit` writes +10/+14).
    fn same_data_746130(&mut self, id: u32, list: usize, count: &mut i32, commit: bool) -> Result<bool> {
        let (data, self_id) = {
            let inst = self.inst(id)?;
            (Arc::clone(&inst.data), inst.id)
        };
        let mut candidate: Option<u32> = None;
        for other in &self.lists[list] {
            let o = self.inst(*other)?;
            if !Arc::ptr_eq(&o.data, &data) {
                continue;
            }
            *count += 1;
            if let Some(c) = candidate {
                if o.start_4c >= self.inst(c)?.start_4c {
                    continue;
                }
            }
            if o.claimed_by_14 == 0 || o.claimed_by_14 == self_id {
                candidate = Some(o.id);
            }
        }
        if data.control & 1 == 0 || data.category == 1 {
            return Ok(false);
        }
        let Some(c) = candidate else { return Ok(false) };
        if commit {
            self.inst_mut(id)?.interrupts_10 = c;
            self.inst_mut(c)?.claimed_by_14 = self_id;
        }
        Ok(true)
    }
    /// 746A80: whether the simultaneous limit (or the one-stream-per-category
    /// rule) refuses this audio. Counts the list and the queued starts.
    fn over_limit_746a80(&mut self, id: u32) -> Result<bool> {
        let kind = self.kind_745850(id)?;
        let mut count = 0;
        let list = match kind {
            0 => 0,
            1 => 1,
            2 => {
                count = if self.inst(id)?.data.category == 1 { self.music_38 } else { self.speech_3c };
                2
            }
            _ => return Ok(false),
        };
        self.same_data_746130(id, list, &mut count, false)?;
        let (ids, category, channel_kind) = {
            let inst = self.inst(id)?;
            (inst.data.ids.clone(), inst.data.category, inst.channel_18.map(|c| self.miles.channels[c].kind))
        };
        for command in &self.queue {
            if command.kind != 0 {
                continue;
            }
            let Some(other) = command.audio else { continue };
            let other = self.inst(other)?;
            //96CB83 wcscmp of the two IDS strings.
            if other.data.ids == ids {
                count += 1;
                continue;
            }
            let streaming = match channel_kind {
                Some(k) => k == 2,
                None => (1..=2).contains(&category),
            };
            if streaming && category == other.data.category {
                count += 1;
            }
        }
        let streaming = match channel_kind {
            Some(k) => k == 2,
            None => (1..=2).contains(&category),
        };
        if streaming && count > 0 {
            return Ok(true);
        }
        let inst = self.inst_mut(id)?;
        inst.interrupts_10 = 0;
        Ok(count >= inst.data.simultaneous_limit)
    }
    /// 746F40: interrupt an older same-data audio (746130 commit) or another
    /// INTERRUPT audio of the same category (746C00).
    fn interrupt_746f40(&mut self, id: u32) -> Result<bool> {
        let Some(list) = Self::list_for_kind(self.kind_745850(id)?) else { return Ok(false) };
        let mut count = 0;
        if self.same_data_746130(id, list, &mut count, true)? {
            return Ok(true);
        }
        self.interrupt_other_746c00(id, list, count, true)
    }
    /// 746C00.
    fn interrupt_other_746c00(&mut self, id: u32, list: usize, count: i32, commit: bool) -> Result<bool> {
        let (data, self_id) = {
            let inst = self.inst(id)?;
            (Arc::clone(&inst.data), inst.id)
        };
        if count >= data.simultaneous_limit {
            return Ok(false);
        }
        let eligible = |o: &Instance<R, X>| !Arc::ptr_eq(&o.data, &data) && o.data.control & 1 != 0 && o.data.category == data.category;
        let mut candidate: Option<u32> = None;
        for other in &self.lists[list] {
            let o = self.inst(*other)?;
            if !eligible(o) {
                continue;
            }
            match candidate {
                None => {
                    if o.claimed_by_14 == 0 || o.claimed_by_14 == self_id {
                        candidate = Some(o.id);
                    }
                }
                Some(c) => {
                    if o.start_4c < self.inst(c)?.start_4c {
                        candidate = Some(o.id);
                    }
                }
            }
        }
        if candidate.is_none() {
            let mut queued: Option<(usize, u32)> = None;
            for (i, command) in self.queue.iter().enumerate() {
                if command.kind != 0 {
                    continue;
                }
                let Some(other) = command.audio else { continue };
                let o = self.inst(other)?;
                if !eligible(o) {
                    continue;
                }
                match queued {
                    None => {
                        if o.claimed_by_14 == 0 || o.claimed_by_14 == self_id {
                            queued = Some((i, o.id));
                        }
                    }
                    Some((_, c)) => {
                        if o.start_4c < self.inst(c)?.start_4c {
                            queued = Some((i, o.id));
                        }
                    }
                }
            }
            let Some((node, other)) = queued else { return Ok(false) };
            let target = self.inst(other)?.interrupts_10;
            candidate = Some(if target != 0 {
                self.queue[node].kind = 4;
                match self.find_7461c0(target) {
                    Some(found) => found,
                    None => return Ok(false),
                }
            } else {
                other
            });
        }
        let c = candidate.expect("candidate set above");
        self.inst_mut(id)?.interrupts_10 = c;
        if commit {
            self.inst_mut(c)?.claimed_by_14 = self_id;
        }
        Ok(true)
    }
    /// 7461C0: an audio by id in the 3-D, then 2-D, then streaming list.
    fn find_7461c0(&self, id: u32) -> Option<u32> {
        [1usize, 0, 2].into_iter().find_map(|l| self.lists[l].iter().copied().find(|&i| i == id))
    }

    /// 747390 Start: channel, start time, counts, vt+8 Play, pause when the
    /// game is paused, CAUSEFADE, append to its list.
    fn start_747390(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        if !env.ui_present()? {
            return Ok(());
        }
        let channel = self.acquire_749340(env, platform, id)?;
        let start = env.b396e0_ms()?;
        let category = {
            let inst = self.inst_mut(id)?;
            inst.channel_18 = channel;
            inst.start_4c = start;
            inst.data.category
        };
        if category == 1 {
            self.music_38 += 1;
        } else if category == 2 {
            self.speech_3c += 1;
        }
        let kind = self.kind_745850(id)?;
        {
            let ids = self.inst(id)?.data.ids.clone();
            self.events.push(Event::Started { id, ids, kind, channel: channel.map_or(-1, |c| self.miles.channels[c].index) });
        }
        self.play_74ad10(env, platform, id)?;
        if self.paused_188 && pausable_start(self.inst(id)?) {
            self.pause_7484c0(platform, id)?;
        }
        let (cause, data) = {
            let inst = self.inst(id)?;
            (inst.data.control & 0x20 != 0, Arc::clone(&inst.data))
        };
        if cause {
            let kind = self.kind_745850(id)?;
            self.cause_fade_747070(env, kind, &data)?;
        }
        let kind = self.kind_745850(id)?;
        if let Some(list) = Self::list_for_kind(kind) {
            self.lists[list].push(id);
        }
        Ok(())
    }
    /// 747070: fade the other groups to AudioFadeVolume over AudioFadeTime.
    fn cause_fade_747070(&mut self, env: &mut impl Env<R, X>, kind: u32, data: &AudioData<R>) -> Result<()> {
        let mask = match kind {
            0 => 0xe,
            1 => 0xd,
            2 => if data.category == 1 { 0xb } else { 7 },
            _ => 0,
        };
        let time = env.avar_f32("AudioFadeTime")?;
        let value = env.avar_f32("AudioFadeVolume")?;
        let duration = st(&e(f(time)?.mul(&f(THOUSAND)?, self.pc))?)?;
        self.volumes.set_mask_7450e0(mask | 0x80, value, duration, env)?;
        self.fade_18a = true;
        Ok(())
    }

    /// 7477F0: one dequeued command.
    fn process_7477f0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, command: Command) -> Result<()> {
        match command.kind {
            0 => {
                let id = command.audio.ok_or("7477F0 start command without audio")?;
                if !same(command.time, ZERO) {
                    let drop = env.avar_f32("AudioDropTimeout")?;
                    //40B000(B39690): 59ABA0(B39690+50) * 0.001f in ST0.
                    let now = e(fild_unsigned(env.b396e0_ms()?, self.pc)?.mul(&f(MILLI)?, self.pc))?;
                    let age = e(now.sub(&f(command.time)?, self.pc))?;
                    if greater(&age, &f(drop)?) {
                        let ids = self.inst(id)?.data.ids.clone();
                        self.events.push(Event::Dropped { id, ids });
                        return self.release_now(env, platform, id);
                    }
                }
                self.start_747390(env, platform, id)
            }
            1 | 2 => {
                if command.mask != 0 {
                    for id in self.all_ids() {
                        if command.mask & (1u32 << (self.inst(id)?.data.category as u32 & 31)) != 0 {
                            if command.kind == 1 { self.pause_7484c0(platform, id)? } else { self.resume_748500(platform, id)? }
                        }
                    }
                } else if let Some(id) = self.find_7461c0(command.id) {
                    if command.kind == 1 { self.pause_7484c0(platform, id)? } else { self.resume_748500(platform, id)? }
                }
                Ok(())
            }
            3 => {
                if command.mask != 0 {
                    for id in self.all_ids() {
                        if command.mask & (1u32 << (self.inst(id)?.data.category as u32 & 31)) != 0 {
                            self.stop_748540(platform, id)?;
                        }
                    }
                } else if let Some(id) = self.find_7461c0(command.id) {
                    self.stop_or_fade_745670(platform, id)?;
                }
                Ok(())
            }
            4 => {
                let id = command.audio.ok_or("7477F0 release command without audio")?;
                self.release_now(env, platform, id)
            }
            5 => {
                let id = command.audio.ok_or("7477F0 subtitle release without audio")?;
                let ldf = self.inst(id)?.data.ldf_length;
                if greater(&f(ldf)?, &f(ZERO)?) && env.subtitle_owner()? {
                    return Err("7477F0 case 5 6371D0 subtitle display (B3974C+188) is not ported".into());
                }
                self.release_now(env, platform, id)
            }
            _ => Ok(()),
        }
    }
    /// 746790 order: 2-D, 3-D, streaming.
    fn all_ids(&self) -> Vec<u32> {
        self.lists.iter().flatten().copied().collect()
    }
    /// 745670: fade out when the data has a fade-out time and audio is
    /// enabled, else stop.
    fn stop_or_fade_745670(&mut self, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let out = self.inst(id)?.data.fade_time_s[1];
        if greater(&f(out)?, &f(ZERO)?) && !self.disabled_189 {
            self.begin_fade_out_748060(id)
        } else {
            self.stop_748540(platform, id)
        }
    }

    /// 747A80 Update (vt+8): run once per UI frame (5C92D0/5C92F0) or from
    /// the inactive-window loop 40D020.
    pub fn update_747a80(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        self.guard_b28a40 -= 1;
        if self.guard_b28a40 != 0 {
            self.guard_b28a40 += 1;
            return Ok(());
        }
        let result = self.update_body(env, platform);
        self.guard_b28a40 += 1;
        result
    }
    fn update_body(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        let held = self.hold_190 > 0;
        if held != self.latency_18c {
            self.latency_7481d0(env, platform, held)?;
            self.latency_18c = held;
        }
        if env.vworld_ready_43aaf0()? {
            let paused = !env.game_running_76aee0()?;
            if self.paused_188 != paused {
                for id in self.all_ids() {
                    self.pause_resume_745a30(platform, id, paused)?;
                }
                self.paused_188 = paused;
            }
            if env.cinematic_5f41f0()? {
                //747B7D reads [B36554]+20 without a null test.
                let (current, required) = env.speech_gate()?.ok_or("747A80 reads [B36554]+20 while [B36554] is null")?;
                {
                    if !same(current, required) {
                        //745F70(+30, 7456B0): stop streaming SPEECH.
                        for id in self.lists[2].clone() {
                            if self.inst(id)?.data.category == 2 {
                                self.stop_748540(platform, id)?;
                            }
                        }
                        if env.subtitle_owner()? {
                            return Err("747A80 636D20 subtitle hide (B3974C+188) is not ported".into());
                        }
                    }
                }
            }
        } else {
            self.paused_188 = true;
        }
        while let Some(command) = self.queue.pop_front() {
            self.process_7477f0(env, platform, command)?;
        }
        self.volumes.step_7455e0(env, self.pc)?;
        for list in 0..3 {
            for id in self.lists[list].clone() {
                if self.instances.contains_key(&id) {
                    self.update_volume_748790(env, platform, id)?;
                }
            }
        }
        for g in &mut self.volumes.groups {
            g.changed = false;
        }
        self.microphone_745fa0(env, platform)?;
        for list in 0..3 {
            self.sweep_746830(env, platform, list)?;
        }
        if self.fade_18a {
            let causing = self.all_ids().into_iter().filter(|id| self.instances.get(id).is_some_and(|i| i.data.control & 0x20 != 0)).count();
            if causing == 0 {
                if !self.recover_18b {
                    self.recover_18b = true;
                    self.recover_168 = Timer::start(env)?;
                }
            } else if self.recover_18b {
                self.recover_18b = false;
            }
            let wait = env.avar_f32("AudioRecoverFromFadeTime")?;
            if self.recover_18b {
                let elapsed = st(&fild_unsigned(self.recover_168.elapsed_59aba0(env)?, self.pc)?)?;
                let seconds = e(f(elapsed)?.div(&f(THOUSAND)?, self.pc))?;
                if greater(&seconds, &f(wait)?) {
                    self.fade_18a = false;
                    self.recover_18b = false;
                    let time = env.avar_f32("AudioFadeTime")?;
                    let duration = st(&e(f(time)?.mul(&f(THOUSAND)?, self.pc))?)?;
                    self.volumes.set_mask_7450e0(0xf | 0x80, ONE, duration, env)?;
                }
            }
        }
        if env.camera()?.is_some() {
            for id in self.lists[1].clone() {
                if self.instances.contains_key(&id) {
                    self.update_3d_7485b0(env, platform, id)?;
                }
            }
        }
        self.deferred_streams_748ce0(platform)
    }
    /// 745A30 callback of 746790: pause (paused) or resume pausable audio.
    fn pause_resume_745a30(&mut self, platform: &mut impl Miles<R>, id: u32, paused: bool) -> Result<()> {
        let inst = self.inst(id)?;
        if !pausable(inst) {
            return Ok(());
        }
        if paused { self.pause_7484c0(platform, id) } else { self.resume_748500(platform, id) }
    }
    /// 745FA0: microphone at the camera look-at plus AudioMicrophoneHeight,
    /// facing the horizontal look direction; vt+34 (748700) listener.
    fn microphone_745fa0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        let Some(camera) = env.camera()? else { return Ok(()) };
        let height = env.uvar_f32("AudioMicrophoneHeight")?;
        let pc = self.pc;
        self.mic_170 = camera.center;
        self.mic_170[2] = st(&e(f(camera.center[2])?.add(&f(height)?, pc))?)?;
        let mut face = [ZERO; 3];
        for i in 0..3 {
            face[i] = st(&e(f(camera.center[i])?.sub(&f(camera.eye[i])?, pc))?)?;
        }
        face[2] = ZERO;
        self.orient_17c = normalize_4453a0(face, pc)?.0;
        self.listener_748700(platform)
    }
    /// 746830: stop faded-out audio, then remove every released one.
    fn sweep_746830(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, list: usize) -> Result<()> {
        for id in self.lists[list].clone() {
            if self.inst(id)?.fade_64 == 3 {
                self.stop_748540(platform, id)?;
            }
        }
        let (removed, kept): (Vec<u32>, Vec<u32>) = self.lists[list].iter().partition(|id| self.instances.get(id).is_some_and(|i| i.released_39));
        self.lists[list] = kept;
        for id in removed {
            let category = self.inst(id)?.data.category;
            if category == 1 {
                self.music_38 -= 1;
            } else if category == 2 {
                self.speech_3c -= 1;
            }
            if self.inst(id)?.fx_54.is_some() && env.on_view_thread()? {
                let link = self.inst(id)?.fx_54.as_ref().expect("checked");
                env.fx_complete_428af0(link)?;
            }
            self.release_voice_7458f0(id)?;
            self.release_resources_745b30(env, id)?;
            let ids = self.inst(id)?.data.ids.clone();
            self.events.push(Event::Removed { id, ids });
            self.destroy_748330(env, platform, id)?;
        }
        Ok(())
    }
    /// 745B30: release the leadin/fallout/main requests, or every file when
    /// the main part was not fixed.
    fn release_resources_745b30(&mut self, env: &mut impl Env<R, X>, id: u32) -> Result<()> {
        let inst = self.inst(id)?;
        let (leadin, fallout, main, data) = (inst.leadin_40.clone(), inst.fallout_44.clone(), inst.main_48.clone(), Arc::clone(&inst.data));
        if let Some(r) = &leadin {
            env.resource_release(r, 1)?;
        }
        if let Some(r) = &fallout {
            env.resource_release(r, 1)?;
        }
        if let Some(r) = &main {
            env.resource_release(r, 1)?;
            return Ok(());
        }
        for r in &data.files {
            env.resource_release(r, 1)?;
        }
        Ok(())
    }
    /// 748330 (vt+5C): release the channel and free the instance.
    fn destroy_748330(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<()> {
        let _ = env;
        let Some(inst) = self.instances.get(&id) else { return Ok(()) };
        if !inst.released_39 {
            return Ok(());
        }
        let channel = inst.channel_18;
        self.release_channel_749390(platform, channel)?;
        self.instances.remove(&id);
        Ok(())
    }
}

/// 745970: object attach (mode lowers only), or the global mode for a null
/// object; +24 is written only when the mode was applied.
fn set_object_745970<R, X>(inst: &mut Instance<R, X>, object: Option<u32>, mode: i32, owner: u32) {
    match object {
        None => {
            inst.mode_1c = MODE_GLOBAL;
            inst.object_20 = 0;
        }
        Some(handle) => {
            if mode >= inst.mode_1c {
                return;
            }
            inst.mode_1c = mode;
            inst.object_20 = handle;
        }
    }
    inst.owner_24 = owner;
}
/// 745850 without a channel.
fn data_kind<R>(data: &AudioData<R>, mode: i32) -> u32 {
    match data.category {
        0 => if data.control & 0x10 != 0 && mode != MODE_NONE { 1 } else { 0 },
        1 | 2 => 2,
        _ => 0,
    }
}
/// 747390's paused-at-start test.
fn pausable_start<R, X>(inst: &Instance<R, X>) -> bool {
    let d = &inst.data;
    d.control & 0x40 == 0
        && (d.category == 2 || (d.who & 1 == 0 && (d.control & 0x10 != 0 || (d.who & 2 == 0 && inst.mode_1c == 0))))
}
/// 745A30's test: the 747390 test or the +38 loop byte, and never MUSIC.
fn pausable<R, X>(inst: &Instance<R, X>) -> bool {
    (pausable_start(inst) || inst.loop_38) && inst.data.category != 1
}
/// 4453A0 on a float3 (result vector, length in ST0).
fn normalize_4453a0(v: [u32; 3], pc: Precision) -> Result<([u32; 3], Binary)> {
    let (x, y, z) = (f(v[0])?, f(v[1])?, f(v[2])?);
    //(x*x + y*y) + z*z, the 4453A0 instruction order.
    let sum = e(e(e(x.mul(&x, pc))?.add(&e(y.mul(&y, pc))?, pc))?.add(&e(z.mul(&z, pc))?, pc))?;
    let length = e(sum.sqrt(pc))?;
    //[9FFE7C] 0.0001f.
    if greater(&length, &f(0x38d1_b717)?) {
        let inv = e(f(ONE)?.div(&length, pc))?;
        Ok(([st(&e(inv.mul(&x, pc))?)?, st(&e(inv.mul(&y, pc))?)?, st(&e(inv.mul(&z, pc))?)?], length))
    } else {
        Ok(([ZERO; 3], f(ZERO)?))
    }
}

#[cfg(test)]
#[path = "audio_manager_tests.rs"]
mod tests;
