//! GKingdom actor membership used by KKC_GActor::ChangeKingdom `0x4D8450`:
//! add `0x4E6180` (with `0x4E4470`), remove `0x4E44E0`, and the recruit item
//! table at kingdom+0x1FC: find single `0x4E2A60`, remove `0x4E2CB0`, add
//! `0x4E4C30`.
//!
//! ChangeKingdom order (for the caller): `old.recruit.find_single(def,false)`
//! -> if found `old.recruit.remove(item, force=true)`; `old.remove_actor`;
//! actor+0xE8 = new; if new and an item was taken: `new.recruit.find_single`
//! (duplicate -> the native destroys the actor instead) else
//! `new.recruit.add(item)`; `new.add_actor`.
//!
//! Lists owned by the kingdom are modelled here in native order. Effects on
//! other systems are explicit [`RegistryWorld`] callbacks: economy component
//! refresh/removal (`0x570020` / `0x56F4D0`, see aa-runtime economy_refresh),
//! statistics tracking (`0x58F010`, `0x58EA80`+score 14, `0x58EFE0`), city and
//! tech-building tracking `0x4E3C70`, property application/removal (`0x4CEF10`
//! / `0x4CEF70`, aa-body), structure-provided properties (`0x4E3AD0` /
//! `0x4E23D0`, aa-production research), organization refresh `0x4D1350`,
//! counted predicate `0x4E1540`, structure virtual +0x100, actor destruction.
use crate::EconomyError;
use crate::tick::RecruitSingle;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[path = "recruit_item_fields.rs"]
pub mod item_fields;

#[path = "kingdom_registry_storage.rs"]
mod storage;
pub use storage::{
    ActorList, RemoveRegistryWorld, StoredRegistryWorld, add_actor_stored, remove_actor_stored,
};

pub const CITY_CLASS: u32 = 0x1d;
pub const FLAG_COUNTED: u32 = 1 << 27;
pub const FLAG_REGISTERED: u32 = 1 << 28;

/// Actor fields read by the membership code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryActor {
    /// ActorData+0x18 class: +0x1C list index and +0xC kind (0x1D city).
    pub class_index: u32,
    pub class_kind: u32,
    /// Actor+8 flags (bit 27 counted, bit 28 registered).
    pub flags8: u32,
    /// Actor+0x100 state bits (21 skips statistics; 26/36 transient).
    pub states: u64,
    pub economy: bool,
    pub structure: bool,
    pub settlement: bool,
    pub hq: bool,
    pub organization: bool,
    pub supply: bool,
    pub properties: bool,
    pub component_ac: bool,
    pub flare: bool,
}

/// Kingdom lists touched by add/remove.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomRegistry {
    /// +0x26C + 8*class: head-first lists.
    pub class_lists: BTreeMap<u32, Vec<u32>>,
    /// +0x264 supply things, head first (`0x4E1F00` / `0x4E38B0`).
    pub supply: Vec<u32>,
    /// +0x2C4 structures, +0x2D0 settlements, +0x2DC HQs, +0x2E8 organizations.
    pub structures: Vec<u32>,
    pub settlements: Vec<u32>,
    pub hqs: Vec<u32>,
    pub organizations: Vec<u32>,
    /// +0x2F4 counted actors.
    pub counted: i32,
    /// +0x2F8 flare actors, head first.
    pub flares: Vec<u32>,
    /// +0x1F8 capital city.
    pub capital: Option<u32>,
    /// +0x190: nonzero skips property work.
    pub word_190: u32,
}

/// One kingdom property record as seen by add/remove (definition, source and
/// the resolved origin actor), in container order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyApplication {
    pub definition: u32,
    pub source: u32,
    pub origin: Option<u32>,
}

