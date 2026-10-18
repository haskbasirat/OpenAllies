//! Body writes used by numeric property consequences and linked initialization.
//! Graph traversal remains in the actor runtime: it must capture the ratio
//! before linked561D40 dispatch, and reread the linked actor after each562200
//! setter step. These APIs expose those native sequencing boundaries explicitly.
use crate::{Body, Environment, Error, F32, HealthChange, Host, Result, properties::ModifierState};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaximumHealthPair {
    pub health: F32,
    pub maximum: F32,
}

impl Body {
    ///561D40's ratio store occurs before recursive linked-body recalculation,
    /// even if the caller later chooses not to preserve that ratio.
    pub fn capture_maximum_ratio(&self, p: Precision) -> Result<F32> {
        if self.maximum.compare(F32::ZERO)? != Ordering::Greater {
            return Err(Error::InvalidMaximum);
        }
        Ok(F32(self
            .health
            .number()?
            .div(&self.maximum.number()?, p)?
            .store_f32()?))
    }
    ///561D40 after its linked dispatch and derived45D730 getter. The returned
    /// maximum remains extended for scaling/comparison despite the f32 field
    /// store. A below-one health write returns before comparing against max.
    pub fn apply_extended_maximum(
        &mut self,
        ratio: F32,
        maximum: Binary,
        preserve: bool,
        p: Precision,
    ) -> Result<()> {
        ratio.number()?;
        self.health.number()?;
        let stored = F32(maximum.store_f32()?);
        if stored.compare(F32::ZERO)? != Ordering::Greater {
            return Err(Error::InvalidMaximum);
        }
        self.maximum = stored;
        if preserve {
            self.health = F32(ratio.number()?.mul(&maximum, p)?.store_f32()?);
        }
        if self.health.compare(F32::ONE)? == Ordering::Less {
            self.health = F32::ONE;
        } else if maximum.compare(&self.health.number()?) == Ordering::Less {
            self.health = stored;
        }
        Ok(())
    }
    ///561790/45D760: no modifier context returns the authored base unchanged;
    /// a real Modifier context executes501B10 before the binary32 body store.
    pub fn refresh_defense(
        &mut self,
        base: F32,
        modifier: Option<&ModifierState>,
        minimum: F32,
        p: Precision,
    ) -> Result<()> {
        self.defense = match modifier {
            Some(m) => m.defense(base, minimum, p)?,
            None => {
                base.number()?;
                base
            }
        };
        Ok(())
    }
    /// One562200 loop iteration, including its real source-aware561E50 setter.
    /// After success the runtime MUST resolve current actor+168 again and call
    /// this step on the next valid linked Body, using the returned clamped pair.
    pub fn set_maximum_health_step(
        &mut self,
        pair: MaximumHealthPair,
        env: &mut Environment,
        host: &mut impl Host,
        p: Precision,
    ) -> Result<MaximumHealthPair> {
        self.validate_fields(env)?;
        pair.health.number()?;
        pair.maximum.number()?;
        let minimum = F32(0x38d1_b717);
        let maximum = if pair.maximum.compare(minimum)? == Ordering::Less {
            minimum
        } else {
            pair.maximum
        };
        let health = if pair.health.compare(maximum)? == Ordering::Greater {
            maximum
        } else {
            pair.health
        };
        self.maximum = maximum;
        self.set_health_from_source(
            HealthChange {
                requested: health,
                token: 0,
                notify: true,
                source_id: None,
            },
            env,
            host,
            p,
        )?;
        Ok(MaximumHealthPair { health, maximum })
    }
}
