//! TGC_ResourceManagerTaskLoad5B1850 and shared5B17F0 task fields. The actual
//! ResourceManager entry and its queue share one allocation; handles do not
//! copy status, priority, native reference count or resource ownership.
use crate::resource_objects::Handle;
use std::sync::{Arc,Condvar,Mutex,Weak};
type Result<T>=std::result::Result<T,String>;

/// Current B282F0 owner. The fresh native PE initializes this word to1;
/// construction is not a claim that arbitrary later native writers are absent.
/// No caller-selected priority or guessed profile override is accepted.
pub struct DefaultPriority{current:u32,application:Weak<()>}
impl DefaultPriority{
    pub(crate) fn from_fresh_application(application:&Arc<()>)->Self{Self{current:1,application:Arc::downgrade(application)}}
    pub fn current(&self)->u32{self.current}
}

/// Task18 is a reference to the actual TGC thread identity, not an OS-ID cast.
/// The modern backend binds that identity to a real captured application or
/// resource-worker owner, rejecting mismatched/reused execution identities.
#[derive(Clone)]
pub struct ThreadHandle{os_id:u32,host_thread:std::thread::ThreadId,lifetime:ThreadLifetime}
#[derive(Clone)]
enum ThreadLifetime{Application(Weak<crate::execution_context::ApplicationThread>),Worker(crate::execution_context::WorkerWitness),Simulation(crate::simulation_execution::SimulationIdentity)}
impl ThreadHandle{
    pub fn on_application(application:&Arc<crate::execution_context::ApplicationThread>,os:&mut impl crate::execution_context::OsThreadIds)->Result<Self>{
        let id=os.current_thread_id()?;
        if id==0 || id!=application.id(){return Err("Resource task current application thread identity differs".into())}
        Ok(Self{os_id:id,host_thread:std::thread::current().id(),lifetime:ThreadLifetime::Application(Arc::downgrade(application))})
    }
    pub fn on_resource_worker(execution:&mut crate::execution_context::LoadExecution<'_>)->Result<Self>{
        let id=execution.current_os_thread_id()?;
        if id==0 || id!=execution.identity().child_os_thread_id(){return Err("Resource task current worker identity differs".into())}
        Ok(Self{os_id:id,host_thread:std::thread::current().id(),lifetime:ThreadLifetime::Worker(execution.identity().witness())})
    }
    pub fn on_simulation(execution:&mut crate::simulation_execution::Execution<'_>)->Result<Self>{
        let identity=execution.identity();let id=execution.current_os_thread_id()?;
        if id==0||id!=identity.os_thread_id()?{return Err("Resource task current simulation thread identity differs".into())}
        Ok(Self{os_id:id,host_thread:std::thread::current().id(),lifetime:ThreadLifetime::Simulation(identity)})
    }
    ///Host/backend guard validation, not a native scalar role cast. The same
    ///Rust thread and retained native identity must still be alive.
    pub fn require_current(&self)->Result<()>{
        if self.host_thread!=std::thread::current().id(){return Err("Resource task thread capability used on another host thread".into())}
        match &self.lifetime{
            ThreadLifetime::Application(owner)=>{owner.upgrade().ok_or("Resource task application thread is retired")?;}
            ThreadLifetime::Worker(owner)=>if !owner.is_live(){return Err("Resource task worker allocation is retired".into())},
            ThreadLifetime::Simulation(owner)=>if owner.os_thread_id()?!=self.os_id{return Err("Resource task simulation OS identity changed".into())},
        }
        Ok(())
    }
    pub fn os_id(&self)->u32{self.os_id}
    pub fn application_is_live(&self)->bool{match &self.lifetime{ThreadLifetime::Application(v)=>v.strong_count()!=0,ThreadLifetime::Worker(_)|ThreadLifetime::Simulation(_)=>false}}
    pub fn is_for_resource_worker(&self,worker:&crate::execution_context::WorkerIdentity)->bool{
        match &self.lifetime{ThreadLifetime::Worker(witness)=>witness.is_for(worker),ThreadLifetime::Application(_)|ThreadLifetime::Simulation(_)=>false}
    }
}

