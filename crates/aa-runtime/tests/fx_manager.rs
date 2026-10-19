// Disabled: the kernel's tests now replay fixtures through other runtime
// modules (vworld_update, world_broadcasts), which this one-file harness
// cannot compile. The same tests run in the library:
//     cargo test -p aa-runtime --lib fx_manager
#![cfg(any())]
// Compile the production kernel's fixture tests in a small separate binary.
// This does not turn a presentation-queue proof into map/battle acceptance.
#[path="../src/fx_manager.rs"]
mod kernel;
