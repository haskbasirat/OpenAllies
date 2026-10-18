//! Native766D40 uses the actual NetNode mutex and retained Admin+254 stream.
use super::LocalBootstrap;
use crate::{Error,Result};
///Explicit modern diagnostic transport, retained once with the application
///Session owner. It does not claim native logger threshold/ABI behavior.
pub struct Report {records:Vec<DiagnosticRecord>}
#[derive(Debug,serde::Serialize)]
pub struct DiagnosticRecord {pub line:u32,pub function:&'static str,pub message:Vec<u16>}
impl Report {
    pub(super) fn new_modern_transport()->Self{Self{records:Vec::new()}}
    pub fn records(&self)->&[DiagnosticRecord]{&self.records}
    pub(crate) fn record(&mut self,kind:aa_orders::version_ack::Diagnostic,text:Option<&[u16]>)->aa_orders::Result<()> {
        use aa_orders::version_ack::{Diagnostic,c_string};
        let(line,function,message)=match kind {
            Diagnostic::MissingOrderingPlayer=>(0xb46,"TGC_AdminNode::OrderVersionAck","No ordering player!".encode_utf16().collect()),
            Diagnostic::MissingOrderingClient=>(0xb4b,"TGC_AdminNode::OrderVersionAck","No ordering player!".encode_utf16().collect()),
            Diagnostic::OrderingPlayerBecameNull=>(0xa13,"TGC_AdminNode::GetOrderingClient","NET: GetOrderingPlayer() is NULL".encode_utf16().collect()),
            Diagnostic::Accepted=>(0xb52,"TGC_AdminNode::OrderVersionAck",c_string(text.ok_or(aa_orders::Error::SchemaContext("actual%s diagnostic argument absent"))?).to_vec()),
        };
        self.records.try_reserve(1).map_err(|_|aa_orders::Error::Allocation)?;
        self.records.push(DiagnosticRecord{line,function,message});Ok(())
    }
}

impl LocalBootstrap {
    pub fn version_ack_diagnostics(&self)->&[DiagnosticRecord]{self.version_ack_diagnostics.records()}
    /// Called after the actual kind0 application's state-change callback.
    /// Queue this direct request even when Admin+2C0 is clear: native766D40
    /// does not run the ordinary request emitter's active/SetPlayer guards.
    pub fn request_version_ack_766d40(&mut self)->Result<()> {
        let _guard=self.world_lock.lock().map_err(|_|Error::Protocol("actual NetNode+8 world critical section poisoned".into()))?;
        let node=self.host_node.as_mut().ok_or_else(||Error::Protocol("766D40 requires the actual retained AdminNode".into()))?;
        aa_orders::native::write_version_ack_request(&node.orders.registry,&mut node.orders.request_stream)
            .map_err(|e|Error::Protocol(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_orders::{Cursor,Position};
    use std::sync::Arc;

    #[test]
    fn actual_constructor_owners_append_to_the_same_request_stream_without_set_player() {
        // Direct owned-constructor prefix probe, not a completed Host claim.
        let mut host=LocalBootstrap::from_native_constructors();
        host.host_node=Some(crate::local_host::Node::before_session_reset().unwrap());
        host.node_active=false;
        host.host_node.as_mut().unwrap().orders.last_request_player=Some(99);
        let address=&host.host_node.as_ref().unwrap().orders.request_stream as *const _;
        for _ in 0..2 {host.request_version_ack_766d40().unwrap();}
        let orders=&host.host_node.as_ref().unwrap().orders;
        assert_eq!(&orders.request_stream as *const _,address);
        assert_eq!(orders.last_request_player,Some(99));
        assert!(!host.node_active);
        assert_eq!(orders.request_stream.position(),Position{byte:2,bit:5});
        let mut reader=Cursor::with_bounds(orders.request_stream.bytes(),Position::PACKET_START,21).unwrap();
        assert_eq!(reader.read_uint(9).unwrap(),323);
        assert_eq!(reader.read_uint(9).unwrap(),323);
        assert_eq!(reader.position(),orders.request_stream.position());
    }

    #[test]
    fn missing_actual_admin_is_a_required_error_and_preserves_the_owner() {
        let mut host=LocalBootstrap::from_native_constructors();
        let identity=Arc::as_ptr(&host.identity);
        assert!(host.request_version_ack_766d40().unwrap_err().to_string().contains("actual retained AdminNode"));
        assert!(host.host_node.is_none());
        assert_eq!(Arc::as_ptr(&host.identity),identity);
    }

    #[test]
    fn same_actual_session_client_receipt_keeps6c_write_before_required_description() {
        use super::super::{OwnedPlayer,PlayerFields};
        use crate::{lobby::PeerLink,local_order_host};
        use aa_orders::{flow::{Log,Validity},net::admin::{CurrentClient,OrderingHost},payload::Field};
        struct Required;
        impl local_order_host::Effects for Required {
            fn sync_log_gate(&mut self,_:Log)->aa_orders::Result<bool>{unreachable!()}
            fn diagnostic(&mut self,_:local_order_host::Diagnostic)->aa_orders::Result<()>{unreachable!()}
            fn stop_film(&mut self,_:&mut super::super::PlayersBorrow<'_>)->aa_orders::Result<()>{unreachable!()}
            fn set_admin_abort(&mut self)->aa_orders::Result<()>{unreachable!()}
            fn valid(&mut self,_:&mut super::super::PlayersBorrow<'_>,_:&mut u32,_:&CurrentClient,_:Validity,_:usize,_:&[Field])->aa_orders::Result<bool>{unreachable!()}
            fn process(&mut self,_:&mut super::super::PlayersBorrow<'_>,_:&mut super::super::LocalClientBorrow<'_>,_:&mut u32,_:&CurrentClient,_:usize,_:&mut Vec<Field>)->aa_orders::Result<()>{unreachable!()}
        }
        //Isolated actual constructor allocations with explicitly installed
        //player/284 fixture fields; this is not a completed Host/World claim.
        let mut host=LocalBootstrap::from_native_constructors();
        host.host_node=Some(crate::local_host::Node::before_session_reset().unwrap());
        host.session.players.push(OwnedPlayer{allocation_lifetime:Arc::new(()),core:PlayerFields::from_constructor(7,PeerLink::Node),vector_44:vec![],view:None});
        let parts=host.runtime_borrow().unwrap();
        *parts.local_client.version_6c=128;parts.admin.client.field_284=7;
        let byte=parts.local_client.version_6c as *const u8;
        let mut world=local_order_host::World{players:parts.players,local_client:parts.local_client,sync_a:parts.sync_a,sync_s:parts.sync_s,effects:&mut Required};
        let error=world.acknowledge_order_version(&mut parts.admin.client.field_284,&CurrentClient::Null).unwrap_err();
        assert!(error.to_string().contains("actual7515A0 client description"));
        assert_eq!(world.local_client.order_version(),1);
        assert_eq!(world.local_client.version_6c as *const u8,byte);
        assert_eq!(parts.admin.client.field_284,7);
        assert_eq!(parts.admin.decree_stream.position(),Position::PACKET_START);
    }

    #[test]
    fn modern_ack_report_is_one_retained_owner_across_runtime_loans() {
        use aa_orders::version_ack::Diagnostic;
        let mut host=LocalBootstrap::from_native_constructors();
        host.host_node=Some(crate::local_host::Node::before_session_reset().unwrap());
        let owner=&host.version_ack_diagnostics as *const Report;
        for kind in [Diagnostic::MissingOrderingPlayer,Diagnostic::MissingOrderingClient,Diagnostic::OrderingPlayerBecameNull,Diagnostic::Accepted] {
            let parts=host.runtime_borrow().unwrap();
            assert_eq!(parts.local_client.version_ack_report as *const Report,owner);
            parts.local_client.version_ack_report.record(kind,Some(&[91,65,44,55,93,0,88])).unwrap();
        }
        let r=host.version_ack_diagnostics();
        assert_eq!(r.iter().map(|r|r.line).collect::<Vec<_>>(),[0xb46,0xb4b,0xa13,0xb52]);
        assert_eq!(r[0].message,"No ordering player!".encode_utf16().collect::<Vec<_>>());
        assert_eq!(r[1].message,r[0].message);
        assert_eq!(r[2].function,"TGC_AdminNode::GetOrderingClient");
        assert_eq!(r[2].message,"NET: GetOrderingPlayer() is NULL".encode_utf16().collect::<Vec<_>>());
        assert_eq!(r[3].function,"TGC_AdminNode::OrderVersionAck");
        assert_eq!(r[3].message,[91,65,44,55,93]);
        let parts=host.runtime_borrow().unwrap();
        assert!(parts.local_client.version_ack_report.record(Diagnostic::Accepted,None).is_err());
        assert_eq!(parts.local_client.version_ack_report.records().len(),4);
    }
}
