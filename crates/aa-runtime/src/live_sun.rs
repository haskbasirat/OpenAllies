//! GWorld+4C Sun colour/dimmer transitions on the live retained World.
//!
//! * `4EDB10` Sun::SetColors(diffuse, ambient, specular, duration): stores the
//!   three target colours (+14/+20/+2C) and +5C; a zero duration, or targets
//!   already equal to the current colours (`4EDA30`), copies them to +38/+44/+50
//!   and clears +5C; otherwise an unscheduled Sun schedules itself on World+1E8
//!   (`50ACC0`, 0x3D088889, data 0) and stores +6C = World+E0.
//! * `4EDDC0` the Sun Updatable's deferred +18: interpolates the dimmer
//!   (+60 towards +64 over +68) and the nine colour words (+38.. towards +14..
//!   over +5C) by dt = binary32(World+E0 - +6C), with the exact x87 operation
//!   order and binary32 stores of the original (ST-resident terms are rounded
//!   only to the calling thread's precision); it reschedules while either
//!   transition remains and finally stores +6C = World+E0.
//! The Sun's "+4 scheduled" test is the SAME queue owner's event head.
use crate::sun_state::State;
use aa_numeric::{Binary, Precision};
use aa_world::queues::{Owner, Queues};
use std::cmp::Ordering;

type Result<T> = std::result::Result<T, String>;
/// `[9F32DC]` 0.0 and `[9F32E0]` 1.0.
const ONE: u32 = 0x3f80_0000;
/// binary32 1/30 s, the Sun re-update delay.
pub const SUN_DELAY_BITS: u32 = 0x3d08_8889;

fn b(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| e.to_string())
}
fn st(v: aa_numeric::Result<Binary>) -> Result<Binary> {
    v.map_err(|e| e.to_string())
}
fn store(v: Binary) -> Result<u32> {
    v.store_f32().map_err(|e| e.to_string())
}
/// x87 FCOMP/FUCOMPP of two binary32 words: None for unordered.
fn cmp(a: u32, c: u32) -> Option<Ordering> {
    let (x, y) = (f32::from_bits(a), f32::from_bits(c));
    x.partial_cmp(&y)
}
/// `test ah, 0x41; jne`/`jp` style "strictly greater than zero" (ordered).
fn positive(bits: u32) -> bool {
    cmp(bits, 0) == Some(Ordering::Greater)
}
/// `fcomp 0; test ah,0x41; jp` taken: greater OR unordered.
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
fn unflat(words: [u32; 9]) -> [[u32; 3]; 3] {
    [[words[0], words[1], words[2]], [words[3], words[4], words[5]], [words[6], words[7], words[8]]]
}
/// `4EDD30`/`428430`: any of the nine words compares unequal or unordered.
fn differ(a: &[[u32; 3]; 3], c: &[[u32; 3]; 3]) -> bool {
    flat(a).iter().zip(flat(c).iter()).any(|(x, y)| cmp(*x, *y) != Some(Ordering::Equal))
}
fn schedule(queues: &mut Queues, owner: Owner, sun: &mut State, now: u32) -> Result<()> {
    if queues.owner_head(owner).is_none() {
        queues.add_after(3, owner, 0, SUN_DELAY_BITS).map_err(|e| e.to_string())?;
        sun.start_time = now;
    }
    Ok(())
}

/// Native `4EDB10`.
pub(crate) fn set_colors(
    sun: &mut State,
    colors: [[u32; 3]; 3],
    duration: u32,
    queues: &mut Queues,
    owner: Owner,
    now: u32,
) -> Result<()> {
    sun.target_colors = colors;
    sun.color_duration = duration;
    //fld 0; fld duration; fucompp; test ah,44; jnp -> ordered equal copies.
    if cmp(duration, 0) == Some(Ordering::Equal) || !differ(&sun.target_colors, &sun.current_colors) {
        sun.current_colors = sun.target_colors;
        sun.color_duration = 0;
        return Ok(());
    }
    if queues.owner_head(owner).is_none() {
        queues.add_after(3, owner, 0, SUN_DELAY_BITS).map_err(|e| e.to_string())?;
        sun.start_time = now;
    }
    Ok(())
}

/// Native `4EDDC0` (Sun Updatable deferred +18; data unused).
pub(crate) fn update(sun: &mut State, queues: &mut Queues, owner: Owner, now: u32, p: Precision) -> Result<()> {
    let dt = store(st(b(now)?.sub(&b(sun.start_time)?, p))?)?;
    //Dimmer: fld +64; fld +60; fucompp; test ah,44; jnp skips ordered equal.
    if cmp(sun.current_dimmer, sun.target_dimmer) != Some(Ordering::Equal) {
        if positive(sun.dimmer_duration) {
            let v = st(b(sun.target_dimmer)?.sub(&b(sun.current_dimmer)?, p))?;
            let v = st(v.div(&b(sun.dimmer_duration)?, p))?;
            let v = st(v.mul(&b(dt)?, p))?;
            sun.current_dimmer = store(st(v.add(&b(sun.current_dimmer)?, p))?)?;
            sun.dimmer_duration = store(st(b(sun.dimmer_duration)?.sub(&b(dt)?, p))?)?;
        }
        if greater_or_unordered(sun.dimmer_duration) {
            schedule(queues, owner, sun, now)?;
        } else {
            sun.current_dimmer = sun.target_dimmer;
            sun.dimmer_duration = 0;
        }
    }
    if !differ(&sun.current_colors, &sun.target_colors) {
        sun.start_time = now;
        return Ok(());
    }
    let t = flat(&sun.target_colors);
    let mut c = flat(&sun.current_colors);
    //First difference stays in ST0; the other eight are stored binary32.
    let r0 = st(b(t[0])?.sub(&b(c[0])?, p))?;
    let mut d = [0u32; 9];
    for k in 1..9 {
        d[k] = store(st(b(t[k])?.sub(&b(c[k])?, p))?)?;
    }
    if positive(sun.color_duration) {
        let dtb = b(dt)?;
        for group in 0..3 {
            let base = 3 * group;
            let x0 = if group == 0 { st(r0.mul(&dtb, p))? } else { st(b(d[base])?.mul(&dtb, p))? };
            let x1 = st(b(d[base + 1])?.mul(&dtb, p))?;
            let x2 = store(st(b(d[base + 2])?.mul(&dtb, p))?)?;
            let inv = store(st(b(ONE)?.div(&b(sun.color_duration)?, p))?)?;
            let a0 = store(st(x0.mul(&b(inv)?, p))?)?;
            let y1 = st(x1.mul(&b(inv)?, p))?;
            let y2 = st(b(x2)?.mul(&b(inv)?, p))?;
            c[base] = store(st(b(a0)?.add(&b(c[base])?, p))?)?;
            c[base + 1] = store(st(y1.add(&b(c[base + 1])?, p))?)?;
            c[base + 2] = store(st(y2.add(&b(c[base + 2])?, p))?)?;
        }
        sun.current_colors = unflat(c);
        sun.color_duration = store(st(b(sun.color_duration)?.sub(&dtb, p))?)?;
    }
    if greater_or_unordered(sun.color_duration) {
        schedule(queues, owner, sun, now)?;
        sun.start_time = now;
    } else {
        sun.current_colors = sun.target_colors;
        sun.color_duration = 0;
        sun.start_time = now;
    }
    Ok(())
}

#[cfg(test)]
#[path = "live_sun_tests.rs"]
mod tests;
