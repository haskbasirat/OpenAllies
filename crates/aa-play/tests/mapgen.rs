//! Random map generator tests (owner: mapgen). They need the original install
//! (AA_GAME_ROOT, default E:\Games\Axis and Alies) and skip without it.
//! `cargo test -p aa-play --test mapgen -- --nocapture` prints timings.
use aa_play::map::{NOT_LAND_PASSABLE, NOT_WATER_PASSABLE};
use aa_play::mapgen::{GeneratedMap, RandomMapData, RandomMapParams, RmcInfo};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

fn root() -> PathBuf {
    PathBuf::from(std::env::var("AA_GAME_ROOT").unwrap_or_else(|_| r"E:\Games\Axis and Alies".into()))
}

fn data() -> Option<&'static RandomMapData> {
    static DATA: OnceLock<Option<RandomMapData>> = OnceLock::new();
    DATA.get_or_init(|| {
        let root = root();
        if !root.join("Data.rwd").exists() {
            eprintln!("skipping: no original install at {}", root.display());
            return None;
        }
        let t = std::time::Instant::now();
        let d = RandomMapData::load(&root).expect("original random map data");
        eprintln!("original random map data loaded in {:?}", t.elapsed());
        Some(d)
    })
    .as_ref()
}

fn info(rmc: &str) -> RmcInfo {
    data().unwrap().catalog().rmcs.into_iter().find(|r| r.ids == rmc).expect("rmc listed")
}

fn params(rmc: &str, size: f32, sides: u32, seed: u64) -> RandomMapParams {
    let mut p = RandomMapParams::defaults(&info(rmc), sides, seed);
    p.width = size;
    p.height = size;
    p
}

fn generate(p: &RandomMapParams) -> GeneratedMap {
    data().unwrap().generate(p).unwrap_or_else(|e| panic!("{} {}x{} {} sides seed {}: {e:#}", p.rmc, p.width, p.height, p.sides, p.seed))
}

/// Tiles of native terrain type `kind` (0 land, 1 mountain, 2 water).
fn kind_tiles(m: &GeneratedMap, kind: u32) -> usize {
    m.map.flags.iter().filter(|&&f| (f >> 4) & 7 == kind).count()
}
fn tiles_matching(m: &GeneratedMap, f: impl Fn(&str) -> bool) -> usize {
    m.terrain.tiles.iter().filter(|t| f(&m.terrain.palette[t].id)).count()
}
fn things(m: &GeneratedMap, id: &str) -> usize {
    m.placed.iter().filter(|p| p.thing_id == id).count()
}

/// Index of the visible terrain slider for a terrain id substring.
fn slider(info: &RmcInfo, terrain: &str) -> usize {
    info.terrain_brushes.iter().position(|b| b.show_ui && b.ids.contains(terrain)).unwrap_or_else(|| panic!("{} has no {terrain} slider", info.ids))
}

fn summary(m: &GeneratedMap) -> String {
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for &t in &m.terrain.tiles {
        *kinds.entry(m.terrain.palette[&t].id.clone()).or_default() += 1;
    }
    let mut placed: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &m.placed {
        *placed.entry(p.thing_id.as_str()).or_default() += 1;
    }
    let (lo, hi) = m.map.heights.iter().fold((f32::MAX, f32::MIN), |a, &h| (a.0.min(h), a.1.max(h)));
    format!("tiles {kinds:?}\nthings {placed:?}\nheights {lo}..{hi}\nstarts {:?}", m.map.start_locations)
}

/// qa-output/mapgen/<name>.{terrain,scenery,gamemap,params}.json
fn write_outputs(name: &str, m: &GeneratedMap) {
    let dir = std::env::var_os("AA_MAPGEN_QA_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../qa-output/mapgen"));
    std::fs::create_dir_all(&dir).unwrap();
    let w = |suffix: &str, v: serde_json::Value| std::fs::write(dir.join(format!("{name}.{suffix}.json")), serde_json::to_vec(&v).unwrap()).unwrap();
    w("terrain", serde_json::to_value(&m.terrain).unwrap());
    w("scenery", serde_json::to_value(&m.scenery).unwrap());
    w("gamemap", serde_json::to_value(&m.map).unwrap());
    w("params", serde_json::to_value(&m.params).unwrap());
}

