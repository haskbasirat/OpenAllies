//! Morale loss under fire, the breakpoint, routing and regrouping,
//! entrenchment, attack efficiency and experience. Owner: combat.
//!
//! Plain `f32` rendition of the exact ports (read them for the native detail):
//! - aa_combat::morale: state `578140` (0 exhausted at morale 0, 1 wavering
//!   at or below the breakpoint, 2 steady), gain `5786D0`, loss `579660` and
//!   its transition reactions (`51F9F0` exhausted, `51F920` wavering, both
//!   in aa_mind::cai_reactions), slayer `578F10`, breaker `579010`, retreat
//!   penalty `579AB0`, the morale tick `57A0A0` (MoraleReplenish +
//!   MoraleSupplyBonus while the Mind state allows morale_replenish; the
//!   engagement loss `((FactorMax-1)*(1-living/slots)+1)*Loss*dt` while the
//!   organization is engaged by an enemy), attack efficiency `5785D0`,
//!   engagement experience `579600`, rout handler `4D3FA0`.
//! - aa_combat::organization: member removal `57CD70` (MoraleDeathFactor *
//!   member morale_value, called from combat.rs), the entrenchment check of
//!   `57C8E0` (entrenchment_time / ENTRENCHMENT_RATE in an entrenchable state).
//! - The rout (`51F190`, `521240`, `52B530`) and the exhausted state (`521AF0`, `51E260`)
//!   are on the regiment Mind (`crate::movement`'s fight, `aa_mind::cai_rout`,
//!   `aa_mind::rout_select`); here are the reactions that begin them, through
//!   `crate::movement::mind_morale_reaction` (`51F920`, `51F9F0`), and the rout handler
//!   `4D3FA0` its Enter calls ([`rout_handler`]).
//! - aa_body::experience_amounts (`571820/5718B0/5718E0`), aa_body::experience
//!   award routing `4CE770` and aa_body::promotion `571A60` (every crossed
//!   level in list order).
//!
//! The arithmetic follows the ports' x87 chains at PC53 with the original
//! binary32 stores (`combat::x87`).
use crate::combat::{
    self, CombatState, EPSILON, TargetRef,
    x87::{self, load as ld, store as fstp},
};
use crate::world::{Event, Order, OrgId, World};
use crate::DT;

pub fn tick(world: &mut World) {
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    for id in 0..world.orgs.len() as OrgId {
        org_tick(world, &mut st, id);
    }
    world.combat = st;
    // 57C400 for the Mind states not on the hosted stack.
    for id in 0..world.orgs.len() as OrgId {
        if world.orgs[id as usize].alive {
            crate::movement::formation_follow(world, id);
        }
    }
}

/// The Mind state descriptor of actor state group 4 (actor_states.tgi /
/// actor_states_aa.tgi): morale_replenish, broken formation (the layout's
/// BrokenProperty, ATTACK_EFFICIENCY_DESIRED x0.5) and entrenchable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub morale_replenish: bool,
    pub broken: bool,
    pub entrenchable: bool,
}

pub fn descriptor(world: &World, st: &CombatState, id: OrgId) -> Descriptor {
    let d = |morale_replenish, broken, entrenchable| Descriptor { morale_replenish, broken, entrenchable };
    // 4D49B0(4): the regiment's exclusive Mind state. Actor state 5
    // (mind_exhausted, set by the morale at zero) is outside group 4 (0x474270
    // gives it another group, 0x477FC0). The exhausted Mind state (Mind +270,
    // vtable 0xA32AEC) reports actor state 0xC at vtable +0x20 (0x7C5440):
    // mind_resting, so a regiment in it reads as resting below.
    match state_in(world, st, id) {
        RegimentState::Rout => d(false, true, false), // mind_routing
        RegimentState::Engage => d(false, false, false), // mind_engaged
        RegimentState::Reform => d(true, false, false), // mind_reforming (DESIRED formation)
        // 51DC20: a Guard state walking reports its Move state, mind_moving.
        RegimentState::Guard if crate::movement::org_move_active(world, id) => d(false, false, false),
        RegimentState::Guard => d(true, false, true), // mind_guarding (entrenchable)
        RegimentState::Sleep | RegimentState::Exhausted => d(true, false, true), // mind_resting
        // mind_unpacking, mind_moving, mind_attacking, mind_bombard, ...: none replenishes.
        _ => d(false, false, false),
    }
}

