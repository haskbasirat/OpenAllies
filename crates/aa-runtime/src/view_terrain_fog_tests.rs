//! Replays research/phase8-fog/native-terrain-fog.json (the original code
//! under Unicorn) through the ported kernels, with the oracle's seam order.
use super::object::{self, Apparent, GMapWords, ThingReads, ViewReads, ViewWords};
use super::*;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::OnceLock;

fn fixture() -> &'static Value {
    static F: OnceLock<Value> = OnceLock::new();
    F.get_or_init(|| {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-fog/native-terrain-fog.json");
        serde_json::from_slice(&std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
    })
}
fn precision(v: &Value) -> Precision {
    match v.as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn i(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn bytes(v: &Value) -> Vec<u8> {
    v.as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as u8).collect()
}
/// Every case of every precision row of a fixture section.
fn cases(key: &str) -> Vec<(Precision, &'static Value, &'static Value)> {
    let mut out = Vec::new();
    for row in fixture()[key].as_array().unwrap() {
        let p = precision(&row["precision"]);
        for c in row["cases"].as_array().unwrap() {
            out.push((p, &c["input"], &c["output"]));
        }
    }
    assert!(!out.is_empty(), "{key}");
    out
}
fn lab_cells(flags: &[u8]) -> Vec<Cell> {
    flags.iter().map(|&f| Cell { snapshot_0: 0, tile_4: Some(0), block_8: Some(0), flags_c: f }).collect()
}

#[test]
fn darken_71be70_matches_native() {
    for (p, c, o) in cases("be70") {
        let mut res = u(&c["res"]) as u8;
        darken_71be70(&mut res, u(&c["neighbor"]) as u8, u(&c["d2"]), p).unwrap();
        assert_eq!(res, u(&o[0]) as u8, "{c} {p:?}");
        assert_eq!(u(&o[1]), 0xcd_cdcd, "only the result byte is written");
    }
}

#[test]
fn sample_71bef0_matches_native() {
    for (p, c, o) in cases("bef0") {
        let cells = lab_cells(&bytes(&c["flags"]));
        let view = CellView { d0: i(&c["d0"]), cells: Some(&cells), b285d4: u(&c["b285d4"]) as u8 };
        for (pt, want) in c["points"].as_array().unwrap().iter().zip(o.as_array().unwrap()) {
            assert_eq!(sample_71bef0(&view, u(&pt[0]), u(&pt[1]), p).unwrap(), u(want) as u8, "{pt} {p:?}");
        }
    }
}

#[test]
fn blend_71c290_matches_native() {
    for (p, c, o) in cases("c290") {
        let size = i(&c["n"]);
        let px = bytes(&c["pixels"]);
        let mut block = BlockFog::from_723960([0, 0]);
        block.cells = Some(lab_cells(&bytes(&c["flags"])));
        block.grid = Some(Grid {
            n_12c: size,
            w130: size - 1,
            w134: 0,
            step_13c: u(&c["step_bits"]),
            pixels: px.chunks(4).map(|q| [q[0], q[1], q[2], q[3]]).collect(),
            prev_10c: bytes(&c["prev"]),
            target_118: bytes(&c["target"]),
            revisions: 0,
        });
        let g = Globals { fog_debug_b403c8: u(&c["b403c8"]) as u8, reblend_b403c9: 0, explored_b285d4: u(&c["b285d4"]) as u8 };
        block.blend_71c290(i(&c["d0"]), u(&c["t"]), u(&c["rebuild"]) != 0, &g, p).unwrap();
        let grid = block.grid.unwrap();
        assert_eq!(grid.pixels.concat(), bytes(&o["pixels"]), "{p:?}");
        assert_eq!(grid.prev_10c, bytes(&o["prev"]));
        assert_eq!(grid.target_118, bytes(&o["target"]));
        assert_eq!(u(&o["revision"]), u(&c["revision"]) + grid.revisions);
    }
}

#[test]
fn grid_720ad0_matches_native() {
    for (p, c, o) in cases("sizing") {
        let g = grid_720ad0(i(&c["d0"]), i(&c["uvar0"]), i(&c["avar61"]), p);
        let n12c = i(&o["n12c"]);
        if n12c == 1 {
            assert!(g.is_err(), "a 1x1 grid divides by zero");
            continue;
        }
        let g = g.unwrap();
        assert_eq!((g.w134, g.n_12c, g.w130, g.step_13c), (i(&o["w134"]), n12c, i(&o["w130"]), u(&o["step13c"])), "{c} {p:?}");
        assert!(g.pixels.iter().all(|q| *q == [0; 4]) && g.prev_10c.iter().all(|v| *v == 0));
    }
}

#[test]
fn cells_720670_match_native() {
    for (_, c, o) in cases("cells") {
        let (w, h, d0) = (i(&c["w"]), i(&c["h"]), i(&c["d0"]));
        let layout = CellLayout { tiles: [w, h], blocks_x: w / d0, quads_a8: d0 * 2, quads_per_gtile_b4: 2 };
        let mut uses = Vec::new();
        let cells = cells_720670(d0, [i(&c["col"]), i(&c["row"])], layout, &mut |t| {
            uses.push(t);
            Ok(t as u64)
        })
        .unwrap();
        let want = o["cells"].as_array().unwrap();
        assert_eq!(cells.len(), want.len());
        for (cell, want) in cells.iter().zip(want) {
            assert_eq!(cell.snapshot_0, want[0].as_u64().unwrap());
            assert_eq!(cell.tile_4, want[1].as_u64().map(|v| v as usize));
            assert_eq!(cell.block_8, want[2].as_u64().map(|v| v as usize));
            assert_eq!(cell.flags_c, u(&want[3]) as u8);
        }
        // 71FEE0 (terrain texture use) follows the same in-map order.
        let calls: Vec<usize> = o["calls"].as_array().unwrap().iter().map(|c| c[1].as_u64().unwrap() as usize).collect();
        assert_eq!(calls, uses);
    }
}

/// The seams of the oracle as reads; calls are recorded in the oracle's form.
struct Lab<'a> {
    c: &'a Value,
    calls: Vec<Value>,
    looked_up: bool,
    fog: Vec<u32>,
    kingdom: Option<&'static str>,
    variation_changed: bool,
    e4: u32,
    gate: bool,
    god: bool,
    fade: u32,
    kingdoms: Vec<(&'static str, u32, u32)>,
}
impl<'a> Lab<'a> {
    fn new(c: &'a Value) -> Self {
        Self { c, calls: Vec::new(), looked_up: false, fog: Vec::new(), kingdom: None, variation_changed: false, e4: 1, gate: false, god: false, fade: 0, kingdoms: Vec::new() }
    }
    fn byte(&self, key: &str) -> u8 {
        self.c.get(key).map_or(0, |v| u(v) as u8)
    }
    fn lookups(&mut self) {
        if !self.looked_up {
            self.looked_up = true;
            self.calls.push(json!(["4B2E60", "DebugObjectBlocking", 1]));
            self.calls.push(json!(["4B2E60", "DebugWaterBlocking", 1]));
        }
    }
    fn k(&self, name: &str) -> (u32, u32) {
        let k = self.kingdoms.iter().find(|k| k.0 == name).unwrap();
        (k.1, k.2)
    }
}
impl Reads for Lab<'_> {
    type Kingdom = &'static str;
    fn debug_gate_40fca0(&mut self) -> Result<bool> {
        self.calls.push(json!(["40FCA0", 2, 0]));
        Ok(self.gate)
    }
    fn debug_object_blocking(&mut self) -> Result<u8> {
        self.lookups();
        Ok(self.byte("dob"))
    }
    fn debug_water_blocking(&mut self) -> Result<u8> {
        self.lookups();
        Ok(self.byte("dwb"))
    }
    fn uvar_byte(&mut self, slot: Uvar) -> Result<u8> {
        Ok(match slot {
            Uvar::ShowBlocking22 => self.byte("u22"),
            Uvar::SettlementDebug25 => self.byte("u25"),
            Uvar::GuardDebug27 => self.byte("u27"),
            Uvar::ControlDebug29 => self.byte("u29"),
            _ => 0,
        })
    }
    fn tile_blocking_20(&mut self, _tile: usize) -> Result<u32> {
        Ok(u(&self.c["blocking_20"]))
    }
    fn tile_zone_set_4(&mut self, _tile: usize) -> Result<bool> {
        Ok(self.byte("zone_set") != 0)
    }
    fn zone_debug_54ef80(&mut self, _tile: usize, bit: u8) -> Result<bool> {
        self.calls.push(json!(["54EF80", bit]));
        Ok(u(&self.c["zone"][bit.to_string()]) != 0)
    }
    fn god_40cfa0(&mut self) -> Result<bool> {
        self.calls.push(json!(["40CFA0"]));
        Ok(self.god)
    }
    fn kingdom_5c0e50(&mut self) -> Result<Option<&'static str>> {
        self.calls.push(json!(["5C0E50"]));
        Ok(self.kingdom)
    }
    fn fog_of_war(&mut self) -> Result<u32> {
        Ok(u(&self.c["fogofwar"]))
    }
    fn tile_fog_14(&mut self, tile: usize) -> Result<u32> {
        Ok(self.fog[tile])
    }
    fn kingdom_190(&mut self, k: &'static str) -> Result<u32> {
        Ok(self.k(k).0)
    }
    fn kingdom_228(&mut self, k: &'static str) -> Result<u32> {
        Ok(self.k(k).1)
    }
    fn tile_variation_0(&mut self, tile: usize) -> Result<u64> {
        Ok(if self.variation_changed && tile == 0 { u64::MAX } else { tile as u64 })
    }
    fn fade_time_65(&mut self) -> Result<u32> {
        Ok(self.fade)
    }
    fn application_state_e4(&mut self) -> Result<Option<u32>> {
        Ok(Some(self.e4))
    }
}

