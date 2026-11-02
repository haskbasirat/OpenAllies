//! The computer players' general powers: the GENERAL_POWER goal (type 0x15,
//! vtable A9E9D8) that the SAI goal engine builds at each of its thinks
//! (`7A4A60` -> `7C40C0`):
//! - `7C40C0`: the Commander's abilities, in order, that the current ego has
//!   a GeneralPower for (ego `+884` by definition);
//! - `7C4490` -> `7C42F0`: with more than one, the "ping-pong ball" order by
//!   the squared base priority (SAI RNG `[B36528]+2C`);
//! - `7C3A60`: in that order while the experience left covers each cost
//!   (`4CDD40`, the first that does not stops it), the power's location
//!   (`4AE190`: NEAR_ACTOR `4AD590`, ON_ACTOR `4AD3F0`, FOG `4AD8C0`) and its
//!   priority (`4AD2B0`: base, then CV `4A8030`, ActorValue `4ACFA0`,
//!   ActorEconomy `4AB560`, EnemyInfluence `4A82B0`, InfluenceRatio
//!   `4A83F0`, TimeSinceExplored `4A85B0`, EconomyGenerated `4AB7A0`; any
//!   factor below its minimum, or the total below the power's, gives 0;
//!   then the fuzzy factor times an SAI draw); the best above zero flags
//!   the power and adds to the goal's priority (`+38`);
//! - `7C3C10`: each flagged power is ordered (`61E270` on its target actor
//!   while it lives, else `61E2B0` at its location) with the attack flare
//!   of its Followup when it has a priority and a duration (`7B0610`, here
//!   the player's region flare `ai::Player::flares`).
//!
//! The loader `4AFD50` reads neither ActorEconomy (its words stay zero, so
//! it adds nothing) nor uses GeneralRequirements (no_*_actors and
//! requires_supply are read but no evaluation consults them). Recharging
//! powers are evaluated and ordered like the others; `4C8F20` refuses them.
//!
//! The leaf values come from the opponent's tables: combat values
//! (`ai::AiState::bound`), the region analysis and its influence
//! (`ai::AiState::regions` / `influence`, mobile and settlement standing for
//! the region record's `+8` / `+14` vectors), the player's region scouting
//! times, and the `4AC240` / `4AAA30` valuations (`ai::value::Val`).
//! Evaluated once per goal-engine think of the player (`ai::Player::ge_count`).
use super::*;
use crate::ai::SaiRng;
use crate::ai::value::Val;

#[derive(Clone, Debug, Default)]
pub struct SaiState {
    /// Goal-engine think count last evaluated, by kingdom.
    seen: Vec<u32>,
    /// Uses issued (for tests and the HUD log).
    pub uses: u64,
}

/// `45D400`: FILD of the signed draw, +2^32 when negative, times binary64
/// 0x3DF0000000100000; extended, never stored.
fn unit(draw: u32) -> f64 {
    f64::from(draw) * f64::from_bits(0x3df0_0000_0010_0000)
}

/// What a flagged power is ordered on.
#[derive(Clone, Copy, Debug)]
enum Place {
    Actor(ActorId),
    Point(Vec2),
}

/// The evaluation context of one player's think.
struct Eval<'a> {
    world: &'a World,
    me: KingdomId,
    player: &'a crate::ai::Player,
    val: Option<&'a Val<'a, 'a, 'a>>,
    rng: &'a mut SaiRng,
}