/// The regiment Mind's current state, as far as aa-play holds it: the states the
/// fight traces name by vtable (`ms=`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegimentState {
    Sleep,
    Move,
    Kill,
    Chasing,
    Engage,
    Guard,
    Explore,
    Rout,
    Reform,
    Exhausted,
    Unpack,
    Pack,
    Bombard,
    Repair,
    Construct,
    Siege,
    Join,
    Rampage,
    Regroup,
}

impl RegimentState {
    /// The state of a Mind state's vtable: `None` for one that is not a regiment's.
    pub fn from_vtable(vtable: u32) -> Option<Self> {
        Some(match vtable {
            0xA329F0 => Self::Sleep,
            0xA33804 => Self::Move,
            0xA32B5C => Self::Kill,
            0xA32B80 => Self::Chasing,
            0xA33140 => Self::Engage,
            0xA32BC8 => Self::Guard,
            0xA32BEC => Self::Explore,
            0xA32AC8 => Self::Rout,
            0xA32A80 => Self::Reform,
            0xA32AEC => Self::Exhausted,
            0xA32B10 => Self::Unpack,
            0xA32B34 => Self::Pack,
            0xA3348C => Self::Bombard,
            0xA32BA4 => Self::Repair,
            0xA32A38 => Self::Construct,
            0xA32CC8 => Self::Siege,
            0xA32EC4 => Self::Join,
            0xA32A14 => Self::Rampage,
            0xA32A5C => Self::Regroup,
            _ => return None,
        })
    }
}

/// The hosted Mind's Unpack state (Mind +0x280) is on top of the stack.
fn unpack_current(world: &World, id: OrgId) -> bool {
    world.books.minds.top_offset(crate::minds::ORGANIZATION_MIND | id) == Some(0x280)
}

/// The regiment's current Mind state, read from what holds it: `crate::combat`
/// (rout, reform, exhausted, engage, tasks), `crate::orders`, the hosted Mind.
pub fn regiment_state(world: &World, id: OrgId) -> RegimentState {
    state_in(world, &world.combat, id)
}

/// [`regiment_state`] while the combat state is taken out of the world.
fn state_in(world: &World, st: &CombatState, id: OrgId) -> RegimentState {
    let Some(o) = st.org(id) else { return RegimentState::Sleep };
    if o.routing.is_some() {
        return RegimentState::Rout;
    }
    if o.exhausted_state {
        return RegimentState::Exhausted;
    }
    if o.reform.is_some() {
        return RegimentState::Reform;
    }
    // The regiment Mind's fighting states.
    match crate::movement::fight_state(world, id) {
        Some(crate::movement::FightState::Engage) => return RegimentState::Engage,
        Some(crate::movement::FightState::Chasing) => return RegimentState::Chasing,
        Some(crate::movement::FightState::Reform) => return RegimentState::Reform,
        Some(crate::movement::FightState::Kill) => return RegimentState::Kill,
        None => {}
    }
    if o.engaging() {
        return RegimentState::Engage;
    }
    match o.task {
        Some(combat::tasks::Task::Bombard(_)) => return RegimentState::Bombard,
        Some(combat::tasks::Task::Repair(_)) => return RegimentState::Repair,
        Some(combat::tasks::Task::Construct { .. }) => return RegimentState::Construct,
        None => {}
    }
    if crate::orders::guarding(world, id) {
        return RegimentState::Guard;
    }
    if crate::orders::exploring(world, id) {
        return RegimentState::Explore;
    }
    if unpack_current(world, id) {
        return RegimentState::Unpack;
    }
    match world.orgs[id as usize].order {
        Order::Idle => RegimentState::Sleep,
        Order::Attack { .. } | Order::Move { attack_move: true, .. } => RegimentState::Kill,
        // A retreat is the Move state too: its command's kind 7 makes it report
        // mind_retreating (0x520250).
        Order::Move { .. } => RegimentState::Move,
        Order::Deploy { .. } => RegimentState::Unpack,
    }
}

