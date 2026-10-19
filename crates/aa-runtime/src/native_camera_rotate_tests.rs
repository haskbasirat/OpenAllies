use super::*;
use serde_json::Value;

///research/phase8-camera-input/native-camera-rotate.json: the original 6FEBA0
///(6FE3B0, 870720, FSIN, 6FE470) with 6FE050 as its only seam, both images.
#[test]
fn rotate_about_target_6feba0_matches_native() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../research/phase8-camera-input/native-camera-rotate.json"
    ))
    .unwrap();
    let rows = fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 84);
    let globals = Globals::from_static_constructor_9e3520();
    let w = |v: &Value| v.as_u64().unwrap() as u32;
    for row in rows {
        let pc = match row["precision"].as_str().unwrap() {
            "Single24" => Precision::Single24,
            "Double53" => Precision::Double53,
            "Extended64" => Precision::Extended64,
            _ => panic!(),
        };
        let input = &row["input"];
        let output = &row["output"];
        assert_eq!(
            globals.matrix_b40054().to_vec(),
            output["matrix_b40054"].as_array().unwrap().iter().map(w).collect::<Vec<_>>()
        );
        let mut s = State::unbound_allocation();
        for off in [0x208u32, 0x20c, 0x210, 0x240, 0x244, 0x248, 0x24c, 0x250, 0x254, 0x2b8, 0x2bc] {
            s.put(off, w(&input[format!("0x{off:x}")]));
        }
        //Same setup as the harness: a real 6FE3B0 at the initial angles.
        s.put_byte(0x214, 0);
        let (yaw0, pitch0) = (s.word(0x2bc).unwrap(), s.word(0x2b8).unwrap());
        s.put(0x2b8, (-9.0f32).to_bits());
        rotate(&mut s, &globals, yaw0, pitch0, pc).unwrap();
        s.put_byte(0x214, input.get("locked").map(w).unwrap_or(0) as u8);
        let before = output["before"].as_object().unwrap();
        for (k, v) in before {
            let off = u32::from_str_radix(k.trim_start_matches("0x"), 16).unwrap();
            if let Ok(word) = s.word(off) {
                assert_eq!(word, w(v), "setup {k} {row}");
            }
        }
        rotate_about_target_prefix_6feba0(&mut s, &globals, w(&input["yaw"]), w(&input["pitch"]), pc).unwrap();
        for (k, v) in output["after"].as_object().unwrap() {
            let off = u32::from_str_radix(k.trim_start_matches("0x"), 16).unwrap();
            let native = w(v);
            match s.word(off) {
                Ok(word) => assert_eq!(word, native, "{k} {}", row["precision"]),
                //Unwritten Rust fields must still be the harness poison.
                Err(_) => assert_eq!(native, 0xa5a5_a5a5, "{k} written natively but not in Rust"),
            }
        }
        assert_eq!(output["trace"], serde_json::json!([["ni_update", null]]));
    }
}
