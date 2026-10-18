//! Native442C00 fields and43F0A0..43F1D4 startup prefix on ONE VWorld.
//! The bounded prefix yields no view-ready/AD capability. The concrete full
//! composition may perform43FF36 only after its actual remaining children.
use crate::world_revision::{ConstructedWorldRevision,Witness};
type Result<T>=std::result::Result<T,String>;
///Exact43FF36 store, separately exercised against both original binaries.
fn completed_ad_43ff36(initialized:&mut u8){*initialized=1;}

///These are actual VWorld fields, not a second simulation/renderer snapshot.
///G is the concrete constructor-owned430290 VGrid supplied by the backend.
struct Fields<G>{revision:u32,initializing:u8,initialized:u8,uninitializing:u8,
    words:[u32;5],grid:Option<G>}
impl<G> Fields<G>{
    ///442C79..442D08 direct stores. Unrelated base constructors are not
    ///represented as having run by this bounded field projection.
    fn constructed()->Self{Self{revision:0,initializing:0,initialized:0,uninitializing:0,words:[0;5],grid:None}}
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
fn prefix<G>(fields:&mut Fields<G>,revision:u32,services:&mut impl Services<Grid=G>)->Result<()>{
    fields.revision=revision; //43F0C1 precedes every child.
    fields.initializing=1;
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
        if self.phase!=Phase::Constructed{return Err("VWorld fresh prefix requires actual uninitialized fields; existing view requires43E400 teardown".into())}
        self.world=Some(world.witness());self.phase=Phase::Initializing;
        match prefix(&mut self.fields,world.value(),services){
            Ok(())=>{self.phase=Phase::PrefixComplete;Ok(())},
            Err(e)=>{self.phase=Phase::Failed;Err(e)},
        }
    }
    pub fn grid(&self)->Option<&G>{self.fields.grid.as_ref()}
    pub fn grid_mut(&mut self)->Option<&mut G>{self.fields.grid.as_mut()}
    ///Disjoint loans of the SAME constructor-owned grid and nativeAC byte.
    ///The aggregate validates World identity before exposing its typed loan.
    pub(crate) fn object_init_parts(&mut self)->Result<(&mut G,&u8)>{
        Ok((self.fields.grid.as_mut().ok_or("actual VWorld94 grid is absent at object Init")?,&self.fields.initializing))
    }
    pub fn revision(&self)->u32{self.fields.revision}
    pub fn initializing(&self)->bool{self.fields.initializing!=0}
    pub fn initialized(&self)->bool{self.fields.initialized!=0}
    ///A lazy Loading682EE0 read of this SAME VWorld's nativeAD byte. A
    ///numeric revision alone cannot authorize a different/replaced World.
    pub fn initialized_for(&self,world:&ConstructedWorldRevision)->Result<bool>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("VWorld AD read belongs to another or unbound World".into())
        }
        Ok(self.fields.initialized!=0)
    }
    ///40B7C0 reads this SAME current VWorld+8C after the Node timer. This
    ///does not certify that the VWorld initialization or resource queue ran.
    pub fn message_word_8c_for(&self,world:&ConstructedWorldRevision)->Result<u32>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("message VWorld8C read belongs to another or unbound World".into())
        }
        Ok(self.fields.words[2])
    }
    ///5B355D, reached by the actual View-resource task callback. This clears
    ///only AC; it never writes AD or grants initialization/queue readiness.
    pub fn resource_callback_5b355d(&mut self,world:&ConstructedWorldRevision)->Result<()>{
        if !self.world.as_ref().is_some_and(|w|w.is_for(world)){
            return Err("VWorld resource callback belongs to another or unbound World".into())
        }
        self.fields.initializing=0;
        Ok(())
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
        completed_ad_43ff36(&mut self.fields.initialized);
        Ok(())
    }
}
#[cfg(test)]
#[path="view_init_prefix_tests.rs"]
mod tests;
#[cfg(test)]#[path="view_init_completion_tests.rs"]mod completion_tests;
