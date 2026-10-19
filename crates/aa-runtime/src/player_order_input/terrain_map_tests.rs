use super::*;
use crate::player_order_input::terrain_ray::{intersect_71d840, Target};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;

const UNWRITTEN_POINT: u32 = 0xcdcd_cdcd; // native_terrain_map.py point fill
const UNWRITTEN_SECONDARY: u32 = 0xabab_abab; // native_terrain_map.py secondary fill

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/terrain-map/native-terrain-map.json");
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap() as u32
}
fn int(value: &Value) -> i32 {
    value.as_i64().unwrap() as i32
}
fn words3(value: &Value) -> [u32; 3] {
    let v: Vec<u32> = value.as_array().unwrap().iter().map(word).collect();
    v.try_into().unwrap()
}
fn precision(value: &Value) -> Precision {
    match value.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}

struct LabTiles {
    size: [i32; 2],
    classes: Vec<Option<i32>>,
}
impl Tiles for LabTiles {
    fn size(&self) -> [i32; 2] {
        self.size
    }
    /// The laboratory GMaps hold one VariationData per class value.
    fn tile(&self, index: i32) -> Result<Option<(usize, i32)>> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.classes.get(i).copied())
            .map(|c| c.map(|c| (c as usize, c)))
            .ok_or_else(|| format!("lab tile {index} outside the allocation"))
    }
    fn gmap_words(&self) -> Result<[u32; 2]> {
        Err("this oracle seams 7377F0 (no GMap water geometry)".into())
    }
    fn cached_cell(&self, _: [i32; 2]) -> Result<Option<Vec<[u32; 2]>>> {
        Err("this oracle seams 7377F0 (no GMap+5C cache)".into())
    }
    fn base_height(&self, _: [u32; 2], _: Precision) -> Result<Binary> {
        Err("this oracle seams 7377F0 (no 550210 heights)".into())
    }
}
fn lab_tiles(spec: &Value) -> LabTiles {
    let tiles = &spec["tiles"];
    if tiles.is_null() {
        return LabTiles { size: [0, 0], classes: Vec::new() };
    }
    LabTiles {
        size: [int(&tiles["size"][0]), int(&tiles["size"][1])],
        classes: tiles["classes"].as_array().unwrap().iter().map(|c| c.as_i64().map(|c| c as i32)).collect(),
    }
}
fn heights_of<'a>(spec: &Value, samples: &'a [u32]) -> Heights<'a> {
    let h = &spec["heightfield"];
    Heights { columns: int(&h["columns"]), rows: int(&h["rows"]), step_bits: word(&h["step"]), samples }
}
/// native_terrain_map.py's 4F0E50 seam (71AFD0 only): +0.0 for every query.
fn zero_ground(_: [u32; 2], _: u32) -> Result<Binary> {
    Ok(Binary::from_i32(0))
}
fn source_of<'a>(spec: &Value, samples: &'a [u32]) -> Source<'a> {
    Source {
        extents_bits: [word(&spec["extents"][0]), word(&spec["extents"][1])],
        gtile_bits: word(&spec["gtile"]),
        terrain_block_size: int(&spec["block"]),
        terrain_quad_size: int(&spec["quad"]),
        heights: heights_of(spec, samples),
        ground: &zero_ground,
    }
}

