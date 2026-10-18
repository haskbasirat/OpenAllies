//! Original SAI command data (`KKC_SAICommandData`, vtable A9DC10: ctor
//! `7AEEA0`/`7AEE50`, dtor `7ADC60`) and its send `KKC_SAICommandData::Send`
//! (`7AF220`, Source/Sai/SAIPlayer.cpp), the only way SAI decisions leave the
//! SAI. Goals build command data (e.g. `7C8AF0`) and queue it on their player
//! (`7B4FA0` -> `7B4C50`, which marks each listed actor's SAI record `+20`
//! pending); `KKC_SAI::Think` (`7A0080`) sends due entries after the fibers
//! return (`sai::manager`).
//!
//! `7AF220` emits ordinary player order requests, in this order:
//! - an attached faction-ability order (`+14`): `60DE10`, request 51
//!   `KKC_FactionUseAbilityOrderRequest`, and nothing else;
//! - otherwise, for the live (registry, not deleting `+8` bit 0, not `+8`
//!   bit 5) actors of the list: request 331 `SelectionSetOrderRequest`
//!   (`7AD6D0`) for the first and 12 `SelectionAddOrderRequest` (`436AD0`)
//!   for each further one; then, for several actors, request 84
//!   `KKC_TellSelectedCommandOrderRequest` (`60C8A0` order object, virtual
//!   `+4` = `61EEF0` -> `61E960`) carrying a new command of ordinal 0x25
//!   (`4D9E20`) with byte parameter 1 (`4D9B10`), then request 84 with the
//!   command; for one actor only the command. The order object takes the
//!   command over (`60C8A0` nulls `+10`), so the data no longer holds it.
//!
//! [`SaiRequest::descriptor`] gives the aa-orders descriptor ids; the
//! requests carry SAI-side identities (native object handles) which the
//! host's order layer encodes (aa-orders owns the wire encodings).
use super::base_priority::Obj;
use crate::Result;

/// The `4D9E20` command a goal built (0x40-byte `KKC_Command`, vtable
/// A28450), as far as `7AF220` and the order request read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaiCommand {
    /// The native command object identity; `None` for the group command
    /// `7AF220` builds itself.
    pub handle: Option<Obj>,
    /// Ordinal into the ActorCommand registry `[[B36530]+298]` (`4D9E20`).
    pub ordinal: u32,
    /// `+C` flags: bit 0 the command targets the actor GTID `+10`, 0x40 the
    /// registry holds a definition for the ordinal, 0x200 byte `+30` set.
    pub flags: u32,
    /// `+10` target actor GTID.
    pub target: u32,
    /// `+30` byte parameter.
    pub byte30: u8,
}

/// Ordinal of the command `7AF220` sends first to a group (`4D9E20(0x25)`).
pub const GROUP_COMMAND_ORDINAL: u32 = 0x25;

/// `KKC_SAICommandData` fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaiCommandData {
    /// `+4` list object `+8` chain: actor GTIDs, head first.
    pub actors: Vec<u32>,
    /// `+8` the (AI) player the orders are issued for.
    pub player: Option<Obj>,
    /// `+10` the command (refcounted).
    pub command: Option<SaiCommand>,
    /// `+14` an attached faction-ability order object.
    pub ability: Option<Obj>,
}

/// One order request emitted by `7AF220`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaiRequest {
    /// `60DE10`: request 51 `KKC_FactionUseAbilityOrderRequest`.
    FactionUseAbility { player: Obj, ability: Obj },
    /// `7AD6D0`: request 331 `SelectionSetOrderRequest`.
    SelectionSet { player: Obj, actor: Obj },
    /// `436AD0`: request 12 `SelectionAddOrderRequest`.
    SelectionAdd { player: Obj, actor: Obj },
    /// `61EEF0` -> `61E960`: request 84 `KKC_TellSelectedCommandOrderRequest`.
    TellSelectedCommand { player: Obj, command: SaiCommand },
}

