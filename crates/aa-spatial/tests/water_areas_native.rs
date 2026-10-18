//! Replays research/phase8-movement/native-water-areas.json: original x86
//! execution (Unicorn, PC24/53/64) of the water-area flood fill 0x539E30 /
//! 0x4F4050 (goal vtable 0xA36058, solver 0xA36048, FIFO 0xA3602C), the visit
//! stamp 0x552440, free id 0x552C90, tile lookup 0x42A530, area paint
//! 0x552110 / 0x552150 (vertex bytes 0x550F20 / 0x5507C0), the stroke
//! 0x5559F0 and KKC_GMap::PaintTerrain 0x555C10 with their brush seams, plus
//! floods on original RMP maps. Every tile stamp/link/area/direction, record
//! count, vertex byte, solver word, diagnostic and seam call must match.
use aa_heightfield::{Heightfield, binding::{GridLayout, LoadedHeightfield}};
use aa_mapworld::{
    MapArchive, SourceIdentity,
    archive::sha256,
    header::MapHeader,
    terrain::{ResolvedVariation, TerrainChunk, TerrainGridLayout},
};
use aa_numeric::Precision;
use aa_spatial::water_areas::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
/// Read-only original game folder (the one holding AA.exe and `reconstruction/`).
fn game_root() -> PathBuf {
    PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"))
}
fn document() -> Value {
    let path = root().join("research/phase8-movement/native-water-areas.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("fixture")).expect("json")
}
fn u(v: &Value) -> u32 {
    v.as_u64().expect("u32") as u32
}
fn i(v: &Value) -> i32 {
    v.as_i64().expect("i32") as i32
}
fn opt(v: &Value) -> Option<u32> {
    v.as_u64().map(|x| x as u32)
}
fn bits2(v: &Value) -> [u32; 2] {
    [u(&v[0]), u(&v[1])]
}
fn precision(v: &Value) -> Precision {
    serde_json::from_value(v.clone()).expect("precision")
}

struct State {
    map: WaterMap,
    heights: Heightfield,
    vertices: VertexAreas,
    solver: FloodSolver,
}
impl State {
    fn world(&mut self) -> WaterWorld<'_> {
        WaterWorld {
            map: &mut self.map,
            heights: &self.heights,
            vertices: &mut self.vertices,
            solver: &mut self.solver,
        }
    }
}

fn build(input: &Value) -> State {
    let m = &input["map"];
    let tiles = m["tiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| WaterTile {
            terrain_class: t[0].as_i64().map(|c| c as i32),
            link: opt(&t[1]),
            visit_stamp: u(&t[2]),
            area: u(&t[3]) as u8,
            reached_from: u(&t[4]) as u8,
        })
        .collect();
    let areas = m["areas"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (!a.is_null()).then(|| AreaRecord {
                id: u(&a[0]) as u8,
                count: i(&a[1]),
            })
        })
        .collect();
    let f = &input["field"];
    let words = |v: &Value| v.as_array().unwrap().iter().map(u).collect::<Vec<_>>();
    State {
        map: WaterMap {
            width: i(&m["width"]),
            height: i(&m["height"]),
            coordinate_shift: u(&m["shift"]),
            cell_size_bits: u(&m["cell"]),
            extent_bits: bits2(&m["extent"]),
            visit_stamp: u(&m["stamp"]),
            tiles,
            areas,
        },
        heights: Heightfield {
            width: i(&f["width"]),
            height: i(&f["height"]),
            coordinate_shift: u(&f["shift"]),
            cell_size_bits: u(&f["cell"]),
            index_bias: i(&f["bias"]),
            height_bits: words(&f["heights"]),
        },
        vertices: VertexAreas {
            width: i(&f["width"]),
            height: i(&f["height"]),
            coordinate_shift: u(&f["shift"]),
            bytes: words(&f["vertices"]).into_iter().map(|b| b as u8).collect(),
        },
        solver: FloodSolver {
            busy: input["solver"]["busy"].as_bool().unwrap(),
            head: opt(&input["solver"]["head"]),
            tail: opt(&input["solver"]["tail"]),
        },
    }
}

