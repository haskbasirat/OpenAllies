//! StagePlaceRequiredObjects (KKC_RMGPlaceRequiredObjectsStage::Run
//! 0x593F30) as the original runs it, with its callees:
//! - PlaceRequiredObject 0x591E60 (FindRandomPosition for a (0, 0) position,
//!   else CheckPositionValid and the distance_max test; the 0x590740 clear
//!   test and the up to three 0x591710 terrain conforms; CreateObject
//!   0x5910D0);
//! - FindRandomPosition 0x591A30 (90 candidates per round, 4 rounds with the
//!   0x5528F0 shrink when nothing was valid, the lowest score);
//! - CheckPositionValid 0x591350 (0x46EE00 snap, the 0x556070 ring search,
//!   the group context validity, the primary island test, 0x4F3640);
//! - the group context functor (vtable 0xA417F0: validity 0x595430 via
//!   0x594E00, score 0x595340 via 0x594D90, search circle 0x593EB0) and the
//!   group instance bounding box 0x593700;
//! - the CreateGThing / CreateGActor draws and PrepareMapFixStartLocations
//!   0x4FB640.
//!
//! Arithmetic follows the x87 code at PC53 (binary64 registers, binary32
//! stores where the original stores).
use super::super::rng::Rng;
use super::super::space::{self, FACING, P};
use super::{Balance, GroupDef, Marker, Object, Placement, World, balance_markers, pair_penalty_exact};
use aa_spatial::collision::Shape;
use aa_spatial::geometry::Placement as Where;

/// GetRNG unit on the register.
fn unit(rng: &mut Rng, site: u32) -> f64 {
    rng.at(site).unit_d()
}

fn bits(p: [f32; 2]) -> [u32; 2] {
    [p[0].to_bits(), p[1].to_bits()]
}

/// 0x5528F0: grow (or, negative, shrink) a shape by `amount`.
fn grow(shape: &Shape, amount: f32) -> Shape {
    let clamp = |v: f32| if v < 0.0 { 0.0 } else if v > f32::MAX { f32::MAX } else { v };
    match shape.kind {
        1 => {
            let r = (f64::from(amount) * 0.5 + f64::from(f32::from_bits(shape.radius_bits))) as f32;
            if r > 0.0 { Shape { kind: 1, radius_bits: r.to_bits(), bounds: [0; 4] } } else { Shape { kind: 0, radius_bits: 0, bounds: [0; 4] } }
        }
        2 => {
            let side = clamp((f64::from(f32::from_bits(shape.bounds[2])) * 2.0 + f64::from(amount)) as f32);
            aa_rules::objects::shape(2, &[side.to_bits()], P).map_or(shape.clone(), |s| space::shape(s.words))
        }
        3 => {
            let w = clamp((f64::from(f32::from_bits(shape.bounds[2])) * 2.0 + f64::from(amount)) as f32);
            let h = clamp((f64::from(f32::from_bits(shape.bounds[3])) * 2.0 + f64::from(amount)) as f32);
            aa_rules::objects::shape(3, &[w.to_bits(), h.to_bits()], P).map_or(shape.clone(), |s| space::shape(s.words))
        }
        _ => shape.clone(),
    }
}

/// The object balancer's callbacks (KKC_RMGObjectBalancer, vtable 0xA417CC).
struct ObjectBalance<'p, 'a> {
    place: &'p Placement<'a>,
    gi: usize,
    /// +0x2C: width / (trunc(sqrt(count)) + 1).
    desired: f32,
}

