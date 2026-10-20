//! The original audio manager [B36548] (TGC_MilesAudioManager, ported
//! byte-exact in aa-runtime audio_manager*.rs), here behaviour-faithful in
//! plain f32 over the software mixer:
//! - PlaySound 7474F0 / looping 747630: CanPlay 7471E0 = ShouldPlayLocally
//!   746300 (who_flags against the owner's relation to the local player),
//!   in range 745C20 (WORLD sounds closer than range[1] to the microphone),
//!   the object's single VOICE 746410, the simultaneous limit 746A80 (same
//!   IDS playing or queued; one MUSIC and one SPEECH stream), a free Miles
//!   channel 7480A0 (UVars Miles2D/3D/StreamingChannels 8/21/3), else
//!   INTERRUPT 746F40 (the oldest same-IDS sound, or another INTERRUPT
//!   sound of the same category, gives up its channel);
//! - start 747390 with the part selection 745DD0 (leadin, main: SEQUENTIAL
//!   or random, ALL/looping draws a new random main file per repeat,
//!   fallout), pitch_adjust and volume_adjust randomisation, CAUSEFADE
//!   ducking 747070 (AudioFadeVolume over AudioFadeTime, recovered after
//!   AudioRecoverFromFadeTime);
//! - per frame 747A80: volume 748790 (group x user volume x random volume,
//!   fade_time_s in/out), the microphone 745FA0 at the camera look-at plus
//!   AudioMicrophoneHeight facing the horizontal view direction, 3-D
//!   positions 7485B0 pulled toward the microphone by camera zoom x
//!   AudioConsiderCenteredRadius, object sounds following their actor and
//!   stopping when it is gone (745D60);
//! - the Miles "Fast 2D Positional Audio" model of AIL_set_3D_sample_
//!   distances (full volume inside min distance, min/d beyond, silent past
//!   max distance) with the pan from the listener's right axis.
use super::mixer::{Mixer, Pcm};
use crate::effects::data::AudioDef;
use bevy::math::Vec3;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const CONTROL_INTERRUPT: u8 = 1;
pub const CONTROL_SEQUENTIAL: u8 = 2;
pub const CONTROL_ALL: u8 = 4;
pub const CONTROL_VOICE: u8 = 8;
pub const CONTROL_WORLD: u8 = 0x10;
pub const CONTROL_CAUSEFADE: u8 = 0x20;
pub const WORLD_GLOBAL: u8 = 1;

/// The UVars / AVars the manager reads (Data/UVars.tgi, AVars.tgi).
#[derive(Clone, Debug)]
pub struct Vars {
    pub channels: [usize; 3],
    pub main: f32,
    pub d2: f32,
    pub d3: f32,
    pub speech: f32,
    pub music: f32,
    pub mic_height: f32,
    pub centered_radius: f32,
    pub drop_timeout: f32,
    pub fade_volume: f32,
    pub fade_time: f32,
    pub recover_time: f32,
}

impl Default for Vars {
    fn default() -> Self {
        Self {
            channels: [8, 21, 3],
            main: 0.8,
            d2: 0.8,
            d3: 0.8,
            speech: 0.95,
            music: 0.7,
            mic_height: 5.0,
            centered_radius: 12.0,
            drop_timeout: 5.0,
            fade_volume: 0.5,
            fade_time: 1.0,
            recover_time: 2.0,
        }
    }
}