/// `578140`: at or below the breakpoint, 0 at no morale or with the Exhausted state on
/// the stack (Mind `+5C` `51D450`), else 1. (A busy Mind, +28 and +2C, is not modelled.)
pub fn state(o: &combat::OrgCombat, morale: f32) -> u8 {
    if o.morale_max <= 0.0 || morale > o.breakpoint {
        2
    } else if morale == 0.0 || o.exhausted_state {
        0
    } else {
        1
    }
}

pub(crate) fn level<'a>(world: &World, st: &'a CombatState, id: OrgId) -> Option<&'a combat::ExperienceLevel> {
    let s = &st.orgs[id as usize];
    st.extras().def(world.orgs[id as usize].def).levels.get(s.level)
}

/// `578300` +60: over the layout's slots with a definition, `45D580` of the
/// slot's definition (+2C0), through the member's modifiers (`501C90`) when the slot
/// holds one; then the sum through the organization's own (the level's MORALE add
/// reaches both: a level 1 regiment of five members reads the plain sum plus six
/// times the add). The morale keeps its ratio to the maximum.
pub(crate) fn morale_maximum(world: &World, st: &CombatState, id: OrgId) -> f32 {
    let i = id as usize;
    let def = world.data.def(world.orgs[i].def);
    let add = level(world, st, id).map_or(0.0, |l| l.morale);
    let clamp = st.rules().clamp_add;
    let through = |base: f32, extra: f32| crate::zones::clamp_add(base, add + extra, clamp);
    let own: f32 = def
        .properties
        .iter()
        .filter_map(|p| world.data.property(p))
        .flat_map(|p| p.modifiers.iter())
        .filter(|m| m.kind.eq_ignore_ascii_case("ADD") && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("MORALE")))
        .map(|m| m.amount)
        .sum();
    let mut max = 0.0f32;
    if def.slots.is_empty() {
        for &m in &world.orgs[i].members {
            max = fstp(x87::add(ld(through(world.data.def(world.actors[m as usize].def).morale_value, 0.0)), ld(max)));
        }
    } else {
        for (slot, &(d, _)) in def.slots.iter().enumerate().filter(|(_, (d, _))| *d != u32::MAX) {
            let member = world.orgs[i].members.iter().copied().find(|&m| world.actors.get(m as usize).is_some_and(|a| a.alive && !a.removed && a.slot == Some(slot as u16)));
            let value = world.data.def(d).morale_value;
            // The regiment's own properties are in its members' modifier state too:
            // a recon regiment (MORALE +3) reads 25 + 3 + 3 per living member.
            let v = match member {
                Some(m) => through(value, crate::zones::modifier_add(world, crate::zones::Key::Actor(m), "MORALE").unwrap_or(0.0) + own),
                None => value,
            };
            max = fstp(x87::add(ld(v), ld(max)));
        }
    }
    through(max, crate::zones::modifier_add(world, crate::zones::Key::Org(id), "MORALE").unwrap_or(0.0))
}

/// [`morale_maximum`] applied: `578300`'s ratio. It runs where the modifiers change
/// (`4D1350`: a member added or removed, a level).
fn refresh_morale_maximum(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let new = morale_maximum(world, st, id);
    let old = st.orgs[i].morale_max;
    if new.to_bits() == old.to_bits() {
        return;
    }
    let ratio = if old <= 0.0 { 0.0 } else { fstp(x87::div(ld(world.orgs[i].morale), ld(old))) };
    st.orgs[i].morale_max = new;
    world.orgs[i].morale = fstp(x87::mul(ld(ratio), ld(new)));
}

