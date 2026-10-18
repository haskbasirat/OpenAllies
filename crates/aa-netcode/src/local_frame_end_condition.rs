//!76F600's lazy native Session16D guard. A true flag requires the real
//!76BA90 HumanOpposition context before even reading/storing Session16E.
use super::*;
pub fn run(players:&mut PlayersBorrow<'_>)->Result<()>{
    let enabled=players.session.flags_168_16e[5]
        .ok_or_else(||missing("76F600 actual Session16D is unbound"))?;
    if !enabled{return Ok(())}
    Err(missing("76F600 requires actual76BA90 human-opposition World/player context"))
}
#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::Value;
    #[test]
    fn native_guard_does_not_read_or_write_later_session_word(){
        let f:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-frame-end-condition.json")).unwrap();
        assert_eq!(f["rows"].as_array().unwrap().len(),36);
        for r in f["rows"].as_array().unwrap(){
            let mut owner=LocalBootstrap::from_native_constructors();owner.session.flags_168_16e[5]=Some(r["enabled"].as_u64().unwrap()!=0);
            //Native fixture supplies this later word, but the concrete
            //adapter deliberately leaves it unbound: it must stay unread.
            owner.session.flags_168_16e[6]=None;
            let mut p=PlayersBorrow{session:&mut owner.session,local_player:&owner.local_player,identity:&owner.identity};
            assert_eq!(run(&mut p).is_err(),r["stopped"].as_bool().unwrap());
            assert_eq!(owner.session.flags_168_16e[6],None);
        }
    }
}
