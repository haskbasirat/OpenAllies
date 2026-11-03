//! The Rust exporter against the stock conversion the project's art comes from: every NIF
//! and a sample of the textures of the original data, value by value and pixel by pixel.
//! Reads `<game root>/reconstruction`, so ignored by default; set AA_GAME_ROOT. The
//! stock conversion was made by the first exporter, so this holds the port to it.
use aa_assets::convert::{ArtIndex, convert_model, decode_texture};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn first_difference(path: &str, ours: &Value, stock: &Value) -> Option<String> {
    match (ours, stock) {
        (Value::Number(a), Value::Number(b)) => (a.as_f64() != b.as_f64()).then(|| format!("{path}: ours {a} stock {b}")),
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Some(format!("{path}: ours {} items stock {}", a.len(), b.len()));
            }
            a.iter().zip(b).enumerate().find_map(|(i, (x, y))| first_difference(&format!("{path}[{i}]"), x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys()) {
                // Audited per mesh below; the stock files hold the first exporter's guess.
                if key == "teamColorMask" {
                    continue;
                }
                match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => {
                        if let Some(d) = first_difference(&format!("{path}.{key}"), x, y) {
                            return Some(d);
                        }
                    }
                    // The stock files predate these two records of the exporter.
                    (Some(_), None) if key == "baseTextureState" || key == "alphaPropertyState" => {}
                    // A later note on six stock files; nothing reads it.
                    (None, Some(_)) if key == "textureAlias" => {}
                    (x, y) => return Some(format!("{path}.{key}: ours {x:?} stock {y:?}")),
                }
            }
            None
        }
        (a, b) => (a != b).then(|| format!("{path}: ours {a} stock {b}")),
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, out);
        } else {
            out.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
        }
    }
}

