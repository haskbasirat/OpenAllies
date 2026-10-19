//! Replay of research/phase8-orders-input/terrain-ray-masked: 0x71D840 with
//! binary32 overflow (+-infinity stores), real indefinite NaN results and
//! unordered compares, e.g. the first frame's corner casts while the
//! TerrainMap extremes are still +FLT_MAX/-FLT_MAX.
use crate::player_order_input::terrain_ray::{intersect_71d840, Target, Terrain, TerrainBlock};
use aa_numeric::Precision;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, String>;
const UNWRITTEN_POINT: u32 = 0xcdcd_cdcd; // probe_terrain_ray_v4.py point fill
const UNWRITTEN_SECONDARY: u32 = 0xabab_abab; // probe_terrain_ray_v4.py secondary fill

struct Water {
    count: u32,
    triangles: Vec<[u16; 3]>,
    vertices: Vec<[u32; 3]>,
}
struct Cell {
    owner: [[u32; 3]; 3],
    mesh: Option<Vec<[u32; 3]>>,
    min: u32,
    max: u32,
    index: [i32; 2],
    stride: i32,
    quads: i32,
    quad_size: i32,
    water: u8,
    water_mesh: Option<Water>,
}
struct Lab {
    up: [u32; 3],
    size: [i32; 2],
    blocks: [i32; 2],
    range: [u32; 2],
    block_size: i32,
    loaded: u8,
    cells: Vec<Cell>,
}
impl Terrain for Lab {
    type Block = Cell;
    fn up_normal(&self) -> Result<[u32; 3]> { Ok(self.up) }
    fn width(&self) -> Result<i32> { Ok(self.size[0]) }
    fn height(&self) -> Result<i32> { Ok(self.size[1]) }
    fn blocks_x(&self) -> Result<i32> { Ok(self.blocks[0]) }
    fn blocks_y(&self) -> Result<i32> { Ok(self.blocks[1]) }
    fn min_height(&self) -> Result<u32> { Ok(self.range[0]) }
    fn max_height(&self) -> Result<u32> { Ok(self.range[1]) }
    fn block_size(&self) -> Result<i32> { Ok(self.block_size) }
    fn loaded(&self) -> Result<u8> { Ok(self.loaded) }
    fn block(&self, cell: i32) -> Result<&Cell> {
        usize::try_from(cell).ok().and_then(|i| self.cells.get(i)).ok_or_else(|| format!("cell {cell} outside the cell array"))
    }
}
impl TerrainBlock for Cell {
    fn owner_up_normal(&self) -> Result<[u32; 3]> { Ok(self.owner[0]) }
    fn owner_x_normal(&self) -> Result<[u32; 3]> { Ok(self.owner[1]) }
    fn owner_y_normal(&self) -> Result<[u32; 3]> { Ok(self.owner[2]) }
    fn has_mesh(&self) -> Result<bool> { Ok(self.mesh.is_some()) }
    fn mesh_vertex(&self, index: i32) -> Result<[u32; 3]> {
        let mesh = self.mesh.as_ref().ok_or("lab cell has no mesh")?;
        usize::try_from(index).ok().and_then(|i| mesh.get(i).copied()).ok_or_else(|| format!("vertex {index} outside the mesh"))
    }
    fn min_height(&self) -> Result<u32> { Ok(self.min) }
    fn max_height(&self) -> Result<u32> { Ok(self.max) }
    fn water(&self) -> Result<u8> { Ok(self.water) }
    fn water_index_count(&self) -> Result<u32> { Ok(self.water_mesh.as_ref().ok_or("lab cell has no water mesh")?.count) }
    fn water_triangle(&self, triangle: u32) -> Result<[u16; 3]> {
        let water = self.water_mesh.as_ref().ok_or("lab cell has no water mesh")?;
        water.triangles.get(triangle as usize).copied().ok_or_else(|| format!("water triangle {triangle} outside the lab list"))
    }
    fn water_vertex(&self, index: u16) -> Result<[u32; 3]> {
        let water = self.water_mesh.as_ref().ok_or("lab cell has no water mesh")?;
        water.vertices.get(usize::from(index)).copied().ok_or_else(|| format!("water vertex {index} outside the lab list"))
    }
    fn index_x(&self) -> Result<i32> { Ok(self.index[0]) }
    fn index_y(&self) -> Result<i32> { Ok(self.index[1]) }
    fn vertex_stride(&self) -> Result<i32> { Ok(self.stride) }
    fn quads(&self) -> Result<i32> { Ok(self.quads) }
    fn quad_size(&self) -> Result<i32> { Ok(self.quad_size) }
}

