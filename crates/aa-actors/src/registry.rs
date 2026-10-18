//! Native GTIDManager (4EFDB0/4EFEC0/4F0190), independent of component payloads.
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SLOT_COUNT: usize = 65_536;
/// Original Add gate, not a host memory allowance. Saved-ID registration is separate.
pub const NATIVE_ADD_LIMIT: usize = 32_768;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ActorId(u32);
impl ActorId {
    pub const NULL: Self = Self(0);
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
    pub const fn raw(self) -> u32 {
        self.0
    }
    pub const fn slot(self) -> u16 {
        self.0 as u16
    }
    pub const fn generation(self) -> u16 {
        (self.0 >> 16) as u16
    }
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry<T> {
    pub id: ActorId,
    pub is_actor: bool,
    pub value: T,
}

/// Rust checkpoint of proved registry state, not a native RSG codec.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryCheckpoint<T> {
    pub generations: Vec<u16>,
    pub entries: Vec<Entry<T>>,
    pub next_slot: u16,
    pub highest_slot: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum RegistryError {
    #[error("native Add active-object limit reached")]
    Exhausted,
    #[error("slot zero is reserved by native allocation")]
    ReservedSlot,
    #[error("saved ID slot is already occupied")]
    Occupied,
    #[error("saved ID generation does not match registry")]
    GenerationMismatch,
    #[error("ID no longer identifies a live registered object")]
    Stale,
    #[error("invalid registry checkpoint")]
    InvalidCheckpoint,
}

#[derive(Clone, Debug)]
pub struct IdentityRegistry<T> {
    generations: Vec<u16>,
    entries: Vec<Option<Box<Entry<T>>>>,
    occupied: Vec<u32>,
    active: usize,
    next_slot: u16,
    highest_slot: i32,
}

impl<T> Default for IdentityRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T> IdentityRegistry<T> {
    /// Exact native Reset state: zero generations, empty slots, cursor1, high0.
    pub fn new() -> Self {
        Self {
            generations: vec![0; SLOT_COUNT],
            entries: (0..SLOT_COUNT).map(|_| None).collect(),
            occupied: vec![0; SLOT_COUNT / 32],
            active: 0,
            next_slot: 1,
            highest_slot: 0,
        }
    }
    /// Native 4F0230 intermediate state after Reset and IDMGR metadata load,
    /// before any saved object registrations. The stream stores exactly
    /// `highest_slot` generations, not highest+1. Unstored slots remain zero.
    /// This deliberately permits a positive highest slot with empty entries;
    /// it is not a completed-world checkpoint. Version/capacity are validated
    /// by the native stream decoder before calling this method.
    pub fn from_load_metadata(
        next_slot: u32,
        highest_slot: u32,
        serialized_generations: &[u16],
    ) -> Result<Self, RegistryError> {
        if !(1..SLOT_COUNT as u32).contains(&next_slot)
            || highest_slot >= SLOT_COUNT as u32
            || serialized_generations.len() != highest_slot as usize
        {
            return Err(RegistryError::InvalidCheckpoint);
        }
        let mut out = Self::new();
        out.next_slot = next_slot as u16;
        out.highest_slot = highest_slot as i32;
        out.generations[..serialized_generations.len()].copy_from_slice(serialized_generations);
        Ok(out)
    }
    /// Moves existing payloads into another owned representation without any
    /// identity allocation, removal, generation changes or constructor calls.
    /// Callback order is ascending occupied slot. Rust wrapper storage may be
    /// reallocated, while component boxes inside a moved payload can retain their
    /// allocation. Native pointer addresses are never game identities here.
    pub fn map_values<U>(self, mut map: impl FnMut(ActorId, bool, T) -> U) -> IdentityRegistry<U> {
        IdentityRegistry {
            generations: self.generations,
            entries: self
                .entries
                .into_iter()
                .map(|entry| {
                    entry.map(|entry| {
                        let Entry {
                            id,
                            is_actor,
                            value,
                        } = *entry;
                        Box::new(Entry {
                            id,
                            is_actor,
                            value: map(id, is_actor, value),
                        })
                    })
                })
                .collect(),
            occupied: self.occupied,
            active: self.active,
            next_slot: self.next_slot,
            highest_slot: self.highest_slot,
        }
    }
    pub fn len(&self) -> usize {
        self.active
    }
    pub fn is_empty(&self) -> bool {
        self.active == 0
    }
    pub fn next_slot(&self) -> u16 {
        self.next_slot
    }
    pub fn highest_slot(&self) -> i32 {
        self.highest_slot
    }
    pub fn generation_at(&self, slot: u16) -> u16 {
        self.generations[usize::from(slot)]
    }
    pub fn occupied_words(&self) -> &[u32] {
        &self.occupied
    }
    fn advance(slot: u16) -> u16 {
        if slot == u16::MAX { 1 } else { slot + 1 }
    }
    fn install(&mut self, id: ActorId, is_actor: bool, value: T) {
        let slot = usize::from(id.slot());
        self.entries[slot] = Some(Box::new(Entry {
            id,
            is_actor,
            value,
        }));
        self.active += 1;
        self.occupied[slot >> 5] |= 1 << (slot & 31);
        self.highest_slot = self.highest_slot.max(slot as i32);
    }
    pub fn allocate(&mut self, is_actor: bool, value: T) -> Result<ActorId, RegistryError> {
        if self.active >= NATIVE_ADD_LIMIT {
            return Err(RegistryError::Exhausted);
        }
        while self.entries[usize::from(self.next_slot)].is_some() {
            self.next_slot = Self::advance(self.next_slot);
        }
        let slot = self.next_slot;
        let id = ActorId::from_raw(
            (u32::from(self.generations[usize::from(slot)]) << 16) | u32::from(slot),
        );
        self.install(id, is_actor, value);
        self.next_slot = Self::advance(slot);
        Ok(id)
    }
    /// Native SetGeneration is used by saved-record initialization before AddID.
    /// Rust requires the valid loader precondition: the slot is free and nonzero.
    pub fn seed_saved_generation(&mut self, id: ActorId) -> Result<(), RegistryError> {
        let slot = usize::from(id.slot());
        if slot == 0 {
            return Err(RegistryError::ReservedSlot);
        }
        if self.entries[slot].is_some() {
            return Err(RegistryError::Occupied);
        }
        self.generations[slot] = id.generation();
        Ok(())
    }
    /// Original saved-ID Add does not apply Add's 32768 active-count gate.
    pub fn register_saved(
        &mut self,
        id: ActorId,
        is_actor: bool,
        value: T,
    ) -> Result<(), RegistryError> {
        let slot = usize::from(id.slot());
        if slot == 0 {
            return Err(RegistryError::ReservedSlot);
        }
        if self.entries[slot].is_some() {
            return Err(RegistryError::Occupied);
        }
        if self.generations[slot] != id.generation() {
            return Err(RegistryError::GenerationMismatch);
        }
        self.install(id, is_actor, value);
        Ok(())
    }
    pub fn get(&self, id: ActorId) -> Option<&Entry<T>> {
        let slot = usize::from(id.slot());
        if self.generations[slot] != id.generation() {
            return None;
        }
        self.entries[slot].as_deref()
    }
    pub fn value_mut(&mut self, id: ActorId) -> Option<&mut T> {
        let slot = usize::from(id.slot());
        if self.generations[slot] != id.generation() {
            return None;
        }
        self.entries[slot]
            .as_deref_mut()
            .map(|entry| &mut entry.value)
    }
    /// Native 4EFCF0 additionally checks the object's virtual IsActor predicate.
    pub fn get_actor(&self, id: ActorId) -> Option<&Entry<T>> {
        self.get(id).filter(|e| e.is_actor)
    }
    /// Domain is a currently live object, as in native Remove(pointer). A stale
    /// Rust handle is rejected instead of permitting a dangling pointer call.
    pub fn remove(&mut self, id: ActorId) -> Result<Entry<T>, RegistryError> {
        if self.get(id).is_none() {
            return Err(RegistryError::Stale);
        }
        let slot = usize::from(id.slot());
        let entry = *self.entries[slot].take().unwrap();
        self.active -= 1;
        self.occupied[slot >> 5] &= !(1 << (slot & 31));
        self.generations[slot] = self.generations[slot].wrapping_add(1);
        if self.highest_slot == slot as i32 {
            while self.highest_slot >= 0 {
                self.highest_slot -= 1;
                if self.highest_slot >= 0 && self.entries[self.highest_slot as usize].is_some() {
                    break;
                }
            }
        }
        // Native leaves the removed object's old ID field intact until destruction.
        Ok(entry)
    }
    /// Pointer-preserving identity exchange used by native 4EFB90. Both entries
    /// must be live; its special null-pointer overload is deliberately excluded.
    pub fn swap_identities(&mut self, a: ActorId, b: ActorId) -> Result<(), RegistryError> {
        if self.get(a).is_none() || self.get(b).is_none() {
            return Err(RegistryError::Stale);
        }
        let x = usize::from(a.slot());
        let y = usize::from(b.slot());
        self.entries.swap(x, y);
        self.entries[x].as_mut().unwrap().id = a;
        self.entries[y].as_mut().unwrap().id = b;
        Ok(())
    }
    /// Native 4EFBE0 enumerates occupied slots ascending, not insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &Entry<T>> {
        self.entries.iter().skip(1).filter_map(|e| e.as_deref())
    }
    pub fn checkpoint(&self) -> RegistryCheckpoint<T>
    where
        T: Clone,
    {
        RegistryCheckpoint {
            generations: self.generations.clone(),
            entries: self.iter().cloned().collect(),
            next_slot: self.next_slot,
            highest_slot: self.highest_slot,
        }
    }
    pub fn from_checkpoint(checkpoint: RegistryCheckpoint<T>) -> Result<Self, RegistryError> {
        if checkpoint.generations.len() != SLOT_COUNT || checkpoint.next_slot == 0 {
            return Err(RegistryError::InvalidCheckpoint);
        }
        let mut out = Self::new();
        out.generations = checkpoint.generations;
        out.next_slot = checkpoint.next_slot;
        for entry in checkpoint.entries {
            let slot = usize::from(entry.id.slot());
            if slot == 0
                || out.entries[slot].is_some()
                || out.generations[slot] != entry.id.generation()
            {
                return Err(RegistryError::InvalidCheckpoint);
            }
            out.install(entry.id, entry.is_actor, entry.value);
        }
        let expected = out.iter().last().map(|e| i32::from(e.id.slot()));
        if expected.is_some_and(|highest| highest != checkpoint.highest_slot)
            || expected.is_none() && ![-1, 0].contains(&checkpoint.highest_slot)
        {
            return Err(RegistryError::InvalidCheckpoint);
        }
        out.highest_slot = checkpoint.highest_slot;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_are_slots_and_generations_not_dense_actor_indices() {
        let mut r = IdentityRegistry::new();
        let a = r.allocate(true, "a").unwrap();
        let b = r.allocate(false, "b").unwrap();
        assert_eq!(a.raw(), 1);
        assert_eq!(b.raw(), 2);
        assert!(r.get_actor(b).is_none());
        assert_eq!(r.remove(a).unwrap().id, a);
        assert!(r.get(a).is_none());
        assert_eq!(r.generation_at(1), 1);
        assert_eq!(r.allocate(true, "c").unwrap().raw(), 3);
    }
    #[test]
    fn saved_ids_and_swaps_keep_exact_handles() {
        let mut r = IdentityRegistry::new();
        let a = ActorId::from_raw(0xffff_ffff);
        r.seed_saved_generation(a).unwrap();
        r.register_saved(a, true, "a").unwrap();
        let b = r.allocate(true, "b").unwrap();
        r.swap_identities(a, b).unwrap();
        assert_eq!(r.get(a).unwrap().value, "b");
        r.remove(a).unwrap();
        assert_eq!(r.generation_at(u16::MAX), 0);
        assert_eq!(r.highest_slot(), 1);
        r.remove(b).unwrap();
        assert_eq!(r.highest_slot(), -1);
    }
}
