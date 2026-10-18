//! TGC_MessageHistorySender / TGC_MessageHistoryReceiver (MessageHistory.cpp):
//! the order request and decree packet histories carried over a block channel.
//!
//! Sender (ctor 0x763520: BlockSender span 32, copies 2; vtable 0xA921B4):
//! * reset 0x762B20: block-sender reset 0x760800, unbind, block window cleared
//!   and restarted at 0, next history index 0;
//! * bind 0x763600(history, start): reset, bind, next = start, then block 0 =
//!   header 0x762BD0: a finalized bitstream holding the 32-bit start index;
//! * available 0x762FC0 -> 0x762F60 blockifies every history packet from
//!   `next` to the history end (0x762D30): the packet body is cut into blocks
//!   of 255 bits while at least 256 remain, then one last block with the
//!   rest (an empty packet gives one empty last block);
//! * release 0x762FE0 -> 0x7628C0 (block window retire-to);
//! * estimate 0x7625A0: header bits + 32 for block 0, else bits + 1 + 8 if last;
//! * serialize 0x762FF0: block 0 as a byte buffer 0x75FFA0 (32-bit length, then
//!   one 8-bit write per byte), others 0x761FA0 (1 bit last, 8-bit bit count
//!   only when last, then the bits copied one by one by 0x75FF50).
//!
//! Receiver (ctor 0x7636B0: span 32, 0x800-byte reassembly stream; vtable
//! 0xA92238): reset 0x763080, bind 0x763130 (clears the bound history to
//! [INT_MIN, INT_MIN) with 0x7507D0); receive 0x7637D0 reads block 0 as a byte
//! buffer (0x7610A0) and others as bit blocks (0x7633C0), stores blocks at or
//! after the window begin and reassembles in order (0x7632D0): block 0 sets the
//! history window to the start index (0x7631D0 -> 0x750880), others append bits
//! (0x7626F0) and a last block appends the finalized packet at the history end;
//! each processed block retires the window to its index + 1 (0x762A50). Skip
//! 0x7638B0 always parses a bit block, even for index 0 (native behaviour).
use crate::channel::{BlockReceiver, BlockSender, BlockSink, BlockSource, Window};
use crate::{Error, Result};
use aa_orders::commands::WireWriter;
use aa_orders::net::transport::finalize;
use aa_orders::{Cursor, History, Packet, Position};

/// 0x762D30: maximum bits of a non-last block.
pub const BLOCK_BITS: u32 = 0xff;
/// 0x763520 / 0x7636B0 span and copies.
pub const HISTORY_SPAN: i32 = 0x20;
pub const HISTORY_COPIES: i32 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryBlock {
    /// History index of the packet the block belongs to (start for the header).
    pub index: i32,
    pub last: bool,
    pub bits: i32,
    /// Finalized bitstream (3-bit pad header) holding the block bits.
    pub data: Option<Packet>,
}
impl HistoryBlock {
    fn empty() -> Self {
        Self { index: 0, last: false, bits: 0, data: None }
    }
}

fn copy_bits(from: &mut Cursor<'_>, to: &mut WireWriter, count: u32) -> Result<()> {
    for _ in 0..count {
        let bit = from.read_uint(1)?;
        to.write_uint(u32::from(bit == 1), 1)?;
    }
    Ok(())
}

fn body_cursor(packet: &Packet) -> Result<Cursor<'_>> {
    Ok(Cursor::for_packet(packet.bytes(), None)?)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistorySender {
    pub sender: BlockSender,
    pub blocks: Window<HistoryBlock>,
    /// +0x2C bound history present.
    pub bound: bool,
    /// +0x44 next history index to blockify.
    pub next: i32,
}

impl Default for HistorySender {
    fn default() -> Self {
        let mut s = Self {
            sender: BlockSender::new(HISTORY_SPAN, HISTORY_COPIES),
            blocks: Window::new(0),
            bound: false,
            next: 0,
        };
        s.reset();
        s
    }
}

