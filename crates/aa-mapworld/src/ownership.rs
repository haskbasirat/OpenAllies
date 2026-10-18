//! Source identity binding for serialized actor owner handles. This index does
//! not construct a Kingdom runtime object or infer players from header names.
use crate::{Error, IdManagerState, Inventory, Result, SourceIdentity, kingdoms::Kingdoms};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnerTarget {
    Null,
    Kingdom {
        source_slot: usize,
        serialized_id: u32,
    },
    StaleGeneration,
    MissingKingdom,
    DuplicateKingdom {
        source_slots: Vec<usize>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedOwnerReference {
    pub dynamic_record_ordinal: usize,
    pub owner_handle: u32,
    /// A source reference only. GActor::Load applies it only when the final
    /// runtime Thing flags have bit23 set (4D8D98..4D8DBA).
    pub target: OwnerTarget,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OwnershipInventory {
    pub kingdoms_source: SourceIdentity,
    pub references: Vec<SerializedOwnerReference>,
}
impl OwnershipInventory {
    pub fn from_sources(
        kingdoms: &Kingdoms,
        identities: &IdManagerState,
        things: &Inventory,
    ) -> Self {
        let mut slots: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (slot, record) in kingdoms
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, value)| value.record.as_ref().map(|r| (slot, r)))
        {
            slots.entry(record.serialized_id).or_default().push(slot);
        }
        let references = things
            .placements
            .iter()
            .filter_map(|record| {
                let owner_handle = record.actor.as_ref()?.owner_handle;
                let target = if owner_handle == 0 {
                    OwnerTarget::Null
                } else if !identities.generation_matches(owner_handle) {
                    OwnerTarget::StaleGeneration
                } else {
                    match slots.get(&owner_handle).map(Vec::as_slice) {
                        Some([slot]) => OwnerTarget::Kingdom {
                            source_slot: *slot,
                            serialized_id: owner_handle,
                        },
                        Some(values) => OwnerTarget::DuplicateKingdom {
                            source_slots: values.to_vec(),
                        },
                        None => OwnerTarget::MissingKingdom,
                    }
                };
                Some(SerializedOwnerReference {
                    dynamic_record_ordinal: record.record_ordinal,
                    owner_handle,
                    target,
                })
            })
            .collect();
        Self {
            kingdoms_source: kingdoms.source.clone(),
            references,
        }
    }
    /// Caller must supply final resolved runtime Thing flags, not the raw saved
    /// flags word. Returns the exact Kingdom source slot to restore/bind later.
    /// It deliberately cannot return a fabricated live Kingdom pointer.
    pub fn runtime_owner_slot(
        &self,
        dynamic_record_ordinal: usize,
        runtime_thing_flags: u32,
    ) -> Result<Option<usize>> {
        let reference = self
            .references
            .iter()
            .find(|r| r.dynamic_record_ordinal == dynamic_record_ordinal)
            .ok_or_else(|| {
                Error::Format(format!(
                    "no decoded Actor owner prefix for dynamic record {dynamic_record_ordinal}"
                ))
            })?;
        if runtime_thing_flags & (1 << 23) == 0 {
            return Ok(None);
        }
        match &reference.target {
            OwnerTarget::Null => Ok(None),
            OwnerTarget::Kingdom { source_slot, .. } => Ok(Some(*source_slot)),
            target => Err(Error::Format(format!(
                "owner handle {:#010x} cannot be bound: {target:?}",
                reference.owner_handle
            ))),
        }
    }
}