fn word(value: &Value) -> u32 {
    value.as_u64().unwrap() as u32
}
fn words3(value: &Value) -> [u32; 3] {
    [word(&value[0]), word(&value[1]), word(&value[2])]
}
fn int(value: &Value) -> i32 {
    value.as_i64().unwrap() as i32
}
fn precision(value: &Value) -> Precision {
    match value.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}
fn special(word: u32) -> bool {
    (word >> 23) & 0xff == 0xff
}

fn lab(terrain: &Value) -> Lab {
    let owner = [words3(&terrain["up"]), words3(&terrain["x_normal"]), words3(&terrain["y_normal"])];
    let cells = terrain["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| Cell {
            owner,
            mesh: c["mesh"].as_array().map(|m| m.iter().map(words3).collect()),
            min: word(&c["min"]),
            max: word(&c["max"]),
            index: [int(&c["index"][0]), int(&c["index"][1])],
            stride: int(&c["stride"]),
            quads: int(&c["quads"]),
            quad_size: int(&c["quad_size"]),
            water: c["water"].as_u64().unwrap() as u8,
            water_mesh: c["water_mesh"].as_object().map(|w| Water {
                count: word(&w["count"]),
                triangles: w["triangles"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| [0, 1, 2].map(|k| t[k].as_u64().unwrap() as u16))
                    .collect(),
                vertices: w["vertices"].as_array().unwrap().iter().map(words3).collect(),
            }),
        })
        .collect();
    Lab {
        up: owner[0],
        size: [int(&terrain["size"][0]), int(&terrain["size"][1])],
        blocks: [int(&terrain["blocks"][0]), int(&terrain["blocks"][1])],
        range: [word(&terrain["min"]), word(&terrain["max"])],
        block_size: int(&terrain["block_size"]),
        loaded: terrain["loaded"].as_u64().unwrap() as u8,
        cells,
    }
}

/// One row: AL, primary words (unwritten stay 0xCDCDCDCD), secondary words
/// (unwritten stay 0xABABABAB) and the ordered store list.
fn replay(row: &Value, lab: &Lab) -> Result<bool> {
    let input = &row["input"];
    let native = &row["native"];
    let mut secondary = [UNWRITTEN_SECONDARY; 3];
    let has_secondary = input["secondary"].as_bool().unwrap();
    let outcome = intersect_71d840(
        lab,
        words3(&input["eye"]),
        words3(&input["direction"]),
        has_secondary.then_some(&mut secondary),
        int(&input["border"]),
        precision(&row["precision"]),
    )
    .map_err(|e| format!("Rust error: {e}"))?;
    let mut point = [UNWRITTEN_POINT; 3];
    outcome.apply_to(&mut point);
    let writes: Vec<(&str, u32, u32)> = outcome
        .writes
        .iter()
        .map(|w| (match w.target { Target::Point => "point", Target::Secondary => "secondary" }, w.offset, w.value))
        .collect();
    let native_writes: Vec<(&str, u32, u32)> =
        native["writes"].as_array().unwrap().iter().map(|w| (w[0].as_str().unwrap(), word(&w[1]), word(&w[2]))).collect();
    let native_secondary = (!native["secondary"].is_null()).then(|| words3(&native["secondary"]));
    let al = native["result"].as_u64().unwrap() != 0;
    if outcome.al != al || point != words3(&native["out"]) || has_secondary.then_some(secondary) != native_secondary || writes != native_writes {
        return Err(format!(
            "al {} vs {al}; point {point:x?} vs {:x?}; writes {writes:x?} vs {native_writes:x?}",
            outcome.al, native["out"]
        ));
    }
    Ok(writes.iter().any(|w| special(w.2)))
}

