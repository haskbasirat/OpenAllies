//! The regiment Mind's fighting states on the ports: Kill (Mind +0xC0,
//! `aa_mind::cai_thinks::kill`), Chasing (+0x120, `0x523930`), Reform (+0x1B8,
//! `aa_mind::cai_thinks::reform`), Engage (+0x1CC, `aa_mind::cai_engage_think`, its list
//! through `aa_mind::cai_engage`), the Mind's `0x524C10`, `0x524A00`, `0x524AE0`,
//! `EventTargetGone` `0x521F90` and the Kill arm of `0x5222E0`.
//!
//! The list itself stays in `CombatState` (`OrgCombat::engage`), which the zones, the
//! view and the AI read, and a Think carries it across the port. Things are named
//! `ORG | o` or by their actor index, a GTID being that plus one.
//!
//! **Renditions, each at its place:** the Siege state (Mind +0xF8) has no site here
//! (`0x4D11D0`: no wall thing is hosted); a pending kill command does not count as the
//! current target in `0x522700`; `Organization +0x164` of step 6 of `0x526A70` answers
//! false, so the all-engaged byte stays clear (nothing here reads it).
//!
//! The Guard state (+0x17C) is `aa_mind::cai_guard` over the embedded Kill, Move and Sleep;
//! its Bombard (+0xD4) stands for a task of `crate::combat::tasks`, which is not on the port.
//! `MindState::Guard` in `crate::orders` says it is on the stack.
//!
//! The Explore state (+0x220) is `aa_mind::cai_thinks::explore`; its waypoint candidate
//! `0x5242C0` is a rendition (`crate::orders::explore`).
//!
//! The Rout state (+0x20C, `aa_mind::cai_rout`, its destination `aa_mind::rout_select`) and
//! the Exhausted state (+0x270) are hosted too. `OrgCombat::routing` and `exhausted_state`
//! hold nothing of their own: they say to the rest of the game that the state is on the
//! stack.
use super::*;
use crate::combat::{self, TargetRef};
use crate::world::Event;
use aa_mind::cai_engage::{self, EngageWorld, TargetNode, Targets};
use aa_mind::cai_engage_think;
use aa_mind::cai_thinks::{explore as explore_port, kill as kill_port, reform as reform_port, siege_target};
use aa_mind::{cai_guard, cai_rout, rout_score, rout_select};

pub(in crate::movement) const KILL: u32 = cai_thinks::KILL_OFFSET;
pub(in crate::movement) const CHASING: u32 = 0x120;
pub(in crate::movement) const REFORM: u32 = 0x1b8;
pub(in crate::movement) const ENGAGE: u32 = 0x1cc;
pub(in crate::movement) const GUARD: u32 = 0x17c;
/// The Bombard state as the Guard state sees it (the task of `crate::combat::tasks`).
const BOMBARD: u32 = 0xd4;
pub(in crate::movement) const ROUT: u32 = 0x20c;
pub(in crate::movement) const EXPLORE: u32 = 0x220;
pub(in crate::movement) const EXHAUSTED: u32 = 0x270;
/// Kill +0x10, Chasing +0x10 and Reform +0x10 as Mind offsets; Engage +0x20.
const KILL_TARGET: u32 = KILL + 0x10;
const CHASING_TARGET: u32 = CHASING + 0x10;
const REFORM_START: u32 = REFORM + 0x10;
const LAST_ATTACKER: u32 = ENGAGE + 0x20;
/// Guard +0x10 the action, +0x14 the byte "placed", +0x18 the guarded GTID, +0x1C the
/// point, +0x24 the direction, as Mind offsets.
const GUARD_ACTION: u32 = GUARD + 0x10;
const GUARD_PLACED: u32 = GUARD + 0x14;
const GUARDED: u32 = GUARD + 0x18;
const GUARD_POINT: u32 = GUARD + 0x1c;
const GUARD_AUX: u32 = GUARD + 0x24;
/// Explore +0x10 the waypoint, +0x18 the target and its score, count and kind, as Mind
/// offsets.
const EXPLORE_POINT: u32 = EXPLORE + 0x10;
const EXPLORE_TARGET: u32 = EXPLORE + 0x18;
/// Rout +0x10: the game time at its Enter.
const ROUT_ENTERED: u32 = ROUT + 0x10;

fn handle(t: TargetRef) -> u32 {
    match t {
        TargetRef::Actor(a) => a,
        TargetRef::Org(o) => ORG | o,
    }
}
fn thing(h: u32) -> TargetRef {
    if h & ORG != 0 { TargetRef::Org(h & !ORG) } else { TargetRef::Actor(h) }
}
fn gtid(t: TargetRef) -> u32 {
    handle(t).wrapping_add(1)
}
fn resolve_gtid(world: &World, g: u32) -> Option<TargetRef> {
    let t = thing(g.checked_sub(1)?);
    combat::target_alive(world, t).then_some(t)
}

/// The GTID registry for the regiment Thinks: a thing's handle, an organization's too.
pub(super) fn resolve(host: &Regiment<'_>, g: u32) -> aa_mind::Result<Option<u32>> {
    Ok(resolve_gtid(host.world, g).map(handle))
}

/// `0x52A8A0`: the state is on the Mind's stack.
fn in_stack(a: &Arena, mind: MindId, state: StateId) -> bool {
    a.mind(mind).is_ok_and(|m| m.stack.iter().any(|s| *s == Some(state)))
}

impl Regiment<'_> {
    fn is_top(&self, a: &Arena, offset: u32) -> bool {
        self.is_current(a, offset).unwrap_or(false)
    }
    fn list(&self) -> Targets {
        let nodes = self.world.combat.org(self.o).map_or_else(Vec::new, |s| s.engage.iter().map(|n| TargetNode { gtid: gtid(n.target), priority: n.priority }).collect());
        Targets { nodes }
    }
    fn store_list(&mut self, targets: &Targets) {
        let o = self.o as usize;
        if let Some(s) = self.world.combat.orgs.get_mut(o) {
            s.engage = targets.nodes.iter().map(|n| combat::EngageTarget { target: thing(n.gtid.wrapping_sub(1)), priority: n.priority }).collect();
        }
    }
    fn combat_org(&mut self) -> Option<&mut combat::OrgCombat> {
        self.world.combat.orgs.get_mut(self.o as usize)
    }
    /// `0x523DD0(target, 0)` and `0x521060` before it.
    fn add_checked(&mut self, t: TargetRef) -> bool {
        let mut st = std::mem::take(&mut self.world.combat);
        let added = combat::add_engage_target(self.world, &mut st, self.o, t);
        self.world.combat = st;
        added
    }
    fn can_add(&self, t: TargetRef) -> bool {
        combat::can_engage(self.world, &self.world.combat, self.o, t)
    }
    /// Mind virtual +0x1AC `0x527240`.
    fn target_priority(&self, t: TargetRef) -> i32 {
        combat::regiment_priority(self.world, &self.world.combat, self.o, t)
    }
    /// `0x4D4420`'s organization for a thing: its own when attached.
    fn organization_of(&self, t: TargetRef) -> TargetRef {
        match t {
            TargetRef::Actor(a) => combat::target_ref(self.world, a),
            org => org,
        }
    }
    /// `0x51FBC0`: the thing's own position (an organization's, not a member's) within
    /// `CAITargetInRangeZOCMultiplier` times the zone's radius, and it holds the zone
    /// (virtual +0x130(zone 4, 0)).
    pub(super) fn thing_in_range(&self, t: TargetRef) -> bool {
        let p = match t {
            TargetRef::Org(g) => self.world.orgs[g as usize].anchor,
            TargetRef::Actor(a) => self.world.actors[a as usize].pos.truncate(),
        };
        let o = self.o as usize;
        let mut zone = self.world.data.def(self.world.orgs[o].def).control_range;
        if self.world.combat.org(self.o).is_some_and(|c| c.exhausted) {
            zone *= 0.75;
        }
        let d = self.world.orgs[o].anchor - p;
        let distance = (f64::from(d.y) * f64::from(d.y) + f64::from(d.x) * f64::from(d.x)).sqrt() as f32;
        distance <= self.rules.target_in_range_zoc * zone && combat::thing_in_zoc(self.world, self.o, t, false)
    }
    /// The thing a Move's target id stands for: a Kill or a Chasing that names an
    /// organization walks at one of its members ([`Self::walk_to`]).
    pub(super) fn walked_thing(&self, member: ActorId) -> TargetRef {
        let org = self.world.actor(member).and_then(|x| x.org);
        for word in [KILL_TARGET, CHASING_TARGET] {
            let w = self.word(word);
            if let Some(TargetRef::Org(g)) = w.checked_sub(1).map(thing)
                && org == Some(g)
            {
                return TargetRef::Org(g);
            }
        }
        TargetRef::Actor(member)
    }
    /// The embedded Move at a thing (`0x520320(mode, thing, 0, null)`): the path's target
    /// is the live member that stands for it, renewed before each Think.
    fn walk_to(&mut self, t: TargetRef, a: &mut Arena) -> aa_mind::Result<()> {
        let Some(rep) = combat::representative(self.world, t) else { return Ok(()) };
        let to = pos2(self.world, rep);
        let unit = Unit::Org(self.o);
        let offset = crate::orders::group::member_offset(self.world, unit, to);
        start(self.ms, self.world, Owner::Org(self.o), to, offset, Some(rep), false, None, None);
        self.order().follow = true;
        let mv = self.state(MOVE)?;
        let extents = self.extents();
        let mut fields = company(self.order(), 0);
        fields.target_id = rep + 1;
        cm::set_destination(&mut fields, bits(to), extents, PC, self).map_err(err)?;
        store(self.order(), &fields);
        aa_mind::activate(mv, 0, a, self)?;
        Ok(())
    }
    fn walk_target_refresh(&mut self, t: TargetRef) {
        if let Some(rep) = combat::representative(self.world, t) {
            self.order().target = Some(rep);
            // What the rest of the game reads names a live thing.
            if let Order::Attack { target } = self.world.orgs[self.o as usize].order
                && !alive(self.world, target)
            {
                set_order(self.world, self.ms, Owner::Org(self.o), Order::Attack { target: rep });
            }
        }
    }
}