impl Balance for ObjectBalance<'_, '_> {
    /// 0x593170: 0x590E10 at the balancer's desired distance.
    fn pair(&self, rng: &mut Rng, a: &mut Marker, b: &Marker) -> f32 {
        pair_penalty_exact(rng, a, b.pos, self.desired)
    }
    /// 0x593190: CheckPositionValid without group context, snapping allowed.
    fn valid(&self, m: &Marker, cand: &mut [f32; 2]) -> bool {
        let before = *cand;
        let ok = self.place.check_position(None, m.thing, cand, false, m.orient, false);
        if std::env::var_os("AA_RMG_BALANCE_DEBUG").is_some() {
            let c = self.place.creation_check(m.thing, before, m.orient, false);
            eprintln!("  move marker {} from {:?} to {before:?}: valid {ok} -> {cand:?} (check {c})", m.id, m.pos);
        }
        ok
    }
    /// 0x593530: the tile size (0x5906B0), stored.
    fn margin(&self, _m: &Marker) -> f64 {
        f64::from(f32::from_bits(self.place.space.grid.cell_size_bits))
    }
    /// 0x5944A0: 0x590E10 against every object of every restricted group at
    /// max(max(proximity_min, proximity_desired), the balancer's distance).
    fn self_score(&self, rng: &mut Rng, m: &mut Marker) -> f64 {
        let mut sum = 0.0f32;
        for &(t, min, desired) in &self.place.groups[self.gi].restr {
            let v = if min > desired { min } else { desired };
            let d = if v > self.desired { v } else { self.desired };
            for &o in &self.place.groups[t].objects {
                let q = self.place.objects[o].pos;
                sum = (f64::from(pair_penalty_exact(rng, m, q, d)) + f64::from(sum)) as f32;
            }
        }
        f64::from(sum)
    }
}

/// Component slots whose InitCreate is 0x563FD0 (queue 1, `crate::creation`).
const QUEUE_ONE: [u32; 8] = [6, 7, 8, 11, 15, 22, 25, 29];
const ECONOMY: u32 = 9;
const ELEMENT: u32 = 8;
const MOVER: u32 = 3;
const HQ: u32 = 30;

