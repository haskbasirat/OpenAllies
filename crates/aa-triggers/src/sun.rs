//! The one GWorld+4C Sun the trigger elements 45 (global_adjust_sun_dimmer)
//! and 46 (global_set_sun_colors) drive, and its saved SUN chunk.
//!
//! * `4EE0A0` constructor: the nine colour words from `B2814C..B28154`
//!   (all zero in the image), direction (0, 0, 1), start time -1.0;
//! * `4EDBD0` Load: Sun version, Updatable version (+ saved id), direction,
//!   current then target colour triples, colour duration, dimmer words,
//!   start time and the version-1 tail (no floating arithmetic);
//! * `4EDB10` SetColors(diffuse, ambient, specular, duration) and `4ED820`
//!   AdjustDimmer(target, duration): an ordered-equal zero duration (or
//!   targets equal to the current values, `4EDA30`) copies the targets;
//!   otherwise an unscheduled Sun schedules itself on World+1E8 (`50ACC0`,
//!   1/30 s, data 0) and stores +6C = World+E0;
//! * `4EDDC0` the deferred +18 update: the dimmer and the nine colour words
//!   move towards their targets by dt = binary32(World+E0 - +6C) with the
//!   original x87 operation order and binary32 stores, rescheduling while a
//!   transition remains.
//!
//! The same arithmetic as aa-runtime `live_sun` (proven there by
//! research/live-battle-ticks/native-sun.json, replayed here too); this copy
//! keeps the queue outside: `scheduled` is the Updatable +4 head, cleared by
//! the caller's delivery before the update runs (the deferred queue unlinks
//! an event before its callback).
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

type Result<T> = std::result::Result<T, String>;
const ONE: u32 = 0x3f80_0000;
/// binary32 1/30 s (`0x3D088889`), the Sun's re-update delay.
pub const SUN_DELAY_BITS: u32 = 0x3d08_8889;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sun {
    /// Updatable +4: an event of this Sun is on World+1E8.
    pub scheduled: bool,
    /// The saved Updatable id of the SUN chunk (-1 or absent: none).
    pub saved_updatable: Option<i32>,
    pub direction: [u32; 3],
    /// Native +14, +20, +2C (diffuse, ambient, specular).
    pub target_colors: [[u32; 3]; 3],
    /// Native +38, +44, +50.
    pub current_colors: [[u32; 3]; 3],
    /// +5C.
    pub color_duration: u32,
    /// +60 current, +64 target, +68 duration.
    pub current_dimmer: u32,
    pub target_dimmer: u32,
    pub dimmer_duration: u32,
    /// +6C.
    pub start_time: u32,
    pub byte_70: u8,
    pub word_74: u32,
    pub word_78: u32,
    pub byte_7c: u8,
    pub word_80: u32,
    pub word_84: u32,
}

fn b(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| e.to_string())
}
fn st(v: aa_numeric::Result<Binary>) -> Result<Binary> {
    v.map_err(|e| e.to_string())
}
fn store(v: Binary) -> Result<u32> {
    v.store_f32().map_err(|e| e.to_string())
}
/// x87 FUCOMPP of two binary32 words: None when unordered.
fn cmp(a: u32, c: u32) -> Option<Ordering> {
    f32::from_bits(a).partial_cmp(&f32::from_bits(c))
}
fn positive(bits: u32) -> bool {
    cmp(bits, 0) == Some(Ordering::Greater)
}
/// `fcomp 0; test ah,0x41; jp` taken: greater or unordered.
fn greater_or_unordered(bits: u32) -> bool {
    !matches!(cmp(bits, 0), Some(Ordering::Less | Ordering::Equal))
}
fn flat(colors: &[[u32; 3]; 3]) -> [u32; 9] {
    let mut out = [0; 9];
    for (i, c) in colors.iter().flatten().enumerate() {
        out[i] = *c;
    }
    out
}
fn unflat(w: [u32; 9]) -> [[u32; 3]; 3] {
    [[w[0], w[1], w[2]], [w[3], w[4], w[5]], [w[6], w[7], w[8]]]
}
/// `4EDA30` / `4EDD30`: any of the nine words unequal or unordered.
fn differ(a: &[[u32; 3]; 3], c: &[[u32; 3]; 3]) -> bool {
    flat(a).iter().zip(flat(c).iter()).any(|(x, y)| cmp(*x, *y) != Some(Ordering::Equal))
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn u32(&mut self) -> Result<u32> {
        let v = self.bytes.get(self.at..self.at + 4).ok_or("SUN chunk is truncated")?;
        self.at += 4;
        Ok(u32::from_le_bytes(v.try_into().unwrap()))
    }
    fn u8(&mut self) -> Result<u8> {
        let v = *self.bytes.get(self.at).ok_or("SUN chunk is truncated")?;
        self.at += 1;
        Ok(v)
    }
    fn triple(&mut self) -> Result<[u32; 3]> {
        Ok([self.u32()?, self.u32()?, self.u32()?])
    }
}