#[test]
fn native_71d840_masked_special_values_all_rows() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/terrain-ray-masked/native-terrain-ray-masked.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap();
    let labs: BTreeMap<&str, Lab> = doc["terrains"].as_object().unwrap().iter().map(|(k, v)| (k.as_str(), lab(v))).collect();
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2472);
    // precision -> (passed, rows, rows with a nonfinite store)
    let mut tally: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    let mut failures = Vec::new();
    for row in rows {
        assert!(row["native"]["fault"].is_null(), "no native fault is expected in this oracle");
        let entry = tally.entry(row["precision"].as_str().unwrap()).or_default();
        entry.1 += 1;
        match replay(row, &labs[row["input"]["terrain"].as_str().unwrap()]) {
            Ok(nonfinite) => {
                entry.0 += 1;
                entry.2 += usize::from(nonfinite);
            }
            Err(e) => failures.push(format!("{} {}: {e}", row["precision"], row["input"]["name"])),
        }
    }
    eprintln!("71D840 masked replay (passed, rows, nonfinite stores) per precision: {tally:?}");
    assert!(failures.is_empty(), "{} mismatches, first: {:#?}", failures.len(), &failures[..failures.len().min(8)]);
    assert!(tally.values().all(|&(ok, n, nonfinite)| ok == n && n == 824 && nonfinite > 300), "{tally:?}");
}

/// The Development 11 first frame: TerrainMap +8C/+90 and every cell still
/// +FLT_MAX/-FLT_MAX, no block mesh. A shallow corner ray's top-plane hit
/// stores +-infinity (no "overflows binary32" refusal), and the miss path
/// mixes the infinite plane hits into the real indefinite NaN.
#[test]
fn first_frame_extremes_store_infinity_and_indefinite() {
    let flt_max = 0x7f7f_ffff;
    let cell = |x: i32, y: i32| Cell {
        owner: [[0, 0, 0x3f80_0000], [0x3f80_0000, 0, 0], [0, 0x3f80_0000, 0]],
        mesh: None,
        min: flt_max,
        max: flt_max | 0x8000_0000,
        index: [x, y],
        stride: 5,
        quads: 4,
        quad_size: 2,
        water: 0,
        water_mesh: None,
    };
    let mut lab = Lab {
        up: [0, 0, 0x3f80_0000],
        size: [16, 16],
        blocks: [2, 2],
        range: [flt_max, flt_max | 0x8000_0000],
        block_size: 8,
        loaded: 0,
        cells: vec![cell(0, 0), cell(1, 0), cell(0, 1), cell(1, 1)],
    };
    // Eye (8, 8, 20), direction (1, 0.5, -0.25): |dx| > |dz|.
    let eye = [0x4100_0000, 0x4100_0000, 0x41a0_0000];
    let direction = [0x3f80_0000, 0x3f00_0000, 0xbe80_0000];
    for p in [Precision::Single24, Precision::Double53, Precision::Extended64] {
        // Unloaded map: the bottom-plane (+FLT_MAX) hit, behind the eye. At
        // PC24 t * dz rounds past FLT_MAX, so even Z stores +infinity
        // (native: Single24 0x7F800000, Double53/Extended64 0x7F7FFFFF).
        lab.loaded = 0;
        let unloaded = intersect_71d840(&lab, eye, direction, None, 0, p).unwrap();
        let z = if p == Precision::Single24 { 0x7f80_0000 } else { flt_max };
        assert_eq!(unloaded.point_words().unwrap(), [0xff80_0000, 0xff80_0000, z]);
        // Loaded: 71C930 rejects the infinite segment, the miss path stores
        // Z = 0 then the (inf * 1/2) + (1/2 * -inf) mixes.
        lab.loaded = 1;
        let loaded = intersect_71d840(&lab, eye, direction, None, 0, p).unwrap();
        assert_eq!(loaded.point_words().unwrap(), [0xffc0_0000, 0xffc0_0000, 0]);
        assert!(!loaded.al);
    }
}