/// State Enter (virtual +0x18) of the states here; false for another state.
pub(super) fn enter(host: &mut Regiment<'_>, state: StateId, offset: u32, a: &mut Arena) -> aa_mind::Result<bool> {
    match offset {
        // 0x5206A0: status 2, the Move to the target.
        KILL => {
            a.state_mut(state)?.status = 2;
            if let Some(t) = resolve_gtid(host.world, host.word(KILL_TARGET)) {
                host.walk_to(t, a)?;
            }
        }
        // 0x520D20: the same to the chased thing.
        CHASING => {
            a.state_mut(state)?.status = 2;
            if let Some(t) = resolve_gtid(host.world, host.word(CHASING_TARGET)) {
                host.walk_to(t, a)?;
            }
        }
        // 0x523D50: status 2, the start, each member to its slot (0x522890(member, 0)).
        REFORM => {
            a.state_mut(state)?.status = 2;
            host.set_word(REFORM_START, host.world.clock.begin.0);
            let members: Vec<ActorId> = alive_members(host.world, host.o).collect();
            for m in members {
                if def_of(host.world, m).speed_max > 0.0 {
                    host.member_command(m, 0, a)?;
                }
            }
        }
        // 0x525CA0: status 2, 0x523EA0, the formation faces the tail (0x57BF30).
        ENGAGE => {
            a.state_mut(state)?.status = 2;
            let mut targets = host.list();
            cai_engage::refresh_targets(&mut targets, state, a, host)?;
            host.store_list(&targets);
            if let Some(t) = targets.nodes.last().and_then(|n| resolve_gtid(host.world, n.gtid)).and_then(|t| combat::representative(host.world, t)) {
                let d = pos2(host.world, t) - host.world.orgs[host.o as usize].anchor;
                if d.length_squared() > 0.0 {
                    set_org_facing(host.ms, host.world, host.o, d.normalize());
                }
            }
        }
        // 0x529420: status 2, the embedded states leave, Mind +0x2B4 the direction, 0x51EE20.
        GUARD => {
            a.state_mut(state)?.status = 2;
            leave_embedded(host, a)?;
            let o = host.o;
            host.world.books.minds.set_byte(key(o), GUARD_PLACED, 0);
            host.set_word(GUARD_ACTION, 0);
            let aux = [host.word(GUARD_AUX), host.word(GUARD_AUX + 4)];
            host.set_word(SLEEP_POINT, aux[0]);
            host.set_word(SLEEP_POINT + 4, aux[1]);
            guard_stand_point(host);
            host.world.orders.orgs.insert(o, crate::orders::MindState::Guard);
        }
        // 0x5213A0: status 2, the Move to the target (0x520440(0, target, 0, null)).
        EXPLORE => {
            a.state_mut(state)?.status = 2;
            let o = host.o;
            host.world.orders.orgs.insert(o, crate::orders::MindState::Explore);
            let to = vec2([host.word(EXPLORE_TARGET), host.word(EXPLORE_TARGET + 4)]);
            walk_point(host, to, [0, 0], a)?;
        }
        // 0x51F190.
        ROUT => {
            let mut targets = host.list();
            cai_rout::enter(state, &mut targets, a, host)?;
            host.store_list(&targets);
            let now = host.world.time;
            let o = host.o;
            if let Some(s) = host.combat_org() {
                s.routing = Some(combat::Rout { since: now, destination: None, next_think: now });
                s.reform = None;
                s.explicit = None;
                s.saved_order = None;
                s.auto_order = None;
                s.entrenched = false;
                s.rest_since = None;
            }
            host.world.events.push(Event::Routed { org: o });
        }
        // 0x521AF0: status 2, each member a Stop, the Sleep in mode 0.
        EXHAUSTED => {
            a.state_mut(state)?.status = 2;
            if let Some(s) = host.combat_org() {
                s.exhausted_state = true;
            }
            let rules = host.rules;
            let members: Vec<ActorId> = alive_members(host.world, host.o).collect();
            for m in members {
                element::clear_in(host.ms, host.world, rules, m, a);
            }
            let sleep = host.state(SLEEP)?;
            aa_mind::activate(sleep, 0, a, host)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// State Exit (virtual +0x1C) of the states here; false for another state.
pub(super) fn leave(host: &mut Regiment<'_>, state: StateId, offset: u32, a: &mut Arena) -> aa_mind::Result<bool> {
    match offset {
        // 0x51D9C0: the embedded Move leaves.
        KILL | CHASING => {
            let mv = host.state(MOVE)?;
            if a.state(mv)?.status != 0 {
                aa_mind::World::leave(host, mv, a)?;
            }
            a.state_mut(state)?.status = 0;
        }
        REFORM => a.state_mut(state)?.status = 0,
        // 0x520FE0: the list goes (0x51F040), each member Mind +0x194 `0x52DBB0`.
        ENGAGE => {
            if let Some(s) = host.combat_org() {
                s.engage.clear();
            }
            a.state_mut(state)?.status = 0;
            let rules = host.rules;
            let members: Vec<ActorId> = alive_members(host.world, host.o).collect();
            for m in members {
                element::fight::regiment_left_engage(host.ms, host.world, rules, m, a);
            }
        }
        // 0x51E180: the Sleep and the Move leave.
        EXPLORE => {
            let (sleep, mv) = (host.state(SLEEP)?, host.state(MOVE)?);
            if a.state(sleep)?.status == 2 {
                aa_mind::World::leave(host, sleep, a)?;
            }
            if a.state(mv)?.status == 2 {
                aa_mind::World::leave(host, mv, a)?;
            }
            a.state_mut(state)?.status = 0;
            let o = host.o;
            host.world.orders.orgs.remove(&o);
        }
        // 0x529490.
        GUARD => {
            leave_embedded(host, a)?;
            let o = host.o;
            host.world.books.minds.set_byte(key(o), GUARD_PLACED, 0);
            host.set_word(GUARD_ACTION, 0);
            a.state_mut(state)?.status = 0;
            host.world.orders.orgs.remove(&o);
        }
        // The Guard's Bombard: its task ends.
        BOMBARD => {
            a.state_mut(state)?.status = 0;
            let o = host.o;
            let bombarding = host.combat_org().is_some_and(|s| matches!(s.task, Some(combat::tasks::Task::Bombard(_))));
            if bombarding {
                let mut st = std::mem::take(&mut host.world.combat);
                combat::tasks::end_guard_bombard(host.world, &mut st, o);
                host.world.combat = st;
            }
        }
        // 0x51E130: the ZOC's effects at the next update's start (0x4F3D90), the embedded
        // Move leaves. Nothing of the rout stands under it (52A810(2)): what is shown is rest.
        ROUT => {
            if let Some(s) = host.combat_org() {
                s.routing = None;
                s.zoc_rescan = true;
            }
            let mv = host.state(MOVE)?;
            if a.state(mv)?.status == 2 {
                aa_mind::World::leave(host, mv, a)?;
            }
            a.state_mut(state)?.status = 0;
            let o = host.o;
            set_order(host.world, host.ms, Owner::Org(o), Order::Idle);
        }
        // 0x51DDF0: the Sleep leaves.
        EXHAUSTED => {
            if let Some(s) = host.combat_org() {
                s.exhausted_state = false;
            }
            let sleep = host.state(SLEEP)?;
            if a.state(sleep)?.status != 0 {
                aa_mind::World::leave(host, sleep, a)?;
            }
            a.state_mut(state)?.status = 0;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// The state's virtual +0xC; `None` for another state.
pub(super) fn think(host: &mut Regiment<'_>, offset: u32, a: &mut Arena) -> Option<aa_mind::Result<()>> {
    let state = match host.state(offset) {
        Ok(s) => s,
        Err(e) => return Some(Err(e)),
    };
    Some(match offset {
        KILL => {
            if let Some(t) = resolve_gtid(host.world, host.word(KILL_TARGET)) {
                host.walk_target_refresh(t);
            }
            kill_port::think(state, a, host).map(|_| ())
        }
        CHASING => chasing_think(host, state, a),
        REFORM => reform_port::think(state, a, host, PC).map(|_| ()),
        ENGAGE => engage_think(host, state, a),
        GUARD => cai_guard::think(state, a, host, PC).map(|_| ()),
        EXPLORE => explore_port::think(state, a, host, PC).map(|_| ()),
        // The task thinks for itself; an ended task ends the state.
        BOMBARD => bombard_think(host, state, a),
        ROUT => cai_rout::think(state, a, host, PC).map(|_| ()),
        EXHAUSTED => exhausted_think(host, state, a),
        _ => return None,
    })
}

/// The Guard's Bombard: the task thinks for itself; an ended task ends the state.
fn bombard_think(host: &mut Regiment<'_>, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
    let bombarding = host.combat_org().is_some_and(|s| matches!(s.task, Some(combat::tasks::Task::Bombard(_))));
    if !bombarding && a.state(state)?.status != 0 {
        aa_mind::World::leave(host, state, a)?;
    }
    Ok(())
}

/// `0x51E260`: not steady, the Sleep's Think; steady and current, the state ends; else a
/// Think in 1 s.
fn exhausted_think(host: &mut Regiment<'_>, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
    aa_mind::think_base(state, a)?;
    if morale_state(host) != 2 {
        return think_state(host, SLEEP, a);
    }
    if host.is_top(a, EXHAUSTED) {
        aa_mind::pop(MindId(key(host.o)), a, host)?;
        return Ok(());
    }
    think_in(host.world, host.o, f32::from_bits(ONE_BITS));
    Ok(())
}

/// `0x578140`.
pub(super) fn morale_state(host: &Regiment<'_>) -> u8 {
    let i = host.o as usize;
    host.world.combat.org(host.o).map_or(2, |s| crate::morale::state(s, host.world.orgs[i].morale))
}

/// `0x51FAE0`: the Exhausted state in mode 2.
fn exhausted_begin(host: &mut Regiment<'_>, a: &mut Arena) -> aa_mind::Result<()> {
    let exhausted = host.state(EXHAUSTED)?;
    aa_mind::activate(exhausted, 2, a, host)?;
    Ok(())
}

/// Mind +0x178 `0x522B90`: the time of the current state is over. Reform: +0x11C. Rout:
/// still exhausted the Exhausted state, else the stack goes on. Another state: a Think in
/// 1 s. (The Retreat's arm, which walks on to its point, is `crate::orders`'.)
fn time_over(host: &mut Regiment<'_>, a: &mut Arena) -> aa_mind::Result<()> {
    if host.is_top(a, REFORM) {
        aa_mind::pop(MindId(key(host.o)), a, host)?;
        return Ok(());
    }
    if host.is_top(a, ROUT) {
        if morale_state(host) == 0 {
            return exhausted_begin(host, a);
        }
        aa_mind::pop(MindId(key(host.o)), a, host)?;
        return Ok(());
    }
    think_in(host.world, host.o, f32::from_bits(ONE_BITS));
    Ok(())
}

/// The Guard arm of Mind +0x124 `0x5273D0` (`0x527440`): what is walking leaves, a Think
/// in 0.6 s. The Rout arm (`0x527658`): as the time being over, and the next Think in
/// 0.6 s. False: neither.
pub(super) fn rout_move_failed(host: &mut Regiment<'_>, a: &mut Arena) -> aa_mind::Result<bool> {
    // 0x5273FD: the Explore's Move leaves, a Think in 0.6 s.
    if host.is_top(a, EXPLORE) {
        let mv = host.state(MOVE)?;
        if a.state(mv)?.status == 2 {
            aa_mind::World::leave(host, mv, a)?;
        }
        think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
        return Ok(true);
    }
    if host.is_top(a, GUARD) {
        let (kill, bombard, mv) = (host.state(KILL)?, host.state(BOMBARD)?, host.state(MOVE)?);
        if a.state(kill)?.status != 2 && a.state(bombard)?.status != 2 && a.state(mv)?.status == 2 {
            aa_mind::World::leave(host, mv, a)?;
        }
        if a.state(kill)?.status == 2 {
            aa_mind::World::leave(host, kill, a)?;
        }
        if a.state(bombard)?.status == 2 {
            aa_mind::World::leave(host, bombard, a)?;
        }
        think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
        return Ok(true);
    }
    if !host.is_top(a, ROUT) {
        return Ok(false);
    }
    if morale_state(host) == 0 {
        exhausted_begin(host, a)?;
        return Ok(true);
    }
    aa_mind::pop(MindId(key(host.o)), a, host)?;
    think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
    Ok(true)
}

/// `0x51F920` and `0x51F9F0` from the morale (`0x579660`). `exhausted`: not with the
/// Engage or the Rout current. `0x4D1080` is false for a regiment.
pub(in crate::movement) fn morale_reaction(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, exhausted: bool) -> bool {
    let result = with_regiment(ms, world, rules, o, |host, a| -> aa_mind::Result<bool> {
        if host.is_top(a, ROUT) || (exhausted && host.is_top(a, ENGAGE)) {
            return Ok(false);
        }
        // 0x52A810(rout, 2): the Exhausted state outranks it.
        if in_stack(a, MindId(key(host.o)), host.state(EXHAUSTED)?) {
            return Ok(false);
        }
        let rout = host.state(ROUT)?;
        aa_mind::activate(rout, 2, a, host)
    });
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("regiment {o} rout: {e}"));
            false
        }
    }
}

/// `0x52A690`: how much of `delta` fits along `direction` past `base` inside the map.
fn reachable(world: &World, origin: [u32; 2], direction: [u32; 2], base: u32, delta: u32) -> u32 {
    let map = &world.map;
    let (w, h) = (map.width as f32 * map.cell_size, map.height as f32 * map.cell_size);
    let [x, y] = origin.map(|v| f64::from(f32::from_bits(v)));
    let [dx, dy] = direction.map(|v| f64::from(f32::from_bits(v)));
    let (b, d) = (f64::from(f32::from_bits(base)), f64::from(f32::from_bits(delta)));
    let far = b + d;
    let (end_x, end_y) = ((dx * far + x) as f32, (y + dy * far) as f32);
    let inside = |px: f32, py: f32| (0.0..w).contains(&px) && (0.0..h).contains(&py);
    if inside(end_x, end_y) {
        return delta;
    }
    // 0x4282A0: the near point.
    if !inside((dx * b + x) as f32, (dy * b + y) as f32) {
        return 0;
    }
    // 0x46EB30 -> 0x46E150, a box of the map's size about its middle: the far point kept
    // in it.
    let half_h = (f64::from(h) * 0.5) as f32;
    let half_w = (f64::from(w) * 0.5) as f32;
    let kept_x = (end_x - half_w).clamp(-half_w, half_w);
    let kept_y = (end_y - half_h).clamp(-half_h, half_h);
    let rx = (x - (f64::from(half_w) + f64::from(kept_x))) as f32;
    let ry = y - (f64::from(half_h) + f64::from(kept_y));
    let length = (f64::from(rx) * f64::from(rx) + ry * ry).sqrt();
    ((length - b) as f32).to_bits()
}

/// `0x52B530` on the world's stream. The getter `0x4F02B0` logs once; the candidates draw
/// from the object it gave (`0x52B4E4`, `0x52B629`, `0x52B699`, `0x52B8E8`, `0x52B9F5`).
fn rout_destination(host: &mut Regiment<'_>) -> aa_mind::Result<Option<[u32; 2]>> {
    let world = &mut *host.world;
    world.trace.log(crate::trace::SKind::RngGetter(crate::trace::sites::ROUT_GETTER), world.native_rng.state);
    let rule = |name: &str, shipped: f32| world.data.rule_ci(name).unwrap_or(shipped).to_bits();
    let weights = rout_score::Weights {
        base_value: rule("RoutBaseValue", 0.0),
        factor_enemy_dist: rule("RoutFactorEnemyDist", 10.0),
        factor_enemy_dist_cap: rule("RoutFactorEnemyDistCap", 2000.0),
        factor_enemy_angle: rule("RoutFactorEnemyAngle", 0.25),
        factor_moved_dist: rule("RoutFactorMovedDist", 8.0),
        factor_moved_dist_cap: rule("RoutFactorMovedDistCap", 2000.0),
        factor_supply_dist: rule("RoutFactorSupplyDist", 1.0),
        factor_supply_dist_cap: rule("RoutFactorSupplyDistCap", 2000.0),
        penalty_enemy_zoc: rule("RoutPenaltyEnemyZOC", 50.0),
        penalty_enemy_zog: rule("RoutPenaltyEnemyZOG", 75.0),
        bonus_friendly_zos: rule("RoutBonusFriendlyZOS", 50.0),
        bonus_supply_in_range: rule("RoutBonusSupplyInRange", 50.0),
        fuzziness: rule("RoutFuzziness", 20.0),
    };
    let i = host.o as usize;
    let s = world.combat.org(host.o);
    let engaged = s.map_or_else(Vec::new, |s| s.engaged_by.iter().map(|&e| gtid(TargetRef::Org(e))).collect());
    let mut kingdoms = vec![Some(Vec::new()); world.kingdoms.len()];
    for z in &world.books.zones {
        if let Some(Some(list)) = kingdoms.get_mut(z.kingdom as usize) {
            list.push(rout_score::SupplySource { position: bits(z.centre), radius: Some(z.radius.to_bits()) });
        }
    }
    let min_distance = world.combat.org(host.o).map_or(world.data.def(world.orgs[i].def).control_range, |_| combat::control_radius(world, &world.combat, host.o));
    let context = rout_select::Context {
        score: rout_score::Context {
            settings: rout_score::Settings { rout: weights, recover: weights },
            map: rout_score::MapGeometry { shift: world.map.cell_size.log2() as u32, width: world.map.width as i32 },
            organization: bits(world.orgs[i].anchor),
            engaged,
            kingdoms,
        },
        min_distance: min_distance.to_bits(),
        random_candidates: world.data.rule_ci("RoutRandomCandidates").unwrap_or(20.0) as i32,
        max_distance_delta: rule("RoutMaxDistanceDelta", 24.0),
        supply_candidates: world.data.rule_ci("RoutSupplyZoneCandidates").unwrap_or(2.0) as i32,
        supply_max_angle_delta: rule("RoutSupplyZoneMaxAngleDelta", 50.0),
        directions: (0..2048u32).map(|d| aa_movement::formation::random_direction(d).map(f32::to_bits)).collect(),
    };
    rout_select::select(&context, host, PC)
}

impl rout_score::RoutWorld for Regiment<'_> {
    fn actor_position(&mut self, g: u32) -> aa_mind::Result<Option<[u32; 2]>> {
        Ok(resolve_gtid(self.world, g).map(|t| match t {
            TargetRef::Org(o) => bits(self.world.orgs[o as usize].anchor),
            TargetRef::Actor(a) => bits(pos2(self.world, a)),
        }))
    }
    fn angle(&mut self, from: [u32; 2], to: [u32; 2], p: aa_numeric::Precision) -> aa_mind::Result<Binary> {
        rout_score::native_angle(from, to, p)
    }
    fn owner(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.orgs[self.o as usize].kingdom)
    }
    /// `0x54EFC0`.
    fn zone(&mut self, tile: i32, owner: u32, kind: u32) -> aa_mind::Result<u32> {
        let Ok(tile) = usize::try_from(tile) else { return Ok(0) };
        let Some(mut e) = self.world.fog.engine.take() else { return Ok(0) };
        let found = e.enemy_zone(self.world, tile, owner, kind);
        self.world.fog.engine = Some(e);
        Ok(found)
    }
    /// `0x54F8B0(tile, owner, organization)`.
    fn friendly_zone(&mut self, tile: i32, owner: u32, _: bool) -> aa_mind::Result<bool> {
        let Ok(tile) = usize::try_from(tile) else { return Ok(false) };
        let Some(mut e) = self.world.fog.engine.take() else { return Ok(false) };
        let found = e.friendly_supply(self.world, tile, Some(owner), crate::zones::Key::Org(self.o));
        self.world.fog.engine = Some(e);
        Ok(found)
    }
    /// Kingdom virtual +0x138.
    fn kingdom_allied(&mut self, slot: usize, owner: u32) -> aa_mind::Result<bool> {
        Ok(matches!(crate::setup::relation(self.world, Some(owner), Some(slot as KingdomId)), crate::books::SELF | crate::books::FRIEND))
    }
    /// A draw from the object of the one getter: no S entry.
    fn random(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.native_rng.next_u32())
    }
}