impl Vars {
    /// `type Name = value` records of the original var files.
    pub fn read(texts: &[String]) -> Self {
        let mut map = HashMap::new();
        for text in texts {
            for line in text.lines() {
                let line = line.split(";;").next().unwrap_or("").trim();
                let Some((left, value)) = line.split_once('=') else { continue };
                let mut words = left.split_whitespace();
                let (Some(kind), Some(name), None) = (words.next(), words.next(), words.next()) else { continue };
                if !["int", "float", "fixed", "flag", "string"].contains(&kind.to_ascii_lowercase().as_str()) {
                    continue;
                }
                let v = value.trim().trim_matches('"');
                let n = match v.strip_suffix('%') {
                    Some(p) => p.trim().parse::<f32>().ok().map(|x| x / 100.0),
                    None => v.parse::<f32>().ok(),
                };
                if let Some(n) = n {
                    map.insert(name.to_ascii_lowercase(), n);
                }
            }
        }
        let d = Self::default();
        let g = |k: &str, f: f32| map.get(k).copied().unwrap_or(f);
        Self {
            channels: [g("miles2dchannels", 8.0) as usize, g("miles3dchannels", 21.0) as usize, g("milesstreamingchannels", 3.0) as usize],
            main: g("audiomainvolume", d.main),
            d2: g("audio2dvolume", d.d2),
            d3: g("audio3dvolume", d.d3),
            speech: g("audiospeechvolume", d.speech),
            music: g("audiomusicvolume", d.music),
            mic_height: g("audiomicrophoneheight", d.mic_height),
            centered_radius: g("audioconsidercenteredradius", d.centered_radius),
            drop_timeout: g("audiodroptimeout", d.drop_timeout),
            fade_volume: g("audiofadevolume", d.fade_volume),
            fade_time: g("audiofadetime", d.fade_time),
            recover_time: g("audiorecoverfromfadetime", d.recover_time),
        }
    }
}

/// Where a sound plays (TGC_Audio +1C modes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Place {
    /// Follows an object (actor or organization handle); the position is
    /// read every frame from the world.
    Object(u64),
    /// A fixed native position.
    Location(Vec3),
    /// No position (2-D).
    Global,
}

/// The world reads the manager makes (745D60 object position, the
/// 746300 owner relation, the 746410 voice owner).
pub trait World {
    fn object_position(&self, object: u64) -> Option<Vec3>;
    /// 0 the local player, 1 ally, 2 neutral, 3 enemy; None without kingdom.
    fn relation(&self, kingdom: Option<u32>) -> Option<u32>;
}

/// Camera state for the microphone (745FA0 / 7485B0), native coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub center: Vec3,
    pub eye: Vec3,
    pub zoom: f32,
}

struct Inst {
    id: u32,
    def: Arc<AudioDef>,
    /// 0 2-D, 1 3-D, 2 stream.
    kind: u8,
    place: Place,
    owner: Option<u32>,
    voice: Option<u64>,
    position: Vec3,
    looping: bool,
    /// 0 start, 1 leadin, 2 main, 3 fallout, 4 complete.
    state: u8,
    leadin: Option<String>,
    main: Option<String>,
    fallout: Option<String>,
    current: Option<String>,
    start: f64,
    volume: f32,
    pitch: f32,
    /// 0 fading in, 1 full, 2 fading out, 3 faded out.
    fade: u8,
    fade_t0: f64,
    playing: bool,
    /// Waiting for a streamed part to decode.
    waiting: bool,
    interrupts: u32,
    claimed_by: u32,
    released: bool,
    /// Holds one of the kind's Miles channels.
    channel: bool,
}

/// Decoded sound files (WAV now, MP3 on a loader thread).
pub trait Files {
    fn pcm(&mut self, path: &str) -> Option<Arc<Pcm>>;
    /// None while a streamed file is still decoding.
    fn stream(&mut self, path: &str) -> Option<Option<Arc<Pcm>>>;
}

pub struct Manager {
    pub vars: Vars,
    defs: Arc<std::collections::BTreeMap<String, AudioDef>>,
    shared: HashMap<String, Arc<AudioDef>>,
    mixer: Mixer,
    instances: Vec<Inst>,
    queue: Vec<(u32, f64)>,
    available: [usize; 3],
    voices: HashSet<u64>,
    music: i32,
    speech: i32,
    next_id: u32,
    /// Sequential cursor per IDS (AudioData+74).
    sequential: HashMap<String, String>,
    rng: u32,
    /// Group fades (2-D, 3-D, music, speech): current, target, per-second rate.
    fades: [(f32, f32, f32); 4],
    fading: bool,
    recover_at: Option<f64>,
    mic: Vec3,
    right: Vec3,
    camera: Option<Camera>,
    paused: bool,
    pub missing: Vec<String>,
    /// AA_BEVY_SOUND_LOG: every started part (verification runs).
    pub log: Option<Vec<String>>,
    warned: HashSet<String>,
    now: f64,
    last: f64,
}

fn same_def(a: &Arc<AudioDef>, b: &Arc<AudioDef>) -> bool {
    Arc::ptr_eq(a, b)
}

