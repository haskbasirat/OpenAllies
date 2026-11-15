//! The element Mind's fighting states on the ports: Engage (Mind +0x12C,
//! `aa_mind::uai_thinks::uai_engage`, `0x536E10`) and Kill (+0xF8,
//! `aa_mind::uai_kill::think`, `0x535240`), with `0x532070` (the regiment hands its
//! target), `0x535910` (the element's choice), `0x5351C0` / `0x533D90`,
//! `ResultAbilityDecision` `0x5307A0`, wait `0x52D820`, pursue `0x52EF40`, and the
//! Performer through `crate::combat::performer`.
//!
//! The choice of target and ability is `combat::member_choice` and `combat::decide`,
//! which follow `0x534700` and `0x5340C0` in f32 with the original's stores. `0x535240`
//! runs the steps of `aa_mind::uai_kill::think` here: its services take no arena, and
//! these need the Mind's.
//!
//! An element outside an organization still fights through `crate::combat`'s
//! `lone_think`, and a thing without a Mover has no Mind to hand a target to.
use super::*;
use crate::combat::{self, TargetRef};
use aa_mind::uai_kill::{self, KillDecision, PerformerState};
use aa_mind::uai_thinks::{result_collision_thing as collision_thing, uai_avoid, uai_engage};

/// Engage +0x10 and +0x18, Kill +0x10, as Mind offsets.
const ENGAGE_TARGET: u32 = offsets::ENGAGE + 0x10;
const RESELECT: u32 = offsets::ENGAGE + 0x18;
const KILL_TARGET: u32 = offsets::KILL + 0x10;
const POINT_SIX_BITS: u32 = 0x3f19_999a;

fn thing_gtid(a: ActorId) -> u32 {
    a + 1
}

/// Mind +0x17C `0x52DB10`: with the Engage state current, choose again.
pub(in crate::movement) fn reconsider(world: &mut World, id: ActorId) {
    if world.books.minds.top_offset(id) == Some(offsets::ENGAGE) {
        world.books.minds.set_byte(id, RESELECT, 1);
    }
}

/// Mind +0xF0 `0x532070`: the regiment's target for this element. A member without a
/// Performer takes none.
pub(in crate::movement) fn engage_event(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, target: TargetRef) -> bool {
    let def = def_of(world, id);
    if def.speed_max <= 0.0 || !def.weapons.iter().any(|w| !w.defender) || !alive(world, id) {
        return false;
    }
    if !combat::target_alive(world, target) {
        return false;
    }
    let g = match target {
        TargetRef::Actor(a) => thing_gtid(a),
        TargetRef::Org(o) => (ORG | o) + 1,
    };
    let (result, unpack, died) = with_mind(ms, world, rules, id, |host, p, a| -> aa_mind::Result<bool> {
        let engage = host.state(offsets::ENGAGE)?;
        let current = host.is_current(a, offsets::ENGAGE)?;
        if current && host.word(ENGAGE_TARGET) == g {
            if !host.world.is_scheduled(Sched::Mind(id)) {
                host.schedule_mind(POINT_SIX_BITS);
            }
            return Ok(true);
        }
        if host.is_current(a, offsets::KILL)? && host.word(KILL_TARGET) == g {
            return Ok(false);
        }
        // 0x52E7B0(2, target).
        host.set_word(ENGAGE_TARGET, g);
        if !aa_mind::activate(engage, 2, a, &mut Stack { host, p })? {
            return Ok(false);
        }
        // 0x53215B: 0x530E50, the abilities' range jitter.
        let word = host.world.random_u32_at(crate::trace::sites::MIND_JITTER);
        host.world.books.actor_mut(id).ability_jitter = combat::mind_jitter(word);
        if let Some(c) = host.world.combat.actors.get_mut(id as usize) {
            c.engage_target = Some(target);
        }
        Ok(true)
    });
    after_mind(ms, world, id, unpack, died);
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("element {id} engage: {e}"));
            false
        }
    }
}

