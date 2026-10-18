//!Actual Admin stream/284 integration. The codecs and request emitter retain
//!their existing native proofs; this verifies no deferred stream or copyback.
use aa_orders::{Result,Gate,Error,Position,commands::WireWriter,
    flow::{SyncEntry,Validity},net::admin::{AdminNode,CurrentClient,OrderingHost,RequestEmission},
    payload::{Field,SchemaContext,Resolver,Registry,ThingCheck,NameKind}};
struct Context;
impl SchemaContext for Context{
    fn registry_count(&self,_:Registry)->Result<usize>{Err(Error::SchemaContext("no registry in304/315/323"))}
    fn team_command_flags(&self,_:u32)->Result<u32>{Err(Error::SchemaContext("no command in304/315/323"))}
    fn definition_kind(&self,_:Registry,_:u32)->Result<u32>{Err(Error::SchemaContext("no kind in304/315/323"))}
    fn zero_width_list_budget(&self)->usize{usize::MAX}
}
impl Resolver for Context{
    fn thing(&self,_:ThingCheck,_:u16)->Option<u16>{None}
    fn player(&self,id:u32)->bool{id==1}
    fn definition(&self,_:Registry,_:u32)->bool{false}
    fn named(&self,_:NameKind,name:&[u16])->Option<Vec<u16>>{Some(name.to_vec())}
}
fn fields()->Vec<Field>{vec![Field::Text(vec![65,0,66]),Field::Text(vec![0xd800,0x3bb]),
    Field::Text("ai_profile".encode_utf16().collect()),Field::Uint{width:1,value:0}]}
struct Host{fail_after:bool,malformed:bool,seen:Vec<usize>}
impl OrderingHost for Host{
    fn has_ordering_client(&mut self,slot:&mut u32,_:&CurrentClient)->Result<bool>{Ok(*slot!=0)}
    fn version_acknowledged(&mut self,_:&mut u32,_:&CurrentClient)->Result<bool>{Ok(true)}
    fn film_session(&mut self,_:&mut u32,_:&CurrentClient)->Result<bool>{Ok(false)}
    fn valid(&mut self,_:&mut u32,_:&CurrentClient,_:Validity,_:usize,_:&[Field])->Result<bool>{Ok(true)}
    fn sync(&mut self,_:&mut u32,_:&CurrentClient,_:SyncEntry)->Result<()>{Ok(())}
    fn process(&mut self,slot:&mut u32,_:&CurrentClient,decree:usize,fields:&mut Vec<Field>)->Result<()>{
        assert_eq!(decree,298);
        let [Field::Uint{width:32,value}]=fields.as_slice()else{panic!("SetPlayer")};*slot=*value;
        self.seen.push(decree);Ok(())
    }
    fn process_with_emission(&mut self,slot:&mut u32,client:&CurrentClient,decree:usize,
        values:&mut Vec<Field>,emission:&mut RequestEmission<'_>)->Result<()>{
        if decree==298{return self.process(slot,client,decree,values);}
        assert_eq!(decree,315);assert_eq!(*slot,1);assert_eq!(*values,vec![Field::Bounded{n:5,value:3}]);
        self.seen.push(decree);
        let values=if self.malformed{vec![Field::Uint{width:32,value:0}]}else{fields()};
        emission.emit_request(true,323,&values,if self.malformed{4}else{*slot},&Context)?;
        if self.fail_after{return Err(Error::SchemaContext("required post-emission child"));}
        Ok(())
    }
    fn mirror_enabled(&mut self,_:&mut u32,_:&CurrentClient)->Result<bool>{Ok(false)}
}
#[test]
fn real_host_callback_emits_into_same_request_stream_before_return_or_error(){
    for(fail_after,malformed)in[(false,false),(true,false),(false,true)]{
        let mut admin=AdminNode::new(0).unwrap();
        admin.emit_request(true,304,&[Field::Bounded{n:5,value:3}],1,&Context).unwrap();
        admin.seal_requests().unwrap();let begin=admin.requests.begin();
        let mut host=Host{fail_after,malformed,seen:vec![]};
        let result=admin.host_execute_owned_requests(Some(1),&mut[],&mut host,&Context,&Context,&mut Gate::default());
        assert_eq!(result.is_err(),fail_after||malformed);
        assert_eq!(host.seen,[298,298,315]);
        //An error keeps the current packet/284 and already-written emission.
        if fail_after||malformed{assert_eq!(admin.requests.begin(),begin);assert_eq!(admin.client.field_284,1);}
        else{assert_eq!(admin.requests.begin(),begin+1);assert_eq!(admin.client.field_284,0);}
        let mut expected=WireWriter::new(Position::PACKET_START).unwrap();
        if malformed{
            expected.write_uint(320,9).unwrap();expected.write_uint(4,32).unwrap();expected.write_uint(323,9).unwrap();
            assert_eq!(admin.last_request_player,Some(4));
        }else{
            expected.write_uint(323,9).unwrap();aa_orders::payload::encode(323,&fields(),&mut expected,&Context).unwrap();
            assert_eq!(admin.last_request_player,Some(1));
        }
        assert_eq!(aa_orders::net::transport::finalize(&admin.request_stream).unwrap(),aa_orders::net::transport::finalize(&expected).unwrap());
    }
}