#[test]
fn catalog_matches_the_original_options() {
    let Some(d) = data() else { return };
    let cat = d.catalog();
    let ids: Vec<&str> = cat.rmcs.iter().map(|r| r.ids.as_str()).collect();
    assert_eq!(ids, ["rmc_alaska", "rmc_france", "rmc_germany", "rmc_india", "rmc_northafrica", "rmc_philippines", "rmc_southerneurope", "rmc_ukraine"]);
    assert_eq!(cat.default_rmc, "rmc_southerneurope");
    assert_eq!(cat.default_size, [384.0, 384.0]);
    let france = info("rmc_france");
    let sizes: Vec<f32> = france.map_sizes.iter().map(|s| s.width).collect();
    assert_eq!(sizes, [320.0, 384.0, 448.0, 512.0, 576.0, 640.0, 768.0, 1024.0]);
    assert_eq!(france.max_sides, 16);
    let visible: Vec<(&str, [f32; 2])> = france.terrain_brushes.iter().filter(|b| b.show_ui).map(|b| (b.ids.as_str(), b.range)).collect();
    assert_eq!(
        visible,
        [
            ("temperate_grass", [0.0, 20.0]),
            ("temperate_grassdry", [0.0, 10.0]),
            ("temperate_dirt", [0.0, 7.5]),
            ("temperate_forestdeciduous", [0.0, 20.0]),
            ("temperate_mountain", [0.0, 15.0]),
            ("temperate_ocean", [0.0, 15.0]),
        ]
    );
    for b in &france.terrain_brushes {
        assert_eq!(b.default, (b.range[0] + b.range[1]) * 0.5);
    }
}

#[test]
fn same_params_and_seed_give_the_same_map() {
    if data().is_none() {
        return;
    }
    let p = params("rmc_germany", 384.0, 2, 1234);
    let (a, b) = (generate(&p), generate(&p));
    assert_eq!(a.terrain, b.terrain);
    assert_eq!(serde_json::to_value(&a.map).unwrap(), serde_json::to_value(&b.map).unwrap());
    assert_eq!(a.scenery, b.scenery);
    let c = generate(&params("rmc_germany", 384.0, 2, 1235));
    assert_ne!(a.terrain.tiles, c.terrain.tiles, "another seed gives another map");
}

