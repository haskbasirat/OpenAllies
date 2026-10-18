//! TGC_Synchronizer (Synchronizer.cpp): the rolling desync hash and its
//! entry log. Two instances exist: A = [0xB3655C] (validated through
//! TGC_SetSyncValueOrder 302/314) and S = [0xB36560] (simulation log,
//! KKC_SetSimSyncValueOrder 299/311, compared only for film players).
//!
//! Native layout: +0 enabled, +1 file dump, +4 suspend count, +8 hash,
//! +0xC maximum retained entries, +0x10 windowed entry array (begin, end,
//! capacity, ring of 0x14-byte {label, kind, value, ?, hash-after}).
//! Constructor 0x77CCB0(max): disabled, hash -1, window [INT_MIN, INT_MIN).
//!
//! Hash update (0x77DACA..0x77DADE, same in all loggers):
//! `h = (h * 37 + v) % 65536` with 32-bit wrapping multiply/add and the
//! signed (truncating) remainder MSVC emits as `and 0x8000ffff` + fix-up.
//! `v` is the int (0x77DA70), the raw binary32 bits (0x77D930), 0/1
//! (0x77D9D0) or each UTF-16 code unit in turn (0x77DB10). Labels are stored
//! but never hashed.
//!
//! A logger records only when the instance is enabled, its suspend count is
//! below 1, the caller runs on the simulation thread ([0xB39828]) and the
//! application flags [0xB36528]+0x5C/+0x5D are clear. The last two are host
//! state; callers pass them as `gate`.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// 0x77CCB0 argument used for A and S.
pub const DEFAULT_MAX_ENTRIES: i32 = 0x10000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    Empty = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    Text = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub label: String,
    pub kind: EntryKind,
    /// Raw stored value (int, float bits, 0/1; 0 for strings).
    pub value: u32,
    /// Hash after this entry (+0x10).
    pub hash: i32,
}
impl Entry {
    fn empty() -> Self {
        Self { label: String::new(), kind: EntryKind::Empty, value: 0, hash: 0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Synchronizer {
    pub enabled: bool,
    pub file_dump: bool,
    pub suspend: i32,
    pub hash: i32,
    pub max_entries: i32,
    begin: i32,
    entries: VecDeque<Entry>,
}

/// One hash step.
pub fn step(hash: i32, value: u32) -> i32 {
    hash.wrapping_mul(37).wrapping_add(value as i32) % 65536
}

impl Synchronizer {
    /// 0x77CCB0.
    pub fn new(max_entries: i32) -> Self {
        Self {
            enabled: false,
            file_dump: false,
            suspend: 0,
            hash: -1,
            max_entries,
            begin: i32::MIN,
            entries: VecDeque::new(),
        }
    }
    pub fn begin(&self) -> i32 {
        self.begin
    }
    /// Window end (index the next entry receives).
    pub fn end(&self) -> i32 {
        self.begin.wrapping_add(self.entries.len() as i32)
    }
    pub fn entry(&self, index: i32) -> Option<&Entry> {
        if index < self.begin {
            return None;
        }
        self.entries.get(index.wrapping_sub(self.begin) as usize)
    }
    fn open(&self, gate: bool) -> bool {
        self.enabled && self.suspend < 1 && gate
    }
    /// Retire-to (0x77D7D0): entries below `begin` are cleared; the window
    /// end becomes max(end, begin).
    fn retire(&mut self, begin: i32) {
        if begin > self.begin {
            let drop = (begin.wrapping_sub(self.begin) as usize).min(self.entries.len());
            self.entries.drain(..drop);
            self.begin = begin;
        } else {
            // Moving begin backwards exposes ring slots; for the native callers
            // (reset 0x77D900 after the clear 0x77D6B0) the window is empty here.
            let pad = begin.abs_diff(self.begin) as usize;
            for _ in 0..pad {
                self.entries.push_front(Entry::empty());
            }
            self.begin = begin;
        }
    }
    /// Common tail of the loggers: append, then keep the last max-1 entries.
    fn push(&mut self, label: &str, kind: EntryKind, value: u32) {
        self.entries.push_back(Entry { label: label.to_owned(), kind, value, hash: self.hash });
        let floor = self.end().wrapping_sub(self.max_entries).wrapping_add(1);
        if self.begin < floor {
            self.retire(floor);
        }
    }
    /// 0x77DA70 (int) and 0x77D930 (float bits): same arithmetic.
    pub fn log_int(&mut self, gate: bool, label: &str, value: u32) {
        if self.open(gate) {
            self.hash = step(self.hash, value);
            self.push(label, EntryKind::Int, value);
        }
    }
    pub fn log_float_bits(&mut self, gate: bool, label: &str, bits: u32) {
        if self.open(gate) {
            self.hash = step(self.hash, bits);
            self.push(label, EntryKind::Float, bits);
        }
    }
    /// 0x77D9D0.
    pub fn log_bool(&mut self, gate: bool, label: &str, value: bool) {
        if self.open(gate) {
            self.hash = step(self.hash, u32::from(value));
            self.push(label, EntryKind::Bool, u32::from(value));
        }
    }
    ///77DC50: a scope marker hashes zero and stores native kind0/value0.
    /// Its label is retained but never contributes text code units to the hash.
    pub fn log_marker(&mut self, gate: bool, label: &str) {
        if self.open(gate) {
            self.hash = step(self.hash, 0);
            self.push(label, EntryKind::Empty, 0);
        }
    }
    /// 0x77DB10: one hash step per code unit; the entry stores the final hash.
    pub fn log_text(&mut self, gate: bool, label: &str, units: &[u16]) {
        if self.open(gate) {
            for &u in units {
                self.hash = step(self.hash, u32::from(u));
            }
            self.push(label, EntryKind::Text, 0);
        }
    }
    /// 0x77D900 (Host / joining client): continue from (index, hash).
    pub fn reset(&mut self, index: i32, hash: i32) {
        self.hash = hash;
        self.enabled = true;
        self.file_dump = true;
        // 0x77D6B0 clears the window to [INT_MIN, INT_MIN); 0x77D7D0(index+1)
        // then moves both ends to index+1.
        self.entries.clear();
        self.begin = index.wrapping_add(1);
    }
    /// 0x77D050 ValidateChecksum(index, hash). Disabled: true. Beyond the
    /// window end: false. Forgotten (below begin): true. Otherwise compare the
    /// stored hash; asking for `index == end` makes the native accessor
    /// 0x77D010 append one empty entry (hash 0) first.
    pub fn validate(&mut self, index: i32, hash: i32) -> bool {
        if !self.enabled {
            return true;
        }
        if self.end() < index {
            return false;
        }
        if self.begin <= index {
            if index == self.end() {
                self.entries.push_back(Entry::empty());
            }
            return self.entry(index).map(|e| e.hash) == Some(hash);
        }
        true
    }
}
