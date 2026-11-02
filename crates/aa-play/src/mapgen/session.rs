//! What creating a thing does to the world, and the session start of a
//! generated world, as the original runs them:
//! - GThing::PostCreate 0x4EBA40 -> 0x4EB500: the flattened ground under a
//!   blocking static thing (0x551140 on the base map), ClearDoodads 0x4F5560,
//!   the base terrain paint (0x4EA260, 0x5528F0, PaintTerrain 0x555C10);
//! - GActor::PostCreate 0x4CF300: the city layout 0x5644F0;
//! - the trailer position of GActor::InitCreate 0x4D6F10 (0x4D70F2);
//! - ResolvePlaceholdersAndRandomThings 0x4FDA70 (the placeholder resolver
//!   0x4FC4A0 / 0x4FBA50, 0x473370 the nation's substitution).
//!
//! Arithmetic follows the x87 code at PC53 (binary64 registers, binary32
//! stores where the original stores).
use super::super::grid::Batch;
use super::super::height::{self, Brush, Map};
use super::super::rng::Rng;
use super::super::space::{self, P};
use super::doodads::weighted_pick;
use super::{Create, Placement, World};
use aa_spatial::collision::Shape;
use aa_spatial::geometry::Placement as Where;

fn bits(p: [f32; 2]) -> [u32; 2] {
    [p[0].to_bits(), p[1].to_bits()]
}

/// A Defender's zone of control (zone type 4, radius ActorData +0x218): the
/// tiles of the ZoneSolver's flood 0x5402E0, and whether it is on queue 0.
pub(super) struct Zoc {
    defender: usize,
    tiles: std::collections::HashSet<usize>,
    registered: bool,
}

/// One replacement the resolver queued (0x4FD7F0 / 0x4FD8C0).
struct Replacement {
    source: usize,
    thing: usize,
}