#[test]
fn cell_7203f0_matches_native() {
    for (_, c, o) in cases("cell7203f0") {
        let mut lab = Lab::new(c);
        lab.gate = u(&c["gate"]) != 0;
        lab.god = u(&c["god"]) != 0;
        lab.fog = vec![u(&c["fog_word"]); 16];
        lab.kingdom = c["kingdom"].as_str().map(|_| "k");
        lab.kingdoms = vec![("k", u(&c["k190"]), u(&c["k228"]))];
        lab.variation_changed = c["variation_changed"].as_bool().unwrap();
        let in_map = c["in_map"].as_bool().unwrap();
        let mut block = BlockFog::from_723960([0, 0]);
        block.debug_14b = u(&c["b14b"]) as u8;
        block.fog_14c = u(&c["b14c"]) as u8;
        let cell = Cell { snapshot_0: 0, tile_4: in_map.then_some(0), block_8: in_map.then_some(0), flags_c: u(&c["flags_in"]) as u8 };
        block.cells = Some(vec![cell]);
        let r = block.cell_7203f0(0, &mut lab);
        let calls = o["calls"].as_array().unwrap();
        let changed = calls.iter().any(|c| c[0] == "71FFA0");
        assert_eq!(r.is_err(), changed, "{c}: {r:?}");
        let want: Vec<Value> = calls.iter().filter(|c| c[0] != "71FFA0").cloned().collect();
        assert_eq!(lab.calls, want, "{c}");
        let cell = block.cells.as_ref().unwrap()[0];
        assert_eq!((cell.flags_c, block.debug_14b, block.fog_14c), (u(&o["flags"]) as u8, u(&o["b14b"]) as u8, u(&o["b14c"]) as u8), "{c}");
    }
}

