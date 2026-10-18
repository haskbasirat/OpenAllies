//! Shared4CF040/4E52D0/4E10F0 kernels over borrowed live fields.
//! Missing mutable globals are required by the host only on reached null paths.
pub trait Read {
    type Id:Copy+Eq;
    type Error;
    fn kingdom_team(&self,id:Self::Id)->Result<Option<Self::Id>,Self::Error>;
    fn kingdom_slot(&self,id:Self::Id)->Result<u32,Self::Error>;
    fn team_slot(&self,id:Self::Id)->Result<u32,Self::Error>;
    fn kingdom_word(&self,id:Self::Id,team_target:bool,slot:u32)->Result<u32,Self::Error>;
    fn team_word(&self,id:Self::Id,team_target:bool,slot:u32)->Result<u32,Self::Error>;
    fn orphan_relation(&self)->Result<u32,Self::Error>;
    fn team_null_relation(&self)->Result<u32,Self::Error>;
}
pub fn actor<W:Read+?Sized>(world:&W,owner:Option<W::Id>,other:Option<W::Id>)->Result<u32,W::Error> {
    if owner==other {return Ok(0)}
    match owner {Some(id)=>kingdom(world,id,other),None=>world.orphan_relation()}
}
pub fn kingdom<W:Read+?Sized>(world:&W,id:W::Id,other:Option<W::Id>)->Result<u32,W::Error> {
    if Some(id)==other {return Ok(0)}
    let Some(other)=other else{return world.orphan_relation()};
    if let Some(team)=world.kingdom_team(id)? {return team_relation(world,team,Some(other))}
    if let Some(team)=world.kingdom_team(other)? {
        world.kingdom_word(id,true,world.team_slot(team)?)
    }else{world.kingdom_word(id,false,world.kingdom_slot(other)?)}
}
pub fn team_relation<W:Read+?Sized>(world:&W,id:W::Id,other:Option<W::Id>)->Result<u32,W::Error> {
    let Some(other)=other else{return world.team_null_relation()};
    if let Some(team)=world.kingdom_team(other)? {
        world.team_word(id,true,world.team_slot(team)?)
    }else{world.team_word(id,false,world.kingdom_slot(other)?)}
}
