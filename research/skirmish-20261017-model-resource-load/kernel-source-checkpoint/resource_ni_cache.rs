//!59CD10 cache-list constructor and59D240 entries used by5A5410. One growable
//!same-Application list replaces the native pool/bucket storage. Root payload
//!and cache entry have distinct allocation identities and lifetimes.
use crate::resource_objects::{Handle,Kind};
use crate::resource_model_payload::Payload;
use std::sync::{Arc,Weak};
type Result<T>=std::result::Result<T,String>;
#[derive(Clone)]pub struct EntryHandle{application:Weak<()>,identity:Weak<()>,ordinal:usize}
pub(crate) struct Entry{identity:Arc<()>,pub resource:Handle,pub root:Option<Payload>,pub filename:Arc<[u16]>,pub size:u32,
 pub previous:Option<usize>,pub next:Option<usize>,pub hash:u32}
pub(crate) struct State{application:Weak<()>,entries:Vec<Entry>,head:Option<usize>,tail:Option<usize>,
 load_context:Option<(bool,bool)>,profile_enabled:Option<bool>}
impl State{
 pub(crate) fn from_fresh_application(origin:&Arc<()>)->Self{Self{application:Arc::downgrade(origin),entries:vec![],head:None,tail:None,load_context:None,profile_enabled:None}}
 fn require(&self,resource:&Handle)->Result<()>{let app=self.application.upgrade().ok_or("Ni cache Application retired")?;resource.require_application(&app)}
 pub(crate) fn loading_context(&mut self,remove_base:bool){self.load_context=Some((true,remove_base));}
 pub(crate) fn begin_load(&mut self){self.profile_enabled=Some(false);}
 pub(crate) fn profile_enabled(&self)->Result<bool>{self.profile_enabled.ok_or_else(||"Model timing flag read before5A9A83 reset".into())}
 pub(crate) fn append(&mut self,resource:&Handle,root:Option<Payload>,size:u32)->Result<EntryHandle>{
  self.require(resource)?;if !matches!(resource.kind(),Kind::Model|Kind::Sequence){return Err("59D240 cache resource has wrong Ni class".into())}
  if let Some(root)=&root{if resource.kind()!=Kind::Model{return Err("Sequence cache cannot borrow a Model root".into())}root.require_resource(resource)?;}
  let ordinal=self.entries.len();self.entries.try_reserve(1).map_err(|e|e.to_string())?;
  let identity=Arc::new(());let handle=EntryHandle{application:self.application.clone(),identity:Arc::downgrade(&identity),ordinal};
  let hash=filename_hash(resource.logical_ids());
  self.entries.push(Entry{identity,resource:resource.clone(),root,filename:resource.logical_ids().into(),size,previous:self.tail,next:None,hash});
  if let Some(tail)=self.tail{self.entries[tail].next=Some(ordinal)}else{self.head=Some(ordinal)}self.tail=Some(ordinal);Ok(handle)
 }
 pub(crate) fn validate(&self,handle:&EntryHandle,resource:&Handle)->Result<()>{
  self.require(resource)?;if !Weak::ptr_eq(&handle.application,&self.application){return Err("foreign Ni cache entry Application".into())}
  let entry=self.entries.get(handle.ordinal).ok_or("Ni cache entry absent")?;
  if !entry.resource.same_identity(resource)||!Weak::ptr_eq(&handle.identity,&Arc::downgrade(&entry.identity))||handle.identity.upgrade().is_none(){return Err("Ni cache entry allocation differs".into())}Ok(())
 }
}
pub(crate) use crate::ni_cache_math::{filename_hash,weighted_size,weighted_size_constant};
