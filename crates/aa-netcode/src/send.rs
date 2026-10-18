//! Send scheduling: TGC_AdminConn::GetWantedPacketType 0x74BDE0, asked
//! repeatedly by the NetNode send selector 0x763EE0 inside SendPackets
//! 0x764FA0 for every connection until it answers 0.
//!
//! Inputs are the AdminNode/AdminConn/NetConn fields it reads; the backlog
//! test (0x74BCB0, v14 caves 0x9EFF40/0x9EFF40) is
//! [`crate::packets::AdminConn::backlogged`]. Time comparisons are x87
//! in-register statements `fld f32 now; fsub f32 t; fcomp f32 c` whose
//! result depends on the precision control of the network thread, an
//! explicit input.
//!
//! Per-pass counts 8 (0x74BED7, backlogged) and 9 (0x74BF16, `<= 8` while
//! the peer is not yet a session member) are pacing RULES of the original
//! protocol, kept: removing them leaves a backlogged connection without a
//! termination condition inside one pass (the original has no other one),
//! so they are not treated as engine caps. The NetNode bandwidth token
//! buckets of 0x764FA0/0x763E30 are not ported (no application bandwidth
//! cap; see research/phase8-net/README.txt section 10).
//!
//! Evidence: research/phase8-net/native-send-gate.json.
use crate::{Error, Result};
use aa_orders::net::transport::X87Precision;
use serde::{Deserialize, Serialize};

/// [0xA033BC] 0.25 s: Connect / Reconnect / leaving-backup resend interval.
pub const INTERVAL_SLOW_BITS: u32 = 0x3e80_0000;
/// [0xA8C0BC] 0.1 s: steady state interval.
pub const INTERVAL_BITS: u32 = 0x3dcc_cccd;
/// [0xA8C0AC] 0.02 s: backlogged / joining interval after the per-pass count.
pub const INTERVAL_FAST_BITS: u32 = 0x3ca3_d70a;
/// [0x9FBDB8] 0.5: seconds of backup mode per rank step before a
/// lower-ranked backup server offers itself (0x74BD80).
pub const RANK_DELAY_BITS: u32 = 0x3f00_0000;
/// 0x74BED7: datagrams per pass while backlogged (`< 8`).
pub const BACKLOG_BURST: u32 = 8;
/// 0x74BF16: datagrams per pass to a peer that is not a session member yet (`<= 8`).
pub const JOIN_BURST: u32 = 8;

/// AdminNode fields read by 0x74BDE0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateNode {
    /// +0x2CC connecting.
    pub connecting: bool,
    /// AdminNode == +0x2C4 / == +0x2C8.
    pub we_are_host: bool,
    pub we_are_server: bool,
    /// +0x2CD leaving and +0x2C1 leave immediately.
    pub leaving: bool,
    pub leave_now: bool,
    /// +0x70 rank.
    pub rank: i32,
    /// +0x158 admin clock present (with conn+8: 0x74B600 session member).
    pub admin_clock: bool,
    /// +0x2D8 time backup mode was last set (binary32 bits).
    pub backup_since: u32,
}

/// AdminConn / NetConn fields read by 0x74BDE0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateConn {
    /// This conn is AdminNode+0x2C4 / +0x2C8.
    pub is_host: bool,
    pub is_server: bool,
    /// +0x89 disconnecting, +0x88 connected.
    pub disconnecting: bool,
    pub connected: bool,
    /// +0x70 rank.
    pub rank: i32,
    /// +8 session player present.
    pub player: bool,
    /// NetConn+8 datagrams already sent this pass.
    pub sent_this_pass: u32,
    /// NetConn+0x90 last send time (binary32 bits).
    pub last_send: u32,
}

/// What the gate also logs (AdminLayer.cpp lines 0x1087 / 0x109C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wanted {
    pub packet_type: u32,
    pub log_line: Option<u32>,
}

/// Decide `pred(a - b)` for binary32 a, b subtracted on the x87 stack at
/// `precision` (fld f32 / fsub f32, value kept in the register). PC24 and
/// PC53 results are reproduced exactly; at PC64 an inexact binary64
/// difference is bracketed by its neighbours and only a boundary case is
/// unsupported.
fn x87_difference(a: u32, b: u32, precision: X87Precision, pred: impl Fn(f64) -> bool) -> Result<bool> {
    let (a, b) = (f32::from_bits(a), f32::from_bits(b));
    match precision {
        X87Precision::Single => Ok(pred(f64::from(a - b))),
        X87Precision::Double => Ok(pred(f64::from(a) - f64::from(b))),
        X87Precision::Extended => {
            let (x, y) = (f64::from(a), -f64::from(b));
            let s = x + y;
            let bb = s - x;
            let err = (x - (s - bb)) + (y - bb);
            if err == 0.0 || !s.is_finite() {
                return Ok(pred(s));
            }
            let other = if err > 0.0 { next_up(s) } else { next_down(s) };
            let (p, q) = (pred(s), pred(other));
            if p != q {
                return Err(Error::Unsupported("x87 64-bit difference on a comparison boundary"));
            }
            Ok(p)
        }
    }
}

