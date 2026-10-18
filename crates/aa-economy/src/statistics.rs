//! Authoritative kingdom statistics (owner: aa-economy, lead decision 2026-10-14).
//!
//! Two native objects hang off GKingdom:
//! * kingdom+0x32C ordinary statistics (0x108 bytes, constructor `0x58EB60`):
//!   resource production totals inline at +0 (`float[10]`, cap checked at
//!   `0x48BA89`) and ordinary scores at +0x28 + 4*index for indices 0..=55
//!   (names table `0x58E1A0`). `0x58E870` adds one amount.
//! * kingdom+0x340 relation statistics (0xF0 bytes, constructor `0x58D560`):
//!   ten keyed slots, each a map at +0x10*slot (97 buckets for slots 0..=4, 13
//!   for 5..=9), a leader key at +0xA0+4*slot and a maximum at +0xC8+4*slot.
//!   Score indices 57..=61 own two slots each: `0x58D8C0` (keys are ActorData
//!   definition pointers; callers 0x564F77, 0x570E0F, 0x583562/585/5EB/646 with
//!   indices 0x39/0x3A/0x3B/0x3D) uses slot index-57, and `0x58D950` (keys are
//!   kingdom pointers; callers 0x4D5B92/0x4D5BAF kill credit with 0x3C) uses
//!   slot index-52. Both add to a keyed value and replace the leader when the
//!   extended sum exceeds the stored maximum. `0x58D950` does not range-check:
//!   an index 52..=56 would alias the by-definition slots (never called so).
//!
//! Every writer (economy tick, research, Commander XP, kill credit, construction
//! counters) must go through these methods; no second mutable copy may exist.
//! Keys are opaque stable identities (native kingdom/actor pointers); map bucket
//! order is not gameplay state and is not modelled.
use crate::binary::Binary;
use crate::{EconomyError, F32Bits, X87Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Ordinary score slots in the native 0x108-byte object.
pub const ORDINARY_SCORE_COUNT: usize = 56;
/// Scores 0..=46 are zeroed by `0x58EB60`; 47..=55 are left to later writers
/// (`0x58EE80` loads 47..=54 from saves). Rust starts them at zero explicitly.
pub const CONSTRUCTOR_ZEROED_SCORES: usize = 47;
pub const FIRST_RELATION_INDEX: usize = 52;
pub const RELATION_COUNT: usize = 10;
/// `0x58D8C0` indices 57..=61 use slots 0..=4.
pub const FIRST_DEFINITION_RELATION_INDEX: usize = 57;
pub const DEFINITION_RELATION_COUNT: usize = 5;

fn b(value: F32Bits) -> Binary {
    Binary::from_bits(value)
}

/// GKingdom+0x32C.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statistics {
    /// Production totals in registry order (native inline float[10]).
    pub production: Vec<F32Bits>,
    /// Ordinary scores by native index.
    pub scores: Vec<F32Bits>,
}

impl Statistics {
    /// Zeroed object sized to the resource registry (the native 10-resource
    /// inline cap is recorded, not enforced).
    pub fn new(resource_count: usize) -> Self {
        Self {
            production: vec![F32Bits::ZERO; resource_count],
            scores: vec![F32Bits::ZERO; ORDINARY_SCORE_COUNT],
        }
    }

    pub fn score(&self, index: usize) -> Result<F32Bits, EconomyError> {
        self.scores
            .get(index)
            .copied()
            .ok_or(EconomyError::ResourceIndex)
    }

    /// `0x58E870`: score = binary32(PC(amount + score)).
    pub fn add_score(
        &mut self,
        index: usize,
        amount: F32Bits,
        precision: X87Precision,
    ) -> Result<(), EconomyError> {
        let score = self
            .scores
            .get_mut(index)
            .ok_or(EconomyError::ResourceIndex)?;
        if !amount.is_finite() || !score.is_finite() {
            return Err(EconomyError::NonFiniteInput);
        }
        *score = b(amount).add(&b(*score), precision).store_f32()?;
        Ok(())
    }
}

