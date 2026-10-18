//! TGC_NetConn sequencing and datagram header (original 0x764DA0, 0x764CA0,
//! 0x764BB0, 0x764600, 0x764550, 0x764AF0).
//!
//! Wire order of every datagram, after the 3-bit trailing-pad header
//! (finalizer 0x74D0A0, see [`finalize`]):
//! 1. 1 bit session-ids-present (always 1; receiver drops 0, log line 0x391);
//! 2. 32 bits sender session id (NetNode+0xB0); receiver drops -1 (0x377);
//! 3. 32 bits target session id (NetConn+0, the peer id last seen; -1 before
//!    any); receiver accepts -1 or its own id, else drops (0x37E), and stores
//!    the sender id into NetConn+0;
//! 4. 32 bits sequence = ++NetConn+0x30 (first datagram 0);
//! 5. 32 bits ack = NetConn+0x34, highest sequence received, -1 for none;
//! 6. if ack != -1: one received-flag bit for every sequence in
//!    `[max(ack-32,0) (0 when ack-32 < 1), ack)`, oldest first;
//! 7. the admin-layer packet (4-bit type then body, 0x7521F0), appended with
//!    no length or alignment.
//!
//! Receive: ack == -1 ("Accepting first packet") marks the sequence received
//! with RTT sample 0 and dispatches, without the reorder check or ack bits.
//! Otherwise the RTT sample is `now - sent_time[ack]` when ack is inside the
//! sent-time window (else 3.0), clamped to 3.0 when greater; then a sequence
//! below `highest - 4` is rejected (log 0x3FD, nothing else happens); else the
//! sequence is marked received (raising `highest` updates the smoothed RTT
//! `0.75*rtt + 0.25*sample` and the liveness time NetConn+0x2C), each ack bit
//! is reported as (sequence, bit==1), then (ack, true), then the admin payload
//! is dispatched with the received sequence. In-window duplicates are
//! dispatched again (the admin channels are idempotent).
//!
//! Native windows are growable ring arrays; here they are growable vectors.
//! The received-flag window is never trimmed natively (one entry per received
//! sequence for the whole session); it is kept as a bit vector.
use crate::commands::WireWriter;
use crate::{Cursor, Error, Result};
use serde::{Deserialize, Serialize};

/// 0xA32690: RTT sample used when the acked packet is unknown or older.
pub const RTT_SAMPLE_UNKNOWN: f32 = 3.0;
/// 0x764AFF/0x764B16: sent-time history keeps sequences >= latest-200.
pub const SENT_TIME_RETENTION: i32 = 200;
/// 0x7646DE: sequences below highest-4 are rejected.
pub const REORDER_WINDOW: i32 = 4;
/// 0x76471F/0x764CFF: ack bits cover at most 32 earlier sequences.
pub const ACK_BITS: i32 = 32;

/// x87 precision control in effect for the float statements (PC24 is the
/// game's default per the lead; it stays an explicit input).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum X87Precision {
    Single,
    Double,
    Extended,
}

/// `round32(round_p(a - b))` for binary32 operands under precision `p`.
pub fn x87_sub_store(a: f32, b: f32, p: X87Precision) -> Result<f32> {
    match p {
        X87Precision::Single => Ok(a - b),
        X87Precision::Double => Ok((f64::from(a) - f64::from(b)) as f32),
        X87Precision::Extended => exact_f64(f64::from(a), -f64::from(b)).map(|s| s as f32),
    }
}

/// Exact f64 sum or an explicit error when the 64-bit-mantissa x87 result
/// cannot be reproduced through f64 without a double-rounding hazard.
fn exact_f64(x: f64, y: f64) -> Result<f64> {
    let s = x + y;
    let bb = s - x;
    let err = (x - (s - bb)) + (y - bb);
    if err == 0.0 || !s.is_finite() {
        Ok(s)
    } else {
        Err(Error::X87Unsupported)
    }
}

/// 0x7645C8..0x7645E0: `fstp f32(fld rtt * 0.75 + fld sample * 0.25)`.
pub fn smoothed_rtt(rtt: f32, sample: f32, p: X87Precision) -> Result<f32> {
    match p {
        X87Precision::Single => Ok(rtt * 0.75 + sample * 0.25),
        X87Precision::Double => {
            Ok((f64::from(rtt) * 0.75 + f64::from(sample) * 0.25) as f32)
        }
        X87Precision::Extended => {
            // Both products are exact in f64 (24-bit significands times 3/4, 1/4).
            exact_f64(f64::from(rtt) * 0.75, f64::from(sample) * 0.25).map(|s| s as f32)
        }
    }
}

