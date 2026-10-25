//! The generated world's GMap occupancy as the original's placement queries
//! see it during KKC_RandomMapGenerator::_GenerateMap (0x5925E0):
//! - the tile flag word +0x20 of every tile (land 0x200, mountain 0x310,
//!   water 0x120, the outer ring adding 0x301, a static blocker 0x400);
//! - the spatial index members (every created thing, registered in the
//!   tiles of its soft shape; a blocking static thing whose blocking radius
//!   reaches a quarter tile also marks the tiles it covers with 0x400, as
//!   the native index insert does);
//! - the first-hit query 0x553BA0 (aa_spatial::query::first_hit without the
//!   source, owner and hostility filters, which no generator query sets);
//! - the static SAI analysis 0x7ACB00 (aa_navigation::topology_builder) for
//!   CheckPositionValid's primary island test.
//!
//! Geometry is aa-spatial's exact shape code at PC53.
use super::data::{RmgData, ThingDef};
use super::grid::World;
use aa_numeric::Precision;
use aa_spatial::collision::{self, Grid, Shape};
use aa_spatial::geometry::{self, Placement};

pub(super) const P: Precision = Precision::Double53;
/// [[0xB39990]+0x64]+4: the default facing (0, -1).
pub(super) const FACING: [u32; 2] = [0, 0xbf80_0000];

/// Tile flag bits (GMap tile +0x20).
pub(super) const BLOCKS_WATER: u32 = 0x200;
pub(super) const STATIC_BLOCKER: u32 = 0x400;

pub(super) fn shape(words: [u32; 6]) -> Shape {
    Shape { kind: words[0], radius_bits: words[1], bounds: [words[2], words[3], words[4], words[5]] }
}

/// Definition virtual +0x28 (0x49BD10): the blocking shape, expanded by
/// GameData+0x44 (BuildingPaddingMeters) for a blocking static thing
/// (flags & 0x20000400) that is no wall. A circle grows its radius; a
/// square or rect grows each half extent and is rebuilt as a rect
/// (0x49BD59..0x49BE07, aa_rules::objects::shape).
pub(super) fn soft_shape(def: &ThingDef, clearance: f32) -> Shape {
    let original = shape(def.shape);
    if def.native_kind == 2 && def.components & (1 << 12) != 0 {
        return original;
    }
    if def.flags & 0x2000_0400 != 0x2000_0400 || original.kind > 3 {
        return original;
    }
    if original.kind <= 1 {
        let r = (f64::from(clearance) + f64::from(f32::from_bits(original.radius_bits))) as f32;
        return Shape { kind: 1, radius_bits: r.to_bits(), bounds: [0; 4] };
    }
    let mut dims = [0u32; 2];
    for a in [1usize, 0] {
        let v = f64::from(f32::from_bits(original.bounds[a + 2])) + f64::from(clearance);
        dims[a] = ((v + v) as f32).to_bits();
    }
    match aa_rules::objects::shape(3, &dims, P) {
        Ok(s) => shape(s.words),
        Err(_) => original,
    }
}

#[derive(Clone, Debug)]
struct Member {
    flags: u32,
    blocking: Placement,
}

pub(super) struct Space {
    pub grid: Grid,
    pub extent_bits: [u32; 2],
    pub flags: Vec<u32>,
    members: Vec<Member>,
    tiles: Vec<Vec<u32>>,
    cell_shape: Shape,
    /// Per tile: the SAI rectangle's island, and the primary island.
    island: Vec<i32>,
    primary: Option<u32>,
}

/// The terrain part of a tile's flag word.
pub(super) fn terrain_flags(data: &RmgData, var: u32, edge: bool) -> u32 {
    let t = &data.terrains[data.variations[var as usize].terrain];
    let mut f = match t.kind {
        1 => 0x310,
        2 => 0x120,
        _ => BLOCKS_WATER,
    };
    if edge {
        f |= 0x301;
    }
    f
}

