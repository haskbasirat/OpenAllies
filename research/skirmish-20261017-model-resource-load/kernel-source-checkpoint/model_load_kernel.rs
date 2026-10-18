//!Native5A99C0 ordered NiAVObjResource field coordinator. Decoding and graph
//!operations act on the caller's retained payload, never on a scalar stand-in.
pub type Result<T>=std::result::Result<T,String>;
#[cfg(test)]
#[path="model_load_kernel_tests.rs"]
mod tests;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Stage{NotStarted,Started,Input,Payload,Metadata,Textures,Prepared,Bounds,Wrapped,Published,Complete}
pub struct State{stage:Stage}
impl State{pub fn new_call()->Self{Self{stage:Stage::NotStarted}}pub fn stage(&self)->Stage{self.stage}
 pub fn load(&mut self,f:&mut impl Fields)->Result<bool>{
  if f.no_ni_objects()?==1{let flags=f.word(8)?;f.store(8,flags|0x40)?;}
  if f.filename_length()?==0{return Ok(false)}
  f.begin()?;self.stage=Stage::Started;
  if f.word(8)?&0x40==0{
   f.open_input()?;self.stage=Stage::Input;
   let bytes=f.input_size()?;f.store(0x0c,bytes)?;f.store(0x10,bytes)?;
   f.decode_input()?;f.close_input()?;
  }else{f.default_payload()?;f.store(0x0c,100)?;f.store(0x10,100)?;}
  self.stage=Stage::Payload;
  if !f.payload_present()?{return f.unable_to_load().map(|_|false)}
  f.loading_context()?;
  if f.hardware_model()?{let count=f.hardware_count()?;f.hardware_transform(count)?;}
  let has_skin=f.word(8)?&0x8000!=0;f.process_metadata(has_skin)?;self.stage=Stage::Metadata;
  f.store(0x84,0)?;let count=f.metadata_word(0x20)?;f.store(0x7c,count)?;
  if count!=0{f.texture_capacity(count)?;f.collect_texture_names()?;for index in 0..count{f.append_texture(index)?;}}
  self.stage=Stage::Textures;
  if f.metadata_byte(1)?!=0{let flags=f.word(8)?;f.store(8,flags|0x20000)?;}
  let markings=f.metadata_byte(0x24)?!=0;let flags=f.word(8)?;f.store(8,if markings{flags|0x800000}else{flags&!0x800000})?;
  if f.profile_enabled()?{f.profile("PREPARE")?;}
  for(bit,code)in[(10,0x871bc0),(19,4),(22,5),(13,0x871b40)]{if f.word(8)?&(1<<bit)!=0{f.prepare_effect(code)?;}}
  self.stage=Stage::Prepared;if f.profile_enabled()?{f.profile("MARKINGS")?;}
  if f.word(8)?&4==0||f.skip_loaded_bounds()?==0{
   let value=f.metadata_word(4)?;f.store(0x48,value)?;f.profile("TIMECONTROLLERS")?;
   let controllers=f.controllers()?;let flags=f.word(8)?;f.store(8,if controllers{flags|0x200}else{flags&!0x200})?;
   let count=f.metadata_word(0x20)? as i32;let flags=f.word(8)?;f.store(8,if count>0{flags|0x4000}else{flags&!0x4000})?;
   f.selection_cuboid()?;f.profile("CUBOID")?;
   for(o,v)in[(0x6c,0),(0x70,0),(0x74,0x3f800000),(0x68,0x3f800000),(0x64,0x3f800000),(0x78,0)]{f.store(o,v)?;}
   let radius=f.metadata_word(8)?;
   if f32::from_bits(radius)>0.0{
    f.store(0x64,radius)?;let maximum=f.metadata_word(0xc)?;f.store(0x74,maximum)?;let minimum=f.metadata_word(0x10)?;f.store(0x78,minimum)?;
    let footprint=f.metadata_word(0x14)?;if f32::from_bits(footprint)>0.0{
     f.store(0x68,footprint)?;let x=f.metadata_word(0x18)?;f.store(0x6c,x)?;let y=f.metadata_word(0x1c)?;f.store(0x70,y)?;
    }
   }f.profile("BOUNDS")?;
  }
  self.stage=Stage::Bounds;f.wrap_root()?;f.update_root()?;self.stage=Stage::Wrapped;if f.profile_enabled()?{f.profile("PRECACHE")?;}
  if f.property_name_length()?!=0{f.apply_named_property()?;}
  f.publish_payload()?;self.stage=Stage::Published;f.display_profile()?;self.stage=Stage::Complete;Ok(true)
 }
}
pub trait Fields{
 fn no_ni_objects(&mut self)->Result<u8>;fn word(&mut self,offset:u32)->Result<u32>;fn store(&mut self,offset:u32,value:u32)->Result<()>;
 fn filename_length(&mut self)->Result<u32>;fn begin(&mut self)->Result<()>;fn open_input(&mut self)->Result<()>;fn input_size(&mut self)->Result<u32>;fn decode_input(&mut self)->Result<()>;fn close_input(&mut self)->Result<()>;fn default_payload(&mut self)->Result<()>;fn payload_present(&mut self)->Result<bool>;fn unable_to_load(&mut self)->Result<()>;fn loading_context(&mut self)->Result<()>;
 fn hardware_model(&mut self)->Result<bool>;fn hardware_count(&mut self)->Result<u32>;fn hardware_transform(&mut self,count:u32)->Result<()>;
 fn process_metadata(&mut self,unique:bool)->Result<()>;fn metadata_word(&mut self,offset:u32)->Result<u32>;fn metadata_byte(&mut self,offset:u32)->Result<u8>;
 fn texture_capacity(&mut self,count:u32)->Result<()>;fn collect_texture_names(&mut self)->Result<()>;fn append_texture(&mut self,index:u32)->Result<()>;
 fn profile_enabled(&mut self)->Result<bool>;fn profile(&mut self,label:&'static str)->Result<()>;fn prepare_effect(&mut self,code:u32)->Result<()>;fn skip_loaded_bounds(&mut self)->Result<u8>;fn controllers(&mut self)->Result<bool>;fn selection_cuboid(&mut self)->Result<()>;
 fn wrap_root(&mut self)->Result<()>;fn update_root(&mut self)->Result<()>;fn property_name_length(&mut self)->Result<u32>;fn apply_named_property(&mut self)->Result<()>;fn publish_payload(&mut self)->Result<()>;fn display_profile(&mut self)->Result<()>;
}
