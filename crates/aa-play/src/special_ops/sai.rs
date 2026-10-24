//! The computer players' general powers: the GENERAL_POWER goal of the SAI
//! goal engine (`7C40C0` candidates: the commander abilities the current ego
//! has a GeneralPower for; `7C42F0` the "ping-pong ball" order by the
//! squared base priority; `7C3C10` the use on the chosen actor or location),
//! with each power's Location and Priority from the ego data
//! (Templates/template_sai.tgi GeneralPower, loader `4AFD50`).
//!
//! Evaluated once per goal-engine think of the player (`ai::Player::ge_count`).
//! Influence comes from the opponent's region analysis (`ai::AiState::
//! influence`), combat values from its tables (`ai::AiState::bound`).
use super::*;

#[derive(Clone, Debug, Default)]
pub struct SaiState {
    /// Goal-engine think count last evaluated, by kingdom.
    seen: Vec<u32>,
    /// Uses issued (for tests and the HUD log).
    pub uses: u64,
}

fn engaged(world: &World, t: OpTarget) -> bool {
    let org = match t {
        OpTarget::Org(o) => Some(o),
        OpTarget::Actor(a) => world.actors[a as usize].org,
    };
    if let Some(o) = org
        && let Some(s) = world.combat.org(o)
    {
        return s.engaging() || !s.engaged_by.is_empty();
    }
    match t {
        OpTarget::Actor(a) => world.actors[a as usize].combat.target.is_some(),
        OpTarget::Org(_) => false,
    }
}

fn health_ratio(world: &World, t: OpTarget) -> f32 {
    let one = |a: ActorId| {
        let x = &world.actors[a as usize];
        let max = world.combat.actor(a).map(|c| c.max_health).filter(|&m| m > 0.0).unwrap_or(world.data.def(x.def).health);
        (x.health, max)
    };
    let (h, m) = match t {
        OpTarget::Actor(a) => one(a),
        OpTarget::Org(o) => world.orgs[o as usize].members.iter().filter(|&&m| world.actors[m as usize].alive).map(|&m| one(m)).fold((0.0, 0.0), |s, v| (s.0 + v.0, s.1 + v.1)),
    };
    if m > 0.0 { h / m } else { 1.0 }
}

fn properties<'a>(world: &'a World, t: OpTarget) -> &'a [String] {
    match t {
        OpTarget::Actor(a) => &world.data.def(world.actors[a as usize].def).properties,
        OpTarget::Org(o) => &world.data.def(world.orgs[o as usize].def).properties,
    }
}

/// GeneralPowerActorRequirements against one actor or organization.
fn meets(world: &World, me: KingdomId, r: &Requirements, t: OpTarget) -> bool {
    let k = owner(world, t);
    if k.is_none() {
        return false;
    }
    if r.friendly && !allied(world, k, Some(me)) {
        return false;
    }
    if r.enemy && relation(world, Some(me), k) != crate::books::ENEMY {
        return false;
    }
    if r.owned && k != Some(me) {
        return false;
    }
    if r.must_be_engaged && !engaged(world, t) {
        return false;
    }
    if r.must_not_be_engaged && engaged(world, t) {
        return false;
    }
    if health_ratio(world, t) > r.max_health + 1e-6 {
        return false;
    }
    r.properties.is_empty() || r.properties.iter().any(|p| properties(world, t).iter().any(|q| q.eq_ignore_ascii_case(p)))
}

fn considered(world: &World, me: KingdomId, f: &Factor, t: OpTarget) -> bool {
    let k = owner(world, t);
    (f.consider_owned && k == Some(me)) || (f.consider_friendly && allied(world, k, Some(me))) || (f.consider_enemy && relation(world, Some(me), k) == crate::books::ENEMY)
}

fn cv(world: &World, t: OpTarget) -> f32 {
    let table = &world.ai.bound.cv;
    let v = |d: u32| table.get(d as usize).copied().unwrap_or(0.0);
    match t {
        OpTarget::Actor(a) => v(world.actors[a as usize].def),
        OpTarget::Org(o) => world.orgs[o as usize].members.iter().filter(|&&m| world.actors[m as usize].alive).map(|&m| v(world.actors[m as usize].def)).sum(),
    }
}

