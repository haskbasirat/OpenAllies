//! Stack actions (commands) the regiment Thinks build on their stack, deliver
//! with `4D9B60(receiver)` and destroy with `4CA2D0`. Field layout as in
//! aa-triggers/aa-navigation (`4D9E20` constructor): +8 definition, +C
//! flags, +10 target GTID, +18..+28 words, +30 byte.
use super::{RegimentWorld, x87_copy};
use crate::{Arena, Result};

/// Ordinals passed to `4D9E20`.
pub const REPAIR_ACTION: u32 = 3;
pub const MOVE_ACTION: u32 = 5;
pub const UNPACK_ACTION: u32 = 0xb;
pub const PACK_ACTION: u32 = 0xc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActionCommand {
    pub ordinal: u32,
    /// +8: the action registry entry for the ordinal.
    pub definition: u32,
    /// +C.
    pub flags: u32,
    /// +10 (target GTID).
    pub word10: u32,
    /// +18, +1C, +20, +24, +28.
    pub words18_28: [u32; 5],
    /// +30.
    pub byte30: u8,
}
impl ActionCommand {
    /// `4D9E20(ordinal)` with the registry entry: fields cleared, flag 0x40
    /// when the definition is non-null. The Thinks then set vtable A3CC38.
    pub fn new(ordinal: u32, definition: u32) -> Self {
        Self {
            ordinal,
            definition,
            flags: if definition != 0 { 0x40 } else { 0 },
            word10: 0,
            words18_28: [0; 5],
            byte30: 0,
        }
    }
    /// `51E800(&from, &to)`: unpack action, both points copied through the
    /// x87 stack (+18/+1C from, +20/+24 to), flags 4 then 8.
    pub fn unpack(definition: u32, points: [u32; 4]) -> Self {
        let mut action = Self::new(UNPACK_ACTION, definition);
        action.flags |= 4 | 8;
        action.words18_28[..4].copy_from_slice(&points.map(x87_copy));
        action
    }
    /// `51E870(target, flag)`: repair action with `4D9BF0(target)` (flag 1,
    /// +10 the target's GTID) and `4D9B10(flag)` (flag 0x200, +30).
    pub fn repair(definition: u32, target_gtid: u32, flag: u8) -> Self {
        let mut action = Self::new(REPAIR_ACTION, definition);
        action.flags |= 1;
        action.word10 = target_gtid;
        action.flags |= 0x200;
        action.byte30 = flag;
        action
    }
    /// `4CA760(location, word28, words20_24)`: move action, flags 4, 8 and
    /// 0x10 with the words stored unchanged.
    pub fn move_to(definition: u32, location: [u32; 2], word28: u32, words20_24: [u32; 2]) -> Self {
        let mut action = Self::new(MOVE_ACTION, definition);
        action.flags |= 4 | 8 | 0x10;
        action.words18_28 = [
            location[0],
            location[1],
            words20_24[0],
            words20_24[1],
            word28,
        ];
        action
    }
}

/// Action services in native order.
pub trait ActionWorld: RegimentWorld {
    /// The action registry `[B36530]+298` entry for `ordinal` (read by
    /// `4D9E20`).
    fn action_definition(&mut self, ordinal: u32) -> Result<u32>;
    /// `4D9B60(receiver)` on the action: the delivery result.
    fn issue_action(
        &mut self,
        receiver: u32,
        action: &ActionCommand,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// `4CA2D0` on the action (releases a +34 reference set by processing).
    fn release_action(&mut self, action: &ActionCommand, arena: &mut Arena) -> Result<()>;
}