impl Eval<'_> {
    fn draw(&mut self) -> u32 {
        self.rng.next_u32()
    }

    /// `4A7ED0`: the kingdoms in order whose relation the flags admit
    /// (me +138 allied, owned, me +140 enemy), each with `79EAF0`: the
    /// SAI's actors of the kingdom, organizations as their members, within
    /// `radius` of `at` (all with a negative radius), prepended.
    fn candidates(&self, friendly: bool, hostile: bool, owned: bool, at: Vec2, radius: f32) -> Vec<ActorId> {
        let w = self.world;
        let mut out = Vec::new();
        let r2 = f64::from(radius * radius);
        for k in &w.kingdoms {
            let kid = Some(k.id);
            let take = (friendly && allied(w, Some(self.me), kid)) || (owned && k.id == self.me) || (hostile && enemy(w, Some(self.me), kid));
            if !take {
                continue;
            }
            let mut list: Vec<ActorId> = w
                .actors
                .iter()
                .filter(|a| a.kingdom == kid && a.alive && !a.removed)
                .filter(|a| radius < 0.0 || squared(at, a.pos.truncate()) <= r2)
                .map(|a| a.id)
                .collect();
            list.reverse();
            out.extend(list);
        }
        out
    }

    /// `4A6A70`: must_be_friendly (owner +138 me), must_be_enemy (owner
    /// +140 me), must_be_owned.
    fn relation_ok(&self, r: &Requirements, a: ActorId) -> bool {
        let k = self.world.actors[a as usize].kingdom;
        (!r.friendly || allied(self.world, k, Some(self.me))) && (!r.enemy || enemy(self.world, k, Some(self.me))) && (!r.owned || k == Some(self.me))
    }

    /// `4A7B00`: health ratio (`4D0230`) at most max_percent_health; engaged
    /// (actor +100 bit 4 or +118) as required; any one required property.
    fn actor_ok(&self, r: &Requirements, a: ActorId) -> bool {
        let w = self.world;
        if f64::from(r.max_health) < health_ratio(w, a) {
            return false;
        }
        let engaged = engaged(w, a);
        if (r.must_be_engaged && !engaged) || (r.must_not_be_engaged && engaged) {
            return false;
        }
        r.properties.is_empty() || r.properties.iter().any(|p| has_property(w, OpTarget::Actor(a), p))
    }

    /// `4D0180`: the actor's combat value.
    fn cv(&self, a: ActorId) -> f32 {
        self.world.ai.bound.cv.get(self.world.actors[a as usize].def as usize).copied().unwrap_or(0.0)
    }

    /// `4AC240` (definition, owner, 1, 0, 0, 1, actor).
    fn value(&self, a: ActorId) -> f32 {
        let Some(val) = self.val else { return 0.0 };
        let x = &self.world.actors[a as usize];
        let mobile = self.world.data.def(x.def).speed_max > 0.0;
        val.actor_value(x.def, x.kingdom, self.cv(a), mobile, x.pos.truncate(), false)
    }

    /// `4A8030` (CV) / `4ACFA0` (ActorValue): NEAR_ACTOR sums the considered
    /// actors in the ability's area, ON_ACTOR takes the target; each with
    /// the first matching category's multiplier, else the factor's; a zero
    /// multiplier adds nothing. Clamped to a non-negative maximum; None
    /// below the minimum. FOG passes without adding.
    fn actors_factor(&self, f: &Factor, pc: &PowerConfig, area: f32, actor: Option<ActorId>, at: Vec2, cv: bool) -> Option<f32> {
        let worth = |a: ActorId| if cv { self.cv(a) } else { self.value(a) };
        let multiplier = |a: ActorId| f.categories.iter().find(|c| self.relation_ok(c, a) && self.actor_ok(c, a)).map_or(f.multiplier, |c| c.multiplier);
        let mut sum = 0.0f32;
        match pc.location {
            1 => {
                for a in self.candidates(f.consider_friendly, f.consider_enemy, f.consider_owned, at, area) {
                    let m = multiplier(a);
                    if m != 0.0 {
                        sum = (f64::from(worth(a)) * f64::from(m) + f64::from(sum)) as f32;
                    }
                }
            }
            2 => {
                if let Some(a) = actor.filter(|&a| alive(self.world, OpTarget::Actor(a))) {
                    let m = multiplier(a);
                    if m != 0.0 {
                        sum = (f64::from(worth(a)) * f64::from(m)) as f32;
                    }
                }
            }
            _ => return Some(0.0),
        }
        bounded(f, f64::from(sum)).map(|v| v as f32)
    }

    /// The region at a point inside the map (`4282A0`, `7B9300`).
    fn region(&self, at: Vec2) -> Option<u32> {
        let [w, h] = map_extent(self.world);
        if !(at.x >= 0.0 && at.x < w && at.y >= 0.0 && at.y < h) {
            return None;
        }
        let regions = self.world.ai.regions.as_ref()?;
        (!regions.is_empty()).then(|| regions.region_at(at))
    }

    /// The player's region record (`7B9050`): (`7B9600` friendly, `7B9570`
    /// owned, `7B96E0` enemy) mobile and static influence.
    fn influence(&self, region: u32) -> (f64, f64, f64) {
        let inf = &self.world.ai.influence;
        let (mut friendly, mut owned, mut hostile) = (0.0f32, 0.0f32, 0.0f32);
        for k in (0..inf.kingdoms).rev() {
            let kid = Some(k as KingdomId);
            let mobile = inf.at(&inf.mobile, region, k);
            let fixed = inf.at(&inf.settlement, region, k);
            if kid == Some(self.me) {
                owned = mobile + fixed;
            }
            for v in [mobile, fixed] {
                if v > 0.0 {
                    if allied(self.world, Some(self.me), kid) {
                        friendly += v;
                    }
                    if enemy(self.world, Some(self.me), kid) {
                        hostile += v;
                    }
                }
            }
        }
        (f64::from(friendly), f64::from(owned), f64::from(hostile))
    }

    /// `4A82B0`: inside the map, pow(enemy influence, exponent) (0 without
    /// any) times the multiplier, bounded.
    fn enemy_influence(&self, pc: &PowerConfig, at: Vec2) -> Option<f32> {
        let Some(r) = self.region(at) else { return Some(0.0) };
        let f = &pc.enemy_influence;
        let (_, _, hostile) = self.influence(r);
        let v = if hostile as f32 != 0.0 { hostile.powf(f64::from(f.exponent)) } else { 0.0 };
        bounded(f, v * f64::from(f.multiplier)).map(|v| v as f32)
    }

    /// `4A83F0`: inside the map, friendly (owned only with
    /// count_owned_only) over enemy influence, stored binary32 (friendly
    /// times 999.99994 without enemies); above one pow with
    /// exponent_over_one, else when not zero with exponent_under_one;
    /// times the multiplier, bounded.
    fn influence_ratio(&self, pc: &PowerConfig, at: Vec2) -> Option<f32> {
        let Some(r) = self.region(at) else { return Some(0.0) };
        let f = &pc.influence_ratio;
        let (friendly, owned, hostile) = self.influence(r);
        let mine = if f.count_owned_only { owned } else { friendly };
        let ratio = if hostile != 0.0 { (mine / hostile) as f32 } else { (mine * f64::from(f32::from_bits(0x4479_ffff))) as f32 };
        let shaped = if ratio > 1.0 {
            f64::from(ratio).powf(f64::from(f.exponent_over_one))
        } else if ratio != 0.0 {
            f64::from(ratio).powf(f64::from(f.exponent_under_one))
        } else {
            0.0
        };
        bounded(f, shaped * f64::from(f.multiplier)).map(|v| v as f32)
    }

    /// `4A85B0`: inside the map, (GWorld+0xE0 - the region's explored time
    /// `7B8F00`) times the multiplier, bounded.
    fn time_since_explored(&self, pc: &PowerConfig, at: Vec2) -> Option<f32> {
        let Some(r) = self.region(at) else { return Some(0.0) };
        let f = &pc.time_since_explored;
        let seen = self.player.scouted.get(r as usize).copied().filter(|t| t.is_finite()).unwrap_or(0.0);
        bounded(f, (f64::from(native_now(self.world)) - f64::from(seen)) * f64::from(f.multiplier)).map(|v| v as f32)
    }

    /// `4AB7A0`: the `4AAA30` value of the resource amounts times the
    /// multiplier; None below the minimum (no maximum).
    fn economy_generated(&self, pc: &PowerConfig) -> Option<f32> {
        let f = &pc.economy_generated;
        let v = match self.val {
            Some(val) => {
                let n = self.world.data.resources.len();
                let mut delta = vec![0.0f32; n];
                for (id, amount) in &f.resources {
                    if let Some(i) = self.world.data.resource_index(id).filter(|&i| i < n) {
                        delta[i] = *amount;
                    }
                }
                val.econ_value(&delta, &val.income)
            }
            None => 0.0,
        };
        let v = f64::from(v) * f64::from(f.multiplier);
        (v >= f64::from(f.minimum)).then_some(v as f32)
    }

    /// `4AD2B0`: the base plus every factor (stored binary32 after each);
    /// 0 when a factor fails or the total is below the minimum; then the
    /// fuzzy factor times an SAI unit draw.
    fn priority(&mut self, op: &SpecialOp, pc: &PowerConfig, actor: Option<ActorId>, at: Vec2) -> f32 {
        let area = op.area;
        let mut total = pc.base;
        let add = |total: &mut f32, v: Option<f32>| -> bool {
            match v {
                Some(v) => {
                    *total = (f64::from(v) + f64::from(*total)) as f32;
                    true
                }
                None => false,
            }
        };
        if !add(&mut total, self.actors_factor(&pc.cv, pc, area, actor, at, true))
            || !add(&mut total, self.actors_factor(&pc.actor_value, pc, area, actor, at, false))
            // 4AB560 ActorEconomy: not loaded by 4AFD50, adds 0.
            || !add(&mut total, self.enemy_influence(pc, at))
            || !add(&mut total, self.influence_ratio(pc, at))
            || !add(&mut total, self.time_since_explored(pc, at))
            || !add(&mut total, self.economy_generated(pc))
        {
            return 0.0;
        }
        if total < pc.minimum {
            return 0.0;
        }
        let u = unit(self.draw());
        (u * f64::from(pc.fuzzy) + f64::from(total)) as f32
    }

    /// `4AD3F0` ON_ACTOR: the visible (virtual +58) live candidates of the
    /// Location requirements; the best priority above zero.
    fn on_actor(&mut self, op: &SpecialOp, pc: &PowerConfig) -> (f32, Option<Place>) {
        let r = &pc.actor_requirements;
        let mut best = 0.0f32;
        let mut place = None;
        for a in self.candidates(r.friendly, r.enemy, r.owned, Vec2::splat(-1.0), -1.0) {
            if !crate::ai::view::knows(self.world, a, self.me) || !alive(self.world, OpTarget::Actor(a)) {
                continue;
            }
            if !self.relation_ok(r, a) || !self.actor_ok(r, a) {
                continue;
            }
            let at = self.world.actors[a as usize].pos.truncate();
            let p = self.priority(op, pc, Some(a), at);
            if best < p {
                best = p;
                place = Some(Place::Actor(a));
            }
        }
        (best, place)
    }

    /// `4AD590` NEAR_ACTOR: around each visible live candidate,
    /// num_candidates points in a direction of the `[B39AF8]` table (draw %
    /// 2048) at a unit draw times the radius; for a summoning ability the
    /// spot must be open (`554500` within twice the candidate's radius,
    /// which may move it); inside the map; the best priority above zero.
    fn near_actor(&mut self, op: &SpecialOp, pc: &PowerConfig) -> (f32, Option<Place>) {
        let r = &pc.actor_requirements;
        let mut best = 0.0f32;
        let mut place = None;
        let summon = op.summon().and_then(|s| self.world.data.find(&s.object));
        let [width, height] = map_extent(self.world);
        for a in self.candidates(r.friendly, r.enemy, r.owned, Vec2::splat(-1.0), -1.0) {
            if !crate::ai::view::knows(self.world, a, self.me) || !alive(self.world, OpTarget::Actor(a)) {
                continue;
            }
            if !self.relation_ok(r, a) || !self.actor_ok(r, a) {
                continue;
            }
            let count = pc.candidates as f32;
            if count <= 0.0 {
                continue;
            }
            let origin = self.world.actors[a as usize].pos.truncate();
            let actor_radius = self.world.data.def(self.world.actors[a as usize].def).blocking_size.map_or(0.0, |s| s[0]);
            let mut i = 0i32;
            while (i as f32) < count {
                i += 1;
                let [dx, dy] = aa_movement::formation::random_direction(self.draw());
                let distance = unit(self.draw()) * f64::from(pc.radius);
                let y_offset = (f64::from(dy) * distance) as f32;
                let mut at = Vec2::new((distance * f64::from(dx) + f64::from(origin.x)) as f32, (f64::from(y_offset) + f64::from(origin.y)) as f32);
                if let Some(def) = summon
                    && !open_position_within(self.world, def, &mut at, actor_radius + actor_radius)
                {
                    continue;
                }
                if !(at.x >= 0.0 && at.x < width && at.y >= 0.0 && at.y < height) {
                    continue;
                }
                let p = self.priority(op, pc, None, at);
                if best < p {
                    best = p;
                    place = Some(Place::Point(at));
                }
            }
        }
        (best, place)
    }

    /// `4AD8C0` FOG: the centre of every region of the primary island; the
    /// best priority above zero.
    fn fog(&mut self, op: &SpecialOp, pc: &PowerConfig) -> (f32, Option<Place>) {
        let world = self.world;
        let Some(regions) = world.ai.regions.as_ref() else { return (0.0, None) };
        let Some(primary) = regions.primary else { return (0.0, None) };
        let centers: Vec<Vec2> = (0..regions.len()).filter(|&r| regions.island[r] == primary).map(|r| regions.centers[r]).collect();
        let mut best = 0.0f32;
        let mut place = None;
        for at in centers {
            let p = self.priority(op, pc, None, at);
            if best < p {
                best = p;
                place = Some(Place::Point(at));
            }
        }
        (best, place)
    }

    /// `4AE190`.
    fn locate(&mut self, op: &SpecialOp, pc: &PowerConfig) -> (f32, Option<Place>) {
        match pc.location {
            1 => self.near_actor(op, pc),
            2 => self.on_actor(op, pc),
            3 => self.fog(op, pc),
            _ => (0.0, None),
        }
    }
}

