//! The Miles Sound System platform under the audio manager: a software mixer
//! played through Bevy's audio output (one endless rodio source), with the
//! documented Miles semantics the wgpu client's audio_miles.rs implements:
//! linear volume, pan 0..1 with 0.5 centre (balance law: each side keeps
//! full gain up to the centre), playback rate from the pitch, and an
//! end-of-sample report per voice. Samples are mixed at MilesFrequency
//! (44100 Hz) and saturate like a 16-bit Miles mix (no limiter).
//! Original sound files: 16-bit PCM WAV (every shipped effect and voice)
//! decoded here; MP3 music and speech decoded by rodio's MP3 decoder on a
//! loader thread.
use bevy::asset::Asset;
use bevy::audio::Decodable;
use bevy::reflect::TypePath;
use std::num::NonZero;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const OUTPUT_RATE: u32 = 44_100;
/// Frames mixed per lock of the shared voice table.
const BLOCK: usize = 512;
/// Per-frame gain smoothing (avoids zipper noise on 3D gain updates).
const GAIN_SMOOTH: f32 = 0.004;

/// Decoded PCM: interleaved i16.
pub struct Pcm {
    pub samples: Vec<i16>,
    pub channels: u16,
    pub rate: u32,
}

impl Pcm {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }
    pub fn seconds(&self) -> f32 {
        self.frames() as f32 / self.rate.max(1) as f32
    }
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?]))
}
fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([*b.get(at)?, *b.get(at + 1)?, *b.get(at + 2)?, *b.get(at + 3)?]))
}

/// RIFF/WAVE PCM 8/16-bit (the formats the original sound files use).
pub fn decode_wav(bytes: &[u8]) -> Result<Pcm, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not RIFF/WAVE".into());
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
    let (format, channels, rate, bits) = fmt.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    let samples: Vec<i16> = match (format, bits) {
        (1, 16) => data.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect(),
        (1, 8) => data.iter().map(|&b| ((b as i16) - 128) << 8).collect(),
        _ => return Err(format!("WAV format {format} with {bits} bits")),
    };
    if channels == 0 || rate == 0 {
        return Err(format!("{channels} channels at {rate} Hz"));
    }
    Ok(Pcm { samples, channels, rate })
}

/// MP3 (music, speech) through rodio's decoder.
pub fn decode_mp3(bytes: Vec<u8>) -> Result<Pcm, String> {
    use rodio::Source;
    let len = bytes.len() as u64;
    let decoder = rodio::Decoder::builder()
        .with_byte_len(len)
        .with_data(std::io::Cursor::new(bytes))
        .build()
        .map_err(|e| e.to_string())?;
    let channels = decoder.channels().get();
    let rate = decoder.sample_rate().get();
    let samples: Vec<i16> = decoder.map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16).collect();
    Ok(Pcm { samples, channels, rate })
}

struct Voice {
    id: u32,
    pcm: Arc<Pcm>,
    /// Source frame position (fractional).
    pos: f64,
    /// Source frames per output frame (rate * pitch / output rate).
    step: f64,
    target: [f32; 2],
    gain: [f32; 2],
    paused: bool,
}

#[derive(Default)]
struct Table {
    voices: Vec<Voice>,
    ended: Vec<u32>,
    /// Verification capture of the mixed output: (samples, wanted length).
    capture: Option<(Vec<f32>, usize)>,
}

/// The game-thread handle on the mixer.
#[derive(Clone, Default)]
pub struct Mixer(Arc<Mutex<Table>>);