/// `(a - b) > c`: fld/fsub/fcomp with test ah,0x41 (C0|C3 or unordered =
/// not greater).
pub fn x87_elapsed_exceeds(a: u32, b: u32, c: u32, precision: X87Precision) -> Result<bool> {
    let c = f64::from(f32::from_bits(c));
    x87_difference(a, b, precision, |d| d > c)
}
fn next_up(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    if x == 0.0 {
        return f64::from_bits(1);
    }
    let b = x.to_bits();
    f64::from_bits(if x > 0.0 { b + 1 } else { b - 1 })
}
fn next_down(x: f64) -> f64 {
    -next_up(-x)
}

/// 0x74BD80: `f32(conn rank) * 0.5 < now - AdminNode+0x2D8` when the conn
/// ranks below this node (fild + fstp f32 + fld, fmul 0.5, fcompp, test
/// ah,5 / jp: unordered = false).
pub fn rank_delay_elapsed(node: &GateNode, conn: &GateConn, now: u32, precision: X87Precision) -> Result<bool> {
    if conn.rank >= node.rank {
        return Ok(false);
    }
    let rank = conn.rank as f32;
    // rank * 0.5 is exact at every precision (binary32 exponent range).
    let product = f64::from(rank) * 0.5;
    x87_difference(now, node.backup_since, precision, |d| product < d)
}

/// 0x74BDE0. `backlogged` is the result of the backlog test (only consulted
/// on the host for a connected, non-leaving conn).
pub fn wanted_packet_type(
    node: &GateNode,
    conn: &GateConn,
    now: u32,
    backlogged: &mut dyn FnMut() -> Result<bool>,
    precision: X87Precision,
) -> Result<Wanted> {
    let after = |c: u32| x87_elapsed_exceeds(now, conn.last_send, c, precision);
    let pick = |t: u32, ok: bool| Wanted { packet_type: if ok { t } else { 0 }, log_line: None };
    if node.connecting {
        if !conn.is_host {
            return Ok(pick(0, false));
        }
        return Ok(pick(2, after(INTERVAL_SLOW_BITS)?));
    }
    if node.we_are_host {
        if conn.disconnecting {
            return Ok(pick(11, after(INTERVAL_BITS)?));
        }
        if !conn.connected {
            return Ok(pick(0, false));
        }
        if node.leaving {
            return Ok(pick(8, after(INTERVAL_BITS)?));
        }
        if backlogged()? {
            if conn.sent_this_pass < BACKLOG_BURST {
                return Ok(pick(5, true));
            }
            return Ok(pick(5, after(INTERVAL_FAST_BITS)?));
        }
        // 0x74B600: session member.
        if !(node.admin_clock && conn.player) {
            if conn.sent_this_pass <= JOIN_BURST {
                return Ok(pick(5, true));
            }
            return Ok(pick(5, after(INTERVAL_FAST_BITS)?));
        }
        return Ok(pick(5, after(INTERVAL_BITS)?));
    }
    if node.we_are_server {
        if conn.rank < node.rank {
            if node.leaving {
                return Ok(Wanted { packet_type: 0, log_line: Some(0x1087) });
            }
            if !rank_delay_elapsed(node, conn, now, precision)? {
                return Ok(pick(0, false));
            }
            return Ok(pick(3, after(INTERVAL_SLOW_BITS)?));
        }
        if !conn.connected {
            return Ok(pick(0, false));
        }
        if node.leaving {
            let mut w = pick(9, after(INTERVAL_SLOW_BITS)?);
            w.log_line = Some(0x109c);
            return Ok(w);
        }
        return Ok(pick(7, after(INTERVAL_BITS)?));
    }
    if conn.is_server {
        if node.leaving {
            if node.leave_now {
                return Ok(pick(10, true));
            }
            return Ok(pick(10, after(INTERVAL_BITS)?));
        }
        if conn.is_host {
            return Ok(pick(4, after(INTERVAL_BITS)?));
        }
        return Ok(pick(6, after(INTERVAL_BITS)?));
    }
    Ok(pick(0, false))
}

/// One NetConn as the send selector 0x763EE0 sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectorConn {
    /// NetConn+0x94 bandwidth used and +0x98 allowance. The original skips a
    /// connection over its allowance; the bandwidth token buckets are not
    /// ported (no application bandwidth cap), so callers pass
    /// `within_allowance: true` unless they model them.
    pub within_allowance: bool,
    /// GetWantedPacketType 0x74BDE0 of its AdminConn (+0xA0), or `None`
    /// without one (treated as 0, and the -last_served key as 0 too).
    pub wanted: Option<u32>,
    /// NetConn+4: the NetNode send counter (+0x9C) when it was last served.
    pub last_served: i32,
}

/// NetNode send selector 0x763EE0: the connection with the greatest
/// (wanted type, -last served) pair, strictly greater wins (the first in
/// list order on a tie), `None` when that type is 0. The comparison is the
/// native signed 32-bit one, including `-last_served` wrapping for INT_MIN.
pub fn select_connection(conns: &[SelectorConn]) -> Option<usize> {
    let (mut best_type, mut best_key, mut best) = (0i32, 0i32, None);
    for (i, c) in conns.iter().enumerate() {
        if !c.within_allowance {
            continue;
        }
        let (t, key) = match c.wanted {
            Some(t) => (t as i32, c.last_served.wrapping_neg()),
            None => (0, 0),
        };
        let greater = if t != best_type { t > best_type } else { key > best_key };
        if (t != best_type || key != best_key) && greater {
            best_type = t;
            best_key = key;
            best = Some(i);
        }
    }
    if best_type == 0 { None } else { best }
}
