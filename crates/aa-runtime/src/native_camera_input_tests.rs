use super::*;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/phase8-camera-input/native-camera-input.json"
    ))
    .unwrap()
}
fn pc(row: &Value) -> Precision {
    match row["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => panic!(),
    }
}
fn u(v: &Value) -> u32 {
    v.as_u64().unwrap() as u32
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(u).collect()
}

#[test]
fn rect_and_cursor_poll_follow_59ad30() {
    let rect = Rect::client_40e210(800, 600).unwrap();
    assert_eq!(rect, Rect { left: 0, top: 0, right: 799, bottom: 599 });
    let mut c = InputCursor { position_0c: [0, 0], rect_14: rect, inside_24: false };
    c.poll_59ad30([10, 20]);
    assert_eq!((c.position_0c, c.inside_24), ([10, 20], true));
    c.poll_59ad30([-5, 700]);
    assert_eq!((c.position_0c, c.inside_24), ([0, 599], false));
    c.poll_59ad30([900, -3]);
    assert_eq!((c.position_0c, c.inside_24), ([799, 0], false));
}

#[test]
fn accelerate_5ccc20_matches_native() {
    let f = fixture();
    let rows = f["sections"]["accelerate_5ccc20"].as_array().unwrap();
    assert!(rows.len() > 300);
    for row in rows {
        let d = words(&row["direction"]);
        let s = words(&row["speed"]);
        let (stored, out) =
            accelerate_5ccc20(u(&row["accumulated"]), [d[0], d[1]], u(&row["dt"]), [s[0], s[1], s[2]], pc(row))
                .unwrap();
        assert_eq!(stored, u(&row["stored"]), "{row}");
        assert_eq!(out.to_vec(), words(&row["out"]), "{row}");
    }
}

#[test]
fn impulse_dt_5995f0_matches_native() {
    let f = fixture();
    for row in f["sections"]["impulse_dt_5995f0"].as_array().unwrap() {
        assert_eq!(impulse_dt_5995f0(u(&row["last"]), u(&row["now"]), pc(row)).unwrap(), u(&row["dt"]), "{row}");
    }
}

