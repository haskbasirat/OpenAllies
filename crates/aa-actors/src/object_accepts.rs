//! Object virtual +174 `0x4D1EE0`: does this object accept `actor` (the
//! aircraft landing/rearm test). It passes the actor's virtual +108 result to
//! the object's own virtual +134.
//!
//! Every vtable in the image whose slot +174 is `4D1EE0` resolves the two
//! virtuals as follows (read from AA.exe):
//!
//! | vtable   | +108 (owner)          | +134 (owner test)                    |
//! |----------|-----------------------|--------------------------------------|
//! | `A273F0` | `4D1E80`: word +E8    | `4D1E90`: own +E8 nonzero and equal  |
//! | `A29490` | `4D1E80`: word +E8    | `4D1E90`: own +E8 nonzero and equal  |
//! | `A299D8` | `4D1E80`: word +E8    | `4E5240`: the object itself is equal |
//!
//! `A299D8` is the Kingdom class (its +A8 is true). An actor whose class
//! resolves +108 to `4C9FA0` (for example vtable `A28A00`) yields owner 0.
//! Proven over every class pair and owner combination by
//! research/phase8-combat/native_object_accepts.py (real vtables, no seams).
use serde::{Deserialize, Serialize};

/// The object's virtual +134.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnerTest {
    /// `0x4D1E90`: the object's word +E8 is nonzero and equals the argument.
    OwnWord,
    /// `0x4E5240`: the object itself equals the argument.
    Itself,
}

/// The actor's virtual +108.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnerGetter {
    /// `0x4D1E80`: word +E8.
    Word,
    /// `0x4C9FA0`: zero.
    Zero,
}

/// Virtual +134 of the classes that carry `4D1EE0` at +174.
pub fn owner_test(vtable: u32) -> Option<OwnerTest> {
    match vtable {
        0x00a2_73f0 | 0x00a2_9490 => Some(OwnerTest::OwnWord),
        0x00a2_99d8 => Some(OwnerTest::Itself),
        _ => None,
    }
}

/// Virtual +108 of the classes the oracle covers.
pub fn owner_getter(vtable: u32) -> Option<OwnerGetter> {
    match vtable {
        0x00a2_73f0 | 0x00a2_9490 | 0x00a2_99d8 => Some(OwnerGetter::Word),
        0x00a2_8a00 => Some(OwnerGetter::Zero),
        _ => None,
    }
}

/// `0x4D1E80` / `0x4C9FA0`: the actor's owner word.
pub fn owner(getter: OwnerGetter, word_e8: u32) -> u32 {
    match getter {
        OwnerGetter::Word => word_e8,
        OwnerGetter::Zero => 0,
    }
}

/// `0x4D1EE0`: `object` (handle, test class and its word +E8) accepts an
/// actor whose virtual +108 returned `actor_owner`.
pub fn object_accepts(test: OwnerTest, object: u32, object_e8: u32, actor_owner: u32) -> bool {
    match test {
        OwnerTest::OwnWord => object_e8 != 0 && object_e8 == actor_owner,
        OwnerTest::Itself => object == actor_owner,
    }
}