impl Default for Sun {
    fn default() -> Self {
        Self::new()
    }
}

impl Sun {
    /// `4EE0A0` (the `B2814C` words are zero in the image).
    pub fn new() -> Self {
        Self {
            scheduled: false,
            saved_updatable: None,
            direction: [0, 0, ONE],
            target_colors: [[0; 3]; 3],
            current_colors: [[0; 3]; 3],
            color_duration: 0,
            current_dimmer: 0,
            target_dimmer: 0,
            dimmer_duration: 0,
            start_time: 0xbf80_0000,
            byte_70: 0,
            word_74: 0,
            word_78: 0,
            byte_7c: 0,
            word_80: 0,
            word_84: 0,
        }
    }

    /// `4EDBD0` over the SUN chunk: the Sun version (<= 2) is read before its
    /// Updatable base's version (<= 1, then the saved id). Returns the bytes
    /// read. The saved Updatable id is kept for the caller's queue owner.
    pub fn load(&mut self, bytes: &[u8]) -> Result<usize> {
        let mut r = Reader { bytes, at: 0 };
        let version = r.u32()?;
        if version > 2 {
            return Err(format!("Sun version {version} is newer than the original reader (2)"));
        }
        let updatable_version = r.u32()?;
        if updatable_version > 1 {
            return Err(format!("Updatable version {updatable_version} is newer than the original reader (1)"));
        }
        self.scheduled = false;
        if updatable_version >= 1 {
            let id = r.u32()? as i32;
            self.saved_updatable = (id >= 0).then_some(id);
        }
        self.direction = r.triple()?;
        for color in &mut self.current_colors {
            *color = r.triple()?;
        }
        for color in &mut self.target_colors {
            *color = r.triple()?;
        }
        self.color_duration = r.u32()?;
        self.current_dimmer = r.u32()?;
        self.target_dimmer = r.u32()?;
        self.dimmer_duration = r.u32()?;
        self.start_time = r.u32()?;
        if version >= 1 {
            self.byte_70 = r.u8()?;
            self.word_74 = r.u32()?;
            self.word_78 = r.u32()?;
            self.byte_7c = r.u8()?;
            self.word_80 = r.u32()?;
            self.word_84 = r.u32()?;
        }
        if version <= 1 {
            self.word_74 = 0;
            self.word_78 = 0;
            self.word_80 = 0;
            self.word_84 = 0;
        }
        Ok(r.at)
    }

    /// `50ACC0` through the +4 test: schedule once and restart the clock.
    fn schedule(&mut self, now: u32) {
        if !self.scheduled {
            self.scheduled = true;
            self.start_time = now;
        }
    }

    /// `4EDB10`.
    pub fn set_colors(&mut self, colors: [[u32; 3]; 3], duration: u32, now: u32) {
        self.target_colors = colors;
        self.color_duration = duration;
        // fld 0; fld duration; fucompp; test ah,44; jnp: ordered equal copies.
        if cmp(duration, 0) == Some(Ordering::Equal) || !differ(&self.target_colors, &self.current_colors) {
            self.current_colors = self.target_colors;
            self.color_duration = 0;
            return;
        }
        self.schedule(now);
    }