fn block_state(b: &BlockFog) -> Value {
    let g = b.grid.as_ref().unwrap();
    json!({"fade_108": b.fade_108, "loaded_141": b.loaded_141, "fading_142": b.fading_142, "debug_14b": b.debug_14b,
        "fog_14c": b.fog_14c, "flags": b.cells.as_ref().unwrap().iter().map(|c| c.flags_c).collect::<Vec<_>>(),
        "pixels": g.pixels.concat(), "prev": g.prev_10c, "target": g.target_118, "revision": g.revisions})
}

#[test]
fn frames_7208d0_725340_match_native() {
    for (p, c, o) in cases("frames") {
        let d0 = 2;
        let n = d0 * i(&c["n_per_gtile"]);
        let layout = CellLayout { tiles: [4, 2], blocks_x: 2, quads_a8: 4, quads_per_gtile_b4: 2 };
        let step = ((d0 as f64) / f64::from(n - 1)) as f32;
        let mut blocks: Vec<BlockFog> = (0..2)
            .map(|col| {
                let mut b = BlockFog::from_723960([col, 0]);
                b.cells = Some(cells_720670(d0, [col, 0], layout, &mut |t| Ok(t as u64)).unwrap());
                let mut g = grid_720ad0(d0, n / d0, 1, p).unwrap();
                g.step_13c = step.to_bits();
                b.grid = Some(g);
                b.loaded_141 = 1;
                b
            })
            .collect();
        let mut globals = Globals::from_fresh_process_image();
        let mut clock = MapFog::<&'static str>::from_721330();
        let mut lab = Lab::new(c);
        lab.god = u(&c["god"]) != 0;
        lab.fade = u(&c["fade_65"]);
        lab.kingdoms = vec![("k", u(&c["k190"]), u(&c["k228"])), ("j", 0, u(&c["j228"]))];
        for (step, want) in c["steps"].as_array().unwrap().iter().zip(o.as_array().unwrap()) {
            lab.calls.clear();
            lab.kingdom = step["kingdom"].as_str().map(|k| if k == "k" { "k" } else { "j" });
            if step["kind"] == "7208D0" {
                lab.fog = step["fog"].as_array().unwrap().iter().map(u).collect();
                lab.e4 = u(&step["app_e4"]);
                let block = &mut blocks[u(&step["block"]) as usize];
                block.update_7208d0(2, d0, &globals, clock.delta_13c, &mut lab, p).unwrap();
                let calls: Vec<Value> = lab.calls.iter().filter(|c| c[0] != "40FCA0").cloned().collect();
                assert_eq!(&calls, want["calls"].as_array().unwrap(), "{p:?}");
                let mut got = block_state(block);
                let rev = u(&got["revision"]);
                got["revision"] = json!(rev);
                assert_eq!(got, want["block"], "seed {} {p:?} step {step}", c["seed"]);
            } else {
                let k = step["kingdom"].as_str().map(|k| if k == "k" { "k" } else { "j" });
                clock.update_725340(&mut globals, Ok(u(&step["time_ms"])), Ok(k), u(&c["gray_64"]), Ok(u(&step["fog_debug"]) as u8), p).unwrap();
                let got = json!({"b403c8": globals.fog_debug_b403c8, "b403c9": globals.reblend_b403c9,
                    "b285d4": globals.explored_b285d4, "w138": clock.seconds_138, "w13c": clock.delta_13c,
                    "kingdom_140": clock.kingdom_140});
                for key in ["b403c8", "b403c9", "b285d4", "w138", "w13c", "kingdom_140"] {
                    assert_eq!(got[key], want[key], "{key} seed {} {p:?}", c["seed"]);
                }
                assert_eq!(want["calls"], json!([["59ABA0"], ["5C0E50"], ["72EF70"]]), "no other 725340 branch is reached");
            }
        }
    }
}

