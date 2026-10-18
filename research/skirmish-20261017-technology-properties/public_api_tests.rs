//! Standalone test crate importing only existing public production APIs.
//!This does not include or recompile runtime implementation source, and
//!private source guards remain inside the original compiled aa_runtime rlib.
pub use aa_runtime::{catalog::Catalog,definition_technology_properties::bind,combat_source_completion,combat_source_receipt,settings};
#[path="../../crates/aa-runtime/src/definition_technology_properties_tests.rs"]
mod cases;
