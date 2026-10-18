//!Actual40EAA0 ordered read predicate. A missing native Session falls through
//!to current DevMode; it is not a false/default permission result. These ports
//!do not authorize a native actor, completed view or simulation frame.
pub type Result<T>=std::result::Result<T,String>;
pub trait Sources {
    fn session_state_e4(&mut self)->Result<Option<u32>>;
    fn multiplayer_f4(&mut self)->Result<bool>;
    ///Actual Node+8, then766380(name,0). None is a real null local player;
    ///an unresolved positive identity must be an error at this reached read.
    fn local_player_first_name_unit(&mut self)->Result<Option<u16>>;
    fn dev_mode_62(&mut self)->Result<u8>;
}
pub fn can_process_40eaa0(sources:&mut impl Sources)->Result<bool>{
    if sources.session_state_e4()?.is_some_and(|state|state!=0)
        && sources.multiplayer_f4()? {
        if sources.local_player_first_name_unit()?!=Some(b'%' as u16){return Ok(false)}
    }
    Ok(sources.dev_mode_62()?!=0)
}
#[cfg(test)]
#[path="local_application_gate_tests.rs"]
mod tests;

