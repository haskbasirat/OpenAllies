//! 4C0A80 damage-type display fields. The original damage_filter groups damage
//! animations; it is not the damage/armor multiplier used in hit calculation.
use crate::{Error, Result};
use aa_body::F32;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageTypePresentation {
    pub color: [F32; 4],
    pub glyph: u32,
    pub animation_filter: i32,
}

pub trait Source {
    ///47D960 base IDS/localized-name loading is owned by the typed rule registry.
    fn load_base_definition(&mut self) -> Result<()>;
    fn color(&mut self) -> Result<[F32; 3]>;
    ///Includes the original glyph string read and registered glyph lookup.
    fn glyph(&mut self) -> Result<u32>;
    fn animation_filter(&mut self) -> Result<i32>;
}

/// Writes follow native order; an invalid glyph/filter leaves earlier fields
/// written, like the original exception path. Finite authored color bits are
/// copied without a color-range clamp; nonfinite colors are outside this API.
pub fn load(state: &mut DamageTypePresentation, source: &mut impl Source) -> Result<()> {
    source.load_base_definition()?;
    let [r, g, b] = source.color()?;
    for component in [r, g, b] {
        aa_numeric::Binary::from_f32_bits(component.0)?;
    }
    state.color = [r, g, b, F32::ONE];
    state.glyph = source.glyph()?;
    if state.glyph == 0 {
        return Err(Error::Invalid("unknown damage-type glyph"));
    }
    state.animation_filter = source.animation_filter()?;
    if state.animation_filter < 0 {
        return Err(Error::Invalid("negative damage animation filter"));
    }
    Ok(())
}