impl Space {
    pub(super) fn new(world: &World, data: &RmgData) -> Self {
        let (w, h) = (world.tw, world.th);
        let flags = (0..w * h)
            .map(|t| {
                let (x, y) = (t % w, t / w);
                terrain_flags(data, world.var[t], x == 0 || y == 0 || x + 1 == w || y + 1 == h)
            })
            .collect();
        let half = f64::from(world.tile) * 0.5;
        let radius = (half * half + half * half).sqrt() as f32;
        let (neg, pos) = ((-half as f32).to_bits(), (half as f32).to_bits());
        let mut s = Self {
            grid: Grid { dimensions: [w as i32, h as i32], coordinate_shift: world.tile_bits, cell_size_bits: world.tile.to_bits() },
            extent_bits: [world.width.to_bits(), world.height.to_bits()],
            flags,
            members: Vec::new(),
            tiles: vec![Vec::new(); w * h],
            cell_shape: Shape { kind: 2, radius_bits: radius.to_bits(), bounds: [neg, neg, pos, pos] },
            island: vec![-1; w * h],
            primary: None,
        };
        s.analyse(data);
        s
    }

    /// 0x7ACB00 on the current tile flags (StaticMapAnalysis), keeping the
    /// tile islands and the primary island 0x7AA2A0.
    pub(super) fn analyse(&mut self, data: &RmgData) {
        use aa_navigation::topology::{GridLayout, StaticTile};
        use aa_navigation::topology_builder::{StaticGrid, StaticMapParameters, build};
        let sv = &data.svars;
        let tiles: Vec<StaticTile> = self.flags.iter().map(|&f| StaticTile { flags20: f, definition_type: 0 }).collect();
        let layout = GridLayout { width: self.grid.dimensions[0], height: self.grid.dimensions[1], coordinate_shift: self.grid.coordinate_shift, extent_bits: self.extent_bits };
        // 0x7AA180: trunc(floor(width * pow((W + H) * 0.5 / base, exp) + 0.5)).
        let mean = (f64::from(self.grid.dimensions[0]) + f64::from(self.grid.dimensions[1])) * 0.5 / f64::from(sv.static_base_width as f32);
        let width = (mean.powf(f64::from(sv.static_width_exp)) * f64::from(sv.static_max_width as f32) + 0.5).floor() as i32;
        let params = StaticMapParameters {
            starting_square_size: sv.static_square,
            use_fast_square_reduction: sv.static_fast,
            fast_square_reduction_mult: sv.static_fast_mult,
            fast_square_reduction_div: sv.static_fast_div,
            min_region_size: sv.static_min_size,
            min_region_width: sv.static_min_width,
            max_region_size_for_expansion: sv.static_max_expansion,
            max_rectangle_width: width,
        };
        let grid = StaticGrid { layout, cell_size_bits: self.grid.cell_size_bits, tiles: &tiles };
        match build(&grid, &params, P) {
            Ok(a) => {
                for (t, r) in a.topology.tile_rectangles.iter().enumerate() {
                    self.island[t] = r.map_or(-1, |r| a.topology.rectangles[r as usize].island);
                }
                self.primary = a.post_load.primary_island;
            }
            Err(_) => {
                self.island.iter_mut().for_each(|i| *i = -1);
                self.primary = None;
            }
        }
    }

    pub(super) fn tile_at(&self, pos: [f32; 2]) -> Option<usize> {
        // 0x4282A0 / 0x5543F0: 0 <= v < extent, then _ftol >> shift.
        let ext = self.extent_bits.map(f32::from_bits);
        if !(pos[0] >= 0.0 && pos[0] < ext[0] && pos[1] >= 0.0 && pos[1] < ext[1]) {
            return None;
        }
        let x = (pos[0] as i32) >> self.grid.coordinate_shift;
        let y = (pos[1] as i32) >> self.grid.coordinate_shift;
        (x >= 0 && y >= 0 && x < self.grid.dimensions[0] && y < self.grid.dimensions[1]).then(|| (y * self.grid.dimensions[0] + x) as usize)
    }

    /// CheckPositionValid's 0x54ECF0 test: the tile's region lies on the
    /// primary island ([[0xB3652C]+0x2C]).
    pub(super) fn on_primary_island(&self, tile: usize) -> bool {
        self.primary.is_some_and(|p| self.island[tile] == p as i32)
    }

