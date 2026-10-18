//! Resolution of the DataRefs the SAIData loaders queue (`473AD0` /
//! `4741E0` push each one at the head of `[B36530]+824`). After every data
//! file is loaded, GameData's load (`48DB60`, at `490263`) runs the pending
//! pass `475230`: from the head (the most recently queued reference) each
//! reference's only virtual resolves it, then all are destroyed. A data
//! error (`410C40`) throws and ends the load.
//!
//! | vtable | resolve | target | Rust |
//! |---|---|---|---|
//! | A17038 | 4B0480 -> 4AEA00 | ego `+1C` filter list (push front) | [`FilterItem`] |
//! | A1703C | 4B04E0 -> 4AEA90 | ego `+830` tech-priority hash (`7BA3C0`) | `Ego::table_830` |
//! | A17040 | 4B0510 -> 4AEBD0 | ego `+984` company templates (push back) | `company_templates` |
//! | A17044 | 4B0520 -> 4AEC30 | ego `+97C` settlement templates (push back) | `settlement_templates` |
//! | A17048 | 4AEC90 | combined-arms element list at `owner+4` (push front) | [`CombinedArmsElement`] |
//! | A1704C | 4B0530 -> 4AED00 | ego `+79C` recruit requests (push front) | [`RecruitRequest`] |
//! | A17054 | 4B0470 -> 4AE340 | profile `+34` ego list | `profile::link_ego` |
//!
//! Lookups: Statistic (`[B36530]+574`, its own find `4AE200`), CompanyTemplate
//! (`+570`) and SettlementTemplate (`+56C`) through `48D500` (registry index
//! `+C`, -1 when unknown), Property `463FA0` (`[B36530]+464`), actor Things
//! `427BD0` (`[B36530]+C`) with the Thing virtual `+38`.
use super::ego::{DataRef, Ego, RefField};
use super::native_hash::NativeHash;
use crate::{Error, Result};
use std::collections::BTreeMap;

/// A17038 filter item (`4AEA00`): node `{index, min, max, next}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FilterItem {
    /// Statistic registry index, -1 when unknown (empty IDS only; an unknown
    /// nonempty IDS is a data error).
    pub statistic: i32,
    pub min: u32,
    pub max: u32,
}

/// A17048 combined-arms element (`4A7300` node `{property, percent,
/// multiplier, next}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombinedArmsElement {
    pub property: u32,
    pub max_percent: u32,
    pub max_multiplier: u32,
}

/// A1704C specific recruit request (`4A72A0` 0x24-byte node).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecruitRequest {
    /// `+0` actor Thing (when the request names a recruit).
    pub thing: u32,
    /// `+4` Property (when it names a property).
    pub property: u32,
    /// `+8..+1C`: number, base, economic, repeat penalty, one-time bonus,
    /// HQ-type-not-available bonus.
    pub number: u32,
    pub base: u32,
    pub economic: u32,
    pub repeat_penalty: u32,
    pub one_time_bonus: u32,
    pub hq_bonus: u32,
}

/// The ego containers the pass fills (beyond `Ego::table_830`), in native
/// list order (head first).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EgoLinks {
    /// ego `+1C` list (`+20` head).
    pub filters: Vec<FilterItem>,
    /// ego `+984`.
    pub company_templates: Vec<i32>,
    /// ego `+97C`.
    pub settlement_templates: Vec<i32>,
    /// Lists at `ego + owner + 4`, by owner offset.
    pub combined_arms: BTreeMap<u32, Vec<CombinedArmsElement>>,
    /// ego `+79C`.
    pub recruit_requests: Vec<RecruitRequest>,
}

/// The registry lookups of the pass.
pub trait RefLookups {
    /// `4AE200` on the Statistic manager: its registry index (`+C`).
    fn statistic(&mut self, ids: &str) -> Result<Option<i32>>;
    /// `48D500` on CompanyTemplate (`+570`): registry index.
    fn company_template(&mut self, ids: &str) -> Result<Option<i32>>;
    /// `48D500` on SettlementTemplate (`+56C`): registry index.
    fn settlement_template(&mut self, ids: &str) -> Result<Option<i32>>;
    /// `463FA0`: the Property handle.
    fn property(&mut self, ids: &str) -> Result<Option<u32>>;
    /// `427BD0` then the Thing virtual `+38` (an actor Thing); `None` when
    /// absent or not an actor.
    fn actor_thing(&mut self, ids: &str) -> Result<Option<u32>>;
    /// `410C40(format, args)`: the data error that ends the load.
    fn data_error(&mut self, format: &str, args: &[String]) -> Error;
}