/// View/Thing seams of the object cases.
struct Obj<'a> {
    c: &'a Value,
    calls: Vec<Value>,
}
impl Obj<'_> {
    fn b(&self, key: &str) -> bool {
        self.c.get(key).is_some_and(|v| v.as_u64() == Some(1))
    }
}
impl ViewReads for Obj<'_> {
    type Kingdom = &'static str;
    fn doodads_45bbd0(&mut self) -> Result<bool> {
        self.calls.push(json!(["45BBD0"]));
        Ok(self.b("doodads"))
    }
    fn god_40cfa0(&mut self) -> Result<bool> {
        self.calls.push(json!(["40CFA0"]));
        Ok(self.b("god"))
    }
    fn local_player_5c0e10(&mut self) -> Result<bool> {
        self.calls.push(json!(["5C0E10"]));
        Ok(self.b("player"))
    }
    fn kingdom_5c0e50(&mut self) -> Result<Option<&'static str>> {
        self.calls.push(json!(["5C0E50"]));
        Ok(self.c["kingdom"].as_str().map(|_| "k"))
    }
    fn thing_visible_5c(&mut self, k: &'static str, a: bool) -> Result<bool> {
        self.calls.push(json!(["vt5c", k, u8::from(a)]));
        Ok(self.b("visible_5c"))
    }
    fn thing_remembered_60(&mut self, k: &'static str, a: bool) -> Result<bool> {
        self.calls.push(json!(["vt60", k, u8::from(a)]));
        Ok(self.b("remembered_60"))
    }
    fn gmap(&mut self) -> Result<GMapWords> {
        Ok(GMapWords { shift: 1, extent_bits: [8.0f32.to_bits(), 4.0f32.to_bits()], width: 4, tiles: 8 })
    }
    fn fog_of_war(&mut self) -> Result<u32> {
        Ok(u(&self.c["fogofwar"]))
    }
    fn tile_fog_14(&mut self, tile: usize) -> Result<u32> {
        Ok(u(&self.c["fog"][tile]))
    }
    fn kingdom_190(&mut self, _k: &'static str) -> Result<u32> {
        Ok(u(&self.c["k190"]))
    }
    fn kingdom_228(&mut self, _k: &'static str) -> Result<u32> {
        Ok(u(&self.c["k228"]))
    }
}

