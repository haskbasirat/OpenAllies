// Read-only serialized placement inventory, no runtime/load readiness claim.
fn main() {
    let path=std::env::args_os().nth(1).unwrap();
    let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(path).unwrap()).unwrap();
    let inventory=aa_mapworld::WorldInventory::decode(&map,u64::MAX).unwrap();
    println!("placements={} unsupported={} source={:?}",inventory.dynamic_things.placements.len(),inventory.dynamic_things.unsupported.len(),map.source());
    let mut n=0;
    for row in &inventory.dynamic_things.placements {
        if row.thing.facing_bits.iter().any(|v|v&0x7f800000==0x7f800000) {
            n+=1;
            println!("ordinal={} saved_id={} class={} definition={} raw_facing={:08x?}",row.record_ordinal,row.serialized_id,row.class_code,String::from_utf16_lossy(&row.thing_id_utf16),row.thing.facing_bits);
        }
    }
    println!("nonfinite_facing_records={n}");
}