/// `578000` +88: `501530` of the organization's modifiers: a positive
/// ATTACK_EFFICIENCY_ACTUAL (the states mind_resting, mind_reforming, mind_chasing:
/// amount 0, so 1), else a positive ATTACK_EFFICIENCY_DESIRED (the current
/// formation's: `formation_required` of the state, `57BF50`), else 1. A state with a
/// BROKEN formation takes the layout's broken property.
fn desired_efficiency(world: &World, st: &CombatState, id: OrgId) -> f32 {
    let i = id as usize;
    let def = world.data.def(world.orgs[i].def);
    // What the state reports at its vtable +0x20 (`51D8F0`, `520250`, `51DC20`,
    // `51E2C0`, `51D9E0`), by its formation_required.
    let formation = match state_in(world, st, id) {
        // mind_routing, mind_repairing, mind_building: BROKEN.
        RegimentState::Rout | RegimentState::Repair | RegimentState::Construct => return 0.5,
        // mind_resting, mind_reforming, mind_chasing: ATTACK_EFFICIENCY_ACTUAL.
        RegimentState::Sleep | RegimentState::Exhausted | RegimentState::Reform | RegimentState::Chasing => return 1.0,
        // COMBAT: mind_engaged, mind_bombard, mind_guarding and mind_attacking.
        RegimentState::Engage | RegimentState::Bombard => def.combat_formation,
        RegimentState::Guard if !crate::movement::org_move_active(world, id) => def.combat_formation,
        RegimentState::Kill if !st.orgs[i].kill_is_move => def.combat_formation,
        // 520250: the Move state's command kind 7 reports mind_retreating.
        RegimentState::Move if crate::orders::retreating(world, id) => def.retreat_formation,
        // DESIRED: mind_moving (a move command's Kill state, `51D8F0` kind 5;
        // a Guard state walking), mind_exploring, mind_unpacking, mind_joining...
        _ => crate::player_orders::desired_formation(world, id),
    };
    let Some(mods) = formation.and_then(|f| def.formations.get(f)).and_then(|f| world.data.formation_modifiers.get(&f.id.to_ascii_lowercase())) else { return 1.0 };
    let sum: f32 = mods.iter().filter(|m| m.kind.eq_ignore_ascii_case("MULTIPLY") && m.attribute.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("ATTACK_EFFICIENCY_DESIRED"))).map(|m| m.amount).sum();
    let value = 1.0 + sum;
    if value > 0.0 { value } else { 1.0 }
}

/// Maximum morale (the elements' morale values with the level MORALE add),
/// the breakpoint and the members' maximum health.
fn refresh_level_values(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let data = world.data.clone();
    let def = data.def(world.orgs[i].def);
    let (_, breakpoint_add, health) =
        level(world, st, id).map_or((0.0, 0.0, 0.0), |l| (l.morale, l.morale_breakpoint, l.health));
    st.orgs[i].morale_max = morale_maximum(world, st, id);
    st.orgs[i].breakpoint = def.morale_breakpoint - breakpoint_add;
    let _ = health;
    for m in world.orgs[i].members.clone() {
        // (core) 502A70: the level amount adds to the handicap/technology
        // HEALTH amounts (and the enchantments') in one multiplier.
        crate::combat::refresh_max_health(world, st, m);
    }
}

/// First sight of an organization: its level from its experience (e.g. a
/// StartingExperience set by production), its maximum morale, and full
/// morale (the value written by whoever created the organization is
/// replaced: a new OrganizationComponent starts at its maximum).
pub(crate) fn init_org(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let def = world.orgs[i].def;
    let levels = &st.extras().def(def).levels;
    let xp = world.orgs[i].experience;
    st.orgs[i].level = levels.iter().rposition(|l| l.required <= xp).unwrap_or(0);
    refresh_level_values(world, st, id);
    world.orgs[i].morale = st.orgs[i].morale_max;
}

/// `5786D0`: add, clamp to the maximum, clear `mind_exhausted` unless at 0.
pub(crate) fn gain(world: &mut World, st: &mut CombatState, id: OrgId, amount: f32) {
    gain_x87(world, st, id, ld(amount));
}

/// The shared tail of `5786D0`/`578F10`/`579010` (aa_combat::morale
/// `gain_amount`): the x87 `amount` plus the morale, stored, then clamped to
/// the maximum; state 5 is cleared unless the state is exhausted.
fn gain_x87(world: &mut World, st: &mut CombatState, id: OrgId, amount: f64) {
    let i = id as usize;
    if !world.orgs[i].alive {
        return;
    }
    let max = st.orgs[i].morale_max;
    let m = &mut world.orgs[i].morale;
    *m = fstp(x87::add(amount, ld(*m)));
    if max < *m {
        *m = max;
    }
    if state(&st.orgs[i], world.orgs[i].morale) != 0 {
        st.orgs[i].exhausted = false;
    }
}

