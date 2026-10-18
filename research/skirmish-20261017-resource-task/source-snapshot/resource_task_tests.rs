use super::*;
use serde_json::{Value,json};
fn laboratory_handle(origin:&Arc<()>,index:usize)->Handle{
    let source=crate::resource_objects::SourceFile::from_catalog(&aa_rules::sources::CatalogEntry{
        logical_path:format!("laboratory/task-{index}.dds"),size:0,origin:aa_rules::sources::Origin::V14Loose,
    });
    Handle::new(origin,crate::resource_objects::Kind::Texture,vec![65+index as u16],source)
}
fn snapshot(queue:&TaskQueue,tasks:&[Task],removed:Option<bool>)->Value{
    let order=queue.resources_in_order().unwrap().iter().map(|h|(h.logical_ids()[0]-65)as usize).collect::<Vec<_>>();
    json!({"order":order,"sequence":queue.storage.fields.lock().unwrap().sequence,
        "tasks":tasks.iter().enumerate().map(|(index,t)|json!({"refs":t.references4().unwrap(),"priority":t.priority8().unwrap(),"sequence":t.sequence_c().unwrap(),"queued":t.queued10().unwrap(),"state":t.state14().unwrap(),"thread":if t.thread18().unwrap().is_none(){0}else{panic!("queue fixture never executes task")},"resource_index":index})).collect::<Vec<_>>(),"removed":removed})
}
#[test]
fn native_task_ctor_and_same_queue_order_references_original_v14_all_pc(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-resource-task/native-resource-task.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),36);
    for row in rows{
        let origin=Arc::new(());let priority=DefaultPriority::from_fresh_application(&origin);
        let queue=TaskQueue::from_fresh_application(&priority).unwrap();
        //Explicit laboratory counter seed for native wrapping cases only.
        queue.storage.fields.lock().unwrap().sequence=row["input"]["sequence"].as_u64().unwrap()as u32;
        let tasks=(0..4).map(|i|{let task=Task::construct_for_resource(&laboratory_handle(&origin,i),&priority).unwrap();assert_eq!(task.references4().unwrap(),0);assert_eq!(task.retain_in_resource_5aa944().unwrap(),1);task}).collect::<Vec<_>>();
        let mut snapshots=vec![snapshot(&queue,&tasks,None)];
        for op in row["input"]["ops"].as_array().unwrap(){
            let task=&tasks[op[1].as_u64().unwrap()as usize];let mut removed=None;
            match op[0].as_str().unwrap(){
                "enqueue"=>queue.enqueue_5aed10(task).unwrap(),
                "priority"=>queue.set_priority_5b1f50(task,op[2].as_u64().unwrap()as u32).unwrap(),
                "remove"=>removed=Some(queue.remove_5aed90(task).unwrap()),_=>unreachable!(),
            }
            snapshots.push(snapshot(&queue,&tasks,removed));
        }
        assert_eq!(json!(snapshots),row["native"],"{row}");
    }
}
struct LabOs(u32);
impl crate::execution_context::OsThreadIds for LabOs{fn current_thread_id(&mut self)->Result<u32>{Ok(self.0)}}
#[test]
fn actual_task_identity_and_status_before_required_thread_error_prefix(){
    let first=Arc::new(());let second=Arc::new(());
    let priority=DefaultPriority::from_fresh_application(&first);let other=DefaultPriority::from_fresh_application(&second);
    assert!(Task::construct_for_resource(&laboratory_handle(&second,0),&priority).is_err());
    let task=Task::construct_for_resource(&laboratory_handle(&first,0),&priority).unwrap();task.retain_in_resource_5aa944().unwrap();
    let foreign=TaskQueue::from_fresh_application(&other).unwrap();
    assert!(foreign.enqueue_5aed10(&task).is_err());assert_eq!(foreign.len().unwrap(),0);
    assert!(task.set_state_5b18c0(1,||Err("required actual thread owner".into())).is_err());
    assert_eq!(task.state14().unwrap(),1);assert!(task.thread18().unwrap().is_none());
    let app=Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs(17)).unwrap());
    let thread=ThreadHandle::on_application(&app,&mut LabOs(17)).unwrap();
    task.set_state_5b18c0(1,||Ok(thread)).unwrap();
    task.set_state_5b18c0(2,||panic!("native state2 must preserve18 without current-thread read")).unwrap();
    assert_eq!(task.thread18().unwrap().unwrap().os_id(),17);
    assert!(ThreadHandle::on_application(&app,&mut LabOs(18)).is_err());
    drop(first);assert!(TaskQueue::from_fresh_application(&priority).is_err());assert!(task.state14().is_err());
}

