use super::*;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

const UNWRITTEN_POINT: u32 = 0xcdcd_cdcd; // probe_native_ray.py point fill
const UNWRITTEN_SECONDARY: u32 = 0xabab_abab; // probe_native_ray.py secondary fill

#[derive(Clone)]
struct LabWater {
    count: u32,
    triangles: Vec<[u16; 3]>,
    vertices: Vec<[u32; 3]>,
}
#[derive(Clone)]
struct LabBlock {
    owner: [[u32; 3]; 3],
    mesh: Option<Vec<[u32; 3]>>,
    min: u32,
    max: u32,
    index: [i32; 2],
    stride: i32,
    quads: i32,
    quad_size: i32,
    water: u8,
    water_mesh: Option<LabWater>,
}
struct Lab {
    up: [u32; 3],
    size: [i32; 2],
    blocks_xy: [i32; 2],
    height_range: [u32; 2],
    block_size: i32,
    loaded: u8,
    cells: Vec<LabBlock>,
}
impl Terrain for Lab {
    type Block = LabBlock;
    fn up_normal(&self) -> Result<[u32; 3]> { Ok(self.up) }
    fn width(&self) -> Result<i32> { Ok(self.size[0]) }
    fn height(&self) -> Result<i32> { Ok(self.size[1]) }
    fn blocks_x(&self) -> Result<i32> { Ok(self.blocks_xy[0]) }
    fn blocks_y(&self) -> Result<i32> { Ok(self.blocks_xy[1]) }
    fn min_height(&self) -> Result<u32> { Ok(self.height_range[0]) }
    fn max_height(&self) -> Result<u32> { Ok(self.height_range[1]) }
    fn block_size(&self) -> Result<i32> { Ok(self.block_size) }
    fn loaded(&self) -> Result<u8> { Ok(self.loaded) }
    fn block(&self, cell: i32) -> Result<&LabBlock> {
        usize::try_from(cell).ok().and_then(|i| self.cells.get(i)).ok_or_else(|| format!("lab cell {cell} is outside the cell array"))
    }
}
impl TerrainBlock for LabBlock {
    fn owner_up_normal(&self) -> Result<[u32; 3]> { Ok(self.owner[0]) }
    fn owner_x_normal(&self) -> Result<[u32; 3]> { Ok(self.owner[1]) }
    fn owner_y_normal(&self) -> Result<[u32; 3]> { Ok(self.owner[2]) }
    fn has_mesh(&self) -> Result<bool> { Ok(self.mesh.is_some()) }
    fn mesh_vertex(&self, index: i32) -> Result<[u32; 3]> {
        let mesh = self.mesh.as_ref().ok_or("lab block has no mesh")?;
        usize::try_from(index).ok().and_then(|i| mesh.get(i).copied()).ok_or_else(|| format!("lab vertex {index} outside the mesh"))
    }
    fn min_height(&self) -> Result<u32> { Ok(self.min) }
    fn max_height(&self) -> Result<u32> { Ok(self.max) }
    fn water(&self) -> Result<u8> { Ok(self.water) }
    fn water_index_count(&self) -> Result<u32> { Ok(self.water_mesh.as_ref().ok_or("lab block has no water mesh")?.count) }
    fn water_triangle(&self, triangle: u32) -> Result<[u16; 3]> {
        let water = self.water_mesh.as_ref().ok_or("lab block has no water mesh")?;
        water.triangles.get(triangle as usize).copied().ok_or_else(|| format!("lab water triangle {triangle} outside the list"))
    }
    fn water_vertex(&self, index: u16) -> Result<[u32; 3]> {
        let water = self.water_mesh.as_ref().ok_or("lab block has no water mesh")?;
        water.vertices.get(index as usize).copied().ok_or_else(|| format!("lab water vertex {index} outside the list"))
    }
    fn index_x(&self) -> Result<i32> { Ok(self.index[0]) }
    fn index_y(&self) -> Result<i32> { Ok(self.index[1]) }
    fn vertex_stride(&self) -> Result<i32> { Ok(self.stride) }
    fn quads(&self) -> Result<i32> { Ok(self.quads) }
    fn quad_size(&self) -> Result<i32> { Ok(self.quad_size) }
}

fn f2b(value: &Value) -> u32 {
    (value.as_f64().unwrap() as f32).to_bits()
}
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap() as u32
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
fn binary32_min_max(values: &[f64]) -> [u32; 2] {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    [(min as f32).to_bits(), (max as f32).to_bits()]
}

