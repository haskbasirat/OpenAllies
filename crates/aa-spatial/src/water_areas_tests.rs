//! Rust-only error paths and x87 special-value rules of the water-area port.
//! Native parity is replayed in tests/water_areas_native.rs.
use super::*;

fn tile(class: i32, area: u8) -> WaterTile {
    WaterTile {
        terrain_class: Some(class),
        link: None,
        visit_stamp: 0,
        area,
        reached_from: 0,
    }
}

struct Fixture {
    map: WaterMap,
    heights: Heightfield,
    vertices: VertexAreas,
    solver: FloodSolver,
}

/// `rows`: '~' water, '.' land, digit = water already in that area.
fn fixture(rows: &[&str]) -> Fixture {
    let (w, h) = (rows[0].len() as i32, rows.len() as i32);
    let tiles = rows
        .iter()
        .flat_map(|r| r.chars())
        .map(|c| match c {
            '~' => tile(2, 0),
            '.' => tile(1, 0),
            d => tile(2, d.to_digit(10).unwrap() as u8),
        })
        .collect::<Vec<_>>();
    let mut areas = vec![None];
    for id in 1..AREA_TABLE_LEN {
        let count = tiles.iter().filter(|t| usize::from(t.area) == id).count() as i32;
        areas.push(Some(AreaRecord { id: id as u8, count }));
    }
    let (vw, vh) = (w + 2, h + 2);
    Fixture {
        map: WaterMap {
            width: w,
            height: h,
            coordinate_shift: 1,
            cell_size_bits: 2f32.to_bits(),
            extent_bits: [(w * 2) as f32, (h * 2) as f32].map(f32::to_bits),
            visit_stamp: 3,
            tiles,
            areas,
        },
        heights: Heightfield {
            width: vw,
            height: vh,
            coordinate_shift: 1,
            cell_size_bits: 2f32.to_bits(),
            index_bias: 1,
            height_bits: vec![0; (vw * vh) as usize],
        },
        vertices: VertexAreas {
            width: vw,
            height: vh,
            coordinate_shift: 1,
            bytes: vec![0; (vw * vh) as usize],
        },
        solver: FloodSolver::default(),
    }
}

impl Fixture {
    fn flood(&mut self, start: [i32; 2], record: usize) -> (Result<u32>, Vec<Diagnostic>) {
        let mut diagnostics = Vec::new();
        let mut world = WaterWorld {
            map: &mut self.map,
            heights: &self.heights,
            vertices: &mut self.vertices,
            solver: &mut self.solver,
        };
        let r = flood_fill_water_area(&mut world, start, record, Precision::Double53, &mut diagnostics);
        (r, diagnostics)
    }
    /// Tile (x,y) centre sample reads vertex (x+1,y+1) with shift 1, bias 1.
    fn set_tile_height(&mut self, x: i32, y: i32, bits: u32) {
        let w = self.heights.width;
        self.heights.height_bits[((y + 1) * w + x + 1) as usize] = bits;
    }
}

#[test]
fn water_areas_table_index_255_is_rejected_before_effects() {
    let mut f = fixture(&["~~", "~~"]);
    let before = f.map.clone();
    assert!(matches!(f.flood([0, 0], 255).0, Err(Error::AreaTableIndex(255))));
    assert_eq!(f.map, before);
    assert!(!f.solver.busy);
}

#[test]
fn water_areas_start_outside_allocation_keeps_native_partial_effects() {
    let mut f = fixture(&["~~", "~~"]);
    assert!(matches!(f.flood([5, 9], 1).0, Err(Error::TileOutside(23))));
    // Stamp advanced and the busy byte set before the faulting reach.
    assert_eq!(f.map.visit_stamp, 4);
    assert!(f.solver.busy);
    // The original's next solve sees the busy byte, clears it and returns 0.
    assert_eq!(f.flood([0, 0], 1).0.unwrap(), 0);
    assert!(!f.solver.busy);
    assert_eq!(f.flood([0, 0], 1).0.unwrap(), 1);
    assert!(f.map.tiles.iter().all(|t| t.area == 1));
}

#[test]
fn water_areas_vertex_outside_allocation_is_an_error() {
    let mut f = fixture(&["~~", "~~"]);
    f.vertices.coordinate_shift = 0;
    f.heights.coordinate_shift = 0;
    // Shift 0 maps tile centre x=3.0 to vertex 3 and 4: the second row
    // still fits the 4x4 allocation linearly, the last tile does not.
    let r = f.flood([0, 0], 1).0;
    assert!(matches!(r, Err(Error::VertexOutside(_))), "{r:?}");
}

