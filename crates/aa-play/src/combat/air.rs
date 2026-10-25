//! The aircraft Mind (AAI.cpp, vtable 0xA32710) hosted on aa-mind
//! (`crate::minds`). Owner: core (the HUD air orders).
//!
//! The native flow of a denizen aircraft:
//! 1. Created (0x4F9C10): the Mind's InitExisting 0x517C60 clears the move
//!    state's +0x48 byte, leaves any formation (0x55EB30(0)) and puts the
//!    Mind into aai_queued (Mind+0x188, replace_all). Queued, takeoff and
//!    landing have priority 3 and defer rejected commands (state virtual +8).
//! 2. CreateDenizen sets the home (+0x28, 0x52C280) and mode (+0x24,
//!    0x51C220); the mission (0x568180) sets the group (+0x30, 0x52A020).
//! 3. The mission's use (0x568480) delivers the command (0x4D9B60 -> Mind
//!    0x52C320, `aa_mind::command::process`). The handler (kill 0x51C340,
//!    move 0x5187D0) cannot activate its priority-2 state over the queued
//!    one, so the command waits on the pending queue.
//! 4. The runway release (Mind +0x184 = 0x518980) does replace_all to
//!    takeoff (0x5181D0). The takeoff legs are flown; the path end (Mind
//!    +0x144 = 0x517DD0, from MoveTick) marks it launched. Its next Think pops
//!    it, and the default (0x517D00) dispatches the pending command
//!    (0x52BF30): attack, or recon to a point.
//! 5. Recon done (or no command): join (0x5176A0) flies to the home's
//!    approach point (0x518260); the path end starts the landing (0x5184C0);
//!    the landing's path end enters the base (0x519F30 -> 0x4CE9C0).
//!
//! The Thinks are aa-mind's exact ports. Their services and the activations
//! they call are ported here from AA.exe. The flight (AircraftComponent,
//! actor+0xBC) is the movement owner's `books.actor(a).aircraft.flight`,
//! read and written through its native word accessors and the
//! `crate::movement` flight entry points.
use crate::books::AirOrder;
use crate::minds::{Hosted, HostedCommand, HostedKind, core_word, mind_id};
use crate::scheduler::{Owner, THINK};
use crate::world::{ActorId, World};
use aa_mind::command::{self, Context, Definition};
use aa_mind::uai_thinks::{AircraftWorld, ThinkWorld, aai_join, aai_landing, aai_queued, aai_recon, aai_takeoff, aircraft as off};
use aa_mind::{Arena, Error, MindId, Result, StateId};
use aa_numeric::{Binary, Precision};
use glam::{Vec2, Vec3};
use std::collections::BTreeMap;

/// (core, temporary) the attack states while they are written.
#[cfg(feature = "air-b2")]
mod attack;

/// The simulation thread's x87 precision.
const PC: Precision = Precision::Double53;
/// Mind words.
const MODE: u32 = 0x24;
const HOME: u32 = 0x28;
const GROUP: u32 = 0x30;
/// Mind+0x7C: the move state's +0x48 byte (formation flag).
const MOVE_FORMATION: u32 = off::MOVE + 0x48;
/// Mind+0x1D4: the home denizen's mission holding this aircraft (0x51A1A0).
const FORMATION_GROUP: u32 = 0x1d4;
/// Predefined actor commands (aa-rules PREDEFINED_ACTOR_COMMANDS).
const COMMAND_KILL: u32 = 0;
const COMMAND_MOVE: u32 = 5;
/// Aircraft Mind command handlers (aa-mind `command::HANDLER_SLOTS`).
const HANDLER_KILL: u32 = 0xc0;
const HANDLER_GUARD: u32 = 0xc8;
const HANDLER_MOVE: u32 = 0xbc;

fn fail(what: impl Into<String>) -> Error {
    Error::World(what.into())
}
fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn b(value: f32) -> u32 {
    value.to_bits()
}
fn num(value: f32) -> Result<Binary> {
    Binary::from_f32_bits(value.to_bits()).map_err(|e| fail(format!("aircraft Mind arithmetic: {e}")))
}

/// 0x529FC0: the state metadata +4 word.
fn priority(offset: u32) -> u32 {
    match offset {
        off::TAKEOFF | off::LANDING | off::QUEUED => 3,
        _ => 2,
    }
}

/// State virtual +8 (0x6A71D0 true for takeoff, landing and queued).
fn defers(offset: u32) -> bool {
    matches!(offset, off::TAKEOFF | off::LANDING | off::QUEUED)
}

/// World effects that run after the Mind host returns.
enum After {
    /// 0x4CE9C0: the aircraft enters its base.
    Enter { aircraft: ActorId, base: ActorId },
    /// 0x4D7960 on the aircraft.
    Destroy(ActorId),
}

struct Host<'a> {
    world: &'a mut World,
    hosted: &'a mut BTreeMap<ActorId, Hosted>,
    owners: &'a BTreeMap<StateId, (ActorId, u32)>,
    commands: &'a mut BTreeMap<u32, HostedCommand>,
    next_command: &'a mut u32,
    resetting: &'a mut bool,
    after: Vec<After>,
    /// Each aircraft's attack state status when the host started (Mind
    /// virtual +0x4C of another aircraft reads it).
    attack_status: BTreeMap<ActorId, u32>,
}

