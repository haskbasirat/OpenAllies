//! `GameMap::height_at` against the exact port of the ground query 0x4F0E50
//! (`aa_spatial::query_actor_height` over `aa_heightfield`, fixtures in
//! research/phase5-heightfield), on the shipped maps' own height layers.
//! Reads the original data; set AA_GAME_ROOT.
use aa_numeric::{Binary, Precision};
use aa_play::map::{GameMap, NativeTerrain};
use aa_spatial::{Actor, Cell, HeightLayer, HeightMethod, ShapeKind, ShapePlacement, TerrainGrid, World};
use std::path::PathBuf;

struct Layers {
    base: aa_heightfield::Heightfield,
    overlay: aa_heightfield::Heightfield,
}

impl World for Layers {
    fn sample_height(&mut self, _: &mut Actor, layer: HeightLayer, method: HeightMethod, position: [u32; 2], precision: Precision) -> aa_spatial::Result<Binary> {
        let field = match layer {
            HeightLayer::Base => &self.base,
            HeightLayer::Overlay => &self.overlay,
        };
        Ok(match method {
            HeightMethod::Nearest => field.sample_nearest(position)?,
            HeightMethod::Interpolated => field.sample_interpolated(position, precision)?,
        })
    }
    fn shape(&mut self, _: &mut Actor, _: ShapeKind) -> aa_spatial::Result<u32> {
        unreachable!("a height query reads no shape")
    }
    fn update_tiles(&mut self, _: &mut Actor, _: ShapePlacement, _: ShapePlacement, _: ShapePlacement, _: ShapePlacement) -> aa_spatial::Result<()> {
        unreachable!()
    }
    fn position_changed(&mut self, _: &mut Actor) -> aa_spatial::Result<()> {
        unreachable!()
    }
    fn world_phase(&self) -> u32 {
        0
    }
    fn presentation_available(&self) -> bool {
        false
    }
    fn presentation_changed(&mut self, _: &mut Actor) -> aa_spatial::Result<()> {
        unreachable!()
    }
}

fn port_parts(n: &NativeTerrain) -> (Layers, TerrainGrid) {
    let field = |v: &[f32]| aa_heightfield::Heightfield {
        width: n.width,
        height: n.height,
        coordinate_shift: n.shift,
        cell_size_bits: n.cell.to_bits(),
        index_bias: 1 << (n.shift - 1),
        height_bits: v.iter().map(|x| x.to_bits()).collect(),
    };
    let cells = n.overlay_kind.iter().zip(&n.noise).map(|(k, m)| Cell { overlay_kind: i32::from(*k), offset_min: m[0].to_bits(), offset_max: m[1].to_bits() }).collect();
    let cell_size = f32::from_bits((127 + n.tile_shift) << 23);
    (
        Layers { base: field(&n.base), overlay: field(&n.overlay) },
        TerrainGrid { width: n.tile_width, height: n.tile_height, coordinate_shift: n.tile_shift, cell_size: cell_size.to_bits(), cells },
    )
}

fn check(n: &NativeTerrain, x: f32, y: f32, layers: &mut Layers, grid: &TerrainGrid) {
    let mut actor = Actor { flags: 0, position: [x.to_bits(), y.to_bits(), 0], facing: [0, 0] };
    let want = aa_spatial::query_actor_height(&mut actor, 0x122, grid, Precision::Double53, layers).unwrap().store_f32().unwrap();
    let got = n.ground(x, y).to_bits();
    assert_eq!(got, want, "({x}, {y}): {} against the port's {}", f32::from_bits(got), f32::from_bits(want));
}

/// The same LCG the game draws from; only a source of test points.
fn points(seed: u32, n: usize, w: f32, h: f32) -> impl Iterator<Item = (f32, f32)> {
    let mut s = seed;
    let mut next = move || {
        s = s.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039);
        (s >> 8) as f32 / (1u32 << 24) as f32
    };
    (0..n).map(move |_| (next() * w, next() * h))
}

#[test]
#[ignore = "reads the original data; set AA_GAME_ROOT"]
fn height_at_is_the_ground_query_on_the_shipped_maps() {
    let root = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let data = aa_play::GameData::load(&root).unwrap();
    for id in ["(2v2)Berlin", "(2v2)Kursk", "DeveloperMaps/(2v2)Comrades", "DeveloperMaps/(4FFA)Farmhouses"] {
        let map = GameMap::load_with(&root, id, &data).unwrap();
        let n = map.native_terrain.as_ref().expect("height layers");
        let (mut layers, grid) = port_parts(n);
        let (w, h) = (map.width as f32 * map.cell_size, map.height as f32 * map.cell_size);
        // Inside, on the sample and tile lines, and past the edges.
        for (x, y) in points(0x1234_5678, 60_000, w, h) {
            check(n, x, y, &mut layers, &grid);
        }
        for i in 0..200 {
            let (x, y) = (i as f32 * n.cell, (i / 3) as f32 * n.cell);
            check(n, x, y, &mut layers, &grid);
            check(n, x + 0.25, y, &mut layers, &grid);
            check(n, x, y + 0.25, &mut layers, &grid);
        }
        check(n, w, h, &mut layers, &grid);
        check(n, w + 3.5, h + 3.5, &mut layers, &grid);
        // The renderer's corner heights stay within the JSON's rounding of the
        // query at their own vertices, away from the map edge. The developer
        // maps name terrain variations that no longer exist: the query takes
        // the previous tile's (557D4B), the drawn corners do not, and they
        // differ by up to 3 m on those tiles.
        let mut worst = 0.0f32;
        for y in 1..map.height {
            for x in 1..map.width {
                let corner = map.heights[(y * (map.width + 1) + x) as usize];
                let q = n.ground(x as f32 * map.cell_size, y as f32 * map.cell_size);
                worst = worst.max((corner - q).abs());
            }
        }
        eprintln!("{id}: worst corner difference {worst}");
        if !id.starts_with("DeveloperMaps") {
            assert!(worst < 0.01, "{id}: the drawn corners and the ground query differ by {worst} m");
        }
    }
}