fn word(r: &DataRef, i: usize) -> Result<u32> {
    match r.fields.get(i) {
        Some(RefField::Word(w)) => Ok(*w),
        other => Err(Error::World(format!(
            "DataRef {:#x} field {i} is not a word: {other:?}",
            r.vtable
        ))),
    }
}
fn ego_offset(r: &DataRef) -> Result<u32> {
    match r.fields.first() {
        Some(RefField::Ego(off)) => Ok(*off),
        other => Err(Error::World(format!(
            "DataRef {:#x} has no ego target: {other:?}",
            r.vtable
        ))),
    }
}
fn byte(r: &DataRef, i: usize) -> Result<u8> {
    match r.fields.get(i) {
        Some(RefField::Byte(b)) => Ok(*b),
        other => Err(Error::World(format!(
            "DataRef {:#x} field {i} is not a byte: {other:?}",
            r.vtable
        ))),
    }
}

/// `4AEA90` insert into the ego's tech hash: duplicate check, then
/// `7BA3C0` with the bucket computed before a possible rehash.
fn insert_tech(table: &mut NativeHash<u32>, key: u32, value: u32) -> Result<bool> {
    if table.contains(key)? {
        return Ok(false);
    }
    table.insert(key, value)?;
    Ok(true)
}

/// Resolve one queued reference of `ego` (its vtable's virtual).
pub fn resolve(
    r: &DataRef,
    ego: &mut Ego,
    links: &mut EgoLinks,
    l: &mut impl RefLookups,
) -> Result<()> {
    match r.vtable {
        0x00a1_7038 => {
            if r.ids.is_empty() {
                return Err(l.data_error("Filter item with no ids specified", &[]));
            }
            let index = l.statistic(&r.ids)?.unwrap_or(-1);
            if index < 0 {
                return Err(l.data_error("Unrecognized filter item IDS: %s", &[r.ids.clone()]));
            }
            if ego_offset(r)? != 0x1c {
                return Err(Error::World("filter DataRef outside ego +1C".into()));
            }
            links.filters.insert(
                0,
                FilterItem {
                    statistic: index,
                    min: word(r, 1)?,
                    max: word(r, 2)?,
                },
            );
            Ok(())
        }
        0x00a1_703c => {
            if r.ids.is_empty() {
                return Err(l.data_error(
                    "Technology priority preference with no ids specified",
                    &[],
                ));
            }
            let Some(property) = l.property(&r.ids)? else {
                return Err(l.data_error("Unrecognized tech IDS: %s", &[r.ids.clone()]));
            };
            if !insert_tech(&mut ego.table_830, property, word(r, 2)?)? {
                return Err(l.data_error(
                    "Duplicate tech priority (%s) in ego (%s)",
                    &[r.ids.clone(), ego.ids.clone()],
                ));
            }
            Ok(())
        }
        0x00a1_7040 => match l.company_template(&r.ids)? {
            Some(index) if index >= 0 => {
                links.company_templates.push(index);
                Ok(())
            }
            _ => Err(l.data_error(
                "Unknown company template (%s) in ego (%s)",
                &[r.ids.clone(), ego.ids.clone()],
            )),
        },
        0x00a1_7044 => match l.settlement_template(&r.ids)? {
            Some(index) if index >= 0 => {
                links.settlement_templates.push(index);
                Ok(())
            }
            _ => Err(l.data_error(
                "Unknown settlement template (%s) in ego (%s)",
                &[r.ids.clone(), ego.ids.clone()],
            )),
        },
        0x00a1_7048 => {
            let Some(property) = l.property(&r.ids)? else {
                return Err(l.data_error(
                    "One of the egos contains a combined arms element with property (%s), which is unknown",
                    &[r.ids.clone()],
                ));
            };
            links.combined_arms.entry(ego_offset(r)?).or_default().insert(
                0,
                CombinedArmsElement {
                    property,
                    max_percent: word(r, 1)?,
                    max_multiplier: word(r, 2)?,
                },
            );
            Ok(())
        }
        0x00a1_704c => {
            // 4A72A0 links the zeroed record first; the lookups may throw
            // afterwards (the record stays in the list).
            links.recruit_requests.insert(
                0,
                RecruitRequest {
                    thing: 0,
                    property: 0,
                    number: 0,
                    base: 0,
                    economic: 0,
                    repeat_penalty: 0,
                    one_time_bonus: 0,
                    hq_bonus: 0,
                },
            );
            let recruit = byte(r, 6)? != 0;
            if recruit {
                let thing = l.actor_thing(&r.ids)?;
                if thing.is_none() {
                    return Err(l.data_error(
                        "Bad actor ids (%s) in a specific recruit request for ego (%s)",
                        &[r.ids.clone(), ego.ids.clone()],
                    ));
                }
                links.recruit_requests[0].thing = thing.unwrap_or(0);
            } else {
                let property = l.property(&r.ids)?;
                if property.is_none() {
                    return Err(l.data_error(
                        "Bad property ids (%s) in a specific recruit request for ego (%s)",
                        &[r.ids.clone(), ego.ids.clone()],
                    ));
                }
                links.recruit_requests[0].property = property.unwrap_or(0);
            }
            let rec = &mut links.recruit_requests[0];
            rec.number = word(r, 1)?;
            rec.base = word(r, 2)?;
            rec.economic = word(r, 3)?;
            rec.repeat_penalty = word(r, 4)?;
            rec.one_time_bonus = word(r, 5)?;
            rec.hq_bonus = word(r, 7)?;
            Ok(())
        }
        other => Err(Error::World(format!(
            "DataRef vtable {other:#x} is not an SAI ego reference (A17054 resolves through profile::link_ego)"
        ))),
    }
}

