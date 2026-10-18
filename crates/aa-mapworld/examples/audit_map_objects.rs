//! Corpus proof of Object prefix and bounded Area/Camera payload coverage.
use aa_mapworld::{
    MapArchive, WorldInventory,
    map_objects::{Area, Camera},
    restore::{RestoreDefinitions, RuntimeDefinition},
    restore_components::UpdatableTarget,
};
use aa_numeric::Precision;
use std::path::{Path, PathBuf};
struct Bindings;
impl RestoreDefinitions for Bindings {
    fn resolve(&self, _: &[u16], _: Precision) -> Result<RuntimeDefinition, String> {
        Err("unused".into())
    }
    fn localize(&self, _: &[u16]) -> Result<Vec<u16>, String> {
        Err("corpus audit does not bind localization".into())
    }
    fn register_updatable(&self, _: i32, _: &UpdatableTarget) -> Result<(), String> {
        Ok(())
    }
}
fn walk(path: &Path, files: &mut Vec<PathBuf>) {
    for item in std::fs::read_dir(path).unwrap() {
        let p = item.unwrap().path();
        if p.is_dir() {
            walk(&p, files)
        } else if p.extension().is_some_and(|v| v.eq_ignore_ascii_case("rmp")) {
            files.push(p);
        }
    }
}
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../reconstruction/assets/original/Data/Maps");
    let mut files = vec![];
    walk(&root, &mut files);
    files.sort();
    let mut rows = vec![];
    let mut errors = vec![];
    for path in files {
        let map = MapArchive::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let world = WorldInventory::decode(&map, 128 << 20).unwrap();
        let mut counts = [0u32; 2];
        let mut ids = 0;
        for record in world
            .dynamic_things
            .placements
            .iter()
            .filter(|v| matches!(v.class_code, 6 | 7))
        {
            assert!(record.object.is_some());
            let mut trace = vec![];
            let result = if record.class_code == 6 {
                Area::default().load(
                    &record.opaque_tail,
                    world.world.version,
                    record.serialized_id,
                    &Bindings,
                    &mut trace,
                )
            } else {
                Camera::default().load(&record.opaque_tail, &Bindings)
            };
            match result{Ok(n)if n==record.opaque_tail.len()=>{counts[(record.class_code-6)as usize]+=1;ids+=trace.len();},v=>errors.push(serde_json::json!({"map":path,"ordinal":record.record_ordinal,"class":record.class_code,"result":format!("{v:?}")}))};
        }
        rows.push(serde_json::json!({"map":path.strip_prefix(&root).unwrap().to_string_lossy(),"areas":counts[0],"cameras":counts[1],"observed_updatable_ids":ids}));
    }
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"scope":"Bounded saved Object prefix plus Area/Camera payload coverage; Updatable IDs observed only, localization intentionally unavailable","maps":rows,"errors":errors})).unwrap());
}
