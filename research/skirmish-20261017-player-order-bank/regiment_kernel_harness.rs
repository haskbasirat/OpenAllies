//Isolated native-derived kernel replay. Uses actual production source and
//the exact same public test module; does NOT run the private live Bound.
extern crate self as aa_runtime;
#[path="../../crates/aa-runtime/src/regiment_command.rs"]
pub mod regiment_command;
#[path="../../crates/aa-runtime/tests/regiment_commands.rs"]
mod tests;