/// The Rust map in native_terrain_map.py's state layout.
fn snapshot(map: &TerrainMap, vertices_of: Option<usize>) -> Value {
    let blocks: Vec<Value> = map
        .blocks
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let mut v = json!({
                "owner": b.owner == [map.up, map.x_normal, map.y_normal],
                "mesh": matches!(b.residency, Residency::Resident(_)),
                "min": b.min, "max": b.max, "water": b.water,
                "index": b.index, "stride": b.stride, "quads": b.quads,
                "quad_size": b.quad_size, "water_dirty": u8::from(b.water_dirty),
            });
            if let (Some(cell), Residency::Resident(vertices)) = (vertices_of, &b.residency) {
                if cell == i {
                    v["vertices"] = json!(vertices.iter().flatten().copied().collect::<Vec<u32>>());
                }
            }
            v
        })
        .collect();
    json!({
        "map": {
            "up": map.up, "x_normal": map.x_normal, "y_normal": map.y_normal,
            "width": map.size[0], "height": map.size[1],
            "blocks": map.blocks_xy, "min": map.min, "max": map.max, "dirty": u8::from(map.changed),
            "max_holder": map.max_holder, "min_holder": map.min_holder, "block_size": map.block_size,
            "stride": map.stride, "quads": map.quads, "quad_size": map.quad_size,
            "quads_per_gtile": map.quads_per_gtile, "loaded": map.loaded, "recompute": u8::from(map.recompute),
        },
        "blocks": blocks,
    })
}
/// The native state without the fields the port does not keep: map +7C/+80
/// quads and +B0 gtile size, block +D0 gtiles and the +90 water holder.
fn native_state(state: &Value) -> Value {
    let mut state = state.clone();
    for key in ["quads_x", "quads_y", "gtile"] {
        state["map"].as_object_mut().unwrap().remove(key);
    }
    for block in state["blocks"].as_array_mut().unwrap() {
        for key in ["water_holder", "gtiles"] {
            block.as_object_mut().unwrap().remove(key);
        }
    }
    state
}
fn construct_error(source: &Source<'_>) -> String {
    match TerrainMap::construct(source, Precision::Double53) {
        Ok(_) => panic!("construction succeeded"),
        Err(e) => e,
    }
}
fn first_difference(path: &str, a: &Value, b: &Value) -> Option<String> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for key in x.keys().chain(y.keys()) {
                if let Some(d) = first_difference(&format!("{path}.{key}"), &x.get(key).cloned().unwrap_or(Value::Null), &y.get(key).cloned().unwrap_or(Value::Null)) {
                    return Some(d);
                }
            }
            None
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            x.iter().zip(y).enumerate().find_map(|(i, (p, q))| first_difference(&format!("{path}[{i}]"), p, q))
        }
        _ if a == b => None,
        _ => Some(format!("{path}: rust {a} native {b}")),
    }
}

