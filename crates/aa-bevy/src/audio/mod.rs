//! The original sound of the battle. Owner: effects and sound.
//! - `manager`: the original audio manager's rules (categories, volumes,
//!   3-D falloff by range, simultaneous limits, interrupts, voices, random
//!   variants, fades) over `mixer`, a Miles-like software mixer played
//!   through Bevy's audio output.
//! - Sound requests come from the FX system (FXC PLAY_SFX of weapons,
//!   deaths, explosions, movers; see crate::effects) through [`SfxQueue`].
//! - Music: at the first battle frame the local player's nation
//!   `game_music` (Nations/*.tgi, e.g. rus_ambient) plays looping as
//!   4F96D0 -> 747630 does: each repeat draws a random file of its list.
//! - Ambient: map-placed audio doodads (Doodads/Audio) play their
//!   ObjectAnimation PlaySFX at random intervals (RANDOM start_min..max),
//!   and disturbable doodads play when units come within affect_radius
//!   (DisturbableDynamicDoodadComponent, recharge_time_min..max).
//! - Unit responses: a new selection plays the selected organization's View
//!   WhenSelected PlaySFX; a local Move / Attack command its WhenCommanded or
//!   WhenCommandedAggressive (attack moves and attacks).
pub mod manager;
pub mod mixer;

use crate::effects::FxData;
use crate::effects::data::{AmbientDef, Fxc};
use crate::session::{AppState, Battle};
use aa_play::{Command, Unit, World};
use bevy::audio::{AddAudioSource, AudioPlayer, PlaybackSettings};
use bevy::prelude::*;
pub use manager::Place;
use manager::{Camera, Manager, Vars};
use mixer::{Mixer, MixerAsset, Pcm};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// A sound request (FXC PLAY_SFX and the game's own plays).
#[derive(Clone, Debug)]
pub struct Sfx {
    pub ids: String,
    pub place: Place,
    /// The owning kingdom (who_flags).
    pub owner: Option<u32>,
    /// The object whose single voice a VOICE sound takes.
    pub voice: Option<u64>,
    pub looping: bool,
}

/// Sounds requested this frame (consumed by the sound update).
#[derive(Resource, Default)]
pub struct SfxQueue {
    pub play: Vec<Sfx>,
    /// Stop requests: (IDS, place).
    pub stop: Vec<(String, Option<Place>)>,
    /// Native positions of the effects' own sound objects (air transports).
    pub objects: HashMap<u64, Vec3>,
}

/// Organization handles for sound objects (actors use their id).
pub fn org_object(org: u32) -> u64 {
    (1u64 << 32) | org as u64
}

/// Handles of the effects' own moving sound objects (not sim actors).
pub fn effect_object(n: u32) -> u64 {
    (2u64 << 32) | n as u64
}

pub struct SoundPlugin;

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_source::<MixerAsset>()
            .init_resource::<SfxQueue>()
            .init_resource::<VoiceSeen>()
            .add_systems(Startup, start_output)
            .add_systems(OnExit(AppState::Battle), leave_battle)
            .add_systems(
                Update,
                (
                    voices.after(crate::input::battle_input).before(crate::session::step_battle),
                    voices_consumed.after(crate::session::step_battle),
                    update.after(crate::effects::EffectsSet).run_if(in_state(AppState::Battle)),
                ),
            );
    }
}

#[derive(Resource, Clone)]
struct Output(Mixer);

fn start_output(mut commands: Commands, mut assets: ResMut<Assets<MixerAsset>>) {
    let mixer = Mixer::default();
    let handle = assets.add(MixerAsset { mixer: mixer.clone() });
    commands.spawn((AudioPlayer::<MixerAsset>(handle), PlaybackSettings::ONCE));
    commands.insert_resource(Output(mixer));
}

/// Sound files below the original Audio mount (Data.rwd, Music.rwd and
/// Speech.rwd all mount over `Audio/`; here their extracted folders).
struct Files {
    roots: Vec<PathBuf>,
    pcm: HashMap<String, Option<Arc<Pcm>>>,
    streams: HashMap<String, Option<Arc<Pcm>>>,
    pending: HashMap<String, ()>,
    tx: mpsc::Sender<(String, Option<Arc<Pcm>>)>,
    rx: Mutex<mpsc::Receiver<(String, Option<Arc<Pcm>>)>>,
}

