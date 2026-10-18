//! Services shared by the regiment pack (`527090`) and unpack (`526E60`)
//! Thinks: the organization's unit (organization component +44) and the
//! player error message.
use super::stack_action::ActionWorld;
use crate::{Arena, MindId, Result};

/// `40C230` message kind used by the pack/unpack errors.
pub const ERROR_MESSAGE_KIND: u32 = 0x11;
/// Wide literal `A33918`.
pub const PACK_ERROR_KEY: &str = "tai_error_pack|s";
/// Wide literal `A338F0`.
pub const UNPACK_ERROR_KEY: &str = "tai_error_unpack|s";
pub use super::stack_action::{PACK_ACTION, UNPACK_ACTION};

/// Compact stack action description (`4D9E20(ordinal)` retagged with vtable
/// A3CC38; `51E800` also stores four binary32 words at +18..+24 copied
/// through the x87 stack and sets flags 4 and 8). Kept for
/// `uai_thinks::uai_sleep`; the regiment Thinks deliver
/// [`super::stack_action::ActionCommand`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StackAction {
    pub ordinal: u32,
    pub words18_24: Option<[u32; 4]>,
}

/// Services in native order.
pub trait UnitOrderWorld: ActionWorld {
    /// Organization component +44 (the unit), `None` when null.
    fn component_unit(&mut self, mind: MindId) -> Result<Option<u32>>;
    /// `4D1080` on the unit.
    fn unit_4d1080(&mut self, unit: u32, arena: &mut Arena) -> Result<bool>;
    /// Organization virtual +24: its name string.
    fn organization_name(&mut self, mind: MindId, arena: &mut Arena) -> Result<String>;
    /// `4B9F40` (`KKC_TextDataManager::GetDisplayString` on `[B36544]`,
    /// default `B43B48`) for a key built by `401B00`.
    fn display_string(&mut self, key: &str, arena: &mut Arena) -> Result<String>;
    /// Organization virtual +108 (owner).
    fn organization_owner(&mut self, mind: MindId, arena: &mut Arena) -> Result<u32>;
    /// `40C230(B399B0, kind, organization, owner, 0, format, argument)`.
    fn post_message(
        &mut self,
        mind: MindId,
        kind: u32,
        owner: u32,
        format: &str,
        argument: &str,
        arena: &mut Arena,
    ) -> Result<()>;
}

/// The error message block of `527090`/`526E60`: the display string for
/// `key` formatted with the organization name, posted to its owner.
pub fn report_error(
    mind: MindId,
    key: &str,
    arena: &mut Arena,
    world: &mut impl UnitOrderWorld,
) -> Result<()> {
    let name = world.organization_name(mind, arena)?;
    let format = world.display_string(key, arena)?;
    let owner = world.organization_owner(mind, arena)?;
    world.post_message(mind, ERROR_MESSAGE_KIND, owner, &format, &name, arena)
}
