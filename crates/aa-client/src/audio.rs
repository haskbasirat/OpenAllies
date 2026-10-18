//! Audio for the Axis & Allies rebuild.
//!
//! * Sound definitions come from the original `Data/Audio/*.tgi` files (`[Audio Template=...]`
//!   blocks) with defaults inherited from `Data/Templates/template_audio*.tgi` (a built-in copy of
//!   the essential template fields is used when those files are absent).
//! * The original game mounts `Data.rwd`, `Music.rwd` and `Speech.rwd` over one virtual `Audio/`
//!   tree, so a definition path such as `Sounds/General/Attack/Grenade01.wav`,
//!   `Music/NationUSA01.mp3` or `Speech/Campaign/Allies01Intro.mp3` is resolved (case-insensitively)
//!   against `assets/original/{Data,Music,Speech}/Audio/`.
//! * WAV effects and unit voice responses are decoded lazily on first use and cached as
//!   `Arc<[i16]>`; MP3 speech and music stream from disk through `rodio::Decoder`.
//! * Everything is mixed into one stereo sub-mixer guarded by a limiter, so dense battles do not
//!   clip. Each playing sound carries a small shared control block (stop / done / gain).
//!
//! The module never panics on missing data or a missing output device: calls are ignored and a
//! warning is logged once per offending id or file.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::BufReader;
use std::num::NonZero;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

use rodio::source::{LimitSettings, SeekError, Source, Zero};
use rodio::{ChannelCount, Decoder, MixerDeviceSink, SampleRate};

/// Global cap on concurrently playing (non-music) voices.
pub const MAX_EFFECT_VOICES: usize = 48;

const STEREO: ChannelCount = match NonZero::new(2) {
    Some(v) => v,
    None => unreachable!(),
};
const FALLBACK_RATE: SampleRate = match NonZero::new(44_100) {
    Some(v) => v,
    None => unreachable!(),
};
/// Frames between reads of the shared control block inside the audio thread.
const CONTROL_PERIOD: u32 = 32;
/// Per-frame one-pole smoothing factor for gain changes (≈4 ms at 48 kHz).
const GAIN_SMOOTH: f32 = 0.005;
/// Fade applied when a voice is cut (interrupt / voice stealing) to avoid clicks.
const STOP_FADE_SECS: f32 = 0.015;
/// `AVars.tgi`: AudioFadeVolume / AudioFadeTime / AudioRecoverFromFadeTime (CAUSEFADE ducking).
const DUCK_LEVEL: f32 = 0.5;
const DUCK_FALL_SECS: f32 = 1.0;
const DUCK_RISE_SECS: f32 = 2.0;
/// Combat is considered over this long after the last `set_combat(true)`.
const COMBAT_HOLD_SECS: f64 = 12.0;
/// Minimum time between combat-triggered music changes, and minimum age of the track replaced.
const COMBAT_SWITCH_COOLDOWN_SECS: f64 = 45.0;
const COMBAT_SWITCH_MIN_TRACK_AGE_SECS: f64 = 20.0;
/// Music gain outside combat (combat ramps it to 1.0).
const CALM_MUSIC_GAIN: f32 = 0.85;
/// Longest WAV accepted through the generic (non-PCM) decode fallback.
const MAX_FALLBACK_DECODE_SECS: usize = 120;

// ---------------------------------------------------------------------------------------------
// Public enums
// ---------------------------------------------------------------------------------------------

/// Game event notifications backed by the original feedback sounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feedback {
    RegimentRecruited,
    BuildingComplete,
    BuildingGained,
    BuildingLost,
    RegimentDestroyed,
    RegimentRouted,
    RegimentEngaged,
    LevelGained,
    ResearchComplete,
    ObjectiveGained,
    ObjectiveCompleted,
    ObjectiveFailed,
    Victory,
    Defeat,
    UnderAttack,
}

impl Feedback {
    pub const ALL: [Feedback; 15] = [
        Feedback::RegimentRecruited,
        Feedback::BuildingComplete,
        Feedback::BuildingGained,
        Feedback::BuildingLost,
        Feedback::RegimentDestroyed,
        Feedback::RegimentRouted,
        Feedback::RegimentEngaged,
        Feedback::LevelGained,
        Feedback::ResearchComplete,
        Feedback::ObjectiveGained,
        Feedback::ObjectiveCompleted,
        Feedback::ObjectiveFailed,
        Feedback::Victory,
        Feedback::Defeat,
        Feedback::UnderAttack,
    ];

