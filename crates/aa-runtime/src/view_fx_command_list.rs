//! 43B870's retained E0/E4/E8 intrusive list. Keys replace native pointers;
//! payloads move once, and the native count retains u32 wrapping semantics.
//! 43EC60's FX segment pops the head (`pop_head`), then either re-appends the
//! same command (`reappend`) or frees it (`free`). A freed slot may be reused;
//! its generation makes every older key for that slot stale.
use std::sync::{Arc, Weak};

#[derive(Clone, Debug)]
pub struct Key {
    slot: usize,
    generation: u64,
    owner: Weak<()>,
}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && self.generation == other.generation && Weak::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for Key {}

pub(crate) trait Message {
    fn set_next(&mut self, next: Option<Key>);
    fn next(&self) -> Option<&Key>;
}

struct Slot<T> {
    generation: u64,
    value: Option<T>,
}

pub(crate) struct List<T> {
    identity: Arc<()>,
    nodes: Vec<Slot<T>>,
    vacant: Vec<usize>,
    head: Option<Key>,
    tail: Option<Key>,
    pub(crate) count: u32,
}
impl<T> List<T> {
    pub(crate) fn from_native_constructor() -> Self {
        Self { identity: Arc::new(()), nodes: Vec::new(), vacant: Vec::new(), head: None, tail: None, count: 0 }
    }
    pub fn head(&self) -> Option<&Key> { self.head.as_ref() }
    pub fn tail(&self) -> Option<&Key> { self.tail.as_ref() }
    pub fn native_count(&self) -> u32 { self.count }
    pub fn owned_count(&self) -> usize { self.nodes.iter().filter(|s| s.value.is_some()).count() }
    fn owns(&self, key: &Key) -> bool {
        Weak::ptr_eq(&key.owner, &Arc::downgrade(&self.identity))
            && self.nodes.get(key.slot).is_some_and(|s| s.generation == key.generation && s.value.is_some())
    }
    pub fn get(&self, key: &Key) -> Option<&T> {
        if self.owns(key) { self.nodes[key.slot].value.as_ref() } else { None }
    }
    pub(crate) fn get_mut(&mut self, key: &Key) -> Option<&mut T> {
        if self.owns(key) { self.nodes[key.slot].value.as_mut() } else { None }
    }
    /// The command leaves storage after its completion (6F7430).
    pub(crate) fn free(&mut self, key: &Key) -> Option<T> {
        if !self.owns(key) {
            return None;
        }
        let slot = &mut self.nodes[key.slot];
        slot.generation += 1;
        self.vacant.push(key.slot);
        slot.value.take()
    }
}
impl<T: Message> List<T> {
    ///429020 runs before any link changes. On refusal the caller retains its
    ///actual payload and this list remains unchanged. Host storage reservation
    ///is a portable allocation preflight, not native allocator-failure parity.
    pub(crate) fn append(
        &mut self,
        message: &mut Option<T>,
        retain_429020: impl FnOnce(&mut T) -> Result<(), String>,
    ) -> Result<Key, String> {
        let command = message.as_mut().ok_or("43B870 requires nonnull command at429023")?;
        if self.vacant.is_empty() {
            self.nodes.try_reserve(1).map_err(|e| format!("VWorld command storage: {e}"))?;
        }
        retain_429020(command)?;
        let slot = match self.vacant.pop() {
            Some(slot) => slot,
            None => {
                self.nodes.push(Slot { generation: 0, value: None });
                self.nodes.len() - 1
            }
        };
        let key = Key { slot, generation: self.nodes[slot].generation, owner: Arc::downgrade(&self.identity) };
        command.set_next(None);
        self.nodes[slot].value = message.take();
        self.link_tail(key.clone());
        Ok(key)
    }
    fn link_tail(&mut self, key: Key) {
        if self.head.is_none() {
            self.head = Some(key.clone());
        } else {
            //Only these closed operations mutate links/head/tail.
            let tail = self.tail.clone().expect("a nonempty list has a tail");
            if let Some(t) = self.get_mut(&tail) {
                t.set_next(Some(key.clone()));
            }
        }
        self.tail = Some(key);
        self.count = self.count.wrapping_add(1);
    }
    /// 43ED60: take the head (head = head+68, count - 1). The tail is not
    /// touched, exactly as natively; a null head returns None.
    pub(crate) fn pop_head(&mut self) -> Option<Key> {
        let head = self.head.clone()?;
        self.head = self.get(&head).and_then(|c| c.next().cloned());
        self.count = self.count.wrapping_sub(1);
        Some(head)
    }
    /// 43EDA0: +68 = 0, then the tail append (head when the list is empty).
    pub(crate) fn reappend(&mut self, key: Key) -> Result<(), String> {
        let command = self.get_mut(&key).ok_or("43EDA0 command is not owned by this VWorld list")?;
        command.set_next(None);
        self.link_tail(key);
        Ok(())
    }
}

#[cfg(test)]
#[path="view_fx_command_list_tests.rs"]
mod tests;