/// Run `f` with the Mind host; its errors become notes.
fn with_host<R>(world: &mut World, f: impl FnOnce(&mut Arena, &mut Host<'_>) -> Result<R>) -> Option<R> {
    let mut minds = std::mem::take(&mut world.books.minds);
    let (result, after) = {
        let crate::minds::Minds { arena, hosted, owners, commands, next_command, resetting, .. } = &mut minds;
        let attack_status = hosted
            .iter()
            .filter_map(|(&a, h)| h.states.get(&off::ATTACK).and_then(|s| arena.states.get(s)).map(|s| (a, s.status)))
            .collect();
        let mut host = Host { world: &mut *world, hosted, owners, commands, next_command, resetting, after: Vec::new(), attack_status };
        let result = f(arena, &mut host);
        (result, std::mem::take(&mut host.after))
    };
    world.books.minds = minds;
    let out = match result {
        Ok(v) => Some(v),
        Err(e) => {
            world.books.minds.note(e);
            None
        }
    };
    for a in after {
        match a {
            After::Enter { aircraft, base } => enter_base(world, aircraft, base),
            After::Destroy(aircraft) => destroy(world, aircraft),
        }
    }
    out
}

/// 0x4CE9C0 for an aircraft entering a base with a DenizenComponent: its
/// slot takes it back (0x569740), then the actor leaves the world
/// (0x4F6760).
fn enter_base(world: &mut World, a: ActorId, base: ActorId) {
    crate::denizen::returned(world, base, a);
    remove(world, a);
}

/// 0x4D7960 on an aircraft (destroyed): its slot frees (0x568B10) and it
/// leaves the world.
fn destroy(world: &mut World, a: ActorId) {
    crate::denizen::died(world, a);
    remove(world, a);
}

fn remove(world: &mut World, a: ActorId) {
    if let Some(g) = group(world, a) {
        set_group(world, a, None);
        let _ = g;
    }
    if let Some(x) = world.actors.get_mut(a as usize) {
        x.alive = false;
        x.removed = true;
    }
}

impl Host<'_> {
    fn owner(&self, state: StateId) -> Result<(ActorId, u32)> {
        self.owners.get(&state).copied().ok_or_else(|| fail(format!("unknown state {}", state.0)))
    }
    fn state_at(&self, a: ActorId, offset: u32) -> Result<StateId> {
        self.hosted.get(&a).and_then(|h| h.states.get(&offset)).copied().ok_or_else(|| fail(format!("aircraft {a} has no state at {offset:#x}")))
    }
    fn word(&self, arena: &Arena, a: ActorId, offset: u32) -> u32 {
        if let Some(i) = core_word(offset) {
            return arena.minds.get(&mind_id(a)).map_or(0, |m| m.words_24_30[i]);
        }
        self.hosted.get(&a).and_then(|h| h.words.get(&offset)).copied().unwrap_or(0)
    }
    fn set_word(&mut self, arena: &mut Arena, a: ActorId, offset: u32, bits: u32) {
        if let Some(i) = core_word(offset) {
            if let Some(m) = arena.minds.get_mut(&mind_id(a)) {
                m.words_24_30[i] = bits;
            }
            return;
        }
        if let Some(h) = self.hosted.get_mut(&a) {
            h.words.insert(offset, bits);
        }
    }
    fn byte(&self, a: ActorId, offset: u32) -> u8 {
        self.hosted.get(&a).and_then(|h| h.bytes.get(&offset)).copied().unwrap_or(0)
    }
    fn set_byte(&mut self, a: ActorId, offset: u32, value: u8) {
        if let Some(h) = self.hosted.get_mut(&a) {
            h.bytes.insert(offset, value);
        }
    }
    fn flight(&self, a: ActorId) -> Result<&aa_movement::aircraft::AircraftState> {
        self.world.books.actor(a).and_then(|x| x.aircraft.as_ref()).map(|x| &x.flight).ok_or_else(|| fail(format!("actor {a} has no AircraftComponent")))
    }
    fn flight_mut(&mut self, a: ActorId) -> Result<&mut aa_movement::aircraft::AircraftState> {
        self.world.books.actors.get_mut(a as usize).and_then(|x| x.aircraft.as_mut()).map(|x| &mut x.flight).ok_or_else(|| fail(format!("actor {a} has no AircraftComponent")))
    }
    fn gtid(&self, a: ActorId) -> u32 {
        self.world.ids.actor(a).map_or(0, |g| g.raw())
    }
    /// [B364C0]: the object a GTID names (removed actors still resolve).
    fn lookup(&self, gtid: u32) -> Option<ActorId> {
        match self.world.ids.thing(gtid)? {
            crate::ids::Thing::Actor(a) => Some(a),
            _ => None,
        }
    }
    fn def(&self, a: ActorId) -> &crate::data::Def {
        self.world.data.def(self.world.actors[a as usize].def)
    }
    fn heading(&self, a: ActorId) -> [u32; 2] {
        let facing = self.world.actors[a as usize].facing;
        [b(facing.cos()), b(facing.sin())]
    }
    fn position3(&self, a: ActorId) -> [u32; 3] {
        let p = self.world.actors[a as usize].pos;
        [b(p.x), b(p.y), b(p.z)]
    }

    // ---- the Mind activations and move-state calls ported from AA.exe ----

    /// 0x52A810 on the embedded state at `offset`.
    fn activate(&mut self, arena: &mut Arena, a: ActorId, offset: u32, mode: u32) -> Result<bool> {
        let state = self.state_at(a, offset)?;
        aa_mind::activate(state, mode, arena, self)
    }

    /// 0x55EB30(0) on the flight: leave the formation (the movement owner's
    /// 0x55EDC0, `movement::join_formation(w, a, None)` once ported).
    fn leave_formation(&mut self, a: ActorId) -> Result<()> {
        let _ = self.flight(a)?;
        Ok(())
    }

    /// 0x55E1B0 on the flight path.
    fn clear_path(&mut self, a: ActorId) -> Result<()> {
        aa_movement::aircraft::clear_path(&mut self.flight_mut(a)?.path);
        Ok(())
    }

    /// 0x5176A0(mode): join. The cruise (flight +0x18 / +0x80) becomes the
    /// operational height, the move state's formation byte clears, the
    /// formation is left, the join fields +0x14..+0x2C clear, then
    /// 0x52A810(mode).
    fn join_activate(&mut self, arena: &mut Arena, a: ActorId, mode: u32) -> Result<bool> {
        let cruise = self.definition_word_of(a, 0x414)?;
        let flight = self.flight_mut(a)?;
        flight.set_word(0x80, cruise);
        flight.set_word(0x18, cruise);
        self.set_byte(a, MOVE_FORMATION, 0);
        self.leave_formation(a)?;
        for k in [0x14, 0x18, 0x1c, 0x20, 0x24, 0x28] {
            self.set_word(arena, a, off::JOIN + k, 0);
        }
        self.set_byte(a, off::JOIN + 0x2c, 0);
        self.activate(arena, a, off::JOIN, mode)
    }

    /// 0x5181D0(mode): takeoff. +0x10 (launched) clears, the move state's
    /// formation byte clears, the formation is left, the flight is on the
    /// ground (+0x10 = 1), +0x14 = the home GTID (0 when it does not
    /// resolve), then 0x52A810(mode).
    fn takeoff_activate(&mut self, arena: &mut Arena, a: ActorId, mode: u32) -> Result<bool> {
        self.set_byte(a, off::TAKEOFF + 0x10, 0);
        self.set_byte(a, MOVE_FORMATION, 0);
        self.leave_formation(a)?;
        self.flight_mut(a)?.set_byte(0x10, 1);
        let home = self.word(arena, a, HOME);
        let base = self.lookup(home).map_or(0, |h| self.gtid(h));
        self.set_word(arena, a, off::TAKEOFF + 0x14, base);
        self.activate(arena, a, off::TAKEOFF, mode)
    }

    /// 0x5184C0(mode): landing, the same with landing +0x10 = the home GTID.
    fn landing_activate(&mut self, arena: &mut Arena, a: ActorId, mode: u32) -> Result<bool> {
        self.set_byte(a, MOVE_FORMATION, 0);
        self.leave_formation(a)?;
        self.flight_mut(a)?.set_byte(0x10, 1);
        let home = self.word(arena, a, HOME);
        let base = self.lookup(home).map_or(0, |h| self.gtid(h));
        self.set_word(arena, a, off::LANDING + 0x10, base);
        self.activate(arena, a, off::LANDING, mode)
    }

    /// 0x516D70(flag) on the move state: +0x48 = flag; with it, the
    /// formation group (Mind+0x1D4) is joined (0x55EB30), or the flag clears
    /// when there is none; without it the formation is left.
    fn move_formation(&mut self, arena: &mut Arena, a: ActorId, flag: bool) -> Result<()> {
        self.set_byte(a, MOVE_FORMATION, u8::from(flag));
        if flag {
            if self.word(arena, a, FORMATION_GROUP) == 0 {
                self.set_byte(a, MOVE_FORMATION, 0);
            }
            // 0x55EB30(group): the movement owner's join_formation.
        }
        if self.byte(a, MOVE_FORMATION) == 0 {
            self.leave_formation(a)?;
        }
        Ok(())
    }

    /// 0x516BA0(x, y, target) on the move state: activated in place (mode 0)
    /// unless running; +0x14 = the target GTID (0 without; a target that
    /// does not resolve fails), the goal (x, y, operational height), the
    /// flight path cleared, +0x10 = 1, +0x30 = 0.
    fn move_goal(&mut self, arena: &mut Arena, a: ActorId, x: u32, y: u32, target: Option<ActorId>) -> Result<bool> {
        let state = self.state_at(a, off::MOVE)?;
        if arena.state(state)?.status != 2 && !aa_mind::activate(state, 0, arena, self)? {
            return Ok(false);
        }
        match target {
            None => self.set_word(arena, a, off::MOVE + 0x14, 0),
            Some(t) => {
                let g = self.gtid(t);
                self.set_word(arena, a, off::MOVE + 0x14, g);
                if self.lookup(g).is_none() {
                    return Ok(false);
                }
            }
        }
        self.set_word(arena, a, off::MOVE + 0x18, x);
        self.set_word(arena, a, off::MOVE + 0x1c, y);
        self.set_word(arena, a, off::MOVE + 0x20, 0);
        let cruise = self.definition_word_of(a, 0x414)?;
        self.set_word(arena, a, off::MOVE + 0x20, cruise);
        self.clear_path(a)?;
        self.set_byte(a, off::MOVE + 0x10, 1);
        self.set_byte(a, off::MOVE + 0x30, 0);
        Ok(true)
    }

    /// 0x517130(mode, x, y, formation): recon. +0x14 / +0x18 = the point,
    /// +0x10 (done) clears, 0x516D70(formation), 0x516BA0(x, y, null), then
    /// 0x52A810(mode).
    fn recon_activate(&mut self, arena: &mut Arena, a: ActorId, mode: u32, point: [u32; 2], formation: bool) -> Result<bool> {
        self.set_word(arena, a, off::RECON + 0x18, point[1]);
        self.set_word(arena, a, off::RECON + 0x14, point[0]);
        self.set_byte(a, off::RECON + 0x10, 0);
        self.move_formation(arena, a, formation)?;
        self.move_goal(arena, a, point[0], point[1], None)?;
        self.activate(arena, a, off::RECON, mode)
    }

    /// The aircraft definition words the Thinks read (ActorData
    /// +0x408..+0x418).
    fn definition_word_of(&self, a: ActorId, offset: u32) -> Result<u32> {
        let d = self.def(a);
        let air = d.aircraft.as_ref().ok_or_else(|| fail(format!("{} has no AircraftComponent", d.id)))?;
        Ok(match offset {
            0x408 => b(air.taxi_scale),
            0x40c => b(air.taxi_height),
            0x410 => b(air.taxi_speed),
            0x414 => b(air.operational_height),
            0x418 => b(air.bank_factor),
            _ => return Err(fail(format!("aircraft definition word {offset:#x}"))),
        })
    }

    /// A base definition's Runway words (ActorData +0x370..+0x3D8).
    fn runway_word(&self, base: ActorId, offset: u32) -> Result<u32> {
        let d = self.def(base);
        let r = d.denizen.as_ref().and_then(|x| x.runway.as_ref()).cloned().unwrap_or_default();
        let v3 = |v: [f32; 3], i: u32| b(v[i as usize]);
        let v2 = |v: [f32; 2], i: u32| b(v[i as usize]);
        Ok(match offset {
            0x374..=0x37c => v3(r.hangar_offset, (offset - 0x374) / 4),
            0x380 | 0x384 => v2(r.hangar_orientation, (offset - 0x380) / 4),
            0x388 => b(r.hangar_length),
            0x38c..=0x394 => v3(r.offset, (offset - 0x38c) / 4),
            0x398 | 0x39c => v2(r.orientation, (offset - 0x398) / 4),
            0x3a0 => b(r.length),
            0x3a4 => b(r.delay),
            0x3ac..=0x3b4 => v3(r.landing_offset, (offset - 0x3ac) / 4),
            0x3b8 | 0x3bc => v2(r.landing_orientation, (offset - 0x3b8) / 4),
            0x3c0 => b(r.landing_length),
            0x3c4..=0x3cc => v3(r.landing_hangar_offset, (offset - 0x3c4) / 4),
            0x3d0 | 0x3d4 => v2(r.landing_hangar_orientation, (offset - 0x3d0) / 4),
            0x3d8 => b(r.landing_hangar_length),
            _ => return Err(fail(format!("base definition word {offset:#x}"))),
        })
    }

    /// 0x518260(base, &point, &direction): the join approach point and
    /// direction of `base`'s runway (the takeoff runway flown backwards, or
    /// the landing runway when the definition has one).
    fn approach(&self, mover: ActorId, base: ActorId) -> Result<([u32; 3], [u32; 3])> {
        let [hx, hy] = self.heading(base).map(f);
        let landing = self.def(base).denizen.as_ref().and_then(|x| x.runway.as_ref()).is_some_and(|r| r.has_landing_runway);
        let rw = |o: u32| self.runway_word(base, o).map(f);
        let cruise = self.definition_word_of(mover, 0x414)?;
        let origin = self.world.actors[base as usize].pos;
        let frame = |q: [f32; 3]| origin + Vec3::new(hy, -hx, 0.0) * q[0] + Vec3::new(hx, hy, 0.0) * q[1] + Vec3::Z * q[2];
        if !landing {
            let (ox, oy) = (rw(0x398)?, rw(0x39c)?);
            let y = -hx * ox + hy * oy;
            let x = hy * ox + oy * hx;
            let p = frame([rw(0x38c)?, rw(0x390)?, rw(0x394)?]);
            let mut run = rw(0x3a0)?;
            if run <= 0.0 {
                run = 6.0;
            }
            let point = [b(x * run * 4.0 + p.x), b(y * run * 4.0 + p.y), cruise];
            Ok((point, [b(-x), b(-y), 0]))
        } else {
            let (ox, oy) = (rw(0x3d0)?, rw(0x3d4)?);
            let y = -hx * ox + oy * hy;
            let x = ox * hy + oy * hx;
            let p = frame([rw(0x3c4)?, rw(0x3c8)?, rw(0x3cc)?]);
            let mut run = rw(0x3d8)?;
            if run <= 0.0 {
                run = 6.0;
            }
            let point = [b(p.x - x * run * 3.0), b(p.y - y * run * 3.0), cruise];
            Ok((point, [b(x), b(y), 0]))
        }
    }

    /// The move-state Think 0x51A720: a goal set by 0x516BA0 / 0x516740 is
    /// flown when the path is empty or circling (0x560ED0), or, for the
    /// formation leader, followed with its slot offset (0x560F10).
    fn move_think(&mut self, arena: &mut Arena, a: ActorId) -> Result<()> {
        let state = self.state_at(a, off::MOVE)?;
        aa_mind::think_base(state, arena)?;
        let formation = self.byte(a, MOVE_FORMATION) != 0 && self.word(arena, a, FORMATION_GROUP) != 0;
        if !formation {
            if self.lookup(self.word(arena, a, off::MOVE + 0x14)).is_some() {
                // 0x518A30(0): following a moving target (attack); not
                // reached by recon or join.
                return Err(fail("0x518A30 (move to a moving target) is not ported"));
            }
            let (circling, segments) = {
                let fl = self.flight(a)?;
                (fl.byte(0x90).unwrap_or(0), fl.word(0x34).unwrap_or(0))
            };
            if (circling != 0 || segments == 0) && self.byte(a, off::MOVE + 0x10) != 0 {
                let speed = self.word(arena, a, off::MOVE + 0x34);
                let fl = self.flight_mut(a)?;
                fl.set_word(0x14, speed);
                fl.set_word(0x84, speed);
                let goal = Vec3::new(f(self.word(arena, a, off::MOVE + 0x18)), f(self.word(arena, a, off::MOVE + 0x1c)), f(self.word(arena, a, off::MOVE + 0x20)));
                let dir = (self.byte(a, off::MOVE + 0x30) != 0)
                    .then(|| Vec3::new(f(self.word(arena, a, off::MOVE + 0x24)), f(self.word(arena, a, off::MOVE + 0x28)), f(self.word(arena, a, off::MOVE + 0x2c))));
                // 0x560ED0 = 0x55EDC0 + 0x55E1B0 + 0x560DB0 (the movement owner's
                // replace_path once it exists).
                self.leave_formation(a)?;
                self.clear_path(a)?;
                crate::movement::fly_to(self.world, a, goal, dir.unwrap_or(Vec3::ZERO));
                let (x, y) = (self.word(arena, a, off::MOVE + 0x18), self.word(arena, a, off::MOVE + 0x1c));
                self.set_word(arena, a, off::MOVE + 0x40, x);
                self.set_word(arena, a, off::MOVE + 0x44, y);
            }
            return Ok(());
        }
        Err(fail("0x51A720 formation branch (0x560F10) is not ported"))
    }

    /// The current state's Think by its Mind offset.
    fn state_think(&mut self, arena: &mut Arena, a: ActorId, offset: u32) -> Result<()> {
        let state = self.state_at(a, offset)?;
        match offset {
            off::MOVE => self.move_think(arena, a),
            off::QUEUED => aai_queued::think(state, arena, self).map(|_| ()),
            off::TAKEOFF => aai_takeoff::think(state, arena, self, PC).map(|_| ()),
            off::RECON => aai_recon::think(state, arena, self).map(|_| ()),
            off::JOIN => aai_join::think(state, arena, self, PC).map(|_| ()),
            off::LANDING => aai_landing::think(state, arena, self, PC).map(|_| ()),
            #[cfg(feature = "air-b2")]
            off::ATTACK => aa_mind::uai_thinks::aai_attack::think(state, arena, self, PC).map(|_| ()),
            _ => Err(fail(format!("aircraft state {offset:#x} Think is not hosted yet"))),
        }
    }

    /// State virtual +0x20 at the path end (0x517DD0 calls the move state's,
    /// then the current state's).
    fn path_done(&mut self, arena: &mut Arena, a: ActorId, offset: u32) -> Result<()> {
        let mind = mind_id(a);
        match offset {
            // 0x516D10: the move state's +0x10 clears and the flight path
            // with it (the transport unload branch is not reached).
            off::MOVE => {
                self.set_byte(a, off::MOVE + 0x10, 0);
                self.clear_path(a)?;
            }
            // 0x517850: done (+0x10), think now.
            off::TAKEOFF | off::RECON => {
                self.set_byte(a, offset + 0x10, 1);
                aa_mind::World::schedule(self, mind, 0, 0, arena)?;
            }
            // 0x519120: landing (mode 2), think now.
            off::JOIN => {
                self.landing_activate(arena, a, 2)?;
                aa_mind::World::schedule(self, mind, 0, 0, arena)?;
            }
            // 0x519F30: the base (+0x10) takes the aircraft (0x4CE9C0).
            off::LANDING => {
                if let Some(base) = self.lookup(self.word(arena, a, off::LANDING + 0x10)) {
                    self.after.push(After::Enter { aircraft: a, base });
                }
            }
            // 0x517450: attack retarget pending (+0x14), think now.
            off::ATTACK => {
                self.set_byte(a, off::ATTACK + 0x14, 1);
                aa_mind::World::schedule(self, mind, 0, 0, arena)?;
            }
            // 0x6A71D0: nothing.
            off::QUEUED => {}
            _ => return Err(fail(format!("aircraft state {offset:#x} path end is not ported"))),
        }
        Ok(())
    }

    /// The kill handler 0x51C340 and the move handler 0x5187D0.
    fn handle(&mut self, arena: &mut Arena, a: ActorId, slot: u32, cmd: &HostedCommand) -> Result<bool> {
        match slot {
            HANDLER_KILL => {
                #[cfg(feature = "air-b2")]
                {
                    if cmd.flags & 1 != 0 {
                        // 0x51B790(2, target, 1): attack an actor.
                        let target = self.lookup(cmd.target);
                        return self.attack_actor(arena, a, 2, target, true);
                    }
                    if cmd.flags & 4 != 0 {
                        // 0x517360(2, point, 1): attack a point.
                        return self.attack_point(arena, a, 2, cmd.point, true);
                    }
                }
                #[cfg(not(feature = "air-b2"))]
                if cmd.flags & 5 != 0 {
                    return Err(fail("0x51B790 / 0x517360 (aircraft attack) are not switched on"));
                }
                Ok(false)
            }
            HANDLER_MOVE => {
                let x = &self.world.actors[a as usize];
                let has_mover = self.def(a).has_component("MoverComponent");
                if x.removed || !has_mover || cmd.flags & 4 == 0 {
                    return Ok(false);
                }
                // actor+0xC0 Transport with +0x39 set: 0x518550 (not
                // reached: no transport drops in aa-play); else recon.
                self.recon_activate(arena, a, 2, cmd.point, false)
            }
            HANDLER_GUARD => Err(fail("0x518850 (aircraft guard) is not ported")),
            _ => Ok(false),
        }
    }
}

