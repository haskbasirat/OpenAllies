//! TGC_AudioManager volume groups (B36548+A0, five 0x28-byte records) and the
//! x87 helpers the manager's arithmetic needs.
//!
//! Group index: 0 2-D (`Audio2DVolume`), 1 3-D, 2 music, 3 speech, 4 main.
//! Record layout: +0 user, +4 game, +8 fade current, +C fade target,
//! +10 fade remaining (ms, float), +18 QPC timer (59AB60), +20 changed byte.
//! Mode bits 0xC0 of the index/mask select the field: 0x00 user, 0x40 game,
//! 0x80 fade (value, duration), 0xC0 none.
use super::{Clock, Result, Timer};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// One finite binary32 operand. Native volumes and times are never NaN in the
/// reached paths; a NaN or infinity is an explicit unsupported input here,
/// because the x87 NaN propagation of these ops is not modelled.
pub(crate) fn f(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| format!("audio manager binary32 operand {bits:#010x}: {e}"))
}
pub(crate) fn st(value: &Binary) -> Result<u32> {
    value.store_f32().map_err(|e| format!("audio manager binary32 store: {e}"))
}
pub(crate) fn e<T>(r: aa_numeric::Result<T>) -> Result<T> {
    r.map_err(|e| format!("audio manager x87: {e}"))
}
/// `fild dword` of a 59ABA0 result followed by `fadd [9F240C]` (2^32, float)
/// when the signed integer is negative: the unsigned millisecond count.
pub(crate) fn fild_unsigned(value: u32, pc: Precision) -> Result<Binary> {
    let v = Binary::from_i32(value as i32);
    if (value as i32) < 0 {
        e(v.add(&f(0x4f80_0000)?, pc))
    } else {
        Ok(v)
    }
}
/// fucompp equality as the `test ah,44h` parity tests read it. Unordered
/// operands compare unequal (both C3 and C2 set).
pub(crate) fn same(a: u32, b: u32) -> bool {
    let (x, y) = (f32::from_bits(a), f32::from_bits(b));
    x == y
}
/// `fcomp` + `test ah,41h` + `jne`: true when ST0 > src (ordered).
pub(crate) fn greater(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Greater
}
pub(crate) const ZERO: u32 = 0;
pub(crate) const ONE: u32 = 0x3f80_0000;
/// [9F4580] 0.001f.
pub(crate) const MILLI: u32 = 0x3a83_126f;
/// [9F4978] 1000.0f.
pub(crate) const THOUSAND: u32 = 0x447a_0000;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Group {
    pub user: u32,
    pub game: u32,
    pub fade: u32,
    pub target: u32,
    pub remaining: u32,
    pub timer: Timer,
    pub changed: bool,
}
impl Group {
    /// 7459D0 -> 745790(1,1,1): +C target 1.0, +10 remaining 1.0, timer
    /// started, changed = 1.
    pub(crate) fn constructed(clock: &mut impl Clock) -> Result<Self> {
        Ok(Self { user: ONE, game: ONE, fade: ONE, target: ONE, remaining: ONE, timer: Timer::start(clock)?, changed: true })
    }
}

