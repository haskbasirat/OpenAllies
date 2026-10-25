//! Placement stages of KKC_RandomMapGenerator::_GenerateMap (0x5925E0):
//! - StagePlaceKingdoms (RMGPlaceKingdomsStage.cpp 0x5963B0): team markers,
//!   then kingdom markers, spread by the marker balancer
//!   (KKC_RMGBalancer::BalanceMarkers 0x590B10, pair penalty 0x590E10, the
//!   kingdom balancer callbacks 0x5954B0..0x5956A0);
//! - StagePlaceRequiredObjects (0x593F30 / PlaceRequiredObject 0x591E60);
//! - StagePlaceNonRequiredObjects (0x595180: PlaceGroup 0x594590 and
//!   BalancePlaceGroup 0x594E60), with the group context callbacks
//!   (search circle 0x593EB0, score 0x595340, validity 0x595430);
//! - FindRandomPosition (0x591A30, 90 tries, lowest score) and
//!   CheckPositionValid (0x591350: terrain, main land region, overlap);
//! - the doodad seeding (PlaceAutoDoodads 0x54FB40 over every tile, weighted
//!   picks 0x4B6FF0) and CleanupBlockingDoodads (0x592170).
use super::data::{GroupDef, RmcDef, RmgData};
use super::grid::{Batch, World};
use super::rng::Rng;

#[derive(Clone, Debug)]
pub(super) struct Object {
    pub thing: usize,
    pub pos: [f32; 2],
    pub facing: [f32; 2],
    pub scale: f32,
    pub variant: i32,
    /// Index into `Placement::kingdoms`.
    pub owner: Option<usize>,
    pub group: Option<usize>,
}

#[derive(Clone, Debug)]
pub(super) struct Kingdom {
    /// Index into `RmcDef::kingdoms`.
    pub def: usize,
    pub major: bool,
    pub team: Option<u32>,
    /// The start location object: the balanced marker until the Corps HQ
    /// placeholder (starting_location_priority 1000) moves it onto the HQ.
    pub start: [f32; 2],
    pub start_priority: i32,
    /// The balanced marker (where start_loc stands).
    pub marker: [f32; 2],
    /// Position of the highest-priority owned actor (the Corps HQ).
    pub hq: Option<[f32; 2]>,
}

struct GroupInst {
    def: GroupDef,
    ids: String,
    kingdom: Option<usize>,
    objects: Vec<usize>,
    /// The group instance (0x593690) bounding box (min x, min y, max x,
    /// max y), its centre and radius (0x593700).
    bbox: [f32; 4],
    centre: [f32; 2],
    radius: f32,
    /// The parsed restriction list (0x4A2150): (target group, proximity_min,
    /// proximity_desired). A wildcard lists the groups parsed before this one
    /// whose ids start with its prefix.
    restr: Vec<(usize, f32, f32)>,
    /// Group +0x50 resolved by 0x4F48E0 (search_relative_kingdom_ids) and
    /// +0x4C (search_relative_group_ids).
    search_kingdom: Option<usize>,
    search_group: Option<usize>,
}

#[path = "place_exact.rs"]
mod exact;

pub(super) struct Placement<'a> {
    data: &'a RmgData,
    rmc: &'a RmcDef,
    pub objects: Vec<Object>,
    pub scenery: Vec<Object>,
    pub kingdoms: Vec<Kingdom>,
    groups: Vec<GroupInst>,
    passable: Vec<bool>,
    region: Vec<u32>,
    main_region: u32,
    /// Footprint radius per thing (settlements include their layouts).
    radii: Vec<f32>,
    pub notes: Vec<String>,
    /// The marker balancer's (iteration, score) log of the kingdom stage
    /// (the original's "balance iteration %d score %f" lines).
    pub balance_log: Vec<(i32, f64)>,
    /// The GMap occupancy the original's placement queries read.
    space: super::space::Space,
}

const DEFAULT_FACING: [f32; 2] = [0.0, -1.0];
/// BuildingPaddingMeters (SVars).
const PADDING: f32 = 2.0;

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

impl<'a> Placement<'a> {
    pub(super) fn new(data: &'a RmgData, rmc: &'a RmcDef, world: &World) -> Self {
        let mut p = Self {
            data,
            rmc,
            objects: Vec::new(),
            scenery: Vec::new(),
            kingdoms: Vec::new(),
            groups: Vec::new(),
            passable: Vec::new(),
            region: Vec::new(),
            main_region: 0,
            radii: Vec::new(),
            notes: Vec::new(),
            balance_log: Vec::new(),
            space: super::space::Space::new(world, data),
        };
        p.radii = (0..data.things.len()).map(|t| p.compute_radius(t)).collect();
        p.update_regions(world);
        p
    }

