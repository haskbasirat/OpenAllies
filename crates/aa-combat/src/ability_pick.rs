//! Original aircraft Mind `GetValidAbility` `51A260` (thiscall Mind,
//! `(a, b, start, d, out*, enabled_only)`, `ret 0x18`): the ability an
//! aircraft releases and its status class.
//!
//! An attack descriptor is built once (`4C8300(1)`, `+8` = the Mind's owner,
//! `+1C`/`+20`/`+24`/`+28` = the arguments `a`, `b`, `start`, `d`). The
//! iteration (`517F60`) then starts at index 0 (the `start` argument only
//! reaches the descriptor) at the first ability whose `+18` byte is set, and
//! each step moves to the next such ability. With `enabled_only` (the low
//! byte of the last argument) an ability whose definition (`+4`) `+54` bit 3
//! is clear is skipped. `4C8F20(descriptor, 1)` ([`crate::admission`])
//! classifies each remaining ability:
//! - 0 -> class 0; 1, 2, 3 -> skipped; 5 -> class 2 when the owner has a
//!   Mover (`+6C`), else skipped; 6, 8 -> class 1; 7 -> class 3 when the
//!   owner definition's `+1E8` (mana regeneration) compares equal to 0.0,
//!   else class 1;
//! - 4 and any status above 8 keep the class register as it was: the
//!   previous class, or for the first such ability the whole last argument
//!   word.
//!
//! The first classified ability is the best. Later ones replace it when the
//! best's class is not 0 and: the class is 0; or the class is 2 and the
//! best's is not; or the classes are equal or both in {1, 3}, by the
//! tie-break. Tie-break (the owner's modifier `4CF110` read each time):
//! class 2 compares range `4C8270(modifier)` (the best's stored to binary32
//! minus the new one's on the x87 stack): a negative difference takes the
//! new ability. Otherwise recharge `4C81C0` likewise: positive keeps, negative
//! takes, equal (or unordered) compares the ranges: best less than new keeps,
//! greater takes, equal draws `85D840(4F02B0())` and an odd draw takes the
//! new ability. The class is not updated by a tie-break. `*out` receives the
//! best ability (or null) and the best class (3 when none) is returned.
use crate::Result;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// The reads and calls of `51A260`, in native order. Ability indices are
/// the Performer (`owner+74`) `+14` array positions.
pub trait AbilityPickWorld {
    type Modifier: Copy;
    /// `4C8300(1)` on the local descriptor and its `+8`/`+1C..+28` stores.
    fn begin_descriptor(&mut self, words: [u32; 4]) -> Result<()>;
    /// Fact: Performer `+18` (signed count).
    fn ability_count(&mut self) -> Result<i32>;
    /// Fact: ability `+18` byte (enabled) is nonzero.
    fn ability_enabled(&mut self, index: i32) -> Result<bool>;
    /// Fact: ability definition `+54` bit 3.
    fn ability_flag_54_bit3(&mut self, index: i32) -> Result<bool>;
    /// `4C8F20(descriptor, 1)` on the ability: its status.
    fn admission(&mut self, index: i32) -> Result<u32>;
    /// Fact: owner `+6C` (Mover) is nonnull.
    fn owner_has_mover(&mut self) -> Result<bool>;
    /// Fact: owner definition `+1E8` binary32 bits.
    fn owner_mana_regeneration(&mut self) -> Result<u32>;
    /// `4CF110` on the owner.
    fn owner_modifier(&mut self) -> Result<Self::Modifier>;
    /// `4C81C0` on the ability (ST0, unstored).
    fn recharge(&mut self, index: i32) -> Result<Binary>;
    /// `4C8270(modifier)` on the ability (ST0, unstored).
    fn range(&mut self, index: i32, modifier: Self::Modifier) -> Result<Binary>;
    /// `85D840(4F02B0())` on GWorld: the draw.
    fn random_draw(&mut self) -> Result<u32>;
}

/// The result of `51A260`: `*out` and the returned class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pick {
    pub ability: Option<i32>,
    pub class: u32,
}

fn next_enabled<W: AbilityPickWorld + ?Sized>(w: &mut W, mut index: i32, count: i32) -> Result<i32> {
    while index < count && index >= 0 && !w.ability_enabled(index)? {
        index += 1;
    }
    Ok(index)
}

/// `stored(best) - new` against 0.0 at the caller's precision.
fn difference(best: &Binary, new: &Binary, p: Precision) -> Result<Ordering> {
    let best = Binary::from_f32_bits(best.store_f32()?)?;
    Ok(best.sub(new, p)?.compare(&Binary::from_f32_bits(0)?))
}

/// `51A260`. `last_word` is the whole last argument (its low byte is the
/// enabled-only flag).
pub fn pick<W: AbilityPickWorld + ?Sized>(
    world: &mut W,
    words: [u32; 4],
    last_word: u32,
    precision: Precision,
) -> Result<Pick> {
    let p = precision;
    world.begin_descriptor(words)?;
    let mut best: Option<i32> = None;
    let mut best_class = 3u32;
    let mut class = last_word;
    let count = world.ability_count()?;
    let mut index = next_enabled(world, 0, count)?;
    while index < count && index >= 0 {
        let skip = last_word & 0xff != 0 && !world.ability_flag_54_bit3(index)?;
        let considered = !skip
            && match world.admission(index)? {
                0 => {
                    class = 0;
                    true
                }
                1..=3 => false,
                5 => {
                    let mover = world.owner_has_mover()?;
                    if mover {
                        class = 2;
                    }
                    mover
                }
                6 | 8 => {
                    class = 1;
                    true
                }
                7 => {
                    let regeneration = world.owner_mana_regeneration()?;
                    class = if regeneration & 0x7fff_ffff == 0 { 3 } else { 1 };
                    true
                }
                _ => true,
            };
        if considered {
            match best {
                None => {
                    best = Some(index);
                    best_class = class;
                }
                Some(current) => {
                    let tie = if best_class == class {
                        true
                    } else if best_class == 0 {
                        false
                    } else if class == 0 {
                        best = Some(index);
                        best_class = class;
                        false
                    } else if best_class == 2 {
                        false
                    } else if class != 2 {
                        true
                    } else {
                        best = Some(index);
                        best_class = class;
                        false
                    };
                    if tie && tie_break(world, current, index, class, p)? {
                        best = Some(index);
                    }
                }
            }
        }
        index = next_enabled(world, index + 1, count)?;
    }
    Ok(Pick { ability: best, class: best_class })
}

/// The tie-break between the best ability and the new one: true takes the new.
fn tie_break<W: AbilityPickWorld + ?Sized>(
    w: &mut W,
    best: i32,
    new: i32,
    class: u32,
    p: Precision,
) -> Result<bool> {
    let modifier = w.owner_modifier()?;
    if class == 2 {
        let a = w.range(best, modifier)?;
        let b = w.range(new, modifier)?;
        return Ok(difference(&a, &b, p)? == Ordering::Less);
    }
    let a = w.recharge(best)?;
    let b = w.recharge(new)?;
    match difference(&a, &b, p)? {
        Ordering::Greater => Ok(false),
        Ordering::Less => Ok(true),
        Ordering::Equal => {
            let a = Binary::from_f32_bits(w.range(best, modifier)?.store_f32()?)?;
            let b = w.range(new, modifier)?;
            match a.compare(&b) {
                Ordering::Less => Ok(false),
                Ordering::Greater => Ok(true),
                Ordering::Equal => Ok(w.random_draw()? & 1 != 0),
            }
        }
    }
}
