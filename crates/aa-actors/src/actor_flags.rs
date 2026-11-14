//! Small native actor field helpers used inside per-tick callers:
//!
//! * `4D07D0(bit, set)`: a 64-bit state word gets `_allshl(1, bit)` set or
//!   cleared (only the low byte of `bit` counts; 64 and up shift to zero).
//! * `4D0960`: `+100` bit 30.
//! * `4CFFD0`: the actor when its `+D8` is set, else its organization when
//!   that one's `+D8` is set; nothing when actor `+8` bit 0 is set.
//! * `4CE880`: clears `+164`.
//! * `4CEE40`: `+98` set, or `+9C` set and virtual `+100` answering 0.
//! * `4CEFA0`: the membership's organization (`[+80]+10`), else virtual
//!   `+100`'s answer.
//! * `4D1240(object)` / `4D1290`: the reference-counted `+154` object (count
//!   at its `+4`; the old one is released through its virtual `+0(1)` when
//!   its count reaches zero).
//! * `45E660(index)`: an ActorData stance index, or the last one (`+1C0 - 1`)
//!   for -1 or an empty `+1BC` slot; -1 without stances.
//! * `471A40(index)`: words `+18` / `+1C` / `+20` for 0 / 1 / 2, else 0.
//!
//! Oracle: research/phase8-combat/native_actor_flags.py.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("actor field callback failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, Error>;

pub trait World {
    /// The 32-bit word at `thing + offset`.
    fn word(&mut self, thing: u32, offset: u32) -> Result<u32>;
    fn set_word(&mut self, thing: u32, offset: u32, value: u32) -> Result<()>;
    /// Actor virtual `+100`.
    fn virtual100(&mut self, actor: u32) -> Result<u32>;
    /// Object virtual `+0(1)` (release).
    fn release(&mut self, object: u32) -> Result<()>;
}

/// `_allshl(1, count)` as `4D07D0` uses it.
fn bit_mask(count: u32) -> u64 {
    let cl = count & 0xFF;
    if cl >= 64 { 0 } else { 1u64 << cl }
}

/// Native `4D07D0(bit, set)` on a 64-bit state word.
pub fn with_state_bit(word: u64, bit: u32, set: bool) -> u64 {
    let mask = bit_mask(bit);
    if set { word | mask } else { word & !mask }
}

/// Native `4D0960`.
pub fn flag_bit30(word100: u32) -> bool {
    word100 & 0x4000_0000 != 0
}

/// Native `4CFFD0` on `actor`.
pub fn d8_holder(world: &mut impl World, actor: u32) -> Result<Option<u32>> {
    if world.word(actor, 8)? & 1 != 0 {
        return Ok(None);
    }
    if world.word(actor, 0xD8)? != 0 {
        return Ok(Some(actor));
    }
    let membership = world.word(actor, 0x80)?;
    if membership == 0 {
        return Ok(None);
    }
    let organization = world.word(membership, 0x10)?;
    if organization == 0 || world.word(organization, 0xD8)? == 0 {
        return Ok(None);
    }
    Ok(Some(organization))
}

/// Native `4CE880` on `actor`.
pub fn clear_164(world: &mut impl World, actor: u32) -> Result<()> {
    world.set_word(actor, 0x164, 0)
}

/// Native `4CEE40` on `actor`.
pub fn has_98_or_detached_9c(world: &mut impl World, actor: u32) -> Result<bool> {
    if world.word(actor, 0x98)? != 0 {
        return Ok(true);
    }
    Ok(world.word(actor, 0x9C)? != 0 && world.virtual100(actor)? == 0)
}

/// Native `4CEFA0` on `actor`: its returned word.
pub fn organization_or_virtual100(world: &mut impl World, actor: u32) -> Result<u32> {
    let membership = world.word(actor, 0x80)?;
    if membership != 0 {
        let organization = world.word(membership, 0x10)?;
        if organization != 0 {
            return Ok(organization);
        }
    }
    world.virtual100(actor)
}

fn release_154(world: &mut impl World, old: u32) -> Result<()> {
    let count = world.word(old, 4)?.wrapping_sub(1);
    world.set_word(old, 4, count)?;
    if count == 0 {
        world.release(old)?;
    }
    Ok(())
}

/// Native `4D1240(object)` on `actor` (0 for no object).
pub fn set_154(world: &mut impl World, actor: u32, object: u32) -> Result<()> {
    let old = world.word(actor, 0x154)?;
    if old == object {
        return Ok(());
    }
    if object != 0 {
        let count = world.word(object, 4)?.wrapping_add(1);
        world.set_word(object, 4, count)?;
    }
    let old = world.word(actor, 0x154)?;
    if old != 0 {
        release_154(world, old)?;
    }
    world.set_word(actor, 0x154, object)
}

/// Native `4D1290` on `actor`.
pub fn clear_154(world: &mut impl World, actor: u32) -> Result<()> {
    let old = world.word(actor, 0x154)?;
    if old != 0 {
        release_154(world, old)?;
        world.set_word(actor, 0x154, 0)?;
    }
    Ok(())
}

/// Native `45E660(index)` on an ActorData.
pub fn stance_or_last(world: &mut impl World, data: u32, index: i32) -> Result<i32> {
    let count = world.word(data, 0x1C0)?;
    if count == 0 {
        return Ok(-1);
    }
    if index == -1 {
        return Ok(count.wrapping_sub(1) as i32);
    }
    let slots = world.word(data, 0x1BC)?;
    if world.word(slots, (index as u32).wrapping_mul(4))? == 0 {
        return Ok(count.wrapping_sub(1) as i32);
    }
    Ok(index)
}

/// Native `471A40(index)`.
pub fn indexed_word(world: &mut impl World, object: u32, index: i32) -> Result<u32> {
    match index {
        0 => world.word(object, 0x18),
        1 => world.word(object, 0x1C),
        2 => world.word(object, 0x20),
        _ => Ok(0),
    }
}