/// `0x58E870` as a free function (kept for existing callers).
pub fn add_score(
    statistics: &mut Statistics,
    index: usize,
    amount: F32Bits,
    precision: X87Precision,
) -> Result<(), EconomyError> {
    statistics.add_score(index, amount, precision)
}

/// One keyed relation statistic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationStatistic {
    pub values: BTreeMap<u32, F32Bits>,
    pub maximum: F32Bits,
    pub leader: Option<u32>,
}
impl Default for RelationStatistic {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            maximum: F32Bits::ZERO,
            leader: None,
        }
    }
}

/// GKingdom+0x340.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationStatistics {
    /// Index `FIRST_RELATION_INDEX + i`.
    pub entries: Vec<RelationStatistic>,
}

impl Default for RelationStatistics {
    fn default() -> Self {
        Self {
            entries: vec![RelationStatistic::default(); RELATION_COUNT],
        }
    }
}

impl RelationStatistics {
    /// Slot `index - 52` (the `0x58D950` view).
    pub fn entry(&self, index: usize) -> Result<&RelationStatistic, EconomyError> {
        index
            .checked_sub(FIRST_RELATION_INDEX)
            .and_then(|i| self.entries.get(i))
            .ok_or(EconomyError::ResourceIndex)
    }

    /// Slot `index - 57` (the `0x58D8C0` by-definition view, indices 57..=61).
    pub fn definition_entry(&self, index: usize) -> Result<&RelationStatistic, EconomyError> {
        Self::definition_slot(index).map(|slot| &self.entries[slot])
    }

    fn definition_slot(index: usize) -> Result<usize, EconomyError> {
        index
            .checked_sub(FIRST_DEFINITION_RELATION_INDEX)
            .filter(|&slot| slot < DEFINITION_RELATION_COUNT)
            .ok_or(EconomyError::ResourceIndex)
    }

    /// `0x58D8C0(index, definition, amount)`: same arithmetic as
    /// [`RelationStatistics::add`] on slot `index - 57`.
    pub fn add_by_definition(
        &mut self,
        index: usize,
        definition: Option<u32>,
        amount: F32Bits,
        precision: X87Precision,
    ) -> Result<(), EconomyError> {
        let slot = Self::definition_slot(index)?;
        self.add_slot(slot, definition, amount, precision)
    }

    /// `0x58D950(index, other, amount)`: a null `other` does nothing; a missing
    /// key is inserted as +0 (`0x7BA3C0`); the value is stored as binary32
    /// (FST, no pop) and the unrounded sum is compared with the maximum, so a
    /// rounded tie can still change the leader.
    pub fn add(
        &mut self,
        index: usize,
        other: Option<u32>,
        amount: F32Bits,
        precision: X87Precision,
    ) -> Result<(), EconomyError> {
        let slot = index
            .checked_sub(FIRST_RELATION_INDEX)
            .filter(|&slot| slot < RELATION_COUNT)
            .ok_or(EconomyError::ResourceIndex)?;
        self.add_slot(slot, other, amount, precision)
    }

    fn add_slot(
        &mut self,
        slot: usize,
        other: Option<u32>,
        amount: F32Bits,
        precision: X87Precision,
    ) -> Result<(), EconomyError> {
        let Some(other) = other else {
            return Ok(());
        };
        let entry = &mut self.entries[slot];
        let before = entry.values.get(&other).copied().unwrap_or(F32Bits::ZERO);
        if !amount.is_finite() || !before.is_finite() || !entry.maximum.is_finite() {
            return Err(EconomyError::NonFiniteInput);
        }
        let sum = b(amount).add(&b(before), precision);
        let stored = sum.store_f32()?;
        entry.values.insert(other, stored);
        if sum.cmp(&b(entry.maximum)) == Ordering::Greater {
            entry.maximum = stored;
            entry.leader = Some(other);
        }
        Ok(())
    }
}