    /// Land-passable tiles (terrain `land`, not the blocked outer ring) and
    /// their connected regions; the largest is the main land region the
    /// original requires every placed object to stand in (0x591350).
    pub(super) fn update_regions(&mut self, world: &World) {
        let (w, h) = (world.tw, world.th);
        self.passable = (0..w * h)
            .map(|t| {
                let (x, y) = (t % w, t / w);
                let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
                !edge && self.data.terrains[self.data.variations[world.var[t] as usize].terrain].land
            })
            .collect();
        self.region = vec![0; w * h];
        let mut sizes = vec![0usize];
        let mut stack = Vec::new();
        for s in 0..w * h {
            if !self.passable[s] || self.region[s] != 0 {
                continue;
            }
            let id = sizes.len() as u32;
            sizes.push(0);
            self.region[s] = id;
            stack.push(s);
            while let Some(t) = stack.pop() {
                sizes[id as usize] += 1;
                let (x, y) = ((t % w) as i32, (t / w) as i32);
                for d in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
                    if let Some(n) = world.tile_at(x + d[0], y + d[1]) {
                        if self.passable[n] && self.region[n] == 0 {
                            self.region[n] = id;
                            stack.push(n);
                        }
                    }
                }
            }
        }
        self.main_region = (1..sizes.len()).max_by_key(|&i| (sizes[i], std::cmp::Reverse(i))).unwrap_or(0) as u32;
    }

    pub(super) fn in_main_region(&self, world: &World, p: [f32; 2]) -> bool {
        world.tile_of(p).is_some_and(|t| self.passable[t] && self.region[t] == self.main_region)
    }

    fn radius(&self, thing: usize) -> f32 {
        self.radii[thing]
    }
    fn compute_radius(&self, thing: usize) -> f32 {
        let t = &self.data.things[thing];
        let mut r = t.half_extent[0].max(t.half_extent[1]);
        for layout in &t.layouts {
            for (id, off, _) in layout {
                let h = self.data.thing(id).map_or(4.0, |b| b.half_extent[0].max(b.half_extent[1]));
                r = r.max(off[0].abs().max(off[1].abs()) + h);
            }
        }
        r
    }

    /// CheckPositionValid (0x591350) without the group context.
    fn position_valid(&self, world: &World, thing: usize, pos: [f32; 2]) -> bool {
        let r = self.radius(thing);
        if pos[0] < r || pos[1] < r || pos[0] > world.width - r || pos[1] > world.height - r {
            return false;
        }
        if !self.in_main_region(world, pos) {
            return false;
        }
        // Every tile under the footprint passable and in the main region.
        let t0 = |v: f32| ((v / world.tile).floor() as i32).max(0);
        let (x0, y0) = (t0(pos[0] - r), t0(pos[1] - r));
        let (x1, y1) = (t0(pos[0] + r).min(world.tw as i32 - 1), t0(pos[1] + r).min(world.th as i32 - 1));
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let t = ty as usize * world.tw + tx as usize;
                if !self.passable[t] || self.region[t] != self.main_region {
                    return false;
                }
            }
        }
        // No overlap with blocking objects already placed.
        if r > 0.0 {
            for o in &self.objects {
                let def = &self.data.things[o.thing];
                if !def.blocking {
                    continue;
                }
                let ro = self.radius(o.thing);
                if ro > 0.0 && dist(o.pos, pos) < ro + r + PADDING {
                    return false;
                }
            }
        }
        true
    }

    /// 0x556070 (called by CheckPositionValid with a 12 m search radius): an
    /// invalid position is moved to the first valid point on rings of
    /// `step` = the object radius (a quarter tile at least, half the radius
    /// above one tile) out to 12 m, 8 directions per ring starting at the
    /// facing and turning by 45 degrees.
    fn snap_valid(&self, world: &World, thing: usize, pos: [f32; 2], facing: [f32; 2]) -> Option<[f32; 2]> {
        if self.position_valid(world, thing, pos) {
            return Some(pos);
        }
        const SEARCH: f32 = 12.0;
        let r = self.radius(thing);
        let step = if r < world.tile * 0.25 {
            world.tile * 0.25
        } else if r > world.tile {
            r * 0.5
        } else {
            r
        };
        let rings = ((SEARCH / step) as i32).clamp(1, 20);
        let (sin, cos) = (-std::f32::consts::FRAC_PI_4).sin_cos();
        for ring in 1..=rings {
            let d = ring as f32 * (SEARCH / rings as f32);
            let (mut dx, mut dy) = (d * facing[0], d * facing[1]);
            for _ in 0..8 {
                let p = [pos[0] + dx, pos[1] + dy];
                if self.position_valid(world, thing, p) {
                    return Some(p);
                }
                let (nx, ny) = (dx * cos - dy * sin, dx * sin + dy * cos);
                dx = nx;
                dy = ny;
            }
        }
        None
    }

    fn matches(pattern: &str, g: &GroupInst) -> bool {
        if let Some(prefix) = pattern.strip_suffix('*') {
            g.ids.to_ascii_lowercase().starts_with(&prefix.to_ascii_lowercase())
        } else {
            g.ids.eq_ignore_ascii_case(pattern) || g.def.ids.eq_ignore_ascii_case(pattern)
        }
    }
    /// The restrictions group `gi` itself declares, against the groups they
    /// match. Only the placing group's own list counts (validity 0x595430,
    /// score 0x595340, balanced self-score 0x5944A0 all read group+68): a
    /// restriction a group declares against one placed after it (StartCorps
    /// against the villages) constrains nothing, so villages may land next to
    /// a start. A wildcard never matches the group itself (StartCorps
    /// restricts "start_corps*" to 96 m yet places its depot within 16 m).
    fn restrictions(&self, gi: usize) -> Vec<(usize, f32, f32)> {
        let g = &self.groups[gi];
        let mut out = Vec::new();
        for (oi, o) in self.groups.iter().enumerate() {
            if oi == gi {
                continue;
            }
            for r in &g.def.restrictions {
                if Self::matches(&r.group_ids, o) {
                    out.push((oi, r.min, r.desired));
                }
            }
        }
        out
    }
    /// Group validity (0x595430 -> 0x594E00).
    fn context_valid(&self, gi: usize, pos: [f32; 2]) -> bool {
        let g = &self.groups[gi];
        if g.objects.iter().any(|&o| dist(self.objects[o].pos, pos) < g.def.intra_min) {
            return false;
        }
        for (oi, min, _) in self.restrictions(gi) {
            if self.groups[oi].objects.iter().any(|&o| dist(self.objects[o].pos, pos) < min) {
                return false;
            }
        }
        true
    }
    /// Group score (0x595340), lower is better.
    fn context_score(&self, gi: usize, pos: [f32; 2]) -> f32 {
        let g = &self.groups[gi];
        let mut score = 0.0;
        // 0x595340: only a kingdom with a start location object (a playing
        // side, never the independents owning the villages) pulls its group.
        if let Some(k) = g.kingdom.filter(|&k| self.kingdoms[k].major) {
            if g.def.start_weight != 0.0 {
                score = dist(self.kingdoms[k].start, pos) * g.def.start_weight;
            }
        }
        let penalty = |objs: &[usize], desired: f32| -> f32 { objs.iter().map(|&o| (desired - dist(self.objects[o].pos, pos)).abs()).sum() };
        if g.def.intra_desired != 0.0 {
            score += penalty(&g.objects, g.def.intra_desired);
        }
        for (oi, _, desired) in self.restrictions(gi) {
            if desired != 0.0 {
                score += penalty(&self.groups[oi].objects, desired);
            }
        }
        score
    }
    /// Search circle (0x593EB0).
    fn search_circle(&self, gi: usize) -> Option<([f32; 2], f32)> {
        let g = &self.groups[gi];
        if g.def.search_radius <= 0.0 {
            return None;
        }
        if g.def.search_relative_kingdom.is_some() {
            let k = match g.def.search_relative_kingdom.as_deref() {
                Some(s) if s.eq_ignore_ascii_case("this") => g.kingdom,
                Some(s) => self.kingdoms.iter().position(|k| self.rmc.kingdoms[k.def].ids.eq_ignore_ascii_case(s)),
                None => None,
            }?;
            return Some((self.kingdoms[k].start, g.def.search_radius));
        }
        let other = match g.def.search_relative_group.as_deref() {
            Some(s) if s.eq_ignore_ascii_case("this") => Some(gi),
            Some(s) => self.groups.iter().position(|o| Self::matches(s, o)),
            None => None,
        }?;
        let objs = &self.groups[other].objects;
        if objs.is_empty() {
            return None;
        }
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for &o in objs {
            let p = self.objects[o].pos;
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
        }
        Some(([(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5], g.def.search_radius))
    }

    /// FindRandomPosition (0x591A30): 90 tries, the valid one with the lowest score.
    /// `rounds`: required objects ("conform", 0x591E60 passes 1) get up to 4
    /// rounds of `tries`, stopping after the first round that found a valid
    /// position; other groups get one.
    fn find_position(&self, world: &World, rng: &mut Rng, gi: Option<usize>, thing: usize, tries: u32, rounds: u32) -> Option<[f32; 2]> {
        let d = self.radius(thing) + world.tile;
        let circle = gi.and_then(|g| self.search_circle(g));
        let mut best: Option<(f32, [f32; 2])> = None;
        let mut tried = 0u32;
        for _ in 0..tries * rounds.max(1) {
            let pos = match circle {
                Some((c, r)) => {
                    let x = rng.signed() * r + c[0];
                    let y = rng.signed() * r + c[1];
                    [x.clamp(d, world.width - d), y.clamp(d, world.height - d)]
                }
                None => [(world.width - d - d) * rng.unit() + d, (world.height - d - d) * rng.unit() + d],
            };
            // CheckPositionValid: snapped to valid terrain, then the group rules.
            let snapped = self.snap_valid(world, thing, pos, DEFAULT_FACING);
            let valid = snapped.is_some_and(|p| gi.is_none_or(|g| self.context_valid(g, p)));
            let pos = snapped.unwrap_or(pos);
            if !valid {
                tried += 1;
                if tried % tries == 0 && best.is_some() {
                    break;
                }
                continue;
            }
            let score = gi.map_or(0.0, |g| self.context_score(g, pos));
            if best.is_none_or(|b| score < b.0) {
                best = Some((score, pos));
            }
            tried += 1;
            if tried % tries == 0 && best.is_some() {
                break;
            }
        }
        best.map(|b| b.1)
    }

    fn random_facing(&self, thing: usize, rng: &mut Rng) -> [f32; 2] {
        if self.data.things[thing].random_orientation {
            let a = rng.unit() * std::f32::consts::TAU;
            [a.cos(), a.sin()]
        } else {
            DEFAULT_FACING
        }
    }

    /// The `properties` of a generated group (RMCActorGroup), given to
    /// every actor it places (e.g. tech_noupkeep on the start bunkers).
    pub(super) fn group_properties(&self, group: Option<usize>) -> Vec<String> {
        group
            .and_then(|g| self.groups.get(g))
            .and_then(|g| g.def.properties.as_deref())
            .map(|s| s.split([',', ' ', '\t']).map(str::trim).filter(|p| !p.is_empty() && !p.eq_ignore_ascii_case("null")).map(str::to_string).collect())
            .unwrap_or_default()
    }

    fn add(&mut self, thing: usize, pos: [f32; 2], facing: [f32; 2], owner: Option<usize>, group: Option<usize>, rng: &mut Rng) -> usize {
        let def = &self.data.things[thing];
        let variant = if def.models.len() > 1 { rng.below(def.models.len() as u32) as i32 } else { -1 };
        let scale = if def.scale[1] > def.scale[0] { rng.range(def.scale) } else { def.scale[0].max(1e-3) };
        let index = self.objects.len();
        let moves_start = owner.is_some_and(|k| self.kingdoms[k].major && def.start_priority > 0 && def.start_priority > self.kingdoms[k].start_priority);
        self.objects.push(Object { thing, pos, facing, scale, variant, owner, group });
        if let Some(g) = group {
            self.groups[g].objects.push(index);
        }
        if moves_start {
            // 0x5910D0 -> KKC_GWorld::PrepareMapFixStartLocations (0x4FB640):
            // creating an actor with starting_location_priority > 0 moves the
            // kingdom's start location object (kingdom+1F8) to its owned actor
            // with the highest priority (0x4F1C20) - the Corps HQ placeholder.
            // Every later "relative to this kingdom" search (the bunker,
            // anti-air and artillery groups) is centred on it.
            let k = owner.unwrap();
            self.kingdoms[k].start_priority = def.start_priority;
            self.kingdoms[k].start = pos;
            self.kingdoms[k].hq = Some(pos);
            if let Some(start_thing) = self.data.thing_by_id.get("start_loc").copied() {
                for o in self.objects.iter_mut() {
                    if o.owner == Some(k) && o.thing == start_thing {
                        o.pos = pos;
                    }
                }
            }
        }
        index
    }

    /// Weighted pick of a group object; random actors resolve to one of
    /// their member things (RandomActor weights).
    fn resolve(&self, ids: &str, rng: &mut Rng) -> Option<usize> {
        if let Some(list) = self.data.random_actors.get(&ids.to_ascii_lowercase()) {
            let total: f32 = list.iter().map(|e| e.1.max(0.0)).sum();
            if total > 0.0 {
                let mut r = rng.unit() * total - list[0].1;
                let mut i = 0;
                while r >= 0.0 && i + 1 < list.len() {
                    i += 1;
                    r -= list[i].1;
                }
                return self.data.thing_by_id.get(&list[i].0.to_ascii_lowercase()).copied();
            }
        }
        self.data.thing_by_id.get(&ids.to_ascii_lowercase()).copied()
    }

    // ---------------- kingdoms ----------------

    /// Active kingdoms: the first `sides` playable majors plus every minor.
    pub(super) fn setup_kingdoms(&mut self, sides: u32, teams: &[u32]) -> anyhow::Result<()> {
        let mut majors = 0u32;
        for (i, k) in self.rmc.kingdoms.iter().enumerate() {
            if k.independent {
                self.kingdoms.push(Kingdom { def: i, major: false, team: None, start: [0.0, 0.0], start_priority: 0, marker: [0.0, 0.0], hq: None });
            } else if majors < sides {
                let team = teams.get(majors as usize).copied();
                self.kingdoms.push(Kingdom { def: i, major: true, team, start: [0.0, 0.0], start_priority: 0, marker: [0.0, 0.0], hq: None });
                majors += 1;
            }
        }
        anyhow::ensure!(majors == sides, "{} has {majors} playable sides, {sides} requested", self.rmc.ids);
        Ok(())
    }

    /// BalanceMarkers (0x590B10) over marker positions.
    fn balance(&self, world: &World, rng: &mut Rng, pos: &mut [[f32; 2]], desired: &dyn Fn(usize, usize) -> f32, margin: f32, valid: &dyn Fn(usize, [f32; 2]) -> bool, self_score: &dyn Fn(usize, [f32; 2], &mut [f32; 2]) -> f32) {
        let sv = &self.data.svars;
        let n = pos.len();
        let mut step = vec![sv.nudge_step; n];
        let mut best = pos.to_vec();
        let (mut best_total, mut prev_total) = (f32::MAX, f32::MAX);
        let mut worse = 0;
        let mut prev_score = vec![f32::MAX; n];
        let mut score = vec![0.0f32; n];
        for iter in 0..=sv.balance_iterations.max(0) {
            let mut force = vec![[0.0f32; 2]; n];
            let mut total = 0.0;
            for i in 0..n {
                let mut s = 0.0;
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    s += pair_penalty(rng, pos[i], pos[j], desired(i, j), &mut force[i]);
                }
                s += self_score(i, pos[i], &mut force[i]);
                score[i] = s;
                total += s;
            }
            if total < best_total {
                best_total = total;
                best.copy_from_slice(pos);
                if worse > 0 {
                    worse -= 1;
                }
                if total < 1e-4 {
                    break;
                }
            }
            if prev_total <= total {
                worse += 1;
                if worse > sv.balance_worse {
                    break;
                }
            }
            prev_total = total;
            if iter == sv.balance_iterations {
                break;
            }
            for i in 0..n {
                let f = force[i];
                let l = (f[0] * f[0] + f[1] * f[1]).sqrt();
                let dir = if l > 0.0 { [f[0] / l, f[1] / l] } else { f };
                let lo = margin;
                let x = (dir[0] * step[i] + pos[i][0]).clamp(lo, world.width - margin - 1e-4);
                let y = (dir[1] * step[i] + pos[i][1]).clamp(lo, world.height - margin - 1e-4);
                if valid(i, [x, y]) {
                    pos[i] = [x, y];
                } else {
                    step[i] += sv.nudge_delta;
                }
                // RMGBalanceMarkerNudgeStepDelta: added for a better score,
                // subtracted (down to the base step) for a worse one.
                if score[i] < prev_score[i] {
                    step[i] += sv.nudge_delta;
                } else if sv.nudge_step < step[i] {
                    step[i] -= sv.nudge_delta;
                }
                prev_score[i] = score[i];
            }
        }
        pos.copy_from_slice(&best);
    }

    /// KKC_RMGPlaceKingdomsStage::Run (0x5963B0) with BalancePlaceKingdoms
    /// (0x595940): one team marker per team with members (team order), then
    /// per kingdom without a team; all start in the map centre and are
    /// spread by the marker balancer (0x590B10) to W / (trunc(sqrt(n)) + 1);
    /// each kingdom marker then starts on its team marker's best position and
    /// is balanced again (same-team pairs to min(team_desired_distance, that
    /// distance)). The kingdoms' start locations are their markers' best
    /// positions. Returns the balance log (iteration, score) of both passes.
    pub(super) fn place_kingdoms_exact(&mut self, world: &World, rng: &mut Rng) -> anyhow::Result<Vec<(i32, f64)>> {
        let mut log = Vec::new();
        let majors: Vec<usize> = (0..self.kingdoms.len()).filter(|&k| self.kingdoms[k].major).collect();
        let start_thing = self.data.thing_by_id.get("start_loc").copied();
        // 0x595530: tile + the start location's shape radius (ObjectData
        // +0x3C) + 0.0001, on the register.
        let radius = start_thing.map_or(0.0, |t| self.data.things[t].shape_radius);
        let margin = f64::from(world.tile) + f64::from(radius) + f64::from(f32::from_bits(0x38D1_B717));
        let orient = DEFAULT_FACING;
        if majors.is_empty() {
            return Ok(log);
        }
        let sv = &self.data.svars;
        let (iterations, max_worse) = (sv.balance_iterations, sv.balance_worse);
        // Team markers: teams with members in team order, then teamless kingdoms.
        let mut teams: Vec<u32> = majors.iter().filter_map(|&k| self.kingdoms[k].team).collect();
        teams.sort_unstable();
        teams.dedup();
        let mut keys: Vec<(Option<u32>, Option<usize>)> = teams.iter().map(|&t| (Some(t), None)).collect();
        keys.extend(majors.iter().filter(|&&k| self.kingdoms[k].team.is_none()).map(|&k| (None, Some(k))));
        if majors.len() == 1 {
            // A single kingdom (0x595940 first branch): up to 50 uniform
            // positions in [d, W - d] with d = tile + radius + 0.0001, the
            // first valid one (CheckPositionValid without context).
            let d = (f64::from(world.tile) + f64::from(radius) + f64::from(f32::from_bits(0x38D1_B717))) as f32;
            let d2 = (f64::from(d) + f64::from(d)) as f32;
            for _ in 0..50 {
                let x = ((f64::from(world.width) - f64::from(d2)) * rng.at(0x595A43).unit_d() + f64::from(d)) as f32;
                let y = ((f64::from(world.height) - f64::from(d2)) * rng.at(0x595A8B).unit_d() + f64::from(d)) as f32;
                if self.in_main_region(world, [x, y]) {
                    self.kingdoms[majors[0]].start = [x, y];
                    self.kingdoms[majors[0]].marker = [x, y];
                    return Ok(log);
                }
            }
            anyhow::bail!("could not place the only kingdom");
        }
        let centre = [(f64::from(world.width) * 0.5) as f32, (f64::from(world.height) * 0.5) as f32];
        let mut team_markers: Vec<Marker> = (0..keys.len()).map(|i| Marker::new(i, centre, orient, None)).collect();
        let n = team_markers.len() as i32;
        let desired = (f64::from(world.width) / f64::from(blob_ftol((f64::from(n)).sqrt()) + 1)) as f32;
        let team_cb = KingdomBalance { mode: 0, desired, same_team: desired, other: desired, margin, teams: Vec::new() };
        balance_markers(world, rng, &mut team_markers, 2.0, 2.0, &team_cb, iterations, max_worse, &mut log);
        // Kingdom markers on their team marker's best position.
        let mut kingdom_markers: Vec<Marker> = Vec::new();
        let mut kingdom_teams = Vec::new();
        for (i, &k) in majors.iter().enumerate() {
            let key = match self.kingdoms[k].team {
                Some(t) => (Some(t), None),
                None => (None, Some(k)),
            };
            let at = keys.iter().position(|&x| x == key).map_or(centre, |m| team_markers[m].best);
            kingdom_markers.push(Marker::new(i, at, orient, Some(k)));
            kingdom_teams.push(self.kingdoms[k].team);
        }
        let same_team = if desired < self.rmc.team_desired_distance { desired } else { self.rmc.team_desired_distance };
        let kingdom_cb = KingdomBalance { mode: 1, desired, same_team, other: desired, margin, teams: kingdom_teams };
        balance_markers(world, rng, &mut kingdom_markers, 2.0, 2.0, &kingdom_cb, iterations, max_worse, &mut log);
        for (m, &k) in kingdom_markers.iter().zip(&majors) {
            self.kingdoms[k].start = m.best;
            self.kingdoms[k].marker = m.best;
        }
        Ok(log)
    }

    /// The behaviour-level kingdom placement (kept for reference).
    #[allow(dead_code)]
    pub(super) fn place_kingdoms(&mut self, world: &World, rng: &mut Rng) -> anyhow::Result<()> {
        let majors: Vec<usize> = (0..self.kingdoms.len()).filter(|&k| self.kingdoms[k].major).collect();
        let start_thing = self.data.thing_by_id.get("start_loc").copied();
        let margin = world.tile + start_thing.map_or(0.0, |t| self.radius(t)) + 1e-4;
        if majors.len() == 1 {
            // A single kingdom: 50 random positions until one is valid.
            for _ in 0..50 {
                let p = [(world.width - margin * 2.0) * rng.unit() + margin, (world.height - margin * 2.0) * rng.unit() + margin];
                if self.in_main_region(world, p) {
                    self.kingdoms[majors[0]].start = p;
                    self.kingdoms[majors[0]].marker = p;
                    return Ok(());
                }
            }
            anyhow::bail!("could not place the only kingdom");
        }
        // Team markers: one per team with members, one per kingdom without team.
        let mut markers: Vec<Option<u32>> = Vec::new();
        for &k in &majors {
            match self.kingdoms[k].team {
                Some(t) if markers.contains(&Some(t)) => {}
                t => markers.push(t),
            }
        }
        let center = [world.width * 0.5, world.height * 0.5];
        let team_desired = world.width / ((markers.len() as f32).sqrt() as i32 + 1) as f32;
        let mut team_pos = vec![center; markers.len()];
        if markers.len() > 1 {
            self.balance(world, rng, &mut team_pos, &|_, _| team_desired, margin, &|_, _| true, &|_, _, _| 0.0);
        }
        // Kingdom markers start on their team's marker.
        let mut pos: Vec<[f32; 2]> = majors
            .iter()
            .map(|&k| {
                let t = self.kingdoms[k].team;
                let m = markers.iter().position(|&m| m == t && t.is_some()).or_else(|| markers.iter().position(|&m| m.is_none() && t.is_none())).unwrap_or(0);
                team_pos[m]
            })
            .collect();
        // Kingdoms without a team each have their own marker: use them in order.
        let mut solo = markers.iter().enumerate().filter(|(_, m)| m.is_none()).map(|(i, _)| i);
        for (i, &k) in majors.iter().enumerate() {
            if self.kingdoms[k].team.is_none() {
                if let Some(m) = solo.next() {
                    pos[i] = team_pos[m];
                }
            }
        }
        let same_team = self.rmc.team_desired_distance.min(team_desired);
        let teams: Vec<Option<u32>> = majors.iter().map(|&k| self.kingdoms[k].team).collect();
        self.balance(
            world,
            rng,
            &mut pos,
            &|i, j| if teams[i].is_some() && teams[i] == teams[j] { same_team } else { team_desired },
            margin,
            &|_, _| true,
            &|_, _, _| 0.0,
        );
        for (i, &k) in majors.iter().enumerate() {
            // CreateStartingLocation: the nearest main-region land to the marker.
            let p = self.nearest_valid(world, pos[i]).ok_or_else(|| anyhow::anyhow!("no land for kingdom {}", i))?;
            self.kingdoms[k].start = p;
            self.kingdoms[k].marker = p;
        }
        Ok(())
    }

    fn nearest_valid(&self, world: &World, p: [f32; 2]) -> Option<[f32; 2]> {
        if self.in_main_region(world, p) {
            return Some(p);
        }
        let (x, y) = ((p[0] / world.tile) as i32, (p[1] / world.tile) as i32);
        for r in 1..(world.tw.max(world.th) as i32) {
            let mut best: Option<(f32, [f32; 2])> = None;
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    if let Some(t) = world.tile_at(x + dx, y + dy) {
                        if self.passable[t] && self.region[t] == self.main_region {
                            let c = world.tile_center(t);
                            let d = dist(c, p);
                            if best.is_none_or(|b| d < b.0) {
                                best = Some((d, c));
                            }
                        }
                    }
                }
            }
            if let Some(b) = best {
                return Some(b.1);
            }
        }
        None
    }

    // ---------------- groups ----------------

    /// 0x4A4600 expands apply_to_all_major_kingdoms into one group per major.
    pub(super) fn setup_groups(&mut self) {
        let lists = [&self.rmc.actor_groups, &self.rmc.doodad_groups];
        let inst = |def: &GroupDef, ids: String, kingdom: Option<usize>| GroupInst {
            def: def.clone(),
            ids,
            kingdom,
            objects: Vec::new(),
            bbox: [0.0; 4],
            centre: [0.0; 2],
            radius: 0.0,
            restr: Vec::new(),
            search_kingdom: None,
            search_group: None,
        };
        for list in lists {
            for g in list.iter() {
                if g.all_major_kingdoms {
                    for (k, kd) in self.kingdoms.iter().enumerate() {
                        if kd.major {
                            let ids = format!("{}_{}", g.ids, self.rmc.kingdoms[kd.def].ids);
                            self.groups.push(inst(g, ids, Some(k)));
                        }
                    }
                } else {
                    let kingdom = g.kingdom_ids.as_ref().and_then(|ids| self.kingdoms.iter().position(|k| self.rmc.kingdoms[k.def].ids.eq_ignore_ascii_case(ids)));
                    if g.kingdom_ids.is_some() && kingdom.is_none() {
                        continue;
                    }
                    self.groups.push(inst(g, g.ids.clone(), kingdom));
                }
            }
        }
        // 0x4A2150 restrictions in declaration order, wildcards matching the
        // groups parsed before; 0x4A31A0 / 0x4A4600 search references.
        for gi in 0..self.groups.len() {
            let mut restr = Vec::new();
            for r in &self.groups[gi].def.restrictions {
                if let Some(at) = r.group_ids.find('*') {
                    let prefix = &r.group_ids[..at];
                    for oi in 0..gi {
                        if self.groups[oi].ids.get(..prefix.len()) == Some(prefix) {
                            restr.push((oi, r.min, r.desired));
                        }
                    }
                } else if let Some(oi) = self.groups.iter().position(|o| o.ids == r.group_ids) {
                    restr.push((oi, r.min, r.desired));
                }
            }
            let g = &self.groups[gi];
            let search_kingdom = match g.def.search_relative_kingdom.as_deref() {
                Some("this") => g.kingdom,
                Some(s) => self.kingdoms.iter().position(|k| self.rmc.kingdoms[k.def].ids.eq_ignore_ascii_case(s)),
                None => None,
            };
            let search_group = match g.def.search_relative_group.as_deref() {
                Some("this") => Some(gi),
                Some(s) => self.groups.iter().position(|o| o.def.ids == s || o.ids == s),
                None => None,
            };
            let g = &mut self.groups[gi];
            g.restr = restr;
            g.search_kingdom = search_kingdom;
            g.search_group = search_group;
        }
    }

    /// The generated groups (AA_RMG_GROUPS debugging aid).
    pub(super) fn describe_groups(&self) -> String {
        let mut s = String::new();
        for (i, g) in self.groups.iter().enumerate() {
            let d = &g.def;
            s += &format!(
                "group {i} {} (def {}) kingdom {:?} required {:?} objects {} balanced {} search {} rel_group {:?} rel_kingdom {:?} intra {} {} start_weight {} restrictions {:?}\n",
                g.ids,
                d.ids,
                g.kingdom.map(|k| &self.rmc.kingdoms[self.kingdoms[k].def].ids),
                d.required.iter().map(|r| (&r.object, r.position, r.distance_max)).collect::<Vec<_>>(),
                d.objects.len(),
                d.balanced,
                d.search_radius,
                d.search_relative_group,
                d.search_relative_kingdom,
                d.intra_min,
                d.intra_desired,
                d.start_weight,
                d.restrictions.iter().map(|r| (&r.group_ids, r.min, r.desired)).collect::<Vec<_>>()
            );
            for r in &d.required {
                if let Some(&t) = self.data.thing_by_id.get(&r.object.to_ascii_lowercase()) {
                    let t = &self.data.things[t];
                    s += &format!(
                        "  thing {} category {} kind {} flags {:08x} shape {:08x?} radius {} start_priority {}\n",
                        t.ids, t.category, t.native_kind, t.flags, t.shape, t.shape_radius, t.start_priority
                    );
                }
            }
        }
        for id in ["start_loc", "southerneurope_village01"] {
            if let Some(&t) = self.data.thing_by_id.get(id) {
                let t = &self.data.things[t];
                s += &format!("  thing {} category {} kind {} flags {:08x} shape {:08x?} radius {}\n", t.ids, t.category, t.native_kind, t.flags, t.shape, t.shape_radius);
            }
        }
        s
    }

    /// CreateStartingLocation (0x595820 -> 0x5910D0 -> CreateGActor) for
    /// every placed kingdom, in kingdom order, on its marker.
    pub(super) fn create_start_locations(&mut self, rng: &mut Rng) {
        if let Some(t) = self.data.thing_by_id.get("start_loc").copied() {
            for k in 0..self.kingdoms.len() {
                if self.kingdoms[k].major {
                    let p = self.kingdoms[k].start;
                    self.create_exact(t, p, DEFAULT_FACING, -1, Some(k), None, rng);
                }
            }
        }
    }

    /// CreateGThing's world-RNG draws for a new thing of `thing` (0x4F9C10:
    /// the GThing constructor's random facing 0x4EF2A0 when the definition
    /// orients randomly, the variant 0x49B900 when one is requested as -1,
    /// the scale 0x49B440 in InitCreate), then the object is recorded.
    /// Component draws follow in later stages of this port.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_exact(&mut self, thing: usize, pos: [f32; 2], facing: [f32; 2], variant: i32, owner: Option<usize>, group: Option<usize>, rng: &mut Rng) -> usize {
        let index = self.create_thing(thing, pos, facing, variant, owner, group, rng);
        if let Some(g) = group {
            self.groups[g].objects.push(index);
        }
        index
    }

    fn area_scale(&self, world: &World, factor: f32) -> f32 {
        if factor == 0.0 {
            return 1.0;
        }
        let s = world.width / self.rmc.reference_size;
        if factor == 1.0 {
            s
        } else if factor == 2.0 {
            s * s
        } else {
            s.powf(factor)
        }
    }

    fn pick_object(&self, gi: usize, rng: &mut Rng) -> Option<usize> {
        let objs = &self.groups[gi].def.objects;
        if objs.is_empty() {
            return None;
        }
        let w = |e: &(String, f32)| if e.1 > 0.0 { e.1 } else { 1.0 };
        let total: f32 = objs.iter().map(w).sum();
        let mut r = rng.unit() * total - w(&objs[0]);
        let mut i = 0;
        while r >= 0.0 && i + 1 < objs.len() {
            i += 1;
            r -= w(&objs[i]);
        }
        self.resolve(&objs[i].0, rng)
    }

    /// StagePlaceNonRequiredObjects (0x595180).
    pub(super) fn place_groups(&mut self, world: &mut World, biome: &str, values: &dyn Fn(&GroupDef) -> f32, rng: &mut Rng) -> anyhow::Result<()> {
        for gi in 0..self.groups.len() {
            if self.groups[gi].def.objects.is_empty() {
                continue;
            }
            let def = &self.groups[gi].def;
            let scale = self.area_scale(world, def.slider.area_factor);
            let count = (values(def) * scale) as i32;
            let minimum = (def.objects_min as f32 * scale) as i32;
            let created = if def.balanced { self.balanced_group(world, gi, count, rng) } else { self.plain_group(world, gi, count, rng) };
            if created < minimum {
                anyhow::bail!("group {} placed {created} of at least {minimum}", self.groups[gi].ids);
            }
        }
        // Settlements: expand city layouts and paint their base terrain.
        self.expand_cities(world, biome, rng);
        Ok(())
    }

    /// PlaceGroup (0x594590).
    fn plain_group(&mut self, world: &mut World, gi: usize, count: i32, rng: &mut Rng) -> i32 {
        let mut created = 0;
        for _ in 0..count.max(0) {
            let Some(thing) = self.pick_object(gi, rng) else { break };
            let Some(pos) = self.find_position(world, rng, Some(gi), thing, 90, 1) else { continue };
            let facing = self.random_facing(thing, rng);
            let owner = self.groups[gi].kingdom;
            self.add(thing, pos, facing, owner, Some(gi), rng);
            created += 1;
        }
        created
    }

    /// BalancePlaceGroup (0x594E60): random valid markers, balanced apart
    /// W / (floor(sqrt(n)) + 1) and away from restricted groups, then created.
    fn balanced_group(&mut self, world: &mut World, gi: usize, count: i32, rng: &mut Rng) -> i32 {
        if count <= 0 {
            return 0;
        }
        let desired = world.width / ((count as f32).sqrt() as i32 + 1) as f32;
        let mut things = Vec::new();
        let mut pos = Vec::new();
        for _ in 0..count {
            let Some(thing) = self.pick_object(gi, rng) else { break };
            let margin = self.radius(thing) + world.tile;
            // 0x593290: random valid marker positions.
            for _ in 0..90 {
                let p = [(world.width - margin * 2.0) * rng.unit() + margin, (world.height - margin * 2.0) * rng.unit() + margin];
                if self.position_valid(world, thing, p) {
                    things.push(thing);
                    pos.push(p);
                    break;
                }
            }
        }
        if pos.is_empty() {
            return 0;
        }
        let restrictions = self.restrictions(gi);
        let margin = things.iter().map(|&t| self.radius(t)).fold(0.0, f32::max) + world.tile;
        let fixed: Vec<([f32; 2], f32)> = restrictions
            .iter()
            .flat_map(|&(oi, min, d)| {
                let v = min.max(d).max(desired);
                self.groups[oi].objects.iter().map(move |&o| (o, v))
            })
            .map(|(o, v)| (self.objects[o].pos, v))
            .collect();
        {
            let me = &*self;
            let world_ref = &*world;
            let things_ref = &things;
            let mut balanced = pos.clone();
            me.balance(
                world_ref,
                rng,
                &mut balanced,
                &|_, _| desired,
                margin,
                &|i, p| me.position_valid(world_ref, things_ref[i], p),
                &|_, p, force| fixed.iter().map(|&(q, v)| pair_penalty_fixed(p, q, v, force)).sum(),
            );
            pos = balanced;
        }
        let mut created = 0;
        for (i, &thing) in things.iter().enumerate() {
            // CreateMarkers, then CreateExtraMarkers for the failed ones.
            let p = if self.position_valid(world, thing, pos[i]) && self.context_valid(gi, pos[i]) {
                Some(pos[i])
            } else {
                self.find_position(world, rng, Some(gi), thing, 90, 1)
            };
            let Some(p) = p else { continue };
            let facing = self.random_facing(thing, rng);
            let owner = self.groups[gi].kingdom;
            self.add(thing, p, facing, owner, Some(gi), rng);
            created += 1;
        }
        created
    }

    /// Settlement centres create one of their CityComponent layouts; every
    /// building paints its [BaseTerrain] (biome variation, padding) and
    /// flattens the ground when flatten_terrain is set.
    fn expand_cities(&mut self, world: &mut World, biome: &str, rng: &mut Rng) {
        let data = self.data;
        let mut index = 0;
        while index < self.objects.len() {
            let o = self.objects[index].clone();
            index += 1;
            let def = &data.things[o.thing];
            if def.layouts.is_empty() {
                continue;
            }
            let layout = &def.layouts[rng.below(def.layouts.len() as u32) as usize];
            let mut members = vec![o.thing];
            let mut placed = vec![(o.pos, o.thing)];
            for (ids, offset, facing) in layout {
                let Some(t) = data.thing_by_id.get(&ids.to_ascii_lowercase()).copied() else {
                    self.notes.push(format!("city object {ids} unknown"));
                    continue;
                };
                let p = [o.pos[0] + offset[0], o.pos[1] + offset[1]];
                if p[0] < 0.0 || p[1] < 0.0 || p[0] >= world.width || p[1] >= world.height {
                    continue;
                }
                let l = (facing[0] * facing[0] + facing[1] * facing[1]).sqrt();
                let f = if l > 0.0 { [facing[0] / l, facing[1] / l] } else { DEFAULT_FACING };
                self.add(t, p, f, o.owner, o.group, rng);
                members.push(t);
                placed.push((p, t));
            }
            let biome_def = &data.biomes[&biome.to_ascii_lowercase()];
            let mut batch = Batch::new(data, biome_def);
            for &(p, t) in &placed {
                let d = &data.things[t];
                let var = d.base_terrain.get(&biome.to_ascii_lowercase()).or(d.base_default.as_ref());
                let half = d.half_extent;
                if let Some(v) = var.and_then(|v| data.variation_by_id.get(&v.to_ascii_lowercase()).copied()) {
                    let pad = d.base_padding.max(0.0);
                    let (hx, hy) = (half[0] + pad, half[1] + pad);
                    let t0 = |v: f32| ((v / world.tile).floor() as i32).max(0);
                    for ty in t0(p[1] - hy)..=t0(p[1] + hy).min(world.th as i32 - 1) {
                        for tx in t0(p[0] - hx)..=t0(p[0] + hx).min(world.tw as i32 - 1) {
                            let tile = ty as usize * world.tw + tx as usize;
                            if self.passable[tile] {
                                batch.paint(world, tile, v as u32);
                            }
                        }
                    }
                }
            }
            batch.commit(world);
            for &(p, t) in &placed {
                let d = &data.things[t];
                if d.flatten && d.half_extent[0] > 0.0 {
                    super::height::flatten(world, p, [d.half_extent[0] + 1.0, d.half_extent[1] + 1.0]);
                }
            }
            let _ = members;
        }
    }

    // ---------------- doodads ----------------

    /// PlaceAutoDoodads (0x54FB40) on every tile, then CleanupBlockingDoodads.
    pub(super) fn doodify(&mut self, world: &World, biome: &str, rng: &mut Rng) {
        let data = self.data;
        let Some(b) = data.biomes.get(&biome.to_ascii_lowercase()) else { return };
        let blockers: Vec<([f32; 2], f32)> = self
            .objects
            .iter()
            .filter(|o| data.things[o.thing].half_extent[0] > 0.0)
            .map(|o| (o.pos, self.radius(o.thing) + PADDING))
            .collect();
        let cell = 16.0f32;
        let (gw, gh) = ((world.width / cell) as usize + 1, (world.height / cell) as usize + 1);
        let mut grid: Vec<Vec<usize>> = vec![Vec::new(); gw * gh];
        for (i, &(p, r)) in blockers.iter().enumerate() {
            let (x0, x1) = (((p[0] - r) / cell).max(0.0) as usize, (((p[0] + r) / cell) as usize).min(gw - 1));
            let (y0, y1) = (((p[1] - r) / cell).max(0.0) as usize, (((p[1] + r) / cell) as usize).min(gh - 1));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    grid[y * gw + x].push(i);
                }
            }
        }
        let pick = |list: &[(usize, f32)], rng: &mut Rng| -> Option<usize> {
            if list.is_empty() {
                return None;
            }
            let total: f32 = list.iter().map(|e| e.1).sum();
            let mut r = f64::from(rng.unit()) * f64::from(total) - f64::from(list[0].1);
            let mut i = 0;
            while r >= 0.0 && i + 1 < list.len() {
                i += 1;
                r -= f64::from(list[i].1);
            }
            Some(list[i].0)
        };
        for t in 0..world.tw * world.th {
            let terrain = data.variations[world.var[t] as usize].terrain;
            let tdef = &data.terrains[terrain];
            let r = rng.unit();
            if !(r < tdef.doodad_frequency) {
                continue;
            }
            let c = world.tile_center(t);
            let half = world.tile * 0.5;
            let x = c[0] - half + rng.unit() * (world.tile - 1e-4);
            let y = c[1] - half + rng.unit() * (world.tile - 1e-4);
            let r2 = r / tdef.doodad_frequency;
            let Some(lists) = b.doodads.get(&terrain) else { continue };
            let mut thing = None;
            if r2 < tdef.primary_doodad_frequency {
                thing = pick(&lists.primary, rng);
            }
            if thing.is_none() {
                thing = pick(&lists.normal, rng);
            }
            let Some(thing) = thing else { continue };
            let def = &data.things[thing];
            let variant = if def.models.len() > 1 { rng.below(def.models.len() as u32) as i32 } else { -1 };
            let scale = if def.scale[1] > def.scale[0] { rng.range(def.scale) } else { def.scale[0].max(1e-3) };
            let facing = if def.random_orientation {
                let a = rng.unit() * std::f32::consts::TAU;
                [a.cos(), a.sin()]
            } else {
                DEFAULT_FACING
            };
            // CleanupBlockingDoodads: nothing on or around placed buildings.
            let (gx, gy) = (((x / cell) as usize).min(gw - 1), ((y / cell) as usize).min(gh - 1));
            if grid[gy * gw + gx].iter().any(|&i| dist(blockers[i].0, [x, y]) < blockers[i].1) {
                continue;
            }
            self.scenery.push(Object { thing, pos: [x, y], facing, scale, variant, owner: None, group: None });
        }
    }
}

