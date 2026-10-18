//! Original-instruction-backed packet cursor, decree dispatch and client history.
//! The native registry and movement payload are in [`native`]; gameplay scheduling
//! and actor component implementation remain explicit external boundaries.
#![forbid(unsafe_code)]

pub mod native;
pub mod commands;
pub mod payload;
pub mod flow;
pub mod net;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("packet length, padding or reserved-header position is invalid")]
    InvalidPacket,
    #[error("cursor position is outside its byte/bit boundary")]
    InvalidPosition,
    #[error("native scalar read width {0} is outside 0..=32")]
    ScalarWidth(u8),
    #[error("packet has {remaining} bits left but {requested} were requested")]
    Truncated { requested: usize, remaining: usize },
    #[error("native finite-registry scalar supports 1..=65536 possibilities")]
    RegistrySize,
    #[error("decoded registry index {index} is outside {count} descriptors")]
    RegistryIndex { index: u32, count: usize },
    #[error("opaque handler {index} failed at {position:?}: {message}")]
    Handler {
        index: usize,
        position: Position,
        message: String,
    },
    #[error("opaque handler {index} made no forward bit progress")]
    NoProgress { index: usize },
    #[error("history allocation failed")]
    Allocation,
    #[error("history capacity/range cannot be represented in the native signed-index domain")]
    HistoryRange,
    #[error("history index {index} precedes retained begin {begin}")]
    BeforeWindow { index: i32, begin: i32 },
    #[error("history retirement must be monotonic")]
    BackwardRetirement,
    #[error("history index exhausted")]
    IndexExhausted,
    #[error("native descriptor {0} has no implemented payload decoder")]
    UnsupportedNativeDescriptor(usize),
    #[error("generic command flags exceed the native eleven-bit field")]
    CommandFlags,
    #[error("parameter count {0} is outside the native nonnegative signed domain")]
    ParameterCount(u32),
    #[error("parameter list needs {requested} entries, above the caller budget {budget}")]
    ParameterBudget { requested: usize, budget: usize },
    #[error("selected-command processing requires an established current player")]
    MissingCurrentPlayer,
    #[error("native writer source has fewer bits than requested")]
    WriteSource,
    #[error("payload field list does not match the native schema: expected {expected}")]
    SchemaField { expected: &'static str },
    #[error("schema context lookup failed: {0}")]
    SchemaContext(&'static str),
    #[error("native re-serialization of descriptor {0} is not ported")]
    UnsupportedReserialization(usize),
    #[error("x87 extended-precision result cannot be reproduced exactly here")]
    X87Unsupported,
    #[error("list of {requested} entries exceeds what the remaining {remaining} bits can hold")]
    ListLength { requested: u64, remaining: usize },
    /// KKC_GActorCommand::Process 0x4D9B60 would loop forever: the
    /// redirect / container walk revisited an actor.
    #[error("actor command walk revisits an actor (the original 0x4D9B60 loops forever)")]
    CommandRoutingCycle,
}
pub type Result<T> = std::result::Result<T, Error>;

/// Native stores bit offsets 0..8; offset8 is intentionally not normalized to
/// next-byte/zero because 0x40D5D0 preserves this representation at boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub byte: usize,
    pub bit: u8,
}
impl Position {
    pub const PACKET_START: Self = Self { byte: 0, bit: 3 };
    pub fn absolute(self) -> Result<usize> {
        if self.bit > 8 {
            return Err(Error::InvalidPosition);
        }
        self.byte
            .checked_mul(8)
            .and_then(|n| n.checked_add(self.bit as usize))
            .ok_or(Error::InvalidPosition)
    }
}