fn ray_outcome(map: &TerrainMap, ray: &Value, pc: Precision) -> std::result::Result<(), String> {
    let native = &ray["native"];
    let has_secondary = ray["secondary"].as_bool().unwrap();
    let mut secondary = [UNWRITTEN_SECONDARY; 3];
    let outcome = intersect_71d840(
        map,
        words3(&ray["eye"]),
        words3(&ray["direction"]),
        has_secondary.then_some(&mut secondary),
        int(&ray["margin"]),
        pc,
    )
    .map_err(|e| format!("Rust error: {e}"))?;
    let mut point = [UNWRITTEN_POINT; 3];
    outcome.apply_to(&mut point);
    let writes: Vec<(String, u32, u32)> = outcome
        .writes
        .iter()
        .map(|w| (match w.target { Target::Point => "point", Target::Secondary => "secondary" }.to_string(), w.offset, w.value))
        .collect();
    let native_writes: Vec<(String, u32, u32)> = native["writes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| (w[0].as_str().unwrap().to_string(), word(&w[1]), word(&w[2])))
        .collect();
    let native_secondary = if native["secondary"].is_null() { None } else { Some(words3(&native["secondary"])) };
    let rust_secondary = has_secondary.then_some(secondary);
    let al = native["result"].as_u64().unwrap() != 0;
    if outcome.al != al || point != words3(&native["out"]) || rust_secondary != native_secondary || writes != native_writes {
        return Err(format!(
            "al {} vs {al}; point {point:x?} vs {:x?}; secondary {rust_secondary:x?} vs {native_secondary:x?}; writes {writes:x?} vs {native_writes:x?}",
            outcome.al,
            words3(&native["out"])
        ));
    }
    Ok(())
}

#[derive(Default, Debug)]
struct Tally {
    constructions: usize,
    construction_errors: usize,
    ops: usize,
    rays: usize,
}

/// Replays every native row: construction (or its native assertion/#DE),
/// every operation's full map/block state, then every native 71D840 ray on
/// the Rust-built map.
#[test]
fn native_terrain_map_constructors_and_71d840_end_to_end() {
    let doc = fixture();
    let specs = &doc["specs"];
    let rows = doc["rows"].as_array().unwrap();
    let mut tally: BTreeMap<String, Tally> = BTreeMap::new();
    let mut failures = Vec::new();
    for row in rows {
        let pc = precision(&row["precision"]);
        let name = row["case"].as_str().unwrap();
        let spec = &specs[name];
        let entry = tally.entry(row["precision"].as_str().unwrap().to_string()).or_default();
        let mut samples: Vec<u32> = spec["heightfield"]["heights"].as_array().unwrap().iter().map(word).collect();
        let mut tiles = lab_tiles(spec);
        let construct = &row["construct"];
        let built = TerrainMap::construct(&source_of(spec, &samples), pc);
        if construct["fault"].is_string() {
            match built {
                Err(e) if e.contains("#DE") && e.to_lowercase().contains(construct["eip"].as_str().unwrap()) => {
                    entry.construction_errors += 1
                }
                other => failures.push(format!("{name} {pc:?}: native #DE at {} but Rust {:?}", construct["eip"], other.map(|_| ()))),
            }
            continue;
        }
        let asserts = construct["asserts"].as_array().unwrap();
        if let Some(first) = asserts.first() {
            match built {
                Err(e) if e.contains(&format!("assertion {:#x}", word(first))) => entry.construction_errors += 1,
                other => failures.push(format!("{name} {pc:?}: native assertion {first} but Rust {:?}", other.map(|_| ()))),
            }
            continue;
        }
        let mut map = match built {
            Ok(map) => map,
            Err(e) => {
                failures.push(format!("{name} {pc:?}: construct {e}"));
                continue;
            }
        };
        // 0x721330's own words before 0x723E00 overwrites +8C/+90/+A0/+AC.
        let early = &construct["after_721330"];
        let limits = &spec["heightfield"]["limits"];
        assert_eq!((words3(&early["up"]), words3(&early["x_normal"]), words3(&early["y_normal"])), (UP, X_NORMAL, Y_NORMAL), "{name}");
        assert_eq!((word(&early["min"]), word(&early["max"])), (word(&limits[0]), word(&limits[1])), "{name}");
        assert_eq!((early["loaded"].as_u64(), early["recompute"].as_u64()), (Some(0), Some(0)), "{name}");
        if let Some(d) = first_difference("construct", &snapshot(&map, None), &native_state(&construct["state"])) {
            failures.push(format!("{name} {pc:?}: {d}"));
            continue;
        }
        entry.constructions += 1;
        let mut failed = false;
        for (k, op) in row["ops"].as_array().unwrap().iter().enumerate() {
            let o = &op["op"];
            let cell = o["cell"].as_u64().map(|c| c as usize);
            let result = match o["op"].as_str().unwrap() {
                "load" => map.load_720ad0(heights_of(spec, &samples), cell.unwrap(), pc),
                "release" => map.release_721030(cell.unwrap()),
                "update" => map.update_725340().map(|_| ()),
                "water" => {
                    let cell = cell.unwrap();
                    if map.blocks[cell].water_dirty { map.water_7208d0_without_7377f0(cell, &tiles) } else { Ok(()) }
                }
                "invalidate" => {
                    let [column, row] = map.blocks[cell.unwrap()].index;
                    map.invalidate_water_71ffa0(column, row)
                }
                "tiles" => {
                    for set in o["set"].as_array().unwrap() {
                        tiles.classes[set[0].as_u64().unwrap() as usize] = Some(int(&set[1]));
                    }
                    Ok(())
                }
                "heights" => {
                    for set in o["set"].as_array().unwrap() {
                        samples[set[0].as_u64().unwrap() as usize] = word(&set[1]);
                    }
                    Ok(())
                }
                other => panic!("op {other}"),
            };
            let vertices_of = (o["op"] == "load").then(|| cell.unwrap());
            let mismatch = match result {
                Err(e) => Some(format!("op {k} {o}: Rust error {e}")),
                Ok(()) => first_difference(&format!("op {k} {}", o["op"]), &snapshot(&map, vertices_of), &native_state(&op["state"])),
            };
            if let Some(d) = mismatch {
                failures.push(format!("{name} {pc:?}: {d}"));
                failed = true;
                break;
            }
            entry.ops += 1;
        }
        if failed {
            continue;
        }
        let rays = row["rays"].as_array().unwrap();
        if rays.is_empty() {
            continue;
        }
        // The laboratory history is complete: every block never loaded has
        // had +44 = 0 since 0x723E00.
        for cell in 0..map.blocks.len() {
            if map.blocks[cell].residency == Residency::Unreported {
                let [column, row] = map.blocks[cell].index;
                map.report_absent(column, row).unwrap();
            }
        }
        let resident: Vec<usize> = (0..map.blocks.len()).filter(|&c| matches!(map.blocks[c].residency, Residency::Resident(_))).collect();
        let native_resident: Vec<usize> = row["resident"].as_array().unwrap().iter().map(|c| c.as_u64().unwrap() as usize).collect();
        assert_eq!(resident, native_resident, "{name}");
        for (k, ray) in rays.iter().enumerate() {
            match ray_outcome(&map, ray, pc) {
                Ok(()) => entry.rays += 1,
                Err(e) => failures.push(format!("{name} {pc:?} ray {k}: {e}")),
            }
        }
    }
    eprintln!("TerrainMap replay per precision: {tally:#?}");
    assert!(failures.is_empty(), "{} mismatches, first: {:#?}", failures.len(), &failures[..failures.len().min(8)]);
    for (pc, t) in &tally {
        assert!(t.constructions > 0 && t.rays > 0 && t.construction_errors > 0, "{pc}: {t:?}");
    }
}

fn retail_lab(doc: &Value) -> (Value, Vec<u32>) {
    let spec = doc["specs"]["retail-16"].clone();
    let samples = spec["heightfield"]["heights"].as_array().unwrap().iter().map(word).collect();
    (spec, samples)
}

#[test]
fn unreported_residency_is_an_error_not_a_flat_block() {
    let doc = fixture();
    let (spec, samples) = retail_lab(&doc);
    let mut map = TerrainMap::construct(&source_of(&spec, &samples), Precision::Double53).unwrap();
    let eye = [0x4100_0000, 0x4100_0000, 0x4120_0000];
    let down = [0x3e80_0000, 0x3e00_0000, 0xbf80_0000];
    let error = intersect_71d840(&map, eye, down, None, 0, Precision::Double53).unwrap_err();
    assert!(error.contains("unreported"), "{error}");
    assert!(map.blocks[0].has_mesh().unwrap_err().contains("4312F0"));
    // One reported load still leaves the map extremes unknown.
    map.load_720ad0(heights_of(&spec, &samples), 0, Precision::Double53).unwrap();
    assert!(map.min_height().unwrap_err().contains("71A690"));
    for cell in 1..map.blocks.len() {
        let [column, row] = map.blocks[cell].index;
        map.report_absent(column, row).unwrap();
    }
    intersect_71d840(&map, eye, down, None, 0, Precision::Double53).unwrap();
    assert!(map.report_absent(0, 0).is_err(), "a resident mesh is removed only by 721030");
    assert!(map.release_block(1, 0).is_err(), "721030 needs +44");
    map.release_block(0, 0).unwrap();
    assert_eq!(map.blocks[0].has_mesh(), Ok(false));
}

#[test]
fn water_blocks_refuse_the_unported_7377f0_triangles() {
    let doc = fixture();
    let spec = &doc["specs"]["water-tiles"];
    let samples: Vec<u32> = spec["heightfield"]["heights"].as_array().unwrap().iter().map(word).collect();
    let mut map = TerrainMap::construct(&source_of(spec, &samples), Precision::Double53).unwrap();
    let tiles = lab_tiles(spec);
    map.water_7208d0_without_7377f0(0, &tiles).unwrap();
    assert_eq!(map.blocks[0].water(), Ok(1));
    assert!(map.blocks[0].water_index_count().unwrap_err().contains("7377F0"));
    // With 0x7377F0 the laboratory GMap has no geometry source: an error.
    assert!(map.water_7208d0(0, &tiles, Precision::Double53).unwrap_err().contains("7377F0"));
    // A centre tile beyond the GMap allocation is a native out-of-bounds read.
    let small = LabTiles { size: [4, 4], classes: vec![Some(0); 16] };
    assert!(map.water_7208d0_without_7377f0(15, &small).unwrap_err().contains("outside"));
    assert!(map.blocks[15].water().is_err());
    // Out-of-range samples are refused, not clamped.
    let short = Heights { samples: &samples[..samples.len() - 1], ..heights_of(spec, &samples) };
    let mut map = TerrainMap::construct(&source_of(spec, &samples), Precision::Double53).unwrap();
    assert!(map.load_720ad0(short, 15, Precision::Double53).unwrap_err().contains("outside"));
}

#[test]
fn constructor_domain_errors_name_the_native_site() {
    let doc = fixture();
    let (spec, samples) = retail_lab(&doc);
    let mut source = source_of(&spec, &samples);
    source.terrain_quad_size = 0;
    assert!(construct_error(&source).contains("0x72423C"));
    let mut source = source_of(&spec, &samples);
    source.extents_bits[0] = 0x7f80_0000;
    assert!(construct_error(&source).contains("nonfinite"));
    let mut source = source_of(&spec, &samples);
    source.terrain_block_size = 0;
    assert!(construct_error(&source).contains("0x95d"));
    let mut nan = samples.clone();
    nan[0] = 0x7fc0_0000;
    assert!(construct_error(&source_of(&spec, &nan)).contains("unordered"));
}
