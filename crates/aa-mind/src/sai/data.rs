//! The native TGI data-node getter contract used by the SAIData loaders
//! (Source/Data/SAIData.cpp) and shared helpers.
//!
//! Native getters (thiscall on a DataNode, wide field name argument):
//! `5B5130` fixed (the field's +18 virtual, an exact binary32 or `fild` of an
//! int field, returned in ST0), `5B4ED0` int, `5B5380` bool, `5B54D0` string
//! (the field's CString), `5B5620` localized, `5B4220` named child count,
//! `42BE10` the i-th named child through the `{node, index, name, count}`
//! iterator, `5B4410(name, required)` child section. A missing field throws
//! "Node doesn't contain field.", a missing required section throws
//! "SubNode not found."; `410C40` formats a data error and throws. All of
//! them end the load: here they are `Err`.
use crate::{
    Error, Result,
    x87::{n, num},
};
use aa_numeric::{Binary, Precision};

/// Opaque handle of a data node.
pub type NodeId = u32;

/// Getter services in native call order.
pub trait DataNode {
    /// `5B5130`.
    fn fixed(&mut self, node: NodeId, name: &str) -> Result<Binary>;
    /// `5B4ED0`.
    fn int(&mut self, node: NodeId, name: &str) -> Result<i32>;
    /// `5B5380`.
    fn boolean(&mut self, node: NodeId, name: &str) -> Result<bool>;
    /// `5B54D0`.
    fn string(&mut self, node: NodeId, name: &str) -> Result<String>;
    /// `5B5620`.
    fn localized(&mut self, node: NodeId, name: &str) -> Result<String>;
    /// `5B4220`.
    fn count(&mut self, node: NodeId, name: &str) -> Result<i32>;
    /// `42BE10` on the iterator `{node, index, name, count}`.
    fn item(&mut self, node: NodeId, name: &str, index: i32, count: i32) -> Result<NodeId>;
    /// `5B4410(name, required)`.
    fn section(&mut self, node: NodeId, name: &str, required: bool) -> Result<Option<NodeId>>;
    /// `410C40(format, ...)`: the native data error (throws). Returns the error
    /// to propagate; `format` is the native format string.
    fn data_error(&mut self, format: &str, args: &[String]) -> Error;
}

/// `5B4410(name, 1)`: a required section (absent is an error).
pub fn required(data: &mut impl DataNode, node: NodeId, name: &str) -> Result<NodeId> {
    data.section(node, name, true)?
        .ok_or_else(|| Error::World(format!("SubNode not found. ({name})")))
}

/// The native named-list loop `for (i = 0; i < count && i >= 0; i++)` with
/// the `5B4220` count taken once.
pub fn items(data: &mut impl DataNode, node: NodeId, name: &str) -> Result<Vec<(i32, i32)>> {
    let count = data.count(node, name)?;
    Ok((0..count.max(0)).map(|i| (i, count)).collect())
}

/// `fstp dword` of a getter's ST0.
pub fn f32_of(value: &Binary) -> Result<u32> {
    num(value.store_f32())
}

/// `_ftol(floor(binary64(PC(v * 1000.0f + 0.5f))))` (`96D5E0` floor, `96CC4C`
/// truncation): seconds to milliseconds as the loaders store them.
pub fn milliseconds(value: &Binary, p: Precision) -> Result<i32> {
    let scaled = num(value.mul(&n(0x447a_0000)?, p))?;
    let rounded = num(scaled.add(&n(0x3f00_0000)?, p))?;
    // fstp qword: round to binary64.
    let double = num(rounded.add(&Binary::from_i32(0), Precision::Double53))?;
    let truncated = num(double.truncate_i32())?;
    let floor = if double.compare(&Binary::from_i32(truncated)) == std::cmp::Ordering::Less {
        truncated
            .checked_sub(1)
            .ok_or_else(|| Error::World("millisecond value outside i32".into()))?
    } else {
        truncated
    };
    Ok(floor)
}
