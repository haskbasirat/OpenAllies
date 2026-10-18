//!The twelve property allocations published by the first8C3ED0 prefix.
//!This is an explicit portable representation of that initialized prefix,
//!not a NiMain completion, native pointer/refcount ABI or GPU-state receipt.
//!One Application retains the directory; models borrow these same owners.
use std::sync::{Arc,Weak};

type Result<T>=std::result::Result<T,String>;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
#[repr(usize)]
pub enum Kind{Alpha,Dither,Fog,Material,RendererSpecific,Shade,Specular,Stencil,Texturing,VertexColor,Wireframe,ZBuffer}
impl Kind{
 pub fn index(self)->usize{self as usize}
 pub fn name(self)->&'static str{match self{
  Self::Alpha=>"NiAlphaProperty",Self::Dither=>"NiDitherProperty",Self::Fog=>"NiFogProperty",Self::Material=>"NiMaterialProperty",
  Self::RendererSpecific=>"NiRendererSpecificProperty",Self::Shade=>"NiShadeProperty",Self::Specular=>"NiSpecularProperty",Self::Stencil=>"NiStencilProperty",
  Self::Texturing=>"NiTexturingProperty",Self::VertexColor=>"NiVertexColorProperty",Self::Wireframe=>"NiWireframeProperty",Self::ZBuffer=>"NiZBufferProperty"}}
 pub fn from_name(name:&str)->Result<Self>{ALL.into_iter().find(|kind|kind.name()==name).ok_or_else(||format!("actual Property type {name} needs its native enum/fields"))}
}
const ALL:[Kind;12]=[Kind::Alpha,Kind::Dither,Kind::Fog,Kind::Material,Kind::RendererSpecific,Kind::Shade,Kind::Specular,Kind::Stencil,Kind::Texturing,Kind::VertexColor,Kind::Wireframe,Kind::ZBuffer];

#[derive(Debug)]struct Property{
 application:Weak<()>,kind:Kind,bytes:[u8;0x44],known:[bool;0x44],
 //Native2C owns seven NULL descriptor slots. The portable growable owner
 //does not expose an invented address for that allocation or the28 codeptr.
 descriptors:Option<Vec<Option<()>>>,
}
impl Property{
 fn write(&mut self,offset:usize,bytes:&[u8]){let start=offset-0x20;self.bytes[start..start+bytes.len()].copy_from_slice(bytes);self.known[start..start+bytes.len()].fill(true);}
 fn word(&mut self,offset:usize,value:u32){self.write(offset,&value.to_le_bytes());}
 fn short(&mut self,offset:usize,value:u16){self.write(offset,&value.to_le_bytes());}
 fn byte(&mut self,offset:usize,value:u8){self.write(offset,&[value]);}
}
///Clone aliases the SAME initialized property allocation. Rust ownership does
///not project native reference counts. Unknown/padding fields stay unreadable.
#[derive(Clone,Debug)]pub struct Handle(Arc<Property>);
impl Handle{
 pub fn require_current(&self)->Result<()>{self.0.application.upgrade().ok_or("default Property Application is retired")?;Ok(())}
 pub fn kind(&self)->Kind{self.0.kind}
 pub fn same_identity(&self,other:&Self)->bool{Arc::ptr_eq(&self.0,&other.0)}
 pub fn scalar_byte(&self,offset:usize)->Result<Option<u8>>{
  self.require_current()?;let Some(index)=offset.checked_sub(0x20).filter(|i|*i<self.0.bytes.len())else{return Err("default Property scalar offset outside bounded constructor fields".into())};
  Ok(self.0.known[index].then_some(self.0.bytes[index]))
 }
 pub fn scalar_word(&self,offset:usize)->Result<Option<u32>>{let mut bytes=[0;4];for(i,b)in bytes.iter_mut().enumerate(){let Some(value)=self.scalar_byte(offset+i)?else{return Ok(None)};*b=value;}Ok(Some(u32::from_le_bytes(bytes)))}
 pub fn descriptor_slots(&self)->Result<Option<&[Option<()>]>>{self.require_current()?;Ok(self.0.descriptors.as_deref())}
}

