//! Native442C00 fields and43F0A0..43F1D4 startup prefix on ONE VWorld.
//! The bounded prefix yields no view-ready/AD capability. The concrete full
//! composition may perform43FF36 only after its actual remaining children.
use crate::world_revision::{ConstructedWorldRevision,Witness};
use std::sync::{Arc,Weak,atomic::{AtomicU8,Ordering}};
type Result<T>=std::result::Result<T,String>;
///Exact43FF36 store, separately exercised against both original binaries.
fn completed_ad_43ff36(initialized:&AtomicU8){initialized.store(1,Ordering::Release);}

///These are actual VWorld fields, not a second simulation/renderer snapshot.
///G is the concrete constructor-owned430290 VGrid supplied by the backend.
struct Fields<G>{init_word_b8:u32,initializing:Arc<AtomicU8>,initialized:Arc<AtomicU8>,uninitializing:u8,
    words:[u32;5],grid:Option<G>}
impl<G> Fields<G>{
    ///442C79..442D08 direct stores. Unrelated base constructors are not
    ///represented as having run by this bounded field projection.
    fn constructed()->Self{Self{init_word_b8:0,initializing:Arc::new(AtomicU8::new(0)),initialized:Arc::new(AtomicU8::new(0)),uninitializing:0,words:[0;5],grid:None}}
}
///Actual ordered child services. Each implementation retains its same owner
///and all partial mutations on failure. A successful prefix is never sufficient
///to advertise74FF50, unfreeze a World or enable gameplay orders.
pub trait Services{
    type Grid;
    fn begin_screen_7040e0(&mut self)->Result<()>;
    fn reset_user_746e30(&mut self)->Result<()>;
    fn resource_context_5ae0a0(&mut self)->Result<()>;
    fn construct_grid_430290(&mut self)->Result<Option<Self::Grid>>;
    fn resetting_editor_variables_diagnostic(&mut self)->Result<()>;
    ///The actual213-entry UVar owner executes43F156..43F1B2 here. Partial
    ///cell resets remain committed if its real Session getter fails.
    fn reset_view_variables(&mut self)->Result<()>;
}
fn prefix<G>(fields:&mut Fields<G>,init_word_b8:u32,services:&mut impl Services<Grid=G>)->Result<()>{
    fields.init_word_b8=init_word_b8; //43F0C1 precedes every child.
    fields.initializing.store(1,Ordering::Release);
    services.begin_screen_7040e0()?;
    services.reset_user_746e30()?;
    services.resource_context_5ae0a0()?;
    let grid=services.construct_grid_430290()?;
    fields.grid=grid; //43F148, including native allocation-null.
    services.resetting_editor_variables_diagnostic()?;
    services.reset_view_variables()?;
    //Native store order is90,8C,84,88,EC, not sorted by address.
    for index in[3,2,0,1,4]{fields.words[index]=0;}
    Ok(())
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Phase{Constructed,Initializing,PrefixComplete,Failed}
///Retained native field owner. It has no Clone/Default/raw-ready setter.
pub struct State<G>{fields:Fields<G>,world:Option<Witness>,phase:Phase}
impl<G> State<G>{
    pub fn from_constructor_fields()->Self{Self{fields:Fields::constructed(),world:None,phase:Phase::Constructed}}
    pub fn initialize_prefix(&mut self,world:&ConstructedWorldRevision,services:&mut impl Services<Grid=G>)->Result<()>{
        //Historical fixture adapter, not a source for native Notification38.
        self.initialize_prefix_with_argument(world,world.value(),services)
    }
    pub fn initialize_prefix_with_argument(&mut self,world:&ConstructedWorldRevision,init_word_b8:u32,services:&mut impl Services<Grid=G>)->Result<()>{
        if self.phase!=Phase::Constructed{return Err("VWorld fresh prefix requires actual uninitialized fields; existing view requires43E400 teardown".into())}
        self.world=Some(world.witness());self.phase=Phase::Initializing;
        match prefix(&mut self.fields,init_word_b8,services){
            Ok(())=>{self.phase=Phase::PrefixComplete;Ok(())},
            Err(e)=>{self.phase=Phase::Failed;Err(e)},
        }
    }
    pub fn grid(&self)->Option<&G>{self.fields.grid.as_ref()}
    pub fn grid_mut(&mut self)->Option<&mut G>{self.fields.grid.as_mut()}
    ///Disjoint loans of the SAME constructor-owned grid and nativeAC byte.
    ///The aggregate validates World identity before exposing its typed loan.
    pub(crate) fn object_init_parts(&mut self)->Result<(&mut G,&AtomicU8)>{
        Ok((self.fields.grid.as_mut().ok_or("actual VWorld94 grid is absent at object Init")?,&self.fields.initializing))
    }
    ///Native VWorld+B8, copied from notification+38. It is independent of
    ///the World constructor revision, which is guarded by the private Witness.
    pub fn init_word_b8(&self)->u32{self.fields.init_word_b8}
    ///Legacy diagnostic name; this reads B8, not the World identity.
    pub fn revision(&self)->u32{self.init_word_b8()}
    pub fn initializing(&self)->bool{self.fields.initializing.load(Ordering::Acquire)!=0}
    pub fn initialized(&self)->bool{self.fields.initialized.load(Ordering::Acquire)!=0}
    ///Nonowning read access to this allocation's sole nativeAD byte. This
    ///exists before a World is bound, as required by414620 EndSession2.
    ///It grants no writer or completed-view capability.
    pub fn initialized_reader(&self)->InitializedRead{InitializedRead{field:Arc::downgrade(&self.fields.initialized)}}
    ///A lazy Loading682EE0 read of this SAME VWorld's nativeAD byte. A
    ///numeric revision alone cannot authorize a different/replaced World.
    pub fn initialized_for(&self,world:&ConstructedWorldRevision)->Result<bool>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("VWorld AD read belongs to another or unbound World".into())
        }
        Ok(self.initialized())
    }
    ///40B7C0 reads this SAME current VWorld+8C after the Node timer. This
    ///does not certify that the VWorld initialization or resource queue ran.
    pub fn message_word_8c_for(&self,world:&ConstructedWorldRevision)->Result<u32>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("message VWorld8C read belongs to another or unbound World".into())
        }
        Ok(self.fields.words[2])
    }
    ///43AB30's words +84, +88, +8C, +90 of this SAME VWorld (5C92F0 calls it
    ///per UI frame while AD is set). The prefix zeroed them; 43AB30 is their
    ///per-frame writer (vworld_update::view_time_43ab30).
    pub(crate) fn view_time_words_mut_for(&mut self,world:&ConstructedWorldRevision)->Result<&mut [u32;4]>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("43AB30 VWorld words belong to another or unbound World".into())
        }
        let (time,_)=self.fields.words.split_at_mut(4);
        <&mut [u32;4]>::try_from(time).map_err(|_|"VWorld time word layout".into())
    }
    ///+84, +88, +8C, +90 as 43AB30 last stored them.
    pub fn view_time_words_for(&self,world:&ConstructedWorldRevision)->Result<[u32;4]>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("VWorld time word read belongs to another or unbound World".into())
        }
        Ok([self.fields.words[0],self.fields.words[1],self.fields.words[2],self.fields.words[3]])
    }
    ///5B355D, reached by the actual View-resource task callback. This clears
    ///only AC; it never writes AD or grants initialization/queue readiness.
    #[cfg(test)]
    pub fn resource_callback_5b355d(&mut self,world:&ConstructedWorldRevision)->Result<()>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("VWorld resource callback belongs to another or unbound World".into())
        }
        self.fields.initializing.store(0,Ordering::Release);
        Ok(())
    }
    pub(crate) fn resource_callback_writer(&self,world:&ConstructedWorldRevision)->Result<ResourceCallback>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){return Err("VWorld callback writer belongs to another or unbound World".into())}
        Ok(ResourceCallback{field:Arc::downgrade(&self.fields.initializing),world:world.witness()})
    }
    pub fn uninitializing(&self)->bool{self.fields.uninitializing!=0}
    pub fn require_prefix_complete(&self)->Result<()>{
        if self.phase==Phase::PrefixComplete{Ok(())}else{Err("VWorld native prefix has not completed".into())}
    }
    ///Only the concrete view_init_world full composition calls this after
    ///validating actual assets, scene/terrain and trigger-request completion.
    ///The native store changes AD only; AC remains1 until5B355D runs later.
    pub(crate) fn complete_full_init_43ff36(&mut self,world:&ConstructedWorldRevision)->Result<()>{
        self.require_prefix_complete()?;
        self.initialized_for(world)?;
        completed_ad_43ff36(&self.fields.initialized);
        Ok(())
    }
}
#[cfg(test)]
pub(crate) fn laboratory_queue_fields(world:&ConstructedWorldRevision,ac:u8,ad:u8)->State<()>{
    //Explicit poison/sentinel input from the original-instruction queue fixture;
    //this constructor is absent from every production build.
    let mut state=State::from_constructor_fields();state.world=Some(world.witness());
    state.fields.initializing.store(ac,Ordering::Release);state.fields.initialized.store(ad,Ordering::Release);state
}
#[cfg(test)]
#[path="view_init_prefix_tests.rs"]
mod tests;
#[cfg(test)]#[path="view_init_completion_tests.rs"]mod completion_tests;

///Only a real queued View task retains this nonowning writer capability.
pub(crate) struct ResourceCallback{field:Weak<AtomicU8>,world:Witness}
impl ResourceCallback{pub(crate) fn clear_5b355d(&self)->Result<()>{self.world.require_live()?;self.field.upgrade().ok_or("VWorld AC allocation is retired")?.store(0,Ordering::Release);Ok(())}}

///Read-only414620/682EE0 access to the actual retained nativeAD cell. Moving
///its VWorld owner preserves the cell; dropping that owner retires this read.
pub struct InitializedRead{field:Weak<AtomicU8>}
impl InitializedRead{
    pub fn require_live(&self)->Result<()>{self.field.upgrade().map(|_|()).ok_or_else(||"VWorld AD allocation is retired".into())}
    pub fn read(&self)->Result<bool>{Ok(self.field.upgrade().ok_or("VWorld AD allocation is retired")?.load(Ordering::Acquire)!=0)}
    ///Allocation identity check, independent of scalar byte/revision equality.
    pub fn is_for<G>(&self,state:&State<G>)->bool{self.field.ptr_eq(&Arc::downgrade(&state.fields.initialized))&&self.require_live().is_ok()}
}
