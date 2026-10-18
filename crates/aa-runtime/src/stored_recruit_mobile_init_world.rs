//!Fresh Recruit581F30 and MobileDeployment5757C0 on the SAME ActorStore
//!allocations, canonical source Bank, clock and deferred queue. The existing
//!component_virtuals kernels own their native branch/store ordering.
use crate::{actor_world::{ActorStore,ObjectKey,RuntimeComponent},catalog::Catalog,definitions::Bank,
    restoration::Staged,scheduler_owners::Target};
use aa_actors::{ActorId,actor_creation::{Error,Result},components::ComponentSlot as Slot,component_virtuals as native};
use aa_numeric::Precision;
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
fn checked(catalog:&Catalog<'_>,actors:&ActorStore,bank:&Bank,key:ObjectKey,slot:Slot)->Result<(ActorId,Precision)>{
    let (owner,pc)=crate::stored_component_init_source::checked(catalog,actors,bank,key,slot)?;
    match (slot,actors.component(owner,slot).map_err(error)?){
        (Slot::Recruit,Some(RuntimeComponent::Recruit(v)))if v.header.owner==owner=>{},
        (Slot::MobileDeployment,Some(RuntimeComponent::MobileDeployment(v)))if v.header.owner==owner=>{},
        _=>return Err(error("stored Recruit/Mobile actual typed allocation/owner differs")),
    }
    let leaf=native::leaves(aa_actors::init_create::FactoryKind::Standard(slot));
    let expected=if slot==Slot::Recruit{0x581f30}else{0x5757c0};
    if leaf.init_existing!=expected||leaf.vf14!=0x4b2830{return Err(error("stored Recruit/Mobile actual vtable differs"))}
    Ok((owner,pc))
}
struct Stored<'a,'source>{staged:&'a mut Staged,catalog:&'a Catalog<'source>,key:ObjectKey,owner:ActorId,source_pc:Precision,live_pc:Precision}
impl Stored<'_,'_>{
    fn owner(&self)->Result<ActorId>{
        if self.staged.actors.id(self.key)!=Some(self.owner)||self.staged.actors.key(self.owner)!=Some(self.key){return Err(error("stored Recruit/Mobile Actor generation changed"))}Ok(self.owner)
    }
    fn source_id(&self)->Result<&str>{self.owner()?;Ok(&self.staged.actors.object(self.key).map_err(error)?.definition.proof.id)}
}
impl native::RecruitInitWorld for Stored<'_,'_>{
    type Actor=ObjectKey;type Target=String;
    fn recruit_init_targets(&mut self,key:ObjectKey)->Result<Vec<String>>{
        if key!=self.key{return Err(error("581F30 current Recruit allocation differs"))}
        //Completed460310+4EC source projection preserves authored order and
        //duplicates. Each184 classification remains lazy before its store.
        aa_rules::native::recruit_data::actor_recruit_targets(self.catalog.actors,self.source_id()?).map(|v|v.ids).map_err(error)
    }
    fn recruit_init_location(&mut self,target:&String)->Result<bool>{
        aa_rules::native::recruit_data::actor_transport_class(self.catalog.actors,target).map_err(error)
    }
    fn recruit_init_store(&mut self,key:ObjectKey,offset:u32,value:u8)->Result<()>{
        if key!=self.key{return Err(error("581F30 current Actor differs"))}let owner=self.owner()?;
        let Some(RuntimeComponent::Recruit(v))=self.staged.actors.component_mut(owner,Slot::Recruit).map_err(error)? else{return Err(error("581F30 actual Recruit payload absent"))};
        if v.header.owner!=owner{return Err(error("581F30 actual Recruit owner changed"))}
        match offset{0x10=>v.byte_10=value,0x11=>v.byte_11=value,_=>return Err(error("581F30 unknown component store"))}Ok(())
    }
}
impl native::MobileInitWorld for Stored<'_,'_>{
    type Actor=ObjectKey;
    fn mobile_init_active(&mut self,key:ObjectKey)->Result<bool>{
        if key!=self.key{return Err(error("5757C0 current Actor differs"))}self.owner()?;
        let state=self.staged.actors.object(key).map_err(error)?.components().map_err(error)?.state;
        //Actual4D1050 reads only Actor100 bit26 and104 bit4.
        Ok(state.low&(1<<26)!=0||state.high&(1<<4)!=0)
    }
    fn mobile_init_delay(&mut self,key:ObjectKey)->Result<u32>{
        if key!=self.key{return Err(error("529C30 current Mobile owner differs"))}
        let node=self.catalog.actors.resolved_actor(self.source_id()?).map_err(error)?;
        let source=node.child("MobileDeploymentComponent").map_err(error)?.ok_or_else(||error("5757C0 actual MobileDeployment source absent"))?;
        let value=source.field("unpack_to_actor_ids",self.source_pc).map_err(error)?.ok_or_else(||error("Mobile forward source absent"))?;
        let aa_rules::native::TypedValue::String{value,..}=value else{return Err(error("Mobile forward source is not native String"))};
        if !value.is_empty(){return Ok(0)} //4624B0 constructor554, no forward-branch store.
        source.field("unpacking_length",self.source_pc).map_err(error)?.ok_or_else(||error("4685C0 actual ActorData554 source absent"))?.fixed_bits().map_err(error)
    }
    fn mobile_init_schedule(&mut self,key:ObjectKey,delay:u32)->Result<()>{
        if key!=self.key{return Err(error("5757C0 current Updatable differs"))}self.owner()?;
        let owner=self.staged.owners.get(Target::Component{object:key,slot:Slot::MobileDeployment as u8}).map_err(error)?;
        if self.staged.queues.precision!=self.live_pc||self.staged.queues.clock.begin_bits!=self.staged.clock.begin.0||self.staged.queues.clock.step_bits!=self.staged.clock.step.0{
            return Err(error("5757C0 actual deferred queue precision/WorldE0 differs"))
        }
        //Actual embedded Updatable+C is6A71D0=true;50ACC0 chooses queue3.
        self.staged.queues.add_after(3,owner,0,delay).map_err(error)?;Ok(())
    }
    fn mobile_init_begin(&mut self)->Result<u32>{Ok(self.staged.clock.begin.0)}
    fn mobile_init_store(&mut self,key:ObjectKey,value:u32)->Result<()>{
        if key!=self.key{return Err(error("5757FE current Actor differs"))}let owner=self.owner()?;
        let Some(RuntimeComponent::MobileDeployment(v))=self.staged.actors.component_mut(owner,Slot::MobileDeployment).map_err(error)? else{return Err(error("5757FE actual Mobile payload absent"))};
        if v.header.owner!=owner{return Err(error("5757FE actual Mobile owner changed"))}v.word_14=aa_body::F32(value);Ok(())
    }
}
pub fn recruit(catalog:&Catalog<'_>,staged:&mut Staged,key:ObjectKey,pc:Precision)->Result<bool>{
    let (owner,source_pc)=checked(catalog,&staged.actors,&staged.definitions,key,Slot::Recruit)?;
    native::init_recruit(key,&mut Stored{staged,catalog,key,owner,source_pc,live_pc:pc})
}
pub fn mobile_deployment(catalog:&Catalog<'_>,staged:&mut Staged,key:ObjectKey,pc:Precision)->Result<bool>{
    let (owner,source_pc)=checked(catalog,&staged.actors,&staged.definitions,key,Slot::MobileDeployment)?;
    native::init_mobile_deployment(key,&mut Stored{staged,catalog,key,owner,source_pc,live_pc:pc},pc)
}