fn diagnostic_json(d: &Diagnostic) -> Value {
    match d {
        Diagnostic::SpreadBlocked {
            area,
            x_bits,
            y_bits,
            neighbour_area,
            height_bits,
            base_bits,
        } => json!({"kind": "spread_blocked", "area": area, "x": x_bits, "y": y_bits,
            "neighbour_area": neighbour_area, "height": height_bits, "base": base_bits}),
        Diagnostic::NoFreeArea => json!({"kind": "no_free_area"}),
    }
}

fn tiles_json(map: &WaterMap) -> Vec<Value> {
    map.tiles
        .iter()
        .map(|t| json!([t.link, t.visit_stamp, t.area, t.reached_from]))
        .collect()
}
fn areas_json(map: &WaterMap) -> Vec<Value> {
    map.areas
        .iter()
        .map(|a| match a {
            None => Value::Null,
            Some(r) => json!([r.id, r.count as u32]),
        })
        .collect()
}
fn solver_json(s: &FloodSolver) -> Value {
    json!({"busy": s.busy, "head": s.head, "tail": s.tail})
}

/// Seam recorder for 0x5526C0 / 0x559FB0 / 0x5528F0 / 0x551140 / 0x551B40.
struct Recorder {
    calls: Vec<Value>,
    a1: u32,
    a4: Value,
    a5: u32,
    a7: u32,
    point: [u32; 2],
    surface: bool,
}
impl StrokeWorld for Recorder {
    fn stamp(&mut self, _: &mut WaterMap, batch: &StrokeBatch, point_bits: [u32; 2]) -> Result<()> {
        self.calls.push(json!({"op": "stamp", "brush": 1, "a1": self.a1, "point": point_bits,
            "a4": self.a4, "batch_stamp": batch.visit_stamp, "batch_flag": batch.flag, "a7": self.a7}));
        Ok(())
    }
    fn commit(&mut self, _: &mut WaterMap, batch: &StrokeBatch) -> Result<()> {
        self.calls.push(json!({"op": "commit", "a5": self.a5, "batch_stamp": batch.visit_stamp,
            "batch_flag": batch.flag}));
        Ok(())
    }
}
impl PaintTerrainStages for Recorder {
    fn land_height(&mut self, _: &mut WaterMap) -> Result<()> {
        self.calls.push(json!({"op": "height_paint", "point": self.point}));
        Ok(())
    }
    fn water_margin(&mut self, margin_bits: u32) -> Result<()> {
        self.calls.push(json!({"op": "water_margin", "margin": margin_bits}));
        Ok(())
    }
    fn water_surface(&mut self, _: &mut WaterMap, area: u8) -> Result<bool> {
        self.calls.push(json!({"op": "water_surface", "area": area, "point": self.point}));
        Ok(self.surface)
    }
    fn water_bed(&mut self, _: &mut WaterMap) -> Result<()> {
        self.calls.push(json!({"op": "height_paint", "point": self.point}));
        Ok(())
    }
}

fn brush(v: &Value) -> StrokeBrush {
    StrokeBrush {
        kind: i(&v["kind"]),
        size_bits: u(&v["w04"]),
        extent_bits: [u(&v["w10"]), u(&v["w14"])],
    }
}

