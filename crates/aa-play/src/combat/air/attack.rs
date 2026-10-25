//! The aircraft attack: the kill handler's activations 0x51B790 (an actor)
//! and 0x517360 (a point), the attack state's Think 0x51B8C0 (aa-mind's
//! exact port) with its helpers, and the ability pick 0x51A260 that arms
//! the release MoveTick flies (flight +0x94..+0xA4).
//!
//! Ported for a bomber (Mind+0x1D0 clear: every ability drops a projectile,
//! 0x517C60): 0x51A920 follows the target object (0x51A670 -> 0x518A30).
//! The dog-fight branches of 0x51A920 (Mind+0x1D0 set: phases 0..4 with
//! 0x516B30 turns) are not ported yet.
//!
//! Compare against the original: the ability values are aa-play's (range,
//! recharge left, mana); 0x4C8F20 is reduced to its target filter, range,
//! mana (7) and recharge (8) outcomes for these abilities.
use super::*;
use aa_mind::uai_thinks::aai_attack;

/// [9FBDB8] 0.5, [9FD47C] -0.5, [A326B8] 1.75, [A326BC] 0.9.
const HALF: f32 = 0.5;
const MINUS_HALF: f32 = -0.5;
const BACKOFF: f32 = 1.75;
const AHEAD: f32 = 0.9;

