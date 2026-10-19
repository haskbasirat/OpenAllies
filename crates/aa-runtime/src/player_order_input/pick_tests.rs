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
fn fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/phase8-orders-input/pick-math/native-pick-math.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// Every row of the original-code oracle (0x875700, 0x8707E0, 0x870720,
/// 0x715430, 0x436480, 0x42D750, 0x44B600, 0x640A90) at its precision.
#[test]
fn pick_math_matches_native_rows() {
    let doc = fixture();
    let rows = doc["rows"].as_array().unwrap();
    assert!(rows.len() >= 800);
    let mut checked = 0;
    for row in rows {
        let p = precision(row["precision"].as_str().unwrap());
        let a = &row["args"];
        let n = &row["native"];
        let kind = row["kind"].as_str().unwrap();
        match kind {
            "sphere" => {
                let got = ray_sphere_875700(words(&a[0]), words(&a[1]), words(&a[2]), a[3].as_u64().unwrap() as u32, p).unwrap();
                assert_eq!(got, n["al"].as_u64().unwrap() == 1, "{row}");
            }
            "inverse" => {
                let (out, al) = inverse_8707e0(words(&a[0]), p).unwrap();
                assert_eq!(out, words::<9>(&n["out"]), "{row}");
                assert_eq!(al, n["al"].as_u64().unwrap() == 1, "{row}");
            }
            "transform" => {
                assert_eq!(transform_870720(words(&a[0]), words(&a[1]), p).unwrap(), words::<3>(&n["out"]), "{row}");
            }
            "box" => {
                let got = ray_box_715430(words(&a[0]), words(&a[1]), words(&a[2]), p).unwrap();
                assert_eq!(got, n["al"].as_u64().unwrap() == 1, "{row}");
            }
            "distance" => {
                let d = x87::distance_436480(words(&a[0]), words(&a[1]), p).unwrap();
                let ext = d.to_x87_extended_bits().unwrap();
                let st0 = &n["st0"];
                assert_eq!(ext as u64, st0["extended_significand"].as_u64().unwrap(), "{row}");
                assert_eq!((ext >> 64) as u64, st0["extended_sign_exponent"].as_u64().unwrap(), "{row}");
            }
            "normalize3" => {
                let out = crate::native_camera::ni::normalize(words(&a[0]), p).unwrap();
                assert_eq!(out, words::<3>(&n["out"]), "{row}");
            }
            "normalize2" => {
                let out = super::super::command::normalize_44b600(words(&a[0]), p).unwrap();
                assert_eq!(out, words::<2>(&n["out"]), "{row}");
            }
            "centroid" => {
                let points: Vec<[u32; 2]> = a[0].as_array().unwrap().iter().map(words::<2>).collect();
                assert_eq!(centroid_points(&points, p), words::<2>(&n["out"]), "{row}");
            }
            other => panic!("kind {other}"),
        }
        checked += 1;
    }
    assert_eq!(checked, rows.len());
}

/// 0x640A90 through the selection manager's own implementation.
fn centroid_points(points: &[[u32; 2]], p: Precision) -> [u32; 2] {
    struct Points<'a>(&'a [[u32; 2]], Precision);
    impl super::super::ui_world::Leaves for Points<'_> {
        type View = usize;
        fn precision(&self) -> Precision {
            self.1
        }
        fn position_38(&mut self, v: usize) -> Result<[u32; 3]> {
            Ok([self.0[v][0], self.0[v][1], 0])
        }
        fn flags_54(&mut self, _: usize) -> Result<u32> { unreachable!() }
        fn set_flags_54(&mut self, _: usize, _: u32) -> Result<()> { unreachable!() }
        fn field_10_58cb30(&mut self, _: usize) -> Result<u64> { unreachable!() }
        fn pickable_9c(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn class_78(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn class_84(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn class_7c(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn class_80(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn virtual_50(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn virtual_54(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_present_14(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_45b7a0(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_flag8_bit0(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_virtual_58(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_virtual_84(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_4d12c0(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_virtual_20(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_70(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_90(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn link_16c_view(&mut self, _: usize) -> Result<Option<usize>> { unreachable!() }
        fn link_94_view(&mut self, _: usize) -> Result<Option<usize>> { unreachable!() }
        fn link_94_live(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn hidden_4549c0(&mut self, _: usize) -> Result<bool> { unreachable!() }
        fn actor_view_0c(&mut self, _: usize) -> Result<Option<usize>> { unreachable!() }
        fn root_view_4ce6c0(&mut self, _: usize) -> Result<Option<usize>> { unreachable!() }
        fn voiced_436600(&mut self, _: usize) -> Result<Option<Option<usize>>> { unreachable!() }
        fn actor_virtual_150(&mut self, _: usize, _: &mut aa_orders::commands::CommandWire) -> Result<bool> { unreachable!() }
        fn selection_sound_436850(&mut self, _: usize) -> Result<()> { unreachable!() }
        fn command_masks_437eb0(&mut self, _: &[usize], _: super::super::ui_world::StaticRef, _: &mut [u32; 4]) -> Result<()> { unreachable!() }
        fn brush_changed_42d4d0(&mut self) -> Result<()> { unreachable!() }
        fn vworld_list_changed_4325c0(&mut self) -> Result<()> { unreachable!() }
        fn vworld_ad(&mut self) -> Result<bool> { unreachable!() }
        fn floater_remove_42eb10(&mut self, _: usize) -> Result<()> { unreachable!() }
        fn floater_update_453470(&mut self, _: usize) -> Result<()> { unreachable!() }
        fn requests(&mut self) -> &mut dyn super::super::requests::Requests<usize> { unreachable!() }
    }
    let list: Vec<usize> = (0..points.len()).collect();
    super::super::ui_world::centroid_640a90(&mut Points(points, p), &list, p).unwrap()
}