impl rout_select::SelectWorld for Regiment<'_> {
    fn organization_virtual_4(&mut self) -> aa_mind::Result<()> {
        Ok(())
    }
    fn reachable_delta(&mut self, origin: [u32; 2], direction: [u32; 2], base: u32, delta: u32) -> aa_mind::Result<u32> {
        Ok(reachable(self.world, origin, direction, base, delta))
    }
    /// `0x4F46E0(organization, &position, 4.0)`. **Rendition:** the descriptor is the
    /// leader's (no thing is hosted for the banner).
    fn find_open_position(&mut self, position: &mut [u32; 2], radius: u32) -> aa_mind::Result<bool> {
        let Some(leader) = self.leader() else { return Ok(false) };
        Ok(route::nearest_placement(self.ms, self.world, leader, position, radius).unwrap_or(false))
    }
}

impl cai_rout::RoutEnterWorld for Regiment<'_> {
    fn world_time(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.clock.begin.0)
    }
    fn set_entered_time(&mut self, _: StateId, time: u32) -> aa_mind::Result<()> {
        self.set_word(ROUT_ENTERED, time);
        Ok(())
    }
    /// `0x4D3FA0`: the attackers' breaker morale and rout experience.
    fn organization_halt(&mut self, _: &mut Arena) -> aa_mind::Result<()> {
        let o = self.o;
        let mut st = std::mem::take(&mut self.world.combat);
        crate::morale::rout_handler(self.world, &mut st, o);
        self.world.combat = st;
        Ok(())
    }
    fn release_command(&mut self, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
}

