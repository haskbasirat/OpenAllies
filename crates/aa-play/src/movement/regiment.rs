//! The regiment Mind (CAI.cpp) on the exact ports: a `HostedKind::Regiment` Mind per
//! organization on update queue 4 (`Owner::OrgMind`), running `aa_mind::regiment_update`,
//! `cai_sleep`, `aa_navigation::company_move` (`KKC_CAIStateMove`), `cai_thinks::unpack`
//! and `cai_member_command` over [`MovementState`]. Census cause M7.
//!
//! Hosted states: Sleep (Mind+34), Rampage (+54, [`rampage`]), Move (+68), Unpack (+280). The kill order of a
//! regiment is still `order_think` (census X1) and its rout and engage live in
//! `crate::combat`: those reach this Mind as plain moves, or not at all.
//!
//! The Move state's fields stay in [`OrderMove`], which the rest of the module and
//! the element Mind read; [`company`] and [`store`] carry them across a port call.
//!
//! The route solver `0x53E1F0` is [`route`]; only a map whose tile grid cannot take the
//! native search (a test map) goes to `crate::path`, with its cut routes standing in
//! for the region hops of `0x52CAE0`.
use super::*;
use crate::minds::{HostedKind, ORGANIZATION_MIND as ORG};
use aa_mind::{Arena, MindId, StateId, cai_member_command, cai_sleep, cai_thinks, regiment_update};
use aa_navigation::company_move as cm;
use aa_navigation::{OwnedPath, PathNode};
use aa_numeric::Binary;

mod rampage;

const PC: aa_numeric::Precision = PC53;
const SLEEP: u32 = cai_thinks::SLEEP_OFFSET;
const MOVE: u32 = cai_thinks::MOVE_OFFSET;
/// Regiment Mind +0x280, `KKC_CAIStateUnpack`.
const UNPACK: u32 = 0x280;
/// binary32 1e-4, 0.6, 1.0.
const IMMEDIATE_BITS: u32 = 0x38d1_b717;
const POINT_SIX_BITS: u32 = 0x3f19_999a;
const ONE_BITS: u32 = 0x3f80_0000;
/// `+0x10..+0x1C` of the Sleep state, as Mind offsets.
const SLEEP_BYTE10: u32 = SLEEP + 0x10;
const SLEEP_COUNT14: u32 = SLEEP + 0x14;
const SLEEP_BYTE18: u32 = SLEEP + 0x18;
const SLEEP_WORD1C: u32 = SLEEP + 0x1c;
/// Mind +0x2B4 / +0x2B8, the sleep point.
const SLEEP_POINT: u32 = 0x2b4;
/// `+0x10..+0x20` of the Unpack state, as Mind offsets.
const UNPACK_POINTS: u32 = UNPACK + 0x10;
const UNPACK_ATTEMPTS: u32 = UNPACK + 0x20;
/// Site name of the census (`aa_parity::native_trace::getter_group`).
const SCATTER_SITE: &str = "regiment scatter 5233F0";

fn err(what: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(format!("regiment mind: {what}"))
}
fn nav(what: impl std::fmt::Display) -> aa_navigation::Error {
    aa_navigation::Error::World(format!("regiment mind: {what}"))
}
fn bits(v: Vec2) -> [u32; 2] {
    [v.x.to_bits(), v.y.to_bits()]
}
fn vec2(b: [u32; 2]) -> Vec2 {
    Vec2::new(f32::from_bits(b[0]), f32::from_bits(b[1]))
}
fn key(o: OrgId) -> u32 {
    ORG | o
}

struct NoDraws;
impl aa_mind::construct::World for NoDraws {
    fn random_word(&mut self) -> aa_mind::Result<u32> {
        Err(err("a regiment Mind does not draw"))
    }
    fn performer_ability_count(&self, _: u32) -> aa_mind::Result<usize> {
        Ok(0)
    }
    fn set_ability_jitter(&mut self, _: u32, _: usize, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
}

/// `0x4CF480` for an organization, and its first state: Sleep, thinking in 0.6 s (the
/// Mind default `0x51E3A0` at creation; the Berlin film: a regiment created in 1202
/// drains its arrivals at 1212, 1244, 1276).
pub(super) fn ensure(ms: &mut MovementState, world: &mut World, o: OrgId) {
    if world.books.minds.is_hosted(key(o), HostedKind::Regiment) {
        return;
    }
    match aa_mind::construct::construct(aa_mind::construct::Kind::Regiment, key(o), &mut NoDraws, PC) {
        Ok(built) => {
            if let Err(e) = world.books.minds.install(key(o), HostedKind::Regiment, built) {
                ms.note(format!("regiment Mind {o}: {e}"));
                return;
            }
            // 0x51E3A0: a definition with +0x310 `rampages` takes the Rampage in mode 2
            // (an immediate update, 0x52A810), any other Sleep and a wake in 0.6 s.
            let rampages = world.data.def(world.orgs[o as usize].def).rampages;
            let first = if rampages { rampage::RAMPAGE } else { SLEEP };
            let minds = &mut world.books.minds;
            if let Some(state) = minds.state(key(o), first) {
                if let Some(m) = minds.arena.minds.get_mut(&MindId(key(o))) {
                    m.stack.push_front(Some(state));
                }
                if let Some(s) = minds.arena.states.get_mut(&state) {
                    s.status = 2;
                }
            }
            think_in(world, o, if rampages { 0.0 } else { f32::from_bits(POINT_SIX_BITS) });
        }
        Err(e) => ms.note(format!("regiment Mind {o}: {e}")),
    }
}

/// The Load of a saved regiment Mind (`0x528C20`) over `0x4CF480` case 4, and the
/// organization fields the Move state and the rampage read. Nothing is scheduled:
/// the queue Load names the Mind's Updatable.
pub(super) fn restore(ms: &mut MovementState, world: &mut World, o: OrgId, saved: &aa_mind::saved_load::Saved, placed: &crate::map::PlacedThing) -> Result<(), String> {
    let built = aa_mind::construct::construct(aa_mind::construct::Kind::Regiment, key(o), &mut NoDraws, PC).map_err(|e| e.to_string())?;
    world.books.minds.install_saved(key(o), HostedKind::Regiment, built, saved)?;
    if world.books.minds.sleep_unless_hosted(key(o), &[SLEEP, MOVE, UNPACK, rampage::RAMPAGE], SLEEP) {
        ms.note(format!("regiment {o}: the saved Mind state is not hosted, it sleeps"));
    }
    let om = &mut ms.orgs[o as usize];
    om.facing = Vec2::new(placed.facing[0], placed.facing[1]);
    if let Some(p) = placed.organization.as_ref() {
        if let Some(position) = p.position_bits {
            om.goal = Some(vec2(position));
        }
        om.slots = p.offsets.iter().map(|&offset| vec2(offset)).collect();
    }
    let aa_mind::saved_load::Saved::Regiment(regiment) = saved else { return Ok(()) };
    let minds = &world.books.minds;
    let word = |offset: u32| minds.word(key(o), MOVE + offset);
    let moving = minds.state(key(o), MOVE).and_then(|s| minds.arena.states.get(&s)).is_some_and(|s| s.status == 2);
    if moving {
        let nodes = regiment.path.as_ref().map_or_else(Vec::new, |p| p.loaded_nodes());
        let target = thing_actor(world, word(0x10)).map_or(0, |t| t + 1);
        let path = OwnedPath {
            kind1_count: nodes.iter().filter(|n| n.words[4] == 1).count() as i32,
            kind2_count: nodes.iter().filter(|n| n.words[4] == 2).count() as i32,
            nodes,
            dirty: minds.byte(key(o), MOVE + 0x48) != 0,
            word18: word(0x4c),
        };
        let fields = cm::CompanyMoveState {
            status: 2,
            target_id: target,
            path_destination_bits: [word(0x14), word(0x18)],
            destination_bits: [word(0x1c), word(0x20)],
            auxiliary_bits: [word(0x24), word(0x28)],
            speed_bits: word(0x2c),
            tries: word(0x30) as i32,
            path,
            formation_word: word(0x50),
            start_time_bits: word(0x54),
        };
        let order = &mut ms.orgs[o as usize].order;
        store(order, &fields);
        order.active = true;
        order.auxiliary = vec2(fields.auxiliary_bits);
    }
    Ok(())
}

/// The actor a saved GTID names.
fn thing_actor(world: &World, gtid: u32) -> Option<ActorId> {
    match world.ids.thing(gtid) {
        Some(crate::ids::Thing::Actor(a)) => Some(a),
        _ => None,
    }
}

/// Mind+8 virtual +4 (`0x50AD80`).
fn think_in(world: &mut World, o: OrgId, delay: f32) {
    let owner = Sched::OrgMind(o);
    if world.is_scheduled(owner) {
        world.unschedule(owner);
    }
    world.schedule_after(crate::scheduler::THINK, owner, 0, delay);
}

/// The Move state's fields out of [`OrderMove`].
fn company(order: &OrderMove, status: u32) -> cm::CompanyMoveState {
    cm::CompanyMoveState {
        status,
        target_id: order.target.map_or(0, |t| t + 1),
        path_destination_bits: bits(order.path_dest),
        destination_bits: bits(order.destination),
        auxiliary_bits: bits(order.auxiliary),
        speed_bits: order.speed.to_bits(),
        tries: order.tries,
        path: owned_path(order),
        formation_word: order.formation_word,
        start_time_bits: order.start_time.to_bits(),
    }
}

/// The regiment path as the ports hold it.
pub(super) fn owned_path(order: &OrderMove) -> OwnedPath {
    match &order.route {
        Some(route) if route.nodes.len() == order.path.len() => route.clone(),
        _ => {
            let nodes: Vec<PathNode> = order.path.iter().map(|w| element::node_words(w.pos, w.dir, 1)).collect();
            OwnedPath { kind1_count: nodes.len() as i32, nodes, kind2_count: 0, dirty: false, word18: u32::MAX }
        }
    }
}

/// Back into [`OrderMove`].
fn store(order: &mut OrderMove, state: &cm::CompanyMoveState) {
    order.target = (state.target_id != 0).then(|| state.target_id - 1);
    order.path_dest = vec2(state.path_destination_bits);
    order.destination = vec2(state.destination_bits);
    order.speed = f32::from_bits(state.speed_bits);
    order.tries = state.tries;
    order.path.clear();
    order.path.extend(state.path.nodes.iter().map(|n| Waypoint { pos: vec2([n.words[0], n.words[1]]), dir: vec2([n.words[2], n.words[3]]) }));
    order.route = Some(state.path.clone());
    order.formation_word = state.formation_word;
    order.start_time = f32::from_bits(state.start_time_bits);
}

/// One regiment's world. Its Mind's stores stay in `world.books.minds` while a Move
/// port runs (those calls take no arena); [`Regiment::with_arena`] lends them out.
pub(super) struct Regiment<'w> {
    ms: &'w mut MovementState,
    world: &'w mut World,
    rules: MoveRules,
    o: OrgId,
    /// [B3CF58]: the Mind default `0x51E3A0` is running.
    resetting: bool,
}

