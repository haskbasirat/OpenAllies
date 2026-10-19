//! TGC_MilesAudioManager: Miles driver/provider setup (74AD70, 748360,
//! 749F90), the 2-D/3-D/streaming channel pools (7497B0/749910/749A80,
//! 748F20/748FC0/749070), reservation (7480A0), acquisition (749340 ->
//! 748D60/748DD0/748E40) and release (749390), listener (748700), latency
//! (7481D0), deferred stream closes (748C50/748CE0) and the resource-load
//! services the AudioResource calls (vf10/vf14/vf18/vf1C/vf20/vf30).
use super::*;
use std::collections::BTreeSet;

/// "Miles Fast 2D Positional Audio" (A8B350), compared over 31 bytes.
pub const PROVIDER_FAST_2D: &str = "Miles Fast 2D Positional Audio";

/// One 0x1C-byte Miles channel (vtable A8B2CC).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    /// +4 0 2-D, 1 3-D, 2 streaming.
    pub kind: u32,
    /// +8 allocation index.
    pub index: i32,
    /// +C HSAMPLE, +10 H3DSAMPLE, +14 HSTREAM, +18 HDIGDRIVER (streams).
    pub sample_0c: u32,
    pub sample3d_10: u32,
    pub stream_14: u32,
    pub driver_18: u32,
}

/// AIL_WAV_info's AILSOUNDINFO fields the validators read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavInfo {
    pub format: i32,
    pub rate: i32,
    pub bits: i32,
    pub channels: i32,
}

/// The mss32.dll imports TGC_MilesAudioManager calls. Handles are the
/// platform's own nonzero values (0 = failure, as in Miles). Floats are
/// binary32 bits. `R` is the KKC_AudioResource a stream is opened from
/// (74A130 "%x.%s" + the 74A5E0 file callbacks).
pub trait Miles<R> {
    fn set_redist_directory(&mut self, directory: &str) -> Result<()>;
    fn startup(&mut self) -> Result<()>;
    fn open_digital_driver(&mut self, frequency: i32, bits: i32, channels: i32, flags: u32) -> Result<u32>;
    /// AIL_enumerate_3D_providers: the next (handle, name) or None at the end.
    fn enumerate_3d_providers(&mut self, cursor: &mut u32) -> Result<Option<(u32, String)>>;
    /// 0 (M3D_NOERR) on success.
    fn open_3d_provider(&mut self, provider: u32) -> Result<u32>;
    fn close_3d_provider(&mut self, provider: u32) -> Result<()>;
    fn set_3d_speaker_type(&mut self, provider: u32, speaker: u32) -> Result<()>;
    fn open_3d_listener(&mut self, provider: u32) -> Result<u32>;
    fn close_3d_listener(&mut self, listener: u32) -> Result<()>;
    fn set_file_callbacks(&mut self) -> Result<()>;
    fn set_preference(&mut self, number: u32, value: i32) -> Result<()>;
    fn close_digital_driver(&mut self, driver: u32) -> Result<()>;
    fn shutdown(&mut self) -> Result<()>;
    fn allocate_sample_handle(&mut self, driver: u32) -> Result<u32>;
    fn init_sample(&mut self, sample: u32) -> Result<()>;
    fn release_sample_handle(&mut self, sample: u32) -> Result<()>;
    fn set_sample_file(&mut self, sample: u32, memory: u32, block: u32) -> Result<()>;
    fn register_eos_callback(&mut self, sample: u32, enabled: bool) -> Result<()>;
    fn start_sample(&mut self, sample: u32) -> Result<()>;
    fn stop_sample(&mut self, sample: u32) -> Result<()>;
    fn resume_sample(&mut self, sample: u32) -> Result<()>;
    fn sample_status(&mut self, sample: u32) -> Result<u32>;
    fn sample_playback_rate(&mut self, sample: u32) -> Result<i32>;
    fn set_sample_playback_rate(&mut self, sample: u32, rate: i32) -> Result<()>;
    fn set_sample_volume_pan(&mut self, sample: u32, volume: u32, pan: u32) -> Result<()>;
    fn allocate_3d_sample_handle(&mut self, provider: u32) -> Result<u32>;
    fn release_3d_sample_handle(&mut self, sample: u32) -> Result<()>;
    fn set_3d_sample_file(&mut self, sample: u32, memory: u32) -> Result<()>;
    fn register_3d_eos_callback(&mut self, sample: u32, enabled: bool) -> Result<()>;
    fn start_3d_sample(&mut self, sample: u32) -> Result<()>;
    fn stop_3d_sample(&mut self, sample: u32) -> Result<()>;
    fn resume_3d_sample(&mut self, sample: u32) -> Result<()>;
    fn sample_3d_status(&mut self, sample: u32) -> Result<u32>;
    fn sample_3d_playback_rate(&mut self, sample: u32) -> Result<i32>;
    fn set_3d_sample_playback_rate(&mut self, sample: u32, rate: i32) -> Result<()>;
    fn set_3d_sample_volume(&mut self, sample: u32, volume: u32) -> Result<()>;
    /// AIL_set_3D_sample_distances(S, max_dist, min_dist).
    fn set_3d_sample_distances(&mut self, sample: u32, max: u32, min: u32) -> Result<()>;
    /// AIL_set_3D_position on a sample or the listener.
    fn set_3d_position(&mut self, object: u32, position: [u32; 3]) -> Result<()>;
    fn set_3d_orientation(&mut self, object: u32, face: [u32; 3], up: [u32; 3]) -> Result<()>;
    fn open_stream(&mut self, driver: u32, resource: &R) -> Result<u32>;
    fn close_stream(&mut self, stream: u32) -> Result<()>;
    fn start_stream(&mut self, stream: u32) -> Result<()>;
    fn pause_stream(&mut self, stream: u32, on: i32) -> Result<()>;
    fn stream_status(&mut self, stream: u32) -> Result<u32>;
    fn register_stream_callback(&mut self, stream: u32, enabled: bool) -> Result<()>;
    fn set_stream_volume_pan(&mut self, stream: u32, volume: u32, pan: u32) -> Result<()>;
    /// AIL_stream_ms_position total length.
    fn stream_length_ms(&mut self, stream: u32) -> Result<i32>;
    /// AIL_file_read of the resource bytes: Miles memory or 0.
    fn file_read(&mut self, resource: &R) -> Result<u32>;
    fn mem_free_lock(&mut self, memory: u32) -> Result<()>;
    fn wav_info(&mut self, memory: u32) -> Result<WavInfo>;
}

