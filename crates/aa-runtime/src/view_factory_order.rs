//!4F0750/50C220/43D160/43C420 ordered publication. Actual constructors,
//!derived Init, destruction and view-list ownership are required children.
//!Completing this traversal alone never certifies VWorld AD or a battle.
pub type Result<T> = std::result::Result<T,String>;

pub trait Fields {
    type Thing:Copy;
    type View:Copy;
    fn begin_diagnostic(&mut self)->Result<()>;
    fn attach_view(&mut self)->Result<()>;
    fn kingdom_count(&mut self)->Result<i32>;
    fn kingdom(&mut self,index:i32)->Result<Option<Self::Thing>>;
    fn world_phase(&mut self)->Result<u32>;
    fn attached_view_present(&mut self)->Result<bool>;
    fn source_view_type(&mut self,thing:Self::Thing)->Result<u32>;
    ///43AC20 performs its OWN second source getter before allocation/ctor.
    fn construct_view(&mut self,thing:Self::Thing)->Result<Option<Self::View>>;
    fn initialize_view(&mut self,view:Self::View)->Result<bool>;
    fn destroy_view(&mut self,view:Self::View,free:u32)->Result<()>;
    fn view_flags(&mut self,view:Self::View)->Result<u32>;
    fn prepend_update_view(&mut self,view:Self::View)->Result<()>;
    fn publish_view(&mut self,thing:Self::Thing,view:Option<Self::View>)->Result<()>;
    fn next_thing(&mut self,cursor:&mut u32)->Result<Option<Self::Thing>>;
    fn thing_has_view(&mut self,thing:Self::Thing)->Result<bool>;
    fn end_diagnostic(&mut self,existing:i32,attempted_missing:i32)->Result<()>;
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage {NotAttempted,BeginDiagnostic,Attach,KingdomCount,KingdomSlot,
    WorldPhase,AttachedView,SourceType,Construct,Init,Destroy,ViewFlags,
    UpdateList,Publish,NextThing,ExistingView,EndDiagnostic,Complete}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Report {pub existing:u32,pub attempted_missing:u32}
///Retained error-prefix metadata for one real OnVWorldCreated call. No retry
///resets prior attachments/publications or manufactures a readiness receipt.
pub struct State<T> {
    attempted:bool,complete:bool,stage:Stage,current:Option<T>,
    kingdom_index:i32,cursor:u32,counts:Report,
}
impl<T:Copy> State<T> {
    pub fn not_attempted()->Self {Self {attempted:false,complete:false,stage:Stage::NotAttempted,
        current:None,kingdom_index:0,cursor:0,counts:Report{existing:0,attempted_missing:0}}}
    pub fn stage(&self)->Stage{self.stage}
    pub fn current(&self)->Option<T>{self.current}
    pub fn cursor(&self)->u32{self.cursor}
    pub fn kingdom_index(&self)->i32{self.kingdom_index}
    pub fn counts(&self)->Report{self.counts}
    pub fn traversal_completed(&self)->bool{self.complete}
    pub fn run<F:Fields<Thing=T>>(&mut self,fields:&mut F)->Result<Report>{
        if self.attempted{return Err("4F0750 retained view publication was already attempted".into())}
        self.attempted=true;
        self.stage=Stage::BeginDiagnostic;fields.begin_diagnostic()?;
        self.stage=Stage::Attach;fields.attach_view()?;
        self.stage=Stage::KingdomCount;let mut count=fields.kingdom_count()?;
        while self.kingdom_index<count {
            self.stage=Stage::KingdomSlot;let thing=fields.kingdom(self.kingdom_index)?;
            self.current=thing;self.add_thing(fields,thing)?;
            //4F07D5 reads14C BEFORE incrementing the current slot index.
            self.stage=Stage::KingdomCount;count=fields.kingdom_count()?;
            self.kingdom_index=self.kingdom_index.wrapping_add(1);
        }
        self.cursor=0;
        loop {
            self.stage=Stage::NextThing;let thing=fields.next_thing(&mut self.cursor)?;
            self.current=thing;let Some(thing)=thing else{break};
            self.stage=Stage::ExistingView;
            if fields.thing_has_view(thing)? {
                self.counts.existing=self.counts.existing.wrapping_add(1);
            }else{
                self.add_thing(fields,Some(thing))?;
                //Native counts attempted NULL-link dispatch, even when phase,
                //view or source-type gates perform no actual allocation.
                self.counts.attempted_missing=self.counts.attempted_missing.wrapping_add(1);
            }
        }
        self.stage=Stage::EndDiagnostic;
        fields.end_diagnostic(self.counts.existing as i32,self.counts.attempted_missing as i32)?;
        self.stage=Stage::Complete;self.complete=true;Ok(self.counts)
    }
    fn add_thing<F:Fields<Thing=T>>(&mut self,fields:&mut F,thing:Option<T>)->Result<()> {
        self.stage=Stage::WorldPhase;if fields.world_phase()?==6{return Ok(())}
        let Some(thing)=thing else{return Ok(())};
        self.stage=Stage::AttachedView;if !fields.attached_view_present()?{return Ok(())}
        self.stage=Stage::SourceType;if fields.source_view_type(thing)?==0{return Ok(())}
        self.stage=Stage::Construct;
        let view=fields.construct_view(thing)?;
        self.stage=Stage::Init;
        let view=view.ok_or("43C426 requires nonnull factory result before virtualC8 Init")?;
        let result=if fields.initialize_view(view)? {
            self.stage=Stage::ViewFlags;
            if fields.view_flags(view)?&(1<<20)!=0 {
                self.stage=Stage::UpdateList;fields.prepend_update_view(view)?;
            }
            Some(view)
        }else{
            self.stage=Stage::Destroy;fields.destroy_view(view,1)?;None
        };
        self.stage=Stage::Publish;fields.publish_view(thing,result)
    }
}
///Same native50C220 leaf for later actual Thing creation. This does not attach
///a view, run4F0750 or create a traversal/readiness capability.
pub fn on_thing_added<F:Fields>(fields:&mut F,thing:Option<F::Thing>)->Result<()> {
    State::not_attempted().add_thing(fields,thing)
}
#[cfg(test)]
#[path="view_factory_order_tests.rs"]
mod tests;