/// Balance law: each side keeps full gain up to the centre.
fn balance(pan: f32) -> [f32; 2] {
    let pan = if pan.is_finite() { pan.clamp(0.0, 1.0) } else { 0.5 };
    [(2.0 * (1.0 - pan)).min(1.0), (2.0 * pan).min(1.0)]
}

impl Manager {
    pub fn new(vars: Vars, defs: Arc<std::collections::BTreeMap<String, AudioDef>>, mixer: Mixer) -> Self {
        let available = vars.channels;
        Self {
            vars,
            defs,
            shared: HashMap::new(),
            mixer,
            instances: Vec::new(),
            queue: Vec::new(),
            available,
            voices: HashSet::new(),
            music: 0,
            speech: 0,
            next_id: 2,
            sequential: HashMap::new(),
            rng: 0x1234_5678,
            fades: [(1.0, 1.0, 0.0); 4],
            fading: false,
            recover_at: None,
            mic: Vec3::ZERO,
            right: Vec3::X,
            camera: None,
            paused: false,
            missing: Vec::new(),
            log: std::env::var_os("AA_BEVY_SOUND_LOG").map(|_| Vec::new()),
            warned: HashSet::new(),
            now: 0.0,
            last: 0.0,
        }
    }

    pub fn has(&self, ids: &str) -> bool {
        self.defs.contains_key(&ids.to_ascii_lowercase())
    }

    pub fn playing_count(&self) -> usize {
        self.instances.iter().filter(|i| i.playing).count()
    }

    fn random(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }
    fn unit(&mut self) -> f32 {
        (self.random() >> 8) as f32 / (1u32 << 24) as f32
    }
    fn range(&mut self, r: [f32; 2]) -> f32 {
        if r[0] == r[1] { r[0] } else { r[0] + (r[1] - r[0]) * self.unit() }
    }

    fn def(&mut self, ids: &str) -> Option<Arc<AudioDef>> {
        let key = ids.to_ascii_lowercase();
        if let Some(d) = self.shared.get(&key) {
            return Some(d.clone());
        }
        let d = Arc::new(self.defs.get(&key)?.clone());
        self.shared.insert(key, d.clone());
        Some(d)
    }

    fn kind_of(def: &AudioDef, place: Place) -> u8 {
        match def.category {
            0 => {
                if def.control & CONTROL_WORLD != 0 && place != Place::Global { 1 } else { 0 }
            }
            1 | 2 => 2,
            _ => 0,
        }
    }

    /// 7474F0 PlaySound (747630 when `looping`). Returns the instance id.
    pub fn play(&mut self, ids: &str, place: Place, owner: Option<u32>, voice: Option<u64>, looping: bool, world: &impl World) -> Option<u32> {
        let Some(def) = self.def(ids) else {
            if self.warned.insert(ids.to_ascii_lowercase()) {
                self.missing.push(format!("audio {ids}: no definition"));
            }
            return None;
        };
        let kind = Self::kind_of(&def, place);
        let position = match place {
            Place::Location(p) => p,
            Place::Object(o) => world.object_position(o).unwrap_or(Vec3::ZERO),
            Place::Global => Vec3::ZERO,
        };
        let id = self.next_id;
        self.next_id += 1;
        let inst = Inst {
            id,
            def: def.clone(),
            kind,
            place,
            owner,
            voice: None,
            position,
            looping,
            state: 0,
            leadin: None,
            main: None,
            fallout: None,
            current: None,
            start: self.now,
            volume: 0.0,
            pitch: 1.0,
            fade: 0,
            fade_t0: self.now,
            playing: false,
            waiting: false,
            interrupts: 0,
            claimed_by: 0,
            released: false,
            channel: false,
        };
        self.instances.push(inst);
        if self.can_play(id, voice, world) {
            self.queue.push((id, self.now));
            Some(id)
        } else {
            self.release(id);
            self.instances.retain(|i| i.id != id);
            None
        }
    }

    fn inst(&self, id: u32) -> Option<&Inst> {
        self.instances.iter().find(|i| i.id == id)
    }
    fn inst_mut(&mut self, id: u32) -> Option<&mut Inst> {
        self.instances.iter_mut().find(|i| i.id == id)
    }

