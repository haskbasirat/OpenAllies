//! Replays research/phase8-ui/native-text.json: the original GetTextExtent
//! (0x7352C0), its reference-unit wrapper (0x735C00) and the ellipsis
//! truncation (0x735D50) executed under Unicorn at PC24 and PC53 over the
//! synthetic glyph tables of native_text.py (font_specs / glyph_table).

use aa_ui::layout::Precision;
use aa_ui::rtext::{Glyph, GlyphTable, Measured, RText, TextEnv};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

const SMALL: u32 = 1;
const BIG: u32 = 2;
const NOSHADOW: u32 = 3;
const OUTLINE: u32 = 4;

struct Tables {
    fonts: HashMap<u32, HashMap<u32, Glyph>>,
}

fn glyph_table(seed_h: u16, shadow: bool) -> HashMap<u32, Glyph> {
    let mut g = HashMap::new();
    for c in 32u32..384 {
        let mut w = 3 + (c * 7 + 3) % 11;
        if c == 32 {
            w = 5;
        }
        g.insert(c, Glyph { w: w as u16, h: seed_h, shadow, outline: false });
    }
    g.insert('W' as u32, Glyph { w: 17, h: seed_h, shadow, outline: false });
    g.insert('M' as u32, Glyph { w: 15, h: seed_h, shadow, outline: false });
    for c in [128u32, 129, 133] {
        g.insert(c, Glyph { w: 17, h: 17, shadow, outline: false });
    }
    g
}

impl Tables {
    fn new() -> Self {
        let small = glyph_table(17, true);
        let big = small.iter().map(|(c, g)| (*c, Glyph { w: g.w + 4, h: 23, shadow: true, outline: false })).collect();
        let mut noshadow: HashMap<u32, Glyph> =
            small.iter().map(|(c, g)| (*c, Glyph { w: g.w, h: 15, shadow: false, outline: false })).collect();
        noshadow.remove(&149);
        let outline = small.iter().map(|(c, g)| (*c, Glyph { w: g.w + 1, h: 18, shadow: true, outline: true })).collect();
        let mut fonts = HashMap::new();
        fonts.insert(SMALL, small);
        fonts.insert(BIG, big);
        fonts.insert(NOSHADOW, noshadow);
        fonts.insert(OUTLINE, outline);
        Tables { fonts }
    }
}

impl GlyphTable for Tables {
    fn glyph(&self, font: u32, c: u32) -> Option<Glyph> {
        self.fonts.get(&font)?.get(&c).copied()
    }
    fn fixed(&self, font: u32) -> i32 {
        self.fonts.get(&font).map(|g| g.values().map(|x| x.w as i32).max().unwrap_or(0) + 2).unwrap_or(0)
    }
    fn font_by_name(&self, name: &str) -> u32 {
        match name {
            "small" => SMALL,
            "big" => BIG,
            "noshadow" => NOSHADOW,
            "outline" => OUTLINE,
            _ => 0,
        }
    }
}

fn font_id(name: &str) -> u32 {
    match name {
        "small" => SMALL,
        "big" => BIG,
        "noshadow" => NOSHADOW,
        "outline" => OUTLINE,
        _ => 0,
    }
}

fn bits(v: &Value) -> f32 {
    f32::from_bits(v.as_u64().unwrap() as u32)
}

#[test]
fn text_measure_replays_the_native_cases() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-ui/native-text.json");
    let d: Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
    assert_eq!(d["stats"]["mismatch"], 0);
    let tables = Tables::new();
    let mut checked = 0;
    let mut hangs = 0;
    for c in d["cases"].as_array().unwrap() {
        let precision = if c["precision"] == "pc24" { Precision::Pc24 } else { Precision::Pc53 };
        let screen = &c["screen"];
        let env = TextEnv {
            precision,
            default_font: SMALL,
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
        let rt = RText { env: &env, table: &tables };
        let text = c["text"].as_str().unwrap();
        let native = &c["native"];
        match c["fn"].as_str().unwrap() {
            "0x7352C0" => {
                let font = font_id(c["font"].as_str().unwrap());
                let boxb = &c["box_bits"];
                let got = rt.extent(
                    text,
                    (bits(&boxb[0]), bits(&boxb[1])),
                    font,
                    c["px"] == 1,
                    c["fixed"] == 1,
                    c["trail"] == 1,
                    bits(&c["wrap_bits"]),
                    c["clip"] == 1,
                );
                if native.get("hang").is_some() {
                    assert_eq!(got, Measured::Hang, "{c}");
                    hangs += 1;
                } else {
                    let want = Measured::Done {
                        count: native["ret"].as_i64().unwrap() as i32,
                        size: [bits(&native["size_bits"][0]), bits(&native["size_bits"][1])],
                        rows: native["rows"].as_i64().map(|r| r as i32),
                    };
                    assert_eq!(got, want, "{c}");
                }
            }
            "0x735C00" => {
                let got = rt.ref_extent(text, SMALL, c["fixed"] == 1, c["trail"] == 1, bits(&c["width_bits"]));
                let want = Measured::Done {
                    count: native["ret"].as_i64().unwrap() as i32,
                    size: [bits(&native["size_bits"][0]), bits(&native["size_bits"][1])],
                    rows: native["rows"].as_i64().map(|r| r as i32),
                };
                assert_eq!(got, want, "{c}");
            }
            "0x735D50" => {
                let font = font_id(c["font"].as_str().unwrap());
                let mut cache = None;
                let (t, out) = rt.ellipsis(text, bits(&c["w_bits"]), 0.0, font, &mut cache);
                assert_eq!((t as i64, out.as_str()), (native["ret"].as_i64().unwrap(), native["out"].as_str().unwrap()), "{c}");
            }
            other => panic!("{other}"),
        }
        checked += 1;
    }
    assert_eq!(checked, 18328);
    assert_eq!(hangs, 324);
}