#[test]
fn view_virtuals_match_native() {
    for (key, thing) in [("view", false), ("thing_view", true)] {
        for (_, c, o) in cases(key) {
            let v = ViewWords { thing_14: thing, flags_54: u(&c["flags_54"]), position_38: Some([c.get("x").map_or(0, u), c.get("y").map_or(0, u)]) };
            let a = u(&c["a"]) != 0;
            let mut r = Obj { c, calls: Vec::new() };
            assert_eq!(u8::from(object::visible_45bf20(&mut r, &v, a).unwrap()), u(&o["vt50"]) as u8, "{c}");
            assert_eq!(json!(r.calls), o["vt50_calls"], "{c}");
            let mut r = Obj { c, calls: Vec::new() };
            assert_eq!(u8::from(object::remembered_45bd40(&mut r, &v, a).unwrap()), u(&o["vt54"]) as u8, "{c}");
            assert_eq!(json!(r.calls), o["vt54_calls"], "{c}");
        }
    }
}

#[test]
fn doodads_45bbd0_matches_native() {
    for (_, c, o) in cases("doodads") {
        let calls_cell = std::cell::RefCell::new(Vec::new());
        let r = object::doodads_45bbd0(
            &mut || {
                calls_cell.borrow_mut().push(json!(["4B2E60", "PerformanceDisableDoodads", 0]));
                Ok(u(&c["uvar"]) as u8)
            },
            &mut || Ok((u(&c["thing"]) != 0).then(|| u(&c["flags_8"]))),
            &mut || Ok(u(&c["category"])),
            &mut || {
                calls_cell.borrow_mut().push(json!(["data_40"]));
                Ok(u(&c["doodad_40"]) != 0)
            },
        )
        .unwrap();
        assert_eq!(u8::from(r), u(&o["doodads"]) as u8, "{c}");
        assert_eq!(json!(calls_cell.into_inner()), o["calls"], "{c}");
    }
}

/// Thing seams; `seam_350/430` replace the direct calls like the oracle.
struct Thing<'a> {
    c: &'a Value,
    calls: Vec<Value>,
    seam_350: bool,
    seam_430: bool,
}
impl Thing<'_> {
    fn n(&self, key: &str) -> u32 {
        self.c.get(key).map_or(0, u)
    }
    fn k(k: Option<&'static str>) -> Value {
        k.map_or(Value::Null, |k| json!(k))
    }
}
impl ThingReads for Thing<'_> {
    type Kingdom = &'static str;
    type Record = u32;
    fn visible_5c(&mut self, k: Option<&'static str>, a: bool) -> Result<bool> {
        self.calls.push(json!(["vt5c", Self::k(k), u8::from(a)]));
        Ok(self.n("visible_5c") != 0)
    }
    fn remembered_60(&mut self, k: Option<&'static str>, a: bool) -> Result<bool> {
        self.calls.push(json!(["vt60", Self::k(k), u8::from(a)]));
        Ok(self.n("remembered_60") != 0)
    }
    fn explored_74(&mut self, k: Option<&'static str>, a: bool) -> Result<bool> {
        self.calls.push(json!(["vt74", Self::k(k), u8::from(a)]));
        Ok(self.n("explored_74") != 0)
    }
    fn flags_8(&mut self) -> Result<u32> {
        Ok(self.n("flags_8"))
    }
    fn data_actor_38(&mut self) -> Result<bool> {
        self.calls.push(json!(["data_38"]));
        Ok(self.n("actor_38") != 0)
    }
    fn data_doodad_40(&mut self) -> Result<bool> {
        self.calls.push(json!(["data_40"]));
        Ok(self.n("doodad_40") != 0)
    }
    fn data_mask_174(&mut self) -> Result<u32> {
        Ok(self.n("mask_174"))
    }
    fn data_16d(&mut self) -> Result<u8> {
        Ok(self.n("byte_16d") as u8)
    }
    fn vt_a8(&mut self) -> Result<bool> {
        self.calls.push(json!(["vt_a8"]));
        Ok(self.n("vt_a8") != 0)
    }
    fn vt_8c(&mut self) -> Result<bool> {
        self.calls.push(json!(["vt_8c"]));
        Ok(self.n("vt_8c") != 0)
    }
    fn kingdom_190(&mut self, _k: &'static str) -> Result<u32> {
        Ok(self.n("k190"))
    }
    fn kingdom_18c(&mut self, _k: &'static str) -> Result<u32> {
        Ok(self.n("k18c"))
    }
    fn memory_slot_5c(&mut self, index: u32) -> Result<Option<u32>> {
        Ok((u(&self.c["slots"][index as usize]) != 0).then_some(index))
    }
    fn call_4ea350(&mut self) -> Result<bool> {
        if !self.seam_350 {
            return object::potential_4ea350(self);
        }
        self.calls.push(json!(["4EA350"]));
        Ok(self.n("potential_4ea350") != 0)
    }
    fn call_4ea430(&mut self, k: Option<&'static str>) -> Result<Option<u32>> {
        if !self.seam_430 {
            return object::seen_4ea430(self, k);
        }
        self.calls.push(json!(["4EA430", Self::k(k)]));
        Ok(if self.c.get("record").is_some_and(|r| u(r) == 0) { None } else { Some(0) })
    }
}

