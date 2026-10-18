//!6FF130→6FEDB0 same-camera target movement. Native scalar stores are retained;
//!interior shape links are typed aliases, never invented native addresses.
use super::{State,Result,b,f,mul,sub};
use aa_numeric::{Binary,Precision};
use aa_spatial::{collision::Shape,geometry::Placement};
use std::cmp::Ordering;

pub(super) struct Links{rectangle_270:bool,point_2a0:bool}
impl Links{
    pub(super) fn unbound()->Self{Self{rectangle_270:false,point_2a0:false}}
}
impl State{
    ///Only the actual constructor's interior-pointer producer may bind this
    ///alias. The scalar shape/descriptor words still need their real writers.
    pub(crate) fn bind_target_rectangle_270_after_native_writer(&mut self){self.target_links.rectangle_270=true;}
    ///7016A0 stores2A0 as the interior alias of its actual point shape288.
    pub(crate) fn bind_target_point_2a0_after_native_writer(&mut self){self.target_links.point_2a0=true;}
    pub fn target_rectangle_alias_present(&self)->bool{self.target_links.rectangle_270}
    pub fn target_point_alias_present(&self)->bool{self.target_links.point_2a0}
}
pub trait Services{
    fn map_dimensions_74_78(&mut self)->Result<Option<[i32;2]>>;
    fn fallback_dimensions_uvar_a8_ac(&mut self)->Result<[i32;2]>;
    fn margin_uvar_24(&mut self)->Result<i32>;
    fn world_editor_275(&mut self)->Result<bool>;
    fn session_state_e4(&mut self)->Result<u32>;
    fn map_height_550210(&mut self,xy:[u32;2])->Result<Binary>;
    fn update_camera_6fe050(&mut self,camera:&mut State,pc:Precision)->Result<()>;
}
fn add(a:&Binary,c:&Binary,p:Precision)->Result<Binary>{a.add(c,p).map_err(|e|e.to_string())}
fn n<T>(r:std::result::Result<T,aa_numeric::Error>)->Result<T>{r.map_err(|e|e.to_string())}
fn shape(camera:&State,offset:u32)->Result<Shape>{
    Ok(Shape{kind:camera.word(offset)?,radius_bits:camera.word(offset+4)?,bounds:[camera.word(offset+8)?,camera.word(offset+12)?,camera.word(offset+16)?,camera.word(offset+20)?]})
}
fn placement(camera:&State,descriptor:u32,shape_offset:u32)->Result<Placement>{
    let linked=match(descriptor,shape_offset){(0x270,0x258)=>camera.target_links.rectangle_270,(0x2a0,0x288)=>camera.target_links.point_2a0,_=>false};
    if !linked{return Err(format!("native Camera+{descriptor:X} has no reached typed shape-alias writer"))}
    Ok(Placement{shape:shape(camera,shape_offset)?,facing:[camera.word(descriptor+4)?,camera.word(descriptor+8)?],position:[camera.word(descriptor+12)?,camera.word(descriptor+16)?]})
}
fn track_ground(camera:&State,world:&mut impl Services)->Result<bool>{
    if camera.byte(0x33c)?==0{return Ok(false)}
    if camera.word(0x2c0)?==0{return Ok(true)}
    if world.world_editor_275()?{return Ok(false)}
    Ok(world.session_state_e4()?!=3)
}
///Native6FEDB0. The finite-domain geometry helpers use the same current
///six-word shape fields; no rendered bounds or replacement clamp is used.
pub fn move_position_6fedb0(camera:&mut State,world:&mut impl Services,position:[u32;3],pc:Precision)->Result<()>{
    if camera.byte(0x214)?!=0{return Ok(())}
    let dx=sub(&b(position[0])?,&b(camera.word(0x240)?)?,pc)?;
    let dy=sub(&b(position[1])?,&b(camera.word(0x244)?)?,pc)?;
    let dz=f(sub(&b(position[2])?,&b(camera.word(0x248)?)?,pc)?)?;
    camera.put(0x240,f(add(&dx,&b(camera.word(0x240)?)?,pc)?)?);
    camera.put(0x244,f(add(&dy,&b(camera.word(0x244)?)?,pc)?)?);
    camera.put(0x248,f(add(&b(dz)?,&b(camera.word(0x248)?)?,pc)?)?);
    camera.put(0x24c,f(add(&dx,&b(camera.word(0x24c)?)?,pc)?)?);
    camera.put(0x250,f(add(&dy,&b(camera.word(0x250)?)?,pc)?)?);
    camera.put(0x254,f(add(&b(dz)?,&b(camera.word(0x254)?)?,pc)?)?);
    if camera.byte(0x33c)?!=0{
        let dimensions=match world.map_dimensions_74_78()?{Some(v)=>v,None=>world.fallback_dimensions_uvar_a8_ac()?};
        let width=n(Binary::from_i32(dimensions[0]).store_f32())?;
        let height=n(Binary::from_i32(dimensions[1]).store_f32())?;
        let margin=Binary::from_i32(world.margin_uvar_24()?);
        let width_minus=f(sub(&b(width)?,&margin,pc)?)?;
        let kind=camera.word(0x258)?;
        let extent=camera.word(if matches!(kind,2|3){0x268}else{0x25c})?;
        if b(extent)?.compare(&mul(&b(width_minus)?,&b(0x3f000000)?,pc)?)!=Ordering::Equal{
            let height_minus=f(sub(&b(height)?,&margin,pc)?)?;
            let rect=aa_navigation::uai_collision::swept_shape(width_minus,height_minus,pc).map_err(|e|e.to_string())?;
            for(i,word)in rect.words().into_iter().enumerate(){camera.put(0x258+4*i as u32,word)}
            camera.target_links.rectangle_270=true;
            camera.put(0x274,0);camera.put(0x278,0xbf800000);camera.put(0x284,0);
            camera.put(0x27c,f(mul(&b(width)?,&b(0x3f000000)?,pc)?)?);
            camera.put(0x280,f(mul(&b(height)?,&b(0x3f000000)?,pc)?)?);
        }
        camera.target_links.point_2a0=true;
        camera.put(0x2a4,0);camera.put(0x2a8,0xbf800000);camera.put(0x2b4,0);
        camera.put(0x2ac,camera.word(0x24c)?);camera.put(0x2b0,camera.word(0x250)?);
        let before=[camera.word(0x24c)?,camera.word(0x250)?];
        let rectangle=placement(camera,0x270,0x258)?;let point=placement(camera,0x2a0,0x288)?;
        if !aa_spatial::shape_overlap::overlaps(&rectangle,&point,pc).map_err(|e|e.to_string())?{
            //46E4F0/46E150 are reused. The point/default-static path requires
            //its actual process owner rather than supplying assumed zeros.
            if !matches!(rectangle.shape.kind,1..=3){return Err("Camera target projection needs actual unsupported shape/default-point owner".into())}
            //For these supported kinds the helper never reads its fallback
            //argument. Passing the captured point represents no static owner.
            let projected=aa_combat::surface::placed_projection(&rectangle,before,before,pc).map_err(|e|e.to_string())?;
            let dx=f(sub(&b(projected[0])?,&b(camera.word(0x24c)?)?,pc)?)?;
            let dy=f(sub(&b(projected[1])?,&b(camera.word(0x250)?)?,pc)?)?;
            camera.put(0x24c,projected[0]);camera.put(0x250,projected[1]);
            camera.put(0x240,f(add(&b(dx)?,&b(camera.word(0x240)?)?,pc)?)?);
            camera.put(0x244,f(add(&b(dy)?,&b(camera.word(0x244)?)?,pc)?)?);
        }
        let old_z=camera.word(0x254)?;
        if track_ground(camera,world)?{
            let xy=[camera.word(0x24c)?,camera.word(0x250)?];let z=f(world.map_height_550210(xy)?)?;camera.put(0x254,z);
        }
        //The height delta is NOT spilled before adding camera248.
        let dz=sub(&b(camera.word(0x254)?)?,&b(old_z)?,pc)?;
        camera.put(0x248,f(add(&dz,&b(camera.word(0x248)?)?,pc)?)?);
    }
    let direction=[camera.word(0x114)?,camera.word(0x118)?,camera.word(0x11c)?];let zoom=camera.word(0x208)?;
    super::pose::zoom_6fe470(camera,zoom,direction,pc)?;
    world.update_camera_6fe050(camera,pc)
}
///6FF130 computes its local XYZ argument BEFORE6FEDB0's byte214 early return.
pub fn set_look_at_6ff130(camera:&mut State,world:&mut impl Services,target:[u32;2],pc:Precision)->Result<()>{
    let z=b(camera.word(0x254)?)?;
    let dx=sub(&b(target[0])?,&b(camera.word(0x24c)?)?,pc)?;
    let dy=f(sub(&b(target[1])?,&b(camera.word(0x250)?)?,pc)?)?;
    let dz=f(sub(&z,&b(camera.word(0x254)?)?,pc)?)?;
    let x=f(add(&dx,&b(camera.word(0x240)?)?,pc)?)?;
    let y=f(add(&b(dy)?,&b(camera.word(0x244)?)?,pc)?)?;
    let z=f(add(&b(dz)?,&b(camera.word(0x248)?)?,pc)?)?;
    move_position_6fedb0(camera,world,[x,y,z],pc)
}
#[cfg(test)]#[path="native_camera_target_tests.rs"]mod tests;
