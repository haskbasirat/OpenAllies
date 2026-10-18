//! Native NiTimeController89E9B0 clock, separated from key interpolation.
//! Only finite external-time controllers are supported; application-clock bit0
//! requires a real native clock service and is rejected. Shared cache mirrors
//! native global ordering, including its stored-f32 comparison boundary.
use crate::runtime::{Error, Result};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
const UNSET: F32 = F32(0xff7fffff);
fn n(x: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(x.0)?)
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ControllerClock {
    pub first: F32,
    pub last: F32,
    pub accumulated: F32,
}
impl Default for ControllerClock {
    fn default() -> Self {
        Self {
            first: UNSET,
            last: UNSET,
            accumulated: F32::ZERO,
        }
    }
}
impl ControllerClock {
    ///89E990 clears active bit separately; clock reset retains accumulator until
    ///first subsequent89E9B0 evaluation explicitly zeroes it.
    pub fn reset(&mut self) {
        self.first = UNSET;
        self.last = UNSET;
    }
}
#[derive(Clone, Debug, Default)]
pub struct ClockCache {
    last: Option<Cached>,
}
#[derive(Clone, Debug)]
struct Cached {
    start: F32,
    stop: F32,
    raw: F32,
    mode: u16,
    result: F32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Controller {
    pub flags: u16,
    pub frequency: F32,
    pub phase: F32,
    pub start: F32,
    pub stop: F32,
}
impl ControllerClock {
    pub fn update(
        &mut self,
        input: F32,
        c: Controller,
        cache: &mut ClockCache,
        p: Precision,
    ) -> Result<F32> {
        if c.flags & 1 != 0 {
            return Err(Error::Invalid(
                "application-time controller needs original clock provider",
            ));
        }
        let start = n(c.start)?;
        let stop = n(c.stop)?;
        if start.compare(&stop) == Ordering::Greater {
            return Err(Error::Invalid("reversed controller time range"));
        }
        let now = n(input)?;
        let frequency = n(c.frequency)?;
        let phase = n(c.phase)?;
        let mut next = self.clone();
        if self.first == UNSET {
            next.first = input;
        }
        let delta = if self.last == UNSET {
            next.accumulated = F32::ZERO;
            now
        } else {
            now.sub(&n(self.last)?, p)?
        };
        let accumulator = delta.mul(&frequency, p)?.add(&n(next.accumulated)?, p)?;
        next.last = input;
        next.accumulated = F32(accumulator.store_f32()?);
        let mut raw = accumulator.add(&phase, p)?;
        let mode = (c.flags >> 1) & 3;
        if let Some(old) = &cache.last
            && n(old.start)?.compare(&start) == Ordering::Equal
            && n(old.stop)?.compare(&stop) == Ordering::Equal
            && n(old.raw)?.compare(&raw) == Ordering::Equal
            && old.mode == mode
        {
            *self = next;
            return Ok(old.result);
        }
        let cached_raw = F32(raw.store_f32()?);
        if mode < 2 {
            let width = F32(stop.sub(&start, p)?.store_f32()?);
            let width = n(width)?;
            if width.is_zero() {
                raw = start.clone();
            } else if mode == 0 {
                raw = n(cached_raw)?
                    .sub(&start, p)?
                    .remainder_trunc(&width)?
                    .add(&start, p)?;
                if raw.compare(&start) == Ordering::Less {
                    raw = raw.add(&width, p)?;
                }
            } else {
                let period = n(F32(width.add(&width, p)?.store_f32()?))?;
                raw = n(cached_raw)?.remainder_trunc(&period)?;
                if raw.compare(&Binary::from_u32(0)) == Ordering::Less {
                    raw = raw.add(&period, p)?;
                }
                if raw.compare(&width) == Ordering::Greater {
                    raw = period.sub(&raw, p)?;
                }
                raw = raw.add(&start, p)?;
            }
        }
        if raw.compare(&stop) == Ordering::Greater {
            raw = stop.clone();
        } else if raw.compare(&start) == Ordering::Less {
            raw = start.clone();
        }
        if c.flags & 16 != 0 {
            raw = stop.sub(&raw.sub(&start, p)?, p)?;
        }
        let result = F32(raw.store_f32()?);
        cache.last = Some(Cached {
            start: c.start,
            stop: c.stop,
            raw: cached_raw,
            mode,
            result,
        });
        *self = next;
        Ok(result)
    }
}
///883F50 stores relative sequence time before invoking each controller. Native
///7170F0 sets the sequence offset to negative adjusted Animator start.
pub fn sequence_time(client_time: F32, adjusted_start: F32, p: Precision) -> Result<F32> {
    if adjusted_start.0 == 0x7f7fffff {
        return Err(Error::Invalid(
            "uninitialized native sequence offset requires first-frame setup",
        ));
    }
    Ok(F32(n(client_time)?
        .sub(&n(adjusted_start)?, p)?
        .store_f32()?))
}
