use aa_assets::{AssetStore, HeightMode};
use rayon::prelude::*;
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = AssetStore::open(
        std::env::args()
            .nth(1)
            .ok_or("usage: audit <reconstruction-root>")?,
    )?;
    let mut errors = Vec::new();
    let mut maps = Vec::new();
    for info in store.maps() {
        match store.load_map(&info.id).and_then(|map|map.terrain.prepare(HeightMode::NativeConditional).map(|terrain|(map,terrain))) {
            Ok((map,terrain))=>maps.push(json!({"id":info.id,"placements":map.placements.actors.len(),"decode_errors":map.placements.errors.len(),"vertices":terrain.vertices.len(),"warnings":terrain.warnings})),
            Err(e)=>errors.push(e.to_string())
        }
    }
    let models: Vec<_> = store
        .model_infos()
        .par_iter()
        .map(|info| {
            store
                .load_model(&info.source)
                .and_then(|model| model.prepare())
                .map(|m| json!({"source":info.source,"meshes":m.meshes.len()}))
                .map_err(|e| e.to_string())
        })
        .collect();
    let mut valid_models = 0;
    for m in models {
        match m {
            Ok(_) => valid_models += 1,
            Err(e) => errors.push(e),
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"passed":errors.is_empty(),"maps_loaded":maps.len(),"models_prepared":valid_models,"errors":errors,"maps":maps})
        )?
    );
    if errors.is_empty() {
        Ok(())
    } else {
        Err("asset audit failed; see errors".into())
    }
}