impl Files {
    fn new(reconstruction: &std::path::Path) -> Self {
        let base = reconstruction.join("assets/original");
        let (tx, rx) = mpsc::channel();
        Self {
            roots: vec![base.join("Data"), base.join("Music"), base.join("Speech")],
            pcm: HashMap::new(),
            streams: HashMap::new(),
            pending: HashMap::new(),
            tx,
            rx: Mutex::new(rx),
        }
    }
    fn locate(&self, path: &str) -> Option<PathBuf> {
        self.roots.iter().find_map(|r| crate::effects::bones::find_file(r, path))
    }
    fn poll(&mut self) {
        let done: Vec<_> = self.rx.lock().map(|rx| rx.try_iter().collect()).unwrap_or_default();
        for (path, pcm) in done {
            self.pending.remove(&path);
            self.streams.insert(path, pcm);
        }
    }
}

impl manager::Files for Files {
    fn pcm(&mut self, path: &str) -> Option<Arc<Pcm>> {
        let key = path.to_ascii_lowercase();
        if let Some(p) = self.pcm.get(&key) {
            return p.clone();
        }
        let pcm = self.locate(path).and_then(|f| std::fs::read(f).ok()).and_then(|b| mixer::decode_wav(&b).ok()).map(Arc::new);
        self.pcm.insert(key, pcm.clone());
        pcm
    }
    fn stream(&mut self, path: &str) -> Option<Option<Arc<Pcm>>> {
        let key = path.to_ascii_lowercase();
        if let Some(p) = self.streams.get(&key) {
            return p.clone().map(Some);
        }
        if !self.pending.contains_key(&key) {
            let Some(file) = self.locate(path) else {
                self.streams.insert(key, None);
                return None;
            };
            self.pending.insert(key.clone(), ());
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let pcm = std::fs::read(&file).ok().and_then(|b| mixer::decode_mp3(b).ok()).map(Arc::new);
                let _ = tx.send((key, pcm));
            });
        }
        Some(None)
    }
}

struct Emitter {
    pos: Vec3,
    def: AmbientDef,
    next_random: f32,
    next_check: f32,
    ready_at: f32,
}

#[derive(Resource)]
struct Sound {
    manager: Manager,
    files: Files,
    emitters: Vec<Emitter>,
    /// Spatial buckets of the emitters that react to units (8 m cells).
    disturbable: HashMap<(i32, i32), Vec<usize>>,
    music_started: bool,
    capturing: bool,
    time: f32,
    rng: u32,
}

impl Sound {
    fn unit(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// The manager's world reads over the battle.
struct BattleWorld<'a> {
    world: &'a World,
    local: u32,
    objects: HashMap<u64, Vec3>,
}

impl manager::World for BattleWorld<'_> {
    fn object_position(&self, object: u64) -> Option<Vec3> {
        if object >> 32 == 2 {
            return self.objects.get(&object).copied();
        }
        if object >> 32 == 1 {
            let org = self.world.orgs.get((object & 0xffff_ffff) as usize)?;
            let member = org.members.iter().find_map(|m| self.world.actor(*m))?;
            return Some(member.pos);
        }
        self.world.actor(object as u32).map(|a| a.pos)
    }
    fn relation(&self, kingdom: Option<u32>) -> Option<u32> {
        let k = kingdom?;
        if k == self.local {
            return Some(0);
        }
        let mine = self.world.kingdoms.get(self.local as usize)?;
        let theirs = self.world.kingdoms.get(k as usize)?;
        Some(if theirs.team == mine.team { 1 } else { 3 })
    }
}

fn read_vars(reconstruction: &std::path::Path) -> Vars {
    let mut texts = Vec::new();
    let overlay = reconstruction.join("native-game/runtime-expanded-v14/Data");
    let data = reconstruction.join("assets/original/Data");
    for name in ["UVars.tgi", "UVars_AA.tgi", "AVars.tgi", "AVars_AA.tgi"] {
        let bytes = std::fs::read(overlay.join(name)).or_else(|_| std::fs::read(data.join(name)));
        if let Ok(b) = bytes {
            texts.push(String::from_utf8_lossy(&b).into_owned());
        }
    }
    Vars::read(&texts)
}

fn leave_battle(mut commands: Commands, sound: Option<ResMut<Sound>>, mut queue: ResMut<SfxQueue>) {
    if let Some(mut s) = sound {
        s.manager.stop_all();
    }
    queue.objects.clear();
    commands.remove_resource::<Sound>();
}

/// The FXC PLAY_SFX entries of a View list as requests.
fn list_sfx(list: &[Fxc], place: Place, owner: Option<u32>, voice: Option<u64>, out: &mut Vec<Sfx>) {
    for f in list.iter().filter(|f| f.kind == "PLAY_SFX" && f.frequency > 0) {
        out.push(Sfx { ids: f.ids.clone(), place, owner, voice, looping: f.looping });
    }
}

#[derive(Default)]
struct VoiceState {
    selection: Vec<Unit>,
}