    /// `4ED820` (binary32 target and duration).
    pub fn adjust_dimmer(&mut self, target: u32, duration: u32, now: u32) {
        self.target_dimmer = target;
        self.dimmer_duration = duration;
        if cmp(duration, 0) == Some(Ordering::Equal) || cmp(target, self.current_dimmer) == Some(Ordering::Equal) {
            self.current_dimmer = target;
            self.dimmer_duration = 0;
            return;
        }
        self.schedule(now);
    }

    /// The World+1E8 delivery of this Sun's event: the event is unlinked
    /// (`scheduled` cleared), then `4EDDC0` runs.
    pub fn deliver(&mut self, now: u32, p: Precision) -> Result<()> {
        self.scheduled = false;
        self.update(now, p)
    }

    /// `4EDDC0`.
    pub fn update(&mut self, now: u32, p: Precision) -> Result<()> {
        let dt = store(st(b(now)?.sub(&b(self.start_time)?, p))?)?;
        // fld +64; fld +60; fucompp; test ah,44; jnp skips ordered equal.
        if cmp(self.current_dimmer, self.target_dimmer) != Some(Ordering::Equal) {
            if positive(self.dimmer_duration) {
                let v = st(b(self.target_dimmer)?.sub(&b(self.current_dimmer)?, p))?;
                let v = st(v.div(&b(self.dimmer_duration)?, p))?;
                let v = st(v.mul(&b(dt)?, p))?;
                self.current_dimmer = store(st(v.add(&b(self.current_dimmer)?, p))?)?;
                self.dimmer_duration = store(st(b(self.dimmer_duration)?.sub(&b(dt)?, p))?)?;
            }
            if greater_or_unordered(self.dimmer_duration) {
                self.schedule(now);
            } else {
                self.current_dimmer = self.target_dimmer;
                self.dimmer_duration = 0;
            }
        }
        if !differ(&self.current_colors, &self.target_colors) {
            self.start_time = now;
            return Ok(());
        }
        let t = flat(&self.target_colors);
        let mut c = flat(&self.current_colors);
        // The first difference stays in ST0; the other eight are stored.
        let r0 = st(b(t[0])?.sub(&b(c[0])?, p))?;
        let mut d = [0u32; 9];
        for k in 1..9 {
            d[k] = store(st(b(t[k])?.sub(&b(c[k])?, p))?)?;
        }
        if positive(self.color_duration) {
            let dtb = b(dt)?;
            for group in 0..3 {
                let base = 3 * group;
                let x0 = if group == 0 { st(r0.mul(&dtb, p))? } else { st(b(d[base])?.mul(&dtb, p))? };
                let x1 = st(b(d[base + 1])?.mul(&dtb, p))?;
                let x2 = store(st(b(d[base + 2])?.mul(&dtb, p))?)?;
                let inv = store(st(b(ONE)?.div(&b(self.color_duration)?, p))?)?;
                let a0 = store(st(x0.mul(&b(inv)?, p))?)?;
                let y1 = st(x1.mul(&b(inv)?, p))?;
                let y2 = st(b(x2)?.mul(&b(inv)?, p))?;
                c[base] = store(st(b(a0)?.add(&b(c[base])?, p))?)?;
                c[base + 1] = store(st(y1.add(&b(c[base + 1])?, p))?)?;
                c[base + 2] = store(st(y2.add(&b(c[base + 2])?, p))?)?;
            }
            self.current_colors = unflat(c);
            self.color_duration = store(st(b(self.color_duration)?.sub(&dtb, p))?)?;
        }
        if greater_or_unordered(self.color_duration) {
            self.schedule(now);
            self.start_time = now;
        } else {
            self.current_colors = self.target_colors;
            self.color_duration = 0;
            self.start_time = now;
        }
        Ok(())
    }

    /// The current colours as f32 (diffuse, ambient, specular) and the
    /// current dimmer, for a renderer.
    pub fn current(&self) -> ([[f32; 3]; 3], f32) {
        (self.current_colors.map(|c| c.map(f32::from_bits)), f32::from_bits(self.current_dimmer))
    }
}
