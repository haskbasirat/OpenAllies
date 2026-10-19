//! KKC_GActorCommand sync dump 0x4D9A40 on synchronizer A ([B3655C]).
//!
//! `4D9A40(label, command)` (cdecl): `77DC50(label, 0)` scope marker, then for
//! a non-null command `77D930("_command->m_value", command+28, 0)` (raw
//! binary32 bits) and `77DA70("_command->m_bits", command+0C, 0)` (int).
//! Each logger returns before any OS read unless the synchronizer is enabled
//! (+0) and its suspend count (+4) is <= 0; only then does it ask
//! `GetCurrentThreadId() == [B39828]` and the SAI flags `[B36528]+5C/+5D`
//! (the caller-supplied `gate` of [`SyncBinding`]). The hash/entry effects are
//! `aa_orders::net::sync::Synchronizer`'s native-backed loggers.
//!
//! Callers and labels: [`COMMAND_LABEL`] "_command" (A2960C) from 4ED360,
//! 4DFE46 and 57C76E; [`CMD_LABEL`] "_cmd" (A346CC) from every component
//! command handler (vt+24), e.g. KKC_RecruitComponent 583A70 at 583AA9 after
//! its own 77DC50 marker. Proven by
//! research/phase8-orders-input/recruit/native-command-sync-4d9a40.json.
use crate::{live_clocked::SyncBinding, mind_world::PlainCommand};
use aa_orders::net::sync::Synchronizer;

/// A2960C, pushed by 4ED375 (TellSelected delivery), 4DFE46 and 57C76E.
pub const COMMAND_LABEL: &str = "_command";
/// A346CC, pushed by the component command handlers (583AA9 Recruit, 52C35B
/// Mind, 566810 Construct, 56E750 Denizen, 571D10 Flare, 576670
/// MobileDeployment, 5849A0 Research, 5882D0 Settlement, 589DB0 Structure).
pub const CMD_LABEL: &str = "_cmd";
/// A28370 / A2834C.
pub const VALUE_LABEL: &str = "_command->m_value";
pub const BITS_LABEL: &str = "_command->m_bits";

/// One reached logger on the SAME synchronizer A owner. Native order: the
/// enabled/suspend guards, then (only when both pass) the OS/application gate.
fn emit(sync: &mut SyncBinding<'_>, effect: impl FnOnce(&mut Synchronizer, bool)) -> Result<(), String> {
    fn log(owner: &mut Synchronizer, gate: &mut dyn FnMut() -> Result<bool, String>,
        effect: impl FnOnce(&mut Synchronizer, bool)) -> Result<(), String> {
        let open = if owner.enabled && owner.suspend < 1 { gate()? } else { false };
        effect(owner, open);
        Ok(())
    }
    match sync {
        SyncBinding::Direct(direct) => log(direct.owner, direct.gate, effect),
        SyncBinding::Shared { owner, gate } => {
            let mut borrowed = owner
                .try_borrow_mut()
                .map_err(|e| format!("4D9A40 same SyncA [B3655C] loan: {e}"))?;
            log(&mut borrowed, *gate, effect)
        }
    }
}

/// Exact 0x4D9A40. `command` is the native command pointer (None = NULL);
/// only +28 (`scalar_bits`) and +0C (`flags`) are read, after the marker.
/// A gate error stops at that logger; earlier entries remain committed.
pub fn sync_command_4d9a40(
    sync: &mut SyncBinding<'_>,
    label: &str,
    command: Option<&PlainCommand>,
) -> Result<(), String> {
    emit(sync, |log, open| log.log_marker(open, label))?;
    if let Some(command) = command {
        let value = command.scalar_bits;
        emit(sync, |log, open| log.log_float_bits(open, VALUE_LABEL, value))?;
        let bits = command.flags;
        emit(sync, |log, open| log.log_int(open, BITS_LABEL, bits))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "player_order_command_sync_tests.rs"]
mod tests;