/// How many of `Battle::pending` the voices have seen (reset after the
/// tick that consumed them).
#[derive(Resource, Default)]
struct VoiceSeen(usize);

/// Unit responses to selection and to the player's commands.
fn voices(battle: Option<Res<Battle>>, data: Option<Res<FxData>>, mut queue: ResMut<SfxQueue>, mut state: Local<VoiceState>, mut seen: ResMut<VoiceSeen>) {
    let (Some(battle), Some(data)) = (battle, data) else { return };
    let world = &battle.world;
    let unit_fx = |u: &Unit| -> Option<(&crate::effects::data::ActorFx, Place, Option<u32>)> {
        match *u {
            Unit::Org(o) => {
                let org = world.orgs.get(o as usize)?;
                let def = world.data.def(org.def);
                Some((data.0.actors.get(&def.id.to_ascii_lowercase())?, Place::Object(org_object(o)), Some(org.kingdom)))
            }
            Unit::Actor(a) => {
                let actor = world.actor(a)?;
                let def = world.data.def(actor.def);
                Some((data.0.actors.get(&def.id.to_ascii_lowercase())?, Place::Object(a as u64), actor.kingdom))
            }
        }
    };
    if battle.selection != state.selection {
        let new = !battle.selection.is_empty() && battle.selection.iter().any(|u| !state.selection.contains(u));
        state.selection = battle.selection.clone();
        if new
            && let Some((fx, place, owner)) = battle.selection.first().and_then(unit_fx)
        {
            let voice = match place {
                Place::Object(o) => Some(o),
                _ => None,
            };
            list_sfx(&fx.selected, place, owner, voice, &mut queue.play);
        }
    }
    if battle.pending.len() < seen.0 {
        seen.0 = 0;
    }
    for c in &battle.pending[seen.0..] {
        let (kingdom, units, aggressive) = match c {
            Command::Move { kingdom, units, attack_move, .. } => (*kingdom, units, *attack_move),
            Command::Attack { kingdom, units, .. } => (*kingdom, units, true),
            _ => continue,
        };
        if kingdom != battle.local {
            continue;
        }
        if let Some((fx, place, owner)) = units.first().and_then(unit_fx) {
            let voice = match place {
                Place::Object(o) => Some(o),
                _ => None,
            };
            let list = if aggressive { &fx.commanded_aggressive } else { &fx.commanded };
            list_sfx(list, place, owner, voice, &mut queue.play);
        }
    }
    seen.0 = battle.pending.len();
}

/// After the tick that consumed the pending commands.
fn voices_consumed(battle: Option<Res<Battle>>, mut seen: ResMut<VoiceSeen>) {
    seen.0 = battle.map_or(0, |b| b.pending.len());
}

