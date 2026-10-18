//! Native-evidence resource arithmetic, not a complete gameplay economy.
//!
//! All operations require explicit resource registry order and an explicit x87
//! precision. Only finite binary32 states, round-to-nearest/ties-even, and the
//! documented function boundaries are supported. Scheduling, purchases' caller
//! authorization, bankruptcy callbacks and actor side effects are not invented.
#![forbid(unsafe_code)]

mod binary;
pub mod kingdom_load;
pub mod kingdom_registry;
pub mod kingdom_init;
pub mod kingdom_start;
pub mod restrictions;
pub mod statistics;
pub mod statistics_live;
pub mod supply;
pub mod tick;
pub mod transfers;
pub mod world_start;
pub mod x87;
use binary::Binary;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;

pub const EXPANDED_PROVIDER_LIMIT: i32 = 100;
pub const SECONDS_TO_MINUTES_BITS: F32Bits = F32Bits(0x3c88_8889);

/// One reached stockpile row of native4E39D0. This shared leaf lets a caller
/// mutate its actual Kingdom+1C4 vector without creating a second Economy
/// owner. The binary32 store occurs before the upper-bound comparison.
pub fn credit_stockpile_cell(
    current: F32Bits,
    delta: F32Bits,
    maximum: F32Bits,
    precision: X87Precision,
) -> Result<F32Bits, EconomyError> {
    Ok(clamp_stockpile_upper(credit_stockpile_sum(current,delta,precision)?,maximum))
}

///4E39F1..4E3A12 first binary32 store, BEFORE the lazy current maximum read.
pub fn credit_stockpile_sum(current:F32Bits,delta:F32Bits,precision:X87Precision)->Result<F32Bits,EconomyError> {
    let result = Binary::from_bits(delta).add(&Binary::from_bits(current), precision);
    Ok(if result.cmp(&Binary::from_bits(F32Bits::ZERO)) != Ordering::Greater {
        F32Bits::ZERO
    } else {
        result.store_f32()?
    })
}

