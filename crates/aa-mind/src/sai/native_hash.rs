//! The original chained hash table used by the SAI ego data (`{buckets, n,
//! count, grow}` at +0/+4/+8/+C, node `{key, value, next}`), keyed by a raw
//! 32-bit word (`key % n`, unsigned):
//!
//! - `4A86D0` contains: walk the key's chain;
//! - `7BA3C0` insert: when `grow` is set and `count >= 2 * n`, `606750`
//!   rehashes first; the node is pushed at the HEAD of its chain;
//! - `606750` rehash: `n' = 2n + 1`, then `+2` while divisible by 3 or 5 (at
//!   most twice); old buckets are walked in index order, each chain head to
//!   tail, every node pushed at the head of its new chain;
//! - `7A6430` find-or-insert (value 0) and return the value.
//!
//! The keys the loaders use are native object pointers (Thing / Property
//! definitions) or enum values (goal types). Here a key is the opaque handle
//! the data services return, hashed exactly as the native pointer would be.
use crate::{Error, Result};

#[derive(Clone, Debug, PartialEq)]
pub struct NativeHash<V> {
    /// Chains per bucket, head first.
    pub buckets: Vec<Vec<(u32, V)>>,
    /// +8 entry count.
    pub count: u32,
    /// +C rehash flag (every constructor in the ego sets 1).
    pub grow: bool,
}

impl<V> NativeHash<V> {
    /// Constructor: `n` zeroed buckets, count 0, grow 1.
    pub fn new(n: u32) -> Self {
        NativeHash {
            buckets: (0..n).map(|_| Vec::new()).collect(),
            count: 0,
            grow: true,
        }
    }
    fn bucket(&self, key: u32) -> Result<usize> {
        let n = self.buckets.len() as u32;
        if n == 0 {
            return Err(Error::World("native hash table without buckets".into()));
        }
        Ok((key % n) as usize)
    }
    /// `4A86D0`.
    pub fn contains(&self, key: u32) -> Result<bool> {
        Ok(self.buckets[self.bucket(key)?]
            .iter()
            .any(|(k, _)| *k == key))
    }
    /// The first node of the key's chain (head first), as the native finds do.
    pub fn get_mut(&mut self, key: u32) -> Result<Option<&mut V>> {
        let b = self.bucket(key)?;
        Ok(self.buckets[b]
            .iter_mut()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v))
    }
    /// `606750`.
    fn rehash(&mut self) {
        let mut n = (self.buckets.len() as u32).wrapping_mul(2).wrapping_add(1);
        if n.is_multiple_of(3) || n.is_multiple_of(5) {
            n = n.wrapping_add(2);
            if n.is_multiple_of(3) || n.is_multiple_of(5) {
                n = n.wrapping_add(2);
            }
        }
        let old = std::mem::replace(&mut self.buckets, (0..n).map(|_| Vec::new()).collect());
        for chain in old {
            for (key, value) in chain {
                self.buckets[(key % n) as usize].insert(0, (key, value));
            }
        }
    }
    /// `7BA3C0` (the bucket the caller computed is recomputed after a rehash).
    pub fn insert(&mut self, key: u32, value: V) -> Result<()> {
        if self.grow && self.count >= (self.buckets.len() as u32).wrapping_mul(2) {
            self.rehash();
        }
        let b = self.bucket(key)?;
        self.buckets[b].insert(0, (key, value));
        self.count = self.count.wrapping_add(1);
        Ok(())
    }
    /// `7ADDA0`: unlink the key's first node (head first); false when absent.
    pub fn erase(&mut self, key: u32) -> Result<bool> {
        let b = self.bucket(key)?;
        match self.buckets[b].iter().position(|(k, _)| *k == key) {
            Some(i) => {
                self.buckets[b].remove(i);
                self.count = self.count.wrapping_sub(1);
                Ok(true)
            }
            None => Ok(false),
        }
    }
    /// `7A6430`: the value of the key's first node, inserting `default` when
    /// the key is absent.
    pub fn find_or_insert(&mut self, key: u32, default: V) -> Result<&mut V> {
        if !self.contains(key)? {
            self.insert(key, default)?;
        }
        let b = self.bucket(key)?;
        Ok(self.buckets[b]
            .iter_mut()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
            .expect("inserted key"))
    }
}
