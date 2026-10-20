//! Print the fixed-function lighting inputs of original models: material
//! colours, vertex-colour modes, texture apply modes and the lighting flag.
//! Usage: cargo run -p aa-formats --example nif_lighting -- <Data dir> <rel.nif>...
use aa_formats::nif::{self, Value};

fn colour(v: Option<&Value>) -> String {
    match v {
        Some(Value::F32s(w)) => format!("{:?}", w.iter().map(|b| (f32::from_bits(*b) * 1000.0).round() / 1000.0).collect::<Vec<_>>()),
        Some(other) => format!("{other:?}"),
        None => "-".into(),
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let data = std::path::PathBuf::from(args.next().expect("Data dir"));
    for rel in args {
        let bytes = match std::fs::read(data.join(&rel)) {
            Ok(b) => b,
            Err(e) => {
                println!("{rel}: {e}");
                continue;
            }
        };
        let file = match nif::load(&bytes) {
            Ok(f) => f,
            Err(e) => {
                println!("{rel}: {e:?}");
                continue;
            }
        };
        println!("== {rel}");
        let mut seen = std::collections::BTreeMap::new();
        for o in &file.objects {
            let line = match o.class {
                "NiMaterialProperty" => format!(
                    "material ambient {} diffuse {} emissive {} alpha {:?}",
                    colour(o.fields.get("ambient")),
                    colour(o.fields.get("diffuse")),
                    colour(o.fields.get("emissive")),
                    o.fields.f32("alpha").map(f32::from_bits)
                ),
                "NiVertexColorProperty" => format!("vertex_color vertex_mode {:?} lighting_mode {:?}", o.fields.u32("vertex_mode"), o.fields.u32("lighting_mode")),
                "NiTexturingProperty" => format!("texturing apply_mode {:?}", o.fields.u32("apply_mode")),
                "NiSpecularProperty" | "NiAlphaProperty" | "NiZBufferProperty" => continue,
                "NiTriShapeData" | "NiTriStripsData" => format!("geometry has_colors {:?}", o.fields.get("has_vertex_colors").or(o.fields.get("has_colors")).map(|v| format!("{v:?}"))),
                _ => continue,
            };
            *seen.entry(line).or_insert(0) += 1;
        }
        for (line, n) in seen {
            println!("  {n:>3} x {line}");
        }
    }
}
