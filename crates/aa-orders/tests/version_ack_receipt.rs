use aa_orders::{version_ack::{self,Diagnostic},Error,Result};
use serde_json::{Value,json};

struct Fields {presence:u32,byte:u8,fail:Option<String>,events:Vec<Value>}
impl version_ack::World for Fields {
    type Player=();type Client=();
    fn ordering_player(&mut self)->Result<Option<()>>{self.events.push(json!(["player",self.presence!=0]));Ok((self.presence!=0).then_some(()))}
    fn player_client(&mut self,_:&())->Result<Option<()>>{self.events.push(json!(["client",self.presence==2]));Ok((self.presence==2).then_some(()))}
    fn store_acknowledged(&mut self,_:&())->Result<()>{self.byte=1;self.events.push(json!(["store",1]));Ok(())}
    fn client_description(&mut self,_:&())->Result<Vec<u16>>{
        self.events.push(json!(["describe"]));
        if self.fail.as_deref()==Some("describe"){return Err(Error::SchemaContext("required7515A0 cutoff"));}
        Ok("[actual-client,7]".encode_utf16().collect())
    }
    fn diagnostic(&mut self,kind:Diagnostic,text:Option<&[u16]>)->Result<()>{
        let line=match kind{Diagnostic::MissingOrderingPlayer=>0xb46,Diagnostic::MissingOrderingClient=>0xb4b,Diagnostic::OrderingPlayerBecameNull=>0xa13,Diagnostic::Accepted=>0xb52};
        self.events.push(json!(["diagnostic",line,text.map(String::from_utf16_lossy)]));
        if self.fail.as_deref()==Some("diagnostic"){return Err(Error::SchemaContext("required404F60 cutoff"));}
        Ok(())
    }
}
#[test]
fn registered_receipt_matches_both_pe_all_pc_ordered_fields_and_failure_prefixes(){
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-order-version-ack-receipt/native-receipt.json");
    let fixture:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for machine in fixture["machines"].as_array().unwrap(){
        assert_eq!(machine["registry_count"],358);assert_eq!(machine["handler"],"0x766cf0");
        for row in machine["rows"].as_array().unwrap(){
            let mut f=Fields{presence:row["presence"].as_u64().unwrap()as u32,byte:row["old6c"].as_u64().unwrap()as u8,
                fail:row["fail"].as_str().map(str::to_owned),events:vec![]};
            let result=version_ack::run(&mut f);
            assert_eq!(result.is_err(),!row["cutoff"].is_null());
            assert_eq!(f.byte as u64,row["new6c"].as_u64().unwrap());
            assert_eq!(json!(f.events),row["events"]);
        }
    }
}

use aa_orders::{Gate,flow::{SyncEntry,Validity},net::admin::{AdminNode,CurrentClient,OrderingHost},payload::{Field,SchemaContext,Resolver,Registry,ThingCheck,NameKind}};
struct Context;
impl SchemaContext for Context {
    fn registry_count(&self,_:Registry)->Result<usize>{unreachable!()}
    fn team_command_flags(&self,_:u32)->Result<u32>{unreachable!()}
    fn definition_kind(&self,_:Registry,_:u32)->Result<u32>{unreachable!()}
    fn zero_width_list_budget(&self)->usize{0}
}
impl Resolver for Context {
    fn thing(&self,_:ThingCheck,_:u16)->Option<u16>{unreachable!()}
    fn player(&self,id:u32)->bool{id==7}
    fn definition(&self,_:Registry,_:u32)->bool{unreachable!()}
    fn named(&self,_:NameKind,_:&[u16])->Option<Vec<u16>>{unreachable!()}
}
struct Host {version:u8,fail:bool,acks:usize}
impl OrderingHost for Host {
    fn acknowledge_order_version(&mut self,slot:&mut u32,_:&CurrentClient)->Result<()>{
        assert_eq!(*slot,7);self.version=1;self.acks+=1;
        if self.fail{return Err(Error::SchemaContext("required diagnostic after same-client6C write"));}Ok(())
    }
    fn has_ordering_client(&mut self,_:&mut u32,_:&CurrentClient)->Result<bool>{panic!("template guard must not run")}
    fn version_acknowledged(&mut self,_:&mut u32,_:&CurrentClient)->Result<bool>{panic!("template version guard must not run")}
    fn film_session(&mut self,_:&mut u32,_:&CurrentClient)->Result<bool>{panic!("template film guard must not run")}
    fn valid(&mut self,_:&mut u32,_:&CurrentClient,_:Validity,_:usize,_:&[Field])->Result<bool>{panic!("template validity must not run")}
    fn sync(&mut self,_:&mut u32,_:&CurrentClient,_:SyncEntry)->Result<()>{Ok(())}
    fn process(&mut self,slot:&mut u32,_:&CurrentClient,id:usize,fields:&mut Vec<Field>)->Result<()>{
        assert_eq!(id,298);assert_eq!(fields.as_slice(),[Field::Uint{width:32,value:7}]);*slot=7;Ok(())
    }
    fn mirror_enabled(&mut self,_:&mut u32,_:&CurrentClient)->Result<bool>{Ok(false)}
}
#[test]
fn actual_admin_history_routes_only323_and_preserves_receipt_failure_owners(){
    for fail in [false,true]{
        let mut admin=AdminNode::new(0).unwrap();
        aa_orders::native::write_version_ack_request(&admin.registry,&mut admin.request_stream).unwrap();
        admin.seal_requests().unwrap();
        let mut host=Host{version:128,fail,acks:0};
        let result=admin.host_execute_owned_requests(Some(7),&mut[],&mut host,&Context,&Context,&mut Gate::default());
        assert_eq!(result.is_err(),fail);assert_eq!(host.version,1);assert_eq!(host.acks,1);
        assert_eq!(admin.requests.end()-admin.requests.begin(),i32::from(fail));
        assert_eq!(admin.ordering_player(),fail.then_some(7));
        assert_eq!(admin.previous_ordering_player,(!fail).then_some(7));
        //Only native7509F0's ordinary peer announcement is in the decree stream.
        assert_eq!(admin.decree_stream.position().absolute().unwrap(),3+9+32);
        assert_eq!(admin.mirror_stream.position(),aa_orders::Position::PACKET_START);
    }
}
