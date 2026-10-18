//!Two borrowed interfaces to ONE retained request driver. Each temporary
//!RefCell loan ends at method return; reentrant use fails instead of aliasing
//!the actual execution/guard stack. No driver or OS identity is cloned.
use std::cell::RefCell;
use aa_runtime::{resource_objects::{Handle,ModelPolicy},resource_depot::{Backend,Resources},
    resource_task::{Services,ThreadHandle,Task,TaskQueue,BlockDiagnostic}};
type Result<T>=std::result::Result<T,String>;
pub(super) struct Policy<'a,T>(pub &'a RefCell<T>);
pub(super) struct Driver<'a,T>(pub &'a RefCell<T>);
impl<T:ModelPolicy>ModelPolicy for Policy<'_,T>{
    fn use_hardware_model_73a560(&mut self)->Result<bool>{self.0.try_borrow_mut().map_err(|_|"reentrant current Model policy/Resource driver borrow")?.use_hardware_model_73a560()}
    fn hardware_model_suffix_40f7b0(&mut self)->Result<Vec<u16>>{self.0.try_borrow_mut().map_err(|_|"reentrant current Model suffix/Resource driver borrow")?.hardware_model_suffix_40f7b0()}
}
macro_rules! forward{
    ($(fn $name:ident($($arg:ident:$ty:ty),*)->$ret:ty;)*)=>{$(
        fn $name(&mut self,$($arg:$ty),*)->Result<$ret>{
            self.0.try_borrow_mut().map_err(|_|"reentrant current Resource driver borrow")?.$name($($arg),*)
        }
    )*}
}
impl<T:Services> Services for Driver<'_,T>{
    forward!{
        fn current_thread()->ThreadHandle;
        fn enter_load(resource:&Handle)->();
        fn load_resource_4c(resource:&Handle)->();
        fn after_resource_3c(resource:&Handle)->();
        fn resource_flags8(resource:&Handle)->u32;
        fn store_resource_flags8(resource:&Handle,value:u32)->();
        fn loaded_diagnostic(resource:&Handle)->();
        fn leave_load(resource:&Handle)->();
        fn enter_request(resource:&Handle)->();
        fn cached_resource_59bfa0(resource:&Handle)->();
        fn cached_resource_virtual2c(resource:&Handle)->();
        fn leave_request(resource:&Handle)->();
        fn request_diagnostic(resource:&Handle,immediate:bool)->();
        fn block_timer_reset_59aa70()->();
        fn block_timer_start_59ab60()->();
        fn removing_from_queue_diagnostic(resource:&Handle)->();
        fn timing_enabled_40eaa0()->bool;
        fn block_diagnostic(resource:&Handle,reason:BlockDiagnostic)->();
        fn block_till_ready_5b1fd0(task:&Task,queue:&TaskQueue)->();
    }
}
impl<T:Backend>Backend for Driver<'_,T>{
    forward!{
        fn load_resource_4c_in(resources:&mut Resources<'_>,resource:&Handle)->();
        fn model_after_base_3c_in(resources:&mut Resources<'_>,resource:&Handle)->();
        fn double_request_warning(resource:&Handle,count:i32)->();
    }
}
