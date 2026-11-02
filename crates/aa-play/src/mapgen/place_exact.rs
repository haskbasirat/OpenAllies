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
use super::{Balance, GroupDef, Marker, Object, Placement, ThingEvent, ThingWhat, World, balance_markers, pair_penalty_exact};
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
pub(super) fn grow(shape: &Shape, amount: f32) -> Shape {
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
        self.place.check_position(None, m.thing, cand, false, m.orient, false)
    }
    /// 0x593530: the tile size (0x5906B0), stored, and the zone radius for a
    /// thing that needs its space.
    fn margin(&self, m: &Marker) -> f64 {
        let tile = f64::from(f32::from_bits(self.place.space.grid.cell_size_bits));
        if self.place.data.things[m.thing].needs_space { tile + f64::from(self.place.data.settlement_radius) } else { tile }
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
const ORGANIZATION: u32 = 7;
const AIRCRAFT: u32 = 23;
const MOVER: u32 = 3;
const HQ: u32 = 30;

/// The parity probe (`AA_RMG_CG` set): every CreateGThing on stderr, as the
/// RmgProbe sampler logs the original's (M lines).
fn creation_probe() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("AA_RMG_CG").is_some())
}

impl Placement<'_> {
    /// 0x5909D0: virtual +0x2C ? the 0xB39F58+0x74 circle : virtual +0x28.
    fn find_shape(&self, thing: usize) -> Shape {
        let def = &self.data.things[thing];
        if def.needs_space {
            return Shape { kind: 1, radius_bits: self.data.settlement_radius.to_bits(), bounds: [0; 4] };
        }
        space::soft_shape(def, self.data.svars.padding)
    }

    /// 0x4F1660 for a thing that needs space for a settlement zone: a circle of
    /// the zone radius (0xB39F58+0x74) must be inside the world, on land, and
    /// free of blocking things that are not of its own definition. The zone
    /// query 0x54EF80 finds nothing: the generator has no zones yet. `arg8`
    /// is 1 from 0x4F3640: hostility does not count.
    fn zone_space_free(&self, thing: usize, pos: [f32; 2]) -> bool {
        let radius = self.data.settlement_radius;
        let circle = Where { shape: Shape { kind: 1, radius_bits: radius.to_bits(), bounds: [0; 4] }, facing: FACING, position: bits(pos) };
        if aa_spatial::query::outside(&circle, self.space.extent_bits, P).unwrap_or(true) {
            return false;
        }
        self.space.zone_circle_free(&circle, |member_flags, other| {
            let other_def = other.map(|i| &self.data.things[self.objects[i].thing]);
            let Some(other_def) = other_def else { return true };
            if other_def.ids == self.data.things[thing].ids || member_flags & (1 << 10) == 0 {
                return true;
            }
            // Bit 11, or an actor with a ... (0x4D1120), without +0x430.
            member_flags & (1 << 11) == 0
        }, &self.by_member)
    }

    /// 0x49B900 after the caller's getter: the aspect by weight.
    pub(super) fn pick_variant(&self, thing: usize, rng: &mut Rng) -> i32 {
        let def = &self.data.things[thing];
        let mut v = rng.at(0x49B90D).unit_d() * f64::from(def.aspect_total) - f64::from(def.aspect_weights.first().copied().unwrap_or(0.0));
        let mut i = 0;
        while !(v < 0.0) && i + 1 < def.aspect_weights.len() {
            i += 1;
            v -= f64::from(def.aspect_weights[i]);
        }
        i as i32
    }

    /// KKC_GWorld::CreateGThing 0x4F9C10: the constructor (0x4EF2A0: the
    /// random facing 0x4EF40E, or a facing that is not of unit length replaced
    /// by the default; 0x4EA5E0: the variant 0x49B90D asked as -1), InitCreate
    /// (the GTID, the scale 0x49B458, the components), InitExisting (0x4EFA40:
    /// the placement test, the index, the height; the doodad components again)
    /// and PostCreate (0x4EBA40 -> 0x4EB500, then an actor's 0x4CF300). A
    /// thing that fails its placement is deleted (0x4F5490) with what it drew.
    pub(super) fn create_gthing(&mut self, world: &mut World, rng: &mut Rng, c: super::Create) -> Option<usize> {
        let def = &self.data.things[c.thing];
        if creation_probe() {
            eprintln!("CG {} {:08x},{:08x}", def.ids, c.pos[0].to_bits(), c.pos[1].to_bits());
        }
        let facing = if def.flags & (1 << 17) != 0 {
            FACING.map(f32::from_bits)
        } else if def.flags & (1 << 15) != 0 {
            let table = super::super::blob::direction_table();
            table[(rng.at(0x4EF40E).next_u32() % table.len() as u32) as usize]
        } else {
            let length = ((f64::from(c.facing[1]) * f64::from(c.facing[1]) + f64::from(c.facing[0]) * f64::from(c.facing[0])).sqrt() as f32 - 1.0).abs();
            // [0xA2A5A0]: 0.0001.
            if length >= f32::from_bits(0x38D1_B717) { FACING.map(f32::from_bits) } else { c.facing }
        };
        let variant = if def.variations && c.variant == -1 { self.pick_variant(c.thing, rng) } else { c.variant };
        let id = match c.id {
            Some(id) => id,
            None => self.next_id(),
        };
        let [lo, hi] = [f32::from_bits(def.scale_bits[0]), f32::from_bits(def.scale_bits[1])];
        let scale = (rng.at(0x49B458).unit_d() * (f64::from(hi) - f64::from(lo)) + f64::from(lo)) as f32;
        let init_create = rng.state();
        for _ in 0..def.doodad_components {
            rng.at(0x5904C4).next_u32();
        }
        let mut trailer = None;
        let mut components = Vec::new();
        if def.native_kind == 2 || def.native_kind == 19 {
            trailer = self.actor_init_create(world, rng, &c, facing, &mut components);
        }
        let def = &self.data.things[c.thing];
        // 0x4EFA40.
        let mut pos = c.pos;
        self.snap_tile(c.thing, &mut pos);
        let check = self.creation_check_full(c.thing, pos, facing, false, c.strict, c.owner, None);
        if check != 0 {
            self.free_id(id);
            if let Some(t) = trailer {
                self.delete_object(t);
            }
            self.dead.push((c.thing, pos));
            return None;
        }
        let z = if def.flags & (1 << 21) != 0 {
            super::super::height::surface(world, self.data, pos, 0x123) as f32
        } else if def.flags & (1 << 12) != 0 {
            c.z
        } else {
            super::super::height::surface(world, self.data, pos, 0x122) as f32
        };
        let index = self.objects.len();
        let blocking = Where { shape: space::shape(def.shape), facing: bits(facing), position: bits(pos) };
        let soft = Where { shape: space::shape(def.soft), facing: bits(facing), position: bits(pos) };
        let member = self.space.add(def.flags, blocking, soft);
        self.objects.push(Object { thing: c.thing, pos, z, facing, scale, variant, owner: c.owner, group: c.group, id, member, alive: true, parent: c.parent, placeholder_of: c.replaces, trailer, auto: false });
        self.by_member.insert(member, index);
        if def.doodad_components != 0 {
            self.events.push(ThingEvent { state: init_create, what: ThingWhat::Doodad { thing: index } });
            self.events.push(ThingEvent { state: rng.state(), what: ThingWhat::Doodad { thing: index } });
        }
        for (state, slot) in components {
            self.events.push(ThingEvent { state, what: ThingWhat::Component { thing: index, slot } });
        }
        // Listed by the tile of its position (GMap tile +8, for deletion on a
        // terrain change).
        if let Some(t) = world.tile_of(pos) {
            self.tile_things[t].push(index as u32);
        }
        for _ in 0..def.doodad_components {
            rng.at(0x5904C4).next_u32();
        }
        if def.native_kind == 2 || def.native_kind == 19 {
            self.actor_init_existing(rng, c.thing);
            self.defender_init(world, rng, index);
        }
        // PostCreate 0x4EBA40 -> 0x4EB500 when the world settings allow it.
        self.post_create(world, rng, index);
        if self.is_actor(c.thing) {
            self.enter_zocs(rng, index);
        }
        Some(index)
    }

    /// GTID next-fit 0x4EFDB0 (slot 0 is never used; the cursor is the slot
    /// after the last one given).
    fn next_id(&mut self) -> u32 {
        loop {
            self.id_cursor += 1;
            let slot = self.id_cursor as usize;
            if slot >= self.id_used.len() {
                self.id_used.resize(slot + 1, false);
            }
            if !self.id_used[slot] {
                self.id_used[slot] = true;
                return slot as u32;
            }
        }
    }

    /// A requested GTID (the resolver's replacement keeps its source's).
    pub(super) fn reserve_id(&mut self, id: u32) {
        if self.id_used.len() <= id as usize {
            self.id_used.resize(id as usize + 1, false);
        }
        self.id_used[id as usize] = true;
        // It comes back under the generation it went with (0x4F0190).
        if let Some(g) = self.id_generation.get_mut(id as usize) {
            *g = g.wrapping_sub(1);
        }
    }

    fn free_id(&mut self, id: u32) {
        if let Some(u) = self.id_used.get_mut(id as usize) {
            *u = false;
        }
        if self.id_generation.len() <= id as usize {
            self.id_generation.resize(id as usize + 1, 0);
        }
        self.id_generation[id as usize] = self.id_generation[id as usize].wrapping_add(1);
    }

    /// 0x4F5490 on an object: out of the world.
    pub(super) fn delete_object(&mut self, index: usize) {
        let o = &mut self.objects[index];
        if !o.alive {
            return;
        }
        o.alive = false;
        let (member, id) = (o.member, o.id);
        self.space.remove(member);
        self.free_id(id);
    }

    /// The InitCreate draws of an actor's components (0x4D6F10, as
    /// `crate::creation::created`): the element mind jitter 0x530E65 (not for
    /// an organization or aircraft), the hitched trailer, the components in
    /// slot order (queue 1 0x50B2EA, the HQ number) and the mover's countdown.
    /// Returns the trailer.
    fn actor_init_create(&mut self, world: &mut World, rng: &mut Rng, c: &super::Create, facing: [f32; 2], components: &mut Vec<(u32, u8)>) -> Option<usize> {
        let thing = c.thing;
        let def = &self.data.things[thing];
        let has = |bit: u32| def.components & (1 << bit) != 0;
        if has(ELEMENT) && !has(ORGANIZATION) && !has(AIRCRAFT) {
            rng.at(0x530E65).next_u32();
        }
        // 0x4D6F8F -> AssignName 0x4F4A20: no getter, a draw % the list's size.
        if def.names > 0 {
            rng.at(0x4F4A82).draw_after_getter();
        }
        let trailer = def.trailer.as_ref().and_then(|t| self.data.thing_by_id.get(&t.to_ascii_lowercase()).copied()).filter(|&t| t != thing);
        let mut made = None;
        if let Some(t) = trailer {
            // 0x4D70F2: behind the actor by the hitch offset.
            let request = super::Create { thing: t, pos: self.trailer_position(thing, t, c.pos, facing), z: c.z, facing, owner: c.owner, parent: None, ..Default::default() };
            made = self.create_gthing(world, rng, request);
        }
        let def = &self.data.things[thing];
        let has = |bit: u32| def.components & (1 << bit) != 0;
        for bit in 0..31 {
            if !has(bit) {
                continue;
            }
            // A CityComponent only exists as a component with a zone radius
            // (0x45D6A0); without one the actor just lays out (0x4CF32F on).
            if bit == 29 && !(def.zone_radius > 0.0) {
                continue;
            }
            if QUEUE_ONE.contains(&bit) || (bit == ECONOMY && def.supply_required) {
                components.push((rng.state(), bit as u8));
                rng.at(0x50B2EA).next_u32();
            } else if bit == HQ {
                rng.at(0x4E2640).next_u32();
            }
        }
        made
    }

    /// The actor's InitExisting draws (0x4D7230): the mover's countdown
    /// 0x50AB66 (0x4D7520 -> 0x4D7450 when a TrackInfo is CONTINUOUS).
    fn actor_init_existing(&self, rng: &mut Rng, thing: usize) {
        let def = &self.data.things[thing];
        if def.components & (1 << MOVER) != 0 && def.continuous_tracks {
            rng.at(0x50AB66).next_u32();
        }
    }

    /// 0x593700: append the object; the bounding box grows unless the point
    /// is inside it (0x53A7D0), then the centre and radius.
    pub(super) fn group_add(&mut self, gi: usize, object: usize) {
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
        self.creation_check_full(thing, pos, facing, conform, false, None, None)
    }

    /// 0x4F3640 through 0x4F4590 / 0x591350: `source` is the thing's own
    /// record while it is created: its flags still carry the constructor's
    /// bit 31, so the placement is not strict for it either.
    fn creation_check_full(&self, thing: usize, pos: [f32; 2], facing: [f32; 2], conform: bool, strict: bool, _owner: Option<usize>, skip: Option<u32>) -> u32 {
        let def = &self.data.things[thing];
        let shape = if strict { space::shape(def.shape) } else { self.find_shape(thing) };
        let mut position = bits(pos);
        if def.flags & 0x10000 != 0 && aa_spatial::snap_half_cell(&mut position, self.space.grid.cell_size_bits, P).is_err() {
            return 3;
        }
        let placement = Where { shape, facing: bits(facing), position };
        if aa_spatial::query::outside(&placement, self.space.extent_bits, P).unwrap_or(true) {
            return 3;
        }
        if !strict && def.needs_space && !self.zone_space_free(thing, pos) {
            return 5;
        }
        // A thing that does not block is not tested against others (+0x52).
        if def.flags & 0x40000 == 0 && def.flags & 0x400 != 0 {
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
            match self.space.first_hit_skipping(&placement, mask, required, def.flags & 0x400 == 0, skip) {
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
    pub(super) fn check_position(&self, gi: Option<usize>, thing: usize, pos: &mut [f32; 2], keep: bool, facing: [f32; 2], conform: bool) -> bool {
        let def = &self.data.things[thing];
        // 0x591350: 12, or the zone radius for a thing that needs its space.
        let radius = if def.needs_space { self.data.settlement_radius } else { 12.0f32 };
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
    pub(super) fn find_random_position(&self, rng: &mut Rng, gi: usize, thing: usize, conform: bool, tries: i32) -> Option<([f32; 2], [f32; 2])> {
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
    pub(super) fn required_facing(&self, thing: usize, rng: &mut Rng) -> [f32; 2] {
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
        for gi in 0..self.groups.len() {
            let required = self.groups[gi].def.required.clone();
            for r in required {
                let Some(thing) = self.data.thing_by_id.get(&r.object.to_ascii_lowercase()).copied() else {
                    self.notes.push(format!("required object {} is not a known thing", r.object));
                    continue;
                };
                let owner = self.groups[gi].kingdom;
                let facing = self.required_facing(thing, rng);
                let mut facing = facing;
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
                // 0x590740 then up to three conforms 0x591710 of the shape grown
                // by 4, to terrain of the kind it needs (flag bit 21: water).
                let shape = self.find_shape(thing);
                let kind = if self.data.things[thing].flags & (1 << 21) != 0 { 2 } else { 0 };
                let grown = grow(&shape, 4.0);
                let mut clear = false;
                for _ in 0..3 {
                    clear = self.clear_at(&shape, pos, facing);
                    if clear {
                        break;
                    }
                    self.conform_terrain(world, &grown, facing, pos, kind);
                }
                if !clear && !self.clear_at(&shape, pos, facing) {
                    anyhow::bail!("PlaceRequiredObject({}): Could not make terrain conforming", r.object);
                }
                let request = super::Create { thing, pos, facing, owner, group: Some(gi), ..Default::default() };
                let Some(index) = self.create_gthing(world, rng, request) else {
                    anyhow::bail!("PlaceRequiredObject({}): creation failed at {pos:?}", r.object);
                };
                self.fix_start(world, owner, index);
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
            let mut m = (tile as i32) as f32;
            if self.data.things[thing].needs_space {
                m = (f64::from(m) + f64::from(self.data.settlement_radius)) as f32;
            }
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
    pub(super) fn area_scale_exact(&self, factor: f32) -> f64 {
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
    fn balanced_group_exact(&mut self, world: &mut World, gi: usize, value: f32, rng: &mut Rng, log: &mut Vec<(i32, f64)>) -> (i32, i32) {
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
            let request = super::Create { thing: m.thing, pos, facing: orient, owner, group: Some(gi), ..Default::default() };
            let Some(index) = self.create_gthing(world, rng, request) else { continue };
            self.fix_start(world, owner, index);
            self.marker_log.push(("marker created", i as i32, pos));
            self.group_add(gi, index);
            created += 1;
        }
        // CreateExtraMarkers 0x594260.
        if created < count {
            created += self.extra_markers(world, gi, count - created, rng);
        }
        (created, minimum)
    }

    /// 0x4B6FF0 on the group's weighted object table, after the caller's
    /// getter: 0x4B6BA0 pushes every object of positive weight at the head
    /// (the table runs last object first, the total a binary32 running sum).
    pub(super) fn pick_weighted_after_getter(&self, gi: usize, rng: &mut Rng) -> Option<usize> {
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
    pub(in crate::mapgen) fn place_groups_exact(&mut self, world: &mut World, values: &dyn Fn(&str, &GroupDef) -> f32, rng: &mut Rng) -> anyhow::Result<()> {
        for gi in 0..self.groups.len() {
            if self.groups[gi].def.objects.is_empty() {
                continue;
            }
            let value = values(&self.groups[gi].ids, &self.groups[gi].def);
            if self.groups[gi].def.balanced {
                let mut log = Vec::new();
                let (created, minimum) = self.balanced_group_exact(world, gi, value, rng, &mut log);
                self.group_balance_log.extend(log);
                if created < minimum {
                    anyhow::bail!("RMG: Balanced Group '{}': failed minimum (created {created}, needed {minimum})", self.groups[gi].ids);
                }
            } else {
                let (created, minimum) = self.plain_group_exact(world, gi, value, rng);
                if created < minimum {
                    anyhow::bail!("RMG: Group '{}': failed to meet minimum (created {created}, needed {minimum})", self.groups[gi].ids);
                }
            }
        }
        Ok(())
    }

    /// 0x5910D0's PrepareMapFixStartLocations 0x4FB640 for an actor with a
    /// starting_location_priority: the kingdom's start location moves to its
    /// owned actor with the highest priority.
    pub(super) fn fix_start(&mut self, world: &World, owner: Option<usize>, index: usize) {
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
            for i in 0..self.objects.len() {
                let o = &mut self.objects[i];
                if o.owner == Some(k) && o.thing == start_thing {
                    let old = o.pos;
                    o.pos = pos;
                    // 0x4EEB40: the height is queried at the position it leaves.
                    let flags = self.data.things[start_thing].flags;
                    if flags & (1 << 21) != 0 {
                        o.z = super::super::height::surface(world, self.data, old, 0x123) as f32;
                    } else if flags & (1 << 12) == 0 {
                        o.z = super::super::height::surface(world, self.data, old, 0x122) as f32;
                    }
                    // Listed by the tile of its new position.
                    if let Some(t) = self.space.tile_at(old) {
                        self.tile_things[t].retain(|&x| x != i as u32);
                    }
                    if let Some(t) = self.space.tile_at(pos) {
                        self.tile_things[t].push(i as u32);
                    }
                }
            }
        }
    }
}