impl aa_mind::World for Host<'_> {
    /// 0x529FC0: a state at least as urgent as the current one.
    fn can_activate(&mut self, state: StateId, arena: &mut Arena) -> Result<bool> {
        let (a, offset) = self.owner(state)?;
        let Some(top) = arena.mind(mind_id(a))?.stack.front().copied().flatten() else { return Ok(true) };
        let (_, current) = self.owner(top)?;
        Ok(priority(offset) >= priority(current))
    }
    /// State virtual +0x18: 0x5166C0 (status 2), the move state's 0x518010,
    /// guard / escort 0x5167D0 (status 2, +0x14 = 0).
    fn enter(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        let (a, offset) = self.owner(state)?;
        arena.state_mut(state)?.status = 2;
        match offset {
            off::MOVE => {
                // 0x518010: +0x34 = the Mover's maximum speed, the followed
                // target's position (+0x38 / +0x3C), the path cleared, the
                // formation left, +0x48 = 0.
                let speed = b(self.def(a).speed_max);
                self.set_word(arena, a, off::MOVE + 0x34, speed);
                if let Some(t) = self.lookup(self.word(arena, a, off::MOVE + 0x14)) {
                    let [x, y, _] = self.position3(t);
                    self.set_word(arena, a, off::MOVE + 0x38, x);
                    self.set_word(arena, a, off::MOVE + 0x3c, y);
                }
                self.clear_path(a)?;
                self.leave_formation(a)?;
                self.set_byte(a, MOVE_FORMATION, 0);
            }
            off::GUARD | off::ESCORT => self.set_word(arena, a, offset + 0x14, 0),
            _ => {}
        }
        Ok(())
    }
    /// State virtual +0x1C: 0x4BDDA0 (status 0); guard 0x51C8C0 and escort
    /// 0x517190 are not hosted yet.
    fn leave(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        let (_, offset) = self.owner(state)?;
        match offset {
            off::GUARD | off::ESCORT => Err(fail(format!("aircraft state {offset:#x} leave is not ported"))),
            _ => {
                arena.state_mut(state)?.status = 0;
                Ok(())
            }
        }
    }
    /// Mind+8 virtual +4 (0x50AD80): cancel the pending think, add one on
    /// queue 4 `delay` after the update's begin.
    fn schedule(&mut self, mind: MindId, delay_bits: u32, data: u32, _arena: &mut Arena) -> Result<()> {
        let owner = Owner::Mind(mind.0);
        if self.world.is_scheduled(owner) {
            self.world.unschedule(owner);
        }
        self.world.schedule_after(THINK, owner, data, f(delay_bits));
        Ok(())
    }
    /// 0x517D00: unless already resetting, the pending commands (0x52BF30);
    /// none taken: join (mode 2), then the current command is released
    /// (0x52A660).
    fn reset_default(&mut self, mind: MindId, arena: &mut Arena) -> Result<()> {
        if *self.resetting {
            return Ok(());
        }
        *self.resetting = true;
        let resumed = command::resume_pending(mind, arena, self, &mut Context::default());
        let resumed = match resumed {
            Ok(r) => r,
            Err(e) => {
                *self.resetting = false;
                return Err(e);
            }
        };
        if resumed {
            *self.resetting = false;
            return Ok(());
        }
        let joined = self.join_activate(arena, mind.0, 2);
        *self.resetting = false;
        joined?;
        command::set_current(mind, None, arena, self)
    }
    /// 0x516AB0 -> 0x529D70 -> 0x4D1350 (the actor's command mask; aa-play
    /// keeps no masks).
    fn state_changed(&mut self, _mind: MindId, _arena: &mut Arena) -> Result<()> {
        Ok(())
    }
}