    /// Original sound id (`sounds_game_events_aa.tgi`) played for this event.
    pub fn ids(self) -> &'static str {
        match self {
            Feedback::RegimentRecruited => "company_recruited_sfx",
            Feedback::BuildingComplete => "building_construction_complete_sfx",
            Feedback::BuildingGained => "actor_captured_sfx",
            Feedback::BuildingLost => "actor_captured_by_enemy_sfx",
            Feedback::RegimentDestroyed => "actor_lost_sfx",
            Feedback::RegimentRouted => "company_routed_sfx",
            Feedback::RegimentEngaged => "company_engaged_sfx",
            Feedback::LevelGained => "company_experience_gained_level_sfx",
            Feedback::ResearchComplete => "research_complete_sfx",
            Feedback::ObjectiveGained => "objective_gained",
            Feedback::ObjectiveCompleted => "objective_completed",
            Feedback::ObjectiveFailed => "objective_failed",
            Feedback::Victory => "allies_won",
            Feedback::Defeat => "allies_lost",
            Feedback::UnderAttack => "building_under_siege_sfx",
        }
    }

    /// Minimum seconds between two audible notifications of this kind (anti-spam).
    pub fn cooldown_secs(self) -> f64 {
        match self {
            Feedback::UnderAttack => 10.0,
            Feedback::RegimentEngaged => 5.0,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundCategory {
    Sound,
    Speech,
    Music,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Lowest,
    Low,
    Medium,
    High,
    Critical,
}

/// `control_flags` bits.
pub mod control {
    pub const INTERRUPT: u32 = 1 << 0;
    pub const SEQUENTIAL: u32 = 1 << 1;
    pub const ALL: u32 = 1 << 2;
    pub const VOICE: u32 = 1 << 3;
    pub const WORLD: u32 = 1 << 4;
    pub const CAUSEFADE: u32 = 1 << 5;
    pub const TESTSOUND: u32 = 1 << 6;
}

/// One resolved `[Audio]` definition.
#[derive(Clone, Debug)]
pub struct SoundDef {
    pub ids: String,
    /// Template named in the block header (e.g. `AudioAttack`).
    pub template: String,
    /// `.tgi` file the definition came from.
    pub source: String,
    pub category: SoundCategory,
    /// Existing files (variants), in definition order.
    pub files: Vec<PathBuf>,
    pub leadin: Vec<PathBuf>,
    pub fallout: Vec<PathBuf>,
    /// Referenced names that could not be found on disk.
    pub missing: Vec<String>,
    pub priority: Priority,
    pub control: u32,
    pub world_global: bool,
    pub world_fogged: bool,
    /// `range = falloff, max` (original units).
    pub range: (f32, f32),
    pub simultaneous_limit: u32,
    /// Full-loudness volume as a fraction (80% -> 0.8). Values above 1 are clamped at playback.
    pub volume: f32,
    /// Random volume reduction, `(-1, 0]`.
    pub volume_adjust: f32,
    /// Random pitch deltas as fractions, e.g. `(-0.01, 0.01)`.
    pub pitch_adjust: (f32, f32),
    pub delay_ms: (f32, f32),
    pub fade_time_s: (f32, f32),
}

impl SoundDef {
    pub fn has(&self, flag: u32) -> bool {
        self.control & flag != 0
    }
}

// ---------------------------------------------------------------------------------------------
// TGI parsing
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct Section {
    /// Lower-case first word of the header (`audio`, `template`, ...).
    kind: String,
    /// Second bare word (template name).
    name: Option<String>,
    /// `Template=` / `Inherit=` value.
    parent: Option<String>,
    /// Lower-case key, raw (unquoted) value.
    fields: Vec<(String, String)>,
}

impl Section {
    fn own(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// Removes `;`, `//` and `/* */` comments (outside quotes) from one line.
fn clean_line(raw: &str, in_block: &mut bool) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut in_quote = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if *in_block {
            if c == '*' && next == Some('/') {
                *in_block = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if in_quote {
            out.push(c);
            if c == '"' {
                in_quote = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => {
                in_quote = true;
                out.push(c);
            }
            ';' => break,
            '/' if next == Some('*') => {
                *in_block = true;
                i += 2;
                continue;
            }
            '/' if next == Some('/') => break,
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

fn find_unquoted(s: &str, needle: char) -> Option<usize> {
    let mut in_quote = false;
    for (i, c) in s.char_indices() {
        if c == '"' {
            in_quote = !in_quote;
        } else if c == needle && !in_quote {
            return Some(i);
        }
    }
    None
}

fn unquote(v: &str) -> String {
    let v = v.trim();
    let v = v.strip_prefix('"').unwrap_or(v);
    let v = v.strip_suffix('"').unwrap_or(v);
    v.trim().to_string()
}

fn parse_header(inner: &str) -> Section {
    let raw: Vec<&str> = inner.split_whitespace().collect();
    let mut toks: Vec<String> = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let t = raw[i];
        if t.starts_with('=') && !toks.is_empty() {
            let mut merged = toks.pop().unwrap_or_default();
            merged.push_str(t);
            if t.ends_with('=') && i + 1 < raw.len() {
                merged.push_str(raw[i + 1]);
                i += 1;
            }
            toks.push(merged);
        } else if t.ends_with('=') && i + 1 < raw.len() {
            toks.push(format!("{t}{}", raw[i + 1]));
            i += 1;
        } else {
            toks.push(t.to_string());
        }
        i += 1;
    }
    let mut section = Section::default();
    let mut iter = toks.into_iter();
    section.kind = iter.next().unwrap_or_default().to_ascii_lowercase();
    for tok in iter {
        if let Some((k, v)) = tok.split_once('=') {
            if k.eq_ignore_ascii_case("template") || k.eq_ignore_ascii_case("inherit") {
                section.parent = Some(v.to_string());
            }
        } else if section.name.is_none() {
            section.name = Some(tok);
        }
    }
    section
}

/// Parses the TimeGate `.tgi` text format into top-level sections. Nested sections
/// (`[Subtitle] { ... }`, brace-less `[Cost]` lists) are skipped; brace-less top-level
/// sections (`[Template AAMusic Template=AudioMusic]` + `volume = 100%`) are supported.
fn parse_tgi(text: &str) -> Vec<Section> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut out = Vec::new();
    let mut cur: Option<Section> = None;
    let mut depth: i32 = 0;
    let mut nested_braceless = false;
    let mut in_block = false;
    for raw in text.lines() {
        let cleaned = clean_line(raw, &mut in_block);
        let mut line = cleaned.trim();
        while !line.is_empty() {
            if let Some(rest) = line.strip_prefix('{') {
                depth += 1;
                nested_braceless = false;
                line = rest.trim_start();
                continue;
            }
            if let Some(rest) = line.strip_prefix('}') {
                depth -= 1;
                nested_braceless = false;
                if depth <= 0 {
                    depth = 0;
                    if let Some(s) = cur.take() {
                        out.push(s);
                    }
                }
                line = rest.trim_start();
                continue;
            }
            if let Some(after) = line.strip_prefix('[') {
                let (inner, rest) = match after.find(']') {
                    Some(c) => (&after[..c], &after[c + 1..]),
                    None => (after, ""),
                };
                if depth == 0 {
                    if let Some(s) = cur.take() {
                        out.push(s);
                    }
                    cur = Some(parse_header(inner));
                } else if depth == 1 {
                    nested_braceless = true;
                }
                line = rest.trim_start();
                continue;
            }
            let split = find_unquoted(line, '}').unwrap_or(line.len());
            let field = &line[..split];
            let record = match depth {
                0 => true,
                1 => !nested_braceless,
                _ => false,
            };
            if record
                && let Some(section) = cur.as_mut()
                && let Some((lhs, rhs)) = field.split_once('=')
                && let Some(key) = lhs.split_whitespace().last()
            {
                section
                    .fields
                    .push((key.to_ascii_lowercase(), unquote(rhs)));
            }
            line = line[split..].trim_start();
        }
    }
    if let Some(s) = cur.take() {
        out.push(s);
    }
    out
}

fn is_null(v: &str) -> bool {
    let v = v.trim();
    v.is_empty() || v.eq_ignore_ascii_case("null")
}

fn parse_list(v: Option<&str>) -> Vec<String> {
    match v {
        Some(v) if !is_null(v) => v.split(',').map(unquote).filter(|s| !is_null(s)).collect(),
        _ => Vec::new(),
    }
}

/// `80%` -> 0.8, `0.5` -> 0.5.
fn parse_fraction(v: &str) -> Option<f32> {
    let v = v.trim();
    let value = match v.strip_suffix('%') {
        Some(p) => p.trim().parse::<f32>().ok()? / 100.0,
        None => v.parse::<f32>().ok()?,
    };
    value.is_finite().then_some(value)
}

fn parse_pair(v: Option<&str>) -> Option<(f32, f32)> {
    let v = v?;
    let mut parts = v.split(',').map(parse_fraction);
    let a = parts.next()??;
    let b = parts.next().flatten().unwrap_or(a);
    Some((a, b))
}

fn parse_flags(v: Option<&str>) -> u32 {
    let mut bits = 0;
    for flag in parse_list(v) {
        bits |= match flag.to_ascii_uppercase().as_str() {
            "INTERRUPT" => control::INTERRUPT,
            "SEQUENTIAL" => control::SEQUENTIAL,
            "ALL" => control::ALL,
            "VOICE" => control::VOICE,
            "WORLD" => control::WORLD,
            "CAUSEFADE" => control::CAUSEFADE,
            "TESTSOUND" => control::TESTSOUND,
            _ => 0,
        };
    }
    bits
}

fn parse_priority(v: Option<&str>) -> Priority {
    match v.map(|s| s.trim().to_ascii_uppercase()).as_deref() {
        Some("LOWEST") => Priority::Lowest,
        Some("LOW") => Priority::Low,
        Some("HIGH") => Priority::High,
        Some("CRITICAL") => Priority::Critical,
        _ => Priority::Medium,
    }
}

/// Essential fields of `template_audio.tgi` / `template_audio_aa.tgi`, used when the template
/// files are missing. The real files (when present) are parsed afterwards and take precedence.
const BUILTIN_TEMPLATES: &str = r#"
[Template Audio]
{
    category = SOUND
    files = NULL
    priority = MEDIUM
    control_flags = NULL
    world_flags = NULL
    range = 4, 64
    simultaneous_limit = 2
    volume = 80%
    delay_ms = 0, 0
    pitch_adjust = 0%, 0%
    volume_adjust = 0%
    fade_time_s = 0, 0
}
[Template AudioSound inherit=Audio]
{
    category = SOUND
}
[Template AudioSpeech inherit=Audio]
{
    category = SPEECH
    control_flags = INTERRUPT, CAUSEFADE
}
[Template AudioMusic inherit=Audio]
{
    category = MUSIC
    control_flags = ALL, INTERRUPT
    simultaneous_limit = 1
}
[Template AudioInterfaceSound inherit=AudioSound]
{
    priority = CRITICAL
    control_flags = INTERRUPT
    volume = 90%
}
[Template AudioAmbient Inherit=AudioSound]
{
    priority = LOW
    control_flags = WORLD
    world_flags = FOGGED
    volume = 50%
    pitch_adjust = -1%, 1%
}
[Template AudioAttack Inherit=AudioSound]
{
    priority = HIGH
    control_flags = WORLD
    world_flags = FOGGED
    pitch_adjust = -1%, 1%
}
[Template AudioDeath Inherit=AudioSound]
{
    priority = HIGH
    control_flags = WORLD
    world_flags = FOGGED
    pitch_adjust = -1%, 1%
}
[Template AudioCallout Inherit=AudioSound]
{
    priority = HIGH
    control_flags = VOICE
    simultaneous_limit = 1
    volume = 70%
}
[Template AudioCutscene Inherit=AudioSpeech]
{
    priority = CRITICAL
    control_flags = INTERRUPT, VOICE, CAUSEFADE
    simultaneous_limit = 1
    volume = 100%
}
[Template AudioFeedback Inherit=AudioSound]
{
    priority = HIGH
    simultaneous_limit = 1
    volume = 90%
    pitch_adjust = -1%, 1%
}
[Template AudioInterface Inherit=AudioInterfaceSound]
{
    priority = CRITICAL
    control_flags = INTERRUPT
    volume = 80%
    pitch_adjust = -5%, 5%
}
[Template AudioSpecialOp Inherit=AudioSound]
{
    simultaneous_limit = 1
    priority = HIGH
    control_flags = WORLD
    volume = 200%
}
[Template AudioCalloutGeneral Inherit=AudioSound]
{
    priority = LOW
    control_flags = VOICE, SEQUENTIAL
    simultaneous_limit = 1
    volume = 90%
}
"#;

// ---------------------------------------------------------------------------------------------
// Sound library (definitions + file index)
// ---------------------------------------------------------------------------------------------

/// Locates `assets/original` (the directory holding `Data/Audio`) from a reconstruction root.
fn original_dir(root: &Path) -> Option<PathBuf> {
    [
        root.join("assets").join("original"),
        root.to_path_buf(),
        root.join("original"),
    ]
    .into_iter()
    .find(|p| p.join("Data").join("Audio").is_dir())
}

fn normalize_rel(name: &str) -> String {
    let mut key = name.trim().replace('\\', "/").to_ascii_lowercase();
    while let Some(rest) = key.strip_prefix('/') {
        key = rest.to_string();
    }
    if let Some(rest) = key.strip_prefix("audio/") {
        key = rest.to_string();
    }
    key
}

#[derive(Default)]
pub struct SoundLibrary {
    defs: Vec<SoundDef>,
    by_id: HashMap<String, usize>,
    /// Lower-case path relative to the virtual `Audio/` root -> file on disk.
    index: HashMap<String, PathBuf>,
    roots: Vec<PathBuf>,
}

impl SoundLibrary {
    /// Loads every definition below `root` (the reconstruction directory). Never fails: a
    /// missing data directory yields an empty library.
    pub fn load(root: &Path) -> Self {
        let mut lib = SoundLibrary::default();
        let Some(orig) = original_dir(root) else {
            log::warn!(
                "audio: no original audio data under {} (expected assets/original/Data/Audio)",
                root.display()
            );
            return lib;
        };
        lib.roots = ["Data", "Music", "Speech"]
            .iter()
            .map(|r| orig.join(r).join("Audio"))
            .filter(|p| p.is_dir())
            .collect();
        lib.build_index();

        let mut templates: HashMap<String, Section> = HashMap::new();
        let add_templates = |sections: &[Section], templates: &mut HashMap<String, Section>| {
            for s in sections.iter().filter(|s| s.kind == "template") {
                if let Some(name) = &s.name {
                    templates.insert(name.to_ascii_lowercase(), s.clone());
                }
            }
        };
        add_templates(&parse_tgi(BUILTIN_TEMPLATES), &mut templates);
        for path in sorted_tgi_files(&orig.join("Data").join("Templates"))
            .into_iter()
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.to_ascii_lowercase().starts_with("template_audio"))
            })
        {
            if let Some(text) = read_text(&path) {
                add_templates(&parse_tgi(&text), &mut templates);
            }
        }

        let audio_files: Vec<(String, Vec<Section>)> =
            sorted_tgi_files(&orig.join("Data").join("Audio"))
                .into_iter()
                .filter_map(|p| {
                    let text = read_text(&p)?;
                    let name = p.file_name()?.to_string_lossy().into_owned();
                    Some((name, parse_tgi(&text)))
                })
                .collect();
        for (_, sections) in &audio_files {
            add_templates(sections, &mut templates);
        }

        let mut missing_total = 0usize;
        for (file, sections) in &audio_files {
            for section in sections.iter().filter(|s| s.kind == "audio") {
                if let Some(def) = lib.build_def(section, &templates, file) {
                    missing_total += def.missing.len();
                    let key = def.ids.to_ascii_lowercase();
                    if lib.by_id.contains_key(&key) {
                        log::debug!("audio: duplicate sound id {} in {file}", def.ids);
                    }
                    lib.by_id.insert(key, lib.defs.len());
                    lib.defs.push(def);
                }
            }
        }
        log::info!(
            "audio: {} sound ids from {} definition files, {} indexed files, {} unresolved references",
            lib.defs.len(),
            audio_files.len(),
            lib.index.len(),
            missing_total
        );
        lib
    }

    fn build_index(&mut self) {
        for root in &self.roots {
            let mut stack = vec![root.clone()];
            while let Some(dir) = stack.pop() {
                let Ok(entries) = fs::read_dir(&dir) else {
                    continue;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    let Ok(kind) = entry.file_type() else {
                        continue;
                    };
                    if kind.is_dir() {
                        stack.push(path);
                    } else if let Ok(rel) = path.strip_prefix(root) {
                        let key = rel
                            .components()
                            .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
                            .collect::<Vec<_>>()
                            .join("/");
                        self.index.entry(key).or_insert(path);
                    }
                }
            }
        }
    }

    /// Resolves a definition path (relative to the virtual `Audio/` root) to a file on disk.
    /// Falls back to a unique same-directory file whose name extends the requested stem
    /// (the original data references `CinematicAxisMoscowFall.mp3` for `...Falls.mp3`).
    pub fn resolve(&self, name: &str) -> Option<PathBuf> {
        let key = normalize_rel(name);
        if let Some(p) = self.index.get(&key) {
            return Some(p.clone());
        }
        let (dir, file) = key.rsplit_once('/').unwrap_or(("", key.as_str()));
        let (stem, ext) = file.rsplit_once('.')?;
        if stem.len() < 4 {
            return None;
        }
        let mut found: Option<&PathBuf> = None;
        for (k, p) in &self.index {
            let (kdir, kfile) = k.rsplit_once('/').unwrap_or(("", k.as_str()));
            if kdir != dir {
                continue;
            }
            let Some((kstem, kext)) = kfile.rsplit_once('.') else {
                continue;
            };
            if kext == ext && kstem.starts_with(stem) && kstem.len() <= stem.len() + 2 {
                if found.is_some() {
                    return None;
                }
                found = Some(p);
            }
        }
        found.cloned()
    }

    fn build_def(
        &self,
        section: &Section,
        templates: &HashMap<String, Section>,
        source: &str,
    ) -> Option<SoundDef> {
        let field = |key: &str| -> Option<&str> {
            if let Some(v) = section.own(key) {
                return Some(v);
            }
            let mut parent = section.parent.as_deref();
            for _ in 0..16 {
                let t = templates.get(&parent?.to_ascii_lowercase())?;
                if let Some(v) = t.own(key) {
                    return Some(v);
                }
                parent = t.parent.as_deref();
            }
            None
        };
        let ids = field("ids")
            .map(str::trim)
            .filter(|s| !is_null(s))?
            .to_string();
        let template = section.parent.clone().unwrap_or_default();
        let category = match field("category")
            .map(|c| c.trim().to_ascii_uppercase())
            .as_deref()
        {
            Some("MUSIC") => SoundCategory::Music,
            Some("SPEECH") => SoundCategory::Speech,
            Some(_) => SoundCategory::Sound,
            None => {
                let t = template.to_ascii_lowercase();
                if t.contains("music") {
                    SoundCategory::Music
                } else if t.contains("speech") || t.contains("cutscene") {
                    SoundCategory::Speech
                } else {
                    SoundCategory::Sound
                }
            }
        };
        let mut missing = Vec::new();
        let mut resolve_all = |key: &str| -> Vec<PathBuf> {
            parse_list(field(key))
                .into_iter()
                .filter_map(|name| {
                    let found = self.resolve(&name);
                    if found.is_none() {
                        missing.push(name);
                    }
                    found
                })
                .collect()
        };
        let files = resolve_all("files");
        let leadin = resolve_all("leadin");
        let fallout = resolve_all("fallout");
        let world = parse_list(field("world_flags"))
            .into_iter()
            .map(|f| f.to_ascii_uppercase())
            .collect::<Vec<_>>();
        let range = parse_pair(field("range")).unwrap_or((4.0, 64.0));
        let pitch = parse_pair(field("pitch_adjust")).unwrap_or((0.0, 0.0));
        let delay = parse_pair(field("delay_ms")).unwrap_or((0.0, 0.0));
        let fade = parse_pair(field("fade_time_s")).unwrap_or((0.0, 0.0));
        Some(SoundDef {
            ids,
            template,
            source: source.to_string(),
            category,
            files,
            leadin,
            fallout,
            missing,
            priority: parse_priority(field("priority")),
            control: parse_flags(field("control_flags")),
            world_global: world.iter().any(|f| f == "GLOBAL"),
            world_fogged: world.iter().any(|f| f == "FOGGED"),
            range: (range.0.max(0.01), range.1.max(0.01)),
            simultaneous_limit: field("simultaneous_limit")
                .and_then(|v| v.trim().parse::<u32>().ok())
                .unwrap_or(2)
                .max(1),
            volume: field("volume")
                .and_then(parse_fraction)
                .unwrap_or(0.8)
                .max(0.0),
            volume_adjust: field("volume_adjust")
                .and_then(parse_fraction)
                .unwrap_or(0.0)
                .clamp(-0.99, 0.0),
            pitch_adjust: (pitch.0.clamp(-0.9, 1.0), pitch.1.clamp(-0.9, 1.0)),
            delay_ms: (delay.0.clamp(0.0, 5000.0), delay.1.clamp(0.0, 5000.0)),
            fade_time_s: (fade.0.clamp(0.0, 30.0), fade.1.clamp(0.0, 30.0)),
        })
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    pub fn index_of(&self, ids: &str) -> Option<usize> {
        self.by_id.get(&ids.trim().to_ascii_lowercase()).copied()
    }

    pub fn get(&self, ids: &str) -> Option<&SoundDef> {
        self.index_of(ids).map(|i| &self.defs[i])
    }

    pub fn defs(&self) -> &[SoundDef] {
        &self.defs
    }

    /// Number of files found under the virtual `Audio/` roots.
    pub fn indexed_files(&self) -> usize {
        self.index.len()
    }
}

fn sorted_tgi_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.is_file()
                        && p.extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|e| e.eq_ignore_ascii_case("tgi"))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|p| p.to_string_lossy().to_ascii_lowercase());
    files
}

fn read_text(path: &Path) -> Option<String> {
    match fs::read(path) {
        Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        Err(e) => {
            log::warn!("audio: cannot read {}: {e}", path.display());
            None
        }
    }
}

/// Maps a nation code (or common alias, or an explicit `xxx_ambient` id) to its music id.
pub fn nation_music_ids(nation: &str) -> String {
    let n = nation.trim().to_ascii_lowercase();
    if n.ends_with("_ambient") {
        return n;
    }
    let code = match n.as_str() {
        "us" | "america" | "american" | "united states" => "usa",
        "uk" | "gb" | "britain" | "british" | "england" | "united kingdom" => "gbr",
        "germany" | "german" | "deu" => "ger",
        "russia" | "russian" | "ussr" | "soviet" | "sov" => "rus",
        "japan" | "japanese" | "jap" => "jpn",
        other => other,
    };
    format!("{code}_ambient")
}

// ---------------------------------------------------------------------------------------------
// Decoded clips
// ---------------------------------------------------------------------------------------------

/// Decoded PCM (interleaved) kept in the cache.
#[derive(Debug)]
pub struct Clip {
    pub samples: Arc<[i16]>,
    pub channels: u16,
    pub rate: u32,
}

impl Clip {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }

    pub fn duration_secs(&self) -> f32 {
        self.frames() as f32 / self.rate.max(1) as f32
    }
}

