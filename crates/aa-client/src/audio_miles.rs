//! The Miles Sound System platform under the ported [B36548] audio manager
//! (aa_runtime::audio_manager): every `_AIL_*` call TGC_MilesAudioManager
//! makes, on the client's rodio output.
//!
//! What is AA.exe behaviour and what is platform:
//! * The manager decides which file plays on which channel, when, at which
//!   volume (fades, groups, randomization), position and pitch; those calls
//!   arrive here exactly as AA.exe makes them (oracle-tested in aa-runtime).
//! * The Miles mixer itself (mss32.dll, MSS 6.5e) is a platform library and
//!   is not ported. This module implements its documented API semantics:
//!   linear volume 0..1, pan 0..1 with 0.5 centre (balance law: each side at
//!   full gain up to the centre), playback rate in Hz, AIL_stop_sample as a
//!   pause and AIL_resume_sample to continue, end-of-sample callbacks, and the
//!   "Miles Fast 2D Positional Audio" provider model of AIL_set_3D_sample_
//!   distances (full volume inside min distance, min/d beyond it, silent past
//!   max distance) with the pan taken from the listener orientation set by
//!   AIL_set_3D_orientation. These platform rules are not oracle-proven.
//! * Sound bytes come from the original install: Data.rwd, Music.rwd and
//!   Speech.rwd, mounted over the same `Audio/` tree as the original depots.
//! End-of-sample callbacks are queued by the mixer thread and handed to the
//! manager at the start of its next Update (see audio_native.rs).
use aa_runtime::audio_manager::{self as am, Miles, WavInfo};
use rodio::source::{SeekError, Source};
use rodio::{ChannelCount, SampleRate};
use std::collections::HashMap;
use std::io::Cursor;
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type Result<T> = std::result::Result<T, String>;

/// One KKC_AudioResource: the shared resource-depot allocation when the depot
/// owns it, and its logical path below the `Audio/` mount.
#[derive(Clone, Debug)]
pub struct SoundRef {
    pub path: String,
    pub handle: Option<aa_runtime::resource_objects::Handle>,
}
impl PartialEq for SoundRef {
    fn eq(&self, other: &Self) -> bool {
        match (&self.handle, &other.handle) {
            (Some(a), Some(b)) => a.same_identity(b),
            _ => self.path.eq_ignore_ascii_case(&other.path),
        }
    }
}

/// Data.rwd, Music.rwd and Speech.rwd. None of the 23 v14 Data overlays is
/// an audio file (research/evidence/v14-baseline.json), so the original
/// archives are read directly.
pub struct Archives {
    archives: Vec<(String, aa_formats::RwdArchive<std::fs::File>)>,
}
impl Archives {
    /// `install` is the game folder (AA.exe, Data.rwd, Music.rwd, Speech.rwd).
    pub fn open(install: &std::path::Path) -> Result<Self> {
        let mut archives = Vec::new();
        for name in ["Data.rwd", "Music.rwd", "Speech.rwd"] {
            match aa_formats::RwdArchive::open(install.join(name)) {
                Ok(archive) => archives.push((name.to_owned(), archive)),
                Err(e) => log::warn!("audio: {name} not mounted ({e})"),
            }
        }
        if archives.is_empty() {
            return Err(format!("audio: no RWD archive under {}", install.display()));
        }
        Ok(Self { archives })
    }
    pub fn read(&mut self, path: &str) -> Result<Vec<u8>> {
        for (_, archive) in &mut self.archives {
            if let Some(index) = archive.find(path).map_err(|e| e.to_string())? {
                let mut bytes = Vec::new();
                archive.copy_entry(index, &mut bytes).map_err(|e| e.to_string())?;
                return Ok(bytes);
            }
        }
        Err(format!("audio resource {path} is in none of Data.rwd/Music.rwd/Speech.rwd"))
    }
}

