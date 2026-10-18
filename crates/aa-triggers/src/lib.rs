//! Native trigger records and ordered world-manager execution.
#![forbid(unsafe_code)]
pub mod actions;
pub mod crt;
pub mod decode;
pub mod elements;
pub mod extended;
pub mod manager;
pub mod mapstrings;
pub mod objectives;
pub mod predicates;
pub mod program;
mod registrations;
pub mod selectors;
pub mod world_program;

/// Which ported module executes a native element opcode, if any. `None` means
/// the opcode stops the manager with an explicit unsupported error.
pub fn native_module(opcode: u32) -> Option<&'static str> {
    if program::CORE.contains(&opcode) {
        Some("core")
    } else if elements::SUPPORTED.contains(&opcode) {
        Some("elements")
    } else if actions::SUPPORTED.contains(&opcode) {
        Some("actions")
    } else if world_program::SUPPORTED.contains(&opcode) {
        Some("world_program")
    } else if extended::SUPPORTED.contains(&opcode) {
        Some("extended")
    } else {
        None
    }
}

/// Original registered element schema: (kind 0 event / 1 condition / 2 action,
/// argument classes).
pub fn element_schema(opcode: u32) -> Option<(u32, &'static [u32])> {
    registrations::schema(opcode)
}

/// Original argument registry index (B3D498 registry) of an `ma_*` name, as
/// stored by 54A640 at parameter+28.
pub fn argument_index(name: &str) -> Option<u32> {
    registrations::argument(name)
}
pub mod thing_references;
