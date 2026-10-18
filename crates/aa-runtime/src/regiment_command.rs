//!Code-registered Regiment state priorities and native51D650 sleep predicate.
//!These immutable CRT values are separate from mutable Arena status/stack.
use aa_mind::{Error,Result};
///Registered metadata+4, not status or an authored state-name heuristic.
///Each value is backed by real metadata initializer execution in both PEs.
pub fn priority(offset:u32)->Result<i32>{
    match offset{
        0x34|0x54=>Ok(0),
        0x68|0xc0|0xd4|0xf8|0x120|0x134|0x160|0x17c|0x1b8|0x1f0|0x220|0x24c|0x280|0x2a4=>Ok(2),
        0x1a8=>Ok(3),0x1cc=>Ok(4),0x20c=>Ok(6),0x270=>Ok(7),
        _=>Err(Error::World("actual Regiment current state metadata binding absent".into())),
    }
}
///51D650's explicit current Retreat exception precedes529F80's signed
///comparison. Native null head/null node use priority0 without a metadata read.
pub fn can_activate_sleep_51d650(current:Option<u32>)->Result<bool>{
    if current==Some(0x1f0){return Ok(true)}
    Ok(2>=current.map(priority).transpose()?.unwrap_or(0))
}
///Actual51D730 reads command flags/type before5247C0. The current priority
///and Organization+8C are deliberately lazy reads of their sole live owners.
pub trait MovePermissionFields {
    fn current_is_retreat_command(&mut self)->Result<bool>;
    fn engage_status_is_two(&mut self)->Result<bool>;
    fn organization_override(&mut self)->Result<u8>;
    fn current_priority(&mut self)->Result<i32>;
    fn engage_is_current(&mut self)->Result<bool>;
}
pub fn can_activate_move_51d730(candidate_priority:i32,fields:&mut impl MovePermissionFields)->Result<bool>{
    let retreat=fields.current_is_retreat_command()?;
    let allowed=if fields.engage_status_is_two()?&&fields.organization_override()?==0 {
        candidate_priority>=2
    }else{candidate_priority>=fields.current_priority()?};
    if allowed{return Ok(true)}
    Ok(retreat&&fields.engage_is_current()?)
}
///5202DD..52030E after the actual current ZOC guard has passed. Native NULL
///current command uses8; a nonnull command's zero type returns0 directly.
pub fn move_actor_state_after_zoc(command_type:Option<u32>)->u8{
    match command_type{Some(0)=>0,Some(7)=>13,_=>8}
}
///520259..52026D reads the actual slot4 pointer only after signed length>4.
///The caller's typed collection has a nonnegative native representable length.
pub fn move_zoc_requires_predicates(zone_count:usize,slot4_present:impl FnOnce()->bool)->bool{
    zone_count>4&&slot4_present()
}
///52FD00 AFTER the same registry's full generation/pointer lookup. Flags
///read precedes the same-actor test; removed/bit29 targets short-circuit it.
pub fn move_has_live_target(flags:Option<u32>,same_actor:impl FnOnce()->bool)->bool{
    flags.is_some_and(|flags|flags&(0x20000000|1)==0&&!same_actor())
}
///527E30's constant argument defaults are backed by both native PE mappings.
///Only optional auxiliary/speed FLD/FSTP fields quiet NaNs; position uses MOV.
pub fn move_arguments(flags:u32,auxiliary:[u32;2],speed:u32)->([u32;2],u32){
    let auxiliary=if flags&8!=0{auxiliary.map(aa_mind::cai_thinks::x87_copy)}else{[0,0]};
    let speed=aa_mind::cai_thinks::x87_copy(if flags&16!=0{speed}else{0});
    (auxiliary,speed)
}