/// A factor's maximum (when not negative and not above the value) and
/// minimum (None below it).
fn bounded(f: &Factor, mut v: f64) -> Option<f64> {
    if f.maximum >= 0.0 && f64::from(f.maximum) <= v {
        v = f64::from(f.maximum);
    }
    (v >= f64::from(f.minimum)).then_some(v)
}

/// Engaged: the actor attacks (actor +100 bit 4, +118), or its regiment is
/// engaging or engaged.
fn engaged(world: &World, a: ActorId) -> bool {
    if let Some(o) = world.actors[a as usize].org
        && let Some(s) = world.combat.org(o)
        && (s.engaging() || !s.engaged_by.is_empty())
    {
        return true;
    }
    world.actors[a as usize].combat.target.is_some()
}

/// `4D0230(1)`: a body's health over its maximum (extended).
fn health_ratio(world: &World, a: ActorId) -> f64 {
    let x = &world.actors[a as usize];
    let max = world.combat.actor(a).map(|c| c.max_health).filter(|&m| m > 0.0).unwrap_or(world.data.def(x.def).health);
    if max > 0.0 { f64::from(x.health) / f64::from(max) } else { 0.0 }
}

/// `7C42F0` from index 0: weights base² (binary32), their binary32 sum;
/// for each place a unit SAI draw times the sum (binary64) walks the
/// weights from it while each is below the rest, the found power swaps
/// in, the sum loses its weight.
fn ping_pong(list: &mut [(usize, PowerConfig)], rng: &mut SaiRng) {
    let n = list.len();
    if n < 2 {
        return;
    }
    let mut weights: Vec<f32> = Vec::with_capacity(n);
    let mut total = 0.0f32;
    for (_, p) in list.iter() {
        let w = p.base * p.base;
        weights.push(w);
        total += w;
    }
    for start in 0..n {
        let mut r = unit(rng.next_u32()) * f64::from(total);
        let mut j = start;
        if f64::from(weights[start]) < r {
            loop {
                r -= f64::from(weights[j]);
                if j + 1 >= n {
                    break;
                }
                j += 1;
                if f64::from(weights[j]) >= r {
                    break;
                }
            }
            if j != start {
                list.swap(start, j);
                weights.swap(start, j);
            }
        }
        total -= weights[start];
    }
}

