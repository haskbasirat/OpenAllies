//!5A99C0 on the SAME Model Entry. Native scalar/publication order belongs to
//!model_load_kernel; retained selected bytes and graph payload never set flags.
use crate::{model_load_kernel as native,resource_depot::Resources,resource_objects::Handle,
 resource_model_payload::{Input,Payload,Metadata,Selection}};
use aa_numeric::Precision;
pub type Result<T>=std::result::Result<T,String>;
pub trait Backend:crate::resource_objects::ModelPolicy{
 fn resource_no_ni_objects_current(&mut self)->Result<u8>{Err("5A99C0 actual named ResourceNoNiObjects is required".into())}
 fn begin_model_load(&mut self,_resource:&Handle)->Result<()>{Err("5A99C0 actual model timer/context reset is required".into())}
 fn decode_selected_model(&mut self,_input:&Input)->Result<Payload>{Err("5A7120 actual selected NIF decoder is required".into())}
 fn default_model(&mut self,_resource:&Handle)->Result<Option<Payload>>{Err("5A6610 actual default model owner is required".into())}
 fn hardware_palette_count(&mut self)->Result<u32>{Err("5A65B0 actual current AVar51 RendererHBPPCount is required".into())}
 fn process_model_metadata(&mut self,_payload:&Payload,_has_skin:bool)->Result<Metadata>{Err("87AA10 actual current per-geometry metadata policy is required".into())}
 fn skip_loaded_model_bounds(&mut self)->Result<u8>{Err("5A99C0 actual indexed UVar54 current owner is required".into())}
 fn profile_stage(&mut self,_resource:&Handle,_label:&'static str)->Result<()>{Err("actual model diagnostic/timing sink is required".into())}
 fn display_profile(&mut self,_resource:&Handle)->Result<()>{Err("41C3F0 actual model timing display is required".into())}
 ///5A94F0 404ED0(B38C28) diagnostics, in call order.
 fn selection_diagnostic_404ed0(&mut self,_resource:&Handle,_diagnostic:SelectionDiagnostic)->Result<()>{Err("5A94F0 actual 404ED0 selection diagnostic sink is required".into())}
}
///The two reached 5A94F0 404ED0 calls (level 1 on channel B38C28).
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SelectionDiagnostic{
 ///0x56F, only when Resource flags+8 bit11 is set.
 NoSelectionBox,
 ///0x57D, after the fallback cuboid was stored.
 BoundingSphereIncorrect,
}
impl SelectionDiagnostic{
 pub const FILE:&'static str=".\\Source\\resource\\NiResource.cpp";
 pub const FUNCTION:&'static str="KKC_NiAVObjResource::ComputeSelectionCuboid";
 pub const CHANNEL:u32=0xb38c28;
 pub const LEVEL:u32=1;
 pub fn line(&self)->u32{match self{Self::NoSelectionBox=>0x56f,Self::BoundingSphereIncorrect=>0x57d}}
 pub fn format(&self)->&'static str{match self{Self::NoSelectionBox=>"Object model has no selection box: %s",Self::BoundingSphereIncorrect=>"Bounding sphere for art is incorrect: %s"}}
}
pub(crate) struct Call{state:native::State,input:Option<Input>,payload:Option<Payload>,metadata:Option<Metadata>,names:Option<Vec<Vec<u16>>>}
impl Call{fn new()->Self{Self{state:native::State::new_call(),input:None,payload:None,metadata:None,names:None}}}
pub(crate) fn load(resources:&mut Resources<'_>,resource:&Handle,pc:Precision,backend:&mut impl Backend)->Result<bool>{
 let mut call=resources.with_entry(resource,|e|{
  let m=e.model.as_mut().ok_or("5A99C0 requires actual Model allocation")?;
  if m.loading||m.load_call.as_ref().is_some_and(|c|c.state.stage()!=native::Stage::NotStarted){return Err("Model retains a prior native load call; actual unload is required".into())}
  m.loading=true;Ok(m.load_call.take().unwrap_or_else(Call::new))
 })?;
 let result={let Call{state,input,payload,metadata,names}=&mut call;
  state.load(&mut Fields{resources,resource,backend,input,payload,metadata,names,pc})};
 let restore=resources.with_entry(resource,|e|{let m=e.model.as_mut().ok_or("same Model fields changed")?;
  if !m.loading||m.load_call.is_some(){return Err("same Model loader owner changed during callback".into())}
  m.load_call=Some(call);m.loading=false;Ok(())});
 match(result,restore){(Err(e),_)=>Err(e),(Ok(_),Err(e))=>Err(e),(Ok(v),Ok(()))=>Ok(v)}
}
struct Fields<'a,'r,B>{resources:&'a mut Resources<'r>,resource:&'a Handle,backend:&'a mut B,input:&'a mut Option<Input>,payload:&'a mut Option<Payload>,metadata:&'a mut Option<Metadata>,names:&'a mut Option<Vec<Vec<u16>>>,pc:Precision}
impl<B:Backend>Fields<'_,'_,B>{fn payload(&self)->Result<&Payload>{self.payload.as_ref().ok_or_else(||"5A99C0 actual local NiAVObject is NULL".into())}
 fn metadata(&self)->Result<&Metadata>{self.metadata.as_ref().ok_or_else(||"87AA10 actual processed metadata is absent".into())}}
impl<B:Backend>native::Fields for Fields<'_,'_,B>{
 fn no_ni_objects(&mut self)->Result<u8>{self.backend.resource_no_ni_objects_current()}
 fn word(&mut self,o:u32)->Result<u32>{self.resources.with_entry(self.resource,|e|{let m=e.model.as_ref().ok_or("actual Model absent")?;Ok(match o{8=>e.flags8,0xc=>e.memory0c,0x10=>e.memory10,0x48=>m.value48,0x64=>m.radius64,0x68..=0x74 if o%4==0=>m.factors68_74[((o-0x68)/4)as usize],0x78|0x7c=>m.words78_7c[((o-0x78)/4)as usize],0x84=>m.count84 as u32,_=>return Err("unknown native Model word read".into())})})}
 fn store(&mut self,o:u32,v:u32)->Result<()>{self.resources.with_entry(self.resource,|e|{let m=e.model.as_mut().ok_or("actual Model absent")?;match o{8=>e.flags8=v,0xc=>e.memory0c=v,0x10=>e.memory10=v,0x48=>m.value48=v,0x64=>m.radius64=v,0x68..=0x74 if o%4==0=>m.factors68_74[((o-0x68)/4)as usize]=v,0x78|0x7c=>m.words78_7c[((o-0x78)/4)as usize]=v,0x84=>m.count84=v as usize,_=>return Err("unknown native Model word store".into())}Ok(())})}
 fn filename_length(&mut self)->Result<u32>{u32::try_from(self.resource.logical_ids().len()).map_err(|e|e.to_string())}
 fn begin(&mut self)->Result<()>{self.resources.model_begin_load_5a9a83()?;self.backend.begin_model_load(self.resource)}
 fn open_input(&mut self)->Result<()>{if self.input.is_some(){return Err("Model selected stream is already retained".into())}*self.input=Some(self.resources.model_input_4c(self.resource)?);Ok(())}
 fn input_size(&mut self)->Result<u32>{let bytes=self.input.as_ref().ok_or("Model selected stream absent")?.source().size();crate::resource_ni_cache::weighted_size_constant(bytes as u32,0x3fa8f5c3,self.pc)}
 fn decode_input(&mut self)->Result<()>{let p=self.backend.decode_selected_model(self.input.as_ref().ok_or("Model selected stream absent")?)?;p.require_resource(self.resource)?;*self.payload=Some(p);Ok(())}
 fn close_input(&mut self)->Result<()>{drop(self.input.take().ok_or("876010 same Model selected stream absent")?);Ok(())}
 fn default_payload(&mut self)->Result<()>{let p=self.backend.default_model(self.resource)?;if let Some(p)=&p{p.require_resource(self.resource)?}*self.payload=p;Ok(())}
 fn payload_present(&mut self)->Result<bool>{Ok(self.payload.is_some())}
 fn unable_to_load(&mut self)->Result<()>{Err("4252A0 Unable to load AVObject from selected source".into())}
 fn loading_context(&mut self)->Result<()>{let flags=self.word(8)?;self.resources.model_loading_context_5a99c0(flags&4!=0)}
 fn hardware_model(&mut self)->Result<bool>{self.backend.use_hardware_model_73a560()}
 fn hardware_count(&mut self)->Result<u32>{self.backend.hardware_palette_count()}
 fn hardware_transform(&mut self,count:u32)->Result<()>{self.payload()?.hardware_transform(count)}
 fn process_metadata(&mut self,has_skin:bool)->Result<()>{let payload=self.payload.as_ref().ok_or("5A99C0 actual local NiAVObject is NULL")?;*self.metadata=Some(self.backend.process_model_metadata(payload,has_skin)?);Ok(())}
 fn metadata_word(&mut self,o:u32)->Result<u32>{let m=self.metadata()?;Ok(match o{4=>m.word4,8=>m.radial8,0xc=>m.max_z0c,0x10=>m.min_z10,0x14=>m.footprint14,0x18=>m.footprint18,0x1c=>m.footprint1c,0x20=>m.texture_count20,_=>return Err("unknown model metadata word".into())})}
 fn metadata_byte(&mut self,o:u32)->Result<u8>{let m=self.metadata()?;Ok(match o{1=>m.byte1,0x24=>m.byte24,_=>return Err("unknown model metadata byte".into())})}
 fn texture_capacity(&mut self,count:u32)->Result<()>{self.resources.with_entry(self.resource,|e|{let m=e.model.as_mut().ok_or("actual Model absent")?;let count=crate::model_texture_path::capacity(m.slots80.len(),count as usize)?;if m.slots80.len()<count{m.slots80.try_reserve(count-m.slots80.len()).map_err(|e|e.to_string())?;m.slots80.resize_with(count,||([].into(),[].into()));}Ok(())})}
 fn collect_texture_names(&mut self)->Result<()>{let names=self.payload()?.texture_names()?;if names.len()!=self.metadata()?.texture_count20 as usize{return Err("87AA10/8784F0 actual texture count differs".into())}*self.names=Some(names);Ok(())}
 fn append_texture(&mut self,index:u32)->Result<()>{let name=self.names.as_ref().and_then(|n|n.get(index as usize)).ok_or("actual model texture ordinal absent")?;
  let pair=crate::model_texture_path::pair(self.resource.logical_ids(),name)?;self.resources.with_entry(self.resource,|e|{let m=e.model.as_mut().ok_or("actual Model absent")?;let slot=m.slots80.get_mut(m.count84).ok_or("actual Model texture capacity exhausted")?;*slot=(pair.0.into(),pair.1.into());m.count84+=1;Ok(())})}
 fn profile(&mut self,label:&'static str)->Result<()>{if self.resources.model_profile_enabled()?{self.backend.profile_stage(self.resource,label)}else{Ok(())}}
 fn profile_enabled(&mut self)->Result<bool>{self.resources.model_profile_enabled()}
 fn prepare_effect(&mut self,code:u32)->Result<()>{self.payload()?.prepare_effect(code)}
 fn skip_loaded_bounds(&mut self)->Result<u8>{self.backend.skip_loaded_model_bounds()}
 fn controllers(&mut self)->Result<bool>{self.payload()?.controllers()}
 fn selection_cuboid(&mut self)->Result<()>{
  let store=|f:&mut Self,words:[u32;6]|f.resources.with_entry(f.resource,|e|{e.model.as_mut().ok_or("actual Model absent")?.words4c_60=words;Ok(())});
  match self.payload()?.selection_5a94f0()?{
   Selection::Geometry(words)=>store(self,words),
   Selection::RootSphere(sphere)=>{
    if (native::Fields::word(self,8)?>>11)&1!=0{self.backend.selection_diagnostic_404ed0(self.resource,SelectionDiagnostic::NoSelectionBox)?;}
    let fallback=crate::resource_model_bounds::selection_from_sphere(sphere,self.pc)?;
    store(self,fallback.cuboid)?;
    if fallback.incorrect{self.backend.selection_diagnostic_404ed0(self.resource,SelectionDiagnostic::BoundingSphereIncorrect)?;}
    Ok(())
   }
  }
 }
 fn wrap_root(&mut self)->Result<()>{self.payload()?.wrap_root()}
 fn update_root(&mut self)->Result<()>{self.payload()?.update_root()}
 fn property_name_length(&mut self)->Result<u32>{self.resources.with_entry(self.resource,|e|Ok(e.model.as_ref().ok_or("actual Model absent")?.name8c.len()as u32))}
 fn apply_named_property(&mut self)->Result<()>{let name=self.resources.with_entry(self.resource,|e|Ok(e.model.as_ref().ok_or("actual Model absent")?.name8c.clone()))?;self.payload()?.apply_named_property(&name)}
 fn publish_payload(&mut self)->Result<()>{let payload=self.payload()?.clone();payload.require_resource(self.resource)?;self.resources.with_entry(self.resource,|e|{e.model.as_mut().ok_or("actual Model absent")?.source40=Some(payload);Ok(())})}
 fn display_profile(&mut self)->Result<()>{if self.resources.model_profile_enabled()?{self.backend.display_profile(self.resource)}else{Ok(())}}
}
