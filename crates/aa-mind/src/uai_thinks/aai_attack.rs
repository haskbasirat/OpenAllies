//! Original `KKC_AAIStateAttack::Think` (`51B8C0`, aircraft state
//! `aai_attack` at Mind+E8) with the inline helpers `401060` (GTID live),
//! `401290` (clamp), `445250` (vector scale), `45E790` (planar distance)
//! and `46B000` (planar `!=`).
//!
//! State words: +10 GTID (group pick), +14 byte (retarget pending), +18 the
//! target GTID, +1C arrival distance, +20 byte (has target), +24/+28/+2C the
//! attack point, +30 the attack phase (0/4 idle, 1 approach, 2 break, 3),
//! +38 the cruise value. Flight component = actor +BC.
//!
//! Order (diagnostics excluded):
//! 1. `529E60`; `55E660` on the flight component (airborne).
//! 2. Airborne with a target: the group (Mind+30) `5685B0` object's Mind
//!    (+70) virtual +4C names the target (its +14 GTID, or 0) into +10 and
//!    +18; none while idle: cruise +38 (`516720`), join (mode 2), 1e-4 s.
//! 3. With a target: a live +18 failing `4D4FF0(target, 1, 0) == 0` (or a
//!    dead one) retargets from a live +10 (`518C40` when its +7C is set,
//!    else +18 = +10), then `51A920` when +18 is live. A live +18 that passes
//!    the relation, or a busy (non-idle) phase, goes on to step 4; otherwise
//!    +18 = 0, flight +10 byte = 0, `55E1B0(flight+28)`, flight +18/+80 =
//!    +38, and (airborne, an active escort (Mind+DC) or guard (Mind+A8)
//!    state) 1e-4 s, else join (mode 2) and 1e-4 s.
//! 4. Mind +1D0 byte clear: mode 0. Set: without a target, idle joins
//!    (mode 2, 1e-4 s) and busy runs `51B4C0`; with one, the +18 object's
//!    +BC selects mode 2 (set) or 1. Modes 0/1 run `51B0C0` for a live +18
//!    while +30 is 0. `51B290(&distance)` false clears +20 (busy: `51B4C0`;
//!    idle: join mode 2, 1e-4 s). True: a pending retarget (+14) clears it,
//!    in mode 1 advances +30 (idle -> 1, 1/2 -> 3, 3 -> 4, else 0), and runs
//!    `51A920`. In phase 1, a height over ground (`4F0E50(pos, 0x123)`) not
//!    below +38*0.5 and a distance not below 0.5 lead the target (its +2C
//!    direction * its virtual +18 * planar distance / actor virtual +18) and,
//!    when the point moved, store it, `516BA0(x, y, 0)` and cruise
//!    +38*-0.5; otherwise +30 = 2 and `51A920`. The move state Thinks; a
//!    busy phase schedules 1e-4 s, a distance <= 0.5 0.6 s; else the time to
//!    the point (planar distance - +1C, binary32 dx) over the actor speed
//!    (virtual +104), or over the closing speed `max(speed - target speed *
//!    facing dot, 1)` for a target with +6C, clamped to [1e-4, 10] (or
//!    [1e-4, 1]); a non-positive distance schedules 1e-4 s.
use super::{
    AircraftWorld, IMMEDIATE_BITS, POINT_SIX_BITS, aircraft, null, set_flight_cruise, status_at,
};
use crate::{
    Arena, MindId, Result, StateId, think_base,
    x87::{n, num, store},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

const HALF: u32 = 0x3f00_0000;
const MINUS_HALF: u32 = 0xbf00_0000;
const ONE: u32 = 0x3f80_0000;
const TEN: u32 = 0x4120_0000;

/// Services of `51B8C0` in native order.
pub trait AirAttackThinkWorld: AircraftWorld {
    /// Attack state word / byte at `offset`, and writes.
    fn attack_word(&mut self, state: StateId, offset: u32) -> Result<u32>;
    fn set_attack_word(&mut self, state: StateId, offset: u32, bits: u32) -> Result<()>;
    fn attack_byte(&mut self, state: StateId, offset: u32) -> Result<u8>;
    fn set_attack_byte(&mut self, state: StateId, offset: u32, value: u8) -> Result<()>;
    /// Mind byte at `offset` (+1D0).
    fn mind_byte(&mut self, mind: MindId, offset: u32) -> Result<u8>;
    /// `55E660` on the flight component (airborne).
    fn flight_airborne(&mut self, mind: MindId) -> Result<bool>;
    /// `55E1B0` on flight +28.
    fn flight_reset_28(&mut self, mind: MindId) -> Result<()>;
    /// `5685B0` on the Mind+30 group: an object (`None` when null).
    fn group_5685b0(&mut self, group: u32) -> Result<Option<u32>>;
    /// Virtual +4C of the object's Mind (+70): an object or null.
    fn object_mind_4c(&mut self, object: u32) -> Result<Option<u32>>;
    /// `518C40`, `51A920`, `51B0C0`, `51B4C0` on the attack state.
    fn attack_518c40(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    fn attack_51a920(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    fn attack_51b0c0(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    fn attack_51b4c0(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    /// `51B290(&out)`: result and the binary32 it wrote.
    fn attack_51b290(&mut self, state: StateId, arena: &mut Arena) -> Result<(bool, u32)>;
    /// `4F0E50(&xy, flags)` on the world: ground height (ST0).
    fn ground_height(&mut self, xy: [u32; 2], flags: u32) -> Result<Binary>;
    /// Actor virtual +18 and object virtuals +18 / +104 (ST0, unstored).
    fn actor_virtual_18(&mut self, mind: MindId) -> Result<Binary>;
    fn object_virtual_18(&mut self, object: u32) -> Result<Binary>;
    fn object_virtual_104(&mut self, object: u32) -> Result<Binary>;
    /// `516BA0(x, y, z)` on the aircraft move state.
    fn move_to_point(&mut self, mind: MindId, point: [u32; 3], arena: &mut Arena) -> Result<()>;
}

fn less(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Less
}
fn greater(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Greater
}
/// `[B364C0]` lookup of the +18 target GTID.
fn live18(state: StateId, w: &mut impl AirAttackThinkWorld) -> Result<Option<u32>> {
    let gtid = w.attack_word(state, 0x18)?;
    w.resolve(gtid)
}
fn idle(word: u32) -> bool {
    word == 0 || word == 4
}

/// `401290(&value, lo, hi)`.
fn clamp(value: u32, lo: u32, hi: u32) -> Result<u32> {
    let v = n(value)?;
    if less(&v, &n(lo)?) {
        return Ok(lo);
    }
    if greater(&v, &n(hi)?) {
        return Ok(hi);
    }
    Ok(value)
}

/// `46B000`: either component unequal or unordered.
fn differ(a: [u32; 2], b: [u32; 2]) -> bool {
    (0..2).any(|k| f32::from_bits(a[k]) != f32::from_bits(b[k]))
}

fn schedule(
    mind: MindId,
    bits: u32,
    arena: &mut Arena,
    w: &mut impl AirAttackThinkWorld,
) -> Result<()> {
    w.schedule(mind, bits, 0, arena)
}

fn join_soon(mind: MindId, arena: &mut Arena, w: &mut impl AirAttackThinkWorld) -> Result<()> {
    w.join_activate(mind, 2, arena)?;
    schedule(mind, IMMEDIATE_BITS, arena, w)
}

/// Complete `51B8C0`. Returns the base Think's wrong-activity flag.
pub fn think(
    state: StateId,
    arena: &mut Arena,
    w: &mut impl AirAttackThinkWorld,
    p: Precision,
) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    let airborne = w.flight_airborne(mind)?;
    if airborne && w.attack_byte(state, 0x20)? != 0 {
        let group = arena.mind(mind)?.words_24_30[3];
        let object = w
            .group_5685b0(group)?
            .ok_or_else(|| null("5685B0 object +70 read"))?;
        let gtid = match w.object_mind_4c(object)? {
            Some(o) => w.object_gtid(o)?,
            None => 0,
        };
        w.set_attack_word(state, 0x10, gtid)?;
        w.set_attack_word(state, 0x18, gtid)?;
        if gtid == 0 && idle(w.attack_word(state, 0x30)?) {
            let cruise = w.attack_word(state, 0x38)?;
            set_flight_cruise(mind, cruise, w)?;
            join_soon(mind, arena, w)?;
            return Ok(wrong);
        }
    }
    let mut target = None;
    if w.attack_byte(state, 0x20)? != 0 {
        let mut retarget = true;
        if let Some(current) = live18(state, w)?
            && w.relation(mind, Some(current), 1, 0)? == 0
        {
            retarget = false;
        }
        if retarget {
            let pick = w.attack_word(state, 0x10)?;
            if let Some(o10) = w.resolve(pick)? {
                if w.object_word(o10, 0x7c)? != 0 {
                    w.attack_518c40(state, arena)?;
                } else {
                    w.set_attack_word(state, 0x18, pick)?;
                }
                if live18(state, w)?.is_some() {
                    w.attack_51a920(state, arena)?;
                }
            }
        }
        target = live18(state, w)?;
        let keep = match target {
            Some(t) => w.relation(mind, Some(t), 1, 0)? == 0 || !idle(w.attack_word(state, 0x30)?),
            None => false,
        };
        if !keep {
            w.set_attack_word(state, 0x18, 0)?;
            w.set_flight_byte(mind, 0x10, 0)?;
            w.flight_reset_28(mind)?;
            let cruise = w.attack_word(state, 0x38)?;
            w.set_flight_word(mind, 0x18, cruise)?;
            w.set_flight_word(mind, 0x80, cruise)?;
            if airborne
                || status_at(w, mind, aircraft::ESCORT, arena)? == 2
                || status_at(w, mind, aircraft::GUARD, arena)? == 2
            {
                return schedule(mind, IMMEDIATE_BITS, arena, w).map(|()| wrong);
            }
            join_soon(mind, arena, w)?;
            return Ok(wrong);
        }
    }
    // 51BB65.
    let mode;
    if w.mind_byte(mind, 0x1d0)? == 0 {
        mode = 0;
    } else if w.attack_byte(state, 0x20)? == 0 {
        if idle(w.attack_word(state, 0x30)?) {
            join_soon(mind, arena, w)?;
        } else {
            w.attack_51b4c0(state, arena)?;
        }
        return Ok(wrong);
    } else {
        let o18 = live18(state, w)?.ok_or_else(|| null("attack target +BC read"))?;
        mode = if w.object_word(o18, 0xbc)? != 0 { 2 } else { 1 };
    }
    if mode != 2 && live18(state, w)?.is_some() && w.attack_word(state, 0x30)? == 0 {
        w.attack_51b0c0(state, arena)?;
    }
    let (ok, distance) = w.attack_51b290(state, arena)?;
    if !ok {
        w.set_attack_byte(state, 0x20, 0)?;
        if idle(w.attack_word(state, 0x30)?) {
            join_soon(mind, arena, w)?;
        } else {
            w.attack_51b4c0(state, arena)?;
        }
        return Ok(wrong);
    }
    if w.attack_byte(state, 0x14)? != 0 {
        w.set_attack_byte(state, 0x14, 0)?;
        if mode == 1 {
            let phase = w.attack_word(state, 0x30)?;
            let next = match phase {
                0 | 4 => 1,
                1 | 2 => 3,
                3 => 4,
                _ => 0,
            };
            w.set_attack_word(state, 0x30, next)?;
        }
        w.attack_51a920(state, arena)?;
    }
    if w.attack_word(state, 0x30)? == 1 {
        approach(state, mind, target, distance, arena, w, p)?;
    }
    w.embedded_think(mind, aircraft::MOVE, arena)?;
    if !idle(w.attack_word(state, 0x30)?) {
        return schedule(mind, IMMEDIATE_BITS, arena, w).map(|()| wrong);
    }
    if !greater(&n(distance)?, &n(HALF)?) {
        return schedule(mind, POINT_SIX_BITS, arena, w).map(|()| wrong);
    }
    let pos = w.actor_position3(mind)?;
    let dx = store(num(n(w.attack_word(state, 0x24)?)?.sub(&n(pos[0])?, p))?)?;
    let dy = num(n(w.attack_word(state, 0x28)?)?.sub(&n(pos[1])?, p))?;
    let sum = num(num(dy.mul(&dy, p))?.add(&num(n(dx)?.mul(&n(dx)?, p))?, p))?;
    let reach = num(num(sum.sqrt(p))?.sub(&n(w.attack_word(state, 0x1c)?)?, p))?;
    let reach32 = store(reach.clone())?;
    if !greater(&reach, &Binary::from_i32(0)) {
        return schedule(mind, IMMEDIATE_BITS, arena, w).map(|()| wrong);
    }
    let speed = store(w.actor_virtual_104(mind)?)?;
    let delay = match target {
        Some(t) if w.object_word(t, 0x6c)? != 0 => {
            let a30 = w.actor_word(mind, 0x30)?;
            let a2c = w.actor_word(mind, 0x2c)?;
            let t30 = w.object_word(t, 0x30)?;
            let t2c = w.object_word(t, 0x2c)?;
            let dot = num(n(a30)?.mul(&n(t30)?, p))?;
            let dot = store(num(dot.add(&num(n(t2c)?.mul(&n(a2c)?, p))?, p))?)?;
            let other = w.object_virtual_104(t)?;
            let mut closing = num(n(speed)?.sub(&num(other.mul(&n(dot)?, p))?, p))?;
            if less(&closing, &n(ONE)?) {
                closing = n(ONE)?;
            }
            let t = store(num(n(reach32)?.div(&closing, p))?)?;
            clamp(t, IMMEDIATE_BITS, ONE)?
        }
        _ => {
            let t = store(num(n(reach32)?.div(&n(speed)?, p))?)?;
            clamp(t, IMMEDIATE_BITS, TEN)?
        }
    };
    schedule(mind, delay, arena, w)?;
    Ok(wrong)
}

/// Phase 1 of `51B8C0` (51BCD8..51BE95).
fn approach(
    state: StateId,
    mind: MindId,
    target: Option<u32>,
    distance: u32,
    arena: &mut Arena,
    w: &mut impl AirAttackThinkWorld,
    p: Precision,
) -> Result<()> {
    let cruise = w.attack_word(state, 0x38)?;
    let floor = store(num(n(cruise)?.mul(&n(HALF)?, p))?)?;
    let pos = w.actor_position3(mind)?;
    let ground = w.ground_height([pos[0], pos[1]], 0x123)?;
    let height = num(n(pos[2])?.sub(&ground, p))?;
    if less(&height, &n(floor)?) || less(&n(distance)?, &n(HALF)?) {
        w.set_attack_word(state, 0x30, 2)?;
        return w.attack_51a920(state, arena);
    }
    let mut point = [w.attack_word(state, 0x24)?, w.attack_word(state, 0x28)?];
    if w.attack_byte(state, 0x20)? != 0 {
        let t = target.ok_or_else(|| null("attack target position read"))?;
        let tp = w.object_position(t)?;
        // 45E790(actor+20, &target): planar distance.
        let dx = num(n(tp[0])?.sub(&n(pos[0])?, p))?;
        let dy = num(n(tp[1])?.sub(&n(pos[1])?, p))?;
        let d2 = num(num(dx.mul(&dx, p))?.add(&num(dy.mul(&dy, p))?, p))?;
        let d = store(num(d2.sqrt(p))?)?;
        let time = store(num(n(d)?.div(&w.actor_virtual_18(mind)?, p))?)?;
        let along = store(w.object_virtual_18(t)?)?;
        let dir = [
            w.object_word(t, 0x2c)?,
            w.object_word(t, 0x30)?,
            w.object_word(t, 0x34)?,
        ];
        // 445250 twice: dir * along, then * time (each binary32).
        let mut lead = [0u32; 3];
        for k in (0..3).rev() {
            let v = store(num(n(along)?.mul(&n(dir[k])?, p))?)?;
            lead[k] = store(num(n(time)?.mul(&n(v)?, p))?)?;
        }
        point = [
            store(num(n(lead[0])?.add(&n(tp[0])?, p))?)?,
            store(num(n(lead[1])?.add(&n(tp[1])?, p))?)?,
        ];
    }
    let current = [w.attack_word(state, 0x24)?, w.attack_word(state, 0x28)?];
    if differ(point, current) {
        w.set_attack_word(state, 0x24, point[0])?;
        w.set_attack_word(state, 0x28, point[1])?;
        w.set_attack_word(state, 0x2c, 0)?;
        w.move_to_point(mind, [point[0], point[1], 0], arena)?;
        let cruise = store(num(n(w.attack_word(state, 0x38)?)?.mul(&n(MINUS_HALF)?, p))?)?;
        set_flight_cruise(mind, cruise, w)?;
    }
    Ok(())
}
