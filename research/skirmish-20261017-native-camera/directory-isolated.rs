#![allow(dead_code,unused_imports)]
#[path="directory-isolated/native_camera.rs"]mod native_camera;
#[path="directory-isolated/view_scene_init.rs"]mod view_scene_init;
#[path="directory-isolated/world_revision.rs"]mod world_revision;
#[path="../../crates/aa-runtime/src/native_render_properties.rs"]mod native_render_properties;
#[path="../../crates/aa-runtime/src/native_render_defaults.rs"]mod native_render_defaults;
mod view_grid_state{pub trait Globals{fn word(&mut self,address:u32)->Result<u32,String>;}}
