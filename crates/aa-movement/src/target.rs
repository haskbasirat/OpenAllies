//! Native576AD0 target assignment. The target generation ID is written before
//! the range arithmetic; failures do not undo that write.
use crate::{MoverState, Result};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub fn set_target(
    mover: &mut MoverState,
    target_id: Option<u32>,
    range_bits: u32,
    world_delta_bits: u32,
    precision: Precision,
) -> Result<()> {
    mover.target_id = target_id.unwrap_or(0);
    let step = Binary::from_f32_bits(world_delta_bits)?.mul(
        &Binary::from_f32_bits(mover.requested_speed.to_bits())?,
        precision,
    )?;
    let range = Binary::from_f32_bits(range_bits)?;
    let chosen = if range.compare(&step) == Ordering::Greater {
        range_bits
    } else {
        step.store_f32()?
    };
    mover.target_range = Some(f32::from_bits(chosen));
    Ok(())
}