impl cai_rout::RoutThinkWorld for Regiment<'_> {
    fn world_time(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.clock.begin.0)
    }
    fn routing_time(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.combat.rules.as_deref().map_or(25.0f32, |r| r.routing_time).to_bits())
    }
    fn entered_time(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(self.word(ROUT_ENTERED))
    }
    fn end_rout(&mut self, a: &mut Arena) -> aa_mind::Result<()> {
        time_over(self, a)
    }
    fn move_status(&mut self, a: &Arena) -> aa_mind::Result<u32> {
        Ok(a.state(self.state(MOVE)?)?.status)
    }
    fn select_destination(&mut self, _: &mut Arena) -> aa_mind::Result<Option<[u32; 2]>> {
        rout_destination(self)
    }
    /// `0x520440(0, destination, 0, null)`.
    fn activate_move(&mut self, destination: [u32; 2], a: &mut Arena) -> aa_mind::Result<()> {
        let to = vec2(destination);
        let o = self.o;
        // The fields a new Move starts from; a target here keeps `start` from activating.
        let any = alive_members(self.world, o).next();
        if let Some(any) = any {
            start(self.ms, self.world, Owner::Org(o), to, Vec2::ZERO, Some(any), false, None, None);
            self.order().target = None;
        }
        let speed = self.order().speed.to_bits();
        self.activate_move_fields(to, 0, speed, [0, 0], a)?;
        if let Some(s) = self.combat_org()
            && let Some(r) = s.routing.as_mut()
        {
            r.destination = Some(to);
        }
        set_order(self.world, self.ms, Owner::Org(o), Order::Move { to, attack_move: false });
        Ok(())
    }
    fn schedule(&mut self, delay_bits: u32, _: &mut Arena) -> aa_mind::Result<()> {
        think_in(self.world, self.o, f32::from_bits(delay_bits));
        Ok(())
    }
    fn has_current_command(&mut self, _: &Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn mark_command_destination(&mut self, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn move_destination(&mut self, _: &Arena) -> aa_mind::Result<[u32; 2]> {
        Ok(bits(self.order().destination))
    }
    fn actor_position(&mut self, _: &Arena) -> aa_mind::Result<[u32; 2]> {
        Ok(bits(self.world.orgs[self.o as usize].anchor))
    }
    fn reached(&mut self, a: &mut Arena) -> aa_mind::Result<()> {
        self.finished(a)
    }
    fn move_think(&mut self, a: &mut Arena) -> aa_mind::Result<()> {
        think_state(self, MOVE, a)
    }
}

/// Organization +0x74 as `0x57BFF0` chooses it from the current state's
/// formation_required (`0x4D49B0(4)`, its +0x20): DESIRED +0x70, BROKEN none, COMBAT the
/// layout's +0x44, RETREAT the layout's +0x48. `top` is the Mind's current state.
pub(in crate::movement) fn required_formation(world: &World, o: OrgId, a: &Arena) -> Option<usize> {
    let def = world.data.def(world.orgs[o as usize].def);
    let desired = crate::player_orders::desired_formation(world, o);
    let combat = || def.combat_formation.or(desired);
    let s = world.combat.org(o);
    let status = |offset: u32| world.books.minds.state(key(o), offset).and_then(|st| a.state(st).ok()).map_or(0, |x| x.status);
    let top = a.mind(MindId(key(o))).ok().and_then(|m| m.stack.front().copied().flatten()).and_then(|st| world.books.minds.owner(st)).map(|x| x.1);
    match s.and_then(|s| s.task.as_ref()) {
        // mind_bombard COMBAT; mind_repairing, mind_building BROKEN.
        Some(combat::tasks::Task::Bombard(_)) => return combat(),
        Some(_) => return None,
        None => {}
    }
    match top {
        Some(ROUT | CHASING) => None,
        Some(ENGAGE) => combat(),
        // 0x51D8F0: a move command's Kill state is mind_moving.
        Some(KILL) if s.is_some_and(|s| s.kill_is_move) => desired,
        Some(KILL) => combat(),
        // 0x51DC20: what the Guard does is what it reports: its Kill, mind_attacking; its
        // Move, mind_moving; standing, mind_guarding.
        Some(GUARD) if status(KILL) == 2 => combat(),
        Some(GUARD) if status(MOVE) == 2 => desired,
        Some(GUARD) => combat(),
        // 0x520250: the Move state's command kind 7 reports mind_retreating.
        Some(MOVE) if crate::orders::retreating(world, o) => def.retreat_formation.or(desired),
        _ => desired,
    }
}

/// `0x57C400` -> `0x57BFF0` when the state changes: `0x57BF50` takes the old formation's
/// property off and puts the new one's on, the slot table is recomputed (`0x57BEF0`), and
/// the members' Mover maximum follows. The same formation does nothing.
pub(super) fn formation_follow(host: &mut Regiment<'_>, a: &Arena) {
    let o = host.o;
    let required = required_formation(host.world, o, a);
    if crate::player_orders::formation(host.world, o) != required {
        set_formation_in(host.ms, host.world, o, |world| {
            world.player_orders.current_formation.insert(o, required);
        });
    }
}

/// `0x523930`.
fn chasing_think(host: &mut Regiment<'_>, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
    aa_mind::think_base(state, a)?;
    let g = host.word(CHASING_TARGET);
    if !chase_continues(host, g, a) {
        return target_gone(host, g, a);
    }
    if let Some(t) = resolve_gtid(host.world, g) {
        host.walk_target_refresh(t);
    }
    think_state(host, MOVE, a)
}

/// `0x526A70`. The port's member and Mind callbacks reach the arena through the
/// store, so it holds the real arena and the port reads a view of this Mind.
fn engage_think(host: &mut Regiment<'_>, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
    aa_mind::think_base(state, a)?;
    let chasing = host.state(CHASING)?;
    let mind = MindId(key(host.o));
    let mut targets = host.list();
    let mut view = Arena::default();
    view.minds.insert(mind, a.mind(mind)?.clone());
    view.states.insert(state, a.state(state)?.clone());
    view.states.insert(chasing, a.state(chasing)?.clone());
    std::mem::swap(&mut host.world.books.minds.arena, a);
    let result = cai_engage_think::think(state, chasing, &mut targets, &mut view, host);
    std::mem::swap(&mut host.world.books.minds.arena, a);
    if host.is_top(a, ENGAGE) && host.world.combat.org(host.o).is_some_and(|c| c.routing.is_none()) {
        host.store_list(&targets);
    }
    result?;
    reflect_order(host, a);
    Ok(())
}

/// `0x520D90(gtid)`: the Mind's target or a pending one, that is retreating, routing
/// or moving, still acceptable, the only one on an active list, and out of the ZOC.
fn chase_continues(host: &mut Regiment<'_>, g: u32, a: &Arena) -> bool {
    let Some(t) = resolve_gtid(host.world, g) else { return false };
    let current = current_target(host, a).map(gtid);
    if current != Some(g) {
        return false;
    }
    if !combat::thing_is_fleeing(host.world, t) {
        return false;
    }
    if !combat::zone_accepts(host.world, &host.world.combat, host.o, t) {
        return false;
    }
    if let Ok(engage) = host.state(ENGAGE)
        && a.state(engage).is_ok_and(|s| s.status == 2)
    {
        let list = host.list();
        if list.nodes.len() != 1 || list.nodes.last().map(|n| n.gtid) != Some(g) {
            return false;
        }
    }
    !combat::thing_in_zoc(host.world, host.o, t, true)
}

/// Mind virtual +0x4C `0x522700`: the Kill state's target while it is on the stack.
fn current_target(host: &Regiment<'_>, a: &Arena) -> Option<TargetRef> {
    let kill = host.state(KILL).ok()?;
    if !in_stack(a, MindId(key(host.o)), kill) {
        return None;
    }
    resolve_gtid(host.world, host.word(KILL_TARGET))
}

/// Mind virtual +0xF8 `0x524AE0`, from the Engage state: the Chasing state in mode 1;
/// otherwise a Think in 1 s.
fn chase_begin(host: &mut Regiment<'_>, t: Option<TargetRef>, a: &mut Arena) -> aa_mind::Result<bool> {
    if !host.is_top(a, ENGAGE) {
        think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
        return Ok(false);
    }
    let g = t.map_or(0, gtid);
    let chasing = host.state(CHASING)?;
    if chase_continues(host, g, a) {
        host.set_word(CHASING_TARGET, g);
        if aa_mind::activate(chasing, 1, a, host)? {
            return Ok(true);
        }
    }
    think_in(host.world, host.o, f32::from_bits(ONE_BITS));
    Ok(false)
}

/// `0x51F600`: with the Kill state on the stack and its target there, the regiment is
/// farther from the Move's destination than its leader: no reform.
fn skip_reform(host: &mut Regiment<'_>, a: &Arena) -> bool {
    if current_target(host, a).is_none() {
        return false;
    }
    let dest = host.order().destination;
    let Some(leader) = host.leader() else { return false };
    let lead = pos2(host.world, leader);
    let own = host.world.orgs[host.o as usize].anchor;
    let d = |p: Vec2| fstp_len2(dest, p);
    d(own) > d(lead)
}
fn fstp_len2(a: Vec2, b: Vec2) -> f32 {
    let (dy, dx) = (a.y - b.y, a.x - b.x);
    dy * dy + dx * dx
}

/// Mind virtual +0x140 `EventTargetGone` `0x521F90`.
pub(super) fn target_gone(host: &mut Regiment<'_>, g: u32, a: &mut Arena) -> aa_mind::Result<()> {
    let mind = MindId(key(host.o));
    let immediate = f32::from_bits(IMMEDIATE_BITS);
    // The Guard: its guarded thing ends it; a threat that goes leaves its Kill or Bombard.
    if host.is_top(a, GUARD) {
        if g == host.word(GUARDED) {
            return aa_mind::pop(mind, a, host);
        }
        let (kill, bombard) = (host.state(KILL)?, host.state(BOMBARD)?);
        if a.state(kill)?.status == 2 && g == host.word(KILL_TARGET) {
            aa_mind::World::leave(host, kill, a)?;
        }
        if a.state(bombard)?.status == 2 && g == host.word(BOMBARD + 0x10) {
            aa_mind::World::leave(host, bombard, a)?;
        }
        think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
        guard_reflect(host, a);
        return Ok(());
    }
    if host.is_top(a, ENGAGE) {
        let k = host.word(KILL_TARGET);
        if chase_continues(host, k, a) && chase_begin(host, resolve_gtid(host.world, k), a)? {
            return Ok(());
        }
        if !skip_reform(host, a) {
            let reform = host.state(REFORM)?;
            aa_mind::activate(reform, 1, a, host)?;
            think_in(host.world, host.o, immediate);
            return Ok(());
        }
        aa_mind::pop(mind, a, host)?;
        think_in(host.world, host.o, immediate);
        return Ok(());
    }
    if host.is_top(a, KILL) {
        aa_mind::pop(mind, a, host)?;
        host.world.books.minds.set_byte(key(host.o), SLEEP_BYTE18, 0);
        return Ok(());
    }
    if host.is_top(a, MOVE) {
        aa_mind::pop(mind, a, host)?;
        return Ok(());
    }
    if host.is_top(a, CHASING) {
        // 0x51F100: the list keeps only what its ZOC holds.
        let keep: Vec<_> = host.list().nodes.into_iter().filter(|n| resolve_gtid(host.world, n.gtid).is_some_and(|t| combat::thing_in_zoc(host.world, host.o, t, true))).collect();
        host.store_list(&Targets { nodes: keep.clone() });
        if !keep.is_empty() {
            let engage = host.state(ENGAGE)?;
            aa_mind::activate(engage, 1, a, host)?;
            think_in(host.world, host.o, immediate);
            return Ok(());
        }
        if current_target(host, a).is_none() {
            let reform = host.state(REFORM)?;
            aa_mind::activate(reform, 1, a, host)?;
            think_in(host.world, host.o, immediate);
            return Ok(());
        }
        aa_mind::pop(mind, a, host)?;
        return Ok(());
    }
    think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
    Ok(())
}

/// Mind virtual +0xF0 `0x524A00`.
fn engage_begin(host: &mut Regiment<'_>, t: TargetRef, a: &mut Arena) -> aa_mind::Result<bool> {
    let t = host.organization_of(t);
    if !host.can_add(t) {
        return Ok(false);
    }
    if host.is_top(a, ENGAGE) {
        host.add_checked(t);
        return Ok(false);
    }
    let engage = host.state(ENGAGE)?;
    if host.is_top(a, CHASING) || host.is_top(a, REFORM) {
        aa_mind::activate(engage, 1, a, host)?;
        host.add_checked(t);
        reflect_order(host, a);
        think_in(host.world, host.o, f32::from_bits(IMMEDIATE_BITS));
        return Ok(true);
    }
    if !aa_mind::activate(engage, 3, a, host)? {
        if let Some(s) = host.combat_org() {
            s.engage.clear();
        }
        return Ok(false);
    }
    host.add_checked(t);
    reflect_order(host, a);
    Ok(true)
}

/// Mind `0x524C10(target)`: from the ZOC (`0x57AB70`) and from damage (`0x579490`).
pub(in crate::movement) fn add_target(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, t: TargetRef) -> bool {
    let result = with_regiment(ms, world, rules, o, |host, a| -> aa_mind::Result<bool> {
        let t = host.organization_of(t);
        if host.is_top(a, ENGAGE) {
            if host.can_add(t) {
                host.add_checked(t);
            }
            return Ok(false);
        }
        if host.is_top(a, CHASING)
            && let Some(chased) = resolve_gtid(host.world, host.word(CHASING_TARGET))
            && host.target_priority(chased) >= host.target_priority(t)
        {
            return Ok(false);
        }
        engage_begin(host, t, a)
    });
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("regiment {o} engage: {e}"));
            false
        }
    }
}

