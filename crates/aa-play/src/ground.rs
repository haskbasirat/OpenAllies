//! What a created thing does to the ground under it in a running battle, as the
//! original runs it (GThing::PostCreate 0x4EBA40 -> 0x4EB500):
//! - a blocking static thing (0x4EE700) with the Thing flag `flatten_terrain` sets the
//!   base map under its blocking shape to the ground height at its position: the height
//!   brush 0x551140 on HEIGHT_MAP (GWorld+0x34), the same code the generator's objects
//!   run (`crate::mapgen::height`), so the slope spreads outward at ElevationSlopeMax
//!   and every other query (units, the camera, the drawn terrain) reads the new ground;
//! - a blocking static thing clears the doodads under its shape (ClearDoodads 0x4F5560);
//! - the base terrain paint of 0x4EB500 only runs while GWorld+0x48 is 0, before the
//!   session starts: the generator's (`crate::mapgen::session`). A created thing in a
//!   running battle paints nothing.
//!
//! Nothing goes back: no native path writes the old heights again when the thing is
//! destroyed or packed up (0x4EB500 has these callers only: 0x4EBA40, 0x4EB820, 0x585F60,
//! and the height brush besides it only the generator and the editor).
use crate::data::Def;
use crate::map::NativeTerrain;
use crate::mapgen::height::{Brush, HeightGrid, Map, brush_placed};
use crate::world::{ActorId, World};
use aa_numeric::Precision;
use aa_spatial::collision::Shape;
use aa_spatial::geometry::Placement;
use std::sync::Arc;

const PC: Precision = Precision::Double53;
/// Thing flag `flatten_terrain` (bit 13 of Thing +8).
const FLATTEN: u32 = 0x2000;
/// The Thing flag bits 0x4EE700 and 0x4EB5F5 ask for: blocking (0x400), static (0x20000000).
const BLOCKING_STATIC: u32 = 0x2000_0400;
/// The native default facing of an axis_locked thing, (0, -1).
const FACING_DEFAULT: [u32; 2] = [0, 0xbf80_0000];

impl HeightGrid for NativeTerrain {
    fn samples(&self) -> [usize; 2] {
        [self.width as usize, self.height as usize]
    }
    fn spacing(&self) -> f32 {
        self.cell
    }
    fn height_bits(&self) -> u32 {
        self.shift
    }
    fn limits(&self) -> [f32; 2] {
        self.limits
    }
    fn field(&self, map: Map) -> &[f32] {
        match map {
            Map::Base => &self.base,
            Map::Overlay => &self.overlay,
        }
    }
    fn field_mut(&mut self, map: Map) -> &mut [f32] {
        match map {
            Map::Base => &mut self.base,
            Map::Overlay => &mut self.overlay,
        }
    }
    fn vertex_area(&self, map: Map, at: usize) -> u8 {
        match map {
            Map::Base => self.areas.get(at).copied().unwrap_or(0),
            // The overlay's +0x44 bytes: nothing writes them.
            Map::Overlay => 0,
        }
    }
    fn max_slope_at(&self, x: i32, y: i32) -> Option<f32> {
        // 0x4282A0: the tile of the vertex, nothing outside the map.
        let (tx, ty) = ((x << self.shift) >> self.tile_shift, (y << self.shift) >> self.tile_shift);
        if tx < 0 || ty < 0 || tx >= self.tile_width || ty >= self.tile_height {
            return None;
        }
        self.max_slope.get((ty * self.tile_width + tx) as usize).copied()
    }
    fn committed(&mut self, x: i32, y: i32) {
        if let Some(r) = self.changes.last_mut() {
            r[0] = r[0].min(x);
            r[1] = r[1].min(y);
            r[2] = r[2].max(x);
            r[3] = r[3].max(y);
        }
    }
}

fn shape(words: [u32; 6]) -> Shape {
    Shape { kind: words[0], radius_bits: words[1], bounds: [words[2], words[3], words[4], words[5]] }
}

/// A placed thing's facing words. An axis_locked thing stands on the exact default
/// (0x4EF2A0), which the radians `Actor::facing` cannot hold.
fn facing_words(def: &Def, facing: f32) -> [u32; 2] {
    if def.axis_locked {
        return FACING_DEFAULT;
    }
    let dir = glam::Vec2::from_angle(facing);
    [dir.x.to_bits(), dir.y.to_bits()]
}

/// The test of 0x4EB500 for a thing of this definition: 0x4EE700 (blocking, static, its
/// radius at least a quarter of a tile) and `flatten_terrain`.
pub fn flattens(world: &World, def: &Def) -> bool {
    let Some(terrain) = world.map.native_terrain.as_ref() else { return false };
    let tile = (1u32 << terrain.tile_shift) as f32;
    def.thing_flags & FLATTEN != 0 && aa_spatial::geometry::marks_blocker(def.thing_flags, def.blocking_words[1], tile.to_bits(), PC).unwrap_or(false)
}

