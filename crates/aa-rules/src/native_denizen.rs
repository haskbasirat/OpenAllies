//! DenizenComponent inventory exactly as the original Denizen constructor
//! consumes it.
//!
//! - ActorData loader 464BD0, Inventory children (467350..4674C7): each
//!   `[Inventory]` reads `actor_ids` (deferred Thing reference, 461770 ->
//!   vtable A03468) and `count` (5B4ED0). The ActorData+35C list is searched
//!   with the case-SENSITIVE `wcscmp` (96CB83): a new IDS is appended
//!   (45FD30); a repeated IDS becomes a NEW entry inserted after the last
//!   consecutive entry of that IDS (462E60). Nothing is merged.
//! - Organization children (467165..4672E1) fill ActorData+34C with
//!   {organization_ids (deferred), behavior flags from 463520}; behavior
//!   words: worker 2, militia_offensive 4, militia_defensive 8,
//!   air_offensive 0x10, air_defensive 0x20, air_recon 0x40, air_escort
//!   0x80, separated by " +" (unknown -> "Unknown denizen behavior").
//! - Denizen constructor 56B720: for every +35C entry in list order and
//!   `count` times, a slot {target definition, flag 568880(target, 2),
//!   flag 568880(target, 0x2C)} is PREPENDED to Denizen+18 (tail +1C).
//! - 568880(target, mask): requires target mask bits 0 (Body) and 6
//!   (Properties) and an owner +34C entry whose behavior & mask is nonzero
//!   and whose organization layout accepts the target (4C1D60 -> 4C1CF0 ->
//!   45EEF0). With an empty +34C list the result is 0.
use crate::{
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;

/// Original 463520 behavior keywords (case-insensitive, `_wcsicmp`).
pub const DENIZEN_BEHAVIORS: &[(&str, u32)] = &[
    ("worker", 0x2),
    ("militia_offensive", 0x4),
    ("militia_defensive", 0x8),
    ("air_offensive", 0x10),
    ("air_defensive", 0x20),
    ("air_recon", 0x40),
    ("air_escort", 0x80),
];

/// Original 463520 tokenizer: tokens separated by ' ' or '+' (DAT A0571C
/// " +"); empty tokens are skipped; an unknown keyword throws.
pub fn native_behavior_flags(text: &str) -> Result<u32> {
    let mut flags = 0;
    for token in text.split([' ', '+']).filter(|t| !t.is_empty()) {
        let bit = DENIZEN_BEHAVIORS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(token))
            .map(|(_, bit)| *bit)
            .ok_or_else(|| Error::Unsupported(format!("Unknown denizen behavior '{token}'")))?;
        flags |= bit;
    }
    Ok(flags)
}

#[derive(Clone, Debug, Serialize)]
pub struct DenizenInventoryEntry {
    /// `actor_ids` as authored (the +35C list key).
    pub actor_ids: String,
    /// Definition the deferred reference resolves to through 427BD0
    /// (translation applied); canonical IDS.
    pub target: String,
    pub count: i32,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DenizenInventorySlot {
    pub target: String,
    /// 568880(target, 2): WORKER organization slot accepts the target.
    pub worker: bool,
    /// 568880(target, 0x2C): MILITIA_OFFENSIVE/DEFENSIVE or AIR_DEFENSIVE.
    pub militia_or_air_defensive: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct DenizenInventory {
    /// ActorData+35C in native list order.
    pub entries: Vec<DenizenInventoryEntry>,
    /// Owner +34C organization behaviors in authored order.
    pub organization_behaviors: Vec<(String, u32)>,
    /// Denizen+18 after the constructor: head first (reverse creation order).
    pub slots: Vec<DenizenInventorySlot>,
}

/// 4673C6 (case-sensitive wcscmp search) / 46742F..467455 (skip the run of
/// equal entries) / 462E60 (insert after it) / 45FD30 (append when new).
pub fn native_inventory_insert<T>(list: &mut Vec<T>, entry: T, name: impl Fn(&T) -> &String) {
    let key = name(&entry).clone();
    match list.iter().position(|e| *name(e) == key) {
        None => list.push(entry),
        Some(first) => {
            let mut last = first;
            while list.get(last + 1).is_some_and(|e| *name(e) == key) {
                last += 1;
            }
            list.insert(last + 1, entry);
        }
    }
}

/// Original 56B720 slot list (head first) for an EMPTY +34C Organization
/// list, where both 568880 calls return 0: every entry in list order adds
/// `count` slots, each prepended.
pub fn native_denizen_slots<'a>(
    entries: impl Iterator<Item = (&'a str, i32)>,
) -> Vec<DenizenInventorySlot> {
    let mut slots = Vec::new();
    for (target, count) in entries {
        for _ in 0..count.max(0) {
            slots.insert(
                0,
                DenizenInventorySlot {
                    target: target.to_owned(),
                    worker: false,
                    militia_or_air_defensive: false,
                },
            );
        }
    }
    slots
}

impl ActorCatalog {
    /// Producer-side Denizen inventory for the native constructor 56B720.
    /// `Ok(None)` when the actor has no DenizenComponent.
    pub fn native_denizen_inventory(&self, id: &str, pc: Precision) -> Result<Option<DenizenInventory>> {
        let actor = self.resolved_actor(id)?;
        let Some(denizen) = actor.child("DenizenComponent")? else {
            return Ok(None);
        };
        let string = |node: &crate::actors::ResolvedNode<'_>, name: &str| -> Result<String> {
            match node.field(name, pc)? {
                Some(TypedValue::String { value, .. }) => Ok(value),
                _ => Err(Error::Unsupported(format!("Denizen {name} must be a String"))),
            }
        };
        let mut organization_behaviors = Vec::new();
        for org in denizen.children("Organization")? {
            let ids = string(&org, "organization_ids")?;
            let flags = native_behavior_flags(&string(&org, "behavior")?)?;
            if flags == 0 {
                // 463740: "Denizen organization '%s' doesn't specify any behavior".
                return Err(Error::Unsupported(format!(
                    "Denizen organization '{ids}' doesn't specify any behavior"
                )));
            }
            organization_behaviors.push((ids, flags));
        }
        let mut entries: Vec<DenizenInventoryEntry> = Vec::new();
        for item in denizen.children("Inventory")? {
            let actor_ids = string(&item, "actor_ids")?;
            let count = item
                .field("count", pc)?
                .ok_or_else(|| Error::Unsupported("Denizen Inventory count missing".into()))?
                .integer()?;
            let target = self
                .find(&actor_ids)
                .ok_or_else(|| {
                    Error::Unsupported(format!(
                        "Denizen Inventory actor_ids {actor_ids} has no native definition"
                    ))
                })?
                .id
                .clone();
            let entry = DenizenInventoryEntry {
                actor_ids: actor_ids.clone(),
                target,
                count,
                sources: item.sources(),
            };
            native_inventory_insert(&mut entries, entry, |e| &e.actor_ids);
        }
        if !organization_behaviors.is_empty() && entries.iter().any(|e| e.count > 0) {
            return Err(Error::Unsupported(
                "Denizen Inventory flags with a nonempty Organization list require the \
                 OrganizationLayout slot test 4C1D60/4C1CF0/45EEF0 (not ported)"
                    .into(),
            ));
        }
        let slots = native_denizen_slots(
            entries.iter().map(|e| (e.target.as_str(), e.count)),
        );
        Ok(Some(DenizenInventory {
            entries,
            organization_behaviors,
            slots,
        }))
    }
}