/// `475230` over the queued references of the egos, given in creation
/// order as `(ego index, ref index)`: resolved last-queued first.
pub fn resolve_pending(
    queue: &[(usize, usize)],
    egos: &mut [Ego],
    links: &mut [EgoLinks],
    l: &mut impl RefLookups,
) -> Result<()> {
    for &(e, i) in queue.iter().rev() {
        let r = egos
            .get(e)
            .and_then(|ego| ego.refs.get(i))
            .cloned()
            .ok_or_else(|| Error::World(format!("queued DataRef ({e}, {i}) missing")))?;
        let ego = &mut egos[e];
        let link = links
            .get_mut(e)
            .ok_or_else(|| Error::World(format!("ego {e} has no link containers")))?;
        resolve(&r, ego, link, l)?;
    }
    Ok(())
}

/// The creation order of one ego's references (the loaders queue them in
/// `Ego::refs` order).
pub fn ego_queue(ego_index: usize, ego: &Ego) -> Vec<(usize, usize)> {
    (0..ego.refs.len()).map(|i| (ego_index, i)).collect()
}

/// [`RefLookups`] over the aa-rules catalog (read-only): Properties and
/// actor Things through `rules_data::RulesData` (the ego loaders' own
/// lookups), the three SAI template/statistic managers through
/// `ActorCatalog::native_data_find` (`48D500`, registry slot = index `+C`).
/// aa-rules does not register Statistic, CompanyTemplate or
/// SettlementTemplate as native-order managers yet (they are not in
/// `GENERIC_DATA_TAGS`; Statistic also has its own find `4AE200`), so those
/// lookups are explicit errors until it does (requests-to-lead.md).
pub struct RulesRefLookups<'r, 'a> {
    pub data: &'r mut super::rules_data::RulesData<'a>,
}

impl RulesRefLookups<'_, '_> {
    fn registry_index(&self, tag: &str, ids: &str) -> Result<Option<i32>> {
        let registry = self
            .data
            .catalog
            .native_data_registry(tag)
            .ok_or_else(|| {
                Error::World(format!(
                    "aa-rules has no native-order {tag} registry (48D500 lookup of '{ids}' unavailable)"
                ))
            })?;
        Ok(registry.find(ids).map(|entry| {
            registry
                .entries
                .iter()
                .position(|e| std::ptr::eq(e, entry))
                .expect("entry of this registry") as i32
        }))
    }
}

impl RefLookups for RulesRefLookups<'_, '_> {
    fn statistic(&mut self, ids: &str) -> Result<Option<i32>> {
        if ids.is_empty() {
            return Ok(None);
        }
        Err(Error::World(format!(
            "Statistic lookup 4AE200 of '{ids}' is not available from aa-rules"
        )))
    }
    fn company_template(&mut self, ids: &str) -> Result<Option<i32>> {
        self.registry_index("CompanyTemplate", ids)
    }
    fn settlement_template(&mut self, ids: &str) -> Result<Option<i32>> {
        self.registry_index("SettlementTemplate", ids)
    }
    fn property(&mut self, ids: &str) -> Result<Option<u32>> {
        super::ego::EgoServices::property(self.data, ids)
    }
    fn actor_thing(&mut self, ids: &str) -> Result<Option<u32>> {
        super::ego::EgoServices::thing(self.data, super::ego::ThingSource::Root, ids)
    }
    fn data_error(&mut self, format: &str, args: &[String]) -> Error {
        super::data::DataNode::data_error(self.data, format, args)
    }
}
