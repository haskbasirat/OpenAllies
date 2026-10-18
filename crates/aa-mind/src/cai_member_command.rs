//!522890: issue a formation Move/Stop to a newly joined member. Every read
//!and callback follows the original order, including the captured Zone4 and
//!the second CURRENT member position after the Mind target callback.
use crate::{MindId,Result};
use aa_numeric::{Binary,Precision};
pub trait World {
    type Organization: Copy;
    type Tile: Copy;
    type Zone: Copy;
    fn organization(&mut self,mind:MindId)->Result<Self::Organization>;
    fn leader_position(&mut self,captured:Self::Organization)->Result<[u32;2]>;
    fn relocate_current_organization(&mut self,mind:MindId,position:[u32;2])->Result<()>;
    fn formation_origin(&mut self,captured:Self::Organization)->Result<[u32;2]>;
    fn member_slot(&mut self,member:u32)->Result<i32>;
    fn current_slot_offset(&mut self,mind:MindId,slot:i32)->Result<[u32;2]>;
    fn tile(&mut self,position:[u32;2])->Result<Option<Self::Tile>>;
    fn current_zone4(&mut self,mind:MindId)->Result<Option<Self::Zone>>;
    fn contains_zone(&mut self,tile:Self::Tile,zone:Self::Zone)->Result<bool>;
    fn member_position(&mut self,member:u32)->Result<[u32;2]>;
    fn member_active_4cede0(&mut self,member:u32)->Result<bool>;
    ///Member.Mind virtual80, including its actual output-buffer write.
    fn member_target_position(&mut self,member:u32)->Result<[u32;2]>;
    fn current_sleep_point(&mut self,mind:MindId)->Result<[u32;2]>;
    ///Actual4D9E20(6), retagA3CC38,4D9B60,4CA2D0.
    fn issue_stop(&mut self,member:u32)->Result<()>;
    ///Actual4CA760(destination,0,sleep),4D9B60,4CA2D0.
    fn issue_move(&mut self,member:u32,destination:[u32;2],sleep:[u32;2])->Result<()>;
}
fn numeric(e:aa_numeric::Error)->crate::Error{crate::Error::World(e.to_string())}
fn value(bits:u32)->Result<Binary>{Binary::from_f32_bits(bits).map_err(numeric)}
///529F10 compares the current x87 squared distance directly with0.25.
pub fn close(a:[u32;2],b:[u32;2],p:Precision)->Result<bool>{
    let dy=value(b[1])?.sub(&value(a[1])?,p).map_err(numeric)?;
    let dy2=dy.mul(&dy,p).map_err(numeric)?;
    let dx=value(b[0])?.sub(&value(a[0])?,p).map_err(numeric)?;
    let dx2=dx.mul(&dx,p).map_err(numeric)?;
    let squared=dx2.add(&dy2,p).map_err(numeric)?;
    Ok(squared.compare(&value(0x3e80_0000)?).is_lt())
}
pub fn run<W:World>(w:&mut W,mind:MindId,member:u32,flag:u8,p:Precision)->Result<()> {
    let captured=w.organization(mind)?;
    let origin=if flag!=0 {
        let position=w.leader_position(captured)?;
        w.relocate_current_organization(mind,position)?;
        position
    }else{w.formation_origin(captured)?};
    let slot=w.member_slot(member)?;
    let offset=w.current_slot_offset(mind,slot)?;
    let mut destination=[0;2];
    for axis in 0..2 {
        destination[axis]=value(offset[axis])?.add(&value(origin[axis])?,p).map_err(numeric)?
            .store_f32().map_err(numeric)?;
    }
    let tile=w.tile(destination)?;
    let zone=w.current_zone4(mind)?;
    let inside=match(tile,zone){(Some(t),Some(z))=>w.contains_zone(t,z)?,_=>false};
    if !inside {
        let position=w.member_position(member)?;
        let tile=w.tile(position)?;
        let inside=match(tile,zone){(Some(t),Some(z))=>w.contains_zone(t,z)?,_=>false};
        if inside {return w.issue_stop(member);}
        let sleep=w.current_sleep_point(mind)?;
        return w.issue_move(member,destination,sleep);
    }
    if w.member_active_4cede0(member)? {
        let target=w.member_target_position(member)?;
        if close(target,destination,p)? {return Ok(());}
    }
    let position=w.member_position(member)?;
    if close(position,destination,p)? {w.issue_stop(member)}
    else {let sleep=w.current_sleep_point(mind)?;w.issue_move(member,destination,sleep)}
}