#[test]
fn every_option_moves_the_map_the_expected_way() {
    if data().is_none() {
        return;
    }
    let france = info("rmc_france");
    let base = params("rmc_france", 384.0, 2, 99);
    let with = |terrain: &str, value: f32| {
        let mut p = base.clone();
        p.terrain_weights[slider(&france, terrain)] = value;
        generate(&p)
    };
    // Mountains and water (each capped at 15% of the map by the original SVars).
    let (lo, hi) = (with("temperate_mountain", 0.0), with("temperate_mountain", 15.0));
    let (ml, mh) = (kind_tiles(&lo, 1), kind_tiles(&hi, 1));
    eprintln!("mountain tiles: slider 0 -> {ml}, slider 15 -> {mh}");
    assert!(ml < mh / 4, "mountains {ml} vs {mh}");
    let (lo, hi) = (with("temperate_ocean", 0.0), with("temperate_ocean", 15.0));
    let (wl, wh) = (kind_tiles(&lo, 2), kind_tiles(&hi, 2));
    eprintln!("water tiles: slider 0 -> {wl}, slider 15 -> {wh}");
    assert!(wl < wh / 4, "water {wl} vs {wh}");
    // Land terrains.
    for (terrain, max, pick) in [
        ("temperate_forestdeciduous", 20.0, "temperate_forestdeciduous0"),
        ("temperate_grass", 20.0, "temperate_grass0"),
        ("temperate_grassdry", 10.0, "temperate_grassdry0"),
        ("temperate_dirt", 7.5, "temperate_dirt0"),
    ] {
        let (lo, hi) = (with(terrain, 0.0), with(terrain, max));
        let (a, b) = (tiles_matching(&lo, |id| id.starts_with(pick)), tiles_matching(&hi, |id| id.starts_with(pick)));
        eprintln!("{terrain}: slider 0 -> {a} tiles, slider {max} -> {b}");
        assert!(a < b, "{terrain}: {a} vs {b}");
        if terrain == "temperate_forestdeciduous" {
            let (ta, tb) = (things(&lo, "temperate_trees"), things(&hi, "temperate_trees"));
            eprintln!("  trees {ta} -> {tb}");
            assert!(ta < tb);
        }
    }
    // Map size: more ground, more villages (RMCVillageActorGroups area_factor 1.75).
    let small = generate(&params("rmc_france", 320.0, 2, 99));
    let large = generate(&params("rmc_france", 640.0, 2, 99));
    let (vs, vl) = (things(&small, "france_village01"), things(&large, "france_village01"));
    eprintln!("villages 320 m -> {vs}, 640 m -> {vl}; tiles {} -> {}", small.terrain.tiles.len(), large.terrain.tiles.len());
    assert!(large.terrain.tiles.len() == small.terrain.tiles.len() * 4);
    assert!(vs < vl);
    // Sides: one start location (and Corps HQ placeholder) per side.
    for sides in [2u32, 3, 4, 6, 8] {
        let m = generate(&params("rmc_france", 512.0, sides, 5));
        assert_eq!(m.map.start_locations.len(), sides as usize);
        assert_eq!(things(&m, "placeholder_corpsunpacked"), sides as usize);
        assert_eq!(things(&m, "start_loc"), sides as usize);
    }
}

/// Land reachability between two cells over the GameMap flags (4-connected).
fn reachable(m: &GeneratedMap, a: glam::Vec2, b: glam::Vec2) -> bool {
    let g = &m.map;
    let (Some(s), Some(t)) = (g.cell_of(a), g.cell_of(b)) else { return false };
    let w = g.width as usize;
    let idx = |c: (u32, u32)| c.1 as usize * w + c.0 as usize;
    let land = |i: usize| g.flags[i] & NOT_LAND_PASSABLE == 0;
    let (s, t) = (idx(s), idx(t));
    if !land(s) || !land(t) {
        return false;
    }
    let mut seen = vec![false; g.flags.len()];
    let mut stack = vec![s];
    seen[s] = true;
    while let Some(i) = stack.pop() {
        if i == t {
            return true;
        }
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= g.height as i32 {
                continue;
            }
            let n = ny as usize * w + nx as usize;
            if !seen[n] && land(n) {
                seen[n] = true;
                stack.push(n);
            }
        }
    }
    false
}

#[test]
fn start_locations_are_reachable_over_land_on_every_rmc() {
    let Some(d) = data() else { return };
    for (i, rmc) in d.catalog().rmcs.iter().enumerate() {
        for (size, sides) in [(384.0, 2), (512.0, 4)] {
            let mut p = params(&rmc.ids, size, sides, 17 + i as u64);
            // Push water and mountains to their maximum to stress connectivity.
            for (j, b) in rmc.terrain_brushes.iter().enumerate() {
                if b.show_ui && (b.ids.contains("ocean") || b.ids.contains("mountain")) {
                    p.terrain_weights[j] = b.range[1];
                }
            }
            let m = generate(&p);
            let starts = &m.map.start_locations;
            assert_eq!(starts.len(), sides as usize, "{}", rmc.ids);
            for a in starts {
                for b in starts {
                    assert!(reachable(&m, *a, *b), "{} {size} m: {a:?} cannot reach {b:?}", rmc.ids);
                }
            }
            // HQ placeholders stand on passable land too.
            for hq in m.placed.iter().filter(|p| p.thing_id == "placeholder_corpsunpacked") {
                let c = glam::Vec2::new(hq.position[0], hq.position[1]);
                assert!(reachable(&m, starts[0], c), "{}: HQ at {c:?} unreachable", rmc.ids);
            }
            // Mountains are neither land- nor water-passable (water terrains
            // follow their own land/water flags: fordable rivers, shallows).
            for &f in &m.map.flags {
                if (f >> 4) & 7 == 1 {
                    assert!(f & NOT_LAND_PASSABLE != 0 && f & NOT_WATER_PASSABLE != 0);
                }
            }
        }
    }
}

