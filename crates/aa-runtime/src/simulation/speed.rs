//! GameSpeed: KKC_Sim::SetGameSpeed 0x76F370, the conversion 0x76B230 and
//! the MSVC CRT `_CIpow` 0x96D890 it calls with (2.0, speed).
//! Evidence: research/phase8-tick/native-tick-round2.json.
use super::{Error, Result, SyncLog, Timeline, num};
use aa_numeric::{Binary, Precision};
use aa_simulation::FloatBits;
use serde::{Deserialize, Serialize};

/// 0xA92F8C: binary32 1e-4, the "speed changed" threshold of 0x76F42D.
pub const SPEED_CHANGE_EPSILON_BITS: u32 = 0x38d1_b717;

/// Which `_CIpow` body runs (0x96D890..0x96D8C6). Both are recorded in the
/// oracle; the stored binary32 used by 0x76B230 is the same for every row. The CRT flag 0xB4B340 is set
/// by ___sse2_available_init 0x9773D4 on SSE2 CPUs except AMD family 0xF, and
/// the SSE2 body is taken only while MXCSR and the x87 control word mask all
/// exceptions (the Windows/D3D defaults). This is a property of the machine
/// that ran the original game, so it is an explicit input here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowPath {
    /// 0x977A10: binary64 result independent of x87 precision control.
    Sse2,
    /// 0x96D8ED: result rounded to min(precision, 53) significand bits.
    X87,
}

/// Value `_CIpow(2.0, y)` leaves in ST0, where it is known exactly: an
/// integral `y` gives the exact power of two on both bodies. Neither body is
/// correctly rounded for non-integral `y` (the oracle shows one-ulp binary64
/// deviations on the SSE2 body and on the x87 body at PC53/PC64), so those
/// values are Unsupported here; see [`pow2_store_f32`] for the stored result.
pub fn pow2_value(y_bits: u32, _path: PowPath, _precision: Precision) -> Result<Binary> {
    if is_integral(y_bits) {
        return Binary::exp2_binary32(y_bits, Precision::Extended64)?
            .ok_or(Error::Unsupported("integral 2^y is always exact"));
    }
    Err(Error::Unsupported(
        "_CIpow 0x977A10/0x96D8ED non-integral ST0 value is not ported bit-exactly",
    ))
}

fn is_integral(y_bits: u32) -> bool {
    let field = ((y_bits >> 23) & 0xff) as i32;
    if field == 0 {
        return y_bits & 0x7f_ffff == 0;
    }
    let shift = 150 - field; // fractional bits of the significand
    shift <= 0 || ((y_bits & 0x7f_ffff) | 0x80_0000) & ((1u32 << shift.min(24)) - 1) == 0
}

/// The binary32 that 0x76B24C stores from `_CIpow(2.0, y)`. Integral `y` is
/// exact. Otherwise the result is accepted only when the correctly rounded
/// binary64 and both of its neighbours store to the same binary32: every
/// native row (both bodies, PC24/53/64) deviates by at most one binary64 ulp
/// and the x87 body at PC24 rounds directly to 24 bits, so all of them store
/// that value. Closer to a binary32 boundary this is Unsupported.
pub fn pow2_store_f32(y_bits: u32, path: PowPath, precision: Precision) -> Result<FloatBits> {
    if let Ok(v) = pow2_value(y_bits, path, precision) {
        return Ok(FloatBits(v.store_f32()?));
    }
    let d = Binary::exp2_binary32(y_bits, Precision::Double53)?.ok_or(Error::Unsupported(
        "2^y bracket straddles a rounding boundary",
    ))?;
    let stored = d.store_f32()?;
    let ulp = ulp53(&d)?;
    for n in [
        d.sub(&ulp, Precision::Double53)?,
        d.add(&ulp, Precision::Double53)?,
    ] {
        if n.store_f32()? != stored {
            return Err(Error::Unsupported(
                "_CIpow result is within one binary64 ulp of a binary32 boundary",
            ));
        }
    }
    Ok(FloatBits(stored))
}