#[derive(Default)]
struct Gates {
    vworld_ad: bool,
    bit19: bool,
    visual: bool,
    active: bool,
    hover: bool,
    d8: bool,
    d9: bool,
}
impl Ui for Gates {
    fn application_active(&mut self) -> Result<bool> {
        Ok(self.active)
    }
    fn vworld_ad(&mut self) -> Result<bool> {
        Ok(self.vworld_ad)
    }
    fn vworld_flag_24_bit19(&mut self) -> Result<bool> {
        Ok(self.bit19)
    }
    fn visual_map_bc(&mut self) -> Result<bool> {
        Ok(self.visual)
    }
    fn vworld_hovered_5efa80(&mut self) -> Result<bool> {
        Ok(self.hover)
    }
    fn app_byte_48(&mut self) -> Result<bool> {
        Ok(self.d8)
    }
    fn app_byte_49(&mut self) -> Result<bool> {
        Ok(self.d9)
    }
    fn ui_widget_60_consumes_move(&mut self) -> Result<bool> {
        Ok(false)
    }
}
struct Clock {
    counters: Vec<i64>,
    reads: Vec<i64>,
    frequency: i64,
}
impl Qpc for Clock {
    fn counter(&mut self) -> Result<i64> {
        if self.counters.is_empty() {
            return Err("unscripted counter".into());
        }
        let v = self.counters.remove(0);
        self.reads.push(v);
        Ok(v)
    }
    fn frequency(&mut self) -> Result<i64> {
        Ok(self.frequency)
    }
}
#[derive(Default)]
struct Cam {
    words: std::collections::BTreeMap<u32, u32>,
    locked: u8,
    calls: Vec<Value>,
}
impl Camera for Cam {
    fn word(&mut self, offset: u32) -> Result<u32> {
        self.words.get(&offset).copied().ok_or_else(|| format!("camera +{offset:X} not in fixture"))
    }
    fn byte_214(&mut self) -> Result<u8> {
        Ok(self.locked)
    }
    fn developer_6fe9c0(&mut self) -> Result<bool> {
        Ok(false)
    }
    fn pan_700500(&mut self, millis: u32, d: [u32; 2]) -> Result<()> {
        self.calls.push(serde_json::json!(["pan", millis, [d[0], d[1]]]));
        Ok(())
    }
    fn zoom_7005f0(&mut self, value: u32) -> Result<()> {
        self.calls.push(serde_json::json!(["zoom", value]));
        Ok(())
    }
    fn pose_6ff220(&mut self, c: [u32; 3], yaw: u32, pitch: u32, zoom: u32) -> Result<()> {
        self.calls.push(serde_json::json!(["pose", [c[0], c[1], c[2], yaw, pitch, zoom]]));
        Ok(())
    }
    fn rotate_6feba0(&mut self, yaw: u32, pitch: u32) -> Result<()> {
        self.calls.push(serde_json::json!(["rotate", yaw, pitch]));
        Ok(())
    }
    fn look_at_6ff200(&mut self, xy: [u32; 2]) -> Result<()> {
        self.calls.push(serde_json::json!(["look_at", xy[0], xy[1]]));
        Ok(())
    }
}
struct Vars {
    uvars: std::collections::BTreeMap<u32, u32>,
    pitch: [u32; 4],
}
impl Variables for Vars {
    fn uvar(&mut self, offset: u32) -> Result<u32> {
        self.uvars.get(&offset).copied().ok_or_else(|| format!("uvar {offset:X} not in fixture"))
    }
    fn avar(&mut self, offset: u32) -> Result<u32> {
        match offset {
            0x50 => Ok(self.pitch[0]),
            0x54 => Ok(self.pitch[3]),
            _ => Err("avar".into()),
        }
    }
    fn avar_range(&mut self, offset: u32) -> Result<[u32; 2]> {
        assert_eq!(offset, 0x50);
        Ok([self.pitch[1], self.pitch[2]])
    }
    fn metagame_40cee0(&mut self) -> Result<bool> {
        Ok(false)
    }
}
fn hexmap(v: &Value) -> std::collections::BTreeMap<u32, u32> {
    v.as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (u32::from_str_radix(k.trim_start_matches("0x"), 16).unwrap(), u(v)))
        .collect()
}

