use aa_mapworld::{
    MapArchive, WorldInventory, actor_payload::ActorTail, saved_components::SavedComponent,
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    sources::{SourceLimits, V14Sources},
};
use std::path::PathBuf;
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
    let pc = Precision::Double53;
    let flags = catalog.native_final_flags("start_loc", pc).unwrap();
    let numbered = catalog
        .native_field("start_loc", "numbered", pc)
        .unwrap()
        .unwrap()
        .boolean()
        .unwrap();
    let mask = catalog
        .native_component_mask("start_loc", None)
        .unwrap()
        .bits;
    let map = MapArchive::from_bytes(
        std::fs::read(
            game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial01.RMP"),
        )
        .unwrap(),
    )
    .unwrap();
    let world = WorldInventory::decode(&map, 128 << 20).unwrap();
    let mut records = vec![];
    for record in world
        .dynamic_things
        .placements
        .iter()
        .filter(|v| v.thing_id_utf16 == "start_loc".encode_utf16().collect::<Vec<_>>())
    {
        let mut slots = vec![];
        let (tail, n) = ActorTail::decode_prefix_with_components(
            &record.opaque_tail,
            record.actor.as_ref().unwrap().version,
            0,
            |slot, _, bytes| {
                let (v, n) = SavedComponent::decode_prefix(slot, bytes)?;
                slots.push((slot, format!("{v:?}")));
                Ok((n, None))
            },
        )
        .unwrap();
        assert_eq!(n, record.opaque_tail.len());
        records.push(serde_json::json!({"record":record,"tail":tail,"slots":slots,"runtime_flags":record.thing.runtime_flags(0x80000000,flags)}));
    }
    println!("{}",serde_json::to_string_pretty(&serde_json::json!({"definition_flags":flags,"numbered":numbered,"definition_mask":mask,"source":catalog.find("start_loc").unwrap(),"records":records})).unwrap());
}
