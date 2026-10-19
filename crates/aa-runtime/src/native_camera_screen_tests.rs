//! Replays research/phase8-camera-input/native-camera-screen.json (ORIGINAL
//! AA.exe under Unicorn at PC24/PC53/PC64).
use super::*;
use crate::native_render_defaults::SharedSet;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../../research/phase8-camera-input/native-camera-screen.json")).unwrap()
}
fn precision(row: &Value) -> Precision {
    match row["precision"].as_str().unwrap() {
        "Single24" => Precision::Single24,
        "Double53" => Precision::Double53,
        "Extended64" => Precision::Extended64,
        other => panic!("{other}"),
    }
}
fn pair(v: &Value) -> [i32; 2] {
    [v[0].as_i64().unwrap() as i32, v[1].as_i64().unwrap() as i32]
}
fn words(v: &Value) -> Vec<u32> {
    v.as_array().unwrap().iter().map(|w| w.as_u64().unwrap() as u32).collect()
}
fn flat(p: &ScreenPolygon) -> Vec<u32> {
    p.vertices().iter().flatten().copied().collect()
}
fn tracker(row: &Value) -> Tracker {
    Tracker { current: pair(&row["current"]), left_press: pair(&row["press"]) }
}

#[test]
fn box_vertices_match_native_5cca80_rows() {
    let doc = fixture();
    let rows = doc["sections"]["update_5cca80"].as_array().unwrap();
    assert_eq!(rows.len(), 360);
    for row in rows {
        let marks: Vec<[u32; 3]> = (0..4).map(|i| std::array::from_fn(|j| 0x5a5a_0000 + 3 * i + j as u32)).collect();
        let mut p = ScreenPolygon::from_887f40(&marks, None, None, Vec::new()).unwrap();
        p.flags_20 = 0x4440;
        update_5cca80(&mut p, tracker(row), pair(&row["viewport"]), precision(row)).unwrap();
        assert_eq!(flat(&p), words(&row["vertices"]), "{row}");
        assert_eq!(u32::from(p.flags_20()), row["word_20"].as_u64().unwrap() as u32 & 0xffff);
    }
}

///Native bytes the Rust owner leaves unreadable must be the harness's
///uninitialized operator-new fill; every readable byte must be equal.
///The material id word (+20) is compared relative to the process counter.
fn compare_property(installed: &Installed, native: &Value, skip: &[u32]) {
    assert_eq!(u64::from(installed.kind().unwrap().native_vtable_identity()), native["vtable"].as_u64().unwrap());
    for (offset, word) in native["words"].as_object().unwrap() {
        let offset = u32::from_str_radix(offset.trim_start_matches("0x"), 16).unwrap();
        if skip.contains(&offset) {
            continue;
        }
        let bytes = (word.as_u64().unwrap() as u32).to_le_bytes();
        for (i, b) in bytes.into_iter().enumerate() {
            match installed.byte(offset + i as u32) {
                Ok(v) => assert_eq!(v, b, "+{:X}", offset + i as u32),
                Err(_) => assert_eq!(b, 0xcd, "+{:X} has a native writer", offset + i as u32),
            }
        }
    }
}

