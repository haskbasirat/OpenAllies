//! Replays the Unicorn fixtures of `research/phase8-ui/native_decal.py`
//! (original 0x5E6AB0, 0x5E7790, 0x5E7000) against the Rust port.

use aa_ui::layout::{Precision, Rect};
use aa_ui::sprite::{decal_fit, directional_pieces, nine_slice_rects};
use serde_json::Value;
use std::path::PathBuf;

fn fixtures() -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-ui/native-decal.json");
    serde_json::from_str(&std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap()
}

fn bits4(v: &Value) -> [u32; 4] {
    let a = v.as_array().unwrap();
    [0, 1, 2, 3].map(|i| a[i].as_u64().unwrap() as u32)
}

fn rect(v: &Value) -> Rect {
    Rect::from_array(bits4(v).map(f32::from_bits))
}

fn size(v: &Value) -> Option<(u32, u32)> {
    let a = v.as_array()?;
    Some((a[0].as_u64().unwrap() as u32, a[1].as_u64().unwrap() as u32))
}

fn prec(v: &Value) -> Precision {
    if v == "pc24" { Precision::Pc24 } else { Precision::Pc53 }
}

#[test]
fn nine_slice_piece_rects_match_native() {
    let d = fixtures();
    let rows = d["nine_slice"].as_array().unwrap();
    assert!(rows.len() >= 50);
    for row in rows {
        let sizes: Vec<Option<(u32, u32)>> = row["sizes"].as_array().unwrap().iter().map(size).collect();
        let sizes: [Option<(u32, u32)>; 6] = std::array::from_fn(|i| sizes[i]);
        let got = nine_slice_rects(rect(&row["rect_bits"]), sizes);
        for (i, want) in row["pieces_bits"].as_array().unwrap().iter().enumerate() {
            assert_eq!(got[i].to_array().map(f32::to_bits), bits4(want), "piece {i} {row}");
        }
    }
}

#[test]
fn directional_piece_rects_match_native() {
    let d = fixtures();
    for row in d["directional"].as_array().unwrap() {
        let got = directional_pieces(rect(&row["rect_bits"]), size(&row["ends"]), row["horizontal"] == 1);
        for (i, want) in row["pieces_bits"].as_array().unwrap().iter().enumerate() {
            assert_eq!(got[i].1.to_array().map(f32::to_bits), bits4(want), "piece {i} {row}");
        }
    }
}

#[test]
fn decal_fit_matches_native() {
    let d = fixtures();
    for row in d["fit"].as_array().unwrap() {
        let got = decal_fit(
            rect(&row["rect_bits"]),
            size(&row["tex"]).unwrap(),
            row["tiled_x"] == 1,
            row["tiled_y"] == 1,
            row["stretch"] == 1,
            prec(&row["precision"]),
        );
        assert_eq!(got.map(f32::to_bits), bits4(&row["uv_bits"]), "{row}");
    }
}
