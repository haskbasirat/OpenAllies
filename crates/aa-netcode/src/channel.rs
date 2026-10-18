//! Reliable block channels: TGC_BlockSender (ctor 0x760390, serialize
//! 0x760A80, packet result 0x7608E0, retire 0x760A40, refill 0x760820, reset
//! 0x760800) and TGC_BlockReceiver (0x760F20; skip variant 0x75FEA0).
//!
//! Wire of one channel section:
//! * 1 bit present; 0 ends the section;
//! * 32 bits first block index `lo`;
//! * exactly `span` presence bits for `lo..lo+span`, each followed by the
//!   block payload (written by the source, vtable +0xC) when 1.
//!
//! Sender bookkeeping (per transport sequence): packet records
//! `{state 1 sent / 2 acked / 3 lost, first, end, 32-bit mask}` and per block
//! `{in-flight copies (-1 = acked), estimated bits, last sequence}`. Selection
//! makes `copies` passes; pass p takes blocks with exactly p copies in flight
//! whose estimate is below the remaining budget (`limit - span - 33` first),
//! keeping all chosen blocks within `span` indices. There is no wall-clock
//! timer: loss comes from ack bits (0x7608E0 with `acked == false`) and from
//! retiring records older than `sequence - 100` (send) or `ack - 32`.
//!
//! The native windows are growable ring arrays (initial capacity 0x100); here
//! they are deques with the same index arithmetic. The block-selection scratch
//! buffer (static, initial 0x80 bytes at 0xB28A44, doubles) is a Vec.
use crate::Result;
use aa_orders::Cursor;
use aa_orders::commands::WireWriter;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Source of blocks (native sender vtable slots +0, +4, +8, +0xC).
pub trait BlockSource {
    /// +0: index one past the last block that exists now (may create blocks:
    /// message-history blockifying, file read-ahead).
    fn available(&mut self) -> Result<i32>;
    /// +4: blocks below `index` are acknowledged and can be dropped.
    fn release(&mut self, index: i32) -> Result<()>;
    /// +8: estimated serialized bits of block `index`.
    fn estimate(&mut self, index: i32) -> Result<i32>;
    /// +0xC: write block `index`.
    fn serialize(&mut self, w: &mut WireWriter, index: i32) -> Result<()>;
}