struct Backend<'a>{input:&'a Value,task:&'a Task,application:Arc<crate::execution_context::ApplicationThread>,events:Vec<&'static str>,stores:Vec<u32>,flags:u32,load_guard:bool,request_guard:bool}
impl Backend<'_>{
    fn same(&self,resource:&Handle){assert!(resource.same_identity(self.task.resource()))}
    fn store(&mut self,value:u32){self.flags=value;self.stores.push(value)}
}
impl Services for Backend<'_>{
    fn current_thread(&mut self)->Result<ThreadHandle>{
        assert_eq!(self.task.state14()?,1);self.events.push("thread");
        if self.input["stop"]=="thread"{return Err("thread_required".into())}
        ThreadHandle::on_application(&self.application,&mut LabOs(17))
    }
    fn enter_load(&mut self,r:&Handle)->Result<()>{self.same(r);self.events.push("enter_load");self.load_guard=true;Ok(())}
    fn load_resource_4c(&mut self,r:&Handle)->Result<()>{self.same(r);self.events.push("load");if self.input["stop"]=="load"{Err("load_required".into())}else{Ok(())}}
    fn after_resource_3c(&mut self,r:&Handle)->Result<()>{self.same(r);self.events.push("after");let flags=self.flags|4;self.store(flags);self.store(flags|2);Ok(())}
    fn resource_flags8(&mut self,r:&Handle)->Result<u32>{self.same(r);Ok(self.flags)}
    fn store_resource_flags8(&mut self,r:&Handle,value:u32)->Result<()>{self.same(r);self.store(value);Ok(())}
    fn loaded_diagnostic(&mut self,r:&Handle)->Result<()>{self.same(r);assert_eq!(self.task.state14()?,2);self.events.push("loaded");Ok(())}
    fn leave_load(&mut self,r:&Handle)->Result<()>{self.same(r);assert!(self.load_guard);self.load_guard=false;self.events.push("leave_load");Ok(())}
    fn enter_request(&mut self,r:&Handle)->Result<()>{self.same(r);self.events.push("enter_request");self.request_guard=true;if self.input["clear_cache"].as_bool().unwrap(){self.flags&=!0x20}Ok(())}
    fn cached_resource_59bfa0(&mut self,r:&Handle)->Result<()>{self.same(r);self.events.push("cache");Err("cache_required".into())}
    fn leave_request(&mut self,r:&Handle)->Result<()>{self.same(r);assert!(self.request_guard);self.request_guard=false;self.events.push("leave_request");Ok(())}
    fn request_diagnostic(&mut self,r:&Handle,immediate:bool)->Result<()>{self.same(r);self.events.push(if immediate{"request_immediate"}else{"request_queue"});Ok(())}
    fn block_timer_reset_59aa70(&mut self)->Result<()>{self.events.push("timer_reset");Ok(())}
    fn block_timer_start_59ab60(&mut self)->Result<()>{self.events.push("timer_start");Ok(())}
    fn removing_from_queue_diagnostic(&mut self,r:&Handle)->Result<()>{self.same(r);self.events.push("removing");Ok(())}
    fn timing_enabled_40eaa0(&mut self)->Result<bool>{self.events.push("timing_enabled");Ok(self.input["timing"].as_bool().unwrap())}
    fn block_diagnostic(&mut self,r:&Handle,reason:BlockDiagnostic)->Result<()>{self.same(r);self.events.push(match reason{BlockDiagnostic::RanImmediately=>"diag_immediate",BlockDiagnostic::CompletedWhileWaitingForRemove=>"diag_completed",BlockDiagnostic::WaitedForLoader=>"diag_waited"});Ok(())}
    fn block_till_ready_5b1fd0(&mut self,_:&Task,_:&TaskQueue)->Result<()>{self.events.push("wait");Err("wait_required".into())}
}
#[test]
fn native_request_run_block_same_task_and_required_prefixes(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-resource-task/native-task-wrappers.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),66);
    for row in rows{
        let input=&row["input"];let origin=Arc::new(());let priority=DefaultPriority::from_fresh_application(&origin);
        let queue=TaskQueue::from_fresh_application(&priority).unwrap();
        let task=Task::construct_for_resource(&laboratory_handle(&origin,0),&priority).unwrap();task.retain_in_resource_5aa944().unwrap();
        if input["queued"].as_bool().unwrap(){queue.enqueue_5aed10(&task).unwrap()}
        task.allocation.fields.lock().unwrap().state14=input["state"].as_u64().unwrap()as u32;
        let application=Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs(17)).unwrap());
        let mut backend=Backend{input,task:&task,application,events:vec![],stores:vec![],flags:input["flags"].as_u64().unwrap()as u32,load_guard:false,request_guard:false};
        let result=if input["entry"]=="request"{task.request_5b0270(&queue,input["mode"].as_u64().unwrap()as u32,&mut backend)}else{task.block_5b2180(&queue,&mut backend)};
        let terminal=result.err().unwrap_or_else(||"returned".into());
        //Rust releases temporary guard loans on a required-service return.
        //The native fixture stops before that service, not via C++ unwind;
        //compare the exact native prefix separately from this host cleanup.
        if terminal!="returned" && matches!(backend.events.last(),Some(&"leave_load")|Some(&"leave_request")){backend.events.pop();}
        assert!(!backend.load_guard && !backend.request_guard);
        let output=json!({"terminal":terminal,"events":backend.events,"flag_stores":backend.stores,"flags":backend.flags,
            "task":{"refs":task.references4().unwrap(),"priority":task.priority8().unwrap(),"sequence":task.sequence_c().unwrap(),"state":task.state14().unwrap(),"thread_present":task.thread18().unwrap().is_some(),"queued":task.queued10().unwrap()},"queue_contains":queue.len().unwrap()!=0});
        assert_eq!(output,row["native"],"{row}");
    }
}