/// Run `f` on `o`'s Mind with the arena out of its store.
fn with_regiment<T>(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, f: impl FnOnce(&mut Regiment<'_>, &mut Arena) -> T) -> T {
    ensure(ms, world, o);
    let mut arena = std::mem::take(&mut world.books.minds.arena);
    let r = {
        let mut host = Regiment { ms, world, rules, o, resetting: false };
        f(&mut host, &mut arena)
    };
    world.books.minds.arena = arena;
    r
}

impl Regiment<'_> {
    fn state(&self, offset: u32) -> aa_mind::Result<StateId> {
        self.world.books.minds.state(key(self.o), offset).ok_or_else(|| err(format!("Mind+{offset:X} state absent")))
    }
    fn offset_of(&self, state: StateId) -> u32 {
        self.world.books.minds.owner(state).map_or(u32::MAX, |(_, o)| o)
    }
    fn word(&self, offset: u32) -> u32 {
        self.world.books.minds.word(key(self.o), offset)
    }
    fn set_word(&mut self, offset: u32, value: u32) {
        self.world.books.minds.set_word(key(self.o), offset, value);
    }
    fn order(&mut self) -> &mut OrderMove {
        &mut self.ms.orgs[self.o as usize].order
    }
    fn leader(&mut self) -> Option<ActorId> {
        let leader = pick_leader(self.world, self.o);
        self.ms.orgs[self.o as usize].leader = leader;
        leader
    }
    fn extents(&self) -> cm::Extents {
        let m = &self.world.map;
        cm::Extents { extent_bits: bits(Vec2::new(m.width as f32, m.height as f32) * m.cell_size) }
    }
    fn settings(&self) -> cm::ThinkSettings {
        cm::ThinkSettings {
            group_speed_minimum_bits: self.rules.group_speed_minimum.to_bits(),
            min_seconds_before_fail_bits: self.rules.min_seconds_before_fail.to_bits(),
            extents: self.extents(),
        }
    }
    /// Lend the store's arena to `f` (a state change from a Move port's callback).
    fn with_arena<T>(&mut self, f: impl FnOnce(&mut Self, &mut Arena) -> T) -> T {
        let mut arena = std::mem::take(&mut self.world.books.minds.arena);
        let r = f(self, &mut arena);
        self.world.books.minds.arena = arena;
        r
    }
    /// `0x51E9B0`'s destination and `0x523080` run on a copy of the fields.
    fn run_on_fields<T>(&mut self, f: impl FnOnce(&mut cm::CompanyMoveState, &mut Self) -> T) -> T {
        let status = u32::from(self.order().active) * 2;
        let mut fields = company(self.order(), status);
        let generation = self.order().generation;
        let r = f(&mut fields, self);
        // A callback that ended or replaced the state wrote what stands.
        let order = self.order();
        if order.active && order.generation == generation {
            store(order, &fields);
        }
        r
    }
    /// `0x52A000` on the Mind's stack.
    fn is_current(&self, a: &Arena, offset: u32) -> aa_mind::Result<bool> {
        Ok(a.mind(MindId(key(self.o)))?.stack.front().copied().flatten() == Some(self.state(offset)?))
    }
    /// Mind +0x144 `0x5222E0` for the hosted states.
    fn finished(&mut self, a: &mut Arena) -> aa_mind::Result<()> {
        let mind = MindId(key(self.o));
        if self.is_current(a, MOVE)? {
            // 0x52FD00: a live target that is not the regiment itself holds the
            // state: think again in 1 s.
            if let Some(t) = self.order().target.filter(|_| true) {
                if self.world.actor(t).is_some_and(|x| x.alive && !x.removed) {
                    think_in(self.world, self.o, 1.0);
                    return Ok(());
                }
            }
            aa_mind::pop(mind, a, self)?;
            think_in(self.world, self.o, 0.0);
            set_order(self.world, self.ms, Owner::Org(self.o), Order::Idle);
            return Ok(());
        }
        if self.is_current(a, UNPACK)? {
            // The embedded Move leaves, +0xB8, the next update thinks.
            let mv = self.state(MOVE)?;
            if a.state(mv)?.status != 0 {
                aa_mind::World::leave(self, mv, a)?;
            }
            think_in(self.world, self.o, f32::from_bits(IMMEDIATE_BITS));
            return Ok(());
        }
        if self.is_current(a, rampage::RAMPAGE)? {
            // 0x52241E: the embedded Move leaves, the next update thinks.
            let mv = self.state(MOVE)?;
            if a.state(mv)?.status != 0 {
                aa_mind::World::leave(self, mv, a)?;
            }
            think_in(self.world, self.o, f32::from_bits(IMMEDIATE_BITS));
        }
        Ok(())
    }
    /// Mind +0x124 `0x5273D0` for the hosted states: the regiment stops where it
    /// stands; an Unpack that cannot be reached posts `tai_error_unpack`.
    fn move_failed(&mut self, a: &mut Arena) -> aa_mind::Result<()> {
        let mind = MindId(key(self.o));
        if self.is_current(a, rampage::RAMPAGE)? {
            // 0x5274B9: the embedded Move leaves, a think in 1 s.
            let mv = self.state(MOVE)?;
            if a.state(mv)?.status != 0 {
                aa_mind::World::leave(self, mv, a)?;
            }
            think_in(self.world, self.o, f32::from_bits(ONE_BITS));
            return Ok(());
        }
        if self.is_current(a, UNPACK)? {
            refuse(self.world, Owner::Org(self.o), "the deploy spot cannot be reached".into());
        }
        let members: Vec<ActorId> = alive_members(self.world, self.o).collect();
        for m in members {
            let rules = self.rules;
            element::clear_in(self.ms, self.world, rules, m, a);
        }
        aa_mind::pop(mind, a, self)?;
        set_order(self.world, self.ms, Owner::Org(self.o), Order::Idle);
        Ok(())
    }
    /// `0x522890(member, flag)`.
    fn member_command(&mut self, member: ActorId, flag: u8, a: &mut Arena) -> aa_mind::Result<()> {
        let mind = MindId(key(self.o));
        let mut w = MemberCommand { host: self, a };
        cai_member_command::run(&mut w, mind, member, flag, PC)
    }
    /// `0x57BEF0` before the regiment first moved: the table for its heading, its
    /// members as they stand (`reform` of the earlier code, no reassignment).
    fn ensure_slots(&mut self) {
        let o = self.o;
        if !self.ms.orgs[o as usize].slots.is_empty() {
            return;
        }
        let Some(leader) = self.leader() else { return };
        let def = self.world.data.def(self.world.orgs[o as usize].def);
        let facing = facing_vector(self.world.orgs[o as usize].heading);
        let offsets = formation_offsets(def, crate::player_orders::formation(self.world, o));
        let members = slot_members(self.world, o, offsets.len());
        let center = self.world.orgs[o as usize].anchor;
        let slots = formation::organization_slots(&offsets, facing, center, &members, Some((leader, member_slot(self.world, o, leader))), primary_slot(def), &slot_groups(def), false);
        let om = &mut self.ms.orgs[o as usize];
        om.facing = facing;
        om.slots = slots;
    }
}

