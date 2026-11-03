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
//! false, so the all-engaged byte stays clear (nothing here reads it). The rout, exhausted
//! and guard states are still `crate::morale` and `crate::orders`: what they do to the
//! stack is `stand_down`.
use super::*;
use crate::combat::{self, TargetRef};
use aa_mind::cai_engage::{self, EngageWorld, TargetNode, Targets};
use aa_mind::cai_engage_think;
use aa_mind::cai_thinks::{kill as kill_port, reform as reform_port, siege_target};

pub(in crate::movement) const KILL: u32 = cai_thinks::KILL_OFFSET;
pub(in crate::movement) const CHASING: u32 = 0x120;
pub(in crate::movement) const REFORM: u32 = 0x1b8;
pub(in crate::movement) const ENGAGE: u32 = 0x1cc;
/// Kill +0x10, Chasing +0x10 and Reform +0x10 as Mind offsets; Engage +0x20.
const KILL_TARGET: u32 = KILL + 0x10;
const CHASING_TARGET: u32 = CHASING + 0x10;
const REFORM_START: u32 = REFORM + 0x10;
const LAST_ATTACKER: u32 = ENGAGE + 0x20;

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
    /// `0x51FBC0`: within `CAITargetInRangeZOCMultiplier` times the zone radius.
    fn thing_in_range(&self, t: TargetRef) -> bool {
        let Some(rep) = combat::representative(self.world, t) else { return false };
        let o = self.o as usize;
        let mut zone = self.world.data.def(self.world.orgs[o].def).control_range;
        if self.world.combat.org(self.o).is_some_and(|c| c.exhausted) {
            zone *= 0.75;
        }
        let p = self.world.actors[rep as usize].pos.truncate();
        (self.world.orgs[o].anchor - p).length() <= self.rules.target_in_range_zoc * zone
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
    // A rout has begun in this update (`crate::morale`): 0x52A810(rout, 2).
    if matches!(offset, KILL | CHASING | REFORM | ENGAGE) && host.world.combat.org(host.o).is_some_and(|c| c.routing.is_some()) {
        return Some(host.state(SLEEP).and_then(|sleep| aa_mind::activate(sleep, 2, a, host).map(|_| ())));
    }
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
        _ => return None,
    })
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
    !combat::thing_in_zoc(host.world, host.o, t)
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
pub(super) fn target_gone(host: &mut Regiment<'_>, _g: u32, a: &mut Arena) -> aa_mind::Result<()> {
    let mind = MindId(key(host.o));
    let immediate = f32::from_bits(IMMEDIATE_BITS);
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
        let keep: Vec<_> = host.list().nodes.into_iter().filter(|n| resolve_gtid(host.world, n.gtid).is_some_and(|t| combat::thing_in_zoc(host.world, host.o, t))).collect();
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
    Ok(false)
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
        Ok(combat::thing_in_zoc(self.world, self.o, thing(target)))
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
        let o = self.o;
        let mut st = std::mem::take(&mut self.world.combat);
        crate::morale::begin_rout(self.world, &mut st, o);
        self.world.combat = st;
        self.with_arena(|host, a| {
            let sleep = host.state(SLEEP)?;
            aa_mind::activate(sleep, 2, a, host).map(|_| ())
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
