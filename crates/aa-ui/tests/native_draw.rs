//! Replays research/phase8-ui/native-draw.json: the original label rebuild
//! 0x735B90 (PreProcessText 0x734120, GetRowStartingXPixels 0x730130,
//! Parse 0x733B10, quad builder 0x730F00, AddQuad 0x72FA00) executed under
//! Unicorn at PC24 and PC53 over the synthetic glyph tables of
//! native_draw.py (`table` / `specs`).

use aa_ui::layout::Precision;
use aa_ui::rdraw::{DrawParams, RDraw};
use aa_ui::rtext::{Glyph, GlyphTable, TextEnv};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

const NAMES: [&str; 4] = ["small", "big", "base7", "noshadow"];

struct Font {
    cells: HashMap<u32, (Glyph, (u16, u16))>,
    baseline: i32,
    fixed: i32,
}

struct Tables {
    fonts: Vec<Font>,
}

/// native_draw.py `table(h, shadow, seed)`.
fn table(h: u16, shadow: bool, seed: u32, baseline: i32) -> Font {
    let mut g = HashMap::new();
    let cell = |w: u16, h: u16| Glyph { w, h, shadow, outline: false };
    for c in 32u32..384 {
        let mut w = 3 + (c * 7 + 3 + seed) % 11;
        if c == 32 {
            w = 5;
        }
        g.insert(c, (cell(w as u16, h), (((c * 13) % 480) as u16, ((c * 7) % 480) as u16)));
    }
    g.insert('W' as u32, (cell(17, h), (400, 3)));
    for (c, w, hh) in [(128u32, 17u16, 21u16), (129, 15, 15), (133, 9, 30)] {
        g.insert(c, (cell(w, hh), ((300 + c) as u16, (100 + c) as u16)));
    }
    let fixed = g.values().map(|(x, _)| x.w as i32).max().unwrap_or(0) + 2;
    Font { cells: g, baseline, fixed }
}

impl Tables {
    fn new() -> Self {
        Tables { fonts: vec![table(17, true, 0, 0), table(23, true, 4, 0), table(19, true, 2, 7), table(15, false, 1, 0)] }
    }
    fn get(&self, font: u32) -> Option<&Font> {
        (font as usize).checked_sub(1).and_then(|i| self.fonts.get(i))
    }
}

impl GlyphTable for Tables {
    fn glyph(&self, font: u32, c: u32) -> Option<Glyph> {
        self.get(font)?.cells.get(&c).map(|x| x.0)
    }
    fn fixed(&self, font: u32) -> i32 {
        self.get(font).map(|f| f.fixed).unwrap_or(0)
    }
    fn font_by_name(&self, name: &str) -> u32 {
        NAMES.iter().position(|n| *n == name).map(|i| i as u32 + 1).unwrap_or(0)
    }
    fn atlas(&self, font: u32, c: u32) -> (u16, u16) {
        self.get(font).and_then(|f| f.cells.get(&c)).map(|x| x.1).unwrap_or((0, 0))
    }
    fn baseline(&self, font: u32) -> i32 {
        self.get(font).map(|f| f.baseline).unwrap_or(0)
    }
    fn own_color(&self, c: u32) -> bool {
        matches!(c, 128 | 133)
    }
}

fn bits(v: &Value) -> f32 {
    f32::from_bits(v.as_u64().unwrap() as u32)
}