    /// The index insert (0x553xxx full membership): every tile of the soft
    /// shape's rectangle lists the member; a static blocker marks 0x400 on the
    /// tiles whose probe its blocking shape overlaps.
    pub(super) fn add(&mut self, flags: u32, blocking: Placement, soft: Placement) {
        let index = self.members.len() as u32;
        let Ok(rect) = geometry::tile_rectangle(&self.grid, &soft, P) else { return };
        let marks = geometry::marks_blocker(flags, blocking.shape.radius_bits, self.grid.cell_size_bits, P).unwrap_or(false);
        let actor = collision::Actor { definition: 0, flags, position: blocking.position, facing: blocking.facing };
        for x in rect.min[0]..=rect.max[0] {
            for y in rect.min[1]..=rect.max[1] {
                let t = (y * self.grid.dimensions[0] + x) as usize;
                self.tiles[t].push(index);
                if marks {
                    if let Ok(probe) = collision::probe(&self.grid, [x, y], P) {
                        if collision::overlaps(&blocking.shape, &actor, &probe, P).unwrap_or(false) {
                            self.flags[t] |= STATIC_BLOCKER;
                        }
                    }
                }
            }
        }
        self.members.push(Member { flags, blocking });
    }

    fn tile_placement(&self, cell: [i32; 2]) -> Placement {
        let half = f64::from(f32::from_bits(self.grid.cell_size_bits)) * 0.5;
        let position = [0usize, 1].map(|a| ((f64::from(cell[a] << self.grid.coordinate_shift)) + half) as f32).map(f32::to_bits);
        Placement { shape: self.cell_shape.clone(), facing: FACING, position }
    }

    fn find_blocker(&self, cell: [i32; 2]) -> bool {
        let t = (cell[1] * self.grid.dimensions[0] + cell[0]) as usize;
        let Ok(probe) = collision::probe(&self.grid, cell, P) else { return false };
        self.tiles[t].iter().any(|&m| {
            let m = &self.members[m as usize];
            let actor = collision::Actor { definition: 0, flags: m.flags, position: m.blocking.position, facing: m.blocking.facing };
            geometry::marks_blocker(m.flags, m.blocking.shape.radius_bits, self.grid.cell_size_bits, P).unwrap_or(false)
                && collision::overlaps(&m.blocking.shape, &actor, &probe, P).unwrap_or(false)
        })
    }

    /// 0x553BA0 with a descriptor that has no source actor, ignore or owner:
    /// 0 clear, 1 a blocking tile, 2 an actor, 3 outside the world.
    /// `large_only`: the source definition is not blocking (0x553BE2).
    pub(super) fn first_hit(&self, query: &Placement, mask: u32, required: u32, large_only: bool) -> u32 {
        let Ok(boundary) = aa_spatial::query::outside(query, self.extent_bits, P) else { return 3 };
        let Ok(rect) = geometry::tile_rectangle(&self.grid, query, P) else { return 3 };
        if boundary && mask != 0 {
            return 3;
        }
        let mut seen = vec![false; self.members.len()];
        let tile_mask = if mask & STATIC_BLOCKER != 0 { mask & !STATIC_BLOCKER } else { mask };
        for y in rect.min[1]..=rect.max[1] {
            for x in rect.min[0]..=rect.max[0] {
                let t = (y * self.grid.dimensions[0] + x) as usize;
                let flags = self.flags[t];
                if flags & tile_mask != 0 && aa_spatial::shape_overlap::overlaps(&self.tile_placement([x, y]), query, P).unwrap_or(false) {
                    if flags & STATIC_BLOCKER == 0 {
                        if std::env::var_os("AA_RMG_HIT_DEBUG").is_some() {
                            eprintln!("    tile hit at ({x}, {y}) flags {flags:x}");
                        }
                        return 1;
                    }
                    if self.find_blocker([x, y]) {
                        return 2;
                    }
                }
                for &m in &self.tiles[t] {
                    let member = &self.members[m as usize];
                    if large_only && !geometry::marks_blocker(member.flags, member.blocking.shape.radius_bits, self.grid.cell_size_bits, P).unwrap_or(false) {
                        continue;
                    }
                    if std::mem::replace(&mut seen[m as usize], true) {
                        continue;
                    }
                    if member.flags & required != required {
                        continue;
                    }
                    if aa_spatial::shape_overlap::overlaps(query, &member.blocking, P).unwrap_or(false) {
                        return 2;
                    }
                }
            }
        }
        0
    }
}