///4E3A22..4E3A37 upper-bound comparison after the first store. Only compares
/// binary32 inputs; unordered maximum is ignored and signed zero is retained.
pub fn clamp_stockpile_upper(current:F32Bits,maximum:F32Bits)->F32Bits {
    if maximum.as_f32()<current.as_f32() {
        maximum
    } else {
        current
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct F32Bits(pub u32);
impl F32Bits {
    pub const ZERO: Self = Self(0);
    pub fn from_f32(value: f32) -> Self {
        Self(value.to_bits())
    }
    pub fn as_f32(self) -> f32 {
        f32::from_bits(self.0)
    }
    pub fn is_finite(self) -> bool {
        self.0 & 0x7f80_0000 != 0x7f80_0000
    }
}

/// The x87 precision-control field affects each arithmetic instruction, while
/// FST/FSTP binary32 stores still round separately. No global/native FPU mode is
/// assumed: callers must select the mode established by their parity context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum X87Precision {
    Single24,
    Double53,
    Extended64,
}
impl X87Precision {
    pub fn control_word(self) -> u16 {
        match self {
            Self::Single24 => 0x007f,
            Self::Double53 => 0x027f,
            Self::Extended64 => 0x037f,
        }
    }
    pub(crate) fn significand_bits(self) -> u64 {
        match self {
            Self::Single24 => 24,
            Self::Double53 => 53,
            Self::Extended64 => 64,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceKind {
    Stockpiled,
    Upkeep,
    LimitedConsumed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub id: String,
    pub kind: ResourceKind,
    pub limited_provider: bool,
    /// Dense coefficients in the same explicit registry order as `Catalog`.
    /// Only STOCKPILED destination coefficients are consumed by 0x4E1F60.
    pub purchase_cost: Vec<F32Bits>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalog {
    pub resources: Vec<ResourceDefinition>,
}

/// Direct snapshots of native kingdom arrays. `consumed` already includes the
/// currently applied shortage charges; it is not merely raw actor upkeep.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub raw_provided: Vec<F32Bits>,
    pub effective_provided: Vec<F32Bits>,
    pub consumed: Vec<F32Bits>,
    pub stockpiled: Vec<F32Bits>,
}
impl State {
    pub fn zeroed(count: usize) -> Self {
        Self {
            raw_provided: vec![F32Bits::ZERO; count],
            effective_provided: vec![F32Bits::ZERO; count],
            consumed: vec![F32Bits::ZERO; count],
            stockpiled: vec![F32Bits::ZERO; count],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Native signed integer EconomyLimitedResourceMax, expanded build = 100.
    pub provided_maximum: i32,
    /// Native float EconomyStockpiledResourceMax, supplied by the resolved rules.
    pub stockpiled_maximum: F32Bits,
}

/// Observations at complete shortage helper returns, preserving invocation order.
/// These are laboratory traces, not invented gameplay/event-bus messages.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShortageStep {
    pub add: bool,
    pub consumed: Vec<F32Bits>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationTrace {
    pub shortage_steps: Vec<ShortageStep>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowArithmetic {
    pub minute_fraction: F32Bits,
    pub gross_production: Vec<F32Bits>,
    pub net_change: Vec<F32Bits>,
    /// Native local BL at 0x4E9DA6, before bankruptcy/event processing.
    pub stock_went_negative: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EconomyError {
    Shape,
    DuplicateId,
    NonFiniteInput,
    NonFiniteResult,
    ResourceIndex,
    /// x87 division by zero (native would produce an infinity/NaN ratio).
    DivisionByZero,
    /// `_ftol` result outside signed 32-bit range.
    IntegerRange,
    /// A required world callback failed; earlier native effects remain applied.
    World(String),
}
impl fmt::Display for EconomyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "economy laboratory: {self:?}")
    }
}
impl std::error::Error for EconomyError {}

pub struct Economy {
    catalog: Catalog,
    state: State,
    limits: Limits,
    precision: X87Precision,
}

fn validate_floats(values: &[F32Bits]) -> Result<(), EconomyError> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(EconomyError::NonFiniteInput)
    }
}

impl Economy {
    pub fn new(
        catalog: Catalog,
        state: State,
        limits: Limits,
        precision: X87Precision,
    ) -> Result<Self, EconomyError> {
        let count = catalog.resources.len();
        for (index, definition) in catalog.resources.iter().enumerate() {
            if definition.purchase_cost.len() != count {
                return Err(EconomyError::Shape);
            }
            if catalog.resources[..index]
                .iter()
                .any(|previous| previous.id == definition.id)
            {
                return Err(EconomyError::DuplicateId);
            }
            validate_floats(&definition.purchase_cost)?;
        }
        for values in [
            &state.raw_provided,
            &state.effective_provided,
            &state.consumed,
            &state.stockpiled,
        ] {
            if values.len() != count {
                return Err(EconomyError::Shape);
            }
            validate_floats(values)?;
        }
        validate_floats(&[limits.stockpiled_maximum])?;
        Ok(Self {
            catalog,
            state,
            limits,
            precision,
        })
    }
    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }
    pub fn precision(&self) -> X87Precision {
        self.precision
    }

    fn check_vector(&self, values: &[F32Bits]) -> Result<(), EconomyError> {
        if values.len() != self.catalog.resources.len() {
            return Err(EconomyError::Shape);
        }
        validate_floats(values)
    }

    fn shortage(
        &self,
        state: &mut State,
        add: bool,
        trace: &mut MutationTrace,
    ) -> Result<(), EconomyError> {
        // Exact ascending nested loops and binary32 spill at 0x4E1F92. Product
        // and destination add/sub remain x87 precision until the 0x4E1FF3 store.
        for (source, definition) in self.catalog.resources.iter().enumerate() {
            let difference = Binary::from_bits(state.effective_provided[source])
                .sub(&Binary::from_bits(state.consumed[source]), self.precision);
            let stored_difference = difference.store_f32()?;
            if difference.is_negative_nonzero() && definition.kind == ResourceKind::Upkeep {
                for (destination, target) in self.catalog.resources.iter().enumerate() {
                    if target.kind == ResourceKind::Stockpiled {
                        let charge = Binary::from_bits(stored_difference).neg().mul(
                            &Binary::from_bits(definition.purchase_cost[destination]),
                            self.precision,
                        );
                        let old = Binary::from_bits(state.consumed[destination]);
                        let result = if add {
                            charge.add(&old, self.precision)
                        } else {
                            old.sub(&charge, self.precision)
                        };
                        state.consumed[destination] = result.store_f32()?;
                    }
                }
            }
        }
        trace.shortage_steps.push(ShortageStep {
            add,
            consumed: state.consumed.clone(),
        });
        Ok(())
    }

    fn provider(
        &self,
        state: &mut State,
        index: usize,
        delta: F32Bits,
        trace: &mut MutationTrace,
    ) -> Result<(), EconomyError> {
        self.shortage(state, false, trace)?;
        state.raw_provided[index] = Binary::from_bits(delta)
            .add(
                &Binary::from_bits(state.raw_provided[index]),
                self.precision,
            )
            .store_f32()?;
        let raw = Binary::from_bits(state.raw_provided[index]);
        let cap = Binary::from_i32(self.limits.provided_maximum);
        state.effective_provided[index] =
            if self.catalog.resources[index].limited_provider && cap.cmp(&raw) == Ordering::Less {
                cap.store_f32()?
            } else {
                state.raw_provided[index]
            };
        self.shortage(state, true, trace)
    }

    /// Complete 0x4E3940 including old-charge removal and new-charge application.
    pub fn adjust_provider(
        &mut self,
        index: usize,
        delta: F32Bits,
    ) -> Result<MutationTrace, EconomyError> {
        if index >= self.catalog.resources.len() {
            return Err(EconomyError::ResourceIndex);
        }
        validate_floats(&[delta])?;
        let mut state = self.state.clone();
        let mut trace = MutationTrace::default();
        self.provider(&mut state, index, delta, &mut trace)?;
        self.state = state;
        Ok(trace)
    }

    /// Complete 0x4E4930: per-element provider calls in ascending registry order.
    /// Zero deltas still execute their complete before/after shortage passes.
    pub fn adjust_providers(
        &mut self,
        values: &[F32Bits],
        add: bool,
    ) -> Result<MutationTrace, EconomyError> {
        self.check_vector(values)?;
        let mut state = self.state.clone();
        let mut trace = MutationTrace::default();
        for (index, value) in values.iter().enumerate() {
            let delta = if add {
                *value
            } else {
                F32Bits(value.0 ^ 0x8000_0000)
            };
            self.provider(&mut state, index, delta, &mut trace)?;
        }
        self.state = state;
        Ok(trace)
    }

    /// Complete 0x4E48C0: remove/mutate/reapply per consumed element, not once
    /// around the entire vector. This ordering can cause measurable rounding drift.
    pub fn adjust_consumption(
        &mut self,
        values: &[F32Bits],
        add: bool,
    ) -> Result<MutationTrace, EconomyError> {
        self.check_vector(values)?;
        let mut state = self.state.clone();
        let mut trace = MutationTrace::default();
        for (index, value) in values.iter().enumerate() {
            let delta = Binary::from_bits(if add {
                *value
            } else {
                F32Bits(value.0 ^ 0x8000_0000)
            });
            self.shortage(&mut state, false, &mut trace)?;
            state.consumed[index] = delta
                .add(&Binary::from_bits(state.consumed[index]), self.precision)
                .store_f32()?;
            self.shortage(&mut state, true, &mut trace)?;
        }
        self.state = state;
        Ok(trace)
    }

    /// Complete 0x4E39D0: add each stock vector element then clamp to [0, maximum].
    /// Native visits all rows, even non-STOCKPILED definitions.
    pub fn credit_stockpiles(&mut self, values: &[F32Bits]) -> Result<(), EconomyError> {
        self.check_vector(values)?;
        let mut state = self.state.clone();
        for (index, value) in values.iter().enumerate() {
            state.stockpiled[index] = credit_stockpile_cell(
                state.stockpiled[index], *value, self.limits.stockpiled_maximum, self.precision,
            )?;
        }
        self.state = state;
        Ok(())
    }

    /// Complete 0x4E1690. This debit does not check affordability or clamp at zero;
    /// those are caller responsibilities in the native program as well.
    pub fn debit_stockpiles(&mut self, values: &[F32Bits]) -> Result<(), EconomyError> {
        self.check_vector(values)?;
        let mut state = self.state.clone();
        for (index, value) in values.iter().enumerate() {
            state.stockpiled[index] = Binary::from_bits(state.stockpiled[index])
                .sub(&Binary::from_bits(*value), self.precision)
                .store_f32()?;
        }
        self.state = state;
        Ok(())
    }

    /// 0x4E20A0 checks STOCKPILED rows only. No purchase is performed.
    pub fn can_afford(&self, values: &[F32Bits]) -> Result<bool, EconomyError> {
        self.check_vector(values)?;
        Ok(self
            .catalog
            .resources
            .iter()
            .enumerate()
            .all(|(index, definition)| {
                definition.kind != ResourceKind::Stockpiled
                    || Binary::from_bits(self.state.stockpiled[index])
                        .cmp(&Binary::from_bits(values[index]))
                        != Ordering::Less
            }))
    }

    /// Arithmetic prefix of 0x4E9C40, ending at 0x4E9DA6. This is deliberately
    /// NOT a complete tick: no bankruptcy ratio/events, attrition, callbacks,
    /// experience, scheduling or actor effects are run. `suppress_overdrawn` is
    /// the observed economy+0x10 nonzero guard; its gameplay meaning is not guessed.
    pub fn flow_arithmetic(
        &mut self,
        seconds: F32Bits,
        suppress_overdrawn: bool,
    ) -> Result<FlowArithmetic, EconomyError> {
        validate_floats(&[seconds])?;
        let minute_fraction = Binary::from_bits(seconds)
            .mul(&Binary::from_bits(SECONDS_TO_MINUTES_BITS), self.precision)
            .store_f32()?;
        let factor = Binary::from_bits(minute_fraction);
        let count = self.catalog.resources.len();
        let mut result = FlowArithmetic {
            minute_fraction,
            gross_production: vec![F32Bits::ZERO; count],
            net_change: vec![F32Bits::ZERO; count],
            stock_went_negative: false,
        };
        let mut state = self.state.clone();
        for index in (0..count).rev() {
            let provided = Binary::from_bits(state.effective_provided[index]);
            result.gross_production[index] = factor.mul(&provided, self.precision).store_f32()?;
            result.net_change[index] = provided
                .sub(&Binary::from_bits(state.consumed[index]), self.precision)
                .mul(&factor, self.precision)
                .store_f32()?;
        }
        for index in (0..count).rev() {
            if self.catalog.resources[index].kind == ResourceKind::Stockpiled {
                let stored = Binary::from_bits(result.net_change[index])
                    .add(&Binary::from_bits(state.stockpiled[index]), self.precision)
                    .store_f32()?;
                let value = Binary::from_bits(stored);
                state.stockpiled[index] = if value.is_negative_nonzero() {
                    result.stock_went_negative |= !suppress_overdrawn;
                    F32Bits::ZERO
                } else if Binary::from_bits(self.limits.stockpiled_maximum).cmp(&value)
                    == Ordering::Less
                {
                    self.limits.stockpiled_maximum
                } else {
                    stored
                };
            }
        }
        self.state = state;
        Ok(result)
    }
}
pub mod borrowed_flows;