/// Run one fixture case; returns (result, calls, diagnostics).
fn run(case: &Value, state: &mut State) -> (std::result::Result<Value, Error>, Vec<Value>, Vec<Diagnostic>) {
    let input = &case["input"];
    let p = precision(&case["precision"]);
    let mut diagnostics = Vec::new();
    let mut calls = Vec::new();
    let result = match input["op"].as_str().unwrap() {
        "flood" => flood_fill_water_area(
            &mut state.world(),
            [i(&input["start"][0]), i(&input["start"][1])],
            u(&input["area"]) as usize,
            p,
            &mut diagnostics,
        )
        .map(|r| json!(r)),
        "stamp" => advance_visit_stamp(&mut state.map).map(|r| json!(r)),
        "allocate" => free_area_id(&state.map).map(|r| json!(r)),
        "tile_at" => tile_at_point(&state.map, bits2(&input["point"])).map(|r| json!(r)),
        "paint" => {
            let (record, tile) = (u(&input["area"]) as usize, u(&input["tile"]) as usize);
            if input["paint_op"] == "add" {
                add_tile_to_area(&mut state.world(), record, tile, p)
            } else {
                remove_tile_from_area(&mut state.world(), record, tile, p)
            }
            .map(|_| Value::Null)
        }
        "stroke" => {
            let s = &input["stroke"];
            let mut rec = Recorder {
                calls: Vec::new(),
                a1: u(&s["a1"]),
                a4: s["a4"].clone(),
                a5: u(&s["a5"]),
                a7: u(&s["a7"]),
                point: [0; 2],
                surface: false,
            };
            let request = StrokeRequest {
                brush: brush(&s["brush"]),
                from_bits: bits2(&s["from"]),
                to_bits: bits2(&s["to"]),
                area: u(&s["area"]) as u8,
                batch_flag: u(&s["a8"]) as u8,
            };
            let r = stroke_and_flood(&mut state.world(), &mut rec, &request, p, &mut diagnostics);
            calls = rec.calls;
            r.map(|_| Value::Null)
        }
        "paint_terrain" => {
            let s = &input["paint"];
            let mut rec = Recorder {
                calls: Vec::new(),
                a1: u(&s["a1"]),
                a4: json!("variation"),
                a5: u(&s["a5"]),
                a7: u(&s["a7"]),
                point: bits2(&s["point"]),
                surface: case["behaviour"]["surface"].as_i64() == Some(1),
            };
            let request = PaintTerrainRequest {
                brush: brush(&s["brush"]),
                point_bits: bits2(&s["point"]),
                end_bits: bits2(&s["end"]),
                terrain_class: i(&s["class"]),
                batch_flag: u(&s["a8"]) as u8,
            };
            let mut stroke = rec_clone(&rec);
            let r = paint_terrain(&mut state.world(), &mut stroke, &mut rec, &request, p, &mut diagnostics);
            // Every stage call precedes 0x5559F0, which issues the stroke calls.
            calls = rec.calls;
            calls.append(&mut stroke.calls);
            r.map(|o| json!(format!("{o:?}")))
        }
        op => panic!("unknown op {op}"),
    };
    (result, calls, diagnostics)
}

fn rec_clone(r: &Recorder) -> Recorder {
    Recorder {
        calls: Vec::new(),
        a1: r.a1,
        a4: r.a4.clone(),
        a5: r.a5,
        a7: r.a7,
        point: r.point,
        surface: r.surface,
    }
}

fn check_error(name: &str, out: &Value, result: &std::result::Result<Value, Error>) {
    if out.get("error").is_some() {
        assert_eq!(out["error"], "null_dereference", "{name}");
        assert!(
            matches!(result, Err(Error::NullAreaRecord(_) | Error::NullTerrain(_))),
            "{name}: native null dereference, Rust {result:?}"
        );
    } else {
        let r = result.as_ref().unwrap_or_else(|e| panic!("{name}: {e}"));
        // Void native routines (paint, stroke, PaintTerrain) record null.
        if !out["result"].is_null() {
            assert_eq!(r, &out["result"], "{name}");
        }
    }
}

