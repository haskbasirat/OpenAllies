use super::*;
use serde_json::{Value,json};
struct LabOs;
impl crate::execution_context::OsThreadIds for LabOs{fn current_thread_id(&mut self)->Result<u32>{Ok(17)}}
struct Backend<'a>{
 input:&'a Value,application:Arc<crate::execution_context::ApplicationThread>,origin:&'a Arc<()>,
 current:usize,headers:&'a [Arc<Header>],events:Vec<Value>,dispatch_held:bool,load_held:bool,
}
impl Services for Backend<'_>{
 fn current_thread(&mut self)->Result<ThreadHandle>{
  assert_eq!(self.headers[self.current].state()?,1);self.events.push(json!(["thread",self.current]));
  if self.input["stop"]=="thread"{return Err("thread".into())}
  ThreadHandle::on_application(&self.application,&mut LabOs)
 }
 fn enter_load(&mut self,_:&Handle)->Result<()>{assert!(self.dispatch_held);self.load_held=true;Ok(())}
 fn load_resource_4c(&mut self,_:&Handle)->Result<()>{
  self.events.push(json!(["resource",self.current]));if self.input["stop"]=="resource"{Err("resource".into())}else{Ok(())}
 }
 fn after_resource_3c(&mut self,_:&Handle)->Result<()>{Ok(())}
 fn resource_flags8(&mut self,_:&Handle)->Result<u32>{Ok(0)}
 fn store_resource_flags8(&mut self,_:&Handle,_:u32)->Result<()>{Ok(())}
 fn loaded_diagnostic(&mut self,_:&Handle)->Result<()>{Ok(())}
 fn leave_load(&mut self,_:&Handle)->Result<()>{assert!(self.load_held);self.load_held=false;Ok(())}
}
impl DispatchServices for Backend<'_>{
 fn enter_dispatch(&mut self,key:&ApplicationKey,thread:&ThreadHandle)->Result<()>{assert_eq!(thread.os_id(),17);thread.require_current()?;key.require(&Arc::downgrade(self.origin))?;assert!(!self.dispatch_held);self.dispatch_held=true;Ok(())}
 fn dispatch_diagnostic(&mut self,_:DispatchKind<'_>)->Result<()>{
  self.events.push(json!(["diagnostic",self.current]));
  if self.input["stop"]=="diagnostic"{return Err("diagnostic".into())}
  if self.input["cancel"]==true{self.headers[self.current].fields.lock().unwrap().state14=4}Ok(())
 }
 fn leave_dispatch(&mut self)->Result<()>{assert!(self.dispatch_held);self.dispatch_held=false;Ok(())}
}
fn lab_handle(origin:&Arc<()>,index:usize)->Handle{
 let source=crate::resource_objects::SourceFile::from_catalog(&aa_rules::sources::CatalogEntry{logical_path:format!("laboratory/dispatch-{index}.dds"),size:0,origin:aa_rules::sources::Origin::V14Loose});
 Handle::new(origin,crate::resource_objects::Kind::Texture,vec![65+index as u16],source)
}
fn queue_ids(queue:&TaskQueue,headers:&[Arc<Header>])->Vec<usize>{queue.storage.fields.lock().unwrap().tasks.iter().map(|t|headers.iter().position(|h|Arc::ptr_eq(h,&t.allocation)).unwrap()).collect()}
#[test]
fn native_heterogeneous_tail_dispatch_and_actual_weak_callback_prefixes(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-view-task-queue/native-view-task-queue-final.json")).unwrap();
 assert_eq!(fixture["rows"].as_array().unwrap().len(),51);
 for row in fixture["rows"].as_array().unwrap(){
  let input=&row["input"];let origin=Arc::new(());let priority=DefaultPriority::from_fresh_application(&origin);let queue=TaskQueue::from_fresh_application(&priority).unwrap();
  queue.storage.fields.lock().unwrap().sequence=input["sequence"].as_u64().unwrap()as u32;
  let world=crate::world_revision::Counter::fresh_application().construct();
  let view=crate::view_init_prefix::laboratory_queue_fields(&world,input["ac"].as_u64().unwrap_or(1)as u8,0x5a);
  let loading=crate::application_loading::laboratory_queue_fields(input["loading17"].as_u64().unwrap_or(0)as u8);
  let mut loading=Some(loading);let mut resources=vec![];let mut headers=vec![];
  for (i,spec) in input["tasks"].as_array().unwrap().iter().enumerate(){
   if spec["kind"]=="resource"{
    let task=Task::construct_for_resource(&lab_handle(&origin,i),&priority).unwrap();task.retain_in_resource_5aa944().unwrap();
    task.allocation.fields.lock().unwrap().priority8=spec["priority"].as_u64().unwrap()as u32;
    headers.push(Arc::clone(&task.allocation.header));queue.enqueue_5aed10(&task).unwrap();resources.push(task);
   }else{
    let task=crate::view_resource_task::Task::construct_for_ui(&world);task.bind_callback(&world,&view,loading.as_ref().unwrap()).unwrap();
    task.header().fields.lock().unwrap().priority8=spec["priority"].as_u64().unwrap()as u32;
    headers.push(Arc::clone(task.header()));queue.enqueue_view_5aed10(&task).unwrap();
    //Same native UI task has no retained caller ref; only the actual queue
    //payload owns it after successful adoption. Headers below are lab-only
    //retained memory observations matching the oracle's allocator transport.
    drop(task);
   }
  }
  let enqueued=queue_ids(&queue,&headers);let mut terminal="returned".to_string();
  let application=Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs).unwrap());
  let mut backend=Backend{input,application,origin:&origin,current:0,headers:&headers,events:vec![],dispatch_held:false,load_held:false};
  while let Some(current)=queue_ids(&queue,&headers).last().copied(){
   backend.current=current;
   if input["stop"]=="loading"&&input["tasks"][current]["kind"]=="view"{drop(loading.take())}
   match queue.dispatch_next_5addd0(&mut backend){
    Ok(Some(_))=>{},Ok(None)=>panic!("locked next node vanished"),Err(e)=>{terminal=if e=="Loading17 allocation is retired"{"loading".into()}else{e};break}
   }
  }
  assert!(!backend.dispatch_held&&!backend.load_held,"host guards unwind on portable error");
  let native_events=row["output"]["events"].as_array().unwrap().iter().filter(|e|matches!(e[0].as_str(),Some("thread"|"diagnostic"|"resource"))).cloned().collect::<Vec<_>>();
  assert_eq!(backend.events,native_events,"native child-read order {row}");
  let output=json!({"enqueued":enqueued,"remaining":queue_ids(&queue,&headers),"sequence":queue.storage.fields.lock().unwrap().sequence,"terminal":terminal,
   "ac":u8::from(view.initializing()),"ad":if view.initialized(){0x5a}else{0},"loading17":loading.as_ref().map_or(0,|l|l.flags_14_17()[3]),
   "tasks":headers.iter().map(|h|{let f=h.fields.lock().unwrap();json!({"refs":f.references4,"priority":f.priority8,"sequence":f.sequence_c,"queued":f.queue10.is_some(),"state":f.state14,"thread":if f.thread18.is_some(){0x12340000u32}else{0}})}).collect::<Vec<_>>()});
  let mut expected=row["output"].clone();expected.as_object_mut().unwrap().remove("events");expected["ac"]=json!(u8::from(expected["ac"].as_u64().unwrap()!=0));
  assert_eq!(output,expected,"{row}");
  if terminal!="returned"{assert!(queue.dispatch_next_5addd0(&mut backend).is_err(),"failed native prefix cannot be retried")}
  drop(resources);
 }
}
#[test]
fn actual_view_task_identity_refuses_foreign_application_world_and_dead_callback(){
 let origin=Arc::new(());let other=Arc::new(());let priority=DefaultPriority::from_fresh_application(&origin);let queue=TaskQueue::from_fresh_application(&priority).unwrap();
 let other_queue=TaskQueue::from_fresh_application(&DefaultPriority::from_fresh_application(&other)).unwrap();
 let mut worlds=crate::world_revision::Counter::fresh_application();let world=worlds.construct();let foreign=worlds.construct();
 let view=crate::view_init_prefix::laboratory_queue_fields(&world,1,1);let loading=crate::application_loading::State::from_native_constructor();
 let task=crate::view_resource_task::Task::construct_for_ui(&world);
 assert!(task.bind_callback(&foreign,&view,&loading).is_err());task.bind_callback(&world,&view,&loading).unwrap();assert!(task.bind_callback(&world,&view,&loading).is_err());
 queue.enqueue_view_5aed10(&task).unwrap();assert!(other_queue.enqueue_view_5aed10(&task).is_err());assert_eq!(other_queue.len().unwrap(),0);
 let headers=[Arc::clone(task.header())];drop(task);drop(world);
 let input=json!({});let application=Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs).unwrap());
 let mut backend=Backend{input:&input,application,origin:&origin,current:0,headers:&headers,events:vec![],dispatch_held:false,load_held:false};
 assert!(queue.dispatch_next_5addd0(&mut backend).err().unwrap().contains("destroyed"));assert!(view.initializing());assert_eq!(loading.flags_14_17()[3],0);
}
#[test]
fn dispatch_guard_uses_actual_application_key_and_same_recursive_load_lock(){
 let origin=Arc::new(());let foreign=Arc::new(());
 let cap=crate::resource_task_guards::State::from_fresh_application(&origin).capability();
 let key=ApplicationKey(Arc::downgrade(&origin));let wrong=ApplicationKey(Arc::downgrade(&foreign));
 let app=Arc::new(crate::execution_context::ApplicationThread::capture(&mut LabOs).unwrap());let thread=ThreadHandle::on_application(&app,&mut LabOs).unwrap();
 assert!(cap.acquire_task(&wrong,&thread).is_err());
 let first=cap.acquire_task(&key,&thread).unwrap();
 let nested=cap.acquire(&lab_handle(&origin,0),crate::resource_task_guards::Kind::Load04,&thread).unwrap();
 drop(nested);drop(first);
 let captured=thread.clone();let worker=std::thread::spawn(move ||captured.require_current());assert!(worker.join().unwrap().is_err());
 drop(origin);assert!(cap.acquire_task(&key,&thread).is_err());
}
