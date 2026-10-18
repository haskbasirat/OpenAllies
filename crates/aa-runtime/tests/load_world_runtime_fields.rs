// Compile only this owned subject's tests; do not pull every in-flight runtime
// unit-test module into an unrelated owner proof.
pub use aa_runtime::live_clocked;
#[path="../src/load_world_runtime_fields.rs"]
mod subject;
