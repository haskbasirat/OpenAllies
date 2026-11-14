//! Original `4DDC50(&out, object)` on an actor: the actor's 16-byte group
//! record (vector +188, count +18C) whose first word is the object's GTID
//! (+14), searched in vector order; without one, the default record (words
//! 0, bytes +C = 0 and +D = 1). Only bytes +0..+D are written.

/// One 16-byte record (`+0` GTID, `+4`, `+8` words, `+C`, `+D` bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupRecord {
    pub gtid: u32,
    pub word4: u32,
    pub word8: u32,
    pub byte_c: u8,
    pub byte_d: u8,
}
impl GroupRecord {
    /// The record `4DDC50` writes when no entry matches.
    pub const DEFAULT: Self = Self {
        gtid: 0,
        word4: 0,
        word8: 0,
        byte_c: 0,
        byte_d: 1,
    };
}

/// Complete `4DDC50`: the first record with `gtid` (the object's +14), or
/// [`GroupRecord::DEFAULT`].
pub fn group_record(records: &[GroupRecord], gtid: u32) -> GroupRecord {
    records
        .iter()
        .find(|r| r.gtid == gtid)
        .copied()
        .unwrap_or(GroupRecord::DEFAULT)
}
