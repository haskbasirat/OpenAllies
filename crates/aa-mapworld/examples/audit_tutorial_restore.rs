use aa_mapworld::{
    MapArchive, WorldInventory, actor_payload::ActorTail, saved_components::SavedComponent,
};
use serde_json::json;
use std::{collections::BTreeMap, path::PathBuf};
fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../reconstruction/assets/original/Data/Maps/Campaign");
    let mut rows = vec![];
    for name in ["tutorial01.RMP", "tutorial02.RMP"] {
        let map = MapArchive::from_bytes(std::fs::read(root.join(name)).unwrap()).unwrap();
        let world = WorldInventory::decode(&map, 128 << 20).unwrap();
        let mut classes = BTreeMap::new();
        let mut actors = vec![];
        for r in &world.dynamic_things.placements {
            *classes.entry(r.class_code).or_insert(0usize) += 1;
            let Some(actor) = &r.actor else { continue };
            let mut slots = vec![];
            let result = ActorTail::decode_prefix_with_components(
                &r.opaque_tail,
                actor.version,
                0,
                |slot, _, bytes| {
                    slots.push(slot);
                    let (value, n) = SavedComponent::decode_prefix(slot, bytes)?;
                    Ok((
                        n,
                        if let SavedComponent::Properties(v) = value {
                            Some(v)
                        } else {
                            None
                        },
                    ))
                },
            );
            actors.push(json!({"ordinal":r.record_ordinal,"id":String::from_utf16_lossy(&r.thing_id_utf16),"saved_id":r.serialized_id,"owner":actor.owner_handle,"slots_until_stop":slots,"error":result.err().map(|e|e.to_string())}));
        }
        rows.push(json!({"map":name,"directory":map.directory(),"classes":classes,"actors":actors,"kingdoms":world.kingdoms,"teams":world.teams}));
    }
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
}
