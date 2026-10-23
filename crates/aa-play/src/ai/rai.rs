//! The reactive AI of one player (`7B8AC0`, every `ReactiveAI/think_frequency`
//! seconds): retreat decisions (`7B6810`), the execution of the goal
//! engine's selected combat goals (`7B7D30`/`7B8540`/`7B8800` with `7A5E10`),
//! and engaging nearby threats.
//!
//! Retreat rules (Templates/template_sai.tgi `[ReactiveAI]`, ego +89C..+8B8):
//! - unit retreat: an engaged organization at or below
//!   `unit_retreat_max_health` whose CV ratio in the immediate fight
//!   (itself, the hostile organizations engaging it or engaged by it, and the
//!   friendly organizations engaging those, `unit_retreat_depth` 1) is below
//!   `unit_retreat_cv_ratio` retreats with `unit_retreat_probability`; a
//!   decision not to retreat holds for `unit_retreat_cycles_til_reconsider`
//!   RAI thinks;
//! - region retreat: when friendly / hostile CV over the region and the
//!   regions within `region_retreat_depth` drops below
//!   `region_retreat_cv_ratio`, every engaged organization there retreats
//!   with `region_retreat_probability`;
//! - halt: a retreating organization whose local friendly / hostile ratio
//!   reaches `halt_retreat_cv_ratio` stops and fights again.
use super::data::{AiData, DEFEND_REGION};
use super::goals::{Ctx, Key, city_target, supply_point};
use super::regions::{Influence, Regions};
use super::view::View;
use super::{Bound, Player, SaiRng, enqueue};
use crate::combat::TargetRef;
use crate::world::{Command, Order, OrgId, Unit};
use glam::Vec2;
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Default)]
pub struct Memory {
    /// Organizations retreating and since when.
    pub retreating: BTreeMap<OrgId, f32>,
    /// RAI cycle before which a "do not retreat" decision stands.
    pub reconsider: BTreeMap<OrgId, u32>,
    pub region_reconsider: BTreeMap<u32, u32>,
    /// Retreat orders given: (time, organization).
    pub retreats: Vec<(f32, OrgId)>,
    /// The last goal-execution order sent to each organization.
    pub sent: BTreeMap<OrgId, Resend>,
}

/// A goal-execution order the reactive AI sent: where to (or whom), when,
/// and where the organization stood.
#[derive(Clone, Copy, Debug)]
pub struct Resend {
    pub to: Vec2,
    pub target: Option<crate::world::ActorId>,
    pub at: f32,
    pub pos: Vec2,
}

impl Memory {
    /// The reactive AI re-sends a goal's order to an idle organization
    /// (`7B7D30` orders the goal's actors that are not busy). An order the
    /// organization could not follow (an unreachable or blocked target: it
    /// stands where it stood when the same order was sent) is not repeated
    /// every think: it waits for the next goal-engine period.
    fn gate(&mut self, o: OrgId, to: Vec2, target: Option<crate::world::ActorId>, pos: Vec2, now: f32, tol: f32, period: f32) -> bool {
        if let Some(last) = self.sent.get(&o)
            && last.target == target
            && last.to.distance(to) < tol
            && last.pos.distance(pos) < STUCK_METRES
            && now - last.at < period
        {
            return false;
        }
        self.sent.insert(o, Resend { to, target, at: now, pos });
        true
    }
}

/// An organization that moved less than this since the same order was sent
/// did not get any closer.
const STUCK_METRES: f32 = 2.0;