/// Decoded AIL_file_read memory and the archives it is read from, shared
/// between the platform and the manager's resource services.
pub struct SoundStore {
    archives: Option<Archives>,
    memory: HashMap<u32, Arc<Memory>>,
    next: u32,
}
impl SoundStore {
    pub fn new(archives: Option<Archives>) -> Self {
        Self { archives, memory: HashMap::new(), next: 0x7000_0000 }
    }
    pub fn read(&mut self, path: &str) -> Result<Vec<u8>> {
        self.archives.as_mut().ok_or("audio platform has no mounted archives")?.read(path)
    }
    /// AIL_file_read + decode.
    pub fn load(&mut self, path: &str) -> Result<u32> {
        let bytes = self.read(path)?;
        let memory = decode_wav(path, &bytes)?;
        self.next += 1;
        self.memory.insert(self.next, Arc::new(memory));
        Ok(self.next)
    }
    pub fn free(&mut self, memory: u32) {
        self.memory.remove(&memory);
    }
}
/// Decoded AIL_file_read memory.
struct Memory {
    path: String,
    samples: Arc<[i16]>,
    channels: u16,
    rate: u32,
    info: WavInfo,
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?]))
}
fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([*b.get(at)?, *b.get(at + 1)?, *b.get(at + 2)?, *b.get(at + 3)?]))
}
/// RIFF/WAVE PCM 8/16-bit (the formats the original sound files use).
fn decode_wav(path: &str, bytes: &[u8]) -> Result<Memory> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(format!("{path}: not RIFF/WAVE"));
    }
    let (mut pos, mut fmt, mut data) = (12usize, None, None);
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = le32(bytes, pos + 4).unwrap_or(0) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size).min(bytes.len());
        if id == b"fmt " && end - start >= 16 {
            let b = &bytes[start..end];
            fmt = Some((le16(b, 0).unwrap_or(0), le16(b, 2).unwrap_or(0), le32(b, 4).unwrap_or(0), le16(b, 14).unwrap_or(0)));
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        pos = start.saturating_add(size).saturating_add(size & 1);
    }
    let (format, channels, rate, bits) = fmt.ok_or(format!("{path}: no fmt chunk"))?;
    let data = data.ok_or(format!("{path}: no data chunk"))?;
    let samples: Vec<i16> = match (format, bits) {
        (1, 16) => data.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect(),
        (1, 8) => data.iter().map(|&b| ((b as i16) - 128) << 8).collect(),
        _ => return Err(format!("{path}: WAV format {format} with {bits} bits is not decoded by this platform")),
    };
    if channels == 0 || rate == 0 {
        return Err(format!("{path}: {channels} channels at {rate} Hz"));
    }
    Ok(Memory {
        path: path.to_owned(),
        samples: samples.into(),
        channels,
        rate,
        info: WavInfo { format: format as i32, rate: rate as i32, bits: bits as i32, channels: channels as i32 },
    })
}

struct AtomicF32(AtomicU32);
impl AtomicF32 {
    fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed)
    }
}
/// Shared between the platform and one playing source.
struct Voice {
    left: AtomicF32,
    right: AtomicF32,
    paused: AtomicBool,
    cut: AtomicBool,
    eos: AtomicBool,
    kind: u8,
    handle: u32,
    events: Arc<Mutex<Vec<(u8, u32)>>>,
}
impl Voice {
    fn finished_naturally(&self) {
        if !self.cut.load(Ordering::Acquire) && self.eos.load(Ordering::Acquire) {
            if let Ok(mut e) = self.events.lock() {
                e.push((self.kind, self.handle));
            }
        }
    }
}

const STEREO: ChannelCount = match NonZero::new(2) {
    Some(v) => v,
    None => unreachable!(),
};

