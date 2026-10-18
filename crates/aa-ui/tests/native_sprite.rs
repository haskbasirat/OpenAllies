//! Replays research/phase8-ui/native-sprite.json: the original UI sprite
//! vertex grid (SetRect 0x73EED0 / 0x740900, SetUV 0x73EF70, counts
//! 0x73F750, build 0x73FF60 with 0x73F1B0 and 0x73F290) under Unicorn at
//! PC24 and PC53 for the address modes 0..3.

use aa_ui::layout::Precision;
use aa_ui::sprite::{SpriteInput, sprite_grid};
use serde_json::Value;
use std::path::PathBuf;

fn bits(v: &Value) -> f32 {
    f32::from_bits(v.as_u64().unwrap() as u32)
}

fn fnv(values: impl IntoIterator<Item = u32>) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for v in values {
        for b in v.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

#[test]
fn sprite_grid_replays_the_native_cases() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-ui/native-sprite.json");
    let d: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
    let far = d["far"].as_f64().unwrap() as f32;
    let zstep = bits(&d["zstep_bits"]);
    let colour: Vec<f32> = d["colour"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap() as f32).collect();
    let mut checked = 0;
    let mut vertices = 0usize;
    for c in d["cases"].as_array().unwrap() {
        let precision = if c["precision"] == "pc24" { Precision::Pc24 } else { Precision::Pc53 };
        let arr = |k: &str| -> [f32; 4] { std::array::from_fn(|i| bits(&c[k][i])) };
        let input = SpriteInput {
            rect: arr("rect_bits"),
            uv: arr("uv_bits"),
            tex: c["tex"].as_array().map(|t| (t[0].as_u64().unwrap() as u32, t[1].as_u64().unwrap() as u32)),
            modes: [c["modes"][0].as_u64().unwrap() as u32, c["modes"][1].as_u64().unwrap() as u32],
            layer: c["layer"].as_u64().unwrap() as u32,
            color: [colour[0], colour[1], colour[2], colour[3]],
            color2: [colour[0], colour[1], colour[2]],
            screen: (c["screen"][0].as_f64().unwrap() as f32, c["screen"][1].as_f64().unwrap() as f32),
            ip: (c["ip"][0].as_f64().unwrap() as f32, c["ip"][1].as_f64().unwrap() as f32),
            far,
            zstep,
        };
        let ((nx, ny), verts) = sprite_grid(&input, precision);
        let ctx = || serde_json::to_string(&c["modes"]).unwrap() + &serde_json::to_string(&c["rect_bits"]).unwrap() + &serde_json::to_string(&c["tex"]).unwrap() + c["precision"].as_str().unwrap();
        assert_eq!([nx, ny], [c["counts"][0].as_u64().unwrap() as u32, c["counts"][1].as_u64().unwrap() as u32], "{}", ctx());
        assert_eq!(verts.len() as u64, c["n"].as_u64().unwrap(), "{}", ctx());
        let words: Vec<u32> = verts
            .iter()
            .flat_map(|v| [v.pos[0].to_bits(), v.pos[1].to_bits(), v.pos[2].to_bits(), v.c0, v.c1, v.uv[0].to_bits(), v.uv[1].to_bits(), 0])
            .collect();
        if fnv(words.iter().copied()) != c["fnv"].as_u64().unwrap() {
            // locate the first differing kept vertex
            let kept = c["vertices"].as_array().unwrap();
            let n = verts.len();
            let idx: Vec<usize> = if n <= 32 { (0..n).collect() } else { (0..16).chain(n - 16..n).collect() };
            for (k, i) in idx.iter().enumerate() {
                let want: Vec<u32> = kept[k].as_array().unwrap().iter().map(|v| v.as_u64().unwrap() as u32).collect();
                assert_eq!(&words[i * 8..i * 8 + 8], &want[..], "vertex {i} {}", ctx());
            }
            panic!("digest differs in an unkept vertex {}", ctx());
        }
        vertices += verts.len();
        checked += 1;
    }
    assert!(checked > 1000, "{checked}");
    assert!(vertices > 10_000, "{vertices}");
}