impl aa_mind::World for Regiment<'_> {
    /// State virtual +4: `0x51D730` (Move), `0x51D650` (Sleep), `0x51D610` (the
    /// others); of what those read only the metadata priorities stand here (all 2,
    /// Sleep 0, the Mind's Move current or not).
    fn can_activate(&mut self, _: StateId, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(true)
    }
    fn enter(&mut self, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        match self.offset_of(state) {
            SLEEP => {
                // 0x51D0F0.
                a.state_mut(state)?.status = 2;
                self.set_word(SLEEP_WORD1C, 0);
                self.world.books.minds.set_byte(key(self.o), SLEEP_BYTE10, 0);
                self.world.books.minds.set_byte(key(self.o), SLEEP_BYTE18, 0);
                self.set_word(SLEEP_COUNT14, 0);
                Ok(())
            }
            MOVE => {
                a.state_mut(state)?.status = 2;
                self.order().active = true;
                self.order().generation += 1;
                let extents = self.extents();
                let mut fields = company(self.order(), 2);
                cm::enter(&mut fields, extents, PC, self).map_err(err)?;
                store(self.order(), &fields);
                Ok(())
            }
            UNPACK => {
                // 0x521BE0: status 2, the embedded Move to the point (0x520440 mode
                // 0), attempts 0.
                a.state_mut(state)?.status = 2;
                let to = vec2([self.word(UNPACK_POINTS), self.word(UNPACK_POINTS + 4)]);
                self.activate_move_fields(to, 0, 0, [0, 0], a)?;
                self.set_word(UNPACK_ATTEMPTS, 0);
                Ok(())
            }
            _ => {
                a.state_mut(state)?.status = 2;
                Ok(())
            }
        }
    }
    fn leave(&mut self, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        match self.offset_of(state) {
            MOVE => {
                // 0x51D7B0.
                a.state_mut(state)?.status = 0;
                let order = self.order();
                order.active = false;
                order.generation += 1;
                order.path.clear();
                order.tries = 0;
                order.capped = false;
                Ok(())
            }
            UNPACK => {
                // 0x51E350: the Move, the Sleep, then its status.
                let mv = self.state(MOVE)?;
                if a.state(mv)?.status != 0 {
                    self.leave(mv, a)?;
                }
                let sleep = self.state(SLEEP)?;
                a.state_mut(sleep)?.status = 0;
                a.state_mut(state)?.status = 0;
                Ok(())
            }
            _ => {
                a.state_mut(state)?.status = 0;
                Ok(())
            }
        }
    }
    fn schedule(&mut self, _: MindId, delay: u32, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        think_in(self.world, self.o, f32::from_bits(delay));
        Ok(())
    }
    /// Mind +0x1A0 `0x51E3A0`: Sleep, a wake in 0.6 s (no pending commands on a
    /// regiment here, no home; the +0x310 flag state is not hosted).
    fn reset_default(&mut self, _: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        if self.resetting {
            return Ok(());
        }
        self.resetting = true;
        // 0x51E405: +0x310 `rampages` takes the Rampage and no wake of its own.
        let rampages = self.world.data.def(self.world.orgs[self.o as usize].def).rampages;
        let state = self.state(if rampages { rampage::RAMPAGE } else { SLEEP })?;
        let r = aa_mind::activate(state, 2, a, self);
        self.resetting = false;
        r?;
        if !rampages {
            think_in(self.world, self.o, f32::from_bits(POINT_SIX_BITS));
        }
        Ok(())
    }
    /// Mind +0x1A4: the view's state.
    fn state_changed(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
}

impl Regiment<'_> {
    /// `0x520440(mode, point, speed, auxiliary)`: past the eligibility, the fields
    /// through `0x51E9B0`, then `0x52A810`.
    fn activate_move_fields(&mut self, to: Vec2, mode: u32, speed: u32, _auxiliary: [u32; 2], a: &mut Arena) -> aa_mind::Result<bool> {
        let extents = self.extents();
        let mut fields = company(self.order(), 0);
        fields.target_id = 0;
        let destination = bits(to);
        cm::set_destination(&mut fields, destination, extents, PC, self).map_err(err)?;
        fields.speed_bits = speed;
        store(self.order(), &fields);
        let mv = self.state(MOVE)?;
        aa_mind::activate(mv, mode, a, self)
    }
}