/// A sample at its own playback rate (rodio's mixer resamples).
struct PcmSource {
    memory: Arc<[i16]>,
    channels: usize,
    frame: usize,
    right: Option<f32>,
    rate: SampleRate,
    voice: Arc<Voice>,
    done: bool,
}
impl Iterator for PcmSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if let Some(r) = self.right.take() {
            return Some(r);
        }
        if self.done || self.voice.cut.load(Ordering::Relaxed) {
            self.done = true;
            return None;
        }
        if self.voice.paused.load(Ordering::Relaxed) {
            self.right = Some(0.0);
            return Some(0.0);
        }
        let base = self.frame * self.channels;
        if base + self.channels > self.memory.len() {
            self.done = true;
            self.voice.finished_naturally();
            return None;
        }
        let l = self.memory[base] as f32 / 32768.0;
        let r = if self.channels > 1 { self.memory[base + 1] as f32 / 32768.0 } else { l };
        self.frame += 1;
        self.right = Some(r * self.voice.right.get());
        Some(l * self.voice.left.get())
    }
}
impl Source for PcmSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> ChannelCount {
        STEREO
    }
    fn sample_rate(&self) -> SampleRate {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
    fn try_seek(&mut self, _: Duration) -> std::result::Result<(), SeekError> {
        Err(SeekError::NotSupported { underlying_source: "PcmSource" })
    }
}
/// A streamed MP3/WAV (Music.rwd/Speech.rwd) with the stream's gains.
struct StreamSource {
    inner: rodio::Decoder<Cursor<Vec<u8>>>,
    channel: u16,
    voice: Arc<Voice>,
    done: bool,
}
impl Iterator for StreamSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.done || self.voice.cut.load(Ordering::Relaxed) {
            self.done = true;
            return None;
        }
        let channels = self.inner.channels().get();
        if self.voice.paused.load(Ordering::Relaxed) && self.channel == 0 {
            //Paused streams hold their position; emit silence frames.
            return Some(0.0);
        }
        let Some(sample) = self.inner.next() else {
            self.done = true;
            self.voice.finished_naturally();
            return None;
        };
        let gain = if self.channel == 0 { self.voice.left.get() } else { self.voice.right.get() };
        self.channel = (self.channel + 1) % channels.max(1);
        Some(sample * gain)
    }
}
impl Source for StreamSource {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }
    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }
    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
    fn try_seek(&mut self, _: Duration) -> std::result::Result<(), SeekError> {
        Err(SeekError::NotSupported { underlying_source: "StreamSource" })
    }
}

#[derive(Default)]
struct Sample {
    three_d: bool,
    memory: Option<u32>,
    rate: i32,
    volume: f32,
    pan: f32,
    eos: bool,
    voice: Option<Arc<Voice>>,
    position: [f32; 3],
    distances: [f32; 2],
}
struct Stream {
    resource: SoundRef,
    bytes: Option<Vec<u8>>,
    voice: Option<Arc<Voice>>,
    volume: f32,
    pan: f32,
    eos: bool,
    paused: bool,
}

/// One played or stopped sound for the run log (native identities).
#[derive(Clone, Debug)]
pub struct PlayLog {
    pub kind: &'static str,
    pub handle: u32,
    pub file: String,
}

pub struct MilesPlatform {
    mixer: Option<rodio::mixer::Mixer>,
    store: Arc<Mutex<SoundStore>>,
    next: u32,
    samples: HashMap<u32, Sample>,
    streams: HashMap<u32, Stream>,
    listener: ([f32; 3], [f32; 3], [f32; 3]),
    listener_handle: u32,
    events: Arc<Mutex<Vec<(u8, u32)>>>,
    pub played: Vec<PlayLog>,
}
fn bits(v: u32) -> f32 {
    f32::from_bits(v)
}
/// Balance law: each side keeps full gain up to the centre.
fn balance(pan: f32) -> [f32; 2] {
    let pan = if pan.is_finite() { pan.clamp(0.0, 1.0) } else { 0.5 };
    [(2.0 * (1.0 - pan)).min(1.0), (2.0 * pan).min(1.0)]
}
fn volume(v: f32) -> f32 {
    if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 }
}

