//! Original element Mind memory of targets its move could not reach: the
//! list at Mind+1B0 (head) / +1B4 (tail) of 0x18-byte records (+0 target
//! GTID, +4 world time, +8 other GTID, +C/+10 point, +14 next).
//!
//! * `530E00(object)`: the object's GTID (0 for null) is in the list,
//!   searched from the head ([`remembered`]).
//! * `532840(object, other)`: unless remembered, `530ED0` pushes a record
//!   at the head with the object's GTID (0 for null), the world time
//!   (`[B36534]+E0`), the other's GTID (0 for null) and the other's +20/+24
//!   or the null point `[B279C0]` ([`remember`]).
use serde::{Deserialize, Serialize};

/// One record of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreachableRecord {
    pub gtid: u32,
    pub time: u32,
    pub other: u32,
    pub point: [u32; 2],
}

/// `530E00`: `gtid` (the object's +14, 0 for a null object) is remembered.
pub fn remembered(list: &[UnreachableRecord], gtid: u32) -> bool {
    list.iter().any(|r| r.gtid == gtid)
}

/// `532840(object, other)`. `list` is head first; `other` is the other
/// object's GTID and +20/+24 position.
pub fn remember(
    list: &mut Vec<UnreachableRecord>,
    gtid: u32,
    now: u32,
    other: Option<(u32, [u32; 2])>,
    null_point: [u32; 2],
) {
    if remembered(list, gtid) {
        return;
    }
    let (other, point) = other.unwrap_or((0, null_point));
    list.insert(
        0,
        UnreachableRecord {
            gtid,
            time: now,
            other,
            point,
        },
    );
}
