//!Native request-list metadata and4BE420's linked Definition+70 loop.
type Result<T>=std::result::Result<T,String>;
pub(crate) const PAIR_OFFSETS:[u32;8]=[0x194,0x19c,0x24c,0x254,0x294,0x29c,0x598,0x5a0];
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) enum Group { Mover, Body, Portal, Ability }
impl Group {
    pub(crate) fn names(self)->&'static[&'static str]{match self{
        Self::Mover=>&["WhenStartMoving","WhenStopMoving"],
        Self::Body=>&["WhenStartDie","WhenHealth"],
        Self::Portal=>&["WhenTeleportedFrom","WhenTeleportedTo"],
        Self::Ability=>&["WhenStart","WhenDo","WhenEnd","WhenDoPerTarget","WhenDoPerSummoned"],
    }}
    pub(crate) fn base(self)->u32{match self{Self::Mover=>0x24c,Self::Body=>0x294,Self::Portal=>0x598,Self::Ability=>0x70}}
    pub(crate) fn component(self)->Option<&'static str>{match self{Self::Mover=>Some("MoverComponent"),Self::Body=>Some("BodyComponent"),Self::Portal=>Some("PortalComponent"),Self::Ability=>None}}
}
///4BE420 dereferences the SAME AbilityData+0 once, then five lists from its
///actual linked definition. Each list and each node is read at its callback.
pub(crate) fn request_ability(delta:u32,mode:u32,mut list:impl FnMut(u32,u32,u32)->Result<()>)->Result<()>{
    for i in 0..5 { list(0x70+8*i,delta,mode)?; } Ok(())
}
pub(crate) enum Compile<'a>{Load(&'a str),Synthesize{source:usize,destination:usize},ValidateHealth}
///The actual component list calls and their post-list operations. Each call
///returns before the next operation; a failure preserves that exact prefix.
pub(crate) fn compile_group(group:Group,mut operation:impl FnMut(Compile<'_>)->Result<()>)->Result<()>{
    for name in group.names(){operation(Compile::Load(name))?;}
    match group{
        Group::Mover=>operation(Compile::Synthesize{source:0,destination:1})?,
        Group::Body=>{operation(Compile::Synthesize{source:1,destination:1})?;operation(Compile::ValidateHealth)?;},
        Group::Portal|Group::Ability=>{},
    }Ok(())
}
pub(crate) fn validate_health(trigger:u32)->Result<()>{if (trigger as i32)<0{Err("466532 WhenHealth requires nonnegative health trigger".into())}else{Ok(())}}
#[cfg(test)]
#[path="actor_request_core_tests.rs"]
mod tests;