/// Sink of blocks (native receiver vtable slots +0 receive, +4 skip).
pub trait BlockSink {
    fn receive(&mut self, r: &mut Cursor<'_>, index: i32) -> Result<()>;
    /// Decode and discard (used for stale file generations / backup indices).
    fn skip(&mut self, r: &mut Cursor<'_>, index: i32) -> Result<()>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketRecord {
    /// 0 none, 1 sent, 2 acknowledged, 3 lost.
    pub state: i32,
    pub first: i32,
    pub end: i32,
    pub mask: u32,
}
impl PacketRecord {
    /// Value of a fresh or cleared slot (0x760400 / 0x760660).
    pub const EMPTY: Self = Self { state: 0, first: i32::MIN, end: i32::MIN, mask: 0 };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRecord {
    /// Copies in flight; -1 once acknowledged.
    pub inflight: i32,
    pub estimate: i32,
    /// Last sequence carrying the block (native leaves it uninitialised
    /// until first sent; 0 here, never read before being written).
    pub last: i32,
}

/// Index-addressed window with native retire/extend semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window<T> {
    pub begin: i32,
    pub items: VecDeque<T>,
}
impl<T: Clone> Window<T> {
    pub fn new(begin: i32) -> Self {
        Self { begin, items: VecDeque::new() }
    }
    pub fn end(&self) -> i32 {
        self.begin.wrapping_add(self.items.len() as i32)
    }
    /// Extend to `end` with `fill` (0x760400 / 0x760490).
    pub fn extend_to(&mut self, end: i32, fill: T) {
        while self.end() < end {
            self.items.push_back(fill.clone());
        }
    }
    /// Retire to `begin` (0x760660 / 0x760770): end becomes max(end, begin).
    pub fn retire_to(&mut self, begin: i32, fill: T) {
        if begin >= self.begin {
            let drop = (begin.wrapping_sub(self.begin) as usize).min(self.items.len());
            self.items.drain(..drop);
            self.begin = begin;
        } else {
            // Moving begin backwards re-exposes cleared slots.
            for _ in 0..begin.abs_diff(self.begin) {
                self.items.push_front(fill.clone());
            }
            self.begin = begin;
        }
    }
    pub fn get(&self, index: i32) -> Option<&T> {
        if index < self.begin {
            return None;
        }
        self.items.get(index.wrapping_sub(self.begin) as usize)
    }
    pub fn get_mut(&mut self, index: i32) -> Option<&mut T> {
        if index < self.begin {
            return None;
        }
        self.items.get_mut(index.wrapping_sub(self.begin) as usize)
    }
    /// Clear to the empty window [INT_MIN, INT_MIN) (0x7605F0 / 0x760700).
    pub fn clear(&mut self) {
        self.items.clear();
        self.begin = i32::MIN;
    }
}

const BLOCK_FILL: BlockRecord = BlockRecord { inflight: 0, estimate: 0, last: 0 };

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockSender {
    pub packets: Window<PacketRecord>,
    pub blocks: Window<BlockRecord>,
    /// +0x24 presence-bit span per section (32 for orders, AVar
    /// NetworkFileTransferBlocksPerFrame for files).
    pub span: i32,
    /// +0x28 maximum copies of a block in flight (2 orders, 1 files).
    pub copies: i32,
}

impl BlockSender {
    /// 0x760390: both windows start empty at index 0 (capacity 0x100 natively).
    pub fn new(span: i32, copies: i32) -> Self {
        Self { packets: Window::new(0), blocks: Window::new(0), span, copies }
    }

    /// 0x760800: clear packet records and blocks, block window restarts at 0.
    pub fn reset(&mut self) {
        self.packets.clear();
        self.blocks.clear();
        self.blocks.retire_to(0, BLOCK_FILL);
    }

    /// 0x760590: first block in the window with no copy in flight, else the
    /// window end.
    pub fn first_unsent(&self) -> i32 {
        let mut index = self.blocks.begin;
        while index < self.blocks.end() {
            if self.blocks.get(index).map(|b| b.inflight) == Some(0) {
                return index;
            }
            index += 1;
        }
        self.blocks.end()
    }

    /// 0x760820.
    pub fn refill(&mut self, source: &mut dyn BlockSource) -> Result<()> {
        let mut index = self.blocks.begin;
        while index < self.blocks.end() {
            if self.blocks.get(index).map(|b| b.inflight) != Some(-1) {
                break;
            }
            self.blocks.retire_to(index + 1, BLOCK_FILL);
            source.release(index + 1)?;
            index += 1;
        }
        while self.blocks.end() < source.available()? {
            let estimate = source.estimate(self.blocks.end())?;
            let end = self.blocks.end();
            self.blocks.extend_to(end + 1, BlockRecord { inflight: 0, estimate, last: 0 });
        }
        Ok(())
    }

    /// 0x760A40: report records below `limit` as lost, then drop them.
    pub fn retire(&mut self, source: &mut dyn BlockSource, limit: i32) -> Result<()> {
        let mut i = self.packets.begin;
        if i < limit {
            while i < limit {
                if self.packets.end() <= i {
                    break;
                }
                self.packet_result(source, i, false)?;
                i += 1;
            }
            self.packets.retire_to(limit, PacketRecord::EMPTY);
        }
        Ok(())
    }

    /// 0x7608E0: transport result for `sequence`.
    pub fn packet_result(
        &mut self,
        source: &mut dyn BlockSource,
        sequence: i32,
        acked: bool,
    ) -> Result<()> {
        if sequence < self.packets.begin || sequence >= self.packets.end() {
            return Ok(());
        }
        let record = *self.packets.get(sequence).expect("inside window");
        if record.state == 0 || record.state == 2 || (!acked && record.state == 3) {
            return Ok(());
        }
        self.packets.get_mut(sequence).expect("inside window").state = if acked { 2 } else { 3 };
        let mut i = record.first;
        while i < record.end {
            let bit = 1u32 << ((i.wrapping_sub(record.first)) & 31);
            if record.mask & bit != 0 && i >= self.blocks.begin {
                if self.blocks.end() <= i {
                    self.blocks.extend_to(i + 1, BLOCK_FILL);
                }
                let block = self.blocks.get_mut(i).expect("extended");
                if acked {
                    block.inflight = -1;
                } else if block.inflight > 0 {
                    block.inflight -= 1;
                }
            }
            i += 1;
        }
        self.retire(source, sequence.wrapping_sub(32))?;
        self.refill(source)
    }

    /// 0x760A80: write one section for transport `sequence` within `limit`
    /// bits (`limit` = target absolute bit position minus the current one).
    pub fn serialize(
        &mut self,
        source: &mut dyn BlockSource,
        w: &mut WireWriter,
        sequence: i32,
        limit: i32,
    ) -> Result<()> {
        self.refill(source)?;
        let mut budget = limit.wrapping_sub(self.span).wrapping_sub(0x21);
        if self.blocks.begin == self.blocks.end() {
            w.write_uint(0, 1)?;
            return Ok(());
        }
        self.retire(source, sequence.wrapping_sub(100))?;
        let begin = self.blocks.begin;
        let end = self.blocks.end();
        let mut lo = end;
        let mut hi = begin - 1;
        let mut selected = vec![false; (end - begin) as usize];
        let mut pass = 0;
        while pass < self.copies {
            let mut idx = begin;
            while idx < end {
                let block = *self.blocks.get(idx).expect("inside window");
                if block.inflight as u32 == pass as u32 && block.estimate < budget {
                    let mut new_lo = lo;
                    if idx < lo {
                        new_lo = idx;
                        if self.span < hi - idx + 1 {
                            break;
                        }
                    }
                    lo = new_lo;
                    let mut new_hi = hi;
                    if hi < idx {
                        new_hi = idx;
                        if self.span < idx - lo + 1 {
                            break;
                        }
                    }
                    hi = new_hi;
                    budget = budget.wrapping_sub(block.estimate);
                    selected[(idx - begin) as usize] = true;
                }
                idx += 1;
            }
            pass += 1;
        }
        if hi < lo {
            w.write_uint(0, 1)?;
            return Ok(());
        }
        w.write_uint(1, 1)?;
        w.write_uint(lo as u32, 32)?;
        if sequence < self.packets.begin {
            return Err(crate::Error::Protocol(format!(
                "sequence {sequence} precedes the packet window {}",
                self.packets.begin
            )));
        }
        self.packets.extend_to(sequence + 1, PacketRecord::EMPTY);
        *self.packets.get_mut(sequence).expect("extended") =
            PacketRecord { state: 1, first: lo, end: hi + 1, mask: 0 };
        for idx in lo..=hi {
            let chosen = selected[(idx - begin) as usize];
            let record = self.packets.get_mut(sequence).expect("extended");
            let bit = 1u32 << ((idx.wrapping_sub(record.first)) & 31);
            if chosen {
                record.mask |= bit;
            } else {
                record.mask &= !bit;
            }
            w.write_uint(u32::from(chosen), 1)?;
            if chosen {
                let block = self.blocks.get_mut(idx).expect("inside window");
                block.inflight += 1;
                block.last = sequence;
                source.serialize(w, idx)?;
            }
        }
        let mut idx = hi + 1;
        while idx < self.span + lo {
            w.write_uint(0, 1)?;
            idx += 1;
        }
        Ok(())
    }
}

/// TGC_BlockReceiver: only the span (+4) is state; the sink holds blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockReceiver {
    pub span: i32,
}
impl BlockReceiver {
    /// 0x760F20.
    pub fn receive(&self, sink: &mut dyn BlockSink, r: &mut Cursor<'_>) -> Result<()> {
        if r.read_uint(1)? == 1 {
            let start = r.read_uint(32)? as i32;
            let mut idx = start;
            while idx < self.span.wrapping_add(start) {
                if r.read_uint(1)? == 1 {
                    sink.receive(r, idx)?;
                }
                idx += 1;
            }
        }
        Ok(())
    }
    /// 0x75FEA0.
    pub fn skip(&self, sink: &mut dyn BlockSink, r: &mut Cursor<'_>) -> Result<()> {
        if r.read_uint(1)? == 1 {
            let start = r.read_uint(32)? as i32;
            let mut idx = start;
            while idx < self.span.wrapping_add(start) {
                if r.read_uint(1)? == 1 {
                    sink.skip(r, idx)?;
                }
                idx += 1;
            }
        }
        Ok(())
    }
}