/// 0x590E10: penalty for being closer than `desired`; adds the push-away force.
fn pair_penalty(rng: &mut Rng, p: [f32; 2], q: [f32; 2], desired: f32, force: &mut [f32; 2]) -> f32 {
    let (mut dx, mut dy) = (q[0] - p[0], q[1] - p[1]);
    let mut d = (dx * dx + dy * dy).sqrt();
    if d < 1e-4 {
        let a = rng.unit() * std::f32::consts::TAU;
        dx = a.cos();
        dy = a.sin();
        d = 0.5;
    } else {
        dx /= d;
        dy /= d;
    }
    let f = d - desired;
    if f > 0.0 || desired <= 0.0 {
        return 0.0;
    }
    let inv = 1.0 / desired;
    force[0] += inv * dx * f;
    force[1] += inv * dy * f;
    f.abs()
}
fn pair_penalty_fixed(p: [f32; 2], q: [f32; 2], desired: f32, force: &mut [f32; 2]) -> f32 {
    let (mut dx, mut dy) = (q[0] - p[0], q[1] - p[1]);
    let mut d = (dx * dx + dy * dy).sqrt();
    if d < 1e-4 {
        dx = 1.0;
        dy = 0.0;
        d = 0.5;
    } else {
        dx /= d;
        dy /= d;
    }
    let f = d - desired;
    if f > 0.0 || desired <= 0.0 {
        return 0.0;
    }
    let inv = 1.0 / desired;
    force[0] += inv * dx * f;
    force[1] += inv * dy * f;
    f.abs()
}

