//! Native ability ready/cooldown predicates (x87 `fcomp` / `fnstsw` tests,
//! compared against the binary32 zero at `[9F32DC]`) and the ActorData `+308`
//! getter `45D4F0`.
//!
//! * `45D430`: true unless `+38 <= 0` (a NaN `+38` answers true).
//! * `4C85D0`: false only when `+38 <= 0`, `+4C` is not above 0 (NaN is not)
//!   and the `+54` word has bit 5 clear.
//! * `4C8100`: `+4C > 0` (NaN false).
//! * `4C8120`: `+10 < +14` (NaN false).
//! * `45D4F0(context)`: `+308`, or with a modifier context its `501CF0`
//!   getter (`ModifierState::adjust_with_floor(0x24, ..)` against setting
//!   `[B39AB0]+1A4`). ST0 holds the returned binary32 value exactly.
//!
//! Oracle: research/phase8-combat/native_ability_state.py.
use crate::Result;
use aa_body::{F32, properties::ModifierState};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// `[9F32DC]`.
const ZERO: F32 = F32(0);

/// The x87 compare outcome of two binary32 operands (exact; NaN unordered).
fn compare(a: F32, b: F32) -> Option<Ordering> {
    f32::from_bits(a.0).partial_cmp(&f32::from_bits(b.0))
}

/// Native `45D430`.
pub fn field38_pending(field38: F32) -> bool {
    // test ah,41 / jp: taken for greater or unordered.
    matches!(compare(field38, ZERO), Some(Ordering::Greater) | None)
}

/// Native `4C85D0`.
pub fn busy(field38: F32, field4c: F32, flags54: u32) -> bool {
    if field38_pending(field38) {
        return true;
    }
    if compare(field4c, ZERO) == Some(Ordering::Greater) {
        return true;
    }
    flags54 >> 5 & 1 != 0
}

/// Native `4C8100`.
pub fn field4c_positive(field4c: F32) -> bool {
    compare(field4c, ZERO) == Some(Ordering::Greater)
}

/// Native `4C8120`.
pub fn field10_below_14(field10: F32, field14: F32) -> bool {
    // test ah,5 / jp: only "less" leaves the parity odd.
    compare(field10, field14) == Some(Ordering::Less)
}

/// Native `45D4F0(context)` on an ActorData with `+308 = base`.
pub fn modified_308(base: F32, context: Option<&ModifierState>, setting_1a4: F32, precision: Precision) -> Result<F32> {
    match context {
        None => Ok(base),
        Some(state) => state
            .adjust_with_floor(0x24, base, setting_1a4, precision)
            .map_err(|e| crate::Error::World(e.to_string())),
    }
}