fn le_u16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

fn le_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *b.get(at)?,
        *b.get(at + 1)?,
        *b.get(at + 2)?,
        *b.get(at + 3)?,
    ]))
}

/// Minimal RIFF/WAVE reader for PCM (8/16/24/32-bit) and 32-bit float data.
fn decode_wav(bytes: &[u8]) -> Result<Clip, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let mut pos = 12usize;
    let mut fmt: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = le_u32(bytes, pos + 4).unwrap_or(0) as usize;
        let start = pos + 8;
        let end = start.saturating_add(size).min(bytes.len());
        let body = &bytes[start..end];
        if id == b"fmt " && body.len() >= 16 {
            let mut tag = le_u16(body, 0).unwrap_or(0);
            if tag == 0xFFFE {
                tag = le_u16(body, 24).unwrap_or(0);
            }
            fmt = Some((
                tag,
                le_u16(body, 2).unwrap_or(0),
                le_u32(body, 4).unwrap_or(0),
                le_u16(body, 14).unwrap_or(0),
            ));
        } else if id == b"data" {
            data = Some(body);
        }
        if fmt.is_some() && data.is_some() {
            break;
        }
        pos = start.saturating_add(size).saturating_add(size & 1);
    }
    let (tag, channels, rate, bits) = fmt.ok_or("missing fmt chunk")?;
    let data = data.ok_or("missing data chunk")?;
    if channels == 0 || channels > 8 || rate == 0 {
        return Err(format!("bad format: {channels} channels at {rate} Hz"));
    }
    let samples: Vec<i16> = match (tag, bits) {
        (1, 16) => data
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect(),
        (1, 8) => data.iter().map(|&b| ((b as i16) - 128) << 8).collect(),
        (1, 24) => data
            .chunks_exact(3)
            .map(|b| i16::from_le_bytes([b[1], b[2]]))
            .collect(),
        (1, 32) => data
            .chunks_exact(4)
            .map(|b| i16::from_le_bytes([b[2], b[3]]))
            .collect(),
        (3, 32) => data
            .chunks_exact(4)
            .map(|b| {
                let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                (v.clamp(-1.0, 1.0) * 32767.0) as i16
            })
            .collect(),
        _ => {
            return Err(format!(
                "unsupported WAV encoding (format {tag}, {bits} bits)"
            ));
        }
    };
    let usable = samples.len() - samples.len() % channels as usize;
    let mut samples = samples;
    samples.truncate(usable);
    if samples.is_empty() {
        return Err("no audio data".into());
    }
    Ok(Clip {
        samples: samples.into(),
        channels,
        rate,
    })
}

/// Generic decode through rodio (any format the enabled features support).
fn decode_with_rodio(path: &Path) -> Result<Clip, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let decoder = Decoder::try_from(file).map_err(|e| e.to_string())?;
    let channels = decoder.channels().get();
    let rate = decoder.sample_rate().get();
    let limit = MAX_FALLBACK_DECODE_SECS * rate as usize * channels as usize;
    let mut samples: Vec<i16> = decoder
        .take(limit)
        .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();
    samples.truncate(samples.len() - samples.len() % channels.max(1) as usize);
    if samples.is_empty() {
        return Err("no audio data".into());
    }
    Ok(Clip {
        samples: samples.into(),
        channels,
        rate,
    })
}

pub fn load_clip(path: &Path) -> Result<Clip, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    match decode_wav(&bytes) {
        Ok(clip) => Ok(clip),
        Err(wav_err) => decode_with_rodio(path).map_err(|e| format!("{wav_err}; {e}")),
    }
}

// ---------------------------------------------------------------------------------------------
// Shared voice control and sources
// ---------------------------------------------------------------------------------------------

struct AtomicF32(AtomicU32);