#[derive(Debug)]
pub struct State {
    /// +198.
    pub initialized_198: bool,
    /// +19C Fast 2D provider index, +1A0 open provider index (-1 none).
    pub fast_2d_19c: i32,
    pub open_1a0: i32,
    /// +1A8.. (name, handle) pairs; the native 64-entry array is unbounded.
    pub providers: Vec<(String, u32)>,
    /// +3A8 HDIGDRIVER, +3D0 listener.
    pub driver_3a8: u32,
    pub listener_3d0: u32,
    /// Channel arena (native heap objects) and the +3AC/+3B4/+3BC free lists.
    pub channels: Vec<Channel>,
    pub free: [VecDeque<usize>; 3],
    /// +3C4/+3C8/+3CC available counts (unwritten until the first pool
    /// allocation natively; zero here).
    pub available: [i32; 3],
    /// +40C/+410 streams to close at the next Update (head insertion).
    pub deferred: VecDeque<u32>,
    /// Voice owners whose +5BC byte is set.
    pub voices_5bc: BTreeSet<u32>,
}
impl State {
    pub(super) fn constructed() -> Self {
        Self {
            initialized_198: false,
            fast_2d_19c: -1,
            open_1a0: -1,
            providers: Vec::new(),
            driver_3a8: 0,
            listener_3d0: 0,
            channels: Vec::new(),
            free: [VecDeque::new(), VecDeque::new(), VecDeque::new()],
            available: [0; 3],
            deferred: VecDeque::new(),
            voices_5bc: BTreeSet::new(),
        }
    }
    fn provider_handle(&self, index: i32) -> Result<u32> {
        usize::try_from(index).ok().and_then(|i| self.providers.get(i)).map(|p| p.1)
            .ok_or_else(|| format!("Miles provider index {index} outside the enumerated providers"))
    }
}