/// One CV / ActorValue factor over the actors concerned; None below its minimum.
fn value_factor(world: &World, me: KingdomId, f: &Factor, targets: &[OpTarget]) -> Option<f32> {
    let mut sum = 0.0;
    for &t in targets {
        if !considered(world, me, f, t) {
            continue;
        }
        let m = f.categories.iter().find(|c| meets(world, me, c, t)).map_or(f.multiplier, |c| c.multiplier);
        sum += cv(world, t) * m;
    }
    if f.maximum >= 0.0 {
        sum = sum.min(f.maximum);
    }
    (sum >= f.minimum).then_some(sum)
}

fn capped(f: &Factor, v: f32) -> Option<f32> {
    let mut v = v * f.multiplier;
    if f.maximum >= 0.0 {
        v = v.min(f.maximum);
    }
    (v >= f.minimum).then_some(v)
}

/// Friendly (or owned) and enemy mobile influence of the region at `p`.
fn influence(world: &World, me: KingdomId, p: Vec2, owned_only: bool) -> (f32, f32) {
    let Some(regions) = world.ai.regions.as_ref() else { return (0.0, 0.0) };
    let inf = &world.ai.influence;
    if inf.kingdoms == 0 {
        return (0.0, 0.0);
    }
    let region = regions.region_at(p);
    let mut friendly = 0.0;
    let mut enemy = 0.0;
    for k in 0..inf.kingdoms {
        let v = inf.at(&inf.mobile, region, k);
        let kid = Some(k as KingdomId);
        if owned_only && kid == Some(me) || !owned_only && allied(world, kid, Some(me)) {
            friendly += v;
        } else if relation(world, Some(me), kid) == crate::books::ENEMY {
            enemy += v;
        }
    }
    (friendly, enemy)
}

/// The priority of one candidate (Priority: base + each factor + fuzzy).
fn priority(world: &mut World, me: KingdomId, op: &SpecialOp, pc: &PowerConfig, point: Vec2, on: Option<OpTarget>) -> Option<f32> {
    // The actors the power affects: in its area, or the targeted one.
    let affected: Vec<OpTarget> = match on {
        Some(t) if op.area <= 0.0 => vec![t],
        _ => in_area(world, point, op.area.max(0.0)),
    };
    let around: Vec<OpTarget> = in_area(world, point, op.area.max(0.0));
    // GeneralRequirements.
    for &t in &around {
        let k = owner(world, t);
        if (pc.no_owned && k == Some(me)) || (pc.no_friendly && allied(world, k, Some(me))) || (pc.no_enemy && relation(world, Some(me), k) == crate::books::ENEMY) {
            return None;
        }
    }
    if pc.requires_supply && !world.books.zones.iter().any(|z| allied(world, Some(z.kingdom), Some(me)) && z.centre.distance(point) <= z.radius) {
        return None;
    }
    let mut total = pc.base;
    total += value_factor(world, me, &pc.cv, &affected)?;
    total += value_factor(world, me, &pc.actor_value, &affected)?;
    if pc.actor_economy.multiplier != 0.0 {
        let production = match on {
            Some(OpTarget::Actor(a)) => world.data.def(world.actors[a as usize].def).production_all.iter().sum::<f32>(),
            _ => 0.0,
        };
        total += capped(&pc.actor_economy, production)?;
    }
    let (friendly, enemy) = influence(world, me, point, false);
    if pc.enemy_influence.multiplier != 0.0 || pc.enemy_influence.minimum > 0.0 {
        total += capped(&pc.enemy_influence, enemy.max(0.0).powf(pc.enemy_influence.exponent))?;
    }
    if pc.influence_ratio.multiplier != 0.0 || pc.influence_ratio.minimum > 0.0 {
        let (f, e) = if pc.influence_ratio.count_owned_only { influence(world, me, point, true) } else { (friendly, enemy) };
        let ratio = if e > 0.0 { f / e } else if f > 0.0 { f32::MAX.sqrt() } else { 1.0 };
        let shaped = if ratio > 1.0 { ratio.powf(pc.influence_ratio.exponent_over_one) } else { ratio.powf(pc.influence_ratio.exponent_under_one) };
        total += capped(&pc.influence_ratio, shaped)?;
    }
    if pc.economy_generated.multiplier != 0.0 {
        total += pc.economy_generated.resources.iter().map(|(_, a)| a).sum::<f32>() * pc.economy_generated.multiplier;
    }
    total += world.random() * pc.fuzzy;
    (total >= pc.minimum).then_some(total)
}