impl AtomicF32 {
    fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    fn load(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
    fn store(&self, v: f32) {
        self.0.store(v.to_bits(), Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bus {
    Effects = 0,
    Voice = 1,
    Music = 2,
}

struct Buses([AtomicF32; 3]);

impl Buses {
    fn new() -> Self {
        Self([
            AtomicF32::new(1.0),
            AtomicF32::new(1.0),
            AtomicF32::new(1.0),
        ])
    }
    fn get(&self, bus: Bus) -> f32 {
        self.0[bus as usize].load()
    }
    fn set(&self, bus: Bus, v: f32) {
        self.0[bus as usize].store(v);
    }
}

/// Control block shared between the UI thread and a playing source.
struct VoiceCtl {
    stop: AtomicBool,
    done: AtomicBool,
    gain: AtomicF32,
    frames: AtomicU64,
}

impl VoiceCtl {
    fn new(gain: f32) -> Arc<Self> {
        Arc::new(Self {
            stop: AtomicBool::new(false),
            done: AtomicBool::new(false),
            gain: AtomicF32::new(gain),
            frames: AtomicU64::new(0),
        })
    }
    fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }
}

/// Plays a cached clip as stereo at the mixer rate: linear-interpolated resampling (which also
/// applies the random pitch), constant gain, left/right balance, bus gain and stop fade.
struct ClipSource {
    clip: Arc<Clip>,
    frames: usize,
    pos: f64,
    step: f64,
    out_rate: SampleRate,
    pan: [f32; 2],
    base_gain: f32,
    ctl: Arc<VoiceCtl>,
    buses: Arc<Buses>,
    bus: Bus,
    delay_frames: u32,
    pending_right: Option<f32>,
    gain: f32,
    target: f32,
    countdown: u32,
    fade: Option<u32>,
    fade_len: u32,
    rendered: u64,
    finished: bool,
}

impl ClipSource {
    #[allow(clippy::too_many_arguments)]
    fn new(
        clip: Arc<Clip>,
        out_rate: SampleRate,
        pitch: f32,
        pan: [f32; 2],
        base_gain: f32,
        delay_secs: f32,
        ctl: Arc<VoiceCtl>,
        buses: Arc<Buses>,
        bus: Bus,
    ) -> Self {
        let rate = out_rate.get() as f64;
        let step = clip.rate.max(1) as f64 * pitch.clamp(0.25, 4.0) as f64 / rate;
        let target = buses.get(bus) * ctl.gain.load();
        Self {
            frames: clip.frames(),
            clip,
            pos: 0.0,
            step,
            out_rate,
            pan,
            base_gain,
            ctl,
            buses,
            bus,
            delay_frames: (delay_secs.max(0.0) as f64 * rate) as u32,
            pending_right: None,
            gain: target,
            target,
            countdown: 0,
            fade: None,
            fade_len: ((STOP_FADE_SECS as f64 * rate) as u32).max(1),
            rendered: 0,
            finished: false,
        }
    }

    #[inline]
    fn frame(&self, i: usize) -> (f32, f32) {
        let s = &self.clip.samples;
        if self.clip.channels == 1 {
            let v = s[i] as f32 / 32768.0;
            (v, v)
        } else {
            let base = i * self.clip.channels as usize;
            (s[base] as f32 / 32768.0, s[base + 1] as f32 / 32768.0)
        }
    }

    fn finish(&mut self) {
        self.finished = true;
        self.ctl.frames.store(self.rendered, Ordering::Relaxed);
        self.ctl.done.store(true, Ordering::Release);
    }
}

impl Iterator for ClipSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if let Some(r) = self.pending_right.take() {
            return Some(r);
        }
        if self.finished {
            return None;
        }
        if self.countdown == 0 {
            self.countdown = CONTROL_PERIOD;
            self.target = self.buses.get(self.bus) * self.ctl.gain.load();
            if self.fade.is_none() && self.ctl.stop.load(Ordering::Relaxed) {
                self.fade = Some(self.fade_len);
            }
            self.ctl.frames.store(self.rendered, Ordering::Relaxed);
        }
        self.countdown -= 1;
        self.rendered += 1;
        if self.delay_frames > 0 {
            if self.fade.is_some() {
                self.finish();
                return None;
            }
            self.delay_frames -= 1;
            self.pending_right = Some(0.0);
            return Some(0.0);
        }
        let i = self.pos as usize;
        if i >= self.frames {
            self.finish();
            return None;
        }
        let frac = (self.pos - i as f64) as f32;
        let (l0, r0) = self.frame(i);
        let (l1, r1) = if i + 1 < self.frames {
            self.frame(i + 1)
        } else {
            (0.0, 0.0)
        };
        let l = l0 + (l1 - l0) * frac;
        let r = r0 + (r1 - r0) * frac;
        self.pos += self.step;
        self.gain += (self.target - self.gain) * GAIN_SMOOTH;
        let mut g = self.gain * self.base_gain;
        if let Some(f) = self.fade {
            if f == 0 {
                self.finish();
                return None;
            }
            g *= f as f32 / self.fade_len as f32;
            self.fade = Some(f - 1);
        }
        self.pending_right = Some(r * g * self.pan[1]);
        Some(l * g * self.pan[0])
    }
}

impl Source for ClipSource {
    fn current_span_len(&self) -> Option<usize> {
        if self.finished && self.pending_right.is_none() {
            Some(0)
        } else {
            None
        }
    }
    fn channels(&self) -> ChannelCount {
        STEREO
    }
    fn sample_rate(&self) -> SampleRate {
        self.out_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        let secs = self.frames as f64 / self.step.max(1e-6) / self.out_rate.get() as f64;
        Some(Duration::from_secs_f64(secs.max(0.0)))
    }
    fn try_seek(&mut self, _pos: Duration) -> Result<(), SeekError> {
        Err(SeekError::NotSupported {
            underlying_source: std::any::type_name::<Self>(),
        })
    }
}

impl Drop for ClipSource {
    fn drop(&mut self) {
        self.ctl.done.store(true, Ordering::Release);
    }
}

/// Wraps a streamed source (MP3 speech / music): bus gain, per-voice gain, stop fade,
/// rendered-frame counter and completion flag. Channel layout and rate pass through.
struct StreamVoice<S: Source> {
    inner: S,
    ctl: Arc<VoiceCtl>,
    buses: Arc<Buses>,
    bus: Bus,
    base_gain: f32,
    gain: f32,
    target: f32,
    frame_gain: f32,
    countdown: u32,
    fade: Option<u32>,
    fade_len: u32,
    chan: u16,
    channels: u16,
    rendered: u64,
    finished: bool,
}

impl<S: Source> StreamVoice<S> {
    fn new(inner: S, base_gain: f32, ctl: Arc<VoiceCtl>, buses: Arc<Buses>, bus: Bus) -> Self {
        let target = buses.get(bus) * ctl.gain.load();
        let fade_len = ((STOP_FADE_SECS * inner.sample_rate().get() as f32) as u32).max(1);
        let channels = inner.channels().get();
        Self {
            inner,
            ctl,
            buses,
            bus,
            base_gain,
            gain: target,
            target,
            frame_gain: target * base_gain,
            countdown: 0,
            fade: None,
            fade_len,
            chan: 0,
            channels,
            rendered: 0,
            finished: false,
        }
    }

    fn finish(&mut self) {
        self.finished = true;
        self.ctl.frames.store(self.rendered, Ordering::Relaxed);
        self.ctl.done.store(true, Ordering::Release);
    }
}

impl<S: Source> Iterator for StreamVoice<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.finished {
            return None;
        }
        if self.chan == 0 {
            self.channels = self.inner.channels().get().max(1);
            if self.countdown == 0 {
                self.countdown = CONTROL_PERIOD;
                self.target = self.buses.get(self.bus) * self.ctl.gain.load();
                if self.fade.is_none() && self.ctl.stop.load(Ordering::Relaxed) {
                    self.fade = Some(self.fade_len);
                }
                self.ctl.frames.store(self.rendered, Ordering::Relaxed);
            }
            self.countdown -= 1;
            self.rendered += 1;
            self.gain += (self.target - self.gain) * GAIN_SMOOTH;
            let mut g = self.gain * self.base_gain;
            if let Some(f) = self.fade {
                if f == 0 {
                    self.finish();
                    return None;
                }
                g *= f as f32 / self.fade_len as f32;
                self.fade = Some(f - 1);
            }
            self.frame_gain = g;
        }
        let Some(sample) = self.inner.next() else {
            self.finish();
            return None;
        };
        self.chan = (self.chan + 1) % self.channels;
        Some(sample * self.frame_gain)
    }
}

impl<S: Source> Source for StreamVoice<S> {
    fn current_span_len(&self) -> Option<usize> {
        if self.finished {
            Some(0)
        } else {
            self.inner.current_span_len()
        }
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
    fn try_seek(&mut self, _pos: Duration) -> Result<(), SeekError> {
        Err(SeekError::NotSupported {
            underlying_source: std::any::type_name::<Self>(),
        })
    }
}

impl<S: Source> Drop for StreamVoice<S> {
    fn drop(&mut self) {
        self.ctl.done.store(true, Ordering::Release);
    }
}

fn open_stream_decoder(path: &Path) -> Result<Decoder<BufReader<fs::File>>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let mut builder = Decoder::builder()
        .with_data(BufReader::new(file))
        .with_seekable(true);
    if len > 0 {
        builder = builder.with_byte_len(len);
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        builder = builder.with_hint(&ext.to_ascii_lowercase());
    }
    builder.build().map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------------
// Spatialisation
// ---------------------------------------------------------------------------------------------

/// Distance attenuation and left/right balance on the ground plane.
/// Returns `(gain, pan)` with `pan` in `[-1, 1]` (positive = right), or `None` when inaudible.
pub fn spatialize(
    at: [f32; 2],
    listener: [f32; 2],
    listener_right: [f32; 2],
    hearing_radius: f32,
) -> Option<(f32, f32)> {
    let radius = if hearing_radius.is_finite() {
        hearing_radius.max(1.0)
    } else {
        return None;
    };
    let dx = at[0] - listener[0];
    let dy = at[1] - listener[1];
    let d = (dx * dx + dy * dy).sqrt();
    if !d.is_finite() || d >= radius {
        return None;
    }
    // Full loudness inside 15% of the radius, then a (1-t)^1.5 roll-off to silence at the edge.
    let inner = radius * 0.15;
    let t = ((d - inner) / (radius - inner)).clamp(0.0, 1.0);
    let gain = (1.0 - t).powf(1.5);
    if gain < 0.01 {
        return None;
    }
    let rl = (listener_right[0] * listener_right[0] + listener_right[1] * listener_right[1]).sqrt();
    let pan = if rl > 1e-6 && rl.is_finite() {
        let lateral = (dx * listener_right[0] + dy * listener_right[1]) / rl;
        (lateral / (radius * 0.5)).clamp(-1.0, 1.0)
    } else {
        0.0
    };
    Some((gain, pan))
}

/// Balance law: centre plays both sides at full level, hard pan keeps 30% in the far ear.
pub fn pan_gains(pan: f32) -> [f32; 2] {
    let p = if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    [1.0 - 0.7 * p.max(0.0), 1.0 - 0.7 * (-p).max(0.0)]
}

// ---------------------------------------------------------------------------------------------
// Music playlists
// ---------------------------------------------------------------------------------------------

fn is_intense_track(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.to_ascii_lowercase())
        .is_some_and(|n| n.starts_with("nation") || n.starts_with("cinematic"))
}

struct Rng(u64);

impl Rng {
    fn seeded() -> Self {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng(t ^ 0x2545_F491_4F6C_DD1D | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }
    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        lo + (hi - lo) * self.unit()
    }
    fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

/// `ALL, INTERRUPT` music: an optional lead-in, then every file of the list in shuffled order
/// (duplicates in the list act as weights), refilled forever, never repeating a track back to back.
struct Playlist {
    ids: String,
    leadin: Vec<PathBuf>,
    files: Vec<PathBuf>,
    bag: Vec<usize>,
    last: Option<PathBuf>,
    leadin_done: bool,
    volume: f32,
    fade_in: f32,
    fade_out: f32,
    failures: u32,
}

impl Playlist {
    fn from_def(def: &SoundDef) -> Self {
        Self {
            ids: def.ids.clone(),
            leadin: def.leadin.clone(),
            files: def.files.clone(),
            bag: Vec::new(),
            last: None,
            leadin_done: false,
            volume: def.volume.clamp(0.0, 1.0),
            fade_in: def.fade_time_s.0,
            fade_out: def.fade_time_s.1,
            failures: 0,
        }
    }

    fn has_intense(&self) -> bool {
        self.files.iter().any(|f| is_intense_track(f))
    }

