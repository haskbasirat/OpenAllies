//! Portable definition identity, distinct from native pointers, authored IDs,
//! native registry ordinals and actor GTIDs. Token assignment never determines
//! simulation traversal order and tokens are not an original save-file codec.
use serde::Serialize;
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Key {
    pub domain: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    pub key: Key,
    /// Constructor projection can bind a saved object before all ability-use
    /// dependencies exist. Such an entry MUST NOT authorize gameplay use.
    pub unresolved_use: Option<String>,
    /// Native Faction loader overwrites shared AbilityDefinition.range as well.
    pub range_override_bits: Option<u32>,
}
#[derive(Debug, Default, Serialize)]
pub struct Bank {
    ids: BTreeMap<Key, u32>,
    entries: BTreeMap<u32, Entry>,
    /// One authoritative source-CV owner, moved with this Bank. Diagnostic
    /// source state is not serialized as a native save/startup certificate.
    #[serde(skip)]
    pub(crate) combat_source: Option<Box<crate::combat_source_completion::State>>,
    ///Immutable replacement-writer projection belongs to this SAME source
    ///Bank and moves with it; it is not a native save or readiness flag.
    #[serde(skip)]
    pub(crate) replacement_flags: Option<crate::replacement_flags_source::State>,
}
impl Bank {
    /// Read an existing portable identity without registering a new object.
    pub fn token(&self, domain: &str, name: &str) -> Option<u32> {
        self.ids
            .get(&Key {
                domain: domain.to_ascii_lowercase(),
                name: name.to_ascii_lowercase(),
            })
            .copied()
    }
    pub fn combat_source(&self) -> Result<&crate::combat_source_completion::State, String> {
        self.combat_source
            .as_deref()
            .ok_or_else(|| "combat source State is not installed".into())
    }
    pub fn intern(&mut self, domain: &str, name: &str) -> Result<u32, String> {
        let key = Key {
            domain: domain.to_ascii_lowercase(),
            name: name.to_ascii_lowercase(),
        };
        if let Some(&id) = self.ids.get(&key) {
            return Ok(id);
        }
        let id = u32::try_from(self.entries.len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or("portable definition identity exhausted")?;
        self.entries.insert(
            id,
            Entry {
                key: key.clone(),
                unresolved_use: None,
                range_override_bits: None,
            },
        );
        self.ids.insert(key, id);
        Ok(id)
    }
    pub fn entry(&self, id: u32) -> Result<&Entry, String> {
        self.entries
            .get(&id)
            .ok_or_else(|| "unknown portable definition token".into())
    }
    pub fn constructor_only(
        &mut self,
        id: u32,
        reason: &str,
        range: Option<u32>,
    ) -> Result<(), String> {
        let entry = self
            .entries
            .get_mut(&id)
            .ok_or("unknown portable definition token")?;
        entry.unresolved_use = Some(reason.into());
        entry.range_override_bits = range;
        Ok(())
    }
    pub fn unresolved(&self) -> impl Iterator<Item = (u32, &Entry)> {
        self.entries
            .iter()
            .filter(|(_, e)| e.unresolved_use.is_some())
            .map(|(&id, e)| (id, e))
    }
    pub fn require_usable(&self, id: u32) -> Result<&Entry, String> {
        let entry = self.entry(id)?;
        if let Some(reason) = &entry.unresolved_use {
            return Err(reason.clone());
        }
        Ok(entry)
    }
}