/// Actor facts read by the construction/upgrade completion statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureFacts {
    /// Actor+0x94 structure component present.
    pub structure: bool,
    /// Actor virtual +0x100 (called only when the structure component exists).
    pub virtual_100: bool,
    /// Actor+0x9C ConstructionComponent present.
    pub construction: bool,
    /// ActorData+0x43C list non-empty (`0x45ECA0`).
    pub data_list_43c: bool,
    /// ActorData+0x4E0 site class.
    pub site_class: u32,
    /// Actor+0x88 component present.
    pub component_88: bool,
    /// Actor+0x90 component present.
    pub component_90: bool,
}

/// `0x58EAB0`: structure component and virtual +0x100.
pub fn structure_scored(facts: &StructureFacts) -> bool {
    facts.structure && facts.virtual_100
}

/// `0x58EE00` structure class: 0 without a ConstructionComponent or for a
/// scored structure, 1 with an ActorData+0x43C list, 2 with a site class,
/// 3 with component +0x88, else 4.
pub fn structure_class(facts: &StructureFacts) -> u32 {
    if !facts.construction || structure_scored(facts) {
        0
    } else if facts.data_list_43c {
        1
    } else if facts.site_class != 0 {
        2
    } else if facts.component_88 {
        3
    } else {
        4
    }
}

const ONE: F32Bits = F32Bits(0x3F80_0000);

/// `0x564ED0` construction completion: owner statistics (+0x32C) get score
/// 19 for a scored structure, then 27/38/34 for classes 1/2/3; the owner
/// relation statistics (+0x340) get `0x58D8C0(59, definition, 1.0)` unless
/// the actor has component +0x90. `None` objects (or no owner) skip.
pub fn construction_completed(
    statistics: Option<&mut Statistics>,
    relations: Option<&mut RelationStatistics>,
    facts: &StructureFacts,
    definition: u32,
    precision: X87Precision,
) -> Result<(), EconomyError> {
    if let Some(statistics) = statistics {
        if structure_scored(facts) {
            statistics.add_score(19, ONE, precision)?;
        }
        let index = match structure_class(facts) {
            1 => Some(27),
            2 => Some(38),
            3 => Some(34),
            _ => None,
        };
        if let Some(index) = index {
            statistics.add_score(index, ONE, precision)?;
        }
    }
    if let Some(relations) = relations
        && !facts.component_90
    {
        relations.add_by_definition(59, Some(definition), ONE, precision)?;
    }
    Ok(())
}

/// `0x564E10` upgrade completion (old actor facts): score 20 for a scored
/// structure, then 39/35 for classes 2/3 (class 1 adds nothing).
pub fn upgrade_completed(
    statistics: Option<&mut Statistics>,
    facts: &StructureFacts,
    precision: X87Precision,
) -> Result<(), EconomyError> {
    let Some(statistics) = statistics else {
        return Ok(());
    };
    if structure_scored(facts) {
        statistics.add_score(20, ONE, precision)?;
    }
    match structure_class(facts) {
        2 => statistics.add_score(39, ONE, precision),
        3 => statistics.add_score(35, ONE, precision),
        _ => Ok(()),
    }
}

/// Production rows held inline by the native statistics object (and saved).
pub const NATIVE_PRODUCTION_ROWS: usize = 10;
/// Scores reset by `0x58EA00` at world start (+0xE4..+0x104).
pub const MISSION_SCORES: std::ops::RangeInclusive<usize> = 47..=55;

impl Statistics {
    /// `0x58EE80` over the decoded words (aa-mapworld restores them as the
    /// "strategic" words of GKingdom::Load v10+; they are this object).
    /// Version 0 (60 words): production and scores 0..=46 are zeroed, then
    /// production 0..10, scores 0..=41 and 47..=54 are read; 42..=46 stay zero
    /// and 55 keeps its previous value. Version 1 (66 words): production and
    /// all 56 scores. Other versions fail natively (`0x8589B0`). Returns the
    /// saved production rows beyond the resource registry (kept by the native
    /// inline array, read by nothing).
    pub fn load_saved(
        &mut self,
        version: u32,
        words: &[F32Bits],
    ) -> Result<Vec<F32Bits>, EconomyError> {
        if self.production.len() > NATIVE_PRODUCTION_ROWS {
            return Err(EconomyError::World(
                "saved statistics hold 10 production rows; the registry is larger".into(),
            ));
        }
        let expected = match version {
            0 => 60,
            1 => 66,
            _ => return Err(EconomyError::World("unsupported statistics version".into())),
        };
        if words.len() != expected || self.scores.len() != ORDINARY_SCORE_COUNT {
            return Err(EconomyError::Shape);
        }
        let n = self.production.len();
        self.production.copy_from_slice(&words[..n]);
        let tail = words[n..NATIVE_PRODUCTION_ROWS].to_vec();
        if version == 0 {
            self.scores[..42].copy_from_slice(&words[10..52]);
            self.scores[42..47].fill(F32Bits::ZERO);
            self.scores[47..55].copy_from_slice(&words[52..60]);
        } else {
            self.scores.copy_from_slice(&words[10..66]);
        }
        Ok(tail)
    }