pub const MODE_USER: u32 = 0x00;
pub const MODE_GAME: u32 = 0x40;
pub const MODE_FADE: u32 = 0x80;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Volumes {
    pub groups: [Group; 5],
}
impl Volumes {
    /// Mode 0/0x40 store: changed = (old != new) by fucompp.
    fn store(group: &mut Group, field: fn(&mut Group) -> &mut u32, value: u32) {
        let changed = !same(*field(group), value);
        *field(group) = value;
        group.changed = changed;
    }
    /// 744F90: one group by index (bits 0x3F | 0xC0 mode). Mode 0 on group 2
    /// first calls [B3656C] vt+4(1) (`music_user_changed`).
    pub(crate) fn set_one_744f90(
        &mut self,
        index_mode: u32,
        value: u32,
        duration: u32,
        clock: &mut impl Clock,
        mut music_user_changed: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        let mode = index_mode & 0xc0;
        let index = (index_mode & 0xffff_ff3f) as usize;
        let group = self.groups.get_mut(index).ok_or_else(|| format!("744F90 volume group {index} outside the five native records"))?;
        match mode {
            MODE_GAME => Self::store(group, |g| &mut g.game, value),
            MODE_FADE => Self::fade(group, value, duration, clock)?,
            0xc0 => {}
            _ => {
                if index_mode == 2 {
                    music_user_changed()?;
                }
                Self::store(group, |g| &mut g.user, value)
            }
        }
        Ok(())
    }
    /// 7450E0: every group whose bit is set in the low five bits.
    pub(crate) fn set_mask_7450e0(&mut self, mask_mode: u32, value: u32, duration: u32, clock: &mut impl Clock) -> Result<()> {
        let mode = mask_mode & 0xc0;
        let mask = mask_mode & 0xffff_ff3f;
        for (i, group) in self.groups.iter_mut().enumerate() {
            if mask & (1 << i) == 0 {
                continue;
            }
            match mode {
                MODE_GAME => Self::store(group, |g| &mut g.game, value),
                MODE_FADE => Self::fade(group, value, duration, clock)?,
                0xc0 => {}
                _ => Self::store(group, |g| &mut g.user, value),
            }
        }
        Ok(())
    }
    /// 7450E0/744F90 mode 0x80: changed = (fade != value); fade = value only
    /// when the duration is 0.0; target = value, remaining = duration, timer
    /// restarted.
    fn fade(group: &mut Group, value: u32, duration: u32, clock: &mut impl Clock) -> Result<()> {
        group.changed = !same(group.fade, value);
        if same(duration, ZERO) {
            group.fade = value;
        }
        group.target = value;
        group.remaining = duration;
        group.timer = Timer::start(clock)?;
        Ok(())
    }
    /// 7453A0: one field (0xC0 mode = fade * game, x87 result).
    pub fn field_7453a0(&self, index_mode: u32, pc: Precision) -> Result<Binary> {
        let index = (index_mode & 0xffff_ff3f) as usize;
        let g = self.groups.get(index).ok_or("7453A0 volume group outside the five native records")?;
        match index_mode & 0xc0 {
            MODE_GAME => f(g.game),
            MODE_FADE => f(g.fade),
            0xc0 => e(f(g.fade)?.mul(&f(g.game)?, pc)),
            _ => f(g.user),
        }
    }
    /// 745400: user*fade*game stored to binary32, times the main (group 4)
    /// result unless this is group 4. The product stays in ST0.
    pub fn effective_745400(&self, index: u32, pc: Precision) -> Result<Binary> {
        let index = index & 0xffff_ff3f;
        let g = self.groups.get(index as usize).ok_or("745400 volume group outside the five native records")?;
        let product = st(&e(e(f(g.user)?.mul(&f(g.fade)?, pc))?.mul(&f(g.game)?, pc))?)?;
        if index == 4 {
            return f(product);
        }
        let main = self.effective_745400(4, pc)?;
        e(main.mul(&f(product)?, pc))
    }
    /// 745450: the volume for one channel kind. Kind 2 picks music when the
    /// AudioData category is 1, else speech. Other kinds return 0.0.
    pub fn channel_745450(&self, kind: u32, category: i32, pc: Precision) -> Result<Binary> {
        let index = match kind {
            0 => 0,
            1 => 1,
            2 if category == 1 => 2,
            2 => 3,
            _ => return f(ZERO),
        };
        let g = &self.groups[index];
        let product = st(&e(e(f(g.fade)?.mul(&f(g.game)?, pc))?.mul(&f(g.user)?, pc))?)?;
        let main = self.effective_745400(4, pc)?;
        e(main.mul(&f(product)?, pc))
    }
    /// 7455E0: advance every group whose remaining time is positive.
    pub(crate) fn step_7455e0(&mut self, clock: &mut impl Clock, pc: Precision) -> Result<()> {
        for g in &mut self.groups {
            if !greater(&f(g.remaining)?, &f(ZERO)?) {
                continue;
            }
            let elapsed = fild_unsigned(g.timer.elapsed_59aba0(clock)?, pc)?;
            let remaining = f(g.remaining)?;
            if elapsed.compare(&remaining) == Ordering::Less {
                let delta = e(f(g.target)?.sub(&f(g.fade)?, pc))?;
                let ratio = e(elapsed.div(&remaining, pc))?;
                let value = e(e(delta.mul(&ratio, pc))?.add(&f(g.fade)?, pc))?;
                g.fade = st(&value)?;
                g.remaining = st(&e(remaining.sub(&elapsed, pc))?)?;
                g.timer = Timer::start(clock)?;
            } else {
                g.fade = g.target;
                g.remaining = ZERO;
            }
            g.changed = true;
        }
        Ok(())
    }
}
