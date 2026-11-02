//! DynamicDoodad deferred updates (their Updatable at component +0xC, deferred
//! +18 on GWorld+1E8). Owner: core.
//!
//! Exact steps, proven against the original by research/phase8-doodad
//! (`disturbable_step`, `breakable_step`; `tests/doodad_native.rs`):
//!
//! - Disturbable 0x5902C0: 0x5901F0 scans the circle of `affect_radius` at the
//!   doodad for a moving thing (4CEDE0); only when one is found it runs its
//!   action list if its time has come (vf+0xC 0x590380 -> 0x42A590) and always
//!   takes a new time, `now + min + u * (max - min)`, one world draw (getter
//!   0x590323). Then it re-arms with `update_time` through 0x50ACC0.
//! - Breakable 0x58FA30: state 0 and 1 cancel its events (0x50B250), the others
//!   walk the topple and regrow states (no draw) and re-arm 0x50ACC0 with what
//!   is left of the state they stop in.
//!
//! The action lists and the topple are presentation: aa-play runs no doodad
//! effect in the simulation and tells the view with [`crate::books::Notice`].
//! What the original's Break 0x58FF70 starts (state 2, from a hit) is not bound:
//! no recorded film of a stock map reaches a saved state other than 0, and the
//! Load 0x58FD60 zeroes the words on every map with GWorld+48 = 0.
use crate::scheduler::{DEFERRED, Owner};
use crate::trace::{native_unit_f64, sites};
use crate::world::World;

/// What one 0x5902C0 delivery does, from the words it reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisturbableOut {
    /// vf+0xC ran (the action list).
    pub acted: bool,
    /// Component +0x18 after the delivery.
    pub next: f32,
    /// The world draws taken (0 or 1).
    pub draws: u32,
    /// The 0x50ACC0 delay.
    pub rearm: f32,
}

/// 0x5902C0 with the scan's result as input. `draw` is called at most once.
/// `next` is component +0x18; `now` GWorld+E0. PC53: `fild`, `fmul` the
/// constant 0x3DF0000000100000, `fld max`, `fsub min`, `fmulp`, `fadd min`,
/// `fadd now`, `fstp` binary32. `fcomp` takes the action branch unless
/// `now < next` or a NaN.
pub fn disturbable_step(disturbed: bool, next: f32, now: f32, d: &crate::data::DisturbableDef, draw: impl FnOnce() -> u32) -> DisturbableOut {
    if !disturbed {
        return DisturbableOut { acted: false, next, draws: 0, rearm: d.update_time };
    }
    let acted = !(now < next || now.is_nan() || next.is_nan());
    let u = native_unit_f64(draw());
    let v = (f64::from(d.recharge_time_max) - f64::from(d.recharge_time_min)) * u;
    let v = v + f64::from(d.recharge_time_min);
    let v = v + f64::from(now);
    DisturbableOut { acted, next: v as f32, draws: 1, rearm: d.update_time }
}

/// The words of a Breakable's Updatable (component +0x18 state, +0x1C time,
/// +0x20 grow start, +0x2C scale).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BreakableWords {
    pub state: u32,
    pub time: f32,
    pub grow_start: f32,
    pub scale: f32,
}

/// What one 0x58FA30 delivery does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BreakableOut {
    pub words: BreakableWords,
    /// 0x50B250: its events leave the queue.
    pub cancelled: bool,
    /// 0x42A590 ran on the topple action list (state 2 left).
    pub toppled: bool,
    /// The 0x50ACC0 delay, when it re-arms.
    pub rearm: Option<f32>,
}

/// 0x58FA30, PC53. `left` stays on the x87 stack (binary64 at PC53), stored and
/// reloaded as binary32 only around the 0x42A590 call of state 2. The loop walks
/// the states with what is left of the elapsed time, at most ten times (then
/// a delay of exactly 1), while the delay is exactly 0 (a NaN leaves it: `jnp`
/// after `fucompp`). State 4's "not yet" branch loads +0x1C, the topple
/// duration, not its own +0x20 (0x58FACF jumps to 0x58FAEF). Reaching state 0
/// from state 7 goes round once more and cancels. An unknown state runs the
/// loop out.
pub fn breakable_step(words: BreakableWords, now: f32, d: &crate::data::BreakableDef) -> BreakableOut {
    let mut w = words;
    let mut toppled = false;
    if w.state == 0 || w.state == 1 {
        return BreakableOut { words: w, cancelled: true, toppled, rearm: None };
    }
    let mut left = f64::from(now) - f64::from(w.time);
    let mut delay = 0.0f32;
    let remaining = |duration: f32, left: f64| (f64::from(duration) - left) as f32;
    let mut i = 0;
    loop {
        i += 1;
        if i > 10 {
            delay = 1.0;
            break;
        }
        match w.state {
            0 => return BreakableOut { words: w, cancelled: true, toppled, rearm: None },
            2 => {
                if left < f64::from(d.topple_duration) || left.is_nan() {
                    delay = remaining(d.topple_duration, left);
                } else {
                    left = f64::from((left - f64::from(d.topple_duration)) as f32);
                    w.state = 4;
                    w.time = now;
                    toppled = true;
                }
            }
            3 => {
                if !d.regrow {
                    return BreakableOut { words: w, cancelled: true, toppled, rearm: None };
                }
                w.state = 5;
                w.time = now;
            }
            4 => {
                if left < f64::from(d.bounce_duration) || left.is_nan() {
                    delay = remaining(d.topple_duration, left);
                } else {
                    left -= f64::from(d.bounce_duration);
                    w.state = 3;
                    w.time = now;
                }
            }
            5 => {
                if left < f64::from(d.regrow_sink_duration) || left.is_nan() {
                    delay = remaining(d.regrow_sink_duration, left);
                } else {
                    left -= f64::from(d.regrow_sink_duration);
                    w.state = 6;
                    w.time = now;
                }
            }
            6 => {
                if left < f64::from(d.regrow_dead_duration) || left.is_nan() {
                    delay = remaining(d.regrow_dead_duration, left);
                } else {
                    left -= f64::from(d.regrow_dead_duration);
                    w.state = 7;
                    w.time = now;
                    w.grow_start = (f64::from(now) - left) as f32;
                }
            }
            7 => {
                if left < f64::from(d.regrow_grow_duration) || left.is_nan() {
                    delay = remaining(d.regrow_grow_duration, left);
                } else {
                    left -= f64::from(d.regrow_grow_duration);
                    w.state = 0;
                    w.time = now;
                    w.scale = 1.0;
                }
            }
            _ => {}
        }
        if delay != 0.0 {
            break;
        }
    }
    let rearm = (w.state != 0).then_some(delay);
    BreakableOut { words: w, cancelled: false, toppled, rearm }
}