/// Safe read-only counterpart of the native iterator fields +4/+8/+C.
/// read_uint models the original unusual byte-block/fragment packing, not an
/// assumed conventional contiguous LSB-first integer bitstream.
pub struct Cursor<'a> {
    bytes: &'a [u8],
    position: Position,
    end: usize,
}
impl<'a> Cursor<'a> {
    /// Raw serializer boundary. Packet users should use for_packet instead.
    pub fn with_bounds(bytes: &'a [u8], position: Position, end: usize) -> Result<Self> {
        let length = bytes.len().checked_mul(8).ok_or(Error::InvalidPosition)?;
        if end > length || position.absolute()? > end {
            return Err(Error::InvalidPosition);
        }
        Ok(Self {
            bytes,
            position,
            end,
        })
    }
    pub fn for_packet(bytes: &'a [u8], start: Option<Position>) -> Result<Self> {
        let end = packet_end(bytes)?;
        let position = start.unwrap_or(Position::PACKET_START);
        if position.absolute()? < 3 {
            return Err(Error::InvalidPacket);
        }
        Self::with_bounds(bytes, position, end)
    }
    pub fn position(&self) -> Position {
        self.position
    }
    pub fn remaining_bits(&self) -> usize {
        self.end - self.position.absolute().expect("validated cursor")
    }
    pub fn relative_bits(&self) -> Result<usize> {
        self.position
            .absolute()?
            .checked_sub(3)
            .ok_or(Error::InvalidPosition)
    }
    pub fn read_uint(&mut self, width: u8) -> Result<u32> {
        if width > 32 {
            return Err(Error::ScalarWidth(width));
        }
        let requested = width as usize;
        if requested > self.remaining_bits() {
            return Err(Error::Truncated {
                requested,
                remaining: self.remaining_bits(),
            });
        }
        if width == 0 {
            return Ok(0);
        }
        let mask = |n: usize| -> u8 { ((1u16 << n) - 1) as u8 };
        let position = self.position;
        let available = 8 - position.bit as usize;
        // Five-byte scratch also covers native's harmless zero-byte OR at
        // output+4 for an aligned32-bit read. We never write past a u32.
        let mut out = [0u8; 5];
        if requested <= available {
            out[0] = (self.bytes[position.byte] >> position.bit) & mask(requested);
            self.position.bit += width;
        } else {
            let mut remaining = requested - available;
            let whole = remaining / 8;
            out[whole] =
                ((u16::from(self.bytes[position.byte]) >> position.bit) as u8) & mask(available);
            out[..whole].copy_from_slice(&self.bytes[position.byte + 1..position.byte + 1 + whole]);
            self.position.byte += whole;
            remaining -= whole * 8;
            if remaining == 0 {
                self.position.bit = 8;
            } else {
                self.position.byte += 1;
                self.position.bit = remaining as u8;
                let value = self.bytes[self.position.byte];
                if available + remaining <= 8 {
                    out[whole] |= ((u16::from(value & mask(remaining))) << available) as u8;
                } else {
                    let low = 8 - available;
                    out[whole] |= ((u16::from(value & mask(low))) << available) as u8;
                    out[whole + 1] |= (value >> low) & mask(remaining - low);
                }
            }
        }
        Ok(u32::from_le_bytes(
            out[..4].try_into().expect("four scalar bytes"),
        ))
    }
    /// One original 0x40D5D0 buffer read. Splitting this into scalar reads changes
    /// its cross-byte fragment packing. Used by native length-prefixed UTF-16.
    pub fn read_raw_bits(&mut self, requested: usize) -> Result<Vec<u8>> {
        if requested > self.remaining_bits() {
            return Err(Error::Truncated {
                requested,
                remaining: self.remaining_bits(),
            });
        }
        let bytes = requested.div_ceil(8);
        let scratch = bytes.checked_add(1).ok_or(Error::Allocation)?;
        let mut out = Vec::new();
        out.try_reserve_exact(scratch)
            .map_err(|_| Error::Allocation)?;
        out.resize(scratch, 0);
        if requested != 0 {
            let mask = |n: usize| -> u8 { ((1u16 << n) - 1) as u8 };
            let available = 8 - self.position.bit as usize;
            if requested <= available {
                out[0] = (self.bytes[self.position.byte] >> self.position.bit) & mask(requested);
                self.position.bit += requested as u8;
            } else {
                let remaining = requested - available;
                let whole = remaining / 8;
                out[whole] = ((u16::from(self.bytes[self.position.byte]) >> self.position.bit)
                    as u8)
                    & mask(available);
                out[..whole].copy_from_slice(
                    &self.bytes[self.position.byte + 1..self.position.byte + 1 + whole],
                );
                self.position.byte += whole;
                let tail = remaining % 8;
                if tail == 0 {
                    self.position.bit = 8;
                } else {
                    self.position.byte += 1;
                    self.position.bit = tail as u8;
                    let value = self.bytes[self.position.byte];
                    if available + tail <= 8 {
                        out[whole] |= (u16::from(value & mask(tail)) << available) as u8;
                    } else {
                        let low = 8 - available;
                        out[whole] |= (u16::from(value & mask(low)) << available) as u8;
                        out[whole + 1] |= (value >> low) & mask(tail - low);
                    }
                }
            }
        }
        out.truncate(bytes);
        Ok(out)
    }
    /// 0x411FA0 +0x40C7C0, with invalid index rejection instead of assertion.
    pub fn read_possibility(&mut self, count: usize) -> Result<u32> {
        let index = self.read_uint(registry_bits(count)?)?;
        if index as usize >= count {
            return Err(Error::RegistryIndex { index, count });
        }
        Ok(index)
    }
}
fn packet_end(bytes: &[u8]) -> Result<usize> {
    let first = *bytes.first().ok_or(Error::InvalidPacket)?;
    let end = bytes
        .len()
        .checked_mul(8)
        .and_then(|n| n.checked_sub((first & 7) as usize))
        .ok_or(Error::InvalidPacket)?;
    if end < 3 {
        return Err(Error::InvalidPacket);
    }
    Ok(end)
}
pub fn registry_bits(count: usize) -> Result<u8> {
    if !(1..=65536).contains(&count) {
        return Err(Error::RegistrySize);
    }
    Ok((usize::BITS - (count - 1).leading_zeros()) as u8)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet(Arc<[u8]>);
impl Packet {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        packet_end(&bytes)?;
        Ok(Self(bytes.into()))
    }
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gate {
    pub session: bool,
    pub admin: bool,
}
impl Gate {
    pub fn stops(self) -> bool {
        self.session && self.admin
    }
}
///The native dispatcher reads Session24 first, then Node2E5 only when the
///first byte is nonzero. Actual live owners lend these reads; the legacy Gate
///remains a laboratory adapter. Getter failures retain the decoded descriptor.
pub trait GateSource {
    fn session_byte(&mut self)->Result<u8>;
    fn admin_byte(&mut self)->Result<u8>;
    fn store_session_byte(&mut self,value:u8)->Result<()>;
}
impl GateSource for Gate {
    fn session_byte(&mut self)->Result<u8>{Ok(self.session as u8)}
    fn admin_byte(&mut self)->Result<u8>{Ok(self.admin as u8)}
    fn store_session_byte(&mut self,value:u8)->Result<()>{self.session=value!=0;Ok(())}
}

pub struct Registry {
    names: Vec<String>,
}
impl Registry {
    /// Caller must provide verified native registration order before claiming
    /// game-decree identity. These names do not infer any payload schema.
    pub fn new(names: Vec<String>) -> Result<Self> {
        registry_bits(names.len())?;
        Ok(Self { names })
    }
    pub fn names(&self) -> &[String] {
        &self.names
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    DiagnosticBits {
        bits: usize,
    },
    Decoded {
        index: usize,
        position: Position,
    },
    Executed {
        index: usize,
        start: Position,
        end: Position,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dispatch {
    pub events: Vec<Event>,
    pub position: Position,
    pub executed: usize,
    pub stopped_before_execute: Option<usize>,
}

/// Ordered control flow of0x74D710. Handler is the explicit unported766CE0
/// gameplay leaf. Only reads advance the cursor; no parallel dispatch occurs.
/// Handler side effects are not rolled back if a later read/handler fails.
pub fn dispatch<F,G:GateSource+?Sized>(
    packet: Option<&Packet>,
    start: Option<Position>,
    registry: &Registry,
    diagnostics: bool,
    gate: &mut G,
    handler: F,
) -> Result<Dispatch>
where
    F: FnMut(usize, &mut Cursor<'_>, &mut G) -> std::result::Result<(), String>,
{
    dispatch_observed(packet, start, registry, diagnostics, gate, handler, |_| {})
}

/// [`dispatch`] that also reports every event the moment it happens, so a
/// caller can interleave the native sync-log entries ("i.GetBitSize()",
/// 0x77DA70 on synchronizer A) with the handlers' own entries in native order.
pub fn dispatch_observed<F, O,G:GateSource+?Sized>(
    packet: Option<&Packet>,
    start: Option<Position>,
    registry: &Registry,
    diagnostics: bool,
    gate: &mut G,
    handler: F,
    mut observe: O,
) -> Result<Dispatch>
where
    F: FnMut(usize, &mut Cursor<'_>, &mut G) -> std::result::Result<(), String>,
    O: FnMut(&Event),
{
    dispatch_observed_fallible(packet,start,registry,diagnostics,gate,handler,|event| { observe(event); Ok(()) })
}

/// Same native dispatcher, with an explicit fallible required diagnostic
/// service. A refused service stops before the next native read/effect; prior
/// effects remain. The infallible API above delegates to this same algorithm.
pub fn dispatch_observed_fallible<F, O,G:GateSource+?Sized>(
    packet: Option<&Packet>,
    start: Option<Position>,
    registry: &Registry,
    diagnostics: bool,
    gate: &mut G,
    mut handler: F,
    mut observe: O,
) -> Result<Dispatch>
where
    F: FnMut(usize, &mut Cursor<'_>, &mut G) -> std::result::Result<(), String>,
    O: FnMut(&Event) -> Result<()>,
{
    let mut result = Dispatch {
        events: Vec::new(),
        position: start.unwrap_or(Position::PACKET_START),
        executed: 0,
        stopped_before_execute: None,
    };
    let Some(packet) = packet else {
        return Ok(result);
    };
    let mut push = |events: &mut Vec<Event>, e: Event| -> Result<()> {
        observe(&e)?;
        events.push(e);
        Ok(())
    };
    let mut reader = Cursor::for_packet(packet.bytes(), start)?;
    while reader.remaining_bits() != 0 {
        let before = reader.position().absolute()?;
        if diagnostics {
            push(&mut result.events, Event::DiagnosticBits { bits: reader.relative_bits()? })?;
        }
        let index = reader.read_possibility(registry.names.len())? as usize;
        push(&mut result.events, Event::Decoded { index, position: reader.position() })?;
        // Original tests flags AFTER decoding the next descriptor, not before.
        if gate.session_byte()?!=0 && gate.admin_byte()?!=0 {
            result.stopped_before_execute = Some(index);
            break;
        }
        let start = reader.position();
        handler(index, &mut reader, gate).map_err(|message| Error::Handler {
            index,
            position: reader.position(),
            message,
        })?;
        push(&mut result.events, Event::Executed { index, start, end: reader.position() })?;
        result.executed += 1;
        if diagnostics && registry.names[index] != "TGC_PlayerJoinOrderDecree" {
            push(&mut result.events, Event::DiagnosticBits { bits: reader.relative_bits()? })?;
        }
        if reader.position().absolute()? <= before {
            return Err(Error::NoProgress { index });
        }
    }
    result.position = reader.position();
    Ok(result)
}

/// Native TGC_WindowedArray<Packet> finite monotonic-index domain. Null slots
/// are distinct from a valid header-only empty packet. No fixed history cap is
/// introduced; growth follows the original capacity doubling.
pub struct History {
    begin: i32,
    end: i32,
    slots: Vec<Option<Packet>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistorySnapshot {
    pub begin: i32,
    pub end: i32,
    pub capacity: usize,
    pub slots: Vec<Option<Vec<u8>>>,
    pub entries: Vec<Option<Vec<u8>>>,
}
fn slot(index: i32, capacity: usize) -> usize {
    (u32::from_ne_bytes(index.to_ne_bytes()).wrapping_add(0x8000_0000) as usize) % capacity
}
fn null_slots(capacity: usize) -> Result<Vec<Option<Packet>>> {
    if capacity == 0 || capacity > i32::MAX as usize {
        return Err(Error::HistoryRange);
    }
    let mut slots = Vec::new();
    slots
        .try_reserve_exact(capacity)
        .map_err(|_| Error::Allocation)?;
    slots.resize_with(capacity, || None);
    Ok(slots)
}
impl History {
    pub fn new(begin: i32, capacity: usize) -> Result<Self> {
        Ok(Self {
            begin,
            end: begin,
            slots: null_slots(capacity)?,
        })
    }
    pub fn begin(&self) -> i32 {
        self.begin
    }
    pub fn end(&self) -> i32 {
        self.end
    }
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }
    /// 0x74EC30. Shrinking end intentionally retains out-of-window slot refs;
    /// later extension clears them, matching the original packet assignments.
    pub fn set_end(&mut self, end: i32) -> Result<()> {
        let needed = i64::from(end) - i64::from(self.begin);
        if !(0..=i64::from(i32::MAX)).contains(&needed) {
            return Err(Error::HistoryRange);
        }
        if needed as usize > self.capacity() {
            let mut capacity = self.capacity();
            while capacity < needed as usize {
                capacity = capacity
                    .checked_mul(2)
                    .filter(|&n| n <= i32::MAX as usize)
                    .ok_or(Error::HistoryRange)?;
            }
            let mut grown = null_slots(capacity)?;
            for index in self.begin..self.end {
                grown[slot(index, capacity)] = self.slots[slot(index, self.capacity())].clone();
            }
            self.slots = grown;
        }
        for index in self.end..end {
            let i = slot(index, self.capacity());
            self.slots[i] = None;
        }
        self.end = end;
        Ok(())
    }
    /// 0x750880 valid monotonic-retirement domain. Retiring beyond end leaves
    /// an empty window at the requested index without fabricating packets.
    pub fn retire_before(&mut self, begin: i32) -> Result<()> {
        if begin < self.begin {
            return Err(Error::BackwardRetirement);
        }
        for index in self.begin..begin.min(self.end) {
            let i = slot(index, self.capacity());
            self.slots[i] = None;
        }
        self.begin = begin;
        self.end = self.end.max(begin);
        Ok(())
    }
    pub fn ensure_index(&mut self, index: i32) -> Result<()> {
        if index < self.begin {
            return Err(Error::BeforeWindow {
                index,
                begin: self.begin,
            });
        }
        if index >= self.end {
            self.set_end(index.checked_add(1).ok_or(Error::IndexExhausted)?)?;
        }
        Ok(())
    }
    pub fn put(&mut self, index: i32, packet: Option<Packet>) -> Result<()> {
        self.ensure_index(index)?;
        let i = slot(index, self.capacity());
        self.slots[i] = packet;
        Ok(())
    }
    pub fn get(&self, index: i32) -> Result<Option<&Packet>> {
        if index < self.begin {
            return Err(Error::BeforeWindow {
                index,
                begin: self.begin,
            });
        }
        if index >= self.end {
            return Err(Error::HistoryRange);
        }
        Ok(self.slots[slot(index, self.capacity())].as_ref())
    }
    pub fn snapshot(&self) -> HistorySnapshot {
        let bytes = |p: &Option<Packet>| p.as_ref().map(|p| p.bytes().to_vec());
        HistorySnapshot {
            begin: self.begin,
            end: self.end,
            capacity: self.capacity(),
            slots: self.slots.iter().map(bytes).collect(),
            entries: (self.begin..self.end)
                .map(|i| bytes(&self.slots[slot(i, self.capacity())]))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientState {
    pub next_index: i32,
    /// +248/-1 selects normal start; otherwise +248/+24C are a raw resume pair.
    pub resume_byte: i32,
    pub resume_bit: i32,
    /// Native +250 copied into+284 before decoding. Meaning remains opaque.
    pub field_250: u32,
    pub field_284: u32,
}
impl ClientState {
    pub fn new(next_index: i32) -> Self {
        Self {
            next_index,
            resume_byte: -1,
            resume_bit: -1,
            field_250: 0,
            field_284: 0,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientReceipt {
    pub initial_index: i32,
    pub completed_index: i32,
    pub dispatch: Dispatch,
}

/// Full successful finite-index boundary of0x74FD10. Decode is explicit so a
/// runtime can connect dispatch() with its own verified handlers. The callback
/// can mutate the current index/history: native rereads it after dispatch.
/// Consuming retains stored history, resets resume/scratch fields, then advances
/// exactly one index from the reread value, including when dispatch's gate stops.
/// Callback failure is nontransactional and MUST NOT be retried automatically.
pub fn consume_next<F>(
    history: &mut History,
    state: &mut ClientState,
    decode: F,
) -> Result<ClientReceipt>
where
    F: FnOnce(
        Option<&Packet>,
        Option<Position>,
        &mut ClientState,
        &mut History,
    ) -> Result<Dispatch>,
{
    state.field_284 = state.field_250;
    let initial_index = state.next_index;
    history.ensure_index(initial_index)?;
    let packet = history.get(initial_index)?.cloned(); // Native ref-counted local ownership.
    let start = if state.resume_byte == -1 {
        None
    } else {
        let byte = usize::try_from(state.resume_byte).map_err(|_| Error::InvalidPosition)?;
        let bit = u8::try_from(state.resume_bit).map_err(|_| Error::InvalidPosition)?;
        let position = Position { byte, bit };
        position.absolute()?;
        Some(position)
    };
    let dispatch = decode(packet.as_ref(), start, state, history)?;
    let completed_index = state.next_index;
    history.ensure_index(completed_index)?;
    let next = completed_index
        .checked_add(1)
        .ok_or(Error::IndexExhausted)?;
    state.resume_byte = -1;
    state.resume_bit = -1;
    state.field_250 = 0;
    state.field_284 = 0;
    // Native diagnostic between reset and increment is observational only here.
    state.next_index = next;
    Ok(ClientReceipt {
        initial_index,
        completed_index,
        dispatch,
    })
}
