//!One activation/destination coordinator for lab fields and live split owners.
//!Live adapters write actual Installed words/Arena directly; no temporary
//!CompanyMoveState or second mutable status/path owner is required.
use super::{CompanyMoveState,CompanyWorld,Extents,clamp_to_extent,b,store};
use crate::{Error,Result};
use aa_numeric::Precision;

pub trait CompanyActivationFields{
    fn can_activate(&mut self,mode:u32)->Result<bool>;
    fn store_target(&mut self,id:u32)->Result<()>;
    ///Only after successful eligibility and target-ID store (520320).
    fn target_position(&mut self,id:u32)->Result<[u32;2]>;
    ///Store both raw words, then clamp in axis order through5525D0.
    fn requested_destination(&mut self,point:[u32;2],extents:Extents,p:Precision)->Result<()>;
    fn group_offset(&mut self)->Result<Option<[u32;2]>>;
    fn path_destination(&mut self,point:[u32;2],extents:Extents,p:Precision)->Result<()>;
    ///Native store order: +2C, +24, +28.
    fn speed_auxiliary(&mut self,speed:u32,auxiliary:[u32;2])->Result<()>;
    ///Reached native debug gate/diagnostic before location stores.
    fn location_diagnostic(&mut self,point:[u32;2])->Result<()>;
    ///Reached native debug gate/diagnostic after target fields are stored.
    fn target_diagnostic(&mut self,target:u32)->Result<()>;
    fn activate(&mut self,mode:u32)->Result<bool>;
}
pub fn set_destination_on_fields(fields:&mut impl CompanyActivationFields,
    point:[u32;2],extents:Extents,p:Precision)->Result<()>{
    fields.requested_destination(point,extents,p)?;
    let goal=if let Some(offset)=fields.group_offset()?{
        [store(&b(point[0])?.add(&b(offset[0])?,p)?)?,store(&b(point[1])?.add(&b(offset[1])?,p)?)?]
    }else{point};
    fields.path_destination(goal,extents,p)
}
#[allow(clippy::too_many_arguments)]
pub fn activate_to_actor_on_fields(fields:&mut impl CompanyActivationFields,
    mode:u32,target:Option<u32>,speed:u32,auxiliary:[u32;2],extents:Extents,p:Precision)->Result<bool>{
    let Some(target)=target else{return Ok(false)};
    if !fields.can_activate(mode)?{return Ok(false)}
    fields.store_target(target)?;
    let position=fields.target_position(target)?;
    set_destination_on_fields(fields,position,extents,p)?;
    fields.speed_auxiliary(speed,auxiliary)?;
    fields.target_diagnostic(target)?;
    fields.activate(mode)
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct LocationActivation{
    ///Historical lab API reports whether activation was requested.
    pub requested:bool,
    ///Actual AL from52A810, consumed by527E30 (not request admission).
    pub result:bool,
}
#[allow(clippy::too_many_arguments)]
pub fn activate_to_location_on_fields(fields:&mut impl CompanyActivationFields,
    mode:u32,point:[u32;2],speed:u32,auxiliary:[u32;2],extents:Extents,p:Precision)->Result<LocationActivation>{
    if !fields.can_activate(mode)?{return Ok(LocationActivation{requested:false,result:false})}
    fields.location_diagnostic(point)?;
    fields.store_target(0)?;
    set_destination_on_fields(fields,point,extents,p)?;
    fields.speed_auxiliary(speed,auxiliary)?;
    Ok(LocationActivation{requested:true,result:fields.activate(mode)?})
}

///Compatibility wrapper around the same original laboratory state/callbacks.
///These existing APIs omitted B3D43C debug diagnostics; they retain that exact
///scope. Actual live callers implement the required diagnostic methods above.
pub(super) struct Legacy<'a,W>{pub state:&'a mut CompanyMoveState,pub world:&'a mut W,pub target:Option<(u32,[u32;2])>}
impl<W:CompanyWorld> CompanyActivationFields for Legacy<'_,W>{
    fn can_activate(&mut self,mode:u32)->Result<bool>{self.world.can_activate(mode,self.state)}
    fn store_target(&mut self,id:u32)->Result<()>{self.state.target_id=id;Ok(())}
    fn target_position(&mut self,id:u32)->Result<[u32;2]>{self.target.filter(|t|t.0==id).map(|t|t.1).ok_or_else(||Error::World("lab supplied positive target position absent".into()))}
    fn requested_destination(&mut self,point:[u32;2],e:Extents,p:Precision)->Result<()>{self.state.destination_bits=point;clamp_to_extent(e,&mut self.state.destination_bits,p)}
    fn group_offset(&mut self)->Result<Option<[u32;2]>>{self.world.group_offset(self.state)}
    fn path_destination(&mut self,point:[u32;2],e:Extents,p:Precision)->Result<()>{self.state.path_destination_bits=point;clamp_to_extent(e,&mut self.state.path_destination_bits,p)}
    fn speed_auxiliary(&mut self,speed:u32,auxiliary:[u32;2])->Result<()>{self.state.speed_bits=speed;self.state.auxiliary_bits=auxiliary;Ok(())}
    fn location_diagnostic(&mut self,_:[u32;2])->Result<()>{Ok(())}
    fn target_diagnostic(&mut self,_:u32)->Result<()>{Ok(())}
    fn activate(&mut self,mode:u32)->Result<bool>{self.world.activate(mode,self.state)}
}