#[allow(clippy::too_many_arguments)]
pub fn think(
    view: &View<'_>,
    bound: &Bound,
    data: &AiData,
    regions: &Regions,
    inf: &Influence,
    player: &mut Player,
    rng: &mut SaiRng,
    log: &mut Vec<String>,
) {
    let ego = &data.egos[player.ego];
    let c = Ctx { view, bound, ego, regions, inf, now: view.world.time };
    let w = view.world;
    let me = view.kingdom;
    let r = &ego.reactive;
    let cycle = player.rai_count;
    let mut commands = Vec::new();
    let index: HashMap<OrgId, usize> = view.orgs.iter().enumerate().map(|(i, o)| (o.id, i)).collect();
    let engage_targets = |o: OrgId| -> Vec<OrgId> {
        let mut out = Vec::new();
        if let Some(s) = w.combat.org(o) {
            for t in &s.engage {
                match t.target {
                    TargetRef::Org(x) => out.push(x),
                    TargetRef::Actor(a) => {
                        if let Some(x) = w.actors.get(a as usize).and_then(|a| a.org) {
                            out.push(x);
                        }
                    }
                }
            }
            out.extend(s.engaged_by.iter().copied());
        }
        out.sort_unstable();
        out.dedup();
        out
    };
    let cv_of = |o: OrgId| index.get(&o).map_or(0.0, |&i| view.orgs[i].cv);
    player.rai.retreating.retain(|o, _| index.contains_key(o));
    player.rai.sent.retain(|o, _| index.contains_key(o));
    let period = ego.think_frequency.max(0.5);

    // Halt retreat or finish it at the destination.
    let retreating: Vec<OrgId> = player.rai.retreating.keys().copied().collect();
    for o in retreating {
        let ov = &view.orgs[index[&o]];
        let (friend, foe) = local_strength(&c, c.region_of(ov.pos), 1);
        if foe > 0.0 && friend / foe >= r.halt_retreat_cv_ratio {
            player.rai.retreating.remove(&o);
            commands.push(Command::Stop { kingdom: me, units: vec![Unit::Org(o)] });
            log.push(format!("halt retreat of {} (ratio {:.2})", c.def(ov.def).id, friend / foe));
        } else if !ov.engaged && matches!(ov.order, Order::Idle) {
            player.rai.retreating.remove(&o);
        }
    }

    // Unit retreat.
    let mut retreat: Vec<usize> = Vec::new();
    for (i, ov) in view.orgs.iter().enumerate() {
        if ov.owner != view.me || !ov.military || !ov.engaged || ov.routing || player.rai.retreating.contains_key(&ov.id) {
            continue;
        }
        if ov.health > r.unit_retreat_max_health || r.unit_retreat_depth < 1 {
            continue;
        }
        if player.rai.reconsider.get(&ov.id).is_some_and(|&until| cycle < until) {
            continue;
        }
        let attackers: Vec<OrgId> = engage_targets(ov.id).into_iter().filter(|x| index.get(x).is_some_and(|&j| view.hostile[view.orgs[j].owner])).collect();
        let foe: f32 = attackers.iter().map(|&x| cv_of(x)).sum();
        if foe <= 0.0 {
            continue;
        }
        let mut friends = vec![ov.id];
        if r.unit_retreat_depth >= 2 {
            for &x in &attackers {
                for f in engage_targets(x) {
                    if index.get(&f).is_some_and(|&j| view.allied[view.orgs[j].owner]) && !friends.contains(&f) {
                        friends.push(f);
                    }
                }
            }
        } else {
            for o2 in view.orgs.iter().filter(|o2| view.allied[o2.owner] && o2.id != ov.id) {
                if engage_targets(o2.id).iter().any(|t| attackers.contains(t)) {
                    friends.push(o2.id);
                }
            }
        }
        let friend: f32 = friends.iter().map(|&x| cv_of(x)).sum();
        if friend / foe < r.unit_retreat_cv_ratio {
            if rng.unit() <= r.unit_retreat_probability {
                retreat.push(i);
            } else {
                player.rai.reconsider.insert(ov.id, cycle + r.unit_retreat_cycles_til_reconsider.max(0) as u32);
            }
        }
    }

    // Region retreat.
    let mut engaged_regions: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (i, ov) in view.orgs.iter().enumerate() {
        if ov.owner == view.me && ov.military && ov.engaged && !ov.routing && !player.rai.retreating.contains_key(&ov.id) {
            engaged_regions.entry(c.region_of(ov.pos)).or_default().push(i);
        }
    }
    for (region, list) in engaged_regions {
        if player.rai.region_reconsider.get(&region).is_some_and(|&until| cycle < until) {
            continue;
        }
        let (friend, foe) = local_strength(&c, region, r.region_retreat_depth.max(0) as u32);
        if foe <= 0.0 || friend / foe >= r.region_retreat_cv_ratio {
            continue;
        }
        if rng.unit() <= r.region_retreat_probability {
            for i in list {
                if !retreat.contains(&i) {
                    retreat.push(i);
                }
            }
            log.push(format!("region {region} retreat (ratio {:.2})", friend / foe));
        } else {
            player.rai.region_reconsider.insert(region, cycle + r.region_retreat_cycles_til_reconsider.max(0) as u32);
        }
    }
    for i in retreat {
        let ov = &view.orgs[i];
        let Some(dest) = supply_point(&c, ov.pos) else { continue };
        player.rai.retreating.insert(ov.id, c.now);
        player.rai.retreats.push((c.now, ov.id));
        commands.push(Command::Move { kingdom: me, units: vec![Unit::Org(ov.id)], to: dest, attack_move: false });
        log.push(format!("retreat {} (health {:.2})", c.def(ov.def).id, ov.health));
    }

    // Goal execution between goal-engine thinks.
    let tol = regions.size * 0.5;
    let attack_reach = r.attack_distance.max(0) as u32;
    let hostiles: Vec<usize> = (0..view.orgs.len()).filter(|&j| view.hostile[view.orgs[j].owner] && view.orgs[j].cv > 0.0).collect();
    let eyes: Vec<(Vec2, f32)> = view.orgs.iter().filter(|o| o.owner == view.me).map(|o| (o.pos, o.sight)).collect();
    let visible = |p: Vec2| eyes.iter().any(|&(e, s)| e.distance(p) <= s);
    let mut gate = Memory { sent: std::mem::take(&mut player.rai.sent), ..Default::default() };
    for (&o, a) in &player.memory.assigned {
        let Some(&i) = index.get(&o) else { continue };
        let ov = &view.orgs[i];
        if !a.combat || ov.engaged || ov.routing || player.rai.retreating.contains_key(&o) || player.pending(o, c.now) {
            continue;
        }
        let idle = matches!(ov.order, Order::Idle);
        // A defend goal attacks visible threats within attack_distance regions.
        if matches!(a.key, Key::Defend(_)) {
            let threat = hostiles
                .iter()
                .map(|&j| &view.orgs[j])
                .filter(|h| regions.hops(c.region_of(h.pos), a.region) <= attack_reach && visible(h.pos))
                .min_by(|x, y| x.pos.distance(ov.pos).partial_cmp(&y.pos.distance(ov.pos)).unwrap_or(std::cmp::Ordering::Equal));
            if let Some(h) = threat {
                if !matches!(&ov.order, Order::Move { to, attack_move: true } if to.distance(h.pos) < tol) && gate.gate(o, h.pos, None, ov.pos, c.now, tol, period) {
                    commands.push(Command::Move { kingdom: me, units: vec![Unit::Org(o)], to: h.pos, attack_move: true });
                }
                continue;
            }
            if idle && regions.hops(c.region_of(ov.pos), a.region) > 1 && gate.gate(o, a.target, None, ov.pos, c.now, tol, period) {
                commands.push(Command::Move { kingdom: me, units: vec![Unit::Org(o)], to: a.target, attack_move: true });
            }
            continue;
        }
        // Attack goals: attack the target building once in reach (a city is
        // taken by presence: stay in its zone).
        // The target must still be an enemy's: a stale assignment may name a
        // building that changed hands since the goal-engine think.
        if let Some(t) = a.target_actor
            && !city_target(&c, Some(t))
            && let Some(ta) = w.actor(t).filter(|ta| ta.alive && super::enemy_of(w, view.kingdom, ta.kingdom))
        {
            let tp = ta.pos.truncate();
            let reach = w.orgs[o as usize]
                .members
                .iter()
                .filter_map(|&m| w.actors.get(m as usize))
                .flat_map(|m| c.def(m.def).weapons.iter().map(|wp| wp.range))
                .fold(0.0f32, f32::max);
            if ov.pos.distance(tp) <= reach + tol && !matches!(ov.order, Order::Attack { target } if target == t) {
                if gate.gate(o, tp, Some(t), ov.pos, c.now, tol, period) {
                    commands.push(Command::Attack { kingdom: me, units: vec![Unit::Org(o)], target: t });
                }
                continue;
            }
        }
        if idle && ov.pos.distance(a.target) > tol && gate.gate(o, a.target, None, ov.pos, c.now, tol, period) {
            commands.push(Command::Move { kingdom: me, units: vec![Unit::Org(o)], to: a.target, attack_move: true });
        }
    }
    player.rai.sent = gate.sent;
    let _ = DEFEND_REGION;
    let profile = &data.profiles[player.profile];
    enqueue(player, data, c.now, r.reaction_time, profile.max_rai_commands, commands);
}

/// Friendly and hostile CV of the organizations within `depth` regions.
fn local_strength(c: &Ctx<'_, '_>, region: u32, depth: u32) -> (f32, f32) {
    let (mut friend, mut foe) = (0.0, 0.0);
    for o in &c.view.orgs {
        if c.regions.hops(c.region_of(o.pos), region) > depth {
            continue;
        }
        if c.view.allied[o.owner] {
            friend += o.cv;
        } else if c.view.hostile[o.owner] {
            foe += o.cv;
        }
    }
    (friend, foe)
}
