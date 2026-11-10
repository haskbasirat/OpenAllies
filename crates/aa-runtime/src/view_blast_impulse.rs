//! View blast impulse `0x43B910` (view only: debris and loose view objects
//! pushed away from a blast; no simulation result reads it).
//!
//! The blast radius squared is stored once to binary32. Each object of the
//! list at `[this+94]+100` (linked through +8, `0x42EEC0`) of type 2 (+18)
//! whose virtual +9C holds is pushed when its squared distance from the blast
//! centre (binary32 offsets from its +38/+3C, the squared length stored to
//! binary32) less its virtual +38 radius squared (kept in the register) is
//! below the blast radius squared. The push is the table-normalized
//! direction (`0x43A970`, see `aa_trig::fast_normalize`) times a falloff
//! (1.0 at or inside the object's own radius, else 1 - excess / radius
//! squared on the stored excess) times the strength, delivered to
//! `0x455510` with a zero fourth argument.
//!
//! Proven at PC24, PC53 and PC64 by research/phase8-combat/native_view_blast.py
//! (the original code with virtual +9C, +38 and `0x455510` recorded).
use aa_numeric::{Fast, Precision, Result};
use aa_trig::fast_normalize::fast_normalize;
use std::cmp::Ordering;

/// The view object list and the calls `0x43B910` makes on it.
pub trait BlastObjects {
    type Object: Copy;
    /// The head of `[this+94]+100`.
    fn first(&mut self) -> Option<Self::Object>;
    /// Object +8.
    fn next(&mut self, object: Self::Object) -> Option<Self::Object>;
    /// Object +18.
    fn kind(&mut self, object: Self::Object) -> u32;
    /// Object virtual +9C (its low byte).
    fn reacts(&mut self, object: Self::Object) -> bool;
    /// Object +38, +3C (binary32 words).
    fn position(&mut self, object: Self::Object) -> [u32; 2];
    /// Object virtual +38, the x87 return.
    fn radius(&mut self, object: Self::Object) -> Fast;
    /// `0x455510(impulse, word)` on the object.
    fn apply(&mut self, object: Self::Object, impulse: [u32; 3], word: u32);
}

fn load(bits: u32) -> Result<Fast> {
    Fast::from_f32_bits(bits)
}

/// `0x43B910`: `center` x, y, `strength` and `radius` are binary32 words.
pub fn blast_impulse<W: BlastObjects>(
    world: &mut W,
    center: [u32; 2],
    strength: u32,
    radius: u32,
    table: &[u32; 256],
    p: Precision,
) -> Result<()> {
    let r = load(radius)?;
    let radius2 = r.mul(&r, p)?.store_f32()?;
    let mut node = world.first();
    while let Some(object) = node {
        if world.kind(object) == 2 && world.reacts(object) {
            let at = world.position(object);
            let own = world.radius(object);
            let dx = load(at[0])?.sub(&load(center[0])?, p)?.store_f32()?;
            let dy = load(at[1])?.sub(&load(center[1])?, p)?.store_f32()?;
            let distance2 = load(dy)?
                .mul(&load(dy)?, p)?
                .add(&load(dx)?.mul(&load(dx)?, p)?, p)?
                .store_f32()?;
            let excess = load(distance2)?.sub(&own.mul(&own, p)?, p)?;
            let stored_excess = excess.store_f32()?;
            if excess.compare(&load(radius2)?) == Ordering::Less {
                let unit = fast_normalize([dx, dy, 0], table, p)?;
                let falloff = if load(stored_excess)?.compare(&Fast::from_i32(0)) != Ordering::Greater {
                    Fast::from_i32(1)
                } else {
                    Fast::from_i32(1).sub(&load(stored_excess)?.div(&load(radius2)?, p)?, p)?
                };
                let s = load(strength)?;
                let x = load(unit[0])?.mul(&falloff, p)?.mul(&s, p)?.store_f32()?;
                let y = falloff.mul(&load(unit[1])?, p)?.mul(&s, p)?.store_f32()?;
                let z = load(load(unit[2])?.mul(&falloff, p)?.store_f32()?)?.mul(&s, p)?.store_f32()?;
                world.apply(object, [x, y, z], 0);
            }
        }
        node = world.next(object);
    }
    Ok(())
}