/// Member Mind +0x194 `0x52DBB0` in the regiment's Engage exit.
pub(in crate::movement) fn regiment_left_engage(ms: &mut MovementState, world: &mut World, rules: MoveRules, id: ActorId, a: &mut Arena) {
    if a.mind(MindId(id)).is_err() || !alive(world, id) {
        return;
    }
    let (result, unpack, died) = with_mind_in(ms, world, rules, id, a, |host, p, a| -> aa_mind::Result<()> {
        if host.is_current(a, offsets::ENGAGE)? {
            aa_mind::pop(MindId(id), a, &mut Stack { host, p })?;
        }
        Ok(())
    });
    if let Err(e) = result {
        ms.note(format!("element {id} left engage: {e}"));
    }
    after_mind(ms, world, id, unpack, died);
}

impl Host<'_> {
    fn word(&self, offset: u32) -> u32 {
        self.world.books.minds.word(self.id, offset)
    }
    fn set_word(&mut self, offset: u32, value: u32) {
        self.world.books.minds.set_word(self.id, offset, value);
    }
}

/// State Enter of these states; false for another state.
pub(super) fn enter(stack: &mut Stack<'_, '_>, state: StateId, offset: u32, a: &mut Arena) -> aa_mind::Result<bool> {
    match offset {
        // 0x52D6C0: status 2, choose.
        offsets::ENGAGE => {
            a.state_mut(state)?.status = 2;
            let id = stack.host.id;
            stack.host.world.books.minds.set_byte(id, RESELECT, 1);
        }
        // 0x5166C0.
        offsets::KILL => a.state_mut(state)?.status = 2,
        _ => return Ok(false),
    }
    Ok(true)
}