/// `579660` / `lose_amount`: subtract (not below 0); a transition to
/// exhausted or wavering triggers the Mind reaction.
pub(crate) fn lose(world: &mut World, st: &mut CombatState, id: OrgId, amount: f32) {
    let i = id as usize;
    if !world.orgs[i].alive || st.orgs[i].destroyed || amount <= 0.0 {
        return;
    }
    let before = state(&st.orgs[i], world.orgs[i].morale);
    let m = fstp(x87::sub(ld(world.orgs[i].morale), ld(amount)));
    world.orgs[i].morale = if m < 0.0 { 0.0 } else { m };
    let after = state(&st.orgs[i], world.orgs[i].morale);
    if after == 0 {
        st.orgs[i].exhausted = true;
    }
    if after != before {
        match after {
            0 => {
                crate::movement::mind_morale_reaction(world, st, id, true);
            }
            1 => {
                crate::movement::mind_morale_reaction(world, st, id, false);
            }
            _ => {}
        }
    }
}

/// `579AB0` on a retreat order out of an engagement (MoraleRetreatPenalty).
pub(crate) fn retreat_penalty(world: &mut World, st: &mut CombatState, id: OrgId) {
    let p = st.rules().morale_retreat_penalty;
    lose(world, st, id, p);
}

/// `578F10` slayer gain of the killer's organization:
/// `((victim/killer + 1) * 0.5)` clamped to [MoraleSlayerFactorMin,
/// MoraleSlayerFactorMax], times MoraleSlayerBase, all unstored.
pub(crate) fn slayer(world: &mut World, st: &mut CombatState, id: OrgId, killer_value: f32, victim_value: f32) {
    if killer_value == 0.0 {
        return;
    }
    let r = st.rules();
    let mut factor = x87::mul(x87::add(x87::div(ld(victim_value), ld(killer_value)), 1.0), 0.5);
    if factor < ld(r.morale_slayer_min) {
        factor = ld(r.morale_slayer_min);
    } else if factor > ld(r.morale_slayer_max) {
        factor = ld(r.morale_slayer_max);
    }
    let amount = x87::mul(factor, ld(r.morale_slayer_base));
    gain_x87(world, st, id, amount);
}

/// `579010` breaker gain of an attacker when its target routed: the routed
/// maximum over this maximum, clamped to [MoraleBreakerFactorMin,
/// MoraleBreakerFactorMax], times MoraleBreakerBase, unstored.
fn breaker(world: &mut World, st: &mut CombatState, id: OrgId, routed: OrgId) {
    let own = st.orgs[id as usize].morale_max;
    if own == 0.0 {
        return;
    }
    let r = st.rules();
    let mut ratio = x87::div(ld(st.orgs[routed as usize].morale_max), ld(own));
    if ratio < ld(r.morale_breaker_min) {
        ratio = ld(r.morale_breaker_min);
    } else if ratio > ld(r.morale_breaker_max) {
        ratio = ld(r.morale_breaker_max);
    }
    let amount = x87::mul(ratio, ld(r.morale_breaker_base));
    gain_x87(world, st, id, amount);
}

/// `4CE770 -> 571A60`: add experience, cross every reached level.
pub(crate) fn award(world: &mut World, st: &mut CombatState, id: OrgId, amount: f32) {
    let i = id as usize;
    if amount < EPSILON || !world.orgs[i].alive {
        return;
    }
    world.orgs[i].experience = fstp(x87::add(ld(world.orgs[i].experience), ld(amount)));
    let levels_len = st.extras().def(world.orgs[i].def).levels.len();
    let mut changed = false;
    while st.orgs[i].level + 1 < levels_len {
        let next = &st.extras().def(world.orgs[i].def).levels[st.orgs[i].level + 1];
        if next.required > world.orgs[i].experience {
            break;
        }
        st.orgs[i].level += 1;
        changed = true;
        world.events.push(Event::Promoted { org: id, level: st.orgs[i].level as u32 });
    }
    if changed {
        refresh_level_values(world, st, id);
    }
}

