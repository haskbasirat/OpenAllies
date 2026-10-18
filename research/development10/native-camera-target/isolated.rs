#![allow(dead_code)]
use aa_numeric::{Binary,Precision};
use std::cmp::Ordering;
type Result<T>=std::result::Result<T,String>;
#[path="../../../crates/aa-runtime/src/native_camera_pose.rs"]pub mod pose;
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

fn b(word:u32)->Result<Binary>{Binary::from_f32_bits(word).map_err(|e|e.to_string())}
fn f(value:Binary)->Result<u32>{value.store_f32().map_err(|e|e.to_string())}
fn mul(a:&Binary,b:&Binary,pc:Precision)->Result<Binary>{a.mul(b,pc).map_err(|e|e.to_string())}
fn div(a:&Binary,b:&Binary,pc:Precision)->Result<Binary>{a.div(b,pc).map_err(|e|e.to_string())}
fn sub(a:&Binary,b:&Binary,pc:Precision)->Result<Binary>{a.sub(b,pc).map_err(|e|e.to_string())}