fn b4(v: &Value) -> [f32; 4] {
    [bits(&v[0]), bits(&v[1]), bits(&v[2]), bits(&v[3])]
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
fn label_draw_layout_replays_the_native_cases() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-ui/native-draw.json");
    let d: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
    assert_eq!(d["stats"]["mismatch"], 0);
    let tables = Tables::new();
    let mut checked = 0;
    let mut quads = 0usize;
    for c in d["cases"].as_array().unwrap() {
        assert!(c.get("hang").is_none());
        let precision = if c["precision"] == "pc24" { Precision::Pc24 } else { Precision::Pc53 };
        let screen = &c["screen"];
        let env = TextEnv {
            precision,
            default_font: 1,
            undisplayable: 149,
            fallback: 35,
            standin: 116,
            sets: vec![(32, 383, 149, 116), (0x4e00, 0x9fff, 152, 116)],
            world_scale: 25.0,
            baseline_scale: 30.0,
            screen: (screen[0].as_f64().unwrap() as f32, screen[1].as_f64().unwrap() as f32),
            reference: (1024.0, 768.0),
            ellipsis: "...".into(),
        };
        let params = DrawParams {
            font: tables.font_by_name(c["font"].as_str().unwrap()),
            color: b4(&c["colour_bits"]),
            highlight: b4(&c["hl_bits"]),
            origin: (bits(&c["origin_bits"][0]), bits(&c["origin_bits"][1])),
            area: b4(&c["area_bits"]),
            justify: c["justify"].as_i64().unwrap() as i32,
            multiline: c["multiline"] == 1,
            fixed: c["fixed"] == 1,
            debug_tint: c["debug_tint"] == 1,
        };
        let rd = RDraw { env: &env, table: &tables };
        let got = rd.rebuild(c["text"].as_str().unwrap(), &params);
        let ctx = || format!("{}", serde_json::to_string(c).unwrap().chars().take(600).collect::<String>());
        assert!(got.unsupported.is_none(), "{}", ctx());
        let want_text: Vec<u32> = c["processed"].as_str().unwrap().encode_utf16().map(|u| u as u32).collect();
        assert_eq!(got.processed, want_text, "processed {}", ctx());
        let rows: Vec<[u32; 5]> =
            got.rows.iter().map(|r| [r.w.to_bits(), r.h.to_bits(), r.first, r.last, r.start_x.to_bits()]).collect();
        let want_rows: Vec<[u32; 5]> = c["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| std::array::from_fn(|k| r[k].as_u64().unwrap() as u32))
            .collect();
        assert_eq!(rows, want_rows, "rows {}", ctx());
        assert_eq!([got.size[0].to_bits(), got.size[1].to_bits()], [bits(&c["size_bits"][0]).to_bits(), bits(&c["size_bits"][1]).to_bits()], "size {}", ctx());
        assert_eq!(got.flags.map(|f| f as u64), [c["flags"][0].as_u64().unwrap(), c["flags"][1].as_u64().unwrap()], "flags {}", ctx());
        assert_eq!([got.pen[0].to_bits(), got.pen[1].to_bits()], [bits(&c["pen_bits"][0]).to_bits(), bits(&c["pen_bits"][1]).to_bits()], "pen {}", ctx());
        let errors: Vec<String> = c["errors"].as_array().unwrap().iter().map(|e| e.as_str().unwrap().to_string()).collect();
        assert_eq!(got.errors, errors, "errors {}", ctx());
        assert_eq!(got.glyphs.len() as u64, c["n_glyphs"].as_u64().unwrap(), "glyph count {}", ctx());
        let g = got.glyphs.iter().flat_map(|r| {
            [r.c, r.row, r.idx, r.text_idx as u32, r.rect[0].to_bits(), r.rect[1].to_bits(), r.rect[2].to_bits(), r.rect[3].to_bits()]
        });
        assert_eq!(fnv(g), c["glyph_fnv"].as_u64().unwrap(), "glyph records {:?} {}", got.glyphs, ctx());
        assert_eq!(got.quads.len() as u64, c["n_quads"].as_u64().unwrap(), "quad count {:?} {}", got.quads, ctx());
        let q = got.quads.iter().flat_map(|q| {
            let mut v = vec![q.highlight as u32, q.font - 1];
            v.extend(q.geo.iter().map(|z| z.to_bits()));
            v.extend(q.color.iter().map(|z| z.to_bits()));
            v
        });
        assert_eq!(fnv(q), c["quad_fnv"].as_u64().unwrap(), "quads {:?} {}", got.quads, ctx());
        quads += got.quads.len();
        checked += 1;
    }
    assert_eq!(checked, 14604);
    assert!(quads > 100_000, "{quads}");
}