impl<R: Clone + PartialEq + std::fmt::Debug, X> Manager<R, X> {
    /// 74AD70 (vt+48) Init.
    pub(super) fn init_74ad70(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        platform.set_redist_directory("mss")?;
        platform.startup()?;
        let frequency = env.uvar_i32("MilesFrequency")?;
        let bits = env.uvar_i32("MilesBits")?;
        let channels = env.uvar_i32("MilesHWChannels")?;
        self.miles.driver_3a8 = platform.open_digital_driver(frequency, bits, channels, 0)?;
        self.enumerate_748360(platform)?;
        self.open_provider_749f90(env, platform, self.miles.fast_2d_19c)?;
        if self.miles.open_1a0 != -1 {
            let provider = self.miles.provider_handle(self.miles.open_1a0)?;
            platform.set_3d_speaker_type(provider, 4)?;
        }
        self.allocate_2d_7497b0(env, platform)?;
        self.allocate_streams_749a80(env, platform)?;
        platform.set_file_callbacks()?;
        self.miles.initialized_198 = true;
        Ok(())
    }
    /// 748360: record every 3-D provider; remember the Fast 2D one.
    fn enumerate_748360(&mut self, platform: &mut impl Miles<R>) -> Result<()> {
        self.miles.providers.clear();
        let mut cursor = 0;
        while let Some((handle, name)) = platform.enumerate_3d_providers(&mut cursor)? {
            //repe cmpsb over 31 bytes: the 30 characters and the NUL.
            if name == PROVIDER_FAST_2D {
                self.miles.fast_2d_19c = self.miles.providers.len() as i32;
            }
            self.miles.providers.push((name, handle));
        }
        Ok(())
    }
    /// 749F90: open a provider (re-opening the previous one on failure),
    /// then the 3-D pool and listener.
    fn open_provider_749f90(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, index: i32) -> Result<bool> {
        if self.miles.open_1a0 == index {
            return Ok(true);
        }
        if self.miles.open_1a0 != -1 {
            self.free_3d_748fc0(platform)?;
            platform.close_3d_provider(self.miles.provider_handle(self.miles.open_1a0)?)?;
        }
        if platform.open_3d_provider(self.miles.provider_handle(index)?)? == 0 {
            self.miles.open_1a0 = index;
        } else if self.miles.open_1a0 != -1 {
            platform.open_3d_provider(self.miles.provider_handle(self.miles.open_1a0)?)?;
        }
        if self.miles.open_1a0 != -1 {
            self.allocate_3d_749910(env, platform)?;
        }
        Ok(self.miles.open_1a0 == index)
    }
    fn new_channel(&mut self, kind: u32, index: i32) -> usize {
        self.miles.channels.push(Channel { kind, index, sample_0c: 0, sample3d_10: 0, stream_14: 0, driver_18: 0 });
        self.miles.channels.len() - 1
    }
    /// 7497B0. A failed 2-D handle falls through to a 3-D allocation on the
    /// open provider (native fall-through), then logs 404F30 and stops.
    fn allocate_2d_7497b0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        if self.miles.open_1a0 == -1 {
            return Ok(());
        }
        self.free_2d_748f20(platform)?;
        let count = env.uvar_i32("Miles2DChannels")?;
        let provider = self.miles.provider_handle(self.miles.open_1a0)?;
        for i in 0..count {
            let c = self.new_channel(0, i);
            let h = platform.allocate_sample_handle(self.miles.driver_3a8)?;
            self.miles.channels[c].sample_0c = h;
            if h != 0 {
                platform.init_sample(h)?;
            } else {
                let h3 = platform.allocate_3d_sample_handle(provider)?;
                self.miles.channels[c].sample3d_10 = h3;
                if h3 == 0 {
                    //404F30 MilesAudio.cpp 0x56D "Allocation of %d 2-D channels failed (%d succeeded)".
                    break;
                }
            }
            self.miles.free[0].push_back(c);
            self.miles.available[0] += 1;
        }
        Ok(())
    }
    /// 749910: 3-D pool, then AIL_open_3D_listener.
    fn allocate_3d_749910(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        if self.miles.open_1a0 == -1 {
            return Ok(());
        }
        self.free_3d_748fc0(platform)?;
        let count = env.uvar_i32("Miles3DChannels")?;
        let provider = self.miles.provider_handle(self.miles.open_1a0)?;
        for i in 0..count {
            let c = self.new_channel(1, i);
            let h = platform.allocate_3d_sample_handle(provider)?;
            self.miles.channels[c].sample3d_10 = h;
            if h == 0 {
                //404F30 0x587 "Allocation of %d 3-D channels failed".
                break;
            }
            self.miles.free[1].push_back(c);
            self.miles.available[1] += 1;
        }
        self.miles.listener_3d0 = platform.open_3d_listener(provider)?;
        Ok(())
    }
    /// 749A80: streaming channels only record the driver.
    fn allocate_streams_749a80(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        if self.miles.open_1a0 == -1 {
            return Ok(());
        }
        self.free_streams_749070(platform)?;
        let count = env.uvar_i32("MilesStreamingChannels")?;
        for i in 0..count {
            let c = self.new_channel(2, i);
            self.miles.channels[c].driver_18 = self.miles.driver_3a8;
            self.miles.free[2].push_back(c);
            self.miles.available[2] += 1;
        }
        Ok(())
    }
    /// 747FF0 by the channel's own kind.
    fn close_channel_747ff0(&mut self, platform: &mut impl Miles<R>, c: usize) -> Result<()> {
        let ch = self.miles.channels[c].clone();
        match ch.kind {
            0 => platform.release_sample_handle(ch.sample_0c),
            1 => platform.release_3d_sample_handle(ch.sample3d_10),
            2 => {
                platform.close_stream(ch.stream_14)?;
                self.miles.channels[c].stream_14 = 0;
                Ok(())
            }
            _ => Ok(()),
        }
    }
    fn free_pool(&mut self, platform: &mut impl Miles<R>, pool: usize) -> Result<()> {
        while let Some(c) = self.miles.free[pool].pop_front() {
            self.close_channel_747ff0(platform, c)?;
        }
        self.miles.available[pool] = 0;
        Ok(())
    }
    /// 748F20 / 748FC0 (+ AIL_close_3D_listener) / 749070.
    fn free_2d_748f20(&mut self, platform: &mut impl Miles<R>) -> Result<()> {
        self.free_pool(platform, 0)
    }
    fn free_3d_748fc0(&mut self, platform: &mut impl Miles<R>) -> Result<()> {
        while let Some(c) = self.miles.free[1].pop_front() {
            self.close_channel_747ff0(platform, c)?;
        }
        platform.close_3d_listener(self.miles.listener_3d0)?;
        self.miles.available[1] = 0;
        Ok(())
    }
    fn free_streams_749070(&mut self, platform: &mut impl Miles<R>) -> Result<()> {
        self.free_pool(platform, 2)
    }
    /// 749460 (vt+4C) Shutdown. +1A0 keeps its value (native).
    pub(super) fn shutdown_749460(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        if self.miles.open_1a0 != -1 {
            self.stop_all_747e40(env, platform)?;
            self.free_2d_748f20(platform)?;
            self.free_3d_748fc0(platform)?;
            self.free_streams_749070(platform)?;
            platform.close_3d_provider(self.miles.provider_handle(self.miles.open_1a0)?)?;
        }
        platform.close_digital_driver(self.miles.driver_3a8)?;
        platform.shutdown()?;
        self.miles.initialized_198 = false;
        Ok(())
    }
    /// 747E40: when disabled, drain the queue; stop (or fade) everything;
    /// when disabled, sweep all lists.
    fn stop_all_747e40(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>) -> Result<()> {
        if self.disabled_189 {
            while let Some(command) = self.queue.pop_front() {
                self.process_7477f0(env, platform, command)?;
            }
        }
        for id in self.all_ids() {
            let out = self.inst(id)?.data.fade_time_s[1];
            if !greater(&f(out)?, &f(ZERO)?) || self.disabled_189 {
                self.stop_748540(platform, id)?;
            } else {
                self.begin_fade_out_748060(id)?;
            }
        }
        if self.disabled_189 {
            for list in 0..3 {
                self.sweep_746830(env, platform, list)?;
            }
        }
        Ok(())
    }
    /// 7480A0 (vt+38): reserve one available channel of the audio's kind.
    /// A streaming category already playing refuses.
    pub(super) fn reserve_channel_7480a0(&mut self, id: u32) -> Result<bool> {
        match self.kind_745850(id)? {
            0 => Ok(take(&mut self.miles.available[0])),
            1 => Ok(take(&mut self.miles.available[1])),
            2 => {
                let busy = if self.inst(id)?.data.category == 1 { self.music_38 } else { self.speech_3c };
                if busy > 0 {
                    return Ok(false);
                }
                Ok(take(&mut self.miles.available[2]))
            }
            _ => Ok(false),
        }
    }
    /// 749340 (vt+3C).
    pub(super) fn acquire_749340(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, id: u32) -> Result<Option<usize>> {
        let _ = env;
        let kind = self.kind_745850(id)?;
        if kind > 2 {
            return Ok(None);
        }
        let victim = self.find_7461c0(self.inst(id)?.interrupts_10);
        if kind == 2 {
            //748E40.
            if let Some(v) = victim {
                if self.miles.free[2].is_empty() {
                    let c = self.inst(v)?.channel_18;
                    self.stop_748540(platform, v)?;
                    self.inst_mut(v)?.channel_18 = None;
                    return Ok(c);
                }
                self.begin_fade_out_748060(v)?;
                let c = self.miles.free[2].pop_front();
                self.miles.available[2] -= 1;
                return Ok(c);
            }
            return Ok(self.miles.free[2].pop_front());
        }
        //748D60 / 748DD0.
        if let Some(v) = victim {
            let c = self.inst(v)?.channel_18;
            self.stop_748540(platform, v)?;
            self.inst_mut(v)?.channel_18 = None;
            return Ok(c);
        }
        Ok(self.miles.free[kind as usize].pop_front())
    }
    /// 749390 (vt+40): back to its pool (streams are closed first).
    pub(super) fn release_channel_749390(&mut self, platform: &mut impl Miles<R>, channel: Option<usize>) -> Result<()> {
        let Some(c) = channel else { return Ok(()) };
        let kind = self.miles.channels[c].kind;
        match kind {
            0 | 1 => self.miles.available[kind as usize] += 1,
            2 => {
                self.close_channel_747ff0(platform, c)?;
                self.miles.available[2] += 1;
            }
            _ => return Ok(()),
        }
        self.miles.free[kind as usize].push_back(c);
        Ok(())
    }
    /// 7481D0 (vt+44): AIL_set_preference(42, latency ms / 8).
    pub(super) fn latency_7481d0(&mut self, env: &mut impl Env<R, X>, platform: &mut impl Miles<R>, high: bool) -> Result<()> {
        let ms = env.uvar_i32(if high { "AudioLatencyHighMS" } else { "AudioLatencyNormalMS" })?;
        platform.set_preference(0x2a, ms / 8)
    }
    /// 748700 (vt+34): listener position and orientation, up (0,0,-1).
    pub(super) fn listener_748700(&mut self, platform: &mut impl Miles<R>) -> Result<()> {
        let listener = self.miles.listener_3d0;
        platform.set_3d_position(listener, self.mic_170)?;
        platform.set_3d_orientation(listener, self.orient_17c, [ZERO, ZERO, 0xbf80_0000])
    }
    /// 748C50 (vt+28): head insertion.
    pub(super) fn defer_close_748c50(&mut self, stream: u32) {
        self.miles.deferred.push_front(stream);
    }
    /// 748CE0 (vt+2C): close every deferred stream from the head.
    pub(super) fn deferred_streams_748ce0(&mut self, platform: &mut impl Miles<R>) -> Result<()> {
        while let Some(stream) = self.miles.deferred.pop_front() {
            if stream != 0 {
                platform.close_stream(stream)?;
            }
        }
        Ok(())
    }

    /// vf1C 74A0D0: load a non-streaming resource into Miles memory (59B550).
    pub fn file_read_74a0d0(&mut self, platform: &mut impl Miles<R>, resource: &R) -> Result<u32> {
        platform.file_read(resource)
    }
    /// vf20 748290: AIL_mem_free_lock (59AFE0 release).
    pub fn free_memory_748290(&mut self, platform: &mut impl Miles<R>, memory: u32) -> Result<()> {
        platform.mem_free_lock(memory)
    }
    /// vf14 749D40 IsValid3DFile: uncompressed (format 1) mono only. The rate
    /// and bit-depth comparison only logs (404ED0 line 0x42D).
    pub fn valid_3d_749d40(&mut self, platform: &mut impl Miles<R>, memory: u32) -> Result<bool> {
        let info = platform.wav_info(memory)?;
        Ok(info.format == 1 && info.channels == 1)
    }
    /// vf10 749BE0 IsValid2DFile: always valid (rate/bits only log).
    pub fn valid_2d_749be0(&mut self, platform: &mut impl Miles<R>, memory: u32, check_rate: bool) -> Result<bool> {
        if check_rate {
            platform.wav_info(memory)?;
        }
        Ok(true)
    }
}
fn take(count: &mut i32) -> bool {
    if *count > 0 {
        *count -= 1;
        true
    } else {
        false
    }
}
