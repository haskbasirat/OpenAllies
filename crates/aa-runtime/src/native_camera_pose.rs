//!6FF220 position,6FEA20 constraints,6FE3B0 rotations and6FE470 zoom.
//!NiCamera virtual update6FE050 remains a required same-owner child.
use super::{State,Result,b,f,mul,div,sub};
use aa_numeric::{Binary,Precision};
use std::cmp::Ordering;

///Retained process B40054 matrix from actual static constructor9E3520. No
///caller-selected matrix, Clone, per-reset default or inspection camera.
pub struct Globals{matrix_b40054:[u32;9]}
impl Globals{
 pub fn from_static_constructor_9e3520()->Self{Self{matrix_b40054:[0,0,0x3f800000,0x3f800000,0,0,0,0x3f800000,0]}}
 pub fn matrix_b40054(&self)->&[u32;9]{&self.matrix_b40054}
}
pub trait Services{
 fn metagame_40cee0(&mut self)->Result<bool>;
 fn developer_camera_6fe9c0(&mut self,camera:&State)->Result<bool>;
 fn avar(&mut self,offset:u32)->Result<u32>;
 fn free_yaw_uvar_b4(&mut self)->Result<bool>;
 fn world_editor_275(&mut self)->Result<bool>;
 fn session_state_e4(&mut self)->Result<u32>;
 fn map_height_550210(&mut self,xy:[u32;2])->Result<Binary>;
 fn update_camera_6fe050(&mut self,camera:&mut State,pc:Precision)->Result<()>;
}
fn add(a:&Binary,b:&Binary,p:Precision)->Result<Binary>{a.add(b,p).map_err(|e|e.to_string())}
fn rad(value:u32,p:Precision)->Result<Binary>{mul(&b(value)?,&b(0x3c8efa35)?,p)}
///All incoming pointers are stable local arguments in6FF220; their partial
///changes do not authorize reading or copying another live Character/World.
fn constrain(camera:&State,world:&mut impl Services,yaw:&mut u32,pitch:&mut u32,zoom:&mut u32,p:Precision)->Result<()>{
 let (minimum,maximum,low_pitch,high_pitch)=if world.metagame_40cee0()?&&world.developer_camera_6fe9c0(camera)?{
  let low=rad(world.avar(0x54)?,p)?;let min=world.avar(0x30)?;let max=world.avar(0x34)?;(min,max,low,0x3fc6d3f2)
 }else if world.metagame_40cee0()?&&!world.developer_camera_6fe9c0(camera)?{
  *yaw=f(rad(world.avar(0x4c)?,p)?)?;let low=rad(world.avar(0x50)?,p)?;let high=f(low.clone())?;let min=world.avar(0x30)?;let max=world.avar(0x34)?;(min,max,low,high)
 }else if world.developer_camera_6fe9c0(camera)?{
  let low=rad(world.avar(0x54)?,p)?;let min=world.avar(0x24)?;let max=world.avar(0x28)?;(min,max,low,0x3fc6d3f2)
 }else{
  if !world.free_yaw_uvar_b4()?{*yaw=f(rad(world.avar(0x48)?,p)?)?}
  let low=rad(world.avar(0x50)?,p)?;let high=f(low.clone())?;let min=world.avar(0x3c)?;let max=world.avar(0x40)?;(min,max,low,high)
 };
 if low_pitch.compare(&b(*pitch)?)==Ordering::Greater{*pitch=f(low_pitch)?}else if b(*pitch)?.compare(&b(high_pitch)?)==Ordering::Greater{*pitch=high_pitch}
 if b(*zoom)?.compare(&b(minimum)?)==Ordering::Less{*zoom=minimum}else if b(*zoom)?.compare(&b(maximum)?)==Ordering::Greater{*zoom=maximum}Ok(())
}
fn matrix_product(lhs:&[u32;9],rhs:&[u32;9],p:Precision)->Result<[u32;9]>{
 //870590's exact reduction order differs between rows/columns. Each output
 //is stored32 before either next matrix multiplication reads that temporary.
 const TERMS:[(usize,[(usize,usize);3]);9]=[(0,[(1,3),(0,0),(2,6)]),(3,[(5,6),(4,3),(3,0)]),(6,[(8,6),(6,0),(7,3)]),(1,[(2,7),(0,1),(1,4)]),(4,[(5,7),(4,4),(3,1)]),(7,[(6,1),(8,7),(7,4)]),(2,[(2,8),(0,2),(1,5)]),(5,[(5,8),(4,5),(3,2)]),(8,[(6,2),(8,8),(7,5)])];
 let mut output=[0;9];for(i,t)in TERMS{let mut v=mul(&b(lhs[t[0].0])?,&b(rhs[t[0].1])?,p)?;for (a,c)in&t[1..]{v=add(&v,&mul(&b(lhs[*a])?,&b(rhs[*c])?,p)?,p)?}output[i]=f(v)?}Ok(output)
}
fn rotate(camera:&mut State,globals:&Globals,yaw:u32,pitch:u32,p:Precision)->Result<()>{
 if b(camera.word(0x2bc)?)?.compare(&b(yaw)?)==Ordering::Equal&&b(camera.word(0x2b8)?)?.compare(&b(pitch)?)==Ordering::Equal{return Ok(())}
 if camera.byte(0x214)?!=0{return Ok(())}camera.put(0x2b8,pitch);camera.put(0x2bc,yaw);
 let(s,c)=aa_trig::turning::small_sin_cos(pitch,p).map_err(|e|e.to_string())?;let sin=f(s)?;let cos=f(c)?;let x=[0x3f800000,0,0,0,cos,sin,0,sin^0x80000000,cos];
 let(s,c)=aa_trig::turning::small_sin_cos(yaw,p).map_err(|e|e.to_string())?;let sin=f(s)?;let cos=f(c)?;let z=[cos,sin,0,sin^0x80000000,cos,0,0,0,0x3f800000];
 let temporary=matrix_product(&z,&x,p)?;let final_matrix=matrix_product(&temporary,globals.matrix_b40054(),p)?;
 for(i,word)in final_matrix.into_iter().enumerate(){camera.put(0x38+4*i as u32,word)}Ok(())
}
fn basis(camera:&State,p:Precision)->Result<[u32;3]>{
 //870720 transforms [1,0,0], retaining actual zero multiplications/signed0.
 let mut matrix=[0;9];for(i,v)in matrix.iter_mut().enumerate(){*v=camera.word(0x38+4*i as u32)?}
 let vector=[0x3f800000,0,0];let mut result=[0;3];
 for(i,terms)in [[(2,2),(1,1),(0,0)],[(5,2),(3,0),(4,1)],[(8,2),(6,0),(7,1)]].into_iter().enumerate(){let mut value=mul(&b(matrix[terms[0].0])?,&b(vector[terms[0].1])?,p)?;for(a,c)in&terms[1..]{value=add(&value,&mul(&b(matrix[*a])?,&b(vector[*c])?,p)?,p)?}result[i]=f(value)?}Ok(result)
}
pub fn zoom_6fe470(camera:&mut State,zoom:u32,direction:[u32;3],p:Precision)->Result<()>{
 let minimum=camera.word(0x20c)?;let maximum=camera.word(0x210)?;camera.put(0x208,zoom);
 if b(minimum)?.compare(&b(zoom)?)==Ordering::Greater{camera.put(0x208,minimum)}else if b(zoom)?.compare(&b(maximum)?)==Ordering::Greater{camera.put(0x208,maximum)}
 let delta=f(sub(&b(camera.word(0x248)?)?,&b(camera.word(0x254)?)?,p)?)?;
 let distance=if b(delta)?.is_zero(){b(0)?}else{let sine=aa_trig::turning::small_sin_cos(camera.word(0x2b8)?,p).map_err(|e|e.to_string())?.0;div(&b(delta)?,&sine,p)?};
 let scale=sub(&b(0x3f800000)?,&b(camera.word(0x208)?)?,p)?;
 //X product stays extended until its distance product store; Y/Z first
 //round scale products to binary32. These asymmetries are original stores.
 let x_scaled=mul(&b(direction[0])?,&scale,p)?;let y_scaled=f(mul(&b(direction[1])?,&scale,p)?)?;let z_scaled=f(mul(&b(direction[2])?,&scale,p)?)?;
 let x_delta=f(mul(&x_scaled,&distance,p)?)?;let y_delta=f(mul(&b(y_scaled)?,&distance,p)?)?;
 let z_delta=mul(&distance,&b(z_scaled)?,p)?;
 let x=f(add(&b(x_delta)?,&b(camera.word(0x240)?)?,p)?)?;let y=f(add(&b(y_delta)?,&b(camera.word(0x244)?)?,p)?)?;
 camera.put(0x234,x);camera.put(0x238,y);camera.put(0x5c,x);
 let z=f(add(&z_delta,&b(camera.word(0x248)?)?,p)?)?;camera.put(0x23c,z);camera.put(0x60,y);camera.put(0x64,z);Ok(())
}
pub fn set_pose_6ff220(camera:&mut State,globals:&Globals,world:&mut impl Services,mut center:[u32;3],mut yaw:u32,mut pitch:u32,mut zoom:u32,p:Precision)->Result<()>{
 if camera.byte(0x214)?!=0{return Ok(())}constrain(camera,world,&mut yaw,&mut pitch,&mut zoom,p)?;
 if camera.byte(0x33c)?!=0&&(camera.word(0x2c0)?==0||(!world.world_editor_275()?&&world.session_state_e4()?!=3)){center[2]=f(world.map_height_550210([center[0],center[1]])?)?}
 let dz=f(sub(&b(camera.word(0x254)?)?,&b(camera.word(0x248)?)?,p)?)?;let dy=f(sub(&b(camera.word(0x250)?)?,&b(camera.word(0x244)?)?,p)?)?;let dx=f(sub(&b(camera.word(0x24c)?)?,&b(camera.word(0x240)?)?,p)?)?;
 let xy=add(&mul(&b(dx)?,&b(dx)?,p)?,&mul(&b(dy)?,&b(dy)?,p)?,p)?;let squared=f(add(&xy,&mul(&b(dz)?,&b(dz)?,p)?,p)?)?;
 let distance=f(b(squared)?.sqrt(p).map_err(|e|e.to_string())?)?;rotate(camera,globals,yaw,pitch,p)?;
 camera.put(0x24c,center[0]);camera.put(0x250,center[1]);camera.put(0x254,center[2]);let direction=basis(camera,p)?;
 let z_delta=f(mul(&b(direction[2])?,&b(distance)?,p)?)?;let y_delta=f(mul(&b(direction[1])?,&b(distance)?,p)?)?;let x_delta=f(mul(&b(direction[0])?,&b(distance)?,p)?)?;
 let z=f(sub(&b(center[2])?,&b(z_delta)?,p)?)?;let y=f(sub(&b(center[1])?,&b(y_delta)?,p)?)?;let x=f(sub(&b(center[0])?,&b(x_delta)?,p)?)?;
 camera.put(0x240,x);camera.put(0x244,y);camera.put(0x248,z);zoom_6fe470(camera,zoom,direction,p)?;world.update_camera_6fe050(camera,p)
}