impl Placement<'_> {
    /// The placement 0x4CF210 / 0x554350 give of a thing: its blocking shape
    /// (virtual +0xC) at its position and facing.
    pub(super) fn blocking_placement(&self, index: usize) -> Where {
        let o = &self.objects[index];
        Where { shape: space::shape(self.data.things[o.thing].shape), facing: bits(o.facing), position: bits(o.pos) }
    }

    /// 0x4D70F2: the trailer of `thing` facing `facing` at `pos`: behind it by
    /// the two definitions' TrailerHitch node_offset (+0x270 / +0x274).
    pub(super) fn trailer_position(&self, thing: usize, trailer: usize, pos: [f32; 2], facing: [f32; 2]) -> [f32; 2] {
        let (a, b) = (self.data.things[thing].hitch, self.data.things[trailer].hitch);
        let dy = (f64::from(a[1]) - f64::from(b[1])) as f32;
        let dx = (f64::from(a[0]) - f64::from(b[0])) as f32;
        let (dx, dy) = (f64::from(dx), f64::from(dy));
        let (fx, fy) = (f64::from(facing[0]), f64::from(facing[1]));
        let ox = (dx * fy + dy * fx) as f32;
        let oy = (dy * fy - dx * fx) as f32;
        [(f64::from(ox) + f64::from(pos[0])) as f32, (f64::from(oy) + f64::from(pos[1])) as f32]
    }

    /// The flood of a zone at `pos` (0x5402E0 with the functor 0xA36910): a
    /// tile takes the zone at a cost within the radius, no step changes the land
    /// passability (tile flag 0x100), a tile with bit 0 stops it; it expands
    /// while half a tile plus the cost is below the radius.
    fn zone_tiles(&self, world: &World, pos: [f32; 2], radius: f32) -> std::collections::HashSet<usize> {
        let mut out = std::collections::HashSet::new();
        let (w, h) = (world.tw as i32, world.th as i32);
        let start = [((pos[0] as i32) >> world.tile_bits).clamp(0, w - 1), ((pos[1] as i32) >> world.tile_bits).clamp(0, h - 1)];
        let at = |c: [i32; 2]| (c[1] * w + c[0]) as usize;
        if self.space.flags[at(start)] & 1 != 0 {
            return out;
        }
        out.insert(at(start));
        let sqrt2 = f64::from(f32::from_bits(0x3FB5_04F3));
        let half = f64::from(world.tile) * 0.5;
        let eps = f64::from(f32::from_bits(0x38D1_B717));
        let mut cost: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();
        cost.insert(at(start), 0.0);
        let mut open: Vec<([i32; 2], f32)> = vec![(start, 0.0)];
        const DIRECTIONS: [[i32; 2]; 8] = [[-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];
        while !open.is_empty() {
            let best = (0..open.len()).min_by(|&a, &b| open[a].1.total_cmp(&open[b].1)).unwrap_or(0);
            let (cur, c) = open.swap_remove(best);
            if cost.get(&at(cur)).is_some_and(|&k| k < c) {
                continue;
            }
            for d in DIRECTIONS {
                let n = [cur[0] + d[0], cur[1] + d[1]];
                if n[0] < 0 || n[1] < 0 || n[0] >= w || n[1] >= h || self.space.flags[at(n)] & 1 != 0 {
                    continue;
                }
                let diagonal = d[0] != 0 && d[1] != 0;
                if diagonal && (self.space.flags[at([cur[0] + d[0], cur[1]])] & 1 != 0 || self.space.flags[at([cur[0], cur[1] + d[1]])] & 1 != 0) {
                    continue;
                }
                let step = if diagonal { sqrt2 } else { 1.0 } * f64::from(world.tile);
                let next = (step + f64::from(c)) as f32;
                if next > radius {
                    continue;
                }
                if let Some(&old) = cost.get(&at(n)) {
                    if !(f64::from(next) + eps < f64::from(old)) {
                        continue;
                    }
                }
                if (self.space.flags[at(n)] ^ self.space.flags[at(cur)]) & 0x100 != 0 {
                    continue;
                }
                cost.insert(at(n), next);
                out.insert(at(n));
                if half + f64::from(next) < f64::from(radius) {
                    open.push((n, next));
                }
            }
        }
        out
    }

    /// DefenderComponent InitExisting 0x566DB0: the zone of control; the actors
    /// on the tiles it takes enter it (0x54F7F0).
    pub(super) fn defender_init(&mut self, world: &World, rng: &mut Rng, index: usize) {
        let o = &self.objects[index];
        let Some(radius) = self.data.things[o.thing].control_range else { return };
        if self.game.is_none() {
            return;
        }
        let tiles = self.zone_tiles(world, o.pos, radius);
        self.zocs.push(Zoc { defender: index, tiles, registered: false });
        let zoc = self.zocs.len() - 1;
        let mut order: Vec<usize> = (0..self.objects.len()).filter(|&i| i != index && self.objects[i].alive && self.is_actor(self.objects[i].thing)).collect();
        order.sort_by_key(|&i| self.objects[i].id);
        for a in order {
            if self.on_zone_tile(zoc, a) {
                self.zoc_entered(rng, zoc, a);
            }
        }
    }

    /// 0x54F7F0's test for a tile of the zone: a thing that marks tiles (0x4EE700)
    /// is listed on every tile of its soft shape's rectangle, any other counts on
    /// the tile it stands on (0x4EA200).
    fn on_zone_tile(&self, zoc: usize, a: usize) -> bool {
        let o = &self.objects[a];
        let def = &self.data.things[o.thing];
        let zone = &self.zocs[zoc].tiles;
        if !aa_spatial::geometry::marks_blocker(def.flags, def.shape[1], self.space.grid.cell_size_bits, P).unwrap_or(false) {
            return self.space.tile_at(o.pos).is_some_and(|t| zone.contains(&t));
        }
        let placed = Where { shape: space::shape(def.soft), facing: bits(o.facing), position: bits(o.pos) };
        let Ok(rect) = aa_spatial::geometry::tile_rectangle(&self.space.grid, &placed, P) else { return false };
        let w = self.space.grid.dimensions[0];
        (rect.min[1]..=rect.max[1]).any(|y| (rect.min[0]..=rect.max[0]).any(|x| zone.contains(&((y * w + x) as usize))))
    }

    /// 0x4CFEA0: the tiles of 0x554CC0 over the thing's blocking shape enter the
    /// zones on them.
    pub(super) fn enter_zocs(&mut self, rng: &mut Rng, index: usize) {
        if self.zocs.is_empty() {
            return;
        }
        let tiles = self.standing_tiles(index);
        for zoc in 0..self.zocs.len() {
            if tiles.iter().any(|t| self.zocs[zoc].tiles.contains(t)) {
                self.zoc_entered(rng, zoc, index);
            }
        }
    }

    /// 0x554CC0: a thing that marks tiles (0x4EE700) stands on the rectangle of
    /// its blocking shape (0x5534F0): the tile under its position and every tile
    /// whose centre is in the shape (0x5541F0, the point shape 0xB3A8C4); any other
    /// on the tile under its position.
    fn standing_tiles(&self, index: usize) -> Vec<usize> {
        let o = &self.objects[index];
        let def = &self.data.things[o.thing];
        let grid = &self.space.grid;
        let centre = self.space.tile_at(o.pos);
        if !aa_spatial::geometry::marks_blocker(def.flags, def.shape[1], grid.cell_size_bits, P).unwrap_or(false) {
            return centre.into_iter().collect();
        }
        let placed = self.blocking_placement(index);
        let Ok(rect) = aa_spatial::geometry::tile_rectangle(grid, &placed, P) else { return Vec::new() };
        let w = grid.dimensions[0];
        let tile = f64::from(f32::from_bits(grid.cell_size_bits));
        let point = Shape { kind: 0, radius_bits: 0, bounds: [0; 4] };
        let mut out = Vec::new();
        for y in rect.min[1]..=rect.max[1] {
            for x in rect.min[0]..=rect.max[0] {
                let t = (y * w + x) as usize;
                let cx = (f64::from(x << grid.coordinate_shift) + tile * 0.5) as f32;
                let cy = (f64::from(y << grid.coordinate_shift) + tile * 0.5) as f32;
                let probe = Where { shape: point.clone(), facing: space::FACING, position: bits([cx, cy]) };
                if Some(t) == centre || aa_spatial::shape_overlap::overlaps(&probe, &placed, P).unwrap_or(false) {
                    out.push(t);
                }
            }
        }
        out
    }

    /// 0x514DA0 -> 0x514E40 -> 0x567840 for an actor entering a zone of control:
    /// alive and targetable (flag bit 14), an enemy of the zone's owner, an ability
    /// of the Defender that may take it (0x5673B0): the first such actor puts the
    /// Defender on queue 0 (0x50AC00, a draw).
    fn zoc_entered(&mut self, rng: &mut Rng, zoc: usize, a: usize) {
        let d = self.zocs[zoc].defender;
        if d == a || !self.objects[d].alive || !self.objects[a].alive {
            return;
        }
        let (a_thing, a_owner) = (self.objects[a].thing, self.objects[a].owner);
        // A random village waiting for its replacement never entered a zone in
        // the recordings (films d9eb, 0cca).
        if self.data.things[a_thing].native_kind == 19 {
            return;
        }
        let d_owner = self.objects[d].owner;
        if self.data.things[a_thing].flags & (1 << 14) == 0 {
            return;
        }
        // Virtual +0x140 on the owners: no friend (a kingdom of the same team).
        let friend = match (a_owner, d_owner) {
            (Some(x), Some(y)) => x == y || (self.kingdoms[x].major && self.kingdoms[y].major && self.kingdoms[x].team.is_some() && self.kingdoms[x].team == self.kingdoms[y].team),
            _ => true,
        };
        if friend || !self.defender_may_target(d, a) {
            return;
        }
        self.events.push(super::ThingEvent { state: rng.state(), what: super::ThingWhat::Defender { defender: d, intruder: a } });
        if !self.zocs[zoc].registered {
            self.zocs[zoc].registered = true;
            rng.at(0x50B2EA).next_u32();
        }
    }

    /// 0x5673B0 over the Defender's abilities (GameData's `Weapon`s): the target
    /// filter (0x4C8740, 0x4C5A20), the minimum range to the target's position
    /// (0x4C8460: 0x4EE7C0 on the Defender's range shape, inverted) and, for a
    /// definition with a projectile (+0x3C), the range 0x4BE910 centre to centre.
    /// A turret's arc (0x4C84B0) is not tested: no Defender of the data has a
    /// limited one in the corpus's maps. The range's bonuses (0x501D50, 0x501DB0)
    /// are the owner's: nothing has given any yet.
    fn defender_may_target(&self, d: usize, a: usize) -> bool {
        let Some(game) = self.game else { return false };
        let (dobj, aobj) = (&self.objects[d], &self.objects[a]);
        let (d_def, a_def) = (&self.data.things[dobj.thing], &self.data.things[aobj.thing]);
        let (Some(g_d), Some(g_a)) = (game.find(&d_def.ids), game.find(&a_def.ids)) else { return false };
        let bit = |name: &str| game.target_types.iter().position(|t| t.eq_ignore_ascii_case(name)).map_or(0u32, |b| 1u32 << b);
        let mut flags = game.def(g_a).target_flags | bit("enemy") | bit("body") | bit("actor");
        if aobj.owner.is_none_or(|k| !self.kingdoms[k].major) {
            flags |= bit("independent");
        }
        let range_shape = Where { shape: space::shape(d_def.soft), facing: bits(dobj.facing), position: bits(dobj.pos) };
        let dx = ((f64::from(aobj.pos[0]) - f64::from(dobj.pos[0])) as f32) as f64;
        let dy = ((f64::from(aobj.pos[1]) - f64::from(dobj.pos[1])) as f32) as f64;
        let distance2 = dy * dy + dx * dx;
        game.def(g_d).weapons.iter().filter(|w| w.defender).any(|w| {
            if flags & w.targets_required != w.targets_required || flags & w.targets_forbidden != 0 {
                return false;
            }
            if !(w.min_range < f32::from_bits(0x38D1_B717)) && !aa_spatial::range::shape_point_in_range(&range_shape, bits(aobj.pos), w.min_range.to_bits(), true, P).unwrap_or(false) {
                return false;
            }
            w.projectile.is_none() || distance2 <= f64::from(w.range) * f64::from(w.range)
        })
    }

    /// 0x4EB500 and, for an actor, the rest of 0x4CF300.
    pub(super) fn post_create(&mut self, world: &mut World, rng: &mut Rng, index: usize) {
        let (thing, pos, z, parent) = {
            let o = &self.objects[index];
            (o.thing, o.pos, o.z, o.parent)
        };
        let def = &self.data.things[thing];
        let radius_bits = def.shape[1];
        // 0x4EE700 and flag bit 13: the ground under it is flattened, at its
        // own ground height or, for a city's object, at the city's height.
        let marks = aa_spatial::geometry::marks_blocker(def.flags, radius_bits, world.tile.to_bits(), P).unwrap_or(false);
        if marks && def.flags & (1 << 13) != 0 {
            let level = match parent {
                Some(p) => self.objects[p].z,
                None => height::surface(world, self.data, pos, 0x122) as f32,
            };
            let slope = self.data.svars.elevation_slope_max;
            // The falloff word 0x4EB500 leaves unset holds the thing's own Z
            // from InitExisting (recorded: every native flatten brush).
            let brush = Brush { mode: 0, height: level, slope, min: f32::MIN, max: f32::MAX, protect: true, smooth_edge: true, strength: 1.0, falloff: z, area: 0 };
            let placement = self.blocking_placement(index);
            height::brush_shape(world, self.data, Map::Base, &placement, &brush);
        }
        if def.flags & (1 << 29) != 0 && def.flags & (1 << 10) != 0 {
            let placement = self.blocking_placement(index);
            self.clear_doodads(&placement);
        }
        if !self.started {
            self.paint_base_terrain(world, index);
        }
        // 0x4CF300: a city lays out from its CityComponent's PostCreate (a zone
        // radius above 0, 0x564BFD) or right after the components (0x5644F0).
        if self.is_actor(thing) && self.data.things[thing].components & (1 << 29) != 0 {
            self.city_layout(world, rng, index);
        }
    }

    /// 0x4EA260: the parent's base terrain of this biome (virtual +0x100), else
    /// its own; then 0x4EB65A: unless an actor with +0x24 set, the shape grown by
    /// the definition's padding painted (0x555C10; only land paints here, a
    /// mountain or water base terrain is refused).
    fn paint_base_terrain(&mut self, world: &mut World, index: usize) {
        let (thing, pos, facing, parent) = {
            let o = &self.objects[index];
            (o.thing, o.pos, o.facing, o.parent)
        };
        let biome_id = self.biome.ids.to_ascii_lowercase();
        let variation = |t: usize| -> Option<u32> {
            let def = &self.data.things[t];
            let by_biome = def.base_terrain.get(&biome_id).or(def.base_default.as_ref())?;
            self.data.variation_by_id.get(&by_biome.to_ascii_lowercase()).map(|&v| v as u32)
        };
        // Virtual +0x100 is an actor's: a plain thing of a layout has no parent here.
        let parent = parent.filter(|_| self.is_actor(thing));
        let Some(var) = parent.and_then(|p| variation(self.objects[p].thing)).or_else(|| variation(thing)) else { return };
        let def = &self.data.things[thing];
        let grown = super::exact::grow(&space::shape(def.shape), def.base_padding);
        let extent = [world.width, world.height];
        if !(pos[0] >= 0.0 && pos[0] < extent[0] && pos[1] >= 0.0 && pos[1] < extent[1]) {
            return;
        }
        let kind = self.data.terrains[self.data.variations[var as usize].terrain].kind;
        if kind != 0 {
            self.notes.push(format!("{}: a base terrain of kind {kind} is not painted (0x551B40 not ported)", def.ids));
            return;
        }
        // 0x5559F0 with equal ends: one stamp 0x5526C0, the commit 0x559FB0.
        let mut batch = Batch::protecting(self.data, self.biome);
        stamp_shape(world, &mut batch, &grown, facing, pos, var);
        batch.commit(world);
        self.after_terrain_change(world);
    }

    /// 0x54FDC0 for the tiles whose terrain changed: the listed doodads
    /// (flag bit 3, not an actor) standing on that tile are deleted.
    pub(super) fn after_terrain_change(&mut self, world: &mut World) {
        for t in world.take_changed() {
            let on_tile = std::mem::take(&mut self.tile_things[t]);
            for &i in &on_tile {
                let o = &self.objects[i as usize];
                if o.alive && o.auto && !self.is_actor(o.thing) {
                    self.delete_object(i as usize);
                }
            }
            self.tile_things[t] = on_tile;
            self.space.retile(t, self.data, world.var[t]);
        }
    }

    /// 0x591710: the terrain under a required object made fit for it: from the
    /// tile under `pos` outward (a stack, 0xB322E0's directions), every tile
    /// whose square meets `shape` and whose terrain is not of `kind` takes the
    /// variation of the tile it was reached from (a batch, committed 0x559FB0).
    pub(super) fn conform_terrain(&mut self, world: &mut World, shape: &Shape, facing: [f32; 2], pos: [f32; 2], kind: u8) {
        let Ok(tile_shape) = aa_rules::objects::shape(2, &[world.tile.to_bits()], P).map(|s| space::shape(s.words)) else { return };
        let placed = Where { shape: shape.clone(), facing: bits(facing), position: bits(pos) };
        let half = (f64::from(world.tile) * 0.5) as f32;
        let mut records: std::collections::HashMap<usize, (bool, u32)> = std::collections::HashMap::new();
        let start = [(pos[0] as i32) >> world.tile_bits, (pos[1] as i32) >> world.tile_bits];
        let Some(first) = world.tile_at(start[0], start[1]) else { return };
        records.insert(first, (false, world.var[first]));
        let mut stack = vec![first];
        let mut batch = Batch::new(self.data, self.biome);
        const DIRECTIONS: [[i32; 2]; 8] = [[-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];
        while let Some(t) = stack.pop() {
            let current = {
                let r = records.entry(t).or_insert((false, world.var[t]));
                r.0 = true;
                r.1
            };
            let [x, y] = world.tile_xy(t);
            for d in DIRECTIONS {
                let (nx, ny) = (x + d[0], y + d[1]);
                let Some(n) = world.tile_at(nx, ny) else { continue };
                let cx = (f64::from(nx << world.tile_bits) + f64::from(half)) as f32;
                let cy = (f64::from(ny << world.tile_bits) + f64::from(half)) as f32;
                let tile = Where { shape: tile_shape.clone(), facing: space::FACING, position: bits([cx, cy]) };
                if !aa_spatial::shape_overlap::overlaps(&tile, &placed, P).unwrap_or(false) {
                    continue;
                }
                let rec = records.entry(n).or_insert((false, world.var[n]));
                if rec.0 {
                    continue;
                }
                if self.data.terrains[self.data.variations[rec.1 as usize].terrain].kind != kind {
                    rec.1 = current;
                    batch.paint(world, n, current);
                }
                rec.0 = true;
                stack.push(n);
            }
        }
        batch.commit(world);
        self.after_terrain_change(world);
    }

    /// 0x5644F0(actor, -1, 0): the city's layout (the getter and 0x45DBD0 pick
    /// one with more than one), its objects around the city: rows of offset
    /// and facing turned by the city's facing, height 0, mode 6 with the city
    /// as parent, the city's owner.
    fn city_layout(&mut self, world: &mut World, rng: &mut Rng, index: usize) {
        let (thing, pos, facing, owner) = {
            let o = &self.objects[index];
            (o.thing, o.pos, o.facing, o.owner)
        };
        let def = &self.data.things[thing];
        if def.layouts.is_empty() {
            return;
        }
        rng.getter();
        let chosen = if def.layouts.len() == 1 { 0 } else { (rng.at(0x45DBF6).draw_after_getter() % def.layouts.len() as u32) as usize };
        let (fx, fy) = (f64::from(facing[0]), f64::from(facing[1]));
        let a = fy * 0.0 - fx;
        let b = -fy - fx * 0.0;
        for (ids, offset, row_facing) in def.layouts[chosen].clone() {
            let Some(&t) = self.data.thing_by_id.get(&ids.to_ascii_lowercase()) else {
                self.notes.push(format!("city layout object {ids} is not a known thing"));
                continue;
            };
            let (ox, oy) = (f64::from(offset[0]), f64::from(offset[1]));
            let x = (ox * fy + oy * fx + f64::from(pos[0])) as f32;
            let y = (oy * fy - ox * fx + f64::from(pos[1])) as f32;
            let (rx, ry) = (f64::from(row_facing[0]), f64::from(row_facing[1]));
            let turned = [(a * ry + b * rx) as f32, (b * ry - a * rx) as f32];
            let request = Create { thing: t, pos: [x, y], z: 0.0, facing: turned, owner: if self.is_actor(t) { owner } else { None }, parent: Some(index), ..Default::default() };
            self.create_gthing(world, rng, request);
        }
    }

    /// ResolvePlaceholdersAndRandomThings 0x4FDA70 after StartMission (which
    /// draws nothing): in GTID order a placeholder of a kingdom that is no
    /// independent is queued with its nation's substitution, a random thing
    /// with a weighted pick (0x4B6FF0, its table in load order); then the
    /// objects and the actors apart (0x4FC4A0): each source destroyed
    /// (0x4F53D0), the replacement made with its GTID, position and facing.
    pub(in crate::mapgen) fn resolve_session(&mut self, world: &mut World, rng: &mut Rng) {
        let mut order: Vec<usize> = (0..self.objects.len()).filter(|&i| self.objects[i].alive).collect();
        order.sort_by_key(|&i| self.objects[i].id);
        let mut objects = Vec::new();
        let mut actors = Vec::new();
        for source in order {
            let o = &self.objects[source];
            let def = &self.data.things[o.thing];
            match def.category_index {
                0x1F => {
                    let Some(k) = o.owner else { continue };
                    if !self.kingdoms[k].major {
                        continue;
                    }
                    let nation = self.kingdoms[k].nation.as_deref().map(str::to_ascii_lowercase);
                    let substitution = nation.as_ref().and_then(|n| self.data.nations.iter().find(|d| d.ids.eq_ignore_ascii_case(n))).and_then(|d| d.placeholders.get(&def.ids.to_ascii_lowercase()).copied());
                    let Some(thing) = substitution else {
                        self.notes.push(format!("placeholder {} don't know how to replace", o.id));
                        continue;
                    };
                    if self.is_actor(thing) {
                        actors.push(Replacement { source, thing });
                    } else {
                        objects.push(Replacement { source, thing });
                    }
                }
                0x22 | 0x23 => {
                    let list = self.random_table(o.thing);
                    let actor_kind = def.category_index == 0x23;
                    if let Some(thing) = weighted_pick(rng, &list) {
                        if actor_kind {
                            actors.push(Replacement { source, thing });
                        } else {
                            objects.push(Replacement { source, thing });
                        }
                    }
                }
                _ => {}
            }
        }
        for r in objects.into_iter().chain(actors) {
            self.replace(world, rng, &r);
        }
    }

    /// The members of a random thing, in load order.
    fn random_table(&self, thing: usize) -> Vec<(usize, f32)> {
        let ids = self.data.things[thing].ids.to_ascii_lowercase();
        let Some(members) = self.data.random_actors.get(&ids) else { return Vec::new() };
        members.iter().filter(|(_, w)| *w > 0.0).filter_map(|(id, w)| self.data.thing_by_id.get(&id.to_ascii_lowercase()).map(|&t| (t, *w))).collect()
    }

    /// KKC_PlaceholderResolver::CreateReplacement 0x4FBA50.
    fn replace(&mut self, world: &mut World, rng: &mut Rng, r: &Replacement) {
        let (pos, z, facing, id, owner, group) = {
            let o = &self.objects[r.source];
            (o.pos, o.z, o.facing, o.id, o.owner, o.group)
        };
        self.delete_object(r.source);
        self.reserve_id(id);
        // 0x4FD7F0 / 0x4FD8C0: variant 0; an actor also its source's owner and
        // the source's GTID at +0x3C.
        let actor = self.is_actor(r.thing);
        let request = Create { thing: r.thing, pos, z, facing, variant: 0, id: Some(id), owner: if actor { owner } else { None }, group, replaces: Some(r.source), strict: actor, ..Default::default() };
        if self.create_gthing(world, rng, request).is_none() {
            self.notes.push(format!("failure to create replacement {} at {}, {}", self.data.things[r.thing].ids, pos[0], pos[1]));
        }
    }
}

/// 0x5526C0 with any shape: the tile bounds of the shape's radius, every tile
/// whose square meets the placed shape (0x46EF30) painted.
fn stamp_shape(world: &World, batch: &mut Batch<'_>, shape: &Shape, facing: [f32; 2], pos: [f32; 2], var: u32) {
    let inv = 1.0 / f64::from(world.tile);
    let r = f64::from(f32::from_bits(shape.radius_bits));
    let (px, py) = (f64::from(pos[0]), f64::from(pos[1]));
    let ftol = super::super::blob::ftol_d;
    let x0 = ftol((px - r) * inv).clamp(0, world.tw as i32 - 1);
    let y0 = ftol((py - r) * inv).clamp(0, world.th as i32 - 1);
    let x1 = ftol((r + px) * inv).clamp(0, world.tw as i32 - 1);
    let y1 = ftol((r + py) * inv).clamp(0, world.th as i32 - 1);
    let half = (f64::from(world.tile) * 0.5) as f32;
    let placed = Where { shape: shape.clone(), facing: bits(facing), position: bits(pos) };
    let tile_shape = aa_rules::objects::shape(2, &[world.tile.to_bits()], P).map(|s| space::shape(s.words));
    let Ok(tile_shape) = tile_shape else { return };
    for y in y0..=y1 {
        for x in x0..=x1 {
            let cx = (f64::from(x << world.tile_bits) + f64::from(half)) as f32;
            let cy = (f64::from(y << world.tile_bits) + f64::from(half)) as f32;
            let tile = Where { shape: tile_shape.clone(), facing: space::FACING, position: bits([cx, cy]) };
            if aa_spatial::shape_overlap::overlaps(&tile, &placed, P).unwrap_or(false) {
                batch.paint(world, y as usize * world.tw + x as usize, var);
            }
        }
    }
}
