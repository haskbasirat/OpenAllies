//! Concrete local75B820 over the SAME retained Node histories and process
//! counter. The existing heartbeat coordinator, codecs and cursor kernel are
//! reused. Remote peers/chat/host takeover stay required at their reached leaf.
use super::*;
use crate::heartbeat::{self,Decrees,Pulse};
use aa_orders::{Cursor,History,Packet};
use aa_orders::net::transport::finalize;
use aa_runtime::heartbeat_process::Counter;

pub(crate) struct Fields<'a>{
    pub requests:&'a mut History,pub decrees:&'a mut History,pub next:&'a mut i32,
    pub flags:&'a mut [u8;6],pub vote:&'a mut u8,pub votes:&'a mut i32,
    pub max_patience:&'a mut u32,pub rtt:&'a mut u32,pub received:&'a mut u32,
    pub frame_ratio:&'a mut Option<u32>,
}
///Actual borrowed Node/NetNode fields, read lazily at each native use.
pub trait Sources {
    fn owner(&self)->&Arc<()>;
    fn server_is_self(&self)->bool;
    fn falling_behind(&self)->u8;
}
pub trait Diagnostics {
    ///404F90 "NET: Backups not ready" before75A0C0's empty -1 return.
    fn backups_not_ready(&mut self)->Result<()>;
    ///404F90 "NET: Processing backup decree %d" before packet decoding.
    fn processing_backup_decree(&mut self,index:i32)->Result<()>;
}
pub fn run(players:&mut PlayersBorrow<'_>,local:&mut LocalClientBorrow<'_>,
    counter:&mut Counter,sources:&dyn Sources,diagnostics:&mut impl Diagnostics,
)->Result<()> {
    if !Arc::ptr_eq(players.identity,local.owner_identity)||!Arc::ptr_eq(players.identity,sources.owner()){
        return Err(missing("75B820 Session/Node/process field loans have different owners"));
    }
    heartbeat::pulse(&mut Bound{local,counter,sources,diagnostics})
}
struct Bound<'a,'owner,D>{local:&'a mut LocalClientBorrow<'owner>,counter:&'a mut Counter,
    sources:&'a dyn Sources,diagnostics:&'a mut D}