#[test]
#[ignore = "reads the original data and its conversion; set AA_GAME_ROOT"]
fn every_stock_model_converts_as_the_stock_conversion_did() {
    let game = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
    let data = game.join("reconstruction/assets/original/Data");
    let converted = game.join("reconstruction/assets/converted/models");
    let manifest: Value = serde_json::from_slice(&std::fs::read(converted.join("manifest.json")).unwrap()).unwrap();
    let stock_failed: std::collections::BTreeSet<String> = manifest["failures"].as_array().unwrap().iter().map(|f| f["source"].as_str().unwrap().to_lowercase()).collect();
    let index = ArtIndex::new(&data, None);
    let mut files = Vec::new();
    walk(&data, &data, &mut files);
    files.sort();
    let audit: Value = serde_json::from_slice(&std::fs::read(game.join("rust-rebuild/assets-cache/model-materials.json")).unwrap()).unwrap();
    let (mut audited, mut audit_different) = (0, 0);
    let (mut same, mut different, mut failed_both, mut only_ours_failed, mut only_stock_failed, mut no_stock) = (0, 0, 0, 0, 0, 0);
    let mut shown = 0;
    let started = std::time::Instant::now();
    for rel in files.iter().filter(|f| f.to_lowercase().ends_with(".nif")) {
        let bytes = std::fs::read(data.join(rel)).unwrap();
        let ours = convert_model(rel, &bytes, &index);
        let stock_path = converted.join(rel).with_extension("json");
        match ours {
            Err(e) => {
                if stock_failed.contains(&rel.to_lowercase()) {
                    failed_both += 1;
                } else {
                    only_ours_failed += 1;
                    eprintln!("only ours failed: {rel}: {e}");
                }
            }
            Ok(model) => {
                if stock_failed.contains(&rel.to_lowercase()) {
                    only_stock_failed += 1;
                    eprintln!("only stock failed: {rel}");
                    continue;
                }
                let Ok(stock) = std::fs::read(&stock_path) else {
                    no_stock += 1;
                    eprintln!("no stock conversion: {rel}");
                    continue;
                };
                let stock: Value = serde_json::from_slice(&stock).unwrap();
                let rows = audit["models"][rel.as_str()]["corrections"].as_array().map_or(&[][..], Vec::as_slice);
                let corrections = model.corrections();
                if rows.len() != corrections.len() {
                    audit_different += 1;
                    eprintln!("{rel}: {} audited rows, ours {}", rows.len(), corrections.len());
                }
                for (row, c) in rows.iter().zip(&corrections) {
                    audited += 1;
                    let wanted = (row["skip"].as_bool().unwrap(), row["team_color_mask"].as_bool().unwrap(), row["render_state"]["alpha_flags"].as_u64().unwrap() as u16, row["render_state"]["alpha_threshold"].as_u64().unwrap() as u8, row["render_state"]["zbuffer_flags"].as_u64().unwrap() as u16, row["render_state"]["zbuffer_function"].as_u64().unwrap() as u32);
                    let ours = (c.skip, c.team_color_mask, c.render_state.alpha_flags, c.render_state.alpha_threshold, c.render_state.zbuffer_flags, c.render_state.zbuffer_function);
                    let centre: Vec<f64> = row["sort_center"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
                    if wanted != ours || centre != c.sort_center {
                        audit_different += 1;
                        if shown < 40 {
                            shown += 1;
                            eprintln!("{rel} {}: audit {wanted:?} {centre:?} ours {ours:?} {:?}", row["name"], c.sort_center);
                        }
                    }
                }
                let ours = serde_json::to_value(&model).unwrap();
                match first_difference("", &ours, &stock) {
                    None => same += 1,
                    Some(d) => {
                        different += 1;
                        if shown < 40 {
                            shown += 1;
                            eprintln!("{rel}: {d}");
                        }
                    }
                }
            }
        }
    }
    eprintln!("audited meshes {audited}, different {audit_different}");
    eprintln!("models: same {same}, different {different}, both failed {failed_both}, only ours failed {only_ours_failed}, only stock failed {only_stock_failed}, no stock conversion {no_stock}, {:.0} s", started.elapsed().as_secs_f32());
    let textures = game.join("reconstruction/assets/converted/textures");
    let every: usize = std::env::var("AA_CONVERT_TEXTURE_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let (mut equal, mut unequal, mut failed) = (0, 0, 0);
    let images = files.iter().filter(|f| {
        let l = f.to_lowercase();
        l.ends_with(".dds") || l.ends_with(".tga")
    });
    for rel in images.step_by(every.max(1)) {
        // Two textures of one stem (a DDS and a TGA) made one stock PNG: the last one's.
        let stem = |f: &str| f.rsplit_once('.').map_or(f, |(s, _)| s).to_lowercase();
        if files.iter().filter(|f| {
            let l = f.to_lowercase();
            (l.ends_with(".dds") || l.ends_with(".tga")) && stem(f) == stem(rel)
        }).count() > 1 {
            continue;
        }
        let bytes = std::fs::read(data.join(rel)).unwrap();
        let stock_path = textures.join(rel).with_extension("png");
        let Ok(stock) = image::open(&stock_path) else { continue };
        let stock = stock.into_rgba8();
        match decode_texture(rel, &bytes) {
            Err(e) => {
                failed += 1;
                eprintln!("texture failed: {rel}: {e}");
            }
            Ok((w, h, pixels)) => {
                // A 16 bit TGA widens its five bit channels by another rounding: one step off.
                let steps = if rel.to_lowercase().ends_with(".tga") && bytes[16] == 16 { 1 } else { 0 };
                if (w, h) == (stock.width(), stock.height()) && pixels.iter().zip(stock.as_raw()).all(|(a, b)| a.abs_diff(*b) <= steps) {
                    equal += 1;
                } else {
                    unequal += 1;
                    let diff = pixels.iter().zip(stock.as_raw()).filter(|(a, b)| a != b).count();
                    let first = pixels.chunks(4).zip(stock.as_raw().chunks(4)).find(|(a, b)| a != b);
                    eprintln!("texture differs: {rel}: {w}x{h} vs {}x{}, {diff} bytes, first {first:?}", stock.width(), stock.height());
                }
            }
        }
    }
    eprintln!("textures: equal {equal}, different {unequal}, failed {failed}");
    assert_eq!((different, audit_different, only_ours_failed, only_stock_failed, unequal, failed), (0, 0, 0, 0, 0, 0));
}

/// A mod's folder through the store's path, for its census: `AA_CONVERT_DATA`, `AA_CONVERT_OUT`,
/// `AA_CONVERT_EVERY` (every Nth file, default 1): what does not convert, and why.
#[test]
#[ignore = "reads AA_CONVERT_DATA, writes AA_CONVERT_OUT; set AA_GAME_ROOT"]
fn a_folder_through_the_store() {
    let game = PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("set AA_GAME_ROOT"));
    let data = PathBuf::from(std::env::var_os("AA_CONVERT_DATA").expect("AA_CONVERT_DATA"));
    let out = PathBuf::from(std::env::var_os("AA_CONVERT_OUT").expect("AA_CONVERT_OUT"));
    let every: usize = std::env::var("AA_CONVERT_EVERY").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let stock = game.join("reconstruction/assets/original/Data");
    let art = aa_assets::convert::FolderArt::new(&data, &out, Some(&stock));
    let mut files = Vec::new();
    walk(&data, &data, &mut files);
    files.sort();
    let started = std::time::Instant::now();
    let (mut own, mut as_stock, mut failed) = (0, 0, std::collections::BTreeMap::<String, u32>::new());
    for rel in files.iter().filter(|f| f.to_lowercase().ends_with(".nif")).step_by(every.max(1)) {
        match art.model(rel) {
            None => as_stock += 1,
            Some(Ok(path)) => {
                own += 1;
                // what the store reads
                serde_json::from_slice::<aa_assets::ModelAsset>(&std::fs::read(path).unwrap()).unwrap();
            }
            Some(Err(reason)) => *failed.entry(reason).or_default() += 1,
        }
    }
    eprintln!("models: {own} own, {as_stock} as stock, not converted {failed:#?}, {:.0} s", started.elapsed().as_secs_f32());
    let started = std::time::Instant::now();
    let (mut own, mut other) = (0, 0);
    for rel in files.iter().filter(|f| {
        let l = f.to_lowercase();
        l.ends_with(".dds") || l.ends_with(".tga")
    }).step_by(every.max(1)) {
        if art.texture(rel).is_some() { own += 1 } else { other += 1 }
    }
    eprintln!("textures: {own} own, {other} as stock or not converted, {:.0} s", started.elapsed().as_secs_f32());
}