fn target_value(world: &World, st: &CombatState, t: TargetRef) -> (f32, f32) {
    match t {
        TargetRef::Org(o) => (combat::org_value(world, st, o), st.extras().def(world.orgs[o as usize].def).fake_cv_value),
        TargetRef::Actor(a) => {
            let ex = st.extras().def(world.actors[a as usize].def);
            (ex.combat_value, ex.fake_cv_value)
        }
    }
}

/// `52A810(rout, 2)`, a rout command: the rout state replaces the Mind's states unless
/// the regiment is routing already.
pub(crate) fn begin_rout(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    if st.orgs[i].routing.is_some() || !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    crate::movement::mind_morale_reaction(world, st, id, false);
}

/// `4D3FA0` in the rout's Enter `51F190`: every live engaged-by attacker gets breaker
/// morale and the rout experience.
pub(crate) fn rout_handler(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    let attackers = st.orgs[i].engaged_by.clone();
    let rules = st.rules.clone().expect("prepared");
    for a in attackers {
        if !world.orgs.get(a as usize).is_some_and(|o| o.alive) {
            continue;
        }
        breaker(world, st, a, id);
        let (victim, fake) = target_value(world, st, TargetRef::Org(id));
        let source = combat::org_value(world, st, a);
        let amount = combat::experience_clamped(combat::experience_ratio(victim, fake, source, &rules), rules.experience_rout, &rules);
        award(world, st, a, amount);
    }
}

/// The first steps of TickMaintenance `57C8E0` (queue 1, once a second in its
/// bucket), the scheduler's delivery: morale `57A0A0`, engagement experience
/// `579600`, attack efficiency `5785D0`.
pub(crate) fn maintenance_delivery(world: &mut World, id: OrgId, elapsed: f32) {
    if !world.orgs.get(id as usize).is_some_and(|o| o.alive) {
        return;
    }
    let mut st = std::mem::take(&mut world.combat);
    st.prepare(world);
    maintenance(world, &mut st, id, elapsed);
    world.combat = st;
}

/// See [`maintenance_delivery`]; an organization of a world built without the
/// native creation has no registration and runs it every update.
fn maintenance(world: &mut World, st: &mut CombatState, id: OrgId, dt: f32) {
    let i = id as usize;
    if !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    let rules = st.rules.clone().expect("prepared");
    let desc = descriptor(world, st, id);
    let lvl = level(world, st, id).cloned().unwrap_or_default();
    // 57A0A0: replenish (the supply bonus unless out of supply, +100 bit19):
    // fstp(rate * dt), then fstp(that * MORALE_RECOVERY_RATE (+44)).
    if desc.morale_replenish {
        // Organization state 19 out_of_supply (+100 bit19), kept by supply.rs.
        let in_supply = world.books.org(id).is_none_or(|b| !b.out_of_supply);
        let mut rate = ld(rules.morale_replenish);
        if in_supply {
            rate = x87::add(rate, ld(rules.morale_supply_bonus));
        }
        let recovery = (1.0 + lvl.morale_recovery).max(rules.clamp_multiply);
        let amount = fstp(x87::mul(ld(fstp(x87::mul(rate, ld(dt)))), ld(recovery)));
        gain(world, st, id, amount);
    }
    // 57A0A0: engagement loss while targeted by an enemy (+118), from the
    // second consecutive tick: fstp(((FactorMax - 1) * (1 - living/slots)
    // + 1) * Loss * dt), then fstp(that / MORALE_LOSS_RATE (+48)).
    let engaged = st.orgs[i].is_engaged();
    if st.orgs[i].engaged_latch && engaged {
        let slots = st.orgs[i].slots.max(1);
        let fraction = x87::div(f64::from(st.orgs[i].living), f64::from(slots));
        let factor = x87::add(x87::mul(x87::sub(ld(rules.morale_engagement_loss_factor_max), 1.0), x87::sub(1.0, fraction)), 1.0);
        let mut loss = fstp(x87::mul(x87::mul(factor, ld(rules.morale_engagement_loss)), ld(dt)));
        if st.orgs[i].entrenched {
            // organization_entrenched MORALE_LOSS_RATE, when the regiment
            // itself meets the property's required_properties.
            let rate = 1.0 + st.extras().def(world.orgs[i].def).entrench_morale_loss;
            loss = fstp(x87::div(ld(loss), ld(rate.max(rules.clamp_multiply))));
        }
        lose(world, st, id, loss);
    }
    st.orgs[i].engaged_latch = engaged;
    if !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    // 579600: engagement experience from the second consecutive engaged
    // tick, fstp(5718B0 ratio * ExperienceEngageMultiplier * dt).
    if let Some(t) = st.orgs[i].current_target().filter(|_| st.orgs[i].routing.is_none()) {
        if st.orgs[i].engage_latch && combat::target_alive(world, t) {
            let (victim, fake) = target_value(world, st, t);
            let source = combat::org_value(world, st, id);
            let rate = x87::mul(combat::experience_ratio(victim, fake, source, &rules), ld(rules.experience_engage));
            award(world, st, id, fstp(x87::mul(rate, ld(dt))));
        }
        st.orgs[i].engage_latch = true;
    } else {
        st.orgs[i].engage_latch = false;
    }
    // 578000 desired (the organization's 501530) and 5785D0 approach:
    // fstp(actual - dt * falling) not below the desired value, or
    // fstp(dt * climbing + actual) not above it.
    if st.orgs[i].saved_morale_living != Some(st.orgs[i].living) {
        st.orgs[i].saved_morale_living = None;
        refresh_morale_maximum(world, st, id);
    }
    let desired = desired_efficiency(world, st, id);
    let o = &mut st.orgs[i];
    o.attack_efficiency_desired = desired;
    if o.attack_efficiency > o.attack_efficiency_desired {
        o.attack_efficiency = fstp(x87::sub(ld(o.attack_efficiency), x87::mul(ld(dt), ld(rules.attack_efficiency_falling))));
        if o.attack_efficiency_desired > o.attack_efficiency {
            o.attack_efficiency = o.attack_efficiency_desired;
        }
    } else if o.attack_efficiency < o.attack_efficiency_desired {
        o.attack_efficiency = fstp(x87::add(x87::mul(ld(dt), ld(rules.attack_efficiency_climbing)), ld(o.attack_efficiency)));
        if o.attack_efficiency_desired < o.attack_efficiency {
            o.attack_efficiency = o.attack_efficiency_desired;
        }
    }
}