pub trait RegistryWorld {
    fn actor(&self, actor: u32) -> Result<RegistryActor, EconomyError>;
    fn set_flags8(&mut self, actor: u32, flags: u32) -> Result<(), EconomyError>;
    /// Structure actor virtual +0x100 (nonzero keeps it off the structure list).
    fn structure_excluded(&mut self, actor: u32) -> Result<bool, EconomyError>;
    fn economy_add(&mut self, actor: u32) -> Result<(), EconomyError>;
    fn economy_remove(&mut self, actor: u32) -> Result<(), EconomyError>;
    fn refresh_organization(&mut self, organization: u32) -> Result<(), EconomyError>;
    fn counted(&mut self, actor: u32) -> Result<bool, EconomyError>;
    fn statistics_present(&mut self) -> Result<bool, EconomyError>;
    /// `0x58F010` (add) / `0x58EFE0` (remove).
    fn statistics_track(&mut self, actor: u32, add: bool) -> Result<(), EconomyError>;
    /// `0x58EA80` classification for score 14.
    fn statistics_classified(&mut self, actor: u32) -> Result<bool, EconomyError>;
    /// `stats.0x58E870(index, amount)` (callers use `Statistics::add_score`).
    fn add_score(&mut self, index: usize, amount_bits: u32) -> Result<(), EconomyError>;
    /// `0x4E3C70(actor, add)`.
    fn ownership_track(&mut self, actor: u32, add: bool) -> Result<(), EconomyError>;
    /// Kingdom property records (+0x318) in order.
    fn property_records(&mut self) -> Result<Vec<PropertyApplication>, EconomyError>;
    fn apply_property(
        &mut self,
        actor: u32,
        record: PropertyApplication,
    ) -> Result<(), EconomyError>;
    fn remove_property_effect(
        &mut self,
        actor: u32,
        record: PropertyApplication,
    ) -> Result<(), EconomyError>;
    /// ActorData research items (`0x45ECA0`) of a structure, in list order.
    fn structure_properties(&mut self, actor: u32) -> Result<Vec<u32>, EconomyError>;
    /// `kingdom.0x4E3AD0(definition, 10, actor, 1)`.
    fn add_kingdom_property(&mut self, definition: u32, actor: u32) -> Result<(), EconomyError>;
    /// `kingdom.0x4E23D0(definition, actor)`.
    fn remove_kingdom_property(&mut self, definition: u32, actor: u32) -> Result<(), EconomyError>;
    /// MaxFlaresPerKingdom setting.
    fn max_flares(&mut self) -> Result<i32, EconomyError>;
    /// Flare actor+0x1C time (binary32) used by `0x4E1ED0`.
    fn flare_time(&mut self, actor: u32) -> Result<u32, EconomyError>;
    /// `world.0x4F6760(actor, 0)`.
    fn destroy_actor(&mut self, actor: u32) -> Result<(), EconomyError>;
}

fn bit(states: u64, n: u32) -> bool {
    states & (1u64 << n) != 0
}

fn remove_value(list: &mut Vec<u32>, actor: u32, name: &str) -> Result<(), EconomyError> {
    let index = list.iter().position(|&a| a == actor).ok_or_else(|| {
        EconomyError::World(format!(
            "{name}: native removal of an absent actor corrupts the vector"
        ))
    })?;
    list.remove(index);
    Ok(())
}

/// `0x4E1540` + counter at `0x4E4470`/`0x4E63AE`.
fn count_actor(
    registry: &mut KingdomRegistry,
    actor: u32,
    world: &mut impl RegistryWorld,
) -> Result<(), EconomyError> {
    let flags = world.actor(actor)?.flags8;
    if flags & FLAG_COUNTED == 0 && world.counted(actor)? {
        registry.counted += 1;
        world.set_flags8(actor, flags | FLAG_COUNTED)?;
    }
    Ok(())
}

/// Binary32 ordering helper for `0x4E1ED0` (finite flare times only).
fn f32_le(a: u32, b: u32) -> Result<bool, EconomyError> {
    let (x, y) = (f32::from_bits(a), f32::from_bits(b));
    if !x.is_finite() || !y.is_finite() {
        return Err(EconomyError::NonFiniteInput);
    }
    Ok(x <= y)
}