impl SaiRequest {
    /// The native order descriptor id (research/phase8-orders inventory).
    pub fn descriptor(&self) -> u32 {
        match self {
            SaiRequest::FactionUseAbility { .. } => 51,
            SaiRequest::SelectionSet { .. } => 331,
            SaiRequest::SelectionAdd { .. } => 12,
            SaiRequest::TellSelectedCommand { .. } => 84,
        }
    }
}

/// Services of `7AF220`.
pub trait SendWorld {
    /// `[B364C0]` GTID registry.
    fn resolve(&mut self, gtid: u32) -> Result<Option<Obj>>;
    /// Field: object `+8` flags.
    fn flags(&mut self, obj: Obj) -> Result<u32>;
    /// Field: player `+C` byte (an AI player).
    fn player_is_ai(&mut self, player: Obj) -> Result<bool>;
    /// Field: object `+154` SAI actor record.
    fn sai_record(&mut self, obj: Obj) -> Result<Option<Obj>>;
    /// Field write: SAI actor record `+20` byte (command pending).
    fn set_record_pending(&mut self, record: Obj, pending: bool) -> Result<()>;
    /// Field: `[[B36530]+298][ordinal]` is non-null (`4D9E20` flag 0x40).
    fn command_defined(&mut self, ordinal: u32) -> Result<bool>;
    /// `404ED0` assertion log "AI sending a command for a non-AI player"
    /// (SAIPlayer.cpp line 0xFF5); execution continues.
    fn log_non_ai_player(&mut self) -> Result<()>;
}

/// Complete `7AF220`. Returns the requests in emission order; `data` is
/// updated as natively (released commands become `None`).
pub fn send(data: &mut SaiCommandData, w: &mut impl SendWorld) -> Result<Vec<SaiRequest>> {
    let mut out = Vec::new();
    let player = match data.player {
        Some(p) if w.player_is_ai(p)? => p,
        _ => {
            w.log_non_ai_player()?;
            data.command = None;
            return Ok(out);
        }
    };
    if let Some(ability) = data.ability {
        out.push(SaiRequest::FactionUseAbility { player, ability });
        data.ability = None;
        return Ok(out);
    }
    let Some(command) = data.command else {
        return Ok(out);
    };
    if command.flags & 1 != 0 {
        let stale = match w.resolve(command.target)? {
            None => true,
            Some(t) => w.flags(t)? & 1 != 0,
        };
        if stale {
            data.command = None;
            return Ok(out);
        }
    }
    let mut count = 0_u32;
    for &gtid in &data.actors {
        let Some(actor) = w.resolve(gtid)? else {
            continue;
        };
        let flags = w.flags(actor)?;
        if flags & 1 != 0 || (flags >> 5) & 1 != 0 {
            continue;
        }
        count += 1;
        out.push(if count == 1 {
            SaiRequest::SelectionSet { player, actor }
        } else {
            SaiRequest::SelectionAdd { player, actor }
        });
    }
    if count > 1 {
        let defined = w.command_defined(GROUP_COMMAND_ORDINAL)?;
        let group = SaiCommand {
            handle: None,
            ordinal: GROUP_COMMAND_ORDINAL,
            flags: if defined { 0x40 } else { 0 } | 0x200,
            target: 0,
            byte30: 1,
        };
        out.push(SaiRequest::TellSelectedCommand {
            player,
            command: group,
        });
        out.push(SaiRequest::TellSelectedCommand { player, command });
    } else if count == 1 {
        out.push(SaiRequest::TellSelectedCommand { player, command });
    }
    // 60C8A0 moves the command into the order object (nulling +10); with
    // no live actor 7AF479 releases it.
    data.command = None;
    for &gtid in &data.actors {
        if let Some(actor) = w.resolve(gtid)?
            && let Some(record) = w.sai_record(actor)?
        {
            w.set_record_pending(record, false)?;
        }
    }
    Ok(out)
}
