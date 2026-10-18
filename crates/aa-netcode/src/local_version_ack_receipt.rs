//!Actual751FC0 access to the same Admin284/Session player/client6C owner.
use super::*;
use aa_orders::version_ack;
struct Current<'a,'world,'owner,E,P,L> {
    world:&'a mut World<'world,'owner,E,P,L>,slot:&'a mut u32,
}
impl<'owner,E:Effects,P:AsRef<PlayersBorrow<'owner>>+AsMut<PlayersBorrow<'owner>>,
    L:AsRef<LocalClientBorrow<'owner>>+AsMut<LocalClientBorrow<'owner>>>
    version_ack::World for Current<'_,'_,'owner,E,P,L>
{
    type Player=PlayerHandle;
    type Client=PeerLink;
    fn ordering_player(&mut self)->Result<Option<PlayerHandle>> {self.world.current_player(*self.slot)}
    fn player_client(&mut self,p:&PlayerHandle)->Result<Option<PeerLink>> {
        let link=source_at(323,self.world.players.as_ref().player_link(p))?;
        Ok((link!=PeerLink::Null).then_some(link))
    }
    fn store_acknowledged(&mut self,client:&PeerLink)->Result<()> {
        match client {
            PeerLink::Node=>{
                source_at(323,self.world.players.as_ref().require_same_client_owner(self.world.local_client.as_ref()))?;
                self.world.local_client.as_mut().acknowledge_version_751fc0();Ok(())
            }
            PeerLink::Conn(_)=>required("751FC0 actual remote client6C owner"),
            PeerLink::Null=>required("751FC0 would dereference null client6C"),
        }
    }
    fn client_description(&mut self,client:&PeerLink)->Result<Vec<u16>> {
        self.world.effects.version_ack_client_description(self.world.players.as_mut(),self.world.local_client.as_mut(),client)
    }
    fn diagnostic(&mut self,kind:version_ack::Diagnostic,text:Option<&[u16]>)->Result<()> {
        source_at(323,self.world.players.as_ref().require_same_client_owner(self.world.local_client.as_ref()))?;
        self.world.local_client.as_mut().version_ack_report.record(kind,text)
    }
}
impl<'owner,E:Effects,P:AsRef<PlayersBorrow<'owner>>+AsMut<PlayersBorrow<'owner>>,
    L:AsRef<LocalClientBorrow<'owner>>+AsMut<LocalClientBorrow<'owner>>>
    World<'_,'owner,E,P,L>
{
    pub(super) fn receive_version_ack_751fc0(&mut self,slot:&mut u32)->Result<()> {
        version_ack::run(&mut Current{world:self,slot}).map(drop)
    }
}
