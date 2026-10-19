//!59CD10 cache-list constructor and59D240 entries used by5A5410. One growable
//!same-Application list replaces the native pool/bucket storage. Root payload
//!and cache entry have distinct allocation identities and lifetimes.
use crate::resource_objects::{Handle,Kind};
use crate::resource_model_payload::Payload;
use std::sync::{Arc,Weak,Mutex};
type Result<T>=std::result::Result<T,String>;
#[derive(Clone)]pub struct EntryHandle{application:Weak<()>,identity:Weak<()>,ordinal:usize}
///5A5410's +40 root of the cached Ni Resource: the Model NiAVObject or the
///Sequence NiControllerSequence (5A8190's decoded payload).
#[derive(Clone)]pub(crate) enum Root{Model(Payload),Sequence(crate::sequence_resource_load::SequencePayload)}
pub(crate) struct Entry{identity:Arc<()>,pub resource:Handle,pub root:Option<Root>,pub filename:Arc<[u16]>,pub size:u32,
 pub previous:Option<usize>,pub next:Option<usize>,pub hash:u32}
pub(crate) struct State{application:Weak<()>,entries:Vec<Entry>,head:Option<usize>,tail:Option<usize>,
 load_context:LoadingContext,profile_enabled:Option<bool>}
#[derive(Debug)]struct LoadingFields{application:Weak<()>,current:Mutex<Option<(bool,bool)>>}
///Read capability for the SAME5A99C0 process loading fields. No constructor,
///caller-provided booleans or per-Payload flag copy can create this owner.
#[derive(Clone,Debug)]pub struct LoadingContext(Arc<LoadingFields>);
impl LoadingContext{
 pub(crate) fn require_application(&self,application:&Arc<()>)->Result<()>{if self.0.application.upgrade().is_none_or(|owner|!Arc::ptr_eq(&owner,application)){return Err("Model loading context belongs to another retired Application".into())}Ok(())}
 pub fn remove_base_mapping_current(&self)->Result<bool>{
  self.0.application.upgrade().ok_or("Model loading context Application retired")?;
  self.0.current.lock().map_err(|_|"Model loading context lock poisoned")?.map(|(_,remove)|remove).ok_or_else(||"879A90 currentB43E88 read precedes actual5A99C0 writer".into())
 }
}
impl State{
 pub(crate) fn from_fresh_application(origin:&Arc<()>)->Self{Self{application:Arc::downgrade(origin),entries:vec![],head:None,tail:None,load_context:LoadingContext(Arc::new(LoadingFields{application:Arc::downgrade(origin),current:Mutex::new(None)})),profile_enabled:None}}
 fn require(&self,resource:&Handle)->Result<()>{let app=self.application.upgrade().ok_or("Ni cache Application retired")?;resource.require_application(&app)}
 pub(crate) fn loading_context(&mut self,remove_base:bool)->Result<()>{*self.load_context.0.current.lock().map_err(|_|"Model loading context lock poisoned")?=Some((true,remove_base));Ok(())}
 pub(crate) fn loading_context_capability(&self)->LoadingContext{self.load_context.clone()}
 pub(crate) fn begin_load(&mut self){self.profile_enabled=Some(false);}
 pub(crate) fn profile_enabled(&self)->Result<bool>{self.profile_enabled.ok_or_else(||"Model timing flag read before5A9A83 reset".into())}
 pub(crate) fn append(&mut self,resource:&Handle,root:Option<Root>,size:u32)->Result<EntryHandle>{
  self.require(resource)?;if !matches!(resource.kind(),Kind::Model|Kind::Sequence){return Err("59D240 cache resource has wrong Ni class".into())}
  match &root{
   Some(Root::Model(root))=>{if resource.kind()!=Kind::Model{return Err("Sequence cache cannot borrow a Model root".into())}root.require_resource(resource)?;}
   Some(Root::Sequence(_))=>if resource.kind()!=Kind::Sequence{return Err("Model cache cannot borrow a Sequence root".into())},
   None=>{}
  }
  let ordinal=self.entries.len();self.entries.try_reserve(1).map_err(|e|e.to_string())?;
  let identity=Arc::new(());let handle=EntryHandle{application:self.application.clone(),identity:Arc::downgrade(&identity),ordinal};
  let hash=filename_hash(resource.logical_ids());
  self.entries.push(Entry{identity,resource:resource.clone(),root,filename:resource.logical_ids().into(),size,previous:self.tail,next:None,hash});
  if let Some(tail)=self.tail{self.entries[tail].next=Some(ordinal)}else{self.head=Some(ordinal)}self.tail=Some(ordinal);Ok(handle)
 }
 ///59DAB0 then85C2B0 on the SAME Resource+3C entry (Model vf40 5A6560,
 ///Sequence vf40 5A6530): unlink from the B3D7A0 head+20/tail+24 list exactly
 ///as 59DAB0 does, then release the entry's root/filename. Pool and string
 ///frees are host ownership drops; the stale EntryHandle can never validate.
 pub(crate) fn remove_59dab0(&mut self,handle:&EntryHandle,resource:&Handle)->Result<()>{
  self.validate(handle,resource)?;let i=handle.ordinal;let(previous,next)=(self.entries[i].previous,self.entries[i].next);
  if self.head==Some(i){self.head=next}
  if self.tail==Some(i){self.tail=previous}
  if let Some(p)=previous{self.entries[p].next=next}
  if let Some(n)=next{self.entries[n].previous=previous}
  let entry=&mut self.entries[i];entry.previous=None;entry.next=None;entry.root=None;entry.identity=Arc::new(());Ok(())
 }
 ///Resources reachable from the list head, in list order (test).
 #[cfg(test)]
 pub(crate) fn linked(&self)->Vec<Handle>{let mut out=vec![];let mut at=self.head;while let Some(i)=at{out.push(self.entries[i].resource.clone());at=self.entries[i].next}out}
 pub(crate) fn validate(&self,handle:&EntryHandle,resource:&Handle)->Result<()>{
  self.require(resource)?;if !Weak::ptr_eq(&handle.application,&self.application){return Err("foreign Ni cache entry Application".into())}
  let entry=self.entries.get(handle.ordinal).ok_or("Ni cache entry absent")?;
  if !entry.resource.same_identity(resource)||!Weak::ptr_eq(&handle.identity,&Arc::downgrade(&entry.identity))||handle.identity.upgrade().is_none(){return Err("Ni cache entry allocation differs".into())}Ok(())
 }
}
pub(crate) use crate::ni_cache_math::{filename_hash,weighted_size,weighted_size_constant};
#[cfg(test)]mod loading_context_tests{
 use super::*;
 #[test]fn actual_writer_and_retained_read_capability_share_one_current_field(){
  let origin=Arc::new(());let mut state=State::from_fresh_application(&origin);let read=state.loading_context_capability();
  assert!(read.remove_base_mapping_current().is_err());state.loading_context(true).unwrap();assert!(read.remove_base_mapping_current().unwrap());
  state.loading_context(false).unwrap();assert!(!read.remove_base_mapping_current().unwrap());
  let foreign=Arc::new(());assert!(read.require_application(&foreign).is_err());drop(state);drop(origin);assert!(read.remove_base_mapping_current().is_err());
 }
}