/// Sender sent-time history (NetConn+0xC..+0x18, binary32 seconds).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SentTimes {
    pub begin: i32,
    pub times: Vec<f32>,
}
impl SentTimes {
    pub fn end(&self) -> i32 {
        self.begin + self.times.len() as i32
    }
    pub fn get(&self, sequence: i32) -> Option<f32> {
        (sequence >= self.begin && sequence < self.end())
            .then(|| self.times[(sequence - self.begin) as usize])
    }
    /// 0x750990 retire-to, called only with `begin > self.begin`: entries
    /// below `begin` are cleared and the window end becomes max(end, begin).
    fn retire(&mut self, begin: i32) {
        let drop = ((begin - self.begin) as usize).min(self.times.len());
        self.times.drain(..drop);
        self.begin = begin;
    }
    /// 0x764AF0 record (retire, extend with zero slots, store `now`).
    fn record(&mut self, sequence: i32, now: f32) {
        if self.begin < sequence - SENT_TIME_RETENTION {
            self.retire(sequence - SENT_TIME_RETENTION);
        }
        let floor = self.end() - SENT_TIME_RETENTION;
        if self.begin < floor {
            self.retire(floor);
        }
        if sequence < self.begin {
            return; // unreachable for monotonic sequences; native writes a stale slot
        }
        let index = (sequence - self.begin) as usize;
        if index >= self.times.len() {
            self.times.resize(index + 1, 0.0);
        }
        self.times[index] = now;
    }
}

/// Receiver received-flag window (NetConn+0x1C..+0x28).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceivedFlags {
    pub begin: i32,
    pub flags: Vec<bool>,
}
impl ReceivedFlags {
    pub fn end(&self) -> i32 {
        self.begin + self.flags.len() as i32
    }
    pub fn get(&self, sequence: i32) -> bool {
        sequence >= self.begin
            && sequence < self.end()
            && self.flags[(sequence - self.begin) as usize]
    }
    /// 0x7644B0 (first packet): window begins at `sequence`.
    fn restart(&mut self, sequence: i32) {
        if sequence >= self.begin {
            let drop = ((sequence - self.begin) as usize).min(self.flags.len());
            self.flags.drain(..drop);
        } else {
            // Native moves begin back over never-written (zero) slots of a fresh conn.
            let pad = (self.begin - sequence) as usize;
            self.flags.splice(0..0, std::iter::repeat_n(false, pad));
        }
        self.begin = sequence;
    }
    fn mark(&mut self, sequence: i32) {
        if sequence < self.begin {
            return; // rejected earlier by the reorder window for every reachable case
        }
        let index = (sequence - self.begin) as usize;
        if index >= self.flags.len() {
            self.flags.resize(index + 1, false);
        }
        self.flags[index] = true;
    }
}

/// The NetConn fields used by sequencing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sequencing {
    /// +0x00 peer session id (sender id of its latest valid datagram).
    pub peer_session: i32,
    pub sent: SentTimes,
    pub received: ReceivedFlags,
    /// +0x2C time the highest sequence last advanced (liveness).
    pub last_advance: f32,
    /// +0x30 last sent sequence (-1 initially).
    pub last_sent: i32,
    /// +0x34 highest received sequence (-1 initially).
    pub highest: i32,
    /// +0x38 smoothed RTT.
    pub rtt: f32,
    /// +0x08 datagrams sent in the current SendPackets pass.
    pub sent_this_pass: u32,
    /// +0x90 last send time (-1.0 initially).
    pub last_send: f32,
}
impl Default for Sequencing {
    /// Constructor 0x7647A0 values.
    fn default() -> Self {
        Self {
            peer_session: -1,
            sent: SentTimes::default(),
            received: ReceivedFlags::default(),
            last_advance: 0.0,
            last_sent: -1,
            highest: -1,
            rtt: 0.0,
            sent_this_pass: 0,
            last_send: -1.0,
        }
    }
}

/// What the receiver did with one datagram.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Received {
    NoSessionIds,
    InvalidSender,
    InvalidTarget,
    /// Sequence below highest-4 (log 0x3FD); no state other than nothing changed.
    OutOfOrder { sequence: i32 },
    /// Accepted. `acks` are the (sequence, acked) results reported in order to
    /// AdminConn 0x74B4F0; the admin payload follows at the cursor.
    Accepted { sequence: i32, first: bool, acks: Vec<(i32, bool)> },
}