#[test]
fn two_and_four_player_maps_generate_quickly() {
    if data().is_none() {
        return;
    }
    for (rmc, size, sides) in [("rmc_france", 384.0, 2), ("rmc_germany", 512.0, 4), ("rmc_southerneurope", 640.0, 8), ("rmc_ukraine", 1024.0, 16)] {
        let p = params(rmc, size, sides, 2026);
        let t = std::time::Instant::now();
        let m = generate(&p);
        let elapsed = t.elapsed();
        eprintln!("{rmc} {size} m {sides} sides: {elapsed:?} ({} tiles, {} placed)", m.terrain.tiles.len(), m.placed.len());
        for note in &m.map.load_notes {
            eprintln!("  note: {note}");
        }
        if size <= 512.0 {
            assert!(elapsed.as_secs_f32() < 5.0, "{rmc} took {elapsed:?}");
        }
    }
}

#[test]
fn writes_a_map_for_the_renderer() {
    if data().is_none() {
        return;
    }
    for (name, rmc, size, sides, seed) in [("france_384_2p", "rmc_france", 384.0, 2, 7), ("germany_512_4p", "rmc_germany", 512.0, 4, 11)] {
        let m = generate(&params(rmc, size, sides, seed));
        m.terrain.validate().expect("valid TerrainMap");
        assert!(m.scenery.iter().filter(|s| s.model.is_some()).count() > 100);
        eprintln!("{name}\n{}", summary(&m));
        write_outputs(name, &m);
    }
}

/// Every RMC at its defaults, written for inspection (ignored: run on demand).
#[test]
#[ignore]
fn probe_every_rmc() {
    let Some(d) = data() else { return };
    for r in d.catalog().rmcs {
        let m = generate(&params(&r.ids, 384.0, 2, 3));
        eprintln!("{}\n{}", r.ids, summary(&m));
        write_outputs(&format!("probe_{}", r.ids), &m);
    }
}

/// The core's custom battle setup runs unchanged on a generated map: each
/// side's placeholders become its nation's buildings, villages are neutral.
#[test]
fn a_custom_battle_starts_on_a_generated_map() {
    if data().is_none() {
        return;
    }
    use aa_play::{Controller, PlayerSetup, SkirmishSetup, World};
    use std::sync::Arc;
    let m = generate(&params("rmc_southerneurope", 384.0, 2, 42));
    let game = Arc::new(aa_play::GameData::load(&root()).expect("original rules"));
    let setup = SkirmishSetup {
        map_id: m.map.id.clone(),
        players: vec![
            PlayerSetup { name: "Player".into(), nation: "usa".into(), team: 0, color: [0, 76, 255], controller: Controller::Human, start: 0 },
            PlayerSetup { name: "Computer".into(), nation: "ger".into(), team: 1, color: [217, 0, 25], controller: Controller::Computer(1), start: 1 },
        ],
        starting_resources: None,
        seed: 42,
    };
    let world = World::new_skirmish(game, Arc::new(m.map.clone()), &setup).expect("skirmish on a generated map");
    let owned = |k: u32| world.actors.iter().filter(|a| a.kingdom == Some(k)).count();
    eprintln!("actors {} (player {}, computer {}), kingdoms {}", world.actors.len(), owned(0), owned(1), world.kingdoms.len());
    assert!(owned(0) > 0 && owned(1) > 0, "both sides got their starting buildings");
}

