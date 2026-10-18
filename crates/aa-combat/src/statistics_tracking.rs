//! Kingdom statistics class tracking on aa-economy's authoritative
//! `Statistics` (GKingdom+0x32C) and the actor's class word (+0x13C):
//! `0x58F010` track, `0x58EFE0` untrack, their helpers `0x58E8B0` (enter a
//! class) / `0x58EBA0` (leave the class) / `0x58E890` (score 33 maximum),
//! and the `0x58EA80` predicate. Callers: capture `0x4D5230` (new owner) and
//! the kingdom registry add/remove actor (`aa_economy::kingdom_registry`
//! `RegistryWorld::statistics_track` / `statistics_classified`).
//!
//! Every score change goes through `Statistics::add_score` (`0x58E870`
//! arithmetic: binary32(PC(amount + score))), which equals the native
//! `fadd 1.0` / `fsub 1.0` / `fsubr int` forms; the score-33 maximum is the
//! only direct store.
use crate::{Error, Result};
use aa_economy::{
    F32Bits, X87Precision,
    statistics::{Statistics, StructureFacts, structure_class, structure_scored},
};
use aa_numeric::{Binary, Precision};

/// Class word value meaning "not classified" (`0x38`).
pub const NO_CLASS: u32 = 0x38;
const ONE: F32Bits = F32Bits(0x3f80_0000);
const MINUS_ONE: F32Bits = F32Bits(0xbf80_0000);

fn x87(p: Precision) -> X87Precision {
    match p {
        Precision::Single24 => X87Precision::Single24,
        Precision::Double53 => X87Precision::Double53,
        Precision::Extended64 => X87Precision::Extended64,
    }
}
fn economy(e: aa_economy::EconomyError) -> Error {
    Error::World(e.to_string())
}

/// Actor facts read by `0x58F010`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackFacts {
    /// Actor +7C component present.
    pub component_7c: bool,
    /// Actor +D8 component present.
    pub component_d8: bool,
    /// Structure facts (+94/virtual +100, +9C, +43C list, +4E0, +88, +90);
    /// `0x58EE00` is aa-economy `structure_class`.
    pub structure: StructureFacts,
    /// ActorData +4D0 nonzero.
    pub definition_4d0: bool,
    /// Actor +98 component's +10 integer, when the component exists.
    pub component_98: Option<i32>,
    /// Actor +D4 component present.
    pub component_d4: bool,
}

fn add(stats: &mut Statistics, index: usize, amount: F32Bits, p: Precision) -> Result<()> {
    stats.add_score(index, amount, x87(p)).map_err(economy)
}

/// `0x58E8B0(actor, class, amount)`: class word := class, then the class's
/// counters (+1; class 50/51 add `amount` to score 51; class 52 raises the
/// score-33 maximum through `0x58E890`).
fn enter(
    stats: &mut Statistics,
    class_word: &mut u32,
    class: u32,
    amount: F32Bits,
    p: Precision,
) -> Result<()> {
    *class_word = class;
    match class {
        47 => add(stats, 47, ONE, p),
        48 => add(stats, 48, ONE, p),
        49 => {
            add(stats, 48, ONE, p)?;
            add(stats, 49, ONE, p)
        }
        50 | 51 => {
            add(stats, 50, ONE, p)?;
            add(stats, 51, amount, p)
        }
        52 => {
            add(stats, 52, ONE, p)?;
            let value = stats.score(52).map_err(economy)?;
            let maximum = stats.score(33).map_err(economy)?;
            if Binary::from_f32_bits(maximum.0)?.compare(&Binary::from_f32_bits(value.0)?)
                == std::cmp::Ordering::Less
            {
                stats.scores[33] = value;
            }
            Ok(())
        }
        53 => add(stats, 53, ONE, p),
        54 => {
            add(stats, 53, ONE, p)?;
            add(stats, 54, ONE, p)
        }
        55 => add(stats, 55, ONE, p),
        _ => Ok(()),
    }
}

