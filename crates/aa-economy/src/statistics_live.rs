//! Live bookkeeping `58F010`/`58EFE0` and snapshot construction `58F530`/`58F420`.
//! Inputs are owned world facts, never a second authoritative statistics copy.
//! Finite arithmetic has the same explicit x87 precision contract as this crate.
use crate::binary::Binary;
use crate::statistics::{ORDINARY_SCORE_COUNT, Statistics};
use crate::{Catalog, EconomyError, F32Bits, ResourceKind, X87Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const ONE: F32Bits = F32Bits(0x3f80_0000);
pub const UNTRACKED: u32 = 0x38;

/// Fields inspected by `58F010`, `58EBA0` and `58EE00`. The component names
/// deliberately retain native offsets until their source ownership is bound.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackingFacts {
    pub component_7c: bool,
    pub component_d8: bool,
    pub structure_94: bool,
    pub component_98_level: Option<i32>,
    pub construction_9c: bool,
    pub component_88: bool,
    pub component_d4: bool,
    pub definition_4d0: bool,
    pub definition_43c_nonempty: bool,
    pub definition_4e0: u32,
}

fn finite(value: F32Bits) -> Result<Binary, EconomyError> {
    if value.is_finite() {
        Ok(Binary::from_bits(value))
    } else {
        Err(EconomyError::NonFiniteInput)
    }
}

fn add(
    stats: &mut Statistics,
    index: usize,
    amount: F32Bits,
    pc: X87Precision,
) -> Result<(), EconomyError> {
    stats.add_score(index, amount, pc)
}

fn subtract(
    stats: &mut Statistics,
    index: usize,
    amount: Binary,
    pc: X87Precision,
) -> Result<(), EconomyError> {
    let score = stats
        .scores
        .get_mut(index)
        .ok_or(EconomyError::ResourceIndex)?;
    *score = finite(*score)?.sub(&amount, pc).store_f32()?;
    Ok(())
}

/// `58EBA0`: clear the actor category before removing its previous contribution.
/// The +98 component count subtraction keeps the `fild` integer unrounded until
/// `fsub`, unlike the add path which first stores the integer as binary32.
pub fn untrack_actor(
    stats: &mut Statistics,
    field_13c: &mut u32,
    facts: &TrackingFacts,
    pc: X87Precision,
) -> Result<(), EconomyError> {
    let old = *field_13c;
    *field_13c = UNTRACKED;
    let one = Binary::from_bits(ONE);
    match old {
        47 => subtract(stats, 47, one, pc),
        48 => subtract(stats, 48, one, pc),
        49 => {
            subtract(stats, 48, one.clone(), pc)?;
            subtract(stats, 49, one, pc)
        }
        50 | 51 => {
            // The native dereference occurs before its first score store.
            let level = facts
                .component_98_level
                .ok_or_else(|| EconomyError::World("58EBA0 requires Actor+98 component".into()))?;
            subtract(stats, 50, one, pc)?;
            subtract(stats, 51, Binary::from_i32(level.wrapping_add(1)), pc)
        }
        52 => subtract(stats, 52, one, pc),
        53 => subtract(stats, 53, one, pc),
        54 => {
            subtract(stats, 53, one.clone(), pc)?;
            subtract(stats, 54, one, pc)
        }
        55 => subtract(stats, 55, one, pc),
        _ => Ok(()),
    }
}

/// `58EFE0`, used by kingdom unregistration as well as world start.
pub fn remove_actor(
    stats: &mut Statistics,
    field_13c: &mut u32,
    facts: &TrackingFacts,
    world_started: bool,
    pc: X87Precision,
) -> Result<(), EconomyError> {
    if world_started && *field_13c != UNTRACKED {
        untrack_actor(stats, field_13c, facts, pc)?;
    }
    Ok(())
}

fn replace_category(
    stats: &mut Statistics,
    field: &mut u32,
    facts: &TrackingFacts,
    category: u32,
    amount: F32Bits,
    pc: X87Precision,
) -> Result<(), EconomyError> {
    if *field == category {
        return Ok(());
    }
    if *field != UNTRACKED {
        untrack_actor(stats, field, facts, pc)?;
    }
    // 58E8B0 performs 4CEED0 before arithmetic.
    *field = category;
    match category {
        47 | 48 | 53 | 55 => add(stats, category as usize, ONE, pc),
        49 => {
            add(stats, 48, ONE, pc)?;
            add(stats, 49, ONE, pc)
        }
        50 | 51 => {
            add(stats, 50, ONE, pc)?;
            add(stats, 51, amount, pc)
        }
        52 => {
            add(stats, 52, ONE, pc)?;
            let now = stats.score(52)?;
            let peak = stats
                .scores
                .get_mut(33)
                .ok_or(EconomyError::ResourceIndex)?;
            if finite(*peak)?.cmp(&finite(now)?) == Ordering::Less {
                *peak = now;
            }
            Ok(())
        }
        54 => {
            add(stats, 53, ONE, pc)?;
            add(stats, 54, ONE, pc)
        }
        _ => Err(EconomyError::ResourceIndex),
    }
}

