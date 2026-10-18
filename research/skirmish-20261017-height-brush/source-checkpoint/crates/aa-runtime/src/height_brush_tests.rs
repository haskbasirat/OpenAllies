use crate::height_brush_geometry as geometry;
use aa_heightfield::{binding, brush};
use aa_numeric::{Binary, Precision};
use aa_spatial::{collision::Shape, geometry::Placement};
use serde_json::{Value, json};
struct Services<'a> {
    c: &'a Value,
    events: Vec<Value>,
    notifications: Vec<Value>,
    failure: Option<&'static str>,
}
impl brush::Effects for Services<'_> {
    fn falloff_distance(&mut self) -> brush::Result<u32> {
        if self.failure == Some("falloff") {
            return Err(brush::Error::Service("reached falloff".into()));
        }
        Ok(self.c["falloff"].as_u64().unwrap() as u32)
    }
    fn terrain_slope(&mut self, xy: [u32; 2]) -> brush::Result<Option<u32>> {
        if self.failure == Some("slope") {
            return Err(brush::Error::Service("reached terrain slope".into()));
        }
        let g = &self.c["grid"];
        let shift = g["shift"].as_u64().unwrap() as u32;
        let (w, h) = (
            g["width"].as_i64().unwrap() as i32,
            g["height"].as_i64().unwrap() as i32,
        );
        let [x, y] = xy.map(|v| Binary::from_f32_bits(v).unwrap());
        if x.is_negative_nonzero()
            || y.is_negative_nonzero()
            || x.compare(&Binary::from_i32((w - 1) << shift)) != std::cmp::Ordering::Less
            || y.compare(&Binary::from_i32((h - 1) << shift)) != std::cmp::Ordering::Less
        {
            return Ok(None);
        }
        let cell = [x.truncate_i32()? >> shift, y.truncate_i32()? >> shift];
        let index = (cell[1] * w + cell[0]) as usize;
        let v = self.c["slopes"][index].as_u64().unwrap() as u32;
        self.events
            .push(json!({"kind":"slope","index":index,"value":v}));
        Ok(Some(v))
    }
    fn protected_diagnostic(
        &mut self,
        cell: [i32; 2],
        area: u8,
        requested: u8,
        _: u32,
        _: u32,
        _: u32,
    ) -> brush::Result<()> {
        self.events
            .push(json!({"kind":"protected","cell":cell,"area":area,"requested":requested}));
        Ok(())
    }
    fn notify(&mut self, center: [u32; 2], radius: u32) -> brush::Result<()> {
        if self.failure == Some("notify") {
            return Err(brush::Error::Service("reached notify".into()));
        }
        self.notifications
            .push(json!([center[0], center[1], radius]));
        Ok(())
    }
}
impl geometry::Sources for Services<'_> {
    fn orientation(&mut self) -> brush::Result<[u32; 3]> {
        self.events
            .push(json!({"kind":"prepare","value":self.c["orientation"]}));
        Ok(read::<3>(&self.c["orientation"]))
    }
    fn point_shape(&mut self) -> brush::Result<Shape> {
        Ok(Shape {
            kind: 0,
            radius_bits: 0,
            bounds: [0; 4],
        })
    }
}
fn read<const N: usize>(v: &Value) -> [u32; N] {
    std::array::from_fn(|i| v[i].as_u64().unwrap() as u32)
}
fn run_row(r: &Value, failure: Option<&'static str>) -> (brush::Result<bool>, Value) {
    let c = &r["input"];
    let g = &c["grid"];
    let pc = match r["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        _ => unreachable!(),
    };
    let (w, h, shift) = (
        g["width"].as_i64().unwrap() as i32,
        g["height"].as_i64().unwrap() as i32,
        g["shift"].as_u64().unwrap() as u32,
    );
    let layout = binding::GridLayout {
        extent_i32: [(w - 1) << shift, (h - 1) << shift],
        width: w,
        height: h,
        coordinate_shift: shift,
        cell_size_bits: g["cell"].as_u64().unwrap() as u32,
        index_bias: g["bias"].as_i64().unwrap() as i32,
    };
    let mut field = binding::InitializedHeightfield::new(
        layout,
        c["limits"][0].as_u64().unwrap() as u32,
        c["limits"][1].as_u64().unwrap() as u32,
        0,
    )
    .unwrap();
    field.field.height_bits = g["heights"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect();
    field.sample_flags = g["flags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();
    let mut state = brush::State::from_height_map_constructor();
    let s = read::<6>(&c["shape"]);
    let position = read::<2>(&c["xy"]);
    let placement = Placement {
        shape: Shape {
            kind: s[0],
            radius_bits: s[1],
            bounds: [s[2], s[3], s[4], s[5]],
        },
        position,
        facing: read::<2>(&c["facing"]),
    };
    let mut geometry = geometry::Placed {
        placement: &placement,
        orientation: None,
    };
    let mut effects = Services {
        c,
        events: vec![],
        notifications: vec![],
        failure,
    };
    let s = read::<5>(&c["settings"]);
    let settings = brush::Settings {
        mode: s[0],
        height: s[1],
        slope: s[2],
        minimum: s[3],
        maximum: s[4],
        protect: c["protect"] == 1,
        smooth_edge: c["smooth"] == 1,
        strength: c["strength"].as_u64().unwrap() as u32,
        area: c["area"].as_u64().unwrap() as u8,
    };
    let returned = brush::apply(
        &mut field,
        &mut state,
        &mut geometry,
        position,
        position,
        settings,
        &mut effects,
        pc,
    );
    let actual = json!({"returned":returned.as_ref().copied().ok(),"heights":field.field.height_bits,"dirty":state.dirty_bounds(),"events":effects.events,"notifications":effects.notifications});
    (returned, actual)
}
#[test]
fn complete_native_same_endpoint_mode0_and1_height_brush() {
    let f: Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261017-height-brush/native-height-brush.json"
    ))
    .unwrap();
    assert_eq!(f["rows"].as_array().unwrap().len(), 75);
    for r in f["rows"].as_array().unwrap() {
        let (returned, actual) = run_row(r, None);
        returned.unwrap();
        assert_eq!(
            actual, r["native"],
            "{} / {}",
            r["input"]["name"], r["precision"]
        );
    }
}
#[test]
fn required_services_preserve_native_commit_boundaries() {
    let f: Value = serde_json::from_slice(include_bytes!(
        "../../../research/skirmish-20261017-height-brush/native-height-brush.json"
    ))
    .unwrap();
    for r in f["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["input"]["name"] == "rectangle")
    {
        for failure in ["falloff", "slope"] {
            let (result, actual) = run_row(r, Some(failure));
            assert!(result.is_err(), "{failure} / {}", r["precision"]);
            assert_eq!(actual["heights"], r["input"]["grid"]["heights"]);
            let p = &r["input"]["xy"];
            assert_eq!(actual["dirty"], json!([p[0], p[1], p[0], p[1]]));
            assert_eq!(actual["notifications"], json!([]));
        }
        let (result, actual) = run_row(r, Some("notify"));
        assert!(result.is_err());
        assert_eq!(actual["heights"], r["native"]["heights"]);
        assert_eq!(actual["dirty"], r["native"]["dirty"]);
        assert_eq!(actual["notifications"], json!([]));
    }
}
