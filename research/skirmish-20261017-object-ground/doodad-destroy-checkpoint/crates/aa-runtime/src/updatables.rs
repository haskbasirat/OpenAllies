//! The temporary original world+284 table binding saved Updatable IDs to actual
//! objects. It is created from the first UPDATABLES word before object Load and
//! is distinct from both GTIDs and scheduler event identities.
use serde::Serialize;
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("invalid saved updatable table count")]
    Count,
    #[error("updatable table allocation failed")]
    Allocation,
    #[error("saved updatable ID {id} is outside the {count}-entry allocation")]
    Bounds { id: i32, count: usize },
    #[error("duplicate saved updatable ID {0}")]
    Duplicate(i32),
    #[error("updatable source: {0}")]
    Source(String),
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Debug, Serialize)]
pub struct Table<T> {
    slots: Vec<Option<T>>,
}
impl<T> Table<T> {
    /// Original50B470 allocates exactly count pointers, clears them, then
    /// publishes the table. Negative counts are rejected by world4FC9C8 first.
    pub fn new(count: i32) -> Result<Self> {
        if count < 0 {
            return Err(Error::Count);
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(count as usize)
            .map_err(|_| Error::Allocation)?;
        slots.resize_with(count as usize, || None);
        Ok(Self { slots })
    }
    pub fn from_chunk(chunk: &aa_mapworld::Chunk) -> Result<Self> {
        chunk.verify().map_err(|e| Error::Source(e.to_string()))?;
        if chunk.directory.name != "UPDATABLES" {
            return Err(Error::Source("wrong chunk".into()));
        }
        let word = chunk.bytes.get(..4).ok_or(Error::Count)?;
        Self::new(i32::from_le_bytes(word.try_into().unwrap()))
    }
    pub fn len(&self) -> usize {
        self.slots.len()
    }
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
    pub fn slots(&self) -> &[Option<T>] {
        &self.slots
    }
    /// Original50BCD0 only registers nonnegative IDs. Its comparison mistakenly
    /// permits id==count before dereferencing outside the allocation; that
    /// undefined memory access is an explicit malformed-input error in Rust.
    /// Prior entries remain committed on all failures.
    pub fn register(&mut self, id: i32, target: T) -> Result<bool> {
        if id < 0 {
            return Ok(false);
        }
        let count = self.slots.len();
        let slot = self
            .slots
            .get_mut(id as usize)
            .ok_or(Error::Bounds { id, count })?;
        if slot.is_some() {
            return Err(Error::Duplicate(id));
        }
        *slot = Some(target);
        Ok(true)
    }
    /// Original50BD70 accepts -1 as null; an unbound in-range slot is discarded
    /// when restoring a queued update. Values below -1 are corrupt input.
    pub fn resolve(&self, id: i32) -> Result<Option<&T>> {
        if id == -1 {
            return Ok(None);
        }
        if id < 0 {
            return Err(Error::Bounds {
                id,
                count: self.slots.len(),
            });
        }
        self.slots
            .get(id as usize)
            .map(Option::as_ref)
            .ok_or(Error::Bounds {
                id,
                count: self.slots.len(),
            })
    }
}
impl<T: PartialEq> Table<T> {
    /// Original50B120 removes the first matching pointer and leaves holes.
    pub fn remove(&mut self, target: &T) -> bool {
        for slot in &mut self.slots {
            if slot.as_ref() == Some(target) {
                *slot = None;
                return true;
            }
        }
        false
    }
}