/// `0x51D920(2, target)` from the kill command `0x527FB0`, or the move command's
/// `0x527E30` on a hostile thing (`Mind +0x20`'s kind 5). False when refused
/// (`0x57A500`), which leaves the handlers' Move at the thing.
pub(in crate::movement) fn kill_command(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, target: ActorId) -> bool {
    // A command can come before the first combat pass of a new thing.
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    world.combat = st;
    let t = combat::target_ref(world, target);
    let result = with_regiment(ms, world, rules, o, |host, a| -> aa_mind::Result<bool> {
        let g = gtid(t);
        if host.is_top(a, KILL) && host.word(KILL_TARGET) == g {
            if !host.world.is_scheduled(Sched::OrgMind(host.o)) {
                think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
            }
            return Ok(true);
        }
        if !combat::kill_accepts_thing(host.world, host.o, t) {
            return Ok(false);
        }
        host.set_word(KILL_TARGET, g);
        let kill = host.state(KILL)?;
        let moved = crate::orders::take_move_onto(host.world, host.o);
        if let Some(s) = host.combat_org() {
            s.kill_is_move = moved;
        }
        aa_mind::activate(kill, 2, a, host)
    });
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("regiment {o} kill: {e}"));
            false
        }
    }
}

/// `0x52A810(state, 2)` of a Mind state not hosted here (a rout, a task): the hosted
/// states leave as they would. The Sleep stands under what `crate::morale` keeps.
pub(in crate::movement) fn stand_down(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId) {
    let top = world.books.minds.top_offset(key(o));
    if matches!(top, Some(KILL | CHASING | ENGAGE | REFORM)) {
        stop(ms, world, rules, o);
    }
}

/// `0x523EA0` at the Mind's update: the list follows the ZOC and the operating radius.
pub(super) fn refresh(host: &mut Regiment<'_>, a: &mut Arena) -> aa_mind::Result<()> {
    let mut targets = host.list();
    if targets.nodes.is_empty() {
        return Ok(());
    }
    let engage = host.state(ENGAGE)?;
    cai_engage::refresh_targets(&mut targets, engage, a, host)?;
    host.store_list(&targets);
    Ok(())
}

/// The Kill arm of Mind +0x144 `0x5222E0`: at the target it engages (Mind +0xF0), and
/// what cannot be engaged is gone (Mind +0x140); short of it the Move has failed
/// (Mind +0x124). Chasing and the other states think again in 1e-4. False: not these.
pub(super) fn reached(host: &mut Regiment<'_>, a: &mut Arena) -> aa_mind::Result<bool> {
    // 0x522567: at the rout's point. Still exhausted the Exhausted state, else the Reform.
    if host.is_top(a, ROUT) {
        if morale_state(host) == 0 {
            exhausted_begin(host, a)?;
        } else {
            let reform = host.state(REFORM)?;
            aa_mind::activate(reform, 1, a, host)?;
        }
        return Ok(true);
    }
    let kill = host.state(KILL)?;
    if a.state(kill)?.status == 2 {
        let g = host.word(KILL_TARGET);
        let Some(t) = resolve_gtid(host.world, g) else { return Ok(true) };
        if host.thing_in_range(t) {
            if !engage_begin(host, t, a)? {
                target_gone(host, g, a)?;
            }
            return Ok(true);
        }
        host.move_failed(a)?;
        return Ok(true);
    }
    if host.is_top(a, CHASING) {
        think_in(host.world, host.o, f32::from_bits(IMMEDIATE_BITS));
        return Ok(true);
    }
    // 0x51E1B0: the Move leaves, then the next update (0x52245F) in 0.6 s.
    if host.is_top(a, EXPLORE) {
        let mv = host.state(MOVE)?;
        if a.state(mv)?.status == 2 {
            aa_mind::World::leave(host, mv, a)?;
        }
        think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
        return Ok(true);
    }
    // 0x52243C: at the stand point the Move leaves, a Think in 0.6 s.
    if host.is_top(a, GUARD) {
        let (bombard, mv) = (host.state(BOMBARD)?, host.state(MOVE)?);
        if a.state(bombard)?.status != 2 && a.state(mv)?.status == 2 {
            aa_mind::World::leave(host, mv, a)?;
        }
        think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
        guard_reflect(host, a);
        return Ok(true);
    }
    Ok(false)
}

/// `0x520440(0, point, 0, auxiliary)`: the embedded Move at a point, from a new Move's
/// fields (a target given keeps `start` from activating it).
fn walk_point(host: &mut Regiment<'_>, to: Vec2, auxiliary: [u32; 2], a: &mut Arena) -> aa_mind::Result<()> {
    let o = host.o;
    let any = alive_members(host.world, o).next();
    if let Some(any) = any {
        start(host.ms, host.world, Owner::Org(o), to, Vec2::ZERO, Some(any), false, None, None);
        host.order().target = None;
    }
    let speed = host.order().speed.to_bits();
    host.activate_move_fields(to, 0, speed, auxiliary, a)?;
    Ok(())
}

/// `0x5249A0`: with flag bit 4 and a scalar of 1.0 and the state active, a new waypoint;
/// else `0x524100`: a coordinate below 1 is 1, the waypoint and the target there, kind 1,
/// the target scored (`0x5213E0`), the state in mode 2.
pub(in crate::movement) fn explore_command(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, at: Vec2, scalar: Option<f32>) -> bool {
    let result = with_regiment(ms, world, rules, o, |host, a| -> aa_mind::Result<bool> {
        let explore = host.state(EXPLORE)?;
        let at = Vec2::new(if at.x < 1.0 { 1.0 } else { at.x }, if at.y < 1.0 { 1.0 } else { at.y });
        let [x, y] = bits(at);
        if scalar == Some(1.0) && a.state(explore)?.status == 2 {
            host.set_word(EXPLORE_POINT, x);
            host.set_word(EXPLORE_POINT + 4, y);
            return Ok(true);
        }
        host.set_word(EXPLORE_POINT, x);
        host.set_word(EXPLORE_POINT + 4, y);
        let mut target = explore_port::Candidate { x, y, score: 0, count: 0, word: 1 };
        explore_score(host, &mut target, 0, a)?;
        for (k, w) in target.words().into_iter().enumerate() {
            host.set_word(EXPLORE_TARGET + 4 * k as u32, w);
        }
        if let Some(s) = host.combat_org() {
            s.task = None;
            s.task_order = None;
        }
        aa_mind::activate(explore, 2, a, host)
    });
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("regiment {o} explore: {e}"));
            false
        }
    }
}

/// `0x5213E0`: the candidate's fog count (2 unexplored, 1 grey, 0 seen) and its score. It
/// moves to the first open place (`0x4F46E0`, radius 2); no place, or a count below the
/// best's, refuses it. **Rendition:** `ExploreFinishAreaBonus` (the regions of `0x7AA3D0`)
/// is not added.
fn explore_score(host: &mut Regiment<'_>, c: &mut explore_port::Candidate, best_count: u32, _: &mut Arena) -> aa_mind::Result<()> {
    let refuse = |c: &mut explore_port::Candidate| {
        c.x = 0xbf80_0000;
        c.y = 0xbf80_0000;
        c.count = 0;
    };
    let mut point = [c.x, c.y];
    let placed = host.leader().is_some_and(|l| route::nearest_placement(host.ms, host.world, l, &mut point, 0x4000_0000).unwrap_or(false));
    if !placed {
        refuse(c);
        return Ok(());
    }
    [c.x, c.y] = point;
    let p = vec2(point);
    let o = host.o;
    let count = crate::orders::explore::fog_count(host.world, host.world.orgs[o as usize].kingdom, p) as u32;
    c.count = count;
    if (count as i32) < best_count as i32 {
        refuse(c);
        return Ok(());
    }
    let rule = |host: &Regiment<'_>, name: &str, shipped: f32| f64::from(host.world.data.rule_ci(name).unwrap_or(shipped));
    let mut score = match c.word {
        0 => rule(host, "ExploreActorBaseValue", 40.0),
        1 => rule(host, "ExploreWaypointBaseValue", 0.0),
        _ => f64::from(f32::from_bits(c.score)),
    } as f32;
    let way = vec2([host.word(EXPLORE_POINT), host.word(EXPLORE_POINT + 4)]);
    let (dx, dy) = (f64::from(p.x) - f64::from(way.x), f64::from(p.y) - f64::from(way.y));
    score = ((dx * dx + dy * dy).sqrt() * rule(host, "ExploreDistanceMultiplier", -1.0) + f64::from(score)) as f32;
    let target = vec2([host.word(EXPLORE_TARGET), host.word(EXPLORE_TARGET + 4)]);
    let map = &host.world.map;
    let (w, h) = (map.width as f32 * map.cell_size, map.height as f32 * map.cell_size);
    if (0.0..w).contains(&target.x) && (0.0..h).contains(&target.y) {
        let own = host.world.orgs[o as usize].anchor;
        let unit = |v: Vec2| {
            let l = v.length();
            if l > 0.0001 { v / l } else { Vec2::ZERO }
        };
        let (u, v) = (unit(target - own), unit(p - own));
        let dot = f64::from(u.x) * f64::from(v.x) + f64::from(v.y) * f64::from(u.y);
        score = (dot * rule(host, "ExploreDotProductMultiplier", 20.0) + f64::from(score)) as f32;
    }
    // 0x5216D7: the getter and its draw.
    let fuzz = rule(host, "ExploreFuzzyFactor", 20.0);
    let u = crate::trace::native_unit(host.world.random_u32_at("site 5216d7"));
    c.score = ((f64::from(u) * fuzz + f64::from(score)) as f32).to_bits();
    Ok(())
}

