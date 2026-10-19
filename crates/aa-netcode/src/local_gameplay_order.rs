//!Actual accepted86/87 leaves. The same Admin/Session and Frame graph remain
//!owned throughout; World loans end before payload mirror/next reference read.
use super::*;
use aa_runtime::{catalog::Catalog,live_sim::Sim,live_world_frame::Frame,
    load_world_globals::SessionPhase,player_order_frame as binding,player_order_world as orders};
use aa_orders::{payload::Field,Error as OrderError,Result as OrderResult};
fn error(e:impl ToString)->OrderError{OrderError::SchemaSource(e.to_string())}
trait Session{
    fn state(&self)->std::result::Result<u32,String>;
    fn phase(&self)->std::result::Result<u32,String>;
    fn multiplayer(&self)->bool;
    fn local_first_unit(&self)->std::result::Result<Option<u16>,String>;
}
impl Session for PlayersBorrow<'_>{
    fn state(&self)->std::result::Result<u32,String>{Ok(self.session.state)}
    fn phase(&self)->std::result::Result<u32,String>{Ok(self.session.phase)}
    fn multiplayer(&self)->bool{self.session.multiplayer}
    fn local_first_unit(&self)->std::result::Result<Option<u16>,String>{
        let Some(id)=*self.local_player else{return Ok(None)};
        let player=self.session.players.iter().find(|p|p.core.id==id).ok_or("actual Node8 player allocation is absent")?;
        Ok(Some(player.core.name.first().copied().unwrap_or(0)))
    }
}
impl Session for SelectionSessionBorrow<'_>{
    fn state(&self)->std::result::Result<u32,String>{self.application_state()}
    fn phase(&self)->std::result::Result<u32,String>{self.current_phase()}
    fn multiplayer(&self)->bool{self.multiplayer()}
    fn local_first_unit(&self)->std::result::Result<Option<u16>,String>{Ok(self.local_player_name_8()?.map(|s|s.first().copied().unwrap_or(0)))}
}
struct Globals<'a,'backend,S,F:?Sized>{session:&'a S,frontend:&'a RefCell<&'backend mut F>}
impl<S:Session,F:live_frame::Frontend+?Sized> application_gate::Sources for Globals<'_,'_,S,F>{
    fn session_state_e4(&mut self)->std::result::Result<Option<u32>,String>{self.session.state().map(Some)}
    fn multiplayer_f4(&mut self)->std::result::Result<bool,String>{Ok(self.session.multiplayer())}
    fn local_player_first_name_unit(&mut self)->std::result::Result<Option<u16>,String>{self.session.local_first_unit()}
    fn dev_mode_62(&mut self)->std::result::Result<u8,String>{self.frontend.try_borrow_mut().map_err(|e|e.to_string())?.dev_mode_current()}
}
impl<S:Session,F:live_frame::Frontend+?Sized> orders::Globals for Globals<'_,'_,S,F>{
    fn can_process_40eaa0(&mut self)->orders::Result<bool>{application_gate::can_process_40eaa0(self).map_err(orders::Error::Required)}
    fn session_phase_76ac90(&mut self)->orders::Result<u32>{self.session.phase().map_err(orders::Error::Required)}
    fn swap_right_click_with_alt_right_click_4be150(&mut self)->orders::Result<bool>{
        self.frontend.try_borrow_mut().map_err(|e|orders::Error::Required(e.to_string()))?
            .swap_right_click_with_alt_right_click_current().map_err(orders::Error::Required)
    }
    fn hotkey_alt_right_click_command_4b2e60(&mut self)->orders::Result<Vec<u16>>{
        self.frontend.try_borrow_mut().map_err(|e|orders::Error::Required(e.to_string()))?
            .hotkey_alt_right_click_command_current().map_err(orders::Error::Required)
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn process<F:live_frame::Frontend+?Sized>(frame:&Frame<'_,'static>,sim:&RefCell<&mut Sim>,
    catalog:&Catalog<'_>,players:&mut PlayersBorrow<'_>,slot:&mut u32,descriptor:usize,fields:&[Field],frontend:&RefCell<&mut F>,
    sync_a:&RefCell<aa_orders::net::sync::Synchronizer>,gate:&mut dyn FnMut()->std::result::Result<bool,String>,sink:&mut Vec<String>)->OrderResult<()>{
    if aa_runtime::player_selection_frame::supports(descriptor){
        let decoded=frame.with_order_data(|actors,bank,_|aa_runtime::player_selection_frame::decode(catalog,actors,bank,descriptor,fields)).map_err(error)?;
        //Null Add/Remove do not resolve Admin284/player/view as5F4490/C0.
        if !decoded.needs_view(){return Ok(())}
        let sim=sim.try_borrow().map_err(error)?;
        return frame.with_gameplay_order(|parts|{
            sim.require_created_world(parts.revision)?;
            let(mut view,session)=players.ordering_selection_and_session_for_slot(slot).map_err(|e|e.to_string())?;
            aa_runtime::player_selection_frame::execute(catalog,parts,decoded,slot,&mut view,&session)
        }).map_err(error)
    }
    let wire=binding::decoded_in(catalog,descriptor,fields).map_err(error)?;
    let sim=sim.try_borrow().map_err(error)?;
    frame.with_gameplay_order(|parts|{
        sim.require_created_world(parts.revision)?;
        match wire{
            aa_runtime::player_order_decree::Decoded::Actor(wire)=>{
                let mut globals=Globals{session:&*players,frontend};
                binding::actor(catalog,parts,&wire,&mut globals).map(|_|())
            },
            aa_runtime::player_order_decree::Decoded::Selected(wire)=>{
                //765E00 Player+28 of the SAME Admin+284 allocation.
                let kingdom_28=players.session.players.iter().find(|p|p.core.id==*slot).map(|p|p.core.field_28)
                    .ok_or("actual Admin+284 player id is not owned by this Session")?;
                let(mut view,session)=players.ordering_selection_and_session_for_slot(slot).map_err(|e|e.to_string())?;
                let mut globals=Globals{session:&session,frontend};
                let mut sources=order_describe::Describe{name_8:session.ordering_player_name_8(),kingdom_28,frontend,sink};
                let sync=aa_runtime::live_clocked::SyncBinding::Shared{owner:sync_a,gate};
                binding::selected_with(catalog,parts,&wire,&mut globals,slot,&mut view,sync,&mut sources).map(|_|())
            },
        }
    }).map_err(error)
}