/// `58F010`: category transitions are sequential; an actor may pass through
/// several categories in one call. `virtual_100` is called in original order,
/// including the second call by `58EE00` when Construction is present.
pub fn track_actor(
    stats: &mut Statistics,
    field_13c: &mut u32,
    facts: &TrackingFacts,
    world_started: bool,
    pc: X87Precision,
    mut virtual_100: impl FnMut() -> Result<bool, EconomyError>,
) -> Result<(), EconomyError> {
    if facts.component_7c && !facts.component_d8 && world_started {
        replace_category(stats, field_13c, facts, 47, F32Bits(0xbf80_0000), pc)?;
    }
    if facts.structure_94 && virtual_100()? && world_started {
        replace_category(
            stats,
            field_13c,
            facts,
            if facts.definition_4d0 { 49 } else { 48 },
            F32Bits(0xbf80_0000),
            pc,
        )?;
    }
    if let Some(level) = facts.component_98_level {
        let count = Binary::from_i32(level.wrapping_add(1)).store_f32()?;
        if world_started {
            replace_category(stats, field_13c, facts, 50, count, pc)?;
        }
    }
    let class = if !facts.construction_9c || (facts.structure_94 && virtual_100()?) {
        0
    } else if facts.definition_43c_nonempty {
        1
    } else if facts.definition_4e0 != 0 {
        2
    } else if facts.component_88 {
        3
    } else {
        4
    };
    if class != 0 {
        if world_started {
            replace_category(stats, field_13c, facts, 53, F32Bits(0xbf80_0000), pc)?;
        }
        if facts.definition_4d0 && world_started {
            replace_category(stats, field_13c, facts, 54, F32Bits(0xbf80_0000), pc)?;
        }
        if class == 1 && world_started {
            replace_category(stats, field_13c, facts, 52, F32Bits(0xbf80_0000), pc)?;
        }
    }
    if facts.component_d4 && world_started {
        replace_category(stats, field_13c, facts, 55, F32Bits(0xbf80_0000), pc)?;
    }
    Ok(())
}

/// One linked-list node in a derived StatData+1C formula. Keep native head-first
/// order; there is a binary32 store after every multiply/add pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightedTerm {
    pub index: usize,
    pub weight: F32Bits,
}

/// `4B4960` fixes production IDs at62, resource levels at62+n and derived
/// scores at62+2*n. The rows must come from the live StatData registry, including
/// explicitly empty rows; an absent/unloaded catalog is not an empty catalog.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivedCatalog {
    pub rows: Vec<Vec<WeightedTerm>>,
}

/// Owned counterpart of the native 0x160-byte snapshot. The relation reference
/// is an identity to the kingdom's live object, not a cloned relation payload.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub statistics: Statistics,
    pub resource_levels: Vec<F32Bits>,
    pub kingdom_count: F32Bits,
    pub derived: Vec<F32Bits>,
    pub relation_identity: Option<u32>,
}

impl Snapshot {
    /// `58F3C0`/`58F420` lookup, including the native57..61 aliases into the
    /// first five derived rows. Those IDs are not relation-map dereferences.
    pub fn value(&self, index: usize) -> Result<F32Bits, EconomyError> {
        let n = self.resource_levels.len();
        if index >= 62 + 2 * n {
            self.derived.get(index - 62 - 2 * n).copied()
        } else if index >= 62 + n {
            self.resource_levels.get(index - 62 - n).copied()
        } else if index >= 62 {
            self.statistics.production.get(index - 62).copied()
        } else if index < 56 {
            self.statistics.scores.get(index).copied()
        } else if index == 56 {
            Some(self.kingdom_count)
        } else {
            self.derived.get(index - 57).copied()
        }
        .ok_or(EconomyError::ResourceIndex)
    }
}

/// `58F530` followed by its unhooked `58F420` calculation. `provided` is the
/// kingdom+1A0 array, `consumed` is+1B8, and `kingdom_count` is signed word+218.
/// Fixed10 resource/derived storage is replaced by growable vectors. Forward
/// and self references read the initial+0 or previous completed row in order.
#[allow(clippy::too_many_arguments)] // Explicit independent native owners; no temporary kingdom clone.
pub fn snapshot(
    statistics: &Statistics,
    resources: &Catalog,
    provided: &[F32Bits],
    consumed: &[F32Bits],
    kingdom_count: i32,
    relation_identity: Option<u32>,
    derived: &DerivedCatalog,
    pc: X87Precision,
) -> Result<Snapshot, EconomyError> {
    let n = resources.resources.len();
    if statistics.production.len() != n
        || statistics.scores.len() != ORDINARY_SCORE_COUNT
        || provided.len() != n
        || consumed.len() != n
    {
        return Err(EconomyError::Shape);
    }
    let mut result = Snapshot {
        statistics: statistics.clone(),
        resource_levels: vec![F32Bits::ZERO; n],
        kingdom_count: Binary::from_i32(kingdom_count).store_f32()?,
        derived: vec![F32Bits::ZERO; derived.rows.len().max(5)],
        relation_identity,
    };
    for (i, resource) in resources.resources.iter().enumerate() {
        result.resource_levels[i] = match resource.kind {
            ResourceKind::LimitedConsumed => consumed[i],
            _ if resource.limited_provider => provided[i],
            ResourceKind::Stockpiled | ResourceKind::Upkeep => finite(provided[i])?
                .sub(&finite(consumed[i])?, pc)
                .store_f32()?,
        };
    }
    for (i, row) in derived.rows.iter().enumerate() {
        let mut sum = F32Bits::ZERO;
        for term in row {
            let value = finite(result.value(term.index)?)?;
            sum = value
                .mul(&finite(term.weight)?, pc)
                .add(&finite(sum)?, pc)
                .store_f32()?;
        }
        result.derived[i] = sum;
    }
    Ok(result)
}
