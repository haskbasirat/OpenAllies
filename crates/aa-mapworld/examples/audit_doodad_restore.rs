//! Source corpus validation of the derived Doodad constructor/wire stage only.
//! Updatable callbacks record IDs here; this is not a live registry/world test.
use aa_mapworld::{
    MapArchive, WorldInventory,
    dynamic_doodad::{Definition, DynamicDoodad},
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
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
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut source = V14Sources::open(
        game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {
            max_member_bytes: 64 << 20,
        },
    )
    .unwrap();
    let catalog = ActorCatalog::load_v14(&mut source).unwrap();
    let mut definitions = BTreeMap::new();
    let mut files = vec![];
    walk(
        &game.join("reconstruction/assets/original/Data/Maps"),
        &mut files,
    );
    files.sort();
    let mut rows = vec![];
    let mut total = 0;
    let mut errors = vec![];
    for path in files {
        let map = MapArchive::from_bytes(std::fs::read(&path).unwrap()).unwrap();
        let world = WorldInventory::decode(&map, 128 << 20).unwrap();
        let mut count = 0;
        let mut observed = 0;
        for record in world
            .dynamic_things
            .placements
            .iter()
            .filter(|v| v.class_code == 9)
        {
            let id = String::from_utf16(&record.thing_id_utf16).unwrap();
            if !definitions.contains_key(&id) {
                match catalog.native_dynamic_doodad(&id, Precision::Double53) {
                    Ok(native) => {
                        definitions.insert(
                            id.clone(),
                            Definition {
                                owner_id: native.id,
                                component_mask: native.component_mask,
                            },
                        );
                    }
                    Err(error) => {
                        errors.push(serde_json::json!({"map":path,"ordinal":record.record_ordinal,"definition":id,"definition_error":error.to_string()}));
                        continue;
                    }
                }
            }
            let mut state = DynamicDoodad::default();
            let mut events = vec![];
            match state.load(&record.opaque_tail,&definitions[&id],record.serialized_id,world.world.word_48,|_,_|Ok(()),&mut events){
    Ok(n)if n==record.opaque_tail.len()=>{count+=1;observed+=events.len();}
    result=>errors.push(serde_json::json!({"map":path,"ordinal":record.record_ordinal,"definition":id,"result":format!("{result:?}"),"bytes":record.opaque_tail.len()})),
   }
        }
        total += count;
        rows.push(serde_json::json!({"map":path.file_name().unwrap().to_string_lossy(),"records":count,"observed_updatable_ids":observed,"world_word48":world.world.word_48}));
    }
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"scope":"Source-bound Doodad derived constructor and wire/application only; callbacks observe ID order without publishing a runtime registry","maps":rows,"definitions":definitions.len(),"records":total,"errors":errors})).unwrap());
}