/// One organization's Mind reactions of an update: rout, entrenchment, the
/// engage/sleep Think reactions (and [`maintenance`] without a registration).
fn org_tick(world: &mut World, st: &mut CombatState, id: OrgId) {
    let i = id as usize;
    if !world.orgs[i].alive || st.orgs[i].destroyed {
        return;
    }
    let now = world.time;
    if !world.is_scheduled(crate::scheduler::Owner::OrgComponent(id, crate::scheduler::ORGANIZATION_SLOT)) {
        maintenance(world, st, id, DT);
        if !world.orgs[i].alive || st.orgs[i].destroyed {
            return;
        }
    }
    // 57C8E0 entrenchment: the unstored 45D520 delay plus the +B4 start not
    // after now, once in an entrenchable state (resting); left when the
    // regiment moves.
    let desc = descriptor(world, st, id);
    if desc.entrenchable {
        let since = *st.orgs[i].rest_since.get_or_insert(now);
        let delay = world.data.def(world.orgs[i].def).entrenchment_time;
        if !st.orgs[i].entrenched && delay > 0.0 && x87::add(ld(delay), ld(since)) <= ld(now) {
            st.orgs[i].entrenched = true;
        }
    } else if !st.orgs[i].engaging() {
        st.orgs[i].rest_since = None;
        st.orgs[i].entrenched = false;
    }
    // A new order replaces the exhausted Mind state (52A420). The rout, the exhausted
    // state and the Thinks that begin them are the regiment Mind's.
    if st.orgs[i].exhausted_state && world.orgs[i].order != Order::Idle {
        st.orgs[i].exhausted_state = false;
    }
    st.orgs[i].previous_order = world.orgs[i].order.clone();
}

/// Morale state (0 exhausted, 1 wavering, 2 steady) of an organization.
pub fn morale_state(world: &World, org: OrgId) -> Option<u8> {
    let s = world.combat.org(org)?;
    Some(state(s, world.orgs.get(org as usize)?.morale))
}
