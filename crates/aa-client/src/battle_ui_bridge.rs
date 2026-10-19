//!Actual UI/backend-thread work requested by the retained simulation thread.
//!Only immutable Send packets/results cross the queue. UI/World/renderer owners
//!stay on their existing threads. Closing this queue cancels pending replies
//!BEFORE SimulationThread is joined, including when the window is closed.
use std::{sync::{Arc,Mutex,Weak,mpsc,atomic::{AtomicBool,Ordering}},thread::ThreadId};
use aa_runtime::{execution_context::ApplicationThread,simulation_execution::Execution};
type Result<T>=std::result::Result<T,String>;
pub struct Context<'a>{
    pub ui:&'a mut aa_ui::ui::Ui,pub renderer:&'a mut crate::gpu::SceneRenderer,
    pub store:&'a Arc<aa_assets::AssetStore>,pub load_settings:&'a crate::loading::LoadSettings,
    pub battle_scene:&'a mut crate::battle_scene_startup::UiScene,
    pub menus:&'a mut crate::battle_menus::State,
    pub audio:&'a mut crate::audio::Audio,
}
trait Job:Send{
    fn run(self:Box<Self>,context:&mut Context<'_>);
    fn cancel(self:Box<Self>,reason:&str);
}
struct Work<F,T>{call:F,reply:mpsc::Sender<Result<T>>}
impl<F,T> Job for Work<F,T> where F:FnOnce(&mut Context<'_>)->Result<T>+Send,T:Send{
    fn run(self:Box<Self>,context:&mut Context<'_>){
        let Self{call,reply}=*self;
        let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||call(context)))
            .unwrap_or_else(|_|Err("UI/backend operation panicked; partial native owners are retained".into()));
        let _=reply.send(result);
    }
    fn cancel(self:Box<Self>,reason:&str){let _=self.reply.send(Err(reason.into()));}
}
struct Shared{
    sender:Mutex<Option<mpsc::Sender<Box<dyn Job>>>>,
    application:Mutex<Option<Weak<ApplicationThread>>>,
    ui_thread:ThreadId,closed:AtomicBool,
    wake:Option<Arc<dyn Fn()+Send+Sync>>,
}
#[derive(Clone)]
pub struct Client(Arc<Shared>);
pub struct Queue{shared:Arc<Shared>,receiver:mpsc::Receiver<Box<dyn Job>>}
impl Queue{
    pub fn new()->(Self,Client){
        Self::build(None)
    }
    pub fn with_wakeup(wake:impl Fn()+Send+Sync+'static)->(Self,Client){
        Self::build(Some(Arc::new(wake)))
    }
    fn build(wake:Option<Arc<dyn Fn()+Send+Sync>>)->(Self,Client){
        let(sender,receiver)=mpsc::channel();
        let shared=Arc::new(Shared{sender:Mutex::new(Some(sender)),application:Mutex::new(None),
            ui_thread:std::thread::current().id(),closed:AtomicBool::new(false),wake});
        (Self{shared:Arc::clone(&shared),receiver},Client(shared))
    }
    ///Called each real GUI update, before polling simulation results. Running
    ///only queued work keeps the event loop available while native code waits.
    pub fn pump(&mut self,context:&mut Context<'_>)->Result<usize>{
        if std::thread::current().id()!=self.shared.ui_thread{return Err("UI/backend queue pumped on a different thread".into())}
        if self.shared.closed.load(Ordering::Acquire){return Ok(0)}
        let mut count=0;
        while let Ok(job)=self.receiver.try_recv(){job.run(context);count+=1;}
        //The simulation thread blocks on each call and usually posts the next
        //one right after its reply. Keep serving for a short bounded budget so
        //such a chain runs within one GUI frame instead of one call per frame.
        //Jobs still run in FIFO order on this same UI thread; only latency
        //changes, and no job runs that would not have run on a later frame.
        if count>0{
            let deadline=std::time::Instant::now()+std::time::Duration::from_millis(8);
            loop{
                let now=std::time::Instant::now();
                if now>=deadline||self.shared.closed.load(Ordering::Acquire){break}
                let wait=(deadline-now).min(std::time::Duration::from_millis(2));
                match self.receiver.recv_timeout(wait){
                    Ok(job)=>{job.run(context);count+=1;}
                    Err(_)=>break,
                }
            }
        }
        Ok(count)
    }
    pub fn close(&mut self){
        self.shared.closed.store(true,Ordering::Release);
        //Recover this same channel owner if a previous sender panicked while
        //holding its lock. Never replace it with a new connected queue.
        self.shared.sender.lock().unwrap_or_else(|e|e.into_inner()).take();
        while let Ok(job)=self.receiver.try_recv(){job.cancel("application UI/backend queue closed");}
    }
}
impl Drop for Queue{fn drop(&mut self){self.close();}}
impl Client{
    pub fn wake(&self){if let Some(wake)=&self.0.wake{wake();}}
    pub fn terrain_quad_current(&self,execution:&mut Execution<'_>)->Result<i32>{
        self.call(execution,|c|c.ui.uvars.terrain_quad_size_current())
    }
    pub fn pre_request_all_vthings_current(&self,execution:&mut Execution<'_>)->Result<u8>{
        self.call(execution,|c|c.ui.uvars.renderer_pre_request_all_vthings())
    }
    pub fn use_render_lists_current(&self,execution:&mut Execution<'_>)->Result<bool>{
        self.call(execution,|c|c.ui.uvars.renderer_use_render_lists_current())
    }
    ///Bind the actual Engine application's captured frontend allocation, not a
    ///numeric role. A later attempt can bind only after the old owner expires.
    pub fn bind_application(&self,application:&Arc<ApplicationThread>)->Result<()>{
        if std::thread::current().id()!=self.0.ui_thread{return Err("UI/backend binding requires its original frontend thread".into())}
        if self.0.closed.load(Ordering::Acquire){return Err("application UI/backend queue closed".into())}
        let mut slot=self.0.application.lock().map_err(|_|"UI/backend application identity lock poisoned")?;
        if let Some(old)=slot.as_ref().and_then(Weak::upgrade){
            if !Arc::ptr_eq(&old,application){return Err("UI/backend queue already belongs to another live application".into())}
        }
        *slot=Some(Arc::downgrade(application));Ok(())
    }
    fn post<T:Send+'static>(&self,call:impl FnOnce(&mut Context<'_>)->Result<T>+Send+'static)->Result<mpsc::Receiver<Result<T>>>{
        let(reply,result)=mpsc::channel();let job:Box<dyn Job>=Box::new(Work{call,reply});
        let guard=self.0.sender.lock().map_err(|_|"UI/backend sender lock poisoned")?;
        if self.0.closed.load(Ordering::Acquire){return Err("application UI/backend queue closed".into())}
        guard.as_ref().ok_or("application UI/backend sender retired")?.send(job)
            .map_err(|_|"application UI/backend receiver retired")?;
        drop(guard);self.wake();
        Ok(result)
    }
    ///Synchronous native child on the simulation thread; the GUI continues to
    ///pump. Do not use Controller::with_activated (which blocks the GUI) around
    ///this call: submit_activated plus normal GUI polling is the required path.
    pub fn call<T:Send+'static>(&self,execution:&mut Execution<'_>,
        call:impl FnOnce(&mut Context<'_>)->Result<T>+Send+'static)->Result<T>{
        if std::thread::current().id()==self.0.ui_thread{return Err("UI/backend synchronous wait refused on frontend thread".into())}
        let application=self.0.application.lock().map_err(|_|"UI/backend application identity lock poisoned")?
            .as_ref().and_then(Weak::upgrade).ok_or("UI/backend application owner is absent or expired")?;
        let identity=execution.identity();identity.require_application(&application)?;
        if execution.current_os_thread_id()?!=identity.os_thread_id()?{
            return Err("UI/backend operation is outside its retained simulation thread".into())
        }
        self.post(call)?.recv().map_err(|_|"UI/backend operation lost its reply")?
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn closing_ui_queue_releases_blocked_reply_and_rejects_later_work(){
        let(mut queue,client)=Queue::new();
        let reply=client.post(|_|->Result<u32>{panic!("closed job must not execute")}).unwrap();
        let waiting=std::thread::spawn(move||reply.recv().unwrap());
        queue.close();assert!(waiting.join().unwrap().unwrap_err().contains("closed"));
        assert!(client.post(|_|Ok(1u32)).is_err());
    }
    #[test]
    fn dropping_queue_cancels_all_pending_jobs_even_while_clients_survive(){
        let(queue,client)=Queue::new();let a=client.post(|_|Ok(1u32)).unwrap();let b=client.post(|_|Ok(2u32)).unwrap();
        drop(queue);assert!(a.recv().unwrap().is_err());assert!(b.recv().unwrap().is_err());
        assert!(client.post(|_|Ok(3u32)).is_err());
    }
}
