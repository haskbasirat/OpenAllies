//!5A8190 KKC_NiSeqResource::Load (Sequence vtable A45290 +0x4C) over the
//!SAME Sequence Entry:
//!
//! * 404F90 "Loading KF resource '%s'" (line 0x6E2);
//! * the ResourceNoNiObjects UVar (B3D8E8, bound once) == 1 sets flags+8
//!   bit 0x40; the load still continues (unlike the Model path);
//! * an empty file name returns 0;
//! * 5AA340 opens the selected file; its size times 60.0 (A44EC0), through
//!   96CC4C, is stored in +0x0C and +0x10 (5AA270);
//! * 5A7230 KKC_NiSeqResource::LoadSequence decodes the stream (the
//!   backend's decoder), the file is closed; a NULL result reports
//!   "Unable to load Sequence: %s" (4252A0) and 5A5650 then reads the NULL
//!   sequence (explicit error here);
//! * 5A5650 ComputeLatestTimeController -> +0x4C, the sequence -> +0x40;
//! * returns flags+8 bit 0.
use crate::resource_depot::Resources;
use crate::resource_objects::Handle;
use aa_numeric::Precision;
use std::sync::Arc;
pub type Result<T>=std::result::Result<T,String>;

///The decoded NiControllerSequence root (+0x40), owned by the decoder.
#[derive(Clone)]
pub struct SequencePayload(pub Arc<dyn std::any::Any+Send+Sync>);
impl std::fmt::Debug for SequencePayload{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.write_str("SequencePayload")}}

pub trait Backend{
 fn resource_no_ni_objects_current(&mut self)->Result<u8>{Err("5A8190 actual named ResourceNoNiObjects is required".into())}
 ///404F90(B38B98) "Loading KF resource '%s'", line 0x6E2.
 fn sequence_loading_404f90(&mut self,_resource:&Handle)->Result<()>{Err("5A8190 actual 404F90 loading log is required".into())}
 ///5A7230 over the selected bytes: the root and its 5A5650 length (binary32 bits).
 fn decode_selected_sequence(&mut self,_resource:&Handle,_bytes:&[u8])->Result<Option<(SequencePayload,u32)>>{Err("5A7230 actual selected KF decoder is required".into())}
 ///4252A0 "Unable to load Sequence: %s".
 fn unable_to_load_sequence_4252a0(&mut self,_resource:&Handle)->Result<()>{Err("5A8190 actual 4252A0 diagnostic is required".into())}
}

impl Resources<'_>{
 pub fn load_sequence_4c(&mut self,handle:&Handle,pc:Precision,backend:&mut impl Backend)->Result<bool>{
  if handle.kind()!=crate::resource_objects::Kind::Sequence{return Err("5A8190 requires an actual Sequence Resource".into())}
  self.with_entry(handle,|e|e.sequence.as_ref().map(|_|()).ok_or_else(||"5A8190 Sequence fields absent".to_string()))?;
  backend.sequence_loading_404f90(handle)?;
  if backend.resource_no_ni_objects_current()?==1{self.with_entry(handle,|e|{e.flags8|=0x40;Ok(())})?;}
  if handle.logical_ids().is_empty(){return Ok(false)}
  let file=self.read_source(handle)?;
  let size=u32::try_from(file.bytes.len()).map_err(|e|e.to_string())?;
  let weight=crate::resource_ni_cache::weighted_size_constant(size,0x4270_0000,pc)?;
  self.with_entry(handle,|e|{e.memory0c=weight;e.memory10=weight;Ok(())})?;
  let decoded=backend.decode_selected_sequence(handle,&file.bytes)?;
  drop(file);
  let Some((payload,length))=decoded else{
   backend.unable_to_load_sequence_4252a0(handle)?;
   return Err("5A82EB 5A5650 reads the NULL sequence 5A7230 returned".into());
  };
  self.with_entry(handle,|e|{
   let s=e.sequence.as_mut().ok_or("5A8190 Sequence fields changed")?;
   s.value4c=length;s.source40=Some(());s.root40=Some(payload);Ok(e.flags8&1!=0)
  })
 }
 ///The +0x40 NiControllerSequence of a loaded Sequence (NULL after unload).
 pub fn sequence_payload_40(&self,handle:&Handle)->Result<Option<SequencePayload>>{
  self.with_entry(handle,|e|{let s=e.sequence.as_ref().ok_or("actual Sequence allocation required")?;Ok(if s.source40.is_some(){s.root40.clone()}else{None})})
 }
 ///The +0x4C length 5A5650 stored (binary32 bits).
 pub fn sequence_length_4c(&self,handle:&Handle)->Result<u32>{
  self.with_entry(handle,|e|Ok(e.sequence.as_ref().ok_or("actual Sequence allocation required")?.value4c))
 }
}