impl MilesPlatform {
    pub fn new(mixer: Option<rodio::mixer::Mixer>, store: Arc<Mutex<SoundStore>>) -> Self {
        Self {
            mixer,
            store,
            next: 0x100,
            samples: HashMap::new(),
            streams: HashMap::new(),
            listener: ([0.0; 3], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]),
            listener_handle: 0,
            events: Arc::new(Mutex::new(Vec::new())),
            played: Vec::new(),
        }
    }
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        self.store.lock().map_err(|_| "audio store poisoned")?.read(path)
    }
    fn memory(&self, memory: u32) -> Result<Arc<Memory>> {
        self.store.lock().map_err(|_| "audio store poisoned")?.memory.get(&memory).cloned().ok_or_else(|| format!("Miles memory {memory:#x} is not allocated"))
    }
    fn handle(&mut self) -> u32 {
        self.next += 1;
        self.next
    }
    /// End-of-sample/stream callbacks since the last call: (0 2-D, 1 3-D,
    /// 2 stream, handle), in mixer order.
    pub fn take_events(&mut self) -> Vec<(u8, u32)> {
        self.events.lock().map(|mut e| std::mem::take(&mut *e)).unwrap_or_default()
    }
    fn gains_2d(s: &Sample) -> [f32; 2] {
        let b = balance(s.pan);
        let v = volume(s.volume);
        [v * b[0], v * b[1]]
    }
    /// The Fast 2D Positional Audio model (platform, see the module doc).
    fn gains_3d(&self, s: &Sample) -> [f32; 2] {
        let (pos, face, up) = self.listener;
        let d = [s.position[0] - pos[0], s.position[1] - pos[1], s.position[2] - pos[2]];
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        let [max, min] = s.distances;
        let mut gain = volume(s.volume);
        if dist > max {
            gain = 0.0;
        } else if dist > min && dist > 0.0 {
            gain *= min / dist;
        }
        let right = [up[1] * face[2] - up[2] * face[1], up[2] * face[0] - up[0] * face[2], up[0] * face[1] - up[1] * face[0]];
        let rl = (right[0] * right[0] + right[1] * right[1] + right[2] * right[2]).sqrt();
        let lateral = if dist > 1e-4 && rl > 1e-6 { (d[0] * right[0] + d[1] * right[1] + d[2] * right[2]) / (dist * rl) } else { 0.0 };
        let b = balance(0.5 + 0.5 * lateral);
        [gain * b[0], gain * b[1]]
    }
    fn refresh(&self, handle: u32) {
        if let Some(s) = self.samples.get(&handle) {
            if let Some(v) = &s.voice {
                let g = if s.three_d { self.gains_3d(s) } else { Self::gains_2d(s) };
                v.left.set(g[0]);
                v.right.set(g[1]);
            }
        }
    }
    fn refresh_all_3d(&self) {
        for (h, s) in &self.samples {
            if s.three_d {
                self.refresh(*h);
            }
        }
    }
    fn cut(voice: &mut Option<Arc<Voice>>) {
        if let Some(v) = voice.take() {
            v.cut.store(true, Ordering::Release);
        }
    }
    fn sample(&mut self, h: u32) -> Result<&mut Sample> {
        self.samples.get_mut(&h).ok_or_else(|| format!("Miles sample handle {h:#x} is not allocated"))
    }
    fn start(&mut self, h: u32, kind: u8) -> Result<()> {
        let events = Arc::clone(&self.events);
        let s = self.samples.get_mut(&h).ok_or("start of an unallocated sample")?;
        Self::cut(&mut s.voice);
        let Some(m) = s.memory else { return Ok(()) };
        let eos = s.eos;
        let memory = self.memory(m)?;
        let s = self.samples.get_mut(&h).ok_or("start of an unallocated sample")?;
        let voice = Arc::new(Voice { left: AtomicF32::new(0.0), right: AtomicF32::new(0.0), paused: AtomicBool::new(false),
            cut: AtomicBool::new(false), eos: AtomicBool::new(eos), kind, handle: h, events });
        s.voice = Some(Arc::clone(&voice));
        let rate = s.rate.max(1) as u32;
        self.refresh(h);
        self.played.push(PlayLog { kind: if kind == 0 { "2d" } else { "3d" }, handle: h, file: memory.path.clone() });
        if let Some(mixer) = &self.mixer {
            mixer.add(PcmSource {
                channels: memory.channels as usize,
                memory: Arc::clone(&memory.samples),
                frame: 0,
                right: None,
                rate: NonZero::new(rate).unwrap_or(NonZero::new(22050).unwrap()),
                voice,
                done: false,
            });
        }
        Ok(())
    }
}