#[test]
fn box_polygon_properties_and_camera_list_match_native_5cd850_5cca30() {
    let doc = fixture();
    let rows = doc["sections"]["box_5cd850_5cca30"].as_array().unwrap();
    assert_eq!(rows.len(), 21);
    for row in rows {
        let e = &row["enter"];
        let n = &e["polygon"];
        let mut defaults = SharedSet::from_fresh_native_process().unwrap();
        defaults.initialize_properties_prefix_8c3ed0().unwrap();
        let state = defaults.new_property_set_895550().unwrap();
        let before = defaults.material_next_id();
        let seeds: [u32; 3] = words(&row["seeds"]).try_into().unwrap();
        let (factory, globals) = defaults.factory_and_globals();
        let p = box_5cd850(seeds, state, factory, globals, tracker(row), pair(&row["viewport"]), precision(row)).unwrap();
        //The material constructor draws one process id, as B323C0 did.
        assert_eq!(defaults.material_next_id() - before, (e["material_after"].as_u64().unwrap() - e["material_before"].as_u64().unwrap()) as u32);
        assert_eq!(flat(&p), words(&n["vertices"]));
        assert_eq!(p.vertices().len(), n["count"].as_u64().unwrap() as usize);
        assert_eq!(p.colors().unwrap().iter().flatten().copied().collect::<Vec<_>>(), words(&n["colors"]));
        assert!(p.texture_coordinates().is_none() && n["uv"] == 0);
        //887F40 +20 = 0, then 5CCA80's |= 1 (5CD7E0's comes with the list).
        assert_eq!(u64::from(p.flags_20()), n["flags_20"].as_u64().unwrap());
        assert_eq!((p.word_24(), p.word_28()), (0, 0));
        assert_eq!((n["word_24"].as_u64(), n["word_28"].as_u64()), (Some(0), Some(0)));
        for (slot, native) in n["slots"].as_array().unwrap().iter().enumerate() {
            match (p.property(slot).unwrap(), native.is_null()) {
                (Slot::Shared(_), true) => {}
                (Slot::Installed(i), false) => {
                    compare_property(i, native, if slot == 3 { &[0x20] } else { &[] });
                    if slot == 3 {
                        assert_eq!(i.word(0x20).unwrap(), before);
                        assert_eq!(native["words"]["0x20"], e["material_before"]);
                    }
                }
                _ => panic!("slot {slot} differs from native installation"),
            }
        }
        //Camera+168: 5CD730 placement, then 874350 on the CURRENT camera.
        let polygon: Polygon = Arc::new(Mutex::new(p));
        let dummy = || -> Polygon { Arc::new(Mutex::new(ScreenPolygon::from_887f40(&[], None, None, Vec::new()).unwrap())) };
        let mut list = List::default();
        for used in row["list"].as_array().unwrap() {
            list.slots.push(used.as_bool().unwrap().then(dummy));
        }
        let index = list.attach_5cd7e0(&polygon).unwrap();
        let shape = |l: &List| -> Value {
            serde_json::json!(l.slots().iter().map(|s| match s {
                Some(q) if Arc::ptr_eq(q, &polygon) => Value::from("box"),
                Some(_) => Value::from("dummy"),
                None => Value::Null,
            }).collect::<Vec<_>>())
        };
        assert_eq!(shape(&list), e["camera"]["slots"]);
        assert_eq!(e["camera"]["slots"][index], "box");
        assert_eq!(list.count() as u64, e["camera"]["count"].as_u64().unwrap());
        assert_eq!(polygon.lock().unwrap().flags_20(), 1);
        let x = &row["exit"];
        if row["moved_current_camera"].as_bool().unwrap() {
            let mut current = List::default();
            current.slots.push(Some(dummy()));
            current.detach_874350(&polygon);
            assert_eq!(shape(&current), x["current_camera"]["slots"]);
            assert_eq!(shape(&list), x["original_camera"]["slots"]);
        } else {
            list.detach_874350(&polygon);
            assert_eq!(shape(&list), x["original_camera"]["slots"]);
            assert_eq!(list.count() as u64, x["original_camera"]["count"].as_u64().unwrap());
        }
        //Releasing +4 destroys the polygon only when no camera still holds it.
        let held_elsewhere = Arc::strong_count(&polygon) > 1;
        assert_eq!(held_elsewhere, x["refcount"].as_u64().unwrap() > 0);
        assert_eq!(x["destroyed"].as_array().unwrap().iter().any(|d| d == true), !held_elsewhere);
    }
}

#[test]
fn facing_lock_rows_are_the_camera_byte_only() {
    let doc = fixture();
    for row in doc["sections"]["facing_5ccd60_5ccda0"].as_array().unwrap() {
        assert_eq!(row["enter"]["lock_214"], 1);
        assert_eq!(row["exit"]["lock_214"], 0);
        assert_eq!(row["enter"]["arrow"], serde_json::json!([[0, true]]));
        assert_eq!(row["exit"]["arrow"], serde_json::json!([[0, true]]));
    }
}
