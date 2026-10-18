#![allow(dead_code)]
//! Ordered KKC_Camera Reset700E30 and projection6FE920/6FDF80 fields.
//! This retained partial owner is NOT a completed7016A0/NiCamera constructor.
//! Unwritten fields remain unbound; positive base/pose/render children are
//! required. In particular +204 has no constructor-zero default:702380 writes
//! the actual camera-directory ordinal after7016A0 returns.
use aa_numeric::{Binary,Precision};
use std::cmp::Ordering;
type Result<T> = std::result::Result<T,String>;
#[path="../../../crates/aa-runtime/src/native_camera_pose.rs"]pub mod pose;
#[path="../../../crates/aa-runtime/src/native_camera_ni.rs"]pub mod ni;
#[path="../../../crates/aa-runtime/src/native_camera_target.rs"]pub mod target;

///One move-only field allocation; no Clone/Default/Deserialize/raw-word ctor.
#[derive(Clone,Copy)]struct Word{bits:u32,known:u32}
pub struct State {words:[Word;0x394/4],target_links:target::Links,revision:std::sync::Arc<std::sync::atomic::AtomicU64>}
impl State {
 fn unbound_allocation()->Self{Self{words:[Word{bits:0,known:0};0x394/4],target_links:target::Links::unbound(),revision:std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0))}}
 ///Only the directly stored fields needed before7016A0 calls700E30. This is
 ///a prefix projection, not permission to omit874B40/46EB30/46E060 children.
 pub fn before_constructor_reset_7016a0()->Self{
  let mut this=Self::unbound_allocation();
  for(offset,value)in[(0x21c,0xbf800000),(0x220,0x3f800000),(0x224,0x3f800000),(0x228,0xbf800000),(0x2d0,0),(0x304,0x3f800000),(0x308,0),(0x30c,0),(0x310,0),(0x314,0),(0x318,0),(0x320,0x3f800000),(0x324,0),(0x328,0),(0x32c,0),(0x330,0),(0x334,0)]{this.put(offset,value)}for offset in[0x31c,0x338,0x33c]{this.put_byte(offset,0)}this
 }
 pub fn word(&self,offset:u32)->Result<u32>{self.words.get((offset/4)as usize).filter(|word|offset%4==0&&word.known==u32::MAX).map(|word|word.bits).ok_or_else(||format!("native Camera+{offset:X} has no reached bound writer"))}
 fn put(&mut self,offset:u32,value:u32){self.revision.fetch_add(1,std::sync::atomic::Ordering::Release);self.words[(offset/4)as usize]=Word{bits:value,known:u32::MAX}}
 fn byte(&self,offset:u32)->Result<u8>{let word=self.words.get((offset/4)as usize).ok_or("camera byte offset outside owner")?;let shift=8*(offset&3);if word.known&(0xff<<shift)!=(0xff<<shift){return Err(format!("native Camera byte+{offset:X} has no reached writer"))}Ok((word.bits>>shift)as u8)}
 fn put_byte(&mut self,offset:u32,value:u8){self.revision.fetch_add(1,std::sync::atomic::Ordering::Release);let word=&mut self.words[(offset/4)as usize];let shift=8*(offset&3);word.bits=(word.bits&!(0xff<<shift))|(u32::from(value)<<shift);word.known|=0xff<<shift}
 ///702380's post-constructor directory publication is a distinct operation.
 ///The caller must supply the actual append result, never an assumed zero.
 pub(crate) fn directory_index_702380(&mut self,index:i32){self.put(0x204,index as u32)}
 ///Only the positive701847 store, after actual712660 construction.
 pub(crate) fn ground_tracking_701847(&mut self){self.put_byte(0x33c,1)}
}

///Every dependency is read at its reached native point. A concrete adapter
///must borrow actual retained Session/AVars/Map/VWorld/seed owners.
pub trait ResetServices{
 fn metagame_40cee0(&mut self)->Result<bool>;
 fn developer_camera_6fe9c0(&mut self,camera:&State)->Result<bool>;
 fn avar(&mut self,fast_offset:u32)->Result<u32>;
 fn map_dimensions_74_78(&mut self)->Result<Option<[i32;2]>>;
 fn fallback_dimensions_uvar_a8_ac(&mut self)->Result<[i32;2]>;
 fn visual_height_43d050(&mut self,xy:[u32;2],flags:u32)->Result<Binary>;
 fn seed_word(&mut self,address:u32)->Result<u32>;
 ///873D90's actual retained NiCamera plane-object list. Empty is legal only
 ///when the real874B40 count174=0 producer has been bound.
 fn dirty_planes_873d90(&mut self,camera:&mut State)->Result<()>;
 fn set_pose_6ff220(&mut self,camera:&mut State,center:[u32;3],yaw:u32,pitch:u32,zoom:u32,pc:Precision)->Result<()>;
}
fn b(word:u32)->Result<Binary>{Binary::from_f32_bits(word).map_err(|e|e.to_string())}
fn f(value:Binary)->Result<u32>{value.store_f32().map_err(|e|e.to_string())}
fn mul(a:&Binary,b:&Binary,pc:Precision)->Result<Binary>{a.mul(b,pc).map_err(|e|e.to_string())}
fn div(a:&Binary,b:&Binary,pc:Precision)->Result<Binary>{a.div(b,pc).map_err(|e|e.to_string())}
fn sub(a:&Binary,b:&Binary,pc:Precision)->Result<Binary>{a.sub(b,pc).map_err(|e|e.to_string())}