    fn refill(&mut self, rng: &mut Rng) {
        let mut round: Vec<usize> = (0..self.files.len()).collect();
        rng.shuffle(&mut round);
        self.bag.extend(round);
    }

    /// Bag position of the next track that differs from the last one played, honouring the
    /// intensity preference when possible.
    fn pick_position(&self, prefer_intense: Option<bool>) -> Option<usize> {
        let fresh = |i: usize| Some(&self.files[i]) != self.last.as_ref();
        prefer_intense
            .and_then(|want| {
                self.bag
                    .iter()
                    .position(|&i| fresh(i) && is_intense_track(&self.files[i]) == want)
            })
            .or_else(|| self.bag.iter().position(|&i| fresh(i)))
    }

    /// Next track; `prefer_intense = Some(true)` favours nation/cinematic themes (combat).
    fn next(&mut self, rng: &mut Rng, prefer_intense: Option<bool>) -> Option<PathBuf> {
        if !self.leadin_done {
            self.leadin_done = true;
            if !self.leadin.is_empty() {
                let p = self.leadin[rng.below(self.leadin.len())].clone();
                self.last = Some(p.clone());
                return Some(p);
            }
        }
        if self.files.is_empty() {
            return None;
        }
        if self.bag.is_empty() {
            self.refill(rng);
        }
        let mut pos = self.pick_position(prefer_intense);
        if pos.is_none() {
            // Only repeats of the last track remain: append the next shuffled round.
            self.refill(rng);
            pos = self.pick_position(prefer_intense);
        }
        let i = self.bag.remove(pos.unwrap_or(0));
        let p = self.files[i].clone();
        self.last = Some(p.clone());
        Some(p)
    }
}

struct MusicTrack {
    ctl: Arc<VoiceCtl>,
    path: PathBuf,
    level: f32,
    fade_in_rate: f32,
    fade_out_rate: f32,
    fade_out_secs: f32,
    duration: Option<f32>,
    rate: f32,
    intense: bool,
    started: f64,
    volume: f32,
}

impl MusicTrack {
    fn position_secs(&self) -> f32 {
        self.ctl.frames.load(Ordering::Relaxed) as f32 / self.rate.max(1.0)
    }
    fn apply(&self, combat_gain: f32) {
        self.ctl
            .gain
            .store(self.level.clamp(0.0, 1.0) * self.volume * combat_gain);
    }
}

#[derive(Default)]
struct MusicState {
    playlist: Option<Playlist>,
    current: Option<MusicTrack>,
    fading: Vec<MusicTrack>,
    combat_gain: f32,
}

// ---------------------------------------------------------------------------------------------
// Output device
// ---------------------------------------------------------------------------------------------

struct Output {
    /// Keeps the device stream alive.
    _sink: Option<MixerDeviceSink>,
    /// Stereo sub-mixer (behind a limiter) every sound is added to.
    mix: rodio::mixer::Mixer,
    rate: SampleRate,
}

fn open_output() -> Option<Output> {
    let opened = std::panic::catch_unwind(|| {
        rodio::DeviceSinkBuilder::from_default_device()
            .map(|b| {
                b.with_error_callback(|e: rodio::cpal::StreamError| {
                    log::warn!("audio: stream error: {e}")
                })
            })
            .and_then(|b| b.open_sink_or_fallback())
            .or_else(|_| rodio::DeviceSinkBuilder::open_default_sink())
    });
    let mut sink = match opened {
        Ok(Ok(sink)) => sink,
        Ok(Err(e)) => {
            log::warn!("audio: no usable output device ({e}); sound disabled");
            return None;
        }
        Err(_) => {
            log::warn!("audio: output device initialisation panicked; sound disabled");
            return None;
        }
    };
    sink.log_on_drop(false);
    let rate = sink.config().sample_rate();
    let (mix, mix_out) = rodio::mixer::mixer(STEREO, rate);
    // An endless silent input keeps the sub-mixer alive while nothing else plays.
    mix.add(Zero::new(STEREO, rate));
    sink.mixer().add(mix_out.limit(LimitSettings::gaming()));
    log::info!(
        "audio: output {} Hz, {} channels",
        rate.get(),
        sink.config().channel_count().get()
    );
    Some(Output {
        _sink: Some(sink),
        mix,
        rate,
    })
}

// ---------------------------------------------------------------------------------------------
// Audio
// ---------------------------------------------------------------------------------------------

struct ActiveVoice {
    def: usize,
    ctl: Arc<VoiceCtl>,
    priority: Priority,
    started: f64,
    voice: bool,
    causefade: bool,
    stopping: bool,
}

impl ActiveVoice {
    fn stop(&mut self) {
        self.ctl.stop.store(true, Ordering::Relaxed);
        self.stopping = true;
    }
}

pub struct Audio {
    lib: Arc<SoundLibrary>,
    out: Option<Output>,
    buses: Arc<Buses>,
    clips: HashMap<PathBuf, Option<Arc<Clip>>>,
    warned: HashSet<String>,
    voices: Vec<ActiveVoice>,
    sequence: HashMap<usize, usize>,
    last_variant: HashMap<usize, usize>,
    rng: Rng,
    master: f32,
    music_volume: f32,
    effects_volume: f32,
    voice_volume: f32,
    duck: f32,
    time: f64,
    music: MusicState,
    combat_flag: bool,
    last_combat: f64,
    combat_active: bool,
    last_combat_switch: f64,
    feedback_last: [f64; Feedback::ALL.len()],
}

fn warn_once(warned: &mut HashSet<String>, key: String, message: impl FnOnce() -> String) {
    if warned.insert(key) {
        log::warn!("audio: {}", message());
    }
}

impl Audio {
    /// `root` = the reconstruction directory (E:\Games\Axis and Alies\reconstruction). Never fails hard:
    /// if no output device exists, returns an Audio that silently ignores calls (log a warning).
    pub fn new(root: &std::path::Path) -> Self {
        let lib = SoundLibrary::load(root);
        Self::with_parts(lib, open_output())
    }

    /// Loads the definitions but never opens an output device (headless runs, tests).
    pub fn without_output(root: &Path) -> Self {
        Self::with_parts(SoundLibrary::load(root), None)
    }

    fn with_parts(lib: SoundLibrary, out: Option<Output>) -> Self {
        let audio = Self {
            lib: Arc::new(lib),
            out,
            buses: Arc::new(Buses::new()),
            clips: HashMap::new(),
            warned: HashSet::new(),
            voices: Vec::new(),
            sequence: HashMap::new(),
            last_variant: HashMap::new(),
            rng: Rng::seeded(),
            // UVars.tgi defaults: music 70%, 2D/3D 80%, speech 95%.
            master: 1.0,
            music_volume: 0.7,
            effects_volume: 0.8,
            voice_volume: 0.95,
            duck: 1.0,
            time: 0.0,
            music: MusicState {
                combat_gain: CALM_MUSIC_GAIN,
                ..MusicState::default()
            },
            combat_flag: false,
            last_combat: f64::NEG_INFINITY,
            combat_active: false,
            last_combat_switch: f64::NEG_INFINITY,
            feedback_last: [f64::NEG_INFINITY; Feedback::ALL.len()],
        };
        audio.refresh_buses();
        audio
    }

    /// True when an output device is open.
    pub fn is_enabled(&self) -> bool {
        self.out.is_some()
    }

    pub fn library(&self) -> &SoundLibrary {
        &self.lib
    }

    pub fn has_id(&self, ids: &str) -> bool {
        self.lib.index_of(ids).is_some()
    }