impl Placement<'_> {
    /// 0x5909D0: virtual +0x2C ? the 0xB39F58+0x74 circle : virtual +0x28.
    fn find_shape(&self, thing: usize) -> Shape {
        space::soft_shape(&self.data.things[thing], self.data.svars.padding)
    }

    /// KKC_GWorld::CreateGThing 0x4F9C10 / CreateGActor 0x4FA030 draws, then
    /// the index insert: the GThing constructor's random facing 0x4EF40E,
    /// the variant 0x49B90D (asked as -1), the scale 0x49B458, the element
    /// mind jitter 0x530E65, the hitched trailer, the component InitCreates
    /// in slot order (queue 1 0x50B2EA, the HQ number) and the mover's
    /// countdown 0x50AB66 (as `crate::creation::created`).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn create_thing(&mut self, thing: usize, pos: [f32; 2], facing: [f32; 2], variant: i32, owner: Option<usize>, group: Option<usize>, rng: &mut Rng) -> usize {
        let def = &self.data.things[thing];
        let facing = if def.random_orientation {
            let table = super::super::blob::direction_table();
            table[(rng.at(0x4EF40E).next_u32() % table.len() as u32) as usize]
        } else {
            facing
        };
        let variant = if variant == -1 && def.models.len() > 1 { (rng.at(0x49B90D).next_u32() % def.models.len() as u32) as i32 } else { variant };
        let [lo, hi] = [f32::from_bits(def.scale_bits[0]), f32::from_bits(def.scale_bits[1])];
        let scale = (rng.at(0x49B458).unit_d() * (f64::from(hi) - f64::from(lo)) + f64::from(lo)) as f32;
        self.component_draws(thing, rng);
        let index = self.objects.len();
        self.objects.push(Object { thing, pos, facing, scale, variant, owner, group });
        let def = &self.data.things[thing];
        let blocking = Where { shape: space::shape(def.shape), facing: bits(facing), position: bits(pos) };
        let soft = Where { shape: space::shape(def.soft), facing: bits(facing), position: bits(pos) };
        self.space.add(def.flags, blocking, soft);
        index
    }

    fn component_draws(&self, thing: usize, rng: &mut Rng) {
        let def = &self.data.things[thing];
        let has = |bit: u32| def.components & (1 << bit) != 0;
        if has(ELEMENT) {
            rng.at(0x530E65).next_u32();
        }
        if let Some(t) = def.trailer.as_ref().and_then(|t| self.data.thing_by_id.get(&t.to_ascii_lowercase()).copied()).filter(|&t| t != thing) {
            let td = &self.data.things[t];
            if td.random_orientation {
                rng.at(0x4EF40E).next_u32();
            }
            rng.at(0x49B458).next_u32();
            self.component_draws(t, rng);
        }
        for bit in 0..31 {
            if !has(bit) {
                continue;
            }
            if QUEUE_ONE.contains(&bit) || (bit == ECONOMY && def.supply_required) {
                rng.at(0x50B2EA).next_u32();
            } else if bit == HQ {
                rng.at(0x4E2640).next_u32();
            }
        }
        if has(MOVER) {
            rng.at(0x50AB66).next_u32();
        }
    }

    /// 0x593700: append the object; the bounding box grows unless the point
    /// is inside it (0x53A7D0), then the centre and radius.
    fn group_add(&mut self, gi: usize, object: usize) {
        let p = self.objects[object].pos;
        let g = &mut self.groups[gi];
        g.objects.push(object);
        let b = &mut g.bbox;
        if g.objects.len() == 1 {
            *b = [p[0], p[1], p[0], p[1]];
        } else {
            if p[0] >= b[0] && p[0] <= b[2] && p[1] >= b[1] && p[1] <= b[3] {
                return;
            }
            if p[0] < b[0] {
                b[0] = p[0];
            }
            if p[1] < b[1] {
                b[1] = p[1];
            }
            if p[0] > b[2] {
                b[2] = p[0];
            }
            if p[1] > b[3] {
                b[3] = p[1];
            }
        }
        let wx = (f64::from(b[2]) - f64::from(b[0])) as f32;
        let hx = (f64::from(wx) * 0.5) as f32;
        let hy = ((f64::from(b[3]) - f64::from(b[1])) * 0.5) as f32;
        g.centre = [(f64::from(b[0]) + f64::from(hx)) as f32, (f64::from(hy) + f64::from(b[1])) as f32];
        g.radius = (f64::from(hy) * f64::from(hy) + f64::from(hx) * f64::from(hx)).sqrt() as f32;
    }

    /// 0x593EB0: the search circle (radius > 0): the relative kingdom's start
    /// location, else the relative group's centre once it has objects.
    fn search_circle_exact(&self, gi: usize) -> Option<([f32; 2], f32)> {
        let g = &self.groups[gi];
        let radius = g.def.search_radius;
        if radius <= 0.0 {
            return None;
        }
        if let Some(k) = g.search_kingdom {
            return self.kingdoms[k].major.then(|| (self.kingdoms[k].start, radius));
        }
        let rel = g.search_group?;
        if self.groups[rel].objects.is_empty() {
            return None;
        }
        Some((self.groups[rel].centre, radius))
    }

    /// 0x594E00: no object of group `target` closer than `min`.
    fn far_from(&self, target: usize, pos: [f32; 2], min: f32) -> bool {
        self.groups[target].objects.iter().all(|&o| {
            let q = self.objects[o].pos;
            let dx = f64::from(q[0]) - f64::from(pos[0]);
            let dy = f64::from(q[1]) - f64::from(pos[1]);
            !((dx * dx + dy * dy).sqrt() < f64::from(min))
        })
    }

    /// 0x595430: own group at intraproximity_min, then every restriction.
    fn valid_exact(&self, gi: usize, pos: [f32; 2]) -> bool {
        let g = &self.groups[gi];
        self.far_from(gi, pos, g.def.intra_min) && g.restr.iter().all(|&(t, min, _)| self.far_from(t, pos, min))
    }

    /// 0x594D90: sum of |desired - distance| over the group's objects.
    fn deviation(&self, target: usize, desired: f32, pos: [f32; 2]) -> f64 {
        let mut acc = 0.0f64;
        for &o in &self.groups[target].objects {
            let q = self.objects[o].pos;
            let dx = f64::from(q[0]) - f64::from(pos[0]);
            let dy = f64::from(q[1]) - f64::from(pos[1]);
            let mut v = f64::from(desired) - (dx * dx + dy * dy).sqrt();
            if v < 0.0 {
                v = -v;
            }
            acc += v;
        }
        acc
    }

    /// 0x595340: the start distance times starting_location_weight (a
    /// kingdom with a start location), the intraproximity deviation, the
    /// restriction deviations (desired != 0); binary32 after each term.
    fn score_exact(&self, gi: usize, pos: [f32; 2]) -> f32 {
        let g = &self.groups[gi];
        let mut score = 0.0f32;
        if let Some(k) = g.kingdom.filter(|&k| self.kingdoms[k].major) {
            if g.def.start_weight != 0.0 {
                let s = self.kingdoms[k].start;
                let dx = f64::from(s[0]) - f64::from(pos[0]);
                let dy = f64::from(s[1]) - f64::from(pos[1]);
                score = ((dy * dy + dx * dx).sqrt() * f64::from(g.def.start_weight)) as f32;
            }
        }
        if g.def.intra_desired != 0.0 {
            score = (self.deviation(gi, g.def.intra_desired, pos) + f64::from(score)) as f32;
        }
        for &(t, _, desired) in &g.restr {
            if desired != 0.0 {
                score = (self.deviation(t, desired, pos) + f64::from(score)) as f32;
            }
        }
        score
    }

    /// The 0x4F3640 creation test as the generator reaches it (no source,
    /// no other actor, arguments 5 = conform): 0 clear, 3 outside, 1 / 2 the
    /// first tile / actor hit. The settlement zone (0x4F1660) and zone
    /// overlap branches see an empty zone map during the required stage.
    fn creation_check(&self, thing: usize, pos: [f32; 2], facing: [f32; 2], conform: bool) -> u32 {
        let def = &self.data.things[thing];
        // GWorld+0x28 settings: not strict while generating.
        let strict = false;
        let shape = if strict { space::shape(def.shape) } else { self.find_shape(thing) };
        let mut position = bits(pos);
        if def.flags & 0x10000 != 0 && aa_spatial::snap_half_cell(&mut position, self.space.grid.cell_size_bits, P).is_err() {
            return 3;
        }
        let placement = Where { shape, facing: bits(facing), position };
        if aa_spatial::query::outside(&placement, self.space.extent_bits, P).unwrap_or(true) {
            return 3;
        }
        if def.flags & 0x40000 == 0 {
            let mut required = 0x400;
            if strict || def.flags & 0x400 == 0 {
                required |= 0x2000_0000;
            }
            let mask = if conform {
                0x400
            } else if def.flags & 0x20_0000 != 0 {
                0x600
            } else {
                0x500
            };
            match self.space.first_hit(&placement, mask, required, def.flags & 0x400 == 0) {
                0 => {}
                1 => return 1,
                3 => return 3,
                _ => return 2,
            }
        }
        0
    }

    /// 0x556070: the point itself, else rings out to `radius`.
    fn snap_point(&self, thing: usize, pos: &mut [f32; 2], facing: [f32; 2], radius: f32, conform: bool) -> bool {
        let def = &self.data.things[thing];
        let tile = f32::from_bits(self.space.grid.cell_size_bits);
        let r0 = f32::from_bits(def.shape[1]);
        let quarter = (f64::from(tile) * 0.25) as f32;
        let step = if r0 < quarter {
            f64::from(quarter)
        } else if r0 <= tile {
            f64::from(r0)
        } else {
            f64::from(r0) * 0.5
        };
        let rings = ((f64::from(radius) / step) as i32).clamp(1, 20);
        let ring_step = (f64::from(radius) / f64::from(rings)) as f32;
        if self.space.tile_at(*pos).is_some() && self.creation_check(thing, *pos, facing, conform) == 0 {
            return true;
        }
        let sin = f64::from(f32::from_bits(aa_spatial::placement::ROTATION_SIN_BITS));
        let cos = f64::from(f32::from_bits(aa_spatial::placement::ROTATION_COS_BITS));
        for k in 1..=rings {
            let d = f64::from(k) * f64::from(ring_step);
            let mut dx = (d * f64::from(facing[0])) as f32;
            let mut dy = (d * f64::from(facing[1])) as f32;
            for _ in 0..8 {
                let cand = [(f64::from(dx) + f64::from(pos[0])) as f32, (f64::from(dy) + f64::from(pos[1])) as f32];
                if self.space.tile_at(cand).is_some() && self.creation_check(thing, cand, facing, conform) == 0 {
                    *pos = cand;
                    return true;
                }
                let s = f64::from(sin as f32);
                let nx = (f64::from(dx) * cos - f64::from(dy) * s) as f32;
                let ny = (f64::from(dy) * cos + s * f64::from(dx)) as f32;
                dx = nx;
                dy = ny;
            }
        }
        false
    }

    fn snap_tile(&self, thing: usize, pos: &mut [f32; 2]) {
        if self.data.things[thing].flags & 0x10000 != 0 {
            let mut b = bits(*pos);
            if aa_spatial::snap_half_cell(&mut b, self.space.grid.cell_size_bits, P).is_ok() {
                *pos = [f32::from_bits(b[0]), f32::from_bits(b[1])];
            }
        }
    }

    /// CheckPositionValid 0x591350.
    fn check_position(&self, gi: Option<usize>, thing: usize, pos: &mut [f32; 2], keep: bool, facing: [f32; 2], conform: bool) -> bool {
        let def = &self.data.things[thing];
        let radius = 12.0f32;
        self.snap_tile(thing, pos);
        if !keep {
            if !self.snap_point(thing, pos, facing, radius, conform) {
                return false;
            }
            self.snap_tile(thing, pos);
        }
        if let Some(gi) = gi {
            if !self.valid_exact(gi, *pos) {
                return false;
            }
        }
        if def.flags & 0x20_0000 == 0 {
            if let Some(t) = self.space.tile_at(*pos) {
                if !self.space.on_primary_island(t) {
                    return false;
                }
            }
        }
        self.creation_check(thing, *pos, facing, conform) == 0
    }

    /// 0x590740 / the 591A30 conform test: 0x553090 with the shape at the
    /// point, 0x552210(0, 0) (actor flags 0x400, mask 0x500), no hit.
    fn clear_at(&self, shape: &Shape, pos: [f32; 2], facing: [f32; 2]) -> bool {
        let placement = Where { shape: shape.clone(), facing: bits(facing), position: bits(pos) };
        self.space.first_hit(&placement, 0x500, 0x400, false) == 0
    }

    /// FindRandomPosition 0x591A30.
    #[allow(clippy::too_many_arguments)]
    fn find_random_position(&self, rng: &mut Rng, gi: usize, thing: usize, conform: bool, tries: i32) -> Option<([f32; 2], [f32; 2])> {
        let mut local = self.find_shape(thing);
        let radius = f32::from_bits(local.radius_bits);
        let tile = f32::from_bits(self.space.grid.cell_size_bits);
        let [w, h] = self.space.extent_bits.map(f32::from_bits);
        let d = (f64::from(radius) + f64::from(tile)) as f32;
        let d2 = (f64::from(d) + f64::from(d)) as f32;
        let rounds = if conform { 4 } else { 1 };
        let mut best = f32::MAX;
        let mut best_pos = [0.0f32; 2];
        let mut facing = FACING.map(f32::from_bits);
        for _ in 0..rounds {
            for _ in 0..tries.max(0) {
                let mut cand = if let Some((c, r)) = self.search_circle_exact(gi) {
                    let u = unit(rng, 0x591B2A);
                    let mut x = ((u + u - 1.0) * f64::from(r) + f64::from(c[0])) as f32;
                    let u = unit(rng, 0x591B6A);
                    let mut y = ((u + u - 1.0) * f64::from(r) + f64::from(c[1])) as f32;
                    let (lx, ly) = (f64::from(w) - f64::from(d), f64::from(h) - f64::from(d));
                    if x < d {
                        x = d;
                    } else if f64::from(x) > lx {
                        x = lx as f32;
                    }
                    if y < d {
                        y = d;
                    } else if f64::from(y) > ly {
                        y = ly as f32;
                    }
                    [x, y]
                } else {
                    let u = unit(rng, 0x591D4E);
                    let x = ((f64::from(w) - f64::from(d2)) * u + f64::from(d)) as f32;
                    let u = unit(rng, 0x591D96);
                    let y = ((f64::from(h) - f64::from(d2)) * u + f64::from(d)) as f32;
                    [x, y]
                };
                facing = FACING.map(f32::from_bits);
                self.snap_tile(thing, &mut cand);
                if !self.check_position(Some(gi), thing, &mut cand, false, facing, conform) {
                    continue;
                }
                if conform && !self.clear_at(&local, cand, facing) {
                    continue;
                }
                let score = self.score_exact(gi, cand);
                if score < best {
                    best = score;
                    best_pos = cand;
                }
            }
            if rounds > 1 {
                if best < f32::MAX {
                    break;
                }
                let amount = (f64::from(radius) * -2.0 / f64::from(rounds)) as f32;
                local = grow(&local, amount);
            }
        }
        (best < f32::MAX).then_some((best_pos, facing))
    }

    /// 0x5931C0: a random-orientation thing faces a random normalized
    /// direction, anything else (0, -1).
    fn required_facing(&self, thing: usize, rng: &mut Rng) -> [f32; 2] {
        if self.data.things[thing].flags & 0x8000 == 0 {
            return FACING.map(f32::from_bits);
        }
        let u = unit(rng, 0x5931E7);
        let x = (u + u - 1.0) as f32;
        let u = unit(rng, 0x59321E);
        let y = (u + u - 1.0) as f32;
        // 0x44B600: scale by 1 / length above 1e-6, else (0, 0).
        let l = (f64::from(x) * f64::from(x) + f64::from(y) * f64::from(y)).sqrt();
        if l > f64::from(f32::from_bits(0x3586_37bd)) {
            let inv = 1.0 / l;
            [(inv * f64::from(x)) as f32, (inv * f64::from(y)) as f32]
        } else {
            [0.0, 0.0]
        }
    }

    /// KKC_RMGPlaceRequiredObjectsStage::Run 0x593F30: every group's required
    /// objects in group order.
    pub(in crate::mapgen) fn place_required_exact(&mut self, world: &mut World, rng: &mut Rng) -> anyhow::Result<()> {
        let _ = world;
        for gi in 0..self.groups.len() {
            let required = self.groups[gi].def.required.clone();
            for r in required {
                let Some(thing) = self.data.thing_by_id.get(&r.object.to_ascii_lowercase()).copied() else {
                    self.notes.push(format!("required object {} is not a known thing", r.object));
                    continue;
                };
                let owner = self.groups[gi].kingdom;
                let mut facing = self.required_facing(thing, rng);
                let pos = if r.position == [0.0, 0.0] {
                    let Some((p, f)) = self.find_random_position(rng, gi, thing, true, 90) else {
                        anyhow::bail!("PlaceRequiredObject({}): no valid position found for {}", r.object, self.groups[gi].ids);
                    };
                    facing = f;
                    p
                } else {
                    let mut p = r.position;
                    if !self.check_position(Some(gi), thing, &mut p, true, facing, true) {
                        anyhow::bail!("PlaceRequiredObject({}): position invalid", r.object);
                    }
                    let (dx, dy) = (f64::from(p[0]) - f64::from(r.position[0]), f64::from(p[1]) - f64::from(r.position[1]));
                    if (dx * dx + dy * dy).sqrt() > f64::from(r.distance_max) {
                        anyhow::bail!("PlaceRequiredObject({}): moved beyond distance_max", r.object);
                    }
                    p
                };
                // 0x590740 then up to three conforms 0x591710 of the grown shape.
                let shape = self.find_shape(thing);
                if !self.clear_at(&shape, pos, facing) {
                    self.notes.push(format!("required object {} at {pos:?} needs a terrain conform (not ported)", r.object));
                }
                let index = self.create_thing(thing, pos, facing, -1, owner, Some(gi), rng);
                self.fix_start(owner, index);
                self.group_add(gi, index);
            }
        }
        Ok(())
    }

    /// 0x593290: up to `tries` positions, uniform over the map (the original
    /// leaves out the margin offset) or, with a radius, around `pos` clamped
    /// to [m, size - m] with m = trunc(tile) (+ the settlement zone radius),
    /// the first that CheckPositionValid (no group context) accepts.
    #[allow(clippy::too_many_arguments)]
    fn random_valid(&self, rng: &mut Rng, thing: usize, pos: &mut [f32; 2], radius: f32, orient: &mut [f32; 2], tries: i32) -> bool {
        let tile = f32::from_bits(self.space.grid.cell_size_bits);
        let [w, h] = self.space.extent_bits.map(f32::from_bits);
        for _ in 0..tries.max(0) {
            let m = (tile as i32) as f32;
            let mut cand = if radius == 0.0 {
                let m2 = (f64::from(m) + f64::from(m)) as f32;
                let u = unit(rng, 0x593352);
                let x = (u * (f64::from(w) - f64::from(m2))) as f32;
                let u = unit(rng, 0x593396);
                let y = (u * (f64::from(h) - f64::from(m2))) as f32;
                [x, y]
            } else {
                let u = unit(rng, 0x5933CF);
                let mut x = ((u * f64::from(radius)) * 2.0 + (f64::from(pos[0]) - f64::from(radius))) as f32;
                let u = unit(rng, 0x59340D);
                let mut y = ((u * f64::from(radius)) * 2.0 + (f64::from(pos[1]) - f64::from(radius))) as f32;
                let (lx, ly) = (f64::from(w) - f64::from(m), f64::from(h) - f64::from(m));
                if x < m {
                    x = m;
                } else if f64::from(x) > lx {
                    x = lx as f32;
                }
                if y < m {
                    y = m;
                } else if f64::from(y) > ly {
                    y = ly as f32;
                }
                [x, y]
            };
            let o = *orient;
            if self.check_position(None, thing, &mut cand, false, o, false) {
                *pos = cand;
                *orient = o;
                return true;
            }
        }
        false
    }

    /// The group's area scale 0x590500: 1 for area_factor 0, else
    /// s = binary32(width / reference_size), s, s*s or pow(s, factor).
    fn area_scale_exact(&self, factor: f32) -> f64 {
        if factor == 0.0 {
            return 1.0;
        }
        let w = f32::from_bits(self.space.extent_bits[0]);
        let s = (f64::from(w) / f64::from(self.rmc.reference_size)) as f32;
        if factor == 1.0 {
            f64::from(s)
        } else if factor == 2.0 {
            f64::from(s) * f64::from(s)
        } else {
            f64::from(s).powf(f64::from(factor))
        }
    }

    /// KKC_RMGObjectBalancer::BalancePlaceGroup 0x594E60: PlaceMarkers
    /// 0x593AA0, BalanceMarkers 0x590B10 with the object callbacks (vtable
    /// 0xA417CC: pair 0x593170, validity 0x593190, margin 0x593530, self
    /// score 0x5944A0), CreateMarkers 0x594100, CreateExtraMarkers 0x594260.
    /// Returns (created, minimum).
    fn balanced_group_exact(&mut self, world: &World, gi: usize, value: f32, rng: &mut Rng, log: &mut Vec<(i32, f64)>) -> (i32, i32) {
        let def = self.groups[gi].def.clone();
        let owner = self.groups[gi].kingdom;
        let scale = self.area_scale_exact(def.slider.area_factor);
        let count = (scale * f64::from(value)) as i32;
        let minimum = (scale * f64::from(def.objects_min)) as i32;
        let [w, h] = self.space.extent_bits.map(f32::from_bits);
        let desired = (f64::from(w) / f64::from((f64::from(count)).sqrt() as i32 + 1)) as f32;
        self.notes.push(format!("RMG: Group '{}': trying to place {count} minimum {minimum} using balancing", self.groups[gi].ids));
        // PlaceMarkers 0x593AA0.
        let small = self.data.svars.balance_small >= count as f32;
        let centre = [(f64::from(w) * 0.5) as f32, (f64::from(h) * 0.5) as f32];
        let mut markers: Vec<Marker> = Vec::new();
        for i in 0..count.max(0) {
            rng.getter();
            let Some(thing) = self.pick_weighted_after_getter(gi, rng) else { continue };
            let mut orient = self.required_facing(thing, rng);
            let mut pos = if small { centre } else { [0.0, 0.0] };
            let radius = if small { (f64::from(self.data.svars.balance_search) * f64::from(w)) as f32 } else { 0.0 };
            if self.random_valid(rng, thing, &mut pos, radius, &mut orient, 10) {
                let mut m = Marker::new(markers.len(), pos, orient, owner);
                m.thing = thing;
                self.marker_log.push(("marker placed", i, pos));
                markers.push(m);
            } else {
                self.notes.push(format!("marker {i} tries 10 failed to find position"));
            }
        }
        if (markers.len() as i32) < minimum {
            return (0, minimum);
        }
        // BalanceMarkers 0x590B10.
        {
            let sv = &self.data.svars;
            let cb = ObjectBalance { place: self, gi, desired };
            balance_markers(world, rng, &mut markers, sv.nudge_step, sv.nudge_delta, &cb, sv.balance_iterations, sv.balance_worse, log);
        }
        // CreateMarkers 0x594100.
        let mut created = 0;
        for (i, m) in markers.iter().enumerate() {
            let mut pos = m.best;
            let orient = m.best_orient;
            if !self.check_position(Some(gi), m.thing, &mut pos, false, orient, false) {
                self.notes.push(format!("marker {i} failed to create at {},{}", pos[0], pos[1]));
                continue;
            }
            let index = self.create_thing(m.thing, pos, orient, -1, owner, Some(gi), rng);
            self.fix_start(owner, index);
            self.marker_log.push(("marker created", i as i32, pos));
            self.group_add(gi, index);
            created += 1;
        }
        // CreateExtraMarkers 0x594260 (the grid scan) is not ported yet.
        if created < count {
            self.notes.push(format!("RMG: Group '{}': {} extra markers needed (not ported)", self.groups[gi].ids, count - created));
        }
        (created, minimum)
    }

    /// 0x4B6FF0 on the group's weighted object table, after the caller's
    /// getter: 0x4B6BA0 pushes every object of positive weight at the head
    /// (the table runs last object first, the total a binary32 running sum).
    fn pick_weighted_after_getter(&self, gi: usize, rng: &mut Rng) -> Option<usize> {
        let mut table: Vec<(usize, f32)> = Vec::new();
        let mut total = 0.0f32;
        for (ids, w) in &self.groups[gi].def.objects {
            if !(*w > 0.0) {
                continue;
            }
            if let Some(&t) = self.data.thing_by_id.get(&ids.to_ascii_lowercase()) {
                table.insert(0, (t, *w));
                total = (f64::from(*w) + f64::from(total)) as f32;
            }
        }
        if table.is_empty() {
            return None;
        }
        let u = f64::from(rng.at(0x4B7008).draw_after_getter()) * super::super::rng::UNIT;
        let mut r = u * f64::from(total) - f64::from(table[0].1);
        let mut i = 0;
        while !(r < 0.0) && i + 1 < table.len() {
            i += 1;
            r -= f64::from(table[i].1);
        }
        Some(table[i].0)
    }

    /// KKC_RMGObjectPlacementStage::Run 0x595180: every actor group with
    /// objects (balanced or not), then every doodad group.
    pub(in crate::mapgen) fn place_groups_exact(&mut self, world: &mut World, values: &dyn Fn(&GroupDef) -> f32, rng: &mut Rng) -> anyhow::Result<()> {
        for gi in 0..self.groups.len() {
            if self.groups[gi].def.objects.is_empty() {
                continue;
            }
            let value = values(&self.groups[gi].def);
            if self.groups[gi].def.balanced {
                let mut log = Vec::new();
                let (created, minimum) = self.balanced_group_exact(world, gi, value, rng, &mut log);
                self.group_balance_log.extend(log);
                if created < minimum {
                    anyhow::bail!("RMG: Balanced Group '{}': failed minimum (created {created}, needed {minimum})", self.groups[gi].ids);
                }
            } else {
                let scale = self.area_scale(world, self.groups[gi].def.slider.area_factor);
                let count = (value * scale) as i32;
                let minimum = (self.groups[gi].def.objects_min as f32 * scale) as i32;
                let created = self.plain_group(world, gi, count, rng);
                if created < minimum {
                    anyhow::bail!("group {} placed {created} of at least {minimum}", self.groups[gi].ids);
                }
            }
        }
        Ok(())
    }

    /// 0x5910D0's PrepareMapFixStartLocations 0x4FB640 for an actor with a
    /// starting_location_priority: the kingdom's start location moves to its
    /// owned actor with the highest priority.
    fn fix_start(&mut self, owner: Option<usize>, index: usize) {
        let thing = self.objects[index].thing;
        let def = &self.data.things[thing];
        let Some(k) = owner else { return };
        if !(def.start_priority > 0 && self.kingdoms[k].major && def.start_priority > self.kingdoms[k].start_priority) {
            return;
        }
        let pos = self.objects[index].pos;
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
}