/// `0x4E4470(actor)`: an actor that is complete (AddActor of a finished actor,
/// or OnConstructionFinished `0x565B00`) is counted once (`0x4E1540`,
/// actor+8 bit 27), tracked by the statistics (`0x58F010`, score 14 when
/// `0x58EA80` classifies it) and by city/tech tracking `0x4E3C70(actor, 1)`.
pub fn actor_completed(
    registry: &mut KingdomRegistry,
    actor: u32,
    world: &mut impl RegistryWorld,
) -> Result<(), EconomyError> {
    count_actor(registry, actor, world)?;
    if world.statistics_present()? {
        world.statistics_track(actor, true)?;
        if world.statistics_classified(actor)? {
            world.add_score(14, 0x3f80_0000)?;
        }
    }
    world.ownership_track(actor, true)
}

/// `0x4E6180` GKingdom::AddActor.
pub fn add_actor(
    registry: &mut KingdomRegistry,
    actor: u32,
    world: &mut impl RegistryWorld,
) -> Result<(), EconomyError> {
    storage::add_compat(registry, actor, world)
}

/// `0x4E44E0` GKingdom::RemoveActor.
pub fn remove_actor(
    registry: &mut KingdomRegistry,
    actor: u32,
    world: &mut impl RegistryWorld,
) -> Result<(), EconomyError> {
    storage::remove_compat(registry, actor, world)
}

/// Recruit item table at kingdom+0x1FC (hash keyed by ActorData, singles
/// vector +0x18, change counter +0x28). Node order and actual Base/Counted
/// fields are retained; Single nodes reference the same vector allocation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecruitItemTable {
    items: Vec<item_fields::Item>,
    /// Singles in vector order (index = item+0x20).
    pub singles: Vec<RecruitSingle>,
    /// +0x28.
    pub changes: u32,
}

pub trait RecruitAi {
    /// SAI kingdoms enabled (`[0xB36528]+0x20`).
    fn ai_enabled(&mut self) -> Result<bool, EconomyError>;
    /// `0x79F500`: SAI listeners of the kingdom, in vector order.
    fn ai_listeners(&mut self) -> Result<Vec<u32>, EconomyError>;
    /// `listener+0xC -> 0x7A47C0` (added) / `0x7A48A0` (removed); the native
    /// iterates the listener vector from the last element.
    fn notify_ai(
        &mut self,
        listener: u32,
        single: &RecruitSingle,
        added: bool,
    ) -> Result<(), EconomyError>;
}