/// State Exit of these states; false for another state.
pub(super) fn leave(stack: &mut Stack<'_, '_>, state: StateId, offset: u32, a: &mut Arena) -> aa_mind::Result<bool> {
    match offset {
        // 0x536DD0: an active Kill and an active Move leave (0x529C20).
        offsets::ENGAGE => {
            let kill = stack.host.state(offsets::KILL)?;
            if a.state(kill)?.status == 2 {
                aa_mind::World::leave(stack, kill, a)?;
            }
            let mv = stack.host.state(offsets::MOVE)?;
            if a.state(mv)?.status == 2 {
                aa_mind::World::leave(stack, mv, a)?;
            }
            a.state_mut(state)?.status = 0;
            let id = stack.host.id;
            if let Some(c) = stack.host.world.combat.actors.get_mut(id as usize) {
                c.engage_target = None;
            }
            // The state that follows takes the actor's active component (0x4CFC80, a
            // Mover's activation): the Performer is left with its ability unused.
            crate::combat::performer::cancel(&mut stack.host.world.combat, id);
        }
        // 0x533F00: 0x533D90(0), then its status.
        offsets::KILL => {
            set_kill_target(stack, None, a)?;
            a.state_mut(state)?.status = 0;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// `0x533D90(object)` (`aa_mind::uai_thinks::uai_avoid::set_kill_target`): the old target
/// goes (its turrets' other targets with it, the Move leaves), the new one is walked to
/// (`0x532930(0, object, 2, 0, null)`) and, when the element's kingdom sees it, the turrets
/// of the other abilities turn to it (`0x57DD90`).
fn set_kill_target(stack: &mut Stack<'_, '_>, object: Option<ActorId>, a: &mut Arena) -> aa_mind::Result<()> {
    let id = stack.host.id;
    uai_avoid::set_kill_target(MindId(id), object, a, stack)
}

/// The point the object's virtual +0x88 (`0x4EB920`) answers for a kingdom: `None` unless
/// the kingdom knows the object (virtual +0x58 `0x4EE3C0`: visible, or seen and static or
/// remembered); then where the kingdom has it (`0x4EABD0`): its place when visible or a
/// static map object, else the remembered place.
fn seen_point(world: &World, object: ActorId, kingdom: Option<crate::world::KingdomId>) -> Option<Vec2> {
    let here = world.actors.get(object as usize)?.pos.truncate();
    let Some(k) = kingdom else { return Some(here) };
    if !crate::ai::view::knows(world, object, k) {
        return None;
    }
    Some(match crate::fog::sighting(world, object, kingdom) {
        crate::fog::Sighting::Remembered(m) => m.pos.truncate(),
        _ => here,
    })
}

/// `0x57DEA0` on the element's performer: byte +0xC of each of its turret entries (actor
/// +0x1FC of them) goes, the turrets' other targets.
fn performer_halt(world: &mut World, id: ActorId) {
    if let Some(c) = world.combat.actors.get_mut(id as usize) {
        for s in &mut c.turrets {
            s.aim = None;
        }
    }
}

impl uai_avoid::KillSelectWorld for Stack<'_, '_> {
    fn kill_state_target(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(self.host.word(KILL_TARGET))
    }
    fn set_kill_state_target(&mut self, _: MindId, gtid: u32) -> aa_mind::Result<()> {
        let id = self.host.id;
        self.host.set_word(KILL_TARGET, gtid);
        self.host.world.actors[id as usize].combat.target = gtid.checked_sub(1);
        if gtid == 0 {
            self.host.am().chase = None;
        }
        Ok(())
    }
    /// The Mover's falling byte (+0x56).
    fn locomotion_byte56(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(crate::transport::is_falling(self.host.world, self.host.id))
    }
    /// `0x4D4FF0(object, 1, 0)` on the element.
    fn actor_relation(&mut self, _: MindId, object: u32, _: u32, _: u32) -> aa_mind::Result<i32> {
        let host = &self.host;
        Ok(i32::from(!combat::element_can_use(host.world, &host.world.combat, host.id, object)))
    }
    /// `0x4D0D70`: the element leaves the old target's +0x120 list. The game keeps no such
    /// list: who is targeted is read from every actor's target (the combat state's
    /// refresh), which the target word's write changes.
    fn target_released(&mut self, _: u32, _: MindId) -> aa_mind::Result<()> {
        Ok(())
    }
    fn actor_performer(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        Ok(Some(self.host.id))
    }
    fn performer_halt(&mut self, _: u32) -> aa_mind::Result<()> {
        performer_halt(self.host.world, self.host.id);
        Ok(())
    }
    /// `0x532930(mode, object, 2, 0, null)` on the Move.
    fn kill_move_to_object(&mut self, _: MindId, mode: u32, object: u32, a3: u32, a4: u32, point: [u32; 2], a: &mut Arena) -> aa_mind::Result<()> {
        let mv = self.host.state(offsets::MOVE)?;
        let Stack { host, p } = self;
        host.activate_to_object(mv, mode, thing_gtid(object), a3, a4, point, p, a).map(|_| ())
    }
    fn object_is_actor(&mut self, _: MindId, object: u32) -> aa_mind::Result<bool> {
        Ok(object == self.host.id)
    }
    /// `0x4D3A20`: the element joins the new target's +0x120 list (see `target_released`).
    fn target_acquired(&mut self, _: u32, _: MindId) -> aa_mind::Result<()> {
        Ok(())
    }
    /// The element's virtual +0x108 (`0x4D1E80`) is its kingdom (+0xE8); the object's
    /// virtual +0x88 answers whether that kingdom knows it.
    fn kill_in_reach(&mut self, _: MindId, object: u32) -> aa_mind::Result<bool> {
        let world = &*self.host.world;
        Ok(seen_point(world, object, world.actors[self.host.id as usize].kingdom).is_some())
    }
    /// `0x57DD90(1, object, point)` with the point the object's virtual +0x88 left.
    fn kill_approach(&mut self, _: MindId, object: u32) -> aa_mind::Result<()> {
        let id = self.host.id;
        let world = &mut *self.host.world;
        let Some(point) = seen_point(world, object, world.actors[id as usize].kingdom) else { return Ok(()) };
        let mut st = std::mem::take(&mut world.combat);
        // Performer +0x34: the ability in use is left out.
        let active = st.actor(id).and_then(|c| c.performer.active());
        combat::other_turrets_aim(world, &mut st, id, active, Some(object), point);
        world.combat = st;
        Ok(())
    }
}

/// The state's virtual +0xC; `None` for another state.
pub(super) fn think(stack: &mut Stack<'_, '_>, offset: u32, a: &mut Arena) -> Option<aa_mind::Result<()>> {
    let state = match stack.host.state(offset) {
        Ok(s) => s,
        Err(e) => return Some(Err(e)),
    };
    Some(match offset {
        offsets::ENGAGE => uai_engage::think(state, a, stack, PC).map(|_| ()),
        offsets::KILL => kill_think(stack, state, a),
        _ => return None,
    })
}

/// `0x535240`.
fn kill_think(stack: &mut Stack<'_, '_>, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
    let target_gtid = stack.host.word(KILL_TARGET);
    let mut world = KillWorld { stack, a };
    // `think` takes the state's arena apart from the services': they share the one here.
    aa_mind::think_base(state, world.a)?;
    if waiting(&mut world)? {
        return Ok(());
    }
    use uai_kill::KillThinkWorld as K;
    let target = match K::resolve(&mut world, target_gtid)? {
        Some(t) if K::relation(&mut world, t)? == 0 => t,
        _ => return K::target_lost(&mut world, target_gtid),
    };
    let mut record = KillDecision::initial();
    K::choose(&mut world, target, &mut record)?;
    K::apply(&mut world, &mut record)?;
    if waiting(&mut world)? || record.status == 0 {
        return Ok(());
    }
    K::stop(&mut world)
}

fn waiting(world: &mut KillWorld<'_, '_, '_>) -> aa_mind::Result<bool> {
    use uai_kill::KillThinkWorld as K;
    Ok(K::performer(world)?.is_some_and(|s| s.current && s.stage != 4))
}

struct KillWorld<'k, 'h, 'w> {
    stack: &'k mut Stack<'h, 'w>,
    a: &'k mut Arena,
}

impl uai_kill::KillThinkWorld for KillWorld<'_, '_, '_> {
    fn performer(&mut self) -> aa_mind::Result<Option<PerformerState>> {
        let id = self.stack.host.id;
        let running = self.stack.host.world.combat.actor(id).is_some_and(|c| c.performer.running());
        Ok(Some(PerformerState { current: running, stage: if running { 1 } else { 4 } }))
    }
    fn resolve(&mut self, gtid: u32) -> aa_mind::Result<Option<u32>> {
        Ok(gtid.checked_sub(1).filter(|&t| alive(self.stack.host.world, t)))
    }
    /// `0x4D4FF0(target, 1, 0)` on the element.
    fn relation(&mut self, target: u32) -> aa_mind::Result<i32> {
        let host = &self.stack.host;
        Ok(i32::from(!combat::element_can_use(host.world, &host.world.combat, host.id, target)))
    }
    /// `0x5340C0(target, null, &record, 0)`.
    fn choose(&mut self, target: u32, record: &mut KillDecision) -> aa_mind::Result<u32> {
        let id = self.stack.host.id;
        let mut st = std::mem::take(&mut self.stack.host.world.combat);
        let decision = combat::member_decision(self.stack.host.world, &mut st, id, target);
        self.stack.host.world.combat = st;
        record.target = target;
        match decision {
            Some((weapon, status)) => {
                record.ability = weapon as u32;
                record.status = status;
            }
            None => record.status = 3,
        }
        Ok(record.status)
    }
    /// `0x5307A0(&record, 0)`.
    fn apply(&mut self, record: &mut KillDecision) -> aa_mind::Result<()> {
        let id = self.stack.host.id;
        let (weapon, target) = (record.ability as usize, record.target);
        if record.status == 0 {
            let mut st = std::mem::take(&mut self.stack.host.world.combat);
            let started = crate::combat::performer::start(self.stack.host.world, &mut st, id, weapon, target);
            self.stack.host.world.combat = st;
            if started {
                record.status = 1;
                // The Performer is the active component: 0x4CFC80 of the Mover.
                self.stack.host.deactivate();
                self.stack.host.am().chase = None;
                // 0x52ED30 below 0x57E670: a better ability could be used: choose again.
                if combat::better_ability(self.stack.host.world, id, weapon) {
                    reconsider(self.stack.host.world, id);
                }
                // 0x50B250: no Think until the Performer says (0x57DC90).
                if self.stack.host.world.is_scheduled(Sched::Mind(id)) {
                    self.stack.host.world.unschedule(Sched::Mind(id));
                }
                return Ok(());
            }
            record.status = 3;
        }
        match record.status {
            3 => uai_kill::KillThinkWorld::target_lost(self, thing_gtid(target)),
            1 => {
                // 0x52D820: the recharge left below 0.6 s, else 0.6 s (0x52D8A0 -> 0x4CFC80).
                crate::combat::performer::left_current(self.stack.host.world, id);
                let left = self.stack.host.world.actors[id as usize].combat.recharge.get(weapon).copied().unwrap_or(0.0);
                let delay = if left > 0.0 && left < f32::from_bits(POINT_SIX_BITS) { left.to_bits() } else { POINT_SIX_BITS };
                self.stack.host.stop();
                self.stack.host.schedule_mind(delay);
                if combat::better_ability(self.stack.host.world, id, weapon) {
                    reconsider(self.stack.host.world, id);
                }
                Ok(())
            }
            _ => self.pursue(weapon, target),
        }
    }
    fn in_range(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn approach(&mut self, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
    /// `0x57DEA0`: the turrets' other targets go.
    fn stop(&mut self) -> aa_mind::Result<()> {
        performer_halt(self.stack.host.world, self.stack.host.id);
        Ok(())
    }
    /// Mind +0x140 `0x52D9C0` with the Engage current: choose again in 0.6 s.
    fn target_lost(&mut self, _: u32) -> aa_mind::Result<()> {
        let id = self.stack.host.id;
        if self.stack.host.is_current(self.a, offsets::ENGAGE)? {
            self.stack.host.world.books.minds.set_byte(id, RESELECT, 1);
            self.stack.host.schedule_mind(POINT_SIX_BITS);
            return Ok(());
        }
        aa_mind::pop(MindId(id), self.a, self.stack)
    }
}

impl KillWorld<'_, '_, '_> {
    /// `0x52EF40`: the Move thinks; with the Mover the active component it stops at the
    /// ability's range (`0x576AD0`: range less 1e-4, less a step of a target moving the
    /// same way); a target below priority 5 makes it choose again.
    fn pursue(&mut self, weapon: usize, target: u32) -> aa_mind::Result<()> {
        let id = self.stack.host.id;
        let mv = self.stack.host.state(offsets::MOVE)?;
        {
            let Stack { host, p } = &mut *self.stack;
            element_move::think(mv, p, self.a, PC, *host)?;
        }
        let w_range = {
            let host = &self.stack.host;
            let def = def_of(host.world, id);
            def.weapons.get(weapon).map_or(0.0, |w| combat::effective_range(host.world, id, w))
        };
        let mut range = w_range - 1e-4;
        {
            let host = &mut self.stack.host;
            let (fa, ft) = (facing_vector(host.world.actors[id as usize].facing), facing_vector(host.world.actors[target as usize].facing));
            let moving = host.ms.actors.get(target as usize).is_some_and(|m| m.mover.active);
            if moving && fa.dot(ft) > 0.0 {
                let speed = host.ms.actors[target as usize].mover.speed;
                range -= speed * crate::DT;
            }
            host.am().chase = Some((target, range));
        }
        if combat::member_priority(self.stack.host.world, id, target) < 5 {
            reconsider(self.stack.host.world, id);
        }
        Ok(())
    }
}

impl uai_engage::EngageThinkWorld for Stack<'_, '_> {
    fn engage_target(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(self.host.word(ENGAGE_TARGET))
    }
    fn reselect(&mut self, _: StateId) -> aa_mind::Result<bool> {
        Ok(self.host.world.books.minds.byte(self.host.id, RESELECT) != 0)
    }
    fn set_reselect(&mut self, _: StateId, value: bool) -> aa_mind::Result<()> {
        let id = self.host.id;
        self.host.world.books.minds.set_byte(id, RESELECT, u8::from(value));
        Ok(())
    }
    fn organization(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        Ok(self.host.org())
    }
    /// `0x5210D0(target)` on the regiment's Engage state.
    fn regiment_engage_keeps(&mut self, organization: u32, target: u32) -> aa_mind::Result<bool> {
        let t = target_of(target);
        Ok(combat::engage_keeps_thing(self.host.world, &self.host.world.combat, organization, t))
    }
    /// `0x535910(target, 0)`.
    fn engage_select(&mut self, _: StateId, target: Option<u32>, _: &mut Arena) -> aa_mind::Result<Option<u32>> {
        let id = self.host.id;
        let handed = target.map(target_of);
        let mut st = std::mem::take(&mut self.host.world.combat);
        let bad: Vec<ActorId> = self.host.am().bad_targets.iter().map(|b| b.0).collect();
        combat::or_actors(self.host.ms, self.host.world, &mut st, id);
        let chosen = combat::member_choice(self.host.world, &mut st, id, handed, &bad);
        self.host.world.combat = st;
        Ok(chosen)
    }
    fn kill_target(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(self.host.word(KILL_TARGET))
    }
    /// `0x5351C0(mode, object)`.
    fn kill_select(&mut self, mind: MindId, mode: u32, object: u32, a: &mut Arena) -> aa_mind::Result<()> {
        uai_avoid::kill_select(mind, mode, Some(object), a, self).map(|_| ())
    }
    /// `0x52E9D0`: back to the slot, the Move thinks.
    fn return_to_slot(&mut self, _: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        let id = self.host.id;
        let Some(o) = self.host.org() else { return Ok(()) };
        let slot = member_slot(self.host.world, o, id);
        let om = &self.host.ms.orgs[o as usize];
        let Some(offset) = om.slots.get(slot).copied() else { return Ok(()) };
        let goal = om.goal.unwrap_or(self.host.world.orgs[o as usize].anchor);
        let to = goal + offset;
        let mv = self.host.state(offsets::MOVE)?;
        // 0x52E900: 0x52DF60(0, point, 0, 0, null) unless the Move runs.
        if a.state(mv)?.status != 2 {
            let Stack { host, p } = self;
            host.activate_embedded_move(to, 0, p, a)?;
        }
        let Stack { host, p } = self;
        element_move::think(mv, p, a, PC, *host)
    }
    fn world_random(&mut self) -> aa_mind::Result<u32> {
        Ok(self.host.world.random_u32_at(crate::trace::sites::ENGAGE_REACTION))
    }
    fn engage_reaction_range(&mut self) -> aa_mind::Result<u32> {
        Ok(self.host.world.combat.rules.as_deref().map_or(0.3f32, |r| r.engage_reaction_range).to_bits())
    }
}

/// `0x536840` (`aa_mind::uai_thinks::result_collision_thing`): the Move met a thing on
/// its path. With the Engage active and the Kill not, an enemy the element can attack
/// goes to the Kill (`0x5358A0`); with the Kill active it replaces the Kill's target
/// when its `0x5340C0` record is the better one (`0x5351C0`). True when it was taken.
///
/// Every test ahead of the two states' status only reads (the thing's category, the
/// enemy relation, `0x4D4FF0`), and with neither state active the answer is false: that
/// case is answered here without them.
pub(super) fn result_collision_thing(stack: &mut Stack<'_, '_>, thing: u32, a: &mut Arena) -> aa_mind::Result<bool> {
    let minds = &stack.host.world.books.minds;
    let id = stack.host.id;
    let active = |offset: u32| minds.state(id, offset).and_then(|s| a.states.get(&s)).is_some_and(|s| s.status == 2);
    if !active(offsets::ENGAGE) && !active(offsets::KILL) {
        return Ok(false);
    }
    collision_thing::result_collision_thing(Some(thing), &mut CollisionWorld { stack, a }, PC)
}

/// `0x5358A0(object)`: unless the Kill is active on it already, an active Move leaves
/// (`0x529C20`) and the Kill takes it (`0x5351C0(0, object)`).
fn hand_to_kill(stack: &mut Stack<'_, '_>, object: ActorId, a: &mut Arena) -> aa_mind::Result<()> {
    let id = stack.host.id;
    uai_avoid::hand_to_kill(MindId(id), Some(object), a, stack).map(|_| ())
}

struct CollisionWorld<'k, 'h, 'w> {
    stack: &'k mut Stack<'h, 'w>,
    a: &'k mut Arena,
}

impl collision_thing::CollisionThingWorld for CollisionWorld<'_, '_, '_> {
    /// Thing virtual +0xB4: an actor, not an object of the map or an organization.
    fn thing_is_actor(&mut self, thing: u32) -> aa_mind::Result<bool> {
        Ok(thing & ORG == 0 && !space::is_scenery(thing))
    }
    /// Actor virtual +0x16C (`0x4D1F40` -> +0x140): the thing's owner is an enemy.
    fn actor_accepts(&mut self, thing: u32) -> aa_mind::Result<bool> {
        let world = &*self.stack.host.world;
        let kingdom = |t: ActorId| world.actors.get(t as usize).and_then(|x| x.kingdom);
        Ok(crate::setup::relation(world, kingdom(self.stack.host.id), kingdom(thing)) == crate::books::ENEMY)
    }
    /// `0x4D4FF0(thing, 1, 0)` on the element.
    fn relation(&mut self, thing: u32, _: u32, _: u32) -> aa_mind::Result<i32> {
        let host = &self.stack.host;
        Ok(i32::from(!alive(host.world, thing) || !combat::element_can_use(host.world, &host.world.combat, host.id, thing)))
    }
    fn engage_status(&mut self) -> aa_mind::Result<u32> {
        Ok(self.a.state(self.stack.host.state(offsets::ENGAGE)?)?.status)
    }
    fn kill_status(&mut self) -> aa_mind::Result<u32> {
        Ok(self.a.state(self.stack.host.state(offsets::KILL)?)?.status)
    }
    fn kill_target(&mut self) -> aa_mind::Result<u32> {
        Ok(self.stack.host.word(KILL_TARGET))
    }
    fn resolve(&mut self, gtid: u32) -> aa_mind::Result<Option<u32>> {
        Ok(gtid.checked_sub(1).filter(|&t| t & ORG == 0 && alive(self.stack.host.world, t)))
    }
    fn actor_position(&mut self) -> aa_mind::Result<[u32; 2]> {
        Ok(bits(pos2(self.stack.host.world, self.stack.host.id)))
    }
    fn object_position(&mut self, object: u32) -> aa_mind::Result<[u32; 2]> {
        Ok(bits(pos2(self.stack.host.world, object)))
    }
    /// `0x5340C0(target, null, &record, 0)`: a null target and the ROE back-off leave
    /// the record without an ability, as does a target no ability admits; else the best
    /// ability with its score (+0xC), the stored squared distance (+0x14) and the
    /// element's priority for the target (+0x18, Mind virtual +0x1AC).
    fn choose(&mut self, target: Option<u32>, record: &mut KillDecision) -> aa_mind::Result<u32> {
        let Some(t) = target else {
            record.status = 3;
            return Ok(3);
        };
        let id = self.stack.host.id;
        let mut st = std::mem::take(&mut self.stack.host.world.combat);
        let decision = combat::member_record(self.stack.host.world, &mut st, id, t);
        self.stack.host.world.combat = st;
        record.target = t;
        let Some((weapon, status, score)) = decision else {
            record.status = 3;
            return Ok(3);
        };
        let world = &*self.stack.host.world;
        // A handle, never null: the comparison only asks whether there is one.
        record.ability = weapon as u32 + 1;
        record.status = status;
        record.score = score.to_bits();
        record.priority = combat::member_priority(world, id, t);
        use combat::x87::{add, load, mul, store, sub};
        let (s, o) = (world.actors[id as usize].pos, world.actors[t as usize].pos);
        let (dx, dy) = (sub(load(s.x), load(o.x)), sub(load(s.y), load(o.y)));
        record.distance2 = store(add(mul(dx, dx), mul(dy, dy))).to_bits();
        Ok(status)
    }
    fn engage(&mut self, thing: u32) -> aa_mind::Result<()> {
        hand_to_kill(self.stack, thing, self.a)
    }
    fn kill(&mut self, mode: u32, thing: u32) -> aa_mind::Result<()> {
        let id = self.stack.host.id;
        uai_engage::EngageThinkWorld::kill_select(self.stack, MindId(id), mode, thing, self.a)
    }
    /// Mind +8 virtual +4(0, 0).
    fn schedule(&mut self, delay_bits: u32, _: u32) -> aa_mind::Result<()> {
        self.stack.host.schedule_mind(delay_bits);
        Ok(())
    }
}