impl command::World for Host<'_> {
    fn definition(&self, command: u32) -> Result<Definition> {
        let c = self.commands.get(&command).ok_or_else(|| fail(format!("unknown command {command}")))?;
        let d = aa_rules::native::actor_command::PREDEFINED_ACTOR_COMMANDS.get(c.command as usize).ok_or_else(|| fail(format!("command index {}", c.command)))?;
        Ok(Definition { component_slot: d.component_slot, mind_type: d.mind_type, behavior_flags: d.behavior_flags })
    }
    fn trailing(&self, command: u32) -> Result<bool> {
        self.commands.get(&command).map(|c| c.trailing).ok_or_else(|| fail(format!("unknown command {command}")))
    }
    fn clone_owned(&mut self, command: u32) -> Result<u32> {
        let mut c = self.commands.get(&command).cloned().ok_or_else(|| fail(format!("unknown command {command}")))?;
        c.refs = 1;
        *self.next_command += 1;
        let id = *self.next_command;
        self.commands.insert(id, c);
        Ok(id)
    }
    fn retain(&mut self, command: u32) -> Result<()> {
        let c = self.commands.get_mut(&command).ok_or_else(|| fail(format!("unknown command {command}")))?;
        c.refs += 1;
        Ok(())
    }
    fn release(&mut self, command: u32) -> Result<()> {
        let c = self.commands.get_mut(&command).ok_or_else(|| fail(format!("unknown command {command}")))?;
        c.refs = c.refs.saturating_sub(1);
        if c.refs == 0 {
            self.commands.remove(&command);
        }
        Ok(())
    }
    fn invoke(&mut self, mind: MindId, slot: u32, command: u32, arena: &mut Arena, _context: &mut Context) -> Result<bool> {
        let cmd = self.commands.get(&command).cloned().ok_or_else(|| fail(format!("unknown command {command}")))?;
        self.handle(arena, mind.0, slot, &cmd)
    }
    fn schedule(&mut self, mind: MindId, delay_bits: u32, data: u32, arena: &mut Arena) -> Result<()> {
        aa_mind::World::schedule(self, mind, delay_bits, data, arena)
    }
    fn defers_rejected_command(&mut self, state: StateId, _arena: &mut Arena) -> Result<bool> {
        Ok(defers(self.owner(state)?.1))
    }
    /// A Stop (command 6) to the aircraft: its handler +0xD4 is 0x969B10
    /// (false).
    fn deliver_stop(&mut self, _actor: u32, _arena: &mut Arena) -> Result<bool> {
        Ok(false)
    }
    /// Actor +8: bit 0 removed, bit 2 a deployed denizen, bit 30 hidden.
    fn actor_flags(&self, actor: u32) -> Result<u32> {
        let x = self.world.actors.get(actor as usize).ok_or_else(|| fail(format!("actor {actor}")))?;
        let air = self.world.books.actor(actor).and_then(|q| q.aircraft.as_ref());
        Ok(u32::from(x.removed) | u32::from(air.is_some_and(|q| q.denizen)) << 2 | u32::from(air.is_some_and(|q| q.hidden)) << 30)
    }
    /// 0x44DEA0: aa-play keeps no command groups.
    fn has_live_group(&self, _actor: u32) -> Result<bool> {
        Ok(false)
    }
    fn finish_group_commands(&mut self, _actor: u32, _arena: &mut Arena) -> Result<()> {
        Ok(())
    }
}

