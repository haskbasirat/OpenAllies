//! Replays research/development10/hud-orders/native-press-pick.json: the
//! original 0x5E2B50 / 0x5E1680 / 0x5EDD30 / 0x5EFA80 executed under Unicorn
//! on laboratory view-element trees (original and v14 images, three x87
//! precisions, identical), against aa_ui::press_pick::Arena.

use aa_ui::press_pick::{Arena, PickVe, PickWidget};
use serde_json::Value;

fn arena(input: &Value) -> Arena {
    let opt = |v: &Value| v.as_u64().map(|x| x as usize);
    let rect = |v: &Value| -> [f32; 4] {
        let a = v.as_array().unwrap();
        [0, 1, 2, 3].map(|i| f32::from_bits(a[i].as_u64().unwrap() as u32))
    };
    Arena {
        widgets: input["widgets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| PickWidget {
                parent_08: opt(&w["parent_08"]),
                flags_24: w["flags_24"].as_u64().unwrap() as u32,
                vt_7c_true: w["vt_7c_true"].as_bool().unwrap(),
                top_14: opt(&w["top_14"]),
            })
            .collect(),
        ves: input["ves"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| PickVe {
                group_d0: v["group_d0"].as_bool(),
                visible_88: v["flags_88"].as_u64().unwrap() & 4 != 0,
                outer_20: rect(&v["outer_20"]),
                client_54: rect(&v["client_54"]),
                owner_0c: opt(&v["owner_0c"]),
                children: v["children"].as_array().unwrap().iter().map(|c| c.as_u64().unwrap() as usize).collect(),
            })
            .collect(),
    }
}

#[test]
fn press_pick_matches_native_rows() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/development10/hud-orders/native-press-pick.json");
    let doc: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(doc["precision_independent"], Value::Bool(true));
    let (mut picks, mut hits, mut admits) = (0, 0, 0);
    for row in doc["rows"].as_array().unwrap() {
        let input = &row["input"];
        let a = arena(input);
        for (q, want) in input["queries"].as_array().unwrap().iter().zip(row["output"]["picks"].as_array().unwrap()) {
            let ve = q["ve"].as_u64().unwrap() as usize;
            let x = f32::from_bits(q["x"].as_u64().unwrap() as u32);
            let y = f32::from_bits(q["y"].as_u64().unwrap() as u32);
            let flag = q["flag"].as_u64().unwrap() != 0;
            let got = a.pick_38(ve, x, y, flag);
            assert_eq!(got, want.as_u64().map(|w| w as usize), "{q} in {input}");
            picks += 1;
            hits += usize::from(got.is_some());
        }
        for (q, want) in input["admits"].as_array().unwrap().iter().zip(row["output"]["admits"].as_array().unwrap()) {
            let got = a.admits_5efa80(
                q["widget"].as_u64().unwrap() as usize,
                q["focus_8c"].as_u64().map(|f| f as usize),
                q["root_74"].as_u64().unwrap() as usize,
            );
            assert_eq!(u64::from(got), want.as_u64().unwrap(), "{q}");
            admits += 1;
        }
    }
    assert!(picks >= 3000 && hits >= 400 && admits >= 1500, "{picks} {hits} {admits}");
}