    pub fn set_volumes(&mut self, master: f32, music: f32, effects: f32, voice: f32) {
        let clean = |v: f32| {
            if v.is_finite() {
                v.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        self.master = clean(master);
        self.music_volume = clean(music);
        self.effects_volume = clean(effects);
        self.voice_volume = clean(voice);
        self.refresh_buses();
    }

    fn refresh_buses(&self) {
        self.buses
            .set(Bus::Effects, self.master * self.effects_volume * self.duck);
        self.buses.set(Bus::Voice, self.master * self.voice_volume);
        self.buses
            .set(Bus::Music, self.master * self.music_volume * self.duck);
    }

    /// Play a sound/voice by original IDS (random variant among `files`, respecting simultaneous_limit
    /// and a global cap of ~48 concurrent effect voices). Returns false if unknown/suppressed.
    pub fn play(&mut self, ids: &str, volume: f32) -> bool {
        self.play_internal(ids, volume, None)
    }

    /// Positional: attenuate by distance between `at` and `listener` (native metres on the ground plane)
    /// and pan left/right relative to `listener_right` (unit vector, ground plane). Skip if inaudible.
    pub fn play_at(
        &mut self,
        ids: &str,
        at: [f32; 2],
        listener: [f32; 2],
        listener_right: [f32; 2],
        hearing_radius: f32,
    ) -> bool {
        let Some(def) = self.lib.get(ids) else {
            return self.play_internal(ids, 1.0, None); // logs the unknown id once
        };
        if def.world_global {
            return self.play_internal(ids, 1.0, None);
        }
        // `range` max scales the caller's radius relative to the template default (64).
        let radius = hearing_radius * (def.range.1 / 64.0);
        match spatialize(at, listener, listener_right, radius) {
            Some(spatial) => self.play_internal(ids, 1.0, Some(spatial)),
            None => false,
        }
    }

    fn play_internal(&mut self, ids: &str, volume: f32, spatial: Option<(f32, f32)>) -> bool {
        let lib = self.lib.clone();
        let Some(idx) = lib.index_of(ids) else {
            warn_once(&mut self.warned, format!("id:{ids}"), || {
                format!("unknown sound id '{ids}'")
            });
            return false;
        };
        let def = &lib.defs[idx];
        if def.category == SoundCategory::Music {
            return self.play_music(ids);
        }
        if self.out.is_none() || !volume.is_finite() || volume <= 0.0 {
            return false;
        }
        if def.files.is_empty() {
            warn_once(&mut self.warned, format!("nofiles:{ids}"), || {
                format!(
                    "sound id '{ids}' has no playable files (missing: {:?})",
                    def.missing
                )
            });
            return false;
        }
        self.prune_voices();

        // Voices to cut, applied only once the new sound has actually started.
        let mut cut: Vec<usize> = Vec::new();
        // Per-id simultaneous limit.
        let same: Vec<usize> = (0..self.voices.len())
            .filter(|&i| self.voices[i].def == idx && !self.voices[i].stopping)
            .collect();
        if same.len() >= def.simultaneous_limit as usize {
            if !def.has(control::INTERRUPT) {
                return false;
            }
            cut.push(same[0]);
        }
        // One unit-voice response at a time: the newest response cuts older ones of
        // equal or lower priority (briefing speech is not cut by unit chatter).
        if def.has(control::VOICE) {
            for (i, v) in self.voices.iter().enumerate() {
                if v.voice && !v.stopping && v.priority <= def.priority && !cut.contains(&i) {
                    cut.push(i);
                }
            }
        }
        // Global voice cap: steal the lowest-priority, oldest voice not above our priority.
        let active = self
            .voices
            .iter()
            .filter(|v| !v.stopping)
            .count()
            .saturating_sub(cut.len());
        if active >= MAX_EFFECT_VOICES {
            let victim = self
                .voices
                .iter()
                .enumerate()
                .filter(|(i, v)| !v.stopping && v.priority <= def.priority && !cut.contains(i))
                .min_by(|(_, a), (_, b)| {
                    a.priority
                        .cmp(&b.priority)
                        .then(a.started.total_cmp(&b.started))
                })
                .map(|(i, _)| i);
            match victim {
                Some(i) => cut.push(i),
                None => return false,
            }
        }

        let (attenuation, pan) = spatial.unwrap_or((1.0, 0.0));
        let mut gain = def.volume.min(1.0) * volume.min(4.0) * attenuation;
        if def.volume_adjust < 0.0 {
            gain *= 1.0 + self.rng.range(def.volume_adjust, 0.0);
        }
        let pitch = 1.0 + self.rng.range(def.pitch_adjust.0, def.pitch_adjust.1);
        let delay = self.rng.range(def.delay_ms.0, def.delay_ms.1) / 1000.0;
        let bus = if def.category == SoundCategory::Speech || def.has(control::VOICE) {
            Bus::Voice
        } else {
            Bus::Effects
        };

        let first = self.pick_variant(idx, def);
        let n = def.files.len();
        for attempt in 0..n {
            let path = &def.files[(first + attempt) % n];
            let ctl = VoiceCtl::new(1.0);
            if !self.start_voice(path, gain, pitch, pan, delay, &ctl, bus) {
                continue;
            }
            for &i in &cut {
                self.voices[i].stop();
            }
            self.voices.push(ActiveVoice {
                def: idx,
                ctl,
                priority: def.priority,
                started: self.time,
                voice: def.has(control::VOICE),
                causefade: def.has(control::CAUSEFADE),
                stopping: false,
            });
            return true;
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    fn start_voice(
        &mut self,
        path: &Path,
        gain: f32,
        pitch: f32,
        pan: f32,
        delay: f32,
        ctl: &Arc<VoiceCtl>,
        bus: Bus,
    ) -> bool {
        let is_wav = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
        if is_wav {
            let Some(clip) = self.clip(path) else {
                return false;
            };
            let Some(out) = self.out.as_ref() else {
                return false;
            };
            out.mix.add(ClipSource::new(
                clip,
                out.rate,
                pitch,
                pan_gains(pan),
                gain,
                delay,
                ctl.clone(),
                self.buses.clone(),
                bus,
            ));
            true
        } else {
            let Some(out) = self.out.as_ref() else {
                return false;
            };
            match open_stream_decoder(path) {
                Ok(decoder) => {
                    out.mix.add(StreamVoice::new(
                        decoder,
                        gain,
                        ctl.clone(),
                        self.buses.clone(),
                        bus,
                    ));
                    true
                }
                Err(e) => {
                    warn_once(&mut self.warned, format!("file:{}", path.display()), || {
                        format!("cannot stream {}: {e}", path.display())
                    });
                    false
                }
            }
        }
    }

    fn pick_variant(&mut self, idx: usize, def: &SoundDef) -> usize {
        let n = def.files.len();
        if n <= 1 {
            return 0;
        }
        if def.has(control::SEQUENTIAL) {
            let counter = self.sequence.entry(idx).or_insert(0);
            let i = *counter % n;
            *counter = (i + 1) % n;
            return i;
        }
        let mut i = self.rng.below(n);
        if self.last_variant.get(&idx) == Some(&i) {
            i = (i + 1 + self.rng.below(n - 1)) % n;
        }
        self.last_variant.insert(idx, i);
        i
    }

    /// Decoded clip from the cache (decoding on first use). Failures are cached and logged once.
    fn clip(&mut self, path: &Path) -> Option<Arc<Clip>> {
        if let Some(entry) = self.clips.get(path) {
            return entry.clone();
        }
        let loaded = match load_clip(path) {
            Ok(clip) => Some(Arc::new(clip)),
            Err(e) => {
                log::warn!("audio: cannot decode {}: {e}", path.display());
                None
            }
        };
        self.clips.insert(path.to_path_buf(), loaded.clone());
        loaded
    }

    /// Decodes and caches every file of `ids` ahead of time (avoids a disk hit on first play).
    pub fn preload(&mut self, ids: &str) -> usize {
        let lib = self.lib.clone();
        let Some(def) = lib.get(ids) else { return 0 };
        def.files
            .iter()
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
            })
            .filter(|p| self.clip(p).is_some())
            .count()
    }

    pub fn feedback(&mut self, f: Feedback) {
        let slot = f as usize;
        let cooldown = f.cooldown_secs();
        if cooldown > 0.0 && self.time - self.feedback_last[slot] < cooldown {
            return;
        }
        if self.play(f.ids(), 1.0) {
            self.feedback_last[slot] = self.time;
        }
        match f {
            Feedback::Victory => {
                self.play_music("win_interface_music");
            }
            Feedback::Defeat => {
                self.play_music("loss_interface_music");
            }
            _ => {}
        }
    }

    /// Stops every playing effect and voice (music continues).
    pub fn stop_all_effects(&mut self) {
        for v in self.voices.iter_mut() {
            v.stop();
        }
    }

    /// Number of effect/voice sounds currently playing.
    pub fn active_voices(&self) -> usize {
        self.voices
            .iter()
            .filter(|v| !v.stopping && !v.ctl.is_done())
            .count()
    }

    fn prune_voices(&mut self) {
        self.voices.retain(|v| !v.ctl.is_done());
    }

    // ----- music ------------------------------------------------------------------------------

    /// Music: menu theme, nation ambient playlist (nation code "usa","gbr","ger","rus","jpn"), combat intensity hint.
    pub fn play_menu_music(&mut self) {
        self.play_music("main_title_music");
    }

    pub fn play_nation_music(&mut self, nation: &str) {
        let ids = nation_music_ids(nation);
        if self.lib.get(&ids).is_some() {
            self.play_music(&ids);
        } else {
            warn_once(&mut self.warned, format!("nation:{ids}"), || {
                format!("no music '{ids}' for nation '{nation}', using general ambient")
            });
            self.play_music("menu_background_music");
        }
    }

    /// Starts (or keeps) the music playlist `ids` from `music*.tgi`, cross-fading from the
    /// current one. `music_silent` (no files) fades the music out.
    pub fn play_music(&mut self, ids: &str) -> bool {
        let lib = self.lib.clone();
        let Some(def) = lib.get(ids) else {
            warn_once(&mut self.warned, format!("id:{ids}"), || {
                format!("unknown music id '{ids}'")
            });
            return false;
        };
        if self.out.is_none() {
            return false;
        }
        if self
            .music
            .playlist
            .as_ref()
            .is_some_and(|p| p.ids.eq_ignore_ascii_case(&def.ids))
        {
            return true;
        }
        if let Some(cur) = self.music.current.take() {
            self.music.fading.push(cur);
        }
        if def.files.is_empty() && def.leadin.is_empty() {
            self.music.playlist = None;
            return true;
        }
        self.music.playlist = Some(Playlist::from_def(def));
        self.advance_music();
        self.music.current.is_some()
    }

    pub fn set_combat(&mut self, in_combat: bool) {
        self.combat_flag = in_combat;
        if in_combat {
            self.last_combat = self.time;
        }
    }

    pub fn stop_music(&mut self) {
        self.music.playlist = None;
        if let Some(cur) = self.music.current.take() {
            self.music.fading.push(cur);
        }
    }

    /// Id of the active music playlist, if any.
    pub fn current_music(&self) -> Option<&str> {
        self.music.playlist.as_ref().map(|p| p.ids.as_str())
    }

    /// File of the music track currently playing, if any.
    pub fn current_music_track(&self) -> Option<&Path> {
        self.music.current.as_ref().map(|t| t.path.as_path())
    }

    /// Moves the current track to the fade-out list and starts the playlist's next track.
    fn advance_music(&mut self) {
        let prefer = if self.combat_active { Some(true) } else { None };
        if let Some(cur) = self.music.current.take()
            && !cur.ctl.is_done()
        {
            self.music.fading.push(cur);
        }
        let Some(out) = self.out.as_ref() else { return };
        let Some(pl) = self.music.playlist.as_mut() else {
            return;
        };
        let attempts = pl.files.len() + pl.leadin.len() + 1;
        for _ in 0..attempts {
            let Some(path) = pl.next(&mut self.rng, prefer) else {
                break;
            };
            match open_stream_decoder(&path) {
                Ok(decoder) => {
                    let rate = decoder.sample_rate().get() as f32;
                    let duration = decoder
                        .total_duration()
                        .map(|d| d.as_secs_f32())
                        .filter(|d| d.is_finite() && *d > 1.0);
                    let fade_in = pl.fade_in.max(0.0);
                    let level = if fade_in > 0.0 { 0.0 } else { 1.0 };
                    let track = MusicTrack {
                        ctl: VoiceCtl::new(0.0),
                        intense: is_intense_track(&path),
                        path,
                        level,
                        fade_in_rate: if fade_in > 0.0 {
                            1.0 / fade_in
                        } else {
                            f32::INFINITY
                        },
                        fade_out_rate: 1.0 / pl.fade_out.max(0.5),
                        fade_out_secs: pl.fade_out.max(0.5),
                        duration,
                        rate,
                        started: self.time,
                        volume: pl.volume,
                    };
                    track.apply(self.music.combat_gain);
                    out.mix.add(StreamVoice::new(
                        decoder,
                        1.0,
                        track.ctl.clone(),
                        self.buses.clone(),
                        Bus::Music,
                    ));
                    self.music.current = Some(track);
                    return;
                }
                Err(e) => {
                    pl.failures += 1;
                    warn_once(&mut self.warned, format!("file:{}", path.display()), || {
                        format!("cannot stream music {}: {e}", path.display())
                    });
                }
            }
        }
        log::warn!("audio: music playlist '{}' has no playable tracks", pl.ids);
        self.music.playlist = None;
    }

    /// Call once per frame: advances playlists (next track when one ends, crossfade if easy), prunes finished sinks.
    pub fn update(&mut self, dt: f32) {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.5)
        } else {
            0.0
        };
        self.time += dt as f64;
        self.prune_voices();

        // CAUSEFADE (briefing speech) ducks effects and music.
        let duck_target = if self.voices.iter().any(|v| v.causefade && !v.stopping) {
            DUCK_LEVEL
        } else {
            1.0
        };
        let rate = if duck_target < self.duck {
            (1.0 - DUCK_LEVEL) / DUCK_FALL_SECS
        } else {
            (1.0 - DUCK_LEVEL) / DUCK_RISE_SECS
        };
        self.duck = approach(self.duck, duck_target, rate * dt);
        self.refresh_buses();

        if self.out.is_none() {
            return;
        }

        // Combat hint with hold time; a rising edge may swap a calm track for a nation theme.
        let active = self.combat_flag || self.time - self.last_combat < COMBAT_HOLD_SECS;
        let mut combat_switch = false;
        if active
            && !self.combat_active
            && let (Some(cur), Some(pl)) = (&self.music.current, &self.music.playlist)
        {
            combat_switch = !cur.intense
                && pl.has_intense()
                && self.time - cur.started > COMBAT_SWITCH_MIN_TRACK_AGE_SECS
                && self.time - self.last_combat_switch > COMBAT_SWITCH_COOLDOWN_SECS;
        }
        self.combat_active = active;
        let combat_target = if active { 1.0 } else { CALM_MUSIC_GAIN };
        self.music.combat_gain = approach(self.music.combat_gain, combat_target, 0.25 * dt);
        let combat_gain = self.music.combat_gain;

        self.music.fading.retain_mut(|t| {
            t.level -= dt * t.fade_out_rate;
            if t.level <= 0.0 || t.ctl.is_done() {
                t.ctl.stop.store(true, Ordering::Relaxed);
                false
            } else {
                t.apply(combat_gain);
                true
            }
        });

        let mut advance = combat_switch;
        if combat_switch {
            self.last_combat_switch = self.time;
        }
        match self.music.current.as_mut() {
            Some(cur) if cur.ctl.is_done() => {
                if self.time - cur.started < 1.0
                    && let Some(pl) = self.music.playlist.as_mut()
                {
                    pl.failures += 1;
                }
                advance = true;
            }
            Some(cur) => {
                cur.level = (cur.level + dt * cur.fade_in_rate).min(1.0);
                cur.apply(combat_gain);
                if let Some(d) = cur.duration
                    && cur.position_secs() >= d - cur.fade_out_secs
                {
                    advance = true; // cross-fade into the next track before this one ends
                }
            }
            None => advance = self.music.playlist.is_some(),
        }
        if let Some(pl) = self.music.playlist.as_ref()
            && pl.failures as usize > 2 * (pl.files.len() + pl.leadin.len()).max(1)
        {
            log::warn!("audio: music playlist '{}' keeps failing; stopping", pl.ids);
            self.stop_music();
            advance = false;
        }
        if advance && self.music.playlist.is_some() {
            self.advance_music();
        }
    }