impl HistorySender {
    /// 0x762B20.
    pub fn reset(&mut self) {
        self.sender.reset();
        self.bound = false;
        self.blocks.clear();
        self.blocks.retire_to(0, HistoryBlock::empty());
        self.next = 0;
    }
    /// 0x763600.
    pub fn bind(&mut self, start: i32) -> Result<()> {
        self.reset();
        self.bound = true;
        self.next = start;
        // 0x762BD0: header block.
        let mut w = WireWriter::new(Position::PACKET_START)?;
        w.write_uint(start as u32, 32)?;
        let bits = (w.position().absolute()? - 3) as i32;
        let data = Packet::new(finalize(&w)?)?;
        let end = self.blocks.end();
        self.blocks.extend_to(end + 1, HistoryBlock::empty());
        *self.blocks.get_mut(end).expect("extended") =
            HistoryBlock { index: start, last: true, bits, data: Some(data) };
        Ok(())
    }
    /// One channel section for transport `sequence` within `limit` bits.
    pub fn serialize(
        &mut self,
        history: &History,
        w: &mut WireWriter,
        sequence: i32,
        limit: i32,
    ) -> Result<()> {
        let mut source = SenderSource { blocks: &mut self.blocks, next: &mut self.next, bound: self.bound, history };
        self.sender.serialize(&mut source, w, sequence, limit)
    }
    /// Transport ack/loss for `sequence` (via AdminConn 0x74B4F0 -> 0x7608E0).
    pub fn packet_result(&mut self, history: &History, sequence: i32, acked: bool) -> Result<()> {
        let mut source = SenderSource { blocks: &mut self.blocks, next: &mut self.next, bound: self.bound, history };
        self.sender.packet_result(&mut source, sequence, acked)
    }
    /// 0x762640: history index of the oldest block never sent (first block
    /// with no copy in flight, 0x760590), or the bound history's end when
    /// every block was sent or none exists. The native dereferences the bound
    /// history without a check (0x76268E); unbound is a typed error here.
    pub fn oldest_unsent_index(&self, history: &History) -> Result<i32> {
        let first = self.sender.first_unsent();
        if self.blocks.begin == self.blocks.end() || first >= self.blocks.end() {
            if !self.bound {
                return Err(Error::Protocol("unbound history sender (native null dereference 0x76268E)".into()));
            }
            return Ok(history.end());
        }
        Ok(self.blocks.get(first).map_or(0, |b| b.index))
    }
    /// 0x7626A0: history index of the oldest block the block sender still
    /// holds (+0x14 window begin, looked up in the block ring +0x30, grown
    /// by 0x762410 when beyond its end); the bound history's end when the
    /// block ring is empty; 0 when unbound. Used by the status block
    /// 0x74C6D0 and the send gate 0x74BDE0.
    pub fn oldest_pending_index(&mut self, history: &History) -> i32 {
        if !self.bound {
            return 0;
        }
        if self.blocks.begin == self.blocks.end() {
            return history.end();
        }
        let index = self.sender.blocks.begin;
        if self.blocks.end() <= index {
            self.blocks.extend_to(index + 1, HistoryBlock::empty());
        }
        self.blocks.get(index).map_or(0, |b| b.index)
    }
}