/// One balancer marker (0x5907B0; RMGBalanceMarker): +0 its index, +4/+8
/// position, +0xC/+0x10 position at the best score, +0x14/+0x18 orientation
/// (and its best at +0x1C/+0x20), +0x24 nudge step, +0x28/+0x2C force, +0x30
/// the kingdom of a kingdom marker.
#[derive(Clone, Debug)]
pub(super) struct Marker {
    pub id: usize,
    pub pos: [f32; 2],
    pub best: [f32; 2],
    pub orient: [f32; 2],
    pub best_orient: [f32; 2],
    pub step: f32,
    pub force: [f32; 2],
    /// The marker's kingdom (kingdom markers); kept for the group balancer.
    #[allow(dead_code)]
    pub kingdom: Option<usize>,
}

impl Marker {
    pub(super) fn new(id: usize, pos: [f32; 2], orient: [f32; 2], kingdom: Option<usize>) -> Self {
        Self { id, pos, best: [0.0, 0.0], orient, best_orient: [0.0, 0.0], step: 0.0, force: [0.0, 0.0], kingdom }
    }
}

/// The balancer's callback object (vtable +0 pair penalty, +4 validity, +8
/// margin, +0xC self score).
pub(super) trait Balance {
    fn pair(&self, rng: &mut Rng, a: &mut Marker, b: &Marker) -> f32;
    fn valid(&self, m: &Marker, cand: [f32; 2]) -> bool;
    fn margin(&self, m: &Marker) -> f64;
    fn self_score(&self, m: &mut Marker) -> f64;
}

