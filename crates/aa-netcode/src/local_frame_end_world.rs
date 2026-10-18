//!Positive76F600 over SAME live Session, current PlayerRoster/Kingdom fields,
//!World48 and retained application network owner. No winner is inferred.
use super::*;
use aa_runtime::{actor_world::ActorStore,catalog::Catalog,live_sim::Sim,
    live_world_frame::Frame,mission_fields,world_revision::ConstructedWorldRevision};
use aa_runtime::load_tail_world::SessionRead;

pub trait MessageEffects{
    ///Actual40BC50, kind15 and the formatted current localized text. The
    ///40BC50 return Bool is ignored by76F600; missing service is an error.
    fn show_message(&mut self,text:&[u16])->Result<()>;
    ///793B40 takes the SAME surviving player's current CString+24.
    fn gamespy_remaining(&mut self,player_kingdom_name:&[u16])->Result<()>;
}
pub struct RequiredMessages;
impl MessageEffects for RequiredMessages{
    fn show_message(&mut self,_:&[u16])->Result<()>{Err(missing("76F600 requires actual40BC50 kind15 UI message dispatcher"))}
    fn gamespy_remaining(&mut self,_:&[u16])->Result<()>{Err(missing("76F600 requires actual793B40 GameSpy message owner"))}
}
pub(crate) trait Fields{
    type Player;
    fn enabled(&mut self)->Result<bool>;
    fn opposition(&mut self)->Result<bool>;
    fn notified(&mut self)->Result<bool>;
    fn store_notified(&mut self)->Result<()>;
    fn world48(&mut self)->Result<u32>;
    fn first_active(&mut self)->Result<Option<Self::Player>>;
    fn online(&mut self)->Result<bool>;
    fn localized(&mut self,key:&str)->Result<Vec<u16>>;
    fn format_message(&mut self,text:Vec<u16>)->Result<Vec<u16>>;
    fn show(&mut self,text:&[u16])->Result<()>;
    fn gamespy(&mut self,player:Self::Player)->Result<()>;
}
pub(crate) fn process(f:&mut impl Fields)->Result<()>{
    if !f.enabled()?{return Ok(())}
    after_enabled(f)
}
fn after_enabled(f:&mut impl Fields)->Result<()>{
    if f.opposition()?||f.notified()?{return Ok(())}
    f.store_notified()?; //76F64D, before even reading World48.
    if f.world48()?==2{return Ok(())}
    if let Some(player)=f.first_active()?{
        let key=if f.online()?{"ui_mission_no_human_opponents_remaining_gamespy"}else{"ui_mission_no_human_opponents_remaining"};
        let text=f.localized(key)?;
        let text=f.format_message(text)?;
        f.show(&text)?;
        if f.online()?{f.gamespy(player)?;}
    }
    Ok(())
}
struct Current<'a,'owner,'frame,'source,E>{
    players:&'a mut PlayersBorrow<'owner>,actors:&'a ActorStore,mission:&'a mission_fields::State,
    revision:&'a ConstructedWorldRevision,sim:&'a Sim,frame:&'a Frame<'frame,'static>,
    catalog:&'a Catalog<'source>,effects:&'a mut E,
}
impl<E:MessageEffects> Fields for Current<'_,'_,'_,'_,E>{
    type Player=PlayerHandle;
    fn enabled(&mut self)->Result<bool>{self.players.session.flags_168_16e[5].ok_or_else(||missing("76F600 actual16D unbound"))}
    fn opposition(&mut self)->Result<bool>{
        self.sim.require_created_world(self.revision).map_err(missing)?;
        all_tasks_complete::human_opposition(self.players,self.actors)
    }
    fn notified(&mut self)->Result<bool>{self.players.session.flags_168_16e[6].ok_or_else(||missing("76F600 actual16E unbound"))}
    fn store_notified(&mut self)->Result<()>{self.players.session.flags_168_16e[6]=Some(true);Ok(())}
    fn world48(&mut self)->Result<u32>{self.mission.require_loaded().map_err(missing)?;Ok(self.mission.started48())}
    fn first_active(&mut self)->Result<Option<PlayerHandle>>{
        self.sim.require_created_world(self.revision).map_err(missing)?;
        all_tasks_complete::first_active_human(self.players,self.actors)
    }
    fn online(&mut self)->Result<bool>{self.frame.end_condition_online_active().map_err(missing)}
    fn localized(&mut self,key:&str)->Result<Vec<u16>>{
        self.catalog.with_current_text(|table|table.lookup(&key.encode_utf16().collect::<Vec<_>>())
            .map_err(|e|e.to_string())?.ok_or_else(||format!("76F600 actual4B9F40 missing-key fallback required for {key}"))).map_err(missing)
    }
    fn format_message(&mut self,text:Vec<u16>)->Result<Vec<u16>>{
        //405D60 has no arguments for this format. Plain source text is exact;
        //percent directives require the actual CRT formatting domain.
        if text.contains(&0)||text.contains(&u16::from(b'%')){return Err(missing("76F600 actual405D60 NUL/percent format is required"))}
        Ok(text)
    }
    fn show(&mut self,text:&[u16])->Result<()>{self.effects.show_message(text)}
    fn gamespy(&mut self,player:PlayerHandle)->Result<()>{
        let current=self.players.player_name_24(&player).map_err(missing)?;
        self.effects.gamespy_remaining(current)
    }
}
pub fn run(players:&mut PlayersBorrow<'_>,frame:&Frame<'_,'static>,sim:&Sim,
    catalog:&Catalog<'_>,effects:&mut impl MessageEffects)->Result<()>{
    //False16D must not acquire a World/network loan. Positive callbacks enter
    //only after native WorldServices has released its own update loan.
    if !players.session.flags_168_16e[5].ok_or_else(||missing("76F600 actual16D unbound"))?{return Ok(())}
    frame.with_session_end_condition(|actors,mission,revision|{
        after_enabled(&mut Current{players,actors,mission,revision,sim,frame,catalog,effects}).map_err(|e|e.to_string())
    }).map_err(missing)
}
#[cfg(test)]
#[path="local_frame_end_world_tests.rs"]
mod tests;