pub fn think(world: &mut World) {
    let (Some(data), Some(ai)) = (world.special_ops.data.clone(), world.ai.data.clone()) else { return };
    let players: Vec<(usize, KingdomId, usize, u32)> = world.ai.players.iter().enumerate().map(|(i, p)| (i, p.kingdom, p.ego, p.ge_count)).collect();
    let now = world.time;
    for (pi, kingdom, ego, ge) in players {
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
        let Some(c) = world.special_ops.commanders.get(i) else { continue };
        let (Some(gi), balance) = (c.general, c.experience) else { continue };
        let Some(ego) = ai.egos.get(ego) else { continue };
        let powers = data.powers(&ego.ids);
        // 7C40C0: the commander's abilities the ego configures.
        let mut list: Vec<(usize, PowerConfig)> = data.generals[gi]
            .abilities
            .iter()
            .enumerate()
            .filter_map(|(a, op)| powers.iter().find(|p| p.ability.eq_ignore_ascii_case(&op.id)).map(|p| (a, p.clone())))
            .collect();
        if list.is_empty() {
            continue;
        }
        let mut rng = world.ai.rng.unwrap_or_else(|| SaiRng::from_seed(world.ai.seed));
        ping_pong(&mut list, &mut rng);
        let orders = decide(world, pi, kingdom, ego, &data.generals[gi], &list, balance, &mut rng);
        world.ai.rng = Some(rng);
        // 7C3C10: order each flagged power, then its Followup flare.
        for (a, place, pc) in orders {
            let (target, at) = match place {
                Place::Actor(x) => {
                    if !world.actors.get(x as usize).is_some_and(|y| !y.removed) {
                        continue;
                    }
                    if !world.actors[x as usize].alive {
                        continue;
                    }
                    (AbilityTarget::Actor(x), world.actors[x as usize].pos.truncate())
                }
                Place::Point(p) => (AbilityTarget::Point(p), p),
            };
            if super::use_from_sai(world, kingdom, a, target) {
                world.special_ops.sai.uses += 1;
            }
            if pc.flare_priority != 0.0
                && pc.flare_duration > 0.0
                && let Some(r) = world.ai.regions.as_ref().map(|r| r.region_at(at))
            {
                world.ai.players[pi].flares.push((crate::ai::goals::Key::Attack(r), pc.flare_priority, now + pc.flare_duration));
            }
        }
    }
}