impl Host<'_> {
    fn attack_word(&self, arena: &Arena, a: ActorId, k: u32) -> u32 {
        self.word(arena, a, off::ATTACK + k)
    }
    fn set_attack(&mut self, arena: &mut Arena, a: ActorId, k: u32, bits: u32) {
        self.set_word(arena, a, off::ATTACK + k, bits);
    }

    /// The live actor `a`'s attack state +0x18 names.
    fn attack_target(&self, arena: &Arena, a: ActorId) -> Option<ActorId> {
        self.lookup(self.attack_word(arena, a, 0x18)).filter(|&t| self.world.actor(t).is_some())
    }

    /// 0x4D4FF0(target, 1, 0) on an aircraft: 0 when one of its abilities
    /// may target it (its target filter), else 1.
    pub(super) fn relation_of(&self, a: ActorId, target: ActorId) -> i32 {
        let st = &self.world.combat;
        let def = self.def(a);
        let usable = st.extras.is_some() && def.weapons.iter().any(|w| crate::combat::admits(self.world, st, a, w, target));
        i32::from(!usable)
    }

    /// Actor virtual +0 (a radius): the blocking footprint's bounding radius.
    fn radius(&self, a: ActorId) -> f32 {
        let d = self.def(a);
        match (d.blocking_shape.as_deref(), d.blocking_size) {
            (Some(s), Some([w, h])) if s.eq_ignore_ascii_case("CIRCLE") => w.max(h),
            (Some(_), Some([w, h])) => (Vec2::new(w, h) * 0.5).length(),
            _ => 0.0,
        }
    }

    /// Actor virtual +0x18: its current speed.
    fn speed_now(&self, a: ActorId) -> f32 {
        self.world.actors[a as usize].motion.speed
    }

    /// 0x51B790(mode, target, formation): an actor with a Performer aims at
    /// `target` (+0x10, and +0x18 the target or, for an organization, its
    /// nearest usable member 0x518C40); a usable +0x18 sets +0x20, the move
    /// formation (0x516D70), the plan (0x51A920) and 0x52A810(mode).
    pub(super) fn attack_actor(&mut self, arena: &mut Arena, a: ActorId, mode: u32, target: Option<ActorId>, formation: bool) -> Result<bool> {
        if !self.def(a).has_component("PerformerComponent") {
            return Ok(false);
        }
        let gtid = target.map_or(0, |t| self.gtid(t));
        self.set_attack(arena, a, 0x10, gtid);
        self.set_attack(arena, a, 0x18, 0);
        let Some(t) = self.lookup(gtid) else { return Ok(false) };
        if self.def(t).has_component("OrganizationComponent") {
            self.retarget(arena, a)?;
        } else {
            self.set_attack(arena, a, 0x18, gtid);
        }
        let Some(t18) = self.lookup(self.attack_word(arena, a, 0x18)) else { return Ok(false) };
        if self.relation_of(a, t18) != 0 {
            return Ok(false);
        }
        self.set_byte(a, off::ATTACK + 0x20, 1);
        self.move_formation(arena, a, formation)?;
        self.plan(arena, a)?;
        self.activate(arena, a, off::ATTACK, mode)
    }

    /// 0x517360(mode, x, y, formation): an actor with a Performer aims at a
    /// map point (clamped to the map, 0x552640), its ground height, the
    /// move goal there (0x516BA0), +0x20 / +0x14 / +0x30 cleared, then
    /// 0x52A810(mode).
    pub(super) fn attack_point(&mut self, arena: &mut Arena, a: ActorId, mode: u32, point: [u32; 2], formation: bool) -> Result<bool> {
        if !self.def(a).has_component("PerformerComponent") {
            return Ok(false);
        }
        let mut p = Vec2::new(f(point[0]), f(point[1]));
        let size = Vec2::new(self.world.map.width as f32, self.world.map.height as f32) * self.world.map.cell_size;
        if !(p.x >= 0.0 && p.x < size.x && p.y >= 0.0 && p.y < size.y) {
            p = self.clamp_to_map(p);
        }
        self.set_attack(arena, a, 0x24, b(p.x));
        self.set_attack(arena, a, 0x28, b(p.y));
        self.set_attack(arena, a, 0x2c, 0);
        let ground = self.world.map.height_at(p);
        self.set_attack(arena, a, 0x2c, b(ground));
        self.set_attack(arena, a, 0x3c, b(p.x));
        self.set_attack(arena, a, 0x40, b(p.y));
        self.move_formation(arena, a, formation)?;
        self.move_goal(arena, a, b(p.x), b(p.y), None)?;
        self.set_byte(a, off::ATTACK + 0x20, 0);
        self.set_byte(a, off::ATTACK + 0x14, 0);
        self.set_attack(arena, a, 0x30, 0);
        self.activate(arena, a, off::ATTACK, mode)
    }

    /// 0x552640: each coordinate kept inside [margin, size - margin - 1e-4]
    /// (margin `[GWorld+0x30]+8`, one cell here).
    fn clamp_to_map(&self, p: Vec2) -> Vec2 {
        let m = &self.world.map;
        let margin = m.cell_size;
        let hi = Vec2::new(m.width as f32, m.height as f32) * m.cell_size - Vec2::splat(margin) - Vec2::splat(1e-4);
        Vec2::new(if p.x < margin { margin } else { p.x.min(hi.x) }, if p.y < margin { margin } else { p.y.min(hi.y) })
    }

    /// 0x518C40: the nearest live member of the +0x10 organization that the
    /// aircraft may attack (squared planar distance, strictly less wins)
    /// becomes +0x18.
    fn retarget(&mut self, arena: &mut Arena, a: ActorId) -> Result<()> {
        let Some(org_actor) = self.lookup(self.attack_word(arena, a, 0x10)) else { return Ok(()) };
        let me = self.world.actors[a as usize].pos.truncate();
        let members: Vec<ActorId> = self.world.actors[org_actor as usize].org.map(|o| self.world.orgs[o as usize].members.clone()).unwrap_or_default();
        let mut best: Option<(ActorId, f32)> = None;
        for m in members {
            if self.world.actors.get(m as usize).is_none_or(|x| x.removed) || self.relation_of(a, m) != 0 {
                continue;
            }
            let d = self.world.actors[m as usize].pos.truncate() - me;
            let d2 = d.y * d.y + d.x * d.x;
            if best.is_none_or(|(_, bd)| d2 < bd) {
                best = Some((m, d2));
            }
        }
        let g = best.map_or(0, |(m, _)| self.gtid(m));
        self.set_attack(arena, a, 0x18, g);
        Ok(())
    }

    /// 0x51A920, the attack plan: +0x14 clears; with a live +0x18 the old
    /// point is kept (+0x3C / +0x40); a bomber follows the target object
    /// (0x51A670); the cruise (flight +0x18 / +0x80) is +0x38.
    pub(super) fn plan(&mut self, arena: &mut Arena, a: ActorId) -> Result<()> {
        self.set_byte(a, off::ATTACK + 0x14, 0);
        let Some(t) = self.attack_target(arena, a) else { return Ok(()) };
        let (px, py) = (self.attack_word(arena, a, 0x24), self.attack_word(arena, a, 0x28));
        self.set_attack(arena, a, 0x3c, px);
        self.set_attack(arena, a, 0x40, py);
        let cruise = self.attack_word(arena, a, 0x38);
        if self.byte(a, 0x1d0) != 0 {
            return Err(fail("0x51A920 dog-fight branches (Mind+0x1D0 set) are not ported"));
        }
        self.follow_object(arena, a, t)?;
        let fl = self.flight_mut(a)?;
        fl.set_word(0x18, cruise);
        fl.set_word(0x80, cruise);
        Ok(())
    }

    /// 0x51A670(object) on the move state: activated in place unless
    /// running, +0x14 = the object, +0x18..+0x20 its position, 0x518A30(1),
    /// +0x30 = 0.
    fn follow_object(&mut self, arena: &mut Arena, a: ActorId, t: ActorId) -> Result<bool> {
        let state = self.state_at(a, off::MOVE)?;
        if arena.state(state)?.status != 2 && !aa_mind::activate(state, 0, arena, self)? {
            return Ok(false);
        }
        let g = self.gtid(t);
        self.set_word(arena, a, off::MOVE + 0x14, g);
        let [x, y, z] = self.position3(t);
        self.set_word(arena, a, off::MOVE + 0x18, x);
        self.set_word(arena, a, off::MOVE + 0x1c, y);
        self.set_word(arena, a, off::MOVE + 0x20, z);
        self.lead_target(arena, a, true)?;
        self.set_byte(a, off::MOVE + 0x30, 0);
        Ok(true)
    }

    /// 0x518A30(force) on the move state: the followed target's position
    /// (+0x38 / +0x3C); when the goal is not on it, or forced, and the gap
    /// (planar distance less both radii) is not negative, or forced, the
    /// lead point (target + its heading * its speed * gap / own speed) is
    /// the new goal (0x516BA0) once it moved 1 or more (or forced).
    pub(super) fn lead_target(&mut self, arena: &mut Arena, a: ActorId, force: bool) -> Result<()> {
        let Some(t) = self.lookup(self.word(arena, a, off::MOVE + 0x14)) else { return Err(fail("0x518A30 without a target")) };
        let tp = self.world.actors[t as usize].pos;
        self.set_word(arena, a, off::MOVE + 0x38, b(tp.x));
        self.set_word(arena, a, off::MOVE + 0x3c, b(tp.y));
        let goal = (f(self.word(arena, a, off::MOVE + 0x18)), f(self.word(arena, a, off::MOVE + 0x1c)), f(self.word(arena, a, off::MOVE + 0x20)));
        if !(goal.0 != tp.x || goal.1 != tp.y || goal.2 != 0.0 || force) {
            return Ok(());
        }
        let me = self.world.actors[a as usize].pos;
        let (dy, dx) = (me.y - tp.y, me.x - tp.x);
        let gap = ((dy * dy + dx * dx).sqrt() - self.radius(a)) - self.radius(t);
        if !(gap >= 0.0 || force) {
            return Ok(());
        }
        let ts = self.speed_now(t);
        let facing = self.world.actors[t as usize].facing;
        let (lead_x, lead_y) = if ts != 0.0 {
            let own = self.speed_now(a);
            (ts * facing.cos() * gap * (1.0 / own) + tp.x, tp.y + ts * facing.sin() * gap * (1.0 / own))
        } else {
            (tp.x, tp.y)
        };
        let (ly, lx) = (f(self.word(arena, a, off::MOVE + 0x44)) - lead_y, f(self.word(arena, a, off::MOVE + 0x40)) - lead_x);
        if 1.0 <= ly * ly + lx * lx || force {
            self.move_goal(arena, a, b(lead_x), b(lead_y), Some(t))?;
        }
        Ok(())
    }

    /// 0x51B0C0: the attack point leads the target by its speed over the
    /// time to reach it; a point moved more than half the formation speed
    /// (squared, against +0x3C / +0x40) re-plans, as does an idle flight
    /// without a pending retarget.
    fn lead(&mut self, arena: &mut Arena, a: ActorId) -> Result<()> {
        let Some(t) = self.attack_target(arena, a) else { return Ok(()) };
        let tp = self.world.actors[t as usize].pos;
        let me = self.world.actors[a as usize].pos;
        let (dy, dx) = (tp.y - me.y, tp.x - me.x);
        let time = (dy * dy + dx * dx).sqrt() / self.speed_now(a);
        let ts = self.speed_now(t);
        let facing = self.world.actors[t as usize].facing;
        let px = ts * facing.cos() * time + tp.x;
        let py = ts * facing.sin() * time + tp.y;
        self.set_attack(arena, a, 0x24, b(px));
        self.set_attack(arena, a, 0x28, b(py));
        self.set_attack(arena, a, 0x2c, 0);
        let (ey, ex) = (f(self.attack_word(arena, a, 0x40)) - py, f(self.attack_word(arena, a, 0x3c)) - px);
        let moved = ey * ey + ex * ex;
        if !(self.def(a).speed_max * HALF < moved) {
            let fl = self.flight(a)?;
            let flying = fl.byte(0x90).unwrap_or(0) == 0 && fl.word(0x34).unwrap_or(0) != 0;
            if flying || self.byte(a, off::ATTACK + 0x14) != 0 {
                return Ok(());
            }
        }
        self.plan(arena, a)
    }

    /// 0x51B4C0: the phase steps on (1 sets the retarget flag first); an
    /// idle result lands the flight (+0x10 = 0, 0x55E1B0), a busy one
    /// re-plans and lets the move state think; then the Mind thinks at once.
    fn phase_step(&mut self, arena: &mut Arena, a: ActorId) -> Result<()> {
        let phase = self.attack_word(arena, a, 0x30);
        if phase == 1 {
            self.set_byte(a, off::ATTACK + 0x14, 1);
        }
        if self.byte(a, off::ATTACK + 0x14) != 0 {
            self.set_byte(a, off::ATTACK + 0x14, 0);
            let next = match phase {
                0 | 4 => 1,
                1 | 2 => 3,
                3 => 4,
                _ => 0,
            };
            self.set_attack(arena, a, 0x30, next);
            if next == 0 || next == 4 {
                self.flight_mut(a)?.set_byte(0x10, 0);
                self.clear_path(a)?;
            } else {
                self.plan(arena, a)?;
            }
        }
        let phase = self.attack_word(arena, a, 0x30);
        if phase != 0 && phase != 4 {
            self.move_think(arena, a)?;
        }
        aa_mind::World::schedule(self, mind_id(a), 0x38d1_b717, 0, arena)
    }

    /// 0x51B290(&distance): the distance of the attack point ahead along
    /// the heading (planar for a bomber, 3D with the target otherwise),
    /// then the ability pick (0x51A260). A usable ability with the point at
    /// least 0.9 ahead arms the release: flight +0x98..+0xA0 (a point) or
    /// +0x94 (a target), and +0xA4 the ability. False when no ability can
    /// ever be used (code 3).
    fn release_check(&mut self, arena: &mut Arena, a: ActorId) -> Result<(bool, u32)> {
        let me = self.world.actors[a as usize].pos;
        let heading = Vec2::new(self.world.actors[a as usize].facing.cos(), self.world.actors[a as usize].facing.sin());
        let distance = if self.byte(a, 0x1d0) == 0 {
            let d = Vec2::new(f(self.attack_word(arena, a, 0x24)) - me.x, f(self.attack_word(arena, a, 0x28)) - me.y);
            d.x * heading.x + d.y * heading.y
        } else {
            let t = self.attack_target(arena, a).ok_or_else(|| fail("0x51B290 without a target"))?;
            let tp = self.world.actors[t as usize].pos;
            (tp - me).x * heading.x + (tp - me).y * heading.y
        };
        let target = self.attack_target(arena, a);
        let point = Vec3::new(f(self.attack_word(arena, a, 0x24)), f(self.attack_word(arena, a, 0x28)), f(self.attack_word(arena, a, 0x2c)));
        let (status, ability) = self.valid_ability(a, target, point, false);
        match status {
            0 => {
                if AHEAD <= distance {
                    let actor_target = self.byte(a, off::ATTACK + 0x20) != 0;
                    let g = self.attack_word(arena, a, 0x18);
                    let fl = self.flight_mut(a)?;
                    if actor_target {
                        fl.attack_target = g;
                    } else {
                        fl.attack_point = [point.x, point.y, point.z];
                    }
                    fl.pending_ability = ability.map(|i| aa_movement::aircraft::AbilityKey(i as u32));
                }
                Ok((true, b(distance)))
            }
            3 => Ok((false, b(distance))),
            _ => Ok((true, b(distance))),
        }
    }

    /// 0x4C8F20(context, 1) for ability `i` of `a` at `target` / `point`,
    /// reduced to aa-play's ability values: 1 a target its filter refuses,
    /// 5 out of range, 7 too little mana, 8 recharging, else 0.
    pub(super) fn ability_status(&self, a: ActorId, i: usize, target: Option<ActorId>, point: Vec3) -> u32 {
        let def = self.def(a);
        let w = &def.weapons[i];
        if let Some(t) = target
            && crate::combat::admits(self.world, &self.world.combat, a, w, t) == false
        {
            return 1;
        }
        let me = self.world.actors[a as usize].pos;
        let at = target.map_or(point, |t| self.world.actors[t as usize].pos);
        let d = (at - me).truncate().length();
        if d > w.range {
            return 5;
        }
        let mana = self.world.books.actor(a).and_then(|x| x.aircraft.as_ref()).map_or(0.0, |x| x.mana);
        if mana < w.mana_cost {
            return 7;
        }
        if self.world.actors[a as usize].combat.recharge.get(i).copied().unwrap_or(0.0) > 0.0 {
            return 8;
        }
        0
    }

    /// 0x51A260 GetValidAbility: every ability's 0x4C8F20 outcome as 0 use,
    /// 1 wait (recharging, or out of mana with mana regeneration), 2 move
    /// (out of range, with a Mover), 3 never; the best kept: a usable one
    /// beats the rest, a moving one a waiting one; ties go to the sooner
    /// recharge, then the longer range, then a world draw.
    pub(super) fn valid_ability(&mut self, a: ActorId, target: Option<ActorId>, point: Vec3, bombard_only: bool) -> (u32, Option<usize>) {
        let count = self.def(a).weapons.len();
        let mut best: Option<usize> = None;
        let mut best_status = 3u32;
        for i in 0..count {
            let w = self.def(a).weapons[i].clone();
            if bombard_only && w.flags & 0x8 == 0 {
                continue;
            }
            let s = match self.ability_status(a, i, target, point) {
                0 => 0,
                5 if self.def(a).has_component("MoverComponent") => 2,
                6 | 8 => 1,
                7 => {
                    if self.def(a).mana_regeneration != 0.0 {
                        1
                    } else {
                        3
                    }
                }
                _ => continue,
            };
            let Some(b0) = best else {
                best = Some(i);
                best_status = s;
                continue;
            };
            let same_class = best_status == s || (best_status != 0 && s != 0 && best_status != 2 && s != 2);
            if !same_class {
                // A usable best stays; a usable new one wins; then a moving one.
                if best_status == 0 {
                    continue;
                }
                if s == 0 || (best_status != 2 && s == 2) {
                    best = Some(i);
                    best_status = s;
                }
                continue;
            }
            let pick_new = if s == 2 {
                let (rn, rb) = (w.range, self.def(a).weapons[b0].range);
                rn - rb < 0.0
            } else {
                let left = |k: usize| self.world.actors[a as usize].combat.recharge.get(k).copied().unwrap_or(0.0);
                let diff = left(i) - left(b0);
                if diff > 0.0 {
                    false
                } else if diff < 0.0 {
                    true
                } else {
                    let (rn, rb) = (w.range, self.def(a).weapons[b0].range);
                    if rn < rb {
                        false
                    } else if rn > rb {
                        true
                    } else {
                        self.world.random_u32() & 1 != 0
                    }
                }
            };
            if pick_new {
                best = Some(i);
                best_status = s;
            }
        }
        (best_status, best)
    }
}

