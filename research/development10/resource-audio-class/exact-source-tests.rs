//!Direct inclusion of the current Audio class/source implementation. Existing
//!compiled runtime dependencies supply source-CV/Catalog support only; this is
//!not a rebuild/acceptance of the current whole ResourceManager or application.
pub use aa_runtime::{catalog,definitions,load_view_variables,combat_source_completion,combat_source_receipt,settings};
#[path="../../../crates/aa-runtime/src/resource_audio_class.rs"]
mod audio_class;