impl Mixer {
    /// AIL_start_sample: a new voice at `gains` (left, right) and `pitch`.
    pub fn start(&self, id: u32, pcm: Arc<Pcm>, gains: [f32; 2], pitch: f32) {
        let step = pcm.rate as f64 * pitch.max(0.01) as f64 / OUTPUT_RATE as f64;
        if let Ok(mut t) = self.0.lock() {
            t.voices.push(Voice { id, pcm, pos: 0.0, step, target: gains, gain: gains, paused: false });
        }
    }
    /// Volume/pan (and 3D) updates for many voices at once.
    pub fn set_gains(&self, gains: &[(u32, [f32; 2])]) {
        if gains.is_empty() {
            return;
        }
        if let Ok(mut t) = self.0.lock() {
            for v in &mut t.voices {
                if let Some((_, g)) = gains.iter().find(|(id, _)| *id == v.id) {
                    v.target = *g;
                }
            }
        }
    }
    pub fn stop(&self, id: u32) {
        if let Ok(mut t) = self.0.lock() {
            t.voices.retain(|v| v.id != id);
        }
    }
    pub fn pause(&self, id: u32, paused: bool) {
        if let Ok(mut t) = self.0.lock() {
            for v in t.voices.iter_mut().filter(|v| v.id == id) {
                v.paused = paused;
            }
        }
    }
    /// End-of-sample reports since the last call (the 749510/749550/749590
    /// callbacks the manager handles at the start of its update).
    pub fn take_ended(&self) -> Vec<u32> {
        self.0.lock().map(|mut t| std::mem::take(&mut t.ended)).unwrap_or_default()
    }
    pub fn playing(&self) -> usize {
        self.0.lock().map(|t| t.voices.len()).unwrap_or(0)
    }
    /// Record the next `seconds` of mixed output (verification runs).
    pub fn start_capture(&self, seconds: f32) {
        if let Ok(mut t) = self.0.lock() {
            let n = (seconds * OUTPUT_RATE as f32) as usize * 2;
            t.capture = Some((Vec::with_capacity(n), n));
        }
    }
    /// The capture once complete.
    pub fn take_capture(&self) -> Option<Vec<f32>> {
        let mut t = self.0.lock().ok()?;
        if t.capture.as_ref().is_some_and(|(v, n)| v.len() >= *n) {
            return t.capture.take().map(|(v, _)| v);
        }
        None
    }
}

/// 16-bit stereo WAV of interleaved samples.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&OUTPUT_RATE.to_le_bytes());
    out.extend_from_slice(&(OUTPUT_RATE * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data as u32).to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

fn mix(table: &mut Table, out: &mut [f32]) {
    out.iter_mut().for_each(|s| *s = 0.0);
    let frames = out.len() / 2;
    let mut ended = Vec::new();
    for v in &mut table.voices {
        if v.paused {
            continue;
        }
        let ch = v.pcm.channels.max(1) as usize;
        let total = v.pcm.frames();
        let s = &v.pcm.samples;
        for f in 0..frames {
            let i = v.pos as usize;
            if i + 1 >= total {
                v.pos = total as f64;
                break;
            }
            let frac = (v.pos - i as f64) as f32;
            v.gain[0] += (v.target[0] - v.gain[0]) * GAIN_SMOOTH;
            v.gain[1] += (v.target[1] - v.gain[1]) * GAIN_SMOOTH;
            let (l, r) = if ch == 1 {
                let a = s[i] as f32;
                let b = s[i + 1] as f32;
                let x = (a + (b - a) * frac) * (1.0 / 32768.0);
                (x, x)
            } else {
                let a0 = s[i * ch] as f32;
                let b0 = s[(i + 1) * ch] as f32;
                let a1 = s[i * ch + 1] as f32;
                let b1 = s[(i + 1) * ch + 1] as f32;
                ((a0 + (b0 - a0) * frac) * (1.0 / 32768.0), (a1 + (b1 - a1) * frac) * (1.0 / 32768.0))
            };
            out[f * 2] += l * v.gain[0];
            out[f * 2 + 1] += r * v.gain[1];
            v.pos += v.step;
        }
        if v.pos as usize + 1 >= total {
            ended.push(v.id);
        }
    }
    if !ended.is_empty() {
        table.voices.retain(|v| !ended.contains(&v.id));
        table.ended.extend(ended);
    }
    // 16-bit Miles mix: saturate.
    for s in out.iter_mut() {
        *s = s.clamp(-1.0, 1.0);
    }
    if let Some((v, n)) = table.capture.as_mut()
        && v.len() < *n
    {
        v.extend_from_slice(out);
    }
}

/// The endless output stream Bevy plays.
pub struct MixerSource {
    mixer: Mixer,
    buf: Vec<f32>,
    at: usize,
}

impl Iterator for MixerSource {
    type Item = rodio::Sample;
    fn next(&mut self) -> Option<rodio::Sample> {
        if self.at >= self.buf.len() {
            if let Ok(mut t) = self.mixer.0.lock() {
                mix(&mut t, &mut self.buf);
            } else {
                self.buf.iter_mut().for_each(|s| *s = 0.0);
            }
            self.at = 0;
        }
        let s = self.buf[self.at];
        self.at += 1;
        Some(s as rodio::Sample)
    }
}

impl rodio::Source for MixerSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        NonZero::new(2).expect("two channels")
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        NonZero::new(OUTPUT_RATE).expect("rate")
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// The asset Bevy's audio output plays (once, forever).
#[derive(Asset, TypePath, Clone)]
pub struct MixerAsset {
    pub mixer: Mixer,
}

impl Decodable for MixerAsset {
    type Decoder = MixerSource;
    fn decoder(&self) -> MixerSource {
        MixerSource { mixer: self.mixer.clone(), buf: vec![0.0; BLOCK * 2], at: BLOCK * 2 }
    }
}