impl RecruitItemTable {
    ///Readonly historical metadata projection, never the current field owner.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = (u32, bool)> + ExactSizeIterator + '_ {
        self.items
            .iter()
            .map(|item| (item.definition(), item.is_single()))
    }
    pub fn items(&self) -> &[item_fields::Item] {
        &self.items
    }
    pub fn reserve_entries(&mut self, additional: usize) -> Result<(), EconomyError> {
        self.items
            .try_reserve(additional)
            .map_err(|e| EconomyError::World(e.to_string()))
    }
    pub fn clear(&mut self) {
        self.items.clear();
        self.singles.clear();
    }
    ///4E3FF0 or4E2E00: newest hash entry, its actual virtual4, then optional
    ///virtual8. This never derives Base/Counted fields from metadata.
    pub fn query_item(
        &mut self,
        definition: u32,
        wanted: i32,
        restore: bool,
    ) -> Result<bool, EconomyError> {
        let Some(node) = self
            .items
            .iter()
            .rposition(|item| item.definition() == definition)
        else {
            return Ok(false);
        };
        if let Some(value) = self.items[node].probe_non_single(wanted, restore) {
            return Ok(value);
        }
        let item_fields::Item::Single { index, .. } = self.items[node] else {
            unreachable!()
        };
        let item = self
            .singles
            .get_mut(index)
            .filter(|v| v.definition == definition)
            .ok_or(EconomyError::ResourceIndex)?;
        if wanted > 1 || !item.available {
            return Ok(false);
        }
        item.available = false;
        if restore {
            item.available = true
        }
        Ok(true)
    }
    /// `0x4E2A60(definition, available_only)`: only the most recently inserted
    /// entry for the definition is considered; a plain item yields `None`.
    pub fn find_single(&self, definition: u32, available_only: bool) -> Option<usize> {
        let item_fields::Item::Single { index, .. } = self
            .items
            .iter()
            .rev()
            .find(|item| item.definition() == definition)?
        else {
            return None;
        };
        let index = *index;
        if self.singles.get(index)?.definition != definition {
            return None;
        }
        if available_only && !self.singles[index].available {
            return None;
        }
        Some(index)
    }

    /// `0x4E2CB0(single, force)`. Returns the removed single, or `None` when
    /// the native keeps it (not forced, not available and still bound to a
    /// live actor/organization GTID word).
    pub fn remove_single(
        &mut self,
        index: usize,
        force: bool,
        ai: &mut impl RecruitAi,
    ) -> Result<Option<RecruitSingle>, EconomyError> {
        let single = self
            .singles
            .get(index)
            .cloned()
            .ok_or(EconomyError::ResourceIndex)?;
        if !force && !single.available && (single.actor != 0 || single.organization != 0) {
            return Ok(None);
        }
        let entry = self
            .items
            .iter()
            .rposition(|item| matches!(item,item_fields::Item::Single{definition,index:i}if *definition==single.definition&&*i==index))
            .ok_or(EconomyError::ResourceIndex)?;
        self.items.remove(entry);
        if ai.ai_enabled()? {
            for listener in ai.ai_listeners()?.into_iter().rev() {
                ai.notify_ai(listener, &single, false)?;
            }
        }
        self.singles.remove(index);
        for item in &mut self.items {
            if let item_fields::Item::Single { index: i, .. } = item {
                if *i > index {
                    *i -= 1
                }
            }
        }
        self.changes = self.changes.wrapping_add(1);
        Ok(Some(single))
    }

    /// `0x4E4C30(plain item)`: hash entry only; no singles slot or SAI notice.
    pub fn add_plain(&mut self, definition: u32) {
        self.add_base(item_fields::BaseItem::from_init_constructor(definition));
    }
    pub fn add_base(&mut self, item: item_fields::BaseItem) {
        self.items.push(item_fields::Item::Base(item));
        self.changes = self.changes.wrapping_add(1);
    }
    ///Completed actual type1 Load fields; no current saved-game stream caller
    ///is implied. Nonempty restoration remains guarded at its real producer.
    pub fn add_counted(&mut self, item: item_fields::CountedItem) {
        self.items.push(item_fields::Item::Counted(item));
        self.changes = self.changes.wrapping_add(1);
    }

    /// `0x4E4C30(single)`.
    pub fn add_single(
        &mut self,
        single: RecruitSingle,
        ai: &mut impl RecruitAi,
    ) -> Result<(), EconomyError> {
        self.items.push(item_fields::Item::Single {
            definition: single.definition,
            index: self.singles.len(),
        });
        self.singles.push(single);
        if ai.ai_enabled()? {
            let added = self
                .singles
                .last()
                .cloned()
                .ok_or(EconomyError::ResourceIndex)?;
            for listener in ai.ai_listeners()?.into_iter().rev() {
                ai.notify_ai(listener, &added, true)?;
            }
        }
        self.changes = self.changes.wrapping_add(1);
        Ok(())
    }
}

/// Member side of the recruit-single pairing `0x4E1200`.
pub trait RecruitPairing {
    /// Member+0x14 GTID.
    fn member_gtid(&mut self, member: u32) -> Result<u32, EconomyError>;
    /// Member+0x80 Element component, if present.
    fn member_element(&mut self, member: u32) -> Result<Option<u32>, EconomyError>;
    /// `Element+0x14 = item` (the Element remembers its recruit single).
    fn link_element(&mut self, element: u32, item: usize) -> Result<(), EconomyError>;
}

impl RecruitItemTable {
    /// `item.0x4E1200(member)` (called by Organization load `0x4CEA60`): the
    /// single's actor GTID (+0xC) becomes member+0x14, then a member Element
    /// (+0x80) is linked back to the item (Element+0x14). `index` is the
    /// single's position (item+0x20).
    pub fn pair_single(
        &mut self,
        index: usize,
        member: u32,
        world: &mut impl RecruitPairing,
    ) -> Result<(), EconomyError> {
        let gtid = world.member_gtid(member)?;
        self.singles
            .get_mut(index)
            .ok_or(EconomyError::ResourceIndex)?
            .actor = gtid;
        if let Some(element) = world.member_element(member)? {
            world.link_element(element, index)?;
        }
        Ok(())
    }
}
