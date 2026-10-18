use super::*;
use aa_rules::sources::{ResolvedFile,Origin,sha256};
use aa_runtime::restore_lifecycle::{ScopedManagedLoad,TaskKind};
use serde_json::{Value,json};

struct Source{owner:Arc<()>,behind:u8}
impl Sources for Source{
    fn owner(&self)->&Arc<()>{&self.owner}
    fn server_is_self(&self)->bool{true}
    fn falling_behind(&self)->u8{self.behind}
}
struct Log{fail:bool,events:Vec<Value>}
impl Diagnostics for Log{
    fn backups_not_ready(&mut self)->Result<()>{self.add("NET: Backups not ready",None)}
    fn processing_backup_decree(&mut self,i:i32)->Result<()>{self.add("NET: Processing backup decree %d",Some(i))}
}
impl Log{fn add(&mut self,s:&str,i:Option<i32>)->Result<()>{
    self.events.push(json!({"format":s,"index":i}));
    if self.fail{Err(missing("typed diagnostic boundary"))}else{Ok(())}
}}
fn process_owner()->aa_runtime::restore_lifecycle::StartupOwner{
    //Explicit constructor-only projection. Retiring this incomplete SAI
    //interval mints no ActivatedSession or completed World startup receipt.
    let bytes=b"[Vars]\n{\n string GoalPriorityCSV = null\n}\n".to_vec();
    let file=ResolvedFile{logical_path:"UVars.tgi".into(),origin:Origin::V14Loose,sha256:sha256(&bytes),bytes};
    let mut loading=ScopedManagedLoad::from_uvars(&file,0).unwrap().begin(TaskKind::Normal).unwrap();
    loading.with_startup_engine(|_|Ok(())).unwrap();
    loading.into_startup_owner().ok().unwrap()
}
#[test]
fn native_local_pulses_keep_same_histories_counter_and_error_prefix(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-local-heartbeat.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(),120);
    for row in fixture["rows"].as_array().unwrap(){
        let i=&row["input"];
        let mut owner=LocalBootstrap::from_native_constructors();
        let mut node=crate::local_host::Node::before_session_reset().unwrap();node.after_session_reset().unwrap();
        node.backup_requests=History::new(0,4).unwrap();node.backup_decrees=History::new(0,4).unwrap();
        node.next_backup_decree=i["next"].as_i64().unwrap() as i32;
        node.flags=[37,i["flag31"].as_u64().unwrap() as u8,38,39,40,41];
        node.vote=i["vote"].as_u64().unwrap() as u8;node.votes=7;node.max_patience=0x3f800000;
        node.rtt=0x3e800000;node.received_fraction=0x3f400000;
        node.frame_ratio_60=Some(i["ratio"].as_u64().unwrap() as u32);
        owner.host_node=Some(node);owner.node_host=PeerLink::Node;owner.peer_list=vec![PeerLink::Node];
        let source=Source{owner:Arc::clone(&owner.identity),behind:i["behind"].as_u64().unwrap() as u8};
        let mut app=process_owner();let (_,counter)=app.process_effects();
        for _ in 0..i["counter"].as_u64().unwrap(){counter.increment_wrapping();}
        let node_addr=owner.host_node.as_ref().unwrap() as *const _;
        let hist_addr=&owner.host_node.as_ref().unwrap().backup_decrees as *const _;
        let counter_addr=counter as *const _;
        let mut log=Log{fail:i["fail_diag"].as_bool().unwrap(),events:vec![]};
        for step in row["steps"].as_array().unwrap(){
            let result={let mut b=owner.runtime_borrow().unwrap();run(&mut b.players,&mut b.local_client,counter,&source,&mut log)};
            assert_eq!(result.is_err(),!step["stop"].is_null(),"{i}: {result:?}");
            assert_eq!(log.events,*step["diagnostics"].as_array().unwrap(),"{i}");
            let expected=&step["after"];let n=owner.host_node.as_ref().unwrap();let b=&expected["node"]["base"];
            assert_eq!(counter.word(),expected["heartbeat_counter"].as_i64().unwrap() as i32);
            assert_eq!(n.next_backup_decree,expected["node"]["next_backup_decree"].as_i64().unwrap() as i32);
            assert_eq!(n.votes,expected["node"]["vote_count"].as_i64().unwrap() as i32);
            assert_eq!(n.max_patience,expected["node"]["max_patience"].as_u64().unwrap() as u32);
            for (j,k) in ["flag_30","flag_31","flag_48","flag_49","flag_4a","flag_4b"].iter().enumerate(){assert_eq!(n.flags[j],b[k].as_u64().unwrap() as u8,"{i} {k}");}
            assert_eq!(n.vote,b["vote"].as_u64().unwrap() as u8);assert_eq!(n.rtt,b["rtt"].as_u64().unwrap() as u32);
            assert_eq!(n.received_fraction,b["received_fraction"].as_u64().unwrap() as u32);
            assert_eq!(n.frame_ratio_60,Some(b["frame_ratio"].as_u64().unwrap() as u32));
            let h=&expected["node"]["backup_decrees"];
            assert_eq!(n.backup_decrees.begin(),h["begin"].as_i64().unwrap() as i32);
            assert_eq!(n.backup_decrees.end()-n.backup_decrees.begin(),h["packets"].as_array().unwrap().len() as i32);
            for (j,p) in h["packets"].as_array().unwrap().iter().enumerate(){
                let expected:Vec<u8>=p.as_array().unwrap().iter().map(|x|x.as_u64().unwrap() as u8).collect();
                assert_eq!(heartbeat::packet_at(&n.backup_decrees,n.backup_decrees.begin()+j as i32).unwrap().bytes(),expected);
            }
            assert_eq!(n as *const _,node_addr);assert_eq!(&n.backup_decrees as *const _,hist_addr);assert_eq!(counter as *const _,counter_addr);
        }
    }
}

#[test]
fn foreign_source_owner_is_rejected_before_process_counter_change(){
    let mut owner=LocalBootstrap::from_native_constructors();owner.host_node=Some(crate::local_host::Node::before_session_reset().unwrap());
    let source=Source{owner:Arc::new(()),behind:0};let mut app=process_owner();let (_,counter)=app.process_effects();
    let mut b=owner.runtime_borrow().unwrap();let mut log=Log{fail:false,events:vec![]};
    assert!(run(&mut b.players,&mut b.local_client,counter,&source,&mut log).is_err());
    assert_eq!(counter.word(),0);assert!(log.events.is_empty());
}
