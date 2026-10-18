use aa_numeric::Precision;
use aa_spatial::{
    collision::Grid,
    geometry::{Placement, TileRect, tile_rectangle, world_bounds},
};
use serde::Deserialize;
#[derive(Deserialize)]
struct Case {
    grid: Grid,
    placement: Placement,
    precision: Precision,
    native: Expected,
}
#[derive(Deserialize)]
struct Expected {
    rect: TileRect,
    bounds: [u32; 4],
}
#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}
#[test]
fn native_placed_shapes_and_tile_extents_match_all_precision_modes() {
    let f: Fixture = serde_json::from_str(include_str!(
        "../../../research/phase7-spatial/native-geometry.json"
    ))
    .unwrap();
    assert_eq!(f.cases.len(), 792);
    for (i, c) in f.cases.into_iter().enumerate() {
        assert_eq!(
            world_bounds(&c.placement, c.precision).unwrap(),
            c.native.bounds,
            "bounds case{i}"
        );
        assert_eq!(
            tile_rectangle(&c.grid, &c.placement, c.precision).unwrap(),
            c.native.rect,
            "rect case{i}"
        );
    }
}