impl Sequencing {
    /// 0x764DA0 + 0x764CA0 header writer. Returns the new sequence; the caller
    /// then writes the admin packet (0x7521F0) and must call [`Self::sent`].
    pub fn write_header(&mut self, w: &mut WireWriter, own_session: i32) -> Result<i32> {
        w.write_uint(1, 1)?;
        w.write_uint(own_session as u32, 32)?;
        w.write_uint(self.peer_session as u32, 32)?;
        let sequence = self.last_sent.wrapping_add(1);
        self.last_sent = sequence;
        w.write_uint(sequence as u32, 32)?;
        let ack = self.highest;
        w.write_uint(ack as u32, 32)?;
        if ack != -1 {
            let start = if ack.wrapping_sub(ACK_BITS) < 1 { 0 } else { ack - ACK_BITS };
            let mut i = start;
            while i < ack {
                w.write_uint(u32::from(self.received.get(i)), 1)?;
                i += 1;
            }
        }
        Ok(sequence)
    }
    /// 0x764AF0 after the admin packet is written: sent-time record, per-pass
    /// counter and last-send time. (The interval statistic 0x77C820 is telemetry.)
    pub fn sent(&mut self, sequence: i32, now: f32) {
        self.last_send = now;
        self.sent.record(sequence, now);
        self.sent_this_pass += 1;
    }
    /// 0x764BB0 + 0x764600 + 0x764550. `now` is NetNode+0x58.
    pub fn receive(
        &mut self,
        r: &mut Cursor<'_>,
        own_session: i32,
        now: f32,
        precision: X87Precision,
    ) -> Result<Received> {
        if r.read_uint(1)? != 1 {
            return Ok(Received::NoSessionIds);
        }
        let sender = r.read_uint(32)? as i32;
        let target = r.read_uint(32)? as i32;
        if sender == -1 {
            return Ok(Received::InvalidSender);
        }
        if target != -1 && target != own_session {
            return Ok(Received::InvalidTarget);
        }
        self.peer_session = sender;
        let sequence = r.read_uint(32)? as i32;
        let ack = r.read_uint(32)? as i32;
        if ack == -1 {
            self.mark(sequence, 0.0, now, precision)?;
            return Ok(Received::Accepted { sequence, first: true, acks: Vec::new() });
        }
        let mut sample = RTT_SAMPLE_UNKNOWN;
        if let Some(sent) = self.sent.get(ack) {
            let diff = match precision {
                X87Precision::Single => f64::from(now - sent),
                X87Precision::Double => f64::from(now) - f64::from(sent),
                X87Precision::Extended => exact_f64(f64::from(now), -f64::from(sent))?,
            };
            // fst then fcomp against 3.0 (C0|C3 keeps the stored value).
            if diff <= f64::from(RTT_SAMPLE_UNKNOWN) || diff.is_nan() {
                sample = diff as f32;
            }
        }
        if sequence < self.highest.wrapping_sub(REORDER_WINDOW) {
            return Ok(Received::OutOfOrder { sequence });
        }
        self.mark(sequence, sample, now, precision)?;
        let start = if ack.wrapping_sub(ACK_BITS) < 1 { 0 } else { ack - ACK_BITS };
        let mut acks = Vec::new();
        let mut i = start;
        while i < ack {
            acks.push((i, r.read_uint(1)? == 1));
            i += 1;
        }
        acks.push((ack, true));
        Ok(Received::Accepted { sequence, first: false, acks })
    }
    /// 0x764550.
    fn mark(&mut self, sequence: i32, sample: f32, now: f32, p: X87Precision) -> Result<()> {
        if self.highest == -1 {
            self.highest = sequence;
            self.received.restart(sequence);
            self.received.mark(sequence);
            self.last_advance = now;
            return Ok(());
        }
        self.received.mark(sequence);
        if self.highest < sequence {
            self.highest = sequence;
            self.rtt = smoothed_rtt(self.rtt, sample, p)?;
            self.last_advance = now;
        }
        Ok(())
    }
}

/// 0x74D0A0 finalizer: byte length `ceil(bits/8)` and the unused trailing bit
/// count `(8 - bits % 8) & 7` ORed into byte 0 bits 0..2, where `bits` is the
/// absolute stream position (including the 3 header bits).
pub fn finalize(w: &WireWriter) -> Result<Vec<u8>> {
    let bits = w.position().absolute()?;
    let length = bits.div_ceil(8);
    let mut out = w.bytes().get(..length).ok_or(Error::InvalidPacket)?.to_vec();
    out.resize(length.max(1), 0);
    out[0] |= ((8 - bits % 8) & 7) as u8;
    Ok(out)
}
