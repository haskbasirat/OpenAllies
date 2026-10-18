//!Ordered5C3000 UI -> camera -> VWorld -> world-view-publication -> resource
//!task submission. This is not a renderer-ready stand-in. Each required child
//!must use its actual retained owner; failures keep the same UI/task prefix.
use crate::{view_resource_task,world_revision::{ConstructedWorldRevision,Witness}};
type Result<T>=std::result::Result<T,String>;

///The actual World lock brackets the complete sequence, including the queue
///submission. An adapter must retain/release its real lock on ordinary/error
///unwind; it may not replace it with a successful notification or no-op.
pub trait Services:Sized {
    type Kingdom;
    fn init_diagnostic(&mut self)->Result<()>;
    fn with_world_lock<R>(&mut self,call:impl FnOnce(&mut Self)->Result<R>)->Result<R>;
    fn selected_kingdom_76c2b0(&mut self)->Result<Option<Self::Kingdom>>;
    fn reset_camera_700e30(&mut self)->Result<()>;
    fn initialize_vworld_43f0a0(&mut self,init_word_b8:u32)->Result<()>;
    fn camera_zero_700500(&mut self,zero_pair:[u32;2])->Result<()>;
    fn world_on_vworld_created_4f0750(&mut self)->Result<()>;
    ///Move this SAME newly constructed task into the retained queue by taking
    ///the slot at the actual publication point. The queue owns its identity;
    ///an error before transfer leaves Some here, after transfer the queue
    ///retains it. This call must not complete the task
    ///merely because all current GPU uploads acknowledged.
    fn enqueue_view_task_5aed10(&mut self,task:&mut Option<view_resource_task::Task>)->Result<()>;
}
#[cfg(test)]
#[path="ui_world_init_tests.rs"]
mod tests;
///Only the InitVWorld-written fields. The original complete UI constructor is
///not claimed: before their first native store they remain explicitly unbound.
pub struct State<K>{has_kingdom_1e4:Option<u8>,kingdom_1e0:Option<Option<K>>,
    pending_task:Option<view_resource_task::Task>,task:Option<view_resource_task::TaskWitness>,attempted:bool}
impl<K> State<K>{
    pub fn unbound_ui_fields()->Self{Self{has_kingdom_1e4:None,kingdom_1e0:None,pending_task:None,task:None,attempted:false}}
    ///5C3000. If the first76C2B0 returns NULL,1E0 is deliberately untouched;
    ///otherwise it is reread after1E4=1 and can itself return NULL.
    pub fn initialize_5c3000(&mut self,world:&ConstructedWorldRevision,
        services:&mut impl Services<Kingdom=K>)->Result<()> {
        self.initialize_5c3000_for_witness(&world.witness(),services)
    }
    ///Same algorithm with nonowning immutable identity metadata, so concrete
    ///services can borrow the actual World mutably for4F0750 publication.
    ///No raw revision or task-ready constructor is exposed by this entry.
    pub fn initialize_5c3000_for_witness(&mut self,world:&Witness,
        services:&mut impl Services<Kingdom=K>)->Result<()> {
        //Historical laboratory adapter: fixtures explicitly used the same
        //numeric word as their World revision. Product callers must transport
        //the actual notification+38 argument through the entry below.
        self.initialize_5c3000_for_notification(world,world.value(),services)
    }
    pub fn initialize_5c3000_for_notification(&mut self,world:&Witness,init_word_b8:u32,
        services:&mut impl Services<Kingdom=K>)->Result<()> {
        if self.attempted{return Err("fresh UI VWorld initialization already attempted; actual teardown/retry owner required".into());}
        world.require_live()?;
        self.attempted=true;
        services.init_diagnostic()?;
        services.with_world_lock(|services|{
            if services.selected_kingdom_76c2b0()?.is_some(){
                self.has_kingdom_1e4=Some(1);
                self.kingdom_1e0=Some(services.selected_kingdom_76c2b0()?);
            }else{self.has_kingdom_1e4=Some(0);}
            services.reset_camera_700e30()?;
            services.initialize_vworld_43f0a0(init_word_b8)?;
            services.camera_zero_700500([0,0])?;
            services.world_on_vworld_created_4f0750()?;
            self.pending_task=Some(view_resource_task::Task::construct_for_ui_witness(world)?);
            self.task=Some(self.pending_task.as_ref().expect("constructed task").witness());
            services.enqueue_view_task_5aed10(&mut self.pending_task)?;
            if self.pending_task.is_some(){return Err("5AED10 resource queue returned without adopting the actual task".into());}
            Ok(())
        })
    }
    pub fn has_kingdom_1e4(&self)->Result<u8>{self.has_kingdom_1e4.ok_or_else(||"UI1E4 has no reached native writer".into())}
    pub fn kingdom_1e0(&self)->Result<Option<&K>>{self.kingdom_1e0.as_ref().map(Option::as_ref).ok_or_else(||"UI1E0 has no reached native writer".into())}
    pub fn task(&self)->Option<&view_resource_task::TaskWitness>{self.task.as_ref()}
}