/// `aa_navigation::company_move` over the regiment.
impl cm::CompanyWorld for Regiment<'_> {
    fn can_activate(&mut self, _: u32, _: &mut cm::CompanyMoveState) -> aa_navigation::Result<bool> {
        Ok(true)
    }
    fn activate(&mut self, mode: u32, state: &mut cm::CompanyMoveState) -> aa_navigation::Result<bool> {
        store(self.order(), state);
        let mv = self.state(MOVE).map_err(nav)?;
        let r = self.with_arena(|host, a| aa_mind::activate(mv, mode, a, host)).map_err(nav);
        let status = u32::from(self.order().active) * 2;
        *state = company(self.order(), status);
        r
    }
    /// `0x51D840`: `crate::movement`'s group offset (census X2).
    fn group_offset(&mut self, _: &mut cm::CompanyMoveState) -> aa_navigation::Result<Option<[u32; 2]>> {
        let offset = self.order().group_offset;
        Ok((offset != Vec2::ZERO).then(|| bits(offset)))
    }
    fn actor_position(&mut self, id: u32) -> aa_navigation::Result<Option<[u32; 2]>> {
        Ok(id.checked_sub(1).and_then(|t| self.world.actor(t)).filter(|x| !x.removed).map(|x| bits(x.pos.truncate())))
    }
    fn set_mind_auxiliary(&mut self, auxiliary: [u32; 2]) -> aa_navigation::Result<()> {
        self.set_word(SLEEP_POINT, auxiliary[0]);
        self.set_word(SLEEP_POINT + 4, auxiliary[1]);
        Ok(())
    }
    fn organization_speed_cap(&mut self) -> aa_navigation::Result<u32> {
        Ok(speed_cap(&mut self.ms.speeds, self.world, Owner::Org(self.o)).to_bits())
    }
    fn member_slots(&mut self) -> aa_navigation::Result<Vec<usize>> {
        Ok(alive_members(self.world, self.o).map(|m| m as usize).collect())
    }
    /// Member Mind +0x64 `0x535E40`: its Move's status is 2.
    fn member_accepts_speed(&mut self, member: usize) -> aa_navigation::Result<bool> {
        Ok(element::move_active(self.ms, self.world, member as ActorId))
    }
    /// Member Mind +0x88 `0x52E610`.
    fn member_set_speed(&mut self, member: usize, speed: u32) -> aa_navigation::Result<()> {
        element::set_speed(self.ms, member as ActorId, speed);
        Ok(())
    }
    fn world_time(&mut self) -> aa_navigation::Result<u32> {
        Ok(self.world.clock.begin.0)
    }
    fn schedule(&mut self, delay: u32) -> aa_navigation::Result<()> {
        think_in(self.world, self.o, f32::from_bits(delay));
        Ok(())
    }
    fn move_failed(&mut self) -> aa_navigation::Result<()> {
        self.with_arena(|host, a| host.move_failed(a)).map_err(nav)
    }
}

impl cm::CompanyThinkWorld for Regiment<'_> {
    fn think_base(&mut self, _: &mut cm::CompanyMoveState) -> aa_navigation::Result<()> {
        let mv = self.state(MOVE).map_err(nav)?;
        self.with_arena(|_, a| aa_mind::think_base(mv, a)).map_err(nav)?;
        Ok(())
    }
    fn is_current(&mut self, _: &mut cm::CompanyMoveState) -> aa_navigation::Result<bool> {
        let mv = self.state(MOVE).map_err(nav)?;
        aa_mind::is_current(mv, &self.world.books.minds.arena).map_err(nav)
    }
    /// Mind +0xB8 `0x51E6C0`: the view's state.
    fn mind_virtual_b8(&mut self) -> aa_navigation::Result<()> {
        Ok(())
    }
    fn target(&mut self, id: u32) -> aa_navigation::Result<Option<cm::Target>> {
        let Some(t) = id.checked_sub(1).and_then(|t| self.world.actor(t)).filter(|x| !x.removed) else {
            return Ok(None);
        };
        Ok(Some(cm::Target { position_bits: bits(t.pos.truncate()), is_self: false }))
    }
    /// Mind +0x140 `0x521F90` with the Move current: the state ends.
    fn target_lost(&mut self, _: u32) -> aa_navigation::Result<()> {
        let mind = MindId(key(self.o));
        self.with_arena(|host, a| aa_mind::pop(mind, a, host)).map_err(nav)?;
        set_order(self.world, self.ms, Owner::Org(self.o), Order::Idle);
        Ok(())
    }
    fn finished(&mut self) -> aa_navigation::Result<()> {
        self.with_arena(|host, a| host.finished(a)).map_err(nav)
    }
    /// Mind +0xF4 `0x51E5D0`: a friendly target with a Portal (nothing here has one).
    fn mind_virtual_f4(&mut self, _: u32) -> aa_navigation::Result<()> {
        Ok(())
    }
    /// (orders) `0x51D7F0`.
    fn group_formation_active(&mut self) -> aa_navigation::Result<bool> {
        Ok(crate::orders::group::in_formation(self.world, Unit::Org(self.o)))
    }
    /// `0x51FBC0`: within `CAITargetInRangeZOCMultiplier` times the zone radius.
    fn target_in_range(&mut self, id: u32) -> aa_navigation::Result<bool> {
        let o = self.o;
        let Some(tp) = id.checked_sub(1).and_then(|t| self.world.actor(t)).map(|x| x.pos.truncate()) else {
            return Ok(false);
        };
        let mut zone = self.world.data.def(self.world.orgs[o as usize].def).control_range;
        if crate::combat::org_state(self.world, o).is_some_and(|c| c.exhausted) {
            zone *= 0.75;
        }
        Ok((self.world.orgs[o as usize].anchor - tp).length() <= self.rules.target_in_range_zoc * zone)
    }
    fn engage_target(&mut self, _: u32) -> aa_navigation::Result<bool> {
        Ok(false)
    }
    fn own_position(&mut self) -> aa_navigation::Result<[u32; 2]> {
        Ok(bits(self.world.orgs[self.o as usize].anchor))
    }
    /// `0x52CAE0`, and where the map has no regions (a test map, whose routes come
    /// from `crate::path`), a cut route whose leader stopped.
    fn path_region_ok(&mut self, state: &mut cm::CompanyMoveState, at: [u32; 2]) -> aa_navigation::Result<bool> {
        if let Some(regions) = self.ms.regions.as_ref()
            && state.path.word18 != u32::MAX
        {
            return Ok(regions.path_region_reached(state.path.word18, at));
        }
        let leader = self.ms.orgs[self.o as usize].leader;
        let moving = leader.is_some_and(|l| self.ms.actors[l as usize].mover.active);
        Ok(self.order().capped && !moving)
    }
    /// Organization +0x40, `0x4CEDE0`.
    fn leader_moving(&mut self) -> aa_navigation::Result<Option<bool>> {
        Ok(self.leader().map(|l| self.ms.actors[l as usize].mover.active))
    }
    /// Organization +0x74, the formation thing.
    fn formation_word(&mut self) -> aa_navigation::Result<u32> {
        Ok(crate::player_orders::formation(self.world, self.o).map_or(1, |f| f as u32 + 2))
    }
    fn pop_pending_member(&mut self) -> aa_navigation::Result<Option<Option<cm::Member>>> {
        let o = self.o;
        let om = &mut self.ms.orgs[o as usize];
        if om.pending.is_empty() {
            return Ok(None);
        }
        let m = om.pending.remove(0);
        Ok(Some(alive(self.world, m).then(|| self.member(m))))
    }
    /// (orders) `0x44DEA0` on the actor's +164.
    fn group_handle_valid(&mut self) -> aa_navigation::Result<bool> {
        Ok(crate::orders::group::in_group(self.world, Unit::Org(self.o)))
    }
    fn prepare_leader_query(&mut self) -> aa_navigation::Result<()> {
        Ok(())
    }
    fn leader_position(&mut self) -> aa_navigation::Result<[u32; 2]> {
        let leader = self.leader().ok_or_else(|| nav("no leader"))?;
        Ok(bits(pos2(self.world, leader)))
    }
    fn mind_virtual_98(&mut self) -> aa_navigation::Result<u32> {
        Ok(0)
    }
    fn state_220_current(&mut self) -> aa_navigation::Result<bool> {
        Ok(false)
    }
    fn owner_flag_76b9a0(&mut self) -> aa_navigation::Result<bool> {
        Ok(false)
    }
    /// `0x53E1F0`, on the map's regions and tile lists ([`route`]). A map whose tile
    /// grid cannot take the native search goes to the tile solver (`crate::path`).
    fn solve(&mut self, path: &mut OwnedPath, request: cm::SolveRequest) -> aa_navigation::Result<bool> {
        let leader = self.leader().ok_or_else(|| nav("no leader"))?;
        let class = mover_class(def_of(self.world, leader));
        let max_speed = max_speed(&mut self.ms.speeds, self.world, leader);
        let ignore = request.ignore_target.map(|t| t.wrapping_add(1));
        if let Some(query) = route::query_of(self.ms, self.world, leader, ignore, request.query_word28) {
            let penalty = self.rules.path.terrain_change_penalty;
            if let Some(status) = route::solve(self.ms, self.world, query, request.from_bits, request.to_bits, request.mode, max_speed, penalty, path) {
                self.order().capped = false;
                return Ok(status != 0);
            }
        }
        let route_request = Request { from: vec2(request.from_bits), to: vec2(request.to_bits), class, max_speed };
        let rules = self.rules.path;
        let map = self.world.map.clone();
        let route = self.ms.solve(&map, &rules, &route_request);
        if route.status == Status::Failed {
            return Ok(false);
        }
        self.order().capped = route.status == Status::Capped;
        path.nodes.clear();
        for w in &route.waypoints {
            path.nodes.push(element::node_words(w.pos, w.dir, 1));
        }
        path.kind1_count = path.nodes.len() as i32;
        path.kind2_count = 0;
        path.dirty = true;
        Ok(true)
    }
    fn clear_owner_state_16(&mut self) -> aa_navigation::Result<()> {
        Ok(())
    }
    /// `0x4EEBE0` on the organization and `0x57BEF0` go together in `set_org_facing`.
    fn set_organization_facing(&mut self, facing: [u32; 2]) -> aa_navigation::Result<()> {
        self.leader();
        let f = vec2(facing);
        if f != Vec2::ZERO {
            set_org_facing(self.ms, self.world, self.o, f);
        }
        Ok(())
    }
    fn compute_slot_positions(&mut self) -> aa_navigation::Result<()> {
        self.ensure_slots();
        Ok(())
    }
    /// The byte at Formation +0xC -> +0x18: `organized`, else the members scatter
    /// (`0x5233F0`).
    fn formation_kind(&mut self) -> aa_navigation::Result<cm::FormationKind> {
        let def = self.world.data.def(self.world.orgs[self.o as usize].def);
        let organized = crate::player_orders::formation(self.world, self.o).and_then(|f| def.formations.get(f)).is_none_or(|f| f.organized);
        Ok(if organized { cm::FormationKind::Oriented } else { cm::FormationKind::Scatter })
    }
    /// `0x4364C0` on the slot array (+0x28), which holds a member at its slot.
    fn next_member(&mut self, after: Option<usize>) -> aa_navigation::Result<Option<cm::Member>> {
        let o = self.o;
        let next = alive_members(self.world, o).map(|m| (member_slot(self.world, o, m), m)).filter(|&(s, _)| after.is_none_or(|a| s > a)).min();
        Ok(next.map(|(_, m)| self.member(m)))
    }
    fn slot_offset(&mut self, slot: i32) -> aa_navigation::Result<[u32; 2]> {
        Ok(bits(self.ms.orgs[self.o as usize].slots.get(slot as usize).copied().unwrap_or(Vec2::ZERO)))
    }
    fn own_facing(&mut self) -> aa_navigation::Result<[u32; 2]> {
        Ok(bits(self.ms.orgs[self.o as usize].facing))
    }
    /// Member Mind +0x100 `0x532170`.
    fn member_move_path(&mut self, member: cm::Member, path: &OwnedPath, speed: u32, _: [u32; 2]) -> aa_navigation::Result<()> {
        self.give(member, path, 0, speed);
        Ok(())
    }
    /// Member Mind +0x10C `0x532310`.
    fn member_formation_path(&mut self, member: cm::Member, path: &OwnedPath, speed: u32, _: [u32; 2]) -> aa_navigation::Result<()> {
        self.give(member, path, 3, speed);
        Ok(())
    }
    /// `0x4CA760`, `0x4D9B60`, `0x4CA2D0`.
    fn member_order(&mut self, member: cm::Member, point: [u32; 2], speed: u32, auxiliary: [u32; 2]) -> aa_navigation::Result<()> {
        let rules = self.rules;
        let id = self.member_id(member);
        let mut arena = std::mem::take(&mut self.world.books.minds.arena);
        element::deliver_move(self.ms, self.world, rules, id, point, speed, auxiliary, &mut arena);
        self.world.books.minds.arena = arena;
        Ok(())
    }
    /// `0x554AD0(point, direction, definition, 3, 0)`: a free place near the point.
    fn place_near(&mut self, member: cm::Member, point: &mut [u32; 2], direction: [u32; 2]) -> aa_navigation::Result<()> {
        let id = self.member_id(member);
        space::place_near(self.ms, self.world, id, point, direction);
        Ok(())
    }
    fn next_random(&mut self) -> aa_navigation::Result<u32> {
        Ok(self.world.random_u32_at(SCATTER_SITE))
    }
}