impl ThinkWorld for Host<'_> {
    fn embedded_state(&mut self, mind: MindId, offset: u32) -> Result<StateId> {
        self.state_at(mind.0, offset)
    }
    fn embedded_think(&mut self, mind: MindId, offset: u32, arena: &mut Arena) -> Result<()> {
        self.state_think(arena, mind.0, offset)
    }
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        Ok(self.lookup(gtid))
    }
    fn object_gtid(&mut self, object: u32) -> Result<u32> {
        Ok(self.gtid(object))
    }
    /// [B279C0] / [B279C4].
    fn null_point(&mut self) -> Result<[u32; 2]> {
        Ok([0, 0])
    }
    fn mind_virtual_140(&mut self, _mind: MindId, _gtid: u32, _arena: &mut Arena) -> Result<()> {
        Err(fail("aircraft Mind virtual +0x140 (0x51C6F0) is not ported"))
    }
    /// Actor +0xE4: aa-play keeps no current performer object.
    fn current_performer(&mut self, _mind: MindId) -> Result<Option<u32>> {
        Ok(None)
    }
    /// Actor +0x6C: the Mover.
    fn locomotion(&mut self, mind: MindId) -> Result<Option<u32>> {
        Ok(self.def(mind.0).has_component("MoverComponent").then_some(mind.0))
    }
    fn locomotion_follow(&mut self, _locomotion: u32, _object: u32, _delay_bits: u32) -> Result<()> {
        Err(fail("0x576AD0 on an aircraft is not ported"))
    }
}

