//!874B40/86F710/8751C0 numeric and owned-list constructor on the SAME camera.
//!Rust allocations replace native Ni ABI/reference bookkeeping, not field
//!values. Positive external list/parent/controller mutation is not supplied.
use super::{State,Result};
use crate::view_grid_state::Globals;
use aa_numeric::Precision;

enum Reference {}
struct Array{slots:Vec<Option<Reference>>,used:u32,effective:u32,grow_by:u32}
impl Array{
 fn constructed()->Result<Self>{
  let mut slots=Vec::new();slots.try_reserve_exact(5).map_err(|e|e.to_string())?;
  slots.resize_with(5,||None);Ok(Self{slots,used:0,effective:0,grow_by:5})
 }
 fn word(&self,relative:u32)->Option<Result<u32>>{match relative{
  4=>Some(Err("NiCamera array owns its allocation; no raw native pointer exists".into())),
  8=>Some(u32::try_from(self.slots.len()).map_err(|_|"NiCamera array capacity exceeds native representation".into())),
  12=>Some(Ok(self.used)),16=>Some(Ok(self.effective)),20=>Some(Ok(self.grow_by)),_=>None,
 }}
 fn store(&mut self,relative:u32,value:u32)->bool{match relative{
  12=>self.used=value,16=>self.effective=value,20=>self.grow_by=value,_=>return false,
 }true}
}
#[derive(Clone,Copy,PartialEq,Eq)]enum Phase{Unbound,Constructing,Complete,Failed}
pub(super) struct Fields{first:Option<Array>,second:Option<Array>,phase:Phase}
impl Fields{
 pub(super) fn unbound()->Self{Self{first:None,second:None,phase:Phase::Unbound}}
 pub(super) fn word(&self,offset:u32)->Option<Result<u32>>{
  if(0x168..0x180).contains(&offset){self.first.as_ref()?.word(offset-0x168)}
  else if(0x180..0x198).contains(&offset){self.second.as_ref()?.word(offset-0x180)}else{None}
 }
 pub(super) fn store(&mut self,offset:u32,value:u32)->bool{
  if(0x168..0x180).contains(&offset){self.first.as_mut().is_some_and(|v|v.store(offset-0x168,value))}
  else if(0x180..0x198).contains(&offset){self.second.as_mut().is_some_and(|v|v.store(offset-0x180,value))}else{false}
 }
}
impl State{
 pub(crate) fn require_ni_base_constructor(&self)->Result<()>{
  if self.base.phase==Phase::Complete{Ok(())}else{Err("actual NiCamera874B40 numeric/list constructor has not completed".into())}
 }
}
fn vector(camera:&mut State,offset:u32,globals:&mut impl Globals)->Result<()>{
 for(i,address)in[0xb43cf8,0xb43cfc,0xb43d00].into_iter().enumerate(){
  let word=globals.word(address)?;camera.put(offset+4*i as u32,word);
 }Ok(())
}
fn transform(camera:&mut State,offset:u32,globals:&mut impl Globals)->Result<()>{
 for(i,word)in[0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000].into_iter().enumerate(){camera.put(offset+4*i as u32,word)}
 let x=globals.word(0xb43cf8)?;camera.put(offset+0x24,x);
 let y=globals.word(0xb43cfc)?;camera.put(offset+0x28,y);
 let z=globals.word(0xb43d00)?;camera.put(offset+0x30,0x3f800000);camera.put(offset+0x2c,z);Ok(())
}
///The actual constructor source reads must come from the retained startup
///Globals binder. A laboratory Globals is only a declared numeric test input.
///No renderer/directory/AD capability is granted by this CPU constructor.
pub(crate) fn initialize_874b40(camera:&mut State,globals:&mut impl Globals,pc:Precision)->Result<()>{
 if camera.base.phase!=Phase::Unbound{return Err("NiCamera base constructor already attempted".into())}
 camera.base.phase=Phase::Constructing;
 let result=(||{
  //86A680/86B240 data fields. Native vtables and the Ni-instance
  //counter/refcount ABI are deliberately not portable pointer identities.
  for offset in[4,8,0x10,0x0c,0x14,0x1c,0x18]{camera.put(offset,0)}
  for offset in[0xac,0xa4,0xa8,0xb0]{camera.put(offset,0)}
  camera.put_byte(0x20,0);camera.put_byte(0x21,0);camera.put(0x24,0);
  transform(camera,0x38,globals)?;transform(camera,0x6c,globals)?;
  let x=globals.word(0xb43cf8)?;camera.put(0x28,x);
  let y=globals.word(0xb43cfc)?;camera.put(0x2c,y);
  let z=globals.word(0xb43d00)?;camera.put(0x34,0);camera.put(0x30,z);
  camera.put_byte(0x20,0x10);camera.put_byte(0x21,0);
  camera.put_byte(0x20,0x30);
  for offset in[0xb4,0xb8,0xbc,0xc0]{camera.put(offset,0)}
  camera.put_byte(0x150,0);
  for offset in[0x154,0x158,0x15c,0x160,0x164]{camera.put(offset,0)}
  //The two capacity5 allocations contain actual NULL references. Capacity
  //and both counts project from these owners; no mutable scalar mirror.
  camera.base.first=Some(Array::constructed()?);
  camera.base.second=Some(Array::constructed()?);
  camera.put(0x198,0);
  for index in 0..6{let offset=0x19c+16*index;vector(camera,offset,globals)?;camera.put(offset+12,0)}
  camera.put_byte(0xc4,0);camera.put_byte(0xc5,0);
  for(offset,value)in[(0x148,0x3f800000),(0x14c,0x40000000),(0x140,0x3f000000),(0x144,0xbf000000),(0x138,0xbf000000),(0x13c,0x3f000000)]{camera.put(offset,value)}
  camera.put_byte(0x150,0);
  for(offset,value)in[(0x15c,0x3f800000),(0x160,0),(0x154,0),(0x158,0x3f800000)]{camera.put(offset,value)}
  super::ni::update_8742d0(camera,&mut super::ni::RequiredChildren,pc)?;
  let x=camera.word(0x90)?;camera.put(0x28,x);
  let y=camera.word(0x94)?;camera.put(0x2c,y);
  let z=camera.word(0x98)?;camera.put(0x200,0x3f800000);camera.put(0x30,z);camera.put(0x1fc,0x3f);
  Ok(())
 })();
 camera.base.phase=if result.is_ok(){Phase::Complete}else{Phase::Failed};result
}
#[cfg(test)]#[path="native_camera_base_tests.rs"]mod tests;
