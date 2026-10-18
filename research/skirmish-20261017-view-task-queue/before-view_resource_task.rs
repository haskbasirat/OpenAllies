//!5C3104's derived resource task (A4BBA4), its actual5B17F0(0)
//!constructor fields and5B3550 completion. Constructing/calling this object
//!alone proves no5AED10 queue ordering, resource barrier or native view Init.
use crate::{application_loading,view_init_prefix,world_revision::{ConstructedWorldRevision,Witness}};
use std::sync::{Arc,Mutex,Weak};
type Result<T>=std::result::Result<T,String>;

///One constructor-owned task. No Clone/Default or raw-status setter. Native
///refcount/list/manager links are left to the actual ResourceManager queue;
///they are not invented from this bounded status/thread field projection.
struct Fields{state_14:u32,thread_18:u32}
pub struct Task {world:Witness,fields:Arc<Mutex<Fields>>}
///Nonowning UI observation. Only the concrete resource queue retains Task;
///this cannot extend its lifetime or invoke a completion callback.
pub struct TaskWitness{fields:Weak<Mutex<Fields>>}
impl Task {
    ///5B17F0(0), followed by the derived vtable store5C3126. The supplied
    ///identity is the SAME current World named by UI5C3000's VWorld owner.
    pub(crate) fn construct_for_ui(world:&ConstructedWorldRevision)->Self {
        Self::construct_for_ui_witness(&world.witness()).expect("borrowed World revision is live")
    }
    pub(crate) fn construct_for_ui_witness(world:&Witness)->Result<Self>{
        world.require_live()?;
        Ok(Self{world:world.clone(),fields:Arc::new(Mutex::new(Fields{state_14:0,thread_18:0}))})
    }
    ///The actual5B3550 callback body. Caller must be the concrete queue's
    ///completion dispatch, not a GPU acknowledgment. World AC is cleared
    ///before the current ApplicationE4 Loading17 store, then Task14=2.
    pub fn complete_5b3550<G>(&mut self,world:&ConstructedWorldRevision,
        view:&mut view_init_prefix::State<G>,loading:&mut application_loading::State)->Result<()> {
        if !self.world.is_for(world){return Err("View resource callback names another or destroyed World".into());}
        view.resource_callback_5b355d(world)?;
        loading.resource_callback_5b356d();
        self.fields.lock().map_err(|_|"View resource task field lock poisoned")?.state_14=2; //5B18C0(2) does not touch18.
        Ok(())
    }
    pub fn witness(&self)->TaskWitness{TaskWitness{fields:Arc::downgrade(&self.fields)}}
    pub fn state_14(&self)->Result<u32>{Ok(self.fields.lock().map_err(|_|"View resource task field lock poisoned")?.state_14)}
    pub fn thread_18(&self)->Result<u32>{Ok(self.fields.lock().map_err(|_|"View resource task field lock poisoned")?.thread_18)}
}
impl TaskWitness{
    pub fn state_14(&self)->Result<u32>{let owner=self.fields.upgrade().ok_or("View resource task allocation is retired")?;let result=owner.lock().map_err(|_|"View resource task field lock poisoned")?.state_14;Ok(result)}
}
