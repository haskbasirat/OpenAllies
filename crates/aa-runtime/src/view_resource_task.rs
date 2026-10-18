//!Derived UI resource task A4BBA4. Its sole5B17F0 header is shared by the
//!actual heterogeneous ResourceManager queue; completion is never a GPU receipt.
use crate::{application_loading,view_init_prefix,resource_task::{Header,ThreadHandle},world_revision::{ConstructedWorldRevision,Witness}};
use std::sync::{Arc,Mutex,Weak};
type Result<T>=std::result::Result<T,String>;
struct Callback{ac:view_init_prefix::ResourceCallback,loading:application_loading::ResourceCallback}
pub(crate) struct Allocation{world:Witness,header:Arc<Header>,callback:Mutex<Option<Callback>>}
pub struct Task{allocation:Arc<Allocation>}
///Nonowning observation; it cannot retain or execute a queued task.
pub struct TaskWitness{allocation:Weak<Allocation>,header:Weak<Header>}
impl Task{
    pub(crate) fn construct_for_ui(world:&ConstructedWorldRevision)->Self{Self::construct_for_ui_witness(&world.witness()).expect("borrowed World is live")}
    pub(crate) fn construct_for_ui_witness(world:&Witness)->Result<Self>{world.require_live()?;Ok(Self{allocation:Arc::new(Allocation{world:world.clone(),header:Header::constructed(0,None),callback:Mutex::new(None)})})}
    pub(crate) fn header(&self)->&Arc<Header>{&self.allocation.header}
    pub(crate) fn allocation(&self)->Arc<Allocation>{Arc::clone(&self.allocation)}
    pub(crate) fn bind_callback<G>(&self,world:&ConstructedWorldRevision,view:&view_init_prefix::State<G>,loading:&application_loading::State)->Result<()>{
        if !self.allocation.world.is_for(world){return Err("View resource callback names another or destroyed World".into())}
        let mut callback=self.allocation.callback.lock().map_err(|_|"View task callback lock poisoned")?;
        if callback.is_some(){return Err("View task callback already bound to its actual owners".into())}
        *callback=Some(Callback{ac:view.resource_callback_writer(world)?,loading:loading.resource_callback_writer()});Ok(())
    }
    ///Compatibility entry for the bounded native5B3550 proof. Production uses
    ///the private same-owner callback bound only by actual queue publication.
    #[cfg(test)]
    pub fn complete_5b3550<G>(&mut self,world:&ConstructedWorldRevision,view:&mut view_init_prefix::State<G>,loading:&mut application_loading::State)->Result<()>{
        if !self.allocation.world.is_for(world){return Err("View resource callback names another or destroyed World".into())}
        view.resource_callback_5b355d(world)?;loading.resource_callback_5b356d();self.header().set_state(2,||unreachable!("state2 preserves thread18"))
    }
    pub fn witness(&self)->TaskWitness{TaskWitness{allocation:Arc::downgrade(&self.allocation),header:Arc::downgrade(self.header())}}
    pub fn state_14(&self)->Result<u32>{self.header().state()}
    ///The historical scalar projection supports constructor-null18 only.
    ///Positive references use thread_18_owner, never an OS-ID/pointer cast.
    pub fn thread_18(&self)->Result<u32>{if self.header().thread()?.is_none(){Ok(0)}else{Err("positive Task18 requires its typed retained thread owner".into())}}
    pub fn thread_18_owner(&self)->Result<Option<ThreadHandle>>{self.header().thread()}
}
impl Allocation{
    pub(crate) fn complete_callback(&self)->Result<()>{
        self.world.require_live()?;
        let callback=self.callback.lock().map_err(|_|"View task callback lock poisoned")?;
        let callback=callback.as_ref().ok_or("View task actual callback owners are unbound")?;
        callback.ac.clear_5b355d()?;
        callback.loading.set_5b356d()?;
        self.header.set_state(2,||unreachable!("state2 preserves thread18"))
    }
}
impl TaskWitness{
    pub(crate) fn has_header(&self,header:&Weak<Header>)->bool{Weak::ptr_eq(&self.header,header)}
    pub fn state_14(&self)->Result<u32>{self.allocation.upgrade().ok_or("View resource task allocation is retired")?.header.state()}
}