/// A thing's word in Engage +0x10: an organization's is `ORG | o`.
fn target_of(resolved: u32) -> TargetRef {
    if resolved & ORG != 0 { TargetRef::Org(resolved & !ORG) } else { TargetRef::Actor(resolved) }
}

/// The tail of the Element Tick `0x570B90`, after `0x5709D0`: an element with its Mind
/// asleep (`0x535EA0`) or reattaching (`0x52D780`), not falling (state 24), in an
/// organization and detached has `MoverDetachedSecondsUntilTeleport` to come back;
/// then it stands at its slot (`0x4CF290`). Any other case resets the time.
/// **Rendition:** the free spot is `0x554AD0` (`space::place_near`), not `0x556070`.
pub(in crate::movement) fn detached_tick(ms: &mut MovementState, world: &mut World, _rules: MoveRules, id: ActorId) {
    let org = world.actors[id as usize].org.filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive));
    let idle = world.books.minds.is_hosted(id, HostedKind::Element) && (world.books.minds.top_offset(id) == Some(offsets::SLEEP) || ms.actors[id as usize].reattaching);
    let Some(o) = org.filter(|_| idle && alive(world, id) && !element_attached(world, id)) else {
        ms.actors[id as usize].teleport_at = -1.0;
        return;
    };
    let now = world.time;
    if ms.actors[id as usize].teleport_at <= 0.0 {
        let seconds = world.data.rule_ci("MoverDetachedSecondsUntilTeleport").unwrap_or(10.0);
        ms.actors[id as usize].teleport_at = now + seconds;
        return;
    }
    if now < ms.actors[id as usize].teleport_at {
        return;
    }
    ms.actors[id as usize].teleport_at = -1.0;
    let slot = member_slot(world, o, id);
    let om = &ms.orgs[o as usize];
    let Some(offset) = om.slots.get(slot).copied() else { return };
    let goal = om.goal.unwrap_or(world.orgs[o as usize].anchor);
    let mut point = bits(goal + offset);
    let direction = bits(facing_vector(world.actors[id as usize].facing));
    space::place_near(ms, world, id, &mut point, direction);
    let p = vec2(point);
    let z = world.map.height_at(p);
    world.actors[id as usize].pos = glam::Vec3::new(p.x, p.y, z);
    if let Some(s) = ms.space.as_mut() {
        s.commit(id, [p.x.to_bits(), p.y.to_bits(), z.to_bits()]);
    }
}
