//! The view_fx_execute arithmetic and control kernels with no world or
//! graph types: 4289A0 timing, 42A770 over plain inputs, the 42A920 argument
//! selection and the 43EC60 loop over the E0/E4/E8 list.
use aa_numeric::{Binary, Precision};
pub type Result<T> = std::result::Result<T, String>;

pub(crate) fn f(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| format!("FX x87 operand {bits:#010x}: {e}"))
}
pub(crate) fn st(v: &Binary) -> Result<u32> {
    v.store_f32().map_err(|e| format!("FX x87 store: {e}"))
}
pub(crate) fn x<T>(r: aa_numeric::Result<T>) -> Result<T> {
    r.map_err(|e| format!("FX x87: {e}"))
}

/// The 43ED53..43EDD8 loop over any E0/E4/E8 list.
pub(crate) fn fx_loop_43ec60<T: crate::view_fx_command_list::Message, H>(
    list: &mut crate::view_fx_command_list::List<T>,
    mut run_4289a0: impl FnMut(&mut T, &mut H) -> Result<bool>,
    mut complete: impl FnMut(&mut T, &mut H) -> Result<()>,
    hooks: &mut H,
) -> Result<()> {
    let n = list.native_count() as i32;
    for _ in 0..n.max(0) {
        let Some(key) = list.pop_head() else {
            return Err("43ED79 reads +4 of a null E0 head while E8 is positive".into());
        };
        let command = list.get_mut(&key).ok_or("43ED60 popped command is not owned")?;
        if run_4289a0(command, hooks)? {
            complete(command, hooks)?;
            list.free(&key);
        } else {
            list.reappend(key)?;
        }
    }
    Ok(())
}


/// 4289A0's two binary32 stores: prev = +3C - +38, elapsed = now - +38.
pub fn times_4289a0(start: u32, last: u32, now: u32, pc: Precision) -> Result<(u32, u32)> {
    let prev = st(&x(f(last)?.sub(&f(start)?, pc))?)?;
    let elapsed = st(&x(f(now)?.sub(&f(start)?, pc))?)?;
    Ok((prev, elapsed))
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// delay > elapsed: the command stays alive.
    NotReached,
    /// delay <= prev: already handled.
    Passed,
    /// prev < delay <= elapsed: vf1C then vf18.
    Fire,
}
/// `fcom elapsed; test ah,41h; jp` then `fcomp prev; test ah,41h; jne`.
pub fn node_step(delay: u32, prev: u32, elapsed: u32) -> Result<Step> {
    let d = f(delay)?;
    if d.compare(&f(elapsed)?) == std::cmp::Ordering::Greater {
        return Ok(Step::NotReached);
    }
    if d.compare(&f(prev)?) != std::cmp::Ordering::Greater {
        return Ok(Step::Passed);
    }
    Ok(Step::Fire)
}


/// The node and command words 42A770 reads.
#[derive(Clone, Debug)]
pub struct FilterInput {
    pub byte58: bool,
    pub must_18: u32,
    pub must_not_1c: u32,
    pub group_20: Option<String>,
    pub vf30: bool,
    pub source_xy: [u32; 2],
    pub target_xy: [u32; 2],
    pub threshold_2c: u32,
    pub mode_30: u32,
    pub before_40: u32,
    pub now_44: u32,
    pub state_34: u32,
    pub ctx_state_54: u32,
}
/// The reads 42A770 makes, in call order.
pub trait FilterReads {
    fn vf20(&mut self) -> Result<bool>;
    /// cmd+20 thing exists and vtB4.
    fn target_alive(&mut self) -> Result<bool>;
    fn target_flags_4cf410(&mut self) -> Result<u32>;
    /// None when 4282A0 rejects xy, else the 42A530/54ECE0+34 group id.
    fn terrain_group_at(&mut self, xy: [u32; 2]) -> Result<Option<String>>;
    /// cmd+10 thing exists and vtB4.
    fn source_alive(&mut self) -> Result<bool>;
    /// vt100() is null, its +98 is null, or +98's +14 is the source.
    fn source_owns_14(&mut self) -> Result<bool>;
}
/// 42A770, the FXC vf1C filter, over plain inputs.
pub fn filter_values_42a770(input: &FilterInput, reads: &mut impl FilterReads) -> Result<bool> {
    if input.byte58 && !reads.vf20()? {
        return Ok(false);
    }
    if reads.target_alive()? {
        let flags = reads.target_flags_4cf410()?;
        if flags & input.must_18 != input.must_18 || input.must_not_1c & flags != 0 {
            return Ok(false);
        }
    } else if input.must_18 != 0 {
        return Ok(false);
    }
    if let Some(group) = &input.group_20 {
        let xy = if input.vf30 { input.target_xy } else { input.source_xy };
        match reads.terrain_group_at(xy)? {
            Some(id) if id.eq_ignore_ascii_case(group) => {}
            _ => return Ok(false),
        }
    }
    if input.threshold_2c != 0xbf80_0000 {
        let t = f(input.threshold_2c)?;
        let now = f(input.now_44)?;
        let before = f(input.before_40)?;
        use std::cmp::Ordering::*;
        match input.mode_30 {
            0 => {
                if now.compare(&t) == Greater || before.compare(&t) != Greater {
                    return Ok(false);
                }
            }
            1 => {
                if now.compare(&t) != Greater || before.compare(&t) == Greater {
                    return Ok(false);
                }
            }
            _ => {}
        }
    }
    if input.state_34 != 42 {
        if input.state_34 != input.ctx_state_54 {
            return Ok(false);
        }
        if input.state_34 == 0x14 && reads.source_alive()? && !reads.source_owns_14()? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// One B364C0 thing as 42A920 sees it: its +20 position and vtB4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThingState {
    pub alive_b4: bool,
    pub position: [u32; 3],
}
/// 42A920's 7474F0 arguments: the attached slot (0 source, 1 target), the
/// loop byte and the position (None when x and y are not positive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SfxArgs {
    pub attached: Option<u32>,
    pub loop_38: bool,
    pub position: Option<[u32; 3]>,
}
/// 42A920 between its child call and 7474F0 (jump table 42AAC4).
pub fn sfx_args_42a920(at: u32, loop_40: u8, source_14: [u32; 3], target_24: [u32; 3],
    source: Option<ThingState>, target: Option<ThingState>, map: [u32; 2], pc: Precision) -> Result<SfxArgs> {
    let mut attached = None;
    let mut position = [0u32; 3];
    match at {
        0 | 1 => {
            let thing = if at == 0 { source } else { target };
            match thing {
                Some(t) if t.alive_b4 => attached = Some(at),
                Some(t) => position = t.position,
                None => position = if at == 0 { source_14 } else { target_24 },
            }
        }
        2 | 4 => position = source_14,
        3 | 5 => position = target_24,
        6 => {
            let half = f(0x3f00_0000)?;
            position = [st(&x(f(map[0])?.mul(&half, pc))?)?, st(&x(f(map[1])?.mul(&half, pc))?)?, 0];
        }
        _ => {}
    }
    if attached.is_some() {
        return Ok(SfxArgs { attached, loop_38: loop_40 != 0, position: None });
    }
    let zero = f(0)?;
    let positive = |v: u32| -> Result<bool> { Ok(f(v)?.compare(&zero) == std::cmp::Ordering::Greater) };
    let position = (positive(position[0])? || positive(position[1])?).then_some(position);
    Ok(SfxArgs { attached: None, loop_38: false, position })
}

#[cfg(test)]
#[path = "view_fx_execute_tests.rs"]
mod tests;