///700E30. Successful return additionally requires real6FF220 pose/update;
///providing a callback success is not itself a nativeCamera/view capability.
pub fn reset_700e30(camera:&mut State,world:&mut impl ResetServices,pc:Precision)->Result<()>{
 camera.put(0x2c0,0);camera.put_byte(0x2c4,0);camera.put(0x230,0x3f400000);
 let zoom_offset=if world.metagame_40cee0()?{0x38}else if world.developer_camera_6fe9c0(camera)?{0x2c}else{0x44};
 let zoom=world.avar(zoom_offset)?;camera.put(0x208,zoom);camera.put(0x22c,zoom);
 let (minimum,maximum)=if world.metagame_40cee0()?{(0x30,0x34)}else if world.developer_camera_6fe9c0(camera)?{(0x24,0x28)}else{(0x3c,0x40)};
 let minimum=world.avar(minimum)?;camera.put(0x20c,minimum);let maximum=world.avar(maximum)?;camera.put(0x210,maximum);
 let current=b(camera.word(0x208)?)?;if b(minimum)?.compare(&current)==Ordering::Greater{camera.put(0x208,minimum)}else if current.compare(&b(maximum)?)==Ordering::Greater{camera.put(0x208,maximum)}
 camera.put_byte(0x214,0);let fov=world.avar(0x5c)?;camera.put(0x218,fov);
 update_projection_6fe920(camera,world,pc)?;
 camera.put(0x2bc,0x7f7fffff);camera.put(0x2b8,0x7f7fffff);
 let dimensions=world.map_dimensions_74_78()?;let size=match dimensions{Some(size)=>size,None=>world.fallback_dimensions_uvar_a8_ac()?};
 let half=b(0x3f000000)?;let x=f(mul(&Binary::from_i32(size[0]),&half,pc)?)?;let y=f(mul(&Binary::from_i32(size[1]),&half,pc)?)?;let mut z=0;
 if camera.byte(0x33c)?!=0&&dimensions.is_some(){z=f(world.visual_height_43d050([x,y],0x222)?)?}
 let pitch=f(mul(&b(world.avar(0x50)?)?,&b(0x3c8efa35)?,pc)?)?;
 let seed_x=world.seed_word(0xb43cf8)?;let height=b(world.avar(0x58)?)?;
 camera.put(0x24c,seed_x);let seed_y=world.seed_word(0xb43cfc)?;
 let sine=aa_trig::turning::small_sin_cos(pitch,pc).map_err(|e|e.to_string())?.0;
 camera.put(0x250,seed_y);camera.put(0x254,world.seed_word(0xb43d00)?);
 camera.put(0x240,f(div(&height,&sine,pc)?)?);camera.put(0x244,0);camera.put(0x248,0);
 let yaw_offset=if world.metagame_40cee0()?{0x4c}else{0x48};let yaw=f(mul(&b(world.avar(yaw_offset)?)?,&b(0x3c8efa35)?,pc)?)?;
 let zoom=camera.word(0x208)?;world.set_pose_6ff220(camera,[x,y,z],yaw,pitch,zoom,pc)
}

///6FE920→88A5B0→6FDF80→873D90. Near/far are read before the native
///extended FPTAN. Frustum order at138 is left,right,top,bottom,near,far.
pub fn update_projection_6fe920(camera:&mut State,world:&mut impl ResetServices,pc:Precision)->Result<()>{
 let half=mul(&b(camera.word(0x218)?)?,&b(0x3f000000)?,pc)?;
 let far=world.avar(0x20)?;let near=world.avar(0x1c)?;
 let angle=mul(&sub(&b(0x42b40000)?,&half,pc)?,&b(0x3c8efa35)?,pc)?;
 let tangent=aa_trig::turning::small_tangent_extended(&angle,pc).map_err(|e|e.to_string())?;let inverse=div(&b(0x3f800000)?,&tangent,pc)?;
 let bottom=f(mul(&mul(&inverse,&b(camera.word(0x228)?)?,pc)?,&b(camera.word(0x230)?)?,pc)?)?;
 let top=f(mul(&mul(&inverse,&b(camera.word(0x224)?)?,pc)?,&b(camera.word(0x230)?)?,pc)?)?;
 let right=f(mul(&inverse,&b(camera.word(0x220)?)?,pc)?)?;let left=f(mul(&inverse,&b(camera.word(0x21c)?)?,pc)?)?;
 //6FDF80 stores far/10000 to a binary32 temporary BEFORE comparison.
 let floor=f(div(&b(far)?,&b(0x461c4000)?,pc)?)?;let mut near=near;
 if b(near)?.compare(&b(floor)?)==Ordering::Less{near=floor}if b(near)?.compare(&b(0x3dcccccd)?)==Ordering::Less{near=0x3dcccccd}
 camera.put(0x138,left);camera.put(0x13c,right);camera.put(0x140,top);camera.put(0x148,near);camera.put(0x144,bottom);camera.put(0x14c,far);camera.put_byte(0x150,0);
 world.dirty_planes_873d90(camera)
}

#[cfg(test)]#[path="../../../crates/aa-runtime/src/native_camera_tests.rs"]mod tests;
