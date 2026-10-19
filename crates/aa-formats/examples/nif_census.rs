//! Census of controller and key usage over every shipped NIF (Data.rwd).
//! Usage: cargo run -p aa-formats --example nif_census -- <Data.rwd>
use aa_formats::nif::{self, is_kind_of, Value};
use aa_formats::RwdArchive;
use std::collections::BTreeMap;

fn main() {
    let path = std::env::args().nth(1).expect("Data.rwd path");
    let mut archive = RwdArchive::open(&path).unwrap();
    let members: Vec<(usize, String)> = archive.entries().iter().enumerate()
        .filter(|(_, e)| e.path().to_ascii_lowercase().ends_with(".nif"))
        .map(|(i, e)| (i, e.path().to_string())).collect();
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    let mut add = |k: String| *census.entry(k).or_default() += 1;
    for (index, path) in members {
        let mut data = Vec::new();
        archive.copy_entry(index, &mut data).unwrap();
        let nif = nif::load(&data).unwrap();
        for (i, o) in nif.objects.iter().enumerate() {
            if is_kind_of(o.class, "NiTimeController") {
                let target = nif.linked(i, "target").map(|t| nif.objects[t].class).unwrap_or("NULL");
                let flags = o.fields.u16("flags").unwrap();
                add(format!("controller {} -> {} active={} cycle={} reverse={} apptime={}", o.class, target, flags >> 3 & 1, flags >> 1 & 3, flags >> 4 & 1, flags & 1));
            }
            if o.class == "NiKeyframeData" {
                for (group, count, kind) in [("rot", "rotation_count", "rotation_type"), ("pos", "translation_count", "translation_type"), ("scale", "scale_count", "scale_type")] {
                    let n = o.fields.u32(count).unwrap();
                    let t = o.fields.u32(kind);
                    add(format!("keyframe {group} type={:?} single={}", t, n == 1));
                    if t == Some(4) {
                        let keys = o.fields.list("rotations").unwrap();
                        for k in keys {
                            if let Value::Record(r) = k {
                                for f in r { if f.name == "axes" { if let Value::List(axes) = &f.value { for a in axes { if let Value::Record(ax) = a { let ty = ax.iter().find(|x| x.name == "type").map(|x| format!("{:?}", x.value)); add(format!("euler axis type {:?}", ty)); } } } } }
                            }
                        }
                    }
                }
            }
            for (cls, count, kind) in [("NiFloatData", "key_count", "key_type"), ("NiPosData", "key_count", "key_type"), ("NiColorData", "key_count", "key_type")] {
                if o.class == cls {
                    add(format!("{cls} type={:?} single={}", o.fields.u32(kind), o.fields.u32(count) == Some(1)));
                }
            }
        }
    }
    for (k, v) in census { println!("{v:7} {k}"); }
}