impl Regiment<'_> {
    fn member(&mut self, m: ActorId) -> cm::Member {
        let o = self.o;
        let slot = member_slot(self.world, o, m);
        let is_leader = self.ms.orgs[o as usize].leader == Some(m);
        cm::Member {
            index: slot,
            is_leader,
            attached: element::element_attached(self.world, m),
            slot: slot as i32,
            position_bits: bits(pos2(self.world, m)),
            in_this_organization: self.world.actors[m as usize].org == Some(o),
        }
    }
    fn member_id(&self, member: cm::Member) -> ActorId {
        let o = self.o;
        alive_members(self.world, o).find(|&m| member_slot(self.world, o, m) == member.index).unwrap_or(u32::MAX)
    }
    fn give(&mut self, member: cm::Member, path: &OwnedPath, mode: u8, speed: u32) {
        let id = self.member_id(member);
        if !alive(self.world, id) {
            return;
        }
        if element::enabled() {
            let rules = self.rules;
            element::give_owned_path(self.ms, self.world, rules, id, path, mode, speed);
            return;
        }
        let nodes = path.nodes.iter().map(|n| Node { pos: vec2([n.words[0], n.words[1]]), dir: vec2([n.words[2], n.words[3]]), kind: n.words[4] as u8 }).collect();
        start_element(self.ms, self.world, id, nodes, mode, f32::from_bits(speed));
    }
}

