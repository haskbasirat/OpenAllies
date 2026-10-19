//! Native-oracle replays (see research/phase8-orders-input).
use super::*;
use serde_json::{json, Value};

fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}
struct Camera(Vec<(u32, u32)>);
impl CameraWords for Camera {
    fn word(&self, offset: u32) -> Result<u32> {
        self.0.iter().find(|(o, _)| *o == offset).map(|(_, w)| *w).ok_or_else(|| format!("camera word {offset:#x} not in the oracle"))
    }
}
struct Mock {
    camera: Camera,
    viewport: (i32, i32),
    seed: [u32; 3],
    terrain_ready: bool,
    kind: u32,
    writes: Vec<(usize, u32)>,
    calls: Vec<Value>,
}
impl Sources for Mock {
    type Camera = Camera;
    fn input_block_b396c0(&mut self) -> Result<u32> {
        Ok(self.kind)
    }
    fn code_out_of_range(&mut self, code: u16) -> Result<()> {
        self.calls.push(json!(["log", code]));
        Ok(())
    }
    fn camera(&mut self) -> Result<&Camera> {
        Ok(&self.camera)
    }
    fn viewport_b396d0(&mut self) -> Result<(i32, i32)> {
        Ok(self.viewport)
    }
    fn seed_b43cf8(&mut self) -> Result<[u32; 3]> {
        Ok(self.seed)
    }
    fn terrain_ready_b36518(&mut self) -> Result<bool> {
        Ok(self.terrain_ready)
    }
    fn terrain_71d840(&mut self, eye: [u32; 3], direction: [u32; 3], point: &mut [u32; 3]) -> Result<()> {
        self.calls.push(json!(["terrain", eye, direction]));
        for &(i, w) in &self.writes {
            point[i] = w;
        }
        Ok(())
    }
}

/// Every event of the original-code oracle for 0x59A6E0 / 0x5C2770 at its
/// precision: the same drop/log/terrain calls, the same 0x30-byte event and
/// the same Input+0x44 table after each call.
#[test]
fn add_mouse_event_matches_native_rows() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-orders-input/event/native-event.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert!(rows.len() >= 180);
    let mut events = 0;
    for row in rows {
        let p = precision(row["precision"].as_str().unwrap());
        let a = &row["args"];
        let u = |v: &Value| v.as_u64().unwrap() as u32;
        let camera = Camera(a["camera"].as_array().unwrap().iter().map(|c| (u(&c[0]), u(&c[1]))).collect());
        let seed = a["seed"].as_array().unwrap();
        let mut mock = Mock {
            camera,
            viewport: (a["viewport"][0].as_i64().unwrap() as i32, a["viewport"][1].as_i64().unwrap() as i32),
            seed: [u(&seed[0]), u(&seed[1]), u(&seed[2])],
            terrain_ready: a["terrain"] == "ready",
            kind: 0,
            writes: Vec::new(),
            calls: Vec::new(),
        };
        let mut input = Input::new();
        for (ev, native) in a["events"].as_array().unwrap().iter().zip(row["native"].as_array().unwrap()) {
            mock.kind = u(&ev["kind"]);
            mock.writes = ev["writes"].as_array().unwrap().iter().map(|w| (w[0].as_u64().unwrap() as usize, u(&w[1]))).collect();
            mock.calls.clear();
            let modifiers = u(&ev["modifiers"]);
            for (bit, vk) in [(1, 0x12), (2, 0x10), (4, 0x11)] {
                input.modifier_key_59a5c0(vk, if modifiers & bit != 0 { flag::PRESSED } else { flag::RELEASED });
            }
            assert_eq!(input.modifiers_28(), modifiers);
            let (x, y, delta) = (ev["x"].as_i64().unwrap() as i16, ev["y"].as_i64().unwrap() as i16, ev["delta"].as_i64().unwrap() as i16);
            let got = input.add_mouse_event_59a6e0(&mut mock, u(&ev["code"]) as u16, u(&ev["flags"]), x, y, delta, p).unwrap();
            let mut expected = Vec::new();
            let mut native_event = None;
            for c in native["calls"].as_array().unwrap() {
                match c[0].as_str().unwrap() {
                    "log" => expected.push(json!(["log", c[1]])),
                    "terrain" => {
                        assert_eq!((u(&c[3]), u(&c[4])), (0, 0), "NULL secondary and margin 0: {c}");
                        expected.push(json!(["terrain", c[1], c[2]]));
                    }
                    "event" => native_event = Some(c[1].as_array().unwrap().iter().map(u).collect::<Vec<u32>>()),
                    other => panic!("{other}"),
                }
            }
            assert_eq!(mock.calls, expected, "{ev}");
            match (got, native_event) {
                (None, None) => {}
                (Some(e), Some(w)) => {
                    let words = [
                        u32::from(e.x as u16) | u32::from(e.y as u16) << 16,
                        e.flags,
                        u32::from(e.code) | u32::from(e.delta as u16) << 16,
                        e.world[0],
                        e.world[1],
                        e.world[2],
                        e.origin[0],
                        e.origin[1],
                        e.origin[2],
                        e.direction[0],
                        e.direction[1],
                        e.direction[2],
                    ];
                    assert_eq!(words.to_vec(), w, "{} {ev}", row["precision"]);
                    events += 1;
                }
                (g, n) => panic!("queued mismatch rust {g:?} native {n:?}: {ev}"),
            }
            let table: Vec<u32> = (0..0x111u16).map(|c| input.state_44(c)).collect();
            assert_eq!(table, native["table"].as_array().unwrap().iter().map(u).collect::<Vec<u32>>(), "{ev}");
        }
    }
    assert!(events > 1000);
}
