//! Sole owned GObject+58/+5C detection-memory allocation and record identities.
//!4EA690 initializes count/pointer zero;4EA3E0 stores the count before its
//!allocation;4EB9F0 initializes each record with two uninitialized stat words.
//!Native pointers are local stable IDs, never diagnostic integer mirrors.
use crate::ActorId;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecordId(u64);
impl RecordId {
    pub fn raw(self) -> u64 {
        self.0
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub timestamp_bits: u32,
    pub name_utf16: Vec<u16>,
    pub position_bits: [u32; 2],
    pub definition: Option<String>,
    pub owner: ActorId,
    pub aspect: u32,
    ///+1C/+20/+24/+28/+2C. The constructor does NOT write+24/+28.
    pub stat_words: [Option<u32>; 5],
    pub byte_30: u8,
    pub flags: u32,
}
impl Record {
    pub fn from_native_constructor() -> Self {
        Self {
            timestamp_bits: 0xbf800000,
            name_utf16: Vec::new(),
            position_bits: [0; 2],
            definition: None,
            owner: ActorId::NULL,
            aspect: 0,
            stat_words: [Some(0), Some(0), None, None, Some(0)],
            byte_30: 0,
            flags: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct State {
    count: i32,
    slots: Option<Vec<Option<RecordId>>>,
    records: BTreeMap<RecordId, Record>,
    next: u64,
}
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("detection-memory allocation failed")]
    Allocation,
    #[error("detection-memory identity exhausted")]
    Identity,
    #[error("detection-memory slot allocation is unavailable")]
    MissingSlots,
    #[error("detection-memory index is outside the actual allocation")]
    Index,
    #[error("detection-memory record is unavailable")]
    MissingRecord,
    #[error("detection-memory slots still retain records")]
    Occupied,
    #[error("detection-memory slot allocation already exists")]
    AlreadyAllocated,
}
pub type Result<T> = std::result::Result<T, Error>;
impl State {
    pub fn from_object_constructor() -> Self {
        Self {
            count: 0,
            slots: None,
            records: BTreeMap::new(),
            next: 1,
        }
    }
    pub fn count(&self) -> i32 {
        self.count
    }
    pub fn allocation_present(&self) -> bool {
        self.slots.is_some()
    }
    ///4EA3E0: stores+58 before allocating+5C. Allocation failure therefore
    ///retains the new count and the old absent pointer.
    pub fn allocate_slots(&mut self, count: i32) -> Result<()> {
        if self.count > 0 {
            return Err(Error::AlreadyAllocated);
        }
        self.count = count;
        let count = usize::try_from(count).map_err(|_| Error::Index)?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        slots.resize(count, None);
        self.slots = Some(slots);
        Ok(())
    }
    pub fn slot(&self, index: u32) -> Result<Option<RecordId>> {
        self.slots
            .as_ref()
            .ok_or(Error::MissingSlots)?
            .get(index as usize)
            .copied()
            .ok_or(Error::Index)
    }
    pub fn set_slot(&mut self, index: u32, value: Option<RecordId>) -> Result<()> {
        if let Some(value) = value {
            self.record(value)?;
        }
        *self
            .slots
            .as_mut()
            .ok_or(Error::MissingSlots)?
            .get_mut(index as usize)
            .ok_or(Error::Index)? = value;
        Ok(())
    }
    ///Successful native allocation followed by4EB9F0. A later refresh error
    ///leaves this actual allocation in the arena before any slot publication.
    pub fn allocate_record(&mut self) -> Result<RecordId> {
        let id = RecordId(self.next);
        self.next = self.next.checked_add(1).ok_or(Error::Identity)?;
        self.records.insert(id, Record::from_native_constructor());
        Ok(id)
    }
    pub fn record(&self, id: RecordId) -> Result<&Record> {
        self.records.get(&id).ok_or(Error::MissingRecord)
    }
    pub fn record_mut(&mut self, id: RecordId) -> Result<&mut Record> {
        self.records.get_mut(&id).ok_or(Error::MissingRecord)
    }
    pub fn free_record(&mut self, id: RecordId) -> Result<()> {
        self.records
            .remove(&id)
            .map(|_| ())
            .ok_or(Error::MissingRecord)
    }
    pub fn allocated_records(&self) -> usize {
        self.records.len()
    }
    pub fn release_slots(&mut self) -> Result<()> {
        if self
            .slots
            .as_ref()
            .is_some_and(|s| s.iter().any(Option::is_some))
        {
            return Err(Error::Occupied);
        }
        self.slots = None;
        self.count = 0;
        Ok(())
    }
}