/// `cai_member_command`: `0x522890` over the organization and its zone 4.
struct MemberCommand<'c, 'w> {
    host: &'c mut Regiment<'w>,
    a: &'c mut Arena,
}
impl cai_member_command::World for MemberCommand<'_, '_> {
    type Organization = OrgId;
    type Tile = Vec2;
    type Zone = OrgId;
    fn organization(&mut self, _: MindId) -> aa_mind::Result<OrgId> {
        Ok(self.host.o)
    }
    fn leader_position(&mut self, _: OrgId) -> aa_mind::Result<[u32; 2]> {
        let leader = self.host.leader().ok_or_else(|| err("no leader"))?;
        Ok(bits(pos2(self.host.world, leader)))
    }
    /// `0x57C680`.
    fn relocate_current_organization(&mut self, _: MindId, position: [u32; 2]) -> aa_mind::Result<()> {
        let rules = self.host.rules;
        set_goal(self.host.ms, self.host.world, self.host.o, vec2(position), &rules);
        Ok(())
    }
    /// Organization +0xBC / +0xC0.
    fn formation_origin(&mut self, o: OrgId) -> aa_mind::Result<[u32; 2]> {
        let anchor = self.host.world.orgs[o as usize].anchor;
        Ok(bits(self.host.ms.orgs[o as usize].goal.unwrap_or(anchor)))
    }
    fn member_slot(&mut self, member: u32) -> aa_mind::Result<i32> {
        Ok(member_slot(self.host.world, self.host.o, member) as i32)
    }
    fn current_slot_offset(&mut self, _: MindId, slot: i32) -> aa_mind::Result<[u32; 2]> {
        self.host.ensure_slots();
        Ok(bits(self.host.ms.orgs[self.host.o as usize].slots.get(slot as usize).copied().unwrap_or(Vec2::ZERO)))
    }
    fn tile(&mut self, position: [u32; 2]) -> aa_mind::Result<Option<Vec2>> {
        let p = vec2(position);
        Ok(self.host.world.map.cell_of(p).map(|_| p))
    }
    fn current_zone4(&mut self, _: MindId) -> aa_mind::Result<Option<OrgId>> {
        Ok(Some(self.host.o))
    }
    /// `0x54EF40`: the tile holds the organization's zone 4.
    fn contains_zone(&mut self, tile: Vec2, o: OrgId) -> aa_mind::Result<bool> {
        Ok(self.host.world.fog.engine.as_ref().is_none_or(|e| e.tile_holds_org_zone(self.host.world, tile, o)))
    }
    fn member_position(&mut self, member: u32) -> aa_mind::Result<[u32; 2]> {
        Ok(bits(pos2(self.host.world, member)))
    }
    fn member_active_4cede0(&mut self, member: u32) -> aa_mind::Result<bool> {
        Ok(self.host.ms.actors[member as usize].mover.active)
    }
    /// Member Mind +0x80 `0x535EE0`: where its Move goes, else where it stands.
    fn member_target_position(&mut self, member: u32) -> aa_mind::Result<[u32; 2]> {
        Ok(element::move_destination(self.host.ms, member).unwrap_or_else(|| bits(pos2(self.host.world, member))))
    }
    fn current_sleep_point(&mut self, _: MindId) -> aa_mind::Result<[u32; 2]> {
        Ok([self.host.word(SLEEP_POINT), self.host.word(SLEEP_POINT + 4)])
    }
    fn issue_stop(&mut self, member: u32) -> aa_mind::Result<()> {
        let rules = self.host.rules;
        element::deliver_stop(self.host.ms, self.host.world, rules, member, self.a);
        Ok(())
    }
    fn issue_move(&mut self, member: u32, destination: [u32; 2], sleep: [u32; 2]) -> aa_mind::Result<()> {
        let rules = self.host.rules;
        element::deliver_move(self.host.ms, self.host.world, rules, member, destination, 0, sleep, self.a);
        Ok(())
    }
}