    /// 7471E0.
    fn can_play(&mut self, id: u32, voice: Option<u64>, world: &impl World) -> bool {
        let Some(inst) = self.inst(id) else { return false };
        // 746300 ShouldPlayLocally.
        let who = inst.def.who;
        if who & 0x21 == 0 {
            if let Some(rel) = world.relation(inst.owner) {
                let ok = match rel {
                    0 => who >> 1 & 1 != 0,
                    1 => who >> 2 & 1 != 0,
                    2 => who >> 3 & 1 != 0,
                    3 => who >> 4 & 1 != 0,
                    _ => false,
                };
                if !ok {
                    return false;
                }
            }
        }
        // 745C20 in range of the microphone.
        if inst.def.control & CONTROL_WORLD != 0 && inst.def.world & WORLD_GLOBAL == 0 && inst.place != Place::Global && self.camera.is_some() {
            if self.mic.distance(inst.position) >= inst.def.range[1] {
                return false;
            }
        }
        // 746410 the object's voice.
        if inst.def.control & CONTROL_VOICE != 0
            && matches!(inst.place, Place::Object(_))
            && let Some(v) = voice
        {
            if self.voices.contains(&v) {
                return false;
            }
            self.voices.insert(v);
            if let Some(i) = self.inst_mut(id) {
                i.voice = Some(v);
            }
        }
        if !self.over_limit(id) && self.reserve_channel(id) {
            return true;
        }
        self.interrupt(id)
    }

    fn list_ids(&self, kind: u8) -> Vec<u32> {
        self.instances.iter().filter(|i| i.playing && i.kind == kind).map(|i| i.id).collect()
    }

    /// 746A80.
    fn over_limit(&mut self, id: u32) -> bool {
        let Some(inst) = self.inst(id) else { return true };
        let (def, kind) = (inst.def.clone(), inst.kind);
        let mut count = match kind {
            2 => {
                if def.category == 1 { self.music } else { self.speech }
            }
            _ => 0,
        };
        count += self.instances.iter().filter(|o| o.playing && o.kind == kind && same_def(&o.def, &def)).count() as i32;
        for (q, _) in &self.queue {
            if let Some(o) = self.inst(*q) {
                if o.def.ids == def.ids || (kind == 2 && o.def.category == def.category) {
                    count += 1;
                }
            }
        }
        if kind == 2 && count > 0 {
            return true;
        }
        if let Some(i) = self.inst_mut(id) {
            i.interrupts = 0;
        }
        count >= def.simultaneous_limit
    }

    /// 7480A0.
    fn reserve_channel(&mut self, id: u32) -> bool {
        let Some(inst) = self.inst(id) else { return false };
        let kind = inst.kind as usize;
        if kind == 2 {
            let busy = if inst.def.category == 1 { self.music } else { self.speech };
            if busy > 0 {
                return false;
            }
        }
        if self.available[kind] > 0 {
            self.available[kind] -= 1;
            if let Some(i) = self.inst_mut(id) {
                i.channel = true;
            }
            true
        } else {
            false
        }
    }

    /// 746F40: the oldest same-IDS INTERRUPT sound (746130), else another
    /// INTERRUPT sound of the category (746C00), gives up its channel.
    fn interrupt(&mut self, id: u32) -> bool {
        let Some(inst) = self.inst(id) else { return false };
        let (def, kind) = (inst.def.clone(), inst.kind);
        let list = self.list_ids(kind);
        let mut count = 0;
        let mut candidate: Option<(u32, f64)> = None;
        for o in list.iter().filter_map(|i| self.inst(*i)) {
            if !same_def(&o.def, &def) {
                continue;
            }
            count += 1;
            if (o.claimed_by == 0 || o.claimed_by == id) && candidate.is_none_or(|(_, s)| o.start < s) {
                candidate = Some((o.id, o.start));
            }
        }
        if def.control & CONTROL_INTERRUPT != 0
            && def.category != 1
            && let Some((c, _)) = candidate
        {
            self.claim(id, c);
            return true;
        }
        if count >= def.simultaneous_limit {
            return false;
        }
        let mut other: Option<(u32, f64)> = None;
        for o in list.iter().filter_map(|i| self.inst(*i)) {
            if same_def(&o.def, &def) || o.def.control & CONTROL_INTERRUPT == 0 || o.def.category != def.category {
                continue;
            }
            if (o.claimed_by == 0 || o.claimed_by == id) && other.is_none_or(|(_, s)| o.start < s) {
                other = Some((o.id, o.start));
            }
        }
        match other {
            Some((c, _)) => {
                self.claim(id, c);
                true
            }
            None => false,
        }
    }
    fn claim(&mut self, id: u32, victim: u32) {
        if let Some(i) = self.inst_mut(id) {
            i.interrupts = victim;
        }
        if let Some(v) = self.inst_mut(victim) {
            v.claimed_by = id;
        }
    }