/// `7C3A60`: in order, while the experience left covers each power's cost,
/// its location; the flagged powers with their places.
#[allow(clippy::too_many_arguments)]
fn decide(world: &World, pi: usize, me: KingdomId, ego: &crate::ai::data::Ego, general: &General, list: &[(usize, PowerConfig)], balance: f32, rng: &mut SaiRng) -> Vec<(usize, Place, PowerConfig)> {
    let player = &world.ai.players[pi];
    let view;
    let census;
    let ctx;
    let val;
    let valuation = match (world.ai.regions.as_ref(), crate::ai::view::kingdom_index(world, me)) {
        (Some(regions), Some(index)) => {
            view = crate::ai::view::View::build(world, &world.ai.bound, index);
            ctx = crate::ai::goals::Ctx { view: &view, bound: &world.ai.bound, ego, regions, inf: &world.ai.influence, now: world.time };
            census = crate::ai::value::Census::build(&ctx);
            val = Val::new(&ctx, &census);
            Some(&val)
        }
        _ => None,
    };
    let mut eval = Eval { world, me, player, val: valuation, rng };
    let mut left = balance;
    let mut out = Vec::new();
    for (a, pc) in list {
        let op = &general.abilities[*a];
        // 4CDD40: the integer cost as a float.
        if left < op.cost {
            break;
        }
        left -= op.cost;
        let (best, place) = eval.locate(op, pc);
        if best > 0.0
            && let Some(place) = place
        {
            out.push((*a, place, pc.clone()));
        }
    }
    out
}