/// PostCreate 0x4EBA40 of a thing created in the running world, what the ground does.
pub fn post_create(world: &mut World, a: ActorId) {
    let data = world.data.clone();
    let actor = &world.actors[a as usize];
    let def = data.def(actor.def);
    if def.thing_flags & BLOCKING_STATIC != BLOCKING_STATIC {
        return;
    }
    let (pos, facing) = (actor.pos, facing_words(def, actor.facing));
    let place = Placement { shape: shape(def.blocking_words), facing, position: [pos.x.to_bits(), pos.y.to_bits()] };
    if flattens(world, def) {
        flatten(world, &place);
    }
    clear_doodads(world, &place);
}

fn flatten(world: &mut World, place: &Placement) {
    let slope = world.data.rule_ci("ElevationSlopeMax").unwrap_or_else(|| {
        // SVars default of the generator's table.
        f32::from_bits(0x3f00_0000)
    });
    let started = world.session_started;
    let map = Arc::make_mut(&mut world.map);
    let Some(terrain) = map.native_terrain.as_mut() else { return };
    flatten_terrain(terrain, place, slope, started);
}

/// 0x4EB55E..0x4EB5F0 on the terrain: the brush settings of the flattening: mode 0 at the
/// ground height 0x4F0E50(0x122) of the position, ElevationSlopeMax, no limits, protect,
/// smooth edge, full strength, area 0. Settings +0x1C is never written: the word on the
/// stack holds what the last call at that depth left. Measured on every film under the
/// sampler: the thing's own Z while the session has not started (0x4F9C10 from 0x55398D,
/// the starting buildings, whose InitExisting reaches 0x42A5D3), 0 on an unpack in the
/// running battle (0x575EB1, state bit 26: 0x4D7230 runs no ExistingFX).
pub fn flatten_terrain(terrain: &mut NativeTerrain, place: &Placement, slope: f32, started: bool) {
    let level = terrain.ground(f32::from_bits(place.position[0]), f32::from_bits(place.position[1]));
    let falloff = if started { 0.0 } else { level };
    let brush = Brush { mode: 0, height: level, slope, min: f32::MIN, max: f32::MAX, protect: true, smooth_edge: true, strength: 1.0, falloff, area: 0 };
    terrain.changes.push([i32::MAX, i32::MAX, i32::MIN, i32::MIN]);
    let done = brush_placed(terrain, Map::Base, place, &brush);
    let empty = terrain.changes.last().is_none_or(|r| r[0] > r[2]);
    if !done || empty {
        terrain.changes.pop();
    }
}

/// The doodads of `scenery` a blocking shape clears (0x4F5560 -> 0x555620): runtime class
/// 2 or 9, not blocking, and their own shape meets it (0x46EF30), as `map.placed` indices.
/// `0x54C200` keeps a thing a script names; aa-play holds no such list, and no doodad of the
/// shipped films is one.
pub fn doodads_under(map: &crate::map::GameMap, data: &crate::data::GameData, scenery: &[usize], place: &Placement) -> Vec<usize> {
    let radius = f64::from(f32::from_bits(place.shape.radius_bits));
    let (px, py) = (f64::from(f32::from_bits(place.position[0])), f64::from(f32::from_bits(place.position[1])));
    let mut gone = Vec::new();
    for &index in scenery {
        let Some(p) = map.placed.get(index) else { continue };
        // The tile lists hold a thing by its shape; nothing further away can meet this one.
        let (dx, dy) = (f64::from(p.position[0]) - px, f64::from(p.position[1]) - py);
        if dx * dx + dy * dy > (radius + 64.0) * (radius + 64.0) {
            continue;
        }
        let Some(d) = data.find(&p.thing_id).map(|d| data.def(d)) else { continue };
        if !matches!(d.runtime_class, 2 | 9) || d.thing_flags & 0x400 != 0 {
            continue;
        }
        let own = Placement { shape: shape(d.blocking_words), facing: [p.facing[0].to_bits(), p.facing[1].to_bits()], position: [p.position[0].to_bits(), p.position[1].to_bits()] };
        if aa_spatial::shape_overlap::overlaps(&own, place, PC).unwrap_or(false) {
            gone.push(index);
        }
    }
    gone
}

/// ClearDoodads 0x4F5560 with the thing's blocking shape: what [`doodads_under`] finds goes
/// (0x4F5490: out of the world, its deferred updates with it).
fn clear_doodads(world: &mut World, place: &Placement) {
    let data = world.data.clone();
    let gone = doodads_under(&world.map, &data, &world.books.scenery, place);
    for &index in &gone {
        world.books.scenery.retain(|&i| i != index);
        world.ids.destroy_placed(index);
        for slot in 0..2 {
            world.unschedule(crate::scheduler::Owner::Doodad(index as u32, slot));
        }
        crate::scheduler::release_placed(world, index);
        world.books.cleared_doodads.push(index);
    }
}