#[test]
fn water_areas_native_fixtures() {
    let doc = document();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut outcomes: BTreeMap<String, Value> = BTreeMap::new();
    for case in doc["cases"].as_array().unwrap() {
        if case.get("real_map").is_some() {
            continue;
        }
        let name = format!("{}/{}", case["name"].as_str().unwrap(), case["precision"].as_str().unwrap());
        let out = &case["output"];
        let mut state = build(&case["input"]);
        let (result, calls, diagnostics) = run(case, &mut state);
        check_error(&name, out, &result);
        if case["input"]["op"] == "paint_terrain" {
            // The native routine returns nothing; keep the Rust outcome
            // consistent across precisions and with the recorded calls.
            let outcome = result.as_ref().unwrap().clone();
            let key = case["name"].as_str().unwrap().to_string();
            assert_eq!(outcomes.entry(key).or_insert(outcome.clone()), &outcome, "{name}");
        }
        assert_eq!(state.map.visit_stamp, u(&out["stamp"]), "{name} stamp");
        assert_eq!(tiles_json(&state.map), out["tiles"].as_array().unwrap().clone(), "{name} tiles");
        assert_eq!(areas_json(&state.map), out["areas"].as_array().unwrap().clone(), "{name} areas");
        let vertices: Vec<u32> = out["vertices"].as_array().unwrap().iter().map(u).collect();
        assert_eq!(
            state.vertices.bytes.iter().map(|b| u32::from(*b)).collect::<Vec<_>>(),
            vertices,
            "{name} vertices"
        );
        assert_eq!(solver_json(&state.solver), out["solver"], "{name} solver");
        let diags: Vec<Value> = diagnostics.iter().map(diagnostic_json).collect();
        assert_eq!(diags, out["diagnostics"].as_array().unwrap().clone(), "{name} diagnostics");
        assert_eq!(calls, out["calls"].as_array().unwrap().clone(), "{name} calls");
        *counts.entry(case["input"]["op"].as_str().unwrap().to_string()).or_default() += 1;
    }
    let expected: BTreeMap<String, usize> = serde_json::from_value(doc["generated_counts"].clone()).unwrap();
    assert_eq!(counts, expected);
}

// ---- original maps -----------------------------------------------------------

struct Loaded {
    map: WaterMap,
    heights: Heightfield,
    vertices: VertexAreas,
}

fn load_real_map(relative: &str, map_sha256: &str) -> Loaded {
    let root = root();
    let data = game_root().join("reconstruction/assets/original/Data");
    let types: Value = serde_json::from_slice(
        &std::fs::read(root.join("research/engine-baseline/terrain-variation-metadata.json")).unwrap(),
    )
    .unwrap();
    let metadata: Value = serde_json::from_slice(
        &std::fs::read(root.join("research/phase6-terrain/terrain-render-metadata.json")).unwrap(),
    )
    .unwrap();
    let resolve = |id: &str| -> Option<ResolvedVariation> {
        let r = metadata["variants"].get(id)?;
        let native = &types["variations"][r["canonical"].as_str().unwrap()];
        let source = SourceIdentity::of(&std::fs::read(data.join(r["path"].as_str().unwrap())).unwrap());
        assert_eq!(source.sha256, r["sha256"]);
        Some(ResolvedVariation {
            id_utf16: id.encode_utf16().collect(),
            native_type: native["native_type"].as_u64().unwrap() as u32,
            passability_bits: u8::from(r["land"].as_bool().unwrap()) | u8::from(r["water"].as_bool().unwrap()) << 1,
            source,
        })
    };
    let bytes = std::fs::read(data.join(relative)).expect("original RMP");
    assert_eq!(sha256(&bytes), map_sha256, "{relative}");
    let archive = MapArchive::from_bytes(bytes).unwrap();
    let header = MapHeader::decode(&archive.decode_chunk("MAP_HEADER", u64::MAX).unwrap()).unwrap();
    let chunk = TerrainChunk::decode(&archive.decode_chunk("TERRAIN", u64::MAX).unwrap()).unwrap();
    let extents = header.creator.world_extents_bits;
    let layout = TerrainGridLayout::from_world_extents(extents, 1).unwrap();
    let palette: Vec<_> = chunk
        .palette_utf16
        .iter()
        .map(|s| resolve(&String::from_utf16(s).unwrap()))
        .collect();
    let biome = String::from_utf16(&chunk.biome_utf16).unwrap();
    let initial = metadata["biome_defaults"][&biome]["variation_id"].as_str().unwrap();
    let bound = chunk
        .bind_with_native_fallback(layout, &palette, resolve(initial).unwrap())
        .unwrap();
    let tiles = bound
        .cells
        .iter()
        .map(|c| WaterTile {
            // Bound flags carry the variation's native type in bits 4..6.
            terrain_class: Some(((c.flags20 >> 4) & 7) as i32),
            link: None,
            visit_stamp: 0,
            area: c.byte_2c,
            reached_from: 0,
        })
        .collect::<Vec<WaterTile>>();
    let mut areas = vec![None];
    for id in 1..AREA_TABLE_LEN {
        let count = tiles.iter().filter(|t| usize::from(t.area) == id).count() as i32;
        areas.push(Some(AreaRecord { id: id as u8, count }));
    }
    let height_layout = GridLayout::from_world_extents(extents, 1).unwrap();
    let loaded = LoadedHeightfield::decode(
        height_layout,
        &archive.decode_chunk("HEIGHT_MAP", u64::MAX).unwrap().bytes,
    )
    .unwrap();
    let field = loaded.state.field;
    Loaded {
        map: WaterMap {
            width: layout.width,
            height: layout.height,
            coordinate_shift: layout.coordinate_shift,
            cell_size_bits: layout.cell_size_bits,
            extent_bits: extents,
            visit_stamp: 1,
            tiles,
            areas,
        },
        vertices: VertexAreas {
            width: field.width,
            height: field.height,
            coordinate_shift: field.coordinate_shift,
            bytes: loaded.state.sample_flags,
        },
        heights: field,
    }
}