impl explore_port::ExploreThinkWorld for Regiment<'_> {
    /// Organization +0x164. **Rendition:** a group's following has no host here.
    fn organization_word164(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn group_4dce60(&mut self, _: u32, _: MindId, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(true)
    }
    fn group_4dcf00(&mut self, _: u32, _: StateId, _: &mut Arena) -> aa_mind::Result<()> {
        Ok(())
    }
    fn group_word194(&mut self, _: u32) -> aa_mind::Result<u32> {
        Ok(0)
    }
    fn object_mind(&mut self, _: u32) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn mind_explore_target(&mut self, _: u32) -> aa_mind::Result<[u32; 5]> {
        Ok([0; 5])
    }
    fn explore_target(&mut self, _: StateId) -> aa_mind::Result<[u32; 5]> {
        Ok(std::array::from_fn(|k| self.word(EXPLORE_TARGET + 4 * k as u32)))
    }
    fn set_explore_target(&mut self, _: StateId, target: [u32; 5]) -> aa_mind::Result<()> {
        for (k, w) in target.into_iter().enumerate() {
            self.set_word(EXPLORE_TARGET + 4 * k as u32, w);
        }
        Ok(())
    }
    fn explore_point(&mut self, _: StateId) -> aa_mind::Result<[u32; 2]> {
        Ok([self.word(EXPLORE_POINT), self.word(EXPLORE_POINT + 4)])
    }
    /// `0x4282A0`.
    fn region_contains(&mut self, point: [u32; 2], _: &mut Arena) -> aa_mind::Result<bool> {
        let [w, h] = self.extents().extent_bits;
        let p = vec2(point);
        Ok((0.0..f32::from_bits(w)).contains(&p.x) && (0.0..f32::from_bits(h)).contains(&p.y))
    }
    /// `0x552640`.
    fn region_clamp(&mut self, _: StateId, _: &mut Arena) -> aa_mind::Result<()> {
        let p = vec2([self.word(EXPLORE_POINT), self.word(EXPLORE_POINT + 4)]);
        let [x, y] = bits(clamp_extent(self.world, p));
        self.set_word(EXPLORE_POINT, x);
        self.set_word(EXPLORE_POINT + 4, y);
        Ok(())
    }
    fn region_size(&mut self) -> aa_mind::Result<[u32; 2]> {
        Ok(self.extents().extent_bits)
    }
    fn organization_position(&mut self, _: MindId) -> aa_mind::Result<[u32; 2]> {
        Ok(bits(self.world.orgs[self.o as usize].anchor))
    }
    fn explore_radius(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.data.rule_ci("ExploreInertiaCloseEnough").unwrap_or(3.0).to_bits())
    }
    fn candidate_counts(&mut self) -> aa_mind::Result<[u32; 2]> {
        Ok([self.world.data.rule_ci("ExploreNumActorLocations").unwrap_or(20.0) as u32, self.world.data.rule_ci("ExploreNumWaypointLocations").unwrap_or(20.0) as u32])
    }
    /// `0x524170`: the table direction (`0x5241B3`) and the radius (`0x5241E7`) from the
    /// organization, scored when it is on the map.
    fn candidate_524170(&mut self, _: StateId, best_count: u32, c: &mut explore_port::Candidate, a: &mut Arena) -> aa_mind::Result<()> {
        let min = self.world.data.rule_ci("ExploreActorRadiusMin").unwrap_or(20.0);
        let max = self.world.data.rule_ci("ExploreActorRadiusMax").unwrap_or(30.0);
        let draw = self.world.random_u32_at("site 5241b3");
        let direction = aa_movement::formation::random_direction(draw);
        let u = crate::trace::native_unit(self.world.random_u32_at("site 5241e7"));
        let radius = (f64::from(u) * f64::from(max - min) + f64::from(min)) as f32;
        let own = self.world.orgs[self.o as usize].anchor;
        let x = (f64::from(direction[0]) * f64::from(radius) + f64::from(own.x)) as f32;
        let y = (f64::from(direction[1]) * f64::from(radius) + f64::from(own.y)) as f32;
        c.word = 0;
        c.x = x.to_bits();
        c.y = y.to_bits();
        let map = &self.world.map;
        let (w, h) = (map.width as f32 * map.cell_size, map.height as f32 * map.cell_size);
        if (0.0..w).contains(&x) && (0.0..h).contains(&y) {
            explore_score(self, c, best_count, a)?;
        }
        Ok(())
    }
    /// `0x5242C0`: the table direction (`0x52430B`) and a walk along the ray into the fog.
    /// **Rendition:** the walk is its bisection (`crate::orders::explore`).
    fn candidate_5242c0(&mut self, _: StateId, best_count: u32, c: &mut explore_port::Candidate, a: &mut Arena) -> aa_mind::Result<()> {
        let draw = self.world.random_u32_at("site 52430b");
        let direction = Vec2::from(aa_movement::formation::random_direction(draw));
        let way = vec2([self.word(EXPLORE_POINT), self.word(EXPLORE_POINT + 4)]);
        let o = self.o;
        let Some(p) = crate::orders::explore::ray_into_fog(self.world, o, way, direction) else {
            c.x = 0xbf80_0000;
            c.y = 0xbf80_0000;
            c.count = 0;
            return Ok(());
        };
        [c.x, c.y] = bits(p);
        c.word = 1;
        explore_score(self, c, best_count, a)
    }
    /// `0x4F46E0(organization, &candidate, 4.0)`. **Rendition:** the descriptor is the
    /// leader's.
    fn world_4f46e0(&mut self, _: MindId, c: &explore_port::Candidate, radius: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        let mut point = [c.x, c.y];
        let Some(leader) = self.leader() else { return Ok(false) };
        Ok(route::nearest_placement(self.ms, self.world, leader, &mut point, radius).unwrap_or(false))
    }
    fn null_point(&mut self) -> aa_mind::Result<[u32; 2]> {
        Ok([0, 0])
    }
    fn move_order(&mut self, _: MindId, args: [u32; 6], a: &mut Arena) -> aa_mind::Result<()> {
        let to = vec2([args[1], args[2]]);
        walk_point(self, to, [args[4], args[5]], a)?;
        let o = self.o;
        set_order(self.world, self.ms, Owner::Org(o), Order::Move { to, attack_move: false });
        Ok(())
    }
    /// `0x51E9B0(x, y)` on the running Move.
    fn move_retarget(&mut self, _: MindId, point: [u32; 2], _: &mut Arena) -> aa_mind::Result<()> {
        let extents = self.extents();
        self.run_on_fields(|fields, host| cm::set_destination(fields, point, extents, PC, host).map_err(err))?;
        let o = self.o;
        set_order(self.world, self.ms, Owner::Org(o), Order::Move { to: vec2(point), attack_move: false });
        Ok(())
    }
}

/// The embedded states of the Guard, as `0x529420` leaves them.
fn leave_embedded(host: &mut Regiment<'_>, a: &mut Arena) -> aa_mind::Result<()> {
    for offset in [KILL, BOMBARD, MOVE, SLEEP] {
        let s = host.state(offset)?;
        if a.state(s)?.status != 0 {
            aa_mind::World::leave(host, s, a)?;
        }
    }
    Ok(())
}

/// `0x51EE20`: the point of a guarded thing, from it along the direction (+0x24, else from
/// the thing to the regiment, else the regiment's facing) by its radius and half the
/// regiment's. True when the point moved (`0x51D510`: more than 0.0002 in `|dx| + |dy|`).
fn guard_stand_point(host: &mut Regiment<'_>) -> bool {
    let Some(t) = resolve_gtid(host.world, host.word(GUARDED)) else { return false };
    let o = host.o;
    let at = match t {
        TargetRef::Org(g) => host.world.orgs[g as usize].anchor,
        TargetRef::Actor(x) => pos2(host.world, x),
    };
    let aux = vec2([host.word(GUARD_AUX), host.word(GUARD_AUX + 4)]);
    let direction = if f64::from(aux.y) * f64::from(aux.y) + f64::from(aux.x) * f64::from(aux.x) <= f64::from(f32::from_bits(0x38d1_b717)) {
        let d = host.world.orgs[o as usize].anchor - at;
        let length = d.length();
        if length == 0.0 {
            let facing = host.ms.orgs[o as usize].facing;
            if facing != Vec2::ZERO { facing } else { facing_vector(host.world.orgs[o as usize].heading) }
        } else if length > 0.0001 {
            d / length
        } else {
            Vec2::ZERO
        }
    } else {
        aux
    };
    let own = host.world.data.def(host.world.orgs[o as usize].def).control_range;
    let radius = match t {
        TargetRef::Actor(x) => crate::orders::guard::shape_radius(def_of(host.world, x)),
        TargetRef::Org(g) => crate::orders::guard::shape_radius(host.world.data.def(host.world.orgs[g as usize].def)),
    };
    let reach = f64::from(own) * 0.5 + f64::from(radius);
    let p = Vec2::new((f64::from(direction.x) * reach + f64::from(at.x)) as f32, (f64::from(at.y) + f64::from(direction.y) * reach) as f32);
    let old = vec2([host.word(GUARD_POINT), host.word(GUARD_POINT + 4)]);
    if (p.x - old.x).abs() + (p.y - old.y).abs() <= 0.0002 {
        return false;
    }
    let [x, y] = bits(p);
    host.set_word(GUARD_POINT, x);
    host.set_word(GUARD_POINT + 4, y);
    let [dx, dy] = bits(direction);
    host.set_word(SLEEP_POINT, dx);
    host.set_word(SLEEP_POINT + 4, dy);
    true
}

/// What the rest of the game reads of the Guard's action: the walk, the kill or rest.
fn guard_reflect(host: &mut Regiment<'_>, a: &Arena) {
    let o = host.o;
    let (Ok(kill), Ok(mv)) = (host.state(KILL), host.state(MOVE)) else { return };
    let order = if a.state(kill).is_ok_and(|s| s.status == 2) {
        resolve_gtid(host.world, host.word(KILL_TARGET)).and_then(|t| combat::representative(host.world, t)).map_or(Order::Idle, |t| Order::Attack { target: t })
    } else if a.state(mv).is_ok_and(|s| s.status == 2) {
        Order::Move { to: host.order().destination, attack_move: false }
    } else {
        Order::Idle
    };
    if host.world.orgs[o as usize].order != order {
        set_order(host.world, host.ms, Owner::Org(o), order);
    }
}

/// `0x51F360`: `0x51DC80` on a thing, `0x51DD30` on a point. The same guard again only
/// wakes a Mind with no Think to come, in 0.6 s. False when refused.
pub(in crate::movement) fn guard_command(ms: &mut MovementState, world: &mut World, rules: MoveRules, o: OrgId, target: Option<TargetRef>, at: Vec2, aux: Vec2) -> bool {
    let result = with_regiment(ms, world, rules, o, |host, a| -> aa_mind::Result<bool> {
        let guarded = target.map_or(0, gtid);
        let [ax, ay] = bits(aux);
        let current = host.is_top(a, GUARD) && host.word(GUARDED) == guarded;
        if current {
            // A saved Guard comes back through its command.
            let o = host.o;
            host.world.orders.orgs.insert(o, crate::orders::MindState::Guard);
        }
        if let Some(t) = target {
            if current {
                if !host.world.is_scheduled(Sched::OrgMind(host.o)) {
                    think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
                }
                return Ok(true);
            }
            if t == TargetRef::Org(host.o) {
                return Ok(false);
            }
        } else {
            host.world.books.minds.set_byte(key(host.o), GUARD_PLACED, 0);
            host.set_word(GUARD_AUX, ax);
            host.set_word(GUARD_AUX + 4, ay);
            if current && vec2([host.word(GUARD_POINT), host.word(GUARD_POINT + 4)]) == at {
                if !host.world.is_scheduled(Sched::OrgMind(host.o)) {
                    think_in(host.world, host.o, f32::from_bits(POINT_SIX_BITS));
                }
                return Ok(true);
            }
        }
        // 0x529C00: the Rout and Exhausted states outrank it.
        if host.is_top(a, ROUT) || host.is_top(a, EXHAUSTED) {
            return Ok(false);
        }
        let point = match target {
            Some(TargetRef::Org(g)) => host.world.orgs[g as usize].anchor,
            Some(TargetRef::Actor(x)) => pos2(host.world, x),
            None => at,
        };
        let o = host.o;
        host.world.books.minds.set_byte(key(o), GUARD_PLACED, 0);
        host.set_word(GUARD_AUX, ax);
        host.set_word(GUARD_AUX + 4, ay);
        host.set_word(GUARDED, guarded);
        let [x, y] = bits(point);
        host.set_word(GUARD_POINT, x);
        host.set_word(GUARD_POINT + 4, y);
        if let Some(s) = host.combat_org() {
            s.task = None;
            s.task_order = None;
        }
        let guard = host.state(GUARD)?;
        aa_mind::activate(guard, 2, a, host)
    });
    match result {
        Ok(r) => r,
        Err(e) => {
            ms.note(format!("regiment {o} guard: {e}"));
            false
        }
    }
}

