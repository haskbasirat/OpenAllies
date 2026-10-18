//! Native clock and five-queue initialization within GWorld Init4F27E0.
//! This does not replace the surrounding map, manager, registry or actor setup.
use crate::queues::{Clock, Kind, Queues, period_ticks};
use aa_numeric::{Binary, Precision};
use aa_simulation::{FloatBits, NativeClock};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Numeric(#[from] aa_numeric::Error),
    #[error(transparent)]
    Queue(#[from] crate::queues::Error),
    #[error("GameTicksPerGameSecond must resolve to a finite positive native scalar")]
    TickRate,
    #[error("queue initialization precision differs from its native clock plan")]
    Precision,
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuePlan {
    pub id: u8,
    pub kind: Kind,
    pub bucket_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClockPlan {
    pub precision: Precision,
    /// Stored binary32 reciprocal from4F2885..4F28A0.
    pub step_bits: u32,
    pub queues: [QueuePlan; 5],
}
impl ClockPlan {
    /// The caller supplies the resolved native SVar, not a guessed tick rate.
    pub fn from_tick_rate(rate_bits: u32, precision: Precision) -> Result<Self> {
        let rate = Binary::from_f32_bits(rate_bits)?;
        if rate.is_zero() || rate.is_negative_nonzero() {
            return Err(Error::TickRate);
        }
        let step_bits = Binary::from_i32(1).div(&rate, precision)?.store_f32()?;
        let mut queues = [QueuePlan {
            id: 0,
            kind: Kind::Periodic,
            bucket_count: 0,
        }; 5];
        // Original world call sites4F297E..4F29C9. Deferred constructors
        // initialize a five-second ring, with capacity growth kept separate.
        for (index, period) in [
            0x38d1_b717,
            0x3f80_0000,
            0x40a0_0000,
            0x40a0_0000,
            0x40a0_0000,
        ]
        .into_iter()
        .enumerate()
        {
            let count = period_ticks(period, step_bits, precision)?;
            if count <= 0 {
                return Err(crate::queues::Error::BucketCount.into());
            }
            queues[index] = QueuePlan {
                id: index as u8,
                kind: if index < 3 {
                    Kind::Periodic
                } else {
                    Kind::Deferred
                },
                bucket_count: count,
            };
        }
        Ok(Self {
            precision,
            step_bits,
            queues,
        })
    }
    pub fn new_clock(&self) -> NativeClock {
        NativeClock {
            tick: 0,
            begin: FloatBits::ZERO,
            end: FloatBits::ZERO,
            step: FloatBits(self.step_bits),
        }
    }
    pub fn initialize_queues(&self, clock: NativeClock, precision: Precision) -> Result<Queues> {
        if precision != self.precision {
            return Err(Error::Precision);
        }
        if clock.step.0 != self.step_bits {
            return Err(Error::TickRate);
        }
        let mut out = Queues::new(
            Clock {
                begin_bits: clock.begin.0,
                end_bits: clock.end.0,
                step_bits: clock.step.0,
            },
            precision,
        )?;
        for plan in self.queues {
            out.initialize(plan.id, plan.kind, plan.bucket_count)?;
        }
        Ok(out)
    }
}