#[test]
fn scroller_state_machine_matches_native() {
    let f = fixture();
    let rows = f["sections"]["scroller"].as_array().unwrap();
    assert_eq!(rows.len(), 30);
    for row in rows {
        let p = pc(row);
        let r = words(&row["rect"]);
        let rect = Rect { left: r[0] as i32, top: r[1] as i32, right: r[2] as i32, bottom: r[3] as i32 };
        let anchor = row["start_anchor"].as_i64().unwrap();
        let mut c = Controls::from_native_anchors(Timer::from_anchor(0), Timer::from_anchor(anchor), rect);
        let cur = words(&row["cursor"]);
        c.set_ui_cursor_5bf2b0([cur[0] as i32, cur[1] as i32]);
        c.set_input_inside_24(true);
        let s = words(&row["speed"]);
        let mut vars = Vars { uvars: [(0x18, s[0]), (0x1c, s[1]), (0x20, s[2])].into(), pitch: [0; 4] };
        let mut ui = Gates { vworld_ad: true, bit19: true, visual: true, active: true, hover: true, ..Default::default() };
        let mut cam = Cam::default();
        let mut clock = Clock { counters: Vec::new(), reads: Vec::new(), frequency: row["frequency"].as_i64().unwrap() };
        for step in row["steps"].as_array().unwrap() {
            let op = step["op"].as_array().unwrap();
            cam.calls.clear();
            clock.reads.clear();
            //Counters the native harness pre-scripted for this non-frame step.
            clock.counters = step["calls"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c[0] == "qpc")
                .map(|c| c[1].as_i64().unwrap())
                .collect();
            match op[0].as_str().unwrap() {
                "begin" => c.begin_scroll_5cd560(u(&op[1]), &mut ui, &mut clock, p).unwrap(),
                "end" => c.end_scroll_5ccd30(u(&op[1]), &mut ui, p).unwrap(),
                "cursor" => {
                    c.set_ui_cursor_5bf2b0([op[1].as_i64().unwrap() as i32, op[2].as_i64().unwrap() as i32]);
                    if c.active() == Some(Slot::Scroller) {
                        c.scroller_mouse_move_5cd410(&mut ui, p).unwrap();
                    }
                }
                "frame" => {
                    clock.counters = op[1].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
                    c.game_state_update_5cd6b0(&mut ui, &mut clock, p).unwrap();
                    if c.active() == Some(Slot::Scroller) {
                        let mut trace = Vec::new();
                        c.scroller_update_5cd320(&mut ui, &mut vars, &mut cam, &mut clock, &mut trace, p).unwrap();
                    }
                    assert!(clock.counters.is_empty(), "unused counters {row}");
                }
                "set" => {
                    for (k, v) in op[1].as_object().unwrap() {
                        let v = v.as_u64().unwrap() != 0;
                        match k.as_str() {
                            "vworld_ad" => ui.vworld_ad = v,
                            "vworld_bit19" => ui.bit19 = v,
                            "visual_bc" => ui.visual = v,
                            "app_active" => ui.active = v,
                            "inside" => c.set_input_inside_24(v),
                            "hover" => ui.hover = v,
                            "d8" => ui.d8 = v,
                            "d9" => ui.d9 = v,
                            "locked" => cam.locked = v as u8,
                            _ => panic!("{k}"),
                        }
                    }
                }
                other => panic!("{other}"),
            }
            //Native calls in order; the VWorld+13C hover pick is not owned here.
            let native: Vec<&Value> =
                step["calls"].as_array().unwrap().iter().filter(|c| c[0] != "pick").collect();
            let mut mine = Vec::new();
            let mut reads = clock.reads.iter();
            let mut pans = cam.calls.iter();
            for call in &native {
                match call[0].as_str().unwrap() {
                    "qpc" => mine.push(serde_json::json!(["qpc", reads.next().copied()])),
                    "pan" => mine.push(pans.next().cloned().unwrap_or(Value::Null)),
                    _ => panic!(),
                }
            }
            assert!(reads.next().is_none() && pans.next().is_none(), "extra Rust effects {step}");
            assert_eq!(native.into_iter().cloned().collect::<Vec<_>>(), mine, "{} {step}", row["name"]);
            let st = &step["state"];
            let sc = c.scroller();
            assert_eq!(sc.bits(), u(&st["bits"]), "{step}");
            assert_eq!(sc.edge(), u(&st["edge"]), "{step}");
            assert_eq!(sc.keys(), u(&st["keys"]), "{step}");
            assert_eq!(sc.direction().unwrap_or([0, 0]).to_vec(), words(&st["direction"]), "{step}");
            assert_eq!(sc.timer().anchor(), st["anchor"].as_i64().unwrap(), "{step}");
            assert_eq!(sc.accumulated(), u(&st["accumulated"]), "{step}");
            assert_eq!(sc.millis(), u(&st["millis"]), "{step}");
            let active = match c.active() {
                Some(Slot::Scroller) => Value::from("scroller"),
                Some(Slot::Rotate) => Value::from("rotate"),
                None => Value::Null,
            };
            assert_eq!(active, st["active"], "{step}");
        }
    }
}

#[test]
fn impulses_and_rotation_setters_match_native() {
    let f = fixture();
    let rows = f["sections"]["impulses"].as_array().unwrap();
    assert!(rows.len() > 300);
    for row in rows {
        let p = pc(row);
        let mut cam = Cam::default();
        let cw = hexmap(&row["camera"]);
        cam.locked = cw.get(&0x214).copied().unwrap_or(0) as u8;
        cam.words = cw.into_iter().filter(|(k, _)| *k != 0x214).collect();
        let mut trace = Vec::new();
        match row["function"].as_str().unwrap() {
            "yaw" => yaw_700440(u(&row["value"]), &mut cam, &mut trace).unwrap(),
            "pitch" => pitch_7004a0(u(&row["value"]), &mut cam, &mut trace).unwrap(),
            name => {
                let mut vars = Vars { uvars: hexmap(&row["uvars"]), pitch: {
                    let w = words(&row["pitch_avar"]);
                    [w[0], w[1], w[2], w[3]]
                } };
                let function = ImpulseFn::lookup_5c36a0(name).unwrap();
                run_impulse(function, u(&row["dt"]), &mut vars, &mut cam, &mut trace, p).unwrap();
            }
        }
        assert_eq!(Value::from(cam.calls.clone()), row["calls"], "{row}");
    }
}