    fn release(&mut self, id: u32) {
        let Some(i) = self.inst_mut(id) else { return };
        i.released = true;
        if let Some(v) = i.voice.take() {
            self.voices.remove(&v);
        }
    }

    /// 745DD0 leadin / fallout (count - 1 modulo) and main.
    fn pick(&mut self, list: &[String], minus_one: bool) -> Option<String> {
        match list.len() {
            0 => None,
            1 => Some(list[0].clone()),
            n => {
                let m = if minus_one { n - 1 } else { n };
                let r = self.random() as usize % m;
                Some(list[r].clone())
            }
        }
    }
    fn pick_main(&mut self, def: &AudioDef) -> Option<String> {
        let n = def.files.len();
        if n == 0 {
            return None;
        }
        if n == 1 {
            return Some(def.files[0].clone());
        }
        if def.control & CONTROL_SEQUENTIAL != 0 {
            let next = match self.sequential.get(&def.ids) {
                None => def.files[0].clone(),
                Some(last) => {
                    let i = def.files.iter().position(|f| f == last).map_or(-1i64, |i| i as i64);
                    def.files[((i + 1) % n as i64) as usize].clone()
                }
            };
            self.sequential.insert(def.ids.clone(), next.clone());
            return Some(next);
        }
        let r = self.random() as usize % n;
        Some(def.files[r].clone())
    }

    /// 744EB0: Start -> Leadin -> Main (repeated while looping) -> Fallout -> Complete.
    fn advance(&mut self, id: u32) {
        loop {
            let Some(i) = self.inst_mut(id) else { return };
            match i.state {
                0 => i.state = 1,
                1 => i.state = 2,
                2 if !i.looping => i.state = 3,
                2 => {}
                _ => {
                    i.state = 4;
                    i.current = None;
                    return;
                }
            }
            let state = i.state;
            let current = match state {
                1 => i.leadin.clone(),
                2 => match i.main.clone() {
                    Some(m) => Some(m),
                    None => {
                        let def = i.def.clone();
                        self.pick_main(&def)
                    }
                },
                3 => i.fallout.clone(),
                _ => None,
            };
            let Some(i) = self.inst_mut(id) else { return };
            i.current = current.clone();
            if current.is_some() {
                return;
            }
            if i.looping && i.state == 2 {
                i.looping = false;
            }
        }
    }