fn input_digest(l: &Loaded) -> String {
    let mut bytes = Vec::new();
    for t in &l.map.tiles {
        bytes.extend(t.terrain_class.unwrap_or(-1).to_le_bytes());
        bytes.push(t.area);
    }
    for h in &l.heights.height_bits {
        bytes.extend(h.to_le_bytes());
    }
    bytes.extend(&l.vertices.bytes);
    sha256(&bytes)
}

#[test]
#[ignore = "requires original local maps; run with AA_GAME_ROOT and --include-ignored"]
fn water_areas_original_map_floods() {
    let doc = document();
    let mut maps: BTreeMap<String, Loaded> = BTreeMap::new();
    let mut ran = 0;
    for case in doc["cases"].as_array().unwrap() {
        let Some(relative) = case["real_map"].as_str() else {
            continue;
        };
        let name = format!("{}/{}", case["name"].as_str().unwrap(), case["precision"].as_str().unwrap());
        let loaded = maps
            .entry(relative.to_string())
            .or_insert_with(|| load_real_map(relative, case["map_sha256"].as_str().unwrap()));
        assert_eq!(input_digest(loaded), case["input_sha256"], "{name} input");
        let input = &case["input"];
        let mut state = State {
            map: loaded.map.clone(),
            heights: loaded.heights.clone(),
            vertices: loaded.vertices.clone(),
            solver: FloodSolver::default(),
        };
        state.map.visit_stamp = u(&input["stamp"]);
        let mut diagnostics = Vec::new();
        let r = flood_fill_water_area(
            &mut state.world(),
            [i(&input["start"][0]), i(&input["start"][1])],
            u(&input["area"]) as usize,
            precision(&case["precision"]),
            &mut diagnostics,
        )
        .unwrap();
        let out = &case["output"];
        assert_eq!(json!(r), out["result"], "{name}");
        assert_eq!(state.map.visit_stamp, u(&out["stamp"]), "{name}");
        let before = &loaded.map;
        let changed: Vec<Value> = state
            .map
            .tiles
            .iter()
            .zip(&before.tiles)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(k, (t, _))| json!([k, t.link, t.visit_stamp, t.area, t.reached_from]))
            .collect();
        assert_eq!(changed, out["tiles_changed"].as_array().unwrap().clone(), "{name} tiles");
        let vchanged: Vec<Value> = state
            .vertices
            .bytes
            .iter()
            .zip(&loaded.vertices.bytes)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(k, (v, _))| json!([k, v]))
            .collect();
        assert_eq!(vchanged, out["vertices_changed"].as_array().unwrap().clone(), "{name} vertices");
        let achanged: Vec<Value> = state
            .map
            .areas
            .iter()
            .zip(&before.areas)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(k, (a, _))| {
                let a = a.unwrap();
                json!([k, a.id, a.count as u32])
            })
            .collect();
        assert_eq!(achanged, out["areas_changed"].as_array().unwrap().clone(), "{name} areas");
        assert_eq!(solver_json(&state.solver), out["solver"], "{name} solver");
        let diags: Vec<Value> = diagnostics.iter().map(diagnostic_json).collect();
        assert_eq!(diags, out["diagnostics"].as_array().unwrap().clone(), "{name} diagnostics");
        ran += 1;
    }
    assert_eq!(ran, doc["real_case_count"].as_u64().unwrap() as usize);
}
