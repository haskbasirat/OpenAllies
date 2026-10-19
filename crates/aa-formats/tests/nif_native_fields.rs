//! The native object graph after NiStream::Load, field by field, for every
//! shipped NIF (`research/phase8-nif/native_nif_fields.py` reads the native
//! objects' memory under Unicorn when LoadStream finishes linking).
//!
//! The Rust side builds the same canonical record per object from the ported
//! reader: names, controller links, NiAVObject flags/transform/property list
//! (native head-first order), NiNode child slots and effect list, geometry
//! data links, counts, flags, consistency words, bounds and array contents,
//! time-controller flags/times/target/next, property values, source-texture
//! and texturing-map fields. Files the native old-particle post-process
//! rewrites are compared by class only.
use aa_formats::nif::{self, is_kind_of, NifFile, Value};
use aa_formats::RwdArchive;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[path = "support/sha256.rs"]
mod sha256;

enum J {
    Null,
    Int(u64),
    Str(String),
    List(Vec<J>),
}

fn write(j: &J, out: &mut String) {
    match j {
        J::Null => out.push_str("null"),
        J::Int(v) => out.push_str(&v.to_string()),
        J::Str(s) => {
            // Python json.dumps with ensure_ascii.
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    '\u{8}' => out.push_str("\\b"),
                    '\u{c}' => out.push_str("\\f"),
                    c if (c as u32) < 0x20 || (c as u32) > 0x7e => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        J::List(v) => {
            out.push('[');
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(x, out);
            }
            out.push(']');
        }
    }
}

fn int(v: impl Into<u64>) -> J {
    J::Int(v.into())
}

fn ints(v: &[u32]) -> J {
    J::List(v.iter().map(|&w| int(w)).collect())
}

fn opt(v: Option<usize>) -> J {
    v.map(|i| int(i as u64)).unwrap_or(J::Null)
}

fn latin1(bytes: Option<&[u8]>) -> J {
    match bytes {
        None => J::Null,
        Some(b) => J::Str(b.split(|&c| c == 0).next().unwrap_or(&[]).iter().map(|&c| c as char).collect()),
    }
}