#[allow(clippy::too_many_arguments)]
fn update(
    mut commands: Commands,
    time: Res<Time>,
    battle: Option<Res<Battle>>,
    data: Option<Res<FxData>>,
    output: Option<Res<Output>>,
    sound: Option<ResMut<Sound>>,
    mut queue: ResMut<SfxQueue>,
    cameras: Query<&crate::camera::NativeCamera>,
) {
    let (Some(battle), Some(data), Some(output)) = (battle, data, output) else {
        queue.play.clear();
        queue.stop.clear();
        return;
    };
    let Some(mut sound) = sound else {
        let Some(reconstruction) = crate::effects::reconstruction_root() else { return };
        let vars = read_vars(&reconstruction);
        let defs = Arc::new(data.0.audio.clone());
        let manager = Manager::new(vars, defs, output.0.clone());
        let mut s = Sound { manager, files: Files::new(&reconstruction), emitters: Vec::new(), disturbable: HashMap::new(), music_started: false, capturing: false, time: 0.0, rng: 0x2545_F491 };
        // Map-placed audio doodads.
        for p in &battle.world.map.placed {
            if let Some(def) = data.0.ambient.get(&p.thing_id.to_ascii_lowercase()) {
                if def.start[1] <= 0.0 && def.disturb_radius <= 0.0 {
                    continue;
                }
                let first = if def.start[1] > 0.0 { def.start[0] + (def.start[1] - def.start[0]) * s.unit() } else { f32::INFINITY };
                let i = s.emitters.len();
                if def.disturb_radius > 0.0 {
                    s.disturbable.entry(((p.position[0] / 8.0).floor() as i32, (p.position[1] / 8.0).floor() as i32)).or_default().push(i);
                }
                s.emitters.push(Emitter { pos: Vec3::from(p.position), def: def.clone(), next_random: first, next_check: 0.0, ready_at: 0.0 });
            }
        }
        info!("sound: {} definitions, {} ambient emitters", data.0.audio.len(), s.emitters.len());
        commands.insert_resource(s);
        return;
    };
    let sound = &mut *sound;
    let world = &battle.world;
    let bw = BattleWorld { world, local: battle.local, objects: queue.objects.clone() };
    sound.files.poll();
    // 4F96D0 first-tick music: the local player's nation game_music, looping.
    if !sound.music_started {
        sound.music_started = true;
        let nation = world.kingdoms.get(battle.local as usize).map(|k| k.nation.to_ascii_lowercase());
        let music = nation.as_ref().and_then(|n| data.0.nation_music.get(n.trim_start_matches("nation_")).cloned());
        if let Some(ids) = music {
            sound.manager.play(&ids, Place::Global, Some(battle.local), None, true, &bw);
        }
    }
    let dt = if battle.paused { 0.0 } else { time.delta_secs() * battle.speed };
    sound.time += dt;
    let now = sound.time;
    // Ambient doodads.
    let mut ambient = Vec::new();
    for i in 0..sound.emitters.len() {
        if now >= sound.emitters[i].next_random {
            let (lo, hi) = (sound.emitters[i].def.start[0], sound.emitters[i].def.start[1]);
            let u = sound.unit();
            let e = &mut sound.emitters[i];
            e.next_random = now + lo + (hi - lo) * u;
            for s in &e.def.sounds {
                ambient.push(Sfx { ids: s.clone(), place: Place::Location(e.pos), owner: None, voice: None, looping: false });
            }
        }
    }
    // Disturbable doodads: units within affect_radius, every update_time.
    if !sound.disturbable.is_empty() {
        let mut near: Vec<usize> = Vec::new();
        for a in world.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom.is_some()) {
            let (cx, cy) = ((a.pos.x / 8.0).floor() as i32, (a.pos.y / 8.0).floor() as i32);
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let Some(list) = sound.disturbable.get(&(cx + dx, cy + dy)) {
                        for &i in list {
                            let e = &sound.emitters[i];
                            if now >= e.next_check && e.pos.truncate().distance(a.pos.truncate()) <= e.def.disturb_radius {
                                near.push(i);
                            }
                        }
                    }
                }
            }
        }
        near.sort_unstable();
        near.dedup();
        for i in near {
            let u = sound.unit();
            let e = &mut sound.emitters[i];
            e.next_check = now + e.def.disturb_update.max(0.1);
            if now >= e.ready_at {
                let (lo, hi) = (e.def.disturb_recharge[0], e.def.disturb_recharge[1]);
                e.ready_at = now + lo + (hi - lo) * u;
                // The Disturbable action plays the object animation once.
                if let Some(s) = e.def.sounds.first() {
                    ambient.push(Sfx { ids: s.clone(), place: Place::Location(e.pos), owner: None, voice: None, looping: false });
                }
            }
        }
    }
    let camera = cameras.iter().next().map(|c| Camera { center: c.target, eye: c.eye(), zoom: c.zoom });
    sound.manager.set_paused(battle.paused);
    for (ids, place) in queue.stop.drain(..) {
        sound.manager.stop_ids(&ids, place);
    }
    let requests: Vec<Sfx> = queue.play.drain(..).chain(ambient).collect();
    // 7474F0 queues; the update (747A80) starts them this frame.
    let started = std::time::Instant::now();
    for r in requests {
        sound.manager.play(&r.ids, r.place, r.owner, r.voice, r.looping, &bw);
    }
    let real = time.elapsed_secs_f64();
    sound.manager.update(real, camera, &mut sound.files, &bw);
    let micros = started.elapsed().as_secs_f32() * 1e6;
    if std::env::var_os("AA_BEVY_FX_STATS").is_some() && (real % 2.0) < time.delta_secs_f64() {
        info!("sound stats: {} playing, update {:.0} us", sound.manager.playing_count(), micros);
    }
    for m in sound.manager.missing.drain(..) {
        warn!("sound: {m}");
    }
    if let Some(log) = sound.manager.log.as_mut() {
        for l in log.drain(..) {
            info!("sound: {l}");
        }
    }
    // AA_BEVY_SOUND_WAV=file: the first 30 s of the mix (verification).
    if let Some(path) = std::env::var_os("AA_BEVY_SOUND_WAV") {
        if !sound.capturing {
            sound.capturing = true;
            output.0.start_capture(30.0);
        } else if let Some(samples) = output.0.take_capture() {
            let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt();
            let clipped = samples.iter().filter(|s| s.abs() >= 0.999).count();
            info!("sound: captured {} s, peak {peak:.3}, rms {rms:.4}, {clipped} clipped samples", samples.len() / 2 / mixer::OUTPUT_RATE as usize);
            let _ = std::fs::write(path, mixer::wav_bytes(&samples));
        }
    }
}