/// Distances of every side's starting objects from its Corps HQ placeholder
/// and from its start location object (printed; ignored probe).
#[test]
#[ignore]
fn probe_start_groups() {
    if data().is_none() {
        return;
    }
    for seed in [1u64, 2, 3, 42, 2026] {
        let m = generate(&params("rmc_southerneurope", 384.0, 2, seed));
        for slot in m.map.player_slots.iter().map(|&s| s as u32) {
            let of = |id: &str| m.placed.iter().filter(|p| p.owner_slot == Some(slot) && p.thing_id == id).map(|p| glam::Vec2::new(p.position[0], p.position[1])).collect::<Vec<_>>();
            let hq = of("placeholder_corpsunpacked")[0];
            let start = of("start_loc")[0];
            let mut line = format!("seed {seed} slot {slot}: hq-start {:.1}", hq.distance(start));
            for id in ["placeholder_depot_supply", "placeholder_bunker", "placeholder_bunker_antiaircraft", "placeholder_bunker_artillery"] {
                for p in of(id) {
                    line += &format!(" | {} hq {:.1} start {:.1}", id.trim_start_matches("placeholder_"), p.distance(hq), p.distance(start));
                }
            }
            eprintln!("{line}");
        }
    }
}

/// Every RMC at every map size, with the fewest and the most recommended
/// sides, places all of every side's required starting objects, and the
/// defences ring the Corps HQ: the start location moves onto the HQ
/// placeholder (PrepareMapFixStartLocations 0x4FB640) and the bunker groups
/// search 12 m around it, at least 12 m from the HQ and the depot.
#[test]
fn every_rmc_size_and_side_count_places_the_starting_defences_around_the_hq() {
    let Some(d) = data() else { return };
    let cat = d.catalog();
    let expected = [
        ("placeholder_corpsunpacked", 1),
        ("placeholder_depot_supply", 1),
        ("placeholder_bunker", 1),
        ("placeholder_bunker_antiaircraft", 2),
        ("placeholder_bunker_artillery", 1),
    ];
    let retries = std::sync::atomic::AtomicUsize::new(0);
    let maps = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for (ri, rmc) in cat.rmcs.iter().enumerate() {
            let (retries, maps, expected) = (&retries, &maps, &expected);
            scope.spawn(move || {
                for size in &rmc.map_sizes {
                    let mut sides: Vec<u32> = vec![2, size.recommended_kingdoms_max.clamp(2, rmc.max_sides)];
                    sides.dedup();
                    for n in sides {
                        let p = params(&rmc.ids, size.width, n, 1000 + ri as u64 * 37 + size.width as u64 + n as u64);
                        let m = generate(&p);
                        maps.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        retries.fetch_add(m.map.load_notes.iter().filter(|n| n.contains("placement try")).count(), std::sync::atomic::Ordering::Relaxed);
                        assert_eq!(m.map.start_locations.len(), n as usize);
                        for (i, &slot) in m.map.player_slots.iter().enumerate() {
                            let own = |id: &str| -> Vec<glam::Vec2> {
                                m.placed
                                    .iter()
                                    .filter(|t| t.owner_slot == Some(slot as u32) && t.thing_id == id)
                                    .map(|t| glam::Vec2::new(t.position[0], t.position[1]))
                                    .collect()
                            };
                            for &(id, count) in expected {
                                assert_eq!(own(id).len(), count, "{} {} m {n} sides: side {i} has {} {id}", rmc.ids, size.width, own(id).len());
                            }
                            let hq = own("placeholder_corpsunpacked")[0];
                            let start = m.map.start_locations[i];
                            assert!(start.distance(hq) < 1e-3, "{}: start location {start:?} not on the HQ {hq:?}", rmc.ids);
                            assert!(own("start_loc")[0].distance(hq) < 1e-3);
                            for &(id, _) in &expected[1..] {
                                for p in own(id) {
                                    let dist = p.distance(hq);
                                    assert!((11.9..=25.0).contains(&dist), "{} {} m {n} sides: {id} {dist:.1} m from the HQ", rmc.ids, size.width);
                                }
                            }
                        }
                    }
                }
            });
        }
    });
    eprintln!("{} maps, {} placement retries", maps.into_inner(), retries.into_inner());
}
