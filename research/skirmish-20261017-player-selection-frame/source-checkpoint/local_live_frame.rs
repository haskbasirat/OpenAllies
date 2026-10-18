//!Product75CA70 composition on the SAME retained Host/World/application.
//!Only the genuine ActivatedSession exposes this entry. No Driver, Gate,
//!Timeline, clock or Admin is reconstructed. Positive missing leaves error
//!at their actual call and the existing process/World latches retain effects.
use super::*;
use aa_numeric::Precision;
use aa_orders::{flow::{Log,Validity},payload::Field,net::admin::{AdminNode,CurrentClient}};
use aa_runtime::{catalog::Catalog,execution_context::OsThreadIds,live_first_tick::AudioOutput,live_world_frame::{Frame,TickInputs},simulation::{self,Action},simulation_execution::SimulationIdentity};
use crate::{local_host::{NodeTime,MillisecondTimer},local_order_host};

#[derive(Default)]
pub struct Diagnostics {pub tasks:Vec<all_tasks_complete::Diagnostic>,pub effects:Vec<String>}
///Lazy current Ui.uvars read on the SAME backend used by audio.
pub trait Frontend{fn dev_mode_current(&mut self)->std::result::Result<u8,String>;}
fn order_error(e:impl ToString)->aa_orders::Error{aa_orders::Error::SchemaSource(e.to_string())}
fn frame_error(e:impl ToString)->simulation::Error{simulation::Error::Seam(e.to_string())}
fn required<T>(what:&'static str)->aa_orders::Result<T>{Err(aa_orders::Error::SchemaContext(what))}

struct Hooks<'a,'frame,'backend,'source,O,A:?Sized>{frame:&'a Frame<'frame,'static>,simulation:&'a SimulationIdentity,os:&'a RefCell<O>,sim:&'a RefCell<&'backend mut aa_runtime::live_sim::Sim>,frontend:&'a RefCell<&'backend mut A>,catalog:&'a Catalog<'source>,abort:order_gate::AdminWriter,precision:Precision,diagnostics:&'a mut Vec<String>}
impl<O:OsThreadIds,A:Frontend+?Sized> local_order_host::Effects for Hooks<'_, '_, '_,'_,O,A>{
 fn sync_log_gate(&mut self,_:Log)->aa_orders::Result<bool>{self.simulation.sync_gate(&mut*self.os.try_borrow_mut().map_err(order_error)?,||{let f=self.frame.sai_sync_flags();Ok(Some([f[0],f[1]]))}).map_err(order_error)}
 fn diagnostic(&mut self,v:local_order_host::Diagnostic)->aa_orders::Result<()>{self.diagnostics.push(format!("Native order {v:?}"));Ok(())}
 fn stop_film(&mut self,_:&mut PlayersBorrow<'_>)->aa_orders::Result<()>{required("actual retained film stop/restart service")}
 fn set_admin_abort(&mut self)->aa_orders::Result<()>{self.abort.store(1)}
 fn valid(&mut self,_:&mut PlayersBorrow<'_>,_:&mut u32,_:&CurrentClient,_:Validity,_:usize,_:&[Field])->aa_orders::Result<bool>{required("actual gameplay order validity child")}
 fn process(&mut self,p:&mut PlayersBorrow<'_>,_:&mut LocalClientBorrow<'_>,slot:&mut u32,_:&CurrentClient,d:usize,f:&mut Vec<Field>)->aa_orders::Result<()>{
    if matches!(d,16|17|18|86|87|333){return gameplay_order::process(self.frame,self.sim,self.catalog,p,slot,d,f,self.frontend)}
    required("actual gameplay decree child")
 }
}
struct HeartbeatLog<'a>(&'a mut Vec<String>);
impl frame_heartbeat::Diagnostics for HeartbeatLog<'_>{
 fn backups_not_ready(&mut self)->Result<()>{self.0.push("NET: Backups not ready".to_owned());Ok(())}
 fn processing_backup_decree(&mut self,i:i32)->Result<()>{self.0.push(format!("NET: Processing backup decree {i}"));Ok(())}
}
impl<O:OsThreadIds,A:Frontend+?Sized> frame_host::Hooks for Hooks<'_, '_, '_,'_,O,A>{
 fn net_pump(&mut self,p:&mut PlayersBorrow<'_>,l:&mut LocalClientBorrow<'_>)->simulation::Result<()>{session_frame_update::run(p,l).map_err(frame_error)}
 fn process_commands(&mut self,_:&mut AdminNode,_:&mut PlayersBorrow<'_>,_:&mut LocalClientBorrow<'_>,_:&mut order_gate::Borrow)->simulation::Result<()>{Err(simulation::Error::Unsupported("actual positive Node118/14C command pump"))}
 fn avi_pump(&mut self)->simulation::Result<i32>{Err(simulation::Error::Unsupported("actual AVI source service"))}
 fn timing_smoothing(&mut self)->simulation::Result<()>{self.frame.with_process_effects(|timing,_|timing.update_with_unbound_services(self.precision)).map_err(frame_error)}
 fn end_condition_check(&mut self,_:&mut PlayersBorrow<'_>)->simulation::Result<()>{Err(simulation::Error::Unsupported("end condition must use same frame task-effects adapter"))}
 fn heartbeat(&mut self,p:&mut PlayersBorrow<'_>,l:&mut LocalClientBorrow<'_>,_:&mut AdminNode,s:&dyn frame_heartbeat::Sources)->simulation::Result<()>{let diagnostics=&mut*self.diagnostics;self.frame.with_process_effects(|_,counter|frame_heartbeat::run(p,l,counter,s,&mut HeartbeatLog(diagnostics)).map_err(|e|e.to_string())).map_err(frame_error)}
 fn backup_mode_entered(&mut self)->simulation::Result<()>{Err(simulation::Error::Unsupported("actual backup-mode presentation callback"))}
}
impl world_ready::ActualActivatedSession {
 #[allow(clippy::too_many_arguments)]
 pub fn run_native_frame<T:MillisecondTimer,F:frame_host::Focus+?Sized,O:OsThreadIds,A:AudioOutput+Frontend+?Sized>(&mut self,
 catalog:&Catalog<'_>,time:&mut NodeTime<T>,focus:&F,notifications:&Notifications,
 identity:&SimulationIdentity,os:&RefCell<O>,audio:&mut A,diagnostics:&mut Diagnostics,precision:Precision,
 )->Result<Action>{
  if identity.os_thread_id().map_err(missing)?!=os.try_borrow_mut().map_err(|e|missing(e.to_string()))?.current_thread_id().map_err(missing)?{return Err(missing("live frame must run on SAME retained simulation thread"))}
  time.begin_update_prefix(precision)?;let now=aa_simulation::FloatBits(time.cached_seconds_bits());
  let (host,world,application)=self.parts();let parts=application.parts(catalog).map_err(missing)?;
  let frame=world.borrow_frame();let cells=frame_host::Cells::from_runtime(host.runtime_borrow()?,time)?;
  let audio_cell=RefCell::new(audio);
  let mut leaf=Hooks{frame:&frame,simulation:identity,os,sim:cells.same_sim(),frontend:&audio_cell,catalog,abort:cells.abort_writer()?,precision,diagnostics:&mut diagnostics.effects};
  let mut effects=frame_task_effects::Effects{frame:&frame,sim:cells.same_sim(),node:cells.same_node(),catalog,variables:parts.frame_variables,focus,inventory:parts.svar_inventory,notifications,simulation:identity,os,abort:cells.abort_writer()?,precision,diagnostics:&mut diagnostics.tasks,delegate:&mut leaf};
  let data=game_data::Current::new(catalog,&frame,cells.same_sim(),cells.players());
  let mut network=frame_host::Host::bind(&cells,catalog,parts.frame_variables,focus,&data,&mut effects,precision)?;
  let mut gate=||identity.sync_gate(&mut*os.try_borrow_mut().map_err(|e|e.to_string())?,||{let f=frame.sai_sync_flags();Ok(Some([f[0],f[1]]))});
  let gate_loan:&mut dyn FnMut()->std::result::Result<bool,String>=&mut gate;
  let gate_cell=RefCell::new(gate_loan);
  frame.run(cells.same_sim(),now,&mut network,TickInputs{session:cells.players(),sync_a:cells.same_a(),sync_gate:&gate_cell,audio:&audio_cell}).map_err(|e|missing(e.to_string()))
 }
}