/// The best location of one power (Location type, Placement candidates).
fn best(world: &mut World, me: KingdomId, index: usize, op: &SpecialOp, pc: &PowerConfig) -> Option<(f32, AbilityTarget)> {
    let mut targets: Vec<OpTarget> = world.actors.iter().filter(|a| a.alive && !a.removed && a.kingdom.is_some()).map(|a| OpTarget::Actor(a.id)).collect();
    targets.extend(world.orgs.iter().filter(|o| o.alive).map(|o| OpTarget::Org(o.id)));
    targets.retain(|&t| alive(world, t) && meets(world, me, &pc.actor_requirements, t));
    let mut best: Option<(f32, AbilityTarget)> = None;
    for t in targets {
        let candidates: Vec<(Vec2, Option<OpTarget>)> = match pc.location {
            // ON_ACTOR: the actor itself.
            2 => vec![(position(world, t), Some(t))],
            // NEAR_ACTOR: random points within the placement radius.
            1 => (0..pc.candidates.max(1))
                .map(|_| {
                    let r = pc.radius.max(0.0) * world.random();
                    let a = world.random() * std::f32::consts::TAU;
                    (position(world, t) + Vec2::new(a.cos(), a.sin()) * r, None)
                })
                .collect(),
            _ => Vec::new(),
        };
        for (p, on) in candidates {
            let mut check = on;
            if admit(world, me, index, &mut check).is_err() {
                continue;
            }
            let Some(v) = priority(world, me, op, pc, p, on) else { continue };
            if best.as_ref().is_none_or(|b| v > b.0) {
                let target = match on {
                    Some(OpTarget::Actor(a)) => AbilityTarget::Actor(a),
                    Some(OpTarget::Org(o)) => match world.orgs[o as usize].members.iter().copied().find(|&m| world.actors[m as usize].alive) {
                        Some(m) => AbilityTarget::Actor(m),
                        None => continue,
                    },
                    None => AbilityTarget::Point(p),
                };
                best = Some((v, target));
            }
        }
    }
    best
}

/// `7C42F0`: order the powers by drawing "ping-pong balls" weighted by the
/// squared base priority.
fn ping_pong(world: &mut World, list: &mut [(usize, PowerConfig)]) {
    let n = list.len();
    if n < 2 {
        return;
    }
    let mut weights: Vec<f32> = list.iter().map(|(_, p)| p.base * p.base).collect();
    let mut total: f32 = weights.iter().sum();
    for start in 0..n {
        let mut r = world.random() * total;
        let mut j = start;
        while j + 1 < n && weights[j] < r {
            r -= weights[j];
            j += 1;
        }
        list.swap(start, j);
        weights.swap(start, j);
        total -= weights[start];
    }
}

pub fn think(world: &mut World) {
    let (Some(data), Some(ai)) = (world.special_ops.data.clone(), world.ai.data.clone()) else { return };
    let players: Vec<(KingdomId, usize, u32)> = world.ai.players.iter().map(|p| (p.kingdom, p.ego, p.ge_count)).collect();
    for (kingdom, ego, ge) in players {
        let i = kingdom as usize;
        if world.special_ops.sai.seen.len() <= i {
            world.special_ops.sai.seen.resize(i + 1, 0);
        }
        if world.special_ops.sai.seen[i] == ge {
            continue;
        }
        world.special_ops.sai.seen[i] = ge;
        if world.kingdoms.iter().find(|k| k.id == kingdom).is_none_or(|k| k.defeated) {
            continue;
        }
        let Some(gi) = world.special_ops.commanders.get(i).and_then(|c| c.general) else { continue };
        let Some(ego) = ai.egos.get(ego) else { continue };
        let powers = data.powers(&ego.ids);
        // 7C40C0: the commander's abilities the ego configures.
        let mut list: Vec<(usize, PowerConfig)> = data.generals[gi]
            .abilities
            .iter()
            .enumerate()
            .filter_map(|(a, op)| powers.iter().find(|p| p.ability.eq_ignore_ascii_case(&op.id)).map(|p| (a, p.clone())))
            .collect();
        ping_pong(world, &mut list);
        for (a, pc) in list {
            if !view(world, kingdom, a).is_some_and(|v| v.usable) {
                continue;
            }
            let op = data.generals[gi].abilities[a].clone();
            if let Some((_, target)) = best(world, kingdom, a, &op, &pc) {
                world.apply(Command::UseAbility { kingdom, ability: a as u32, target: Some(target) });
                world.special_ops.sai.uses += 1;
            }
        }
    }
}