    /// `0x58EA00`: scores 47..=55 = +0 (world start, after promotion).
    pub fn reset_mission_scores(&mut self) {
        for score in &mut self.scores[MISSION_SCORES] {
            *score = F32Bits::ZERO;
        }
    }
}

/// One decoded relation statistics entry with its key resolved: a definition
/// (`0x58DB20`: empty or unknown name -> None) or a kingdom GTID resolved
/// through the GTID table (stale -> None).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedRelationEntry {
    pub key: Option<u32>,
    pub value: F32Bits,
}

impl RelationStatistics {
    /// `0x58DBE0` over decoded entries. Version 0 data is read and discarded.
    /// Versions 1 and 2 carry 4 or 5 groups of each family: by-definition
    /// groups fill slots 0.., by-kingdom groups slots 5..; an unresolved key
    /// is skipped; a resolved key is inserted (a later duplicate shadows the
    /// earlier one for every lookup) and becomes the leader when its value is
    /// greater than the slot maximum (unordered comparisons never are).
    pub fn load_saved(
        &mut self,
        version: u32,
        by_definition: &[Vec<SavedRelationEntry>],
        by_kingdom: &[Vec<SavedRelationEntry>],
    ) -> Result<(), EconomyError> {
        let groups = match version {
            0 => return Ok(()),
            1 => 4,
            2 => 5,
            _ => {
                return Err(EconomyError::World(
                    "unsupported relation statistics version".into(),
                ));
            }
        };
        if by_definition.len() != groups || by_kingdom.len() != groups {
            return Err(EconomyError::Shape);
        }
        let families = [(0, by_definition), (DEFINITION_RELATION_COUNT, by_kingdom)];
        for (first, family) in families {
            for (offset, group) in family.iter().enumerate() {
                let entry = &mut self.entries[first + offset];
                for saved in group {
                    let Some(key) = saved.key else { continue };
                    entry.values.insert(key, saved.value);
                    if f32::from_bits(saved.value.0) > f32::from_bits(entry.maximum.0) {
                        entry.leader = Some(key);
                        entry.maximum = saved.value;
                    }
                }
            }
        }
        Ok(())
    }
}

/// The four statistics pointers of GKingdom: +0x32C/+0x340 are live,
/// +0x330/+0x344 receive GKingdom::Load (`0x4E8550` v10+).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomStatistics {
    pub active: Option<Statistics>,
    pub loaded: Option<Statistics>,
    pub relations: Option<RelationStatistics>,
    pub loaded_relations: Option<RelationStatistics>,
}

impl KingdomStatistics {
    /// `0x4E3690`: the loaded objects (created zeroed when absent) replace the
    /// live ones, which are freed; the loaded slots become empty.
    pub fn promote_loaded(&mut self, resource_count: usize) {
        self.active = Some(
            self.loaded
                .take()
                .unwrap_or_else(|| Statistics::new(resource_count)),
        );
        self.relations = Some(self.loaded_relations.take().unwrap_or_default());
    }

    /// World start `0x4F1E10` for one kingdom: `0x4E3690`, then `0x58EA00` on
    /// the live object.
    pub fn start_world(&mut self, resource_count: usize) {
        self.promote_loaded(resource_count);
        if let Some(active) = &mut self.active {
            active.reset_mission_scores();
        }
    }
}
