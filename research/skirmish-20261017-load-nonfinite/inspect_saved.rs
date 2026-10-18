// Read-only serialized-input diagnostic; this does not reconstruct runtime state.
fn main() {
    let path = std::env::args_os().nth(1).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(std::fs::read(path).unwrap()).unwrap();
    let inventory = aa_mapworld::WorldInventory::decode(&map, u64::MAX).unwrap();
    println!("map_source={:?}", map.source());
    for row in &inventory.dynamic_things.placements {
        if row.serialized_id == 16669 {
            println!("definition={}", String::from_utf16_lossy(&row.thing_id_utf16));
            println!("serialized_input={row:#?}");
        }
    }
    let game = std::path::PathBuf::from("E:/Games/Axis and Alies");
    let mut files = aa_rules::sources::V14Sources::open(game.join("Data.rwd"),
        game.join("reconstruction/native-game/runtime-expanded-v14"),
        aa_rules::sources::SourceLimits { max_member_bytes: 64*1024*1024 }).unwrap();
    let actors = aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
    let pc = aa_numeric::Precision::Double53;
    println!("category={:?}", actors.native_category("waves").unwrap());
    println!("final_flags={:#010x}", actors.native_final_flags("waves", pc).unwrap());
    println!("object_definition={:#?}", actors.native_object_definition("waves", pc).unwrap());
}
