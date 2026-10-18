use aa_mapworld::{MapArchive, kingdoms::Kingdoms};
use std::path::{Path, PathBuf};
fn paths(p: &Path, v: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(p).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            paths(&p, v)
        } else if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("rmp")) {
            v.push(p)
        }
    }
}
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../reconstruction/assets/original/Data/Maps");
    let mut maps = Vec::new();
    paths(&root, &mut maps);
    maps.sort();
    let mut rows = Vec::new();
    for p in maps {
        let m = MapArchive::from_bytes(std::fs::read(&p).unwrap()).unwrap();
        let c = m.decode_chunk("KINGDOMS", 128 << 20).unwrap();
        let w = m.decode_chunk("GWORLD", 128 << 20).unwrap();
        let version = u32::from_le_bytes(w.bytes[..4].try_into().unwrap());
        let result = match Kingdoms::decode(&c, version) {
            Ok(v) => serde_json::json!({"map":p,"decoded":v}),
            Err(e) => serde_json::json!({"map":p,"error":e.to_string()}),
        };
        rows.push(result);
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
