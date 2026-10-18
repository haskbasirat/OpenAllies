//! KKC_SetGameSpeedOrder decree process 0x75D810 (request descriptor 54,
//! decree descriptor 73; payload one 32-bit word = the binary32 speed at
//! order+4, schema p25 in aa-orders), bound to
//! [`aa_runtime::simulation::Timeline::set_game_speed`] (0x76F370).
//!
//! Evidence: research/phase8-net/native-speed.json (every authority
//! combination run through the original 0x75D810 / 0x76AD90 / 0x74B3B0 /
//! 0x74B900); 0x76F370 itself is proven in aa-runtime (phase8-tick).
use crate::{Error, Result};
use aa_orders::payload::Field;
use aa_runtime::simulation::{Driver, PowPath, SpeedChange, SpeedContext, SpeedSettings};
use aa_simulation::FloatBits;
use serde::{Deserialize, Serialize};

/// Descriptor ordinals (crates/aa-orders/src/native_registry.rs).
pub const SET_GAME_SPEED_REQUEST: usize = 54;
pub const SET_GAME_SPEED_DECREE: usize = 73;

/// What 0x75D810 and 0x76AD90 read besides the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeedAuthority {
    /// [0xB36550] session exists.
    pub session: bool,
    /// AdminNode+0x284 ordering player exists (0x74B3B0).
    pub ordering_player: bool,
    /// Ordering player +4 (0x8569A0): its client exists.
    pub ordering_client: bool,
    /// 0x74B900: that client is AdminNode+0x2C4 (the host).
    pub client_is_host: bool,
}

/// Speed word of the decree payload (order+4).
pub fn decree_speed(fields: &[Field]) -> Result<FloatBits> {
    match fields {
        [Field::Uint { width: 32, value }] => Ok(FloatBits(*value)),
        _ => Err(Error::Protocol(format!("SetGameSpeed decree payload {fields:?}"))),
    }
}

/// 0x75D810 authority test: session, ordering player, then 0x76AD90 (no
/// world in cutscene mode [0xB36534]+0x275, the player's client exists and
/// is the host). Returns the speed when 0x76F370 is called.
pub fn set_game_speed_gate(
    fields: &[Field],
    authority: &SpeedAuthority,
    world_cutscene: Option<bool>,
) -> Result<Option<FloatBits>> {
    let speed = decree_speed(fields)?;
    if !authority.session || !authority.ordering_player {
        return Ok(None);
    }
    if world_cutscene == Some(true) {
        return Ok(None);
    }
    if !authority.ordering_client || !authority.client_is_host {
        return Ok(None);
    }
    Ok(Some(speed))
}

/// 0x75D810 with KKC_Sim::SetGameSpeed 0x76F370 on the driver's timeline
/// ([0xB36554]) at the driver's x87 precision (simulation thread, PC53 by
/// the lead's policy). `None` when the decree is ignored. The caller owns
/// the UI message of [`SpeedChange::changed`] and writes
/// [`SpeedChange::sync_log`] to the synchronizer.
pub fn process_set_game_speed(
    driver: &mut Driver,
    fields: &[Field],
    authority: &SpeedAuthority,
    settings: &SpeedSettings,
    context: &SpeedContext,
    path: PowPath,
) -> Result<Option<SpeedChange>> {
    let Some(speed) = set_game_speed_gate(fields, authority, context.world_cutscene)? else {
        return Ok(None);
    };
    let precision = driver.precision;
    Ok(Some(driver.timeline.set_game_speed(speed, settings, context, path, precision)?))
}
