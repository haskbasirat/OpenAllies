//! 43B870's retained E0/E4/E8 intrusive list. Keys replace native pointers;
//! payloads move once, and the native count retains u32 wrapping semantics.
use std::sync::{Arc, Weak};

#[derive(Clone, Debug)]
pub struct Key {
    slot: usize,
    owner: Weak<()>,
}
impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && Weak::ptr_eq(&self.owner, &other.owner)
    }
}
impl Eq for Key {}

pub(crate) trait Message {
    fn set_next(&mut self, next: Option<Key>);
}

pub(crate) struct List<T> {
    identity: Arc<()>,
    nodes: Vec<T>,
    head: Option<Key>,
    tail: Option<Key>,
    count: u32,
}
impl<T> List<T> {
    pub(crate) fn from_native_constructor() -> Self {
        Self { identity: Arc::new(()), nodes: Vec::new(), head: None, tail: None, count: 0 }
    }
    pub fn head(&self) -> Option<&Key> { self.head.as_ref() }
    pub fn tail(&self) -> Option<&Key> { self.tail.as_ref() }
    pub fn native_count(&self) -> u32 { self.count }
    pub fn owned_count(&self) -> usize { self.nodes.len() }
    pub fn get(&self, key: &Key) -> Option<&T> {
        if Weak::ptr_eq(&key.owner, &Arc::downgrade(&self.identity)) {
            self.nodes.get(key.slot)
        } else { None }
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
        self.nodes.try_reserve(1).map_err(|e| format!("VWorld command storage: {e}"))?;
        retain_429020(command)?;
        let key = Key { slot: self.nodes.len(), owner: Arc::downgrade(&self.identity) };
        command.set_next(None);
        if self.head.is_none() {
            self.head = Some(key.clone());
        } else {
            //Only this closed append operation mutates links/head/tail.
            self.nodes[self.tail.as_ref().unwrap().slot].set_next(Some(key.clone()));
        }
        self.tail = Some(key.clone());
        self.count = self.count.wrapping_add(1);
        self.nodes.push(message.take().unwrap());
        Ok(key)
    }
}

#[cfg(test)]
#[path="view_fx_command_list_tests.rs"]
mod tests;