impl Miles<SoundRef> for MilesPlatform {
    fn set_redist_directory(&mut self, _: &str) -> Result<()> { Ok(()) }
    fn startup(&mut self) -> Result<()> { Ok(()) }
    fn open_digital_driver(&mut self, _f: i32, _b: i32, _c: i32, _fl: u32) -> Result<u32> { Ok(self.handle()) }
    fn enumerate_3d_providers(&mut self, cursor: &mut u32) -> Result<Option<(u32, String)>> {
        if *cursor > 0 {
            return Ok(None);
        }
        *cursor += 1;
        Ok(Some((0x6000, am::PROVIDER_FAST_2D.to_owned())))
    }
    fn open_3d_provider(&mut self, _: u32) -> Result<u32> { Ok(0) }
    fn close_3d_provider(&mut self, _: u32) -> Result<()> { Ok(()) }
    fn set_3d_speaker_type(&mut self, _: u32, _: u32) -> Result<()> { Ok(()) }
    fn open_3d_listener(&mut self, _: u32) -> Result<u32> {
        self.listener_handle = self.handle();
        Ok(self.listener_handle)
    }
    fn close_3d_listener(&mut self, _: u32) -> Result<()> { Ok(()) }
    fn set_file_callbacks(&mut self) -> Result<()> { Ok(()) }
    fn set_preference(&mut self, _: u32, _: i32) -> Result<()> { Ok(()) }
    fn close_digital_driver(&mut self, _: u32) -> Result<()> { Ok(()) }
    fn shutdown(&mut self) -> Result<()> {
        for s in self.samples.values_mut() {
            Self::cut(&mut s.voice);
        }
        for s in self.streams.values_mut() {
            Self::cut(&mut s.voice);
        }
        Ok(())
    }
    fn allocate_sample_handle(&mut self, _: u32) -> Result<u32> {
        let h = self.handle();
        self.samples.insert(h, Sample { rate: 22050, volume: 1.0, pan: 0.5, ..Sample::default() });
        Ok(h)
    }
    fn init_sample(&mut self, h: u32) -> Result<()> {
        let s = self.sample(h)?;
        Self::cut(&mut s.voice);
        s.memory = None;
        s.eos = false;
        Ok(())
    }
    fn release_sample_handle(&mut self, h: u32) -> Result<()> {
        if let Some(mut s) = self.samples.remove(&h) {
            Self::cut(&mut s.voice);
        }
        Ok(())
    }
    fn set_sample_file(&mut self, h: u32, memory: u32, _: u32) -> Result<()> {
        let rate = self.memory(memory)?.rate as i32;
        let s = self.sample(h)?;
        Self::cut(&mut s.voice);
        s.memory = Some(memory);
        s.rate = rate;
        Ok(())
    }
    fn register_eos_callback(&mut self, h: u32, enabled: bool) -> Result<()> {
        let s = self.sample(h)?;
        s.eos = enabled;
        if let Some(v) = &s.voice {
            v.eos.store(enabled, Ordering::Release);
        }
        Ok(())
    }
    fn start_sample(&mut self, h: u32) -> Result<()> { self.start(h, 0) }
    fn stop_sample(&mut self, h: u32) -> Result<()> {
        if let Some(v) = &self.sample(h)?.voice {
            v.paused.store(true, Ordering::Relaxed);
        }
        Ok(())
    }
    fn resume_sample(&mut self, h: u32) -> Result<()> {
        if let Some(v) = &self.sample(h)?.voice {
            v.paused.store(false, Ordering::Relaxed);
        }
        Ok(())
    }
    fn sample_status(&mut self, h: u32) -> Result<u32> {
        Ok(if self.sample(h)?.voice.is_some() { 4 } else { 2 })
    }
    fn sample_playback_rate(&mut self, h: u32) -> Result<i32> { Ok(self.sample(h)?.rate) }
    fn set_sample_playback_rate(&mut self, h: u32, rate: i32) -> Result<()> {
        self.sample(h)?.rate = rate;
        Ok(())
    }
    fn set_sample_volume_pan(&mut self, h: u32, v: u32, p: u32) -> Result<()> {
        let s = self.sample(h)?;
        s.volume = bits(v);
        s.pan = bits(p);
        self.refresh(h);
        Ok(())
    }
    fn allocate_3d_sample_handle(&mut self, _: u32) -> Result<u32> {
        let h = self.handle();
        self.samples.insert(h, Sample { three_d: true, rate: 22050, volume: 1.0, pan: 0.5, distances: [64.0, 4.0], ..Sample::default() });
        Ok(h)
    }
    fn release_3d_sample_handle(&mut self, h: u32) -> Result<()> { self.release_sample_handle(h) }
    fn set_3d_sample_file(&mut self, h: u32, memory: u32) -> Result<()> { self.set_sample_file(h, memory, 0) }
    fn register_3d_eos_callback(&mut self, h: u32, enabled: bool) -> Result<()> { self.register_eos_callback(h, enabled) }
    fn start_3d_sample(&mut self, h: u32) -> Result<()> { self.start(h, 1) }
    fn stop_3d_sample(&mut self, h: u32) -> Result<()> { self.stop_sample(h) }
    fn resume_3d_sample(&mut self, h: u32) -> Result<()> { self.resume_sample(h) }
    fn sample_3d_status(&mut self, h: u32) -> Result<u32> { self.sample_status(h) }
    fn sample_3d_playback_rate(&mut self, h: u32) -> Result<i32> { self.sample_playback_rate(h) }
    fn set_3d_sample_playback_rate(&mut self, h: u32, r: i32) -> Result<()> { self.set_sample_playback_rate(h, r) }
    fn set_3d_sample_volume(&mut self, h: u32, v: u32) -> Result<()> {
        self.sample(h)?.volume = bits(v);
        self.refresh(h);
        Ok(())
    }
    fn set_3d_sample_distances(&mut self, h: u32, max: u32, min: u32) -> Result<()> {
        self.sample(h)?.distances = [bits(max), bits(min)];
        self.refresh(h);
        Ok(())
    }
    fn set_3d_position(&mut self, object: u32, p: [u32; 3]) -> Result<()> {
        let p = [bits(p[0]), bits(p[1]), bits(p[2])];
        if object == self.listener_handle {
            self.listener.0 = p;
            self.refresh_all_3d();
            return Ok(());
        }
        self.sample(object)?.position = p;
        self.refresh(object);
        Ok(())
    }
    fn set_3d_orientation(&mut self, object: u32, face: [u32; 3], up: [u32; 3]) -> Result<()> {
        if object == self.listener_handle {
            self.listener.1 = [bits(face[0]), bits(face[1]), bits(face[2])];
            self.listener.2 = [bits(up[0]), bits(up[1]), bits(up[2])];
            self.refresh_all_3d();
        }
        Ok(())
    }
    fn open_stream(&mut self, _: u32, resource: &SoundRef) -> Result<u32> {
        let bytes = self.read(&resource.path)?;
        let h = self.handle();
        self.streams.insert(h, Stream { resource: resource.clone(), bytes: Some(bytes), voice: None, volume: 1.0, pan: 0.5, eos: false, paused: false });
        Ok(h)
    }
    fn close_stream(&mut self, h: u32) -> Result<()> {
        if let Some(mut s) = self.streams.remove(&h) {
            Self::cut(&mut s.voice);
        }
        Ok(())
    }
    fn start_stream(&mut self, h: u32) -> Result<()> {
        let events = Arc::clone(&self.events);
        let s = self.streams.get_mut(&h).ok_or("AIL_start_stream of an unopened stream")?;
        Self::cut(&mut s.voice);
        let bytes = s.bytes.take().ok_or("AIL_start_stream restarted a consumed stream")?;
        let len = bytes.len() as u64;
        let hint = s.resource.path.rsplit('.').next().unwrap_or("mp3").to_ascii_lowercase();
        let decoder = rodio::Decoder::builder().with_data(Cursor::new(bytes)).with_byte_len(len).with_hint(&hint).with_seekable(true).build()
            .map_err(|e| format!("stream {}: {e}", s.resource.path))?;
        let b = balance(s.pan);
        let v = volume(s.volume);
        let voice = Arc::new(Voice { left: AtomicF32::new(v * b[0]), right: AtomicF32::new(v * b[1]), paused: AtomicBool::new(s.paused),
            cut: AtomicBool::new(false), eos: AtomicBool::new(s.eos), kind: 2, handle: h, events });
        s.voice = Some(Arc::clone(&voice));
        self.played.push(PlayLog { kind: "stream", handle: h, file: s.resource.path.clone() });
        if let Some(mixer) = &self.mixer {
            mixer.add(StreamSource { inner: decoder, channel: 0, voice, done: false });
        }
        Ok(())
    }
    fn pause_stream(&mut self, h: u32, on: i32) -> Result<()> {
        if let Some(s) = self.streams.get_mut(&h) {
            s.paused = on != 0;
            if let Some(v) = &s.voice {
                v.paused.store(on != 0, Ordering::Relaxed);
            }
        }
        Ok(())
    }
    fn stream_status(&mut self, h: u32) -> Result<u32> {
        Ok(if self.streams.get(&h).is_some_and(|s| s.voice.is_some()) { 4 } else { 2 })
    }
    fn register_stream_callback(&mut self, h: u32, enabled: bool) -> Result<()> {
        if let Some(s) = self.streams.get_mut(&h) {
            s.eos = enabled;
            if let Some(v) = &s.voice {
                v.eos.store(enabled, Ordering::Release);
            }
        }
        Ok(())
    }
    fn set_stream_volume_pan(&mut self, h: u32, v: u32, p: u32) -> Result<()> {
        if let Some(s) = self.streams.get_mut(&h) {
            s.volume = bits(v);
            s.pan = bits(p);
            if let Some(voice) = &s.voice {
                let b = balance(s.pan);
                voice.left.set(volume(s.volume) * b[0]);
                voice.right.set(volume(s.volume) * b[1]);
            }
        }
        Ok(())
    }
    fn stream_length_ms(&mut self, _: u32) -> Result<i32> {
        Err("74A4B0 AIL_stream_ms_position is not used by the battle path".into())
    }
    fn file_read(&mut self, resource: &SoundRef) -> Result<u32> {
        self.store.lock().map_err(|_| "audio store poisoned")?.load(&resource.path)
    }
    fn mem_free_lock(&mut self, memory: u32) -> Result<()> {
        self.store.lock().map_err(|_| "audio store poisoned")?.free(memory);
        Ok(())
    }
    fn wav_info(&mut self, memory: u32) -> Result<WavInfo> {
        Ok(self.memory(memory)?.info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn balance_law_centre_and_sides() {
        assert_eq!(balance(0.5), [1.0, 1.0]);
        assert_eq!(balance(0.0), [1.0, 0.0]);
        assert_eq!(balance(1.0), [0.0, 1.0]);
    }
    #[test]
    fn fast_2d_distance_model() {
        let mut p = MilesPlatform::new(None, Arc::new(Mutex::new(SoundStore::new(None))));
        let s = Sample { three_d: true, volume: 1.0, distances: [64.0, 4.0], position: [0.0, 8.0, 0.0], ..Sample::default() };
        p.listener = ([0.0; 3], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]);
        let g = p.gains_3d(&s);
        assert!((g[0] - 0.5).abs() < 1e-6 && (g[1] - 0.5).abs() < 1e-6, "{g:?}");
        let s = Sample { position: [100.0, 0.0, 0.0], ..s };
        assert_eq!(p.gains_3d(&s), [0.0, 0.0]);
        let s = Sample { position: [3.0, 0.0, 0.0], ..s };
        let g = p.gains_3d(&s);
        assert!(g[1] == 1.0 && g[0] == 0.0, "east of a north-facing listener is right: {g:?}");
    }
}
