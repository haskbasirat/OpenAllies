//! Native registry and a complete actor-movement wire payload.
//!
//! Registry IDs are the original CRT registration sequence, not alphabetical
//! names. Several native names are duplicated. Unknown payloads are errors, not
//! skipped fields: the stream has no independently established per-order length.
use crate::{Cursor, Error, Registry, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Descriptor {
    pub id: u16,
    pub name: &'static str,
    pub handler_va: u32,
}
include!("native_registry.rs");

pub const MOVE_REQUEST_ID: usize = 47;
pub const MOVE_DECREE_ID: usize = 66;
pub const MOVE_ACTOR_COMMAND_TYPE: u32 = 5;

pub fn registry() -> Result<Registry> {
    Registry::new(DESCRIPTORS.iter().map(|d| d.name.to_owned()).collect())
}

/// Exact 0x60F760 fields. The selector is a native CString serialized as an
/// unsigned16 code-unit count and one complete bit-buffer read. Preserve invalid
/// Unicode and embedded NULs; do not reinterpret it as a numeric ActorId.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveOrder {
    pub selector_utf16: Vec<u16>,
    /// Native +8, +C, +10. Process uses only the first two words as XY.
    /// Raw IEEE bits retain signed zero, NaNs and unused third-word data.
    pub words: [u32; 3],
}
impl MoveOrder {
    pub fn decode(cursor: &mut Cursor<'_>) -> Result<Self> {
        let count = cursor.read_uint(16)? as usize;
        let raw = cursor.read_raw_bits(count * 16)?;
        let mut selector_utf16 = Vec::new();
        selector_utf16
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        selector_utf16.extend(
            raw.chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]])),
        );
        let words = [
            cursor.read_uint(32)?,
            cursor.read_uint(32)?,
            cursor.read_uint(32)?,
        ];
        Ok(Self {
            selector_utf16,
            words,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownPayload {
    MoveRequest(MoveOrder),
    MoveDecree(MoveOrder),
}
/// Requests and authoritative decrees share the decoder but not their role.
/// Never execute a decoded request as an accepted decree.
pub fn decode_payload(descriptor: usize, cursor: &mut Cursor<'_>) -> Result<KnownPayload> {
    match descriptor {
        MOVE_REQUEST_ID => MoveOrder::decode(cursor).map(KnownPayload::MoveRequest),
        MOVE_DECREE_ID => MoveOrder::decode(cursor).map(KnownPayload::MoveDecree),
        other => Err(Error::UnsupportedNativeDescriptor(other)),
    }
}

/// Semantic fields constructed by 0x61E830 -> 0x4CA760. No actor pointer, native
/// vtable or command-definition pointer crosses this safe interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveCommand {
    pub destination_bits: [u32; 2],
    pub auxiliary_bits: [u32; 2],
    pub word_28: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveOutcome {
    ProcessingDisabled,
    ActorNotFound,
    CommandUnavailable,
    Delivered,
}
/// Explicit unported leaves of the real native movement process. Actor lookup
/// includes native alias/GTID rules; command eligibility and component dispatch
/// must be supplied by their own verified implementations.
pub trait MoveContext {
    type Actor;
    fn can_process(&mut self) -> bool;
    fn resolve_actor(&mut self, selector_utf16: &[u16]) -> Option<Self::Actor>;
    fn allows_command(&mut self, actor: &Self::Actor, command_type: u32) -> bool;
    /// 0x61E830 ignores the generic dispatcher return value. This is a delivery
    /// boundary, not a promise that an actor accepted or completed movement.
    fn deliver(&mut self, actor: &Self::Actor, command_type: u32, command: MoveCommand);
}
/// Real guard ordering from 0x61E830. Call only for an accepted movement decree,
/// at the external verified scheduling point. This does not choose a path,
/// advance the world, bypass authorization, or retry rejected commands.
pub fn process_move(order: &MoveOrder, context: &mut impl MoveContext) -> MoveOutcome {
    if !context.can_process() {
        return MoveOutcome::ProcessingDisabled;
    }
    let Some(actor) = context.resolve_actor(&order.selector_utf16) else {
        return MoveOutcome::ActorNotFound;
    };
    if !context.allows_command(&actor, MOVE_ACTOR_COMMAND_TYPE) {
        return MoveOutcome::CommandUnavailable;
    }
    context.deliver(
        &actor,
        MOVE_ACTOR_COMMAND_TYPE,
        MoveCommand {
            destination_bits: [order.words[0], order.words[1]],
            auxiliary_bits: [0, 0],
            word_28: 0,
        },
    );
    MoveOutcome::Delivered
}