/// probe_native_ray.py's 16x16 laboratory: 2x2 blocks of 8 units, 2-unit
/// quads, 9x9 heights, native-winding 5x5 vertex arrays, owner normals
/// (0,0,1), (1,0,0), (0,1,0), no water byte.
fn lab_v3(input: &Value) -> Lab {
    let heights: Vec<f64> = input["heights"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    let owner = [[0, 0, 0x3f80_0000], [0x3f80_0000, 0, 0], [0, 0x3f80_0000, 0]];
    let mesh = input["mesh"].as_bool().unwrap();
    let mut cells = Vec::new();
    for y in 0..2 {
        for x in 0..2 {
            let mut vertices = Vec::new();
            let mut block_heights = Vec::new();
            for yy in 0..5 {
                for xx in 0..5 {
                    let h = heights[(y * 4 + yy) * 9 + x * 4 + xx];
                    block_heights.push(h);
                    vertices.push([((x * 8 + xx * 2) as f32).to_bits(), ((y * 8 + yy * 2) as f32).to_bits(), (h as f32).to_bits()]);
                }
            }
            let [min, max] = binary32_min_max(&block_heights);
            cells.push(LabBlock {
                owner,
                mesh: mesh.then_some(vertices),
                min,
                max,
                index: [x as i32, y as i32],
                stride: 5,
                quads: 4,
                quad_size: 2,
                water: 0,
                water_mesh: None,
            });
        }
    }
    Lab {
        up: owner[0],
        size: [16, 16],
        blocks_xy: [2, 2],
        height_range: binary32_min_max(&heights),
        block_size: 8,
        loaded: input["loaded"].as_bool().unwrap() as u8,
        cells,
    }
}

/// Explicit terrain words written by research/phase8-orders-input/terrain-ray.
fn lab_explicit(terrain: &Value) -> Lab {
    let owner = [words3(&terrain["up"]), words3(&terrain["x_normal"]), words3(&terrain["y_normal"])];
    let int = |v: &Value| v.as_i64().unwrap() as i32;
    let cells = terrain["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| LabBlock {
            owner,
            mesh: c["mesh"].as_array().map(|m| m.iter().map(words3).collect()),
            min: word(&c["min"]),
            max: word(&c["max"]),
            index: [int(&c["index"][0]), int(&c["index"][1])],
            stride: int(&c["stride"]),
            quads: int(&c["quads"]),
            quad_size: int(&c["quad_size"]),
            water: c["water"].as_u64().unwrap() as u8,
            water_mesh: c["water_mesh"].as_object().map(|w| LabWater {
                count: word(&w["count"]),
                triangles: w["triangles"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|t| {
                        let t: Vec<u16> = t.as_array().unwrap().iter().map(|i| i.as_u64().unwrap() as u16).collect();
                        t.try_into().unwrap()
                    })
                    .collect(),
                vertices: w["vertices"].as_array().unwrap().iter().map(words3).collect(),
            }),
        })
        .collect();
    Lab {
        up: owner[0],
        size: [int(&terrain["size"][0]), int(&terrain["size"][1])],
        blocks_xy: [int(&terrain["blocks"][0]), int(&terrain["blocks"][1])],
        height_range: [word(&terrain["min"]), word(&terrain["max"])],
        block_size: int(&terrain["block_size"]),
        loaded: terrain["loaded"].as_u64().unwrap() as u8,
        cells,
    }
}

/// Replays one native row; returns a mismatch description.
fn replay(row: &Value, lab: &Lab, eye: [u32; 3], direction: [u32; 3]) -> std::result::Result<(), String> {
    let input = &row["input"];
    let native = &row["native"];
    let pc = precision(&row["precision"]);
    let mut secondary = [UNWRITTEN_SECONDARY; 3];
    let has_secondary = input["secondary"].as_bool().unwrap();
    let margin = input["border"].as_i64().unwrap() as i32;
    let outcome = intersect_71d840(lab, eye, direction, has_secondary.then_some(&mut secondary), margin, pc)
        .map_err(|e| format!("Rust error: {e}"))?;
    let mut applied = [UNWRITTEN_POINT; 3];
    outcome.apply_to(&mut applied);
    let point = applied.to_vec();
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
    let native_secondary = if native["secondary"].is_null() { None } else { Some(words3(&native["secondary"]).to_vec()) };
    let rust_secondary = has_secondary.then(|| secondary.to_vec());
    let al = native["result"].as_u64().unwrap() != 0;
    if outcome.al != al || point != words3(&native["out"]).to_vec() || rust_secondary != native_secondary || writes != native_writes {
        return Err(format!(
            "al {} vs {al}; point {point:x?} vs {:x?}; secondary {rust_secondary:x?} vs {native_secondary:x?}; writes {writes:x?} vs {native_writes:x?}",
            outcome.al, native["out"]
        ));
    }
    Ok(())
}

