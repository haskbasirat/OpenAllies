//! Independent focused harness: exact frozen production module and unchanged author external tests.
extern crate self as aa_runtime;
#[path = "<game root>//rust-rebuild//crates//aa-runtime//src//execution_context.rs"]
pub mod execution_context;
#[path = "<game root>//rust-rebuild//crates//aa-runtime//tests//execution_context.rs"]
mod captured_identity_tests;
#[path = "<game root>//rust-rebuild//crates//aa-runtime//tests//resource_worker.rs"]
mod windows_worker_tests;