    /// 747390 Start.
    fn start(&mut self, id: u32, files: &mut impl Files, world: &impl World) {
        // 749340: take the victim's channel.
        let victim = self.inst(id).map(|i| i.interrupts).unwrap_or(0);
        let has_channel = self.inst(id).is_some_and(|i| i.channel);
        if victim != 0 && !has_channel {
            let kind = self.inst(id).map_or(0, |i| i.kind) as usize;
            let fade_out = self.inst(victim).is_some_and(|v| v.kind == 2 && v.def.fade_time_s[1] > 0.0);
            if fade_out && self.available[2] > 0 {
                // 748E40: the stream fades out on its own channel; the new
                // one takes a free stream channel.
                let now = self.now;
                if let Some(v) = self.inst_mut(victim) {
                    v.fade = 2;
                    v.fade_t0 = now;
                }
            } else {
                // 748D60/748DD0: the victim stops and gives its channel.
                self.mixer.stop(victim);
                self.release(victim);
                if self.inst(victim).is_some_and(|v| v.channel) {
                    if let Some(v) = self.inst_mut(victim) {
                        v.channel = false;
                    }
                    self.available[kind] += 1;
                }
            }
            if self.available[kind] > 0 {
                self.available[kind] -= 1;
                if let Some(i) = self.inst_mut(id) {
                    i.channel = true;
                }
            }
        }
        let def = match self.inst(id) {
            Some(i) => i.def.clone(),
            None => return,
        };
        // 745DD0.
        let leadin = self.pick(&def.leadin, true);
        let fallout = self.pick(&def.fallout, true);
        let looping = self.inst(id).is_some_and(|i| i.looping);
        let main = if !looping && def.control & CONTROL_ALL == 0 { self.pick_main(&def) } else { None };
        let pitch = 1.0 + self.range(def.pitch_adjust);
        let low = def.volume_adjust + 1.0;
        let factor = if low == 1.0 { low } else { low + (1.0 - low) * self.unit() };
        let now = self.now;
        if let Some(i) = self.inst_mut(id) {
            i.leadin = leadin;
            i.fallout = fallout;
            i.main = main;
            i.pitch = pitch;
            i.volume = factor * def.volume;
            i.start = now;
            i.fade_t0 = now;
            i.playing = true;
        }
        if def.category == 1 {
            self.music += 1;
        } else if def.category == 2 {
            self.speech += 1;
        }
        if def.control & CONTROL_CAUSEFADE != 0 {
            // 747070: fade the other groups.
            let kind = self.inst(id).map_or(0, |i| i.kind);
            let mask: u8 = match kind {
                0 => 0b1110,
                1 => 0b1101,
                _ => if def.category == 1 { 0b1011 } else { 0b0111 },
            };
            let rate = (1.0 - self.vars.fade_volume) / self.vars.fade_time.max(1e-3);
            for g in 0..4 {
                if mask & (1 << g) != 0 {
                    self.fades[g].1 = self.vars.fade_volume;
                    self.fades[g].2 = rate;
                }
            }
            self.fading = true;
            self.recover_at = None;
        }
        self.advance(id);
        self.play_part(id, files, world);
    }

    /// 74AD10: start the current part on the channel.
    fn play_part(&mut self, id: u32, files: &mut impl Files, world: &impl World) {
        let Some(i) = self.inst(id) else { return };
        let Some(path) = i.current.clone() else {
            self.release(id);
            return;
        };
        let streamed = path.to_ascii_lowercase().ends_with(".mp3");
        let pcm = if streamed {
            match files.stream(&path) {
                Some(Some(p)) => Some(p),
                Some(None) => {
                    if let Some(i) = self.inst_mut(id) {
                        i.waiting = true;
                    }
                    return;
                }
                None => None,
            }
        } else {
            files.pcm(&path)
        };
        let Some(pcm) = pcm else {
            if self.warned.insert(path.clone()) {
                self.missing.push(format!("audio file {path} not found or not decodable"));
            }
            // A missing part ends at once (59B030 raw data null -> end of sample).
            self.end_of_part(id, files, world);
            return;
        };
        let gains = self.gains(id, world).unwrap_or([0.0; 2]);
        let pitch = self.inst(id).map_or(1.0, |i| i.pitch);
        if let Some(i) = self.inst_mut(id) {
            i.waiting = false;
        }
        if let Some(log) = self.log.as_mut()
            && let Some(i) = self.instances.iter().find(|i| i.id == id)
        {
            let d = i.position.distance(self.mic);
            log.push(format!("{} kind {} plays {} gain {:.3}/{:.3} pitch {:.3} at {:.1} m", i.def.ids, i.kind, path, gains[0], gains[1], pitch, d));
        }
        self.mixer.start(id, pcm, gains, pitch);
        if self.paused {
            self.mixer.pause(id, true);
        }
    }

    fn end_of_part(&mut self, id: u32, files: &mut impl Files, world: &impl World) {
        self.advance(id);
        if self.inst(id).is_some_and(|i| i.state != 4 && i.current.is_some()) {
            self.play_part(id, files, world);
        } else {
            self.release(id);
        }
    }

    /// 748540 Stop (or the 748060 fade-out of a stream with a fade-out time).
    pub fn stop(&mut self, id: u32, allow_fade: bool) {
        let Some(i) = self.inst_mut(id) else { return };
        if allow_fade && i.kind == 2 && i.def.fade_time_s[1] > 0.0 && i.fade < 2 {
            i.fade = 2;
            i.fade_t0 = 0.0;
            return;
        }
        self.mixer.stop(id);
        self.release(id);
    }