/// The kingdom balancer (vtable 0xA4228C: 0x5954B0 / 0x595520 / 0x595530 /
/// 0x5956A0): +0x30 mode (0 team markers, 1 kingdom markers), +0x34 the
/// desired distance, +0x38 same-team and +0x3C other-team distances.
pub(super) struct KingdomBalance {
    pub mode: u32,
    pub desired: f32,
    pub same_team: f32,
    pub other: f32,
    /// tile + start location radius + 0.0001 (0x595530, on the register).
    pub margin: f64,
    /// Team of each kingdom marker (by marker id).
    pub teams: Vec<Option<u32>>,
}

impl Balance for KingdomBalance {
    fn pair(&self, rng: &mut Rng, a: &mut Marker, b: &Marker) -> f32 {
        let desired = if self.mode == 0 {
            self.desired
        } else {
            match (self.teams.get(a.id).copied().flatten(), self.teams.get(b.id).copied().flatten()) {
                (Some(x), Some(y)) if x == y => self.same_team,
                _ => self.other,
            }
        };
        pair_penalty_exact(rng, a, b.pos, desired)
    }
    fn valid(&self, _m: &Marker, _cand: [f32; 2]) -> bool {
        true
    }
    fn margin(&self, _m: &Marker) -> f64 {
        self.margin
    }
    fn self_score(&self, _m: &mut Marker) -> f64 {
        0.0
    }
}

