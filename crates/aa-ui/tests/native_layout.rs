//! Replays every Unicorn fixture produced by
//! `research/phase8-ui/native_layout.py` against the Rust port.

use aa_ui::layout::{
    Bounds, Precision, Rect, Reference, Screen, VeLayout, layout_ve, parse_bounds, resolve_bounds,
};
use aa_ui::state::{ATTRIBUTE_KEYWORDS, WHEN_KEYWORDS, state_bits, ve_visible};
use serde_json::Value;
use std::path::PathBuf;

fn fixtures() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-ui/native-layout.json");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

fn f(v: &Value) -> f32 {
    f32::from_bits(v.as_u64().unwrap() as u32)
}
fn bits4(v: &Value) -> [u32; 4] {
    let a = v.as_array().unwrap();
    [0, 1, 2, 3].map(|i| a[i].as_u64().unwrap() as u32)
}
fn floats4(v: &Value) -> [f32; 4] {
    bits4(v).map(f32::from_bits)
}
fn prec(v: &Value) -> Precision {
    match v.as_str().unwrap() {
        "pc24" => Precision::Pc24,
        "pc53" => Precision::Pc53,
        other => panic!("{other}"),
    }
}

#[test]
fn keyword_tables_match_native() {
    let d = fixtures();
    let when: Vec<(String, u32)> = d["when_table"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["keyword"].as_str().unwrap().to_string(), r["bit"].as_u64().unwrap() as u32))
        .collect();
    let ours: Vec<(String, u32)> = WHEN_KEYWORDS.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    assert_eq!(when, ours);
    let attrs: Vec<(String, u32)> = d["label_attribute_table"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["keyword"].as_str().unwrap().to_string(), r["bit"].as_u64().unwrap() as u32))
        .collect();
    let ours: Vec<(String, u32)> = ATTRIBUTE_KEYWORDS.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    assert_eq!(attrs, ours);
}

#[test]
fn bounds_parse_matches_native() {
    let d = fixtures();
    let rows = d["parse"].as_array().unwrap();
    assert!(rows.len() > 1000);
    for row in rows {
        let input: Vec<&str> = row["input"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
        let mut b = Bounds { raw: floats4(&row["prior_bits"]), flags: 0xdead_beef, ..Default::default() };
        let ok = parse_bounds(&mut b, input[0], input[1], input[2], input[3]);
        assert_eq!(ok as u64, row["ok"].as_u64().unwrap(), "{input:?}");
        assert_eq!(b.raw.map(f32::to_bits), bits4(&row["ltrb_bits"]), "{input:?}");
        assert_eq!(b.flags as u64, row["flags"].as_u64().unwrap(), "{input:?}");
    }
}

#[test]
fn bounds_resolve_matches_native() {
    let d = fixtures();
    let rows = d["resolve"].as_array().unwrap();
    assert!(rows.len() > 20000);
    let mut pc24_vs_pc53_differ = 0;
    for row in rows {
        let p = prec(&row["precision"]);
        let mut b = Bounds {
            raw: floats4(&row["raw_bits"]),
            flags: row["flags"].as_u64().unwrap() as u32,
            ..Default::default()
        };
        // The fixture poisons the outputs; the native code always writes the
        // rect, and writes clip amounts only when clipping. Compare the clip
        // only when it was written.
        let parent = Rect::from_array(floats4(&row["parent_bits"]));
        let ok = resolve_bounds(&mut b, &parent, f(&row["sx_bits"]), f(&row["sy_bits"]), row["clip"].as_u64().unwrap() != 0, p);
        assert_eq!(ok as u64, row["ok"].as_u64().unwrap(), "{row}");
        assert_eq!(b.rect.to_array().map(f32::to_bits), bits4(&row["out_bits"]), "{row}");
        let clip = bits4(&row["clip_bits"]);
        if clip != [0x7fc0_0001; 4] {
            assert_eq!(b.clip.map(f32::to_bits), clip, "{row}");
        } else {
            assert_eq!(row["clip"].as_u64().unwrap(), 0, "{row}");
        }
        assert_eq!(parent.to_array().map(f32::to_bits), bits4(&row["parent_after_bits"]));
        if p == Precision::Pc24 {
            let mut b53 = Bounds {
                raw: floats4(&row["raw_bits"]),
                flags: row["flags"].as_u64().unwrap() as u32,
                ..Default::default()
            };
            resolve_bounds(&mut b53, &parent, f(&row["sx_bits"]), f(&row["sy_bits"]), row["clip"].as_u64().unwrap() != 0, Precision::Pc53);
            if b53.rect != b.rect {
                pc24_vs_pc53_differ += 1;
            }
        }
    }
    // The precision setting is observable in this arithmetic.
    assert!(pc24_vs_pc53_differ > 0);
}

#[test]
fn ve_layout_matches_native() {
    let d = fixtures();
    let rows = d["ve_layout"].as_array().unwrap();
    assert!(!rows.is_empty());
    for row in rows {
        let p = prec(&row["precision"]);
        let s = row["screen"].as_array().unwrap();
        let screen = Screen { width: s[0].as_i64().unwrap() as i32, height: s[1].as_i64().unwrap() as i32 };
        let mut laid: Vec<VeLayout> = Vec::new();
        for (i, ve) in row["chain"].as_array().unwrap().iter().enumerate() {
            let mut v = VeLayout {
                outer: Bounds { raw: floats4(&ve["outer_bits"]), flags: ve["outer_flags"].as_u64().unwrap() as u32, ..Default::default() },
                client: Bounds { raw: floats4(&ve["client_bits"]), flags: ve["client_flags"].as_u64().unwrap() as u32, ..Default::default() },
                flags: ve["ve_flags"].as_u64().unwrap() as u32,
            };
            layout_ve(&mut v, laid.last(), screen, Reference::default(), p);
            let want = &row["result"][i];
            assert_eq!(v.outer.rect.to_array().map(f32::to_bits), bits4(&want["outer_rect_bits"]), "{row}");
            assert_eq!(v.outer.clip.map(f32::to_bits), bits4(&want["outer_clip_bits"]), "{row}");
            assert_eq!(v.client.rect.to_array().map(f32::to_bits), bits4(&want["client_rect_bits"]), "{row}");
            assert_eq!(v.client.clip.map(f32::to_bits), bits4(&want["client_clip_bits"]), "{row}");
            assert_eq!(v.flags as u64, want["ve_flags"].as_u64().unwrap(), "{row}");
            laid.push(v);
        }
    }
}

#[test]
fn state_and_visibility_match_native() {
    let d = fixtures();
    for row in d["state"].as_array().unwrap() {
        let flags = row["widget_flags"].as_u64().unwrap() as u32;
        let when = row["when"].as_u64().unwrap() as u32;
        assert_eq!(state_bits(flags) as u64, row["state"].as_u64().unwrap());
        let vis = ve_visible(
            when,
            flags,
            row["render_threshold"].as_i64().unwrap() as i32,
            row["render_level"].as_i64().unwrap() as i32,
        );
        assert_eq!(vis as u64, row["visible"].as_u64().unwrap(), "{row}");
    }
}