struct Fields{
    references4:u32,priority8:u32,sequence_c:u32,
    queue10:Option<Weak<QueueStorage>>,state14:u32,thread18:Option<ThreadHandle>,
}
///One actual5B17F0 header, shared by its typed payload and queue node. The
///Application association is bound once at real enqueue for UI-created tasks.
pub(crate) struct Header{application:Mutex<Option<Weak<()>>>,fields:Mutex<Fields>}
impl Header{
    pub(crate) fn constructed(priority:u32,application:Option<Weak<()>>)->Arc<Self>{Arc::new(Self{application:Mutex::new(application),fields:Mutex::new(Fields{
        references4:0,priority8:priority,sequence_c:0,queue10:None,state14:0,thread18:None,
    })})}
    fn require_live_application(&self)->Result<()>{
        self.application.lock().map_err(|_|"Resource task Application lock poisoned")?.as_ref()
            .and_then(Weak::upgrade).ok_or("Resource task Application is unbound or retired")?;Ok(())
    }
    pub(crate) fn bind_application(&self,application:&Weak<()>)->Result<()>{
        application.upgrade().ok_or("Resource task queue Application is retired")?;
        let mut current=self.application.lock().map_err(|_|"Resource task Application lock poisoned")?;
        if let Some(old)=current.as_ref(){
            if old.upgrade().is_none()||!Weak::ptr_eq(old,application){return Err("Resource task belongs to another/retired Application".into())}
        }else{*current=Some(application.clone())}Ok(())
    }
    fn require_for(&self,application:&Weak<()>)->Result<()>{
        self.require_live_application()?;
        let owner=self.application.lock().map_err(|_|"Resource task Application lock poisoned")?;
        if !owner.as_ref().is_some_and(|old|Weak::ptr_eq(old,application)){return Err("Resource task queue belongs to another Application".into())}Ok(())
    }
    pub(crate) fn state(&self)->Result<u32>{Ok(self.fields.lock().map_err(|_|"Resource task field lock poisoned")?.state14)}
    pub(crate) fn thread(&self)->Result<Option<ThreadHandle>>{Ok(self.fields.lock().map_err(|_|"Resource task field lock poisoned")?.thread18.clone())}
    pub(crate) fn set_state(&self,state:u32,current:impl FnOnce()->Result<ThreadHandle>)->Result<()>{
        self.fields.lock().map_err(|_|"Resource task field lock poisoned")?.state14=state;
        if state==1{let thread=current()?;self.fields.lock().map_err(|_|"Resource task field lock poisoned")?.thread18=Some(thread)}Ok(())
    }
}
struct Allocation{resource:Handle,header:Arc<Header>}
impl std::ops::Deref for Allocation{type Target=Header;fn deref(&self)->&Header{&self.header}}
pub struct Task{allocation:Arc<Allocation>}
///Queue-only shared reference to the SAME task. Logical native+4 refcounts
///are changed explicitly by native retain/release operations, not Arc cloning.
#[derive(Clone)]
pub(crate) struct QueueHandle{allocation:Arc<Header>,payload:Payload}
#[derive(Clone)]
enum Payload{Resource(Arc<Allocation>),View(Arc<crate::view_resource_task::Allocation>)}
///Resource4C decoding, Resource3C completion and native cache/lock/log owners
///are mandatory real backend services. The task cannot fabricate a loaded bit
///or completion from a GPU request, source filename or queue membership.
pub trait Services{
    fn current_thread(&mut self)->Result<ThreadHandle>{Err("required native service: ResourceTask actual current thread owner".into())}
    fn enter_load(&mut self,_resource:&Handle)->Result<()>{Err("required native service: ResourceTask same manager load/cache guards".into())}
    fn load_resource_4c(&mut self,_resource:&Handle)->Result<()>{Err("required native service: Resource actual virtual4C decoder/backend".into())}
    fn after_resource_3c(&mut self,_resource:&Handle)->Result<()>{Err("required native service: Resource actual virtual3C".into())}
    fn resource_flags8(&mut self,_resource:&Handle)->Result<u32>{Err("required native service: Resource actual flags8 owner".into())}
    fn store_resource_flags8(&mut self,_resource:&Handle,_value:u32)->Result<()>{Err("required native service: Resource actual flags8 store".into())}
    fn loaded_diagnostic(&mut self,_resource:&Handle)->Result<()>{Err("required native service: ResourceTask Loaded diagnostic".into())}
    fn leave_load(&mut self,_resource:&Handle)->Result<()>{Err("required native service: ResourceTask load/cache guard release".into())}
    fn enter_request(&mut self,_resource:&Handle)->Result<()>{Err("required native service: Resource request cache guard".into())}
    fn cached_resource_59bfa0(&mut self,_resource:&Handle)->Result<()>{Err("required native service: Resource cache59BFA0 owner".into())}
    fn cached_resource_virtual2c(&mut self,_resource:&Handle)->Result<()>{Err("required native service: Resource cached virtual2C".into())}
    fn leave_request(&mut self,_resource:&Handle)->Result<()>{Err("required native service: Resource request cache guard release".into())}
    fn request_diagnostic(&mut self,_resource:&Handle,_immediate:bool)->Result<()>{Err("required native service: Resource RequestResource diagnostic".into())}
    fn block_timer_reset_59aa70(&mut self)->Result<()>{Err("required native service: Resource block current timer reset".into())}
    fn block_timer_start_59ab60(&mut self)->Result<()>{Err("required native service: Resource block current timer start".into())}
    fn removing_from_queue_diagnostic(&mut self,_resource:&Handle)->Result<()>{Err("required native service: Resource RemoveFromTaskQueue diagnostic".into())}
    fn timing_enabled_40eaa0(&mut self)->Result<bool>{Err("required native service: Resource block timing predicate".into())}
    fn block_diagnostic(&mut self,_resource:&Handle,_reason:BlockDiagnostic)->Result<()>{Err("required native service: Resource block diagnostic/current timer/thread".into())}
    fn block_till_ready_5b1fd0(&mut self,_task:&Task,_queue:&TaskQueue)->Result<()>{Err("required native service: Resource actual running-task waiter/events".into())}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum BlockDiagnostic{RanImmediately,CompletedWhileWaitingForRemove,WaitedForLoader}
///Opaque identity of the SAME application queue, for its actual dispatch lock.
pub struct ApplicationKey(Weak<()>);
impl ApplicationKey{pub(crate) fn require(&self,application:&Weak<()>)->Result<()>{
    self.0.upgrade().ok_or("Task dispatch Application is retired")?;
    if !Weak::ptr_eq(&self.0,application){return Err("Task dispatch guard belongs to another Application".into())}Ok(())
}}
pub enum DispatchKind<'a>{Resource(&'a Handle),View}
///Native5ADDD0 children outside the queue/header owners. Both admitted task
///classes have actual vf10/vf14=false and take manager+4 in this closed domain.
pub trait DispatchServices:Services{
    fn enter_dispatch(&mut self,application:&ApplicationKey,thread:&ThreadHandle)->Result<()>;
    fn dispatch_diagnostic(&mut self,kind:DispatchKind<'_>)->Result<()>;
    fn leave_dispatch(&mut self)->Result<()>;
}
///Diagnostic metadata only; this cannot complete a View or enable gameplay.
pub struct DispatchReport{pub sequence:u32,pub state:u32,pub view:bool,header:Weak<Header>}
impl DispatchReport{
    ///Identity metadata only. The actual Loading17 writer remains the callback.
    pub fn is_for_view(&self,task:&crate::view_resource_task::TaskWitness)->bool{self.view&&task.has_header(&self.header)}
}
struct QueueStorage{application:Weak<()>,fields:Mutex<QueueFields>,wake:Condvar}
struct QueueFields{sequence:u32,tasks:Vec<QueueHandle>,signalled:bool,running:Option<QueueHandle>,failed:Vec<QueueHandle>,waiters_128:Vec<()>}
///One Application's real task queue. Native fixed pool/list capacity is replaced
///by growable storage; ordering and explicit native references stay unchanged.
pub struct TaskQueue{storage:Arc<QueueStorage>}
///The native sequence+C assigned to another actual task class at its
///5AE160 publication. No raw constructor/Clone/default; this names the SAME
///Application counter and remains distinct from a ready/completion token.
pub struct ExternalSequence{queue:Weak<QueueStorage>,value:u32}
impl ExternalSequence{
    pub fn value(&self)->Result<u32>{
        let queue=self.queue.upgrade().ok_or("External task sequence queue is retired")?;
        queue.application.upgrade().ok_or("External task sequence Application is retired")?;
        Ok(self.value)
    }
}
impl TaskQueue{
    pub(crate) fn from_fresh_application(priority:&DefaultPriority)->Result<Self>{
        priority.application.upgrade().ok_or("Resource task queue Application is retired")?;
        Ok(Self{storage:Arc::new(QueueStorage{application:priority.application.clone(),fields:Mutex::new(QueueFields{sequence:0,tasks:Vec::new(),signalled:false,running:None,failed:Vec::new(),waiters_128:Vec::new()}),wake:Condvar::new()})})
    }
    fn require(&self,task:&QueueHandle)->Result<()>{
        self.storage.application.upgrade().ok_or("Resource task queue Application is retired")?;
        task.allocation.require_for(&self.storage.application)
    }
    ///Only the concrete application bridge for a real specialized task may
    ///call this at native5AE160, after construction and before list insertion.
    ///It does not enqueue that task, certify its private list owner or invent
    ///a successful heterogeneous ResourceManager worker.
    pub(crate) fn reserve_external_enqueue_5ae160(&self)->Result<ExternalSequence>{
        self.storage.application.upgrade().ok_or("Resource task queue Application is retired")?;
        let mut queue=self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?;
        let value=queue.sequence;queue.sequence=queue.sequence.wrapping_add(1);
        Ok(ExternalSequence{queue:Arc::downgrade(&self.storage),value})
    }
    pub fn len(&self)->Result<usize>{Ok(self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?.tasks.len())}
    pub fn resources_in_order(&self)->Result<Vec<Handle>>{
        Ok(self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?.tasks.iter().filter_map(|v|match &v.payload{Payload::Resource(r)=>Some(r.resource.clone()),Payload::View(_)=>None}).collect())
    }
    ///5AFB00→857840(0) creates an unnamed auto-reset event, initially clear.
    ///This real blocking backend consumes one pending SetEvent notification;
    ///it neither dequeues a task nor marks any resource ready.
    pub(crate) fn wait_for_work(&self)->Result<()>{
        self.storage.application.upgrade().ok_or("Resource task queue Application is retired")?;
        let fields=self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?;
        let mut fields=self.storage.wake.wait_while(fields,|v|!v.signalled).map_err(|_|"Resource task queue wait lock poisoned")?;
        fields.signalled=false;Ok(())
    }
    ///5B15B0: native HEAD order is signed priority ascending, then unsigned
    ///sequence descending. This is list order, not a claim about dequeue end.
    ///Equal entries insert before the first equal, as5ADBE0 actually does.
    fn insert(fields:&mut QueueFields,task:QueueHandle)->Result<()>{
        let proposed=task.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;
        let key=(proposed.priority8 as i32,proposed.sequence_c);drop(proposed);
        let mut index=0;
        while index<fields.tasks.len(){
            let old=fields.tasks[index].allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;
            let before=(old.priority8 as i32)<key.0 || (old.priority8 as i32==key.0 && old.sequence_c>key.1);
            if !before{break}index+=1;
        }
        fields.tasks.try_reserve(1).map_err(|e|e.to_string())?;
        task.retain()?;fields.tasks.insert(index,task);Ok(())
    }
    fn reorder(fields:&mut QueueFields,task:&QueueHandle)->Result<()>{
        let Some(index)=fields.tasks.iter().position(|v|Arc::ptr_eq(&v.allocation,&task.allocation))else{return Ok(())};
        let old=fields.tasks.remove(index);old.release_nonzero()?;
        Self::insert(fields,task.clone())
    }
    fn require_link(&self,link:&Option<Weak<QueueStorage>>)->Result<()>{
        if let Some(link)=link{
            if link.strong_count()==0 || !Weak::ptr_eq(link,&Arc::downgrade(&self.storage)){return Err("Resource task names another/retired native queue owner".into())}
        }Ok(())
    }
    ///5B1F50's immediate priority store followed by first-node reordering.
    pub fn set_priority_5b1f50(&self,task:&Task,priority:u32)->Result<()>{
        let task=task.queue_handle();self.require(&task)?;
        let mut queue=self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?;
        let mut fields=task.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;
        if fields.priority8==priority{return Ok(())}
        fields.priority8=priority;let link=fields.queue10.clone();drop(fields);
        self.require_link(&link)?;
        if link.is_some(){task.retain()?;let result=Self::reorder(&mut queue,&task);let released=task.release_nonzero();result?;released?}
        Ok(())
    }
    ///5AED10/5AE160: temporary retain, sequence advance, possible first-node
    ///reorder, node retain/insertion, state0, queue-owner store, temp release.
    pub fn enqueue_5aed10(&self,task:&Task)->Result<()>{
        self.enqueue_node(task.queue_handle())
    }
    pub(crate) fn enqueue_view_5aed10(&self,task:&crate::view_resource_task::Task)->Result<()>{
        task.header().bind_application(&self.storage.application)?;
        self.enqueue_node(QueueHandle{allocation:Arc::clone(task.header()),payload:Payload::View(task.allocation())})
    }
    fn enqueue_node(&self,task:QueueHandle)->Result<()>{
        self.require(&task)?;
        let mut queue=self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?;
        task.retain()?;
        let result:Result<()>=(||{
            let sequence=queue.sequence;queue.sequence=queue.sequence.wrapping_add(1);
            let mut fields=task.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;
            let changed=fields.sequence_c!=sequence;fields.sequence_c=sequence;let link=fields.queue10.clone();drop(fields);
            if changed && link.is_some(){self.require_link(&link)?;task.retain()?;let result=Self::reorder(&mut queue,&task);let released=task.release_nonzero();result?;released?}
            Self::insert(&mut queue,task.clone())?;
            //5AE1AE/8573A0 SetEvent, before state0 and queue-owner stores.
            queue.signalled=true;self.storage.wake.notify_one();
            let mut fields=task.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;
            fields.state14=0;fields.queue10=Some(Arc::downgrade(&self.storage));Ok(())
        })();
        let released=task.release_nonzero();result?;released
    }
    ///5AED90: remove only the first matching node, clear10 only on success,
    ///and balance the native temporary and node references separately.
    pub fn remove_5aed90(&self,task:&Task)->Result<bool>{
        let task=task.queue_handle();self.require(&task)?;
        let mut queue=self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?;
        task.retain()?;
        let result:Result<bool>=(||{
            let Some(index)=queue.tasks.iter().position(|v|Arc::ptr_eq(&v.allocation,&task.allocation))else{return Ok(false)};
            let old=queue.tasks.remove(index);old.release_nonzero()?;
            task.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?.queue10=None;
            Ok(true)
        })();
        let released=task.release_nonzero();let result=result?;released?;Ok(result)
    }
    ///5ADDD0's next actual tail node. Required child failure retains its
    ///popped allocation and all native status/refcount stores, refusing retry.
    ///The backend uses its real same-Application recursive manager+4 lock.
    pub fn dispatch_next_5addd0(&self,services:&mut impl DispatchServices)->Result<Option<DispatchReport>>{
        self.storage.application.upgrade().ok_or("Task dispatch Application is retired")?;
        let task={
            let mut queue=self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?;
            if queue.running.is_some()||!queue.failed.is_empty(){return Err("resource queue retains an in-flight/failed native dispatch prefix".into())}
            let Some(task)=queue.tasks.last().cloned()else{return Ok(None)};
            task.retain()?;
            let node=queue.tasks.pop().expect("same locked tail");node.release_nonzero()?;
            queue.running=Some(task.clone());task
        };
        let mut entered=false;
        let result:Result<DispatchReport>=(||{
            task.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?.queue10=None;
            task.allocation.set_state(1,||services.current_thread())?;
            let thread=task.allocation.thread()?.ok_or("native state1 did not retain actual Task18 thread")?;
            services.enter_dispatch(&ApplicationKey(self.storage.application.clone()),&thread)?;entered=true;
            services.dispatch_diagnostic(match &task.payload{Payload::Resource(r)=>DispatchKind::Resource(&r.resource),Payload::View(_)=>DispatchKind::View})?;
            if task.allocation.state()?==4{
                task.allocation.set_state(5,||unreachable!("state5 preserves18"))?;
            }else{match &task.payload{
                Payload::Resource(allocation)=>Task{allocation:Arc::clone(allocation)}.run_body_5b1a10(services)?,
                Payload::View(view)=>view.complete_callback()?,
            }}
            services.leave_dispatch()?;entered=false;
            //5B0400's final stores construct actual empty128/12C. No writer
            //is admitted yet: positive5B1FD0 remains explicitly required.
            if !self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?.waiters_128.is_empty(){
                return Err("5AC890 positive actual waiter signaling is required".into())
            }
            let fields=task.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;
            Ok(DispatchReport{sequence:fields.sequence_c,state:fields.state14,view:matches!(task.payload,Payload::View(_)),header:Arc::downgrade(&task.allocation)})
        })();
        //Portable required-service errors release host lock loans, preserving
        //the exact native field prefix separately from cleanup (no C++ throw claim).
        if entered{let cleanup=services.leave_dispatch();if result.is_ok(){cleanup?}}
        let mut queue=self.storage.fields.lock().map_err(|_|"Resource task queue lock poisoned")?;
        queue.running=None;
        match result{
            Ok(report)=>{if let Err(error)=task.release_after_dispatch(){queue.failed.push(task);return Err(error)}Ok(Some(report))},
            Err(error)=>{queue.failed.push(task);Err(error)},
        }
    }
}
impl QueueHandle{
    fn retain(&self)->Result<u32>{let mut fields=self.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;fields.references4=fields.references4.wrapping_add(1);Ok(fields.references4)}
    fn release_nonzero(&self)->Result<()>{
        let mut fields=self.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;
        fields.references4=fields.references4.wrapping_sub(1);
        if fields.references4==0{return Err("required native service: ResourceTask deleting destructor after final reference".into())}Ok(())
    }
    fn release_after_dispatch(&self)->Result<()>{
        let mut fields=self.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned")?;
        fields.references4=fields.references4.wrapping_sub(1);
        if fields.references4==0&&matches!(self.payload,Payload::Resource(_)){
            return Err("ResourceTask final release requires actual resource-owned destructor".into())
        }
        //View's actual5C38A0→5B18B0 only changes its dying vtable and frees
        //the header. Rust drops this same private allocation after the node.
        Ok(())
    }
}
impl Task{
    fn fields(&self)->Result<std::sync::MutexGuard<'_,Fields>>{
        self.allocation.require_live_application()?;
        self.allocation.fields.lock().map_err(|_|"Resource task field lock poisoned".into())
    }
    ///5B1850(resource,currentB282F0): all fields4..18 initialized, +1C names
    ///the same real Resource. Caller performs the separate5AA944 retain.
    pub(crate) fn construct_for_resource(resource:&Handle,priority:&DefaultPriority)->Result<Self>{
        let application=priority.application.upgrade().ok_or("Resource task default-priority Application is retired")?;
        resource.require_application(&application)?;
        Ok(Self{allocation:Arc::new(Allocation{resource:resource.clone(),header:Header::constructed(priority.current(),Some(priority.application.clone()))})})
    }
    pub fn resource(&self)->&Handle{&self.allocation.resource}
    pub fn references4(&self)->Result<u32>{Ok(self.fields()?.references4)}
    pub fn priority8(&self)->Result<u32>{Ok(self.fields()?.priority8)}
    pub fn sequence_c(&self)->Result<u32>{Ok(self.fields()?.sequence_c)}
    pub fn queued10(&self)->Result<bool>{Ok(self.fields()?.queue10.is_some())}
    pub fn state14(&self)->Result<u32>{Ok(self.fields()?.state14)}
    pub fn thread18(&self)->Result<Option<ThreadHandle>>{Ok(self.fields()?.thread18.clone())}
    ///InterlockedIncrement at5AA944, separate from task constructor5B1850.
    pub(crate) fn retain_in_resource_5aa944(&self)->Result<u32>{
        let mut fields=self.fields()?;
        fields.references4=fields.references4.wrapping_add(1);Ok(fields.references4)
    }
    pub(crate) fn queue_handle(&self)->QueueHandle{QueueHandle{allocation:Arc::clone(&self.allocation.header),payload:Payload::Resource(Arc::clone(&self.allocation))}}
    ///5B18C0 stores status BEFORE the fallible actual current-thread read.
    ///Other statuses deliberately preserve the old18 reference.
    pub(crate) fn set_state_5b18c0(&self,state:u32,current:impl FnOnce()->Result<ThreadHandle>)->Result<()>{
        self.allocation.require_live_application()?;self.allocation.set_state(state,current)
    }
    ///5B1910 followed by the real5B1A10 resource-task body. Required backend
    ///boundaries retain every earlier mutation (status1 until completion).
    ///A portable required-service Err does not emulate a thrown C++ exception;
    ///the native5B1B1D diagnostic/status3/rethrow path remains separately required.
    pub fn run_5b1910(&self,services:&mut impl Services)->Result<()>{
        self.set_state_5b18c0(1,||services.current_thread())?;
        self.run_body_5b1a10(services)
    }
    fn run_body_5b1a10(&self,services:&mut impl Services)->Result<()>{
        services.enter_load(self.resource())?;
        let result=(||{
            services.load_resource_4c(self.resource())?;
            services.after_resource_3c(self.resource())?;
            let flags=services.resource_flags8(self.resource())?;
            services.store_resource_flags8(self.resource(),flags|1)?;
            self.set_state_5b18c0(2,||unreachable!("native state2 never reads current thread"))?;
            services.loaded_diagnostic(self.resource())
        })();
        let cleanup=services.leave_load(self.resource());
        result?;cleanup
    }
    ///5B0270 operates on this task's SAME Resource. The bit5 cache branch
    ///rereads flags after entering the actual manager guard.
    pub fn request_5b0270(&self,queue:&TaskQueue,mode:u32,services:&mut impl Services)->Result<()>{
        if services.resource_flags8(self.resource())?&0x20!=0{
            services.enter_request(self.resource())?;
            let result:Result<bool>=(||{
                if services.resource_flags8(self.resource())?&0x20!=0{
                    services.cached_resource_59bfa0(self.resource())?;
                    services.cached_resource_virtual2c(self.resource())?;
                    Ok(true)
                }else{Ok(false)}
            })();
            let cleanup=services.leave_request(self.resource());
            let cached=result?;cleanup?;if cached{return Ok(())}
        }
        let immediate=mode==4;services.request_diagnostic(self.resource(),immediate)?;
        if immediate{self.run_5b1910(services)}else{
            queue.set_priority_5b1f50(self,mode)?;
            queue.enqueue_5aed10(self)
        }
    }
    ///5B2180 for the constructor-proven TaskLoad vtableA47790: virtual10 and
    ///14 are actual6A71F0 false leaves. Remove from the real queue before Run.
    ///Only a genuinely nonqueued/noncomplete running task reaches waiter5B1FD0.
    pub fn block_5b2180(&self,queue:&TaskQueue,services:&mut impl Services)->Result<()>{
        services.block_timer_reset_59aa70()?;services.block_timer_start_59ab60()?;
        services.removing_from_queue_diagnostic(self.resource())?;
        if queue.remove_5aed90(self)?{
            self.run_5b1910(services)?;
            if services.timing_enabled_40eaa0()?{services.block_diagnostic(self.resource(),BlockDiagnostic::RanImmediately)?}
            return Ok(())
        }
        let state=self.state14()?; //5B2211 loads once; both CMPs use EAX.
        if state==2 || state==3{
            //ActualTaskLoad RTTI5B1780 succeeds; its name/resource/thread
            //diagnostic is still a mandatory real current-owner callback.
            services.block_diagnostic(self.resource(),BlockDiagnostic::CompletedWhileWaitingForRemove)
        }else{
            services.block_till_ready_5b1fd0(self,queue)?;
            if services.timing_enabled_40eaa0()?{services.block_diagnostic(self.resource(),BlockDiagnostic::WaitedForLoader)?}
            Ok(())
        }
    }
}

#[cfg(test)]
#[path="resource_task_tests.rs"]
mod tests;

#[cfg(test)]
#[path="resource_task_dispatch_tests.rs"]
mod dispatch_tests;
