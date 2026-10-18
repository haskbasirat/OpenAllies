//! Original World+F0 name table operations. Names map to raw GTIDs; the live
//! IdentityRegistry performs generation validation only when a name is resolved.
//! Duplicate aliases remain ordered; publishing removes only the first match.
use aa_actors::{ActorId, IdentityRegistry, registry::Entry as RegistryEntry};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("world name needs an unimplemented native locale or embedded-null rule")]
    UnsupportedName,
    #[error("world name allocation failed")]
    Allocation,
    #[error("world name table growth exceeds the native unsigned count domain")]
    Count,
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub name: Vec<u16>,
    pub id: ActorId,
}
#[derive(Debug)]
pub struct Table {
    // Cross-name bucket order is not exposed by these operations. Growth is
    // observable: native593570 prepends old nodes and reverses equal-name order.
    entries: BTreeMap<Vec<u16>, Vec<Entry>>,
    buckets: u32,
    count: u32,
}
impl Default for Table {
    ///Original4F7D00:59 empty buckets, count0 and automatic growth enabled.
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
            buckets: 59,
            count: 0,
        }
    }
}
fn key(name: &[u16]) -> Result<Vec<u16>> {
    if name.iter().any(|c| *c > 127 || *c == 0) {
        return Err(Error::UnsupportedName);
    }
    Ok(name
        .iter()
        .map(|c| if (97..=122).contains(c) { c - 32 } else { *c })
        .collect())
}
impl Table {
    pub fn len(&self) -> u32 {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn bucket_count(&self) -> u32 {
        self.buckets
    }
    fn grow_if_needed(&mut self) -> Result<()> {
        let twice = self.buckets.checked_mul(2).ok_or(Error::Count)?;
        if self.count < twice {
            return Ok(());
        }
        let mut next = twice.checked_add(1).ok_or(Error::Count)?;
        if next.is_multiple_of(3) || next.is_multiple_of(5) {
            next = twice.checked_add(3).ok_or(Error::Count)?;
            if next.is_multiple_of(3) || next.is_multiple_of(5) {
                next = twice.checked_add(5).ok_or(Error::Count)?;
            }
        }
        for rows in self.entries.values_mut() {
            rows.reverse();
        }
        self.buckets = next;
        Ok(())
    }
    ///4F94D0→4F7B70 prepends without removing existing same-name entries.
    pub fn alias(&mut self, name: &[u16], id: ActorId) -> Result<bool> {
        if name.is_empty() {
            return Ok(false);
        }
        let key = key(name)?;
        self.grow_if_needed()?;
        let count = self.count.checked_add(1).ok_or(Error::Count)?;
        let rows = self.entries.entry(key).or_default();
        rows.try_reserve(1).map_err(|_| Error::Allocation)?;
        rows.insert(
            0,
            Entry {
                name: name.to_vec(),
                id,
            },
        );
        self.count = count;
        Ok(true)
    }
    ///Name branch of4F9510. Clearing Thing.flags31 precedes the empty-name gate
    ///and every table operation. Other class/publication effects belong to caller.
    pub fn publish(&mut self, name: &[u16], id: ActorId, thing_flags: &mut u32) -> Result<bool> {
        *thing_flags &= 0x7fff_ffff;
        if name.is_empty() {
            return Ok(false);
        }
        self.remove_first(name)?;
        self.alias(name, id)
    }
    ///4154D0 returns the first matching node, even when its saved ID is stale.
    ///Do not walk older duplicates after generation lookup rejects this first ID.
    pub fn lookup(&self, name: &[u16]) -> Result<Option<ActorId>> {
        Ok(self
            .entries
            .get(&key(name)?)
            .and_then(|rows| rows.first())
            .map(|row| row.id))
    }
    ///4F8E90/4F8F00: first name entry, then exact live generation, then the
    ///requested native virtual class predicate. Rejected first entries do not
    ///search older aliases. Runtime callers inspect the one authoritative store.
    pub fn resolve<'a, T>(
        &self,
        name: &[u16],
        registry: &'a IdentityRegistry<T>,
        class: impl FnOnce(&RegistryEntry<T>) -> bool,
    ) -> Result<Option<&'a RegistryEntry<T>>> {
        Ok(self
            .lookup(name)?
            .and_then(|id| registry.get(id))
            .filter(|entry| class(entry)))
    }
    ///5C9180 unlinks exactly the first same-name node.
    pub fn remove_first(&mut self, name: &[u16]) -> Result<bool> {
        let key = key(name)?;
        let Some(rows) = self.entries.get_mut(&key) else {
            return Ok(false);
        };
        rows.remove(0);
        self.count -= 1;
        if rows.is_empty() {
            self.entries.remove(&key);
        }
        Ok(true)
    }
    ///Review/checkpoint view of equal-name order; not native hash bucket order.
    pub fn entries(&self, name: &[u16]) -> Result<&[Entry]> {
        Ok(self.entries.get(&key(name)?).map_or(&[], Vec::as_slice))
    }
}