#[derive(Debug)]pub(crate) struct State{application:Weak<()>,properties:[Handle;12]}
///A sealed read-only loan of the single initialized directory. No caller can
///supply replacement default pointers, fields or a successful init flag.
#[derive(Clone,Debug)]pub struct Capability{application:Weak<()>,properties:[Handle;12]}
impl State{
 pub(crate) fn initialized_prefix(application:&Arc<()>)->Self{
  let properties=ALL.map(|kind|{
   let mut p=Property{application:Arc::downgrade(application),kind,bytes:[0;0x44],known:[false;0x44],descriptors:None};
   match kind{
    Kind::Alpha=>{p.short(0x20,0xec);p.byte(0x22,0);},
    Kind::Dither|Kind::Specular|Kind::Wireframe=>p.short(0x20,0),
    Kind::Fog=>{p.short(0x20,0);p.word(0x24,0x3f800000);for offset in [0x28,0x2c,0x30]{p.word(offset,0)}},
    Kind::Material=>{
     //The process counter's PE initializer is1. This first material owns1;
     //the bounded producer increments the counter to2 after publication.
     p.word(0x20,1);for offset in (0x24..=0x38).step_by(4){p.word(offset,0x3f000000)}
     for offset in (0x3c..=0x50).step_by(4){p.word(offset,0)}
     p.word(0x54,0x40800000);p.word(0x58,0x3f800000);p.word(0x5c,1);
     //60 is renderer-specific mutable cache ownership. Its ctorNULL is
     //diagnostic only; no portable native address/current cache is exposed.
    },
    Kind::RendererSpecific=>p.word(0x20,0),Kind::Shade=>p.short(0x20,1),
    Kind::Stencil=>{p.byte(0x20,0);for(offset,value)in [(0x24,4),(0x28,0),(0x2c,u32::MAX),(0x30,0),(0x34,0),(0x38,3),(0x3c,0)]{p.word(offset,value)}},
    Kind::Texturing=>{p.word(0x20,2);p.byte(0x24,0);p.descriptors=Some(vec![None;7]);for(offset,value)in [(0x30,7),(0x34,7),(0x38,0),(0x3c,1),(0x40,0),(0x44,0)]{p.word(offset,value)}},
    Kind::VertexColor=>{p.word(0x24,0);p.word(0x28,1)},Kind::ZBuffer=>{p.short(0x20,3);p.word(0x24,3)},
   }
   Handle(Arc::new(p))
  });Self{application:Arc::downgrade(application),properties}
 }
 pub(crate) fn capability(&self)->Capability{Capability{application:self.application.clone(),properties:self.properties.clone()}}
}
impl Capability{
 pub(crate) fn require_application(&self,application:&Arc<()>)->Result<()>{if self.application.upgrade().is_none_or(|owner|!Arc::ptr_eq(&owner,application)){return Err("default Property directory belongs to another retired Application".into())}Ok(())}
 pub fn get(&self,kind:Kind)->Result<Handle>{let handle=&self.properties[kind.index()];handle.require_current()?;Ok(handle.clone())}
}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn initialized_prefix_matches_actual_twelve_native_constructors(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-model-defaults.json")).unwrap();
  let origin=Arc::new(());let state=State::initialized_prefix(&origin);let capability=state.capability();
  for row in fixture["rows"].as_array().unwrap(){for property in row["properties"].as_array().unwrap(){
   let index=property["index"].as_u64().unwrap()as usize;let handle=capability.get(ALL[index]).unwrap();assert_eq!(handle.kind().name(),property["class_name"].as_str().unwrap());
   for byte in property["written_scalar_bytes"].as_array().unwrap(){let offset=byte["offset"].as_u64().unwrap()as usize;
    //Native addresses and mutable renderer cache are intentionally opaque.
    if(index==8&&(0x28..0x30).contains(&offset))||(index==3&&(0x60..0x64).contains(&offset)){continue}
    assert_eq!(handle.scalar_byte(offset).unwrap(),Some(byte["value"].as_u64().unwrap()as u8),"{} offset{offset:X}",handle.kind().name());
   }
   if index==8{assert_eq!(handle.descriptor_slots().unwrap(),Some([None;7].as_slice()))}
  }}
  assert!(capability.get(Kind::Material).unwrap().same_identity(&state.capability().get(Kind::Material).unwrap()));
  assert_eq!(capability.get(Kind::VertexColor).unwrap().scalar_word(0x20).unwrap(),None);
  assert_eq!(capability.get(Kind::Alpha).unwrap().scalar_byte(0x23).unwrap(),None);
  let foreign=Arc::new(());assert!(capability.require_application(&foreign).is_err());drop(state);drop(origin);assert!(capability.get(Kind::Alpha).is_err());
 }
}
