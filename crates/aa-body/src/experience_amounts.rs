//! Native experience amounts: value ratio `571820` and its four callers
//! `571960` (kill), `5719E0` (capture), `5718E0` (rout) and `5718B0`
//! (engagement). `credit::experience_amount` keeps its kill-credit specific
//! adapter; this module exposes the shared original routines to every caller
//! (capture reward, rout handler, engagement tick) without duplicating rules.
use crate::{F32, Result, credit::Object};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Resolved native settings-cache values (`B39AB0` table offsets). They are
/// constant for a running game; this API reads them as an immutable snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// `+160` ExperienceGainMinimum.
    pub minimum: F32,
    /// `+164` ExperienceGainMaximum.
    pub maximum: F32,
    /// `+168` ExperienceKillMultiplier.
    pub kill: F32,
    /// `+16C` ExperienceCaptureMultiplier.
    pub capture: F32,
    /// `+170` ExperienceRoutMultiplier.
    pub rout: F32,
    /// `+174` ExperienceEngageMultiplier.
    pub engagement: F32,
    /// `+178` ExperienceDefaultValue.
    pub default_value: F32,
}

/// The three clamped callers share one instruction stream except the factor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    /// `571960`.
    Kill,
    /// `5719E0`.
    Capture,
    /// `5718E0`.
    Rout,
}

pub trait World {
    /// Native `4D01E0(actor,0)` extended return. It may mutate world state;
    /// it is called once for the victim and then once for the source.
    fn actor_value(&mut self, actor: Object, precision: Precision) -> Result<Binary>;
    /// Victim definition `+4F4`, read only after a zero stored victim value.
    fn definition4f4(&self, actor: Object) -> Result<F32>;
}

fn number(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `571820(source, victim)`. A null victim returns `+0` without any
/// value call. The victim value is stored to binary32; zero is replaced by
/// `-1` when definition+4F4 is exactly `0xBF800000`, else by the default
/// setting. A nonpositive source value returns `+0`. Otherwise the result is
/// the extended `victim/source*0.5+0.5`, with no binary32 store.
pub fn ratio(
    source: Object,
    victim: Option<Object>,
    settings: &Settings,
    world: &mut impl World,
    precision: Precision,
) -> Result<Binary> {
    let Some(victim) = victim else {
        return Ok(Binary::from_u32(0));
    };
    let mut value = F32(world.actor_value(victim, precision)?.store_f32()?);
    if number(value)?.compare(&Binary::from_u32(0)) == Ordering::Equal {
        value = if world.definition4f4(victim)?.0 == 0xbf80_0000 {
            F32(0xbf80_0000)
        } else {
            settings.default_value
        };
    }
    let source_value = world.actor_value(source, precision)?;
    if source_value.compare(&Binary::from_u32(0)) != Ordering::Greater {
        return Ok(Binary::from_u32(0));
    }
    let half = Binary::from_f32_bits(0x3f00_0000)?;
    Ok(number(value)?
        .div(&source_value, precision)?
        .mul(&half, precision)?
        .add(&half, precision)?)
}

/// Native `571960`/`5719E0`/`5718E0`: the factor product is stored to
/// binary32, then clamped below by the minimum first and above by the
/// maximum. An inverted range therefore returns the minimum when the product
/// is below it, and otherwise the maximum when the product exceeds that.
pub fn clamped(
    kind: Kind,
    source: Object,
    victim: Option<Object>,
    settings: &Settings,
    world: &mut impl World,
    precision: Precision,
) -> Result<F32> {
    let factor = match kind {
        Kind::Kill => settings.kill,
        Kind::Capture => settings.capture,
        Kind::Rout => settings.rout,
    };
    let product = F32(ratio(source, victim, settings, world, precision)?
        .mul(&number(factor)?, precision)?
        .store_f32()?);
    if number(product)?.compare(&number(settings.minimum)?) == Ordering::Less {
        Ok(settings.minimum)
    } else if number(product)?.compare(&number(settings.maximum)?) == Ordering::Greater {
        Ok(settings.maximum)
    } else {
        Ok(product)
    }
}

/// Native `5718B0`: unclamped extended `ratio * ExperienceEngageMultiplier`.
pub fn engagement(
    source: Object,
    victim: Option<Object>,
    settings: &Settings,
    world: &mut impl World,
    precision: Precision,
) -> Result<Binary> {
    Ok(ratio(source, victim, settings, world, precision)?
        .mul(&number(settings.engagement)?, precision)?)
}
