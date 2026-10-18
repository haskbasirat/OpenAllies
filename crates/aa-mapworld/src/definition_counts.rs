//! Kingdom+35C saved definition/value table. Equality and per-key lookup order
//! are portable; native cross-key bucket iteration requires original pointer
//! hashes and is deliberately not exposed by this representation.
use crate::{Error, Result};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug)]
pub struct DefinitionCounts {
    buckets: u32,
    count: u32,
    chains: BTreeMap<u32, VecDeque<u32>>,
}
impl Default for DefinitionCounts {
    fn default() -> Self {
        Self {
            buckets: 10,
            count: 0,
            chains: BTreeMap::new(),
        }
    }
}
impl DefinitionCounts {
    pub fn bucket_count(&self) -> u32 {
        self.buckets
    }
    pub fn len(&self) -> u32 {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    /// First matching node in the original bucket chain. Duplicate definitions
    /// are retained; no associative-map overwrite is substituted for insertion.
    pub fn get(&self, token: u32) -> Option<u32> {
        self.chains.get(&token).and_then(|v| v.front()).copied()
    }
    /// Original 4E4980 on Kingdom +35C (numbered names): the first node of
    /// `token` is incremented; a missing key, or a count that wraps to 0,
    /// inserts a new head node with 1 (7BA3C0). Returns the number.
    pub fn assign(&mut self, token: u32) -> Result<u32> {
        if let Some(front) = self.chains.get_mut(&token).and_then(|v| v.front_mut()) {
            let next = front.wrapping_add(1);
            *front = next;
            if next != 0 {
                return Ok(next);
            }
        }
        self.insert(token, 1)?;
        Ok(1)
    }
    /// Original4E910C..4E9171 inserts at the bucket head. Rehash606750 reverses
    /// each same-key subsequence while traversing the old bucket chains, even
    /// though the ordering among distinct pointer hashes is not portable.
    pub fn insert(&mut self, token: u32, value: u32) -> Result<()> {
        if token == 0 {
            return Err(Error::Format(
                "null definition cannot enter Kingdom count table".into(),
            ));
        }
        let next_count = self.count.checked_add(1).ok_or(Error::Allocation)?;
        let next_buckets = if self.count >= self.buckets.checked_mul(2).ok_or(Error::Allocation)? {
            let mut n = self
                .buckets
                .checked_mul(2)
                .and_then(|v| v.checked_add(1))
                .ok_or(Error::Allocation)?;
            for _ in 0..2 {
                if !n.is_multiple_of(3) && !n.is_multiple_of(5) {
                    break;
                }
                n = n.checked_add(2).ok_or(Error::Allocation)?;
            }
            n
        } else {
            self.buckets
        };
        // Reserve before changing existing lookup order on allocation failure.
        self.chains
            .entry(token)
            .or_default()
            .try_reserve(1)
            .map_err(|_| Error::Allocation)?;
        if next_buckets != self.buckets {
            for values in self.chains.values_mut() {
                values.make_contiguous().reverse();
            }
            self.buckets = next_buckets;
        }
        self.chains
            .get_mut(&token)
            .ok_or_else(|| Error::Format("definition chain disappeared".into()))?
            .push_front(value);
        self.count = next_count;
        Ok(())
    }
}