/// 0x590E10: the penalty of `a` for being closer than `desired` to `q`,
/// adding the push-away force to `a` (on the register at PC53, with the
/// original's binary32 stores). Coincident markers take a random table
/// direction and a distance of 0.5 (one draw).
pub(super) fn pair_penalty_exact(rng: &mut Rng, a: &mut Marker, q: [f32; 2], desired: f32) -> f32 {
    let dx = f64::from(q[0]) - f64::from(a.pos[0]);
    let dy_d = f64::from(q[1]) - f64::from(a.pos[1]);
    let dy = dy_d as f32;
    let d = (dy_d * f64::from(dy) + dx * dx).sqrt() as f32;
    let (ux, uy, dd) = if d < f32::from_bits(0x38D1_B717) {
        let table = super::blob::direction_table();
        let t = table[(rng.at(0x590E66).next_u32() % table.len() as u32) as usize];
        (f64::from(t[0]), t[1], 0.5f64)
    } else {
        let inv = (1.0 / f64::from(d)) as f32;
        (dx * f64::from(inv), (f64::from(dy) * f64::from(inv)) as f32, f64::from(d))
    };
    let f = (dd - f64::from(desired)) as f32;
    if f > 0.0 {
        return 0.0;
    }
    let inv_des = 1.0 / f64::from(desired);
    let fx = (inv_des * (ux * f64::from(f))) as f32;
    let fy = inv_des * (f64::from(uy) * f64::from(f));
    a.force[0] = (f64::from(fx) + f64::from(a.force[0])) as f32;
    a.force[1] = (fy + f64::from(a.force[1])) as f32;
    f.abs()
}

