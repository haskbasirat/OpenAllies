use super::*;
use crate::player_order_input::terrain_map::Source;
use crate::player_order_input::terrain_ray::Target;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

const UNWRITTEN_POINT: u32 = 0xcdcd_cdcd;
const UNWRITTEN_SECONDARY: u32 = 0xabab_abab;

fn fixture() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/phase8-orders-input/terrain-visibility/native-terrain-visibility.json");
    serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}
fn word(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn int(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(word).collect()
}
fn words3(v: &Value) -> [u32; 3] {
    words(v).try_into().unwrap()
}
fn precision(v: &Value) -> Precision {
    match v.as_str().unwrap() {
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

/// Records the view-cell hook calls in native order.
#[derive(Default)]
struct Recorder {
    calls: Vec<Vec<u64>>,
}
impl ViewCellHooks for Recorder {
    fn sweep_42fad0(&mut self, index: [i32; 2], block_size_a0: i32, state_b8: u32) -> Result<()> {
        assert!(block_size_a0 > 0);
        self.calls.push(vec![0x42fad0, index[0] as u64, index[1] as u64, u64::from(state_b8)]);
        Ok(())
    }
    fn traverse_42f670(&mut self) -> Result<()> {
        self.calls.push(vec![0x42f670]);
        Ok(())
    }
}
fn native_view_calls(frame: &Value) -> Vec<Vec<u64>> {
    frame["view_calls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let c = c.as_array().unwrap();
            let name = u64::from_str_radix(c[0].as_str().unwrap(), 16).unwrap();
            std::iter::once(name).chain(c[1..].iter().map(|v| v.as_u64().unwrap())).collect()
        })
        .collect()
}

fn camera(frame: &Value) -> CameraWords {
    let cam: HashMap<u32, u32> = frame["camera"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (u32::from_str_radix(k.trim_start_matches("0x"), 16).unwrap(), word(v)))
        .collect();
    let w3 = |o: u32| [cam[&o], cam[&(o + 4)], cam[&(o + 8)]];
    let mut planes = [[0; 4]; 6];
    for (i, p) in planes.iter_mut().enumerate() {
        let o = 0x19c + 16 * i as u32;
        *p = [cam[&o], cam[&(o + 4)], cam[&(o + 8)], cam[&(o + 12)]];
    }
    CameraWords { eye: w3(0x90), direction: w3(0x114), planes, rays: [w3(0x2d4), w3(0x2e0), w3(0x2ec), w3(0x2f8)] }
}

fn nonfinite(word: u32) -> bool {
    (word >> 23) & 0xff == 0xff
}

fn ray_outcome(map: &TerrainMap, ray: &Value, pc: Precision) -> std::result::Result<(), String> {
    let native = &ray["native"];
    let has_secondary = ray["secondary"].as_bool().unwrap();
    let mut secondary = [UNWRITTEN_SECONDARY; 3];
    let outcome = intersect_71d840(map, words3(&ray["eye"]), words3(&ray["direction"]), has_secondary.then_some(&mut secondary), int(&ray["margin"]), pc)
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
    if outcome.al != (native["result"].as_u64().unwrap() != 0)
        || point != words3(&native["out"])
        || has_secondary.then_some(secondary) != native_secondary
        || writes != native_writes
    {
        return Err(format!("point {point:x?} vs {:x?}; writes {writes:x?} vs {native_writes:x?}", native["out"]));
    }
    Ok(())
}

#[derive(Default, Debug)]
struct Tally {
    frames: usize,
    view_calls: usize,
    footprints: usize,
    footprint_errors: usize,
    rays: usize,
}

/// Replays every native frame of 0x4312F0 (+ 0x725340's 725429 branch): all
/// block request/state/previous/active/count/mesh/water fields, the active
/// list order, the eight hit points, the VWorld+144 footprint, the map
/// extremes; then the end-to-end 71D840 rays on the resulting residency.
#[test]
fn native_4312f0_visibility_frames_and_rays() {
    let doc = fixture();
    let specs = &doc["specs"];
    let mut tally: BTreeMap<String, Tally> = BTreeMap::new();
    let mut failures = Vec::new();
    for row in doc["rows"].as_array().unwrap() {
        let pc = precision(&row["precision"]);
        let name = row["case"].as_str().unwrap();
        let spec = &specs[name];
        let entry = tally.entry(row["precision"].as_str().unwrap().to_string()).or_default();
        let samples = words(&spec["heightfield"]["heights"]);
        let h = &spec["heightfield"];
        let heights = Heights { columns: int(&h["columns"]), rows: int(&h["rows"]), step_bits: word(&h["step"]), samples: &samples };
        let table: HashMap<(u32, u32, u32), u32> =
            row["ground"].as_array().unwrap().iter().map(|g| ((word(&g[0]), word(&g[1]), word(&g[2])), word(&g[3]))).collect();
        let ground = |xy: [u32; 2], flags: u32| -> Result<Binary> {
            let v = table.get(&(xy[0], xy[1], flags)).ok_or_else(|| format!("4F0E50 query {xy:x?} {flags:#x} not in the oracle"))?;
            b(*v)
        };
        let source = Source {
            extents_bits: [word(&spec["extents"][0]), word(&spec["extents"][1])],
            gtile_bits: word(&spec["gtile"]),
            terrain_block_size: int(&spec["block"]),
            terrain_quad_size: int(&spec["quad"]),
            heights,
            ground: &ground,
        };
        let mut map = TerrainMap::construct(&source, pc).unwrap();
        let spheres: Vec<Vec<u32>> = row["spheres"].as_array().unwrap().iter().map(words).collect();
        for (cell, sphere) in spheres.iter().enumerate() {
            if map.sphere_e8(cell).to_vec() != *sphere {
                failures.push(format!("{name} {pc:?} 71AFD0 sphere {cell}: {:x?} vs {sphere:x?}", map.sphere_e8(cell)));
            }
        }
        let tiles = LabTiles {
            size: [int(&spec["tiles"]["size"][0]), int(&spec["tiles"]["size"][1])],
            classes: spec["tiles"]["classes"].as_array().unwrap().iter().map(|c| c.as_i64().map(|c| c as i32)).collect(),
        };
        let limits = [word(&spec["base_limits"][0]), word(&spec["base_limits"][1])];
        let mut vis = TerrainVisibility::for_map(&mut map).unwrap();
        let mut ok = true;
        for (k, frame) in row["frames"].as_array().unwrap().iter().enumerate() {
            let cam = camera(frame);
            let mut views = Recorder::default();
            if let Err(e) = vis.pass_4312f0(&mut map, &cam, limits, heights, &tiles, &mut views, pc) {
                failures.push(format!("{name} {pc:?} frame {k}: Rust error {e}"));
                ok = false;
                break;
            }
            let (min, max, ..) = map.extremes();
            let after = &frame["after_pass"];
            let mut diffs = Vec::new();
            if views.calls != native_view_calls(frame) {
                diffs.push(format!("view-cell hook calls {:x?} vs {:x?}", views.calls, native_view_calls(frame)));
            }
            entry.view_calls += views.calls.len();
            if (min, max) != (word(&after["min"]), word(&after["max"])) {
                diffs.push(format!("after-pass extremes {min:x} {max:x} vs {after}"));
            }
            map.update_725340().unwrap();
            let (min, max, min_holder, max_holder, recompute, dirty) = map.extremes();
            let m = &frame["map"];
            let holder = |v: &Value| v.as_u64().map(|c| c as usize);
            if (min, max, min_holder, max_holder, u64::from(recompute), u64::from(dirty))
                != (word(&m["min"]), word(&m["max"]), holder(&m["min_holder"]), holder(&m["max_holder"]), m["recompute"].as_u64().unwrap(), m["dirty"].as_u64().unwrap())
            {
                diffs.push(format!("map {min:x} {max:x} {min_holder:?} {max_holder:?} {recompute} {dirty} vs {m}"));
            }
            for (cell, nb) in frame["blocks"].as_array().unwrap().iter().enumerate() {
                let v = vis.block(cell).unwrap();
                let rust = (
                    v.request_bc,
                    v.state_b8,
                    v.previous_b4,
                    u64::from(v.active_140),
                    v.requests_4 as u32,
                    map.resident(cell),
                    u64::from(map.water_byte(cell)),
                    u64::from(map.water_dirty_14f(cell)),
                    u64::from(map.refresh_flags(cell).0),
                );
                let native = (
                    word(&nb["bc"]),
                    word(&nb["b8"]),
                    word(&nb["b4"]),
                    nb["active"].as_u64().unwrap(),
                    word(&nb["requests"]),
                    nb["mesh"].as_bool().unwrap(),
                    nb["water"].as_u64().unwrap(),
                    nb["water_dirty"].as_u64().unwrap(),
                    nb["refresh"].as_u64().unwrap(),
                );
                if rust != native {
                    diffs.push(format!("block {cell}: {rust:?} vs {native:?}"));
                }
            }
            let active: Vec<usize> = frame["active"].as_array().unwrap().iter().map(|c| c.as_u64().unwrap() as usize).collect();
            if vis.active() != active.as_slice() {
                diffs.push(format!("active {:?} vs {active:?}", vis.active()));
            }
            let (max_hits, min_hits) = vis.hits();
            let rust_hits: Vec<Option<u32>> = max_hits
                .iter()
                .chain(min_hits.iter())
                .flat_map(|h| match h {
                    Some(p) => p.map(Some),
                    None => [None; 3],
                })
                .collect();
            let native_hits = words(&frame["hits"]);
            // vis+28..+84: max hits at +28/+34/+40/+4C, min at +58/+64/+70/+7C.
            let native_ordered: Vec<u32> = (0..4).flat_map(|k| native_hits[3 * k..3 * k + 3].to_vec()).chain((0..4).flat_map(|k| native_hits[12 + 3 * k..15 + 3 * k].to_vec())).collect();
            if rust_hits.iter().zip(&native_ordered).any(|(r, n)| *r != Some(*n)) {
                diffs.push(format!("hits {rust_hits:x?} vs {native_ordered:x?}"));
            }
            let native_footprint = words(&frame["footprint"]);
            match vis.footprint() {
                Some(Ok(points)) => {
                    entry.footprints += 1;
                    for (i, w) in points.iter().flatten().enumerate() {
                        if let Some(w) = w {
                            if *w != native_footprint[i] {
                                diffs.push(format!("footprint word {i}: {w:x} vs {:x}", native_footprint[i]));
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    // The Rust 71D840 refuses binary32 overflow where the
                    // native stores an infinity (map extremes still +-FLT_MAX).
                    entry.footprint_errors += 1;
                    if !(e.contains("overflow") && native_footprint.iter().any(|w| nonfinite(*w))) {
                        diffs.push(format!("footprint error {e} with native {native_footprint:x?}"));
                    }
                }
                None => diffs.push("footprint never computed".into()),
            }
            if !diffs.is_empty() {
                failures.push(format!("{name} {pc:?} frame {k}: {}", diffs[..diffs.len().min(4)].join("; ")));
                ok = false;
                break;
            }
            entry.frames += 1;
        }
        if !ok {
            continue;
        }
        for (k, ray) in row["rays"].as_array().unwrap().iter().enumerate() {
            match ray_outcome(&map, ray, pc) {
                Ok(()) => entry.rays += 1,
                Err(e) => failures.push(format!("{name} {pc:?} ray {k}: {e}")),
            }
        }
    }
    eprintln!("4312F0 replay per precision: {tally:#?}");
    assert!(failures.is_empty(), "{} mismatches, first: {:#?}", failures.len(), &failures[..failures.len().min(6)]);
    for (pc, t) in &tally {
        assert!(t.frames > 0 && t.rays > 0, "{pc}: {t:?}");
    }
}

#[test]
fn segment_and_sphere_helpers_follow_the_native_comparisons() {
    // 42F330: parallel segments (zero binary32 denominator) never cross.
    let p = Precision::Double53;
    let one = 0x3f80_0000;
    assert_eq!(segment_42f330([0, 0], [0x4000_0000, 0], [0, one], [0x4000_0000, one], p), Ok(None));
    // Endpoints are inclusive on both segments.
    let hit = segment_42f330([0, 0], [0x4000_0000, 0], [0x4000_0000, 0xbf80_0000], [0x4000_0000, one], p).unwrap().unwrap();
    assert_eq!(hit, ([0x4000_0000, 0], one));
    // 42ED10: a sphere exactly on the -r boundary is outside.
    assert!(outside_42ed10([0, 0, 0, one], [one, 0, 0, one], p).unwrap());
    assert!(!outside_42ed10([0x3400_0000, 0, 0, one], [one, 0, 0, one], p).unwrap());
}