impl AircraftWorld for Host<'_> {
    fn join_activate(&mut self, mind: MindId, mode: u32, arena: &mut Arena) -> Result<()> {
        Host::join_activate(self, arena, mind.0, mode).map(|_| ())
    }
    fn object_removed(&mut self, object: u32) -> Result<bool> {
        Ok(self.world.actors.get(object as usize).is_none_or(|x| x.removed))
    }
    /// Object virtual +0x174(actor) = 0x4D1EE0: the object's owner (+0xE8)
    /// is the aircraft's owner (virtual +0x108).
    fn object_accepts(&mut self, object: u32, mind: MindId) -> Result<bool> {
        let k = self.world.actors.get(object as usize).and_then(|x| x.kingdom);
        Ok(k.is_some() && k == self.world.actors[mind.0 as usize].kingdom)
    }
    fn actor_4d7960(&mut self, mind: MindId, _arena: &mut Arena) -> Result<()> {
        self.after.push(After::Destroy(mind.0));
        Ok(())
    }
    /// Actor virtual +0x104: its top speed.
    fn actor_virtual_104(&mut self, mind: MindId) -> Result<Binary> {
        num(self.def(mind.0).speed_max)
    }
    fn definition_word(&mut self, mind: MindId, offset: u32) -> Result<u32> {
        self.definition_word_of(mind.0, offset)
    }
    fn flight_word(&mut self, mind: MindId, offset: u32) -> Result<u32> {
        self.flight(mind.0)?.word(offset).ok_or_else(|| fail(format!("flight word {offset:#x}")))
    }
    fn set_flight_word(&mut self, mind: MindId, offset: u32, bits: u32) -> Result<()> {
        if self.flight_mut(mind.0)?.set_word(offset, bits) { Ok(()) } else { Err(fail(format!("flight word {offset:#x} write"))) }
    }
    fn flight_byte(&mut self, mind: MindId, offset: u32) -> Result<u8> {
        self.flight(mind.0)?.byte(offset).ok_or_else(|| fail(format!("flight byte {offset:#x}")))
    }
    fn set_flight_byte(&mut self, mind: MindId, offset: u32, value: u8) -> Result<()> {
        if self.flight_mut(mind.0)?.set_byte(offset, value) { Ok(()) } else { Err(fail(format!("flight byte {offset:#x} write"))) }
    }
    fn set_move_word34(&mut self, mind: MindId, bits: u32) -> Result<()> {
        if let Some(h) = self.hosted.get_mut(&mind.0) {
            h.words.insert(off::MOVE + 0x34, bits);
        }
        Ok(())
    }
    /// 0x516740(position, direction): the move state (activated in place
    /// unless running) takes the goal and direction (+0x18..+0x2C, +0x10 and
    /// +0x30 set, +0x14 = 0), then 0x560DB0 appends the leg.
    fn move_fly_to(&mut self, mind: MindId, position: [u32; 3], direction: [u32; 3], arena: &mut Arena) -> Result<()> {
        let a = mind.0;
        let state = self.state_at(a, off::MOVE)?;
        if arena.state(state)?.status != 2 && !aa_mind::activate(state, 0, arena, self)? {
            return Ok(());
        }
        for (k, v) in [(0x18, position[0]), (0x1c, position[1]), (0x20, position[2])] {
            self.set_word(arena, a, off::MOVE + k, v);
        }
        self.set_byte(a, off::MOVE + 0x10, 1);
        self.set_byte(a, off::MOVE + 0x30, 1);
        self.set_word(arena, a, off::MOVE + 0x24, direction[0]);
        self.set_word(arena, a, off::MOVE + 0x28, direction[1]);
        self.set_word(arena, a, off::MOVE + 0x14, 0);
        self.set_word(arena, a, off::MOVE + 0x2c, direction[2]);
        let point = Vec3::new(f(position[0]), f(position[1]), f(position[2]));
        let dir = Vec3::new(f(direction[0]), f(direction[1]), f(direction[2]));
        crate::movement::fly_to(self.world, a, point, dir);
        Ok(())
    }
    /// 0x428430: any component unequal (unordered counts as unequal).
    fn vectors_differ(&mut self, a: [u32; 3], bb: [u32; 3]) -> Result<bool> {
        Ok((0..3).any(|k| f(a[k]) != f(bb[k])))
    }
    fn object_position3(&mut self, object: u32) -> Result<[u32; 3]> {
        Ok(self.position3(object))
    }
    fn object_definition_word(&mut self, object: u32, offset: u32) -> Result<u32> {
        self.runway_word(object, offset)
    }
    fn relation(&mut self, _mind: MindId, _object: Option<u32>, _a: u32, _b: u32) -> Result<i32> {
        Err(fail("0x4D4FF0 on an aircraft is not ported"))
    }
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]> {
        let [x, y, _] = self.position3(object);
        Ok([x, y])
    }
    fn actor_position(&mut self, mind: MindId) -> Result<[u32; 2]> {
        let [x, y, _] = self.position3(mind.0);
        Ok([x, y])
    }
    fn actor_position3(&mut self, mind: MindId) -> Result<[u32; 3]> {
        Ok(self.position3(mind.0))
    }
    /// Object words: +0x2C..+0x34 the facing vector, +0x6C / +0x7C / +0xBC
    /// the Mover / Organization / AircraftComponent pointers (non-zero when
    /// present).
    fn object_word(&mut self, object: u32, offset: u32) -> Result<u32> {
        let [hx, hy] = self.heading(object);
        let d = self.def(object);
        Ok(match offset {
            0x2c => hx,
            0x30 => hy,
            0x34 => 0,
            0x6c => u32::from(d.has_component("MoverComponent")),
            0x7c => u32::from(d.has_component("OrganizationComponent")),
            0xbc => u32::from(d.has_component("AircraftComponent")),
            _ => return Err(fail(format!("object word {offset:#x}"))),
        })
    }
    fn actor_word(&mut self, mind: MindId, offset: u32) -> Result<u32> {
        self.object_word(mind.0, offset)
    }
}

impl aai_queued::QueuedThinkWorld for Host<'_> {
    /// Actor +0xC0: the TransportComponent (a bomber's paratroop hold).
    fn base(&mut self, mind: MindId) -> Result<Option<u32>> {
        Ok(self.def(mind.0).has_component("TransportComponent").then_some(mind.0))
    }
    /// Transport +0x4C: aa-play drops no paratroopers from aircraft.
    fn base_released(&mut self, _base: u32) -> Result<bool> {
        Ok(false)
    }
    fn base_point(&mut self, _base: u32) -> Result<[u32; 2]> {
        Err(fail("transport +0x30 point is not ported"))
    }
    fn transport_activate(&mut self, _mind: MindId, _mode: u32, _point: [u32; 2], _arena: &mut Arena) -> Result<()> {
        Err(fail("0x518550 (aircraft transport) is not ported"))
    }
}

impl aai_takeoff::TakeoffThinkWorld for Host<'_> {
    fn launched(&mut self, state: StateId) -> Result<bool> {
        let (a, o) = self.owner(state)?;
        Ok(self.byte(a, o + 0x10) != 0)
    }
    fn set_launched(&mut self, state: StateId, value: bool) -> Result<()> {
        let (a, o) = self.owner(state)?;
        self.set_byte(a, o + 0x10, u8::from(value));
        Ok(())
    }
    fn takeoff_base(&mut self, state: StateId) -> Result<u32> {
        let (a, o) = self.owner(state)?;
        Ok(self.hosted.get(&a).and_then(|h| h.words.get(&(o + 0x14))).copied().unwrap_or(0))
    }
    fn object_heading(&mut self, object: u32) -> Result<[u32; 2]> {
        Ok(self.heading(object))
    }
}

