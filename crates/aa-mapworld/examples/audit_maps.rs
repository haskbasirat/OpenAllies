use aa_mapworld::{MapArchive, WorldInventory};
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
fn maps(path: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            maps(&path, out)?
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("rmp"))
        {
            out.push(path)
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let original = root.join("../reconstruction/assets/original/Data/Maps");
    let mut paths = Vec::new();
    maps(&original, &mut paths)?;
    paths.sort();
    let mut report = BTreeMap::new();
    for path in paths {
        let map = MapArchive::from_bytes(std::fs::read(&path)?)?;
        let world = WorldInventory::decode(&map, 128 * 1024 * 1024)?;
        let mut classes = BTreeMap::<u32, usize>::new();
        let mut owners = BTreeMap::<u32, usize>::new();
        let mut nested = 0;
        for p in &world.dynamic_things.placements {
            *classes.entry(p.class_code).or_default() += 1;
            if let Some(actor) = &p.actor {
                *owners.entry(actor.owner_handle).or_default() += 1;
            }
            if p.object
                .as_ref()
                .is_some_and(|o| o.nested_count.unwrap_or(0) > 0)
            {
                nested += 1;
            }
        }
        report.insert(path.strip_prefix(&original)?.to_string_lossy().replace('\\',"/"),json!({"map_source":map.source(),"directory":map.directory(),"world":world.world,"id_manager_cursor":world.id_manager.next_slot,"id_manager_highest":world.id_manager.highest_slot,"id_manager_source":world.id_manager.source,"placements":world.dynamic_things.placements.len(),"classes":classes,"owner_handles":owners,"nested_object_records":nested,"generation_mismatches":world.generation_mismatches,"unsupported":world.dynamic_things.unsupported.iter().map(|u|json!({"record":u.record_ordinal,"span":u.source_span,"reason":u.reason})).collect::<Vec<_>>() }));
    }
    let bytes = serde_json::to_vec_pretty(&report)?;
    let path = root.join("research/phase6-mapworld/corpus-audit.json");
    std::fs::write(&path, &bytes)?;
    println!(
        "{}",
        json!({"maps":report.len(),"placements":report.values().map(|r|r["placements"].as_u64().unwrap()).sum::<u64>(),"unsupported":report.values().map(|r|r["unsupported"].as_array().unwrap().len()).sum::<usize>(),"report_sha256":aa_mapworld::archive::sha256(&bytes),"report":path})
    );
    Ok(())
}