    /// Names of known sound ids (for diagnostics/tests).
    pub fn known_ids(&self) -> usize {
        self.lib.len()
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        for v in &self.voices {
            v.ctl.stop.store(true, Ordering::Relaxed);
        }
        if let Some(t) = &self.music.current {
            t.ctl.stop.store(true, Ordering::Relaxed);
        }
        for t in &self.music.fading {
            t.ctl.stop.store(true, Ordering::Relaxed);
        }
    }
}

fn approach(value: f32, target: f32, step: f32) -> f32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Reconstruction root with the extracted original assets, if available on this machine.
    fn data_root() -> Option<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(p) = std::env::var_os("AA_ASSET_ROOT") {
            candidates.push(PathBuf::from(p));
        }
        candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../reconstruction"));
        candidates.push(PathBuf::from(r"E:\Games\Axis and Alies\reconstruction"));
        candidates.into_iter().find(|p| original_dir(p).is_some())
    }

    macro_rules! require_data {
        () => {
            match data_root() {
                Some(root) => root,
                None => {
                    eprintln!("skipping: original audio assets not found (set AA_ASSET_ROOT)");
                    return;
                }
            }
        };
    }

    fn library() -> Option<SoundLibrary> {
        data_root().map(|r| SoundLibrary::load(&r))
    }

    #[test]
    fn parser_handles_comments_nesting_and_braceless_templates() {
        let text = "\u{feff};; header comment\r\n\
            /* block\r\n comment [Audio] { IDS = nope } */\r\n\
            [Template AAMusic Template=AudioMusic]\r\n\
            volume = 100%\r\n\
            \r\n\
            [Audio Template = AudioFeedback]\r\n\
            {\r\n\
            \tIDS = chat_sfx ; trailing comment\r\n\
            \tname = \"#quoted; not a comment\"\r\n\
            \tfiles = Sounds/A.wav, Sounds/B.wav\r\n\
            \t[Subtitle]\r\n\
            \t{\r\n\
            \t\tduration = 7\r\n\
            \t}\r\n\
            \t[Cost]\r\n\
            \tmoney = 50\r\n\
            }\r\n\
            [Template X inherit=Y]\r\n\
            {\r\n\
            \tpoint2 range = 4, 64\r\n\
            \tstring IDS\r\n\
            }\r\n";
        let sections = parse_tgi(text);
        assert_eq!(sections.len(), 3, "{sections:#?}");
        assert_eq!(sections[0].kind, "template");
        assert_eq!(sections[0].name.as_deref(), Some("AAMusic"));
        assert_eq!(sections[0].parent.as_deref(), Some("AudioMusic"));
        assert_eq!(sections[0].own("volume"), Some("100%"));
        let audio = &sections[1];
        assert_eq!(audio.kind, "audio");
        assert_eq!(audio.parent.as_deref(), Some("AudioFeedback"));
        assert_eq!(audio.own("ids"), Some("chat_sfx"));
        assert_eq!(audio.own("name"), Some("#quoted; not a comment"));
        assert_eq!(audio.own("duration"), None, "nested section leaked");
        assert_eq!(audio.own("money"), None, "brace-less nested section leaked");
        assert_eq!(
            parse_list(audio.own("files")),
            vec!["Sounds/A.wav".to_string(), "Sounds/B.wav".to_string()]
        );
        assert_eq!(sections[2].parent.as_deref(), Some("Y"));
        assert_eq!(parse_pair(sections[2].own("range")), Some((4.0, 64.0)));
        assert_eq!(sections[2].own("ids"), None);
    }

    #[test]
    fn value_helpers() {
        assert_eq!(parse_fraction("80%"), Some(0.8));
        assert_eq!(parse_fraction("133%"), Some(1.33));
        assert_eq!(parse_pair(Some("-1%, 1%")), Some((-0.01, 0.01)));
        assert_eq!(parse_flags(Some("INTERRUPT, VOICE, CAUSEFADE")), 1 | 8 | 32);
        assert_eq!(parse_flags(Some("null")), 0);
        assert_eq!(parse_priority(Some("critical")), Priority::Critical);
        assert_eq!(normalize_rel("/Audio/Default/X.wav"), "default/x.wav");
        assert_eq!(nation_music_ids("USA"), "usa_ambient");
        assert_eq!(nation_music_ids("germany"), "ger_ambient");
        assert_eq!(nation_music_ids("rus_ambient"), "rus_ambient");
    }

    #[test]
    fn wav_decoder_reads_pcm16_with_extra_chunks() {
        let samples: [i16; 4] = [0, 1000, -1000, 32767];
        let mut data = Vec::new();
        for s in samples {
            data.extend_from_slice(&s.to_le_bytes());
        }
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&0u32.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&18u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&22050u32.to_le_bytes());
        wav.extend_from_slice(&44100u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(&0u16.to_le_bytes()); // cbSize
        wav.extend_from_slice(b"fact");
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        let clip = decode_wav(&wav).expect("decode");
        assert_eq!(clip.channels, 1);
        assert_eq!(clip.rate, 22050);
        assert_eq!(&clip.samples[..], &samples[..]);
        assert!(decode_wav(b"not a wav").is_err());
    }

    #[test]
    fn clip_source_renders_panned_stereo_and_signals_completion() {
        let clip = Arc::new(Clip {
            samples: vec![16384i16; 100].into(),
            channels: 1,
            rate: 1000,
        });
        let buses = Arc::new(Buses::new());
        let ctl = VoiceCtl::new(1.0);
        let rate = NonZero::new(1000).unwrap();
        let src = ClipSource::new(
            clip.clone(),
            rate,
            1.0,
            pan_gains(1.0),
            1.0,
            0.0,
            ctl.clone(),
            buses.clone(),
            Bus::Effects,
        );
        assert_eq!(src.channels().get(), 2);
        let out: Vec<f32> = src.collect();
        assert_eq!(out.len(), 200);
        let (l, r) = (out[2], out[3]);
        assert!((r - 0.5).abs() < 1e-3, "right {r}");
        assert!((l - 0.5 * 0.3).abs() < 1e-3, "left {l}");
        assert!(ctl.is_done());

        // A stop request fades the voice out quickly instead of playing it to the end.
        let long = Arc::new(Clip {
            samples: vec![1000i16; 100_000].into(),
            channels: 1,
            rate: 1000,
        });
        let ctl = VoiceCtl::new(1.0);
        let mut src = ClipSource::new(
            long,
            rate,
            1.0,
            [1.0, 1.0],
            1.0,
            0.0,
            ctl.clone(),
            buses,
            Bus::Effects,
        );
        for _ in 0..10 {
            src.next();
        }
        ctl.stop.store(true, Ordering::Relaxed);
        let rest = src.count();
        assert!(rest < 200, "stop fade too long: {rest} samples");
        assert!(ctl.is_done());
    }

    #[test]
    fn spatialization_attenuates_and_pans() {
        let right = [1.0, 0.0];
        let (g, p) = spatialize([0.0, 0.0], [0.0, 0.0], right, 100.0).unwrap();
        assert_eq!((g, p), (1.0, 0.0));
        let (g_far, p_right) = spatialize([60.0, 0.0], [0.0, 0.0], right, 100.0).unwrap();
        assert!(g_far < 1.0 && g_far > 0.05, "{g_far}");
        assert!(p_right > 0.9);
        let (_, p_left) = spatialize([-30.0, 10.0], [0.0, 0.0], right, 100.0).unwrap();
        assert!(p_left < -0.5);
        assert!(spatialize([150.0, 0.0], [0.0, 0.0], right, 100.0).is_none());
        assert!(spatialize([f32::NAN, 0.0], [0.0, 0.0], right, 100.0).is_none());
        let [l, r] = pan_gains(0.0);
        assert_eq!((l, r), (1.0, 1.0));
    }

    #[test]
    fn real_definitions_load_with_template_defaults() {
        let Some(lib) = library() else {
            eprintln!("skipping: original audio assets not found");
            return;
        };
        assert!(lib.len() > 100, "only {} ids", lib.len());

        let grenade = lib.get("grenade").expect("grenade");
        assert_eq!(grenade.files.len(), 3);
        assert!(grenade.files.iter().all(|f| f.is_file()));
        assert!(grenade.has(control::WORLD));
        assert_eq!(grenade.priority, Priority::High);
        assert_eq!(grenade.pitch_adjust, (-0.01, 0.01));
        assert!((grenade.volume - 0.8).abs() < 1e-6);
        assert_eq!(grenade.simultaneous_limit, 2);
        assert!(grenade.world_fogged);

        let heavy = lib.get("explosion_artillery_heavy").unwrap();
        assert!((heavy.volume - 0.75).abs() < 1e-6);
        assert_eq!(lib.get("hammer").unwrap().simultaneous_limit, 1);

        let voice = lib.get("usa_infantry_select").expect("usa_infantry_select");
        assert_eq!(voice.files.len(), 2);
        assert!(voice.files.iter().all(|f| f.is_file()));
        assert!(voice.has(control::VOICE));
        assert_eq!(voice.simultaneous_limit, 1);
        assert!((voice.volume - 0.7).abs() < 1e-6);
        for ids in [
            "usa_infantry_move",
            "usa_infantry_attack",
            "ger_rommel_select",
        ] {
            assert!(!lib.get(ids).unwrap().files.is_empty(), "{ids}");
        }

        let general = lib.get("usa_patton_select").unwrap();
        assert!(general.has(control::SEQUENTIAL));

        let speech = lib.get("allies01_intro").expect("campaign speech");
        assert_eq!(speech.category, SoundCategory::Speech);
        assert!(speech.has(control::CAUSEFADE));
        assert!(speech.files[0].to_string_lossy().contains("Speech"));
        assert!(speech.files[0].is_file());

        let ui = lib.get("push_button").unwrap();
        assert_eq!(ui.priority, Priority::Critical);
        assert_eq!(ui.files.len(), 4);
        // Mixed-case paths ("sounds/Feedback/...") resolve case-insensitively.
        assert!(!lib.get("actor_captured_sfx").unwrap().files.is_empty());
    }

    #[test]
    fn music_definitions_cover_menu_nations_and_end_screens() {
        let Some(lib) = library() else {
            eprintln!("skipping: original audio assets not found");
            return;
        };
        let menu = lib.get("main_title_music").expect("menu music");
        assert_eq!(menu.category, SoundCategory::Music);
        assert_eq!(menu.leadin.len(), 1);
        assert_eq!(menu.files.len(), 3);
        assert_eq!(menu.fade_time_s, (4.0, 2.0));
        assert!(menu.has(control::ALL));
        for nation in ["usa", "gbr", "ger", "rus", "jpn"] {
            let ids = nation_music_ids(nation);
            let def = lib.get(&ids).unwrap_or_else(|| panic!("{ids}"));
            assert_eq!(def.category, SoundCategory::Music);
            assert_eq!(def.files.len(), 20, "{ids}: missing {:?}", def.missing);
            assert!(def.missing.is_empty(), "{ids}: {:?}", def.missing);
            let own = format!("nation{nation}");
            assert!(
                def.files
                    .iter()
                    .any(|f| f.to_string_lossy().to_ascii_lowercase().contains(&own))
            );
            assert!(def.files.iter().all(|f| f.is_file()));
        }
        for ids in [
            "menu_background_music",
            "win_interface_music",
            "loss_interface_music",
        ] {
            assert!(!lib.get(ids).unwrap().files.is_empty(), "{ids}");
        }
        let silent = lib.get("music_silent").unwrap();
        assert!(silent.files.is_empty());
        assert_eq!(silent.category, SoundCategory::Music);
    }

    #[test]
    fn feedback_ids_resolve_to_files() {
        let Some(lib) = library() else {
            eprintln!("skipping: original audio assets not found");
            return;
        };
        for f in Feedback::ALL {
            let def = lib
                .get(f.ids())
                .unwrap_or_else(|| panic!("{f:?} -> {}", f.ids()));
            assert!(!def.files.is_empty(), "{f:?}");
            assert!(def.files.iter().all(|p| p.is_file()), "{f:?}");
        }
    }

    #[test]
    fn every_referenced_wav_decodes() {
        let Some(lib) = library() else {
            eprintln!("skipping: original audio assets not found");
            return;
        };
        let mut seen = HashSet::new();
        let mut failures = Vec::new();
        let mut decoded = 0usize;
        for def in lib.defs() {
            for path in &def.files {
                let is_wav = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
                if !is_wav || !seen.insert(path.clone()) {
                    continue;
                }
                match fs::read(path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| decode_wav(&b))
                {
                    Ok(clip) => {
                        assert!(clip.frames() > 0);
                        decoded += 1;
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.display())),
                }
            }
        }
        let missing: Vec<String> = lib
            .defs()
            .iter()
            .flat_map(|d| d.missing.iter().map(move |m| format!("{}: {m}", d.ids)))
            .collect();
        eprintln!(
            "decoded {decoded} WAVs; {} unresolved references: {missing:?}",
            missing.len()
        );
        assert!(decoded > 500, "{decoded}");
        assert!(failures.is_empty(), "{failures:#?}");
        // The shipped data references 12 WAVs that are absent from the original Data.rwd
        // (truck select callouts, RUS rocket truck attack, two special-op stings).
        assert!(missing.len() <= 16, "{missing:#?}");
    }

    #[test]
    fn playlist_shuffles_without_immediate_repeats_and_prefers_intense_in_combat() {
        let Some(lib) = library() else {
            eprintln!("skipping: original audio assets not found");
            return;
        };
        let def = lib.get("usa_ambient").unwrap();
        let mut pl = Playlist::from_def(def);
        let mut rng = Rng(12345);
        let mut last: Option<PathBuf> = None;
        for _ in 0..400 {
            let p = pl.next(&mut rng, None).unwrap();
            assert_ne!(Some(&p), last.as_ref());
            last = Some(p);
        }
        assert!(pl.has_intense());
        let combat = pl.next(&mut rng, Some(true)).unwrap();
        assert!(is_intense_track(&combat), "{}", combat.display());

        let menu = lib.get("main_title_music").unwrap();
        let mut pl = Playlist::from_def(menu);
        let first = pl.next(&mut rng, None).unwrap();
        assert!(first.to_string_lossy().contains("MainTheme"));
        let second = pl.next(&mut rng, None).unwrap();
        assert_ne!(first, second);
    }

    /// Audio wired to an in-memory mixer instead of a device; the returned source is the mix.
    fn audio_with_test_mixer(root: &Path) -> (Audio, rodio::mixer::MixerSource) {
        let rate = NonZero::new(22_050).unwrap();
        let (mix, source) = rodio::mixer::mixer(STEREO, rate);
        mix.add(Zero::new(STEREO, rate));
        let out = Output {
            _sink: None,
            mix,
            rate,
        };
        (
            Audio::with_parts(SoundLibrary::load(root), Some(out)),
            source,
        )
    }

    #[test]
    fn voice_limits_cap_and_completion_with_headless_mixer() {
        let root = require_data!();
        let (mut audio, mut mix) = audio_with_test_mixer(&root);
        audio.set_volumes(1.0, 1.0, 1.0, 1.0);

        // simultaneous_limit = 2 without INTERRUPT: the third request is refused.
        assert!(audio.play("grenade", 1.0));
        assert!(audio.play("grenade", 1.0));
        assert!(!audio.play("grenade", 1.0));
        // INTERRUPT (interface sounds): the oldest instance is cut instead.
        for _ in 0..3 {
            assert!(audio.play("push_button", 1.0));
        }
        // Unit responses: same id is refused while speaking, a new response cuts the old one.
        assert!(audio.play("usa_infantry_select", 1.0));
        assert!(!audio.play("usa_infantry_select", 1.0));
        assert!(audio.play("usa_infantry_move", 1.0));
        let speaking = audio
            .voices
            .iter()
            .filter(|v| v.voice && !v.stopping)
            .count();
        assert_eq!(speaking, 1);

        // Fill the global cap with HIGH-priority combat sounds.
        let lib = audio.lib.clone();
        let attack: Vec<&str> = lib
            .defs()
            .iter()
            .filter(|d| d.template == "AudioAttack" && !d.files.is_empty())
            .map(|d| d.ids.as_str())
            .collect();
        for ids in &attack {
            audio.play_at(ids, [5.0, 0.0], [0.0, 0.0], [1.0, 0.0], 200.0);
        }
        assert_eq!(audio.active_voices(), MAX_EFFECT_VOICES);
        // A LOW-priority ambient sound cannot steal from HIGH voices; a CRITICAL one can.
        let ambient = lib
            .defs()
            .iter()
            .find(|d| d.template == "AudioAmbient" && !d.files.is_empty())
            .unwrap();
        assert!(!audio.play(&ambient.ids, 1.0));
        assert!(audio.play("menu_button", 1.0));
        assert!(audio.active_voices() <= MAX_EFFECT_VOICES);

        // Render the mix: every voice finishes and is pruned; output stays finite.
        let mut peak = 0.0f32;
        for _ in 0..40 {
            for _ in 0..22_050 * 2 {
                let s = mix.next().unwrap_or(0.0);
                assert!(s.is_finite());
                peak = peak.max(s.abs());
            }
            audio.update(1.0 / 60.0);
            if audio.active_voices() == 0 {
                break;
            }
        }
        assert_eq!(audio.active_voices(), 0);
        assert!(audio.voices.is_empty());
        assert!(peak > 0.01, "nothing was rendered");
        // Cached clips are reused across plays.
        assert!(audio.clips.len() < 200);
    }

    #[test]
    fn music_playlists_switch_and_stop_with_headless_mixer() {
        let root = require_data!();
        let (mut audio, mut mix) = audio_with_test_mixer(&root);
        audio.play_menu_music();
        assert_eq!(audio.current_music(), Some("main_title_music"));
        let first = audio.current_music_track().unwrap().to_path_buf();
        assert!(first.to_string_lossy().contains("MainTheme"));
        // Same playlist again is a no-op (safe to call every frame).
        audio.play_menu_music();
        assert_eq!(audio.current_music_track(), Some(first.as_path()));

        audio.play_nation_music("jpn");
        assert_eq!(audio.current_music(), Some("jpn_ambient"));
        assert_eq!(audio.music.fading.len(), 1);
        for _ in 0..30 {
            for _ in 0..4_410 {
                mix.next();
            }
            audio.update(0.1);
        }
        // Menu fade-out (2 s) finished; the old stream was told to stop.
        assert!(audio.music.fading.is_empty());
        let track = audio.music.current.as_ref().unwrap();
        assert!(track.level > 0.99, "fade-in incomplete: {}", track.level);
        assert!(track.position_secs() > 0.5);

        audio.feedback(Feedback::Defeat);
        assert_eq!(audio.current_music(), Some("loss_interface_music"));
        assert!(audio.play_music("music_silent"));
        assert_eq!(audio.current_music(), None);
        assert!(audio.current_music_track().is_none());
        audio.play_nation_music("ger");
        audio.stop_music();
        assert_eq!(audio.current_music(), None);
    }

    #[test]
    fn silent_audio_ignores_calls_without_panicking() {
        let root = require_data!();
        let mut audio = Audio::without_output(&root);
        assert!(audio.known_ids() > 100);
        assert!(!audio.is_enabled());
        audio.set_volumes(1.0, 0.5, 0.8, 0.9);
        assert!(!audio.play("grenade", 1.0));
        assert!(!audio.play("no_such_sound", 1.0));
        assert!(!audio.play_at("grenade", [10.0, 0.0], [0.0, 0.0], [1.0, 0.0], 100.0));
        for f in Feedback::ALL {
            audio.feedback(f);
        }
        audio.play_menu_music();
        audio.play_nation_music("usa");
        audio.play_nation_music("atlantis");
        audio.set_combat(true);
        audio.update(0.016);
        audio.set_combat(false);
        audio.stop_music();
        audio.update(f32::NAN);
        assert_eq!(audio.active_voices(), 0);
        assert!(audio.preload("grenade") == 3);

        let mut empty = Audio::without_output(Path::new("Z:/definitely/missing"));
        assert_eq!(empty.known_ids(), 0);
        assert!(!empty.play("grenade", 1.0));
        empty.update(0.1);
    }
}