#[test]
fn water_areas_unordered_heights_never_block_and_infinities_do() {
    let p = Precision::Single24;
    let nan = 0x7FC0_0000;
    let snan = 0x7F80_0001;
    let inf = 0x7F80_0000;
    let neg_inf = 0xFF80_0000;
    assert!(!exceeds_spread_threshold(nan, 0, p).unwrap());
    assert!(!exceeds_spread_threshold(0, snan, p).unwrap());
    assert!(!exceeds_spread_threshold(inf, inf, p).unwrap());
    assert!(!exceeds_spread_threshold(neg_inf, neg_inf, p).unwrap());
    assert!(exceeds_spread_threshold(inf, neg_inf, p).unwrap());
    assert!(exceeds_spread_threshold(0, inf, p).unwrap());
    assert!(exceeds_spread_threshold(neg_inf, 5f32.to_bits(), p).unwrap());
    assert!(!exceeds_spread_threshold(SPREAD_THRESHOLD_BITS, 0, p).unwrap());
    assert!(exceeds_spread_threshold(SPREAD_THRESHOLD_BITS + 1, 0, p).unwrap());

    let mut f = fixture(&["~~3", "~~3"]);
    for y in 0..2 {
        f.set_tile_height(2, y, nan);
    }
    let (r, d) = f.flood([0, 0], 1);
    assert_eq!(r.unwrap(), 1);
    assert!(d.is_empty());
    assert!(f.map.tiles.iter().all(|t| t.area == 1));

    let mut f = fixture(&["~~3", "~~3"]);
    for y in 0..2 {
        f.set_tile_height(2, y, inf);
    }
    let (r, d) = f.flood([0, 0], 1);
    assert_eq!(r.unwrap(), 1);
    assert!(!d.is_empty());
    assert_eq!(f.map.tiles[2].area, 3);
}

#[test]
fn water_areas_null_terrain_and_records_fault_like_the_original() {
    let mut f = fixture(&["~~", "~~"]);
    f.map.tiles[3].terrain_class = None;
    assert!(matches!(f.flood([0, 0], 1).0, Err(Error::NullTerrain(3))));
    let mut f = fixture(&["~~", "~~"]);
    f.map.areas[4] = None;
    assert!(matches!(f.flood([0, 0], 4).0, Err(Error::NullAreaRecord(4))));
    assert!(f.solver.busy);
    let mut f = fixture(&["~~", "~~"]);
    f.map.areas[7] = None;
    // The search stops at the first free id before reaching the null entry.
    assert_eq!(free_area_id(&f.map).unwrap(), 1);
    f.map.areas[1].as_mut().unwrap().count = 1;
    f.map.areas[2] = None;
    assert!(matches!(free_area_id(&f.map), Err(Error::NullAreaRecord(2))));
}

#[test]
fn water_areas_reject_non_finite_stroke_steps() {
    struct NoStroke;
    impl StrokeWorld for NoStroke {
        fn stamp(&mut self, _: &mut WaterMap, _: &StrokeBatch, _: [u32; 2]) -> Result<()> {
            panic!("no stamp expected")
        }
        fn commit(&mut self, _: &mut WaterMap, _: &StrokeBatch) -> Result<()> {
            panic!("no commit expected")
        }
    }
    let mut f = fixture(&["~~", "~~"]);
    let mut world = WaterWorld {
        map: &mut f.map,
        heights: &f.heights,
        vertices: &mut f.vertices,
        solver: &mut f.solver,
    };
    let request = StrokeRequest {
        brush: StrokeBrush {
            kind: 3,
            size_bits: 1f32.to_bits(),
            extent_bits: [0x7FC0_0000, 1f32.to_bits()],
        },
        from_bits: [1f32.to_bits(); 2],
        to_bits: [2f32.to_bits(); 2],
        area: 0,
        batch_flag: 0,
    };
    // NaN extent compares unordered, so the second extent (finite) is used.
    let mut nan_step = request;
    nan_step.brush.extent_bits = [1f32.to_bits(), 0x7FC0_0000];
    let r = stroke_and_flood(&mut world, &mut NoStroke, &nan_step, Precision::Double53, &mut Vec::new());
    assert!(matches!(r, Err(Error::NonFinite(_))));
    // The batch already advanced the stamp (non-transactional).
    assert_eq!(world.map.visit_stamp, 4);
}

#[test]
fn water_areas_nearest_bits_match_the_ported_sampler() {
    let mut f = fixture(&["~~~", "~~~", "~~~"]);
    for (k, h) in f.heights.height_bits.iter_mut().enumerate() {
        *h = (k as f32 * 0.375 - 3.0).to_bits();
    }
    for bias in [0, 1, 3] {
        f.heights.index_bias = bias;
        for x in [-4.5f32, -0.25, 0.0, 1.0, 2.75, 5.0, 9.5, 100.0] {
            for y in [-1.0f32, 0.5, 3.0, 4.999, 7.0, 60.0] {
                let p = [x.to_bits(), y.to_bits()];
                assert_eq!(
                    nearest_height_bits(&f.heights, p).unwrap(),
                    f.heights.sample_nearest(p).unwrap().store_f32().unwrap()
                );
            }
        }
    }
}

#[test]
fn water_areas_layout_mismatch_is_rejected() {
    let mut f = fixture(&["~~", "~~"]);
    f.vertices.width += 1;
    f.vertices.bytes.extend([0; 4]);
    assert!(matches!(f.flood([0, 0], 1).0, Err(Error::InvalidLayout)));
    let mut f = fixture(&["~~", "~~"]);
    f.map.areas.pop();
    assert!(matches!(f.flood([0, 0], 1).0, Err(Error::InvalidLayout)));
}
