#![allow(dead_code)]
mod view_grid_state{pub trait Globals{fn word(&mut self,address:u32)->Result<u32,String>;}}
#[path="../../../crates/aa-runtime/src/view_environment_projection.rs"]
mod view_environment_projection;
#[path="../../../crates/aa-runtime/src/view_texture_effect.rs"]
mod view_texture_effect;