fn fixture(relative: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative);
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

/// Replays every row; `build` returns the lab plus eye and direction words.
fn replay_all(doc: &Value, build: impl Fn(&Value) -> (Lab, [u32; 3], [u32; 3]), expected_rows: usize) -> BTreeMap<String, (usize, usize)> {
    let rows = doc["rows"].as_array().unwrap();
    assert_eq!(rows.len(), expected_rows);
    let mut passed: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut failures = Vec::new();
    for row in rows {
        let input = &row["input"];
        let (lab, eye, direction) = build(input);
        let entry = passed.entry(row["precision"].as_str().unwrap().to_string()).or_default();
        entry.1 += 1;
        let result = if row["native"]["fault"].is_string() {
            // The native run faulted reading below the isolated cell array:
            // the Rust port must refuse the same undefined cell read.
            let mut secondary = [UNWRITTEN_SECONDARY; 3];
            let has_secondary = input["secondary"].as_bool().unwrap();
            match intersect_71d840(&lab, eye, direction, has_secondary.then_some(&mut secondary), input["border"].as_i64().unwrap() as i32, precision(&row["precision"])) {
                Err(e) if e.contains("outside the cell array") => Ok(()),
                other => Err(format!("native fault {} but Rust {other:?}", row["native"]["eip"])),
            }
        } else {
            replay(row, &lab, eye, direction)
        };
        match result {
            Ok(()) => entry.0 += 1,
            Err(e) => failures.push(format!("{} {}: {e}", row["precision"], input["name"])),
        }
    }
    eprintln!("71D840 replay passed per precision (passed, rows): {passed:?}");
    assert!(failures.is_empty(), "{} mismatches, first: {:#?}", failures.len(), &failures[..failures.len().min(8)]);
    passed
}

#[test]
fn native_71d840_v3_oracle_all_rows() {
    let doc = fixture("research/skirmish-20261017-terrain-ray/native-ray-v3.json");
    let passed = replay_all(
        &doc,
        |input| {
            let f3 = |v: &Value| [f2b(&v[0]), f2b(&v[1]), f2b(&v[2])];
            (lab_v3(input), f3(&input["eye"]), f3(&input["direction"]))
        },
        480,
    );
    assert!(passed.values().all(|&(ok, rows)| ok == rows && rows == 160), "{passed:?}");
}

#[test]
fn native_71d840_v4_walker_oracle_all_rows() {
    let doc = fixture("research/phase8-orders-input/terrain-ray/native-terrain-ray-v4.json");
    let terrains = &doc["terrains"];
    let passed = replay_all(
        &doc,
        |input| (lab_explicit(&terrains[input["terrain"].as_str().unwrap()]), words3(&input["eye"]), words3(&input["direction"])),
        3276,
    );
    assert!(passed.values().all(|&(ok, rows)| ok == rows && rows == 1092), "{passed:?}");
}

#[test]
fn truncation_860680_matches_shift_semantics() {
    for (value, expected) in [(0.0f32, 0), (-0.0, 0), (0.75, 0), (-0.75, 0), (1.0, 1), (7.99, 7), (-8.5, -8), (2147483648.0, i32::MIN), (-2147483648.0, i32::MIN)] {
        assert_eq!(truncate_860680(value.to_bits()), expected, "{value}");
    }
}

#[test]
fn parallel_ray_is_an_error_not_a_point() {
    let doc = fixture("research/skirmish-20261017-terrain-ray/native-ray-v3.json");
    let lab = lab_v3(&doc["rows"][0]["input"]);
    let flat = [0x3f80_0000, 0, 0];
    let error = intersect_71d840(&lab, [0x4100_0000; 3], flat, None, 0, Precision::Double53).unwrap_err();
    assert!(error.contains("unwritten"), "{error}");
}
