//! Replacement drain composition with the same retained mutable request.
//! The sealed startup table is borrowed from its application owner. Remaining
//! destruction/property/publication children retain the drain's real required
//! adapters; no request Params clone or failure rollback exists here.
use crate::{actor_creation_startup::StartupInputs,actor_world::ObjectKey,catalog::Catalog,
    map_replacement_drain::{self as drain,Effects,Result},
    map_replacement_queue::{ActorRequest,ObjectRequest,DefinitionRef,RequestRef},restoration::Staged};
use aa_numeric::Precision;
pub struct Creation<'a>{startup:&'a StartupInputs,required:drain::Required}
impl<'a> Creation<'a>{pub fn new(startup:&'a StartupInputs)->Self{Self{startup,required:drain::Required}}}
impl Effects for Creation<'_>{
    fn remove(&mut self,s:&mut Staged,c:&Catalog<'_>,old:ObjectKey,p:Precision)->Result<()>{
        self.required.remove(s,c,old,p)
    }
    fn virtual_160(&mut self,s:&mut Staged,c:&Catalog<'_>,old:ObjectKey,arg:u32,p:Precision)->Result<()>{
        self.required.virtual_160(s,c,old,arg,p)
    }
    fn create_object(&mut self,s:&mut Staged,c:&Catalog<'_>,r:&mut ObjectRequest,p:Precision)->Result<Option<ObjectKey>>{
        self.required.create_object(s,c,r,p)
    }
    fn create_actor(&mut self,s:&mut Staged,c:&Catalog<'_>,r:&mut ActorRequest,p:Precision)->Result<Option<ObjectKey>>{
        crate::map_actor_creation::create(s,c,&mut r.params,self.startup,p)
    }
    fn property(&mut self,s:&mut Staged,c:&Catalog<'_>,key:ObjectKey,d:Option<&DefinitionRef>,p:Precision)->Result<()>{
        let Some(d)=d else{
            crate::staged_property_admission::apply(s,c,key,None,2,None,p)?;
            return Ok(());
        };
        let entry=s.definitions.require_usable(d.token)?;
        if entry.key.domain!="property"||!entry.key.name.eq_ignore_ascii_case(&d.canonical_ids){
            return Err("4FBA50 copied Property definition/actual Bank identity differs".into());
        }
        s.actors.property_allocations.bind_in_bank(c,&mut s.definitions)?;
        let reference=s.actors.property_allocations.facts(aa_body::credit::Object(d.token))?.reference();
        let id=s.actors.id(key).ok_or("4FBA50 created actor GTID is absent before property apply")?;
        //Native ignores4CEF10's AL and continues the actual copied pointer list.
        //A reached insertion child error still retains all preceding writes.
        crate::staged_property_admission::apply(s,c,key,Some(reference),2,Some(id),p)?;
        Ok(())
    }
    fn created_log(&mut self,s:&mut Staged,r:RequestRef<'_>)->Result<()>{self.required.created_log(s,r)}
    fn failure_log(&mut self,s:&mut Staged,r:RequestRef<'_>)->Result<()>{self.required.failure_log(s,r)}
    fn failure_text_ui(&mut self,s:&mut Staged,c:&Catalog<'_>,r:RequestRef<'_>,p:Precision)->Result<()>{
        self.required.failure_text_ui(s,c,r,p)
    }
}