struct SenderSource<'a> {
    blocks: &'a mut Window<HistoryBlock>,
    next: &'a mut i32,
    bound: bool,
    history: &'a History,
}
impl SenderSource<'_> {
    /// 0x762D30.
    fn blockify(&mut self, index: i32, packet: Option<&Packet>) -> Result<()> {
        let mut cursor = match packet {
            Some(p) => Some(body_cursor(p)?),
            None => None,
        };
        loop {
            let remaining = cursor.as_ref().map_or(0, |c| c.remaining_bits() as u32);
            let last = remaining < 0x100;
            let bits = if last { remaining } else { BLOCK_BITS };
            let mut w = WireWriter::new(Position::PACKET_START)?;
            if let Some(c) = cursor.as_mut() {
                copy_bits(c, &mut w, bits)?;
            }
            let data = Packet::new(finalize(&w)?)?;
            let end = self.blocks.end();
            self.blocks.extend_to(end + 1, HistoryBlock::empty());
            *self.blocks.get_mut(end).expect("extended") =
                HistoryBlock { index, last, bits: bits as i32, data: Some(data) };
            match cursor.as_ref() {
                None => break,
                Some(c) if c.remaining_bits() == 0 => break,
                _ => {}
            }
        }
        Ok(())
    }
    fn block(&mut self, index: i32) -> &HistoryBlock {
        if self.blocks.end() <= index {
            self.blocks.extend_to(index + 1, HistoryBlock::empty());
        }
        self.blocks.get(index).expect("extended")
    }
}
impl BlockSource for SenderSource<'_> {
    fn available(&mut self) -> Result<i32> {
        if self.bound {
            while *self.next < self.history.end() {
                let index = *self.next;
                let packet = self.history.get(index)?.cloned();
                self.blockify(index, packet.as_ref())?;
                *self.next += 1;
            }
        }
        Ok(self.blocks.end())
    }
    fn release(&mut self, index: i32) -> Result<()> {
        self.blocks.retire_to(index, HistoryBlock::empty());
        Ok(())
    }
    fn estimate(&mut self, index: i32) -> Result<i32> {
        if index == 0 {
            if self.blocks.end() < 1 {
                self.blocks.extend_to(1, HistoryBlock::empty());
            }
            // 0x7625A0 reads slot 0 of the ring (block 0 is the header).
            return Ok(self.blocks.get(0).map_or(0, |b| b.bits) + 0x20);
        }
        let b = self.block(index);
        Ok(b.bits + 1 + if b.last { 8 } else { 0 })
    }
    fn serialize(&mut self, w: &mut WireWriter, index: i32) -> Result<()> {
        if index == 0 {
            let data = self.block(0).data.clone().ok_or(Error::Protocol("history header missing".into()))?;
            let bytes = data.bytes();
            w.write_uint(bytes.len() as u32, 32)?;
            for &b in bytes {
                w.write_uint(u32::from(b), 8)?;
            }
            return Ok(());
        }
        let block = self.block(index).clone();
        w.write_uint(u32::from(block.last), 1)?;
        if block.last {
            w.write_uint(block.bits as u32, 8)?;
        }
        if let Some(data) = block.data.as_ref() {
            let mut c = body_cursor(data)?;
            copy_bits(&mut c, w, block.bits as u32)?;
        } else if block.bits > 0 {
            return Err(Error::Protocol("history block without data".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedBlock {
    pub last: bool,
    pub bits: i32,
    pub data: Option<Vec<u8>>,
}
impl ReceivedBlock {
    fn empty() -> Self {
        Self { last: false, bits: 0, data: None }
    }
}

#[derive(Debug)]
pub struct HistoryReceiver {
    pub receiver: BlockReceiver,
    pub blocks: Window<ReceivedBlock>,
    /// +0x20 reassembly stream (0x800-byte initial buffer natively; growable).
    pub reassembly: WireWriter,
    pub bound: bool,
}

impl HistoryReceiver {
    pub fn new() -> Result<Self> {
        let mut r = Self {
            receiver: BlockReceiver { span: HISTORY_SPAN },
            blocks: Window::new(0),
            reassembly: WireWriter::new(Position::PACKET_START)?,
            bound: false,
        };
        r.reset();
        Ok(r)
    }
    /// 0x763080.
    pub fn reset(&mut self) {
        self.bound = false;
        self.blocks.clear();
        self.blocks.retire_to(0, ReceivedBlock::empty());
    }
    /// 0x763130: reset, bind and clear the bound history (0x7507D0).
    pub fn bind(&mut self, history: &mut History) -> Result<()> {
        self.reset();
        self.bound = true;
        clear_history(history)?;
        Ok(())
    }
    /// Read one channel section (0x760F20 with this receiver's sink).
    pub fn receive(&mut self, history: &mut History, r: &mut Cursor<'_>) -> Result<()> {
        let receiver = self.receiver;
        let mut sink = ReceiverSink { blocks: &mut self.blocks, reassembly: &mut self.reassembly, history, bound: self.bound };
        receiver.receive(&mut sink, r)
    }
    /// 0x75FEA0 with this receiver (decode and discard).
    pub fn skip(&mut self, history: &mut History, r: &mut Cursor<'_>) -> Result<()> {
        let receiver = self.receiver;
        let mut sink = ReceiverSink { blocks: &mut self.blocks, reassembly: &mut self.reassembly, history, bound: self.bound };
        receiver.skip(&mut sink, r)
    }
}

/// 0x7507D0: every entry released, window [INT_MIN, INT_MIN).
pub fn clear_history(history: &mut History) -> Result<()> {
    let capacity = history.capacity();
    *history = History::new(i32::MIN, capacity)?;
    Ok(())
}

struct ReceiverSink<'a> {
    blocks: &'a mut Window<ReceivedBlock>,
    reassembly: &'a mut WireWriter,
    history: &'a mut History,
    /// Bound history present; the native reassembly dereferences it
    /// without a check (header 0x7631D0 at 0x76322C, completion 0x7626F0).
    bound: bool,
}
fn unbound() -> Error {
    Error::Protocol("history block for an unbound receiver (native null dereference 0x76322C)".into())
}
/// 0x7633C0.
fn read_bit_block(r: &mut Cursor<'_>) -> Result<ReceivedBlock> {
    let last = r.read_uint(1)? == 1;
    let bits = if last { r.read_possibility(0x100)? } else { 0xff };
    let mut w = WireWriter::new(Position::PACKET_START)?;
    copy_bits(r, &mut w, bits)?;
    Ok(ReceivedBlock { last, bits: bits as i32, data: Some(finalize(&w)?) })
}
impl ReceiverSink<'_> {
    /// 0x7632D0.
    fn reassemble(&mut self) -> Result<()> {
        let mut index = self.blocks.begin;
        while index < self.blocks.end() {
            let block = self.blocks.get(index).expect("inside window").clone();
            let Some(data) = block.data else { return Ok(()) };
            if index == 0 {
                // 0x7631D0: 32-bit start index from the header bitstream.
                if !self.bound {
                    return Err(unbound());
                }
                let packet = Packet::new(data)?;
                let mut c = body_cursor(&packet)?;
                let start = c.read_uint(32)? as i32;
                if start < self.history.begin() {
                    return Err(Error::Protocol(format!(
                        "history start {start} precedes {}",
                        self.history.begin()
                    )));
                }
                self.history.retire_before(start)?;
            } else {
                // 0x7626F0.
                let packet = Packet::new(data)?;
                let mut c = body_cursor(&packet)?;
                copy_bits(&mut c, self.reassembly, block.bits as u32)?;
                if block.last {
                    if !self.bound {
                        return Err(unbound());
                    }
                    let packet = Packet::new(finalize(self.reassembly)?)?;
                    let end = self.history.end();
                    self.history.set_end(end.checked_add(1).ok_or(aa_orders::Error::IndexExhausted)?)?;
                    self.history.put(end, Some(packet))?;
                    *self.reassembly = WireWriter::new(Position::PACKET_START)?;
                }
            }
            // 0x762A50(index + 1).
            self.blocks.retire_to(index + 1, ReceivedBlock::empty());
            index = self.blocks.begin;
        }
        Ok(())
    }
}
impl BlockSink for ReceiverSink<'_> {
    /// 0x7637D0.
    fn receive(&mut self, r: &mut Cursor<'_>, index: i32) -> Result<()> {
        let block = if index == 0 {
            // 0x7610A0: 32-bit byte count, then 0x411FA0(256) per byte.
            let length = r.read_uint(32)?;
            if (length as u64) * 8 > r.remaining_bits() as u64 {
                return Err(Error::Protocol(format!("byte buffer of {length} bytes exceeds the datagram")));
            }
            let mut bytes = Vec::with_capacity(length as usize);
            for _ in 0..length {
                bytes.push(r.read_possibility(0x100)? as u8);
            }
            ReceivedBlock { last: true, bits: 0, data: Some(bytes) }
        } else {
            read_bit_block(r)?
        };
        if self.blocks.begin <= index {
            if self.blocks.end() <= index {
                self.blocks.extend_to(index + 1, ReceivedBlock::empty());
            }
            *self.blocks.get_mut(index).expect("extended") = block;
            self.reassemble()?;
        }
        Ok(())
    }
    /// 0x7638B0.
    fn skip(&mut self, r: &mut Cursor<'_>, _index: i32) -> Result<()> {
        read_bit_block(r).map(drop)
    }
}
