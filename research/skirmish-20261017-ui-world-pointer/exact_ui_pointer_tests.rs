mod native_camera { pub struct State; impl State { pub fn word(&self,_:u32)->Result<u32,String>{unreachable!("pure proof excludes concrete Camera owner")}}}
#[path="../../crates/aa-runtime/src/view_ui_world.rs"]mod view_ui_world;
#[path="../../crates/aa-runtime/src/view_pointer.rs"]mod view_pointer;