impl cai_sleep::SleepThinkWorld for Regiment<'_> {
    /// `0x578140`: nothing here is exhausted through this Mind (`crate::combat`).
    fn organization_status(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<u32> {
        Ok(2)
    }
    fn exhausted_reaction(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn resolve(&mut self, gtid: u32) -> aa_mind::Result<Option<u32>> {
        Ok(gtid.checked_sub(1).filter(|&a| alive(self.world, a)))
    }
    fn object_removed(&mut self, object: u32) -> aa_mind::Result<bool> {
        Ok(!alive(self.world, object))
    }
    fn organization_follows(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn mind_virtual_188(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn mind_virtual_190(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn follow_activate(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn embedded_think(&mut self, _: MindId, offset: u32, a: &mut Arena) -> aa_mind::Result<()> {
        think_state(self, offset, a)
    }
    fn organization_4d7960(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn home_component(&mut self, _: u32) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn home_settle(&mut self, _: u32, _: u32, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
    fn mind_virtual_f4(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn take_arrival(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        let om = &mut self.ms.orgs[self.o as usize];
        Ok((!om.pending.is_empty()).then(|| om.pending.remove(0) + 1))
    }
    fn peek_arrival(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<Option<u32>> {
        Ok(self.ms.orgs[self.o as usize].pending.first().map(|&m| m + 1))
    }
    fn remove_arrival(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<()> {
        let om = &mut self.ms.orgs[self.o as usize];
        if !om.pending.is_empty() {
            om.pending.remove(0);
        }
        Ok(())
    }
    fn member_organization(&mut self, object: u32) -> aa_mind::Result<u32> {
        Ok(self.world.actors[object as usize].org.map_or(u32::MAX, |o| o))
    }
    fn organization(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<u32> {
        Ok(self.o)
    }
    /// `0x522890(object, 0)`.
    fn member_arrived(&mut self, _: MindId, object: u32, a: &mut Arena) -> aa_mind::Result<()> {
        self.member_command(object, 0, a)
    }
    fn sleep_point(&mut self, _: MindId) -> aa_mind::Result<[u32; 2]> {
        Ok([self.word(SLEEP_POINT), self.word(SLEEP_POINT + 4)])
    }
    fn set_sleep_point(&mut self, _: MindId, point: [u32; 2]) -> aa_mind::Result<()> {
        self.set_word(SLEEP_POINT, point[0]);
        self.set_word(SLEEP_POINT + 4, point[1]);
        Ok(())
    }
    fn null_point(&mut self) -> aa_mind::Result<[u32; 2]> {
        Ok([0, 0])
    }
    /// `0x57BF30`.
    fn organization_move(&mut self, _: MindId, point: [u32; 2], _: &mut Arena) -> aa_mind::Result<()> {
        self.leader();
        let f = vec2(point);
        if f != Vec2::ZERO {
            set_org_facing(self.ms, self.world, self.o, f);
        }
        Ok(())
    }
    /// `0x520120`: a member's Mind +0x78 (`0x535E70`).
    fn organization_busy(&mut self, _: StateId, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn resume_pending(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn organization_flag310(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn embedded_state(&mut self, _: MindId, offset: u32) -> aa_mind::Result<StateId> {
        self.state(offset)
    }
    /// Organization +0x74.
    fn organization_word74(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<u32> {
        cm::CompanyThinkWorld::formation_word(self).map_err(err)
    }
    fn sleep_word1c(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(self.word(SLEEP_WORD1C))
    }
    fn set_sleep_word1c(&mut self, _: StateId, value: u32) -> aa_mind::Result<()> {
        self.set_word(SLEEP_WORD1C, value);
        Ok(())
    }
    fn sleep_byte10(&mut self, _: StateId) -> aa_mind::Result<bool> {
        Ok(self.world.books.minds.byte(key(self.o), SLEEP_BYTE10) != 0)
    }
    /// `0x522F60`: +0x14 counts it; each member `0x522890(member, +0x18)`, but for the
    /// unit of a current Unpack (Mind +0x28C, `0x570650`). A member a building or a
    /// truck (no Mind to order) or one fighting (`crate::combat`'s, rendition) is left.
    fn sleep_522f60(&mut self, _: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        let count = self.word(SLEEP_COUNT14).wrapping_add(1);
        self.set_word(SLEEP_COUNT14, count);
        let flag = self.world.books.minds.byte(key(self.o), SLEEP_BYTE18);
        let o = self.o;
        // The kill order of `order_think` keeps its members.
        if self.ms.orgs[o as usize].order.active {
            return Ok(());
        }
        let unpack = self.state(UNPACK)?;
        let unpacking = a.state(unpack)?.status == 2;
        let leader = self.ms.orgs[o as usize].leader;
        let mut members: Vec<(usize, ActorId)> = alive_members(self.world, o).map(|m| (member_slot(self.world, o, m), m)).collect();
        members.sort_unstable();
        for (_, m) in members {
            if unpacking && Some(m) == leader {
                continue;
            }
            if def_of(self.world, m).speed_max <= 0.0 || self.world.actors[m as usize].combat.target.is_some() {
                continue;
            }
            self.member_command(m, flag, a)?;
        }
        Ok(())
    }
    /// `0x523000`: `0x5200B0` and `0x520190` hold; below two rounds `0x522F60`, else
    /// the byte +0x10 clears.
    fn sleep_523000(&mut self, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
        if self.word(SLEEP_COUNT14) < 2 {
            return self.sleep_522f60(state, a);
        }
        self.world.books.minds.set_byte(key(self.o), SLEEP_BYTE10, 0);
        Ok(())
    }
}

impl cai_thinks::RegimentWorld for Regiment<'_> {
    fn resolve(&mut self, gtid: u32) -> aa_mind::Result<Option<u32>> {
        cai_sleep::SleepThinkWorld::resolve(self, gtid)
    }
    fn object_gtid(&mut self, object: u32) -> aa_mind::Result<u32> {
        Ok(object + 1)
    }
    fn embedded_state(&mut self, _: MindId, offset: u32) -> aa_mind::Result<StateId> {
        self.state(offset)
    }
    fn embedded_think(&mut self, _: MindId, offset: u32, a: &mut Arena) -> aa_mind::Result<()> {
        think_state(self, offset, a)
    }
    fn mind_virtual_140(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn mind_virtual_b8(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
}
impl cai_thinks::stack_action::ActionWorld for Regiment<'_> {
    fn action_definition(&mut self, ordinal: u32) -> aa_mind::Result<u32> {
        Ok(ordinal + 1)
    }
    /// `0x4D9B60(unit)` -> `0x52C320` -> Mind +0xE8 `0x530960`: `0x530440(2, point,
    /// facing)`, the unit's Unpack.
    fn issue_action(&mut self, receiver: u32, action: &cai_thinks::stack_action::ActionCommand, a: &mut Arena) -> aa_mind::Result<bool> {
        if action.ordinal != cai_thinks::stack_action::UNPACK_ACTION {
            return Err(err(format!("action {} is not hosted", action.ordinal)));
        }
        let rules = self.rules;
        let at = vec2([action.words18_28[0], action.words18_28[1]]);
        Ok(element::begin_unpack_in(self.ms, self.world, rules, receiver, at, a))
    }
    fn release_action(&mut self, _: &cai_thinks::stack_action::ActionCommand, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
}
impl cai_thinks::unit_order::UnitOrderWorld for Regiment<'_> {
    /// Organization +0x44, the unit.
    fn component_unit(&mut self, _: MindId) -> aa_mind::Result<Option<u32>> {
        Ok(self.leader())
    }
    /// `0x4D1080`: the unit has unpacked (actor +0x104 bit 2).
    fn unit_4d1080(&mut self, unit: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(self.world.data.def(self.world.actors[unit as usize].def).unpacks_to.is_none())
    }
    fn organization_name(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<String> {
        Ok(String::new())
    }
    fn display_string(&mut self, _: &str, _: &mut Arena) -> aa_mind::Result<String> {
        Ok(String::new())
    }
    fn organization_owner(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<u32> {
        Ok(self.world.orgs[self.o as usize].kingdom)
    }
    /// `tai_error_unpack`.
    fn post_message(&mut self, _: MindId, _: u32, _: u32, _: &str, _: &str, _: &mut Arena) -> aa_mind::Result<()> {
        refuse(self.world, Owner::Org(self.o), "the deploy spot cannot be used".into());
        Ok(())
    }
}
impl cai_thinks::unpack::UnpackThinkWorld for Regiment<'_> {
    fn unit_mind(&mut self, unit: u32) -> aa_mind::Result<Option<u32>> {
        Ok(Some(unit))
    }
    /// Unit Mind +0x7C `0x535E80`: its Unpack's status is 2.
    fn unit_mind_virtual_7c(&mut self, unit: u32, a: &mut Arena) -> aa_mind::Result<bool> {
        let Some(state) = self.world.books.minds.state(unit, aa_mind::uai_thinks::element::UNPACK) else { return Ok(false) };
        Ok(a.state(state)?.status == 2)
    }
    /// `0x4D1050`: packing or unpacking.
    fn unit_4d1050(&mut self, unit: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        // The building an unpack made is unpacking until its construction ends
        // (state 26, st 0x4001000; `0x4D21A0(1)`).
        Ok(self.world.books.actor(unit).is_some_and(|b| b.deploy.is_some() || b.constructing))
    }
    fn unpack_points(&mut self, _: StateId) -> aa_mind::Result<[u32; 4]> {
        Ok([self.word(UNPACK_POINTS), self.word(UNPACK_POINTS + 4), self.word(UNPACK_POINTS + 8), self.word(UNPACK_POINTS + 12)])
    }
    fn unpack_attempts(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(self.word(UNPACK_ATTEMPTS))
    }
    fn set_unpack_attempts(&mut self, _: StateId, value: u32) -> aa_mind::Result<()> {
        self.set_word(UNPACK_ATTEMPTS, value);
        Ok(())
    }
    /// Mind +0x1B0 `0x51D460`: the state ends (with the flag, `0x4D64F0`, the view).
    fn mind_virtual_1b0(&mut self, _: MindId, _: u32, a: &mut Arena) -> aa_mind::Result<()> {
        let mind = MindId(key(self.o));
        aa_mind::pop(mind, a, self)?;
        set_order(self.world, self.ms, Owner::Org(self.o), Order::Idle);
        Ok(())
    }
}

/// The state's virtual +0xC.
fn think_state(host: &mut Regiment<'_>, offset: u32, a: &mut Arena) -> aa_mind::Result<()> {
    let state = host.state(offset)?;
    match offset {
        SLEEP => cai_sleep::think(state, a, host).map(|_| ()),
        MOVE => {
            // The store's arena is what the Move port's callbacks use.
            std::mem::swap(&mut host.world.books.minds.arena, a);
            let result = think_move(host);
            std::mem::swap(&mut host.world.books.minds.arena, a);
            result
        }
        UNPACK => cai_thinks::unpack::think(state, a, host).map(|_| ()),
        rampage::RAMPAGE => rampage::think(host, state, a),
        other => Err(err(format!("Mind+{other:X} state Think is not hosted"))),
    }
}

/// `0x5276B0` (`aa_navigation::company_move::think`).
fn think_move(host: &mut Regiment<'_>) -> aa_mind::Result<()> {
    let o = host.o;
    let now_leader = host.leader();
    // A cut route that got nowhere since the last one is no hop: it counts as a failed
    // try (0x526940).
    if let Some(l) = now_leader {
        let moving = host.ms.actors[l as usize].mover.active;
        let at = pos2(host.world, l);
        let order = &mut host.ms.orgs[o as usize].order;
        if order.capped && !moving {
            if order.capped_from.is_some_and(|p| (p - at).length_squared() < ARRIVAL_RADIUS_SQUARED) {
                order.capped = false;
                order.path.clear();
            }
            order.capped_from = Some(at);
        }
    }
    let settings = host.settings();
    host.run_on_fields(|fields, host| cm::think(fields, settings, PC, host).map_err(err))
}

/// `aa_mind::regiment_update::World`: `0x524910`.
struct Update<'u, 'w> {
    host: &'u mut Regiment<'w>,
    a: &'u mut Arena,
}
impl regiment_update::BaseWorld for Update<'_, '_> {
    type Actor = OrgId;
    type Kingdom = KingdomId;
    fn actor(&mut self, _: MindId) -> aa_mind::Result<OrgId> {
        Ok(self.host.o)
    }
    fn actor_has_state10(&mut self, _: OrgId) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn current_owner108(&mut self, _: OrgId) -> aa_mind::Result<Option<KingdomId>> {
        Ok(None)
    }
    fn kingdom_byte239(&mut self, _: KingdomId) -> aa_mind::Result<u8> {
        Ok(0)
    }
    fn attached_to_organization(&mut self, _: OrgId) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn current_home_4d11a0(&mut self, _: OrgId) -> aa_mind::Result<bool> {
        Ok(true)
    }
    fn mind_virtual1a8(&mut self, _: MindId) -> aa_mind::Result<()> {
        Ok(())
    }
    fn timed_list_4d37e0(&mut self, _: OrgId) -> aa_mind::Result<()> {
        Ok(())
    }
}
impl regiment_update::World for Update<'_, '_> {
    type State = u32;
    fn actor_flags(&mut self, o: OrgId) -> aa_mind::Result<u32> {
        Ok(u32::from(!self.host.world.orgs[o as usize].alive))
    }
    fn member_count_57a730(&mut self, o: OrgId) -> aa_mind::Result<u32> {
        Ok(alive_members(self.host.world, o).count() as u32)
    }
    /// `0x523EA0`: the engage list is `crate::combat`'s.
    fn refresh_engage(&mut self, _: MindId) -> aa_mind::Result<()> {
        Ok(())
    }
    fn current_state(&mut self, mind: MindId) -> aa_mind::Result<u32> {
        let state = self.a.mind(mind)?.stack.front().copied().flatten().ok_or_else(|| err("the Mind has no current state"))?;
        Ok(self.host.offset_of(state))
    }
    fn think(&mut self, offset: u32) -> aa_mind::Result<()> {
        think_state(self.host, offset, self.a)
    }
    fn updatable_head_present(&mut self, _: MindId) -> aa_mind::Result<bool> {
        Ok(self.host.world.is_scheduled(Sched::OrgMind(self.host.o)))
    }
    fn schedule(&mut self, _: MindId, delay: u32, _: u32) -> aa_mind::Result<()> {
        think_in(self.host.world, self.host.o, f32::from_bits(delay));
        Ok(())
    }
    /// `0x763B90`: the arrivals were drained by the Sleep, or wait for it.
    fn clear_arrivals(&mut self, _: MindId) -> aa_mind::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Entry points for movement.rs.

/// The queue-4 delivery.
pub(super) fn update(ms: &mut MovementState, world: &mut World, o: OrgId, rules: MoveRules) {
    if !world.orgs[o as usize].alive {
        return;
    }
    let result = with_regiment(ms, world, rules, o, |host, a| {
        let mut w = Update { host, a };
        regiment_update::update(&mut w, MindId(key(o)))
    });
    if let Err(e) = result {
        ms.note(format!("regiment {o} update: {e}"));
    }
}

/// `0x520440(2, point, speed, auxiliary)` from the Move command `0x527E30`, or the
/// orders of `apply_order`. False when the Mind refused it.
pub(super) fn activate_move(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId) -> bool {
    let result = with_regiment(ms, world, rules, o, |host, a| {
        let to = host.order().destination;
        let speed = host.order().speed;
        host.activate_move_fields(to, 2, speed.to_bits(), [0, 0], a)
    });
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("regiment {o} move: {e}"));
            false
        }
    }
}

/// `0x520320(2, target, speed, auxiliary)`, the Move command at an actor: the target's
/// id, `0x51E9B0` at its position, then `0x52A810`. False when the Mind refused it.
pub(super) fn activate_to_actor(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, target: ActorId) -> bool {
    let result = with_regiment(ms, world, rules, o, |host, a| {
        let Some(position) = host.world.actor(target).filter(|t| !t.removed).map(|t| bits(t.pos.truncate())) else {
            return Ok(false);
        };
        let speed = host.order().speed.to_bits();
        let extents = host.extents();
        let mut fields = company(host.order(), 0);
        fields.target_id = target + 1;
        cm::set_destination(&mut fields, position, extents, PC, host).map_err(err)?;
        fields.speed_bits = speed;
        store(host.order(), &fields);
        let mv = host.state(MOVE)?;
        aa_mind::activate(mv, 2, a, host)
    });
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("regiment {o} move to actor: {e}"));
            false
        }
    }
}

/// `0x51E2E0(2, point, direction)` from the unpack command `0x51E550`.
pub(super) fn activate_unpack(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, at: Vec2, facing: f32) -> bool {
    let result = with_regiment(ms, world, rules, o, |host, a| {
        let point = bits(at);
        let f = facing_vector(facing);
        host.set_word(UNPACK_POINTS, point[0]);
        host.set_word(UNPACK_POINTS + 4, point[1]);
        host.set_word(UNPACK_POINTS + 8, f.x.to_bits());
        host.set_word(UNPACK_POINTS + 12, f.y.to_bits());
        let unpack = host.state(UNPACK)?;
        aa_mind::activate(unpack, 2, a, host)
    });
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("regiment {o} unpack: {e}"));
            false
        }
    }
}

/// `0x51D1C0`, the Stop command: Sleep in mode 2.
pub(super) fn stop(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId) {
    let result = with_regiment(ms, world, rules, o, |host, a| {
        let sleep = host.state(SLEEP)?;
        aa_mind::activate(sleep, 2, a, host)
    });
    if let Err(e) = result {
        ms.note(format!("regiment {o} stop: {e}"));
    }
}

/// `0x524B80(member)` from `0x57CBA0`: `0x522890(member, 0)`, then the member at the
/// head of the arrival list +0x2BC.
pub(super) fn member_added(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, member: ActorId) {
    let result = with_regiment(ms, world, rules, o, |host, a| host.member_command(member, 0, a));
    if let Err(e) = result {
        ms.note(format!("regiment {o} member: {e}"));
    }
    ms.orgs[o as usize].pending.insert(0, member);
}

/// `0x578ED0` -> `0x522840`: the regiment lists a member that joined, for its next
/// Think.
pub(super) fn list_member(ms: &mut MovementState, o: OrgId, member: ActorId) {
    ms.orgs[o as usize].pending.insert(0, member);
}

/// Mind +0x14C `0x51F7A0` from the leader's node reached, inside the element's Mind.
pub(super) fn leader_node_reached(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, leader: ActorId, leader_nodes: usize, a: &mut Arena) {
    let mut host = Regiment { ms, world, rules, o, resetting: false };
    if let Err(e) = node_reached(&mut host, leader, leader_nodes, a) {
        ms_note(&mut host, format!("regiment {o} node: {e}"));
    }
}
fn ms_note(host: &mut Regiment<'_>, what: String) {
    host.ms.note(what);
}

/// `0x51F7A0`: the leader's node consumes the regiment's while the state is not its
/// Unpack's own wait: `0x52CDB0` the path's head, `0x57BF30` the next node's facing;
/// at the last, inside the arrival radius, Mind +0x144.
fn node_reached(host: &mut Regiment<'_>, leader: ActorId, leader_nodes: usize, a: &mut Arena) -> aa_mind::Result<()> {
    let o = host.o;
    if !host.is_current(a, MOVE)? && !host.is_current(a, UNPACK)? && !host.is_current(a, rampage::RAMPAGE)? {
        return Ok(());
    }
    if host.ms.orgs[o as usize].leader != Some(leader) || host.ms.orgs[o as usize].order.path.is_empty() {
        return Ok(());
    }
    // Regiment path +0xC, `0x51E8D0` on the leader's, `0x52CDB0`.
    let mut route = owned_path(&host.ms.orgs[o as usize].order);
    if route.kind1_count < leader_nodes as i32 {
        return Ok(());
    }
    if let Some(node) = route.nodes.first().copied() {
        match node.words[4] {
            1 => route.kind1_count -= 1,
            2 => route.kind2_count -= 1,
            _ => {}
        }
        route.nodes.remove(0);
        route.dirty = true;
    }
    host.ms.orgs[o as usize].order.route = Some(route);
    host.ms.orgs[o as usize].order.path.remove(0);
    if let Some(head) = host.ms.orgs[o as usize].order.path.first() {
        let dir = head.dir;
        if dir != Vec2::ZERO {
            set_org_facing(host.ms, host.world, o, dir);
        }
    }
    let at = bits(host.world.orgs[o as usize].anchor);
    let order = &host.ms.orgs[o as usize].order;
    if order.path.is_empty() && order.active && !order.capped && aa_mind::within_arrival(at, bits(order.path_dest), PC)? {
        host.finished(a)?;
    }
    Ok(())
}

/// Organization +0xBC / +0xC0 read by `0x57C680`'s callers, for `Binary`-typed
/// helpers elsewhere in this module.
#[allow(dead_code)]
fn binary(bits: u32) -> aa_mind::Result<Binary> {
    Binary::from_f32_bits(bits).map_err(err)
}
#[allow(dead_code)]
const _: u32 = ONE_BITS;
