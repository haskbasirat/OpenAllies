use super::*;
use serde_json::Value;

fn precision(name: &str) -> Precision {
    match name {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("precision {other}"),
    }
}
fn words<const N: usize>(v: &Value) -> [u32; N] {
    let a = v.as_array().unwrap();
    assert_eq!(a.len(), N);
    std::array::from_fn(|i| a[i].as_u64().unwrap() as u32)
}
fn planes(v: &Value) -> [Plane; 4] {
    let w = words::<16>(v);
    std::array::from_fn(|i| Plane { normal: [w[4 * i], w[4 * i + 1], w[4 * i + 2]], constant: w[4 * i + 3] })
}
fn plane_list(v: &Value) -> [Plane; 4] {
    let a = v.as_array().unwrap();
    std::array::from_fn(|i| {
        let w = words::<4>(&a[i]);
        Plane { normal: [w[0], w[1], w[2]], constant: w[3] }
    })
}

/// The oracle camera: exactly the words the native code was given.
struct Camera(Vec<(u32, u32)>);
impl CameraWords for Camera {
    fn word(&self, offset: u32) -> Result<u32> {
        self.0.iter().find(|(o, _)| *o == offset).map(|(_, w)| *w).ok_or_else(|| format!("camera word {offset:#x} not in the oracle"))
    }
}
fn camera(v: &Value) -> Camera {
    Camera(v.as_array().unwrap().iter().map(|p| (p[0].as_u64().unwrap() as u32, p[1].as_u64().unwrap() as u32)).collect())
}
fn model(v: &Value, rotate: &Value, translate: &Value) -> Model {
    Model {
        bound_center_28: words(&v["center"]),
        bound_radius_34: v["radius"].as_u64().unwrap() as u32,
        rotate_6c: words(rotate),
        translate_90: words(translate),
        resource_box_4c: Some(words(&v["box"])),
    }
}

/// Every row of the original-code oracle (0x436310, 0x436DD0, 0x4371A0,
/// 0x715850, 0x454B50 -> 0x715D40) at its precision.
#[test]
fn box_selection_matches_native_rows() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-orders-input/box/native-box.json");
    let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let rows = doc["rows"].as_array().unwrap();
    assert!(rows.len() >= 1100);
    let mut checked = 0;
    for row in rows {
        let p = precision(row["precision"].as_str().unwrap());
        let a = &row["args"];
        let n = &row["native"];
        match row["kind"].as_str().unwrap() {
            "cross" => {
                assert_eq!(cross_436310(words(&a[0]), words(&a[1]), p).unwrap(), words::<3>(&n["out"]), "{row}");
            }
            "release" => {
                let got = box_planes_436dd0(&camera(&a[0]), words(&a[1]), words(&a[2]), p).unwrap();
                assert_eq!(got, planes(&n["planes"]), "{row}");
            }
            "drag" => {
                let ints = |v: &Value| -> Vec<i32> { v.as_array().unwrap().iter().map(|x| x.as_i64().unwrap() as i32).collect() };
                let (vp, rect) = (ints(&a[1]), ints(&a[2]));
                let got = screen_box_planes_4371a0(&camera(&a[0]), (vp[0], vp[1]), [rect[0], rect[1], rect[2], rect[3]], p).unwrap();
                assert_eq!(got, planes(&n["planes"]), "{row}");
            }
            "classify" => {
                let w = words::<4>(&a[0]);
                let plane = Plane { normal: [w[0], w[1], w[2]], constant: w[3] };
                assert_eq!(classify_box_715850(&plane, words(&a[1]), p).unwrap(), n["class"].as_u64().unwrap() as u32, "{row}");
            }
            "inside" => {
                let before = model(&a[0], &a[0]["rotate"], &a[0]["translate"]);
                let after = model(&a[0], &a[1]["rotate"], &a[1]["translate"]);
                let mut updated = 0;
                let got = model_in_planes_715d40(&before, &plane_list(&a[2]), p, &mut || {
                    updated += 1;
                    Ok(after)
                })
                .unwrap();
                assert_eq!(got, n["al"].as_u64().unwrap() == 1, "{row}");
                assert_eq!(updated, n["updated"].as_u64().unwrap(), "{row}");
            }
            other => panic!("kind {other}"),
        }
        checked += 1;
    }
    assert_eq!(checked, rows.len());
}
