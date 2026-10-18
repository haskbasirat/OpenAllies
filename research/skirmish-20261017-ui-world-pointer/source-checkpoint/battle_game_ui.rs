//!The actual source-bound Game interface's retained+10C UIWorldState.
//!A weak capability can name it from VWorld+13C; fields stay on the original
//!GUI thread and are operated through UiBridge, never copied into simulation.
use std::sync::{Arc,Mutex,Weak};
use std::thread::{self,ThreadId};
use aa_runtime::view_ui_world;
use crate::{live_scene::view_directory::ViewHandle,windows_reset_os::PerformanceTimer};
type Result<T>=std::result::Result<T,String>;
type Native=view_ui_world::State<ViewHandle,PerformanceTimer>;
pub(super) struct Owner {allocation:Arc<Mutex<Native>>,thread:ThreadId}
#[derive(Clone)]
pub struct Reference {allocation:Weak<Mutex<Native>>,thread:ThreadId}
impl Owner {
    pub(super) fn allocate()->Self{Self{allocation:Arc::new(Mutex::new(Native::before_children_438e10())),thread:thread::current().id()}}
    pub(super) fn construct(&mut self,ui:&aa_ui::ui::Ui)->Result<()>{
        struct Source<'a>(&'a aa_ui::ui::Ui);
        impl view_ui_world::Constructor for Source<'_>{
            type Timer=PerformanceTimer;
            fn timer_59aa70_59ab60(&mut self)->Result<PerformanceTimer>{
                PerformanceTimer::initialize_frequency()?;
                PerformanceTimer::from_native_constructor().map_err(|e|e.to_string())
            }
            fn wildcard_display_4b9f40(&mut self,key:&str)->Result<Vec<u16>>{
                let text=self.0.strings.get(key).ok_or("actual UIWorldState wildcard display source is absent")?;
                Ok(text.encode_utf16().collect())
            }
        }
        self.allocation.lock().map_err(|_|"actual Game UIWorldState lock is poisoned")?.construct_640640(&mut Source(ui))
    }
    pub(super) fn reference(&self)->Result<Reference>{
        self.allocation.lock().map_err(|_|"actual Game UIWorldState lock is poisoned")?.require_constructed()?;
        Ok(Reference{allocation:Arc::downgrade(&self.allocation),thread:self.thread})
    }
}
impl Reference {
    pub fn same_allocation(&self,other:&Self)->bool{self.allocation.ptr_eq(&other.allocation)}
    pub fn require_live(&self)->Result<()>{
        let allocation=self.allocation.upgrade().ok_or("actual Game UIWorldState allocation was retired")?;
        allocation.lock().map_err(|_|"actual Game UIWorldState lock is poisoned")?.require_constructed()?;Ok(())
    }
    ///The capability is nonowning. Only the original GUI thread may operate
    ///the same fields; simulation callers must use their bound UiBridge.
    pub fn with<R>(&self,operation:impl FnOnce(&mut Native)->Result<R>)->Result<R>{
        if thread::current().id()!=self.thread{return Err("UIWorldState operation must run on its original GUI thread".into())}
        let allocation=self.allocation.upgrade().ok_or("actual Game UIWorldState allocation was retired")?;
        let mut fields=allocation.lock().map_err(|_|"actual Game UIWorldState lock is poisoned")?;
        fields.require_constructed()?;operation(&mut fields)
    }
}
