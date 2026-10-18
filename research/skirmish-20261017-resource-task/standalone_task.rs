//!Exact production task/queue/resource owner sources; only the unrelated
//!resource-map-cache pure name normalizer is mechanically extracted, and
//!ResourceObjects' unrelated integration-test module declaration is excluded.
#[path="../../crates/aa-runtime/src/execution_context.rs"]pub mod execution_context;
#[path="resource_objects-production.rs"]pub mod resource_objects;
#[path="../../crates/aa-runtime/src/resource_task.rs"]pub mod resource_task;
pub mod resource_map_cache {type Result<T>=std::result::Result<T,String>;
pub(crate) fn normalized_key(ids: &[u16]) -> Result<String> {
    let end = ids.iter().position(|&u| u == 0).unwrap_or(ids.len());
    if ids[..end].iter().any(|&u| u > 127) {
        return Err("resource name outside proved ASCII native lookup domain".into());
    }
    let text = String::from_utf16(&ids[..end]).map_err(|e| e.to_string())?;
    Ok(text.replace('\\', "/").to_ascii_lowercase())
}
}