/// One binary64 ulp of a positive normal value (power of two below it at 2^-52 scale).
fn ulp53(v: &Binary) -> Result<Binary> {
    // v is positive; find e with 2^e <= v < 2^(e+1) by halving/doubling 1.0.
    let two = Binary::from_i32(2);
    let half = Binary::from_f32_bits(0x3f00_0000)?;
    let mut p = Binary::from_i32(1);
    while p.compare(v).is_gt() {
        p = p.mul(&half, Precision::Extended64)?;
    }
    while p.mul(&two, Precision::Extended64)?.compare(v).is_le() {
        p = p.mul(&two, Precision::Extended64)?;
    }
    Ok(p.mul(
        &Binary::from_f64_bits(0x3cb0_0000_0000_0000)?,
        Precision::Extended64,
    )?) // 2^-52
}

/// SVars GameSpeedMin/Max (+0x60/+0x64) and GameSpeedFilmMin/Max (+0x68/+0x6C),
/// AdminTicksPerSecond (+0x20). Shipped values: -2, 2, -4, 4; 16 (default).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeedSettings {
    pub game_speed_min: FloatBits,
    pub game_speed_max: FloatBits,
    pub film_min: FloatBits,
    pub film_max: FloatBits,
    pub admin_ticks_per_second: FloatBits,
}

/// Session and world state read by 0x76F370/0x76B3A0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeedContext {
    pub session_mode: i32,
    pub session_f4: bool,
    /// GWorld+0x275 cutscene mode, or None without a world.
    pub world_cutscene: Option<bool>,
}

/// Result of [`Timeline::set_game_speed`]: the sync-log entry written at
/// 0x76F467 and whether the UI "speed changed" message branch 0x76F474 runs
/// (|old - new| >= 1e-4). The caller owns that UI message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeedChange {
    pub changed: bool,
    pub sync_log: SyncLog,
}

impl Timeline {
    /// 0x76B230: +0x2C = min(f32 pow(2, +0x20), +0x24) / AdminTicksPerSecond.
    pub fn apply_game_speed(
        &mut self,
        admin_ticks_per_second: FloatBits,
        path: PowPath,
        precision: Precision,
    ) -> Result<()> {
        let mut factor = pow2_store_f32(self.speed.0, path, precision)?;
        // FCOMP [+0x24]; TEST AH,0x41: keep unless strictly greater.
        if num(factor.0)?.compare(&num(self.max_speed.0)?).is_gt() {
            factor = self.max_speed;
        }
        self.game_per_admin_tick = FloatBits(
            num(factor.0)?
                .div(&num(admin_ticks_per_second.0)?, precision)?
                .store_f32()?,
        );
        Ok(())
    }

    /// 0x76F370 KKC_Sim::SetGameSpeed (decree KKC_SetGameSpeedOrder 0x75D810,
    /// cutscene 0x4F1080, triggers 0x5420D0, UI 0x61E620/0x76F530).
    pub fn set_game_speed(
        &mut self,
        speed: FloatBits,
        settings: &SpeedSettings,
        context: &SpeedContext,
        path: PowPath,
        precision: Precision,
    ) -> Result<SpeedChange> {
        let film = context.session_mode != 0 && !context.session_f4 && context.session_mode == 4;
        let min = if film {
            settings.film_min
        } else {
            settings.game_speed_min
        };
        // 0x76B3A0.
        let max = if context.world_cutscene == Some(true) || film {
            settings.film_max
        } else {
            settings.game_speed_max
        };
        let mut arg = speed;
        if num(arg.0)?.compare(&num(min.0)?).is_lt() {
            arg = min;
        } else if num(arg.0)?.compare(&num(max.0)?).is_gt() {
            arg = max;
        }
        let delta = num(num(self.speed.0)?
            .sub(&num(arg.0)?, precision)?
            .store_f32()?)?;
        let magnitude = if delta.is_negative_nonzero() {
            delta.negated()
        } else {
            delta
        };
        let changed = !magnitude.compare(&num(SPEED_CHANGE_EPSILON_BITS)?).is_lt();
        self.speed = arg;
        self.apply_game_speed(settings.admin_ticks_per_second, path, precision)?;
        Ok(SpeedChange {
            changed,
            sync_log: SyncLog::GameTimePerTick(self.game_per_admin_tick),
        })
    }
}