#[test]
fn native_queue_auto_reset_signal_is_real_state_and_never_task_readiness(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-resource-task/native-queue-signal.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(),6);
    for row in fixture["rows"].as_array().unwrap(){
        assert_eq!(row["native"]["calls"][0],json!(["CreateEvent",[0,0,0,0]]));
        assert_eq!(row["native"]["constructed"]["event_signalled"],false);
        assert_eq!(row["native"]["constructed"]["signal_byte"],row["poison"]);
        assert_eq!(row["native"]["after"]["event_signalled"],true);
        let origin=Arc::new(());let priority=DefaultPriority::from_fresh_application(&origin);
        let queue=TaskQueue::from_fresh_application(&priority).unwrap();
        assert!(!queue.storage.fields.lock().unwrap().signalled);
        let task=Task::construct_for_resource(&laboratory_handle(&origin,0),&priority).unwrap();task.retain_in_resource_5aa944().unwrap();
        queue.enqueue_5aed10(&task).unwrap();queue.enqueue_5aed10(&task).unwrap();
        assert!(queue.storage.fields.lock().unwrap().signalled);
        queue.wait_for_work().unwrap();assert!(!queue.storage.fields.lock().unwrap().signalled);
        assert_eq!(task.state14().unwrap(),0);assert_eq!(queue.len().unwrap(),2);
    }
}
#[test]
fn external_task_publication_advances_the_same_sequence_without_readiness(){
    let origin=Arc::new(());let priority=DefaultPriority::from_fresh_application(&origin);
    let queue=TaskQueue::from_fresh_application(&priority).unwrap();
    let first=queue.reserve_external_enqueue_5ae160().unwrap();
    assert_eq!(first.value().unwrap(),0);assert_eq!(queue.len().unwrap(),0);
    let task=Task::construct_for_resource(&laboratory_handle(&origin,0),&priority).unwrap();task.retain_in_resource_5aa944().unwrap();
    queue.enqueue_5aed10(&task).unwrap();assert_eq!(task.sequence_c().unwrap(),1);assert_eq!(task.state14().unwrap(),0);
    let second=queue.reserve_external_enqueue_5ae160().unwrap();assert_eq!(second.value().unwrap(),2);
    assert_eq!(queue.len().unwrap(),1);assert_eq!(task.state14().unwrap(),0);
    drop(origin);assert!(first.value().is_err());assert!(second.value().is_err());
}