impl aai_attack::AirAttackThinkWorld for Host<'_> {
    fn attack_word(&mut self, state: StateId, offset: u32) -> Result<u32> {
        let (a, o) = self.owner(state)?;
        Ok(self.hosted.get(&a).and_then(|h| h.words.get(&(o + offset))).copied().unwrap_or(0))
    }
    fn set_attack_word(&mut self, state: StateId, offset: u32, bits: u32) -> Result<()> {
        let (a, o) = self.owner(state)?;
        if let Some(h) = self.hosted.get_mut(&a) {
            h.words.insert(o + offset, bits);
        }
        Ok(())
    }
    fn attack_byte(&mut self, state: StateId, offset: u32) -> Result<u8> {
        let (a, o) = self.owner(state)?;
        Ok(self.byte(a, o + offset))
    }
    fn set_attack_byte(&mut self, state: StateId, offset: u32, value: u8) -> Result<()> {
        let (a, o) = self.owner(state)?;
        self.set_byte(a, o + offset, value);
        Ok(())
    }
    fn mind_byte(&mut self, mind: MindId, offset: u32) -> Result<u8> {
        Ok(self.byte(mind.0, offset))
    }
    /// 0x55E660: a formation follower; no formation is joined yet.
    fn flight_airborne(&mut self, mind: MindId) -> Result<bool> {
        let fl = self.flight(mind.0)?;
        Ok(fl.group_a.is_some() && fl.on_ground == 0)
    }
    fn flight_reset_28(&mut self, mind: MindId) -> Result<()> {
        self.clear_path(mind.0)
    }
    /// 0x5685B0: the group's first member when it resolves.
    fn group_5685b0(&mut self, group: u32) -> Result<Option<u32>> {
        let first = self.world.books.groups.get(&group).and_then(|m| m.members.first().copied());
        Ok(first.filter(|&m| self.world.actors.get(m as usize).is_some_and(|x| !x.removed)))
    }
    /// The object's Mind virtual +0x4C (0x5189B0): its attack state's +0x18
    /// target while that state runs.
    fn object_mind_4c(&mut self, object: u32) -> Result<Option<u32>> {
        if self.attack_status.get(&object).copied() != Some(2) {
            return Ok(None);
        }
        let g = self.hosted.get(&object).and_then(|h| h.words.get(&(off::ATTACK + 0x18))).copied().unwrap_or(0);
        Ok(self.lookup(g))
    }
    fn attack_518c40(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        let (a, _) = self.owner(state)?;
        self.retarget(arena, a)
    }
    fn attack_51a920(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        let (a, _) = self.owner(state)?;
        self.plan(arena, a)
    }
    fn attack_51b0c0(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        let (a, _) = self.owner(state)?;
        self.lead(arena, a)
    }
    fn attack_51b4c0(&mut self, state: StateId, arena: &mut Arena) -> Result<()> {
        let (a, _) = self.owner(state)?;
        self.phase_step(arena, a)
    }
    fn attack_51b290(&mut self, state: StateId, arena: &mut Arena) -> Result<(bool, u32)> {
        let (a, _) = self.owner(state)?;
        self.release_check(arena, a)
    }
    /// 0x4F0E50(&xy, 0x123): the ground height.
    fn ground_height(&mut self, xy: [u32; 2], _flags: u32) -> Result<Binary> {
        num(self.world.map.height_at(Vec2::new(f(xy[0]), f(xy[1]))))
    }
    fn actor_virtual_18(&mut self, mind: MindId) -> Result<Binary> {
        num(self.speed_now(mind.0))
    }
    fn object_virtual_18(&mut self, object: u32) -> Result<Binary> {
        num(self.speed_now(object))
    }
    fn object_virtual_104(&mut self, object: u32) -> Result<Binary> {
        num(self.def(object).speed_max)
    }
    /// 0x516BA0(x, y, 0) on the move state.
    fn move_to_point(&mut self, mind: MindId, point: [u32; 3], arena: &mut Arena) -> Result<()> {
        self.move_goal(arena, mind.0, point[0], point[1], None).map(|_| ())
    }
}

/// The cruise a Think writes when it reads the half-cruise floor
/// (kept for the dog-fight branches).
#[allow(dead_code)]
const CRUISE_SCALES: [f32; 3] = [HALF, MINUS_HALF, BACKOFF];
