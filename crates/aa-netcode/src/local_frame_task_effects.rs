//! Actual315 effects in the real Admin callback interval. Other gameplay,
//! transport and graphics services remain the caller's required concrete port.
//! This wrapper grants no activation/view-completion capability.
use super::*;
use crate::local_host::MillisecondTimer;
use crate::local_order_host;
use aa_numeric::Precision;
use aa_orders::{
    flow::{Log, Validity},
    net::admin::{CurrentClient, RequestEmission},
    payload::Field,
};
use aa_runtime::{
    catalog::Catalog, execution_context::OsThreadIds, frame_variables,
    inactive_svar_dump::Inventory, live_sim::Sim, live_world_frame::Frame, simulation,
    simulation_execution::SimulationIdentity,
};

pub struct Effects<
    'a,
    'frame,
    'source,
    'owner,
    T: MillisecondTimer,
    F: frame_host::Focus + ?Sized,
    E: frame_host::Hooks,
    O: OsThreadIds,
> {
    pub frame: &'a Frame<'frame, 'static>,
    pub sim: &'a RefCell<&'owner mut Sim>,
    pub node: &'a RefCell<frame_host::NodeLoan<'owner, T>>,
    pub catalog: &'a Catalog<'source>,
    pub variables: &'a frame_variables::State,
    pub focus: &'a F,
    pub inventory: &'a Inventory,
    pub notifications: &'a Notifications,
    pub simulation: &'a SimulationIdentity,
    pub os: &'a RefCell<O>,
    pub abort: order_gate::AdminWriter,
    pub precision: Precision,
    pub diagnostics: &'a mut Vec<all_tasks_complete::Diagnostic>,
    pub delegate: &'a mut E,
}
fn order_error(e: impl std::fmt::Display) -> aa_orders::Error {
    aa_orders::Error::Handler {
        index: 315,
        position: aa_orders::Position::PACKET_START,
        message: e.to_string(),
    }
}
impl<T: MillisecondTimer, F: frame_host::Focus + ?Sized, E: frame_host::Hooks, O: OsThreadIds>
    local_order_host::Effects for Effects<'_, '_, '_, '_, T, F, E, O>
{
    fn version_ack_client_description(&mut self,p:&mut PlayersBorrow<'_>,l:&mut LocalClientBorrow<'_>,client:&crate::lobby::PeerLink)->aa_orders::Result<Vec<u16>> {
        if *client!=crate::lobby::PeerLink::Node{return Err(aa_orders::Error::SchemaContext("7515A0 actual remote client owner"));}
        let sim=self.sim.try_borrow().map_err(order_error)?;
        client_description::describe(p,l,&sim,self.frame,self.catalog)
    }
    fn version_ack_diagnostic(&mut self,kind:aa_orders::version_ack::Diagnostic,text:Option<&[u16]>)->aa_orders::Result<()> {
        self.delegate.version_ack_diagnostic(kind,text)
    }
    fn sync_log_gate(&mut self, _: Log) -> aa_orders::Result<bool> {
        self.simulation
            .sync_gate(&mut *self.os.try_borrow_mut().map_err(order_error)?, || {
                let flags = self.frame.sai_sync_flags()?;
                Ok(Some([flags[0], flags[1]]))
            })
            .map_err(order_error)
    }
    fn diagnostic(&mut self, v: local_order_host::Diagnostic) -> aa_orders::Result<()> {
        self.delegate.diagnostic(v)
    }
    fn stop_film(&mut self, p: &mut PlayersBorrow<'_>) -> aa_orders::Result<()> {
        self.delegate.stop_film(p)
    }
    fn set_admin_abort(&mut self) -> aa_orders::Result<()> {
        self.abort.store(1)
    }
    fn valid(
        &mut self,
        p: &mut PlayersBorrow<'_>,
        slot: &mut u32,
        c: &CurrentClient,
        v: Validity,
        r: usize,
        f: &[Field],
    ) -> aa_orders::Result<bool> {
        self.delegate.valid(p, slot, c, v, r, f)
    }
    fn process(
        &mut self,
        p: &mut PlayersBorrow<'_>,
        l: &mut LocalClientBorrow<'_>,
        slot: &mut u32,
        c: &CurrentClient,
        d: usize,
        f: &mut Vec<Field>,
    ) -> aa_orders::Result<()> {
        if d == 315 {
            return Err(aa_orders::Error::SchemaContext(
                "actual315 requires SAME reentrant Admin254/210 loan",
            ));
        }
        self.delegate.process(p, l, slot, c, d, f)
    }
    fn process_with_emission<'owner>(
        &mut self,
        p: &mut PlayersBorrow<'owner>,
        l: &mut LocalClientBorrow<'owner>,
        slot: &mut u32,
        c: &CurrentClient,
        d: usize,
        f: &mut Vec<Field>,
        emission: &mut RequestEmission<'_>,
        a: &RefCell<Synchronizer>,
        s: &mut Synchronizer,
    ) -> aa_orders::Result<()> {
        if d != 315 {
            return self
                .delegate
                .process_with_emission(p, l, slot, c, d, f, emission, a, s);
        }
        let [
            Field::Bounded {
                n: 5,
                value: status,
            },
        ] = f.as_slice()
        else {
            return Err(aa_orders::Error::SchemaField {
                expected: "native315 bounded5 task status",
            });
        };
        if *status >= 5 {
            return Err(aa_orders::Error::SchemaField {
                expected: "native315 task status0..4",
            });
        }
        let mut same_sim = self.sim.try_borrow_mut().map_err(order_error)?;
        let frame = self.frame;
        let simulation = self.simulation;
        let os = self.os;
        let mut gate = || {
            simulation.sync_gate(
                &mut *os.try_borrow_mut().map_err(|e| e.to_string())?,
                || {
                    let flags = frame.sai_sync_flags()?;
                    Ok(Some([flags[0], flags[1]]))
                },
            )
        };
        frame
            .with_native_order(|mut orders| {
                let current_world = orders.revision().witness();
                let mut services = all_tasks_services::Current {
                    orders: &mut orders,
                    catalog: self.catalog,
                    node: self.node,
                    variables: self.variables,
                    focus: self.focus,
                    sync_gate: &mut gate,
                    precision: self.precision,
                };
                let mut context = all_tasks_complete::Context {
                    catalog: self.catalog,
                    sim: &mut same_sim,
                    current_world,
                    game_spy: frame.game_spy_manager_gate(),
                    inventory: self.inventory,
                    sync_a: a,
                    sync_s: s,
                    emission,
                    notifications: self.notifications,
                    services: &mut services,
                    diagnostics: self.diagnostics,
                };
                all_tasks_complete::process_task_status(p, l, slot, *status, &mut context)
                    .map_err(|e| e.to_string())
            })
            .map_err(order_error)
    }
}
impl<T: MillisecondTimer, F: frame_host::Focus + ?Sized, E: frame_host::Hooks, O: OsThreadIds>
    frame_host::Hooks for Effects<'_, '_, '_, '_, T, F, E, O>
{
    fn net_pump(
        &mut self,
        p: &mut PlayersBorrow<'_>,
        l: &mut LocalClientBorrow<'_>,
    ) -> simulation::Result<()> {
        self.delegate.net_pump(p, l)
    }
    fn process_commands(
        &mut self,
        n: &mut aa_orders::net::admin::AdminNode,
        p: &mut PlayersBorrow<'_>,
        l: &mut LocalClientBorrow<'_>,
        g: &mut order_gate::Borrow,
    ) -> simulation::Result<()> {
        self.delegate.process_commands(n, p, l, g)
    }
    fn avi_pump(&mut self) -> simulation::Result<i32> {
        self.delegate.avi_pump()
    }
    fn timing_smoothing(&mut self) -> simulation::Result<()> {
        self.delegate.timing_smoothing()
    }
    fn end_condition_check(&mut self, p: &mut PlayersBorrow<'_>) -> simulation::Result<()> {
        struct Messages<'a,H>(&'a mut H);
        impl<H:frame_host::Hooks> frame_end_world::MessageEffects for Messages<'_,H>{
            fn show_message(&mut self,text:&[u16])->Result<()>{self.0.mission_end_message(text).map_err(|e|missing(e.to_string()))}
            fn gamespy_remaining(&mut self,name:&[u16])->Result<()>{self.0.mission_end_gamespy(name).map_err(|e|missing(e.to_string()))}
        }
        let sim=self.sim.try_borrow().map_err(|e|simulation::Error::Seam(e.to_string()))?;
        frame_end_world::run(p,self.frame,&sim,self.catalog,&mut Messages(self.delegate))
            .map_err(|e|simulation::Error::Seam(e.to_string()))
    }
    fn heartbeat(
        &mut self,
        p: &mut PlayersBorrow<'_>,
        l: &mut LocalClientBorrow<'_>,
        n: &mut aa_orders::net::admin::AdminNode,
        sources:&dyn frame_heartbeat::Sources,
    ) -> simulation::Result<()> {
        self.delegate.heartbeat(p, l, n,sources)
    }
    fn backup_mode_entered(&mut self) -> simulation::Result<()> {
        self.delegate.backup_mode_entered()
    }
}