/// 0x5901F0 at `at` with `radius`: is a moving thing in the circle? The scan's
/// tile order and its marks do not matter: the first find ends it and no draw
/// depends on which it is. The per-thing test is 0x558450: `(vf+0 + radius)^2
/// >= dx^2 + dy^2`, `dx` stored binary32, the rest PC53 (`aa_spatial::
/// circle_actors`). Required flag 0x4000 is Thing `targetable`, cleared at death
/// (0x4D7960 -> 0x4EE8E0). Organizations are things in the tile lists too, with
/// their own 4CEDE0 (`crate::movement::org_moving`).
fn disturbed(world: &World, at: [f32; 2], radius: f32) -> bool {
    let within = |p: glam::Vec2, soft: f32| {
        let reach = f64::from(soft) + f64::from(radius);
        let dx = f64::from(p.x - at[0]);
        let dy = f64::from(p.y) - f64::from(at[1]);
        reach * reach >= dy * dy + dx * dx
    };
    let actor = |a: &crate::world::Actor| {
        let d = world.data.def(a.def);
        a.alive && !a.removed && d.targetable && crate::movement::actor_moving(world, a.id) && within(a.pos.truncate(), d.soft_radius)
    };
    let org = |o: &crate::world::Organization| {
        let d = world.data.def(o.def);
        o.alive && d.targetable && within(o.anchor, d.soft_radius) && crate::movement::org_moving(world, o.id)
    };
    world.actors.iter().any(actor) || world.orgs.iter().any(org)
}

/// The Disturbable delivery of `GameMap::placed[index]`.
pub fn disturbable(world: &mut World, index: u32) {
    let Some(p) = world.map.placed.get(index as usize) else { return };
    let at = [p.position[0], p.position[1]];
    let Some(d) = world.data.find(&p.thing_id).and_then(|d| world.data.def(d).doodad.as_ref()).and_then(|d| d.disturbable) else {
        return;
    };
    let scan = disturbed(world, at, d.affect_radius);
    let now = f32::from_bits(world.clock.begin.0);
    let next = world.books.doodad_next.get(&index).copied().unwrap_or(0.0);
    let mut drawn = None;
    if scan {
        drawn = Some(world.random_u32_at(sites::DISTURBABLE));
    }
    let out = disturbable_step(scan, next, now, &d, || drawn.unwrap_or(0));
    if scan {
        world.books.doodad_next.insert(index, out.next);
    }
    if out.acted && d.actions > 0 {
        world.books.notices.push(crate::books::Notice::DoodadDisturbed { placed: index as usize });
    }
    world.schedule_after(DEFERRED, Owner::Doodad(index, 0), 0, out.rearm);
}

/// The Breakable delivery of `GameMap::placed[index]`.
pub fn breakable(world: &mut World, index: u32) {
    let Some(p) = world.map.placed.get(index as usize) else { return };
    let Some(d) = world.data.find(&p.thing_id).and_then(|d| world.data.def(d).doodad.as_ref()).and_then(|d| d.breakable) else {
        return;
    };
    let words = p.breakable.map_or(BreakableWords::default(), |w| BreakableWords { state: w[0], time: f32::from_bits(w[1]), grow_start: f32::from_bits(w[2]), scale: f32::from_bits(w[5]) });
    let words = world.books.doodad_breakable.get(&index).copied().unwrap_or(words);
    let now = f32::from_bits(world.clock.begin.0);
    let out = breakable_step(words, now, &d);
    world.books.doodad_breakable.insert(index, out.words);
    if out.cancelled {
        world.unschedule(Owner::Doodad(index, 1));
    } else if let Some(delay) = out.rearm {
        world.schedule_after(DEFERRED, Owner::Doodad(index, 1), 0, delay);
    }
}
