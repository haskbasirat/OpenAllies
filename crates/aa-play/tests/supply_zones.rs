//! Supply zone tiles: the ZoneSolver flood (supply::flood_tiles).
use aa_play::GameMap;
use aa_play::map::{BLOCKED, NOT_LAND_PASSABLE};
use aa_play::supply::flood_tiles;

fn open_map(size: u32, cell: f32) -> GameMap {
    let n = (size * size) as usize;
    GameMap { id: "test".into(), width: size, height: size, cell_size: cell, flags: vec![0; n], speed_limit: vec![f32::MAX; n], ..Default::default() }
}

fn covered(tiles: &[u32], map: &GameMap, x: u32, y: u32) -> bool {
    tiles.binary_search(&(y * map.width + x)).is_ok()
}

#[test]
fn a_supply_zone_is_an_octagon_not_a_circle() {
    let map = open_map(64, 4.0);
    // 40 m = 10 tiles from tile (32, 32)
    let tiles = flood_tiles(&map, 32, 32, 40.0);
    // straight out: 10 steps of one tile
    assert!(covered(&tiles, &map, 42, 32) && !covered(&tiles, &map, 43, 32));
    assert!(covered(&tiles, &map, 32, 22) && !covered(&tiles, &map, 32, 21));
    // diagonally: 7 steps of sqrt 2 tiles = 9.9 tiles (a circle reaches 7.07)
    assert!(covered(&tiles, &map, 39, 39) && !covered(&tiles, &map, 40, 40));
    // between: 2 diagonal + 7 straight = 9.83 tiles, Euclid 9.22: in; a
    // circle of 10 tiles holds (40, 38) (8.94 tiles), the flood does not
    // (6 diagonal + 2 straight = 10.49)
    assert!(covered(&tiles, &map, 41, 34));
    assert!(!covered(&tiles, &map, 40, 38));
}

#[test]
fn supply_stops_at_impassable_tiles_and_water() {
    let mut map = open_map(64, 4.0);
    // a wall at x = 36 with no gap: nothing beyond it
    for y in 0..64 {
        map.flags[(y * 64 + 36) as usize] = BLOCKED;
    }
    let tiles = flood_tiles(&map, 32, 32, 40.0);
    assert!(covered(&tiles, &map, 35, 32));
    assert!(tiles.iter().all(|t| t % 64 < 36), "beyond the wall");
    // water (not land passable) west of x = 28 is not entered from land
    let mut map = open_map(64, 4.0);
    for y in 0..64 {
        for x in 0..28 {
            map.flags[(y * 64 + x) as usize] = NOT_LAND_PASSABLE;
        }
    }
    let tiles = flood_tiles(&map, 32, 32, 40.0);
    assert!(covered(&tiles, &map, 28, 32) && !covered(&tiles, &map, 27, 32));
    // a zone on an impassable tile covers nothing
    let mut map = open_map(64, 4.0);
    map.flags[(32 * 64 + 32) as usize] = BLOCKED;
    assert!(flood_tiles(&map, 32, 32, 40.0).is_empty());
}