#[test]
fn zoom_commands_and_wheel_match_native() {
    let f = fixture();
    for row in f["sections"]["zoom"].as_array().unwrap() {
        let p = pc(row);
        let mut cam = Cam::default();
        let mut trace = Vec::new();
        match row["command"].as_str().unwrap() {
            "DeltaZoom" => {
                cam.words.insert(0x208, u(&row["zoom"]));
                delta_zoom_43bc80(u(&row["parsed"]), &mut cam, &mut trace, p).unwrap();
            }
            "ZoomMin" => {
                cam.words.insert(0x20c, u(&row["limit"]));
                zoom_limit_43bc20(0x20c, &mut cam, &mut trace, p).unwrap();
            }
            "ZoomMax" => {
                cam.words.insert(0x210, u(&row["limit"]));
                zoom_limit_43bc20(0x210, &mut cam, &mut trace, p).unwrap();
            }
            _ => panic!(),
        }
        //Native also shows the 426330 [-1,1] bound arguments (BF800000,3F800000).
        let native: Vec<&Value> = row["calls"].as_array().unwrap().iter().filter(|c| c[0] != "parse").collect();
        if row["command"] == "DeltaZoom" {
            assert_eq!(row["calls"][0], serde_json::json!(["parse", 0xbf80_0000u32, 0x3f80_0000u32]));
        }
        assert_eq!(native.into_iter().cloned().collect::<Vec<_>>(), cam.calls, "{row}");
    }
    for row in f["sections"]["wheel"].as_array().unwrap() {
        let delta = row["delta"].as_i64().unwrap() as i16;
        assert_eq!(wheel_argument_5efccc(delta, pc(row)).unwrap(), row["argument"].as_u64().unwrap(), "{row}");
    }
    assert!(delta_zoom_43bc80(0x3f80_0001, &mut Cam::default(), &mut Vec::new(), Precision::Double53).is_err());
}

#[test]
fn drag_rotation_matches_native() {
    let f = fixture();
    for row in f["sections"]["rotate"].as_array().unwrap() {
        let p = pc(row);
        let mut c = Controls::from_native_anchors(
            Timer::from_anchor(0),
            Timer::from_anchor(0),
            Rect { left: 0, top: 0, right: 799, bottom: 599 },
        );
        let mut cam = Cam { words: hexmap(&row["camera"]), ..Default::default() };
        let mut vars = Vars {
            uvars: [(0x38, u(&row["rotate_speed"])), (0xb4, u(&row["camera_rotate"]))].into(),
            pitch: [0; 4],
        };
        let mut ui = Gates { vworld_ad: true, bit19: true, visual: true, active: true, hover: true, ..Default::default() };
        for step in row["steps"].as_array().unwrap() {
            cam.calls.clear();
            let op = step["op"].as_array().unwrap();
            match op[0].as_str().unwrap() {
                "begin" => {
                    c.begin_rotate_5c21c0(&mut vars, &mut cam, &mut ui, p).unwrap();
                    assert_eq!(c.rotate().rate(), u(&step["rate"]));
                }
                "cursor" => {
                    c.set_ui_cursor_5bf2b0([op[1].as_i64().unwrap() as i32, op[2].as_i64().unwrap() as i32]);
                    if c.active() == Some(Slot::Rotate) {
                        c.rotate_move_5ccf00(&mut cam, &mut Vec::new(), p).unwrap();
                    }
                    let last = c.rotate().last();
                    assert_eq!(vec![last[0] as u32, last[1] as u32], words(&step["last"]), "{step}");
                }
                "end" => c.end_rotate_5cc870(),
                _ => panic!(),
            }
            assert_eq!(Value::from(cam.calls.clone()), step["calls"], "{step}");
            let active = match c.active() {
                Some(Slot::Rotate) => Value::from("rotate"),
                Some(Slot::Scroller) => Value::from("scroller"),
                None => Value::Null,
            };
            assert_eq!(active, step["active"], "{step}");
        }
    }
}