    /// 7471B0-style stop of every playing instance of an IDS on an object.
    pub fn stop_ids(&mut self, ids: &str, place: Option<Place>) {
        let ids = ids.to_ascii_lowercase();
        let found: Vec<u32> = self.instances.iter().filter(|i| i.def.ids == ids && place.is_none_or(|p| p == i.place) && !i.released).map(|i| i.id).collect();
        for id in found {
            self.stop(id, true);
        }
    }
    /// Stop every sound of a category (0 SOUND, 1 MUSIC, 2 SPEECH).
    pub fn stop_category(&mut self, category: i32) {
        let found: Vec<u32> = self.instances.iter().filter(|i| i.def.category == category && !i.released).map(|i| i.id).collect();
        for id in found {
            self.stop(id, true);
        }
    }
    pub fn stop_all(&mut self) {
        let found: Vec<u32> = self.instances.iter().map(|i| i.id).collect();
        for id in found {
            self.mixer.stop(id);
            self.release(id);
        }
        self.queue.clear();
    }

    /// Pause world sounds while the game is paused (745A30; never MUSIC).
    pub fn set_paused(&mut self, paused: bool) {
        if paused == self.paused {
            return;
        }
        self.paused = paused;
        for i in &self.instances {
            if i.playing && i.def.category != 1 && i.def.who & 1 == 0 {
                self.mixer.pause(i.id, paused);
            }
        }
    }

    fn group(&self, kind: u8, category: i32) -> f32 {
        let (g, user) = match kind {
            0 => (0, self.vars.d2),
            1 => (1, self.vars.d3),
            _ if category == 1 => (2, self.vars.music),
            _ => (3, self.vars.speech),
        };
        self.fades[g].0 * user * self.vars.main
    }

    /// 748790 volume with fades, then 3-D falloff and pan.
    fn gains(&mut self, id: u32, world: &impl World) -> Option<[f32; 2]> {
        let now = self.now;
        let (kind, category, mut volume, fade, fade_t0, fade_times, position, range, place) = {
            let i = self.inst(id)?;
            (i.kind, i.def.category, i.volume, i.fade, i.fade_t0, i.def.fade_time_s, i.position, i.def.range, i.place)
        };
        volume *= self.group(kind, category);
        match fade {
            0 => {
                if fade_times[0] <= 0.0 {
                    self.inst_mut(id)?.fade = 1;
                } else {
                    let t = ((now - fade_t0) as f32 / fade_times[0]).max(0.0);
                    if t < 1.0 {
                        volume *= t;
                    } else {
                        self.inst_mut(id)?.fade = 1;
                    }
                }
            }
            2 => {
                if fade_times[1] <= 0.0 {
                    self.inst_mut(id)?.fade = 3;
                    volume = 0.0;
                } else {
                    if fade_t0 == 0.0 {
                        self.inst_mut(id)?.fade_t0 = now;
                    }
                    let t0 = self.inst(id)?.fade_t0;
                    let t = 1.0 - ((now - t0) as f32 / fade_times[1]);
                    if t > 0.0 {
                        volume *= t;
                    } else {
                        self.inst_mut(id)?.fade = 3;
                        volume = 0.0;
                    }
                }
            }
            3 => volume = 0.0,
            _ => {}
        }
        if kind != 1 {
            let b = balance(0.5);
            return Some([volume * b[0], volume * b[1]]);
        }
        let _ = (place, world);
        // 7485B0: pulled toward the microphone by zoom * centered radius.
        let r = self.camera.map_or(0.0, |c| c.zoom) * self.vars.centered_radius;
        let d = position - self.mic;
        let dist = d.length();
        let target = if dist < r { self.mic } else { position - d / dist.max(1e-6) * r };
        // Miles Fast 2D: full inside min, min/d beyond, silent past max.
        let d = target - self.mic;
        let dist = d.length();
        let (max, min) = (range[1], range[0]);
        let mut gain = volume;
        if dist > max {
            gain = 0.0;
        } else if dist > min && dist > 0.0 {
            gain *= min / dist;
        }
        let lateral = if dist > 1e-4 { d.dot(self.right) / dist } else { 0.0 };
        let b = balance(0.5 + 0.5 * lateral);
        Some([gain * b[0], gain * b[1]])
    }

