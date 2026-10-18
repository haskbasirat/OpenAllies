//!Read-only transport of the actual76AC90 Session+F8 getter while the SAME
//!Host is immutably borrowed and its retained NetNode+8 lock is held. This
//!is an expiring logical read lease, not another mutable Session owner.
use std::sync::{Arc,Weak};

///Only LocalBootstrap's locked scope can issue this Send packet. The UI
///performs each logical read at the original reset-loop branch. It cannot
///retain authority after the simulation callback returns or unwinds.
pub struct SessionPhaseLease { phase:Weak<u32> }
impl SessionPhaseLease {
    pub fn read_76ac90(&self)->Result<u32,String>{
        self.phase.upgrade().map(|phase|*phase)
            .ok_or_else(||"76AC90 Session phase read lease expired with its locked simulation scope".into())
    }
}
pub(super) fn with_locked_phase<R>(phase:u32,call:impl FnOnce(SessionPhaseLease)->Result<R,String>)->Result<R,String>{
    //The caller retains the immutable Host loan AND the same NetNode lock
    //through this synchronous callback/RPC. Thus no portable phase writer or
    //Session replacement can run; transporting this invariant scalar does
    //not move the Session or change the native logical read order.
    let phase=Arc::new(phase);
    let result=call(SessionPhaseLease{phase:Arc::downgrade(&phase)});
    drop(phase);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn locked_phase_packet_reads_on_ui_thread_and_expires_on_return(){
        let stale=with_locked_phase(7,|packet|{
            std::thread::spawn(move||{
                assert_eq!(packet.read_76ac90()?,7);
                assert_eq!(packet.read_76ac90()?,7);
                Ok(packet)
            }).join().unwrap()
        }).unwrap();
        assert!(stale.read_76ac90().unwrap_err().contains("expired"));
    }
    #[test]
    fn canceled_rpc_drops_the_same_read_lease(){
        let mut stale=None;
        let result:Result<(),String>=with_locked_phase(5,|packet|{stale=Some(packet);Err("canceled".into())});
        assert_eq!(result.unwrap_err(),"canceled");
        assert!(stale.unwrap().read_76ac90().is_err());
    }
}