fn required<T>(name:&'static str)->Result<T>{Err(Error::Unsupported(name))}
impl<D:Diagnostics> Bound<'_,'_,D>{
    fn self_peer(&self,p:PeerLink)->Result<()> {match p{
        PeerLink::Node=>Ok(()),PeerLink::Null=>Err(missing("native heartbeat peer is NULL")),
        PeerLink::Conn(_)=>required("heartbeat requires same actual remote peer fields"),
    }}
    fn byte(&self,o:u32)->Result<u8>{let f=&self.local.heartbeat;Ok(match o{
        0x30=>f.flags[0],0x48=>f.flags[2],0x49=>f.flags[3],0x4a=>f.flags[4],0x4b=>f.flags[5],0x55=>*f.vote,
        _=>return required("heartbeat unknown current status byte"),
    })}
    fn word(&self,o:u32)->Result<u32>{let f=&self.local.heartbeat;Ok(match o{
        0x4c=>*f.rtt,0x50=>*f.received,0x60=>f.frame_ratio.ok_or_else(||missing("74C450 actual Node60 ratio writer is unbound"))?,
        0x2b8=>*f.votes as u32,0x2bc=>*f.max_patience,
        _=>return required("heartbeat unknown current status word"),
    })}
}
impl<D:Diagnostics> Pulse for Bound<'_,'_,D>{
    fn is_server(&self)->bool{self.sources.server_is_self()}
    fn is_host(&self)->bool{self.local.is_host_self()}
    fn counter(&self)->i32{self.counter.word()}
    fn increment_counter(&mut self){self.counter.increment_wrapping();}
    fn reset_counter(&mut self){self.counter.reset_after_status_decree();}
    fn dispatch_backup_requests(&mut self)->Result<()>{
        //Native reads each next link AFTER dispatch. No admitted callback
        //mutates this same list; positive packet/remote handlers are required.
        for p in self.local.peers_2ac{
            self.self_peer(*p)?;
            let h=&self.local.heartbeat.requests;
            if h.begin()<h.end(){return required("7593E0 positive backup request stream/diagnostic service");}
        }
        Ok(())
    }
    fn disconnect_lost_players(&mut self)->Result<()>{
        //74DE10/74DED0 call peer virtual+4; the actual Node leaf returnsNULL.
        //A remote is never inferred to be absent/current/fully disconnected.
        for p in self.local.peers_2ac{self.self_peer(*p)?;}
        if !self.local.is_host_self(){return required("759530 positive757010 host takeover");}
        //For this proved self-only list74DED0 is true, so759530(reason5)
        //clears actual vote55 then7501D0 sees no remote reverse histories.
        for p in self.local.peers_2ac{self.self_peer(*p)?;*self.local.heartbeat.vote=0;}
        Ok(())
    }
    fn refresh_lag_state(&mut self)->Result<()>{
        let mut count=0i32;
        for p in self.local.peers_2ac{self.self_peer(*p)?;count=count.wrapping_add(i32::from(*self.local.heartbeat.vote!=0));}
        *self.local.heartbeat.votes=count;
        for p in self.local.peers_2ac{self.self_peer(*p)?;}
        *self.local.heartbeat.max_patience=0; //74DD80, no remote peers.
        for p in self.local.peers_2ac{
            self.self_peer(*p)?;
            let f=&mut self.local.heartbeat;
            *f.rtt=0;*f.received=0;f.flags[5]=0;
            f.flags[3]=self.sources.falling_behind();f.flags[4]=0;f.flags[0]=f.flags[1];f.flags[2]=0;
            //The native dead max-over60 local has no owner write. Require
            //its real value rather than creating an uninitialized-zero ratio.
            f.frame_ratio.ok_or_else(||missing("74F1A0 actual Node60 ratio is unbound"))?;
        }
        Ok(())
    }
    fn build_backup_status_decree(&mut self)->Result<()>{
        let mut w=heartbeat::new_stream()?;
        heartbeat::write_status_decree(&mut w,|o|self.word(o),|w|{
            for p in self.local.peers_2ac{
                self.self_peer(*p)?;
                heartbeat::write_status_fields(w,|o|self.byte(o),|o|self.word(o))?;
            }
            Ok(())
        })?;
        heartbeat::append(self.local.heartbeat.decrees,Packet::new(finalize(&w)?)?,heartbeat::BACKUP_DECREE_RETENTION)
    }
    fn process_backup_decrees(&mut self)->Result<()>{heartbeat::process_decree_history(self)}
    fn update_file_transfer_status(&mut self)->Result<()>{
        //Actual Node virtual+4 returnsNULL, so74F310 is not called.
        for p in self.local.peers_2ac{self.self_peer(*p)?;}
        Ok(())
    }
}
impl<D:Diagnostics> Decrees for Bound<'_,'_,D>{
    fn next(&self)->i32{*self.local.heartbeat.next}
    fn begin(&self)->i32{self.local.heartbeat.decrees.begin()}
    fn end(&self)->i32{self.local.heartbeat.decrees.end()}
    fn set_next(&mut self,v:i32){*self.local.heartbeat.next=v;}
    fn packet(&self,i:i32)->Result<Packet>{heartbeat::packet_at(self.local.heartbeat.decrees,i)}
    fn diagnostic(&mut self,i:Option<i32>)->Result<()>{match i{
        None=>self.diagnostics.backups_not_ready(),Some(i)=>self.diagnostics.processing_backup_decree(i),
    }}
    fn apply(&mut self,kind:u32,_:&mut Cursor<'_>)->Result<()>{match kind{
        0=>if self.sources.server_is_self(){Ok(())}else{required("74E5C0 real remote status reader")},
        1=>required("756E00 actual public chat reader/player/UI binding"),_=>Ok(()),
    }}
}
#[cfg(test)]
#[path="local_frame_heartbeat_tests.rs"]
mod tests;