    /// 747A80 Update.
    pub fn update(&mut self, now: f64, camera: Option<Camera>, files: &mut impl Files, world: &impl World) {
        self.now = now;
        // 745FA0 microphone.
        if let Some(c) = camera {
            self.mic = c.center + Vec3::Z * self.vars.mic_height;
            let mut face = c.center - c.eye;
            face.z = 0.0;
            let face = face.normalize_or_zero();
            // listener up (0,0,-1) in Miles' left-handed space = native right.
            self.right = Vec3::new(face.y, -face.x, 0.0);
        }
        self.camera = camera;
        // End-of-sample callbacks first.
        for id in self.mixer.take_ended() {
            if self.inst(id).is_some_and(|i| !i.released) {
                self.end_of_part(id, files, world);
            }
        }
        // Parts waiting for a stream decode.
        let waiting: Vec<u32> = self.instances.iter().filter(|i| i.waiting && !i.released).map(|i| i.id).collect();
        for id in waiting {
            self.play_part(id, files, world);
        }
        // Queued starts (7477F0), dropped after AudioDropTimeout.
        let queue = std::mem::take(&mut self.queue);
        for (id, at) in queue {
            if (now - at) as f32 > self.vars.drop_timeout {
                self.release(id);
                continue;
            }
            self.start(id, files, world);
        }
        // Group fades (7455E0).
        let dt = (now - self.last_update()) as f32;
        for g in &mut self.fades {
            if g.0 != g.1 {
                let step = g.2.max(1e-3) * dt.max(0.0);
                if g.0 < g.1 { g.0 = (g.0 + step).min(g.1) } else { g.0 = (g.0 - step).max(g.1) }
            }
        }
        // Object sounds follow their actor; gone actors stop them (745D60).
        let mut gone = Vec::new();
        for i in self.instances.iter_mut().filter(|i| i.playing && !i.released) {
            if let Place::Object(o) = i.place {
                match world.object_position(o) {
                    Some(p) => i.position = p,
                    None if i.kind == 1 => gone.push(i.id),
                    None => {}
                }
            }
        }
        for id in gone {
            self.stop(id, false);
        }
        // Volumes and 3-D positions to the mixer.
        let ids: Vec<u32> = self.instances.iter().filter(|i| i.playing && !i.released).map(|i| i.id).collect();
        let mut gains = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(g) = self.gains(id, world) {
                gains.push((id, g));
            }
            if self.inst(id).is_some_and(|i| i.fade == 3) {
                self.mixer.stop(id);
                self.release(id);
            }
        }
        self.mixer.set_gains(&gains);
        // CAUSEFADE recovery.
        if self.fading {
            let causing = self.instances.iter().any(|i| i.playing && !i.released && i.def.control & CONTROL_CAUSEFADE != 0);
            if causing {
                self.recover_at = None;
            } else {
                let at = *self.recover_at.get_or_insert(now);
                if (now - at) as f32 > self.vars.recover_time {
                    self.fading = false;
                    self.recover_at = None;
                    let rate = (1.0 - self.vars.fade_volume) / self.vars.fade_time.max(1e-3);
                    for g in &mut self.fades {
                        g.1 = 1.0;
                        g.2 = rate;
                    }
                }
            }
        }
        // 746830 sweep: released instances give back channels and voices.
        let mut kept = Vec::with_capacity(self.instances.len());
        for i in std::mem::take(&mut self.instances) {
            if i.released {
                if i.channel {
                    self.available[i.kind as usize] += 1;
                }
                if i.playing {
                    self.mixer.stop(i.id);
                    if i.def.category == 1 {
                        self.music -= 1;
                    } else if i.def.category == 2 {
                        self.speech -= 1;
                    }
                }
                if let Some(v) = i.voice {
                    self.voices.remove(&v);
                }
            } else {
                kept.push(i);
            }
        }
        self.instances = kept;
        self.last = now;
    }

    fn last_update(&self) -> f64 {
        self.last
    }
}
