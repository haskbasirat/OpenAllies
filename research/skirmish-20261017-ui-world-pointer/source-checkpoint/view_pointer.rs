//!Native6FE590 camera ray arithmetic and43C550 VWorld pointer producers.
//!Picking and71D840 terrain intersection are reached children, never inspector
//!geometry. The camera ray is local; VWorld178/190/1A8 receive optional hit normals.
use aa_numeric::{Binary, Precision};
type Result<T> = std::result::Result<T, String>;
pub trait CameraWords { fn word(&self, offset:u32)->Result<u32>; }
impl CameraWords for crate::native_camera::State {
    fn word(&self,offset:u32)->Result<u32>{self.word(offset)}
}
fn b(v:u32)->Result<Binary>{Binary::from_f32_bits(v).map_err(|e|e.to_string())}
fn f(v:Binary)->Result<u32>{v.store_f32().map_err(|e|e.to_string())}
fn add(a:&Binary,b:&Binary,p:Precision)->Result<Binary>{a.add(b,p).map_err(|e|e.to_string())}
fn sub(a:&Binary,b:&Binary,p:Precision)->Result<Binary>{a.sub(b,p).map_err(|e|e.to_string())}
fn mul(a:&Binary,b:&Binary,p:Precision)->Result<Binary>{a.mul(b,p).map_err(|e|e.to_string())}
fn rounded(v:Binary)->Result<Binary>{b(f(v)?)}
///6FE590's x87 order, including its asymmetric binary32 stack temporaries.
pub fn ray_6fe590(camera:&impl CameraWords, xy:[u32;2],p:Precision)->Result<[u32;3]>{
    let top=b(camera.word(0x140)?)?;
    let y=sub(&top,&mul(&sub(&top,&b(camera.word(0x144)?)?,p)?,&b(xy[1])?,p)?,p)?;
    let yx=mul(&y,&b(camera.word(0x120)?)?,p)?;
    let yy=rounded(mul(&y,&b(camera.word(0x124)?)?,p)?)?;
    let yz=rounded(mul(&y,&b(camera.word(0x128)?)?,p)?)?;
    let left=b(camera.word(0x138)?)?;
    let x=add(&mul(&sub(&b(camera.word(0x13c)?)?,&left,p)?,&b(xy[0])?,p)?,&left,p)?;
    let xx=mul(&x,&b(camera.word(0x12c)?)?,p)?;
    let xy=rounded(mul(&x,&b(camera.word(0x130)?)?,p)?)?;
    let xz=rounded(mul(&x,&b(camera.word(0x134)?)?,p)?)?;
    let dx=add(&xx,&b(camera.word(0x114)?)?,p)?;
    let dy=rounded(add(&xy,&b(camera.word(0x118)?)?,p)?)?;
    let dz=rounded(add(&xz,&b(camera.word(0x11c)?)?,p)?)?;
    Ok([f(add(&dx,&yx,p)?)?,f(add(&dy,&yy,p)?)?,f(add(&dz,&yz,p)?)?])
}
///43C4B0 normalizes Y first, then X; each quotient is stored binary32 before
///6FE590. The signed dimensions are actual current global fields.
pub fn normalized_pixel(point:[i32;2],width:i32,height:i32,p:Precision)->Result<[u32;2]>{
    let y=Binary::from_i32(point[1]).div(&Binary::from_i32(height),p).map_err(|e|e.to_string())?;
    let y=f(y)?;
    let x=Binary::from_i32(point[0]).div(&Binary::from_i32(width),p).map_err(|e|e.to_string())?;
    Ok([f(x)?,y])
}
///The closed43C4B0 arithmetic before/after71D840. The child must return the
///actually written point even when its native return is false. Undefined
///unwritten outputs must remain an error; no zero point is supplied here.
pub fn project_43c4b0(camera:&impl CameraWords,point:[i32;2],width:i32,height:i32,
    normal:&mut[u32;3],pc:Precision,
    intersect:impl FnOnce([u32;3],[u32;3],&mut[u32;3])->Result<[u32;3]>) -> Result<[u32;3]>{
    let ray=ray_6fe590(camera,normalized_pixel(point,width,height,pc)?,pc)?;
    let eye=[camera.word(0x90)?,camera.word(0x94)?,camera.word(0x98)?];
    intersect(eye,ray,normal)
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Event {pub pixel:[i16;2],pub flags:u32,pub code:u16}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Slot {Current,Down,Up}
pub trait Services {
    fn map_present(&mut self)->Result<bool>;
    fn current_input_pixel(&mut self)->Result<[i32;2]>;
    ///Actual43C4B0. Its local camera ray and this optional71D840 normal
    ///output are distinct. Failures retain reached normal writes. Point miss semantics are not
    ///inferred from an unsuccessful intersection return value.
    fn project_43c4b0(&mut self,point:[i32;2],slot:Slot,normal:&mut[u32;3])->Result<[u32;3]>;
}
pub struct State {normals:[[u32;3];3],positions:[[u32;3];3]}
impl State {
    ///442DF4..442E5A direct zero stores, independent of later view readiness.
    pub fn from_vworld_constructor()->Self{Self{normals:[[0;3];3],positions:[[0;3];3]}}
    fn index(slot:Slot)->usize{match slot{Slot::Current=>0,Slot::Down=>1,Slot::Up=>2}}
    pub fn normal(&self,slot:Slot)->[u32;3]{self.normals[Self::index(slot)]}
    pub fn position(&self,slot:Slot)->[u32;3]{self.positions[Self::index(slot)]}
    fn project(&mut self,world:&mut impl Services,point:[i32;2],slot:Slot)->Result<()>{
        let i=Self::index(slot);let position=world.project_43c4b0(point,slot,&mut self.normals[i])?;
        self.positions[i]=position;Ok(())
    }
    pub fn update_43c550(&mut self,event:Option<&Event>,world:&mut impl Services)->Result<()>{
        if !world.map_present()?{return Ok(())}
        let point=match event{Some(e)=>e.pixel.map(i32::from),None=>world.current_input_pixel()?};
        self.project(world,point,Slot::Current)?;
        if let Some(e)=event{
            if e.code==0x108||(e.code==0x100&&e.flags&1!=0){self.project(world,e.pixel.map(i32::from),Slot::Down)?}
            else if e.code==0x101&&e.flags&1!=0{self.project(world,e.pixel.map(i32::from),Slot::Up)?}
        }
        Ok(())
    }
}
#[cfg(test)]
#[path="view_pointer_tests.rs"]
mod tests;