/// 0x590820: a marker's score, the binary32 sum of its pair penalties
/// against every other marker (list order) plus its self score.
fn marker_score(rng: &mut Rng, markers: &mut [Marker], i: usize, cb: &dyn Balance) -> f64 {
    markers[i].force = [0.0, 0.0];
    let mut sum = 0.0f32;
    for j in 0..markers.len() {
        if markers[j].id == markers[i].id {
            continue;
        }
        let other = markers[j].clone();
        let p = cb.pair(rng, &mut markers[i], &other);
        sum = (f64::from(p) + f64::from(sum)) as f32;
    }
    cb.self_score(&mut markers[i]) + f64::from(sum)
}

/// KKC_RMGBalancer::BalanceMarkers (0x590B10): every iteration scores all
/// markers (the total on the register, logged "balance iteration %d score
/// %f"), keeps the positions of the best score, stops on a score below
/// 0.0001, after more than `max_worse` non-improving iterations or after
/// `iterations`; otherwise each marker moves `step` along its unit force,
/// clamped to [margin, size - margin - 0.0001], when its new place is valid
/// (else its step grows by `delta`), and its step then grows by `delta`
/// after an improving iteration or shrinks by it (down to `step0`) after a
/// non-improving one.
#[allow(clippy::too_many_arguments)]
pub(super) fn balance_markers(world: &World, rng: &mut Rng, markers: &mut [Marker], step0: f32, delta: f32, cb: &dyn Balance, iterations: i32, max_worse: i32, log: &mut Vec<(i32, f64)>) -> bool {
    let (mut last, mut best) = (f32::MAX, f32::MAX);
    let mut worse = 0i32;
    for m in markers.iter_mut() {
        m.step = step0;
    }
    if iterations < 0 {
        return true;
    }
    let eps = f32::from_bits(0x38D1_B717);
    let mut iter = 0i32;
    loop {
        let mut total = 0.0f64;
        for i in 0..markers.len() {
            total += marker_score(rng, markers, i, cb);
        }
        let score = if markers.is_empty() { 0.0 } else { total as f32 };
        log.push((iter, total));
        let mut got_worse = false;
        if score < best {
            best = score;
            for m in markers.iter_mut() {
                m.best = m.pos;
                m.best_orient = m.orient;
            }
            if worse > 0 {
                worse -= 1;
            }
            if score < eps {
                return true;
            }
        }
        if score >= last {
            worse += 1;
            if worse > max_worse {
                return true;
            }
            got_worse = true;
        }
        last = score;
        if iter == iterations {
            return true;
        }
        for i in 0..markers.len() {
            let m = &mut markers[i];
            let len = (f64::from(m.force[0]) * f64::from(m.force[0]) + f64::from(m.force[1]) * f64::from(m.force[1])).sqrt();
            if len > 0.0 {
                let inv = 1.0 / len;
                m.force[0] = (inv * f64::from(m.force[0])) as f32;
                m.force[1] = (inv * f64::from(m.force[1])) as f32;
            }
            let oy = (f64::from(m.force[1]) * f64::from(m.step)) as f32;
            let mut cand = [(f64::from(m.step) * f64::from(m.force[0]) + f64::from(m.pos[0])) as f32, (f64::from(oy) + f64::from(m.pos[1])) as f32];
            let margin = cb.margin(m);
            let lim_x = (f64::from(world.width) - margin) - f64::from(eps);
            if f64::from(cand[0]) < margin {
                cand[0] = margin as f32;
            } else if f64::from(cand[0]) > lim_x {
                cand[0] = lim_x as f32;
            }
            let lim_y = ((f64::from(world.height) - margin) - f64::from(eps)) as f32;
            if f64::from(cand[1]) < margin {
                cand[1] = margin as f32;
            } else if cand[1] > lim_y {
                cand[1] = lim_y;
            }
            let m = &markers[i];
            if cb.valid(m, cand) {
                markers[i].pos = cand;
            } else {
                markers[i].step = (f64::from(delta) + f64::from(markers[i].step)) as f32;
            }
            let m = &mut markers[i];
            if got_worse {
                if m.step > step0 {
                    m.step = (f64::from(m.step) - f64::from(delta)) as f32;
                }
            } else {
                m.step = (f64::from(delta) + f64::from(m.step)) as f32;
            }
        }
        iter += 1;
        if iter > iterations {
            return true;
        }
    }
}

fn blob_ftol(v: f64) -> i32 {
    super::blob::ftol_d(v)
}