//!Concrete7515A0 over actual Node+2C/Session/Sim/World/source owners.
use super::*;
use aa_orders::version_ack::{self,DescriptionWorld};
use aa_runtime::{catalog::Catalog,live_sim::Sim,live_world_frame::Frame};
type OrderResult<T>=aa_orders::Result<T>;
fn error(e:impl std::fmt::Display)->aa_orders::Error{aa_orders::Error::Handler{index:323,position:aa_orders::Position::PACKET_START,message:e.to_string()}}
struct Current<'a,'owner,'frame,'source>{
    players:&'a PlayersBorrow<'owner>,local:&'a LocalClientBorrow<'owner>,
    sim:&'a Sim,frame:&'a Frame<'frame,'static>,catalog:&'a Catalog<'source>,
}
impl Current<'_,'_,'_,'_>{
    fn player(&self)->std::result::Result<&OwnedPlayer,String>{
        let id=self.local.player_2c().ok_or("7515A0 actual client+8 player is null")?;
        let mut found=self.players.session.players.iter().filter(|p|p.core.id==id);
        let player=found.next().ok_or("7515A0 client player allocation absent from same Session")?;
        if found.next().is_some()||player.core.film||player.core.link!=PeerLink::Node{
            return Err("7515A0 local client player allocation is ambiguous/film/foreign".into());
        }
        Ok(player)
    }
    fn kingdom(&self)->std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>,String>{
        let p=self.player()?;
        if !p.core.binding_complete||p.view.is_none(){return Err("7515A0 requires actual765E10 current Player28 binding".into());}
        Ok(p.core.field_28)
    }
}
impl DescriptionWorld for Current<'_,'_,'_,'_>{
    fn has_current_kingdom(&mut self)->OrderResult<bool>{
        if self.sim.world_is_null(){return Ok(false)}
        self.frame.with_version_ack_context(|_,_,_,revision|{
            self.sim.require_created_world(revision)?;Ok(self.kingdom()?.is_some())
        }).map_err(error)
    }
    fn player_name(&mut self)->OrderResult<Vec<u16>>{Ok(self.player().map_err(error)?.core.name.clone())}
    fn kingdom_color_name(&mut self)->OrderResult<Vec<u16>>{
        if self.sim.world_is_null(){return Err(error("7515A0 World vanished before repeated765E00"));}
        self.frame.with_version_ack_context(|actors,bank,text,revision|{
            self.sim.require_created_world(revision)?;
            let kingdom=self.kingdom()?.ok_or("7515A0 Kingdom vanished before Color1E8 read")?;
            aa_runtime::client_description_source::kingdom_color_name(self.catalog,bank,actors,text,kingdom)
        }).map_err(error)
    }
    fn player_id(&mut self)->OrderResult<u32>{Ok(self.player().map_err(error)?.core.id)}
}
pub(crate) fn describe(players:&PlayersBorrow<'_>,local:&LocalClientBorrow<'_>,sim:&Sim,frame:&Frame<'_,'static>,catalog:&Catalog<'_>)->OrderResult<Vec<u16>>{
    players.require_same_client_owner(local).map_err(error)?;
    version_ack::describe(&mut Current{players,local,sim,frame,catalog})
}