#[test]
fn thing_leaves_match_native() {
    for (_, c, o) in cases("remembered") {
        let mut r = Thing { c, calls: Vec::new(), seam_350: true, seam_430: true };
        let k = c["kingdom"].as_str().map(|_| "k");
        let got = object::remembered_4eab20(&mut r, k, u(&c["a"]) != 0).unwrap();
        assert_eq!(u8::from(got), u(&o["remembered"]) as u8, "{c}");
        assert_eq!(json!(r.calls), o["calls"], "{c}");
    }
    for (_, c, o) in cases("potential") {
        let mut r = Thing { c, calls: Vec::new(), seam_350: false, seam_430: false };
        assert_eq!(u8::from(object::potential_4ea350(&mut r).unwrap()), u(&o["potential"]) as u8, "{c}");
        assert_eq!(json!(r.calls), o["calls"], "{c}");
    }
    for (_, c, o) in cases("seen") {
        let mut r = Thing { c, calls: Vec::new(), seam_350: false, seam_430: false };
        let k = c["kingdom"].as_str().map(|_| "k");
        let got = object::seen_4ea430(&mut r, k).unwrap();
        assert_eq!(got.map(u64::from), o["record"].as_u64(), "{c}");
        assert_eq!(json!(r.calls), o["calls"], "{c}");
    }
    for (_, c, o) in cases("apparent") {
        let k = c["kingdom"].as_str().map(|_| "k");
        for (key, current, remembered) in [("aspect_4eae20", "current", "remembered_18"), ("data_4eacc0", "thing_data", "remembered_10")] {
            let mut r = Thing { c, calls: Vec::new(), seam_350: true, seam_430: true };
            let got = match object::apparent_4eae20(&mut r, k).unwrap() {
                Apparent::Current => {
                    if key == "aspect_4eae20" {
                        r.calls.push(json!(["4EAAA0"]));
                    }
                    current
                }
                Apparent::Remembered(_) => remembered,
                Apparent::Default => "default",
            };
            assert_eq!(got, o[key].as_str().unwrap(), "{key} {c}");
            assert_eq!(json!(r.calls), o[format!("{key}_calls")], "{key} {c}");
        }
    }
}

#[test]
fn globals_and_avars_are_the_image_values() {
    let g = Globals::from_fresh_process_image();
    assert_eq!((g.fog_debug_b403c8, g.reblend_b403c9, g.explored_b285d4), (0, 0, 0x80));
    let v = FogVariables::constructor();
    assert_eq!((v.pixels_per_meter_61, f32::from_bits(v.gray_percentage_64), f32::from_bits(v.fade_time_65)), (2, 0.5, 0.5));
}