fn digest_words(words: &[u32]) -> String {
    let mut bytes = Vec::with_capacity(words.len() * 4);
    for w in words {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    sha256::hex(&bytes)
}

fn record(nif: &NifFile, i: usize) -> J {
    let o = &nif.objects[i];
    let f = &o.fields;
    let c = o.class;
    let mut r: Vec<J> = vec![J::List(vec![J::Str("class".into()), J::Str(c.into())])];
    let mut put = |k: &str, v: J| r.push(J::List(vec![J::Str(k.into()), v]));
    if is_kind_of(c, "NiObjectNET") {
        put("name", latin1(f.string("name").unwrap()));
        put("controller", opt(nif.linked(i, "controller")));
    }
    if is_kind_of(c, "NiAVObject") {
        put("flags", int(f.u16("flags").unwrap()));
        let mut t = f.f32s("rotation").unwrap().to_vec();
        t.extend_from_slice(f.f32s("translation").unwrap());
        t.push(f.f32("scale").unwrap());
        put("transform", ints(&t));
        put("properties", J::List(nif.av_properties(i).unwrap().into_iter().map(opt).collect()));
    }
    if is_kind_of(c, "NiNode") {
        put("children", J::List(nif.node_children(i).unwrap().into_iter().map(opt).collect()));
        put("effects", J::List(nif.node_effects(i).unwrap().into_iter().map(|e| opt(Some(e))).collect()));
    }
    if is_kind_of(c, "NiGeometry") {
        put("data", opt(nif.linked(i, "data")));
    }
    if is_kind_of(c, "NiGeometryData") {
        let n = f.u16("vertex_count").unwrap();
        let flags = f.u16("data_flags").unwrap_or(0);
        let consistency =
            f.u16("consistency_flags").or_else(|| f.u16("consistency_flags_native")).unwrap_or(0);
        put("count", int(n));
        put("data_flags", int(flags));
        put("consistency", int(consistency));
        let mut bound = f.f32s("center").unwrap().to_vec();
        bound.push(f.f32("radius").unwrap());
        put("bound", ints(&bound));
        let array = |present: &str, words: &str| -> J {
            if f.u32(present).unwrap() != 0 {
                J::Str(digest_words(f.f32s(words).unwrap_or(&[])))
            } else {
                J::Null
            }
        };
        let uv = if flags & 0x3f != 0 { J::Str(digest_words(f.f32s("uv_sets").unwrap())) } else { J::Null };
        put("arrays", J::List(vec![array("has_vertices", "vertices"), array("has_normals", "normals"), array("has_colors", "colors"), uv]));
    }
    if is_kind_of(c, "NiTriBasedGeomData") {
        put("triangles", int(f.u16("triangle_count").unwrap()));
    }
    if is_kind_of(c, "NiTimeController") {
        put("flags", int(f.u16("flags").unwrap()));
        put("times", ints(&[f.f32("frequency").unwrap(), f.f32("phase").unwrap(), f.f32("start").unwrap(), f.f32("stop").unwrap()]));
        put("target", opt(nif.linked(i, "target")));
        put("next", opt(nif.linked(i, "next")));
    }
    match c {
        "NiAlphaProperty" => {
            put("flags", int(f.u16("flags").unwrap()));
            put("threshold", int(f.u8("threshold").unwrap()));
        }
        "NiVertexColorProperty" => {
            put("flags", int(f.u16("flags").unwrap()));
            put("modes", ints(&[f.u32("vertex_mode").unwrap(), f.u32("lighting_mode").unwrap()]));
        }
        "NiZBufferProperty" => {
            put("flags", int(f.u16("flags").unwrap()));
            put("function", int(f.u32("function").unwrap()));
        }
        "NiMaterialProperty" => {
            let mut w = Vec::new();
            for k in ["ambient", "diffuse", "specular", "emissive"] {
                w.extend_from_slice(f.f32s(k).unwrap());
            }
            w.push(f.f32("glossiness").unwrap());
            w.push(f.f32("alpha").unwrap());
            put("material", ints(&w));
        }
        "NiSourceTexture" => {
            put("file_name", latin1(f.string("file_name").unwrap_or(None)));
            put("pixel_data", opt(nif.linked(i, "pixel_data")));
            put("format", ints(&[f.u32("pixel_layout").unwrap(), f.u32("use_mipmaps").unwrap(), f.u32("alpha_format").unwrap()]));
            put("static", int(f.u8("is_static").unwrap()));
        }
        "NiTextKeyExtraData" => {
            let keys = f.list("keys").unwrap().iter().map(|k| match k {
                Value::Record(fields) => {
                    let get = |name: &str| fields.iter().find(|x| x.name == name).map(|x| &x.value).unwrap();
                    let time = match get("time") { Value::F32(v) => int(*v), _ => panic!("key time") };
                    let text = match get("text") { Value::Str(s) => latin1(s.as_deref()), _ => panic!("key text") };
                    J::List(vec![time, text])
                }
                _ => panic!("text key"),
            });
            put("keys", J::List(keys.collect()));
        }
        "NiControllerSequence" => {
            put("name", latin1(f.string("name").unwrap()));
            put("text_keys_name", latin1(f.string("text_keys_name").unwrap()));
            put("text_keys", opt(nif.linked(i, "text_keys")));
            let blocks = f.list("blocks").unwrap();
            let field = |b: &Value, name: &str| -> Value {
                match b { Value::Record(fields) => fields.iter().find(|x| x.name == name).unwrap().value.clone(), _ => panic!("block") }
            };
            put("targets", J::List(blocks.iter().map(|b| match field(b, "target_name") { Value::Str(s) => latin1(s.as_deref()), _ => panic!("target") }).collect()));
            put("sequence_controllers", J::List(nif.sequence_controllers(i).unwrap().into_iter().map(opt).collect()));
        }
        "NiTexturingProperty" => {
            put("apply", int(f.u32("apply_mode").unwrap()));
            let maps = f.list("maps").unwrap().iter().map(|m| match m {
                Value::Record(fields) => {
                    let get = |name: &str| fields.iter().find(|x| x.name == name).map(|x| &x.value).unwrap();
                    let source = match get("source") { Value::Link(l) => nif.target(*l), _ => panic!("map source") };
                    let word = |name: &str| match get(name) { Value::U32(v) => int(*v), _ => panic!("map word") };
                    J::List(vec![opt(source), word("clamp"), word("filter"), word("uv_set")])
                }
                _ => J::Null,
            });
            put("maps", J::List(maps.collect()));
        }
        _ => {}
    }
    J::List(r)
}

#[test]
fn every_shipped_nif_builds_the_native_object_graph() {
    check(".nif", "native-nif-fields.tsv");
}

/// Every shipped KF: the native object graph (sequence names, text keys,
/// target names, controllers) against NIF_CORPUS_EXT=.kf native_nif_fields.py.
#[test]
fn every_shipped_kf_builds_the_native_object_graph() {
    check(".kf", "native-kf-fields.tsv");
}

fn check(ext: &str, name: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let root = std::env::var_os("AA_GAME_ROOT").map(PathBuf::from).unwrap_or(root);
    let archive_path = root.join("Data.rwd");
    if !archive_path.is_file() {
        eprintln!("skipping: {} not present", archive_path.display());
        return;
    }
    let tsv = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/phase8-nif").join(name))
        .expect("native field fixture");
    let native: BTreeMap<&str, &str> = tsv.lines().skip(1).map(|l| l.split_once('\t').unwrap()).collect();
    let mut archive = RwdArchive::open(&archive_path).unwrap();
    let members: Vec<(usize, String)> = archive
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| e.path().to_ascii_lowercase().ends_with(ext))
        .map(|(i, e)| (i, e.path().to_string()))
        .collect();
    assert_eq!(members.len(), native.len());
    let dump = std::env::var("NIF_FIELDS_DUMP").ok();
    let mut failures = Vec::new();
    for (index, path) in members {
        let mut data = Vec::new();
        archive.copy_entry(index, &mut data).unwrap();
        let nif = nif::load(&data).unwrap();
        let records: Vec<J> = if nif.requires_native_post_process.is_empty() {
            (0..nif.objects.len()).map(|i| record(&nif, i)).collect()
        } else {
            nif.objects.iter().map(|o| J::List(vec![J::List(vec![J::Str("class".into()), J::Str(o.class.into())])])).collect()
        };
        let mut text = String::new();
        write(&J::List(records), &mut text);
        if dump.as_deref() == Some(path.as_str()) {
            std::fs::write(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/nif-fields-dump.json"), &text).unwrap();
        }
        if sha256::hex(text.as_bytes()) != native[path.as_str()] {
            failures.push(path);
        }
    }
    if !failures.is_empty() {
        let n = failures.len();
        failures.truncate(30);
        panic!("{n} files differ from the native object graph:\n{}", failures.join("\n"));
    }
}

/// ThingStartLocation.nif, the NIF 10.0.1.0 model the Firefight start stops
/// on, against the values the native loader stores (native-nif-fields-samples.json).
#[test]
fn thing_start_location_10_0_1_0_native_values() {
    let root = std::env::var_os("AA_GAME_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.."));
    let path = root.join("reconstruction/assets/original/Data/Special/Things/ThingStartLocation/ThingStartLocation.nif");
    let Ok(data) = std::fs::read(&path) else {
        eprintln!("skipping: {} not present", path.display());
        return;
    };
    let nif = nif::load(&data).unwrap();
    assert_eq!((nif.version, nif.user_version), (nif::VERSION_10_0_1_0, 0));
    assert_eq!(nif.header, b"NetImmerse File Format, Version 10.0.1.0");
    assert_eq!(nif.objects.len(), 31);
    assert_eq!(nif.top_level, [Some(0)]);
    assert!(nif.requires_native_post_process.is_empty());
    // NiAVObject::LinkObject 86F3A0 inserts properties at the list head.
    assert_eq!(nif.objects[0].fields.links("properties").unwrap(), [5, 4]);
    assert_eq!(nif.av_properties(0).unwrap(), [Some(4), Some(5)]);
    assert_eq!(nif.av_properties(18).unwrap(), [Some(21), Some(22), Some(11)]);
    assert_eq!(nif.node_children(0).unwrap(), [Some(6), Some(15), Some(24)]);
    // Below 10.0.1.2 the NiAVObject flags keep their low six bits (86F360).
    assert_eq!(nif.objects[15].fields.u16("flags"), Some(22));
    // NiTimeController 89F070 below 10.0.1.1/2/3: (12 -> 12) & 0x1F | 0x20.
    let flags = nif.objects[16].fields.field("flags").unwrap();
    assert_eq!((flags.value.clone(), flags.read.clone()), (Value::U16(44), Some(Value::U16(12))));
    assert_eq!(nif.controllers(15).unwrap(), [16]);
    assert_eq!(nif.linked(16, "target"), Some(15));
    // Property flags come from B43DC8 masked per class below 10.0.1.2.
    assert_eq!(nif.objects[4].fields.u16("flags"), Some(3));
    assert_eq!(nif.objects[22].fields.u16("flags"), Some(4097));
    // NiSourceTexture below 10.0.1.4: external name, no pixel-data link.
    let texture = &nif.objects[13].fields;
    assert_eq!(texture.string("file_name"), Some(Some(&b"startlocationbase.dds"[..])));
    assert_eq!(texture.link("pixel_data"), None);
}