/// `0x58EBA0(actor)`: class word := [`NO_CLASS`], then the old class's
/// counters (-1; class 50/51 subtract the +98 component's +10 plus one from
/// score 51, a null component being a native fault).
fn leave(
    stats: &mut Statistics,
    class_word: &mut u32,
    component_98: Option<i32>,
    p: Precision,
) -> Result<()> {
    let old = std::mem::replace(class_word, NO_CLASS);
    match old {
        47 => add(stats, 47, MINUS_ONE, p),
        48 => add(stats, 48, MINUS_ONE, p),
        49 => {
            add(stats, 48, MINUS_ONE, p)?;
            add(stats, 49, MINUS_ONE, p)
        }
        50 | 51 => {
            let count = component_98
                .ok_or(Error::Invalid(
                    "class 50/51 actor without its +98 component",
                ))?
                .wrapping_add(1);
            add(stats, 50, MINUS_ONE, p)?;
            let amount = count as f32;
            if f64::from(amount) != f64::from(count) {
                return Err(Error::Invalid("+98 count is not exact in binary32"));
            }
            add(stats, 51, F32Bits((-amount).to_bits()), p)
        }
        52 => add(stats, 52, MINUS_ONE, p),
        53 => add(stats, 53, MINUS_ONE, p),
        54 => {
            add(stats, 53, MINUS_ONE, p)?;
            add(stats, 54, MINUS_ONE, p)
        }
        55 => add(stats, 55, MINUS_ONE, p),
        _ => Ok(()),
    }
}

/// One `0x58F010` step: with tracking enabled (`[0xB36534]+0x274`) and the
/// actor not already in `class`, leave the current class (unless
/// unclassified) and enter `class`.
fn step(
    stats: &mut Statistics,
    class_word: &mut u32,
    enabled: bool,
    class: u32,
    amount: F32Bits,
    component_98: Option<i32>,
    p: Precision,
) -> Result<()> {
    if !enabled || *class_word == class {
        return Ok(());
    }
    if *class_word != NO_CLASS {
        leave(stats, class_word, component_98, p)?;
    }
    enter(stats, class_word, class, amount, p)
}

/// Native `0x58F010(statistics; actor)`.
pub fn track(
    stats: &mut Statistics,
    class_word: &mut u32,
    facts: &TrackFacts,
    enabled: bool,
    p: Precision,
) -> Result<()> {
    let c98 = facts.component_98;
    if facts.component_7c && !facts.component_d8 {
        step(stats, class_word, enabled, 47, MINUS_ONE, c98, p)?;
    }
    if structure_scored(&facts.structure) {
        let class = if facts.definition_4d0 { 49 } else { 48 };
        step(stats, class_word, enabled, class, MINUS_ONE, c98, p)?;
    }
    if let Some(count) = c98 {
        let amount = F32Bits(Binary::from_i32(count.wrapping_add(1)).store_f32()?);
        step(stats, class_word, enabled, 50, amount, c98, p)?;
    }
    let class = structure_class(&facts.structure);
    if class != 0 {
        step(stats, class_word, enabled, 53, MINUS_ONE, c98, p)?;
        if facts.definition_4d0 {
            step(stats, class_word, enabled, 54, MINUS_ONE, c98, p)?;
        }
        if class == 1 {
            step(stats, class_word, enabled, 52, MINUS_ONE, c98, p)?;
        }
    }
    if facts.component_d4 {
        step(stats, class_word, enabled, 55, MINUS_ONE, c98, p)?;
    }
    Ok(())
}

/// Native `0x58EFE0(statistics; actor)`.
pub fn untrack(
    stats: &mut Statistics,
    class_word: &mut u32,
    component_98: Option<i32>,
    enabled: bool,
    p: Precision,
) -> Result<()> {
    if enabled && *class_word != NO_CLASS {
        leave(stats, class_word, component_98, p)?;
    }
    Ok(())
}

/// Native `0x58EA80(actor)`: +80 present, +D8 absent and +A4 present.
pub fn classified(component_80: bool, component_d8: bool, component_a4: bool) -> bool {
    component_80 && !component_d8 && component_a4
}