impl aai_recon::ReconThinkWorld for Host<'_> {
    fn recon_done(&mut self, state: StateId) -> Result<bool> {
        let (a, o) = self.owner(state)?;
        Ok(self.byte(a, o + 0x10) != 0)
    }
}

impl aai_join::AirJoinThinkWorld for Host<'_> {
    fn join_target(&mut self, state: StateId) -> Result<u32> {
        let (a, o) = self.owner(state)?;
        Ok(self.hosted.get(&a).and_then(|h| h.words.get(&(o + 0x10))).copied().unwrap_or(0))
    }
    fn set_join_target(&mut self, state: StateId, gtid: u32) -> Result<()> {
        let (a, o) = self.owner(state)?;
        if let Some(h) = self.hosted.get_mut(&a) {
            h.words.insert(o + 0x10, gtid);
        }
        Ok(())
    }
    fn arrived(&mut self, state: StateId) -> Result<bool> {
        let (a, o) = self.owner(state)?;
        Ok(self.byte(a, o + 0x2c) != 0)
    }
    fn set_arrived(&mut self, state: StateId, value: bool) -> Result<()> {
        let (a, o) = self.owner(state)?;
        self.set_byte(a, o + 0x2c, u8::from(value));
        Ok(())
    }
    fn last_position(&mut self, state: StateId) -> Result<[u32; 3]> {
        let (a, o) = self.owner(state)?;
        let h = self.hosted.get(&a);
        let w = |k: u32| h.and_then(|h| h.words.get(&(o + k))).copied().unwrap_or(0);
        Ok([w(0x14), w(0x18), w(0x1c)])
    }
    fn set_last_position(&mut self, state: StateId, point: [u32; 3]) -> Result<()> {
        let (a, o) = self.owner(state)?;
        if let Some(h) = self.hosted.get_mut(&a) {
            for (k, v) in [(0x14, point[0]), (0x18, point[1]), (0x1c, point[2])] {
                h.words.insert(o + k, v);
            }
        }
        Ok(())
    }
    fn approach_point(&mut self, state: StateId) -> Result<[u32; 3]> {
        let (a, o) = self.owner(state)?;
        let h = self.hosted.get(&a);
        let w = |k: u32| h.and_then(|h| h.words.get(&(o + k))).copied().unwrap_or(0);
        Ok([w(0x20), w(0x24), w(0x28)])
    }
    fn set_approach_point(&mut self, state: StateId, point: [u32; 3]) -> Result<()> {
        let (a, o) = self.owner(state)?;
        if let Some(h) = self.hosted.get_mut(&a) {
            for (k, v) in [(0x20, point[0]), (0x24, point[1]), (0x28, point[2])] {
                h.words.insert(o + k, v);
            }
        }
        Ok(())
    }
    /// Object virtual +0x10 (0x4EA020): definition +0x50, the shape; its +4
    /// is the bounding radius (here of the blocking footprint).
    fn object_virtual_10(&mut self, object: u32) -> Result<u32> {
        let d = self.def(object);
        let r = match (d.blocking_shape.as_deref(), d.blocking_size) {
            (Some(s), Some([w, h])) if s.eq_ignore_ascii_case("CIRCLE") => w.max(h),
            (Some(_), Some([w, h])) => (Vec2::new(w, h) * 0.5).length(),
            _ => 0.0,
        };
        Ok(b(r))
    }
    fn actor_owner(&mut self, mind: MindId) -> Result<u32> {
        self.world.actors[mind.0 as usize].kingdom.ok_or_else(|| fail("aircraft without an owner"))
    }
    /// 0x4E1D90(aircraft) on the owner: its nearest live actor with a
    /// DenizenComponent that keeps this aircraft's definition (0x568EA0 /
    /// 0x568120), by planar squared distance (strictly less wins).
    fn owner_base(&mut self, owner: u32, mind: MindId) -> Result<Option<u32>> {
        let me = &self.world.actors[mind.0 as usize];
        let (def, pos) = (me.def, me.pos.truncate());
        let mut best: Option<(ActorId, f32)> = None;
        for (i, x) in self.world.actors.iter().enumerate() {
            if x.removed || !x.alive || x.kingdom != Some(owner) {
                continue;
            }
            let Some(dd) = self.world.data.def(x.def).denizen.as_ref() else { continue };
            if !dd.slots.contains(&def) {
                continue;
            }
            let d = x.pos.truncate() - pos;
            let d2 = d.y * d.y + d.x * d.x;
            if best.is_none_or(|(_, bd)| d2 < bd) {
                best = Some((i as ActorId, d2));
            }
        }
        Ok(best.map(|(a, _)| a))
    }
    /// Mind virtual +0x40 (0x52C280): the new home.
    fn mind_virtual_40(&mut self, mind: MindId, object: Option<u32>, arena: &mut Arena) -> Result<()> {
        let word = match object {
            Some(o) if !self.object_removed(o)? && self.object_accepts(o, mind)? => self.gtid(o),
            _ => 0,
        };
        self.set_word(arena, mind.0, HOME, word);
        Ok(())
    }
    /// Definition +0x370: a Runway.
    fn definition_370(&mut self, object: u32) -> Result<bool> {
        Ok(self.def(object).denizen.as_ref().is_some_and(|d| d.runway.is_some()))
    }
    fn landing_approach(&mut self, mind: MindId, object: u32, _point: [u32; 3], _direction: [u32; 3], _arena: &mut Arena) -> Result<([u32; 3], [u32; 3])> {
        self.approach(mind.0, object)
    }
    /// 0x45D6F0(0) on the definition: its speed.
    fn definition_45d6f0(&mut self, mind: MindId) -> Result<Binary> {
        num(self.def(mind.0).speed_max)
    }
}

impl aai_landing::LandingThinkWorld for Host<'_> {
    fn landing_base(&mut self, state: StateId) -> Result<u32> {
        let (a, o) = self.owner(state)?;
        Ok(self.hosted.get(&a).and_then(|h| h.words.get(&(o + 0x10))).copied().unwrap_or(0))
    }
    fn object_heading(&mut self, object: u32) -> Result<[u32; 2]> {
        Ok(self.heading(object))
    }
    /// Definition bytes: +0x370 a Runway, +0x3A8 has_landing_runway.
    fn object_definition_byte(&mut self, object: u32, offset: u32) -> Result<u8> {
        let r = self.def(object).denizen.as_ref().and_then(|d| d.runway.as_ref());
        Ok(match offset {
            0x370 => u8::from(r.is_some()),
            0x3a8 => u8::from(r.is_some_and(|r| r.has_landing_runway)),
            _ => return Err(fail(format!("base definition byte {offset:#x}"))),
        })
    }
}

// ---- the public entry points -------------------------------------------