impl cai_guard::GuardThinkWorld for Regiment<'_> {
    fn guarded(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(self.word(GUARDED))
    }
    fn resolve(&mut self, g: u32) -> aa_mind::Result<Option<u32>> {
        Ok(resolve_gtid(self.world, g).map(handle))
    }
    fn object_removed(&mut self, object: u32) -> aa_mind::Result<bool> {
        Ok(!combat::target_alive(self.world, thing(object)))
    }
    fn object_gtid(&mut self, object: u32) -> aa_mind::Result<u32> {
        Ok(object.wrapping_add(1))
    }
    fn mind_virtual_140(&mut self, _: MindId, g: u32, a: &mut Arena) -> aa_mind::Result<()> {
        target_gone(self, g, a)
    }
    /// The settled test (`0x585160`) reads false here, as the home has no host.
    fn object_virtual_158(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn object_virtual_15c(&mut self, _: u32) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn denizen_component(&mut self, object: u32) -> aa_mind::Result<Option<u32>> {
        Ok(match thing(object) {
            TargetRef::Actor(x) if def_of(self.world, x).denizen.is_some() => Some(object),
            _ => None,
        })
    }
    fn component_byte20(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn object_word98(&mut self, _: u32) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn check_585160(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    /// `0x569300`. **Rendition:** the denizen's intruders are read as the hostile
    /// organizations in its guard range now (`crate::orders::guard`).
    fn component_threats(&mut self, component: u32) -> aa_mind::Result<Vec<u32>> {
        let TargetRef::Actor(x) = thing(component) else { return Ok(Vec::new()) };
        let list = crate::orders::guard::intruders(self.world, &self.world.combat, Unit::Actor(x)).unwrap_or_default();
        Ok(list.into_iter().map(|e| gtid(TargetRef::Org(e))).collect())
    }
    /// Object +0x118: who has it on an engage list.
    fn object_threats(&mut self, object: u32) -> aa_mind::Result<Vec<u32>> {
        let unit = match thing(object) {
            TargetRef::Org(g) => Unit::Org(g),
            TargetRef::Actor(x) => Unit::Actor(x),
        };
        let list = crate::orders::guard::attackers(self.world, &self.world.combat, unit);
        Ok(list.into_iter().map(|e| gtid(TargetRef::Org(e))).collect())
    }
    fn object_mind(&mut self, object: u32) -> aa_mind::Result<Option<u32>> {
        Ok(Some(object))
    }
    /// Mind +0x90: what the guarded thing is fighting.
    fn object_mind_virtual_90(&mut self, object: u32) -> aa_mind::Result<Option<u32>> {
        Ok(match thing(object) {
            TargetRef::Org(g) => self.world.combat.org(g).and_then(|s| s.current_target()).map(handle),
            TargetRef::Actor(x) => self.world.actors[x as usize].combat.target.filter(|&t| alive(self.world, t)),
        })
    }
    fn state_target(&mut self, _: MindId, offset: u32) -> aa_mind::Result<u32> {
        Ok(self.word(offset + 0x10))
    }
    fn embedded_state(&mut self, _: MindId, offset: u32) -> aa_mind::Result<StateId> {
        self.state(offset)
    }
    fn object_position(&mut self, object: u32) -> aa_mind::Result<[u32; 2]> {
        Ok(match thing(object) {
            TargetRef::Org(g) => bits(self.world.orgs[g as usize].anchor),
            TargetRef::Actor(x) => bits(pos2(self.world, x)),
        })
    }
    fn organization_position(&mut self, _: MindId) -> aa_mind::Result<[u32; 2]> {
        Ok(bits(self.world.orgs[self.o as usize].anchor))
    }
    fn guard_old_target_bias(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.data.rule_ci("GuardOldTargetBias").unwrap_or(0.0).to_bits())
    }
    fn embedded_think(&mut self, _: MindId, offset: u32, a: &mut Arena) -> aa_mind::Result<()> {
        think_state(self, offset, a)
    }
    /// `0x4D4FF0(object, 3, 0)`. **Rendition:** a layout that fires indirect with a member
    /// that reaches the point (`crate::combat::tasks::point_relation`).
    fn relation(&mut self, _: MindId, object: u32) -> aa_mind::Result<i32> {
        let Some(rep) = combat::representative(self.world, thing(object)) else { return Ok(1) };
        let at = pos2(self.world, rep);
        let o = self.o;
        let reaches = crate::orders::guess::bombard_layout(self.world, o) && self.world.orgs[o as usize].members.iter().any(|&m| alive(self.world, m) && combat::tasks::point_relation(self.world, m, at) == 0);
        Ok(i32::from(!reaches))
    }
    /// `0x527CE0(mode, object)`.
    fn guard_527ce0(&mut self, _: StateId, mode: u32, object: u32, a: &mut Arena) -> aa_mind::Result<()> {
        leave_embedded(self, a)?;
        let o = self.o;
        match mode {
            1 => {
                let sleep = self.state(SLEEP)?;
                aa_mind::activate(sleep, 0, a, self)?;
            }
            2 => {
                let to = vec2([self.word(GUARD_POINT), self.word(GUARD_POINT + 4)]);
                let aux = [self.word(GUARD_AUX), self.word(GUARD_AUX + 4)];
                walk_point(self, to, aux, a)?;
            }
            3 => {
                let t = thing(object);
                let Some(rep) = combat::representative(self.world, t) else { return Ok(()) };
                let mut st = std::mem::take(&mut self.world.combat);
                combat::tasks::begin_guard_bombard(self.world, &mut st, o, crate::world::AbilityTarget::Actor(rep));
                self.world.combat = st;
                self.set_word(BOMBARD + 0x10, gtid(t));
                let bombard = self.state(BOMBARD)?;
                aa_mind::activate(bombard, 0, a, self)?;
            }
            4 => {
                if !cai_guard::GuardThinkWorld::follow_activate(self, MindId(key(o)), 0, object, a)? {
                    return Ok(());
                }
            }
            _ => {}
        }
        if mode != 1 {
            self.world.books.minds.set_byte(key(o), GUARD_PLACED, 0);
        }
        self.set_word(GUARD_ACTION, mode);
        formation_follow(self, a);
        guard_reflect(self, a);
        Ok(())
    }
    /// `0x51D920(mode, object)`.
    fn follow_activate(&mut self, _: MindId, mode: u32, object: u32, a: &mut Arena) -> aa_mind::Result<bool> {
        let t = thing(object);
        if !combat::kill_accepts_thing(self.world, self.o, t) {
            return Ok(false);
        }
        self.set_word(KILL_TARGET, gtid(t));
        let kill = self.state(KILL)?;
        aa_mind::activate(kill, mode, a, self)
    }
    /// Mind +0xB8 `0x51E6C0`: the state is reported again.
    fn mind_virtual_b8(&mut self, _: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        formation_follow(self, a);
        guard_reflect(self, a);
        Ok(())
    }
    fn set_guard_word10(&mut self, _: StateId, value: u32) -> aa_mind::Result<()> {
        self.set_word(GUARD_ACTION, value);
        Ok(())
    }
    fn guard_byte14(&mut self, _: StateId) -> aa_mind::Result<bool> {
        Ok(self.world.books.minds.byte(key(self.o), GUARD_PLACED) != 0)
    }
    fn set_guard_byte14(&mut self, _: StateId, value: bool) -> aa_mind::Result<()> {
        let o = self.o;
        self.world.books.minds.set_byte(key(o), GUARD_PLACED, u8::from(value));
        Ok(())
    }
    /// The home's settling: no home has a host.
    fn home_settle(&mut self, _: Option<u32>, _: u32, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
    fn guard_51ee20(&mut self, _: StateId, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(guard_stand_point(self))
    }
}

/// The fighting state on top of the Mind, for `crate::morale`'s state table.
pub(crate) fn top_state(world: &World, o: OrgId) -> Option<u32> {
    world.books.minds.top_offset(key(o)).filter(|t| matches!(*t, KILL | CHASING | ENGAGE | REFORM))
}

/// The Mind's state as the rest of the game reads it: `Order` and the fields of
/// `OrgCombat`. Mind +0x1A4 after every change of the stack.
pub(super) fn reflect_order(host: &mut Regiment<'_>, a: &Arena) {
    let o = host.o;
    let i = o as usize;
    let mind = MindId(key(o));
    let Some(s) = host.world.combat.orgs.get(i) else {
        // No combat pass has seen this regiment yet: it can only have come to rest.
        let sleeping = host.state(SLEEP).ok().is_some_and(|sleep| a.mind(mind).ok().and_then(|m| m.stack.front().copied().flatten()) == Some(sleep));
        if sleeping && matches!(host.world.orgs[i].order, Order::Move { .. } | Order::Attack { .. }) {
            set_order(host.world, host.ms, Owner::Org(o), Order::Idle);
        }
        return;
    };
    if s.routing.is_some() || s.task.is_some() {
        return;
    }
    let kill = host.state(KILL).ok().filter(|k| in_stack(a, mind, *k)).and_then(|_| resolve_gtid(host.world, host.word(KILL_TARGET)));
    let rep_of = |w: &World, t: Option<TargetRef>| t.and_then(|t| combat::representative(w, t));
    let top = a.mind(mind).ok().and_then(|m| m.stack.front().copied().flatten()).and_then(|st| host.world.books.minds.owner(st)).map(|x| x.1);
    let tail = host.world.combat.orgs[i].current_target();
    let wanted = match top {
        Some(ENGAGE) => rep_of(host.world, tail).or_else(|| rep_of(host.world, kill)).map(|t| Order::Attack { target: t }),
        Some(CHASING) => rep_of(host.world, resolve_gtid(host.world, host.word(CHASING_TARGET))).map(|t| Order::Attack { target: t }),
        // The move a fight broke off goes on after the Reform: it stays what is shown.
        Some(REFORM) => Some(match s.saved_order.clone() {
            Some(order @ Order::Move { .. }) => order,
            _ if matches!(host.world.orgs[i].order, Order::Move { .. }) && s.auto_order.is_none() => host.world.orgs[i].order.clone(),
            _ => Order::Idle,
        }),
        _ => None,
    };
    let current = host.world.orgs[i].order.clone();
    let s = &mut host.world.combat.orgs[i];
    s.explicit = kill;
    if let Some(order) = wanted {
        if s.auto_order.is_none() {
            s.saved_order = Some(current);
        }
        s.auto_order = Some(order.clone());
        set_order(host.world, host.ms, Owner::Org(o), order);
        return;
    }
    if matches!(top, Some(ENGAGE | CHASING)) {
        return;
    }
    let had_auto = s.auto_order.take().is_some();
    let saved = s.saved_order.take();
    let order = match top {
        Some(KILL) => rep_of(host.world, kill).map(|t| Order::Attack { target: t }),
        Some(SLEEP) if !matches!(current, Order::Deploy { .. }) && !crate::orders::guarding(host.world, o) => Some(Order::Idle),
        Some(MOVE) if had_auto => saved,
        _ if had_auto => saved.or(Some(Order::Idle)),
        _ => None,
    };
    if let Some(order) = order
        && order != current
    {
        set_order(host.world, host.ms, Owner::Org(o), order);
    }
}

impl EngageWorld for Regiment<'_> {
    fn resolve(&mut self, g: u32) -> aa_mind::Result<Option<u32>> {
        Ok(resolve_gtid(self.world, g).map(handle))
    }
    fn gtid(&mut self, actor: u32) -> aa_mind::Result<u32> {
        Ok(actor.wrapping_add(1))
    }
    fn removed(&mut self, actor: u32) -> aa_mind::Result<bool> {
        Ok(!combat::target_alive(self.world, thing(actor)))
    }
    fn attached_organization(&mut self, actor: u32) -> aa_mind::Result<Option<u32>> {
        let t = thing(actor);
        Ok(match self.organization_of(t) {
            TargetRef::Org(o) if !matches!(t, TargetRef::Org(_)) => Some(ORG | o),
            _ => None,
        })
    }
    fn can_target(&mut self, target: u32) -> aa_mind::Result<bool> {
        Ok(combat::mind_can_target(self.world, &self.world.combat, self.o, thing(target)))
    }
    fn relation(&mut self, target: u32) -> aa_mind::Result<i32> {
        Ok(i32::from(!combat::any_member_ability(self.world, &self.world.combat, self.o, thing(target))))
    }
    /// `0x4D3990`, `0x4D0CD0`: `OrgCombat::engaged_by` is drawn from the lists.
    fn register_attacker(&mut self, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
    fn unregister_attacker(&mut self, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
    fn has_component(&mut self) -> aa_mind::Result<bool> {
        Ok(true)
    }
    /// `0x57A3C0`: the list's first target tells the player (`0x42D490`), view only.
    fn component_target_added(&mut self, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
    /// `0x579BA0`: the view's banner list.
    fn component_update(&mut self) -> aa_mind::Result<()> {
        Ok(())
    }
    fn priority(&mut self, target: u32) -> aa_mind::Result<i32> {
        Ok(self.target_priority(thing(target)))
    }
    /// Reached with the Engage state not current, which the Think does not do.
    fn engage(&mut self, target: u32) -> aa_mind::Result<bool> {
        let t = thing(target);
        self.with_arena(|host, a| engage_begin(host, t, a))
    }
    fn chasing_target(&mut self) -> aa_mind::Result<u32> {
        Ok(self.word(CHASING_TARGET))
    }
    /// Organization +0x148[4], the ZOC.
    fn organization_entry4(&mut self) -> aa_mind::Result<Option<u32>> {
        Ok(self.world.fog.engine.as_ref().is_some_and(|e| e.has_zoc(self.o)).then_some(1))
    }
    /// Target virtual +0x130(zone, 1): `0x4EA2C0`.
    fn target_check130(&mut self, target: u32, _: u32) -> aa_mind::Result<bool> {
        Ok(combat::thing_in_zoc(self.world, self.o, thing(target), true))
    }
    fn target_word90(&mut self, _: u32) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn related(&mut self, a: u32, b: u32) -> aa_mind::Result<bool> {
        Ok(combat::thing_attacks(self.world, &self.world.combat, thing(a), thing(b)))
    }
    fn organization(&mut self) -> aa_mind::Result<u32> {
        Ok(ORG | self.o)
    }
    fn component_accepts(&mut self, target: u32) -> aa_mind::Result<bool> {
        Ok(combat::zone_accepts(self.world, &self.world.combat, self.o, thing(target)))
    }
}

impl cai_engage_think::EngageThinkWorld for Regiment<'_> {
    fn organization_status(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<u32> {
        let i = self.o as usize;
        Ok(self.world.combat.org(self.o).map_or(2, |s| u32::from(crate::morale::state(s, self.world.orgs[i].morale))))
    }
    fn organization_word118(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<bool> {
        Ok(self.world.combat.org(self.o).is_some_and(|s| s.is_engaged()))
    }
    /// `0x51F920`: the rout state in mode 2.
    fn wavering_reaction(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<()> {
        self.with_arena(|host, a| {
            let rout = host.state(ROUT)?;
            aa_mind::activate(rout, 2, a, host).map(|_| ())
        })
    }
    fn mind_virtual_140(&mut self, _: MindId, g: u32, _: &mut Arena) -> aa_mind::Result<()> {
        self.with_arena(|host, a| target_gone(host, g, a))
    }
    fn mind_virtual_4c(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<Option<u32>> {
        let current = self.with_arena(|host, a| current_target(host, a));
        Ok(current.map(handle))
    }
    fn last_attacker(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(self.word(LAST_ATTACKER))
    }
    fn set_last_attacker(&mut self, _: StateId, g: u32) -> aa_mind::Result<()> {
        self.set_word(LAST_ATTACKER, g);
        Ok(())
    }
    fn kill_target_is_highest_priority(&mut self) -> aa_mind::Result<bool> {
        Ok(self.world.combat.rules.as_deref().is_none_or(|r| r.kill_target_highest))
    }
    fn within_engage_range(&mut self, _: MindId, actor: u32, _: &Arena) -> aa_mind::Result<bool> {
        Ok(combat::within_operating(self.world, &self.world.combat, self.o, thing(actor)))
    }
    fn member_count(&mut self, _: MindId, _: &Arena) -> aa_mind::Result<i32> {
        Ok(self.world.orgs[self.o as usize].members.len() as i32)
    }
    fn member_slot(&mut self, _: MindId, index: i32, _: &Arena) -> aa_mind::Result<Option<u32>> {
        Ok(usize::try_from(index).ok().and_then(|k| self.world.orgs[self.o as usize].members.get(k).copied()))
    }
    fn member_mind(&mut self, member: u32) -> aa_mind::Result<Option<u32>> {
        Ok(Some(member))
    }
    /// Member Mind +0x17C `0x52DB10`.
    fn member_mind_virtual_17c(&mut self, member: u32) -> aa_mind::Result<()> {
        element::fight::reconsider(self.world, member);
        Ok(())
    }
    fn chase_continue(&mut self, _: MindId, g: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(self.with_arena(|host, a| chase_continues(host, g, a)))
    }
    fn mind_virtual_f8(&mut self, _: MindId, actor: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        let t = (actor != 0).then(|| thing(actor));
        self.with_arena(|host, a| chase_begin(host, t, a))
    }
    /// Member Mind +0x6C `0x535E50`: its Bombard is active.
    fn member_mind_virtual_6c(&mut self, member: u32) -> aa_mind::Result<bool> {
        Ok(self.world.combat.actor(member).is_some_and(|c| matches!(c.task, Some(combat::tasks::Task::Bombard(_)))))
    }
    /// Member +0x100, for bit 9 `mind_reattaching`.
    fn member_flags100(&mut self, member: u32) -> aa_mind::Result<u32> {
        Ok(if self.ms.actors[member as usize].reattaching { 0x200 } else { 0 })
    }
    /// Member Mind +0xF0 `0x532070`.
    fn member_mind_virtual_f0(&mut self, member: u32, target: u32) -> aa_mind::Result<()> {
        let rules = self.rules;
        element::fight::engage_event(self.ms, self.world, rules, member, thing(target));
        Ok(())
    }
    fn member_e4_status(&mut self, member: u32) -> aa_mind::Result<Option<u32>> {
        Ok(Some(if self.world.combat.actor(member).is_some_and(|c| c.performer.running()) { 2 } else { 0 }))
    }
    fn member_mind_virtual_4c(&mut self, member: u32) -> aa_mind::Result<Option<u32>> {
        Ok(self.world.actors[member as usize].combat.target)
    }
    fn organization_follows(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn set_all_engaged(&mut self, _: StateId, _: bool) -> aa_mind::Result<()> {
        Ok(())
    }
}

impl siege_target::SiegeTargetWorld for Regiment<'_> {
    fn object_word90(&mut self, _: u32) -> aa_mind::Result<u32> {
        Ok(0)
    }
    /// No wall thing is hosted: no site.
    fn object_4d11d0(&mut self, _: u32, _: &mut Arena) -> aa_mind::Result<Option<u32>> {
        Ok(None)
    }
    fn organization_virtual_16c(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(false)
    }
    fn component_57a500(&mut self, _: MindId, object: u32, _: bool, _: bool, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(combat::kill_accepts_thing(self.world, self.o, thing(object)))
    }
    fn set_siege_words(&mut self, _: StateId, _: u32, _: u32) -> aa_mind::Result<()> {
        Ok(())
    }
}

impl kill_port::KillThinkWorld for Regiment<'_> {
    fn kill_target(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(self.word(KILL_TARGET))
    }
}

impl reform_port::ReformThinkWorld for Regiment<'_> {
    fn game_time(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.clock.begin.0)
    }
    fn reform_start(&mut self, _: StateId) -> aa_mind::Result<u32> {
        Ok(self.word(REFORM_START))
    }
    fn reform_timeout(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.combat.rules.as_deref().map_or(10.0f32, |r| r.reform_time).to_bits())
    }
    /// Mind +0x178 `0x522B90`, Reform current: +0x11C.
    fn mind_virtual_178(&mut self, mind: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        reform_port::ReformThinkWorld::mind_virtual_11c(self, mind, a)
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
    fn organization_move(&mut self, _: MindId, point: [u32; 2], _: &mut Arena) -> aa_mind::Result<()> {
        self.leader();
        let f = vec2(point);
        if f != Vec2::ZERO {
            set_org_facing(self.ms, self.world, self.o, f);
        }
        Ok(())
    }
    fn component_word40(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(self.leader().map_or(u32::MAX, |l| l))
    }
    /// `0x4CEDE0` on the leader: its Mover is its active component.
    fn check_4cede0(&mut self, object: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        Ok(self.ms.actors.get(object as usize).is_some_and(|m| m.mover.active))
    }
    /// Mind +0x11C `0x51E680`: Reform current, it ends; else a Think in 1 s.
    fn mind_virtual_11c(&mut self, mind: MindId, a: &mut Arena) -> aa_mind::Result<()> {
        if self.is_top(a, REFORM) {
            aa_mind::pop(mind, a, self)?;
            return Ok(());
        }
        think_in(self.world, self.o, f32::from_bits(ONE_BITS));
        Ok(())
    }
}