/// The aircraft Mind's InitExisting 0x517C60 (a created aircraft): with an
/// empty stack, the move state's +0x48 clears, the formation is left and
/// queued is activated (mode 2); then 0x529EC0.
pub fn init_existing(world: &mut World, a: ActorId) {
    with_host(world, |arena, host| {
        let mind = mind_id(a);
        if arena.mind(mind)?.stack.is_empty() {
            host.set_byte(a, MOVE_FORMATION, 0);
            host.leave_formation(a)?;
            host.activate(arena, a, off::QUEUED, 2)?;
        }
        if arena.mind(mind)?.stack.is_empty() {
            aa_mind::World::reset_default(host, mind, arena)?;
        }
        Ok(())
    });
}

/// Mind virtual +0x184 (0x518980): takeoff (mode 2), then think now.
pub fn release(world: &mut World, a: ActorId) {
    with_host(world, |arena, host| {
        host.takeoff_activate(arena, a, 2)?;
        aa_mind::World::schedule(host, mind_id(a), 0, 0, arena)
    });
}

/// Mind virtual +0x144 (0x517DD0), from MoveTick at the path end: the move
/// state's, then the current state's +0x20.
pub fn path_finished(world: &mut World, a: ActorId) {
    with_host(world, |arena, host| {
        host.path_done(arena, a, off::MOVE)?;
        if let Some(top) = arena.mind(mind_id(a))?.stack.front().copied().flatten() {
            let (_, offset) = host.owner(top)?;
            host.path_done(arena, a, offset)?;
        }
        Ok(())
    });
}

/// 0x4D9B60 for the mission's command: the kill command (0x4CA330) at an
/// actor or a point, or the move command (0x4CA760) for recon, processed by
/// the Mind (0x52C320). The aircraft command mask (0x518720: bits 0, 2, 5)
/// always takes them.
pub fn command(world: &mut World, a: ActorId, order: AirOrder) {
    let gtid = |w: &World, t: ActorId| w.ids.actor(t).map_or(0, |g| g.raw());
    let (index, target, point) = match order {
        AirOrder::AttackActor(t) => (COMMAND_KILL, Some(gtid(world, t)), None),
        AirOrder::AttackPoint(p) => (COMMAND_KILL, None, Some([b(p.x), b(p.y)])),
        AirOrder::Recon(p) => (COMMAND_MOVE, None, Some([b(p.x), b(p.y)])),
        AirOrder::Escort(_) => return,
    };
    let id = world.books.minds.new_command(index, target, point);
    if let Some(c) = world.books.minds.commands.get_mut(&id) {
        c.refs = 1;
    }
    with_host(world, |arena, host| {
        let r = command::process(mind_id(a), id, arena, host, &mut Context::default());
        command::World::release(host, id)?;
        r.map(|_| ())
    });
}

/// The aircraft Mind's queue-4 update 0x51A1A0: an aircraft not removed (and
/// without flag bit 6) finds its formation group in its home's missions
/// when it has none (0x568ED0), its current state Thinks, and with nothing
/// scheduled it thinks again in 2.0 s.
pub fn think(world: &mut World, a: ActorId) {
    if !world.actors.get(a as usize).is_some_and(|x| !x.removed) {
        return;
    }
    if world.books.minds.word(a, FORMATION_GROUP) == 0
        && let Some(home) = home(world, a)
        && let Some(g) = crate::denizen::missions(world, home).into_iter().find(|&g| crate::denizen::group_members(world, g).contains(&a))
    {
        crate::denizen::retain_group(world, g);
        world.books.minds.set_word(a, FORMATION_GROUP, g);
    }
    with_host(world, |arena, host| {
        let mind = mind_id(a);
        if let Some(top) = arena.mind(mind)?.stack.front().copied().flatten() {
            let (_, offset) = host.owner(top)?;
            host.state_think(arena, a, offset)?;
        }
        if !host.world.is_scheduled(Owner::Mind(a)) {
            aa_mind::World::schedule(host, mind, b(2.0), 0, arena)?;
        }
        Ok(())
    });
}

/// The state on top of `a`'s Mind, by its Mind offset (aa-mind
/// `uai_thinks::aircraft`).
pub fn current_state(world: &World, a: ActorId) -> Option<u32> {
    world.books.minds.top_offset(a)
}

// ---- the Mind's construction and native words ---------------------------

/// Whether `a` has a hosted aircraft Mind.
pub fn is_aircraft(world: &World, a: ActorId) -> bool {
    world.books.minds.is_hosted(a, HostedKind::Aircraft)
}

struct NoDraws;
impl aa_mind::construct::World for NoDraws {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        Err(aa_mind::Error::World("the aircraft Mind constructor draws nothing".into()))
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        Ok(0)
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World("the aircraft Mind constructor writes no jitter".into()))
    }
}

/// 0x4CF480 case 4 for an actor with an AircraftComponent: its aircraft
/// Mind (no draws).
pub fn created(world: &mut World, a: ActorId) {
    if is_aircraft(world, a) {
        return;
    }
    match aa_mind::construct::construct(aa_mind::construct::Kind::Aircraft, a, &mut NoDraws, aa_numeric::Precision::Double53) {
        Ok(built) => {
            if let Err(e) = world.books.minds.install(a, HostedKind::Aircraft, built) {
                world.books.minds.note(format!("aircraft Mind {a}: {e}"));
            }
        }
        Err(e) => world.books.minds.note(format!("aircraft Mind {a}: {e}")),
    }
}

/// The GTID word of an actor (0 when it has none).
fn gtid(world: &World, a: ActorId) -> u32 {
    world.ids.actor(a).map_or(0, |g| g.raw())
}

/// The live actor a GTID word names.
fn resolve(world: &World, gtid: u32) -> Option<ActorId> {
    match world.ids.thing(gtid)? {
        crate::ids::Thing::Actor(a) => world.actor(a).map(|_| a),
        _ => None,
    }
}

/// Mind virtual +0x40 (0x52C280): the home becomes `home` when it is live
/// and accepts the aircraft (its virtual +0x174); otherwise +0x28 = 0.
pub fn set_home(world: &mut World, a: ActorId, home: Option<ActorId>) {
    let word = home.filter(|&h| world.actor(h).is_some()).map_or(0, |h| gtid(world, h));
    world.books.minds.set_word(a, HOME, word);
}

/// The live home (+0x28).
pub fn home(world: &World, a: ActorId) -> Option<ActorId> {
    resolve(world, world.books.minds.word(a, HOME))
}

/// Mind virtual +0x3C (0x51C220): +0x24 = the denizen mode.
pub fn set_mode(world: &mut World, a: ActorId, mode: u32) {
    world.books.minds.set_word(a, MODE, mode);
}

/// 0x52A020: Mind +0x30 = the group, retaining the new one and releasing
/// the old one.
pub fn set_group(world: &mut World, a: ActorId, group: Option<u32>) {
    let old = world.books.minds.word(a, GROUP);
    let new = group.unwrap_or(0);
    if old == new || !is_aircraft(world, a) {
        return;
    }
    if new != 0 {
        crate::denizen::retain_group(world, new);
    }
    world.books.minds.set_word(a, GROUP, new);
    if old != 0 {
        crate::denizen::release_group(world, old);
    }
}

/// Mind +0x30.
pub fn group(world: &World, a: ActorId) -> Option<u32> {
    Some(world.books.minds.word(a, GROUP)).filter(|&g| g != 0)
}
